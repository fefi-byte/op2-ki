//! Rust (Nutzer 2026-11-18, Truppnetz): ein gelerntes Netz sagt fuer einen Angriffsplan (Ziel, Rueckzugsschwelle, danach
//! weiter oder heim) vorher, was er beim Gegner kaputt macht und was er selbst kostet -- beides je Startwert des Trupps.
//! Gelernt in der Truppen-Arena des Simulators (sim/src/trupp_arena.rs: jede Uebungslage mit allen Plaenen durchgespielt).
//! Die Merkmale hier muessen genau denen der Arena (plan_merkmale) entsprechen.
//!  - Angriff (PAR truppPlan): Zermuerben waehlt Ziel und Plan nach Schaden - truppRisiko x Verlust.
//!  - Abwehr (PAR truppAbwehr): dasselbe Netz aus Sicht des Gegners auf die eigenen Aussenposten -- wo lohnt sich ein
//!    Ueberfall fuer ihn? Dort so viele Wachen hin, bis es sich nicht mehr lohnt als gegen die Basis.
#![allow(non_snake_case)]
use std::cell::RefCell;

use super::api::*;
use super::hilfen::*;
use super::welt::{Id, Welt};
use super::zustand::Zustand;
use super::ablauf::wissenPfad;
use super::routen::RoutenInfo;

pub const D: usize = 26;
const PH: usize = 32;
/// wie die Arena: Mineneinkommen ueber den Horizont als Zielwert
const MINE_EINKOMMEN: i32 = 12600;

thread_local! {
    static NETZ: RefCell<Option<Vec<f32>>> = const { RefCell::new(None) };
    static GELADEN: RefCell<bool> = const { RefCell::new(false) };
}

fn n_gew() -> usize { PH * D + PH + 2 * PH + 2 }

pub fn aus_text(t: &str) -> Option<Vec<f32>> {
    let g: Vec<f32> = t.split_whitespace().filter_map(|s| s.parse().ok()).collect();
    if g.len() == n_gew() { Some(g) } else { None }
}
/// Fuer Simulator/Arena: Netz direkt setzen
pub fn setze(n: Option<Vec<f32>>) { NETZ.with(|x| *x.borrow_mut() = n); GELADEN.with(|g| *g.borrow_mut() = true); }
pub fn zuruecksetzen() { NETZ.with(|x| *x.borrow_mut() = None); GELADEN.with(|g| *g.borrow_mut() = false); }

/// Netz laden: Umgebungsvariable OP2KI_TRUPP_NETZ (Datei), sonst KI-Trupp-Netz.txt im Wissensordner
pub fn geladen(welt: &mut dyn Welt) -> bool {
    if !GELADEN.with(|g| *g.borrow()) {
        GELADEN.with(|g| *g.borrow_mut() = true);
        let mut n = std::env::var("OP2KI_TRUPP_NETZ").ok().and_then(|p| std::fs::read_to_string(p).ok()).and_then(|t| aus_text(&t));
        if n.is_none() {
            let mut pfad = String::new();
            wissenPfad(welt, &mut pfad, 600, "KI-Trupp-Netz.txt");
            n = welt.dateiLesen(&pfad).and_then(|t| aus_text(&t));
        }
        NETZ.with(|x| *x.borrow_mut() = n);
    }
    NETZ.with(|x| x.borrow().is_some())
}

/// (Schaden beim Gegner, eigener Verlust) je Startwert des Trupps
pub fn vorhersage(x: &[f32; D]) -> Option<[f32; 2]> {
    NETZ.with(|nz| {
        let b = nz.borrow(); let g = b.as_ref()?;
        let o = PH * D + PH; let n = n_gew();
        let mut y = [g[n - 2], g[n - 1]];
        for h in 0..PH {
            let mut a = g[PH * D + h]; for i in 0..D { a += g[h * D + i] * x[i]; }
            let t = a.tanh(); y[0] += g[o + h] * t; y[1] += g[o + PH + h] * t;
        }
        Some(y)
    })
}

/// Ein Fahrzeug, wie die Arena es zaehlt: Erzkosten (Fahrgestell + Waffe) x verbleibende Trefferpunkte
pub fn fzWert(welt: &mut dyn Welt, u: Id) -> i32 {
    let p = welt.OwnerID(u); let t = welt.GetType(u);
    let k = welt.GetOreCost(t, p) + welt.GetOreCost(welt.GetWeapon(u), p);
    let hp = welt.GetHitPoints(t, p).max(1);
    k * (hp - welt.GetDamage(u)).max(0) / hp
}

/// Platz im Zieltyp-Kodierung der Arena (Zentrale, Strukturfabrik, Kraftwerk, Schmelze, Wohnhaus, Mine, Lastwagen)
pub fn zielKlasse(t: i32) -> Option<usize> {
    if t == mapCommandCenter { Some(0) }
    else if t == mapStructureFactory || t == mapVehicleFactory || t == mapArachnidFactory { Some(1) }
    else if t == mapTokamak || t == mapMHDGenerator || t == mapSolarPowerArray || t == mapGeothermalPlant { Some(2) }
    else if t == mapCommonOreSmelter || t == mapRareOreSmelter { Some(3) }
    else if t == mapResidence || t == mapAdvancedResidence || t == mapReinforcedResidence { Some(4) }
    else if t == mapCommonOreMine || t == mapRareOreMine || t == mapMagmaWell { Some(5) }
    else if t == mapCargoTruck { Some(6) }
    else { None }
}

/// Zielwert wie die Arena (ziel_wert): Erz + 2 x Seltenerz (mind. 100), Minen zusaetzlich ihr Einkommen, Lastwagen ihre Kosten
pub fn zielWert(welt: &mut dyn Welt, t: i32, p: i32) -> i32 {
    if t == mapCargoTruck { return welt.GetOreCost(t, p); }
    let b = (welt.GetOreCost(t, p) + 2 * welt.GetRareOreCost(t, p)).max(100);
    if zielKlasse(t) == Some(5) { b + MINE_EINKOMMEN } else { b }
}

/// Eine Seite der Lage in Kacheln: Fahrzeuge (x, y, Wert)
pub struct Seite { pub fz: Vec<(i32, i32, i32)>, pub wert: i32, pub mx: i32, pub my: i32, pub gebAnteil: f32 }

pub fn seite(fz: Vec<(i32, i32, i32)>, gebAnteil: f32, ersatz: (i32, i32)) -> Seite {
    let wert: i32 = fz.iter().map(|e| e.2).sum();
    let n = fz.len() as i32;
    let (mx, my) = if n > 0 { (fz.iter().map(|e| e.0).sum::<i32>() / n, fz.iter().map(|e| e.1).sum::<i32>() / n) } else { ersatz };
    Seite { fz, wert, mx, my, gebAnteil }
}

fn ab(a: (i32, i32), b: (i32, i32)) -> f64 { (((a.0 - b.0).pow(2) + (a.1 - b.1).pow(2)) as f64).sqrt() }
fn strecke(a: (i32, i32), b: (i32, i32), q: (i32, i32)) -> f64 {
    let (dx, dy) = ((b.0 - a.0) as f64, (b.1 - a.1) as f64); let l2 = (dx * dx + dy * dy).max(1e-9);
    let t = (((q.0 - a.0) as f64 * dx + (q.1 - a.1) as f64 * dy) / l2).clamp(0.0, 1.0);
    ((a.0 as f64 + t * dx - q.0 as f64).powi(2) + (a.1 as f64 + t * dy - q.1 as f64).powi(2)).sqrt()
}

/// Merkmale eines Plans -- dieselbe Funktion rechnet in der Arena und im Spiel. ang = Trupp, ver = Verteidiger-Armee,
/// tuerme = Verteidiger-Tuerme (x, y, Wert; in 7 Kacheln des Ziels Abwehr am Ziel). ziel None = kein Angriff.
/// route: Merkmale der gewaehlten Route (None: kein Weg gefunden, Luftlinie).
pub fn merkmale(ang: &Seite, ver: &Seite, tuerme: &[(i32, i32, i32)], ziel: Option<((i32, i32), usize, i32)>, route: Option<&RoutenInfo>,
                basis: (i32, i32), rueck: f32, weiter: bool) -> [f32; D] {
    let eigen = ang.wert.max(1) as f64;
    let m = (ang.mx, ang.my);
    let mut f = [0f32; D];
    f[0] = 1.0; f[1] = (eigen / 3000.0).min(4.0) as f32; f[2] = ang.gebAnteil;
    f[3] = (ver.wert as f64 / eigen).min(4.0) as f32;
    let Some((zp, klasse, wert)) = ziel else { f[21] = 1.0; return f; };
    let mut abw: f64 = ver.fz.iter().map(|&(x, y, w)| w as f64 * (1.0 - (ab((x, y), zp) - 10.0) / 30.0).clamp(0.0, 1.0)).sum();
    abw += tuerme.iter().filter(|t| ab((t.0, t.1), zp) <= 7.0).map(|t| t.2 as f64).sum::<f64>();
    let vm = if ver.fz.is_empty() { None } else { Some((ver.mx, ver.my)) };
    f[4] = (abw / eigen).min(4.0) as f32;
    f[6] = vm.map_or(1.5, |v| (ab(v, zp) / 50.0).min(1.5)) as f32;
    f[7] = (wert as f64 / 5000.0) as f32;
    f[8 + klasse] = 1.0;
    f[16] = rueck; f[17] = weiter as i32 as f32;
    f[18] = (ab(zp, basis) / 50.0).min(3.0) as f32;
    match route {
        Some(r) => {
            f[5] = (r.laenge / 50.0).min(4.0); f[15] = r.umweg.min(1.0); f[19] = r.anzahl; f[20] = r.minFeind;
            f[22] = r.gefahrArmee; f[23] = r.gefahrTurm; f[24] = r.breite;
        }
        None => {
            f[5] = (ab(m, zp) / 50.0).min(4.0) as f32; f[20] = vm.map_or(1.5, |v| (strecke(m, zp, v) / 50.0).min(1.5)) as f32;
            f[24] = 1.0; f[25] = 1.0;
        }
    }
    f
}

/// Kampffahrzeuge eines Spielers in Kacheln (Lastwagen, Bau- und Erkundungsfahrzeuge zaehlen nicht)
pub fn kampfFz(welt: &mut dyn Welt, z: &Zustand, p: i32) -> Vec<(i32, i32, i32)> {
    let mut v = Vec::new();
    for u in welt.PlayerVehicleEnum(p) {
        if !welt.IsLive(u) || kampfWert(welt, u) <= 0 { continue; }
        let lo = welt.Location(u); let w = fzWert(welt, u);
        v.push((edX(welt, z, lo), edY(welt, z, lo), w));
    }
    v
}

/// Anteil, der Gebaeuden ernsthaft schadet (Mikrowelle, EMP, Klebschaum kaum)
pub fn gebAnteil(welt: &mut dyn Welt, us: &[Id]) -> f32 {
    if us.is_empty() { return 0.0; }
    let n = us.iter().filter(|&&u| { let w = welt.GetWeapon(u); w != mapMicrowave && w != mapEMP && w != mapStickyfoam }).count();
    n as f32 / us.len() as f32
}

pub fn tuermeVon(welt: &mut dyn Welt, z: &Zustand, p: i32) -> Vec<(i32, i32, i32)> {
    let mut v = Vec::new();
    for g in welt.PlayerBuildingEnum(p, mapGuardPost) {
        if !welt.IsLive(g) { continue; }
        let lo = welt.Location(g); let t = welt.GetWeapon(g);
        let w = welt.GetOreCost(mapGuardPost, p) + welt.GetOreCost(t, p);
        v.push((edX(welt, z, lo), edY(welt, z, lo), w));
    }
    v
}

pub const RUECK: [f32; 3] = [0.0, 0.4, 0.7];

/// Gewaehlter Plan gegen ein Ziel (route = Nummer in der Routenliste, None = Luftlinie)
#[derive(Clone, Copy, Debug)]
pub struct Wahl { pub ertrag: f32, pub schaden: f32, pub verlust: f32, pub route: Option<usize>, pub rueck: f32, pub weiter: bool }

/// Bester Plan gegen ein Ziel (Ertrag = Schaden - risiko x Verlust) ueber Route, Rueckzugsschwelle, weiter/heim
pub fn besterPlan(ang: &Seite, ver: &Seite, tuerme: &[(i32, i32, i32)], ziel: ((i32, i32), usize, i32), routen: &[RoutenInfo],
                  basis: (i32, i32), risiko: f32) -> Option<Wahl> {
    let mut best: Option<Wahl> = None;
    let wege: Vec<Option<usize>> = if routen.is_empty() { vec![None] } else { (0..routen.len()).map(Some).collect() };
    for &k in wege.iter() { for &r in RUECK.iter() { for weiter in [false, true] {
        let y = vorhersage(&merkmale(ang, ver, tuerme, Some(ziel), k.map(|i| &routen[i]), basis, r, weiter))?;
        let e = y[0] - risiko * y[1];
        if best.map_or(true, |b| e > b.ertrag) { best = Some(Wahl { ertrag: e, schaden: y[0], verlust: y[1], route: k, rueck: r, weiter }); }
    } } }
    best
}

/// Routen des Trupps zum Ziel samt Merkmalen (Gefahr durch Feindarmee/Tuerme aus Sicht des Trupps)
pub fn routenMit(welt: &mut dyn Welt, z: &Zustand, ang: &Seite, ver: &Seite, tuerme: &[(i32, i32, i32)], zp: (i32, i32))
    -> (std::rc::Rc<Vec<super::routen::Route>>, Vec<RoutenInfo>) {
    let rs = super::routen::routen(welt, z, (ang.mx, ang.my), zp);
    let r = super::routen::raster(welt, z);
    let info = (0..rs.len()).map(|k| super::routen::bewerten(&r, &rs, k, &ver.fz, tuerme, ang.wert as f64)).collect();
    (rs, info)
}
