// KI gegen KI zum Zuschauen auf on6_01: Spieler 1 (Plymouth) und 2 (Eden) spielt je die Rust-KI (op2ki.dll), die
// Startplaetze waehlt der Seed. Wer keine Kerngebaeude mehr hat, verliert; dann endet die Mission.
#include <Outpost2DLL/Outpost2DLL.h>
#include <OP2Helper/OP2Helper.h>
#include <HFL/Source/HFL.h>
#include "op2_log.hpp"
#include "op2_crash.hpp"
#include "op2ki.hpp"
#include "duell.hpp"

Export void NoResponseToTrigger() {}

ExportLevelDetailsFull("KI KRR on6_01", "on6_01.map", "MULTITEK.TXT", Colony, 3, 12, 0)
Export const AIModDescEx DescBlockEx = { 0 };

// Kartengroesse und Startplaetze der Original-Mission (Kacheln)
static const int BREITE = 128, HOEHE = 128;
static const int N = 6;
static const int X[] = { 74, 32, 99, 112, 31, 66 }, Y[] = { 99, 92, 36, 87, 37, 27 };

static int fertig = 0, naechster = 1000;
static void spielendePruefen() {
    const int t = TethysGame::Tick();
    if (fertig || t < naechster) return;
    naechster = t + 100;
    const int g1 = spielende::kern(1, false), g2 = spielende::kern(2, false);
    if (g1 && g2) return;
    fertig = 1;
    op2::log::linef("Spielende: Tick %d, Kerngebaeude Spieler 1: %d, Spieler 2: %d", t, g1, g2);
    Trigger tr = CreateTimeTrigger(1, 1, 1, "NoResponseToTrigger");
    CreateVictoryCondition(1, 0, tr, "Duell entschieden");
}

static void initProc() {
    op2::log::line("InitProc: starting");
    if (HFLInit() != HFLLOADED) {
        op2::log::line("InitProc: HFLInit FAILED");
    }

    // --- Player 0 ---
    Player[0].GoEden();
    Player[0].GoHuman();
    Player[0].SetTechLevel(3);

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
    Player[2].SetWorkers(26);
    Player[2].SetScientists(13);
    Player[2].SetKids(15);
    Player[2].SetOre(8000);
    Player[2].SetFoodStored(3000);


    // --- Base layout for player 0 ---
    {
        Unit _u;
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(61, 44), OreTypeCommon, Bar2, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(99, 55), OreTypeCommon, Bar2, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(107, 99), OreTypeCommon, Bar2, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(81, 115), OreTypeCommon, Bar2, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(29, 106), OreTypeCommon, Bar2, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(36, 54), OreTypeCommon, Bar2, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(50, 17), OreTypeRare, Bar1, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(27, 29), OreTypeRare, Bar1, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(25, 82), OreTypeRare, Bar1, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(83, 91), OreTypeRare, Bar1, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(114, 73), OreTypeRare, Bar1, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(102, 23), OreTypeRare, Bar1, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(66, 65), OreTypeRare, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(66, 68), OreTypeRare, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapFumarole, XYPos(53, 46), -1, -1, -1);
        TethysGame::CreateBeacon(mapFumarole, XYPos(70, 40), -1, -1, -1);
        TethysGame::CreateBeacon(mapFumarole, XYPos(35, 62), -1, -1, -1);
        TethysGame::CreateBeacon(mapFumarole, XYPos(40, 42), -1, -1, -1);
        TethysGame::CreateBeacon(mapFumarole, XYPos(19, 81), -1, -1, -1);
        TethysGame::CreateBeacon(mapFumarole, XYPos(50, 85), -1, -1, -1);
        TethysGame::CreateBeacon(mapFumarole, XYPos(66, 101), -1, -1, -1);
        TethysGame::CreateBeacon(mapFumarole, XYPos(88, 114), -1, -1, -1);
        TethysGame::CreateBeacon(mapFumarole, XYPos(94, 90), -1, -1, -1);
        TethysGame::CreateBeacon(mapFumarole, XYPos(99, 74), -1, -1, -1);
        TethysGame::CreateBeacon(mapFumarole, XYPos(82, 57), -1, -1, -1);
        TethysGame::CreateBeacon(mapFumarole, XYPos(89, 26), -1, -1, -1);
        TethysGame::CreateBeacon(mapMagmaVent, XYPos(98, 7), -1, -1, -1);
        TethysGame::CreateBeacon(mapMagmaVent, XYPos(38, 2), -1, -1, -1);
        TethysGame::CreateBeacon(mapMagmaVent, XYPos(2, 83), -1, -1, -1);
        TethysGame::CreateBeacon(mapMagmaVent, XYPos(2, 37), -1, -1, -1);
        TethysGame::CreateBeacon(mapMagmaVent, XYPos(17, 125), -1, -1, -1);
        TethysGame::CreateBeacon(mapMagmaVent, XYPos(127, 32), -1, -1, -1);
        TethysGame::CreateBeacon(mapFumarole, XYPos(59, 67), -1, -1, -1);
        TethysGame::CreateBeacon(mapFumarole, XYPos(10, 64), -1, -1, -1);
        TethysGame::CreateBeacon(mapFumarole, XYPos(23, 68), -1, -1, -1);
        TethysGame::CreateBeacon(mapFumarole, XYPos(56, 121), -1, -1, -1);
        TethysGame::CreateBeacon(mapFumarole, XYPos(120, 60), -1, -1, -1);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(9, 16), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(20, 16), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(39, 5), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(77, 6), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(102, 6), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(126, 16), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(126, 42), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(126, 71), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(126, 105), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(114, 114), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(101, 126), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(63, 126), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(47, 114), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(21, 120), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(9, 110), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(4, 87), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(5, 64), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(5, 47), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(116, 7), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(126, 95), OreTypeCommon, Bar3, VariantRandom);
    }

    startwahl::paar(N, X, Y);
    op2ki::start(1, MkXY(1, 1), BREITE, HOEHE, X[startwahl::a], Y[startwahl::a], true, 3, 3, 500);
    op2::log::linef("InitProc: Aufbau-KI Spieler 1 -> %s", op2::log::kiPath());
    op2ki::start(2, MkXY(1, 1), BREITE, HOEHE, X[startwahl::b], Y[startwahl::b], true, 3, 3, 500);
    op2::log::linef("InitProc: Aufbau-KI Spieler 2 -> %s", op2::log::kiPath());

    TethysGame::ForceMoraleGood(PlayerNum::PlayerAll);
    op2::log::line("InitProc: done");
}

static void aiProc() {
    // Nach einem Spielstand-Load laeuft InitProc nicht erneut -- HFL hier nachinitialisieren (idempotent).
    HFLInit();
    spielendePruefen();
    op2ki::schritt(1);
    op2ki::schritt(2);
}

Export int InitProc() { op2::crash::guard("InitProc", &initProc); return 1; }
Export void AIProc()  { op2::crash::guard("AIProc",   &aiProc); }

// Spielstaende laden wird nicht unterstuetzt (die KI behaelt ihren Zustand nur im laufenden Spiel); die Engine braucht
// trotzdem einen Speicherbereich.
struct MissionSave { int frei; };
static MissionSave g_save;
ExportSaveLoadData(g_save)

extern "C" int __stdcall DllMain(void* inst, unsigned long reason, void*) {
    if (reason == 1 /* DLL_PROCESS_ATTACH */) {
        op2::crash::installHandler();
        op2::log::setTickSource([] { return TethysGame::Tick(); });
    }
    return 1;
}
