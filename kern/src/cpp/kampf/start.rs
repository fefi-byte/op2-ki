//! Angriff starten (losschlagen): Wach-Ueberschuss, Aufmarsch mit/ohne EMP, Heim- und Sammelplatz, Wellen.
use super::*;
use super::{GRUPPE_MAX, EMP_RADIUS, EMP_AUFMARSCH};

/// Lambda heim in kampfFuehrung: Warteplatz der Wache zum Feind hin.
pub(super) fn heim(welt: &mut dyn Welt, z: &mut Zustand, wx: i32, wy: i32, basis: Rect, warum: Option<&str>) {
    let mut hx = wx; let mut hy = wy;
    freierWarteplatz(welt, z, &mut hx, &mut hy, 3, true);
    platzSetzen(z.spieler, 0, hx); platzSetzen(z.spieler, 1, hy);
    let warte = Rect::new(engine(welt, z, hx - 3, hy - 3), engine(welt, z, hx + 3, hy + 3));
    wachen(welt, z.wache, warte, &[basis], 1);
    if let Some(w) = warum { melde(welt, z, "kampf", mapLynx, hx, hy, &format!("Wache an der Kolonie (Warteplatz {},{}): {}", hx, hy, w)); }
}

/// Lambda sammelPlatz in kampfFuehrung: Sammelplatz hinter der Kolonie.
pub(super) fn sammelPlatz(welt: &mut dyn Welt, z: &mut Zustand, sx: i32, sy: i32, basis: Rect) {
    let mut qx = sx; let mut qy = sy;
    freierWarteplatz(welt, z, &mut qx, &mut qy, 2, false);
    platzSetzen(z.spieler, 2, qx); platzSetzen(z.spieler, 3, qy);
    let s = Rect::new(engine(welt, z, qx - 2, qy - 2), engine(welt, z, qx + 2, qy + 2));
    wachen(welt, z.sammel, s, &[basis], 1);
}

/// Lambda losschlagen in kampfFuehrung.
#[allow(clippy::too_many_arguments)]
/// Angriff starten: Wach-Ueberschuss abgeben, Aufmarschpunkt (mit oder ohne EMP-Rakete), Bilanz beginnen, melden.
/// Rust 2026-11-18 aufgeteilt (Verhalten gleich).
pub(super) fn losschlagen(welt: &mut dyn Welt, z: &mut Zustand, t: i32, fz: i32, ueber: i32, sw: &mut i32, ns: &mut i32, art: i32, wunsch: i32, text: &str) {
    if sicherAlle(welt, z, z.angriff, z.sammel) > 0 { sicherAlle(welt, z, z.angriff2, z.sammel); }
    let (bewegt, nb) = wacheAbgeben(welt, z, fz, ueber);
    *sw += bewegt; *ns += nb;
    if *ns <= 0 { z.angriffWunsch = 0; return; }        // nichts zum Losschicken
    z.angriffZiel = 0; z.angriffWunsch = wunsch; z.angriffArt = art;
    // Mit EMP-Rakete: erst Aufmarsch knapp ausserhalb des EMP-Radius (Modus 3), dann Start (Modus 4).
    let mut mitEmp = false;
    if !empBereit(welt, z) { aufmarschOhneEmp(welt, z, t, fz, art, wunsch, *ns); mitEmp = true; }
    else if empBereit(welt, z) { aufmarschMitEmp(welt, z, t); mitEmp = true; }
    else { angriffZielSetzen(welt, z, z.bx, z.by); }
    z.angriffStart = *sw; z.kampfModus = if mitEmp { 3 } else { 2 }; z.letzterAngriffStart = t;
    z.angriffZerstoertStart = z.zerstoertWert; z.angriffVerlorenStart = z.verlorenWert;
    bilanzStart(welt, z);
    melde(welt, z, "kampf", if fz != 0 { fz } else { mapLynx }, z.feindX, z.feindY, &format!("{} ({} Fahrzeuge, Wert {})", text, *ns, *sw));
}

/// Ueberschuss der Wache dazu: fertige, ruhige Kaempfer, bis der Wert erreicht ist. Liefert (Wert, Anzahl).
pub(super) fn wacheAbgeben(welt: &mut dyn Welt, z: &mut Zustand, fz: i32, ueber: i32) -> (i32, i32) {
    let mut bewegt = 0; let mut nb = 0;
    if ueber > 0 {
        let mut raus = [0 as Id; 64]; let mut nr = 0usize;
        for v in welt.GroupEnumerator(z.wache) {
            if nr >= 64 { break; }
            if !welt.IsLive(v) || !kaempfer(welt, v) { continue; }
            let mut schon = false; for i in 0..nr { if raus[i] == v { schon = true; } }
            if !schon { raus[nr] = v; nr += 1; }
        }
        let mut i = 0usize;
        while i < nr && bewegt < ueber {
            let kw = kampfWert(welt, raus[i]);
            if bewegt > 0 && bewegt + kw > ueber + kw / 2 { i += 1; continue; }
            let ersteVoll = welt.TotalUnitCount(z.angriff) >= GRUPPE_MAX;
            if ersteVoll && welt.TotalUnitCount(z.angriff2) >= GRUPPE_MAX { break; }
            welt.RemoveUnit(z.wache, raus[i]);
            welt.TakeUnit(if ersteVoll { z.angriff2 } else { z.angriff }, raus[i]);
            bewegt += kw; nb += 1;
            i += 1;
        }
        if nb != 0 {
            melde(welt, z, "kampf", if fz != 0 { fz } else { mapLynx }, z.bx, z.by,
                  &format!("Wache gibt {} Fahrzeuge (Wert {}) an den Angriff ab -- daheim bleibt, was der Feind neu aufstellen koennte", nb, bewegt));
        }
    }
    (bewegt, nb)
}

/// Steht eine EMP-Rakete bereit und lohnt ein Ziel? (Ziel wird dabei neu bestimmt)
pub(super) fn empBereit(welt: &mut dyn Welt, z: &mut Zustand) -> bool {
    if z.empRakete == 0 { return false; }
    let (mut ex, mut ey) = (z.empZielX, z.empZielY);
    let w = empZiel(welt, z, &mut ex, &mut ey);
    z.empZielX = ex; z.empZielY = ey;
    w > 0
}

/// Sammeln vor jedem Angriff: auf der Linie zur eigenen Basis, ausserhalb der Reichweite aller Tuerme
#[allow(clippy::too_many_arguments)]
pub(super) fn aufmarschOhneEmp(welt: &mut dyn Welt, z: &mut Zustand, t: i32, fz: i32, _art: i32, wunsch: i32, _ns: i32) {
    let mut zx = z.feindX; let mut zy = z.feindY;
    { let w: Id = wunsch; if wunsch != 0 && welt.IsLive(w) { let l0 = welt.Location(w); zx = edX(welt, z, l0); zy = edY(welt, z, l0); } }
    let rx = (z.bx - zx) as f64; let ry = (z.by - zy) as f64;
    let d = if (rx * rx + ry * ry).sqrt() > 1.0 { (rx * rx + ry * ry).sqrt() } else { 1.0 };
    z.aufmarschX = z.bx; z.aufmarschY = z.by;
    let mut abst = 10;
    while abst <= 40 && (abst as f64) < d {
        let ax = (zx as f64 + rx / d * abst as f64 + 0.5) as i32; let ay = (zy as f64 + ry / d * abst as f64 + 0.5) as i32;
        let ok = aufKarte(welt, z, ax - 2, ay - 2, 2) && aufKarte(welt, z, ax + 2, ay + 2, 2) && { let c = zelle(welt, z, ax, ay); zelleBefahrbar(welt, c) };
        if !ok { abst += 1; continue; }
        let mut frei = true;
        let mut p = 0;
        while p < welt.NoPlayers() && frei {
            if p == z.spieler || welt.IsAlliedTo(z.spieler, p) { p += 1; continue; }
            for g in welt.PlayerBuildingEnum(p, mapGuardPost) {
                if !frei { break; }
                if !welt.IsLive(g) { continue; }
                let gl = welt.Location(g); let (gx, gy) = (edX(welt, z, gl), edY(welt, z, gl));
                if abstand2(welt, gx, gy, ax, ay) <= 9 * 9 { frei = false; }
            }
            p += 1;
        }
        if frei { z.aufmarschX = ax; z.aufmarschY = ay; break; }
        abst += 1;
    }
    z.aufmarschSeit = t;
    let warte = Rect::new(engine(welt, z, z.aufmarschX - 2, z.aufmarschY - 2), engine(welt, z, z.aufmarschX + 2, z.aufmarschY + 2));
    let schutz = Rect::new(engine(welt, z, z.aufmarschX - 5, z.aufmarschY - 5), engine(welt, z, z.aufmarschX + 5, z.aufmarschY + 5));
    wachen(welt, z.angriff, warte, &[schutz], 1);
    if z.angriff2An != 0 { wachen(welt, z.angriff2, warte, &[schutz], 1); }
    melde(welt, z, "kampf", if fz != 0 { fz } else { mapLynx }, z.aufmarschX, z.aufmarschY,
          &format!("Sammeln bei {},{} vor dem Angriff auf {},{}", z.aufmarschX, z.aufmarschY, zx, zy));
}

/// Mit EMP-Rakete: Aufmarsch knapp ausserhalb des EMP-Radius, ausserhalb der Reichweite nicht getroffener Tuerme
pub(super) fn aufmarschMitEmp(welt: &mut dyn Welt, z: &mut Zustand, t: i32) {
    let rx = (z.bx - z.empZielX) as f64; let ry = (z.by - z.empZielY) as f64;
    let d = if (rx * rx + ry * ry).sqrt() > 1.0 { (rx * rx + ry * ry).sqrt() } else { 1.0 };
    // so nah wie moeglich, aber ausserhalb der Reichweite feindlicher Tuerme, die die Rakete nicht trifft
    let mut abst = EMP_RADIUS + EMP_AUFMARSCH;
    while abst <= EMP_RADIUS + EMP_AUFMARSCH + 12 {
        z.aufmarschX = (z.empZielX as f64 + rx / d * abst as f64 + 0.5) as i32;
        z.aufmarschY = (z.empZielY as f64 + ry / d * abst as f64 + 0.5) as i32;
        let mut frei = true;
        let mut p = 0;
        while p < welt.NoPlayers() && frei {
            if p == z.spieler || welt.IsAlliedTo(z.spieler, p) { p += 1; continue; }
            for g in welt.PlayerBuildingEnum(p, mapGuardPost) {
                if !frei { break; }
                if !welt.IsLive(g) { continue; }
                let gl = welt.Location(g); let gx = edX(welt, z, gl); let gy = edY(welt, z, gl);
                let getroffen = abstand2(welt, gx, gy, z.empZielX, z.empZielY) <= EMP_RADIUS * EMP_RADIUS;
                if !getroffen && abstand2(welt, gx, gy, z.aufmarschX, z.aufmarschY) <= 9 * 9 { frei = false; }   // Turmreichweite + Rand
            }
            p += 1;
        }
        if frei { break; }
        abst += 1;
    }
    z.aufmarschSeit = t;
    // Wartezone der Angriffsgruppe = Aufmarschpunkt; bewacht wird der Aufmarschpunkt selbst
    let warte = Rect::new(engine(welt, z, z.aufmarschX - 2, z.aufmarschY - 2), engine(welt, z, z.aufmarschX + 2, z.aufmarschY + 2));
    let schutz = Rect::new(engine(welt, z, z.aufmarschX - 5, z.aufmarschY - 5), engine(welt, z, z.aufmarschX + 5, z.aufmarschY + 5));
    wachen(welt, z.angriff, warte, &[schutz], 1);
    if z.angriff2An != 0 { wachen(welt, z.angriff2, warte, &[schutz], 1); }
    melde(welt, z, "kampf", mapEMPMissile, z.aufmarschX, z.aufmarschY,
          &format!("Aufmarsch bei {},{} vor der Abwehr bei {},{} -- EMP-Rakete startet, wenn die Truppe steht",
                   z.aufmarschX, z.aufmarschY, z.empZielX, z.empZielY));
}
