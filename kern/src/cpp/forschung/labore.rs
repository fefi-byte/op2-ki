//! Laborsteuerung: Thema waehlen (Forschungs-Kuchen), Lynx-Vorrang, konzentrieren, starten; Wissenschaftler verteilen.
use super::*;
use super::super::*;
use super::{TECH_RAUMFAHRT, WERT_WT, WERT_LYNX, EMP_TECH, GEBAEUDE};

/// Lambda `liste` aus forschung(): erstes Thema auf dem Weg zu einer der Techniken in `tl`.
pub(super) fn forschungListe(welt: &mut dyn Welt, z: &Zustand, tl: &[i32], n: i32, name: &str, labTyp: i32, belegt: &[i32], nb: i32,
                  nm: &mut String, idx: &mut i32, art: &mut String) {
    let mut i = 0usize;
    while (i as i32) < n && *idx < 0 {
        *idx = zielThema(welt, z, tl[i], labTyp, belegt, nb, nm, 96);
        if *idx >= 0 {
            *art = name.to_string();
            if techInfo(welt, *idx).id != tl[i] { // Voraussetzung auf dem Weg
                *art = kurz(format!("{}auf dem Weg zu {} -- ", name, tl[i]), 96);
            }
        }
        i += 1;
    }
}

/// Ergebnis eines Laborschritts: weiter mit dem naechsten Labor oder fertig fuer diesen Takt (ein Auftrag je Schritt)
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Schritt { Weiter, Fertig }

/// Gewaehltes Thema fuer ein Labor samt Kuchenanteilen (fuer die Meldung)
pub(super) struct ThemenWahl { idx: i32, nm: String, art: String, gewaehlt: i32, zielOffen: bool, ltyp: i32,
                    anteilKolonie: i32, anteilZiel: i32, anteilMilitaer: i32, anteilWirtschaft: i32, warten: bool }

/// Eroeffnung der Forschung (Nutzer 2026-11-19, fest): Kindergarten, Universitaet, dann der Weg zum Lynx -- Cybernetic
/// Teleoperation, Focused Microwave Projection, Mobile Weapons Platform. Die Kolonie muss den fruehen Ansturm ueberleben.
pub const EROEFFNUNG: [i32; 5] = [3304, 3305, 3401, 3408, 3851];

/// Die noch nicht erforschten Themen der Eroeffnung in ihrer Reihenfolge. Leer = vorbei, die KI waehlt frei.
pub fn eroeffnungOffen(welt: &mut dyn Welt, z: &Zustand) -> Vec<i32> {
    if PAR(z.spieler).eroeffAus >= 0.5 { return Vec::new(); }   // Messschalter: altes Verhalten zum Vergleich
    // nur bei fruehem Angriff (geschaetzt aus Bau- und Fahrzeit zur naechsten Feindbasis, planung/scan.rs fruehWehr)
    if z.fwAngriff > 0 && z.fwAngriff as f64 > PAR(z.spieler).eroeffBis { return Vec::new(); }
    // PAR kigaNachLynx (Nutzer 2026-11-21, Test fuer Eden): Kindergarten erst nach der mobilen Waffenplattform
    let spaet = kigaSperre(welt, z);
    // PAR eroeffFolge (Nutzer 2026-11-21, Test fuer Eden): Universitaet zuerst, Kindergarten erst wenn Uni und AdvancedLab stehen
    const FOLGE: [i32; 5] = [3305, 3304, 3401, 3408, 3851];
    let liste: &[i32] = if PAR(z.spieler).eroeffFolge >= 0.5 { &FOLGE } else { &EROEFFNUNG };
    let mut v: Vec<i32> = liste.iter().copied().filter(|&id| techIndex(welt, id) >= 0 && !hatTechId(welt, z, id) && !(spaet && id == 3304)).collect();
    if spaet { v.retain(|&id| id != 3304); }
    v
}

/// PAR kigaNachLynx: 3304 (Kindergarten) gesperrt, solange 3851 (Mobile Waffenplattform) fehlt
pub fn kigaSperre(welt: &mut dyn Welt, z: &Zustand) -> bool {
    if PAR(z.spieler).kigaNachLynx >= 0.5 && !hatTechId(welt, z, 3851) { return true; }
    // PAR eroeffFolge: 3304 erst, wenn Universitaet und AdvancedLab stehen (oder gebaut werden)
    if PAR(z.spieler).eroeffFolge >= 0.5 {
        let steht = |welt: &mut dyn Welt, typ: i32| welt.PlayerBuildingEnum(z.spieler, typ).into_iter().any(|u| welt.IsLive(u));
        if !steht(welt, mapUniversity) || !steht(welt, mapAdvancedLab) { return true; }
    }
    false
}

/// Fehlt noch ein Gebaeude der Eroeffnung (Kindergarten, Universitaet, Labor fuer ihr naechstes Thema)? Auch wenn seine
/// Technik noch erforscht wird: die eine BuildingGroup baut der Reihe nach, eine Schmelze davor schiebt alles um ~4000 Ticks
pub fn eroeffnungBautNoch(welt: &mut dyn Welt, z: &Zustand, l: &Lage) -> bool {
    let offen = eroeffnungOffen(welt, z);
    if offen.is_empty() { return false; }
    for typ in [mapNursery, mapUniversity] {
        if hatGeb(welt, z, l, typ) == 0 && l.imBau[typ as usize] == 0 { return true; }
    }
    let lab = laborImWeg(welt, z, l, offen[0]);
    lab != 0 && hatGeb(welt, z, l, lab) == 0 && l.imBau[lab as usize] == 0
}

/// Bereich der Laborsteuerung (0 Kolonie, 1 Ziel, 2 Kampf, 3 Wirtschaft) aus dem Bereich der Tech-Tabelle
fn bereichVonTech(id: i32) -> i32 {
    match tb::tech(id).map(|t| t.bereich) { Some(1) => 2, Some(2) => 3, Some(3) => 1, _ => 0 }
}

/// C++: void forschung(Zustand& z, const Lage& l)  (op2_ki.hpp:5762). Rust 2026-11-18 in Schritte aufgeteilt (Verhalten gleich).
pub fn forschung(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage) {
    let t = welt.Tick();
    techMelden(welt, z, t);
    auftragPruefen(welt, z, t);
    let typen: [i32; 3] = [mapBasicLab, mapStandardLab, mapAdvancedLab];
    laborSonde(welt, z, l, t, &typen);
    super::hilfen::profil::marke("ki::forschung/wecken");
    if laboreWecken(welt, z, l, &typen) { return; }
    super::hilfen::profil::marke("ki::forschung/labore");
    let (mut belegt, mut nb) = belegteThemen(welt, z, &typen);
    for &typ in typen.iter() {
        for lab in welt.PlayerBuildingEnum(z.spieler, typ) {
            if !welt.IsLive(lab) { continue; } // gerade zerstoert
            if laborBedienen(welt, z, l, t, typ, lab, &typen, &mut belegt, &mut nb) == Schritt::Fertig { return; }
        }
    }
}

/// Spur: abgeschlossene Forschung melden (alle 200 Ticks)
pub(super) fn techMelden(welt: &mut dyn Welt, z: &mut Zustand, t: i32) {
    if t >= z.naechsteTechPruefung {
        z.naechsteTechPruefung = t + 200;
        let mut i = 0usize;
        while (i as i32) < tb::ANZ_TECH && i < 512 {
            if z.techGemeldet[i] == 0 && hatTech(welt, z, tb::TECH[i].id) {
                z.techGemeldet[i] = 1;
                if t > 0 { melde(welt, z, "techFertig", mapNone, z.bx, z.by, &format!("Tech {} abgeschlossen", tb::TECH[i].id)); }
            }
            i += 1;
        }
    }
}

/// Kalibrierung: hat der letzte Auftrag ein Thema gestartet?
pub(super) fn auftragPruefen(welt: &mut dyn Welt, z: &mut Zustand, t: i32) {
    if z.forschungLab != 0 && t - z.forschungSeit > 60 {
        let lab: Id = z.forschungLab;
        if welt.IsLive(lab) && !welt.HasPower(lab) {
            let ty = welt.GetType(lab);
            melde(welt, z, "info", ty, z.bx, z.by, "Labor ohne Strom -- Lesart von DoResearch noch offen");
        } else if welt.IsLive(lab) {
            let thema = welt.GetLabCurrentTopic(lab);
            // Nicht mehr umschalten: DoResearch nimmt den Tabellenindex (Absturz 00472D78 mit techID).
            let ty = welt.GetType(lab);
            if thema <= 0 && welt.GetCurAction(lab) != moDoResearch {
                melde(welt, z, "info", ty, z.bx, z.by, "Labor hat nicht angefangen (Strom, Wissenschaftler oder Leerlauf)");
            } else {
                let lesart = if z.forschungModus != 0 { "Index" } else { "techID" };
                melde(welt, z, "info", ty, z.bx, z.by, &format!("Forschung laeuft (Thema {}, Lesart {})", thema, lesart));
            }
        }
        z.forschungLab = 0;
    }
}

/// Sonde (PAR sonde): Zustand aller Labore in die Spur
pub(super) fn laborSonde(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage, t: i32, typen: &[i32; 3]) {
    if PAR(z.spieler).sonde >= 2.0 && (t as f64) >= PAR(z.spieler).sonde && (t as f64) < PAR(z.spieler).sonde + 200.0 {
        let mut v = Vec::new();
        for &typ in typen.iter() { for lab in welt.PlayerBuildingEnum(z.spieler, typ) {
            v.push(format!("{}:{} busy{} akt{} thema{} wiss{} strom{}", lab, typ, welt.IsBusy(lab) as i32, welt.GetCurAction(lab), welt.GetLabCurrentTopic(lab), welt.GetLabScientistCount(lab), welt.HasPower(lab) as i32));
        } }
        let verf = welt.GetNumAvailableScientists(z.spieler);
        melde(welt, z, "sonde", mapNone, 0, 0, &format!("Sonde Labore wissFrei {} verfuegbar {}: {}", l.wissFrei, verf, v.join("; ")));
    }
}

/// Stillgelegte Labore wecken, sobald es fuer ihren Labortyp ein Ziel- oder Kolonie-Thema gibt. Laeuft eine Forschung
/// unter ihrem Maximum, darf kein weiteres Labor starten (Konzentration) -- dann auch keins wecken. true = geweckt.
pub(super) fn laboreWecken(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage, typen: &[i32; 3]) -> bool {
    let mut konzentriert = false;
    if !standardVolk(welt, l) {
        for &typ in typen.iter() {
            for lab in welt.PlayerBuildingEnum(z.spieler, typ) {
                if !welt.IsLive(lab) { continue; } // gerade zerstoert
                let th = welt.GetLabCurrentTopic(lab);
                if th <= 0 { continue; }
                let i2 = if th < welt.GetTechCount() { th } else { techIndex(welt, th) };
                if i2 >= 0 && welt.GetLabScientistCount(lab) < techInfo(welt, i2).max_scientists { konzentriert = true; }
            }
        }
    }
    for &typ in typen.iter() {
        for lab in welt.PlayerBuildingEnum(z.spieler, typ) {
            if !welt.IsLive(lab) { continue; } // gerade zerstoert
            if !istGeparkt(welt, z, lab) || konzentriert { continue; }
            // Braucht die Forschung diesen Labortyp? Dann wecken.
            let lt = laborTyp(welt, typ);
            if z.laborNutzen[lt as usize] > 0 {
                let warum = kurz(format!("die Forschung braucht dieses Labor (Dringlichkeit {})", z.laborNutzen[lt as usize]), 140);
                leerlaufWecken(welt, z, lab, &warum);
                return true;
            }
            const WECK_T: [i32; 9] = [5104, TECH_RAUMFAHRT, 3502, 3304, 3305, 3303, 3406, 5307, 3403];
            let mut nm = String::new();
            let leer: [i32; 1] = [0];
            for &zt in WECK_T.iter() {
                if (zt == 5104 || zt == TECH_RAUMFAHRT || zt == 3502) && (z.ziel != 1 || z.zielErreicht != 0) { continue; }
                if zielThema(welt, z, zt, lt, &leer, 0, &mut nm, 96) >= 0 {
                    let warum = kurz(format!("Thema fuer dieses Labor: {}", nm), 140);
                    leerlaufWecken(welt, z, lab, &warum);
                    return true;
                }
            }
        }
    }
    false
}

/// Themen, an denen gerade ein Labor arbeitet (nicht doppelt vergeben)
pub(super) fn belegteThemen(welt: &mut dyn Welt, z: &mut Zustand, typen: &[i32; 3]) -> ([i32; 16], i32) {
    let mut belegt = [0i32; 16];
    let mut nb: i32 = 0;
    for &typ in typen.iter() {
        for lab in welt.PlayerBuildingEnum(z.spieler, typ) {
            if !welt.IsLive(lab) { continue; } // gerade zerstoert
            let th = welt.GetLabCurrentTopic(lab);
            if th > 0 && nb < 16 { belegt[nb as usize] = th; nb += 1; }
        }
    }
    (belegt, nb)
}

/// Ein Labor bedienen: Lynx-Vorrang, Bereitschaft, Thema waehlen (Kuchen), konzentrieren, starten
#[allow(clippy::too_many_arguments)]
pub(super) fn laborBedienen(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage, t: i32, typ: i32, lab: Id, typen: &[i32; 3], belegt: &mut [i32; 16], nb: &mut i32) -> Schritt {
    if let Some(s) = lynxVorrang(welt, z, l, t, typ, lab, typen) { return s; }
    if let Some(s) = laborNichtBereit(welt, z, typ, lab) { return s; }
    // PAR kigaNachLynx: 3304 gilt als belegt, solange gesperrt (die Wahl ueberspringt belegte Themen)
    let (mut bel, mut nbel) = (*belegt, *nb);
    if kigaSperre(welt, z) && (nbel as usize) < 16 { bel[nbel as usize] = 3304; nbel += 1; }
    let ThemenWahl { mut idx, mut nm, art, gewaehlt, zielOffen, ltyp, anteilKolonie, anteilZiel, anteilMilitaer, anteilWirtschaft, warten } =
        themaFuerLabor(welt, z, l, typ, &bel, nbel);
    if warten {
        if selten(welt, z, 26, 3000) {
            let lo = welt.Location(lab);
            let (x, y) = (edX(welt, z, lo), edY(welt, z, lo));
            melde(welt, z, "warte", typ, x, y, "Eroeffnung: kein Thema der Eroeffnung passt in dieses Labor -- es bleibt frei");
        }
        return Schritt::Weiter;
    }
    // Bei echter Bevoelkerung keine weiteren Nebenthemen, solange das Ziel offen ist (Wissenschaftler freihalten)
    if idx < 0 && zielOffen && welt.Scientists(z.spieler) < 100 {
        if selten(welt, z, 22, 4000) {
            let lo = welt.Location(lab);
            let x = edX(welt, z, lo);
            let y = edY(welt, z, lo);
            let s = welt.Scientists(z.spieler);
            melde(welt, z, "warte", typ, x, y, &format!("kein Ziel- oder Kolonie-Thema fuer dieses Labor -- haelt {} Wissenschaftler frei", s));
        }
        return Schritt::Weiter;
    }
    if idx < 0 { idx = themaWaehlen(welt, z, ltyp, &mut nm, 96); }
    if idx < 0 {
        if selten(welt, z, 3, 6000) {
            let lo = welt.Location(lab);
            let x = edX(welt, z, lo);
            let y = edY(welt, z, lo);
            melde(welt, z, "warte", typ, x, y, "kein erforschbares Thema fuer dieses Labor");
        }
        return Schritt::Weiter;
    }
    let ti = techInfo(welt, idx);
    let anteile = [anteilKolonie, anteilZiel, anteilMilitaer, anteilWirtschaft];
    if let Some(s) = konzentrieren(welt, z, l, t, typ, lab, typen, idx, &nm, gewaehlt, anteile) { return s; }
    let mut wiss = welt.GetNumAvailableScientists(z.spieler);
    if wiss > ti.max_scientists { wiss = ti.max_scientists; }
    if wiss <= 0 {
        if selten(welt, z, 4, 3000) {
            let lo = welt.Location(lab);
            let x = edX(welt, z, lo);
            let y = edY(welt, z, lo);
            melde(welt, z, "warte", typ, x, y, &format!("keine freien Wissenschaftler fuer {}", nm));
        }
        return Schritt::Weiter;
    }
    let wert = idx; // immer Tabellenindex (siehe Kalibrierung oben)
    welt.DoResearch(lab, wert, wiss);
    if gewaehlt >= 0 { z.forschGestartet[gewaehlt as usize] += 1; }
    bereichMerken(z, lab, gewaehlt);
    z.forschungLab = lab; z.forschungWert = wert; z.forschungSeit = t;
    if *nb < 16 { belegt[*nb as usize] = ti.id; *nb += 1; }
    let lo = welt.Location(lab);
    let x = edX(welt, z, lo);
    let y = edY(welt, z, lo);
    melde(welt, z, "forschung", typ, x, y, &format!("{}{} (Tech {}, {} Wissenschaftler) [Kuchen K{} Z{} M{} W{}]",
        art, nm, ti.id, wiss, anteilKolonie, anteilZiel, anteilMilitaer, anteilWirtschaft));
    return Schritt::Fertig; // ein Auftrag je Schritt
}

/// Grosslabor fuer den Lynx: fehlt er bei bekanntem Feind, startet hier nichts anderes, Laufendes wird gestoppt.
pub(super) fn lynxVorrang(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage, t: i32, typ: i32, lab: Id, typen: &[i32; 3]) -> Option<Schritt> {
    if typ == mapAdvancedLab && z.feindGemeldet != 0 && !hatTechId(welt, z, 3851) && !standardVolk(welt, l) {
        let lt = tb::tech(3851);
        let li = techIndex(welt, 3851);
        if let Some(lt) = lt {
            if li >= 0 && eigeneKosten(welt, z, lt) >= 0 {
                let lti = techInfo(welt, li);
                let mut bereit = true;
                for r in 0..lti.required.len() {
                    let rid = techInfo(welt, lti.required[r]).id;
                    if !hatTechId(welt, z, rid) { bereit = false; }
                }
                let th = welt.GetLabCurrentTopic(lab);
                let ti2 = if th > 0 { if th < welt.GetTechCount() { th } else { techIndex(welt, th) } } else { -1 };
                if ti2 == li { return Some(Schritt::Weiter); } // forscht schon den Lynx
                if th > 0 || welt.IsBusy(lab) || welt.GetCurAction(lab) == moDoResearch {
                    if bereit && (!kampfModellNeu() || laborBereit(welt, z, lab, typ)) {
                        welt.DoStop(lab);
                        let lo = welt.Location(lab);
                        let x = edX(welt, z, lo);
                        let y = edY(welt, z, lo);
                        let th_id = if ti2 >= 0 { techInfo(welt, ti2).id } else { th };
                        melde(welt, z, "forschung", typ, x, y, &format!("Grosslabor gestoppt (Thema {}) -- der Lynx hat Vorrang", th_id));
                    }
                    return Some(Schritt::Weiter);
                }
                if !bereit {
                    if selten(welt, z, 24, 3000) {
                        let lo = welt.Location(lab);
                        let x = edX(welt, z, lo);
                        let y = edY(welt, z, lo);
                        melde(welt, z, "warte", typ, x, y, "Grosslabor wartet auf den Lynx (Voraussetzungen fehlen noch)");
                    }
                    return Some(Schritt::Weiter);
                }
                if !welt.HasPower(lab) { return Some(Schritt::Weiter); }
                if kampfModellNeu() && !laborBereit(welt, z, lab, typ) { return Some(Schritt::Weiter); }
                let mut wiss = welt.GetNumAvailableScientists(z.spieler);
                // Zu wenige frei: andere Labore geben bis auf einen ab, dann einmal kurz warten
                if wiss < lti.max_scientists && z.lynxFreiSeit == 0 {
                    let mut frei = 0;
                    for &typ2 in typen.iter() {
                        for l2 in welt.PlayerBuildingEnum(z.spieler, typ2) {
                            if welt.IsLive(l2) && l2 != lab && welt.GetLabCurrentTopic(l2) > 0 && welt.GetLabScientistCount(l2) > 1 {
                                frei += welt.GetLabScientistCount(l2) - 1;
                                welt.SetLabScientistCount(l2, 1);
                            }
                        }
                    }
                    z.lynxFreiSeit = t;
                    if frei != 0 {
                        welt.RecalculateValues(z.spieler);
                        let lo = welt.Location(lab);
                        let x = edX(welt, z, lo);
                        let y = edY(welt, z, lo);
                        melde(welt, z, "wiss", typ, x, y, &format!("andere Labore geben {} Wissenschaftler fuer den Lynx ab", frei));
                        return Some(Schritt::Weiter);
                    }
                }
                if wiss > lti.max_scientists { wiss = lti.max_scientists; }
                if wiss <= 0 { return Some(Schritt::Weiter); }
                welt.DoResearch(lab, li, wiss);
                bereichMerken(z, lab, 2);   // Lynx = Kampf
                z.forschungLab = lab; z.forschungWert = li; z.forschungSeit = t;
                let lo = welt.Location(lab);
                let x = edX(welt, z, lo);
                let y = edY(welt, z, lo);
                melde(welt, z, "forschung", typ, x, y, &format!("Kampf (Lynx-Vorrang): Mobile Waffenplattform (Tech 3851, {} Wissenschaftler)", wiss));
                return Some(Schritt::Fertig);
            }
        }
    }
    None
}

/// Labor beschaeftigt, ohne Strom oder (neues Modell) ohne Arbeiter / nicht betriebsbereit
pub(super) fn laborNichtBereit(welt: &mut dyn Welt, z: &mut Zustand, typ: i32, lab: Id) -> Option<Schritt> {
    if welt.IsBusy(lab) || welt.GetCurAction(lab) == moDoResearch || welt.GetLabCurrentTopic(lab) > 0 { return Some(Schritt::Weiter); }
    if !welt.HasPower(lab) {
        if selten(welt, z, 7, 3000) {
            let lo = welt.Location(lab);
            let x = edX(welt, z, lo);
            let y = edY(welt, z, lo);
            melde(welt, z, "warte", typ, x, y, "Labor ohne Strom");
        }
        return Some(Schritt::Weiter);
    }
    // Neues Modell: ohne Arbeiter (z.B. noch ohne Roehre) nimmt das Labor keinen Auftrag an -- der Befehl alle
    // 20 Ticks waere verschenkt und blockierte die anderen Labore in dieser Runde.
    // Die Engine nimmt Auftraege nur an, solange der Schaden unter 3/4 der Trefferpunkte liegt (IsEnabled 0x483710)
    // -- auch ein Gebaeude im Aufbau steht so lange still.
    if kampfModellNeu() && !laborBereit(welt, z, lab, typ) {
        if selten(welt, z, 5, 3000) {
            let lo = welt.Location(lab);
            let x = edX(welt, z, lo);
            let y = edY(welt, z, lo);
            melde(welt, z, "warte", typ, x, y, "Labor ohne Arbeiter oder noch nicht betriebsbereit");
        }
        return Some(Schritt::Weiter);
    }
    None
}

/// Forschungs-Kuchen: Kolonie, Ziel (Satellit), Kampf, Wirtschaft nach Anteil; Katastrophen als Lueckenfueller
/// Kandidaten je Bereich aus der Techbaum-Vorhersage (Nutzen pro Wegkosten): Kampf (Lynx und EMP-Rakete vorn bzw. hinten),
/// Wirtschaft, Kolonie, Raumfahrt (nur bei offenem Ziel)
pub(super) struct Listen { kampf: [i32; 7], nKampf: usize, wirt: [i32; 6], nWirt: usize, kolV: [i32; 6], nKolV: usize, raum: [i32; 6], nRaum: usize }

/// Forschungs-Kuchen: Kolonie, Ziel (Satellit), Kampf, Wirtschaft nach Anteil; Katastrophen als Lueckenfueller.
/// (Die C++-Liste KOLONIE_T wurde berechnet, aber nie benutzt -- entfaellt, sie war ohne Nebenwirkung.)
pub(super) fn themaFuerLabor(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage, typ: i32, belegt: &[i32; 16], nb: i32) -> ThemenWahl {
    let mut nm = String::new();
    const FUELLER_T: [i32; 4] = [3201, 3202, 5302, 5303];
    let zielOffen = z.ziel == 1 && z.zielErreicht == 0;
    // Moral nicht "Great" und Kolonie klein: Kolonie-Forschung vor dem Ziel (Spieler)
    super::hilfen::profil::marke("ki::forschung/dringlichkeit");
    let dr = dringlichkeit(welt, z, l);
    super::hilfen::profil::marke("ki::forschung/vorhersage");
    // Missionsmodus (Nutzer 2026-11-22: "Die KI darf im Missionsmodus das Forum bauen sowie Freizeitanlagen und bessere
    // Wohnmodule"): sobald die Basis steht, Freizeit (3306), Forum (5108) und die besseren Wohnmodule (8105 Plymouth, 8104
    // Eden) in die Kolonie-Forschung -- vorher blieb das Freizeitzentrum ueber 200 000 Ticks "Technik 3306 fehlt"
    let koloniePlus: Vec<i32> = if super::super::modus::mission(z) && l.wohnraum >= 100 {
        let wohn = if welt.IsEden(z.spieler) { 8104 } else { 8105 };
        let raum = if PAR(z.spieler).raumhafen >= 0.5 { 5405 } else { 3306 };   // Raumhafen der Mission
        let spinne = if PAR(z.spieler).spinnen >= 0.5 && !welt.IsEden(z.spieler) { 5116 } else { 3306 };   // Legged Robots: Spinnenfabrik
        // Kraftwerke der Mission: MHD (Seltenerz), Solar (SULV + Solarsatellit, Raumhafen noetig)
        let mhd = if PAR(z.spieler).mhd >= 0.5 { 5107 } else { 3306 };
        let (sulv, solar) = if PAR(z.spieler).solar >= 0.5 && PAR(z.spieler).raumhafen >= 0.5 { (5503, 10204) } else { (3306, 3306) };
        [3306, 5108, wohn, raum, spinne, mhd, sulv, solar].into_iter().filter(|&t| !hatTechId(welt, z, t) && tb::tech(t).is_some_and(|tt| eigeneKostenT(welt, z, tt) >= 0)).collect()
    } else { Vec::new() };
    let anteilKolonie = if koloniePlus.is_empty() { dr.kolonie } else { dr.kolonie.max(30) };
    let anteilZiel = if zielOffen { dr.ziel } else { 0 }; // bekommt, was uebrig bleibt
    let mut idx: i32 = -1;
    let mut art = String::new();
    let ltyp = laborTyp(welt, typ);
    // Kampf und Wirtschaft aus der Techbaum-Vorhersage (Nutzen pro Wegkosten); PAR forschKampf gewichtet den Kampf-Anteil
    // (Nutzer 2026-11-19: mehr Fokus auf Kampf -- Standard 1)
    let anteilMilitaer = (dr.kampfForschung as f64 * PAR(z.spieler).forschKampf) as i32;
    let anteilWirtschaft = dr.wirtschaft;
    // Eroeffnung: solange offen, nur ihre Themen (oder deren Voraussetzungen) in ihrer Reihenfolge; passt keines in dieses
    // Labor, bleibt es frei -- seine Wissenschaftler stehen der Eroeffnung zur Verfuegung
    let eroeff = eroeffnungOffen(welt, z);
    if !eroeff.is_empty() {
        let (mut e_idx, mut e_nm, mut e_art) = (-1, String::new(), String::new());
        forschungListe(welt, z, &eroeff, eroeff.len() as i32, "Eroeffnung: ", ltyp, belegt, nb, &mut e_nm, &mut e_idx, &mut e_art);
        // Sicherheitsventil: kommt die Eroeffnung laenger als PAR eroeffWarten nicht weiter (z.B. ihr Labor ohne Anschluss),
        // forscht ein freies Labor wieder wie sonst -- sonst stand die ganze Forschung still (mp6_31: 23000 Ticks, die
        // Kolonie starb aus)
        let t = welt.Tick();
        if e_idx >= 0 { z.eroeffWarteSeit = 0; }
        else if z.eroeffWarteSeit == 0 { z.eroeffWarteSeit = t; }
        if e_idx >= 0 || (t - z.eroeffWarteSeit) < PAR(z.spieler).eroeffWarten as i32 {
            let gewaehlt = if e_idx >= 0 { bereichVonTech(eroeff[0]) } else { -1 };
            return ThemenWahl { idx: e_idx, nm: e_nm, art: e_art, gewaehlt, zielOffen, ltyp, anteilKolonie, anteilZiel,
                                anteilMilitaer, anteilWirtschaft, warten: e_idx < 0 };
        }
    }
    let Listen { kampf: KAMPF_T, nKampf, wirt: WIRT_T, nWirt, kolV: mut KOL_V, nKolV: mut nKolV, raum: RAUM_T, nRaum } =
        vorhersageListen(welt, z, l, anteilMilitaer, zielOffen);
    // Uni vor (Spieler 2026-11-07): ohne akute Bedrohung kommt sie zuerst -- erst nach dem Kindergarten.
    super::hilfen::profil::marke("ki::forschung/liste");
    let uniZuerst = hatTechId(welt, z, 3304) && !hatTechId(welt, z, 3305) && z.bedrohung < 50;
    if uniZuerst {
        if nKolV >= 6 { nKolV = 5; }
        let mut i = nKolV;
        while i > 0 { KOL_V[i] = KOL_V[i - 1]; i -= 1; }
        KOL_V[0] = 3305;
        nKolV += 1;
    }
    #[derive(Clone)]
    struct Anteil { tl: Vec<i32>, n: i32, name: &'static str, wert: i32, bereich: i32, rang: f64 }
    let mut reihe: [Anteil; 4] = [
        Anteil { tl: KOL_V.to_vec(), n: nKolV as i32, name: "Kolonie (Vorhersage): ", wert: anteilKolonie, bereich: 0, rang: 0.0 },
        Anteil { tl: RAUM_T.to_vec(), n: nRaum as i32, name: "Ziel Satellit (Vorhersage): ", wert: anteilZiel, bereich: 1, rang: 0.0 },
        Anteil { tl: KAMPF_T.to_vec(), n: nKampf as i32, name: "Kampf (Vorhersage): ", wert: anteilMilitaer, bereich: 2, rang: 0.0 },
        Anteil { tl: WIRT_T.to_vec(), n: nWirt as i32, name: "Wirtschaft (Vorhersage): ", wert: anteilWirtschaft, bereich: 3, rang: 0.0 },
    ];
    // Kuchen anteilig: wer im Verhaeltnis zu seinem Anteil am wenigsten Themen bekommen hat, ist dran.
    for a in reihe.iter_mut() {
        a.rang = if a.wert > 0 { a.wert as f64 / (1.0 + z.forschGestartet[a.bereich as usize] as f64) } else { -1.0 };
    }
    if uniZuerst { reihe[0].rang = 1e9; }
    // Missionsmodus: die fehlenden Kolonie-Techniken vorn in die Kolonie-Liste
    // Missionsmodus (aufstieg): schwerere Fahrgestelle und die Waffen der Paarungs-Arena vorn in die Kampf-Liste
    if PAR(z.spieler).aufstieg >= 0.5 && super::super::modus::mission(z) {
        let waffen: [i32; 3] = if welt.IsEden(z.spieler) { [5506, 8503, 10102] } else { [5602, 7103, 5508] };
        let mut tl: Vec<i32> = [5201, 8201].into_iter().chain(waffen).filter(|&t| !hatTechId(welt, z, t) && tb::tech(t).is_some_and(|tt| eigeneKostenT(welt, z, tt) >= 0)).collect();
        if !tl.is_empty() {
            for &t in reihe.iter().find(|a| a.bereich == 2).map(|a| a.tl[..a.n.max(0) as usize].to_vec()).unwrap_or_default().iter() { if !tl.contains(&t) { tl.push(t); } }
            if let Some(a) = reihe.iter_mut().find(|a| a.bereich == 2) { a.n = tl.len() as i32; a.tl = tl; }
        }
    }
    if !koloniePlus.is_empty() {
        let mut tl: Vec<i32> = koloniePlus.clone();
        for &t in reihe.iter().find(|a| a.bereich == 0).map(|a| a.tl[..a.n.max(0) as usize].to_vec()).unwrap_or_default().iter() { if !tl.contains(&t) { tl.push(t); } }
        if let Some(a) = reihe.iter_mut().find(|a| a.bereich == 0) { a.n = tl.len() as i32; a.tl = tl; }
    }
    for a in 0..4 {
        for b2 in (a + 1)..4 { if reihe[b2].rang > reihe[a].rang { reihe.swap(a, b2); } }
    }
    let mut gewaehlt: i32 = -1;
    for a in 0..4 {
        let vorher = idx;
        forschungListe(welt, z, &reihe[a].tl, reihe[a].n, reihe[a].name, ltyp, belegt, nb, &mut nm, &mut idx, &mut art);
        if vorher < 0 && idx >= 0 { gewaehlt = reihe[a].bereich; }
    }
    let zielweg = idx >= 0;
    // Lueckenfueller (Katastrophen) nur, wenn das Ziel erreicht ist (oder Standardvolk)
    if idx < 0 && (!zielOffen || standardVolk(welt, l)) {
        forschungListe(welt, z, &FUELLER_T, 4, "Lueckenfueller Katastrophen: ", ltyp, belegt, nb, &mut nm, &mut idx, &mut art);
    }
    ThemenWahl { idx, nm, art, gewaehlt, zielOffen, ltyp, anteilKolonie, anteilZiel, anteilMilitaer, anteilWirtschaft, warten: false }
}

pub(super) fn vorhersageListen(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage, anteilMilitaer: i32, zielOffen: bool) -> Listen {
    let mut KAMPF_T = [0i32; 7];
    let mut WIRT_T = [0i32; 6];
    let mut KOL_V = [0i32; 6];
    let mut RAUM_T = [0i32; 6];
    let mut nKampf = 0usize;
    let mut nWirt = 0usize;
    let mut nKolV = 0usize;
    let mut nRaum = 0usize;
    let mut empHinten = false;
    {
        // ein Feld fuer alle Aufrufe, wie in C++ (Reste des vorigen Aufrufs bleiben stehen)
        let mut v = [Vorhersage::default(); 6];
        // Noch gar kein Kampffahrzeug: Mobile Waffenplattform (Lynx) zuerst.
        if anteilMilitaer > 0 && fahrgestellWahl(welt, z, 1 << 20) == 0 && !hatTechId(welt, z, 3851) { KAMPF_T[nKampf] = 3851; nKampf += 1; }
        // EMP-Rakete (nur Plymouth): vorne, wenn die Abwehr doppelt so viel wert ist wie die Rakete kostet, sonst hinten.
        if anteilMilitaer > 0 && !welt.IsEden(z.spieler) && !hatTechId(welt, z, EMP_TECH) && fahrgestellWahl(welt, z, 1 << 20) != 0 {
            let mut ew = 0;
            if empLohnt(welt, z, Some(&mut ew)) {
                if ew as f64 >= (2 * empKostenWert(welt, z)) as f64 * PAR(z.spieler).empSchwelle {
                    let mut i = nKampf;
                    while i > 0 { KAMPF_T[i] = KAMPF_T[i - 1]; i -= 1; }
                    KAMPF_T[0] = EMP_TECH;
                    nKampf += 1;
                } else {
                    empHinten = true;
                }
            }
        }
        super::hilfen::profil::marke("ki::forschung/v-kampf");
        if anteilMilitaer > 0 {
            let n = bereichZiele(welt, z, l, 1, &mut v, 4);
            if n > 0 { linieSetzen(z.spieler, 1, v[0].id); }
            for i in 0..n as usize { KAMPF_T[nKampf] = v[i].id; nKampf += 1; }
        }
        if empHinten && nKampf < 6 { KAMPF_T[nKampf] = EMP_TECH; nKampf += 1; }
        super::hilfen::profil::marke("ki::forschung/v-wirtschaft");
        let mut n2 = bereichZiele(welt, z, l, 2, &mut v, 6);
        if n2 > 0 { linieSetzen(z.spieler, 2, v[0].id); }
        for i in 0..n2 as usize { WIRT_T[nWirt] = v[i].id; nWirt += 1; }
        super::hilfen::profil::marke("ki::forschung/v-kolonie");
        n2 = bereichZiele(welt, z, l, 0, &mut v, 6);
        if n2 > 0 { linieSetzen(z.spieler, 0, v[0].id); }
        for i in 0..n2 as usize { KOL_V[nKolV] = v[i].id; nKolV += 1; }
        super::hilfen::profil::marke("ki::forschung/v-raum");
        if zielOffen {
            n2 = bereichZiele(welt, z, l, 3, &mut v, 6);
            if n2 > 0 { linieSetzen(z.spieler, 3, v[0].id); }
            for i in 0..n2 as usize { RAUM_T[nRaum] = v[i].id; nRaum += 1; }
        }
    }
    Listen { kampf: KAMPF_T, nKampf, wirt: WIRT_T, nWirt, kolV: KOL_V, nKolV, raum: RAUM_T, nRaum }
}

/// Konzentrieren: lieber eine Forschung auf Maximum als zwei halbe (Spieler). Wartet oder laesst ein unwichtigeres
/// Labor Wissenschaftler abgeben.
#[allow(clippy::too_many_arguments)]
pub(super) fn konzentrieren(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage, t: i32, typ: i32, lab: Id, typen: &[i32; 3], idx: i32, nm: &str,
                            bereich: i32, anteile: [i32; 4]) -> Option<Schritt> {
    let ti = techInfo(welt, idx);
    if !standardVolk(welt, l) {
        // forschungVorrang: Rang = Kuchen-Anteil des Bereichs (Fueller 0); Themen ohne gemerkten Bereich (Lynx-Vorrang)
        // werden nicht angetastet
        let vorrang = PAR(z.spieler).forschungVorrang >= 0.5;
        let anteil = |b: i32| if (0..4).contains(&b) { anteile[b as usize] } else { 0 };
        let gNeu = if vorrang { anteil(bereich) } else { themaGruppe(welt, ti.id) };
        let mut warten = false;
        let mut abgeben: Id = 0;
        let mut abAnz = 0;
        let mut abTech = 0;
        for &typ2 in typen.iter() {
            for lab2 in welt.PlayerBuildingEnum(z.spieler, typ2) {
                if !welt.IsLive(lab2) || lab2 == lab { continue; } // zerstoert (Absturz 00472D78)
                let th = welt.GetLabCurrentTopic(lab2);
                if th <= 0 { continue; }
                let i2 = if th < welt.GetTechCount() { th } else { techIndex(welt, th) };
                if i2 < 0 { continue; }
                let t2 = techInfo(welt, i2);
                let g2 = if vorrang { match gemerkterBereich(z, lab2) { Some(b) => anteil(b), None => i32::MAX } } else { themaGruppe(welt, t2.id) };
                let anz = welt.GetLabScientistCount(lab2);
                if g2 >= gNeu && anz < t2.max_scientists { warten = true; }
                if g2 < gNeu && anz > 1 && anz > abAnz { abgeben = lab2; abAnz = anz; abTech = t2.id; }
            }
        }
        if warten {
            if selten(welt, z, 23, 3000) {
                let lo = welt.Location(lab);
                let x = edX(welt, z, lo);
                let y = edY(welt, z, lo);
                melde(welt, z, "warte", typ, x, y, &format!("konzentriert: {} wartet, bis die laufende Forschung voll besetzt ist", nm));
            }
            return Some(Schritt::Weiter);
        }
        if PAR(z.spieler).sonde >= 2.0 && (t as f64) >= PAR(z.spieler).sonde && (t as f64) < PAR(z.spieler).sonde + 200.0 {
            melde(welt, z, "sonde", typ, 0, 0, &format!("Sonde Konzentration Labor {} Thema {} (idx {}, max {}) gNeu {} warten {} abgeben {} ({} Wiss, Tech {}) wissFrei {}", lab, ti.id, idx, ti.max_scientists, gNeu, warten, abgeben, abAnz, abTech, l.wissFrei));
        }
        if l.wissFrei < ti.max_scientists && abgeben != 0 {
            welt.SetLabScientistCount(abgeben, 1);
            welt.RecalculateValues(z.spieler);
            z.forschungSeit = t; // Umverteilung wartet die Neuberechnung ab
            let lo = welt.Location(lab);
            let x = edX(welt, z, lo);
            let y = edY(welt, z, lo);
            melde(welt, z, "wiss", typ, x, y, &format!("Tech {} gibt {} Wissenschaftler ab fuer {} (wichtiger)", abTech, abAnz - 1, nm));
            return Some(Schritt::Fertig);
        }
    }
    None
}

/// C++: void wissenschaftlerVerteilen(Zustand& z, const Lage& l)  (op2_ki.hpp:6090)
// Wissenschaftler in laufenden Forschungen umverteilen. Vorrang: Ziel > Kolonie > Lueckenfueller/Sonstiges.
pub fn wissenschaftlerVerteilen(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage) {
    let t = welt.Tick();
    if t < z.naechsteVerteilung || standardVolk(welt, l) { return; }
    // Nach einem Forschungsstart erst neu rechnen lassen.
    if t - z.forschungSeit < 60 { return; }
    z.naechsteVerteilung = t + 200;
    #[derive(Clone, Copy, Default)]
    struct Lab { u: Id, anz: i32, max: i32, gruppe: i32, tech: i32 }
    let mut labs = [Lab::default(); 8];
    let mut n = 0usize;
    let typen: [i32; 3] = [mapBasicLab, mapStandardLab, mapAdvancedLab];
    let anzahlTechs = welt.GetTechCount();
    for &typ in typen.iter() {
        for lab in welt.PlayerBuildingEnum(z.spieler, typ) {
            if n >= 8 { break; }
            if !welt.IsLive(lab) { continue; }
            let th = welt.GetLabCurrentTopic(lab);
            if th <= 0 { continue; }
            let idx = if th < anzahlTechs { th } else { techIndex(welt, th) }; // Index oder techID -- beides abfangen
            if idx < 0 { continue; }
            let ti = techInfo(welt, idx);
            if !ti.valid { continue; }
            labs[n].u = lab;
            labs[n].anz = welt.GetLabScientistCount(lab);
            labs[n].max = ti.max_scientists;
            labs[n].gruppe = themaGruppe(welt, ti.id);
            labs[n].tech = ti.id;
            n += 1;
        }
    }
    if n == 0 { return; }
    // Wichtigste Forschung unter ihrem Maximum
    let mut ziel: i32 = -1;
    for i in 0..n {
        if labs[i].anz < labs[i].max && (ziel < 0 || labs[i].gruppe > labs[ziel as usize].gruppe) { ziel = i as i32; }
    }
    if ziel < 0 { return; }
    let zl = labs[ziel as usize];
    let freiVorher = welt.GetNumAvailableScientists(z.spieler);
    // Personal-Luecken gehen vor; ausserdem 1 Wissenschaftler Reserve.
    let wirklichFrei = freiVorher - beansprucht(welt, l, M_WISS as i32, 0, -1) - 1;
    let mut dazu;
    let mut quelle: i32 = -1;
    if wirklichFrei > 0 {
        dazu = if wirklichFrei < zl.max - zl.anz { wirklichFrei } else { zl.max - zl.anz };
    } else {
        // Unwichtigste Forschung mit mehr als einem Wissenschaftler als Quelle
        for i in 0..n {
            if i as i32 != ziel && labs[i].gruppe < zl.gruppe && labs[i].anz > 1 && (quelle < 0 || labs[i].gruppe < labs[quelle as usize].gruppe) { quelle = i as i32; }
        }
        if quelle < 0 { return; }
        let q = labs[quelle as usize];
        let abgebbar = q.anz - 1;
        dazu = if abgebbar < zl.max - zl.anz { abgebbar } else { zl.max - zl.anz };
        if dazu <= 0 { return; }
        welt.SetLabScientistCount(q.u, q.anz - dazu);
    }
    if dazu <= 0 { return; }
    welt.SetLabScientistCount(zl.u, zl.anz + dazu);
    welt.RecalculateValues(z.spieler);
    let zt = welt.GetType(zl.u);
    let lo = welt.Location(zl.u);
    let x = edX(welt, z, lo);
    let y = edY(welt, z, lo);
    let text = format!("Tech {}: {} -> {} Wissenschaftler (max {}), {} {}, frei vorher {} (Personal fehlt {})",
        zl.tech, zl.anz, zl.anz + dazu, zl.max, if quelle >= 0 { "abgezogen von Tech" } else { "freie, Quelle" },
        if quelle >= 0 { labs[quelle as usize].tech } else { 0 }, freiVorher, l.wissFehlt);
    melde(welt, z, "wiss", zt, x, y, &text);
}

/// C++: void forschungsLuecke(const Zustand& z, const Lage& l, int& luecke, int& plaetze, int& rangLabor)  (op2_ki.hpp:6152)
// Forschungs-Luecke: freie Forscherplaetze der laufenden Themen und alle Plaetze, dazu der hoechste Laborrang.
pub fn forschungsLuecke(welt: &mut dyn Welt, z: &Zustand, l: &Lage, luecke: &mut i32, plaetze: &mut i32, rangLabor: &mut i32) {
    *luecke = 0; *plaetze = 0; *rangLabor = 0;
    let typen: [i32; 3] = [mapBasicLab, mapStandardLab, mapAdvancedLab];
    let anzahlTechs = welt.GetTechCount();
    for &typ in typen.iter() {
        for lab in welt.PlayerBuildingEnum(z.spieler, typ) {
            if !welt.IsLive(lab) { continue; }
            let th = welt.GetLabCurrentTopic(lab);
            if th <= 0 { continue; }
            let idx = if th < anzahlTechs { th } else { techIndex(welt, th) };
            if idx < 0 { continue; }
            let ti = techInfo(welt, idx);
            if !ti.valid { continue; }
            let mx = ti.max_scientists;
            let anz = welt.GetLabScientistCount(lab);
            *plaetze += mx;
            *luecke += if mx > anz { mx - anz } else { 0 };
            let r = rang(welt, z, l, typ);
            if r > *rangLabor { *rangLabor = r; }
        }
    }
}

/// forschungVorrang: Bereich des in Labor lab gestarteten Themas merken
pub(super) fn bereichMerken(z: &mut Zustand, lab: Id, bereich: i32) {
    let i = (0..8).find(|&i| z.forschLab[i] == lab).or_else(|| (0..8).find(|&i| z.forschLab[i] == 0)).unwrap_or(7);
    z.forschLab[i] = lab; z.forschLabBereich[i] = bereich;
}

pub(super) fn gemerkterBereich(z: &Zustand, lab: Id) -> Option<i32> {
    (0..8).find(|&i| z.forschLab[i] == lab && lab != 0).map(|i| z.forschLabBereich[i])
}
