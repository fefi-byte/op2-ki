//! ERZEUGT von werkzeug/bruecke.py aus dem Trait Welt: `Welt` ueber die Rufschnittstelle der Missions-DLL.
//! Nicht von Hand aendern.
#![allow(non_snake_case)]
use op2ki::cpp::welt::{Gruppe, Id, Loc, Rect, TechInfo, Voraus, Welt};

pub type RufFn = unsafe extern "C" fn(code: i32, a: *const i32, na: i32, o: *mut i32, no: i32) -> i32;
pub type TextFn = unsafe extern "C" fn(code: i32, s: *const u8, n: i32);
pub type LesenFn = unsafe extern "C" fn(name: *const u8, nn: i32, out: *mut u8, max: i32) -> i32;
pub type SchreibenFn = unsafe extern "C" fn(name: *const u8, nn: i32, s: *const u8, n: i32);
#[repr(C)] #[derive(Clone, Copy)] pub struct Api { pub ruf: RufFn, pub text: TextFn, pub lesen: LesenFn, pub schreiben: SchreibenFn }
pub struct ExternWelt { pub api: Api, puf: std::cell::RefCell<Vec<i32>> }
impl ExternWelt {
    pub fn neu(api: Api) -> Self { ExternWelt { api, puf: std::cell::RefCell::new(vec![0; 4096]) } }
    fn r(&self, code: i32, a: &[i32]) -> i32 { let mut o = self.puf.borrow_mut(); unsafe { (self.api.ruf)(code, a.as_ptr(), a.len() as i32, o.as_mut_ptr(), o.len() as i32) } }
    fn rv(&self, code: i32, a: &[i32]) -> Vec<i32> { let mut o = self.puf.borrow_mut(); let n = unsafe { (self.api.ruf)(code, a.as_ptr(), a.len() as i32, o.as_mut_ptr(), o.len() as i32) }; o[..n.max(0) as usize].to_vec() }
    fn rl(&self, code: i32, a: &[i32]) -> Loc { let v = self.rv(code, a); if v.len() >= 2 { Loc::new(v[0], v[1]) } else { Loc::default() } }
}
impl Welt for ExternWelt {
    fn Tick(&self) -> i32 { self.r(0, &[]) }
    fn NoPlayers(&self) -> i32 { self.r(1, &[]) }
    fn LocalPlayer(&self) -> i32 { self.r(2, &[]) }
    fn FreeMoraleLevel(&mut self, p: i32) { self.r(4, &[p]); }
    fn GetCellType(&self, l: Loc) -> i32 { self.r(5, &[l.x, l.y]) }
    fn CreateUnit(&mut self, typ: i32, l: Loc, spieler: i32, cargo: i32, rotation: i32) -> Id { self.r(6, &[typ, l.x, l.y, spieler, cargo, rotation]) }
    fn CreateBeacon(&mut self, typ: i32, x: i32, y: i32, ore_type: i32, bar: i32, variant: i32) -> Id { self.r(7, &[typ, x, y, ore_type, bar, variant]) }
    fn hoechsteEinheit(&self) -> i32 { self.r(8, &[]) }
    fn IsLive(&self, u: Id) -> bool { self.r(9, &[u]) != 0 }
    fn GetType(&self, u: Id) -> i32 { self.r(10, &[u]) }
    fn OwnerID(&self, u: Id) -> i32 { self.r(11, &[u]) }
    fn IsVehicle(&self, u: Id) -> bool { self.r(12, &[u]) != 0 }
    fn IsBuilding(&self, u: Id) -> bool { self.r(13, &[u]) != 0 }
    fn IsBusy(&self, u: Id) -> bool { self.r(14, &[u]) != 0 }
    fn isEMPed(&self, u: Id) -> bool { self.r(15, &[u]) != 0 }
    fn zeitEMP(&self, u: Id) -> i32 { self.r(139, &[u]).max(0) }
    fn zeitSchaum(&self, u: Id) -> i32 { self.r(140, &[u]).max(0) }
    fn Location(&self, u: Id) -> Loc { self.rl(16, &[u]) }
    fn GetDamage(&self, u: Id) -> i32 { self.r(17, &[u]) }
    fn GetWeapon(&self, u: Id) -> i32 { self.r(18, &[u]) }
    fn GetCargo(&self, u: Id) -> i32 { self.r(19, &[u]) }
    fn GetCurAction(&self, u: Id) -> i32 { self.r(21, &[u]) }
    fn GetLastCommand(&self, u: Id) -> i32 { self.r(22, &[u]) }
    fn HasPower(&self, u: Id) -> bool { self.r(23, &[u]) != 0 }
    fn HasWorkers(&self, u: Id) -> bool { self.r(24, &[u]) != 0 }
    fn HasScientists(&self, u: Id) -> bool { self.r(25, &[u]) != 0 }
    fn GetLabCurrentTopic(&self, u: Id) -> i32 { self.r(26, &[u]) }
    fn GetLabScientistCount(&self, u: Id) -> i32 { self.r(27, &[u]) }
    fn GetFactoryCargo(&self, u: Id, bay: i32) -> i32 { self.r(28, &[u, bay]) }
    fn GetDockLocation(&self, u: Id) -> Loc { self.rl(30, &[u]) }
    fn GetCargoAmount(&self, u: Id) -> i32 { self.r(31, &[u]) }
    fn GetCargoType(&self, u: Id) -> i32 { self.r(32, &[u]) }
    fn GetNumTruckLoadsSoFar(&self, u: Id) -> i32 { self.r(33, &[u]) }
    fn GetBarYield(&self, u: Id) -> i32 { self.r(34, &[u]) }
    fn GetOreType(&self, u: Id) -> i32 { self.r(35, &[u]) }
    fn GetSurveyedBy(&self, u: Id) -> i32 { self.r(36, &[u]) }
    fn GetWorkersInTraining(&self, u: Id) -> i32 { self.r(37, &[u]) }
    fn GetLaunchPadCargo(&self, u: Id) -> i32 { self.r(38, &[u]) }
    fn SetLabScientistCount(&mut self, u: Id, n: i32) { self.r(39, &[u, n]); }
    fn SetLaunchPadCargo(&mut self, u: Id, typ: i32) { self.r(40, &[u, typ]); }
    fn SetFactoryCargo(&mut self, u: Id, bay: i32, typ: i32, cargo: i32) { self.r(41, &[u, bay, typ, cargo]); }
    fn DoMove(&mut self, u: Id, l: Loc) { self.r(42, &[u, l.x, l.y]); }
    fn DoAttack(&mut self, u: Id, ziel: Id) { self.r(43, &[u, ziel]); }
    fn DoBuild(&mut self, u: Id, typ: i32, l: Loc) { self.r(44, &[u, typ, l.x, l.y]); }
    fn DoDeployMiner(&mut self, u: Id, l: Loc) { self.r(45, &[u, l.x, l.y]); }
    fn DoDock(&mut self, u: Id, l: Loc) { self.r(46, &[u, l.x, l.y]); }
    fn DoBuildWall(&mut self, u: Id, typ: i32, r: Rect) { self.r(47, &[u, typ, r.x1, r.y1, r.x2, r.y2]); }
    fn DoProduce(&mut self, u: Id, typ: i32, cargo: i32) { self.r(48, &[u, typ, cargo]); }
    fn DoTransferCargo(&mut self, u: Id, bay: i32) { self.r(49, &[u, bay]); }
    fn DoResearch(&mut self, u: Id, tech_index: i32, n: i32) { self.r(50, &[u, tech_index, n]); }
    fn DoTrainScientists(&mut self, u: Id, n: i32) { self.r(51, &[u, n]); }
    fn DoRepair(&mut self, u: Id, ziel: Id) { self.r(52, &[u, ziel]); }
    fn DoReprogram(&mut self, u: Id, ziel: Id) { self.r(53, &[u, ziel]); }
    fn DoIdle(&mut self, u: Id) { self.r(54, &[u]); }
    fn DoUnIdle(&mut self, u: Id) { self.r(55, &[u]); }
    fn DoStop(&mut self, u: Id) { self.r(56, &[u]); }
    fn DoLaunch(&mut self, u: Id, px: i32, py: i32, force: bool) { self.r(57, &[u, px, py, force as i32]); }
    fn PlayerBuildingEnum(&self, p: i32, typ: i32) -> Vec<Id> { self.rv(60, &[p, typ]) }
    fn PlayerVehicleEnum(&self, p: i32) -> Vec<Id> { self.rv(61, &[p]) }
    fn PlayerUnitEnum(&self, p: i32) -> Vec<Id> { self.rv(62, &[p]) }
    fn InRectEnumerator(&self, r: Rect) -> Vec<Id> { self.rv(63, &[r.x1, r.y1, r.x2, r.y2]) }
    fn InRangeEnumerator(&self, l: Loc, r: i32) -> Vec<Id> { self.rv(64, &[l.x, l.y, r]) }
    fn LocationEnumerator(&self, l: Loc) -> Vec<Id> { self.rv(65, &[l.x, l.y]) }
    fn GroupEnumerator(&self, g: Gruppe) -> Vec<Id> { self.rv(66, &[g]) }
    fn GetHitPoints(&self, typ: i32, p: i32) -> i32 { self.r(67, &[typ, p]) }
    fn GetArmor(&self, typ: i32, p: i32) -> i32 { self.r(68, &[typ, p]) }
    fn GetOreCost(&self, typ: i32, p: i32) -> i32 { self.r(69, &[typ, p]) }
    fn GetRareOreCost(&self, typ: i32, p: i32) -> i32 { self.r(70, &[typ, p]) }
    fn GetWorkersRequired(&self, typ: i32, p: i32) -> i32 { self.r(71, &[typ, p]) }
    fn GetScientistsRequired(&self, typ: i32, p: i32) -> i32 { self.r(72, &[typ, p]) }
    fn GetPowerRequired(&self, typ: i32, p: i32) -> i32 { self.r(73, &[typ, p]) }
    fn GetWeaponRange(&self, typ: i32, p: i32) -> i32 { self.r(74, &[typ, p]) }
    fn GetProductionCapacity(&self, typ: i32, p: i32) -> i32 { self.r(75, &[typ, p]) }
    fn GetReloadTime(&self, typ: i32, p: i32) -> i32 { self.r(76, &[typ, p]) }
    fn GetPenetrationDamage(&self, typ: i32, p: i32) -> i32 { self.r(77, &[typ, p]) }
    fn GetConcussionDamage(&self, typ: i32, p: i32) -> i32 { self.r(78, &[typ, p]) }
    fn GetBuildTime(&self, typ: i32, p: i32) -> i32 { self.r(79, &[typ, p]) }
    fn GetTechCount(&self) -> i32 { self.r(80, &[]) }
    fn GetTechInfo(&self, i: i32) -> TechInfo { let v = self.rv(81, &[i]); if v.is_empty() || v[0] == 0 { return TechInfo::default(); } TechInfo { valid: true, id: v[1], category: v[2], eden_cost: v[3], plymouth_cost: v[4], lab: v[5], max_scientists: v[6], required: Voraus::aus(&v[8..(8 + v[7].max(0) as usize).min(v.len())]) } }
    fn GetPlayerHasTech(&self, i: i32, p: i32) -> bool { self.r(82, &[i, p]) != 0 }
    fn IsEden(&self, p: i32) -> bool { self.r(83, &[p]) != 0 }
    fn Ore(&self, p: i32) -> i32 { self.r(84, &[p]) }
    fn RareOre(&self, p: i32) -> i32 { self.r(85, &[p]) }
    fn FoodStored(&self, p: i32) -> i32 { self.r(86, &[p]) }
    fn Workers(&self, p: i32) -> i32 { self.r(87, &[p]) }
    fn Scientists(&self, p: i32) -> i32 { self.r(88, &[p]) }
    fn Kids(&self, p: i32) -> i32 { self.r(89, &[p]) }
    fn MoraleLevel(&self, p: i32) -> i32 { self.r(90, &[p]) }
    fn MoraleRoh(&self, p: i32) -> i32 { self.r(91, &[p]) }
    fn HasTechnology(&self, p: i32, tech_id: i32) -> bool { self.r(92, &[p, tech_id]) != 0 }
    fn SetOre(&mut self, p: i32, n: i32) { self.r(93, &[p, n]); }
    fn SetRareOre(&mut self, p: i32, n: i32) { self.r(94, &[p, n]); }
    fn CenterViewOn(&mut self, p: i32, x: i32, y: i32) { self.r(95, &[p, x, y]); }
    fn IsAlliedTo(&self, p: i32, q: i32) -> bool { self.r(96, &[p, q]) != 0 }
    fn GetMaxOre(&self, p: i32) -> i32 { self.r(97, &[p]) }
    fn GetMaxRareOre(&self, p: i32) -> i32 { self.r(98, &[p]) }
    fn GetNumAvailableWorkers(&self, p: i32) -> i32 { self.r(99, &[p]) }
    fn GetNumAvailableScientists(&self, p: i32) -> i32 { self.r(100, &[p]) }
    fn GetAmountPowerGenerated(&self, p: i32) -> i32 { self.r(101, &[p]) }
    fn GetAmountPowerConsumed(&self, p: i32) -> i32 { self.r(102, &[p]) }
    fn GetAmountPowerAvailable(&self, p: i32) -> i32 { self.r(103, &[p]) }
    fn GetNumIdleBuildings(&self, p: i32) -> i32 { self.r(104, &[p]) }
    fn GetNumActiveBuildings(&self, p: i32) -> i32 { self.r(105, &[p]) }
    fn GetNumUnpoweredStructures(&self, p: i32) -> i32 { self.r(106, &[p]) }
    fn GetNumWorkersRequired(&self, p: i32) -> i32 { self.r(107, &[p]) }
    fn GetNumScientistsRequired(&self, p: i32) -> i32 { self.r(108, &[p]) }
    fn GetNumScientistsAssignedToResearch(&self, p: i32) -> i32 { self.r(109, &[p]) }
    fn GetNetFoodProduction(&self, p: i32) -> i32 { self.r(110, &[p]) }
    fn GetTotalResidenceCapacity(&self, p: i32) -> i32 { self.r(111, &[p]) }
    fn GetTotalMedCenterCapacity(&self, p: i32) -> i32 { self.r(112, &[p]) }
    fn GetTotalRecreationFacilityCapacity(&self, p: i32) -> i32 { self.r(113, &[p]) }
    fn GetTotalForumCapacity(&self, p: i32) -> i32 { self.r(114, &[p]) }
    fn GetSatelliteCount(&self, p: i32, typ: i32) -> i32 { self.r(115, &[p, typ]) }
    fn RecalculateValues(&mut self, p: i32) { self.r(116, &[p]); }
    fn CreateFightGroup(&mut self, p: i32) -> Gruppe { self.r(117, &[p]) }
    fn CreateBuildingGroup(&mut self, p: i32) -> Gruppe { self.r(118, &[p]) }
    fn CreateMiningGroup(&mut self, p: i32) -> Gruppe { self.r(119, &[p]) }
    fn Destroy(&mut self, g: Gruppe) { self.r(120, &[g]); }
    fn TakeUnit(&mut self, g: Gruppe, u: Id) { self.r(121, &[g, u]); }
    fn RemoveUnit(&mut self, g: Gruppe, u: Id) { self.r(122, &[g, u]); }
    fn TakeAllUnits(&mut self, g: Gruppe, quelle: Gruppe) { self.r(123, &[g, quelle]); }
    fn TotalUnitCount(&self, g: Gruppe) -> i32 { self.r(124, &[g]) }
    fn SetRect(&mut self, g: Gruppe, r: Rect) { self.r(125, &[g, r.x1, r.y1, r.x2, r.y2]); }
    fn DoGuardRect(&mut self, g: Gruppe) { self.r(126, &[g]); }
    fn AddGuardedRect(&mut self, g: Gruppe, r: Rect) { self.r(127, &[g, r.x1, r.y1, r.x2, r.y2]); }
    fn SetTargCount(&mut self, g: Gruppe, typ: i32, waffe: i32, n: i32) { self.r(128, &[g, typ, waffe, n]); }
    fn ClearTargCount(&mut self, g: Gruppe) { self.r(129, &[g]); }
    fn RecordBuilding(&mut self, g: Gruppe, l: Loc, typ: i32, waffe: i32) { self.r(130, &[g, l.x, l.y, typ, waffe]); }
    fn RecordTube(&mut self, g: Gruppe, l: Loc) { self.r(131, &[g, l.x, l.y]); }
    fn RecordWall(&mut self, g: Gruppe, l: Loc, typ: i32) { self.r(132, &[g, l.x, l.y, typ]); }
    fn RecordVehReinforceGroup(&mut self, g: Gruppe, ziel: Gruppe, prio: i32) { self.r(133, &[g, ziel, prio]); }
    fn Setup(&mut self, g: Gruppe, mine: Loc, schmelze: Loc, r: Rect) { self.r(134, &[g, mine.x, mine.y, schmelze.x, schmelze.y, r.x1, r.y1, r.x2, r.y2]); }
    fn kiRaw(&mut self, zeile: &str) { unsafe { (self.api.text)(135, zeile.as_ptr(), zeile.len() as i32) } }
    fn logZeile(&mut self, zeile: &str) { unsafe { (self.api.text)(136, zeile.as_ptr(), zeile.len() as i32) } }
    fn dateiLesen(&self, name: &str) -> Option<String> { let mut b = vec![0u8; 1 << 20]; let k = unsafe { (self.api.lesen)(name.as_ptr(), name.len() as i32, b.as_mut_ptr(), b.len() as i32) }; if k < 0 { None } else { b.truncate(k as usize); Some(String::from_utf8_lossy(&b).into_owned()) } }
    fn dateiSchreiben(&mut self, name: &str, text: &str) { unsafe { (self.api.schreiben)(name.as_ptr(), name.len() as i32, text.as_ptr(), text.len() as i32) } }
}
