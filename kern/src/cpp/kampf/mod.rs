//! Kampf: Gruppen und Sollzahlen (fuehrung), Angriff starten, laufender Angriff, Zermuerben (Hand/Netz), Bilanz, Reserve, Kamera.
//! C++-KI op2_ki.hpp Zeilen 3950-4989, uebertragen nach Rust (siehe UEBERSETZUNG.md).
use super::*;

mod bilanz;
mod lage;
mod ziel;
mod reserve;
mod start;
mod kamera;
pub use bilanz::*;
pub use lage::*;
pub use ziel::*;
pub use reserve::*;
pub use start::*;
pub use kamera::*;

mod fuehrung;
mod zermuerben;
mod marsch;

const GRUPPE_MAX: i32 = 32; // FREMD aus Zeile 3934
const SCAN_TAKT: i32 = 200; // FREMD aus Zeile 979
const WERT_WT: i32 = 4; // FREMD aus Zeile 981
const WERT_LYNX: i32 = 5; // FREMD aus Zeile 982
const EMP_RADIUS: i32 = 6; // FREMD aus Zeile 1037
const EMP_FLUG: i32 = 300; // FREMD aus Zeile 1037
const EMP_AUFMARSCH: i32 = 4; // FREMD aus Zeile 1037
// FREMD aus Zeile 577
const GEBAEUDE: [i32; 38] = [
    mapCommandCenter, mapStructureFactory, mapVehicleFactory, mapTokamak, mapMHDGenerator, mapSolarPowerArray,
    mapCommonOreSmelter, mapRareOreSmelter, mapCommonOreMine, mapRareOreMine, mapCommonStorage, mapRareStorage,
    mapAgridome, mapResidence, mapReinforcedResidence, mapAdvancedResidence, mapBasicLab, mapStandardLab,
    mapAdvancedLab, mapMedicalCenter, mapNursery, mapUniversity, mapRecreationFacility, mapForum, mapDIRT,
    mapGarage, mapConsumerFactory, mapGuardPost, mapLightTower, mapGORF, mapRobotCommand, mapObservatory,
    mapMeteorDefense, mapTradeCenter, mapArachnidFactory, mapSpaceport, mapGeothermalPlant, mapMagmaWell,
];

/// Zielstufen von angriffZielWaehlen (C++: static const int STUFE[][4] in der Funktion).
pub const STUFE: [[i32; 4]; 4] = [
    [mapCommandCenter, mapNone, mapNone, mapNone],
    [mapStructureFactory, mapNone, mapNone, mapNone],
    [mapTokamak, mapMHDGenerator, mapSolarPowerArray, mapGeothermalPlant],
    [mapVehicleFactory, mapArachnidFactory, mapNone, mapNone],
];
/// Namen der Zielstufen (C++: static const char* const ART[] in angriffZielSetzen).
pub const ART: [&str; 5] = ["Kommandozentrale", "Strukturfabrik", "Kraftwerk", "Fahrzeugfabrik", "sonstiges Gebaeude"];

/// snprintf(text, 160, ...): hoechstens 159 Zeichen.
fn text160(s: String) -> String {
    if s.len() <= 159 { return s; }
    let mut n = 159;
    while !s.is_char_boundary(n) { n -= 1; }
    s[..n].to_string()
}

fn platzLesen(spieler: i32, k: usize) -> i32 { g_platz.with(|g| g.borrow()[(spieler & 7) as usize][k / 2][k % 2]) }
fn platzSetzen(spieler: i32, k: usize, v: i32) { g_platz.with(|g| g.borrow_mut()[(spieler & 7) as usize][k / 2][k % 2] = v) }

/// Alle Fahrzeuge des Angriffs, ueber beide Gruppen (C++ struct AngriffEnum).
pub fn AngriffEnum(welt: &mut dyn Welt, z: &Zustand) -> Vec<Id> {
    let mut v = welt.GroupEnumerator(z.angriff);
    if z.angriff2An != 0 { v.extend(welt.GroupEnumerator(z.angriff2)); }
    v
}

/// C++: int angriffWert(Zustand& z, int& n)  (op2_ki.hpp:3959)
pub fn angriffWert(welt: &mut dyn Welt, z: &mut Zustand, n: &mut i32) -> i32 {
    let mut n1 = 0; let mut n2 = 0;
    let w = gruppenWert(welt, z.angriff, &mut n1) + if z.angriff2An != 0 { gruppenWert(welt, z.angriff2, &mut n2) } else { 0 };
    *n = n1 + n2;
    w
}

/// C++: void angriffRaus(Zustand& z, Unit u)  (op2_ki.hpp:3967)
pub fn angriffRaus(welt: &mut dyn Welt, z: &mut Zustand, u: Id) {
    if inGruppeG(welt, z.angriff, u) { welt.RemoveUnit(z.angriff, u); }
    else if z.angriff2An != 0 && inGruppeG(welt, z.angriff2, u) { welt.RemoveUnit(z.angriff2, u); }
}

/// C++: void wachen(FightGroup& g, MAP_RECT warte, const MAP_RECT* bewacht, int n)  (op2_ki.hpp:3972)
pub fn wachen(welt: &mut dyn Welt, g: Gruppe, warte: Rect, bewacht: &[Rect], n: i32) {
    welt.DoGuardRect(g);
    welt.SetRect(g, warte);
    for i in 0..n as usize { let r = bewacht[i]; welt.AddGuardedRect(g, r); }
}

/// C++: void kampfFuehrung(Zustand& z, const Lage& l)  (op2_ki.hpp:4458)
pub fn kampfFuehrung(welt: &mut dyn Welt, z: &mut Zustand, l: &Lage) {
    let t = welt.Tick();
    if t % SCAN_TAKT >= TAKT { return; }                    // einmal je SCAN_TAKT
    super::modus::drohungMerken(z, t);
    // Missionsmodus: leichtere Stufen entscheiden seltener (schwer: jeder Durchgang)
    if !super::modus::kampfTakt(welt, z, t, SCAN_TAKT) { return; }
    // Selbstheilung: ist der letzte Durchlauf abgestuerzt, alle Gruppen neu anlegen
    if z.kampfLaeuft != 0 {
        z.wacheAn = 0; z.sammelAn = 0; z.angriffAn = 0; z.fabrikAn = 0; z.kampfModus = 0; z.wacheAnzahl = 0;
        z.angriff2An = 0; z.reserveAn = 0; z.reserveZiel = 0; z.reserveOrt = -2; z.reserveSeit = 0;
        for i in 0..6 { z.aussenAn[i] = 0; }
        melde(welt, z, "diagnose", mapLynx, z.bx, z.by, "Kampffuehrung war abgestuerzt -- alle Kampfgruppen werden neu angelegt");
    }
    // KampfLaeuft (RAII): 1 beim Eintritt, 0 auf jedem Rueckweg
    z.kampfLaeuft = 1;
    fuehrung::kampfFuehrungInnen(welt, z, l, t);
    z.kampfLaeuft = 0;
}

/// Rust (PAR schwachstelle): was der Verlust eines Gebaeudes kostet -- Erz + 2 x Seltenerz; Zentrale und Strukturfabrik
/// doppelt (Siegbedingung); Minen und Schmelzen zusaetzlich ihr Einkommen ueber den Planungshorizont (~1400 Erz je 1000
/// Ticks je Route), Schmelzen dazu ihren Anteil am Erzlager (die Schmelze IST das Lager: faellt sie, ist ihr Vorrat weg).
pub fn schwachZielWert(welt: &mut dyn Welt, typ: i32, p: i32) -> i32 {
    let basis = (welt.GetOreCost(typ, p) + 2 * welt.GetRareOreCost(typ, p)).max(100) as f64;
    let einkommen = 1400.0 * PAR(g_kraftSpieler.with(|c| c.get())).horizont / 1000.0;
    let w = match typ {
        mapCommandCenter | mapStructureFactory => basis * 2.0,
        mapCommonOreMine | mapRareOreMine => basis + einkommen,
        mapCommonOreSmelter | mapRareOreSmelter => {
            let n = welt.PlayerBuildingEnum(p, typ).len().max(1) as f64;
            let lager = if typ == mapCommonOreSmelter { welt.Ore(p) } else { welt.RareOre(p) } as f64;
            basis + lager / n + einkommen
        }
        mapTokamak => basis * 1.5,
        _ => basis,
    };
    w as i32
}
