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
static NeuFn neu; static ParamFn param; static StartFn startFn; static SchrittFn schrittFn; static void* h = nullptr; static int versucht = 0;

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
  return true;
}

// Jeden Tick aus AIProc (die KI taktet selbst).
inline void schritt(int spieler) { if (h) schrittFn(h, spieler); }

}  // namespace op2ki
