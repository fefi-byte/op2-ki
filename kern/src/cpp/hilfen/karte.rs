//! Karte und Koordinaten: engine/edX/edY, Zellen, Abstand, Technik vorhanden, Bausatz moeglich.
use super::*;
use super::super::*;

/// C++: LOCATION engine(const Zustand& z, int x, int y)  (op2_ki.hpp:471)
pub fn engine(welt: &mut dyn Welt, z: &Zustand, x: i32, y: i32) -> Loc { Loc::new(x + z.ux, y + z.uy) }

/// C++: int edX(const Zustand& z, const LOCATION& l)  (op2_ki.hpp:472)
pub fn edX(welt: &mut dyn Welt, z: &Zustand, l: Loc) -> i32 { l.x - z.ux }

/// C++: int edY(const Zustand& z, const LOCATION& l)  (op2_ki.hpp:473)
pub fn edY(welt: &mut dyn Welt, z: &Zustand, l: Loc) -> i32 { l.y - z.uy }

/// C++: bool aufKarte(const Zustand& z, int x, int y, int rand = 2)  (op2_ki.hpp:474)
pub fn aufKarte(welt: &mut dyn Welt, z: &Zustand, x: i32, y: i32, rand: i32) -> bool {
    x >= rand && y >= rand && x < z.breite - rand && y < z.hoehe - rand
}

/// C++: int zelle(const Zustand& z, int x, int y)  (op2_ki.hpp:477)
pub fn zelle(welt: &mut dyn Welt, z: &Zustand, x: i32, y: i32) -> i32 { let l = engine(welt, z, x, y); welt.GetCellType(l) }

/// C++: int abstand2(int ax, int ay, int bx, int by)  (op2_ki.hpp:478)
pub fn abstand2(welt: &mut dyn Welt, ax: i32, ay: i32, bx: i32, by: i32) -> i32 { (ax - bx) * (ax - bx) + (ay - by) * (ay - by) }

// Forschungsstand: HasTechnology mit unbekannter Nummer stuerzt im Spiel ab -- darum erst techIndex pruefen.
/// C++: bool hatTech(const Zustand& z, int techID)  (op2_ki.hpp:485)
pub fn hatTech(welt: &mut dyn Welt, z: &Zustand, techID: i32) -> bool {
    techID > 0 && techIndex(welt, techID) >= 0 && super::forschung::hatTechnologie(welt, z.spieler, techID)
}

// Technik je Gebaeude, Spalte "Research Topic" aus sheets/building.txt (BuildingGroup prueft sie nicht).
/// C++: int techFuer(int typ)  (op2_ki.hpp:490)
pub fn techFuer(welt: &mut dyn Welt, typ: i32) -> i32 {
    match typ {
        mapCommandCenter => 2103, mapTokamak => 2112,
        mapStructureFactory => 2104, mapCommonOreSmelter => 2110,
        mapCommonStorage => 2111, mapAgridome => 2101,
        mapResidence => 2109, mapBasicLab => 2106,
        mapLightTower => 2107, mapCommonOreMine => 2108,
        mapStandardLab => 3101, mapAdvancedLab => 5104,
        mapVehicleFactory => 3502, mapRobotCommand => 3501,
        mapUniversity => 3305, mapNursery => 3304,
        mapMedicalCenter => 3303, mapRecreationFacility => 3306,
        mapDIRT => 3301, mapGORF => 3302,
        mapAdvancedResidence => 8104, mapReinforcedResidence => 8105,
        mapRareOreSmelter => 5501, mapRareStorage => 5502,
        mapRareOreMine => 5110, mapSpaceport => 5405,
        mapGarage => 5102, mapConsumerFactory => 5101,
        mapForum => 5108, mapGeothermalPlant => 5115,
        mapMHDGenerator => 5107, mapGuardPost => 3801,
        _ => 0,
    }
}

// Kann die Strukturfabrik den Bausatz herstellen (Technik erforscht)?
/// C++: bool kitMoeglich(const Zustand& z, int typ)  (op2_ki.hpp:515)
pub fn kitMoeglich(welt: &mut dyn Welt, z: &Zustand, typ: i32) -> bool {
    let topic = techFuer(welt, typ);
    topic <= 0 || hatTech(welt, z, topic)
}
