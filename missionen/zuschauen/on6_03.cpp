// KI gegen KI zum Zuschauen auf on6_03: Spieler 1 (Plymouth) und 2 (Eden) spielt je die Rust-KI (op2ki.dll), die
// Startplaetze waehlt der Seed. Wer keine Kerngebaeude mehr hat, verliert; dann endet die Mission.
#include <Outpost2DLL/Outpost2DLL.h>
#include <OP2Helper/OP2Helper.h>
#include <HFL/Source/HFL.h>
#include "op2_log.hpp"
#include "op2_crash.hpp"
#include "op2ki.hpp"
#include "duell.hpp"

Export void NoResponseToTrigger() {}

ExportLevelDetailsFull("KI KRR on6_03", "on6_03.map", "MULTITEK.TXT", Colony, 3, 12, 0)
Export const AIModDescEx DescBlockEx = { 0 };

// Kartengroesse und Startplaetze der Original-Mission (Kacheln)
static const int BREITE = 128, HOEHE = 128;
static const int N = 6;
static const int X[] = { 109, 71, 66, 108, 32, 23 }, Y[] = { 87, 106, 25, 35, 93, 46 };

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
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(69, 41), OreTypeCommon, Bar2, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(38, 54), OreTypeCommon, Bar2, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(41, 82), OreTypeCommon, Bar2, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(73, 91), OreTypeCommon, Bar2, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(94, 79), OreTypeCommon, Bar2, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(94, 48), OreTypeCommon, Bar2, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(80, 27), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(93, 29), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(89, 14), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(125, 7), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(114, 12), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(116, 23), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(127, 39), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(108, 55), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(127, 58), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(114, 65), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(126, 81), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(87, 61), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(81, 44), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(66, 52), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(76, 64), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(71, 78), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(89, 94), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(93, 116), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(115, 106), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(118, 122), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(59, 122), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(50, 108), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(55, 95), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(45, 70), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(51, 54), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(24, 70), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(4, 84), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(13, 120), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(8, 94), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(6, 58), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(7, 43), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(4, 20), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(4, 6), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(23, 6), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(49, 27), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(37, 36), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(33, 23), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(53, 10), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(54, 84), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapFumarole, XYPos(61, 78), -1, -1, -1);
        TethysGame::CreateBeacon(mapFumarole, XYPos(91, 65), -1, -1, -1);
        TethysGame::CreateBeacon(mapFumarole, XYPos(85, 14), -1, -1, -1);
        TethysGame::CreateBeacon(mapFumarole, XYPos(95, 119), -1, -1, -1);
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
