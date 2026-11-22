//! PAR bauStau (2026-11-21): Stau zwischen eigenen Bau- und Erdbaufahrzeugen loesen. Gemessen ml4_08 Seed 3: der ConVec
//! mit dem Tokamak-Bausatz und der Earthworker auf dem Weg zu seinen Mauerkacheln standen ab Tick 2750 nebeneinander
//! (160,92 / 161,92) und warteten aufeinander, bis der Wachhund nach 8000 Ticks den Auftrag abbrach -- so lange stand
//! die ganze Eroeffnung (Kindergarten 8683 statt ~3000). Wer fahren will und STAU_TICKS auf derselben Kachel steht, ist
//! festgefahren: das eigene Fahrzeug direkt daneben faehrt 3 Kacheln zur Seite.
use super::*;
use std::cell::RefCell;
use std::collections::HashMap;

const STAU_TICKS: i32 = 600;

thread_local! { static ORT: RefCell<[HashMap<Id, (i32, i32, i32)>; 8]> = RefCell::new(Default::default()); }

fn baufahrzeug(t: i32) -> bool { t == mapConVec || t == mapEarthworker || t == mapRoboMiner || t == mapRoboSurveyor }

pub fn stauLoesen(welt: &mut dyn Welt, z: &mut Zustand) {
    if PAR(z.spieler).bauStau < 0.5 { return; }
    let t = welt.Tick();
    let sp = (z.spieler & 7) as usize;
    // eigene Fahrzeuge (Kacheln), Bau- und Erdbaufahrzeuge mit ihrem Fahrwunsch
    let mut alle: Vec<(Id, i32, i32, i32)> = Vec::new();
    for u in welt.PlayerVehicleEnum(z.spieler) {
        if !welt.IsLive(u) { continue; }
        let lo = welt.Location(u);
        alle.push((u, edX(welt, z, lo), edY(welt, z, lo), welt.GetType(u)));
    }
    let mut fest: Vec<(Id, i32, i32)> = Vec::new();
    ORT.with(|o| {
        let mut o = o.borrow_mut();
        let m = &mut o[sp];
        m.retain(|id, _| alle.iter().any(|a| a.0 == *id));
        for &(u, x, y, ty) in alle.iter() {
            if !baufahrzeug(ty) { continue; }
            let faehrt = welt.GetCurAction(u) == moMove;
            let e = m.entry(u).or_insert((x, y, t));
            if !faehrt || e.0 != x || e.1 != y { *e = (x, y, t); continue; }
            if t - e.2 >= STAU_TICKS { fest.push((u, x, y)); e.2 = t; }
        }
    });
    for (u, x, y) in fest {
        // Nachbar direkt daneben (Schachbrett 1), der nicht derselbe ist
        let Some(&(n, nx, ny, nty)) = alle.iter().find(|a| a.0 != u && (a.1 - x).abs() <= 1 && (a.2 - y).abs() <= 1) else { continue };
        // freie, befahrbare Kachel 3 vom Nachbarn, moeglichst weit vom Festgefahrenen
        let mut best: Option<(i32, i32, i32)> = None;
        for (dx, dy) in [(3, 0), (-3, 0), (0, 3), (0, -3), (2, 2), (2, -2), (-2, 2), (-2, -2)] {
            let (qx, qy) = (nx + dx, ny + dy);
            if !aufKarte(welt, z, qx, qy, 2) { continue; }
            let c = zelle(welt, z, qx, qy);
            if !zelleBefahrbar(welt, c) { continue; }
            if alle.iter().any(|a| a.1 == qx && a.2 == qy) { continue; }
            let d = (qx - x).pow(2) + (qy - y).pow(2);
            if best.map_or(true, |b| d > b.2) { best = Some((qx, qy, d)); }
        }
        let Some((qx, qy, _)) = best else { continue };
        let ziel = engine(welt, z, qx, qy);
        welt.DoMove(n, ziel);
        let ut = welt.GetType(u); let (nm, nmn) = (name(welt, ut), name(welt, nty));
        melde(welt, z, "bau", nty, x, y, &format!("Stau: {} steht seit {} Ticks bei {},{} -- {} daneben faehrt zur Seite nach {},{}", nm, STAU_TICKS, x, y, nmn, qx, qy));
    }
}
