// op2ki.hpp -- die Rust-KI (op2ki.dll) in einer Missions-DLL: laden, Stellschrauben uebergeben, starten, je Tick
// einen Schritt. Die OP2-API reicht op2ki_bruecke.hpp durch (erzeugt von werkzeug/bruecke.py).
// The Rust AI (op2ki.dll) inside a mission DLL: load it, pass parameters, start it, step it every tick.
#pragma once
#include <cstdio>
#include <cstring>
#include "op2_log.hpp"

namespace op2ki {

// Stellschrauben je Spieler ("name=wert" je Zeile); eine Datei logs/KI-Parameter-P<n>.txt geht vor.
inline const char* parameterText[8] = {};

// Datei neben der KI-Spur (OPU\logs): Stellschrauben, Aufzeichnung.
inline void wissenPfad(char* out, int n, const char* datei) {
  const char* k = op2::log::kiPath();
  int ende = -1;
  for (int i = 0; k[i]; ++i) if (k[i] == '\\' || k[i] == '/') ende = i;
  if (ende < 0) { std::snprintf(out, n, "%s", datei); return; }
  std::snprintf(out, n, "%.*s\\%s", ende, k, datei);
}

// Alle Gebaeudetypen, die die KI zaehlt (PlayerBuildingEnum braucht einen Typ).
static const int GEBAEUDE[] = {
  mapCommandCenter, mapStructureFactory, mapVehicleFactory, mapTokamak, mapMHDGenerator, mapSolarPowerArray,
  mapCommonOreSmelter, mapRareOreSmelter, mapCommonOreMine, mapRareOreMine, mapCommonStorage, mapRareStorage,
  mapAgridome, mapResidence, mapReinforcedResidence, mapAdvancedResidence, mapBasicLab, mapStandardLab,
  mapAdvancedLab, mapMedicalCenter, mapNursery, mapUniversity, mapRecreationFacility, mapForum, mapDIRT,
  mapGarage, mapConsumerFactory, mapGuardPost, mapLightTower, mapGORF, mapRobotCommand, mapObservatory,
  mapMeteorDefense, mapTradeCenter, mapArachnidFactory, mapSpaceport, mapGeothermalPlant, mapMagmaWell,
};

// Hoechste belegte Einheitennummer: HFL legt die Kartenstruktur des Spiels offen (mapObj, lastUsedUnitIndex bei Byte 64).
inline int hoechsteEinheit() {
  if (!isInited || !mapObj) return 0;
  const int n = *reinterpret_cast<const int*>(reinterpret_cast<const char*>(mapObj) + 64);
  return (n > 0 && n < 4096) ? n : 0;
}

// techID -> Tabellenindex des geladenen Techbaums (-1: gibt es nicht)
inline int techIndex(int techID) {
  static int ids[512], anzahl = -1;
  if (anzahl < 0) {
    anzahl = Research::GetTechCount();
    if (anzahl > 512) anzahl = 512;
    for (int i = 0; i < anzahl; ++i) { TechInfo ti = Research::GetTechInfo(i); ids[i] = ti.IsValid() ? ti.GetTechID() : -1; }
  }
  for (int i = 0; i < anzahl; ++i) if (ids[i] == techID) return i;
  return -1;
}

}  // namespace op2ki

#include "op2ki_bruecke.hpp"

namespace op2ki {

struct Api { void* ruf; void* text; void* lesen; void* schreiben; };
typedef void* (__cdecl *NeuFn)(Api);
typedef void (__cdecl *ParamFn)(void*, int, const char*, int);
typedef void (__cdecl *StartFn)(void*, int, int, int, int, int, int, int, int, int, int, int);
typedef void (__cdecl *SchrittFn)(void*, int);
typedef int (__cdecl *AbiFn)();
typedef void (__cdecl *BefehlFn)(void*, int, const char*, int);
typedef int (__cdecl *StatusFn)(void*, int, int);
static NeuFn neu; static ParamFn param; static StartFn startFn; static SchrittFn schrittFn; static void* h = nullptr; static int versucht = 0;
static BefehlFn befehlFn = nullptr; static StatusFn statusFn = nullptr;   // ab op2-ki 1.1; aeltere op2ki.dll: ohne Befehle

// Spielstand: was die KI nach dem Laden wieder braucht -- op2ki.dll faengt nach dem Laden leer an und baut ihr Bild aus
// der Spiellage neu auf; Stellschrauben, laufende Phase, Verteidigung und weitere Befehle werden wieder geschickt.
// In das Speicher-Struct der Mission (ExportSaveLoadData) aufnehmen und op2ki::speicher(&g_save.op2ki) in InitProc und
// AIProc aufrufen.
struct SpielerStand { int an, ux, uy, breite, hoehe, x, y; char param[512]; char phase[768]; char vert[192]; char sonst[256]; };
struct Speicher { SpielerStand s[7]; };
static Speicher* g_sp = nullptr;
inline void speicher(Speicher* s) { g_sp = s; }

// op2ki.dll laden: zuerst neben der Missions-DLL (OPU-Ordner), unabhaengig vom Arbeitsverzeichnis des Launchers
inline bool laden() {
  if (versucht) return h != nullptr;
  versucht = 1;
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
  befehlFn = (BefehlFn)GetProcAddress(m, "ki2_befehl"); statusFn = (StatusFn)GetProcAddress(m, "ki2_status");
  if (!neu || !param || !startFn || !schrittFn) { op2::log::line("Rust-KI 2: Funktionen fehlen"); return false; }
  Api a = { (void*)&rsapi::ruf, (void*)&rsapi::text, (void*)&rsapi::lesen, (void*)&rsapi::schreiben };
  h = neu(a);
  op2::log::linef("Rust-KI 2: geladen, ABI %d", abi ? abi() : -1);
  return h != nullptr;
}

// Einmal in InitProc je KI-Spieler. `ursprung` = Engine-Koordinate der Editor-Kachel (0,0), z.B. MkXY(1, 1); x, y =
// Startplatz (Kacheln); `ausruestung`: Startbasis anlegen (Umfang `stufe`). false = op2ki.dll fehlt.
inline bool start(int spieler, LOCATION ursprung, int breite, int hoehe, int x, int y, bool ausruestung, int ziel, int stufe, int wert) {
  if (!laden()) return false;
  for (int p = 0; p < 8; ++p) if (parameterText[p]) param(h, p, parameterText[p], (int)std::strlen(parameterText[p]));
  startFn(h, spieler, ursprung.x, ursprung.y, breite, hoehe, x, y, ausruestung ? 1 : 0, ziel, stufe, wert);
  if (g_sp && spieler >= 0 && spieler < 7) {
    SpielerStand& s = g_sp->s[spieler];
    s.an = 1; s.ux = ursprung.x; s.uy = ursprung.y; s.breite = breite; s.hoehe = hoehe; s.x = x; s.y = y;
    std::snprintf(s.param, sizeof s.param, "%s", parameterText[spieler] ? parameterText[spieler] : "");
  }
  return true;
}

// Jeden Tick aus AIProc (die KI taktet selbst).
inline void schritt(int spieler) { if (h) schrittFn(h, spieler); }

// Befehl an die KI waehrend des Spiels, "name=wert" je Zeile oder durch ";" getrennt (siehe EINBINDEN.md): Drehbuch-Phase,
// Angriff, Verteidigung, Rolle, Stellschrauben. Phase, Verteidigung und Einstellungen kommen in den Spielstand.
inline void befehl(int spieler, const char* text) {
  if (!text) return;
  if (h && befehlFn) befehlFn(h, spieler, text, (int)std::strlen(text));
  if (!g_sp || spieler < 0 || spieler >= 7) return;
  SpielerStand& s = g_sp->s[spieler];
  if (std::strstr(text, "phase=")) std::snprintf(s.phase, sizeof s.phase, "%s", text);
  else if (std::strstr(text, "drehbuch=0")) s.phase[0] = 0;
  else if (std::strstr(text, "verteidigung=0")) s.vert[0] = 0;
  else if (std::strstr(text, "verteidigung=")) std::snprintf(s.vert, sizeof s.vert, "%s", text);
  else if (!std::strstr(text, "angriff=")) {
    const size_t n = std::strlen(s.sonst);
    if (n + std::strlen(text) + 2 < sizeof s.sonst) { if (n) std::strcat(s.sonst, "\n"); std::strcat(s.sonst, text); }
    else std::snprintf(s.sonst, sizeof s.sonst, "%s", text);
  }
}

// Rueckmeldung der KI: 1 zuletzt fertige Phase, 2 laufende Phase, 3 Angriff (0 keiner, 1 Truppe wird gebaut, 2 unterwegs,
// 3 zuletzt gewonnen, 4 zuletzt abgebrochen), 4 beendete befohlene Angriffe, 5 Tick des letzten Angriffs auf die Basis,
// 6 Satelliten, 7 Gebaeude, 8 Kampffahrzeuge, 9 Kampfwert der Armee, 10 selbststaendige zweite Basen; 100 + id: Phase id fertig
inline int status(int spieler, int code) { return (h && statusFn) ? statusFn(h, spieler, code) : 0; }

// Nach dem Laden eines Spielstands (InitProc lief nicht): die KI aus der Spiellage neu starten und die gemerkten Befehle
// wieder schicken. Jeden AIProc vor schritt() aufrufen; tut im laufenden Spiel nichts.
inline void fortsetzen() {
  if (versucht || !g_sp) return;
  bool jemand = false;
  for (int p = 0; p < 7; ++p) if (g_sp->s[p].an) jemand = true;
  if (!jemand) return;
  if (!laden()) return;
  for (int p = 0; p < 7; ++p) {
    SpielerStand& s = g_sp->s[p];
    if (!s.an) continue;
    param(h, p, s.param, (int)std::strlen(s.param));
    startFn(h, p, s.ux, s.uy, s.breite, s.hoehe, s.x, s.y, 0, 3, 3, 500);
    if (befehlFn) {
      if (s.sonst[0]) befehlFn(h, p, s.sonst, (int)std::strlen(s.sonst));
      if (s.vert[0]) befehlFn(h, p, s.vert, (int)std::strlen(s.vert));
      if (s.phase[0]) befehlFn(h, p, s.phase, (int)std::strlen(s.phase));
    }
    op2::log::linef("op2-ki: Spieler %d nach dem Laden fortgesetzt", p);
  }
}

}  // namespace op2ki
