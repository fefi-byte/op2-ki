// ERZEUGT von werkzeug/bruecke.py aus dem Trait Welt (kern/src/cpp/welt.rs). Nicht von Hand aendern.
// Rufschnittstelle der Missions-DLL fuer op2ki.dll: Code + int-Argumente -> OP2-API (OP2MissionSDK/HFL).
#pragma once
namespace rsapi {
static ScGroup* gruppen[256]; static int art[256]; static int nGruppen = 1;   // art: 1 Fight, 2 Building, 3 Mining
template <class G> inline int neueGruppe(G& g, int a) { if (nGruppen >= 256) return 0; G* p = new G(g); gruppen[nGruppen] = p; art[nGruppen] = a; return nGruppen++; }
inline ScGroup* gruppe(int i) { return i > 0 && i < nGruppen ? gruppen[i] : nullptr; }
static int __cdecl ruf(int code, const int* a, int na, int* o, int no) {
  UnitEx U; U.unitID = na > 0 ? a[0] : 0;
  switch (code) {
    case 0: return TethysGame::Tick();   // Tick
    case 1: return TethysGame::NoPlayers();   // NoPlayers
    case 2: return TethysGame::LocalPlayer();   // LocalPlayer
    case 3: return TethysGame::GetRand(a[0]);   // GetRand
    case 4: TethysGame::FreeMoraleLevel(a[0]); return 0;   // FreeMoraleLevel
    case 5: return GameMap::GetCellType(LOCATION(a[0], a[1]));   // GetCellType
    case 6: { Unit u; TethysGame::CreateUnit(u, (map_id)a[0], LOCATION(a[1], a[2]), a[3], (map_id)a[4], a[5]); return u.unitID; }   // CreateUnit
    case 7: { return TethysGame::CreateBeacon((map_id)a[0], a[1], a[2], a[3], a[4], a[5]); }   // CreateBeacon
    case 8: return op2ki::hoechsteEinheit();   // hoechsteEinheit
    case 9: return U.IsLive() ? 1 : 0;   // IsLive
    case 10: return U.GetType();   // GetType
    case 11: return U.OwnerID();   // OwnerID
    case 12: return U.IsVehicle() ? 1 : 0;   // IsVehicle
    case 13: return U.IsBuilding() ? 1 : 0;   // IsBuilding
    case 14: return U.IsBusy() ? 1 : 0;   // IsBusy
    case 15: return U.isEMPed() ? 1 : 0;   // isEMPed
    case 16: { const LOCATION l = U.Location(); o[0] = l.x; o[1] = l.y; return 2; }   // Location
    case 17: return U.GetDamage();   // GetDamage
    case 18: return U.GetWeapon();   // GetWeapon
    case 19: return U.GetCargo();   // GetCargo
    case 20: return U.GetConVecCargoTurret();   // GetConVecCargoTurret
    case 21: return U.GetCurAction();   // GetCurAction
    case 22: return U.GetLastCommand();   // GetLastCommand
    case 23: return U.HasPower() ? 1 : 0;   // HasPower
    case 24: return U.HasWorkers() ? 1 : 0;   // HasWorkers
    case 25: return U.HasScientists() ? 1 : 0;   // HasScientists
    case 26: return U.GetLabCurrentTopic();   // GetLabCurrentTopic
    case 27: return U.GetLabScientistCount();   // GetLabScientistCount
    case 28: return U.GetFactoryCargo(a[1]);   // GetFactoryCargo
    case 29: return U.GetFactoryCargoWeapon(a[1]);   // GetFactoryCargoWeapon
    case 30: { const LOCATION l = U.GetDockLocation(); o[0] = l.x; o[1] = l.y; return 2; }   // GetDockLocation
    case 31: return U.GetCargoAmount();   // GetCargoAmount
    case 32: return (int)U.GetCargoType();   // GetCargoType
    case 33: return U.GetNumTruckLoadsSoFar();   // GetNumTruckLoadsSoFar
    case 34: return U.GetBarYield();   // GetBarYield
    case 35: return U.GetOreType();   // GetOreType
    case 36: return U.GetSurveyedBy();   // GetSurveyedBy
    case 37: return U.GetWorkersInTraining();   // GetWorkersInTraining
    case 38: return U.GetLaunchPadCargo();   // GetLaunchPadCargo
    case 39: U.SetLabScientistCount(a[1]); return 0;   // SetLabScientistCount
    case 40: U.SetLaunchPadCargo((map_id)a[1]); return 0;   // SetLaunchPadCargo
    case 41: U.SetFactoryCargo(a[1], (map_id)a[2], (map_id)a[3]); return 0;   // SetFactoryCargo
    case 42: U.DoMove(LOCATION(a[1], a[2])); return 0;   // DoMove
    case 43: { UnitEx z; z.unitID = a[1]; U.Unit::DoAttack(z); return 0; }   // DoAttack
    case 44: U.DoBuild((map_id)a[1], LOCATION(a[2], a[3])); return 0;   // DoBuild
    case 45: U.DoDeployMiner(LOCATION(a[1], a[2])); return 0;   // DoDeployMiner
    case 46: U.DoDock(LOCATION(a[1], a[2])); return 0;   // DoDock
    case 47: { MAP_RECT r(LOCATION(a[2], a[3]), LOCATION(a[4], a[5])); U.DoBuildWall((map_id)a[1], r); return 0; }   // DoBuildWall
    case 48: U.DoProduce((map_id)a[1], (map_id)a[2]); return 0;   // DoProduce
    case 49: U.DoTransferCargo(a[1]); return 0;   // DoTransferCargo
    case 50: U.DoResearch(a[1], a[2]); return 0;   // DoResearch
    case 51: U.DoTrainScientists(a[1]); return 0;   // DoTrainScientists
    case 52: { UnitEx z; z.unitID = a[1]; U.DoRepair(z); return 0; }   // DoRepair
    case 53: { UnitEx z; z.unitID = a[1]; U.DoReprogram(z); return 0; }   // DoReprogram
    case 54: U.DoIdle(); return 0;   // DoIdle
    case 55: U.DoUnIdle(); return 0;   // DoUnIdle
    case 56: U.DoStop(); return 0;   // DoStop
    case 57: U.DoLaunch(a[1], a[2], a[3]); return 0;   // DoLaunch
    case 58: U.DoSelfDestruct(); return 0;   // DoSelfDestruct
    case 59: U.DoPoof(); return 0;   // DoPoof
    case 60: { int n = 0; if (a[1] == mapNone || a[1] == mapAny) { for (int t : op2ki::GEBAEUDE) { PlayerBuildingEnum e(a[0], (map_id)t); UnitEx u; while (e.GetNext(u) && n < no) o[n++] = u.unitID; } } else { PlayerBuildingEnum e(a[0], (map_id)a[1]); UnitEx u; while (e.GetNext(u) && n < no) o[n++] = u.unitID; } return n; }   // PlayerBuildingEnum
    case 61: { int n = 0; PlayerVehicleEnum e(a[0]); UnitEx u; while (e.GetNext(u) && n < no) o[n++] = u.unitID; return n; }   // PlayerVehicleEnum
    case 62: { int n = 0; PlayerUnitEnum e(a[0]); UnitEx u; while (e.GetNext(u) && n < no) o[n++] = u.unitID; return n; }   // PlayerUnitEnum
    case 63: { int n = 0; MAP_RECT r(LOCATION(a[0], a[1]), LOCATION(a[2], a[3])); InRectEnumerator e(r); UnitEx u; while (e.GetNext(u) && n < no) o[n++] = u.unitID; return n; }   // InRectEnumerator
    case 64: { int n = 0; InRangeEnumerator e(LOCATION(a[0], a[1]), a[2]); UnitEx u; while (e.GetNext(u) && n < no) o[n++] = u.unitID; return n; }   // InRangeEnumerator
    case 65: { int n = 0; LocationEnumerator e(LOCATION(a[0], a[1])); UnitEx u; while (e.GetNext(u) && n < no) o[n++] = u.unitID; return n; }   // LocationEnumerator
    case 66: { int n = 0; ScGroup* g = gruppe(a[0]); if (!g) return 0; GroupEnumerator e(*g); UnitEx u; while (e.GetNext(u) && n < no) o[n++] = u.unitID; return n; }   // GroupEnumerator
    case 67: { UnitInfo i((map_id)a[0]); return i.GetHitPoints(a[1]); }   // GetHitPoints
    case 68: { UnitInfo i((map_id)a[0]); return i.GetArmor(a[1]); }   // GetArmor
    case 69: { UnitInfo i((map_id)a[0]); return i.GetOreCost(a[1]); }   // GetOreCost
    case 70: { UnitInfo i((map_id)a[0]); return i.GetRareOreCost(a[1]); }   // GetRareOreCost
    case 71: { UnitInfo i((map_id)a[0]); return i.GetWorkersRequired(a[1]); }   // GetWorkersRequired
    case 72: { UnitInfo i((map_id)a[0]); return i.GetScientistsRequired(a[1]); }   // GetScientistsRequired
    case 73: { UnitInfo i((map_id)a[0]); return i.GetPowerRequired(a[1]); }   // GetPowerRequired
    case 74: { UnitInfo i((map_id)a[0]); return i.GetWeaponRange(a[1]); }   // GetWeaponRange
    case 75: { UnitInfo i((map_id)a[0]); return i.GetProductionCapacity(a[1]); }   // GetProductionCapacity
    case 76: { UnitInfo i((map_id)a[0]); return i.GetReloadTime(a[1]); }   // GetReloadTime
    case 77: { UnitInfo i((map_id)a[0]); return i.GetPenetrationDamage(a[1]); }   // GetPenetrationDamage
    case 78: { UnitInfo i((map_id)a[0]); return i.GetConcussionDamage(a[1]); }   // GetConcussionDamage
    case 79: { UnitInfo i((map_id)a[0]); return i.GetBuildTime(a[1]); }   // GetBuildTime
    case 80: return Research::GetTechCount();   // GetTechCount
    case 81: { TechInfo ti = Research::GetTechInfo(a[0]); if (!ti.IsValid()) { o[0] = 0; return 1; } o[0] = 1; o[1] = ti.GetTechID(); o[2] = ti.GetCategory(); o[3] = ti.GetEdenCost(); o[4] = ti.GetPlymouthCost(); o[5] = ti.GetLab(); o[6] = ti.GetMaxScientists(); const int nr = ti.GetNumRequiredTechs(); o[7] = nr; for (int k = 0; k < nr && 8 + k < no; ++k) o[8 + k] = ti.GetRequiredTechIndex(k); return 8 + nr; }   // GetTechInfo
    case 82: { TechInfo ti = Research::GetTechInfo(a[0]); return ti.IsValid() && (ti.GetPlayerHasTech() & (1 << a[1])) ? 1 : 0; }   // GetPlayerHasTech
    case 83: return Player[a[0]].IsEden() ? 1 : 0;   // IsEden
    case 84: return Player[a[0]].Ore();   // Ore
    case 85: return Player[a[0]].RareOre();   // RareOre
    case 86: return Player[a[0]].FoodStored();   // FoodStored
    case 87: return Player[a[0]].Workers();   // Workers
    case 88: return Player[a[0]].Scientists();   // Scientists
    case 89: return Player[a[0]].Kids();   // Kids
    case 90: return (int)Player[a[0]].MoraleLevel();   // MoraleLevel
    case 91: { if (isInited && playerArray) { const int* r = reinterpret_cast<const int*>(reinterpret_cast<const char*>(playerArray) + a[0] * 3108); if (r[16] >= 0 && r[16] <= 100) return r[16]; } return -1; }   // MoraleRoh
    case 92: return op2ki::techIndex(a[1]) >= 0 && Player[a[0]].HasTechnology(a[1]) ? 1 : 0;   // HasTechnology
    case 93: Player[a[0]].SetOre(a[1]); return 0;   // SetOre
    case 94: Player[a[0]].SetRareOre(a[1]); return 0;   // SetRareOre
    case 95: Player[a[0]].CenterViewOn(a[1], a[2]); return 0;   // CenterViewOn
    case 96: return ExtPlayer[a[0]].IsAlliedTo(a[1]) ? 1 : 0;   // IsAlliedTo
    case 97: return ExtPlayer[a[0]].GetMaxOre();   // GetMaxOre
    case 98: return ExtPlayer[a[0]].GetMaxRareOre();   // GetMaxRareOre
    case 99: return ExtPlayer[a[0]].GetNumAvailableWorkers();   // GetNumAvailableWorkers
    case 100: return ExtPlayer[a[0]].GetNumAvailableScientists();   // GetNumAvailableScientists
    case 101: return ExtPlayer[a[0]].GetAmountPowerGenerated();   // GetAmountPowerGenerated
    case 102: return ExtPlayer[a[0]].GetAmountPowerConsumed();   // GetAmountPowerConsumed
    case 103: return ExtPlayer[a[0]].GetAmountPowerAvailable();   // GetAmountPowerAvailable
    case 104: return ExtPlayer[a[0]].GetNumIdleBuildings();   // GetNumIdleBuildings
    case 105: return ExtPlayer[a[0]].GetNumActiveBuildings();   // GetNumActiveBuildings
    case 106: return ExtPlayer[a[0]].GetNumUnpoweredStructures();   // GetNumUnpoweredStructures
    case 107: return ExtPlayer[a[0]].GetNumWorkersRequired();   // GetNumWorkersRequired
    case 108: return ExtPlayer[a[0]].GetNumScientistsRequired();   // GetNumScientistsRequired
    case 109: return ExtPlayer[a[0]].GetNumScientistsAssignedToResearch();   // GetNumScientistsAssignedToResearch
    case 110: return ExtPlayer[a[0]].GetNetFoodProduction();   // GetNetFoodProduction
    case 111: return ExtPlayer[a[0]].GetTotalResidenceCapacity();   // GetTotalResidenceCapacity
    case 112: return ExtPlayer[a[0]].GetTotalMedCenterCapacity();   // GetTotalMedCenterCapacity
    case 113: return ExtPlayer[a[0]].GetTotalRecreationFacilityCapacity();   // GetTotalRecreationFacilityCapacity
    case 114: return ExtPlayer[a[0]].GetTotalForumCapacity();   // GetTotalForumCapacity
    case 115: return ExtPlayer[a[0]].GetSatelliteCount((map_id)a[1]);   // GetSatelliteCount
    case 116: ExtPlayer[a[0]].RecalculateValues(); return 0;   // RecalculateValues
    case 117: { FightGroup g = CreateFightGroup(Player[a[0]]); return neueGruppe(g, 1); }   // CreateFightGroup
    case 118: { BuildingGroup g = CreateBuildingGroup(Player[a[0]]); return neueGruppe(g, 2); }   // CreateBuildingGroup
    case 119: { MiningGroup g = CreateMiningGroup(Player[a[0]]); return neueGruppe(g, 3); }   // CreateMiningGroup
    case 120: { ScGroup* g = gruppe(a[0]); if (g) g->Destroy(); return 0; }   // Destroy
    case 121: { ScGroup* g = gruppe(a[0]); UnitEx u; u.unitID = a[1]; if (g) g->TakeUnit(u); return 0; }   // TakeUnit
    case 122: { ScGroup* g = gruppe(a[0]); UnitEx u; u.unitID = a[1]; if (g) g->RemoveUnit(u); return 0; }   // RemoveUnit
    case 123: { ScGroup* g = gruppe(a[0]); ScGroup* q = gruppe(a[1]); if (g && q) g->TakeAllUnits(*q); return 0; }   // TakeAllUnits
    case 124: { ScGroup* g = gruppe(a[0]); return g ? g->TotalUnitCount() : 0; }   // TotalUnitCount
    case 125: { ScGroup* g = gruppe(a[0]); MAP_RECT r(LOCATION(a[1], a[2]), LOCATION(a[3], a[4])); if (g && art[a[0]] == 1) ((FightGroup*)g)->SetRect(r); else if (g && art[a[0]] == 2) ((BuildingGroup*)g)->SetRect(r); return 0; }   // SetRect
    case 126: { ScGroup* g = gruppe(a[0]); if (g && art[a[0]] == 1) ((FightGroup*)g)->DoGuardRect(); return 0; }   // DoGuardRect
    case 127: { ScGroup* g = gruppe(a[0]); MAP_RECT r(LOCATION(a[1], a[2]), LOCATION(a[3], a[4])); if (g && art[a[0]] == 1) ((FightGroup*)g)->AddGuardedRect(r); return 0; }   // AddGuardedRect
    case 128: { ScGroup* g = gruppe(a[0]); if (g) g->SetTargCount((map_id)a[1], (map_id)a[2], a[3]); return 0; }   // SetTargCount
    case 129: { ScGroup* g = gruppe(a[0]); if (g) g->ClearTargCount(); return 0; }   // ClearTargCount
    case 130: { ScGroup* g = gruppe(a[0]); if (g && art[a[0]] == 2) { LOCATION l(a[1], a[2]); ((BuildingGroup*)g)->RecordBuilding(l, (map_id)a[3], (map_id)a[4]); } return 0; }   // RecordBuilding
    case 131: { ScGroup* g = gruppe(a[0]); if (g && art[a[0]] == 2) { LOCATION l(a[1], a[2]); ((BuildingGroup*)g)->RecordTube(l); } return 0; }   // RecordTube
    case 132: { ScGroup* g = gruppe(a[0]); if (g && art[a[0]] == 2) { LOCATION l(a[1], a[2]); ((BuildingGroup*)g)->RecordWall(l, (map_id)a[3]); } return 0; }   // RecordWall
    case 133: { ScGroup* g = gruppe(a[0]); ScGroup* z = gruppe(a[1]); if (g && z && art[a[0]] == 2) ((BuildingGroup*)g)->RecordVehReinforceGroup(*z, a[2]); return 0; }   // RecordVehReinforceGroup
    case 134: { ScGroup* g = gruppe(a[0]); if (g && art[a[0]] == 3) { LOCATION m(a[1], a[2]), s(a[3], a[4]); MAP_RECT r(LOCATION(a[5], a[6]), LOCATION(a[7], a[8])); ((MiningGroup*)g)->Setup(m, s, r); } return 0; }   // Setup
    case 139: { const int v = U.GetTimeEMPed(); return v > 0 ? v : 0; }   // zeitEMP
    case 140: { const int v = U.GetTimeStickyfoamed(); return v > 0 ? v : 0; }   // zeitSchaum
    case 141: return TethysGame::UsesMorale() ? 0 : 1;   // MoralStabil
    case 142: return Player[a[0]].Difficulty() + 1;   // Schwierigkeit
  }
  return 0;
}
static void __cdecl text(int code, const char* s, int n) {
  static char puf[16384]; if (n > 16383) n = 16383; std::memcpy(puf, s, n); puf[n] = 0;
  if (code == 135) { op2::log::kiRaw(puf, n); } else op2::log::linef("%s", puf);
}
static int __cdecl lesen(const char* name, int nn, char* out, int max) {
  char nm[260]; if (nn > 259) nn = 259; std::memcpy(nm, name, nn); nm[nn] = 0; char pfad[600]; op2ki::wissenPfad(pfad, sizeof pfad, nm);
  FILE* f = std::fopen(pfad, "rb");
  if (!f) return -1;
  const int k = (int)std::fread(out, 1, max, f); std::fclose(f); return k;
}
static void __cdecl schreiben(const char* name, int nn, const char* s, int n) {
  char nm[260]; if (nn > 259) nn = 259; std::memcpy(nm, name, nn); nm[nn] = 0; char pfad[600]; op2ki::wissenPfad(pfad, sizeof pfad, nm);
  FILE* f = std::fopen(pfad, "wb"); if (!f) return; std::fwrite(s, 1, n, f); std::fclose(f);
}
} // namespace rsapi
