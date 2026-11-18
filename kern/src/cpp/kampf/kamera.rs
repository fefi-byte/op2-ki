//! Zuschauer-Kamera (OP2KI_KAMERA=1): Ansicht auf den heftigsten Kampf, ohne Spielzustand.
use super::*;

// Zuschauer-Kamera (Rust, nur mit Umgebungsvariable OP2KI_KAMERA): der lokale Spieler blickt auf den heftigsten Kampf.
// Kein Spielzustand -- CenterViewOn verschiebt nur die Ansicht, keine Zufallszahl, kein Befehl.
thread_local! {
    static KAMERA: std::cell::Cell<(i32, i32, i32, i32)> = const { std::cell::Cell::new((0, 0, 0, 0)) };   // x, y, Wertung, seit
}
pub fn kamera(welt: &mut dyn Welt, z: &Zustand) {
    thread_local! { static AN: bool = std::env::var("OP2KI_KAMERA").is_ok_and(|v| v == "1"); }
    if !AN.with(|a| *a) { return; }
    let lp = welt.LocalPlayer();
    if z.spieler != lp { return; }
    let t = welt.Tick();
    if t % 40 >= TAKT { return; }
    // Kampfeinheiten aller Spieler: Ort (Engine-Kacheln), Besitzer, Wert
    let mut e: Vec<(i32, i32, i32, i32)> = Vec::new();
    for p in 0..welt.NoPlayers() {
        for u in welt.PlayerVehicleEnum(p) {
            if !welt.IsLive(u) { continue; }
            let w = kampfWert(welt, u);
            if w > 0 { let l = welt.Location(u); e.push((l.x, l.y, p, w)); }
        }
        for u in welt.PlayerBuildingEnum(p, mapGuardPost) {
            if welt.IsLive(u) { let l = welt.Location(u); e.push((l.x, l.y, p, kampfWert(welt, u).max(1))); }
        }
    }
    const R2: i32 = 9 * 9;
    // Wertung eines Punktes: die schwaechere Seite der zwei staerksten Gegner im Umkreis (umkaempft = beide da)
    let wertung = |x: i32, y: i32| -> (i32, i32, i32) {
        let mut je = [0i32; 8]; let (mut sx, mut sy, mut n) = (0, 0, 0);
        for &(ux, uy, p, w) in e.iter() {
            if (ux - x) * (ux - x) + (uy - y) * (uy - y) <= R2 { je[(p & 7) as usize] += w; sx += ux; sy += uy; n += 1; }
        }
        let mut s = je; s.sort_unstable_by(|a, b| b.cmp(a));
        if n == 0 { (0, x, y) } else { (s[1], sx / n, sy / n) }
    };
    let mut best = (0, 0, 0);
    for &(ux, uy, _, _) in e.iter() {
        let w = wertung(ux, uy);
        if w.0 > best.0 { best = w; }
    }
    let (kx, ky, kw, seit) = KAMERA.with(|k| k.get());
    // beim laufenden Kampf bleiben, solange er nicht deutlich kleiner ist als der staerkste
    if kw > 0 {
        let jetzt = wertung(kx, ky);
        if jetzt.0 > 0 && (jetzt.0 * 10 >= best.0 * 6 || t - seit < 200) {
            KAMERA.with(|k| k.set((jetzt.1, jetzt.2, jetzt.0, seit)));
            welt.CenterViewOn(lp, jetzt.1, jetzt.2);
            return;
        }
    }
    if best.0 > 0 {
        KAMERA.with(|k| k.set((best.1, best.2, best.0, t)));
        welt.CenterViewOn(lp, best.1, best.2);
    } else if kw > 0 {
        // Kampf vorbei: zurueck zur eigenen Basis
        KAMERA.with(|k| k.set((0, 0, 0, t)));
        let b = engine(welt, z, z.bx, z.by);
        welt.CenterViewOn(lp, b.x, b.y);
    }
}
