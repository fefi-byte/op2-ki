//! Angriff starten (losschlagen): Wach-Ueberschuss, Aufmarsch mit/ohne EMP, Heim- und Sammelplatz, Wellen.
use super::*;
use super::super::*;
use super::{GRUPPE_MAX, SCAN_TAKT, WERT_WT, WERT_LYNX, EMP_RADIUS, EMP_FLUG, EMP_AUFMARSCH, GEBAEUDE};

/// Lambda heim in kampfFuehrung: Warteplatz der Wache zum Feind hin.
pub(super) fn heim(welt: &mut dyn Welt, z: &mut Zustand, wx: i32, wy: i32, basis: Rect, warum: Option<&str>) {
    let mut hx = wx; let mut hy = wy;
    freierWarteplatz(welt, z, &mut hx, &mut hy, 3, true);
    platzSetzen(z.spieler, 0, hx); platzSetzen(z.spieler, 1, hy);
    // PAR wacheVorBasis: groesseres Warterechteck (30 Fahrzeuge passten nicht auf 7x7 Kacheln und liefen in die Basis)
    let r = if PAR(z.spieler).wacheVorBasis >= 0.5 { 5 } else { 3 };
    let warte = Rect::new(engine(welt, z, hx - r, hy - r), engine(welt, z, hx + r, hy + r));
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
/// Rust (PAR welle): Angriff auf hoechstens `welle` Fahrzeuge kappen, der Rest geht zurueck in die Sammelgruppe.
pub(super) fn welleKappen(welt: &mut dyn Welt, z: &mut Zustand, welle: i32) {
    let mut n = 0;
    for v in AngriffEnum(welt, z) {
        if !welt.IsLive(v) { continue; }
        n += 1;
        if n > welle && welt.TotalUnitCount(z.sammel) < GRUPPE_MAX {
            welt.RemoveUnit(z.angriff, v); if z.angriff2An != 0 { welt.RemoveUnit(z.angriff2, v); }
            welt.TakeUnit(z.sammel, v);
        }
    }
}

/// Rust (PAR welle): waehrend des Angriffs die naechste Welle aus der Sammelgruppe nachschicken, sobald vorne
/// mindestens eine halbe Welle fehlt. Die neuen Fahrzeuge bekommen ihre Befehle ueber angriffZielSetzen.
pub(super) fn welleNachschub(welt: &mut dyn Welt, z: &mut Zustand, welle: i32) {
    let mut na = 0; angriffWert(welt, z, &mut na);
    let frei = welle - na;
    if frei < (welle / 2).max(1) { return; }
    let mut n = 0;
    for v in welt.GroupEnumerator(z.sammel) {
        if n >= frei { break; }
        if !welt.IsLive(v) || !kaempfer(welt, v) { continue; }
        let ersteVoll = welt.TotalUnitCount(z.angriff) >= GRUPPE_MAX;
        if ersteVoll && welt.TotalUnitCount(z.angriff2) >= GRUPPE_MAX { break; }
        welt.RemoveUnit(z.sammel, v);
        welt.TakeUnit(if ersteVoll { z.angriff2 } else { z.angriff }, v);
        n += 1;
    }
    if n > 0 { melde(welt, z, "kampf", mapLynx, z.bx, z.by, &format!("Nachschub: naechste Welle mit {} Fahrzeugen (vorne noch {})", n, na)); }
}

/// Angriff starten: Wach-Ueberschuss abgeben, Aufmarschpunkt (mit oder ohne EMP-Rakete), Bilanz beginnen, melden.
/// Rust 2026-11-18 aufgeteilt (Verhalten gleich).
pub(super) fn losschlagen(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage, t: i32, fz: i32, ueber: i32, sw: &mut i32, ns: &mut i32, art: i32, wunsch: i32, text: &str) {
    if sicherAlle(welt, z, z.angriff, z.sammel) > 0 { sicherAlle(welt, z, z.angriff2, z.sammel); }
    let welle = if kampfModellNeu() { PAR(z.spieler).welle as i32 } else { 0 };
    if welle > 0 { welleKappen(welt, z, welle); }
    let (bewegt, nb) = wacheAbgeben(welt, z, fz, ueber, welle);
    *sw += bewegt; *ns += nb;
    if PAR(z.spieler).marschHeil > 0.0 { let (w, n) = beschaedigteDaheim(welt, z); *sw -= w; *ns -= n; }
    if welle > 0 { let mut na = 0; *sw = angriffWert(welt, z, &mut na); *ns = na; }
    if *ns <= 0 { z.angriffWunsch = 0; return; }        // nichts zum Losschicken
    z.angriffZiel = 0; z.angriffWunsch = wunsch; z.angriffArt = art;
    // Mit EMP-Rakete: erst Aufmarsch knapp ausserhalb des EMP-Radius (Modus 3), dann Start (Modus 4).
    let mut mitEmp = false;
    if !empBereit(welt, z) || super::super::befehl::emp(z) == Some(false) { aufmarschOhneEmp(welt, z, t, fz, art, wunsch, *ns); mitEmp = true; }
    else if empBereit(welt, z) { aufmarschMitEmp(welt, z, t); mitEmp = true; }
    else { angriffZielSetzen(welt, z, z.bx, z.by); }
    z.angriffStart = *sw; z.kampfModus = if mitEmp { 3 } else { 2 }; z.kampfModusSeit = t; z.letzterAngriffStart = t;
    z.angriffZerstoertStart = z.zerstoertWert; z.angriffVerlorenStart = z.verlorenWert;
    hinweis(welt, z, "ki::kampfFuehrung/bilanzStart"); bilanzStart(welt, z);
    let (mut nz, mut nv) = (0.0f32, 0.0f32);
    if netzAngriff(welt, z, l, art, *ns, *sw, &mut nz, &mut nv) {
        melde(welt, z, "kampf", if fz != 0 { fz } else { mapLynx }, z.feindX, z.feindY,
              &format!("{} ({} Fahrzeuge, Wert {}) [Netz: zerstoert ~{:.0}, verloren ~{:.0}]", text, *ns, *sw, nz, nv));
    } else {
        melde(welt, z, "kampf", if fz != 0 { fz } else { mapLynx }, z.feindX, z.feindY, &format!("{} ({} Fahrzeuge, Wert {})", text, *ns, *sw));
    }
}

/// Feldherr (PAR marschHeil): beschaedigte Fahrzeuge bleiben daheim in der Wache (Garage und Reparatur holen sie dort),
/// statt die Truppe zu bremsen. Liefert (Wert, Anzahl) der Zurueckbehaltenen.
fn beschaedigteDaheim(welt: &mut dyn Welt, z: &mut Zustand) -> (i32, i32) {
    let grenze = PAR(z.spieler).marschHeil;
    let (mut w, mut n) = (0, 0);
    for v in AngriffEnum(welt, z) {
        if !welt.IsLive(v) || !welt.IsVehicle(v) { continue; }
        let hp = welt.GetHitPoints(welt.GetType(v), z.spieler).max(1);
        if (welt.GetDamage(v) as f64) <= grenze * hp as f64 { continue; }
        let kw = kampfWert(welt, v);
        let g = if inGruppeG(welt, z.angriff, v) { z.angriff } else { z.angriff2 };
        welt.RemoveUnit(g, v);
        let wache = z.wache;
        sicherNehmen(welt, z, wache, v);
        w += kw; n += 1;
    }
    if n > 0 { melde(welt, z, "kampf", mapLynx, z.bx, z.by, &format!("Feldherr: {} beschaedigte Fahrzeuge (Wert {}) bleiben daheim -- sie wuerden die Truppe bremsen", n, w)); }
    (w, n)
}

/// Ueberschuss der Wache dazu: fertige, ruhige Kaempfer, bis der Wert erreicht ist. Liefert (Wert, Anzahl).
pub(super) fn wacheAbgeben(welt: &mut dyn Welt, z: &mut Zustand, fz: i32, ueber: i32, welle: i32) -> (i32, i32) {
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
            if welle > 0 { let mut na = 0; angriffWert(welt, z, &mut na); if na >= welle { break; } }
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

/// Sammeln vor jedem Angriff: auf der Linie zur eigenen Basis, ausserhalb der Reichweite aller Tuerme (Truppplan: auf der
/// Route); mit PAR parallel ein zweiter Aufmarschpunkt ueber einen getrennten Weg
#[allow(clippy::too_many_arguments)]
pub(super) fn aufmarschOhneEmp(welt: &mut dyn Welt, z: &mut Zustand, t: i32, fz: i32, art: i32, wunsch: i32, ns: i32) {
    let mut zx = z.feindX; let mut zy = z.feindY;
    { let w: Id = wunsch; if wunsch != 0 && welt.IsLive(w) { let l0 = welt.Location(w); zx = edX(welt, z, l0); zy = edY(welt, z, l0); } }
    let rx = (z.bx - zx) as f64; let ry = (z.by - zy) as f64;
    let d = if (rx * rx + ry * ry).sqrt() > 1.0 { (rx * rx + ry * ry).sqrt() } else { 1.0 };
    z.aufmarschX = z.bx; z.aufmarschY = z.by;
    // Feldherr (PAR aufmarschFeind): feindliche Kampffahrzeuge, die der Aufmarschpunkt meiden soll
    let rf = PAR(z.spieler).aufmarschFeind as i32;
    let mut feindFz: Vec<(i32, i32)> = Vec::new();
    if rf > 0 {
        for p in 0..welt.NoPlayers() {
            if p == z.spieler || welt.IsAlliedTo(z.spieler, p) { continue; }
            for u in welt.PlayerVehicleEnum(p) {
                if !welt.IsLive(u) || kampfWert(welt, u) <= 0 { continue; }
                let lo = welt.Location(u); feindFz.push((edX(welt, z, lo), edY(welt, z, lo)));
            }
        }
    }
    let grenze = if rf > 0 { 60 } else { 40 };
    let mut abst = 10;
    while abst <= grenze && (abst as f64) < d {
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
        if frei && rf > 0 && feindFz.iter().any(|f| abstand2(welt, f.0, f.1, ax, ay) <= rf * rf) { frei = false; }
        if frei { z.aufmarschX = ax; z.aufmarschY = ay; break; }
        abst += 1;
    }
    if art == 2 && z.planAn != 0 && z.planAufX != 0 { z.aufmarschX = z.planAufX; z.aufmarschY = z.planAufY; }
    z.aufmarschSeit = t;
    super::marsch::zuruecksetzen(z);
    let warte = Rect::new(engine(welt, z, z.aufmarschX - 2, z.aufmarschY - 2), engine(welt, z, z.aufmarschX + 2, z.aufmarschY + 2));
    let schutz = Rect::new(engine(welt, z, z.aufmarschX - 5, z.aufmarschY - 5), engine(welt, z, z.aufmarschX + 5, z.aufmarschY + 5));
    // Rust (PAR parallel): zweite Haelfte ueber einen getrennten Weg, eigener Aufmarschpunkt
    z.aufmarsch2X = 0; z.aufmarsch2Y = 0;
    let mut zweiter: Option<(i32, i32)> = None;
    if kampfModellNeu() && (PAR(z.spieler).parallel >= 0.5 || super::super::befehl::parallel(z)) && z.angriff2An != 0 && ns >= 4 {
        let mut breite = 0;
        zweiter = super::parallel::zweiterAufmarsch(welt, z, (zx, zy), (z.aufmarschX, z.aufmarschY), ns, &mut breite);
        if zweiter.is_none() {
            melde(welt, z, "kampf", mapLynx, z.aufmarschX, z.aufmarschY, &format!("Ein Weg: {} Fahrzeuge, schmalste Stelle {} Kacheln", ns, breite));
        } else { z.parallelBreite = breite; }
    }
    if let Some((ax, ay)) = zweiter {
        let (n1, n2) = super::parallel::teilen(welt, z);
        z.aufmarsch2X = ax; z.aufmarsch2Y = ay;
        let warte2 = Rect::new(engine(welt, z, ax - 2, ay - 2), engine(welt, z, ax + 2, ay + 2));
        let schutz2 = Rect::new(engine(welt, z, ax - 5, ay - 5), engine(welt, z, ax + 5, ay + 5));
        wachen(welt, z.angriff, warte, &[schutz], 1);
        wachen(welt, z.angriff2, warte2, &[schutz2], 1);
        melde(welt, z, "kampf", if fz != 0 { fz } else { mapLynx }, ax, ay,
              &format!("Zwei Wege: {} Fahrzeuge sammeln bei {},{}, {} bei {},{} vor dem Angriff auf {},{} (schmalste Stelle des ersten Wegs {} Kacheln)", n1, z.aufmarschX, z.aufmarschY, n2, ax, ay, zx, zy, z.parallelBreite));
    } else {
    wachen(welt, z.angriff, warte, &[schutz], 1);
    if z.angriff2An != 0 { wachen(welt, z.angriff2, warte, &[schutz], 1); }
    melde(welt, z, "kampf", if fz != 0 { fz } else { mapLynx }, z.aufmarschX, z.aufmarschY,
          &format!("Sammeln bei {},{} vor dem Angriff auf {},{}", z.aufmarschX, z.aufmarschY, zx, zy));
    }
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
