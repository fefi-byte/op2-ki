//! Aussenposten: gefaehrdet?, Wachen dort (Hand: aussenSchwach, Netz: truppAbwehr), Turm fehlt.
use super::*;
use super::super::*;
use super::{SCAN_TAKT, WERTUNG_TICK, WERT_WT, WERT_LYNX, MAX_WT, AUSSEN_R, B_KOLONIE, B_ZIEL, B_WIRTSCHAFT, B_VERTEIDIGUNG, B_ANZAHL, BEREICH, TECH_SELTENERZ, GEBAEUDE, snp, ix};

/// C++: bool aussenGefaehrdet(const Zustand& z, int i)  (op2_ki.hpp:1439)
pub fn aussenGefaehrdet(welt: &mut dyn Welt, z: &Zustand, i: i32) -> bool {
    if kampfModellNeu() && PAR(z.spieler).schwachAussen >= 0.5 {
        let iu = i as usize;
        if z.aussenId[iu] == 0 { return false; }
        if z.aussenAngriff[iu] != 0 && welt.Tick() - z.aussenAngriff[iu] < 10000 { return true; }
        return aussenSchwach(welt, z, i) > 0;
    }
    let i = i as usize;
    if z.aussenId[i] == 0 || z.bedrohung == 0 { return false; }
    if z.aussenAngriff[i] != 0 && welt.Tick() - z.aussenAngriff[i] < 10000 { return true; }
    z.feindGeb != 0 && abstand2(welt, z.aussenX[i], z.aussenY[i], z.feindX, z.feindY) < abstand2(welt, z.bx, z.by, z.feindX, z.feindY)
}

/// C++: int aussenFahrzeuge(const Zustand& z, int i)  (op2_ki.hpp:1444)
pub fn aussenFahrzeuge(welt: &mut dyn Welt, z: &Zustand, i: i32) -> i32 {
    if !aussenGefaehrdet(welt, z, i) { return 0; }
    let iu = i as usize;
    if kampfModellNeu() && (PAR(z.spieler).schwachAussen >= 0.5 || PAR(z.spieler).truppAbwehr >= 0.5) {
        let n = aussenSchwach(welt, z, i);
        return if z.aussenAngriff[iu] != 0 && welt.Tick() - z.aussenAngriff[iu] < 10000 { n.max(2) } else { n };
    }
    if z.aussenAngriff[iu] != 0 && welt.Tick() - z.aussenAngriff[iu] < 10000 { return 2; }
    if z.bedrohung >= 50 { 1 } else { 0 }
}

/// C++: int aussenTurmFehlt(const Zustand& z)  (op2_ki.hpp:1449)
pub fn aussenTurmFehlt(welt: &mut dyn Welt, z: &Zustand) -> i32 {
    for i in 0..6 {
        if aussenGefaehrdet(welt, z, i) && tuermeBei(welt, z, z.aussenX[i as usize], z.aussenY[i as usize], 6) == 0 { return i; }
    }
    -1
}


/// Rust (PAR schwachstelle, Nutzer 2026-11-17): Schwachstellen vorhersehen -- dieselbe Rechnung wie beim eigenen Angriff
/// (kampf.rs Zermuerben), aus Sicht des Gegners. Ein Aussenposten ist gefaehrdet, wenn er fuer den Gegner lohnender ist als
/// die eigene Basis (Wert / eigene Abwehr dort; Abwehr = Tuerme in 7 Kacheln + eigene Kampffahrzeuge nach Entfernung, voll bis
/// 10, nichts ab 40 Kacheln) und die Feindarmee dort mehr ausrichten kann als die Abwehr. Liefert die Wachen (Lynx), die ihn
/// wieder unter die Basis druecken, hoechstens ein Drittel der eigenen Armee; 0 = nicht gefaehrdet.
pub fn aussenSchwach(welt: &mut dyn Welt, z: &Zustand, i: i32) -> i32 {
    let iu = i as usize;
    if z.aussenId[iu] == 0 || z.feindArmee <= 0 { return 0; }
    let sp = z.spieler;
    if PAR(sp).truppAbwehr >= 0.5 && super::truppplan::geladen(welt) { return aussenSchwachNetz(welt, z, iu); }
    let mut eigene: Vec<(i32, i32, i32)> = Vec::new();
    for u in welt.PlayerVehicleEnum(sp) {
        if !welt.IsLive(u) { continue; }
        let w = kampfWert(welt, u); if w <= 0 { continue; }
        let lo = welt.Location(u); eigene.push((edX(welt, z, lo), edY(welt, z, lo), w));
    }
    let mut tuerme: Vec<(i32, i32, i32)> = Vec::new();
    for g in welt.PlayerBuildingEnum(sp, mapGuardPost) {
        if !welt.IsLive(g) { continue; }
        let lo = welt.Location(g); let w = kampfWert(welt, g); tuerme.push((edX(welt, z, lo), edY(welt, z, lo), w));
    }
    let abwehr = |welt: &mut dyn Welt, x: i32, y: i32| -> f64 {
        let mut a = 0.0;
        for &(tx, ty, w) in tuerme.iter() { if abstand2(welt, tx, ty, x, y) <= 7 * 7 { a += w as f64; } }
        for &(ex, ey, w) in eigene.iter() {
            let d = (abstand2(welt, ex, ey, x, y) as f64).sqrt();
            a += w as f64 * (1.0 - (d - 10.0) / 30.0).clamp(0.0, 1.0);
        }
        a
    };
    let aus: Id = z.aussenId[iu];
    if !welt.IsLive(aus) { return 0; }
    let typAus = welt.GetType(aus);
    let wOut = super::kampf::schwachZielWert(welt, typAus, sp) as f64;
    let wBasis = super::kampf::schwachZielWert(welt, mapCommandCenter, sp) as f64;
    let (ax, ay) = (z.aussenX[iu], z.aussenY[iu]);
    let aOut = abwehr(welt, ax, ay);
    let aBasis = abwehr(welt, z.bx, z.by);
    if wOut / (1.0 + aOut) <= wBasis / (1.0 + aBasis) || (z.feindArmee as f64) <= aOut { return 0; }
    let noetig = wOut * (1.0 + aBasis) / wBasis - 1.0 - aOut;
    let mehr = ((noetig / WERT_LYNX as f64).ceil() as i32).max(1);
    let grenze = (z.eigeneMobil / WERT_LYNX / 3).max(1);
    mehr.min(grenze)
}

/// Rust (PAR truppAbwehr, Truppnetz aus Feindsicht): lohnt sich fuer die Feindarmee ein Ueberfall auf Aussenposten i mehr
/// als einer auf die Basis (bzw. als truppMin)? Dann so viele zusaetzliche Wachen (Lynx), bis er sich nicht mehr lohnt.
pub(super) fn aussenSchwachNetz(welt: &mut dyn Welt, z: &Zustand, iu: usize) -> i32 {
    use super::truppplan as tp;
    let sp = z.spieler;
    let aus: Id = z.aussenId[iu];
    if !welt.IsLive(aus) { return 0; }
    let Some(k) = tp::zielKlasse(welt.GetType(aus)) else { return 0 };
    let mut feind = Vec::new(); let mut feindUs = Vec::new();
    let mut p = 0;
    while p < welt.NoPlayers() {
        if p == sp || welt.IsAlliedTo(sp, p) { p += 1; continue; }
        feind.extend(tp::kampfFz(welt, z, p));
        for u in welt.PlayerVehicleEnum(p) { if welt.IsLive(u) && kampfWert(welt, u) > 0 { feindUs.push(u); } }
        p += 1;
    }
    if feind.is_empty() { return 0; }
    let anteil = tp::gebAnteil(welt, &feindUs);
    let ang = tp::seite(feind, anteil, (z.feindX, z.feindY));
    let eigene = tp::kampfFz(welt, z, sp);
    let tuerme = tp::tuermeVon(welt, z, sp);
    let basis = (z.bx, z.by);
    let ta = welt.GetType(aus); let wAus = tp::zielWert(welt, ta, sp);
    let ziel = ((z.aussenX[iu], z.aussenY[iu]), k, wAus);
    let wCC = tp::zielWert(welt, mapCommandCenter, sp);
    let ver = tp::seite(eigene.clone(), 0.0, basis);
    let (_, infoB) = tp::routenMit(welt, z, &ang, &ver, &tuerme, basis);
    let eBasis = tp::besterPlan(&ang, &ver, &tuerme, (basis, 0, wCC), &infoB, basis, 1.0).map_or(0.0, |b| b.ertrag);
    let schwelle = eBasis.max(PAR(sp).truppMin as f32);
    let lynx = welt.GetOreCost(mapLynx, sp) + welt.GetOreCost(mapLaser, sp).max(welt.GetOreCost(mapMicrowave, sp));
    let grenze = (z.eigeneMobil / WERT_LYNX / 3).max(1);
    for mehr in 0..=grenze {
        let mut v = eigene.clone();
        for _ in 0..mehr { v.push((ziel.0 .0, ziel.0 .1, lynx)); }
        let ver = tp::seite(v, 0.0, basis);
        let (_, info) = tp::routenMit(welt, z, &ang, &ver, &tuerme, ziel.0);
        let e = tp::besterPlan(&ang, &ver, &tuerme, ziel, &info, basis, 1.0).map_or(0.0, |b| b.ertrag);
        if e <= schwelle { return mehr; }
    }
    grenze
}
