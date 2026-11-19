//! Mauern: aufgezeichnete, noch nicht gebaute Mauerkacheln merken. Die BuildingGroup baut eine aufgezeichnete Mauer erst
//! spaeter; ohne Merkliste plante die KI dort einen Wachturm, und der Bau raeumte die Mauer wieder ab.
//! Walls: remember recorded walls so nothing else is planned on them before they are built.
use std::cell::RefCell;

use super::*;

thread_local! {
    /// je Spieler: aufgezeichnete Mauerkacheln (x, y, Tick)
    static GEPLANT: RefCell<[Vec<(i32, i32, i32)>; 8]> = RefCell::new(Default::default());
}
pub fn zuruecksetzen_mauer() { GEPLANT.with(|g| *g.borrow_mut() = Default::default()); }

fn sp(z: &Zustand) -> usize { (z.spieler.max(0) as usize).min(7) }

/// Liegt eine aufgezeichnete Mauerkachel im Rechteck?
pub fn geplantIn(z: &Zustand, x0: i32, y0: i32, w: i32, h: i32) -> bool {
    GEPLANT.with(|g| g.borrow()[sp(z)].iter().any(|&(x, y, _)| x >= x0 && x < x0 + w && y >= y0 && y < y0 + h))
}

/// Mauer aufzeichnen und merken. Nach 30000 Ticks vergessen, wenn dort nie eine Mauer stand oder sie zerstoert ist
/// (Kachel wieder bebaubar).
pub fn aufzeichnen(welt: &mut dyn Welt, z: &mut Zustand, x: i32, y: i32) {
    let t = welt.Tick();
    let alt: Vec<(i32, i32, i32)> = GEPLANT.with(|g| g.borrow()[sp(z)].clone());
    let behalten: Vec<(i32, i32, i32)> = alt.into_iter().filter(|&(ax, ay, at)| {
        t - at < 30000 || { let c = zelle(welt, z, ax, ay); !zelleBebaubar(welt, c) }
    }).collect();
    GEPLANT.with(|g| g.borrow_mut()[sp(z)] = behalten);
    if geplantIn(z, x, y, 1, 1) { return; }
    let l = engine(welt, z, x, y);
    welt.RecordWall(z.bau, l, mapWall);
    GEPLANT.with(|g| g.borrow_mut()[sp(z)].push((x, y, t)));
}
