//! Globale Variablen der C++-KI (inline/static auf Namensraum-Ebene), je Thread: im Simulator laufen mehrere
//! Partien parallel, jede in einem Thread, beide Spieler nacheinander -- wie im Original ein Prozess je Partie.
//! `zuruecksetzen()` vor jeder neuen Partie.
//! Globals of the C++ AI as thread locals (one game per thread, like one process per game in the original).
#![allow(non_upper_case_globals, dead_code)]
use std::cell::{Cell, RefCell};

use super::zustand::{Anspruch, Parameter, MAX_VORHABEN};

thread_local! {
    /// inline Parameter par[8]
    pub static par: RefCell<[Parameter; 8]> = RefCell::new([Parameter::default(); 8]);
    /// inline const char* parameterText[8] (in der Mission eingebaute Stellschrauben)
    pub static parameterText: RefCell<[Option<String>; 8]> = RefCell::new(Default::default());
    /// inline Anspruch g_vorhaben[8][MAX_VORHABEN]; inline int g_nVorhaben[8], g_vorhabenTick[8]
    pub static g_vorhaben: RefCell<[[Anspruch; MAX_VORHABEN]; 8]> = RefCell::new([[Anspruch::default(); MAX_VORHABEN]; 8]);
    pub static g_nVorhaben: RefCell<[i32; 8]> = const { RefCell::new([0; 8]) };
    pub static g_vorhabenTick: RefCell<[i32; 8]> = const { RefCell::new([0; 8]) };
    /// static int g_kraftSpieler = 1
    pub static g_kraftSpieler: Cell<i32> = const { Cell::new(1) };
    /// inline int g_waffeBegonnen[8]
    pub static g_waffeBegonnen: RefCell<[i32; 8]> = const { RefCell::new([0; 8]) };
    /// inline int g_platz[8][5][2]
    pub static g_platz: RefCell<[[[i32; 2]; 5]; 8]> = const { RefCell::new([[[0; 2]; 5]; 8]) };
}

/// PAR(z): Stellschrauben des Spielers.
pub fn PAR(spieler: i32) -> Parameter { par.with(|p| p.borrow()[(spieler & 7) as usize]) }

thread_local! {
    /// Faktor je Forschung (Nutzer 2026-11-19, Schritt 2: was bringt jede Forschung wirklich): "tf_<id>=f" in den
    /// Stellschrauben, je Spieler; fehlt er, gilt 1.0. Gewichtet den Nutzen der Forschung als Ziel in der Vorhersage.
    pub static techFaktoren: std::cell::RefCell<[std::collections::HashMap<i32, f64>; 8]> = std::cell::RefCell::new(Default::default());
}
thread_local! {
    /// Forschungslinie (PAR zielTreue): zuletzt bestes Ziel je Spieler und Bereich (0 Kolonie, 1 Kampf, 2 Wirtschaft, 3 Raum)
    pub static g_linie: std::cell::RefCell<[[i32; 4]; 8]> = const { std::cell::RefCell::new([[0; 4]; 8]) };
}
pub fn linieZiel(spieler: i32, bereich: i32) -> i32 { g_linie.with(|l| l.borrow()[(spieler & 7) as usize][(bereich & 3) as usize]) }
pub fn linieSetzen(spieler: i32, bereich: i32, id: i32) { g_linie.with(|l| l.borrow_mut()[(spieler & 7) as usize][(bereich & 3) as usize] = id) }
pub fn techFaktor(spieler: i32, id: i32) -> f64 { techFaktoren.with(|t| t.borrow()[(spieler & 7) as usize].get(&id).copied().unwrap_or(1.0)) }

pub fn zuruecksetzen() {
    techFaktoren.with(|t| *t.borrow_mut() = Default::default());
    g_linie.with(|l| *l.borrow_mut() = [[0; 4]; 8]);
    super::ablauf::zuruecksetzen_ablauf();
    super::forschung::zuruecksetzen_forschung();
    super::erweiterung::zuruecksetzen_erweiterung();
    par.with(|p| *p.borrow_mut() = [Parameter::default(); 8]);
    g_vorhaben.with(|v| *v.borrow_mut() = [[Anspruch::default(); MAX_VORHABEN]; 8]);
    g_nVorhaben.with(|v| *v.borrow_mut() = [0; 8]);
    g_vorhabenTick.with(|v| *v.borrow_mut() = [0; 8]);
    g_kraftSpieler.with(|v| v.set(1));
    g_waffeBegonnen.with(|v| *v.borrow_mut() = [0; 8]);
    g_platz.with(|v| *v.borrow_mut() = [[[0; 2]; 5]; 8]);
}
