//! Erweiterung (nicht Teil der C++-KI): Gelaendeanalyse aus den Zelltypen -- Befahrbarkeit und Wegabstaende von der
//! eigenen und der Feindbasis (8 Nachbarn, ohne Ecken zu schneiden).
//! Extension: terrain analysis -- passability and path distances from both bases.
#![allow(non_snake_case)]
use std::collections::VecDeque;

/// Befahrbar fuer Kettenfahrzeuge (wie zelleBefahrbar der C++-KI: bebaubar, Truemmer, Roehren).
pub fn befahrbar(c: i32) -> bool { matches!(c, 0 | 2 | 3 | 4 | 5 | 7 | 21 | 22 | 26..=31) }

#[derive(Clone, Debug, Default)]
pub struct Gelaende {
    /// Wegabstand (8 Nachbarn) von der eigenen bzw. der Feindbasis; -1 = nicht erreichbar.
    pub d_eigen: Vec<i32>,
    pub d_feind: Vec<i32>,
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
    /// `zellen` zeilenweise (Editor-Koordinaten), `eigen`/`feind` = Basismitten. Die Basismitte kann auf Gebaeuden liegen:
    /// die 7x7 Kacheln um beide Mitten gelten als befahrbar.
    pub fn neu(zellen: &[i32], b: i32, h: i32, eigen: (i32, i32), feind: (i32, i32)) -> Gelaende {
        let mut frei: Vec<bool> = zellen.iter().map(|&c| befahrbar(c)).collect();
        for m in [eigen, feind] { for dy in -3..=3 { for dx in -3..=3 {
            let (x, y) = (m.0 + dx, m.1 + dy);
            if x >= 0 && y >= 0 && x < b && y < h { frei[(y * b + x) as usize] = true; }
        } } }
        Gelaende { d_eigen: bfs(&frei, b, h, &[eigen]), d_feind: bfs(&frei, b, h, &[feind]) }
    }
}
