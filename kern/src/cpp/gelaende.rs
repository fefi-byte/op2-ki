//! Erweiterung (nicht Teil der C++-KI): Gelaendeanalyse. Einmal je Karte aus den Zelltypen: Befahrbarkeit, Wege von der
//! Feindbasis zur eigenen Basis, Engstellen (schmale Querschnitte aller Wege) und die Paesse darin.
//! Extension: terrain analysis -- passability, routes from the enemy, chokepoints across all routes.
#![allow(non_snake_case)]
use std::collections::VecDeque;

/// Befahrbar fuer Kettenfahrzeuge (wie zelleBefahrbar der C++-KI: bebaubar, Truemmer, Roehren).
pub fn befahrbar(c: i32) -> bool { matches!(c, 0 | 2 | 3 | 4 | 5 | 7 | 21 | 22 | 26..=31) }

/// Ein Pass: zusammenhaengendes Stueck eines Querschnitts.
#[derive(Clone, Debug, Default)]
pub struct Pass {
    /// Kacheln des Querschnitts (Editor-Koordinaten).
    pub kacheln: Vec<(i32, i32)>,
    pub mitte: (i32, i32),
    /// Richtung zum Feind (Einheitsvektor), aus dem Abstandsfeld.
    pub zum_feind: (f64, f64),
}

/// Eine Engstelle: Querschnitt in Abstand `abstand` (Wegkacheln) von der eigenen Basis, durch den alle Wege des
/// Feindes muessen. `breite` = Kacheln des Querschnitts (alle Paesse zusammen).
#[derive(Clone, Debug, Default)]
pub struct Engstelle { pub abstand: i32, pub breite: i32, pub paesse: Vec<Pass> }

#[derive(Clone, Debug, Default)]
pub struct Gelaende {
    pub b: i32,
    pub h: i32,
    pub frei: Vec<bool>,
    /// Wegabstand (8 Nachbarn) von der eigenen bzw. der Feindbasis; -1 = nicht erreichbar.
    pub d_eigen: Vec<i32>,
    pub d_feind: Vec<i32>,
    /// Kuerzester Weg Feind -> eigene Basis in Kacheln.
    pub weg: i32,
    /// Breite des Querschnitts je Abstand von der eigenen Basis (nur Kacheln auf "brauchbaren" Wegen).
    pub breite: Vec<i32>,
    pub engstellen: Vec<Engstelle>,
}

const N8: [(i32, i32); 8] = [(1, 0), (-1, 0), (0, 1), (0, -1), (1, 1), (1, -1), (-1, 1), (-1, -1)];

fn bfs(frei: &[bool], b: i32, h: i32, start: &[(i32, i32)]) -> Vec<i32> {
    let mut d = vec![-1; (b * h) as usize];
    let mut q = VecDeque::new();
    for &(x, y) in start {
        if x < 0 || y < 0 || x >= b || y >= h { continue; }
        let i = (y * b + x) as usize;
        if d[i] < 0 { d[i] = 0; q.push_back((x, y)); }
    }
    while let Some((x, y)) = q.pop_front() {
        let dv = d[(y * b + x) as usize];
        for (dx, dy) in N8 {
            let (nx, ny) = (x + dx, y + dy);
            if nx < 0 || ny < 0 || nx >= b || ny >= h { continue; }
            let j = (ny * b + nx) as usize;
            if d[j] >= 0 || !frei[j] { continue; }
            // diagonal nur, wenn nicht beide Nachbarn gesperrt sind (keine Ecken schneiden)
            if dx != 0 && dy != 0 && !frei[(y * b + nx) as usize] && !frei[(ny * b + x) as usize] { continue; }
            d[j] = dv + 1;
            q.push_back((nx, ny));
        }
    }
    d
}

impl Gelaende {
    /// `zellen` zeilenweise (Editor-Koordinaten), `eigen`/`feind` = Basismitten.
    /// `umweg`: Kacheln, die ein Weg laenger sein darf als der kuerzeste, um noch zu zaehlen (z. B. 40 % des Wegs).
    /// `rmin`/`rmax`: in diesem Wegabstand von der eigenen Basis wird nach Engstellen gesucht; `max_breite`: schmaler = Engstelle.
    pub fn neu(zellen: &[i32], b: i32, h: i32, eigen: (i32, i32), feind: (i32, i32), umweg: f64, rmin: i32, rmax: i32, max_breite: i32) -> Gelaende {
        // Basismitte kann auf Gebaeuden liegen: Start auf den freien Nachbarn
        let um = |p: (i32, i32)| -> Vec<(i32, i32)> { let mut v = Vec::new(); for dy in -3..=3 { for dx in -3..=3 { v.push((p.0 + dx, p.1 + dy)); } } v };
        Self::neu_quellen(zellen, b, h, &[eigen], &[feind], &um(eigen).into_iter().chain(um(feind)).collect::<Vec<_>>(), umweg, rmin, rmax, max_breite)
    }

    /// Wie `neu`, aber mit Kachelmengen statt Basismitten (Rust, PAR zugang): `eigen` = Kolonie-Raum (eigene Gebaeude, Minen,
    /// Lastwagenwege), `feind` = feindliche Gebaeude; `offen` = Kacheln, die als befahrbar gelten (Gebaeudeflaechen der Quellen).
    /// Die Engstellen liegen dann vor dem ganzen Kolonie-Raum -- auch vor den Aussenposten.
    pub fn neu_quellen(zellen: &[i32], b: i32, h: i32, eigen: &[(i32, i32)], feind: &[(i32, i32)], offen: &[(i32, i32)], umweg: f64, rmin: i32, rmax: i32, max_breite: i32) -> Gelaende {
        let frei: Vec<bool> = zellen.iter().map(|&c| befahrbar(c)).collect();
        let mut frei2 = frei.clone();
        for &(x, y) in offen.iter().chain(eigen.iter()).chain(feind.iter()) { if x >= 0 && y >= 0 && x < b && y < h { frei2[(y * b + x) as usize] = true; } }
        let d_eigen = bfs(&frei2, b, h, eigen);
        let d_feind = bfs(&frei2, b, h, feind);
        let weg = feind.iter().filter(|&&(x, y)| x >= 0 && y >= 0 && x < b && y < h).map(|&(x, y)| d_eigen[(y * b + x) as usize]).filter(|&d| d >= 0).min().unwrap_or(-1);
        let mut g = Gelaende { b, h, frei: frei2, d_eigen, d_feind, weg, breite: Vec::new(), engstellen: Vec::new() };
        if weg <= 0 { return g; }
        let grenze = weg + (weg as f64 * umweg).ceil() as i32;
        // Querschnitte: Kacheln auf brauchbaren Wegen (d_eigen + d_feind <= grenze), nach Abstand von der eigenen Basis
        let mut schichten: Vec<Vec<(i32, i32)>> = vec![Vec::new(); weg as usize + 1];
        for y in 0..h { for x in 0..b {
            let i = (y * b + x) as usize;
            let (de, df) = (g.d_eigen[i], g.d_feind[i]);
            if de < 0 || df < 0 || de + df > grenze || de > weg { continue; }
            schichten[de as usize].push((x, y));
        } }
        g.breite = schichten.iter().map(|s| s.len() as i32).collect();
        // Engstellen: lokale Minima der Breite im Suchbereich, schmaler als max_breite
        let lo = rmin.max(1) as usize; let hi = (rmax as usize).min(weg as usize - 1);
        let mut k = lo;
        while k <= hi {
            let w = g.breite[k];
            let min_nachbar = (k.saturating_sub(3)..=(k + 3).min(weg as usize)).all(|j| g.breite[j] >= w);
            if w > 0 && w <= max_breite && min_nachbar {
                let paesse = Self::paesse(&g, &schichten[k]);
                g.engstellen.push(Engstelle { abstand: k as i32, breite: w, paesse });
                k += 4;   // naechste Engstelle erst ein Stueck weiter
            } else { k += 1; }
        }
        g
    }

    /// Querschnitt in zusammenhaengende Paesse zerlegen (8 Nachbarn), je Pass Mitte und Richtung zum Feind.
    fn paesse(g: &Gelaende, schicht: &[(i32, i32)]) -> Vec<Pass> {
        let mut rest: Vec<(i32, i32)> = schicht.to_vec();
        let mut out = Vec::new();
        while let Some(s) = rest.pop() {
            let mut teil = vec![s];
            let mut i = 0;
            while i < teil.len() {
                let (x, y) = teil[i];
                let mut j = 0;
                while j < rest.len() {
                    let (a, b2) = rest[j];
                    if (a - x).abs() <= 1 && (b2 - y).abs() <= 1 { teil.push(rest.swap_remove(j)); } else { j += 1; }
                }
                i += 1;
            }
            let n = teil.len() as f64;
            let mitte = ((teil.iter().map(|p| p.0).sum::<i32>() as f64 / n).round() as i32, (teil.iter().map(|p| p.1).sum::<i32>() as f64 / n).round() as i32);
            // Richtung zum Feind: Gefaelle von d_feind um die Mitte
            let d = |x: i32, y: i32| -> Option<i32> { if x < 0 || y < 0 || x >= g.b || y >= g.h { None } else { let v = g.d_feind[(y * g.b + x) as usize]; if v < 0 { None } else { Some(v) } } };
            let (mut ux, mut uy) = (0.0, 0.0);
            if let Some(d0) = d(mitte.0, mitte.1) {
                for (dx, dy) in N8 { if let Some(dv) = d(mitte.0 + dx * 3, mitte.1 + dy * 3) { let w = (d0 - dv) as f64; ux += w * dx as f64; uy += w * dy as f64; } }
            }
            let l = (ux * ux + uy * uy).sqrt().max(1e-9);
            out.push(Pass { kacheln: teil, mitte, zum_feind: (ux / l, uy / l) });
        }
        out.sort_by_key(|p| std::cmp::Reverse(p.kacheln.len()));
        out
    }

    /// Bild der Karte als Text: . frei, # gesperrt, Ziffer = Engstelle (Index), E/F Basen.
    pub fn bild(&self, eigen: (i32, i32), feind: (i32, i32)) -> String {
        let mut z: Vec<Vec<char>> = (0..self.h).map(|y| (0..self.b).map(|x| if self.frei[(y * self.b + x) as usize] { '.' } else { '#' }).collect()).collect();
        for (k, e) in self.engstellen.iter().enumerate() {
            for p in &e.paesse { for &(x, y) in &p.kacheln { z[y as usize][x as usize] = char::from_digit((k % 10) as u32, 10).unwrap(); } }
        }
        z[eigen.1 as usize][eigen.0 as usize] = 'E';
        z[feind.1 as usize][feind.0 as usize] = 'F';
        z.iter().map(|r| r.iter().collect::<String>()).collect::<Vec<_>>().join("\n")
    }
}
