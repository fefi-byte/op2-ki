//! Ablauf: Hauptschritt je TAKT, Spur/Diagnose, Wissen und Parameter, Startaufstellung, Wertnetz.
//! C++-KI op2_ki.hpp Zeilen 6330-7300, nach Rust uebertragen.
use super::*;

mod diagnose;
mod aufbau;
mod wissen;
mod schritt;
pub use diagnose::*;
pub use aufbau::*;
pub use wissen::*;
pub use schritt::*;
use std::cell::RefCell;

// ---------------------------------------------------------------- fremde Konstanten (Dubletten beim Zusammenfuegen entfernen)
const WERTUNG_TICK: i32 = 200000; // FREMD aus Zeile 980
const SCAN_TAKT: i32 = 200; // FREMD aus Zeile 979
const B_ANZAHL: i32 = 4; // FREMD aus Zeile 963
// FREMD aus Zeile 577
const GEBAEUDE: &[i32] = &[
    mapCommandCenter, mapStructureFactory, mapVehicleFactory, mapTokamak, mapMHDGenerator, mapSolarPowerArray,
    mapCommonOreSmelter, mapRareOreSmelter, mapCommonOreMine, mapRareOreMine, mapCommonStorage, mapRareStorage,
    mapAgridome, mapResidence, mapReinforcedResidence, mapAdvancedResidence, mapBasicLab, mapStandardLab,
    mapAdvancedLab, mapMedicalCenter, mapNursery, mapUniversity, mapRecreationFacility, mapForum, mapDIRT,
    mapGarage, mapConsumerFactory, mapGuardPost, mapLightTower, mapGORF, mapRobotCommand, mapObservatory,
    mapMeteorDefense, mapTradeCenter, mapArachnidFactory, mapSpaceport, mapGeothermalPlant, mapMagmaWell,
];
// FEHLT: in api.rs (OP2 enums OreType / BarYield / BarVariant)
const OreTypeCommon: i32 = 0;
const Bar3: i32 = 0;
const Bar2: i32 = 1;
const VariantRandom: i32 = -1;


// ---------------------------------------------------------------- statische Variablen
thread_local! {
    /// static int altId[8][16], altX[8][16], altY[8][16], zaehler[8][16], gemeldet[8] (lkwDiagnose)
    static LKW_STATIK: RefCell<LkwStatik> = RefCell::new(LkwStatik::default());
}

#[derive(Clone, Default)]
struct LkwStatik {
    altId: [[i32; 16]; 8],
    altX: [[i32; 16]; 8],
    altY: [[i32; 16]; 8],
    zaehler: [[i32; 16]; 8],
    gemeldet: [i32; 8],
}

/// Statische Variablen dieses Moduls auf den Anfangswert.
pub fn zuruecksetzen_ablauf() {
    LKW_STATIK.with(|s| *s.borrow_mut() = LkwStatik::default());
}

// ---------------------------------------------------------------- Hilfen fuer fscanf/sscanf/fgets
/// Lesekopf ueber einen Text wie fscanf (Leerraum ueberspringen, %s / %d / %f).
struct Leser<'a> { b: &'a [u8], pos: usize }
impl<'a> Leser<'a> {
    fn neu(s: &'a str) -> Self { Leser { b: s.as_bytes(), pos: 0 } }
    fn leer(&mut self) { while self.pos < self.b.len() && self.b[self.pos].is_ascii_whitespace() { self.pos += 1; } }
}

/// Laenge der Gleitkommazahl am Anfang (wie strtod), ohne fuehrenden Leerraum.
fn strtod_laenge(b: &[u8]) -> Option<usize> {
    let mut i = 0;
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') { i += 1; }
    // inf / nan
    let rest = &b[i..];
    for w in [&b"infinity"[..], &b"inf"[..], &b"nan"[..]] {
        if rest.len() >= w.len() && rest[..w.len()].eq_ignore_ascii_case(w) { return Some(i + w.len()); }
    }
    let mut ziffern = 0;
    while i < b.len() && b[i].is_ascii_digit() { i += 1; ziffern += 1; }
    if i < b.len() && b[i] == b'.' {
        i += 1;
        while i < b.len() && b[i].is_ascii_digit() { i += 1; ziffern += 1; }
    }
    if ziffern == 0 { return None; }
    if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
        let mut j = i + 1;
        if j < b.len() && (b[j] == b'+' || b[j] == b'-') { j += 1; }
        let a = j;
        while j < b.len() && b[j].is_ascii_digit() { j += 1; }
        if j > a { i = j; }
    }
    Some(i)
}
fn parse_f(s: &str) -> f64 {
    let t = s.to_ascii_lowercase();
    let (vz, r) = if let Some(r) = t.strip_prefix('-') { (-1.0, r) } else if let Some(r) = t.strip_prefix('+') { (1.0, r) } else { (1.0, &t[..]) };
    if r.starts_with("inf") { return vz * f64::INFINITY; }
    if r.starts_with("nan") { return f64::NAN; }
    let r = if r.starts_with('.') { format!("0{}", r) } else { r.to_string() };
    let r = if r.ends_with('.') { format!("{}0", r) } else { r };
    let r = r.replace(".e", ".0e");
    vz * r.parse::<f64>().unwrap_or(0.0)
}
/// sscanf(s, "<vor>%lf", &d) == 1
fn scan_f64(s: &str, vor: &str) -> Option<f64> {
    let r = s.strip_prefix(vor)?;
    let mut le = Leser::neu(r);
    le.leer();
    let l = strtod_laenge(&le.b[le.pos..])?;
    Some(parse_f(&r[le.pos..le.pos + l]))
}
/// fgets(zeile, groesse, f) in einer Schleife: Zeilen samt '\n', hoechstens groesse-1 Bytes je Stueck.
fn fgets_zeilen(text: &str, groesse: usize) -> Vec<String> {
    let b = text.as_bytes();
    let mut out = Vec::new();
    let mut pos = 0;
    while pos < b.len() {
        let a = pos;
        while pos < b.len() && pos - a < groesse - 1 {
            pos += 1;
            if b[pos - 1] == b'\n' { break; }
        }
        out.push(String::from_utf8_lossy(&b[a..pos]).into_owned());
    }
    out
}
