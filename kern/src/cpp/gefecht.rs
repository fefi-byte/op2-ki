//! Erweiterung der Rust-Fassung (nicht in der C++-KI): Gefechtsfuehrung im Nahbereich mit dem gelernten Netz
//! (gefecht_netz.rs), bei jedem AIProc-Aufruf (alle 4 Ticks), solange gekaempft wird; sonst alle 20 Ticks eine Pruefung.
//! Close combat control with the learned net every AIProc while fighting.
use std::cell::RefCell;

use super::*;

thread_local! {
    /// je Spieler der Tick der letzten Gefechtsfuehrung (Gefecht laeuft)
    static LETZT: RefCell<[i32; 8]> = const { RefCell::new([0; 8]) };
}

pub fn zuruecksetzen_gefecht() { LETZT.with(|s| *s.borrow_mut() = [0; 8]); super::gefecht_netz::zuruecksetzen(); }

/// Gefechtsfuehrung eines Spielers
pub fn fuehren(welt: &mut dyn Welt, z: &Zustand) {
    let Some(n) = super::gefecht_netz::netz(welt, z.spieler) else { return };
    let t = welt.Tick();
    let sp = (z.spieler & 7) as usize;
    let letzt = LETZT.with(|s| s.borrow()[sp]);
    if t - letzt > 40 && t % 20 >= 4 { return; }
    LETZT.with(|s| s.borrow_mut()[sp] = t);
    let (r, sperre) = (PAR(z.spieler).gefechtRadius as i32, PAR(z.spieler).gefechtSperre as i32);
    super::gefecht_netz::fuehren(welt, z, &n, r, sperre);
}
