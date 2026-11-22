//! Rohstoffe: Erzbaken, Minen, Erzrouten mit Lastwagen, Seltenerz, Fahrzeugproduktion.
//! C++-KI op2_ki.hpp Zeilen 3229-3949, uebertragen nach Rust (siehe UEBERSETZUNG.md).
use super::*;

mod baken;
mod routen;
mod selten;
mod fahrzeuge;
pub use baken::*;
pub use routen::*;
pub use selten::*;
pub use fahrzeuge::*;
use std::cell::Cell;

const TECH_SELTENERZ: i32 = 5110; // FREMD aus Zeile 522
const WERT_LYNX: i32 = 5; // FREMD aus Zeile 982
// FREMD aus Zeile 577
const GEBAEUDE: &[i32] = &[
    mapCommandCenter, mapStructureFactory, mapVehicleFactory, mapTokamak, mapMHDGenerator, mapSolarPowerArray,
    mapCommonOreSmelter, mapRareOreSmelter, mapCommonOreMine, mapRareOreMine, mapCommonStorage, mapRareStorage,
    mapAgridome, mapResidence, mapReinforcedResidence, mapAdvancedResidence, mapBasicLab, mapStandardLab,
    mapAdvancedLab, mapMedicalCenter, mapNursery, mapUniversity, mapRecreationFacility, mapForum, mapDIRT,
    mapGarage, mapConsumerFactory, mapGuardPost, mapLightTower, mapGORF, mapRobotCommand, mapObservatory,
    mapMeteorDefense, mapTradeCenter, mapArachnidFactory, mapSpaceport, mapGeothermalPlant, mapMagmaWell,
];

/// Eine Gruppe fasst hoechstens 32 Einheiten (C++: constexpr int GRUPPE_MAX).
pub const GRUPPE_MAX: i32 = 32;

#[inline]
fn ix(t: i32) -> usize { t as usize }

/// Editor-Koordinaten einer Engine-Location (edX, edY), vorab berechnet (Borrow von welt).
fn exy(welt: &mut dyn Welt, z: &Zustand, l: Loc) -> (i32, i32) {
    let x = edX(welt, z, l);
    let y = edY(welt, z, l);
    (x, y)
}

// C++ `static char t[140]` in fahrzeugWahl: der Text wird bei jedem Aufruf ueberschrieben, der Zeiger bleibt gueltig.
// Nachbildung: ein einmal je Thread angelegter Puffer von 140 Byte (snprintf kuerzt auf 139 Zeichen).
thread_local! {
    static FZ_TEXT: Cell<*mut u8> = const { Cell::new(std::ptr::null_mut()) };
}
fn statischerText(s: &str) -> &'static str {
    FZ_TEXT.with(|c| {
        let mut p = c.get();
        if p.is_null() {
            p = Box::leak(Box::new([0u8; 140])).as_mut_ptr();
            c.set(p);
        }
        let n = s.len().min(139);
        // Text ist reines ASCII (Zahlen und feste Woerter), Kuerzen trennt kein Zeichen.
        unsafe {
            std::ptr::copy_nonoverlapping(s.as_ptr(), p, n);
            std::str::from_utf8_unchecked(std::slice::from_raw_parts(p, n))
        }
    })
}
