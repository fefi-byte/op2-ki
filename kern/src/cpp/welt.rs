//! Die Spielschnittstelle der C++-KI, 1:1 nachgebildet: jede Methode entspricht einem Aufruf der OP2-API
//! (Unit/UnitEx, UnitInfo, Player/ExtPlayer, TethysGame, GameMap, Research/TechInfo, Enumeratoren, Gruppen).
//! Koordinaten sind Engine-Koordinaten (LOCATION), wie in der C++-KI vor edX/edY.
//! Zwei Umsetzungen: op2-sim (nativ) und das Original (Funktionstabelle aus der Missions-DLL).
//! The game interface the C++ AI uses, one method per OP2 API call; engine coordinates.
#![allow(non_snake_case)]

/// Einheitennummer (unitID); 0 = keine Einheit (UnitEx() in C++).
pub type Id = i32;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Loc { pub x: i32, pub y: i32 }
impl Loc { pub const fn new(x: i32, y: i32) -> Loc { Loc { x, y } } }

/// MAP_RECT(LOCATION a, LOCATION b), inklusive.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect { pub x1: i32, pub y1: i32, pub x2: i32, pub y2: i32 }
impl Rect { pub const fn new(a: Loc, b: Loc) -> Rect { Rect { x1: a.x, y1: a.y, x2: b.x, y2: b.y } } }

/// Gruppen-Stub (FightGroup, BuildingGroup, MiningGroup); 0 = nicht angelegt.
pub type Gruppe = i32;

/// Vorausgesetzte Techniken (Indizes) ohne Heap: .len() und [k] wie ein Vec (die KI fragt sehr oft, TechInfo bleibt Copy).
#[derive(Clone, Copy, Debug, Default)]
pub struct Voraus { n: u8, v: [i32; 12] }
impl Voraus {
    pub fn aus(s: &[i32]) -> Voraus { assert!(s.len() <= 12, "mehr als 12 Voraussetzungen"); let mut r = Voraus { n: s.len() as u8, v: [0; 12] }; r.v[..s.len()].copy_from_slice(s); r }
    pub fn len(&self) -> usize { self.n as usize }
    pub fn is_empty(&self) -> bool { self.n == 0 }
}
impl std::ops::Index<usize> for Voraus { type Output = i32; fn index(&self, k: usize) -> &i32 { &self.v[..self.n as usize][k] } }

/// Research::GetTechInfo(i) in einem Stueck.
#[derive(Clone, Copy, Debug, Default)]
pub struct TechInfo {
    pub valid: bool,
    pub id: i32,
    pub category: i32,
    pub eden_cost: i32,
    pub plymouth_cost: i32,
    pub lab: i32,
    pub max_scientists: i32,
    /// Indizes (nicht IDs) der vorausgesetzten Techniken, wie GetRequiredTechIndex(k).
    pub required: Voraus,
}

pub trait Welt {
    // ---------------------------------------------------------------- TethysGame / GameMap
    fn Tick(&self) -> i32;
    fn NoPlayers(&self) -> i32;
    fn LocalPlayer(&self) -> i32 { 0 }
    fn GetRand(&mut self, range: i32) -> i32;
    fn FreeMoraleLevel(&mut self, p: i32);
    /// Mehrspieler-Option "Moral stabil" (TethysGame::UsesMorale() == false). Rust 2026-11-21; Standard: nein
    fn MoralStabil(&self) -> bool { false }
    /// Schwierigkeit des Spielers wie OP2 (0 leicht, 1 mittel, 2 schwer); -1 = unbekannt. Rust 2026-11-22
    fn Schwierigkeit(&self, _p: i32) -> i32 { -1 }
    fn GetCellType(&self, l: Loc) -> i32;
    /// TethysGame::CreateUnit(u, typ, loc, spieler, ladung/waffe, richtung): neue Einheitennummer (0 = misslungen).
    fn CreateUnit(&mut self, typ: i32, l: Loc, spieler: i32, cargo: i32, rotation: i32) -> Id;
    /// TethysGame::CreateBeacon(typ, x, y, erzTyp, balken, variante).
    fn CreateBeacon(&mut self, typ: i32, x: i32, y: i32, ore_type: i32, bar: i32, variant: i32) -> Id;
    /// HFL: hoechste vergebene Einheitennummer (ki::hoechsteEinheit).
    fn hoechsteEinheit(&self) -> i32;

    // ---------------------------------------------------------------- Unit / UnitEx (Abfragen)
    fn IsLive(&self, u: Id) -> bool;
    fn GetType(&self, u: Id) -> i32;
    fn OwnerID(&self, u: Id) -> i32;
    fn IsVehicle(&self, u: Id) -> bool;
    fn IsBuilding(&self, u: Id) -> bool;
    fn IsBusy(&self, u: Id) -> bool;
    fn isEMPed(&self, u: Id) -> bool;
    /// Restdauer EMP / Stickyfoam (HFL UnitEx::GetTimeEMPed / GetTimeStickyfoamed, 0 = frei). Nur Rust-Erweiterung.
    fn zeitEMP(&self, _u: Id) -> i32 { 0 }
    fn zeitSchaum(&self, _u: Id) -> i32 { 0 }
    fn Location(&self, u: Id) -> Loc;
    fn GetDamage(&self, u: Id) -> i32;
    fn GetWeapon(&self, u: Id) -> i32;
    /// ConVec: geladener Bausatz (map_id).
    fn GetCargo(&self, u: Id) -> i32;
    fn GetConVecCargoTurret(&self, u: Id) -> i32;
    fn GetCurAction(&self, u: Id) -> i32;
    fn GetLastCommand(&self, u: Id) -> i32;
    fn HasPower(&self, u: Id) -> bool;
    fn HasWorkers(&self, u: Id) -> bool;
    fn HasScientists(&self, u: Id) -> bool;
    fn GetLabCurrentTopic(&self, u: Id) -> i32;
    fn GetLabScientistCount(&self, u: Id) -> i32;
    fn GetFactoryCargo(&self, u: Id, bay: i32) -> i32;
    fn GetFactoryCargoWeapon(&self, u: Id, bay: i32) -> i32;
    fn GetDockLocation(&self, u: Id) -> Loc;
    fn GetCargoAmount(&self, u: Id) -> i32;
    fn GetCargoType(&self, u: Id) -> i32;
    fn GetNumTruckLoadsSoFar(&self, u: Id) -> i32;
    fn GetBarYield(&self, u: Id) -> i32;
    fn GetOreType(&self, u: Id) -> i32;
    fn GetSurveyedBy(&self, u: Id) -> i32;
    fn GetWorkersInTraining(&self, u: Id) -> i32;
    fn GetLaunchPadCargo(&self, u: Id) -> i32;

    // ---------------------------------------------------------------- Unit / UnitEx (Befehle)
    fn SetLabScientistCount(&mut self, u: Id, n: i32);
    fn SetLaunchPadCargo(&mut self, u: Id, typ: i32);
    fn SetFactoryCargo(&mut self, u: Id, bay: i32, typ: i32, cargo: i32);
    fn DoMove(&mut self, u: Id, l: Loc);
    /// Unit::DoAttack(Unit) (C++: `u.Unit::DoAttack(ziel)`).
    fn DoAttack(&mut self, u: Id, ziel: Id);
    fn DoBuild(&mut self, u: Id, typ: i32, l: Loc);
    fn DoDeployMiner(&mut self, u: Id, l: Loc);
    fn DoDock(&mut self, u: Id, l: Loc);
    fn DoBuildWall(&mut self, u: Id, typ: i32, r: Rect);
    fn DoProduce(&mut self, u: Id, typ: i32, cargo: i32);
    fn DoTransferCargo(&mut self, u: Id, bay: i32);
    /// DoResearch(Tabellenindex, Wissenschaftler).
    fn DoResearch(&mut self, u: Id, tech_index: i32, n: i32);
    fn DoTrainScientists(&mut self, u: Id, n: i32);
    fn DoRepair(&mut self, u: Id, ziel: Id);
    fn DoReprogram(&mut self, u: Id, ziel: Id);
    fn DoIdle(&mut self, u: Id);
    fn DoUnIdle(&mut self, u: Id);
    fn DoStop(&mut self, u: Id);
    fn DoLaunch(&mut self, u: Id, px: i32, py: i32, force: bool);
    fn DoSelfDestruct(&mut self, u: Id);
    fn DoPoof(&mut self, u: Id);

    // ---------------------------------------------------------------- Aufzaehler (Enumeratoren)
    /// PlayerBuildingEnum(p, typ); typ = mapNone (0) liefert alle Gebaeude.
    fn PlayerBuildingEnum(&self, p: i32, typ: i32) -> Vec<Id>;
    fn PlayerVehicleEnum(&self, p: i32) -> Vec<Id>;
    fn PlayerUnitEnum(&self, p: i32) -> Vec<Id>;
    fn InRectEnumerator(&self, r: Rect) -> Vec<Id>;
    fn InRangeEnumerator(&self, l: Loc, r: i32) -> Vec<Id>;
    fn LocationEnumerator(&self, l: Loc) -> Vec<Id>;
    fn GroupEnumerator(&self, g: Gruppe) -> Vec<Id>;

    // ---------------------------------------------------------------- UnitInfo(typ).GetX(spieler)
    fn GetHitPoints(&self, typ: i32, p: i32) -> i32;
    fn GetArmor(&self, typ: i32, p: i32) -> i32;
    fn GetOreCost(&self, typ: i32, p: i32) -> i32;
    fn GetRareOreCost(&self, typ: i32, p: i32) -> i32;
    fn GetWorkersRequired(&self, typ: i32, p: i32) -> i32;
    fn GetScientistsRequired(&self, typ: i32, p: i32) -> i32;
    fn GetPowerRequired(&self, typ: i32, p: i32) -> i32;
    fn GetWeaponRange(&self, typ: i32, p: i32) -> i32;
    fn GetProductionCapacity(&self, typ: i32, p: i32) -> i32;
    fn GetReloadTime(&self, typ: i32, p: i32) -> i32;
    fn GetPenetrationDamage(&self, typ: i32, p: i32) -> i32;
    fn GetConcussionDamage(&self, typ: i32, p: i32) -> i32;
    fn GetBuildTime(&self, typ: i32, p: i32) -> i32;

    // ---------------------------------------------------------------- Research / TechInfo
    fn GetTechCount(&self) -> i32;
    fn GetTechInfo(&self, i: i32) -> TechInfo;
    fn GetPlayerHasTech(&self, i: i32, p: i32) -> bool;

    // ---------------------------------------------------------------- Player[p]
    fn IsEden(&self, p: i32) -> bool;
    fn Ore(&self, p: i32) -> i32;
    fn RareOre(&self, p: i32) -> i32;
    fn FoodStored(&self, p: i32) -> i32;
    fn Workers(&self, p: i32) -> i32;
    fn Scientists(&self, p: i32) -> i32;
    fn Kids(&self, p: i32) -> i32;
    fn MoraleLevel(&self, p: i32) -> i32;
    /// Genauer Moralwert 0..100 (C++ moralZahl liest ihn roh aus playerArray, Index 16); -1 = unbekannt.
    fn MoraleRoh(&self, p: i32) -> i32;
    /// Nur mit bekannter techID aufrufen (C++ prueft vorher techIndex).
    fn HasTechnology(&self, p: i32, tech_id: i32) -> bool;
    fn SetOre(&mut self, p: i32, n: i32);
    fn SetRareOre(&mut self, p: i32, n: i32);
    fn CenterViewOn(&mut self, _p: i32, _x: i32, _y: i32) {}

    // ---------------------------------------------------------------- ExtPlayer[p] (HFL PlayerEx)
    fn IsAlliedTo(&self, p: i32, q: i32) -> bool;
    fn GetMaxOre(&self, p: i32) -> i32;
    fn GetMaxRareOre(&self, p: i32) -> i32;
    fn GetNumAvailableWorkers(&self, p: i32) -> i32;
    fn GetNumAvailableScientists(&self, p: i32) -> i32;
    fn GetAmountPowerGenerated(&self, p: i32) -> i32;
    fn GetAmountPowerConsumed(&self, p: i32) -> i32;
    fn GetAmountPowerAvailable(&self, p: i32) -> i32;
    fn GetNumIdleBuildings(&self, p: i32) -> i32;
    fn GetNumActiveBuildings(&self, p: i32) -> i32;
    fn GetNumUnpoweredStructures(&self, p: i32) -> i32;
    fn GetNumWorkersRequired(&self, p: i32) -> i32;
    fn GetNumScientistsRequired(&self, p: i32) -> i32;
    fn GetNumScientistsAssignedToResearch(&self, p: i32) -> i32;
    fn GetNetFoodProduction(&self, p: i32) -> i32;
    fn GetTotalResidenceCapacity(&self, p: i32) -> i32;
    fn GetTotalMedCenterCapacity(&self, p: i32) -> i32;
    fn GetTotalRecreationFacilityCapacity(&self, p: i32) -> i32;
    fn GetTotalForumCapacity(&self, p: i32) -> i32;
    fn GetSatelliteCount(&self, p: i32, typ: i32) -> i32;
    fn RecalculateValues(&mut self, _p: i32) {}

    // ---------------------------------------------------------------- Gruppen
    fn CreateFightGroup(&mut self, p: i32) -> Gruppe;
    fn CreateBuildingGroup(&mut self, p: i32) -> Gruppe;
    fn CreateMiningGroup(&mut self, p: i32) -> Gruppe;
    fn Destroy(&mut self, g: Gruppe);
    fn TakeUnit(&mut self, g: Gruppe, u: Id);
    fn RemoveUnit(&mut self, g: Gruppe, u: Id);
    /// ScGroup::TakeAllUnits(quelle): alle Einheiten der Quelle in diese Gruppe.
    fn TakeAllUnits(&mut self, g: Gruppe, quelle: Gruppe);
    fn TotalUnitCount(&self, g: Gruppe) -> i32;
    /// FightGroup::SetRect / BuildingGroup::SetRect (Ruhe- bzw. Arbeitsbereich).
    fn SetRect(&mut self, g: Gruppe, r: Rect);
    fn DoGuardRect(&mut self, g: Gruppe);
    fn AddGuardedRect(&mut self, g: Gruppe, r: Rect);
    fn SetTargCount(&mut self, g: Gruppe, typ: i32, waffe: i32, n: i32);
    fn ClearTargCount(&mut self, g: Gruppe);
    fn RecordBuilding(&mut self, g: Gruppe, l: Loc, typ: i32, waffe: i32);
    fn RecordTube(&mut self, g: Gruppe, l: Loc);
    fn RecordWall(&mut self, g: Gruppe, l: Loc, typ: i32);
    fn RecordVehReinforceGroup(&mut self, g: Gruppe, ziel: Gruppe, prio: i32);
    /// MiningGroup::Setup(Mine, Schmelze, Bereich).
    fn Setup(&mut self, g: Gruppe, mine: Loc, schmelze: Loc, r: Rect);

    // ---------------------------------------------------------------- Dateien und Spur (statt fopen/op2::log)
    /// Eine JSON-Zeile in die KI-Spur (<Mission>-KI.jsonl).
    fn kiRaw(&mut self, zeile: &str);
    /// Zeile ins Missions-Log.
    fn logZeile(&mut self, zeile: &str);
    /// Datei aus dem Wissensordner (logs/) lesen.
    fn dateiLesen(&self, name: &str) -> Option<String>;
    fn dateiSchreiben(&mut self, name: &str, text: &str);
}
