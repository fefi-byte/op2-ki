// Du gegen die KI auf mp2_08: du spielst Eden (Spieler 0), die Rust-KI (op2ki.dll) Plymouth (Spieler 1). Beide
// bekommen dieselbe Startbasis; deine steht am Startplatz von Spieler 2, der selbst leer bleibt. Alles, was du tust, wird
// aufgezeichnet (aufzeichnung.hpp). Wer keine Kerngebaeude mehr hat, verliert.
#include <Outpost2DLL/Outpost2DLL.h>
#include <OP2Helper/OP2Helper.h>
#include <HFL/Source/HFL.h>
#include "op2_log.hpp"
#include "op2_crash.hpp"
#include "op2ki.hpp"
#include "duell.hpp"
#include "aufzeichnung.hpp"

Export void NoResponseToTrigger() {}

ExportLevelDetailsFull("Du gegen die KI (mp2_08)", "mp2_08.map", "MULTITEK.TXT", Colony, 3, 12, 0)
Export const AIModDescEx DescBlockEx = { 0 };

// Kartengroesse und Startplaetze der Original-Mission (Kacheln)
static const int BREITE = 64, HOEHE = 64;
static const int N = 2;
static const int X[] = { 8, 58 }, Y[] = { 10, 53 };

static void initProc() {
    op2::log::line("InitProc: starting");
    if (HFLInit() != HFLLOADED) {
        op2::log::line("InitProc: HFLInit FAILED");
    }

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
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(19, 4), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(40, 61), OreTypeCommon, Bar3, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(15, 57), OreTypeRare, Bar2, VariantRandom);
        TethysGame::CreateBeacon(mapMiningBeacon, XYPos(43, 14), OreTypeRare, Bar2, VariantRandom);
    }

    startwahl::paar(N, X, Y);
    if (op2ki::laden()) AddGameMessage("KI: Rust-KI (op2ki.dll) geladen");
    else AddGameMessage("KI: op2ki.dll fehlt - ohne sie gibt es keinen Gegner und keine Startbasis. Siehe OPU\\logs.");
    op2ki::start(1, MkXY(1, 1), BREITE, HOEHE, X[startwahl::a], Y[startwahl::a], true, 3, 3, 500);
    op2::log::linef("InitProc: Aufbau-KI Spieler 1 -> %s", op2::log::kiPath());
    aufzeichnung::start("mp2_08", BREITE, HOEHE, X[startwahl::b], Y[startwahl::b]);

    TethysGame::ForceMoraleGood(PlayerNum::PlayerAll);
    op2::log::line("InitProc: done");
}

static void aiProc() {
    // Nach einem Spielstand-Load laeuft InitProc nicht erneut -- HFL hier nachinitialisieren (idempotent).
    HFLInit();
    op2ki::schritt(1);
    aufzeichnung::schritt();
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
