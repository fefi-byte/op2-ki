//! Feldherr (Nutzer 2026-11-19, PAR marschSammeln): auf dem Marsch zum Aufmarschpunkt regelmaessig sammeln. Die Truppe
//! faehrt den Weg in Etappen von PAR marschEtappe Kacheln; am Etappenpunkt warten die Vorderen, bis PAR marschAnteil der
//! Gruppe in PAR marschRadius Kacheln steht (oder die Etappenfrist ablaeuft), dann geht es weiter. Ohne das zog sich die
//! Truppe ueber die Karte und wurde einzeln aufgerieben (gemessen 2026-11-21: ml4_05 48 Fahrzeuge los, 2 am Aufmarsch).
use super::*;
use std::cell::RefCell;

/// Marsch einer Angriffsgruppe: Etappenpunkte (Kacheln), naechster Punkt, seit wann er gilt
#[derive(Clone, Default)]
struct Marsch { punkte: Vec<(i32, i32)>, i: usize, seit: i32, ziel: (i32, i32) }

thread_local! { static MARSCH: RefCell<[[Option<Marsch>; 2]; 8]> = RefCell::new(Default::default()); }

pub fn an(z: &Zustand) -> bool { PAR(z.spieler).marschSammeln >= 0.5 }

/// Neuer Angriff: alte Marschwege vergessen
pub fn zuruecksetzen(z: &Zustand) { MARSCH.with(|m| m.borrow_mut()[(z.spieler & 7) as usize] = [None, None]); }

/// Lebende Fahrzeuge der Gruppe (Kacheln)
fn orte(welt: &mut dyn Welt, z: &Zustand, g: Gruppe) -> Vec<(i32, i32)> {
    let mut v = Vec::new();
    for u in welt.GroupEnumerator(g) {
        if !welt.IsLive(u) || !welt.IsVehicle(u) { continue; }
        let lo = welt.Location(u); v.push((edX(welt, z, lo), edY(welt, z, lo)));
    }
    v
}

fn befehl(welt: &mut dyn Welt, z: &Zustand, g: Gruppe, p: (i32, i32)) {
    let warte = Rect::new(engine(welt, z, p.0 - 2, p.1 - 2), engine(welt, z, p.0 + 2, p.1 + 2));
    let schutz = Rect::new(engine(welt, z, p.0 - 5, p.1 - 5), engine(welt, z, p.0 + 5, p.1 + 5));
    wachen(welt, g, warte, &[schutz], 1);
}

/// Ein Takt fuer Gruppe k (0 = angriff, 1 = angriff2) auf dem Weg zum Aufmarschpunkt `ziel`. true = noch unterwegs.
fn schritt(welt: &mut dyn Welt, z: &mut Zustand, k: usize, g: Gruppe, ziel: (i32, i32), t: i32) -> bool {
    let sp = (z.spieler & 7) as usize;
    let os = orte(welt, z, g);
    if os.is_empty() { return false; }
    let pm = PAR(z.spieler);
    let etappe = pm.marschEtappe.max(4.0) as usize;
    let radius = pm.marschRadius.max(2.0) as i32;
    let alt = MARSCH.with(|m| m.borrow()[sp][k].clone());
    let mut m = match alt {
        Some(m) if m.ziel == ziel => m,
        _ => {
            // Weg vom Schwerpunkt der Gruppe zum Aufmarschpunkt, Etappenpunkte alle `etappe` Wegkacheln
            let n = os.len() as i32;
            let s = (os.iter().map(|o| o.0).sum::<i32>() / n, os.iter().map(|o| o.1).sum::<i32>() / n);
            let rs = super::super::routen::routen(welt, z, s, ziel);
            let mut punkte: Vec<(i32, i32)> = Vec::new();
            if let Some(r) = rs.first() {
                let mut j = etappe;
                while j + etappe / 2 < r.weg.len() { punkte.push(r.weg[j]); j += etappe; }
            }
            if !punkte.is_empty() {
                melde(welt, z, "kampf", mapLynx, ziel.0, ziel.1, &format!("Feldherr: Marsch zum Aufmarsch in {} Etappen zu {} Kacheln (sammeln an jedem Etappenpunkt)", punkte.len() + 1, etappe));
            }
            let m = Marsch { punkte, i: 0, seit: t, ziel };
            if let Some(&p) = m.punkte.first() { befehl(welt, z, g, p); }
            m
        }
    };
    if m.i >= m.punkte.len() { MARSCH.with(|x| x.borrow_mut()[sp][k] = Some(m)); return false; }
    let p = m.punkte[m.i];
    let dort = os.iter().filter(|o| abstand2(welt, o.0, o.1, p.0, p.1) <= radius * radius).count();
    // Frist je Etappe: Lynx ~50 Ticks je Kachel, dazu Zeit fuer die Nachzuegler
    let frist = 600 + 80 * etappe as i32;
    let beisammen = dort as f64 >= pm.marschAnteil * os.len() as f64;
    if beisammen || t - m.seit > frist {
        m.i += 1; m.seit = t;
        if m.i < m.punkte.len() { let q = m.punkte[m.i]; befehl(welt, z, g, q); }
        else {
            // letzte Etappe: wie bisher an den Aufmarschpunkt
            befehl(welt, z, g, ziel);
            if !beisammen { melde(welt, z, "kampf", mapLynx, p.0, p.1, &format!("Feldherr: Etappe {} nach Frist ({} von {} beisammen)", m.i, dort, os.len())); }
        }
    }
    let unterwegs = m.i < m.punkte.len();
    MARSCH.with(|x| x.borrow_mut()[sp][k] = Some(m));
    unterwegs
}

/// Beide Angriffsgruppen marschieren lassen; true = mindestens eine ist noch unterwegs (der Aufmarsch wartet)
pub fn marschieren(welt: &mut dyn Welt, z: &mut Zustand, t: i32) -> bool {
    let mut unterwegs = false;
    let (a1, a2) = (z.angriff, z.angriff2);
    let z1 = (z.aufmarschX, z.aufmarschY);
    let z2 = if z.aufmarsch2X != 0 { (z.aufmarsch2X, z.aufmarsch2Y) } else { z1 };
    if schritt(welt, z, 0, a1, z1, t) { unterwegs = true; }
    if z.angriff2An != 0 && schritt(welt, z, 1, a2, z2, t) { unterwegs = true; }
    unterwegs
}
