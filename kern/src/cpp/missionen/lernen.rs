//! Missionen lernen aus ihren Verlusten: aus geschickter Kampfkraft s und Verlust L die Abwehr zurueckrechnen, die
//! tatsaechlich da war (Lanchester rueckwaerts: L = s (1 - sqrt(1 - (dd/s)^2))  =>  dd = s sqrt(1 - (1 - L/s)^2)),
//! je Missionsart gegen die Schaetzung summieren. Korrektur = (Vorwissen + gemessen) / (Vorwissen + geschaetzt).
use super::*;

/// Gewicht des Vorwissens in Kampfkraft (so viel geschaetzte Abwehr, bis die Messung gleich viel zaehlt)
const VORWISSEN: f64 = 40.0;

fn vorwissen(sp: i32, art: i32) -> f64 {
    let pm = PAR(sp);
    match art { UEBERFALL => pm.missionKorrUeberfall, ENTSCHEIDUNG => pm.missionKorrEntscheidung, ABWEHR => pm.missionKorrAbwehr, _ => pm.missionKorrHalten }
}

/// Faktor auf die geschaetzte Abwehr fuer eine Missionsart (1 ohne missionLernen)
pub fn korrektur(z: &Zustand, art: i32) -> f64 {
    if PAR(z.spieler).missionLernen <= 0.0 { return 1.0; }
    let a = art.clamp(0, 7) as usize;
    let k = vorwissen(z.spieler, art);
    ((k * VORWISSEN + z.missLernIst[a]) / (VORWISSEN + z.missLernErw[a])).clamp(0.3, 5.0)
}

/// Abwehr, die zum Verlust L bei geschickter Kraft s passt; aufgerieben (L >= s): mindestens s
pub fn gemesseneAbwehr(s: f64, l: f64) -> f64 {
    if s <= 0.0 { return 0.0; }
    if l >= s { return s; }
    let q = 1.0 - l.max(0.0) / s;
    s * (1.0 - q * q).max(0.0).sqrt()
}

/// Am Missionsende: Messung buchen und melden
pub fn buchen(welt: &mut dyn Welt, z: &mut Zustand, i: usize, kwRest: i32) {
    let s = z.missKwStart[i] as f64;
    if s <= 0.0 { return; }
    let l = (s - kwRest as f64).max(0.0);
    let ist = gemesseneAbwehr(s, l);
    let a = z.missArt[i].clamp(0, 7) as usize;
    z.missLernIst[a] += ist; z.missLernErw[a] += z.missDd[i];
    let f = korrektur(z, z.missArt[i]);
    let (x, y) = (z.missX[i], z.missY[i]);
    melde(welt, z, "lernen", mapNone, x, y, &format!("Missionslernen {}: geschickt {:.0}, verloren {:.0} -> Abwehr gemessen {:.1}, geschaetzt {:.1}; Summe {:.0}/{:.0}, Faktor {:.2}",
          artName(z.missArt[i]), s, l, ist, z.missDd[i], z.missLernIst[a], z.missLernErw[a], f));
}
