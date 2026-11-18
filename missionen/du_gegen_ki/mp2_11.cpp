// mission.cpp -- generated from the editor model for: Du gegen die KI (mp2_11)
// Built against OP2MissionSDK (Outpost2DLL + OP2Helper + HFL).
// https://github.com/OutpostUniverse/OP2MissionSDK

#include <Outpost2DLL/Outpost2DLL.h>
#include <OP2Helper/OP2Helper.h>
#include <HFL/Source/HFL.h>
#include "op2_log.hpp"
#include "op2_crash.hpp"
#include "op2_ki.hpp"

static const int kTicksPerMark = 100;

static const int kDiff[] = {5, 10, 13};
static int diff = 10;

static int randBetween(int minValue, int maxValue) {
    if (maxValue < minValue) { int _t = minValue; minValue = maxValue; maxValue = _t; }
    return minValue + TethysGame::GetRand(maxValue - minValue + 1);
}

// Erste Einheit auf einer Kachel (LOCATION in Engine-Koordinaten, MkXY).
// First unit on a tile (LOCATION in engine coordinates, MkXY).
static UnitEx unitOnTile(LOCATION where) {
    LocationEnumerator _e(where);
    UnitEx _u;
    if (_e.GetNext(_u)) return _u;
    return UnitEx();
}

// Einheit eines Typs/Spielers auf einer Kachel (fuer Fahrzeug-Capture).
// Unit of a type/player on a tile (for vehicle capture).
static UnitEx findUnitAt(LOCATION where, map_id type, int owner) {
    LocationEnumerator _e(where);
    UnitEx _u;
    while (_e.GetNext(_u)) {
        if (_u.GetType() == type && _u.OwnerID() == owner) return _u;
    }
    return UnitEx();
}

static int countUnitsOfType(int playerNum, map_id type) {
    int _n = 0;
    PlayerUnitEnum _e(playerNum);
    UnitEx _u;
    while (_e.GetNext(_u)) if (_u.GetType() == type) ++_n;
    return _n;
}

// Fertig UND ruhig? WHITELIST auf moDone: alles andere (moDevelop =
// im Bau, moOperationalWait = wartet auf Inbetriebnahme, HFL-Sentinels
// bei nicht initialisiertem HFL) zaehlt als NICHT bereit -- ein
// TakeUnit in diesen Zustaenden macht das Gebaeude kaputt.
// Finished AND quiet? WHITELIST on moDone: anything else (moDevelop =
// under construction, moOperationalWait = waiting to go operational,
// HFL sentinels when HFL is not initialized) counts as NOT ready --
// a TakeUnit in those states corrupts the building.
static bool isCompleted(UnitEx u) {
    if (u.unitID == 0 || !u.IsLive()) return false;
    return u.GetCurAction() == moDone;
}

struct MissionSave {
    ki::Zustand ki_1;
    ki::Zustand ki_2;
};
static MissionSave g_save;


Export void NoResponseToTrigger() {}

ExportLevelDetailsFull("Du gegen die KI (mp2_11)", "mp2_11.map", "MULTITEK.TXT", Colony, 3, 12, 0)
Export const AIModDescEx DescBlockEx = { 0 };

static void initProc() {
    op2::log::line("InitProc: starting");
    if (HFLInit() != HFLLOADED) {
        op2::log::line("InitProc: HFLInit FAILED");
    }
    diff = kDiff[Player[0].Difficulty()];

    // --- Player 0 ---
    Player[0].GoEden();
    Player[0].GoHuman();
    Player[0].SetTechLevel(3);
    Player[0].SetWorkers(26);
    Player[0].SetScientists(13);
    Player[0].SetKids(15);
    Player[0].SetOre(8000);
    Player[0].SetFoodStored(3000);

    // --- Player 1 ---
    Player[1].GoPlymouth();
    Player[1].GoHuman();
    Player[1].SetTechLevel(3);
    Player[1].SetWorkers(26);
    Player[1].SetScientists(13);
    Player[1].SetKids(15);
    Player[1].SetOre(8000);
    Player[1].SetFoodStored(3000);

    // --- Player 2 ---
    Player[2].GoEden();
    Player[2].GoHuman();
    Player[2].SetTechLevel(3);


    // --- Base layout for player 0 ---
    {
        Unit _u;
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(37, 5), OreTypeCommon, Bar2, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(89, 123), OreTypeCommon, Bar2, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(34, 113), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(10, 71), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(55, 64), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(116, 34), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(96, 16), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(65, 88), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(51, 117), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(73, 15), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(52, 51), OreTypeRare, Bar2, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(90, 66), OreTypeRare, Bar2, VariantRandom);
    }

    AddGameMessage("Mit dem OP2 Mission Editor erstellt.");

    // --- Aufbau-KI: baut die Kolonie selbst auf, Spur in <Mission>-KI.jsonl ---
    // --- Building AI: builds the colony itself, trace in <Mission>-KI.jsonl ---
    ki::start(g_save.ki_1, 1, MkXY(1, 1), 128, 128, 5, 15, true, 3, 3, 500);
    op2::log::linef("InitProc: Aufbau-KI Spieler 1 -> %s", op2::log::kiPath());
    ki::start(g_save.ki_2, 2, MkXY(1, 1), 128, 128, 112, 115, true, 3, 3, 500);
    op2::log::linef("InitProc: Aufbau-KI Spieler 2 -> %s", op2::log::kiPath());

    TethysGame::ForceMoraleGood(PlayerNum::PlayerAll);
    op2::log::line("InitProc: done");
}

// --- Eigener Missions-Code (Missions-Einstellungen) ---
// --- Custom mission code (mission settings) ---
// Gelernte Stellschrauben (Optimierer, Mittelwert der Suche) fest eingebaut -- Datei logs/KI-Parameter-P<n>.txt geht vor
namespace { struct GelernteParameter { GelernteParameter() {
  ki::parameterText[1] = "lernen=0\nreaktion=15000.0000\nhorizont=9233.9408\nueberlegenheit=1.2702\nfabrikGewinn=0.8052\nreserveStart=0.8000\nreserveLernrate=0.4760\nreserveRueckgang=1.0000\nreserveMax=4.0000\nturmAnteil=0.8000\nzermuerbAbstand=2000.0000\nschmelzeNutzen=1.4966\nwohnraumGrenze=98.0000\nfreischaltWert=0.5160\nreparaturAb=17.8890\nreparaturConVecAb=16.0656\nnahRadius=40.0000\nheimHalten=1.6362\nsteigerung=1.5000\nvorsorge=57.4926\nschmelzenJeMine=2.0000\ngefecht=1.5\npaarung=1\nturmNetz=1\nwissArbeit=1\nfabrikErz=1\nforschungVorrang=1\nfabrikWirtschaft=1\nseltenNutzen=1";
  ki::parameterText[2] = "lernen=0\nreaktion=15000.0000\nhorizont=9233.9408\nueberlegenheit=1.2702\nfabrikGewinn=0.8052\nreserveStart=0.8000\nreserveLernrate=0.4760\nreserveRueckgang=1.0000\nreserveMax=4.0000\nturmAnteil=0.8000\nzermuerbAbstand=2000.0000\nschmelzeNutzen=1.4966\nwohnraumGrenze=98.0000\nfreischaltWert=0.5160\nreparaturAb=17.8890\nreparaturConVecAb=16.0656\nnahRadius=40.0000\nheimHalten=1.6362\nsteigerung=1.5000\nvorsorge=57.4926\nschmelzenJeMine=2.0000";
} } g_gelernteParameter; }

#define RUST_SPIELER 1
// --- Uebertragene C++-KI in Rust (op2ki.dll, ki2_*) fuehrt Spieler 1; Spieler 2 bleibt die C++-KI. ---
// Die Rust-KI ruft die OP2-API ueber rsapi::ruf (welt_api.cpp, erzeugt). Hooks: ki::fremdStart/fremdSchritt.
#include <type_traits>
// ERZEUGT von ki_rust/gen_bruecke.py -- OP2-API fuer die Rust-KI (ExternWelt). Nicht von Hand aendern.
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
    case 8: return ki::hoechsteEinheit();   // hoechsteEinheit
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
    case 60: { int n = 0; if (a[1] == mapNone || a[1] == mapAny) { for (int t : ki::GEBAEUDE) { PlayerBuildingEnum e(a[0], (map_id)t); UnitEx u; while (e.GetNext(u) && n < no) o[n++] = u.unitID; } } else { PlayerBuildingEnum e(a[0], (map_id)a[1]); UnitEx u; while (e.GetNext(u) && n < no) o[n++] = u.unitID; } return n; }   // PlayerBuildingEnum
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
    case 92: return ki::techIndex(a[1]) >= 0 && Player[a[0]].HasTechnology(a[1]) ? 1 : 0;   // HasTechnology
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
  }
  return 0;
}
static void __cdecl text(int code, const char* s, int n) {
  static char puf[16384]; if (n > 16383) n = 16383; std::memcpy(puf, s, n); puf[n] = 0;
  if (code == 135) { op2::log::kiRaw(puf, n); } else op2::log::linef("%s", puf);
}

static int __cdecl lesen(const char* name, int nn, char* out, int max) {
  char nm[260]; if (nn > 259) nn = 259; std::memcpy(nm, name, nn); nm[nn] = 0; char pfad[600]; ki::wissenPfad(pfad, sizeof pfad, nm);
  FILE* f = std::fopen(pfad, "rb");
  if (!f) return -1;
  const int k = (int)std::fread(out, 1, max, f); std::fclose(f); return k;
}
static void __cdecl schreiben(const char* name, int nn, const char* s, int n) {
  char nm[260]; if (nn > 259) nn = 259; std::memcpy(nm, name, nn); nm[nn] = 0; char pfad[600]; ki::wissenPfad(pfad, sizeof pfad, nm);
  FILE* f = std::fopen(pfad, "wb"); if (!f) return; std::fwrite(s, 1, n, f); std::fclose(f);
}
} // namespace rsapi
#ifndef RUST_SPIELER
#define RUST_SPIELER 1
#endif
namespace rs2 {
struct Api { void* ruf; void* text; void* lesen; void* schreiben; };
typedef void* (__cdecl *NeuFn)(Api);
typedef void (__cdecl *ParamFn)(void*, int, const char*, int);
typedef void (__cdecl *StartFn)(void*, int, int, int, int, int, int, int, int, int, int, int);
typedef void (__cdecl *SchrittFn)(void*, int);
typedef int (__cdecl *AbiFn)();
static NeuFn neu; static ParamFn param; static StartFn startFn; static SchrittFn schrittFn; static void* h = nullptr; static int geladen = 0;
inline bool laden() {
    if (geladen) return h != nullptr;
    geladen = 1;
    // zuerst neben der Missions-DLL suchen (OPU-Ordner), unabhaengig vom Arbeitsverzeichnis des Launchers
    HMODULE m = nullptr; char pfad[MAX_PATH] = {0}; HMODULE selbst = nullptr;
    if (GetModuleHandleExA(GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT, (LPCSTR)&laden, &selbst)
        && GetModuleFileNameA(selbst, pfad, MAX_PATH)) {
        char* s = std::strrchr(pfad, '\\'); if (s) { std::strcpy(s + 1, "op2ki.dll"); m = LoadLibraryA(pfad); }
    }
    if (!m) m = LoadLibraryA("op2ki.dll");
    if (!m) m = LoadLibraryA("OPU\\op2ki.dll");
    if (!m) { op2::log::linef("Rust-KI 2: op2ki.dll nicht geladen (Fehler %lu, gesucht: %s)", GetLastError(), pfad); return false; }
    op2::log::linef("Rust-KI 2: op2ki.dll aus %s", pfad[0] ? pfad : "Suchpfad");
    AbiFn abi = (AbiFn)GetProcAddress(m, "ki_abi");
    neu = (NeuFn)GetProcAddress(m, "ki2_neu"); param = (ParamFn)GetProcAddress(m, "ki2_parameter");
    startFn = (StartFn)GetProcAddress(m, "ki2_start"); schrittFn = (SchrittFn)GetProcAddress(m, "ki2_schritt");
    if (!neu || !param || !startFn || !schrittFn) { op2::log::line("Rust-KI 2: Funktionen fehlen"); return false; }
    Api a = { (void*)&rsapi::ruf, (void*)&rsapi::text, (void*)&rsapi::lesen, (void*)&rsapi::schreiben };
    h = neu(a);
    op2::log::linef("Rust-KI 2: geladen, ABI %d", abi ? abi() : -1);
    return h != nullptr;
}
inline void start(ki::Zustand& z, int spieler, LOCATION u, int breite, int hoehe, int x, int y, bool ausr, int ziel, int stufe, int wert) {
    if (!laden()) {
        // Ersatz: ohne op2ki.dll spielt die C++-KI aus der Missions-DLL -- es gibt in jedem Fall einen Gegner
        AddGameMessage("KI: op2ki.dll nicht gefunden - es spielt die eingebaute C++-KI");
        ki::FremdStart s = ki::fremdStart[spieler]; ki::fremdStart[spieler] = nullptr;
        ki::start(z, spieler, u, breite, hoehe, x, y, ausr, ziel, stufe, wert);
        ki::fremdStart[spieler] = s;
        return;
    }
    AddGameMessage("KI: Rust-KI (op2ki.dll) geladen");
    for (int p = 0; p < 8; ++p) if (ki::parameterText[p]) param(h, p, ki::parameterText[p], (int)std::strlen(ki::parameterText[p]));
    startFn(h, spieler, u.x, u.y, breite, hoehe, x, y, ausr ? 1 : 0, ziel, stufe, wert);
}
inline void schritt(ki::Zustand& z) {
    if (h) { schrittFn(h, z.spieler); return; }
    ki::FremdSchritt s = ki::fremdSchritt[z.spieler]; ki::fremdSchritt[z.spieler] = nullptr;   // Ersatz: C++-KI
    ki::schritt(z);
    ki::fremdSchritt[z.spieler] = s;
}
struct Anmeldung { Anmeldung() {
    ki::fremdStart[RUST_SPIELER] = &start; ki::fremdSchritt[RUST_SPIELER] = &schritt;
#ifdef RUST_SPIELER2
    ki::fremdStart[RUST_SPIELER2] = &start; ki::fremdSchritt[RUST_SPIELER2] = &schritt;   // beide Spieler Rust
#endif
} };
static Anmeldung g_anmeldung;
} // namespace rs2

// --- Du gegen die KI (Nutzer 2026-11-16): Spieler 0 (Mensch) bekommt am Platz von Spieler 2 dieselbe Startbasis wie die
// KI (ki::start mit Stufe 3, aber ohne KI-Schritte); Spieler 2 bleibt leer. Alles, was Spieler 0 tut, wird aufgezeichnet:
// logs/Aufzeichnung-<Datum>-<Zeit>.jsonl. Nicht Teil der KI.
// Zeilen: "kopf" (einmal), "neu"/"ae" (Einheit erschienen/geaendert: Typ, Waffe/Fracht, Ort intern, Aktion, letzter
// Befehl, Schaden, Zusatz, Flags), "weg" (Einheit verschwunden), "kol" (alle 100 Ticks Kolonie + neue Techniken),
// "feind" (alle 500 Ticks alle Einheiten von Spieler 1), "ende". Orte in Engine-Kacheln (Location().x/.y).
namespace mensch {
static ki::Zustand z0;
static FILE* f = nullptr;
static int bx = 0, by = 0, moralFrei = 0, fertig = 0, naechsteKol = 0, naechsterFeind = 0, naechsterFlush = 0, naechstesEnde = 1000;
struct Stand { int typ, w, x, y, a, c, s, z, fl, posT, lebt; };
static const int MAXE = 4096;
static Stand alt[MAXE];
static unsigned char techDa[512];

inline void kopf() {
    SYSTEMTIME st; GetLocalTime(&st);
    char nm[96], pfad[600];
    std::snprintf(nm, sizeof nm, "Aufzeichnung-%04d%02d%02d-%02d%02d%02d.jsonl", st.wYear, st.wMonth, st.wDay, st.wHour, st.wMinute, st.wSecond);
    ki::wissenPfad(pfad, sizeof pfad, nm);
    f = std::fopen(pfad, "wb");
    if (!f) { op2::log::linef("Aufzeichnung: %s nicht schreibbar", pfad); return; }
    static char puffer[1 << 16]; setvbuf(f, puffer, _IOFBF, sizeof puffer);
    int kx = -1, ky = -1, mx = -1, my = -1;
    { PlayerBuildingEnum e(1, mapCommandCenter); Unit u; if (e.GetNext(u)) { kx = u.Location().x; ky = u.Location().y; } }
    { PlayerBuildingEnum e(0, mapCommandCenter); Unit u; if (e.GetNext(u)) { mx = u.Location().x; my = u.Location().y; } }
    std::fprintf(f, "{\"t\":%d,\"k\":\"kopf\",\"version\":1,\"karte\":\"mp2_11\",\"datum\":\"%04d-%02d-%02d %02d:%02d:%02d\",\"mensch\":0,\"ki\":1,"
                    "\"menschEden\":%d,\"kiEden\":%d,\"menschStart\":[%d,%d],\"menschCC\":[%d,%d],\"kiCC\":[%d,%d],\"schwierigkeit\":%d}\n",
                 TethysGame::Tick(), st.wYear, st.wMonth, st.wDay, st.wHour, st.wMinute, st.wSecond, Player[0].IsEden() ? 1 : 0,
                 Player[1].IsEden() ? 1 : 0, bx, by, mx, my, kx, ky, (int)Player[0].Difficulty());
    op2::log::linef("Aufzeichnung: %s", pfad);
}

// Zustand einer Einheit von Spieler 0; Zusatz z: Labor = Forschungsthema, Lastwagen = Ladung, Fabrik = Fracht Bucht 0.
inline Stand stand(UnitEx u, bool gebaeude) {
    Stand s{};
    const LOCATION l = u.Location();
    s.typ = u.GetType(); s.x = l.x; s.y = l.y; s.a = (int)u.GetCurAction(); s.c = (int)u.GetLastCommand(); s.s = u.GetDamage();
    s.w = s.typ == mapConVec ? (int)u.GetCargo() : s.typ == mapCargoTruck ? (int)u.GetCargoType() : (int)u.GetWeapon();
    if (gebaeude) {
        if (s.typ == mapStandardLab || s.typ == mapAdvancedLab || s.typ == mapBasicLab) s.z = u.GetLabCurrentTopic();
        else if (s.typ == mapVehicleFactory || s.typ == mapStructureFactory || s.typ == mapArachnidFactory) s.z = (int)u.GetFactoryCargo(0);
        else if (s.typ == mapUniversity) s.z = u.GetWorkersInTraining();
        s.fl = (u.HasPower() ? 1 : 0) | (u.HasWorkers() ? 2 : 0) | (u.HasScientists() ? 4 : 0);
    } else if (s.typ == mapCargoTruck) s.z = u.GetCargoAmount();
    return s;
}

inline void zeile(const char* k, int t, int id, const Stand& s, int extra) {
    std::fprintf(f, "{\"t\":%d,\"k\":\"%s\",\"id\":%d,\"typ\":%d,\"w\":%d,\"x\":%d,\"y\":%d,\"a\":%d,\"c\":%d,\"s\":%d,\"z\":%d,\"fl\":%d",
                 t, k, id, s.typ, s.w, s.x, s.y, s.a, s.c, s.s, s.z, s.fl);
    if (extra >= 0) std::fprintf(f, ",\"von\":%d", extra);
    std::fputs("}\n", f);
}

inline void einheit(UnitEx u, bool gebaeude, int t, unsigned char* gesehen) {
    const int id = u.unitID;
    if (id <= 0 || id >= MAXE) return;
    gesehen[id] = 1;
    Stand s = stand(u, gebaeude);
    Stand& o = alt[id];
    if (!o.lebt) { s.posT = t; s.lebt = 1; zeile("neu", t, id, s, gebaeude ? -1 : u.CreatorID()); o = s; return; }
    const bool anders = s.typ != o.typ || s.w != o.w || s.a != o.a || s.c != o.c || s.z != o.z || s.fl != o.fl || s.s / 10 != o.s / 10;
    const bool weiter = (s.x != o.x || s.y != o.y) && t - o.posT >= 20;
    if (!anders && !weiter) return;
    s.posT = (s.x != o.x || s.y != o.y) ? t : o.posT; s.lebt = 1;
    zeile("ae", t, id, s, -1);
    o = s;
}

inline void erfassen(int t) {
    static unsigned char gesehen[MAXE];
    std::memset(gesehen, 0, sizeof gesehen);
    { PlayerVehicleEnum e(0); UnitEx u; while (e.GetNext(u)) if (u.IsLive()) einheit(u, false, t, gesehen); }
    { PlayerBuildingEnum e(0, mapNone); UnitEx u; while (e.GetNext(u)) if (u.IsLive()) einheit(u, true, t, gesehen); }
    for (int id = 1; id < MAXE; ++id)
        if (alt[id].lebt && !gesehen[id]) { std::fprintf(f, "{\"t\":%d,\"k\":\"weg\",\"id\":%d,\"typ\":%d,\"x\":%d,\"y\":%d,\"s\":%d}\n", t, id, alt[id].typ, alt[id].x, alt[id].y, alt[id].s); alt[id].lebt = 0; }
}

inline void kolonie(int t) {
    std::fprintf(f, "{\"t\":%d,\"k\":\"kol\",\"erz\":%d,\"selten\":%d,\"nahrung\":%d,\"arbeiter\":%d,\"wiss\":%d,\"kinder\":%d,\"moral\":%d,\"techs\":[",
                 t, Player[0].Ore(), Player[0].RareOre(), Player[0].FoodStored(), Player[0].Workers(), Player[0].Scientists(), Player[0].Kids(), (int)Player[0].MoraleLevel());
    int n = 0;
    for (int i = 0; i < ki::tb::ANZ_TECH && i < 512; ++i) {
        const int id = ki::tb::TECH[i].id;
        if (techDa[i] || ki::techIndex(id) < 0 || !Player[0].HasTechnology(id)) continue;
        techDa[i] = 1; std::fprintf(f, n++ ? ",%d" : "%d", id);
    }
    std::fputs("]}\n", f);
}

inline void feind(int t) {
    std::fprintf(f, "{\"t\":%d,\"k\":\"feind\",\"erz\":%d,\"selten\":%d,\"e\":[", t, Player[1].Ore(), Player[1].RareOre());
    int n = 0;
    { PlayerVehicleEnum e(1); UnitEx u; while (e.GetNext(u)) { if (!u.IsLive()) continue; const LOCATION l = u.Location(); std::fprintf(f, "%s[%d,%d,%d,%d,%d]", n++ ? "," : "", (int)u.GetType(), (int)u.GetWeapon(), l.x, l.y, u.GetDamage()); } }
    { PlayerBuildingEnum e(1, mapNone); UnitEx u; while (e.GetNext(u)) { if (!u.IsLive()) continue; const LOCATION l = u.Location(); std::fprintf(f, "%s[%d,0,%d,%d,%d]", n++ ? "," : "", (int)u.GetType(), l.x, l.y, u.GetDamage()); } }
    std::fputs("]}\n", f);
}

inline int kern(int p) {
    static const map_id T[] = { mapCommandCenter, mapStructureFactory, mapVehicleFactory, mapCommonOreSmelter, mapTokamak,
                                mapAgridome, mapResidence, mapStandardLab, mapAdvancedLab };
    int n = 0;
    for (map_id typ : T) { PlayerBuildingEnum e(p, typ); Unit u; while (e.GetNext(u)) if (u.IsLive()) ++n; }
    return n;
}

// Spielende wie in den Duellen: wer keines der Kerngebaeude mehr hat, hat verloren.
inline void ende(int t) {
    if (fertig || t < naechstesEnde) return;
    naechstesEnde = t + 100;
    const int g0 = kern(0), g1 = kern(1);
    if (g0 && g1) return;
    fertig = 1;
    if (f) { std::fprintf(f, "{\"t\":%d,\"k\":\"ende\",\"sieger\":%d,\"kern0\":%d,\"kern1\":%d}\n", t, g1 ? 1 : 0, g0, g1); std::fflush(f); }
    op2::log::linef("Spielende: Tick %d, Kerngebaeude Mensch: %d, KI: %d", t, g0, g1);
    Trigger tr = CreateTimeTrigger(1, 1, 1, "NoResponseToTrigger");
    if (!g1) CreateVictoryCondition(1, 0, tr, "Die KI ist besiegt");
    else CreateFailureCondition(1, 0, tr, "");
}

inline void schritt(ki::Zustand& z) {
    (void)z;
    const int t = TethysGame::Tick();
    if (!moralFrei) { TethysGame::FreeMoraleLevel(0); moralFrei = 1; }   // InitProc setzt ForceMoraleGood fuer alle
    if (!f) { kopf(); if (!f) { ende(t); return; } }
    erfassen(t);
    if (t >= naechsteKol) { naechsteKol = t + 100; kolonie(t); }
    if (t >= naechsterFeind) { naechsterFeind = t + 500; feind(t); }
    ende(t);
    if (t >= naechsterFlush) { naechsterFlush = t + 1000; std::fflush(f); }
}

// Spieler 2 (leer) traegt die Hooks: sein ki::start setzt Spieler 0 an diesen Platz, seine Schritte zeichnen auf.
inline void start(ki::Zustand& z, int sp, LOCATION u, int br, int ho, int x, int y, bool ausr, int ziel, int stufe, int wert) {
    (void)z; (void)sp;
    bx = x; by = y;
    ki::start(z0, 0, u, br, ho, x, y, ausr, ziel, stufe, wert);
    op2::log::linef("Mensch: Spieler 0 an %d,%d (Startstufe %d)", x, y, stufe);
}
struct Anmeldung { Anmeldung() { ki::fremdStart[2] = &start; ki::fremdSchritt[2] = &schritt; } };
static Anmeldung g_anmeldung;
} // namespace mensch

// --- Startwahl: der Seed (OP2H_SEED des Harness, sonst Zufall) waehlt das Startplatz-Paar fuer Spieler 1 und 2 aus allen
// Startplaetzen der Original-Mission. Haengt sich vor ki::start (ki::fremdStart), auch vor die Rust-KI. Nicht Teil der KI.
namespace startwahl {
static const int N = 2;
static const int X[] = { 5, 112 }, Y[] = { 15, 115 };
static ki::FremdStart innen[8];
static int a = -1, b = -1;
inline void paar() {
    if (a >= 0) return;
    int s = 0; char e[32];
    if (GetEnvironmentVariableA("OP2H_SEED", e, sizeof e)) s = atoi(e); else s = TethysGame::GetRand(1000) + 1;
    const int k = ((s - 1) % (N * (N - 1)) + N * (N - 1)) % (N * (N - 1));
    a = k / (N - 1); b = k % (N - 1); if (b >= a) b++;
    op2::log::linef("Startwahl: Seed %d -> Spieler 1 an Platz %d (%d,%d), Spieler 2 an Platz %d (%d,%d) von %d", s, a, X[a], Y[a], b, X[b], Y[b], N);
}
inline void start(ki::Zustand& z, int sp, LOCATION u, int br, int ho, int x, int y, bool ausr, int ziel, int stufe, int wert) {
    paar();
    const int i = sp == 1 ? a : b;
    if (sp == 1 || sp == 2) { x = X[i]; y = Y[i]; }
    if (innen[sp]) { innen[sp](z, sp, u, br, ho, x, y, ausr, ziel, stufe, wert); return; }
    ki::fremdStart[sp] = nullptr;
    ki::start(z, sp, u, br, ho, x, y, ausr, ziel, stufe, wert);
    ki::fremdStart[sp] = &start;
}
struct Anmeldung { Anmeldung() { for (int p = 1; p <= 2; ++p) { innen[p] = ki::fremdStart[p]; ki::fremdStart[p] = &start; } } };
static Anmeldung g_anmeldung;
} // namespace startwahl



static void aiProc() {
    // Nach einem Spielstand-Load laeuft InitProc NICHT erneut -- HFL
    // muss hier nachinitialisiert werden (intern idempotent), sonst
    // liefern UnitEx-Abfragen wie GetCurAction nur Sentinel-Werte.
    // After a savegame load InitProc does NOT run again -- HFL must
    // be (re)initialized here (internally idempotent), otherwise
    // UnitEx queries like GetCurAction only return sentinel values.
    HFLInit();
    diff = kDiff[Player[0].Difficulty()];
    ki::schritt(g_save.ki_1);
    ki::schritt(g_save.ki_2);
}

Export int InitProc() { op2::crash::guard("InitProc", &initProc); return 1; }
Export void AIProc()  { op2::crash::guard("AIProc",   &aiProc); }

// SaveRegion: g_save wird von der Engine mit dem Spielstand gespeichert
// und beim Laden byte-genau restauriert (Variablen, Gruppen-/Unit-/
// Trigger-Stubs, armed-Flags).
ExportSaveLoadData(g_save)

extern "C" int __stdcall DllMain(void* inst, unsigned long reason, void*) {
    if (reason == 1 /* DLL_PROCESS_ATTACH */) {
        op2::crash::installHandler();
        op2::log::setTickSource([] { return TethysGame::Tick(); });
    }
    return 1;
}
