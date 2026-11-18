//! Planung: Kartenscan (Feindbild), Bauwahl nach Bereichen, Dringlichkeit, Rang, Rohstoff-Bilanz, Aussenposten.
//! C++-KI op2_ki.hpp Zeilen 1206-2265, nach Rust uebertragen.
use super::*;

mod scan;
mod lage;
mod aussen;
mod rang;
mod bauwahl;
mod dringlich;
pub use scan::*;
pub use lage::*;
pub use aussen::*;
pub use rang::*;
pub use bauwahl::*;
pub use dringlich::*;

// ---------------------------------------------------------------- fremde Konstanten (nur fuer die Einzelpruefung)
const SCAN_TAKT: i32 = 200; // FREMD aus Zeile 979
const WERTUNG_TICK: i32 = 200000; // FREMD aus Zeile 980
const WERT_WT: i32 = 4; // FREMD aus Zeile 981
const WERT_LYNX: i32 = 5; // FREMD aus Zeile 982
const MAX_WT: i32 = 8; // FREMD aus Zeile 983
const AUSSEN_R: i32 = 12; // FREMD aus Zeile 984
const B_KOLONIE: i32 = 0; // FREMD aus Zeile 963
const B_ZIEL: i32 = 1; // FREMD aus Zeile 963
const B_WIRTSCHAFT: i32 = 2; // FREMD aus Zeile 963
const B_VERTEIDIGUNG: i32 = 3; // FREMD aus Zeile 963
const B_ANZAHL: i32 = 4; // FREMD aus Zeile 963
const BEREICH: [&str; 4] = ["Kolonie", "Ziel", "Wirtschaft", "Verteidigung"]; // FREMD aus Zeile 964
const TECH_SELTENERZ: i32 = 5110; // FREMD aus Zeile 522
const GEBAEUDE: [i32; 38] = [ // FREMD aus Zeile 577
    mapCommandCenter, mapStructureFactory, mapVehicleFactory, mapTokamak, mapMHDGenerator, mapSolarPowerArray,
    mapCommonOreSmelter, mapRareOreSmelter, mapCommonOreMine, mapRareOreMine, mapCommonStorage, mapRareStorage,
    mapAgridome, mapResidence, mapReinforcedResidence, mapAdvancedResidence, mapBasicLab, mapStandardLab,
    mapAdvancedLab, mapMedicalCenter, mapNursery, mapUniversity, mapRecreationFacility, mapForum, mapDIRT,
    mapGarage, mapConsumerFactory, mapGuardPost, mapLightTower, mapGORF, mapRobotCommand, mapObservatory,
    mapMeteorDefense, mapTradeCenter, mapArachnidFactory, mapSpaceport, mapGeothermalPlant, mapMagmaWell,
];

// ---------------------------------------------------------------- eigene Konstanten dieses Abschnitts
/// gemessen: ~500-600 Ticks je Fahrzeug bei Dauerproduktion einer Fabrik
pub const BAU_FZ: i32 = 600;

/// snprintf(buf, n, ...): Text auf n-1 Bytes kuerzen (wie die festen char-Puffer in C++).
fn snp(s: String, n: usize) -> String {
    if n == 0 { return String::new(); }
    if s.len() < n { return s; }
    let mut k = n - 1;
    while k > 0 && !s.is_char_boundary(k) { k -= 1; }
    s[..k].to_string()
}

#[inline]
fn ix(t: i32) -> usize { t as usize }
