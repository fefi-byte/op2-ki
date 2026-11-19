#!/usr/bin/env python3
"""Erzeugt die Bruecke zwischen Missions-DLL und op2ki.dll aus dem Trait `Welt` (kern/src/cpp/welt.rs):

  dll/src/extern_welt.rs              Rust: `Welt` ueber die Rufschnittstelle der Mission
  missionen/gemeinsam/op2ki_bruecke.hpp   C++: die Rufschnittstelle auf die OP2-API (OP2MissionSDK/HFL)

Jede Methode hat eine feste Rufnummer (CODES), damit aeltere Missions-DLLs mit einer neueren op2ki.dll laufen. Eine neue
Methode im Trait braucht einen C++-Ausdruck in CPP und wird am Ende von CODES angehaengt; entfernte Methoden behalten ihre
Nummer als Luecke. Aufruf aus dem Repo-Wurzelordner: python3 werkzeug/bruecke.py
"""
import re
import sys
from pathlib import Path

WURZEL = Path(__file__).resolve().parent.parent

# Feste Rufnummern (Index = Nummer). Nur anhaengen, nie umsortieren.
CODES = [
    'Tick', 'NoPlayers', 'LocalPlayer', 'GetRand', 'FreeMoraleLevel', 'GetCellType', 'CreateUnit', 'CreateBeacon',
    'hoechsteEinheit', 'IsLive', 'GetType', 'OwnerID', 'IsVehicle', 'IsBuilding', 'IsBusy', 'isEMPed', 'Location',
    'GetDamage', 'GetWeapon', 'GetCargo', 'GetConVecCargoTurret', 'GetCurAction', 'GetLastCommand', 'HasPower',
    'HasWorkers', 'HasScientists', 'GetLabCurrentTopic', 'GetLabScientistCount', 'GetFactoryCargo', 'GetFactoryCargoWeapon',
    'GetDockLocation', 'GetCargoAmount', 'GetCargoType', 'GetNumTruckLoadsSoFar', 'GetBarYield', 'GetOreType',
    'GetSurveyedBy', 'GetWorkersInTraining', 'GetLaunchPadCargo', 'SetLabScientistCount', 'SetLaunchPadCargo',
    'SetFactoryCargo', 'DoMove', 'DoAttack', 'DoBuild', 'DoDeployMiner', 'DoDock', 'DoBuildWall', 'DoProduce',
    'DoTransferCargo', 'DoResearch', 'DoTrainScientists', 'DoRepair', 'DoReprogram', 'DoIdle', 'DoUnIdle', 'DoStop',
    'DoLaunch', 'DoSelfDestruct', 'DoPoof', 'PlayerBuildingEnum', 'PlayerVehicleEnum', 'PlayerUnitEnum',
    'InRectEnumerator', 'InRangeEnumerator', 'LocationEnumerator', 'GroupEnumerator', 'GetHitPoints', 'GetArmor',
    'GetOreCost', 'GetRareOreCost', 'GetWorkersRequired', 'GetScientistsRequired', 'GetPowerRequired', 'GetWeaponRange',
    'GetProductionCapacity', 'GetReloadTime', 'GetPenetrationDamage', 'GetConcussionDamage', 'GetBuildTime',
    'GetTechCount', 'GetTechInfo', 'GetPlayerHasTech', 'IsEden', 'Ore', 'RareOre', 'FoodStored', 'Workers', 'Scientists',
    'Kids', 'MoraleLevel', 'MoraleRoh', 'HasTechnology', 'SetOre', 'SetRareOre', 'CenterViewOn', 'IsAlliedTo',
    'GetMaxOre', 'GetMaxRareOre', 'GetNumAvailableWorkers', 'GetNumAvailableScientists', 'GetAmountPowerGenerated',
    'GetAmountPowerConsumed', 'GetAmountPowerAvailable', 'GetNumIdleBuildings', 'GetNumActiveBuildings',
    'GetNumUnpoweredStructures', 'GetNumWorkersRequired', 'GetNumScientistsRequired', 'GetNumScientistsAssignedToResearch',
    'GetNetFoodProduction', 'GetTotalResidenceCapacity', 'GetTotalMedCenterCapacity', 'GetTotalRecreationFacilityCapacity',
    'GetTotalForumCapacity', 'GetSatelliteCount', 'RecalculateValues', 'CreateFightGroup', 'CreateBuildingGroup',
    'CreateMiningGroup', 'Destroy', 'TakeUnit', 'RemoveUnit', 'TakeAllUnits', 'TotalUnitCount', 'SetRect', 'DoGuardRect',
    'AddGuardedRect', 'SetTargCount', 'ClearTargCount', 'RecordBuilding', 'RecordTube', 'RecordWall',
    'RecordVehReinforceGroup', 'Setup', 'kiRaw', 'logZeile', 'dateiLesen', 'dateiSchreiben', 'zeitEMP', 'zeitSchaum',
]

# Methoden ohne Ruf: Text (Log, KI-Spur) und Dateien gehen ueber eigene Funktionszeiger
TEXT = {'kiRaw', 'logZeile', 'dateiLesen', 'dateiSchreiben'}

# C++-Rumpf je Methode im switch der Rufschnittstelle: a[i] = Argumente, U = UnitEx aus a[0], o/no = Ausgabefeld
CPP = {
    'Tick': 'return TethysGame::Tick();',
    'NoPlayers': 'return TethysGame::NoPlayers();',
    'LocalPlayer': 'return TethysGame::LocalPlayer();',
    'FreeMoraleLevel': 'TethysGame::FreeMoraleLevel(a[0]); return 0;',
    'GetCellType': 'return GameMap::GetCellType(LOCATION(a[0], a[1]));',
    'CreateUnit': '{ Unit u; TethysGame::CreateUnit(u, (map_id)a[0], LOCATION(a[1], a[2]), a[3], (map_id)a[4], a[5]); return u.unitID; }',
    'CreateBeacon': '{ return TethysGame::CreateBeacon((map_id)a[0], a[1], a[2], a[3], a[4], a[5]); }',
    'hoechsteEinheit': 'return op2ki::hoechsteEinheit();',
    'IsLive': 'return U.IsLive() ? 1 : 0;',
    'GetType': 'return U.GetType();',
    'OwnerID': 'return U.OwnerID();',
    'IsVehicle': 'return U.IsVehicle() ? 1 : 0;',
    'IsBuilding': 'return U.IsBuilding() ? 1 : 0;',
    'IsBusy': 'return U.IsBusy() ? 1 : 0;',
    'isEMPed': 'return U.isEMPed() ? 1 : 0;',
    'Location': '{ const LOCATION l = U.Location(); o[0] = l.x; o[1] = l.y; return 2; }',
    'GetDamage': 'return U.GetDamage();',
    'GetWeapon': 'return U.GetWeapon();',
    'GetCargo': 'return U.GetCargo();',
    'GetCurAction': 'return U.GetCurAction();',
    'GetLastCommand': 'return U.GetLastCommand();',
    'HasPower': 'return U.HasPower() ? 1 : 0;',
    'HasWorkers': 'return U.HasWorkers() ? 1 : 0;',
    'HasScientists': 'return U.HasScientists() ? 1 : 0;',
    'GetLabCurrentTopic': 'return U.GetLabCurrentTopic();',
    'GetLabScientistCount': 'return U.GetLabScientistCount();',
    'GetFactoryCargo': 'return U.GetFactoryCargo(a[1]);',
    'GetDockLocation': '{ const LOCATION l = U.GetDockLocation(); o[0] = l.x; o[1] = l.y; return 2; }',
    'GetCargoAmount': 'return U.GetCargoAmount();',
    'GetCargoType': 'return (int)U.GetCargoType();',
    'GetNumTruckLoadsSoFar': 'return U.GetNumTruckLoadsSoFar();',
    'GetBarYield': 'return U.GetBarYield();',
    'GetOreType': 'return U.GetOreType();',
    'GetSurveyedBy': 'return U.GetSurveyedBy();',
    'GetWorkersInTraining': 'return U.GetWorkersInTraining();',
    'GetLaunchPadCargo': 'return U.GetLaunchPadCargo();',
    'SetLabScientistCount': 'U.SetLabScientistCount(a[1]); return 0;',
    'SetLaunchPadCargo': 'U.SetLaunchPadCargo((map_id)a[1]); return 0;',
    'SetFactoryCargo': 'U.SetFactoryCargo(a[1], (map_id)a[2], (map_id)a[3]); return 0;',
    'DoMove': 'U.DoMove(LOCATION(a[1], a[2])); return 0;',
    'DoAttack': '{ UnitEx z; z.unitID = a[1]; U.Unit::DoAttack(z); return 0; }',
    'DoBuild': 'U.DoBuild((map_id)a[1], LOCATION(a[2], a[3])); return 0;',
    'DoDeployMiner': 'U.DoDeployMiner(LOCATION(a[1], a[2])); return 0;',
    'DoDock': 'U.DoDock(LOCATION(a[1], a[2])); return 0;',
    'DoBuildWall': '{ MAP_RECT r(LOCATION(a[2], a[3]), LOCATION(a[4], a[5])); U.DoBuildWall((map_id)a[1], r); return 0; }',
    'DoProduce': 'U.DoProduce((map_id)a[1], (map_id)a[2]); return 0;',
    'DoTransferCargo': 'U.DoTransferCargo(a[1]); return 0;',
    'DoResearch': 'U.DoResearch(a[1], a[2]); return 0;',
    'DoTrainScientists': 'U.DoTrainScientists(a[1]); return 0;',
    'DoRepair': '{ UnitEx z; z.unitID = a[1]; U.DoRepair(z); return 0; }',
    'DoReprogram': '{ UnitEx z; z.unitID = a[1]; U.DoReprogram(z); return 0; }',
    'DoIdle': 'U.DoIdle(); return 0;',
    'DoUnIdle': 'U.DoUnIdle(); return 0;',
    'DoStop': 'U.DoStop(); return 0;',
    'DoLaunch': 'U.DoLaunch(a[1], a[2], a[3]); return 0;',
    'PlayerBuildingEnum': '{ int n = 0; if (a[1] == mapNone || a[1] == mapAny) { for (int t : op2ki::GEBAEUDE) { PlayerBuildingEnum e(a[0], (map_id)t); UnitEx u; while (e.GetNext(u) && n < no) o[n++] = u.unitID; } } else { PlayerBuildingEnum e(a[0], (map_id)a[1]); UnitEx u; while (e.GetNext(u) && n < no) o[n++] = u.unitID; } return n; }',
    'PlayerVehicleEnum': '{ int n = 0; PlayerVehicleEnum e(a[0]); UnitEx u; while (e.GetNext(u) && n < no) o[n++] = u.unitID; return n; }',
    'PlayerUnitEnum': '{ int n = 0; PlayerUnitEnum e(a[0]); UnitEx u; while (e.GetNext(u) && n < no) o[n++] = u.unitID; return n; }',
    'InRectEnumerator': '{ int n = 0; MAP_RECT r(LOCATION(a[0], a[1]), LOCATION(a[2], a[3])); InRectEnumerator e(r); UnitEx u; while (e.GetNext(u) && n < no) o[n++] = u.unitID; return n; }',
    'InRangeEnumerator': '{ int n = 0; InRangeEnumerator e(LOCATION(a[0], a[1]), a[2]); UnitEx u; while (e.GetNext(u) && n < no) o[n++] = u.unitID; return n; }',
    'LocationEnumerator': '{ int n = 0; LocationEnumerator e(LOCATION(a[0], a[1])); UnitEx u; while (e.GetNext(u) && n < no) o[n++] = u.unitID; return n; }',
    'GroupEnumerator': '{ int n = 0; ScGroup* g = gruppe(a[0]); if (!g) return 0; GroupEnumerator e(*g); UnitEx u; while (e.GetNext(u) && n < no) o[n++] = u.unitID; return n; }',
    'GetTechCount': 'return Research::GetTechCount();',
    'GetTechInfo': '{ TechInfo ti = Research::GetTechInfo(a[0]); if (!ti.IsValid()) { o[0] = 0; return 1; } o[0] = 1; o[1] = ti.GetTechID(); o[2] = ti.GetCategory(); o[3] = ti.GetEdenCost(); o[4] = ti.GetPlymouthCost(); o[5] = ti.GetLab(); o[6] = ti.GetMaxScientists(); const int nr = ti.GetNumRequiredTechs(); o[7] = nr; for (int k = 0; k < nr && 8 + k < no; ++k) o[8 + k] = ti.GetRequiredTechIndex(k); return 8 + nr; }',
    'GetPlayerHasTech': '{ TechInfo ti = Research::GetTechInfo(a[0]); return ti.IsValid() && (ti.GetPlayerHasTech() & (1 << a[1])) ? 1 : 0; }',
    'IsEden': 'return Player[a[0]].IsEden() ? 1 : 0;',
    'Ore': 'return Player[a[0]].Ore();',
    'RareOre': 'return Player[a[0]].RareOre();',
    'FoodStored': 'return Player[a[0]].FoodStored();',
    'Workers': 'return Player[a[0]].Workers();',
    'Scientists': 'return Player[a[0]].Scientists();',
    'Kids': 'return Player[a[0]].Kids();',
    'MoraleLevel': 'return (int)Player[a[0]].MoraleLevel();',
    'MoraleRoh': '{ if (isInited && playerArray) { const int* r = reinterpret_cast<const int*>(reinterpret_cast<const char*>(playerArray) + a[0] * 3108); if (r[16] >= 0 && r[16] <= 100) return r[16]; } return -1; }',
    'HasTechnology': 'return op2ki::techIndex(a[1]) >= 0 && Player[a[0]].HasTechnology(a[1]) ? 1 : 0;',
    'SetOre': 'Player[a[0]].SetOre(a[1]); return 0;',
    'SetRareOre': 'Player[a[0]].SetRareOre(a[1]); return 0;',
    'CenterViewOn': 'Player[a[0]].CenterViewOn(a[1], a[2]); return 0;',
    'IsAlliedTo': 'return ExtPlayer[a[0]].IsAlliedTo(a[1]) ? 1 : 0;',
    'GetSatelliteCount': 'return ExtPlayer[a[0]].GetSatelliteCount((map_id)a[1]);',
    'RecalculateValues': 'ExtPlayer[a[0]].RecalculateValues(); return 0;',
    'CreateFightGroup': '{ FightGroup g = CreateFightGroup(Player[a[0]]); return neueGruppe(g, 1); }',
    'CreateBuildingGroup': '{ BuildingGroup g = CreateBuildingGroup(Player[a[0]]); return neueGruppe(g, 2); }',
    'CreateMiningGroup': '{ MiningGroup g = CreateMiningGroup(Player[a[0]]); return neueGruppe(g, 3); }',
    'Destroy': '{ ScGroup* g = gruppe(a[0]); if (g) g->Destroy(); return 0; }',
    'TakeUnit': '{ ScGroup* g = gruppe(a[0]); UnitEx u; u.unitID = a[1]; if (g) g->TakeUnit(u); return 0; }',
    'RemoveUnit': '{ ScGroup* g = gruppe(a[0]); UnitEx u; u.unitID = a[1]; if (g) g->RemoveUnit(u); return 0; }',
    'TakeAllUnits': '{ ScGroup* g = gruppe(a[0]); ScGroup* q = gruppe(a[1]); if (g && q) g->TakeAllUnits(*q); return 0; }',
    'TotalUnitCount': '{ ScGroup* g = gruppe(a[0]); return g ? g->TotalUnitCount() : 0; }',
    'SetRect': '{ ScGroup* g = gruppe(a[0]); MAP_RECT r(LOCATION(a[1], a[2]), LOCATION(a[3], a[4])); if (g && art[a[0]] == 1) ((FightGroup*)g)->SetRect(r); else if (g && art[a[0]] == 2) ((BuildingGroup*)g)->SetRect(r); return 0; }',
    'DoGuardRect': '{ ScGroup* g = gruppe(a[0]); if (g && art[a[0]] == 1) ((FightGroup*)g)->DoGuardRect(); return 0; }',
    'AddGuardedRect': '{ ScGroup* g = gruppe(a[0]); MAP_RECT r(LOCATION(a[1], a[2]), LOCATION(a[3], a[4])); if (g && art[a[0]] == 1) ((FightGroup*)g)->AddGuardedRect(r); return 0; }',
    'SetTargCount': '{ ScGroup* g = gruppe(a[0]); if (g) g->SetTargCount((map_id)a[1], (map_id)a[2], a[3]); return 0; }',
    'ClearTargCount': '{ ScGroup* g = gruppe(a[0]); if (g) g->ClearTargCount(); return 0; }',
    'RecordBuilding': '{ ScGroup* g = gruppe(a[0]); if (g && art[a[0]] == 2) { LOCATION l(a[1], a[2]); ((BuildingGroup*)g)->RecordBuilding(l, (map_id)a[3], (map_id)a[4]); } return 0; }',
    'RecordTube': '{ ScGroup* g = gruppe(a[0]); if (g && art[a[0]] == 2) { LOCATION l(a[1], a[2]); ((BuildingGroup*)g)->RecordTube(l); } return 0; }',
    'RecordWall': '{ ScGroup* g = gruppe(a[0]); if (g && art[a[0]] == 2) { LOCATION l(a[1], a[2]); ((BuildingGroup*)g)->RecordWall(l, (map_id)a[3]); } return 0; }',
    'RecordVehReinforceGroup': '{ ScGroup* g = gruppe(a[0]); ScGroup* z = gruppe(a[1]); if (g && z && art[a[0]] == 2) ((BuildingGroup*)g)->RecordVehReinforceGroup(*z, a[2]); return 0; }',
    'Setup': '{ ScGroup* g = gruppe(a[0]); if (g && art[a[0]] == 3) { LOCATION m(a[1], a[2]), s(a[3], a[4]); MAP_RECT r(LOCATION(a[5], a[6]), LOCATION(a[7], a[8])); ((MiningGroup*)g)->Setup(m, s, r); } return 0; }',
    'zeitEMP': '{ const int v = U.GetTimeEMPed(); return v > 0 ? v : 0; }',
    'zeitSchaum': '{ const int v = U.GetTimeStickyfoamed(); return v > 0 ? v : 0; }',
}
for n in ['GetMaxOre', 'GetMaxRareOre', 'GetNumAvailableWorkers', 'GetNumAvailableScientists', 'GetAmountPowerGenerated',
          'GetAmountPowerConsumed', 'GetAmountPowerAvailable', 'GetNumIdleBuildings', 'GetNumActiveBuildings',
          'GetNumUnpoweredStructures', 'GetNumWorkersRequired', 'GetNumScientistsRequired',
          'GetNumScientistsAssignedToResearch', 'GetNetFoodProduction', 'GetTotalResidenceCapacity',
          'GetTotalMedCenterCapacity', 'GetTotalRecreationFacilityCapacity', 'GetTotalForumCapacity']:
    CPP[n] = f'return ExtPlayer[a[0]].{n}();'
for n in ['GetHitPoints', 'GetArmor', 'GetOreCost', 'GetRareOreCost', 'GetWorkersRequired', 'GetScientistsRequired',
          'GetPowerRequired', 'GetWeaponRange', 'GetProductionCapacity', 'GetReloadTime', 'GetPenetrationDamage',
          'GetConcussionDamage', 'GetBuildTime']:
    CPP[n] = f'{{ UnitInfo i((map_id)a[0]); return i.{n}(a[1]); }}'


def methoden():
    """(Name, self-Art, [(Param, Typ)], Rueckgabe) je Methode des Traits Welt, in Reihenfolge."""
    text = (WURZEL / 'kern/src/cpp/welt.rs').read_text()
    rumpf = text[text.index('pub trait Welt {'):]
    rumpf = rumpf[:rumpf.index('\n}\n')]
    aus = []
    for m in re.finditer(r'^\s*fn (\w+)\((&(?:mut )?self)((?:, [^)]*)?)\)(?: -> ([^;{]+?))?\s*[;{]', rumpf, re.M):
        name, selbst, rest, rueck = m.group(1), m.group(2), m.group(3), (m.group(4) or '').strip()
        params = [tuple(x.strip() for x in p.split(':', 1)) for p in rest.split(', ')[1:] if p.strip()]
        params = [(p.lstrip('_'), t) for p, t in params]
        aus.append((name, selbst, params, rueck))
    return aus


def code(name):
    if name not in CODES:
        sys.exit(f'Neue Methode {name}: in werkzeug/bruecke.py am Ende von CODES eintragen und in CPP beschreiben')
    return CODES.index(name)


def cpp(ms):
    z = ['// ERZEUGT von werkzeug/bruecke.py aus dem Trait Welt (kern/src/cpp/welt.rs). Nicht von Hand aendern.',
         '// Rufschnittstelle der Missions-DLL fuer op2ki.dll: Code + int-Argumente -> OP2-API (OP2MissionSDK/HFL).',
         '#pragma once',
         'namespace rsapi {',
         'static ScGroup* gruppen[256]; static int art[256]; static int nGruppen = 1;   // art: 1 Fight, 2 Building, 3 Mining',
         'template <class G> inline int neueGruppe(G& g, int a) { if (nGruppen >= 256) return 0; G* p = new G(g); gruppen[nGruppen] = p; art[nGruppen] = a; return nGruppen++; }',
         'inline ScGroup* gruppe(int i) { return i > 0 && i < nGruppen ? gruppen[i] : nullptr; }',
         'static int __cdecl ruf(int code, const int* a, int na, int* o, int no) {',
         '  UnitEx U; U.unitID = na > 0 ? a[0] : 0;',
         '  switch (code) {']
    for name, _, _, _ in sorted(ms, key=lambda m: code(m[0])):
        if name in TEXT:
            continue
        if name not in CPP:
            sys.exit(f'Methode {name}: kein C++-Ausdruck in CPP')
        z.append(f'    case {code(name)}: {CPP[name]}   // {name}')
    z += ['  }', '  return 0;', '}',
          'static void __cdecl text(int code, const char* s, int n) {',
          '  static char puf[16384]; if (n > 16383) n = 16383; std::memcpy(puf, s, n); puf[n] = 0;',
          f'  if (code == {code("kiRaw")}) {{ op2::log::kiRaw(puf, n); }} else op2::log::linef("%s", puf);',
          '}',
          'static int __cdecl lesen(const char* name, int nn, char* out, int max) {',
          '  char nm[260]; if (nn > 259) nn = 259; std::memcpy(nm, name, nn); nm[nn] = 0; char pfad[600]; op2ki::wissenPfad(pfad, sizeof pfad, nm);',
          '  FILE* f = std::fopen(pfad, "rb");',
          '  if (!f) return -1;',
          '  const int k = (int)std::fread(out, 1, max, f); std::fclose(f); return k;',
          '}',
          'static void __cdecl schreiben(const char* name, int nn, const char* s, int n) {',
          '  char nm[260]; if (nn > 259) nn = 259; std::memcpy(nm, name, nn); nm[nn] = 0; char pfad[600]; op2ki::wissenPfad(pfad, sizeof pfad, nm);',
          '  FILE* f = std::fopen(pfad, "wb"); if (!f) return; std::fwrite(s, 1, n, f); std::fclose(f);',
          '}',
          '} // namespace rsapi']
    return '\n'.join(z) + '\n'


def flach(n, t):
    return {'Loc': f'{n}.x, {n}.y', 'Rect': f'{n}.x1, {n}.y1, {n}.x2, {n}.y2', 'bool': f'{n} as i32'}.get(t, n)


def rust(ms):
    z = ['//! ERZEUGT von werkzeug/bruecke.py aus dem Trait Welt: `Welt` ueber die Rufschnittstelle der Missions-DLL.',
         '//! Nicht von Hand aendern.',
         '#![allow(non_snake_case)]',
         'use op2ki::cpp::welt::{Gruppe, Id, Loc, Rect, TechInfo, Voraus, Welt};',
         '',
         'pub type RufFn = unsafe extern "C" fn(code: i32, a: *const i32, na: i32, o: *mut i32, no: i32) -> i32;',
         'pub type TextFn = unsafe extern "C" fn(code: i32, s: *const u8, n: i32);',
         'pub type LesenFn = unsafe extern "C" fn(name: *const u8, nn: i32, out: *mut u8, max: i32) -> i32;',
         'pub type SchreibenFn = unsafe extern "C" fn(name: *const u8, nn: i32, s: *const u8, n: i32);',
         '#[repr(C)] #[derive(Clone, Copy)] pub struct Api { pub ruf: RufFn, pub text: TextFn, pub lesen: LesenFn, pub schreiben: SchreibenFn }',
         'pub struct ExternWelt { pub api: Api, puf: std::cell::RefCell<Vec<i32>> }',
         'impl ExternWelt {',
         '    pub fn neu(api: Api) -> Self { ExternWelt { api, puf: std::cell::RefCell::new(vec![0; 4096]) } }',
         '    fn r(&self, code: i32, a: &[i32]) -> i32 { let mut o = self.puf.borrow_mut(); unsafe { (self.api.ruf)(code, a.as_ptr(), a.len() as i32, o.as_mut_ptr(), o.len() as i32) } }',
         '    fn rv(&self, code: i32, a: &[i32]) -> Vec<i32> { let mut o = self.puf.borrow_mut(); let n = unsafe { (self.api.ruf)(code, a.as_ptr(), a.len() as i32, o.as_mut_ptr(), o.len() as i32) }; o[..n.max(0) as usize].to_vec() }',
         '    fn rl(&self, code: i32, a: &[i32]) -> Loc { let v = self.rv(code, a); if v.len() >= 2 { Loc::new(v[0], v[1]) } else { Loc::default() } }',
         '}',
         'impl Welt for ExternWelt {']
    for name, selbst, params, rueck in ms:
        i = code(name)
        sig = f'    fn {name}({selbst}' + ''.join(f', {p}: {t}' for p, t in params) + ')' + (f' -> {rueck}' if rueck else '')
        args = ', '.join(flach(p, t) for p, t in params)
        if name in ('kiRaw', 'logZeile'):
            body = f'unsafe {{ (self.api.text)({i}, zeile.as_ptr(), zeile.len() as i32) }}'
        elif name == 'dateiLesen':
            body = ('let mut b = vec![0u8; 1 << 20]; let k = unsafe { (self.api.lesen)(name.as_ptr(), name.len() as i32, '
                    'b.as_mut_ptr(), b.len() as i32) }; if k < 0 { None } else { b.truncate(k as usize); '
                    'Some(String::from_utf8_lossy(&b).into_owned()) }')
        elif name == 'dateiSchreiben':
            body = 'unsafe { (self.api.schreiben)(name.as_ptr(), name.len() as i32, text.as_ptr(), text.len() as i32) }'
        elif rueck == 'Vec<Id>':
            body = f'self.rv({i}, &[{args}])'
        elif rueck == 'Loc':
            body = f'self.rl({i}, &[{args}])'
        elif rueck == 'bool':
            body = f'self.r({i}, &[{args}]) != 0'
        elif rueck == 'TechInfo':
            body = (f'let v = self.rv({i}, &[{args}]); if v.is_empty() || v[0] == 0 {{ return TechInfo::default(); }} '
                    'TechInfo { valid: true, id: v[1], category: v[2], eden_cost: v[3], plymouth_cost: v[4], lab: v[5], '
                    'max_scientists: v[6], required: Voraus::aus(&v[8..(8 + v[7].max(0) as usize).min(v.len())]) }')
        elif rueck == '':
            body = f'self.r({i}, &[{args}]);'
        elif name in ('zeitEMP', 'zeitSchaum'):
            body = f'self.r({i}, &[{args}]).max(0)'
        else:
            body = f'self.r({i}, &[{args}])'
        z.append(f'{sig} {{ {body} }}')
    z.append('}')
    return '\n'.join(z) + '\n'


if __name__ == '__main__':
    ms = methoden()
    (WURZEL / 'dll/src/extern_welt.rs').write_text(rust(ms))
    (WURZEL / 'missionen/gemeinsam/op2ki_bruecke.hpp').write_text(cpp(ms))
    print(f'{len(ms)} Methoden: dll/src/extern_welt.rs, missionen/gemeinsam/op2ki_bruecke.hpp')
