//! Bauen: Personal (Arbeiter, Stilllegen), Bauplaetze, BuildingGroup, Roehrennetz und Anschluss.
//! C++-KI op2_ki.hpp Zeilen 2266-3228, nach Rust uebertragen.
use super::*;

mod arbeit;
mod plaetze;
mod gruppe;
mod netz;
pub use arbeit::*;
pub use plaetze::*;
pub use gruppe::*;
pub use netz::*;

// ---------------------------------------------------------------- Konstanten anderer Abschnitte
// Alle Gebaeudetypen, die die KI zaehlt (PlayerBuildingEnum braucht einen Typ).
const GEBAEUDE: [i32; 38] = [ // FREMD aus Zeile 577
    mapCommandCenter, mapStructureFactory, mapVehicleFactory, mapTokamak, mapMHDGenerator, mapSolarPowerArray,
    mapCommonOreSmelter, mapRareOreSmelter, mapCommonOreMine, mapRareOreMine, mapCommonStorage, mapRareStorage,
    mapAgridome, mapResidence, mapReinforcedResidence, mapAdvancedResidence, mapBasicLab, mapStandardLab,
    mapAdvancedLab, mapMedicalCenter, mapNursery, mapUniversity, mapRecreationFacility, mapForum, mapDIRT,
    mapGarage, mapConsumerFactory, mapGuardPost, mapLightTower, mapGORF, mapRobotCommand, mapObservatory,
    mapMeteorDefense, mapTradeCenter, mapArachnidFactory, mapSpaceport, mapGeothermalPlant, mapMagmaWell,
];
const TECH_RAUMFAHRT: i32 = 5405; // FREMD aus Zeile 521
const B_ZIEL: i32 = 1; // FREMD aus Zeile 963

// ---------------------------------------------------------------- Konstanten dieses Abschnitts
/// Leerlauf: so lange untaetig, dann bei Personalmangel stilllegen.
pub const LEERLAUF_TICKS: i32 = 1500;
/// so lange darf ein aufgezeichnetes Gebaeude dauern, bevor die Spur mahnt
pub const GEDULD_GRUPPE: i32 = 6000;
/// so weit muss es vor einem Wachturm frei von Gebaeuden sein
pub const SCHUSSFELD: i32 = 6;
pub const MAX_WEG: i32 = 400;
