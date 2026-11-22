//! Befehle der Mission waehrend des Spiels (Nutzer 2026-11-22: "Man sagt ihr, was sie tun soll, und sie macht es").
//! Die Mission (Codegen-Editor, Aktion "op2-ki") schickt Text, "name=wert" je Zeile (";" geht auch):
//!
//!   phase=2  bauen=Tokamak:2,Agridome:1,GuardPost:3@150,140,8  forschen=3401,5201  fahrzeuge=CargoTruck:2,Lynx/Microwave:4
//!            grundversorgung=1  reihenfolge=1                 -> Drehbuch: die KI baut und forscht nur das
//!   drehbuch=0                                                 -> Drehbuch aus, die KI entscheidet wieder selbst
//!   angriff=1  ziel=basis|basis:0|punkt:x,y|schwaechste  art=basis|ziel|ueberfall|halten  parallel=1  emp=1
//!            einheiten=Lynx/Microwave:6,Panther/RPG:4|beste:8|vorhanden  rueckzug=50  frist=20000
//!   verteidigung=1  wache=6  einheiten=Tiger/ESG:4  bereich=x1,y1,x2,y2  turm=ESG  patrouille=0
//!   rolle=2  stufe=1  eigeneAngriffe=0  aussenposten=x,y,groesse[,bakeX,bakeY]  und jede Stellschraube (zbWache=6 ...)
//!
//! Rueckmeldungen liest die Mission mit status(): Phasen, Angriffe, Angriffe auf die Basis, Satelliten, Gebaeude, Armee.
//! Mission commands during the game: script phases (build/research/vehicle lists), attack and defence orders, settings.
use std::cell::RefCell;

use super::*;

/// Ein Bau der Phase: Typ, Anzahl, optional Ort (x, y, Umkreis)
#[derive(Clone, Debug, Default)]
pub struct BauPosten { pub typ: i32, pub n: i32, pub ort: Option<(i32, i32, i32)> }

/// Fahrzeug/Einheit: Fahrgestell, Waffe (mapNone ohne), Anzahl
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Einheit { pub typ: i32, pub waffe: i32, pub n: i32 }

/// Drehbuch-Phase
#[derive(Clone, Debug, Default)]
pub struct Phase { pub id: i32, pub bauen: Vec<BauPosten>, pub forschen: Vec<i32>, pub fahrzeuge: Vec<Einheit>, pub grund: bool, pub reihe: bool, pub seit: i32 }

/// Einheiten eines Angriffs/der Wache
#[derive(Clone, Debug, Default, PartialEq)]
pub enum Truppe { #[default] Auto, Liste(Vec<Einheit>), Beste(i32), Vorhanden }

/// Angriffsbefehl
#[derive(Clone, Debug, Default)]
pub struct Angriff { pub ziel: String, pub art: String, pub parallel: bool, pub emp: Option<bool>, pub truppe: Truppe, pub rueckzug: i32, pub frist: i32, pub halten: Option<(i32, i32)> }

/// Verteidigungsbefehl
#[derive(Clone, Debug, Default)]
pub struct Verteidigung { pub truppe: Truppe, pub bereich: Option<(i32, i32, i32, i32)>, pub turm: i32 }

#[derive(Clone, Debug, Default)]
struct Stand {
    phase: Option<Phase>,
    fertig: Vec<i32>,
    zuletzt: i32,
    angriff: Option<Angriff>,
    /// 0 keiner, 1 Truppe wird gebaut, 2 unterwegs, 3 zuletzt gewonnen, 4 zuletzt abgebrochen
    angriffStand: i32,
    angriffeFertig: i32,
    verteidigung: Option<Verteidigung>,
    listen: (Vec<Einheit>, Vec<Einheit>),
    fzSeit: i32,
    /// hoechste Staerke der laufenden Angriffstruppe (Vergleich fuer den Rueckzug)
    angriffMax: i32,
    ortWunsch: Option<(i32, i32, i32, i32)>,
}

thread_local! { static STAND: RefCell<[Stand; 8]> = RefCell::new(Default::default()); }

fn sp(z: &Zustand) -> usize { (z.spieler & 7) as usize }
fn mit<R>(z: &Zustand, f: impl FnOnce(&mut Stand) -> R) -> R { STAND.with(|s| f(&mut s.borrow_mut()[sp(z)])) }

/// Neues Spiel
pub fn zuruecksetzen(z: &Zustand) { mit(z, |s| *s = Stand::default()); }

// ---------------------------------------------------------------------------------------------- Lesen der Befehle

/// Typ nach Namen ("Tokamak", "mapTokamak", "lynx"); None unbekannt
fn typNach(welt: &mut dyn Welt, s: &str) -> Option<i32> {
    let s = s.trim(); let s = s.strip_prefix("map").unwrap_or(s);
    if s.is_empty() { return None; }
    if s.eq_ignore_ascii_case("none") { return Some(mapNone); }
    (1..0x72).find(|&k| name(welt, k).eq_ignore_ascii_case(s))
}

/// "Lynx/Microwave:6,Panther:2" -> Einheiten
fn einheitenLesen(welt: &mut dyn Welt, s: &str) -> Vec<Einheit> {
    let mut v = Vec::new();
    for teil in s.split(',') {
        let (was, n) = match teil.split_once(':') { Some((a, b)) => (a, b.trim().parse().unwrap_or(1)), None => (teil, 1) };
        let (f, w) = match was.split_once('/') { Some((a, b)) => (a, Some(b)), None => (was, None) };
        let Some(typ) = typNach(welt, f) else { continue };
        let waffe = w.and_then(|w| typNach(welt, w)).unwrap_or(mapNone);
        if n > 0 { v.push(Einheit { typ, waffe, n }); }
    }
    v
}

fn truppeLesen(welt: &mut dyn Welt, s: &str) -> Truppe {
    let s = s.trim();
    if s.eq_ignore_ascii_case("vorhanden") { return Truppe::Vorhanden; }
    if let Some(n) = s.strip_prefix("beste:").or_else(|| s.strip_prefix("beste ")) { return Truppe::Beste(n.trim().parse().unwrap_or(6)); }
    if s.eq_ignore_ascii_case("beste") || s.eq_ignore_ascii_case("auto") || s.is_empty() { return Truppe::Auto; }
    let v = einheitenLesen(welt, s);
    if v.is_empty() { Truppe::Auto } else { Truppe::Liste(v) }
}

fn zahlen(s: &str) -> Vec<i32> { s.split(|c: char| c == ',' || c == ' ').filter_map(|x| x.trim().parse().ok()).collect() }

/// Befehl der Mission ausfuehren (Text "name=wert" je Zeile oder durch ";" getrennt)
pub fn befehl(welt: &mut dyn Welt, z: &mut Zustand, text: &str) {
    let t = welt.Tick();
    let paare: Vec<(String, String)> = text.split(['\n', ';']).filter_map(|z| {
        let z = z.trim(); if z.is_empty() || z.starts_with('#') { return None; }
        z.split_once('=').map(|(a, b)| (a.trim().to_string(), b.trim().to_string()))
    }).collect();
    let wert = |k: &str| paare.iter().find(|(a, _)| a == k).map(|(_, b)| b.clone());
    // Drehbuch-Phase
    if let Some(id) = wert("phase") {
        let mut ph = Phase { id: id.parse().unwrap_or(1), seit: t, grund: wert("grundversorgung").map(|v| v != "0").unwrap_or(true),
                             reihe: wert("reihenfolge").map(|v| v != "0").unwrap_or(false), ..Default::default() };
        // Orte stehen hinter "@" mit Kommas (Typ:Anzahl@x,y,Umkreis)
        ph.bauen = bauenNeuLesen(welt, z, wert("bauen").as_deref().unwrap_or(""));
        if let Some(f) = wert("forschen") { ph.forschen = zahlen(&f).into_iter().filter(|&x| x > 0).collect(); }
        if let Some(f) = wert("fahrzeuge") { ph.fahrzeuge = einheitenLesen(welt, &f); }
        let txt = format!("Befehl: Phase {} -- {} Bauten, {} Forschungen, {} Fahrzeuge{}{}", ph.id, ph.bauen.len(), ph.forschen.len(),
                          ph.fahrzeuge.len(), if ph.grund { ", Grundversorgung selbst" } else { ", ohne Grundversorgung" }, if ph.reihe { ", der Reihe nach" } else { "" });
        mit(z, |s| s.phase = Some(ph));
        melde(welt, z, "befehl", mapNone, z.bx, z.by, &txt);
    }
    if let Some(v) = wert("drehbuch") {
        if v == "0" { mit(z, |s| s.phase = None); melde(welt, z, "befehl", mapNone, z.bx, z.by, "Befehl: Drehbuch aus -- die KI entscheidet wieder selbst"); }
    }
    // Angriff
    if wert("angriff").is_some_and(|v| v != "0") {
        let a = Angriff {
            ziel: wert("ziel").unwrap_or_else(|| "basis".into()), art: wert("art").unwrap_or_else(|| "basis".into()),
            parallel: wert("parallel").is_some_and(|v| v != "0"), emp: wert("emp").map(|v| v != "0"),
            truppe: truppeLesen(welt, wert("einheiten").as_deref().unwrap_or("")),
            rueckzug: wert("rueckzug").and_then(|v| v.parse().ok()).unwrap_or(0).clamp(0, 95),
            frist: wert("frist").and_then(|v| v.parse().ok()).unwrap_or(20000), halten: None,
        };
        let staerke = wert("staerke").and_then(|v| v.parse().ok()).unwrap_or(1.0);
        let ort = zielOrt(welt, z, &a.ziel);
        let txt = format!("Befehl: Angriff ({}, Art {}{}{}, Einheiten {:?}, Rueckzug {} %)", a.ziel, a.art,
                          if a.parallel { ", zwei Wege" } else { "" }, match a.emp { Some(true) => ", mit EMP", Some(false) => ", ohne EMP", None => "" }, a.truppe, a.rueckzug);
        mit(z, |s| { s.angriff = Some(a); s.angriffStand = 1; });
        super::modus::setzeAngriff(welt, z, ort, staerke);
        melde(welt, z, "befehl", mapLynx, z.bx, z.by, &txt);
    }
    if wert("angriff").as_deref() == Some("0") {
        mit(z, |s| { s.angriff = None; if s.angriffStand == 1 { s.angriffStand = 0; } });
        super::modus::auftragErledigt(z);
        melde(welt, z, "befehl", mapLynx, z.bx, z.by, "Befehl: Angriffsauftrag zurueckgenommen");
    }
    // Verteidigung
    if let Some(v) = wert("verteidigung") {
        if v == "0" { mit(z, |s| s.verteidigung = None); melde(welt, z, "befehl", mapGuardPost, z.bx, z.by, "Befehl: Verteidigung wieder selbst"); }
        else {
            let mut d = Verteidigung { truppe: truppeLesen(welt, wert("einheiten").as_deref().unwrap_or("")), ..Default::default() };
            if let Some(n) = wert("wache").and_then(|v| v.parse::<i32>().ok()) { if d.truppe == Truppe::Auto { d.truppe = Truppe::Beste(n); } }
            if let Some(b) = wert("bereich") { let v = zahlen(&b); if v.len() >= 4 { d.bereich = Some((v[0].min(v[2]), v[1].min(v[3]), v[0].max(v[2]), v[1].max(v[3]))); } }
            if let Some(w) = wert("turm") { d.turm = typNach(welt, &w).unwrap_or(0); }
            let txt = format!("Befehl: Verteidigung -- Wache {:?}{}{}", d.truppe, d.bereich.map(|b| format!(", Bereich {},{}-{},{}", b.0, b.1, b.2, b.3)).unwrap_or_default(),
                              if d.turm != 0 { format!(", Tuerme mit {}", name(welt, d.turm)) } else { String::new() });
            mit(z, |s| s.verteidigung = Some(d));
            melde(welt, z, "befehl", mapGuardPost, z.bx, z.by, &txt);
        }
    }
    // Aussenposten
    if let Some(p) = wert("aussenposten") {
        let v = zahlen(&p);
        if v.len() >= 2 {
            let baken: Vec<(i32, i32)> = if v.len() >= 5 { vec![(v[3], v[4])] } else { Vec::new() };
            super::modus::setzeAussenposten(welt, z, v[0], v[1], v.get(2).copied().unwrap_or(4), &baken);
        }
    }
    // alles andere: Rolle, Stufe, eigene Angriffe, Stellschrauben
    for (k, v) in paare.iter() {
        if matches!(k.as_str(), "phase" | "bauen" | "forschen" | "fahrzeuge" | "grundversorgung" | "reihenfolge" | "drehbuch" | "angriff" | "ziel" | "art"
                    | "parallel" | "emp" | "einheiten" | "rueckzug" | "frist" | "staerke" | "verteidigung" | "wache" | "bereich" | "turm" | "aussenposten") { continue; }
        let Ok(w) = v.parse::<f64>() else { melde(welt, z, "fehler", mapNone, z.bx, z.by, &format!("Befehl: '{}={}' ist keine Zahl", k, v)); continue };
        match k.as_str() {
            "rolle" => super::modus::setzeRolle(welt, z, w as i32),
            "stufe" => super::modus::setzeStufe(welt, z, w as i32),
            "eigeneAngriffe" => super::modus::setzeAngriffe(welt, z, w >= 0.5),
            "missionsModus" => super::modus::setzeMission(welt, z, w >= 0.5),
            "patrouille" => { super::modus::parameterAendern(welt, z, "wachePatrouille", w); }
            _ => {
                if super::modus::parameterAendern(welt, z, k, w) { melde(welt, z, "befehl", mapNone, z.bx, z.by, &format!("Befehl: {} = {}", k, w)); }
                else { melde(welt, z, "fehler", mapNone, z.bx, z.by, &format!("Befehl: unbekannte Stellschraube '{}'", k)); }
            }
        }
    }
}

/// "Tokamak:2,GuardPost:3@150,140,8,Agridome" -- ein "@" nimmt die folgenden Zahlen (x, y, Umkreis) mit
fn bauenNeuLesen(welt: &mut dyn Welt, z: &Zustand, s: &str) -> Vec<BauPosten> {
    let mut v: Vec<BauPosten> = Vec::new();
    for teil in s.split(',') {
        let teil = teil.trim();
        if teil.is_empty() { continue; }
        // eine reine Zahl gehoert zum Ort des vorigen Postens
        if let Ok(n) = teil.parse::<i32>() {
            if let Some(b) = v.last_mut() { if let Some(o) = b.ort.as_mut() { if o.1 == i32::MIN { o.1 = n; } else if o.2 == -1 { o.2 = n; } } }
            continue;
        }
        let (vorn, ort) = match teil.split_once('@') { Some((a, b)) => (a, Some(b)), None => (teil, None) };
        let (was, n) = match vorn.split_once(':') { Some((a, b)) => (a, b.trim().parse().unwrap_or(1)), None => (vorn, 1) };
        let Some(typ) = typNach(welt, was) else { let _ = z; continue };
        let ort = ort.and_then(|o| o.trim().parse::<i32>().ok()).map(|x| (x, i32::MIN, -1));
        v.push(BauPosten { typ, n, ort });
    }
    for b in v.iter_mut() {
        if let Some(o) = b.ort { b.ort = if o.1 == i32::MIN { None } else { Some((o.0, o.1, if o.2 < 0 { 6 } else { o.2 })) }; }
    }
    v
}

/// Ziel eines Angriffs: basis (naechste Feindbasis), basis:P, punkt:x,y, schwaechste (das am schwaechsten bewachte Feindgebaeude)
fn zielOrt(welt: &mut dyn Welt, z: &mut Zustand, ziel: &str) -> Option<(i32, i32)> {
    let (art, rest) = ziel.split_once(':').unwrap_or((ziel, ""));
    match art.trim() {
        "punkt" | "ort" => { let v = zahlen(rest); if v.len() >= 2 { Some((v[0], v[1])) } else { None } }
        "basis" if !rest.trim().is_empty() => {
            let p: i32 = rest.trim().parse().unwrap_or(-1);
            if p < 0 || p >= welt.NoPlayers() { return None; }
            let cc = welt.PlayerBuildingEnum(p, mapCommandCenter).into_iter().find(|&u| welt.IsLive(u))?;
            let lo = welt.Location(cc); Some((edX(welt, z, lo), edY(welt, z, lo)))
        }
        "schwaechste" => {
            let mut best: Option<(i32, (i32, i32))> = None;
            for p in 0..welt.NoPlayers() {
                if p == z.spieler || welt.IsAlliedTo(z.spieler, p) { continue; }
                for &ty in GEBAEUDE.iter() {
                    for u in welt.PlayerBuildingEnum(p, ty) {
                        if !welt.IsLive(u) { continue; }
                        let lo = welt.Location(u); let (x, y) = (edX(welt, z, lo), edY(welt, z, lo));
                        let tuerme = welt.PlayerBuildingEnum(p, mapGuardPost).into_iter().filter(|&g| { let gl = welt.Location(g);
                            (edX(welt, z, gl) - x).pow(2) + (edY(welt, z, gl) - y).pow(2) <= 100 }).count() as i32 * WERT_WT;
                        let s = feindStaerkeBei(welt, z, x, y, 10) + tuerme;
                        if best.is_none_or(|b| s < b.0) { best = Some((s, (x, y))); }
                    }
                }
            }
            best.map(|b| b.1)
        }
        _ => None,
    }
}

// ---------------------------------------------------------------------------------------------- Drehbuch

/// Laeuft ein Drehbuch? Dann baut, forscht und ruestet die KI nur nach Befehl.
pub fn drehbuch(z: &Zustand) -> bool { mit(z, |s| s.phase.is_some()) }

/// Mit Grundversorgung darf sie Notfaelle (Strom, Nahrung, Zentrale, Fabrik, Schmelze) und Wohnraum selbst bauen
pub fn grundversorgung(z: &Zustand) -> bool { mit(z, |s| s.phase.as_ref().is_some_and(|p| p.grund)) }

fn vorhanden(welt: &mut dyn Welt, z: &Zustand, l: &Lage, b: &BauPosten, nurFertig: bool) -> i32 {
    match b.ort {
        Some((x, y, r)) => welt.PlayerBuildingEnum(z.spieler, b.typ).into_iter().filter(|&u| {
            let lo = welt.Location(u); (edX(welt, z, lo) - x).pow(2) + (edY(welt, z, lo) - y).pow(2) <= (r + 2) * (r + 2)
                && (!nurFertig || welt.GetCurAction(u) != moObjWait) }).count() as i32
            + if nurFertig { 0 } else { (0..z.anzahlPlaetze.max(0) as usize).filter(|&i| { let q = z.plaetze[i];
                q.typ as i32 == b.typ && (q.zustand == P_FAHRT || q.zustand == P_BAUT) && (q.x0 as i32 - x).pow(2) + (q.y0 as i32 - y).pow(2) <= (r + 4) * (r + 4) }).count() as i32 },
        None => if nurFertig { l.geb[(b.typ & 127) as usize] - l.imBau[(b.typ & 127) as usize] } else { hatGeb(welt, z, l, b.typ) },
    }
}

/// Bauwahl im Drehbuch: der naechste Posten der Phase, der noch fehlt (der Reihe nach oder der erste baubare).
/// None = kein Drehbuch; Some(Wahl) sonst (typ mapNone = nichts zu bauen).
pub fn bauWahl(welt: &mut dyn Welt, z: &Zustand, l: &Lage) -> Option<Wahl> {
    let ph = mit(z, |s| s.phase.clone())?;
    let mut w = Wahl { typ: mapNone, bereich: -1, warum: String::new() };
    // Befohlene Fahrzeuge (Phase, Angriff, Wache) brauchen eine Fahrzeugfabrik; die Startbasis hat nicht immer eine
    // (Original cm03, Startstufe 3: Phase 1 wartete 120 000 Ticks auf "Lynx 0/4")
    let brauchtFz = !ph.fahrzeuge.is_empty()
        || mit(z, |s| s.angriff.is_some() || s.verteidigung.as_ref().is_some_and(|v| v.truppe != Truppe::Auto));
    let vf = BauPosten { typ: mapVehicleFactory, n: 1, ort: None };
    if brauchtFz && vorhanden(welt, z, l, &vf, false) < 1 {
        mit(z, |s| s.ortWunsch = None);
        if waehle(welt, z, l, mapVehicleFactory, B_KOLONIE, true, &mut w, &format!("Drehbuch Phase {}: Fahrzeugfabrik fuer die befohlenen Fahrzeuge", ph.id)) { return Some(w); }
    }
    for b in ph.bauen.iter() {
        if b.typ == mapCommonOreMine || b.typ == mapRareOreMine { continue; }   // Minen baut der Robo-Miner (Erzlogik)
        if vorhanden(welt, z, l, b, false) >= b.n { continue; }
        let nm = name(welt, b.typ);
        mit(z, |s| s.ortWunsch = b.ort.map(|o| (b.typ, o.0, o.1, o.2)));
        if waehle(welt, z, l, b.typ, B_KOLONIE, true, &mut w, &format!("Drehbuch Phase {}: {} ({} gewuenscht)", ph.id, nm, b.n)) { return Some(w); }
        if ph.reihe { break; }
    }
    mit(z, |s| s.ortWunsch = None);
    if w.warum.is_empty() { w.warum = format!("Drehbuch Phase {}: nichts zu bauen", ph.id); }
    w.typ = mapNone;
    Some(w)
}

/// Platzwunsch des Drehbuchs fuer diesen Typ: freier Platz naechst (x, y) im Umkreis
pub fn platzWunsch(welt: &mut dyn Welt, z: &mut Zustand, typ: i32) -> Option<i32> {
    let (t, x, y, r) = mit(z, |s| s.ortWunsch)?;
    if t != typ { return None; }
    let g = grundflaeche(welt, typ);
    let mut hin = [Rechteck::default(); 160];
    let nh = hindernisse(welt, z, &mut hin, 160);
    let mut best: Option<(i32, i32, i32)> = None;
    for dy in -r..=r { for dx in -r..=r {
        let d2 = dx * dx + dy * dy;
        if d2 > r * r || best.is_some_and(|b| d2 >= b.0) { continue; }
        let (x0, y0) = (x + dx - g.w / 2, y + dy - g.h / 2);
        let mut gr: &'static str = "";
        if platzFrei(welt, z, x0 - 1, y0 - 1, g.w + 2, g.h + 2, &hin, nh, &mut gr) { best = Some((d2, x0, y0)); }
    } }
    let Some((_, x0, y0)) = best else {
        // am befohlenen Ort ist kein Platz: nicht woanders bauen (die Phase zaehlt nur Gebaeude dort)
        if welt.Tick() % 2000 < TAKT { let nm = name(welt, typ); melde(welt, z, "warte", typ, x, y, &format!("Drehbuch: kein freier Platz fuer {} bei {},{} (Umkreis {})", nm, x, y, r)); }
        return Some(-1);
    };
    let i = platzEintrag(welt, z, typ, x0, y0);
    if i >= 0 { melde(welt, z, "plan", typ, x0, y0, &format!("Drehbuch: Platz bei {},{} wie befohlen ({},{} Umkreis {})", x0, y0, x, y, r)); }
    Some(i)
}

/// Forschung im Drehbuch: nur ihre Liste (None = kein Drehbuch). Mit Grundversorgung zuerst, was Kindergarten,
/// Universitaet und Medizinzentrum brauchen -- ohne sie gibt es keine Geburten (Sim cm03: die Kolonie starb aus)
pub fn forschung(welt: &mut dyn Welt, z: &Zustand) -> Option<Vec<i32>> {
    let (mut liste, grund) = mit(z, |s| s.phase.as_ref().map(|p| (p.forschen.clone(), p.grund)))?;
    // Was die Befehle bauen lassen, muss erforscht sein (Original cm03: Lynx braucht 3851, der Angriff "beste:6" bekam
    // ohne erforschtes Fahrgestell kein Soll): Gebaeude, Fahrgestelle und Waffen der Phase, des Angriffs und der Wache
    let mut typen: Vec<i32> = Vec::new();
    mit(z, |s| {
        if let Some(p) = &s.phase { typen.extend(p.bauen.iter().map(|b| b.typ)); for e in &p.fahrzeuge { typen.push(e.typ); typen.push(e.waffe); } }
        for t in [s.angriff.as_ref().map(|a| &a.truppe), s.verteidigung.as_ref().map(|v| &v.truppe)].into_iter().flatten() {
            match t { Truppe::Liste(v) => for e in v { typen.push(e.typ); typen.push(e.waffe); }, Truppe::Beste(_) => typen.push(mapLynx), _ => {} }
        }
    });
    let mut noetig: Vec<i32> = Vec::new();
    for t in typen { if t == mapNone { continue; } let id = match techFuer(welt, t) { 0 => super::tb::einheit(t).map(|e| e.tech).unwrap_or(0), a => a }; if id > 0 && !hatTechId(welt, z, id) && !noetig.contains(&id) { noetig.push(id); } }
    noetig.retain(|id| !liste.contains(id));
    noetig.extend(liste); liste = noetig;
    if grund {
        let mut vorn: Vec<i32> = Vec::new();
        for t in [mapNursery, mapUniversity, mapMedicalCenter] { let id = techFuer(welt, t); if id > 0 && !hatTechId(welt, z, id) { vorn.push(id); } }
        vorn.retain(|id| !liste.contains(id));
        vorn.extend(liste); liste = vorn;
    }
    Some(liste)
}

fn fzZahl(welt: &mut dyn Welt, z: &Zustand, e: &Einheit) -> i32 {
    welt.PlayerVehicleEnum(z.spieler).into_iter().filter(|&u| welt.IsLive(u) && welt.GetType(u) == e.typ
        && (e.waffe == mapNone || welt.GetWeapon(u) == e.waffe)).count() as i32
}

/// Je Takt: Fahrzeuge der Phase bauen, fertige Phase melden, Angriffsstand fuehren
pub fn schritt(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage) {
    let t = welt.Tick();
    if t % 200 >= TAKT { return; }
    // Angriffsstand
    let (stand, a) = mit(z, |s| (s.angriffStand, s.angriff.clone()));
    if stand == 1 && z.kampfModus >= 2 { mit(z, |s| s.angriffStand = 2); }
    if stand == 2 && z.kampfModus < 2 {
        let gewonnen = z.angriffWunsch == 0 && (z.feindGeb == 0 || a.as_ref().is_some_and(|a| a.art == "ziel" || a.art == "ueberfall"));
        mit(z, |s| { s.angriffStand = if gewonnen || s.zuletzt == 3 { 3 } else { 4 }; s.angriffeFertig += 1; s.angriff = None; });
    }
    let Some(ph) = mit(z, |s| s.phase.clone()) else { return };
    // Fahrzeuge der Phase
    if t - mit(z, |s| s.fzSeit) > 600 {
        for e in ph.fahrzeuge.iter() {
            if fzZahl(welt, z, e) >= e.n { continue; }
            let k = welt.GetOreCost(e.typ, z.spieler) + if e.waffe != mapNone { welt.GetOreCost(e.waffe, z.spieler) } else { 0 };
            if l.erz < k { break; }
            let vf = l.vfs.iter().copied().take(l.vfAnzahl.max(0) as usize).find(|&v| v != 0 && !welt.IsBusy(v) && welt.HasPower(v) && welt.HasWorkers(v) && welt.GetCurAction(v) == moDone);
            let Some(vf) = vf else { break };
            welt.DoProduce(vf, e.typ, e.waffe);
            mit(z, |s| s.fzSeit = t);
            let nm = name(welt, e.typ);
            melde(welt, z, "befehl", e.typ, z.bx, z.by, &format!("Drehbuch Phase {}: {} in Auftrag", ph.id, nm));
            break;
        }
    }
    // fertig?
    if mit(z, |s| s.fertig.contains(&ph.id)) { return; }
    let bauten = ph.bauen.iter().all(|b| b.typ == mapCommonOreMine || b.typ == mapRareOreMine || vorhanden(welt, z, l, b, true) >= b.n);
    let forsch = ph.forschen.iter().all(|&id| hatTechId(welt, z, id));
    let fz = ph.fahrzeuge.iter().all(|e| fzZahl(welt, z, e) >= e.n);
    if !(bauten && forsch && fz) && t % 4000 < TAKT {
        let mut fehlt: Vec<String> = Vec::new();
        for b in ph.bauen.iter() {
            if b.typ == mapCommonOreMine || b.typ == mapRareOreMine { continue; }
            let n = vorhanden(welt, z, l, b, true);
            if n < b.n { let nm = name(welt, b.typ); fehlt.push(format!("{} {}/{}", nm, n, b.n)); }
        }
        for &id in ph.forschen.iter() { if !hatTechId(welt, z, id) { fehlt.push(format!("Tech {}", id)); } }
        for e in ph.fahrzeuge.iter() { let n = fzZahl(welt, z, e); if n < e.n { let nm = name(welt, e.typ); fehlt.push(format!("{} {}/{}", nm, n, e.n)); } }
        melde(welt, z, "befehl", mapNone, z.bx, z.by, &format!("Drehbuch Phase {} wartet auf: {}", ph.id, fehlt.join(", ")));
    }
    if bauten && forsch && fz {
        mit(z, |s| { s.fertig.push(ph.id); s.zuletzt = ph.id; });
        melde(welt, z, "befehl", mapNone, z.bx, z.by, &format!("Drehbuch: Phase {} fertig nach {} Ticks", ph.id, t - ph.seit));
    }
}

/// Fehlen noch Fahrzeuge der laufenden Phase? Dann darf die Fahrzeugfabrik nicht stillgelegt werden (Original cm03:
/// "Fahrzeugfabrik stillgelegt -- keine neuen Fahrzeuge noetig", die befohlenen Lynx kamen nie)
pub fn fahrzeugeOffen(welt: &mut dyn Welt, z: &Zustand) -> bool {
    let Some(ph) = mit(z, |s| s.phase.clone()) else { return false };
    ph.fahrzeuge.iter().any(|e| fzZahl(welt, z, e) < e.n)
}

// ---------------------------------------------------------------------------------------------- Armee

/// Sollzahlen nach Befehl: (Wache, Sammelgruppe) als Listen; None = die KI bestimmt selbst
pub fn armee(welt: &mut dyn Welt, z: &Zustand, fz: i32, waffe: i32, wacheZiel: i32, sammelZiel: i32) -> Option<(Vec<Einheit>, Vec<Einheit>)> {
    let (dreh, vert, ang) = mit(z, |s| (s.phase.is_some(), s.verteidigung.clone(), s.angriff.clone()));
    let auftrag = super::modus::noetig(z) > 0 || ang.is_some();
    if !dreh && vert.is_none() && ang.as_ref().is_none_or(|a| a.truppe == Truppe::Auto || a.truppe == Truppe::Vorhanden) { return None; }
    let lynxWaffe = if fz == 0 { eigeneWaffe(welt, z) } else { waffe };
    let liste = |t: &Truppe, auto: i32| -> Vec<Einheit> {
        match t {
            Truppe::Liste(v) => v.clone(),
            // ohne erforschtes Fahrgestell: Lynx mit der eigenen Waffe (DoProduce prueft die Forschung nicht)
            Truppe::Beste(n) => if fz != 0 { vec![Einheit { typ: fz, waffe, n: *n }] } else { vec![Einheit { typ: mapLynx, waffe: lynxWaffe, n: *n }] },
            Truppe::Vorhanden => Vec::new(),
            Truppe::Auto => if fz != 0 && auto > 0 { vec![Einheit { typ: fz, waffe, n: auto }] } else { Vec::new() },
        }
    };
    let wache = match &vert { Some(v) => liste(&v.truppe, wacheZiel), None => if dreh { Vec::new() } else { liste(&Truppe::Auto, wacheZiel) } };
    let sammel = match &ang { Some(a) => liste(&a.truppe, sammelZiel), None => if dreh && !auftrag { Vec::new() } else { liste(&Truppe::Auto, sammelZiel) } };
    Some((wache, sammel))
}

/// Listen an die Gruppen geben, wenn sie sich geaendert haben
pub fn listenSetzen(welt: &mut dyn Welt, z: &mut Zustand, wache: &[Einheit], sammel: &[Einheit]) {
    let neu = (wache.to_vec(), sammel.to_vec());
    if mit(z, |s| s.listen == neu) && z.wacheZiel == -3 { return; }
    welt.ClearTargCount(z.wache); welt.ClearTargCount(z.sammel);
    for e in wache { welt.SetTargCount(z.wache, e.typ, e.waffe, e.n); }
    for e in sammel { welt.SetTargCount(z.sammel, e.typ, e.waffe, e.n); }
    let txt = |welt: &mut dyn Welt, v: &[Einheit]| -> String {
        if v.is_empty() { return "keine".into(); }
        v.iter().map(|e| format!("{} {}{}", e.n, name(welt, e.typ), if e.waffe != mapNone { format!("/{}", name(welt, e.waffe)) } else { String::new() })).collect::<Vec<_>>().join(", ")
    };
    let (tw, ts) = (txt(welt, wache), txt(welt, sammel));
    melde(welt, z, "befehl", mapLynx, z.bx, z.by, &format!("Sollzahlen nach Befehl: Wache {}; Sammelgruppe {}", tw, ts));
    mit(z, |s| s.listen = neu);
    z.wacheZiel = -3;   // zurueck zur eigenen Wahl setzt die Sollzahlen neu
}

/// Angriffsauftrag mit Befehl: ist die Truppe bereit? None = ohne Befehl (eigene Regel)
pub fn bereit(welt: &mut dyn Welt, z: &Zustand, t: i32, seit: i32) -> Option<bool> {
    let a = mit(z, |s| s.angriff.clone())?;
    if t - seit > a.frist { return Some(true); }
    Some(match &a.truppe {
        Truppe::Auto => return None,
        Truppe::Vorhanden => true,
        Truppe::Beste(n) => { let mut ns = 0; super::kampf::gruppenWert(welt, z.sammel, &mut ns); ns >= *n }
        Truppe::Liste(v) => v.iter().all(|e| welt.GroupEnumerator(z.sammel).into_iter().filter(|&u| welt.IsLive(u) && welt.GetType(u) == e.typ
            && (e.waffe == mapNone || welt.GetWeapon(u) == e.waffe)).count() as i32 >= e.n),
    })
}

/// Angriffsbefehl: nur die Sammelgruppe zieht los (Liste/beste); "vorhanden" nimmt auch die Wache mit
pub fn nurSammel(z: &Zustand) -> bool { mit(z, |s| s.angriff.as_ref().is_some_and(|a| matches!(a.truppe, Truppe::Liste(_) | Truppe::Beste(_)))) }
pub fn alleMit(z: &Zustand) -> bool { mit(z, |s| s.angriff.as_ref().is_some_and(|a| a.truppe == Truppe::Vorhanden)) }
/// Zwei Wege per Befehl
pub fn parallel(z: &Zustand) -> bool { mit(z, |s| s.angriff.as_ref().is_some_and(|a| a.parallel)) }
/// EMP per Befehl: Some(true) bauen und nutzen, Some(false) nicht
pub fn emp(z: &Zustand) -> Option<bool> { mit(z, |s| s.angriff.as_ref().and_then(|a| a.emp)) }
/// Zielangriff (ziel/ueberfall/halten) statt der ganzen Basis
pub fn zielArt(z: &Zustand) -> bool { mit(z, |s| s.angriff.as_ref().is_some_and(|a| matches!(a.art.as_str(), "ziel" | "ueberfall" | "halten"))) }

/// Angriff gestartet: Haltepunkt merken
pub fn losgeschlagen(z: &Zustand, ort: Option<(i32, i32)>) {
    mit(z, |s| { if let Some(a) = s.angriff.as_mut() { if a.art == "halten" { a.halten = ort; } } s.angriffStand = 2; s.angriffMax = 0; });
}

/// Angriffsbefehl ohne Ziel (kein Feindgebaeude bekannt): als abgebrochen melden
pub fn keinZiel(welt: &mut dyn Welt, z: &Zustand) {
    if !mit(z, |s| s.angriff.is_some()) { return; }
    mit(z, |s| { s.angriffStand = 4; s.angriffeFertig += 1; s.angriff = None; });
    melde(welt, z, "befehl", mapNone, z.bx, z.by, "Befehl: Angriff abgebrochen -- kein feindliches Gebaeude bekannt");
}

/// Angriffsbefehl "halten" (noch nicht losgeschlagen): Zielpunkt
pub fn istHalten(z: &Zustand) -> bool { mit(z, |s| s.angriff.as_ref().is_some_and(|a| a.art == "halten" && a.halten.is_none())) }

/// Haltepunkt eines laufenden Halte-Befehls
pub fn halten(z: &Zustand) -> Option<(i32, i32)> { mit(z, |s| s.angriff.as_ref().and_then(|a| a.halten)) }

/// Abbruch nach Befehl: Rueckzug ab Verlustanteil, Ueberfall/Ziel: Ziel zerstoert -> heim
pub fn abbruch(welt: &mut dyn Welt, z: &Zustand, jetzt: i32) -> Option<&'static str> {
    let a = mit(z, |s| s.angriff.clone())?;
    let max = mit(z, |s| { s.angriffMax = s.angriffMax.max(jetzt); s.angriffMax });
    if a.rueckzug > 0 && jetzt > 0 && max > 0 && (jetzt as i64) * 100 < (100 - a.rueckzug) as i64 * max as i64 {
        mit(z, |s| s.zuletzt = 4); return Some("Befehl: Verlustgrenze erreicht -- Rueckzug");
    }
    if (a.art == "ueberfall" || a.art == "ziel") && z.angriffWunsch != 0 {
        let u = z.angriffWunsch;
        if !welt.IsLive(u) || welt.OwnerID(u) == z.spieler { mit(z, |s| s.zuletzt = 3); return Some("Befehl: Ziel zerstoert -- heim"); }
    }
    None
}

// ---------------------------------------------------------------------------------------------- Verteidigung

/// Wachgebiet nach Befehl (x0, y0, x1, y1)
pub fn wacheBereich(z: &Zustand) -> Option<(i32, i32, i32, i32)> { mit(z, |s| s.verteidigung.as_ref().and_then(|v| v.bereich)) }
/// Warteplatz der Wache nach Befehl (Mitte des Bereichs)
pub fn wachePunkt(z: &Zustand) -> Option<(i32, i32)> { mit(z, |s| s.verteidigung.as_ref().and_then(|v| v.bereich).map(|b| ((b.0 + b.2) / 2, (b.1 + b.3) / 2))) }
/// Waffe der Tuerme nach Befehl (0 = selbst)
pub fn turmWaffe(z: &Zustand) -> i32 { mit(z, |s| s.verteidigung.as_ref().map(|v| v.turm).unwrap_or(0)) }

// ---------------------------------------------------------------------------------------------- Rueckmeldungen

/// Rueckmeldung an die Mission. Codes: 1 zuletzt fertige Phase, 2 laufende Phase (0 keine), 3 Angriff (0 keiner, 1 Truppe
/// wird gebaut, 2 unterwegs, 3 zuletzt gewonnen, 4 zuletzt abgebrochen), 4 beendete befohlene Angriffe, 5 Tick des letzten
/// Angriffs auf die Basis, 6 Satelliten im All, 7 Gebaeude, 8 Kampffahrzeuge, 9 Kampfwert der Armee, 10 zweite Basen
/// selbststaendig; 100+id: Phase id fertig (1/0)
pub fn status(welt: &mut dyn Welt, z: &Zustand, code: i32) -> i32 {
    if code > 100 { return mit(z, |s| s.fertig.contains(&(code - 100))) as i32; }
    match code {
        1 => mit(z, |s| s.zuletzt),
        2 => mit(z, |s| s.phase.as_ref().map(|p| p.id).unwrap_or(0)),
        3 => mit(z, |s| s.angriffStand),
        4 => mit(z, |s| s.angriffeFertig),
        5 => z.letzterAngriff,
        6 => welt.GetSatelliteCount(z.spieler, mapEDWARDSatellite) + welt.GetSatelliteCount(z.spieler, mapSolarSatellite),
        7 => GEBAEUDE.iter().map(|&t| welt.PlayerBuildingEnum(z.spieler, t).len() as i32).sum(),
        8 => welt.PlayerVehicleEnum(z.spieler).into_iter().filter(|&u| welt.IsLive(u) && kaempfer(welt, u)).count() as i32,
        9 => z.eigeneArmee,
        10 => (0..super::zweitbasis::MAX_ZB).filter(|&k| { let s = super::zweitbasis::zb(z.spieler, k); s.groesse == 4 && s.phase >= 5 }).count() as i32,
        _ => 0,
    }
}
