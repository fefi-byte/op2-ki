//! Erweiterung (nicht in der C++-KI, Nutzer 2026-11-17): Waffenwahl nach Paarungen. Die Tabelle stammt aus der Paarungs-
//! Arena der Sim (sim --paar-arena, 2592 Kaempfe: reine Gruppen Fahrgestell x Waffe, Plymouth gegen Eden, gleiches Budget,
//! Seltenerz doppelt, beide mit Handsteuerung samt Abstand halten, Stufe 6/8/10/12, Budget 6000/12000, je 3 Seeds; Mittel).
//! Eintrag = (Restwert Plymouth - Restwert Eden) / Budget. Die KI bewertet eine eigene Kombination gegen die gesehene
//! Feindmischung und gewichtet Waffen-, Fahrgestell- und Forschungswahl mit 1 + PAR paarung x Paarungswert.
//! Matchup table from the sim arena; own pick is weighted by its measured exchange against the enemy mix.
use std::cell::RefCell;

use super::*;

const FG: [i32; 3] = [mapLynx, mapPanther, mapTiger];
const PLY_W: [i32; 3] = [mapMicrowave, mapRPG, mapESG];
const EDEN_W: [i32; 4] = [mapLaser, mapRailGun, mapAcidCloud, mapThorsHammer];

/// Zeilen: Plymouth Fahrgestell x [Mikrowelle, RPG, ESG]; Spalten: Eden Fahrgestell x [Laser, Traegergewehr, Saeure, Thor].
const TABELLE: [[f64; 12]; 9] = [
    [ 0.478,  0.185,  0.007,  0.091,  0.525,  0.198, -0.108, -0.001,  0.266,  0.068, -0.281, -0.098],
    [ 0.407,  0.171, -0.162, -0.129,  0.611, -0.144, -0.278, -0.122,  0.483, -0.175, -0.335, -0.128],
    [ 0.452,  0.218,  0.228,  0.433,  0.538, -0.033, -0.138,  0.183,  0.375, -0.138, -0.209,  0.030],
    [ 0.279, -0.335, -0.393, -0.238,  0.314, -0.339, -0.374, -0.355,  0.118, -0.282, -0.408, -0.332],
    [ 0.325,  0.271, -0.084, -0.182,  0.548,  0.139, -0.187, -0.188,  0.376,  0.028, -0.293, -0.227],
    [ 0.354,  0.235,  0.118,  0.176,  0.390,  0.006, -0.102,  0.058,  0.282, -0.117, -0.213, -0.087],
    [ 0.363, -0.193, -0.380, -0.282,  0.378, -0.275, -0.268, -0.294,  0.266, -0.208, -0.396, -0.359],
    [ 0.283,  0.273, -0.083, -0.179,  0.422,  0.188, -0.096, -0.185,  0.385,  0.118, -0.261, -0.228],
    [ 0.394,  0.378,  0.260,  0.265,  0.473,  0.207,  0.040,  0.107,  0.409,  0.110, -0.079, -0.050],
];

fn ply_index(fg: i32, w: i32) -> Option<usize> {
    let f = FG.iter().position(|&x| x == fg)?; let k = PLY_W.iter().position(|&x| x == w)?; Some(f * 3 + k)
}
fn eden_index(fg: i32, w: i32) -> Option<usize> {
    let f = FG.iter().position(|&x| x == fg)?; let k = EDEN_W.iter().position(|&x| x == w)?; Some(f * 4 + k)
}

thread_local! {
    /// Feindmischung je Spieler, einmal je Tick: (Tick, Feind ist Eden, [(Tabellenindex, Anteil)])
    static MISCHUNG: RefCell<[(i32, bool, Vec<(usize, f64)>); 8]> = RefCell::new(Default::default());
}
pub fn zuruecksetzen_paarung() { MISCHUNG.with(|m| *m.borrow_mut() = Default::default()); }

/// Kampffahrzeuge der Feinde nach Tabellenindex (Anteile); ohne sichtbare Feindpanzer die Standardeinheit der anderen
/// Fraktion (Lynx mit Mikrowelle bzw. Laser).
fn mischung(welt: &mut dyn Welt, z: &Zustand) -> (bool, Vec<(usize, f64)>) {
    let sp = (z.spieler & 7) as usize; let t = welt.Tick();
    if let Some(m) = MISCHUNG.with(|m| { let m = m.borrow(); if m[sp].0 == t && !m[sp].2.is_empty() { Some((m[sp].1, m[sp].2.clone())) } else { None } }) { return m; }
    let feindEden = !welt.IsEden(z.spieler);
    let mut zahl: Vec<(usize, f64)> = Vec::new();
    for p in 0..welt.NoPlayers() {
        if p == z.spieler || welt.IsAlliedTo(z.spieler, p) { continue; }
        for u in welt.PlayerVehicleEnum(p) {
            if !welt.IsLive(u) { continue; }
            let (fg, w) = (welt.GetType(u), welt.GetWeapon(u));
            let i = if feindEden { eden_index(fg, w) } else { ply_index(fg, w) };
            if let Some(i) = i { if let Some(e) = zahl.iter_mut().find(|e| e.0 == i) { e.1 += 1.0; } else { zahl.push((i, 1.0)); } }
        }
    }
    if zahl.is_empty() { zahl.push((0, 1.0)); }   // Index 0 = Lynx mit Mikrowelle bzw. Laser
    let n: f64 = zahl.iter().map(|e| e.1).sum();
    for e in zahl.iter_mut() { e.1 /= n; }
    MISCHUNG.with(|m| m.borrow_mut()[sp] = (t, feindEden, zahl.clone()));
    (feindEden, zahl)
}

/// Steht die Kombination in der Arena-Tabelle (eigene Fraktion)?
pub fn imTabelle(welt: &mut dyn Welt, z: &Zustand, traeger: i32, waffe: i32) -> bool {
    if welt.IsEden(z.spieler) { eden_index(traeger, waffe).is_some() } else { ply_index(traeger, waffe).is_some() }
}

/// Gemessene Bilanz je Budget der eigenen Kombination gegen die Feindmischung (eigene Sicht; 0 ohne Tabelleneintrag).
pub fn paarWert(welt: &mut dyn Welt, z: &Zustand, traeger: i32, waffe: i32) -> f64 {
    let (feindEden, mix) = mischung(welt, z);
    let eigen = if feindEden { ply_index(traeger, waffe) } else { eden_index(traeger, waffe) };
    let Some(i) = eigen else { return 0.0 };
    mix.iter().map(|&(j, a)| a * if feindEden { TABELLE[i][j] } else { -TABELLE[j][i] }).sum()
}

/// Gewicht 1 + PAR paarung x Paarungswert (mindestens 0,1); 1 ohne Schalter oder ohne neues Kampfmodell.
pub fn paarFaktor(welt: &mut dyn Welt, z: &Zustand, traeger: i32, waffe: i32) -> f64 {
    let k = PAR(z.spieler).paarung;
    if k <= 0.0 || !kampfModellNeu() || traeger == mapGuardPost { return 1.0; }
    (1.0 + k * paarWert(welt, z, traeger, waffe)).max(0.1)
}
