//! Ablauf: Hauptschritt je TAKT, Spur/Diagnose, Wissen und Parameter, Startaufstellung, Wertnetz.
//! C++-KI op2_ki.hpp Zeilen 6330-7300, uebertragen nach Rust (siehe UEBERSETZUNG.md).
use super::*;

mod wertnetz;
mod diagnose;
mod aufbau;
mod wissen;
mod schritt;
pub use wertnetz::*;
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

/// Nur im Messbuild (C++ -DKI_MESSUNG).
const KI_MESSUNG: bool = false;

// ---------------------------------------------------------------- globale Netze und statische Variablen
thread_local! {
    /// netz::wert
    pub static NETZ_WERT: RefCell<Netz> = RefCell::new(Netz::default());
    /// netz::angriff
    pub static NETZ_ANGRIFF: RefCell<Netz> = RefCell::new(Netz::default());
    /// static int altId[8][16], altX[8][16], altY[8][16], zaehler[8][16], gemeldet[8] (lkwDiagnose)
    static LKW_STATIK: RefCell<LkwStatik> = RefCell::new(LkwStatik::default());
    /// static int id (messung)
    static MESSUNG_ID: RefCell<i32> = const { RefCell::new(0) };
}

#[derive(Clone, Default)]
struct LkwStatik {
    altId: [[i32; 16]; 8],
    altX: [[i32; 16]; 8],
    altY: [[i32; 16]; 8],
    zaehler: [[i32; 16]; 8],
    gemeldet: [i32; 8],
}

/// Statische Variablen dieses Moduls (und die Netze) auf den Anfangswert.
pub fn zuruecksetzen_ablauf() {
    NETZ_WERT.with(|n| *n.borrow_mut() = Netz::default());
    NETZ_ANGRIFF.with(|n| *n.borrow_mut() = Netz::default());
    LKW_STATIK.with(|s| *s.borrow_mut() = LkwStatik::default());
    MESSUNG_ID.with(|s| *s.borrow_mut() = 0);
}

// ---------------------------------------------------------------- Hilfen fuer fscanf/sscanf/fgets
/// Lesekopf ueber einen Text wie fscanf (Leerraum ueberspringen, %s / %d / %f).
struct Leser<'a> { b: &'a [u8], pos: usize }
impl<'a> Leser<'a> {
    fn neu(s: &'a str) -> Self { Leser { b: s.as_bytes(), pos: 0 } }
    fn leer(&mut self) { while self.pos < self.b.len() && self.b[self.pos].is_ascii_whitespace() { self.pos += 1; } }
    /// %<max>s
    fn wort(&mut self, max: usize) -> Option<String> {
        self.leer();
        let a = self.pos;
        while self.pos < self.b.len() && !self.b[self.pos].is_ascii_whitespace() && self.pos - a < max { self.pos += 1; }
        if self.pos == a { return None; }
        Some(String::from_utf8_lossy(&self.b[a..self.pos]).into_owned())
    }
    /// %d
    fn ganz(&mut self) -> Option<i32> {
        self.leer();
        let (w, l) = strtol_anfang(&self.b[self.pos..])?;
        self.pos += l;
        Some(w)
    }
    /// %f
    fn zahl(&mut self) -> Option<f32> {
        self.leer();
        let l = strtod_laenge(&self.b[self.pos..])?;
        let s = std::str::from_utf8(&self.b[self.pos..self.pos + l]).ok()?;
        self.pos += l;
        Some(parse_f(s) as f32)
    }
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
/// %d / strtol am Anfang (ohne Leerraum): Wert und Laenge.
fn strtol_anfang(b: &[u8]) -> Option<(i32, usize)> {
    let mut i = 0;
    let mut neg = false;
    if i < b.len() && (b[i] == b'+' || b[i] == b'-') { neg = b[i] == b'-'; i += 1; }
    let a = i;
    let mut w: i64 = 0;
    while i < b.len() && b[i].is_ascii_digit() { w = (w * 10 + (b[i] - b'0') as i64).min(1 << 40); i += 1; }
    if i == a { return None; }
    Some(((if neg { -w } else { w }) as i32, i))
}
/// sscanf(s, "<vor>%lf", &d) == 1
fn scan_f64(s: &str, vor: &str) -> Option<f64> {
    let r = s.strip_prefix(vor)?;
    let mut le = Leser::neu(r);
    le.leer();
    let l = strtod_laenge(&le.b[le.pos..])?;
    Some(parse_f(&r[le.pos..le.pos + l]))
}
/// sscanf(s, "<vor>%d", &i) == 1
fn scan_i32(s: &str, vor: &str) -> Option<i32> {
    let r = s.strip_prefix(vor)?;
    let mut le = Leser::neu(r);
    le.ganz()
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
