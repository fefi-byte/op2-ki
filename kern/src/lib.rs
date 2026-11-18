//! OP2-KI: eine Outpost-2-KI in Rust (aus der C++-KI op2_ki.hpp uebertragen). Sie sieht das Spiel nur ueber den
//! Welt-Trait (cpp::welt) und laeuft als op2ki.dll im Original, aufgerufen aus der Missions-DLL.
//! An Outpost 2 AI in Rust; it sees the game only through the Welt trait and runs as op2ki.dll inside the original.
pub mod cpp;

/// Version der C-Schnittstelle der DLL.
pub const ABI: i32 = 1;
