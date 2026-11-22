//! `Welt` (die OP2-API der C++-KI) auf op2-sim. Engine-Koordinaten: x = Dateikachel + 32 (Karten bis 256 breit), y gleich.
//! Nummern wie im Original: die Objektnummer (MapObject::index_), fuer Einheiten wie fuer Baken.
//! The OP2 API of the C++ AI on top of op2-sim.
#![allow(non_snake_case)]
use std::collections::HashMap;

use op2_sim::colony::{caps, BeaconKind, Product, F_POWER, F_SCIENTISTS, F_WORKERS};
use op2_sim::groups::{GroupKind, Objective};
use op2_sim::terrain::Wall;
use op2_sim::{Action, Command, CommandKind, Rules, Sim, StructureCommand, UnitId, UnitKind, WeaponKind};
use crate::api_voll::*;
use op2ki::cpp::tb;
use op2ki::cpp::welt::{Gruppe, Id, Loc, Rect, TechInfo, Voraus, Welt};

pub const UX: i32 = 32;

pub struct SimWelt {
    pub sim: Sim,
    kind_map: HashMap<UnitKind, i32>,
    map_kind: HashMap<i32, UnitKind>,
    waffe_map: HashMap<WeaponKind, i32>,
    map_waffe: HashMap<i32, WeaponKind>,
    gruppen: HashMap<i32, GroupKind>,
    naechste_gruppe: i32,
    /// Techniktabelle wie im Original (Research::GetTechInfo): Platz 0 ist der Platzhalter "keine Voraussetzung" (ID 0),
    /// danach alle Techniken nach ID sortiert. tech_virt: Platz -> Index in rules.eco.techs (None = Platzhalter);
    /// tech_rv: Index in rules.eco.techs -> Platz; tech_idx: Tech-ID -> Platz.
    tech_idx: HashMap<u32, usize>,
    tech_virt: Vec<Option<usize>>,
    tech_rv: Vec<usize>,
    pub spur: Option<std::fs::File>,
    pub log: Vec<String>,
    pub ordner: Option<std::path::PathBuf>,
    /// Objektnummer -> UnitId (Tick, Anzahl Einheiten, Tabelle), siehe uid()
    nummern: std::cell::RefCell<(u32, usize, Vec<u32>)>,
    pub dateien: HashMap<String, String>,
}

impl SimWelt {
    pub fn neu(sim: Sim) -> SimWelt {
        let rules = sim.rules().clone();
        let (mut kind_map, mut map_kind, mut waffe_map, mut map_waffe) = (HashMap::new(), HashMap::new(), HashMap::new(), HashMap::new());
        for &(t, code, _) in crate::typen::NAMEN.iter() {
            if let Some(k) = rules.kind(code) { kind_map.insert(k, t as i32); map_kind.insert(t as i32, k); }
            else if let Some(w) = rules.weapon_kind(code) { waffe_map.insert(w, t as i32); map_waffe.insert(t as i32, w); }
        }
        let mut sortiert: Vec<usize> = (0..rules.eco.techs.len()).collect();
        sortiert.sort_by_key(|&i| rules.eco.techs[i].id);
        let tech_virt: Vec<Option<usize>> = std::iter::once(None).chain(sortiert.iter().map(|&i| Some(i))).collect();
        let mut tech_rv = vec![0usize; rules.eco.techs.len()];
        for (v, r) in tech_virt.iter().enumerate() { if let Some(r) = r { tech_rv[*r] = v; } }
        let tech_idx = tech_virt.iter().enumerate().filter_map(|(v, r)| r.map(|r| (rules.eco.techs[r].id, v))).collect();
        SimWelt { sim, kind_map, map_kind, waffe_map, map_waffe, gruppen: HashMap::new(), naechste_gruppe: 1, tech_idx, tech_virt, tech_rv,
            spur: None, log: Vec::new(), ordner: None, dateien: HashMap::new(), nummern: std::cell::RefCell::new((u32::MAX, usize::MAX, Vec::new())) }
    }
    fn rules(&self) -> &Rules { self.sim.rules() }
    /// Technik an Platz i der Original-Tabelle (None fuer den Platzhalter oder ausserhalb).
    fn tech_rules(&self, i: i32) -> Option<&op2_sim::rules::Tech> {
        let r = (*self.tech_virt.get(usize::try_from(i).ok()?)?)?;
        self.rules().eco.techs.get(r)
    }
    /// Die KI sieht wie im Original die Objektnummern (MapObject::index_, Baken eingeschlossen), nicht op2-sims UnitId.
    /// Zuordnung je Tick (und nach neuen Einheiten) neu aufgebaut.
    fn uid(&self, u: Id) -> UnitId {
        if u <= 0 || u > 4096 { return UnitId(0); }
        let (t, n) = (self.sim.tick(), self.sim.units().len());
        let mut c = self.nummern.borrow_mut();
        if c.0 != t || c.1 != n {
            c.2.clear(); c.2.resize(4097, 0);
            for x in self.sim.units().iter().filter(|x| self.sichtbar(x)) { c.2[x.index as usize] = x.id.0; }
            c.0 = t; c.1 = n;
        }
        UnitId(c.2[u as usize])
    }
    fn unit(&self, u: Id) -> Option<&op2_sim::Unit> {
        let id = self.uid(u);
        if id.0 == 0 { return None; }
        self.sim.unit(id).filter(|x| self.sichtbar(x))
    }
    /// Was die KI ueber eine Objektnummer lesen kann: jedes noch nicht geloeschte Objekt. Ein zerstoertes hat das Tot-Flag
    /// (0x20000, IsLive falsch), aber die SDK-Methoden der Bruecke (GetType, Location, ...) lesen es bis zur Loeschung weiter
    /// (gemessen mp2_08 Seed 1: Agridom 31, tot ab 38996, geloescht 39006, bei 39003 noch Angriffsziel der KI; Mine 7 tot
    /// ab 34040, bei 34043 nicht IsLive).
    fn sichtbar(&self, x: &op2_sim::Unit) -> bool {
        !x.removed
    }
    fn bake(&self, u: Id) -> Option<&op2_sim::colony::Beacon> {
        if u <= 0 { return None; }
        self.sim.beacons().iter().find(|b| b.index as i32 == u && !b.removed)
    }
    /// Objektnummer einer op2-sim-Einheit.
    fn nr(&self, id: UnitId) -> Id { self.sim.unit(id).map(|x| x.index as i32).unwrap_or(0) }
    fn tile(l: Loc) -> (i32, i32) { (l.x - UX, l.y) }
    fn loc(t: (i32, i32)) -> Loc { Loc::new(t.0 + UX, t.1) }
    fn map_von_kind(&self, k: UnitKind) -> i32 { self.kind_map.get(&k).copied().unwrap_or(0) }
    fn produkt(&self, p: Product) -> (i32, i32) {
        if p.is_none() { return (0, 0); }
        let k = if p.kind > 0 { self.map_von_kind(UnitKind(p.kind - 1)) } else { 0 };
        let w = if p.weapon > 0 { self.waffe_map.get(&WeaponKind(p.weapon - 1)).copied().unwrap_or(0) } else { 0 };
        (k, w)
    }
    fn stats(&self, typ: i32, p: i32) -> Option<&op2_sim::rules::UnitStats> { self.map_kind.get(&typ).map(|&k| self.sim.stats_of(k, p as u8)) }
    fn wstats(&self, typ: i32, p: i32) -> Option<&op2_sim::rules::WeaponStats> { self.map_waffe.get(&typ).map(|&k| self.sim.weapon_of(k, p as u8)) }
    fn kolonie(&self, p: i32) -> Option<&op2_sim::colony::Colony> { if p < 0 { None } else { self.sim.colony(p as u8) } }
    /// Lebende Einheiten, neueste zuerst (die Listenreihenfolge des Originals).
    fn neueste(&self) -> impl Iterator<Item = &op2_sim::Unit> + '_ { self.sim.units().iter().rev().filter(|u| u.alive && !u.removed) }
    fn gebaeude_an(&self, l: Loc) -> Option<UnitId> {
        let (x, y) = Self::tile(l);
        self.sim.occupant(x, y).filter(|id| self.sim.unit(*id).is_some_and(|u| self.sim.stats_of(u.kind, u.owner).structure))
    }
}

fn cmd_wert(c: CommandKind) -> i32 {
    match c {
        CommandKind::Nop => ctNop, CommandKind::Move => ctMoMove, CommandKind::Stop => ctMoStop, CommandKind::Attack => ctMoAttackObj,
        CommandKind::Dock => ctMoDock, CommandKind::Guard => ctMoGuard, CommandKind::StandGround => ctMoStandGround, CommandKind::Patrol => ctMoPatrol,
        CommandKind::Build => ctMoBuild, CommandKind::Produce => ctMoProduce, CommandKind::TransferCargo => ctMoTransferCargo, CommandKind::Idle => ctMoIdle,
        CommandKind::Research => ctMoResearch, CommandKind::TrainScientists => ctMoTrainScientists, CommandKind::Develop => ctMoDevelop,
        CommandKind::UnDevelop => ctMoUnDevelop, CommandKind::CargoRoute => ctMoCargoRoute, CommandKind::DumpCargo => ctMoDumpCargo,
        CommandKind::Repair => ctMoRepair, CommandKind::Survey => ctMoSurvey, CommandKind::SpecialWait => ctMoSpecialWait, CommandKind::Doze => ctMoDoze,
        CommandKind::BuildWall => ctMoBuildWall, CommandKind::RemoveWall => ctMoRemoveWall, CommandKind::RepairObj => ctMoRepairObj,
        CommandKind::Scavenge => ctMoScavenge, CommandKind::DockEG => ctMoDockEG, CommandKind::Dismantle => ctMoDismantle, CommandKind::Reprogram => ctMoReprogram,
        #[allow(unreachable_patterns)] _ => ctNop,
    }
}
fn aktion_wert(a: Action) -> i32 {
    match a {
        Action::Done => moDone, Action::Move => moMove, Action::ObjWait => moObjWait, Action::Develop => moDevelop,
        Action::OperationalWait => moOperationalWait, Action::ObjFroze => moObjFroze, Action::DoResearch => moDoResearch,
        Action::ObjDocking => moObjDocking, Action::Build => moBuild, Action::EMPWait => moEMPWait,
    }
}

impl Welt for SimWelt {
    fn GetRand(&mut self, range: i32) -> i32 { if range <= 0 { 0 } else { self.sim.game_rand(range as u32) as i32 } }
    fn Schwierigkeit(&self, _p: i32) -> i32 { self.sim.difficulty as i32 }
    fn DoSelfDestruct(&mut self, u: Id) { op2ki::cpp::hilfen::befehl_log2(self.Tick(), "DoSelfDestruct", &[u]); self.sim.command(self.uid(u), Command::SelfDestruct); }
    fn DoPoof(&mut self, u: Id) { op2ki::cpp::hilfen::befehl_log2(self.Tick(), "DoPoof", &[u]); self.sim.command(self.uid(u), Command::SelfDestruct); }
    fn RecalculateValues(&mut self, p: i32) { if p >= 0 { self.sim.colony_mut(p as u8).recalc = true; } }
    fn Tick(&self) -> i32 { self.sim.tick() as i32 }
    fn NoPlayers(&self) -> i32 { (1..8).rev().find(|&p| self.sim.colony(p as u8).is_some()).map(|p| p + 1).unwrap_or(1) }
    fn FreeMoraleLevel(&mut self, p: i32) { for m in self.sim.forced_morale.iter_mut() { *m &= !(1u8 << p); } }
    fn GetCellType(&self, l: Loc) -> i32 { let (x, y) = Self::tile(l); let t = self.sim.terrain(); if x < 0 || y < 0 || x >= t.width as i32 || y >= t.height as i32 { 1 } else { t.cell(x, y) as i32 } }
    fn CreateUnit(&mut self, typ: i32, l: Loc, spieler: i32, cargo: i32, rotation: i32) -> Id {
        let Some(&k) = self.map_kind.get(&typ) else { return 0 };
        let waffe = self.map_waffe.get(&cargo).copied().filter(|_| self.rules().carries_turret(k));
        let r = self.sim.spawn_placed(k, spieler as u8, Self::tile(l), (rotation & 255) as u8, waffe, true);
        let Ok(id) = r else { return 0 };
        if typ == mapConVec && cargo > 0 {
            if let Some(&ck) = self.map_kind.get(&cargo) { self.sim.set_kit(id, ck, None); }
        }
        if typ == mapCargoTruck { let _ = cargo; }
        self.nr(id)
    }
    fn CreateBeacon(&mut self, typ: i32, x: i32, y: i32, ore_type: i32, bar: i32, variant: i32) -> Id {
        // Erzbaken wie TethysGame::CreateBeacon im Original: Objektnummer und die Zufallsziehungen (3 + 1 bei VariantRandom)
        if typ == mapMagmaVent { self.sim.add_beacon((x - UX, y), BeaconKind::MagmaVent); }
        else if typ == mapFumarole { self.sim.add_beacon((x - UX, y), BeaconKind::Fumarole); }
        else { self.sim.add_ore_beacon((x - UX, y), ore_type, bar, variant); }
        self.sim.beacons().last().map(|b| b.index as i32).unwrap_or(0)
    }
    fn hoechsteEinheit(&self) -> i32 { self.sim.last_index() as i32 }

    fn IsLive(&self, u: Id) -> bool { self.unit(u).is_some_and(|x| x.alive) || self.bake(u).is_some() }
    fn GetType(&self, u: Id) -> i32 {
        if let Some(b) = self.bake(u) { return match b.kind { BeaconKind::Ore { .. } => mapMiningBeacon, BeaconKind::MagmaVent => mapMagmaVent, BeaconKind::Fumarole => mapFumarole }; }
        self.unit(u).map(|x| self.map_von_kind(x.kind)).unwrap_or(mapNone)
    }
    fn OwnerID(&self, u: Id) -> i32 { if self.bake(u).is_some() { return 6; } self.unit(u).map(|x| x.owner as i32).unwrap_or(0) }
    fn IsVehicle(&self, u: Id) -> bool { self.unit(u).is_some_and(|x| !self.sim.stats_of(x.kind, x.owner).structure) }
    fn IsBuilding(&self, u: Id) -> bool { self.unit(u).is_some_and(|x| self.sim.stats_of(x.kind, x.owner).structure) }
    fn IsBusy(&self, u: Id) -> bool { self.unit(u).is_some_and(|x| x.cmd != CommandKind::Nop) }
    fn isEMPed(&self, u: Id) -> bool { self.unit(u).is_some_and(|x| x.status.emped()) }
    fn zeitEMP(&self, u: Id) -> i32 { self.unit(u).filter(|x| x.status.emped()).map(|x| x.status.emp as i32).unwrap_or(0) }
    fn zeitSchaum(&self, u: Id) -> i32 { self.unit(u).filter(|x| x.status.foamed()).map(|x| x.status.foam as i32).unwrap_or(0) }
    fn Location(&self, u: Id) -> Loc {
        if let Some(b) = self.bake(u) { return Self::loc(b.tile); }
        self.unit(u).map(|x| Self::loc(x.tile())).unwrap_or_default()
    }
    fn GetDamage(&self, u: Id) -> i32 { self.unit(u).map(|x| x.damage).unwrap_or(0) }
    fn GetWeapon(&self, u: Id) -> i32 {
        let Some(x) = self.unit(u) else { return mapNone };
        if x.cmd == CommandKind::Produce { return self.produkt(x.eco.product).0; }
        // Ein Earthworker traegt im Waffenfeld (MapObject weapon_) die Wandart seines letzten BuildWall (gemessen mp2_08 Seed 1:
        // Earthworker 11 meldet 18 = mapWall, einer ohne BuildWall 0).
        if x.weapon.is_none() && self.map_von_kind(x.kind) == mapEarthworker {
            if let Some(k) = x.eco.work.as_ref().and_then(|w| w.kind) {
                return match k { Wall::Tube => mapTube, Wall::Normal => mapWall, Wall::Lava => mapLavaWall, Wall::Microbe => mapMicrobeWall };
            }
        }
        x.weapon.and_then(|w| self.waffe_map.get(&w).copied()).unwrap_or(mapNone)
    }
    fn GetConVecCargoTurret(&self, u: Id) -> i32 { self.unit(u).map(|x| self.produkt(x.eco.kit).1).unwrap_or(0) }
    fn GetCargo(&self, u: Id) -> i32 { self.unit(u).map(|x| self.produkt(x.eco.kit).0).unwrap_or(0) }
    fn GetCurAction(&self, u: Id) -> i32 {
        // Das Aktionsbyte wie gelesen (HFL curAction, +0x22). op2-sim fuehrt den Aufbau selbst als ObjWait-Takt mit einem
        // Done je Schritt; die fruehere Umdeutung Develop+Done -> ObjWait (aus einer Zeit ohne diesen Takt) wich ab
        // (Mine im Aufbau: Original meldet zur AIProc-Zeit 0, gemessen 2026-11-12).
        // Ein sterbendes Objekt: der Death-Befehl setzt die Aktion auf ObjWait (0x43ADF6), op2-sim fuehrt den Tod getrennt.
        self.unit(u).map(|x| if x.death.is_some() { moObjWait } else { aktion_wert(x.action) }).unwrap_or(moDone)
    }
    fn GetLastCommand(&self, u: Id) -> i32 { self.unit(u).map(|x| cmd_wert(x.cmd)).unwrap_or(ctNop) }
    fn HasPower(&self, u: Id) -> bool { self.unit(u).is_some_and(|x| x.eco.flags & F_POWER != 0) }
    fn HasWorkers(&self, u: Id) -> bool { self.unit(u).is_some_and(|x| x.eco.flags & F_WORKERS != 0) }
    fn HasScientists(&self, u: Id) -> bool { self.unit(u).is_some_and(|x| x.eco.flags & F_SCIENTISTS != 0) }
    fn GetLabCurrentTopic(&self, u: Id) -> i32 { self.unit(u).filter(|x| x.cmd == CommandKind::Research).map(|x| self.tech_rv.get(x.eco.lab_tech()).copied().unwrap_or(0) as i32).unwrap_or(0) }
    fn GetLabScientistCount(&self, u: Id) -> i32 { self.unit(u).filter(|x| x.cmd == CommandKind::Research).map(|x| x.eco.lab_team()).unwrap_or(0) }
    fn GetFactoryCargoWeapon(&self, u: Id, bay: i32) -> i32 { self.unit(u).and_then(|x| x.eco.bays.get(bay as usize).copied()).map(|p| self.produkt(p).1).unwrap_or(0) }
    fn GetFactoryCargo(&self, u: Id, bay: i32) -> i32 { self.unit(u).and_then(|x| x.eco.bays.get(bay as usize).copied()).map(|p| self.produkt(p).0).unwrap_or(0) }
    fn GetDockLocation(&self, u: Id) -> Loc { self.sim.dock_tile_of(self.uid(u)).map(Self::loc).unwrap_or_default() }
    fn GetCargoAmount(&self, u: Id) -> i32 { self.unit(u).map(|x| x.eco.cargo.1).unwrap_or(0) }
    fn GetCargoType(&self, u: Id) -> i32 { self.unit(u).map(|x| x.eco.cargo.0).unwrap_or(0) }
    fn GetNumTruckLoadsSoFar(&self, u: Id) -> i32 { self.unit(u).map(|x| x.eco.mine[2]).unwrap_or(0) }
    fn GetBarYield(&self, u: Id) -> i32 { match self.bake(u).map(|b| b.kind) { Some(BeaconKind::Ore { bar, .. }) => bar, _ => 0 } }
    fn GetOreType(&self, u: Id) -> i32 { match self.bake(u).map(|b| b.kind) { Some(BeaconKind::Ore { rare, .. }) => rare as i32, _ => 0 } }
    fn GetSurveyedBy(&self, u: Id) -> i32 { self.bake(u).map(|b| b.surveyed as i32).unwrap_or(0) }
    fn GetWorkersInTraining(&self, u: Id) -> i32 { self.unit(u).filter(|x| x.cmd == CommandKind::TrainScientists).map(|x| x.eco.class_size()).unwrap_or(0) }
    fn GetLaunchPadCargo(&self, _u: Id) -> i32 { 0 }

    fn SetLabScientistCount(&mut self, u: Id, n: i32) {  self.sim.set_lab_team(self.uid(u), n); }
    fn SetLaunchPadCargo(&mut self, _u: Id, _typ: i32) { }
    fn SetFactoryCargo(&mut self, u: Id, bay: i32, typ: i32, cargo: i32) { 
        if let Some(&k) = self.map_kind.get(&typ) { let w = self.map_waffe.get(&cargo).copied(); self.sim.set_bay(self.uid(u), bay as usize, k, w); }
    }
    fn DoMove(&mut self, u: Id, l: Loc) {  let (x, y) = Self::tile(l); self.sim.player_command(self.uid(u), Command::Move { tile: [x, y] }); }
    fn DoAttack(&mut self, u: Id, ziel: Id) {  if self.unit(ziel).is_some() { self.sim.player_command(self.uid(u), Command::Attack { target: self.uid(ziel) }); } }
    fn DoBuild(&mut self, u: Id, typ: i32, l: Loc) { 
        let k = self.map_kind.get(&typ).copied();
        let me = self.unit(u).map(|x| self.map_von_kind(x.kind)).unwrap_or(0);
        // Miner/GeoCon: Mine aus der Bake; ConVec: sein Bausatz (Build ruft mit None)
        if me == mapRoboMiner || me == mapGeoCon { let (x, y) = Self::tile(l); self.sim.player_command(self.uid(u), Command::Build { corner: [x, y] }); }
        else { let _ = k; let (x, y) = Self::tile(l); self.sim.player_command(self.uid(u), Command::Build { corner: [x, y] }); }
    }
    fn DoDeployMiner(&mut self, u: Id, l: Loc) {  self.DoMove(u, l); }
    fn DoDock(&mut self, u: Id, l: Loc) {  let (x, y) = Self::tile(l); self.sim.player_command(self.uid(u), Command::Dock { tile: [x, y] }); }
    fn DoBuildWall(&mut self, u: Id, typ: i32, r: Rect) { 
        let kind = match typ { t if t == mapTube => Wall::Tube, t if t == mapLavaWall => Wall::Lava, t if t == mapMicrobeWall => Wall::Microbe, _ => Wall::Normal };
        let (a, b) = (Self::tile(Loc::new(r.x1, r.y1)), Self::tile(Loc::new(r.x2, r.y2)));
        self.sim.player_command(self.uid(u), Command::BuildWall { rect: [a.0.min(b.0), a.1.min(b.1), a.0.max(b.0), a.1.max(b.1)], kind });
    }
    fn DoProduce(&mut self, u: Id, typ: i32, cargo: i32) { 
        let Some(&k) = self.map_kind.get(&typ) else { return };
        let w = self.map_waffe.get(&cargo).copied();
        // Ueber den Paketweg (Wiederholungsring 0x40E87B), wie Unit::DoProduce im Original.
        self.sim.structure_command(self.uid(u), StructureCommand::Produce { kind: k, weapon: w });
    }
    fn DoTransferCargo(&mut self, u: Id, bay: i32) {  if bay >= 0 { self.sim.structure_command(self.uid(u), StructureCommand::TransferCargo { bay: bay as u8 }); } }
    fn DoResearch(&mut self, u: Id, tech_index: i32, n: i32) { 
        if let Some(t) = self.tech_rules(tech_index).map(|t| t.id) { self.sim.structure_command(self.uid(u), StructureCommand::Research { tech: t, scientists: n }); }
    }
    fn DoTrainScientists(&mut self, u: Id, n: i32) {  self.sim.structure_command(self.uid(u), StructureCommand::TrainScientists { students: n }); }
    fn DoRepair(&mut self, u: Id, ziel: Id) {  self.sim.player_command(self.uid(u), Command::Repair { target: Some(self.uid(ziel)) }); }
    fn DoReprogram(&mut self, u: Id, ziel: Id) {  self.sim.player_command(self.uid(u), Command::Reprogram { target: Some(self.uid(ziel)) }); }
    fn DoIdle(&mut self, u: Id) {  self.sim.structure_command(self.uid(u), StructureCommand::Idle); }
    fn DoUnIdle(&mut self, u: Id) {  self.sim.structure_command(self.uid(u), StructureCommand::Unidle); }
    fn DoStop(&mut self, u: Id) {  self.sim.command(self.uid(u), Command::Stop); }
    fn DoLaunch(&mut self, u: Id, px: i32, py: i32, force: bool) { 
        let ziel = Some((px, py));
        if force { self.sim.launch_forced(self.uid(u), ziel); } else { self.sim.launch(self.uid(u), ziel); }
    }

    // Reihenfolge wie im Original (op2-missionhost world.rs query_units): Gebaeude in der Liste der Kolonie (nach
    // Ressourcen-Vorrang, neueste zuerst unter Gleichen), alles andere neueste zuerst.
    fn PlayerBuildingEnum(&self, p: i32, typ: i32) -> Vec<Id> {
        // Ein zerstoertes Gebaeude bleibt bis zur Loeschung (Ende der Zerstoerungsanimation) in der Gebaeudeliste des Spielers
        // (gemessen mp2_08 Seed 1: Agridom 31, tot bei 38997, bei 39003 noch aufgezaehlt, bei 39203 nicht mehr).
        let passt = |x: &op2_sim::Unit| (x.alive || x.death.is_some()) && !x.removed && x.owner as i32 == p && self.sim.stats_of(x.kind, x.owner).structure && (typ == mapNone || typ == mapAny || self.map_von_kind(x.kind) == typ);
        if let Some(c) = u8::try_from(p).ok().and_then(|q| self.sim.colony(q)) {
            return c.list.iter().filter_map(|&id| self.sim.unit(id)).filter(|x| passt(x)).map(|x| x.index as i32).collect();
        }
        self.neueste().filter(|x| passt(x)).map(|x| x.index as i32).collect()
    }
    fn PlayerVehicleEnum(&self, p: i32) -> Vec<Id> {
        self.neueste().filter(|x| x.owner as i32 == p && !self.sim.stats_of(x.kind, x.owner).structure).map(|x| x.index as i32).collect()
    }
    /// Wie OP2 (PlayerUnitEnum zaehlt nur Fahrzeuge auf; Gebaeude ueber PlayerBuildingEnum).
    fn PlayerUnitEnum(&self, p: i32) -> Vec<Id> { self.PlayerVehicleEnum(p) }
    fn InRectEnumerator(&self, r: Rect) -> Vec<Id> {
        let (a, b) = (Self::tile(Loc::new(r.x1, r.y1)), Self::tile(Loc::new(r.x2, r.y2)));
        let drin = |t: (i32, i32)| t.0 >= a.0.min(b.0) && t.0 <= a.0.max(b.0) && t.1 >= a.1.min(b.1) && t.1 <= a.1.max(b.1);
        // wie im Original: neueste zuerst, ohne Gaia-Objekte (Baken)
        self.neueste().filter(|x| drin(x.tile())).map(|x| x.index as i32).collect()
    }
    fn InRangeEnumerator(&self, l: Loc, r: i32) -> Vec<Id> {
        // Original (0x437B90): in Pixeln, Kachelecke des Orts gegen die Pixelposition, dx^2 + dy^2 < (r*32)^2; ohne Baken
        let c = Self::tile(l);
        let (cx, cy) = (c.0 as i64 * 32, c.1 as i64 * 32);
        let r2 = (r as i64 * 32).pow(2);
        self.neueste().filter(|x| (x.px as i64 - cx).pow(2) + (x.py as i64 - cy).pow(2) < r2).map(|x| x.index as i32).collect()
    }
    fn LocationEnumerator(&self, l: Loc) -> Vec<Id> {
        let c = Self::tile(l);
        // wie im Original: nur Fahrzeuge genau auf der Kachel, neueste zuerst, ohne Baken. Gemessen 2026-11-12 (Sonde,
        // mp4_12 Seed 3): eine fertige Mine auf genau dieser Kachel liefert der LocationEnumerator des Originals nie,
        // InRangeEnumerator(l, 1) schon.
        self.neueste().filter(|x| x.tile() == c && !self.sim.stats_of(x.kind, x.owner).structure).map(|x| x.index as i32).collect()
    }
    fn GroupEnumerator(&self, g: Gruppe) -> Vec<Id> {
        if g <= 0 { return Vec::new(); }
        self.sim.group(g as u32).map(|gr| gr.members.iter().filter_map(|m| self.sim.unit(m.unit).filter(|x| x.alive && !x.removed).map(|x| x.index as i32)).collect()).unwrap_or_default()
    }

    fn GetHitPoints(&self, typ: i32, p: i32) -> i32 { self.stats(typ, p).map(|s| s.hp).unwrap_or(0) }
    fn GetArmor(&self, typ: i32, p: i32) -> i32 { self.stats(typ, p).map(|s| s.armor as i32).unwrap_or(0) }
    fn GetOreCost(&self, typ: i32, p: i32) -> i32 {
        // Rohre und Waende (17..20) haben ihre Kosten in den Regeln getrennt (wall_costs, Index MapID - 17).
        if (mapTube..=mapMicrobeWall).contains(&typ) { return self.rules().wall_costs[(typ - mapTube) as usize].0; }
        self.stats(typ, p).map(|s| s.eco.common_cost).or_else(|| self.wstats(typ, p).map(|w| w.cost.0)).unwrap_or(0)
    }
    fn GetRareOreCost(&self, typ: i32, p: i32) -> i32 {
        if (mapTube..=mapMicrobeWall).contains(&typ) { return self.rules().wall_costs[(typ - mapTube) as usize].1; }
        self.stats(typ, p).map(|s| s.eco.rare_cost).or_else(|| self.wstats(typ, p).map(|w| w.cost.1)).unwrap_or(0)
    }
    fn GetWorkersRequired(&self, typ: i32, p: i32) -> i32 { self.stats(typ, p).map(|s| s.eco.workers).unwrap_or(0) }
    fn GetScientistsRequired(&self, typ: i32, p: i32) -> i32 { self.stats(typ, p).map(|s| s.eco.scientists).unwrap_or(0) }
    fn GetPowerRequired(&self, typ: i32, p: i32) -> i32 { self.stats(typ, p).map(|s| s.eco.power).unwrap_or(0) }
    fn GetWeaponRange(&self, typ: i32, p: i32) -> i32 { self.wstats(typ, p).map(|w| w.range).unwrap_or(0) }
    fn GetProductionCapacity(&self, typ: i32, p: i32) -> i32 { self.stats(typ, p).map(|s| s.eco.production).unwrap_or(0) }
    fn GetReloadTime(&self, typ: i32, p: i32) -> i32 { self.wstats(typ, p).map(|w| w.reload).unwrap_or(0) }
    fn GetPenetrationDamage(&self, typ: i32, p: i32) -> i32 { self.wstats(typ, p).map(|w| w.penetration).unwrap_or(0) }
    fn GetConcussionDamage(&self, typ: i32, p: i32) -> i32 { self.wstats(typ, p).map(|w| w.concussion).unwrap_or(0) }
    fn GetBuildTime(&self, typ: i32, p: i32) -> i32 { self.stats(typ, p).map(|s| s.eco.build_points).unwrap_or(0) }

    fn GetTechCount(&self) -> i32 { self.tech_virt.len() as i32 }
    fn GetTechInfo(&self, i: i32) -> TechInfo {
        if i == 0 { return TechInfo { valid: true, ..TechInfo::default() }; }   // Platzhalter ID 0
        let Some(t) = self.tech_rules(i) else { return TechInfo::default() };
        let tbt = tb::tech(t.id as i32);
        // ohne Voraussetzung verweist das Original auf den Platzhalter (Platz 0)
        let req: Vec<i32> = if t.requires.is_empty() { vec![0] } else { t.requires.iter().filter_map(|r| self.tech_idx.get(r).map(|&k| k as i32)).collect() };
        TechInfo { valid: true, id: t.id as i32, category: tbt.map(|x| x.kat).unwrap_or(0), eden_cost: t.cost[1], plymouth_cost: t.cost[0],
            lab: tbt.map(|x| x.lab).unwrap_or(0), max_scientists: t.max_scientists, required: Voraus::aus(&req) }
    }
    fn GetPlayerHasTech(&self, i: i32, p: i32) -> bool {
        if i == 0 { return true; }   // den Platzhalter haben alle
        let Some(t) = self.tech_rules(i) else { return false };
        self.kolonie(p).is_some_and(|c| c.has_tech(t.id))
    }

    fn IsEden(&self, p: i32) -> bool { !self.sim.is_plymouth(p as u8) }
    fn Ore(&self, p: i32) -> i32 { self.kolonie(p).map(|c| c.common).unwrap_or(0) }
    fn RareOre(&self, p: i32) -> i32 { self.kolonie(p).map(|c| c.rare).unwrap_or(0) }
    fn FoodStored(&self, p: i32) -> i32 { self.kolonie(p).map(|c| c.food).unwrap_or(0) }
    fn Workers(&self, p: i32) -> i32 { self.kolonie(p).map(|c| c.workers).unwrap_or(0) }
    fn Scientists(&self, p: i32) -> i32 { self.kolonie(p).map(|c| c.scientists).unwrap_or(0) }
    fn Kids(&self, p: i32) -> i32 { self.kolonie(p).map(|c| c.kids).unwrap_or(0) }
    fn MoraleRoh(&self, p: i32) -> i32 { self.kolonie(p).map(|c| c.morale).unwrap_or(-1) }
    fn MoraleLevel(&self, p: i32) -> i32 { self.kolonie(p).map(|c| c.level).unwrap_or(0) }
    fn HasTechnology(&self, p: i32, tech_id: i32) -> bool { self.kolonie(p).is_some_and(|c| c.has_tech(tech_id as u32)) }
    fn SetOre(&mut self, p: i32, n: i32) {  let c = self.sim.colony_mut(p as u8); c.common = n; c.recalc = true; }
    fn SetRareOre(&mut self, p: i32, n: i32) {  let c = self.sim.colony_mut(p as u8); c.rare = n; c.recalc = true; }

    fn IsAlliedTo(&self, p: i32, q: i32) -> bool { p != q && self.sim.ally_mask(p as u8) & (1 << q) != 0 }
    fn GetMaxOre(&self, p: i32) -> i32 { self.kolonie(p).map(|c| c.max_common).unwrap_or(0) }
    fn GetMaxRareOre(&self, p: i32) -> i32 { self.kolonie(p).map(|c| c.max_rare).unwrap_or(0) }
    fn GetNumAvailableWorkers(&self, p: i32) -> i32 { self.kolonie(p).map(|c| c.avail_workers).unwrap_or(0) }
    fn GetNumAvailableScientists(&self, p: i32) -> i32 { self.kolonie(p).map(|c| c.avail_scientists).unwrap_or(0) }
    fn GetAmountPowerGenerated(&self, p: i32) -> i32 { self.kolonie(p).map(|c| c.generated).unwrap_or(0) }
    fn GetAmountPowerConsumed(&self, p: i32) -> i32 { self.kolonie(p).map(|c| c.consumed).unwrap_or(0) }
    fn GetAmountPowerAvailable(&self, p: i32) -> i32 { self.kolonie(p).map(|c| c.available).unwrap_or(0) }
    fn GetNumIdleBuildings(&self, p: i32) -> i32 { self.kolonie(p).map(|c| c.idle).unwrap_or(0) }
    fn GetNumActiveBuildings(&self, p: i32) -> i32 { self.kolonie(p).map(|c| c.active).unwrap_or(0) }
    fn GetNumUnpoweredStructures(&self, p: i32) -> i32 { self.kolonie(p).map(|c| c.unpowered).unwrap_or(0) }
    fn GetNumWorkersRequired(&self, p: i32) -> i32 { self.kolonie(p).map(|c| c.workers_required).unwrap_or(0) }
    fn GetNumScientistsRequired(&self, p: i32) -> i32 { self.kolonie(p).map(|c| c.scientists_required).unwrap_or(0) }
    fn GetNumScientistsAssignedToResearch(&self, p: i32) -> i32 { self.kolonie(p).map(|c| c.researching).unwrap_or(0) }
    fn GetNetFoodProduction(&self, p: i32) -> i32 { self.kolonie(p).map(|c| c.food_net).unwrap_or(0) }
    fn GetTotalResidenceCapacity(&self, p: i32) -> i32 { self.kolonie(p).map(|c| c.caps[caps::RESIDENCE]).unwrap_or(0) }
    fn GetTotalMedCenterCapacity(&self, p: i32) -> i32 { self.kolonie(p).map(|c| c.caps[caps::MEDICAL]).unwrap_or(0) }
    fn GetTotalRecreationFacilityCapacity(&self, p: i32) -> i32 { self.kolonie(p).map(|c| c.caps[caps::RECREATION]).unwrap_or(0) }
    fn GetTotalForumCapacity(&self, p: i32) -> i32 { self.kolonie(p).map(|c| c.caps[caps::FORUM]).unwrap_or(0) }
    fn GetSatelliteCount(&self, p: i32, typ: i32) -> i32 {
        self.kolonie(p).map(|c| if typ == mapSolarSatellite { c.solar_satellites } else if typ == mapEDWARDSatellite { c.edward_satellites } else { 0 }).unwrap_or(0)
    }

    fn CreateFightGroup(&mut self, p: i32) -> Gruppe {  let g = self.naechste_gruppe; self.naechste_gruppe += 1; self.sim.create_group(g as u32, GroupKind::Fight, p as u8); self.gruppen.insert(g, GroupKind::Fight); g }
    fn CreateBuildingGroup(&mut self, p: i32) -> Gruppe {  let g = self.naechste_gruppe; self.naechste_gruppe += 1; self.sim.create_group(g as u32, GroupKind::Building, p as u8); self.gruppen.insert(g, GroupKind::Building); g }
    fn CreateMiningGroup(&mut self, p: i32) -> Gruppe {  let g = self.naechste_gruppe; self.naechste_gruppe += 1; self.sim.create_group(g as u32, GroupKind::Mining, p as u8); self.gruppen.insert(g, GroupKind::Mining); g }
    fn Destroy(&mut self, g: Gruppe) {  if g > 0 { self.sim.destroy_group(g as u32); self.gruppen.remove(&g); } }
    fn TakeUnit(&mut self, g: Gruppe, u: Id) {  if g > 0 && self.unit(u).is_some() { self.sim.group_add_unit(g as u32, self.uid(u)); } }
    fn TakeAllUnits(&mut self, g: Gruppe, quelle: Gruppe) {  if g > 0 && quelle > 0 { self.sim.group_take_all(g as u32, quelle as u32); } }
    fn RemoveUnit(&mut self, g: Gruppe, u: Id) {  if g > 0 { self.sim.group_remove_unit(g as u32, self.uid(u)); } }
    fn TotalUnitCount(&self, g: Gruppe) -> i32 { self.GroupEnumerator(g).len() as i32 }
    fn SetRect(&mut self, g: Gruppe, r: Rect) { 
        let (a, b) = (Self::tile(Loc::new(r.x1, r.y1)), Self::tile(Loc::new(r.x2, r.y2)));
        let rr = (a.0, a.1, b.0, b.1);
        match self.gruppen.get(&g) { Some(GroupKind::Fight) => self.sim.fight_set_rect(g as u32, rr), Some(GroupKind::Building) => self.sim.building_set_rect(g as u32, rr), _ => {} }
    }
    fn DoGuardRect(&mut self, g: Gruppe) {  if g > 0 { self.sim.fight_set_objective(g as u32, Objective::GuardRect); } }
    fn AddGuardedRect(&mut self, g: Gruppe, r: Rect) { 
        let (a, b) = (Self::tile(Loc::new(r.x1, r.y1)), Self::tile(Loc::new(r.x2, r.y2)));
        if g > 0 { self.sim.fight_add_guarded_rect(g as u32, (a.0, a.1, b.0, b.1)); }
    }
    fn SetTargCount(&mut self, g: Gruppe, typ: i32, waffe: i32, n: i32) { 
        if let (true, Some(&k)) = (g > 0, self.map_kind.get(&typ)) { self.sim.group_set_targ_count(g as u32, k, if waffe > 0 { Some(waffe) } else { None }, n); }
    }
    fn ClearTargCount(&mut self, g: Gruppe) {  if g > 0 { self.sim.group_clear_targ_count(g as u32); } }
    fn RecordBuilding(&mut self, g: Gruppe, l: Loc, typ: i32, waffe: i32) { 
        if let (true, Some(&k)) = (g > 0, self.map_kind.get(&typ)) { self.sim.building_record(g as u32, Self::tile(l), k, waffe, None); }
    }
    fn RecordTube(&mut self, g: Gruppe, l: Loc) {  if g > 0 { self.sim.building_record_wall(g as u32, Self::tile(l), mapTube); } }
    fn RecordWall(&mut self, g: Gruppe, l: Loc, typ: i32) {  if g > 0 { self.sim.building_record_wall(g as u32, Self::tile(l), typ); } }
    fn RecordVehReinforceGroup(&mut self, g: Gruppe, ziel: Gruppe, prio: i32) {  if g > 0 && ziel > 0 { self.sim.building_reinforce(g as u32, ziel as u32, prio); } }
    fn Setup(&mut self, g: Gruppe, mine: Loc, schmelze: Loc, r: Rect) { 
        let (Some(m), Some(s)) = (self.gebaeude_an(mine), self.gebaeude_an(schmelze)) else { return };
        let (a, b) = (Self::tile(Loc::new(r.x1, r.y1)), Self::tile(Loc::new(r.x2, r.y2)));
        if g > 0 { self.sim.mining_setup(g as u32, m, s, (a.0, a.1, b.0, b.1)); }
    }

    fn kiRaw(&mut self, zeile: &str) { if let Some(f) = self.spur.as_mut() { use std::io::Write; let _ = writeln!(f, "{zeile}"); } }
    fn logZeile(&mut self, zeile: &str) { if self.spur.is_some() { self.log.push(zeile.to_string()); } }
    fn dateiLesen(&self, name: &str) -> Option<String> {
        if let Some(t) = self.dateien.get(name) { return Some(t.clone()); }
        self.ordner.as_ref().and_then(|o| std::fs::read_to_string(o.join(name)).ok())
    }
    fn dateiSchreiben(&mut self, name: &str, text: &str) {
        self.dateien.insert(name.to_string(), text.to_string());
        if let Some(o) = &self.ordner { let _ = std::fs::write(o.join(name), text); }
    }
}
