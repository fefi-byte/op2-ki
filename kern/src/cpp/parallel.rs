//! Erweiterung (nicht Teil der C++-KI): parallele Angriffswege. Die Angriffstruppe teilt sich
//! auf zwei Aufmarschpunkte, deren Wege zum Ziel moeglichst wenig gemeinsam haben, und schlaegt von beiden Seiten
//! gleichzeitig los -- statt alle durch denselben Gang (mp2_08: Querschnitt 18-46 Kacheln, aber viele schmale Gaenge).
//! Extension: two staging points with disjoint routes to the target; both halves attack at the same time.
use super::gelaende::befahrbar;
use super::*;
use std::collections::VecDeque;

const N8: [(i32, i32); 8] = [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)];

fn bfs(frei: &[bool], b: i32, h: i32, start: (i32, i32)) -> Vec<i32> {
    let mut d = vec![-1; (b * h) as usize];
    let mut q = VecDeque::new();
    // Start kann auf Gebaeuden liegen: die Umgebung oeffnen
    for dy in -2..=2 { for dx in -2..=2 {
        let (x, y) = (start.0 + dx, start.1 + dy);
        if x < 0 || y < 0 || x >= b || y >= h { continue; }
        let i = (y * b + x) as usize;
        if d[i] < 0 { d[i] = dx.abs().max(dy.abs()); q.push_back((x, y)); }
    } }
    while let Some((x, y)) = q.pop_front() {
        let dv = d[(y * b + x) as usize];
        for (dx, dy) in N8 {
            let (nx, ny) = (x + dx, y + dy);
            if nx < 0 || ny < 0 || nx >= b || ny >= h { continue; }
            let j = (ny * b + nx) as usize;
            if d[j] >= 0 || !frei[j] { continue; }
            if dx != 0 && dy != 0 && !frei[(y * b + nx) as usize] && !frei[(ny * b + x) as usize] { continue; }
            d[j] = dv + 1;
            q.push_back((nx, ny));
        }
    }
    d
}

/// Weg von `p` zum Ziel entlang des Abstandsfelds `d` (Gefaelle), als Kachelliste.
fn weg(d: &[i32], b: i32, h: i32, p: (i32, i32)) -> Vec<(i32, i32)> {
    let mut out = vec![p];
    let mut c = p;
    loop {
        let dc = d[(c.1 * b + c.0) as usize];
        if dc <= 0 { break; }
        let mut best = c; let mut bd = dc;
        for (dx, dy) in N8 {
            let (nx, ny) = (c.0 + dx, c.1 + dy);
            if nx < 0 || ny < 0 || nx >= b || ny >= h { continue; }
            let v = d[(ny * b + nx) as usize];
            if v >= 0 && v < bd { bd = v; best = (nx, ny); }
        }
        if best == c { break; }
        c = best; out.push(c);
        if out.len() > 4096 { break; }
    }
    out
}

/// Gangbreite eines Wegs: Freiraum je Kachel = Schachbrettabstand zum naechsten Hindernis (Rand zaehlt als Hindernis);
/// je Wegkachel der groesste Freiraum im 5x5-Umfeld (kuerzeste Wege laufen an Kanten entlang, die Gangmitte liegt daneben),
/// Breite = 2 x Freiraum - 1. Ergebnis ist das untere Viertel ueber den Weg (einzelne Hindernisse zaehlen nicht);
/// die ersten 2 und die letzten `schluss` Kacheln (Aufmarsch, Zielbasis) bleiben aussen vor.
fn engsteBreite(frei: &[bool], b: i32, h: i32, w: &[(i32, i32)], schluss: usize) -> i32 {
    let mut c = vec![-1i32; (b * h) as usize];
    let mut q = VecDeque::new();
    for y in 0..h { for x in 0..b {
        let i = (y * b + x) as usize;
        if !frei[i] { c[i] = 0; q.push_back((x, y)); }
        else if x == 0 || y == 0 || x == b - 1 || y == h - 1 { c[i] = 1; q.push_back((x, y)); }
    } }
    while let Some((x, y)) = q.pop_front() {
        let cv = c[(y * b + x) as usize];
        for (dx, dy) in N8 {
            let (nx, ny) = (x + dx, y + dy);
            if nx < 0 || ny < 0 || nx >= b || ny >= h { continue; }
            let j = (ny * b + nx) as usize;
            if c[j] < 0 { c[j] = cv + 1; q.push_back((nx, ny)); }
        }
    }
    if w.len() < schluss + 5 { return i32::MAX; }
    let mut breiten: Vec<i32> = (2..w.len() - schluss).map(|i| {
        let mut m = 0;
        for dy in -2..=2 { for dx in -2..=2 {
            let (x, y) = (w[i].0 + dx, w[i].1 + dy);
            if x >= 0 && y >= 0 && x < b && y < h { m = m.max(c[(y * b + x) as usize]); }
        } }
        2 * m - 1
    }).collect();
    breiten.sort();
    breiten[breiten.len() / 4]
}

/// Zweiter Aufmarschpunkt fuer einen Angriff auf `ziel` (Editor-Kacheln), wenn der erste `auf1` ist: etwa gleich weit
/// vom Ziel (gleichzeitige Ankunft), von daheim hoechstens PAR parallelUmweg mal so weit wie `auf1`, ausser Reichweite
/// feindlicher Tuerme, und mit einem Weg zum Ziel, der den ersten Weg (1 Kachel Rand) bis kurz vor dem Ziel moeglichst
/// meidet. None, wenn kein Punkt mindestens PAR parallelAbstand Kacheln vom ersten weg liegt.
/// Geteilt wird nur, wenn die Truppe (`n` Fahrzeuge) nicht durch den ersten Weg passt: n > schmalste Breite x
/// PAR parallelTiefe (Reihen, die gleichzeitig in Reichweite kommen; 0 = immer teilen). `breite` liefert die Messung.
pub fn zweiterAufmarsch(welt: &mut dyn Welt, z: &Zustand, ziel: (i32, i32), auf1: (i32, i32), n: i32, breite: &mut i32) -> Option<(i32, i32)> {
    let p = PAR(z.spieler);
    let (b, h) = (z.breite, z.hoehe);
    if b <= 0 || h <= 0 { return None; }
    let innen = |x: i32, y: i32| x >= 0 && y >= 0 && x < b && y < h;
    if !innen(ziel.0, ziel.1) || !innen(auf1.0, auf1.1) { return None; }
    let mut frei = vec![false; (b * h) as usize];
    for y in 0..h { for x in 0..b { let l = engine(welt, z, x, y); frei[(y * b + x) as usize] = befahrbar(welt.GetCellType(l)); } }
    let d_ziel = bfs(&frei, b, h, ziel);
    let d_heim = bfs(&frei, b, h, (z.bx, z.by));
    let ix = |x: i32, y: i32| (y * b + x) as usize;
    let dz1 = d_ziel[ix(auf1.0, auf1.1)];
    let dh1 = d_heim[ix(auf1.0, auf1.1)];
    if dz1 <= 0 || dh1 < 0 { return None; }
    // erster Weg mit 1 Kachel Rand; die letzten Kacheln vor dem Ziel zaehlen nicht (dort treffen sich alle Wege)
    let w1 = weg(&d_ziel, b, h, auf1);
    let mut belegt = vec![false; (b * h) as usize];
    for &(x, y) in &w1 { for dy in -1..=1 { for dx in -1..=1 { if innen(x + dx, y + dy) { belegt[ix(x + dx, y + dy)] = true; } } } }
    let schluss = 5;
    *breite = engsteBreite(&frei, b, h, &w1, 8);
    if p.parallelTiefe > 0.0 && n as f64 <= *breite as f64 * p.parallelTiefe { return None; }
    // feindliche Tuerme (Reichweite + Rand wie beim ersten Aufmarsch)
    let mut tuerme = Vec::new();
    for q in 0..welt.NoPlayers() {
        if q == z.spieler || welt.IsAlliedTo(z.spieler, q) { continue; }
        for g in welt.PlayerBuildingEnum(q, mapGuardPost) { if welt.IsLive(g) { let l = welt.Location(g); tuerme.push((edX(welt, z, l), edY(welt, z, l))); } }
    }
    let umweg = p.parallelUmweg.max(1.0);
    let mut best: Option<(f64, (i32, i32))> = None;
    for y in 2..h - 2 { for x in 2..b - 2 {
        let i = ix(x, y);
        let (dz, dh) = (d_ziel[i], d_heim[i]);
        if !frei[i] || dz < 0 || dh < 0 { continue; }
        if dz < dz1 - 2 || dz > dz1 + 4 { continue; }
        if dh as f64 > dh1 as f64 * umweg + 4.0 { continue; }
        if tuerme.iter().any(|&(tx, ty)| (tx - x).pow(2) + (ty - y).pow(2) <= 9 * 9) { continue; }
        let sep = (((x - auf1.0).pow(2) + (y - auf1.1).pow(2)) as f64).sqrt();
        if sep < p.parallelAbstand { continue; }
        let w2 = weg(&d_ziel, b, h, (x, y));
        let n = w2.len().saturating_sub(schluss).max(1);
        let gemeinsam = w2[..n].iter().filter(|&&(a, c)| belegt[ix(a, c)]).count() as f64 / n as f64;
        if gemeinsam > 0.35 { continue; }
        // getrennt und kurz: Trennung zaehlt, gemeinsame Strecke und Umweg kosten
        let wert = sep * (1.0 - gemeinsam) - 0.3 * (dh - dh1).max(0) as f64;
        if best.is_none_or(|bb| wert > bb.0) { best = Some((wert, (x, y))); }
    } }
    best.map(|bb| bb.1)
}

/// Angriffstruppe gleichmaessig auf beide Angriffsgruppen verteilen (angriff2 muss bestehen).
pub fn teilen(welt: &mut dyn Welt, z: &mut Zustand) -> (i32, i32) {
    let alle: Vec<Id> = AngriffEnum(welt, z).into_iter().filter(|&v| welt.IsLive(v)).collect();
    let soll2 = alle.len() as i32 / 2;
    let mut n2 = welt.GroupEnumerator(z.angriff2).into_iter().filter(|&v| welt.IsLive(v)).count() as i32;
    for v in welt.GroupEnumerator(z.angriff) {
        if n2 >= soll2 { break; }
        if !welt.IsLive(v) { continue; }
        welt.RemoveUnit(z.angriff, v); welt.TakeUnit(z.angriff2, v); n2 += 1;
    }
    (alle.len() as i32 - n2, n2)
}
