//! Die C++-KI (op2_ki.hpp) nach Rust uebertragen, Funktion fuer Funktion, gleiche Namen.
//! Alle Funktionen nehmen zuerst `w: &mut dyn Welt` (die OP2-API), dann die Parameter wie in C++.
//! The C++ AI translated function by function with the same names.
#![allow(non_snake_case, non_upper_case_globals)]
pub mod api;
pub mod globale;
pub mod tb;
pub mod welt;
pub mod zustand;
pub mod hilfen;
pub mod planung;
pub mod bau;
pub mod erz;
pub mod kampf;
pub mod forschung;
pub mod vorplan;
pub mod gefecht;
pub mod gefecht_netz;
pub mod paarung;
pub mod ablauf;

// Alle Module sehen einander (wie ein C++-Namensraum).
pub use api::*;
pub use globale::*;
pub use welt::*;
pub use zustand::*;
pub use hilfen::*;
pub use planung::*;
pub use bau::*;
pub use erz::*;
pub use kampf::*;
pub use forschung::*;
pub use ablauf::*;
