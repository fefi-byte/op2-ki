// aufzeichnung.hpp -- "Du gegen die KI": Spieler 0 ist der Mensch, alles, was er tut, wird aufgezeichnet:
// logs/Aufzeichnung-<Datum>-<Zeit>.jsonl. Nicht Teil der KI.
// Zeilen: "kopf" (einmal), "neu"/"ae" (Einheit erschienen/geaendert: Typ, Waffe/Fracht, Ort intern, Aktion, letzter
// Befehl, Schaden, Zusatz, Flags), "weg" (Einheit verschwunden), "kol" (alle 100 Ticks Kolonie + neue Techniken),
// "feind" (alle 500 Ticks alle Einheiten von Spieler 1), "ende". Orte in Engine-Kacheln (Location().x/.y).
#pragma once
#include <cstdio>
#include <cstring>
#include "op2ki.hpp"
#include "duell.hpp"

namespace aufzeichnung {
static FILE* f = nullptr;
static const char* karte = "";
static int bx = 0, by = 0, moralFrei = 0, fertig = 0, naechsteKol = 0, naechsterFeind = 0, naechsterFlush = 0, naechstesEnde = 1000;
struct Stand { int typ, w, x, y, a, c, s, z, fl, posT, lebt; };
static const int MAXE = 4096;
static Stand alt[MAXE];
static unsigned char techDa[512];

inline void kopf() {
    SYSTEMTIME st; GetLocalTime(&st);
    char nm[96], pfad[600];
    std::snprintf(nm, sizeof nm, "Aufzeichnung-%04d%02d%02d-%02d%02d%02d.jsonl", st.wYear, st.wMonth, st.wDay, st.wHour, st.wMinute, st.wSecond);
    op2ki::wissenPfad(pfad, sizeof pfad, nm);
    f = std::fopen(pfad, "wb");
    if (!f) { op2::log::linef("Aufzeichnung: %s nicht schreibbar", pfad); return; }
    static char puffer[1 << 16]; setvbuf(f, puffer, _IOFBF, sizeof puffer);
    int kx = -1, ky = -1, mx = -1, my = -1;
    { PlayerBuildingEnum e(1, mapCommandCenter); Unit u; if (e.GetNext(u)) { kx = u.Location().x; ky = u.Location().y; } }
    { PlayerBuildingEnum e(0, mapCommandCenter); Unit u; if (e.GetNext(u)) { mx = u.Location().x; my = u.Location().y; } }
    std::fprintf(f, "{\"t\":%d,\"k\":\"kopf\",\"version\":1,\"karte\":\"%s\",\"datum\":\"%04d-%02d-%02d %02d:%02d:%02d\",\"mensch\":0,\"ki\":1,"
                    "\"menschEden\":%d,\"kiEden\":%d,\"menschStart\":[%d,%d],\"menschCC\":[%d,%d],\"kiCC\":[%d,%d],\"schwierigkeit\":%d}\n",
                 TethysGame::Tick(), karte, st.wYear, st.wMonth, st.wDay, st.wHour, st.wMinute, st.wSecond, Player[0].IsEden() ? 1 : 0,
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
    const int anzahl = Research::GetTechCount();
    for (int i = 0; i < anzahl && i < 512; ++i) {
        TechInfo ti = Research::GetTechInfo(i);
        const int id = ti.IsValid() ? ti.GetTechID() : -1;
        if (techDa[i] || id <= 0 || !Player[0].HasTechnology(id)) continue;
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


// Spielende wie in den Duellen: wer keines der Kerngebaeude mehr hat, hat verloren.
inline void ende(int t) {
    if (fertig || t < naechstesEnde) return;
    naechstesEnde = t + 100;
    const int g0 = spielende::kern(0, true), g1 = spielende::kern(1, true);
    if (g0 && g1) return;
    fertig = 1;
    if (f) { std::fprintf(f, "{\"t\":%d,\"k\":\"ende\",\"sieger\":%d,\"kern0\":%d,\"kern1\":%d}\n", t, g1 ? 1 : 0, g0, g1); std::fflush(f); }
    op2::log::linef("Spielende: Tick %d, Kerngebaeude Mensch: %d, KI: %d", t, g0, g1);
    Trigger tr = CreateTimeTrigger(1, 1, 1, "NoResponseToTrigger");
    if (!g1) CreateVictoryCondition(1, 0, tr, "Die KI ist besiegt");
    else CreateFailureCondition(1, 0, tr, "");
}

inline void schritt() {
    const int t = TethysGame::Tick();
    if (!moralFrei) { TethysGame::FreeMoraleLevel(0); moralFrei = 1; }   // InitProc setzt ForceMoraleGood fuer alle
    if (!f) { kopf(); if (!f) { ende(t); return; } }
    erfassen(t);
    if (t >= naechsteKol) { naechsteKol = t + 100; kolonie(t); }
    if (t >= naechsterFeind) { naechsterFeind = t + 500; feind(t); }
    ende(t);
    if (t >= naechsterFlush) { naechsterFlush = t + 1000; std::fflush(f); }
}

// Startbasis fuer Spieler 0 (Mensch) an x, y auf einer Karte breite x hoehe -- dieselbe Stufe-3-Basis wie die KI, angelegt von op2ki.dll (ohne
// KI-Schritte). false = op2ki.dll fehlt.
inline bool start(const char* k, int breite, int hoehe, int x, int y) {
    karte = k; bx = x; by = y;
    const bool ok = op2ki::start(0, MkXY(1, 1), breite, hoehe, x, y, true, 3, 3, 500);
    op2::log::linef("Mensch: Spieler 0 an %d,%d (Startstufe %d)", x, y, 3);
    return ok;
}
}  // namespace aufzeichnung
