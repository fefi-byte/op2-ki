#pragma once
// op2_ki.hpp - eine KI, die eine Kolonie von null aufbaut (Editor-Geruest, erster Baustein).
//
// Laeuft in AIProc der Missions-DLL. Sie liest den Spielstand (Gebaeude, Fahrzeuge, Rohstoffe, Nahrung,
// Strom), entscheidet nach Bedarf, was als Naechstes gebaut wird, laesst die Strukturfabrik die Bausaetze
// herstellen, holt sie mit ConVecs ab und baut sie so um die Kommandozentrale, dass die Roehrenstuecke,
// die jedes Gebaeude rechts und unten mitbringt, sie verbinden (am Anfang gibt es kein Metall fuer
// weitere Roehren). Robo-Surveyor und Robo-Miner erschliessen Erz, Lastwagen
// fahren es per MiningGroup zur Schmelze. Labore forschen selbststaendig.
//
// Jede Entscheidung landet mit Begruendung in <Mission>-KI.jsonl (eine JSON-Zeile je Ereignis, dazu
// regelmaessig eine Momentaufnahme). Der Editor liest die Datei in der Ansicht "KI-Spur" und zeigt sie
// auf der Karte, mit Zeitleiste -- so sieht man, warum die KI etwas tut oder haengen bleibt.
//
// Der ganze Zustand steckt in `ki::Zustand` mit fester Groesse, ohne Zeiger -- er gehoert in die
// SaveRegion der Mission, sonst hat die KI nach dem Laden eines Spielstands alles vergessen.
// Nur Einzelspieler: Entscheidungen haengen nur vom Spielstand ab (Zufall ueber TethysGame::GetRand),
// fuer Mehrspieler ist das aber noch nicht geprueft.
//
// An AI that builds a colony from scratch (editor scaffolding, first building block). Runs in AIProc,
// decides by need what to build next, has the Structure Factory produce kits, fetches them with ConVecs
// and places them so the tube stubs every building brings (right and bottom) connect them. Every decision
// is written with its reason to <Mission>-KI.jsonl, read by the editor's "AI trace" view. All state lives
// in the fixed-size `ki::Zustand` (no pointers) -- it belongs in the mission's SaveRegion.
//
// Benoetigt / needs: Outpost2DLL, HFL (UnitEx, PlayerEx, Research), op2_log.hpp (kiRaw).

#include <cmath>
#include <cstdarg>
#include "op2_ki_techbaum.hpp"   // erzeugt von techbaum.py: Wirkungen aller Techniken, Grundwerte der Einheiten
#include <cstdlib>
#include <cstdio>
#include <cstring>

// ---------------------------------------------------------------------------------------------------
// Testbeschleunigung / test speed-up (nur mit Umgebungsvariable OP2_TEMPO, z.B. OP2_TEMPO=4)
//
// Das Spiel taktet seine Ticks nach timeGetTime/QueryPerformanceCounter und wartet sonst aktiv (gemessen: ein Kern
// voll, aber meist in Nachrichten- und Mausabfragen). Fuer Testlaeufe biegt die Missions-DLL die beiden Eintraege der
// Importtabelle von Outpost2.exe (1.2.x: 0x5838e0 timeGetTime, 0x583630 QueryPerformanceCounter) auf eine Uhr um,
// die OP2_TEMPO-mal so schnell laeuft. Ohne die Variable aendert sich nichts (Spiel gegen Menschen).
// For test runs only: the IAT entries for the game's clocks are redirected to a clock running OP2_TEMPO times faster.
namespace tempo {
inline double faktor = 1;
inline DWORD (WINAPI* echtZeit)() = nullptr;
inline BOOL (WINAPI* echtQpc)(LARGE_INTEGER*) = nullptr;
inline DWORD basisZeit = 0;
inline LONGLONG basisQpc = 0;
inline DWORD WINAPI zeit() { const DWORD e = echtZeit(); return basisZeit + (DWORD)((double)(DWORD)(e - basisZeit) * faktor); }
inline BOOL WINAPI qpc(LARGE_INTEGER* z) {
  LARGE_INTEGER e; const BOOL ok = echtQpc(&e);
  if (ok && z) z->QuadPart = basisQpc + (LONGLONG)((double)(e.QuadPart - basisQpc) * faktor);
  return ok;
}
// Bild nur selten ins Fenster kopieren (gemessen: bei schneller Uhr gehen 2/3 der Bildzeit in BitBlt auf das
// Fensterbild, 1186x705 Punkte ueber GDI an den X-Server). Nur Kopien auf ein Fenster (OBJ_DC) werden gedrosselt --
// das Zusammensetzen in Speicherbildern laeuft weiter, die Spiellogik merkt nichts. Alle ZEICHNEN_ABSTAND ms (echte
// Spieler: "alle 50 Bilder ein Bild" -- je Bild ein Spieltick, also wird in jedem 50. Tick alles durchgelassen
// (alle Kopien desselben Ticks, damit ein ganzes Bild entsteht; Bildschirmfotos bleiben aktuell).
// Only blits onto a window DC are throttled (every ZEICHNEN_JEDER-th game tick); memory-DC composition continues.
constexpr int ZEICHNEN_JEDER = 50;
inline BOOL (WINAPI* echtBitBlt)(HDC, int, int, int, int, HDC, int, int, DWORD) = nullptr;
inline int (WINAPI* echtStretch)(HDC, int, int, int, int, int, int, int, int, const void*, const BITMAPINFO*, UINT, DWORD) = nullptr;
inline int zeichenTick = -1000;
inline bool zeichnen(HDC ziel) {
  if (GetObjectType(ziel) != OBJ_DC) return true;               // Speicherbild: immer
  const int t = TethysGame::Tick();
  if (t - zeichenTick >= ZEICHNEN_JEDER || t < zeichenTick) zeichenTick = t;
  return t == zeichenTick;
}
inline BOOL WINAPI bitBlt(HDC d, int x, int y, int w, int h, HDC q, int qx, int qy, DWORD rop) {
  return zeichnen(d) ? echtBitBlt(d, x, y, w, h, q, qx, qy, rop) : TRUE;
}
inline int WINAPI stretch(HDC d, int x, int y, int w, int h, int qx, int qy, int qw, int qh, const void* bits,
                          const BITMAPINFO* bi, UINT u, DWORD rop) {
  return zeichnen(d) ? echtStretch(d, x, y, w, h, qx, qy, qw, qh, bits, bi, u, rop) : h;
}
// Maus und Tasten: das Spiel fragt sie je Bild beim X-Server ab (gemessen: ~40 % der Zeit in GetCursorPos bei
// gedrosseltem Bild). Fuer Testlaeufe nur alle ZEICHNEN_JEDER Ticks wirklich abfragen, dazwischen den letzten Wert.
// Mouse and key polling: real query only every ZEICHNEN_JEDER ticks, cached in between.
inline BOOL (WINAPI* echtCursor)(POINT*) = nullptr;
inline SHORT (WINAPI* echtTaste)(int) = nullptr;
inline POINT cursor = { 0, 0 };
inline int cursorTick = -1000;
inline SHORT tasten[256];
inline int tastenTick[256];
inline BOOL WINAPI cursorPos(POINT* p) {
  const int t = TethysGame::Tick();
  if (t - cursorTick >= ZEICHNEN_JEDER || t < cursorTick) { echtCursor(&cursor); cursorTick = t; }
  if (p) *p = cursor;
  return TRUE;
}
inline SHORT WINAPI tasteAsync(int k) {
  const int i = k & 0xff, t = TethysGame::Tick();
  if (t - tastenTick[i] >= ZEICHNEN_JEDER || t < tastenTick[i]) { tasten[i] = echtTaste(k); tastenTick[i] = t; }
  return tasten[i];
}
inline void umbiegen(DWORD* slot, DWORD neu, DWORD* alt) {
  DWORD schutz;
  VirtualProtect(slot, 4, PAGE_READWRITE, &schutz);
  *alt = *slot; *slot = neu;
  VirtualProtect(slot, 4, schutz, &schutz);
}
inline void an() {
  static bool schon = false;
  if (schon) return;
  schon = true;
  char b[32];
  if (!GetEnvironmentVariableA("OP2_TEMPO", b, sizeof b)) return;
  const double f = std::atof(b);
  if (f <= 1.0 || f > 1000) return;
  DWORD* zeitSlot = (DWORD*)0x5838e0; DWORD* qpcSlot = (DWORD*)0x583630;
  // nur, wenn die Eintraege wirklich auf winmm/kernel32 zeigen (andere Spielversion: nichts tun)
  if (*zeitSlot != (DWORD)GetProcAddress(GetModuleHandleA("winmm.dll"), "timeGetTime") &&
      *zeitSlot != (DWORD)GetProcAddress(GetModuleHandleA("winmm"), "timeGetTime")) return;
  if (*qpcSlot != (DWORD)GetProcAddress(GetModuleHandleA("kernel32.dll"), "QueryPerformanceCounter")) return;
  faktor = f;
  DWORD alt;
  VirtualProtect(zeitSlot, 4, PAGE_READWRITE, &alt);
  echtZeit = (DWORD (WINAPI*)())*zeitSlot; basisZeit = echtZeit(); *zeitSlot = (DWORD)&zeit;
  VirtualProtect(zeitSlot, 4, alt, &alt);
  VirtualProtect(qpcSlot, 4, PAGE_READWRITE, &alt);
  echtQpc = (BOOL (WINAPI*)(LARGE_INTEGER*))*qpcSlot;
  { LARGE_INTEGER e; echtQpc(&e); basisQpc = e.QuadPart; }
  *qpcSlot = (DWORD)&qpc;
  VirtualProtect(qpcSlot, 4, alt, &alt);
  // Zeichnen drosseln (Importtabelle 1.2.x: 0x58354c BitBlt, 0x583568 StretchDIBits), nur wenn die Eintraege passen
  HMODULE gdi = GetModuleHandleA("gdi32.dll");
  DWORD* bbSlot = (DWORD*)0x58354c; DWORD* sdSlot = (DWORD*)0x583568;
  if (gdi && *bbSlot == (DWORD)GetProcAddress(gdi, "BitBlt")) umbiegen(bbSlot, (DWORD)&bitBlt, (DWORD*)&echtBitBlt);
  if (gdi && *sdSlot == (DWORD)GetProcAddress(gdi, "StretchDIBits")) umbiegen(sdSlot, (DWORD)&stretch, (DWORD*)&echtStretch);
  // Maus und Tasten puffern (Importtabelle 1.2.x: 0x583898 GetCursorPos, 0x58389c GetAsyncKeyState)
  HMODULE user = GetModuleHandleA("user32.dll");
  for (int i = 0; i < 256; ++i) { tasten[i] = 0; tastenTick[i] = -1000; }
  DWORD* cpSlot = (DWORD*)0x583898; DWORD* akSlot = (DWORD*)0x58389c;
  if (user && *cpSlot == (DWORD)GetProcAddress(user, "GetCursorPos")) umbiegen(cpSlot, (DWORD)&cursorPos, (DWORD*)&echtCursor);
  if (user && *akSlot == (DWORD)GetProcAddress(user, "GetAsyncKeyState")) umbiegen(akSlot, (DWORD)&tasteAsync, (DWORD*)&echtTaste);
}
}  // namespace tempo

namespace ki {

// ---------------------------------------------------------------------------------------------------
// Groessen und Takt / sizes and timing

constexpr int TAKT          = 20;    // alle 20 Ticks ein Entscheidungsschritt

// Stellschrauben (Parameter) je Spieler -- Standardwerte hier, ueberschreibbar aus logs/KI-Parameter-P<n>.txt
// (Zeilen name=wert). Der Optimierer (ki-test/optimierer.py) schreibt diese Datei vor jedem Duell.
// lernen=0: kein Langzeit-Wissen laden oder schreiben (jeder Lauf beginnt gleich -- noetig fuer den Optimierer).
// Tunable parameters per player; defaults here, overridable from logs/KI-Parameter-P<n>.txt.
struct Parameter {
  double reaktion = 6000;          // Ticks, die die KI zum Nachruesten braucht (Feindwachstum, Fabrikentscheidung)
  double horizont = 20000;         // Planungshorizont der Rohstoffbilanz
  double ueberlegenheit = 1.5;     // so viel staerker muss die Truppe fuer Basis- und Zielangriff sein
  double fabrikGewinn = 0.8;       // weitere Fahrzeugfabrik, wenn die Aufruestung danach hoechstens so lange dauert
  double reserveStart = 1.3;       // Reserve auf die Feindarmee zu Beginn
  double reserveLernrate = 0.3;    // Anstieg der Reserve je Anteil verlorenen Kampfwerts in einem Gefecht
  double reserveRueckgang = 0.97;  // Faktor nach einem Gefecht ohne Verluste
  double reserveMax = 2.5;         // Deckel der Reserve
  double turmAnteil = 0.25;        // Anteil des Solls, den Wachtuerme decken duerfen
  double zermuerbAbstand = 8000;   // Ticks zwischen Zermuerbungsangriffen (Start)
  double schmelzeNutzen = 1.0;     // Mindestnutzen (beseitigter Engpass / Kosten) fuer Schmelze bzw. Seltenerz-Schmelze
  double wohnraumGrenze = 85;      // Auslastung in %, ab der Wohnraum dringlich ist und kein Haus ueberzaehlig
  double freischaltWert = 0.3;     // Anteil am Nutzen der Techniken, die eine Technik freischaltet
  double nahRadius = 22;           // Feind-Kampfeinheiten so nah an der Basismitte = Angriff auf die Kolonie
  double heimHalten = 1.0;         // Faktor auf das, was beim Angriff daheim bleiben muss
  double steigerung = 1.15;        // jeder Angriff soll so viel staerker sein als der letzte (Eskalation)
  double empSchwelle = 1.0;         // EMP-Rakete, wenn die gelaehmte Abwehr >= Schwelle x Kosten der Rakete (in Kampfwert)
  double schmelzenJeMine = 2;      // neue Erzmine, sobald so viele Schmelzen je Mine stehen (und Erz fehlt); 2 = Mine voll
  double vorsorge = 60;            // Militaerforschung mindestens, solange ein Feind bekannt und kein Kampffahrzeug erforscht ist
  double seltenGewinn = 0.25;      // Seltenerz-Schmelze bauen und betreiben, sobald Fahrzeuge mit Seltenerz so viel mehr Kampfkraft je Erz bringen (0.25 = +25 %)
  double risikoAnteil = 0.75;      // Anteil der Wache, der beim Angriff mindestens mitfaehrt
  double angriffMarge = 0.03;      // Zermuerben: erwarteter Gewinn mindestens dieser Anteil der eigenen Staerke
  double rueckzugAb = 0.8;         // Rueckzug, wenn eigene Staerke vor Ort < Faktor x feindliche Staerke vor Ort
  double lernen = 1;               // 0 = kein Langzeit-Wissen
};
inline Parameter par[8];
// Eingebaute Stellschrauben einer Mission (z.B. gelernte Werte), Zeilen name=wert wie in der Datei. Die Datei
// logs/KI-Parameter-P<n>.txt geht vor. Setzen per statischem Initialisierer im custom_code der Mission.
// Parameters baked into a mission; the file in logs/ takes precedence.
inline const char* parameterText[8] = {};
#define PAR(z) ::ki::par[(z).spieler & 7]
constexpr int SPUR_TAKT     = 200;   // alle 200 Ticks eine Momentaufnahme in die Spur
constexpr int MAX_PLAETZE   = 48;    // geplante/gebaute Bauplaetze
constexpr int MAX_ROEHREN   = 48;    // offene Roehrenstuecke
constexpr int MAX_MINEN     = 4;     // Minen mit eigener MiningGroup
constexpr int MAX_VF        = 4;     // Fahrzeugfabriken, die die KI verwaltet (Feldgroesse, keine Entscheidung)
constexpr int GEDULD_BAU    = 1500;  // so lange darf ein ConVec fuer einen Bau brauchen
constexpr int GEDULD_KIT    = 2500;  // so lange darf ein Bausatz in der Fabrik dauern

// Zustaende eines Bauplatzes / states of a building site
enum PlatzZustand : short { P_FREI = 0, P_FAHRT = 1, P_BAUT = 2, P_STEHT = 3, P_GESPERRT = 9 };

struct Platz {
  short x0, y0;      // Grundflaeche oben links (Editor-Kacheln)
  short typ;         // map_id
  short zustand;     // PlatzZustand
  int   convec;      // unitID des ConVecs, der hier baut (0 = keiner)
  int   seit;        // Tick der letzten Aenderung
  short versuche;
  short raster;      // frei (frueher Rasterplatz), immer -1
};

struct Roehre {      // ein gerades Roehrenstueck (Editor-Kacheln, inklusive)
  short x1, y1, x2, y2;
  short zustand;     // 0 offen, 1 beauftragt, 2 fertig
  short versuche;
  int   seit;
};

struct Zustand {
  int aktiv;
  int spieler;
  int ux, uy;                 // Engine-Koordinate der Editor-Kachel (0,0)
  int breite, hoehe;          // Kartengroesse in Kacheln
  int bx, by;                 // Basismitte (Editor-Kacheln)
  int naechsterSchritt;
  int naechsteSpur;
  int naechsterAnschluss;
  int forschungModus;         // 0: DoResearch mit techID, 1: mit Tabellenindex (kalibriert sich selbst)
  int forschungLab;           // unitID des Labors, dessen Start geprueft wird
  int forschungWert;          // angefragtes Thema
  int forschungSeit;
  int kitTyp;                 // Bausatz, der gerade in der Fabrik entsteht (0 = keiner)
  int kitSeit;
  int kitGesperrtTyp[8];      // Bausaetze, die die Fabrik (noch) nicht herstellt
  int kitGesperrtBis[8];
  int kitPauseBis;            // Fabrik hat den letzten Auftrag nicht angenommen -> kurz warten
  int fzTyp;                  // Fahrzeug, das gerade in der Fahrzeugfabrik entsteht
  int fzSeit;
  int fzPauseBis;
  int meldungenSeit[32];      // Entprellung wiederkehrender Meldungen
  Platz plaetze[MAX_PLAETZE];
  int anzahlPlaetze;
  Roehre roehren[MAX_ROEHREN];
  int anzahlRoehren;
  int minenId[MAX_MINEN];     // unitID der Mine je MiningGroup (0 = frei)
  int schmelzeId[MAX_MINEN];
  MiningGroup minenGruppe[MAX_MINEN];
  int bakeZiel;               // unitID der Bake, die als Naechstes erschlossen wird
  int bakeMiner;              // unitID des Miners, der gerade eine Mine baut
  int bakeSeit;               // seit wann diese Bake das Ziel ist
  int minerSeit;              // wann der Miner den Auftrag bekam
  int bakenX[4], bakenY[4];   // selbst angelegte Baken (Editor-Kacheln)
  int anzahlBaken;
  short kbX[32], kbY[32];     // alle bekannten Baken (Editor-Kacheln) -- die Aufzaehler liefern keine, platzFrei braucht sie
  int kbN, kbStand;           // Anzahl, Tick der letzten Erfassung
  int sperrBake[8], sperrBis[8];  // Baken, an denen keine Mine zustande kam, gesperrt bis Tick
  int minerWeg;               // wie der Miner zum Bauen gebracht wird: 1 DoBuild, 2 BuildingGroup, 0 HFL
  int minenGesehen;           // so viele Minen standen beim letzten Schritt
  int minenBauAn;
  BuildingGroup minenBau;     // Weg 2: die Spiel-KI setzt den Miner ein
  int ladeConvec, ladeSeit;   // ConVec, der gerade an der Strukturfabrik laedt
  // Ziel Raumfahrt: einen EDWARD-Satelliten mit einer SULV-Rakete ins All schicken.
  // Space goal: launch an EDWARD satellite on a SULV rocket.
  int ziel;                   // 0 = nur Kolonie, 1 = Satellit starten
  int zielErreicht;           // Tick, an dem der Satellit im All war (0 = noch nicht)
  int raumSchritt;            // Ablauf am Spaceport, siehe raumfahrt()
  int raumSeit;
  int raumVersuche;
  int raumAuftrag;            // was der Spaceport gerade herstellt (0 = nichts)
  int seltenMineId[2];        // Seltenerz: Mine je MiningGroup
  int seltenSchmelzeId[2];
  MiningGroup seltenGruppe[2];
  int seltenBake;             // Seltenerz-Bake, die erschlossen wird
  int seltenBakeSeit;
  int seltenMiner, seltenMinerSeit;
  // Startvorrat der Mission: die Engine kappt Erz ohne Lager auf 0 -- die KI lagert ihn ein, sobald
  // das erste Lager (Schmelze) steht. Start ore: the engine clamps ore to 0 without storage.
  int startErz, startSelten;
  // Bauen ueber die BuildingGroup der Engine / building via the engine's BuildingGroup
  BuildingGroup bau;
  int bauAn;
  // Arbeiter-Rangliste: von der KI stillgelegte Gebaeude (DoIdle), um Arbeiter fuer wichtigere freizumachen
  int geparkt[16];
  int geparktGrund[16];        // 0 = Arbeiter-Rangliste, 1 = Leerlauf (nur der zustaendige Bereich weckt es)
  int anzahlGeparkt;
  int leerId[16], leerSeit[16];  // seit wann ein Gebaeude untaetig ist (Leerlauf-Regel)
  int anzahlLeer;
  int naechsteArbeit;
  int moralFrei;               // FreeMoraleLevel schon aufgerufen?
  unsigned char techGemeldet[512]; int naechsteTechPruefung;   // abgeschlossene Forschung fuer die Spur
  int fabrikStill;             // unitID der Fahrzeugfabrik, die die KI mangels Bedarf stillgelegt hat
  int fabrikStillW[MAX_VF];    // weitere Fahrzeugfabriken, die mangels Kampfbedarf stillgelegt sind
  // Arbeiter-Wunsch: ein Bereich braucht ein stillgelegtes Gebaeude, es fehlen aber Arbeiter zum Einschalten
  // (DoUnIdle wirkt nur mit genug freien Arbeitern, Lauf 21/23). Die Rangliste macht dafuer welche frei.
  int wunschArbeiter, wunschRang, wunschSeit;
  int naechsteVerteilung;      // Wissenschaftler umverteilen (wissenschaftlerVerteilen)
  // Kampf: Lagebild der Gegner aus dem Kartenscan (siehe karteScannen) und die eigene Wache
  // Combat: picture of the enemies from the map scan and the own guard group
  int naechsterScan;
  int feindArmee, feindNah, feindGeb, feindX, feindY, feindFabriken, feindWT;
  int feindBestWert, feindBestTyp, feindBestWaffe;   // staerkstes gesehenes Feindfahrzeug (Qualitaet)
  int forschGestartet[4];      // gestartete Themen je Bereich (Kolonie, Ziel, Kampf, Wirtschaft) -- Kuchen anteilig
  // EMP-Rakete (Plymouth): Rakete auf der Rampe, Produktion laeuft seit, Startzeit, Aufmarschpunkt der Truppe
  int empRakete, empAuftrag, empAuftragSeit, empStart, empZielX, empZielY, aufmarschX, aufmarschY, aufmarschSeit;
  int eigeneArmee, sollArmee, bedrohung;
  int letzterAngriff, angriffe, feindGemeldet;
  int staerksterAngriff;       // groesste Feindarmee bisher (auf der Karte oder vor der Basis) -- Gedaechtnis zwischen den Wellen
  int eigeneGeb;               // eigene Gebaeude beim letzten Scan (Verluste erkennen)
  FightGroup wache;
  int wacheAn, wacheAnzahl;
  // Feindliche Kampfeinheiten aus dem Kartenscan (bis 16), die naechste zur Basis
  int feindPosN, feindPosX[16], feindPosY[16];
  int feindMinD2, feindNX, feindNY;
  int eigeneMobil, eigeneWTWert;
  int kampfModus, kampfModusSeit, angriffStart;   // 0 Wache, 1 Abfangen, 2 Angriff auf die Feindbasis
  // Aussenposten: Minen abseits der Basis (eigener Turm, eigene kleine Wache)
  int aussenId[6], aussenX[6], aussenY[6], aussenAngriff[6], aussenAn[6];
  int angriffSektoren;
  int naechsteVorhersage;       // wann die Techbaum-Vorhersage wieder in die Spur geschrieben wird
  // Rohstoff-Bilanz: gemessener Zufluss (Erz/Seltenerz je 1000 Ticks, gleitender Mittelwert)
  int messErz, messSelten, messTick;
  double zuflussErz, zuflussSelten;
  // Wellen-Gedaechtnis fuer die Prognose: Spitzenwert der Feindarmee je Welle
  int wellen[8], anzWellen, welleSpitze;
  int welleStart, wellenAbstand;   // Beginn der letzten Welle, gemessener Abstand zwischen zwei Wellen (0 = unbekannt)
  int feindUnterwegs;              // Kampfwert feindlicher Fahrzeuge ausserhalb ihrer Basis
  double feindWachstum;            // gemessener Zuwachs der Feindarmee je 1000 Ticks (gleitend)
  int feindArmeeVorher;
  int zielWert;                    // Ziel 2: Einwohnerzahl
  double turmAnteil;               // Anteil des Solls, den Wachtuerme decken duerfen (Rest: Fahrzeuge) -- Parameter
  // Lernen (auch ueber Laeufe hinweg, KI-Wissen.txt): Reserve auf die erwartete Welle, aus den Verlusten gelernt
  double reserve;
  int welleEigenStart, welleEigenMin;
  int laufNr, naechstesSpeichern;
  double erzJeRouteGelernt;
  // Lauf-Bewertung
  int verlustSumme, maxBew, ueberstanden, feindVernichtet;
  double moralSumme; int moralN;
  // Zerstoerung zaehlen: Einheiten-IDs (Feind / eigene) mit Erzwert vom letzten Scan; was fehlt, ist zerstoert
  int feindPos[192], eigenPos[192], zerstoertHeim, verlorenHeim, bilZerHeim, bilVerHeim;   // Verluste nach Ort: nah an der eigenen Basis oder nicht
  int garBucht[6], garFrei[6], garDran, garDranSeit, garRaus, garRausSeit, garZaehl;   // Garage: wer steht in welcher Bucht, wer faehrt hin
  int zielEmp, bilUeber, spinneZiel[8], spinneId[8], spinneTakt;   // EMP-Anteil der Sammelgruppe; Spinnen: wer uebernimmt wen
  int bilRueck, bilRueckErz, jagdZiel, jagdSeit, jagdTakt;   // Rueckzug je Angriff; Verfolgung: gejagte Einheit
  int bilK[6], bilK2[3], bilKTick;     // erster Feindkontakt der Truppe: dt, Modus, nah, alle, E nah, D nah; 300 Ticks spaeter: nah, E, D
  int bilE, bilD, bilArmee, bilKontakt, bilErste, bilNach, bilGruppe;   // Angriffsbilanz: erwartete Staerken, Ankunft der Truppe
  int feindId[192], feindWert[192], anzFeindId;   // feindId: unitID | (Typ << 16) -- IDs werden schnell neu vergeben
  int eigenId[192], eigenWert[192], anzEigenId;
  int zweiteSchmelzeTick, lynxFreiSeit;
  int siegTick, niederlageTick, verlustGebaeude, verlustEinheiten, feindHatteBasis;   // neue Wertung
  int zerstoertWert, verlorenWert;    // Erzwert zerstoerter Feind- bzw. verlorener eigener Einheiten
  int bewertet;                       // Wertung beim Lauf-Ende (WERTUNG_TICK) eingefroren
  // Labor-Wunsch aus dem Forschungsbedarf: Labortyp, den ein bestes Forschungsziel braucht, und wie dringend
  int laborWunsch, laborWert, laborFuer, naechsterLaborCheck;
  int laborNutzen[4];
  // Selbstpruefung: wann zuletzt ein Kampffahrzeug entstand / ein Bau aufgezeichnet wurde, seit wann ein Bedarf besteht
  int letztesKampfFz, letzterBau, kampfBedarfSeit, bauDrangSeit, naechstePruefung;
  int kampfLaeuft;                    // 1 waehrend kampfFuehrung -- steht es beim naechsten Aufruf noch, ist sie abgestuerzt
  // Kampfgruppen: Fabrikgruppe verstaerkt Wache und Sammelgruppe, die volle Sammelgruppe wird zur Angriffsgruppe
  FightGroup sammel, angriff;
  FightGroup eingreif; int reserveAn, reserveZiel, reserveOrt, reserveSeit;   // Eingreifreserve fuer die Bereiche
  FightGroup angriff2; int angriff2An;   // zweite Angriffsgruppe: eine Gruppe fasst nur 32
  BuildingGroup fabrik;
  int sammelAn, angriffAn, fabrikAn, fabrikVf;
  int fabrikDrin[MAX_VF];      // Fahrzeugfabriken in der Fabrikgruppe
  int zielFz, zielWaffe, wacheZiel, sammelZiel, gruppenBedarf, kampfFzAnzahl;
  // Angriffsarten: 0 Feindbasis, 1 Zielangriff (ein Gebaeude mit schwacher oertlicher Abwehr), 2 Zermuerben
  int angriffArt, angriffWunsch, angriffZerstoertStart, angriffVerlorenStart, letzterAngriffStart;
  // Angriffsbilanz (nur Messung, Spur art "angriff"): Einsatz und Zusammenstellung beider Seiten beim Start
  int bilStart, bilEmp, bilEinsatzErz, bilEinsatzSelten, bilFeindWT, bilN, bilFN;
  short bilTyp[12], bilWaffe[12], bilAnz[12], bilFTyp[12], bilFWaffe[12], bilFAnz[12];
  double angriffQuote[3];             // gelernt: zerstoert / verloren je Angriffsart (gleitend)
  int zermuerbAbstand;                // gelernt: Ticks zwischen zwei Zermuerbungsangriffen                 // je Labortyp (1 Basic, 2 Standard, 3 Advanced): wie dringend die Forschung ihn braucht
  int angriffZiel;                    // unitID des Feindgebaeudes, das die Angriffstruppe gerade angreift
  int feindPotenzial;                 // was der Feind in der Reaktionszeit aufstellen koennte (Armee + Fabriken)
  int erzUeberschuss;           // Bilanz ohne Angriffstruppe: Erz reicht im Horizont (1) oder nicht (0)          // Bitmaske der 8 Himmelsrichtungen, aus denen Feinde an die Kolonie kamen
  FightGroup aussenGruppe[6];
};

// ---------------------------------------------------------------------------------------------------
// Kleinkram / helpers

struct Groesse { int w, h; };

inline Groesse grundflaeche(int typ) {
  switch (typ) {
    case mapCommandCenter:       return { 3, 2 };
    case mapStructureFactory:    return { 4, 3 };
    case mapVehicleFactory:      return { 4, 3 };
    case mapTokamak:             return { 2, 2 };
    case mapMHDGenerator:        return { 2, 2 };
    case mapSolarPowerArray:     return { 3, 2 };
    case mapCommonOreSmelter:    return { 4, 3 };
    case mapRareOreSmelter:      return { 4, 3 };
    case mapCommonOreMine:       return { 2, 1 };
    case mapRareOreMine:         return { 2, 1 };
    case mapCommonStorage:       return { 1, 2 };
    case mapRareStorage:         return { 1, 2 };
    case mapAgridome:            return { 3, 2 };
    case mapResidence:           return { 2, 2 };
    case mapReinforcedResidence: return { 3, 2 };
    case mapAdvancedResidence:   return { 3, 3 };
    case mapBasicLab:            return { 2, 2 };
    case mapStandardLab:         return { 3, 2 };
    case mapAdvancedLab:         return { 3, 3 };
    case mapMedicalCenter:       return { 2, 2 };
    case mapNursery:             return { 2, 2 };
    case mapUniversity:          return { 2, 2 };
    case mapRecreationFacility:  return { 2, 2 };
    case mapForum:               return { 2, 2 };
    case mapDIRT:                return { 3, 2 };
    case mapGarage:              return { 3, 2 };
    case mapConsumerFactory:     return { 3, 3 };
    case mapGuardPost:           return { 1, 1 };
    case mapLightTower:          return { 1, 1 };
    case mapGORF:                return { 3, 2 };
    case mapRobotCommand:        return { 2, 2 };
    case mapObservatory:         return { 2, 2 };
    case mapMeteorDefense:       return { 2, 2 };
    case mapTradeCenter:         return { 2, 2 };
    case mapSpaceport:           return { 5, 4 };
    default:                     return { 2, 2 };
  }
}

inline const char* name(int typ) {
  switch (typ) {
    case mapCommandCenter: return "CommandCenter";        case mapStructureFactory: return "StructureFactory";
    case mapVehicleFactory: return "VehicleFactory";      case mapTokamak: return "Tokamak";
    case mapMHDGenerator: return "MHDGenerator";          case mapSolarPowerArray: return "SolarPowerArray";
    case mapCommonOreSmelter: return "CommonOreSmelter";  case mapRareOreSmelter: return "RareOreSmelter";
    case mapCommonOreMine: return "CommonOreMine";        case mapRareOreMine: return "RareOreMine";
    case mapCommonStorage: return "CommonStorage";        case mapRareStorage: return "RareStorage";
    case mapAgridome: return "Agridome";                  case mapResidence: return "Residence";
    case mapReinforcedResidence: return "ReinforcedResidence"; case mapAdvancedResidence: return "AdvancedResidence";
    case mapBasicLab: return "BasicLab";                  case mapStandardLab: return "StandardLab";
    case mapAdvancedLab: return "AdvancedLab";            case mapMedicalCenter: return "MedicalCenter";
    case mapNursery: return "Nursery";                    case mapUniversity: return "University";
    case mapRecreationFacility: return "RecreationFacility"; case mapForum: return "Forum";
    case mapDIRT: return "DIRT";                          case mapGarage: return "Garage";
    case mapConsumerFactory: return "ConsumerFactory";    case mapGuardPost: return "GuardPost";
    case mapLightTower: return "LightTower";              case mapGORF: return "GORF";
    case mapRobotCommand: return "RobotCommand";          case mapObservatory: return "Observatory";
    case mapMeteorDefense: return "MeteorDefense";        case mapTradeCenter: return "TradeCenter";
    case mapConVec: return "ConVec";                      case mapCargoTruck: return "CargoTruck";
    case mapRoboMiner: return "RoboMiner";                case mapRoboSurveyor: return "RoboSurveyor";
    case mapRoboDozer: return "RoboDozer";                case mapEarthworker: return "Earthworker";
    case mapScout: return "Scout";                        case mapRepairVehicle: return "RepairVehicle";
    case mapEvacuationTransport: return "EvacuationTransport";
    case mapLynx: return "Lynx";                          case mapPanther: return "Panther";
    case mapTiger: return "Tiger";                        case mapMiningBeacon: return "MiningBeacon";
    case mapSpaceport: return "Spaceport";                case mapSULV: return "SULV";
    case mapEDWARDSatellite: return "EDWARDSatellite";    case mapArachnidFactory: return "ArachnidFactory";
    case mapSpider: return "Spider";                    case mapScorpion: return "Scorpion";
    case mapEMPMissile: return "EMPMissile";
    case mapLaser: return "Laser";  case mapMicrowave: return "Microwave";  case mapRailGun: return "RailGun";
    case mapEMP: return "EMP";      case mapRPG: return "RPG";              case mapAcidCloud: return "AcidCloud";
    case mapESG: return "ESG";      case mapThorsHammer: return "ThorsHammer"; case mapStickyfoam: return "Stickyfoam";
    case mapStarflare: return "Starflare"; case mapSupernova: return "Supernova"; case mapEnergyCannon: return "EnergyCannon";
    case mapNone: return "-";
    default: return "?";
  }
}

// Zelltypen (siehe mapview/celltypes.py)
inline bool zelleBebaubar(int c)  { return c == 0 || c == 2 || c == 3 || c == 4 || c == 5 || c == 7 || c == 21; }
inline bool zelleRoehre(int c)    { return c >= 26 && c <= 31; }
inline bool zelleBefahrbar(int c) { return zelleBebaubar(c) || c == 22 || zelleRoehre(c); }

// Wegmarke fuer das Crash-Log: Spieler + Teilschritt (op2_crash.hpp druckt sie bei einem Absturz mit)
inline int eigeneKostenT(const Zustand& z, const tb::Tech& t) { return Player[z.spieler].IsEden() ? t.eden : t.ply; }

inline void hinweis(const Zustand& z, const char* wo) {
  static char puffer[80];
  std::snprintf(puffer, sizeof(puffer), "P%d %s", z.spieler, wo);
  op2::crash::g_hinweis = puffer;
}

inline LOCATION engine(const Zustand& z, int x, int y) { return LOCATION(x + z.ux, y + z.uy); }
inline int edX(const Zustand& z, const LOCATION& l) { return l.x - z.ux; }
inline int edY(const Zustand& z, const LOCATION& l) { return l.y - z.uy; }
inline bool aufKarte(const Zustand& z, int x, int y, int rand = 2) {
  return x >= rand && y >= rand && x < z.breite - rand && y < z.hoehe - rand;
}
inline int zelle(const Zustand& z, int x, int y) { return GameMap::GetCellType(engine(z, x, y)); }
inline int abstand2(int ax, int ay, int bx, int by) { return (ax - bx) * (ax - bx) + (ay - by) * (ay - by); }

// Forschungsstand / technology
// HasTechnology mit einer Nummer, die der geladene Techbaum nicht kennt, liest im Spiel hinter die Tabelle und stuerzt
// ab (0x472D78: techID -> Index -1 -> Zugriff; gesammelt in abstuerze.log, "ki::forschung"). Darum erst pruefen.
// HasTechnology with an unknown tech ID crashes the game; check the ID exists first.
inline int techIndex(int techID);
inline bool hatTech(const Zustand& z, int techID) { return techID > 0 && techIndex(techID) >= 0 && Player[z.spieler].HasTechnology(techID) != 0; }
// Technik je Gebaeude, Spalte "Research Topic" aus sheets/building.txt. Fest hinterlegt, weil
// UnitInfo::GetResearchTopic im Spiel nichts Brauchbares liefert (Lauf 6) und die BuildingGroup der
// Engine die Technik NICHT prueft -- sie baute ein Advanced Lab ohne Forschung.
// Tech per building from sheets/building.txt; the engine's BuildingGroup does not check it.
inline int techFuer(int typ) {
  switch (typ) {
    case mapCommandCenter: return 2103;      case mapTokamak: return 2112;
    case mapStructureFactory: return 2104;   case mapCommonOreSmelter: return 2110;
    case mapCommonStorage: return 2111;      case mapAgridome: return 2101;
    case mapResidence: return 2109;          case mapBasicLab: return 2106;
    case mapLightTower: return 2107;         case mapCommonOreMine: return 2108;
    case mapStandardLab: return 3101;        case mapAdvancedLab: return 5104;
    case mapVehicleFactory: return 3502;     case mapRobotCommand: return 3501;
    case mapUniversity: return 3305;         case mapNursery: return 3304;
    case mapMedicalCenter: return 3303;      case mapRecreationFacility: return 3306;
    case mapDIRT: return 3301;               case mapGORF: return 3302;
    case mapAdvancedResidence: return 8104;  case mapReinforcedResidence: return 8105;
    case mapRareOreSmelter: return 5501;     case mapRareStorage: return 5502;
    case mapRareOreMine: return 5110;        case mapSpaceport: return 5405;
    case mapGarage: return 5102;             case mapConsumerFactory: return 5101;
    case mapForum: return 5108;              case mapGeothermalPlant: return 5115;
    case mapMHDGenerator: return 5107;       case mapGuardPost: return 3801;
    default: return 0;
  }
}

// Kann die Strukturfabrik den Bausatz herstellen (Technik erforscht)? Im Multiplayer-Techbaum gibt es
// z.B. kein Basic Lab (02106 "NOT AVAILABLE") -- dann gar nicht erst bestellen.
// Can the factory make this kit (technology researched)?
inline bool kitMoeglich(const Zustand& z, int typ) {
  const int topic = techFuer(typ);
  return topic <= 0 || hatTech(z, topic);
}

// Techs fuer das Raumfahrtziel / techs for the space goal
constexpr int TECH_RAUMFAHRT = 5405;   // Space Program: Spaceport, SULV, EDWARD
constexpr int TECH_SELTENERZ = 5110;   // Rare Ore Processing: Seltenerz-Mine, -Schmelze, -Lager

// ---------------------------------------------------------------------------------------------------
// Spur / trace  (JSON-Zeilen in <Mission>-KI.jsonl)

struct Puffer {
  char b[12288];
  int n = 0;
  void roh(const char* s) { while (*s && n < (int)sizeof(b) - 2) b[n++] = *s++; }
  void f(const char* fmt, ...) {
    if (n >= (int)sizeof(b) - 2) return;
    va_list ap; va_start(ap, fmt);
    int k = std::vsnprintf(b + n, sizeof(b) - n - 1, fmt, ap);
    va_end(ap);
    if (k > 0) n += (k < (int)sizeof(b) - n - 1) ? k : (int)sizeof(b) - n - 2;
  }
  // Text als JSON-String (Anfuehrungszeichen und Backslash maskiert)
  void str(const char* s) {
    roh("\"");
    for (; *s && n < (int)sizeof(b) - 4; ++s) {
      if (*s == '"' || *s == '\\') b[n++] = '\\';
      b[n++] = *s;
    }
    roh("\"");
  }
  void schreiben() { op2::log::kiRaw(b, n); n = 0; }
};

// Eine Entscheidung mit Begruendung. `was`: bau, kit, fahrzeug, forschung, mine, roehre, warte, fehler, info
inline void melde(const Zustand& z, const char* was, int typ, int x, int y, const char* fmt, ...) {
  char warum[400];
  va_list ap; va_start(ap, fmt);
  std::vsnprintf(warum, sizeof(warum), fmt, ap);
  va_end(ap);
  Puffer p;
  p.f("{\"t\":%d,\"art\":\"entscheid\",\"p\":%d,\"was\":", TethysGame::Tick(), z.spieler);
  p.str(was);
  p.f(",\"typ\":"); p.str(name(typ));
  p.f(",\"x\":%d,\"y\":%d,\"warum\":", x, y);
  p.str(warum);
  p.roh("}");
  p.schreiben();
}

// Wiederkehrende Meldung hoechstens alle `abstand` Ticks (z.B. "warte auf Erz")
inline bool selten(Zustand& z, int kanal, int abstand) {
  const int t = TethysGame::Tick();
  if (kanal < 0 || kanal >= 32) return true;
  if (z.meldungenSeit[kanal] != 0 && t - z.meldungenSeit[kanal] < abstand) return false;
  z.meldungenSeit[kanal] = t;
  return true;
}

// Alle Gebaeudetypen, die die KI zaehlt (PlayerBuildingEnum braucht einen Typ).
// All building types the AI counts (PlayerBuildingEnum needs a type).
static const int GEBAEUDE[] = {
  mapCommandCenter, mapStructureFactory, mapVehicleFactory, mapTokamak, mapMHDGenerator, mapSolarPowerArray,
  mapCommonOreSmelter, mapRareOreSmelter, mapCommonOreMine, mapRareOreMine, mapCommonStorage, mapRareStorage,
  mapAgridome, mapResidence, mapReinforcedResidence, mapAdvancedResidence, mapBasicLab, mapStandardLab,
  mapAdvancedLab, mapMedicalCenter, mapNursery, mapUniversity, mapRecreationFacility, mapForum, mapDIRT,
  mapGarage, mapConsumerFactory, mapGuardPost, mapLightTower, mapGORF, mapRobotCommand, mapObservatory,
  mapMeteorDefense, mapTradeCenter, mapArachnidFactory, mapSpaceport, mapGeothermalPlant, mapMagmaWell,
};

// ---------------------------------------------------------------------------------------------------
// Lagebild / situation  (wird jeden Schritt neu erhoben, nicht gespeichert)

// Anspruch auf knappe Mittel (Reservierung; Spieler 2026-11-09: so wenig feste Regeln wie moeglich, wie RTS-Bots planen):
// ein Vorhaben meldet, was es braucht und wie wichtig es ist (Rang wie in rang()). Wer weniger wichtig ist, nimmt nur, was
// danach uebrig bleibt. Ersetzt die Sonderregeln fabrikVorrang und uniReserve.
// A claim on scarce resources: lower-ranked consumers only take what remains after more important claims.
enum Mittel { M_ARB = 0, M_WISS = 1, M_ERZ = 2, M_SELTEN = 3 };
struct Anspruch { int typ, rang, menge[4]; bool vorhaben; };
constexpr int MAX_ANSPR = 40;
constexpr int MAX_VORHABEN = 4;
// Vorhaben aus der letzten Bauwahl, die an Arbeitern oder Erz scheiterten: sie halten ihren Bedarf fest, bis die naechste
// Bauwahl neu entscheidet. Plans from the last build choice that lacked workers or ore keep their claim until the next one.
inline Anspruch g_vorhaben[8][MAX_VORHABEN];
inline int g_nVorhaben[8], g_vorhabenTick[8];

struct Lage {
  int geb[128];          // Anzahl fertiger Gebaeude je map_id (map_id < 128)
  int fz[128];           // Anzahl Fahrzeuge je map_id
  int kitsImSpiel[128];  // Bausaetze in Fabrikbuchten und auf ConVecs
  int imBau[128];        // Gebaeude im Bau (Aktion moObjWait) -- wirken erst, wenn sie fertig sind
  UnitEx cc, sf, vf;     // erste Kommandozentrale / Strukturfabrik / Fahrzeugfabrik
  UnitEx vfs[MAX_VF];    // alle Fahrzeugfabriken (vfs[0] == vf)
  int vfAnzahl, vfLaeuft;
  int erz, erzMax, nahrung, nahrungNetto, stromFrei, menschen, wohnraum, wiss;
  int selten, seltenMax;  // Seltenerz / rare ore
  int arbFrei, wissFrei;  // freie Arbeiter / Wissenschaftler
  int arbStilllegbar[11]; // Arbeiter in laufenden Gebaeuden mit Rang < 10*i (koennten stillgelegt werden)
  int vorErz, vorSelten;  // vom Ziel vorgemerkt (vormerkungen())
  const char* vorFuer;
  int arbFehlt, wissFehlt;  // Arbeiter / Wissenschaftler, die fertigen (nicht stillgelegten) Gebaeuden fehlen
  int unbesetzt[128];       // fertige Gebaeude je Typ ohne Arbeiter -- davon nicht noch eins bauen
  int medKap, freizeitKap, forumKap;   // Gesamtkapazitaeten (nur laufende Gebaeude, HFL)
  int dirtAbdeckung, gorfLaeuft, angeschlossen;   // % der angeschlossenen Gebaeude, die laufende DIRTs abdecken (10 je DIRT); GORF in Betrieb
  int kitsFabrik;           // fertige Bausaetze in der Strukturfabrik, die auf einen ConVec warten
  bool seltenSpart;         // die erwartete Seltenerz-Schmelze sammelt Erz: neue Kampffahrzeuge warten (ohne akute Bedrohung)
  Anspruch anspr[MAX_ANSPR]; int nAnspr;   // Ansprueche (arbeitErheben)
};

// Summe der Ansprueche auf ein Mittel mit mindestens diesem Rang. Das eigene Vorhaben (gleicher Typ) zaehlt nicht mit.
// Sum of claims at or above a rank; the caller's own plan is left out.
inline int beansprucht(const Lage& l, int m, int abRang, int ohneVorhaben = -1) {
  int s = 0;
  for (int i = 0; i < l.nAnspr; ++i) {
    const Anspruch& a = l.anspr[i];
    if (a.rang < abRang || (a.vorhaben && a.typ == ohneVorhaben)) continue;
    s += a.menge[m];
  }
  return s;
}
// Hoechster Rang, der ein Mittel beansprucht (-1 = keiner). Highest rank claiming a resource.
inline int hoechsterAnspruch(const Lage& l, int m) {
  int r = -1;
  for (int i = 0; i < l.nAnspr; ++i) if (l.anspr[i].menge[m] > 0 && l.anspr[i].rang > r) r = l.anspr[i].rang;
  return r;
}
inline void anspruchDazu(Lage& l, int typ, int rang, int arb, int wiss, int erz, int selten, bool vorhaben = false) {
  if (l.nAnspr >= MAX_ANSPR || (arb <= 0 && wiss <= 0 && erz <= 0 && selten <= 0)) return;
  Anspruch& a = l.anspr[l.nAnspr++];
  a.typ = typ; a.rang = rang; a.vorhaben = vorhaben;
  a.menge[M_ARB] = arb > 0 ? arb : 0; a.menge[M_WISS] = wiss > 0 ? wiss : 0; a.menge[M_ERZ] = erz > 0 ? erz : 0; a.menge[M_SELTEN] = selten > 0 ? selten : 0;
}

inline void erhebe(const Zustand& z, Lage& l) {
  std::memset(l.geb, 0, sizeof(l.geb));
  std::memset(l.fz, 0, sizeof(l.fz));
  std::memset(l.kitsImSpiel, 0, sizeof(l.kitsImSpiel));
  std::memset(l.imBau, 0, sizeof(l.imBau));
  l.cc = UnitEx(); l.sf = UnitEx(); l.vf = UnitEx(); l.vfAnzahl = 0; l.vfLaeuft = 0; l.kitsFabrik = 0; l.nAnspr = 0; l.seltenSpart = false;
  const int p = z.spieler;
  for (int t : GEBAEUDE) {
    PlayerBuildingEnum e(p, (map_id)t);
    UnitEx u;
    while (e.GetNext(u)) {
      if (t >= 0 && t < 128) l.geb[t]++;
      if (t >= 0 && t < 128 && u.GetCurAction() == moObjWait) l.imBau[t]++;   // im Bau (Lauf 13 gemessen: Aktion 6)
      if (t == mapCommandCenter && !l.cc.unitID) l.cc = u;
      if (t == mapStructureFactory) {
        if (!l.sf.unitID) l.sf = u;
        for (int bay = 0; bay < 6; ++bay) {
          const int k = u.GetFactoryCargo(bay);
          if (k > 0 && k < 128) { l.kitsImSpiel[k]++; l.kitsFabrik++; }
        }
      }
      if (t == mapVehicleFactory && !l.vf.unitID) l.vf = u;
      if (t == mapVehicleFactory && l.vfAnzahl < MAX_VF) l.vfs[l.vfAnzahl++] = u;
      if (t == mapVehicleFactory && u.HasPower() && u.HasWorkers()) l.vfLaeuft++;
    }
  }
  {
    PlayerVehicleEnum e(p);
    UnitEx u;
    while (e.GetNext(u)) {
      const int t = u.GetType();
      if (t >= 0 && t < 128) l.fz[t]++;
      if (t == mapConVec) {
        const int k = u.GetCargo();
        if (k > 0 && k < 128) l.kitsImSpiel[k]++;
      }
    }
  }
  l.erz = Player[p].Ore();
  l.erzMax = ExtPlayer[p].GetMaxOre();
  l.nahrung = Player[p].FoodStored();
  l.nahrungNetto = ExtPlayer[p].GetNetFoodProduction();
  l.stromFrei = ExtPlayer[p].GetAmountPowerAvailable();
  l.menschen = Player[p].Kids() + Player[p].Workers() + Player[p].Scientists();
  l.wohnraum = ExtPlayer[p].GetTotalResidenceCapacity();
  l.wiss = ExtPlayer[p].GetNumAvailableScientists();
  l.selten = Player[p].RareOre();
  l.seltenMax = ExtPlayer[p].GetMaxRareOre();
  l.arbFrei = ExtPlayer[p].GetNumAvailableWorkers();
  l.wissFrei = ExtPlayer[p].GetNumAvailableScientists();
  std::memset(l.arbStilllegbar, 0, sizeof(l.arbStilllegbar));
  l.vorErz = 0; l.vorSelten = 0; l.vorFuer = "-";
  l.arbFehlt = 0; l.wissFehlt = 0;
  std::memset(l.unbesetzt, 0, sizeof(l.unbesetzt));
  l.medKap = ExtPlayer[p].GetTotalMedCenterCapacity();
  l.freizeitKap = ExtPlayer[p].GetTotalRecreationFacilityCapacity();
  l.forumKap = ExtPlayer[p].GetTotalForumCapacity();
  // DIRT-Abdeckung und GORF selbst erheben (keine HFL-Abfrage): angeschlossen = alle Gebaeude ausser Tuermen, Minen, Kraftwerken
  {
    int dirt = 0, angeschlossen = 0; l.gorfLaeuft = 0;
    for (int t : GEBAEUDE) {
      PlayerBuildingEnum e(p, (map_id)t); UnitEx u;
      while (e.GetNext(u)) {
        if (!u.IsLive()) continue;
        const bool laeuft = u.HasPower() && (u.HasWorkers() || UnitInfo((map_id)t).GetWorkersRequired(p) == 0);
        if (t == mapDIRT && laeuft) dirt++;
        if (t == mapGORF && laeuft) l.gorfLaeuft = 1;
        if (t != mapGuardPost && t != mapCommonOreMine && t != mapRareOreMine && t != mapTokamak && t != mapLightTower && t != mapMagmaWell) angeschlossen++;
      }
    }
    l.angeschlossen = angeschlossen;
    l.dirtAbdeckung = angeschlossen > 0 ? (dirt * 10 * 100 / angeschlossen > 100 ? 100 : dirt * 10 * 100 / angeschlossen) : 100;
  }
}

// Gebaeude eines Typs: fertig + geplant/im Bau
inline int geplant(const Zustand& z, int typ) {
  int n = 0;
  for (int i = 0; i < z.anzahlPlaetze; ++i)
    if (z.plaetze[i].typ == typ && (z.plaetze[i].zustand == P_FAHRT || z.plaetze[i].zustand == P_BAUT)) n++;
  return n;
}

// ---------------------------------------------------------------------------------------------------
// Bauplatz / site selection
//
// Jedes Gebaeude bringt zwei Roehrenstuecke mit: eins rechts, eins unten, jeweils in der Mitte der
// Seite und im 1 Kachel breiten Rand (ausser Kraftwerken und Minen). Am Anfang gibt es kein Metall
// fuer weitere Roehren -- also muessen die Gebaeude so stehen, dass diese Stuecke sie verbinden:
// genau 1 Kachel Abstand, und das Stueck des einen trifft die Seite des anderen. Kraftwerke und
// Minen brauchen keinen Anschluss.
//
// Every building comes with two tube stubs (right and bottom, centred in the 1-tile border; not power
// plants or mines). At the start there is no metal for more tubes, so buildings must stand exactly
// one tile apart with a stub of one touching the side of the other.

struct Rechteck { int x0, y0, w, h; bool stutzen; };

inline bool hatStutzen(int typ) {
  switch (typ) {
    case mapTokamak: case mapMHDGenerator: case mapSolarPowerArray: case mapGeothermalPlant:
    case mapCommonOreMine: case mapRareOreMine: case mapMagmaWell:
    case mapGuardPost: case mapLightTower:
      return false;
    default:
      return true;
  }
}

inline Rechteck rechteckUm(int typ, int cx, int cy) {        // aus der Mitte (Location)
  const Groesse g = grundflaeche(typ);
  return { cx - g.w / 2, cy - g.h / 2, g.w, g.h, hatStutzen(typ) };
}

// Alle eigenen Gebaeude und geplanten Plaetze (gesperrte Plaetze als Hindernis ohne Stutzen).
inline int hindernisse(const Zustand& z, Rechteck* out, int max) {
  int n = 0;
  for (int t : GEBAEUDE) {
    PlayerBuildingEnum e(z.spieler, (map_id)t);
    UnitEx u;
    while (e.GetNext(u) && n < max) {
      const LOCATION l = u.Location();
      out[n++] = rechteckUm(t, edX(z, l), edY(z, l));
    }
  }
  for (int i = 0; i < z.anzahlPlaetze && n < max; ++i) {
    const Platz& q = z.plaetze[i];
    if (q.zustand == P_FREI) continue;
    const Groesse g = grundflaeche(q.typ);
    out[n++] = { q.x0, q.y0, g.w, g.h, q.zustand != P_GESPERRT && hatStutzen(q.typ) };
  }
  return n;
}

inline bool ueberlappt(int ax, int ay, int aw, int ah, int bx, int by, int bw, int bh) {
  return ax < bx + bw && bx < ax + aw && ay < by + bh && by < ay + ah;
}

// Grundflaeche bebaubar, Rand befahrbar, keine Bake/kein Wrack, nichts Geplantes im Weg,
// kein Roehrenstutzen unter der Grundflaeche.
inline bool platzFrei(const Zustand& z, int x0, int y0, int w, int h, const Rechteck* hin, int nh,
                      const char** grund) {
  if (!aufKarte(z, x0 - 1, y0 - 1, 2) || !aufKarte(z, x0 + w, y0 + h, 2)) { *grund = "Kartenrand"; return false; }
  // Baken freihalten: Grundflaeche der Mine (2x1, Bake rechts) samt Rand (Karte 3: Tokamak neben der Bake, Mine nie gebaut)
  for (int i = 0; i < z.kbN; ++i)
    if (ueberlappt(x0 - 1, y0 - 1, w + 2, h + 2, z.kbX[i] - 2, z.kbY[i] - 1, 4, 3)) { *grund = "Bake"; return false; }
  for (int i = 0; i < nh; ++i) {
    const Rechteck& r = hin[i];
    // Abstand mindestens 1 Kachel: der Rand des einen darf die Grundflaeche des anderen nicht beruehren
    if (ueberlappt(x0 - 1, y0 - 1, w + 2, h + 2, r.x0, r.y0, r.w, r.h)) { *grund = "Gebaeude/Plan"; return false; }
    if (r.stutzen) {
      const int sx = r.x0 + r.w, sy = r.y0 + r.h / 2;        // rechter Stutzen
      const int ux = r.x0 + r.w / 2, uy = r.y0 + r.h;        // unterer Stutzen
      if (ueberlappt(x0, y0, w, h, sx, sy, 1, 1) || ueberlappt(x0, y0, w, h, ux, uy, 1, 1)) {
        *grund = "Roehrenstutzen"; return false;
      }
    }
  }
  for (int y = y0 - 1; y <= y0 + h; ++y)
    for (int x = x0 - 1; x <= x0 + w; ++x) {
      const int c = zelle(z, x, y);
      const bool innen = x >= x0 && x < x0 + w && y >= y0 && y < y0 + h;
      if (innen ? !zelleBebaubar(c) : !zelleBefahrbar(c)) { *grund = innen ? "Untergrund" : "Rand"; return false; }
    }
  MAP_RECT rr(engine(z, x0 - 1, y0 - 1), engine(z, x0 + w, y0 + h));
  InRectEnumerator e(rr);
  UnitEx u2;
  while (e.GetNext(u2)) {
    if (u2.IsVehicle()) {
      // Fahrende Fahrzeuge sind gleich weg; ein stehendes in der Grundflaeche blockiert den Bau.
      const LOCATION l = u2.Location();
      const int vx = edX(z, l), vy = edY(z, l);
      if (vx >= x0 && vx < x0 + w && vy >= y0 && vy < y0 + h && u2.GetCurAction() == moDone && u2.GetType() != mapConVec) {
        *grund = "Fahrzeug steht im Weg"; return false;
      }
      continue;
    }
    *grund = u2.IsBuilding() ? "Gebaeude" : "Bake/Wrack";
    return false;
  }
  return true;
}

// Neuen Eintrag in z.plaetze anlegen (freigewordene wiederverwenden). -1 = voll.
inline int platzEintrag(Zustand& z, int typ, int x0, int y0) {
  int k = -1;
  for (int i = 0; i < z.anzahlPlaetze; ++i) if (z.plaetze[i].zustand == P_FREI && !z.plaetze[i].convec) { k = i; break; }
  if (k < 0) {
    if (z.anzahlPlaetze >= MAX_PLAETZE) return -1;
    k = z.anzahlPlaetze++;
  }
  Platz& p = z.plaetze[k];
  p.x0 = (short)x0; p.y0 = (short)y0; p.typ = (short)typ; p.zustand = P_FREI;
  p.convec = 0; p.seit = TethysGame::Tick(); p.versuche = 0; p.raster = -1;
  return k;
}

inline void feindRichtung(const Zustand& z, int x, int y, double& ux, double& uy);
// Tokamak und Grosslabor explodieren mit Flaechenschaden (Spieler): moeglichst 3 Felder Luft zu anderen Gebaeuden. Strafe je
// fehlendem Feld; als Wertung, nicht als Verbot, weil Gebaeude mit Anschluss 1 Feld neben dem Nachbarn stehen muessen.
// Tokamak and advanced lab explode: prefer 3 free tiles around them (penalty, not a ban).
inline int explosionsStrafe(const Zustand& z, int typ, int x0, int y0, int w, int h) {
  const bool selbst = typ == mapTokamak || typ == mapAdvancedLab;
  int strafe = 0;
  auto luecke = [&](int bx, int by, int bw, int bh) {
    auto gr = [](int a, int b) { return a > b ? a : b; };
    const int dx = gr(0, gr(bx - (x0 + w), x0 - (bx + bw)));
    const int dy = gr(0, gr(by - (y0 + h), y0 - (by + bh)));
    return gr(dx, dy);
  };
  for (int t : GEBAEUDE) {
    if (t == mapGuardPost) continue;
    const bool expl = t == mapTokamak || t == mapAdvancedLab;
    if (!selbst && !expl) continue;
    PlayerBuildingEnum e(z.spieler, (map_id)t); UnitEx u;
    while (e.GetNext(u)) {
      if (!u.IsLive()) continue;
      const Rechteck r = rechteckUm(t, edX(z, u.Location()), edY(z, u.Location()));
      const int lu = luecke(r.x0, r.y0, r.w, r.h);
      if (lu < 3) strafe += (3 - lu) * 300;
    }
  }
  for (int i = 0; i < z.anzahlPlaetze; ++i) {
    const Platz& q = z.plaetze[i];
    if (q.zustand == P_FREI || q.typ == mapGuardPost) continue;
    if (!selbst && q.typ != mapTokamak && q.typ != mapAdvancedLab) continue;
    const Groesse g = grundflaeche(q.typ);
    const int lu = luecke(q.x0, q.y0, g.w, g.h);
    if (lu < 3) strafe += (3 - lu) * 300;
  }
  return strafe;
}
// Platz fuer `typ` suchen. Liefert Index in z.plaetze oder -1.
inline int platzWaehlen(Zustand& z, int typ) {
  static Rechteck hin[160];
  const int nh = hindernisse(z, hin, 160);
  const Groesse g = grundflaeche(typ);
  const char* grund = "";
  int verworfen = 0;
  int bestX = 0, bestY = 0;
  int besteWertung = 1 << 30;
  bool gefunden = false;
  auto pruefen = [&](int x0, int y0, int wertung) {
    if (wertung >= besteWertung) return;
    if (!platzFrei(z, x0, y0, g.w, g.h, hin, nh, &grund)) { verworfen++; return; }
    wertung += explosionsStrafe(z, typ, x0, y0, g.w, g.h);
    if (wertung >= besteWertung) return;
    besteWertung = wertung; bestX = x0; bestY = y0; gefunden = true;
  };
  const int mx = z.bx, my = z.by;
  // Kraftwerke auf die feindabgewandte Seite (Eden-Karten: Tokamaks standen hinter den Tuermen zum Feind und fielen frueh)
  double fux = 0, fuy = 0;
  const bool hinten = typ == mapTokamak && z.feindGemeldet;
  if (hinten) feindRichtung(z, mx, my, fux, fuy);
  if (!hatStutzen(typ) || typ == mapCommandCenter) {
    // Kommandozentrale: so nah wie moeglich an die Basismitte. Kraftwerke: etwas abseits (Ring um
    // die Mitte), damit sie keine Anschlussplaetze belegen.
    for (int dy = -12; dy <= 12; ++dy)
      for (int dx = -12; dx <= 12; ++dx) {
        const int x0 = mx + dx - g.w / 2, y0 = my + dy - g.h / 2;
        const int d2 = dx * dx + dy * dy;
        int w;
        if (typ == mapCommandCenter) w = d2;
        else { const int d = (int)(d2 > 0 ? d2 : 0); w = (d - 49) * (d - 49) / 8 + d / 4; }  // Ring um ~7
        if (hinten) w += (int)(40 * (dx * fux + dy * fuy));                                 // zum Feind hin teurer
        pruefen(x0, y0, w);
      }
  } else {
    // Gebaeude mit Anschluss: an einen Stutzen eines vorhandenen/geplanten Gebaeudes, oder mit dem
    // eigenen Stutzen an dessen Seite -- immer genau 1 Kachel Abstand.
    for (int i = 0; i < nh; ++i) {
      const Rechteck& a = hin[i];
      if (!a.stutzen) continue;
      const int acx = a.x0 + a.w / 2, acy = a.y0 + a.h / 2;
      auto wert = [&](int x0, int y0, int ausrichtung) {
        const int cx = x0 + g.w / 2, cy = y0 + g.h / 2;
        return abstand2(cx, cy, mx, my) * 4 + ausrichtung;
      };
      // rechts von A (A's rechter Stutzen trifft unsere linke Seite)
      for (int y0 = acy - g.h + 1; y0 <= acy; ++y0) pruefen(a.x0 + a.w + 1, y0, wert(a.x0 + a.w + 1, y0, std::abs(y0 - a.y0)));
      // unter A (A's unterer Stutzen trifft unsere Oberseite)
      for (int x0 = acx - g.w + 1; x0 <= acx; ++x0) pruefen(x0, a.y0 + a.h + 1, wert(x0, a.y0 + a.h + 1, std::abs(x0 - a.x0)));
      // links von A (unser rechter Stutzen trifft A's linke Seite)
      for (int y0 = a.y0 - g.h / 2; y0 <= a.y0 + a.h - 1 - g.h / 2; ++y0) pruefen(a.x0 - 1 - g.w, y0, wert(a.x0 - 1 - g.w, y0, std::abs(y0 - a.y0)));
      // ueber A (unser unterer Stutzen trifft A's Oberseite)
      for (int x0 = a.x0 - g.w / 2; x0 <= a.x0 + a.w - 1 - g.w / 2; ++x0) pruefen(x0, a.y0 - 1 - g.h, wert(x0, a.y0 - 1 - g.h, std::abs(x0 - a.x0)));
    }
  }
  if (!gefunden) {
    if (selten(z, 6, 1000))
      melde(z, "fehler", typ, z.bx, z.by, "kein Bauplatz mit Anschluss (%d verworfen, zuletzt: %s)", verworfen, grund);
    return -1;
  }
  return platzEintrag(z, typ, bestX, bestY);
}

// ---------------------------------------------------------------------------------------------------
// Was als Naechstes gebaut wird / what to build next
//
// Drei Stufen (Gespraech mit dem Spieler, November 2026):
//   1. Notfaelle -- keine Prozente: Grundgebaeude fehlen, Strom aus, Nahrung reicht nur noch kurz.
//   2. Bau-Kuchen -- drei grobe Bereiche, jeder bekommt aus der Lage einen Bedarf, zusammen 100 %:
//        Kolonie (Wohnraum + Moral + Wachstum; wiegt schwer, solange die Kolonie klein ist),
//        Ziel (Mission), Wirtschaft (Fahrzeugfabrik, Lager).
//      Der groesste Bereich mit einem baubaren Kandidaten gewinnt. Kein festes Phasenmodell: das
//      Gewicht der Kolonie faellt von selbst, wenn die Bevoelkerung waechst (ab ~100 kaum noch).
//   3. Lueckenfueller -- Strom (unter ~150 frei) und Nahrung (Defizit, aber noch Vorrat): nur in einer
//      Bauluecke und nur mit Erz, das das Ziel nicht vorgemerkt hat.
// Arbeiter: gebaut wird nur, wenn das Gebaeude betrieben werden kann -- mit freien Arbeitern oder mit
// denen eines unwichtigeren Gebaeudes, das die Arbeiter-Rangliste dafuer stilllegt (siehe arbeit()).
//
// Three stages: emergencies, a coarse building pie (colony / goal / economy, 100 % in total, colony
// weight shrinks as population grows), then gap fillers (power, food) using only unreserved ore.

enum Bereich { B_KOLONIE = 0, B_ZIEL = 1, B_WIRTSCHAFT = 2, B_VERTEIDIGUNG = 3, B_ANZAHL = 4 };
static const char* const BEREICH[B_ANZAHL] = { "Kolonie", "Ziel", "Wirtschaft", "Verteidigung" };

// ---------------------------------------------------------------------------------------------------
// Kampf / combat: Lagebild der Gegner (Kartenscan), Bedrohung, Soll-Staerke
//
// OP2 hat keinen Nebel -- ein Mensch sieht alle Einheiten auf der Karte. Die KI scannt darum alle
// SCAN_TAKT Ticks jeden Spieler, mit dem sie nicht verbuendet ist: Kampfwert seiner Armee, seine Gebaeude
// (wo liegt die Basis, hat er eine Fahrzeugfabrik), was sich der eigenen Basis naehert. Daraus folgen
// die Bedrohung (0..100, steuert Bau- und Forschungskuchen) und die Soll-Staerke: so viel eigener
// Kampfwert (Wachtuerme, Lynx) soll stehen. Ohne bekannten Feind gibt es keine Verteidigung.
//
// No fog of war in OP2, so the AI scans all non-allied players every SCAN_TAKT ticks: army value,
// buildings (base location, vehicle factory), units approaching. Result: threat (0..100, drives the
// building and research pies) and target strength (own guard posts + Lynx). No known enemy, no defence.

constexpr int SCAN_TAKT  = 200;
constexpr int WERTUNG_TICK = 200000;   // Spieler 2026-11-05: lange Spiele, damit Raumhafen/EMP-Raketen zaehlen (vorher 80000)
constexpr int WERT_WT    = 4;     // Kampfwert eines Laser-Wachturms nach SDK (fuer Geplantes)
constexpr int WERT_LYNX  = 5;     // Laser-Lynx nach SDK
constexpr int MAX_WT     = 8;
constexpr int AUSSEN_R   = 12;    // Minen weiter weg von der Basismitte sind Aussenposten
constexpr int ABFANG_R   = 90;    // so weit fahren Lynx einer anrueckenden Welle entgegen

// Kampfwert wie im SDK (Outpost2DLL Player.h, Grundlage von Player::GetPlayerStrength, dort / 8):
// Spider/Scorpion 4; Lynx 5/6/7, Panther 7/8/9, Tiger 8/9/10, Wachturm 4/5/7 -- jeweils Laser/Microwave,
// andere Waffe, Thor's Hammer. Alles andere 0. Schaden zaehlt nicht (wie im SDK).
// Combat value as in the SDK table behind Player::GetPlayerStrength.
// --- Kampfmodell aus der Arena (2026-11-06, ~70000 Kaempfe, Reihen 1-3; ki-netz/arena_formel.py) ---
// Gegenwert g eines Fahrzeugs = Waffenfaktor x Fahrgestellfaktor x Wurzel(HP x Schaden je Tick), Schaden je Treffer =
// 0,9 x Durchschlag + 0,33 x Erschuetterung x Panzerwert/100. Die Staerke einer Gruppe ist die Summe ihrer g (Mischung
// bringt messbar nichts extra); wer die groessere Summe hat, gewinnt (85 % richtig auf ungesehenen gemischten Kaempfen).
// Skala: Lynx mit Laser = 5. Die Faktoren enthalten, was die Formel nicht sieht (Flaechenschaden bei Saeure und ESG).
// Wachtuerme waren nicht in der Arena: gleiche Formel, nicht gemessen. EMP/Klebeschaum: Wert wie Laser x Faktor (aus
// Reihe 3 geschaetzt), gilt fuer Beimischung, nicht fuer reine Gruppen. Arena model: g from live unit values.
static int g_kraftSpieler = 1;
inline double waffenFaktor(int w) {
  switch (w) {
    case mapAcidCloud: return 3.59; case mapESG: return 2.38; case mapRailGun: return 0.85; case mapThorsHammer: return 0.83;
    case mapRPG: return 0.80; case mapLaser: return 0.67; case mapMicrowave: return 0.61; case mapEnergyCannon: return 0.69;
    default: return 0.67;
  }
}
inline double gestellFaktor(int f) { return f == mapLynx ? 1.07 : f == mapPanther ? 0.95 : f == mapScorpion ? 0.69 : 1.0; }
inline double gegenwert(int spieler, int traeger, int waffe, double hpAnteil = 1.0) {
  double zusatz = 1.0;
  if (waffe == mapEMP) { waffe = mapLaser; zusatz = 1.4; }
  else if (waffe == mapStickyfoam) { waffe = mapLaser; zusatz = 1.0; }
  else if (waffe == mapNone || waffe <= 0 || waffe >= 128) waffe = traeger == mapScorpion ? mapEnergyCannon : mapLaser;
  UnitInfo c((map_id)traeger), w((map_id)waffe);
  const int r = w.GetReloadTime(spieler);
  const double dps = (0.9 * w.GetPenetrationDamage(spieler) + 0.33 * w.GetConcussionDamage(spieler) * 1.1) / (r > 0 ? r : 1);
  const double hp = c.GetHitPoints(spieler) * (hpAnteil > 0.05 ? hpAnteil : 0.05);
  return 0.426 * zusatz * waffenFaktor(waffe) * gestellFaktor(traeger) * std::sqrt(hp * dps);
}
inline int kampfWert(UnitEx u) {
  const int t = u.GetType(), w = (int)u.GetWeapon();
  if (t == mapSpider) return 1;                                  // repariert nur
  if (t != mapLynx && t != mapPanther && t != mapTiger && t != mapScorpion && t != mapGuardPost) return 0;
  UnitInfo c((map_id)t); const int hp = c.GetHitPoints(u.OwnerID());
  const double anteil = hp > 0 ? 1.0 - (double)u.GetDamage() / hp : 1.0;
  const int g = (int)(gegenwert(u.OwnerID(), t, w, anteil) + 0.5);
  return g > 0 ? g : 1;
}
// Kampffahrzeug fuer die eigenen Gruppen (der Spider zaehlt im SDK, repariert aber nur)
inline bool kaempfer(UnitEx u) { return u.IsVehicle() && u.GetType() != mapSpider && kampfWert(u) > 0; }

// ---------------------------------------------------------------------------------------------------
// EMP-Rakete (nur Plymouth; Spieler: "die Truppe versammelt sich vor der feindlichen Basis, die Rakete schlaegt ein,
// danach greift sie an -- in dem Zeitpunkt, wo die feindlichen Fahrzeuge betaeubt sind").
// Gemessen (KI_TestEMP): Einschlag 200-300 Ticks nach dem Start (64 Kacheln), Radius ~6-7 Kacheln, laehmt Fahrzeuge
// und Wachtuerme (keine anderen Gebaeude) ~700-750 Ticks. Technik 12201 (braucht 5405, 5508), Rakete 3000 Erz +
// 1500 Seltenerz (Techbaum), gebaut im Raumhafen.
// EMP missile (Plymouth only): measured flight, radius and stun duration.
constexpr int EMP_TECH = 12201, EMP_RADIUS = 6, EMP_FLUG = 300, EMP_AUFMARSCH = 4;
inline bool empMoeglich(const Zustand& z) { return !Player[z.spieler].IsEden() && hatTech(z, EMP_TECH); }
// Bestes Ziel: die Stelle, an der im EMP-Radius die meiste feindliche Abwehr steht (Kampfwert von Fahrzeugen und
// Wachtuermen). Liefert den Wert, zx/zy in Editor-Kacheln. Best target: most enemy defence within the radius.
inline int empZiel(const Zustand& z, int& zx, int& zy) {
  int px[128], py[128], w[128], n = 0;
  for (int p = 0; p < TethysGame::NoPlayers(); ++p) {
    if (p == z.spieler || ExtPlayer[z.spieler].IsAlliedTo(p)) continue;
    { PlayerVehicleEnum e(p); UnitEx u;
      while (e.GetNext(u) && n < 128) if (u.IsLive() && kaempfer(u)) { const LOCATION l = u.Location(); px[n] = edX(z, l); py[n] = edY(z, l); w[n++] = kampfWert(u); } }
    { PlayerBuildingEnum e(p, mapGuardPost); UnitEx u;
      while (e.GetNext(u) && n < 128) if (u.IsLive()) { const LOCATION l = u.Location(); px[n] = edX(z, l); py[n] = edY(z, l); w[n++] = kampfWert(u); } }
  }
  int beste = 0; zx = zy = 0;
  for (int i = 0; i < n; ++i) {
    int s = 0;
    for (int j = 0; j < n; ++j) if (abstand2(px[i], py[i], px[j], py[j]) <= EMP_RADIUS * EMP_RADIUS) s += w[j];
    if (s > beste) { beste = s; zx = px[i]; zy = py[i]; }
  }
  return beste;
}
// Kosten einer Rakete in Kampfwert: so viele beste Kampffahrzeuge haette man fuer dasselbe Erz (Seltenerz doppelt).
// Missile cost expressed as combat value of the vehicles the same ore would buy.
inline int fahrgestellWahl(const Zustand& z, int seltenDa);
inline int waffeWahl(const Zustand& z, int traeger, int seltenDa);
inline int sdkWertT(int traeger, int waffe);
inline int empKostenWert(const Zustand& z) {
  UnitInfo r(mapEMPMissile);
  int fz = fahrgestellWahl(z, 1 << 20); if (!fz) fz = mapLynx;
  const int wf = waffeWahl(z, fz, 1 << 20);
  UnitInfo fi((map_id)fz), wi((map_id)wf);
  const int fzKosten = fi.GetOreCost(z.spieler) + wi.GetOreCost(z.spieler) + 2 * (fi.GetRareOreCost(z.spieler) + wi.GetRareOreCost(z.spieler));
  const int rKosten = (r.GetOreCost(z.spieler) > 0 ? r.GetOreCost(z.spieler) : 3000) + 2 * (r.GetRareOreCost(z.spieler) > 0 ? r.GetRareOreCost(z.spieler) : 1500);
  return fzKosten > 0 ? rKosten * sdkWertT(fz, wf) / fzKosten : 100;
}
// Lohnt eine Rakete gegen die jetzige Abwehr? Is a missile worth it against the current defence?
inline bool empLohnt(const Zustand& z, int* wert = nullptr) {
  int zx, zy; const int v = empZiel(z, zx, zy);
  if (wert) *wert = v;
  return v > 0 && v >= empKostenWert(z) * PAR(z).empSchwelle;
}

inline bool istKampfFz(int typ) { return typ == mapLynx || typ == mapPanther || typ == mapTiger; }

inline int reparaturTyp(const Zustand& z) { return Player[z.spieler].IsEden() ? mapRepairVehicle : mapSpider; }

struct Kraftwerte { int hp, panzer, pen, kon, reload; };
inline double kraft(const Kraftwerte& v, int waffe) {
  if (v.reload <= 0 || v.hp <= 0) return 0;
  double dps = (v.pen + 0.5 * v.kon) / (double)v.reload;
  if (waffe == mapEMP) dps = 0.9;                                  // betaeubt nur kurz: wie eine Laser-Klasse-Waffe
  return std::sqrt(dps * v.hp * (1 + 0.15 * v.panzer));
}

inline int sdkWertT(int traeger, int waffe) {
  if (traeger != mapLynx && traeger != mapPanther && traeger != mapTiger && traeger != mapScorpion && traeger != mapGuardPost) return traeger == mapSpider ? 1 : 0;
  const int g = (int)(gegenwert(g_kraftSpieler, traeger, waffe) + 0.5);
  return g > 0 ? g : 1;
}

// Kampfkraft aus den aktuellen Werten (UnitInfo je Spieler, mit allem Erforschten)
inline double kraftJetzt(int spieler, int traeger, int waffe) {
  UnitInfo c((map_id)traeger), w((map_id)waffe);
  const Kraftwerte v = { c.GetHitPoints(spieler), c.GetArmor(spieler), w.GetPenetrationDamage(spieler), w.GetConcussionDamage(spieler), w.GetReloadTime(spieler) };
  return kraft(v, waffe);
}

inline bool einheitErforscht(const Zustand& z, int typ) {
  const tb::Einheit* e = tb::einheit(typ);
  return e && e->tech > 0 && hatTech(z, e->tech);
}

// Beste erforschte Waffe fuer einen Traeger (Fahrgestell oder Wachturm): hoechster SDK-Wert, bei Gleichstand
// das Kampfkraft-Modell. Nicht als Hauptwaffe: EMP (betaeubt nur kurz), Starflare/Supernova (zerstoeren
// sich selbst), Stickyfoam (haelt nur fest). Seltenerz muss reichen.
// Best researched weapon for a carrier; EMP, self-destruct and foam are not main weapons.
// Gesamtstaerke, die der Vorrat ergibt: Gegenwert x Anzahl, das knappere Erz begrenzt -- kein Umrechnungsfaktor.
// Strength the stock buys: value x count, limited by whichever ore runs out first.
inline double staerkeAusVorrat(const Zustand& z, int traeger, int waffe, int erzDa, int seltenDa) {
  UnitInfo c((map_id)traeger), w((map_id)waffe);
  const int oe = c.GetOreCost(z.spieler) + w.GetOreCost(z.spieler), se = c.GetRareOreCost(z.spieler) + w.GetRareOreCost(z.spieler);
  double n = oe > 0 ? (double)erzDa / oe : 1e6;
  if (se > 0 && (double)seltenDa / se < n) n = (double)seltenDa / se;
  return n * gegenwert(z.spieler, traeger, waffe);
}
inline int erzFuerKampf(const Zustand& z);
inline int waffeWahl(const Zustand& z, int traeger, int seltenDa) {
  static const int KANDIDATEN[] = { mapLaser, mapMicrowave, mapRailGun, mapRPG, mapAcidCloud, mapESG, mapThorsHammer, mapEnergyCannon };
  int beste = Player[z.spieler].IsEden() ? mapLaser : mapMicrowave;
  const int erzDa = erzFuerKampf(z);
  double bestWert = staerkeAusVorrat(z, traeger, beste, erzDa, seltenDa);
  for (int w : KANDIDATEN) {
    if (w == beste || !einheitErforscht(z, w)) continue;
    UnitInfo wi((map_id)w);
    if (wi.GetRareOreCost(z.spieler) > seltenDa) continue;
    const double v = staerkeAusVorrat(z, traeger, w, erzDa, seltenDa);
    if (v > bestWert) { bestWert = v; beste = w; }
  }
  return beste;
}

// Bestes erforschtes Kampf-Fahrgestell (Tiger > Panther > Lynx), Seltenerz muss reichen. 0 = keins.
inline int fahrgestellWahl(const Zustand& z, int seltenDa) {
  static const int REIHE[] = { mapTiger, mapPanther, mapLynx };
  const int erzDa = erzFuerKampf(z);
  int beste = 0; double bestWert = -1;
  for (int f : REIHE) {
    if (!einheitErforscht(z, f)) continue;
    UnitInfo fi((map_id)f);
    if (fi.GetRareOreCost(z.spieler) > seltenDa) continue;
    const int w = waffeWahl(z, f, seltenDa - fi.GetRareOreCost(z.spieler));
    const double v = staerkeAusVorrat(z, f, w, erzDa, seltenDa);
    if (v > bestWert) { bestWert = v; beste = f; }
  }
  return beste;
}

// Erz fuer die Fahrzeugwahl: Vorrat plus Zufluss im Planungshorizont, mindestens 1 (Verhaeltnis der Kosten entscheidet dann)
inline int erzFuerKampf(const Zustand& z) {
  const double e = Player[z.spieler].Ore() + (z.zuflussErz > 0 ? z.zuflussErz : 0) * PAR(z).horizont / 1000;
  return e > 1 ? (int)e : 1;
}
inline int eigeneWaffe(const Zustand& z) { return waffeWahl(z, mapGuardPost, Player[z.spieler].RareOre()); }

// Kampfkraft je Erz (Seltenerz zaehlt wie Erz) der besten erforschten Kombination aus Fahrgestell und Waffe -- mit oder
// ohne Seltenerz. Combat strength per ore of the best researched chassis/weapon, with or without rare ore.
inline double kraftJeErz(const Zustand& z, bool mitSelten) {
  static const int TRAEGER[] = { mapLynx, mapPanther, mapTiger };
  static const int WAFFEN[] = { mapLaser, mapMicrowave, mapRailGun, mapRPG, mapAcidCloud, mapESG, mapThorsHammer, mapEnergyCannon };
  const int grund = Player[z.spieler].IsEden() ? mapLaser : mapMicrowave;
  double beste = 0;
  for (int f : TRAEGER) {
    if (!einheitErforscht(z, f)) continue;
    UnitInfo fi((map_id)f);
    for (int w : WAFFEN) {
      if (w != grund && !einheitErforscht(z, w)) continue;
      UnitInfo wi((map_id)w);
      const int sel = fi.GetRareOreCost(z.spieler) + wi.GetRareOreCost(z.spieler);
      if (!mitSelten && sel > 0) continue;
      const int kosten = fi.GetOreCost(z.spieler) + wi.GetOreCost(z.spieler) + sel;
      if (kosten <= 0) continue;
      const double k = gegenwert(z.spieler, f, w) / kosten;
      if (k > beste) beste = k;
    }
  }
  return beste;
}
// Vorteil der Seltenerz-Fahrzeuge (0.3 = 30 % mehr Kampfkraft je Erz); 0 ohne Kampffahrzeug. Rare-ore advantage.
inline double seltenVorteil(const Zustand& z) {
  const double ohne = kraftJeErz(z, false);
  return ohne > 0 ? kraftJeErz(z, true) / ohne - 1 : 0;
}
// Seltenerz lohnt sich (Parameter seltenGewinn): dann wird die Seltenerz-Schmelze gebaut und mit Mine und Lastwagen betrieben.
inline bool seltenGewollt(const Zustand& z) { return seltenVorteil(z) >= PAR(z).seltenGewinn && seltenVorteil(z) > 0; }

inline int laufPunkte(const Zustand& z);    // weiter unten (Langzeit-Wissen)
inline bool l_kolonieLebt(const Zustand& z) { PlayerBuildingEnum e(z.spieler, mapCommandCenter); UnitEx u; return e.GetNext(u); }

// Prognose der naechsten Welle: Trend der letzten beiden Wellen (nie kleiner als die letzte). Vor der ersten
// Welle: was die Feind-Fahrzeugfabriken schicken koennten (zwei Lynx je Fabrik), nach der ersten die Haelfte mehr.
// Next wave forecast: trend of the last two waves; before the first, two Lynx per enemy factory.
inline int welleProg(const Zustand& z) {
  if (z.anzWellen == 0) return z.feindFabriken * 2 * WERT_LYNX;
  const int letzte = z.wellen[z.anzWellen - 1];
  if (z.anzWellen == 1) return letzte + letzte / 2;
  const int trend = letzte - z.wellen[z.anzWellen - 2];
  return letzte + (trend > 0 ? trend : 0);
}

inline void karteScannen(Zustand& z) {
  const int t = TethysGame::Tick();
  if (t < z.naechsterScan) return;
  z.naechsterScan = t + SCAN_TAKT;
  hinweis(z, "ki::karteScannen/feind");
  int armee = 0, nah = 0, nahN = 0, geb = 0, gx = 0, gy = 0, fabriken = 0, wt = 0;
  int bestW = 0, bestT = 0, bestWf = 0;
  z.feindPosN = 0; z.feindMinD2 = 1 << 30;
  for (int p = 0; p < TethysGame::NoPlayers(); ++p) {
    if (p == z.spieler || ExtPlayer[z.spieler].IsAlliedTo(p)) continue;
    {
      PlayerVehicleEnum e(p);
      UnitEx u;
      while (e.GetNext(u)) {
        if (!u.IsLive()) continue;
        const int w = kampfWert(u);
        if (!w) continue;
        armee += w;
        if (w > bestW) { bestW = w; bestT = u.GetType(); bestWf = u.GetWeapon(); }
        const LOCATION lo = u.Location();
        const int ux = edX(z, lo), uy = edY(z, lo), d2 = abstand2(ux, uy, z.bx, z.by);
        if (d2 < PAR(z).nahRadius * PAR(z).nahRadius) {
          nah += w; nahN++;
          const double wk = std::atan2((double)(uy - z.by), (double)(ux - z.bx));
          z.angriffSektoren |= 1 << (((int)std::floor(wk / (3.14159265358979 / 4) + 0.5) + 8) % 8);
        }
        if (z.feindPosN < 16) { z.feindPosX[z.feindPosN] = ux; z.feindPosY[z.feindPosN] = uy; z.feindPosN++; }
        if (d2 < z.feindMinD2) { z.feindMinD2 = d2; z.feindNX = ux; z.feindNY = uy; }
      }
    }
    for (int ty : GEBAEUDE) {
      PlayerBuildingEnum e(p, (map_id)ty);
      UnitEx b;
      while (e.GetNext(b)) {
        if (!b.IsLive()) continue;
        const LOCATION lo = b.Location();
        geb++; gx += edX(z, lo); gy += edY(z, lo);
        if (ty == mapVehicleFactory) fabriken++;
        if (ty == mapGuardPost) wt++;                              // verteidigen nur -- kein Angriffswert
      }
    }
  }
  int eigen = 0, mobil = 0, wtWert = 0;
  {
    hinweis(z, "ki::karteScannen/eigene-fz");
    PlayerVehicleEnum e(z.spieler);
    UnitEx u;
    while (e.GetNext(u)) if (u.IsLive() && kaempfer(u)) mobil += kampfWert(u);
  }
  {
    PlayerBuildingEnum e(z.spieler, mapGuardPost);
    UnitEx b;
    while (e.GetNext(b)) if (b.IsLive() && b.GetCurAction() != moObjWait) wtWert += kampfWert(b);
  }
  eigen = mobil + wtWert;
  z.eigeneMobil = mobil; z.eigeneWTWert = wtWert;
  // Aussenposten: eigene Minen weiter als AUSSEN_R von der Basismitte. Feind in 10 Kacheln = Angriff dort.
  // Outposts: own mines farther than AUSSEN_R; an enemy within 10 tiles counts as an attack there.
  {
    int gesehen[6] = { 0, 0, 0, 0, 0, 0 };
    static const int MINEN[] = { mapCommonOreMine, mapRareOreMine };
    for (int mt : MINEN) {
      PlayerBuildingEnum e(z.spieler, (map_id)mt);
      UnitEx b;
      while (e.GetNext(b)) {
        const LOCATION lo = b.Location();
        const int mx = edX(z, lo), my = edY(z, lo);
        if (abstand2(mx, my, z.bx, z.by) <= AUSSEN_R * AUSSEN_R) continue;
        int k = -1;
        for (int i = 0; i < 6; ++i) if (z.aussenId[i] == b.unitID) k = i;
        if (k < 0) for (int i = 0; i < 6; ++i) if (!z.aussenId[i] && !z.aussenAn[i]) { k = i; break; }
        if (k < 0) continue;
        if (z.aussenId[k] != b.unitID) {
          z.aussenId[k] = b.unitID; z.aussenX[k] = mx; z.aussenY[k] = my; z.aussenAngriff[k] = 0;
          melde(z, "kampf", mt, mx, my, "Aussenposten: %s %d Kacheln von der Basis", name(mt), (int)std::sqrt((double)abstand2(mx, my, z.bx, z.by)));
        }
        gesehen[k] = 1;
        for (int f = 0; f < z.feindPosN; ++f)
          if (abstand2(z.feindPosX[f], z.feindPosY[f], mx, my) < 10 * 10) {
            if (!z.aussenAngriff[k] || t - z.aussenAngriff[k] > 3000)
              melde(z, "kampf", mt, mx, my, "Aussenposten %s wird angegriffen", name(mt));
            z.aussenAngriff[k] = t;
            break;
          }
      }
    }
    for (int i = 0; i < 6; ++i) if (!gesehen[i]) z.aussenId[i] = 0;
  }
  const bool bekannt = geb > 0 || armee > 0;
  if (bekannt && !z.feindGemeldet) {
    melde(z, "kampf", mapNone, geb ? gx / geb : z.bx, geb ? gy / geb : z.by,
          "Feind entdeckt (Kartenscan): %d Gebaeude, davon %d Fahrzeugfabrik(en) und %d Wachtuerme, Armee Wert %d", geb, fabriken, wt, armee);
    z.feindGemeldet = 1;
  }
  z.feindArmee = armee; z.feindNah = nah; z.feindGeb = geb; z.feindFabriken = fabriken; z.feindWT = wt;
  // Qualitaet merken (das staerkste je gesehene bleibt -- ein Tiger, der einmal da war, kommt wieder)
  if (bestW > z.feindBestWert) {
    z.feindBestWert = bestW; z.feindBestTyp = bestT; z.feindBestWaffe = bestWf;
    const int fz = fahrgestellWahl(z, 1 << 20);
    melde(z, "kampf", bestT, z.feindX, z.feindY, "Feind hat jetzt %s mit %s (Wert %d), unser bestes baubares: %s (Wert %d)",
          name(bestT), name(bestWf), bestW, fz ? name(fz) : "-", fz ? sdkWertT(fz, waffeWahl(z, fz, 1 << 20)) : 0);
  }
  if (geb) { z.feindX = gx / geb; z.feindY = gy / geb; }
  // Unterwegs = feindliche Kampffahrzeuge mehr als 15 Kacheln von der Mitte ihrer Gebaeude (ohne bekannte Basis: alle).
  // Eine stehende Armee zu Hause ist keine Welle (fuer Gegner mit eigener Armee, z.B. eine zweite KI).
  // Wave = enemy combat vehicles outside their base; a standing army at home is not a wave.
  {
    int unterwegs = 0;
    for (int p = 0; p < TethysGame::NoPlayers(); ++p) {
      if (p == z.spieler || ExtPlayer[z.spieler].IsAlliedTo(p)) continue;
      PlayerVehicleEnum e(p);
      UnitEx u;
      while (e.GetNext(u)) {
        const int w = kampfWert(u);
        if (!w) continue;
        const LOCATION lo = u.Location();
        if (!geb || abstand2(edX(z, lo), edY(z, lo), z.feindX, z.feindY) > 15 * 15) unterwegs += w;
      }
    }
    z.feindUnterwegs = unterwegs;
  }
  z.eigeneArmee = eigen;
  if (nah > 0) {
    if (t - z.letzterAngriff > 3000 || !z.letzterAngriff) {
      z.angriffe++;
      melde(z, "kampf", mapNone, z.bx, z.by, "Angriff %d: %d feindliche Kampfeinheiten (Wert %d) im Umkreis %d der Basis, eigener Wert %d",
            z.angriffe, nahN, nah, (int)PAR(z).nahRadius, eigen);
    }
    z.letzterAngriff = t;
    if (nah > z.staerksterAngriff) z.staerksterAngriff = nah;
  }
  if (armee > z.staerksterAngriff) z.staerksterAngriff = armee;
  // Verluste: weniger eigene Gebaeude als beim letzten Scan, waehrend ein Angriff laeuft
  {
    int n = 0;
    for (int ty : GEBAEUDE) { PlayerBuildingEnum e(z.spieler, (map_id)ty); UnitEx b; while (e.GetNext(b)) n++; }
    if (z.eigeneGeb && n < z.eigeneGeb && z.letzterAngriff && t - z.letzterAngriff < 1000)
      melde(z, "kampf", mapNone, z.bx, z.by, "Verlust beim Angriff: %d Gebaeude weniger (jetzt %d)", z.eigeneGeb - n, n);
    z.eigeneGeb = n;
  }
  int bed = 0;
  if (bekannt) {
    bed = 15;
    if (fabriken) bed += 10;                                       // kann Nachschub bauen
    if (armee > eigen) bed += (armee - eigen) * 3 > 40 ? 40 : (armee - eigen) * 3;
    if (z.letzterAngriff && t - z.letzterAngriff < 6000) bed += 40;
  }
  z.bedrohung = bed > 100 ? 100 : bed;
  // Wachstum der Feindarmee (Zuwachs je 1000 Ticks, gleitend): wie schnell der Feind aufruestet
  // Enemy army growth per 1000 ticks (smoothed): how fast the enemy arms.
  {
    const int d = armee - z.feindArmeeVorher;
    z.feindWachstum = z.feindWachstum * 0.9 + (d > 0 ? d : 0) * (1000.0 / SCAN_TAKT) * 0.1;
    z.feindArmeeVorher = armee;
  }
  // Gefechte erfassen (Spieler: keine Wellen -- die Verteidigung richtet sich nach dem Feind): ein Gefecht laeuft,
  // solange Feind-Kampfeinheiten in der Kolonie sind (NAH_RADIUS), und endet 1000 Ticks danach. Gemessen wird der
  // eigene Kampfwert vorher und sein Tiefstwert -- daraus lernt die KI die Reserve.
  // Fights: enemy combat units inside the colony; the loss per fight teaches the reserve.
  if (nah > 0) {
    if (z.welleSpitze == 0) { z.welleStart = t; z.welleEigenStart = eigen; z.welleEigenMin = eigen; }
    if (nah > z.welleSpitze) z.welleSpitze = nah;
    if (eigen < z.welleEigenMin) z.welleEigenMin = eigen;
  }
  else if (z.welleSpitze > 0 && t - z.letzterAngriff > 1000) {
    // Lernen: Anteil des eigenen Kampfwerts, der in dieser Welle verloren ging. Viel verloren -> Reserve hoch,
    // nichts verloren -> langsam herunter (nie unter 1). Learn: lost share raises the reserve, none lowers it.
    const int verlust = z.welleEigenStart - z.welleEigenMin;
    const double anteil = z.welleEigenStart > 0 ? (double)verlust / z.welleEigenStart : (verlust > 0 ? 1.0 : 0.0);
    const double alt = z.reserve;
    // gedaempft (Duell 11: Reserve 4,0 trieb das Soll ueber 540): halb so starker Anstieg, Deckel 2,5
    if (anteil > 0.05) z.reserve *= 1.0 + anteil * PAR(z).reserveLernrate;
    else z.reserve *= PAR(z).reserveRueckgang;
    if (z.reserve < 1.0) z.reserve = 1.0;
    if (z.reserve > PAR(z).reserveMax) z.reserve = PAR(z).reserveMax;
    if (verlust > 0) z.verlustSumme += verlust;
    if (l_kolonieLebt(z)) z.ueberstanden++;
    melde(z, "lernen", mapNone, z.bx, z.by, "Gefecht %d: eigener Wert %d -> Tiefst %d (Verlust %d %%) -- Reserve %.2f -> %.2f",
          z.anzWellen + 1, z.welleEigenStart, z.welleEigenMin, (int)(anteil * 100), alt, z.reserve);
    z.naechstesSpeichern = 0;                                      // Gelerntes gleich sichern
    if (z.anzWellen == 8) { for (int i = 1; i < 8; ++i) z.wellen[i - 1] = z.wellen[i]; z.anzWellen = 7; }
    z.wellen[z.anzWellen++] = z.welleSpitze;
    melde(z, "kampf", mapNone, z.bx, z.by, "Gefecht %d vorbei: Feind in der Kolonie bis Wert %d", z.anzWellen, z.welleSpitze);
    z.welleSpitze = 0;
  }
  // Soll aus dem Feind: seine ganze Armee (auch zu Hause) plus das, was er in der Zeit nachbaut, die wir selbst zum
  // Nachruesten brauchen (REAKTION), mindestens ein Fahrzeug je Feind-Fabrik -- mal der gelernten Reserve.
  // Target from the enemy: whole army plus its growth over our reaction time, times the learned reserve.
  // Potenzial (Spieler: proaktiv, nicht hinterherhinken): was der Feind in der Reaktionszeit aufstellen koennte --
  // seine Armee plus je Fahrzeugfabrik so viele Fahrzeuge, wie in REAKTION Ticks entstehen, mit dem besten Fahrgestell
  // und der besten Waffe, die wir selbst bauen koennten (Annahme: technisch gleichauf). Mindestens Lynx-Laser.
  // Potential: army plus what its factories could build in our reaction time, assuming tech parity.
  {
    int fz = fahrgestellWahl(z, 1 << 20); if (!fz) fz = mapLynx;
    int wert = sdkWertT(fz, waffeWahl(z, fz, 1 << 20));
    if (z.feindBestWert > wert) wert = z.feindBestWert;     // gesehene Feind-Qualitaet (Tiger) statt Annahme Gleichstand
    UnitInfo fi((map_id)fz);
    // Bauzeit je Fahrzeug in Ticks: GetBuildTime ist nicht in Ticks (Duell 8: Soll 189 bei 2 Feind-Fabriken) --
    // mindestens 2000 Ticks je Fahrzeug und Fabrik (gemessen: ~500-600 Ticks Abstand bei Dauerproduktion, plus Erz)
    int bau = fi.GetBuildTime(z.spieler) * 4;
    if (bau < 2000) bau = 2000;
    const int stueck = (int)PAR(z).reaktion / (bau > 0 ? bau : 2400) + 1;
    z.feindPotenzial = armee + fabriken * stueck * wert;
  }
  // Reserve (gelernt aus Verlusten) nur auf die tatsaechliche Armee und ihr Wachstum -- das Potenzial ist schon eine
  // vorsichtige Schaetzung (Duell 9: Soll 148-180, weil beides multipliziert wurde)
  const int echt = (int)((armee + z.feindWachstum * PAR(z).reaktion / 1000) * z.reserve + 0.5);
  z.sollArmee = echt > z.feindPotenzial ? echt : z.feindPotenzial;
}

inline void feindRichtung(const Zustand& z, int x, int y, double& ux, double& uy) {
  const int dx = z.feindGeb ? z.feindX - x : 1, dy = z.feindGeb ? z.feindY - y : 0;
  const double len = std::sqrt((double)(dx * dx + dy * dy));
  ux = len > 0 ? dx / len : 1.0; uy = len > 0 ? dy / len : 0.0;
}

inline int tuermeBei(const Zustand& z, int x, int y, int r = 6) {
  int n = 0;
  PlayerBuildingEnum e(z.spieler, mapGuardPost);
  UnitEx b;
  while (e.GetNext(b)) { const LOCATION l = b.Location(); if (abstand2(edX(z, l), edY(z, l), x, y) <= r * r) n++; }
  for (int i = 0; i < z.anzahlPlaetze; ++i) {
    const Platz& q = z.plaetze[i];
    if (q.typ == mapGuardPost && (q.zustand == P_FAHRT || q.zustand == P_BAUT) && abstand2(q.x0, q.y0, x, y) <= r * r) n++;
  }
  return n;
}


// Aussenposten gefaehrdet: Bedrohung besteht und die Mine liegt naeher am Feind als die Basis, oder sie
// wurde in den letzten 10000 Ticks angegriffen. Dann bekommt sie einen Turm; Fahrzeuge entscheidet die KI
// nach Lage: 2 nach einem Angriff dort, 1 bei hoher Bedrohung, sonst keins.
// Outpost at risk: nearer to the enemy than the base, or attacked recently. Tower yes; vehicles by situation.
inline bool aussenGefaehrdet(const Zustand& z, int i) {
  if (!z.aussenId[i] || !z.bedrohung) return false;
  if (z.aussenAngriff[i] && TethysGame::Tick() - z.aussenAngriff[i] < 10000) return true;
  return z.feindGeb && abstand2(z.aussenX[i], z.aussenY[i], z.feindX, z.feindY) < abstand2(z.bx, z.by, z.feindX, z.feindY);
}
inline int aussenFahrzeuge(const Zustand& z, int i) {
  if (!aussenGefaehrdet(z, i)) return 0;
  if (z.aussenAngriff[i] && TethysGame::Tick() - z.aussenAngriff[i] < 10000) return 2;
  return z.bedrohung >= 50 ? 1 : 0;
}
inline int aussenTurmFehlt(const Zustand& z) {
  for (int i = 0; i < 6; ++i) if (aussenGefaehrdet(z, i) && tuermeBei(z, z.aussenX[i], z.aussenY[i]) == 0) return i;
  return -1;
}
// Fahrzeuge, die ueber die Verteidigung hinaus gebraucht werden: Aussenposten und -- nach dem Ziel -- eine
// Angriffstruppe, die die Feindbasis (Wachtuerme nach SDK-Wert) doppelt schlaegt.
// Mobile strength needed beyond defence: outposts and, after the goal, a strike force.
// mitAngriff: Angriffstruppe, wenn der Feind Nachschub baut und die Wirtschaft Erz uebrig hat -- stark genug
// fuer seine Tuerme plus die naechste Welle, mit 50 % Reserve (kein "erst nach dem Ziel" mehr).
inline int mobilSoll(const Zustand& z, bool mitAngriff = true) {
  int n = 0;
  for (int i = 0; i < 6; ++i) n += aussenFahrzeuge(z, i) * WERT_LYNX;
  // Ziel "Gegner vernichten": die Angriffstruppe IST das Ziel -- auch ohne Erz-Ueberschuss
  if (mitAngriff && z.feindGeb && ((z.feindFabriken && z.erzUeberschuss) || (z.ziel == 3 && !z.zielErreicht))) {
    int a = (int)((z.feindWT * WERT_WT + z.feindArmee) * PAR(z).ueberlegenheit);
    // Beim Ziel "Gegner vernichten" auch gegen eine wehrlose Basis: genug Feuerkraft, dass ihre Gebaeude in der
    // Reaktionszeit fallen (gemessen: 2 Laser-Lynx brauchen ~850 Ticks fuer ein Tokamak). Duell 18: Feind ohne Armee
    // -> Angriffstruppe 0, beide bauten bis Tick 70000 keine Fahrzeugfabrik.
    // Against an undefended base: enough firepower to level its buildings within the reaction time.
    if (z.ziel == 3 && !z.zielErreicht) {
      const int feuer = z.feindGeb * 2 * 850 / (int)PAR(z).reaktion * WERT_LYNX + WERT_LYNX;
      if (feuer > a) a = feuer;
    }
    // Eskalation (Spieler: "die Angriffe muessen mit der Zeit mehr werden ... der Gegner muss immer unter Druck
    // gesetzt werden"): jeder Angriff staerker als der letzte. Daraus folgen von selbst mehr Erzbedarf (Bilanz ->
    // Schmelzen, Minen), mehr Fahrzeugfabriken (weitereFabrikLohnt) und der Druck auf die Kampfforschung.
    // Escalation: each attack stronger than the last; economy, factories and research follow from the demand.
    if (z.angriffStart > 0) {
      const int st = (int)(z.angriffStart * PAR(z).steigerung + 0.5);
      if (st > a) a = st;
    }
    n += a;
  }
  return n;
}

// ---------------------------------------------------------------------------------------------------
// Rohstoff-Bilanz / resource balance
//
// Keine festen Regeln, was zuerst kommt (Spieler): Bedarf je Rohstoff ueber HORIZONT Ticks (was das Ziel
// noch braucht, was der Verteidigung fehlt) gegen Vorrat und gemessenen Zufluss. Der Engpass entscheidet,
// ob eine weitere Erzschmelze, eine Seltenerz-Schmelze oder eine neue Mine den groessten Nutzen hat: wie viel
// Engpass sie im Horizont beseitigt, geteilt durch ihre Kosten. Gebaut wird nur, was sich im Horizont bezahlt.
// No fixed order: demand per resource over the horizon vs. stock and measured income decides.

struct Bilanz {
  int bedarfErz, bedarfSelten;      // im Horizont
  int erzHaengtAnSelten;            // Erz-Bedarf von Vorhaben, die ohne Seltenerz nicht gehen (blockiert)
  int engpassErz, engpassSelten;    // Bedarf - Vorrat - Zufluss im Horizont (> 0 = fehlt)
  int routeErz, routeSelten;        // erwarteter Zufluss einer weiteren Route im Horizont
  const char* treiber;              // wer den Seltenerz-Bedarf vor allem erzeugt
};
inline bool seltenBakeDa(const Zustand& z);       // nach bakeSuchen()
inline int kampfFehlt(const Zustand& z, const Lage& l);
inline bool kitMoeglich(const Zustand& z, int typ);

inline int erzRouten(const Zustand& z) { int n = 0; for (int i = 0; i < MAX_MINEN; ++i) if (z.minenId[i]) n++; return n; }
inline int seltenRouten(const Zustand& z) { int n = 0; for (int i = 0; i < 2; ++i) if (z.seltenMineId[i]) n++; return n; }

inline Bilanz bilanz(const Zustand& z, const Lage& l) {
  Bilanz b = { 0, 0, 0, 0, 0, 0, 0, "-" };
  int seltenZiel = 0, seltenKampf = 0;
  // Ziel: was noch fehlt (Gebaeude, Rakete, Satellit)
  if (z.ziel == 1 && !z.zielErreicht) {
    static const int ZIEL[] = { mapStandardLab, mapAdvancedLab, mapSpaceport };
    for (int t : ZIEL) {
      if (l.geb[t] + geplant(z, t) + l.kitsImSpiel[t]) continue;
      UnitInfo info((map_id)t);
      b.bedarfErz += info.GetOreCost(z.spieler); seltenZiel += info.GetRareOreCost(z.spieler);
      if (info.GetRareOreCost(z.spieler) > 0) b.erzHaengtAnSelten += info.GetOreCost(z.spieler);
    }
    static const int RAUM[] = { mapSULV, mapEDWARDSatellite };
    for (int t : RAUM) {
      const tb::Einheit* e = tb::einheit(t);
      if (e) { b.bedarfErz += e->erz; seltenZiel += e->selten; if (e->selten > 0) b.erzHaengtAnSelten += e->erz; }
    }
  }
  // Verteidigung: fehlende Kampfwerte als Fahrzeuge mit dem gewaehlten Fahrgestell und der gewaehlten Waffe
  {
    int fehlt = 0;
    // Ohne Angriffstruppe (Rueckkopplung: sie haengt am Erz-Ueberschuss) -- ausser beim Ziel "Gegner vernichten": da IST
    // sie das Ziel und haengt nicht am Ueberschuss (Duell 14: Angreifer blieb bei 1 Schmelze, Lager immer ~500 Erz).
    // The attack troop counts toward ore demand when destroying the enemy is the goal.
    const int k = kampfFehlt(z, l), m = mobilSoll(z, z.ziel == 3 && !z.zielErreicht) - z.eigeneMobil;
    if (k > 0) fehlt += k;
    if (m > 0) fehlt += m;
    const int fz = fahrgestellWahl(z, 1 << 20);                   // was die KI bauen wuerde, wenn Seltenerz da waere
    if (fehlt > 0 && fz) {
      const int waffe = waffeWahl(z, fz, 1 << 20);
      UnitInfo fi((map_id)fz), wi((map_id)waffe);
      const int n = (fehlt + sdkWertT(fz, waffe) - 1) / sdkWertT(fz, waffe);
      b.bedarfErz += n * (fi.GetOreCost(z.spieler) + wi.GetOreCost(z.spieler));
      seltenKampf += n * (fi.GetRareOreCost(z.spieler) + wi.GetRareOreCost(z.spieler));
      if (fi.GetRareOreCost(z.spieler) + wi.GetRareOreCost(z.spieler) > 0) b.erzHaengtAnSelten += n * (fi.GetOreCost(z.spieler) + wi.GetOreCost(z.spieler));
    } else if (fehlt > 0) {
      UnitInfo gi(mapGuardPost);
      b.bedarfErz += (fehlt + WERT_WT - 1) / WERT_WT * gi.GetOreCost(z.spieler);
    }
  }
  // EMP-Rakete (Plymouth), wenn sie sich gegen die jetzige Abwehr lohnt: ihr Erz und Seltenerz gehoeren zum Bedarf
  if (empMoeglich(z) && !z.empRakete && empLohnt(z)) {
    UnitInfo r(mapEMPMissile);
    b.bedarfErz += r.GetOreCost(z.spieler); seltenKampf += r.GetRareOreCost(z.spieler);
    b.erzHaengtAnSelten += r.GetOreCost(z.spieler);
  }
  b.bedarfSelten = seltenZiel + seltenKampf;
  b.treiber = seltenZiel >= seltenKampf ? (seltenZiel ? "Ziel" : "-") : "Verteidigung";
  // Netto-Fluss nur, wenn er positiv ist (vorsichtig: ein negativer Fluss heisst, die laufenden Ausgaben fressen alles)
  b.engpassErz = b.bedarfErz - l.erz - (int)((z.zuflussErz > 0 ? z.zuflussErz : 0) * PAR(z).horizont / 1000);
  b.engpassSelten = b.bedarfSelten - l.selten - (int)((z.zuflussSelten > 0 ? z.zuflussSelten : 0) * PAR(z).horizont / 1000);
  // Ertrag einer weiteren Route: gemessen je Route; solange noch nichts gemessen ist, der Erz-Ertrag je Route
  // (Seltenerz: halb so viel -- Schaetzung, bis die erste Seltenerz-Route liefert)
  const int re = erzRouten(z), rs = seltenRouten(z);
  // Ertrag je Route: der Netto-Fluss unterschaetzt ihn (Ausgaben stecken drin) -- darum mindestens die Schaetzung
  // Schaetzung aus der Lage: zwei Lastwagen auf dem Abstand der naechsten eigenen Mine (500 Erz je Fahrt,
  // 300 + 60 Ticks je Kachel), hoechstens was eine Schmelze schafft (~1400 je 1000 Ticks). Die feste Vorgabe 300
  // war zu klein (Karte 3: Nutzen der zweiten Schmelze blieb unter der Schwelle, 400000 Ticks bei 1 Schmelze).
  // Route yield estimated from the distance to the nearest own mine instead of a fixed 300.
  double dMine = -1;                                      // ohne Mine: 10 Kacheln
  for (int i = 0; i < MAX_MINEN; ++i) {
    if (!z.minenId[i]) continue;
    UnitEx m; m.unitID = z.minenId[i];
    if (!m.IsLive()) continue;
    const LOCATION ml = m.Location();
    const double d = std::sqrt((double)abstand2(edX(z, ml), edY(z, ml), z.bx, z.by));
    if (dMine < 0 || d < dMine) dMine = d;
  }
  if (dMine < 0) dMine = 10;
  double geschaetzt = 2 * 500 * 1000 / (300 + 60 * dMine);
  if (geschaetzt > 1400) geschaetzt = 1400;
  const double vorgabe = z.erzJeRouteGelernt > geschaetzt ? z.erzJeRouteGelernt : geschaetzt;
  const double jeErz = re > 0 && z.zuflussErz / re > vorgabe ? z.zuflussErz / re : vorgabe;
  const double jeSelten = rs > 0 && z.zuflussSelten > 0 ? z.zuflussSelten / rs : jeErz / 2;
  b.routeErz = (int)(jeErz * PAR(z).horizont / 1000);
  b.routeSelten = (int)(jeSelten * PAR(z).horizont / 1000);
  return b;
}

// Bestes Wirtschaftsvorhaben aus der Bilanz: Nutzen = beseitigter Engpass im Horizont / Kosten.
struct ErzWahl { int typ; double nutzen; };
inline bool standardVolk(const Lage& l);
inline int rangNeu(const Zustand& z, const Lage& l, int typ, int schon);
inline ErzWahl erzWahl(const Zustand& z, const Lage& l, const Bilanz& b) {
  ErzWahl w = { mapNone, 0 };
  const int schm = l.geb[mapCommonOreSmelter] + geplant(z, mapCommonOreSmelter) + l.kitsImSpiel[mapCommonOreSmelter];
  const int mn = l.geb[mapCommonOreMine];
  // Arbeiter fuer eine neue Schmelze nur aus freien oder ueberzaehligen (Rang < 50) -- holt sie sie aus Wohnhaeusern,
  // Medizin oder der Fahrzeugfabrik, kostet sie mehr, als sie bringt (Spiel gegen den Spieler: 4 Schmelzen, Moral 0).
  // A new smelter only takes free or surplus workers; taking them from housing, medicine or the factory costs more.
  UnitInfo si(mapCommonOreSmelter);
  // Lager leer und der Engpass groesser als eine ganze Route: die Fahrzeugfabrik wartet ohnehin auf Erz, ihre Arbeiter
  // bringen dort kaum etwas -- sie zaehlen fuer die Schmelze mit (Karte 3: 400000 Ticks bei 1 Schmelze, Erz ~0,
  // 1-5 freie Arbeiter, die zweite Schmelze kam nie). Wen die Rangliste dann stilllegt, entscheidet sie.
  // Ore-bound factory: its workers count for a new smelter when the stock is empty and the shortfall exceeds a route.
  int erzGebunden = 0;
  if (l.erz < 2000 && b.engpassErz > b.routeErz && b.routeErz > 0) {
    UnitInfo vi(mapVehicleFactory);
    erzGebunden = vi.GetWorkersRequired(z.spieler) * l.geb[mapVehicleFactory];
  }
  const int stilllegbarS = l.arbStilllegbar[5];
  const int rangS = rangNeu(z, l, mapCommonOreSmelter, schm);
  const bool schmArbeiter = standardVolk(l) || si.GetWorkersRequired(z.spieler) <= l.arbFrei + stilllegbarS + erzGebunden - beansprucht(l, M_ARB, rangS, mapCommonOreSmelter);
  if (mn > 0 && schm < 2 * mn && schm < MAX_MINEN && b.engpassErz > 0 && schmArbeiter && kitMoeglich(z, mapCommonOreSmelter)) {
    UnitInfo i(mapCommonOreSmelter);
    const double n = (b.engpassErz < b.routeErz ? b.engpassErz : b.routeErz) / (double)(i.GetOreCost(z.spieler) + 1);
    if (n > w.nutzen) w = { mapCommonOreSmelter, n };
  }
  const int sel = l.geb[mapRareOreSmelter] + geplant(z, mapRareOreSmelter) + l.kitsImSpiel[mapRareOreSmelter];
  if (sel == 0 && b.engpassSelten > 0 && kitMoeglich(z, mapRareOreSmelter) && seltenBakeDa(z)) {
    UnitInfo i(mapRareOreSmelter);
    // Ohne jeden Seltenerz-Zufluss sind die Vorhaben, die Seltenerz brauchen, ganz blockiert -- ihr Erz-Wert zaehlt
    // mit (Kampf 14: 1200 Seltenerz gegen 2500 Erz Baukosten "lohnte" nie, der Raumhafen wartete ewig).
    // Without any rare income, projects needing rare ore are blocked entirely; their ore value counts too.
    double gewinn = b.engpassSelten < b.routeSelten ? b.engpassSelten : b.routeSelten;
    if (z.zuflussSelten <= 0 && !seltenRouten(z)) gewinn += b.erzHaengtAnSelten;
    const double n = gewinn / (double)(i.GetOreCost(z.spieler) + 1);
    if (n > w.nutzen) w = { mapRareOreSmelter, n };
  }
  return w;
}

// Fehlender Kampfwert nach allem, was schon geplant oder in Arbeit ist.
// Missing combat value after everything already planned or in production.
inline int kampfFehlt(const Zustand& z, const Lage& l) {
  const int unterwegs = (geplant(z, mapGuardPost) + l.kitsImSpiel[mapGuardPost] + l.imBau[mapGuardPost]) * WERT_WT +
                        (istKampfFz(z.fzTyp) ? WERT_LYNX : 0);
  return z.sollArmee - z.eigeneArmee - unterwegs;
}

inline int hatGeb(const Zustand& z, const Lage& l, int t) { return l.geb[t] + geplant(z, t) + l.kitsImSpiel[t]; }
inline bool standardVolk(const Lage& l) { return l.menschen >= 8000; }    // KI-Kolonisten der Engine (4096/4096/256)

// Wie stark die Kolonie zaehlt: 100 bis ~50 Bewohner, dann fallend, ab ~130 nur noch 20.
// Colony weight: 100 up to ~50 inhabitants, then falling, 20 from ~130.
inline int kolonieGewicht(const Lage& l) {
  if (standardVolk(l)) return 20;
  int g = 100 - (l.menschen - 50);
  return g > 100 ? 100 : g < 20 ? 20 : g;
}

// Nahrungsreichweite in Ticks (Vorrat / Defizit); 1<<30 ohne Defizit.
inline int nahrungsReichweite(const Lage& l) {
  if (l.nahrungNetto >= 0) return 1 << 30;
  return l.nahrung * 100 / (-l.nahrungNetto);   // netto ist je 100 Ticks (eine Markierung)
}

// Rang eines Gebaeudes fuer Arbeiter (hoeher = wichtiger). Building rank for workers.
inline int rang(const Zustand& z, const Lage& l, int typ) {
  const bool ziel = z.ziel == 1 && !z.zielErreicht;
  switch (typ) {
    case mapCommandCenter: return 100;
    case mapTokamak: case mapMHDGenerator: case mapSolarPowerArray: case mapGeothermalPlant: return 95;
    case mapStructureFactory: return 90;
    case mapRobotCommand: return 60;
    case mapGarage: return 65;
    case mapAgridome: return l.nahrungNetto > 60 && l.geb[mapAgridome] > 1 ? 25 : 85;   // ueberzaehlig = unwichtig
    // Nie wegen vollem Lager abwerten: die Schmelze IST das Lager -- stillgelegt ist ihr Erz weg (Duell 15: Schmelze
    // mit Rang 60 fuer ein Freizeitzentrum stillgelegt, Erz 9974 -> 0, Kolonie verloren). Zu viele Schmelzen verhindert
    // erzWahl (nur freie Arbeiter). Never devalue the smelter: it holds the ore; idled, the stock is gone.
    case mapCommonOreSmelter: case mapCommonOreMine: return 80;
    case mapNursery: return 78;
    case mapUniversity: return 74;
    case mapMedicalCenter: return 70;
    // Labore: so wichtig, wie die Forschung sie gerade braucht (laborBedarf) -- nicht an ein bestimmtes Ziel gekoppelt
    // (Duell 6: Advanced Lab mit Rang 40 stillgelegt, die Angriffsforschung kam nie an Lynx)
    case mapAdvancedLab: { const int r = 30 + z.laborNutzen[3] * 6 / 10; return (ziel && r < 66) ? 66 : r; }
    case mapStandardLab: { const int r = 30 + z.laborNutzen[2] * 6 / 10; return r < 64 ? 64 : r; }
    // Seltenerz: so wichtig wie gewoehnliches Erz, sobald das gewaehlte Kampffahrzeug Seltenerz braucht (Panther,
    // Tiger, bessere Waffen) -- die Schmelze ist ausserdem das Lager (stillgelegt ist der Vorrat weg). Testlauf
    // Eskalation: Seltenerz-Schmelze mit Rang 30 immer wieder fuer ein Freizeitzentrum stillgelegt, nie ein Panther.
    // Rare ore matters like common ore once the chosen combat vehicle needs it.
    case mapRareOreSmelter: case mapRareOreMine: {
      return (seltenGewollt(z) || (empMoeglich(z) && empLohnt(z))) ? 80 : ziel ? 62 : 30;   // auch fuer die EMP-Rakete
    }
    case mapSpaceport: return ziel ? 62 : (!Player[z.spieler].IsEden() && hatTech(z, 12201)) ? 78 : 30;   // EMP-Rakete
    case mapVehicleFactory:
      // Eden-Analyse 2026-11-07: Fabrik bis Tick 40000 meist stillgelegt (Rang 55 unter GORF 68), Arbeiter gingen an GORF und
      // Wohnhaeuser. Fehlt Kampfwert gegen einen bekannten Feind, steht sie ueber GORF, Freizeit und Laboren (unter Medizin 70).
      if (!l.vfLaeuft) return z.bedrohung >= 50 ? 75 : 71;   // erste Fabrik vor Laboren und Medizin, unter Wohnraum
      return z.bedrohung >= 50 ? 75 : 55;    // Panzernachschub unter Druck
    case mapGuardPost: return 88;
    case mapRecreationFacility: case mapForum: return 68;          // Moral wie Medizin (Freizeit gehoert dazu)
    case mapDIRT: case mapGORF: return 70;   // wie Freizeit: erforscht, aber ohne Betrieb sinkt die Moral (Spieler)
    // Mehr als ein Haus Ueberschuss: unwichtig, damit die Arbeiter-Rangliste eins stilllegt
    case mapResidence: case mapReinforcedResidence: case mapAdvancedResidence:
      // Ueberschuss nur, wenn die Kolonie auch ohne ein Haus unter 85 % Auslastung bleibt (Spiel gegen den Spieler:
      // 64 Bewohner, 105 Platz, ein Haus stillgelegt -> Moral 0, Bevoelkerung von 64 auf 23).
      // Surplus only if the colony stays below 85 % occupancy without one house.
      {
        const int haeuser = l.geb[mapResidence] + l.geb[mapReinforcedResidence] + l.geb[mapAdvancedResidence];
        int proHaus = haeuser > 0 ? l.wohnraum / haeuser : 25; if (proHaus < 25) proHaus = 25;
        return (!standardVolk(l) && (l.wohnraum - proHaus) * PAR(z).wohnraumGrenze / 100 >= l.menschen) ? 15 : 72;   // Wohnraum traegt die Moral
      }
    // Lager: immer zuerst stilllegen -- ein volles Lager schadet nicht, ein fehlender Arbeiter schon (Spieler).
    // Storage is always first to idle: a full store does no harm, a missing worker does.
    case mapCommonStorage: case mapRareStorage: return 10;
    default: return 35;
  }
}

// Rang fuer ein weiteres Gebaeude eines Typs: das erste traegt die Aufgabe, jedes weitere nur noch den Zuwachs (Grenznutzen).
// Wohnraum, Nahrung, Kraftwerke und Tuerme rechnen ihren Bedarf in rang() bzw. bereichWahl selbst. Ersetzt fabrikVorrang
// (Eden 2026-11-08: die zweite Schmelze mit Rang 80 nahm der ersten Fahrzeugfabrik die Arbeiter).
// Rank for one more building of a type: the first carries the task, each further one only its increment.
inline int rangNeu(const Zustand& z, const Lage& l, int typ, int schon) {
  const int r = rang(z, l, typ);
  if (schon <= 0) return r;
  switch (typ) {
    case mapCommonOreSmelter: case mapRareOreSmelter: case mapVehicleFactory: case mapNursery: case mapUniversity:
    case mapMedicalCenter: case mapGarage: case mapRobotCommand: case mapSpaceport: case mapStandardLab: case mapAdvancedLab:
    case mapGORF: return r - 15;
    default: return r;
  }
}

// Vormerkungen: Erz/Seltenerz, die das Ziel als Naechstes braucht. Lueckenfueller und Kolonie-Ausbau
// duerfen nur den Rest nehmen. Reservations: ore the goal needs next.
inline void vormerkungen(const Zustand& z, const Lage& l, int& erz, int& selten, const char** wofuer) {
  erz = 0; selten = 0; *wofuer = "-";
  if (z.ziel != 1 || z.zielErreicht) return;
  static const int REIHE[] = { mapStandardLab, mapAdvancedLab, mapSpaceport };   // Seltenerz entscheidet die Bilanz
  for (int t : REIHE) {
    if (hatGeb(z, l, t)) continue;
    if (!kitMoeglich(z, t)) return;                  // Technik fehlt noch: noch nichts vormerken
    UnitInfo info((map_id)t);
    erz = info.GetOreCost(z.spieler); selten = info.GetRareOreCost(z.spieler); *wofuer = name(t);
    return;
  }
  if (l.geb[mapSpaceport]) { erz = 2500; selten = 700; *wofuer = "EDWARD+SULV"; }   // UnitInfo kennt Raumfahrzeuge nicht sicher
}

struct Wahl { int typ; int bereich; char warum[200]; };

// Lohnt eine weitere Fahrzeugfabrik? Keine feste Anzahl (Spieler): wie lange brauchen die vorhandenen Fabriken,
// um den fehlenden Kampfwert aufzufuellen -- begrenzt durch die Bauzeit je Fahrzeug UND durch das Erz (Vorrat plus
// Zufluss). Bringt eine Fabrik mehr die Aufruestung spuerbar unter die Reaktionszeit naeher, und sind Arbeiter frei
// (frei oder ueberzaehlig, nie aus Wohnhaeusern oder Medizin), wird gebaut. Erz-begrenzt hilft keine Fabrik.
// Another vehicle factory pays off if the existing ones rearm too slowly and ore and workers can carry one more.
constexpr int BAU_FABRIK = 3000;   // grob: Bausatz, Fahrt und Aufbau einer Fahrzeugfabrik
constexpr int BAU_FZ = 600;   // gemessen: ~500-600 Ticks je Fahrzeug bei Dauerproduktion einer Fabrik
inline bool weitereFabrikLohnt(const Zustand& z, const Lage& l, char* text, int n) {
  const int f = l.geb[mapVehicleFactory];
  if (f <= 0 || f >= MAX_VF || standardVolk(l)) return false;
  if (hatGeb(z, l, mapVehicleFactory) > f || l.imBau[mapVehicleFactory]) return false;     // eine ist schon unterwegs
  const int fz = fahrgestellWahl(z, l.selten);
  if (!fz) return false;
  UnitInfo fi((map_id)fz);
  const int waffe = waffeWahl(z, fz, l.selten - fi.GetRareOreCost(z.spieler));
  UnitInfo wi((map_id)waffe);
  const int wert = sdkWertT(fz, waffe) > 0 ? sdkWertT(fz, waffe) : WERT_LYNX;
  int fehlt = 0;
  const int k = kampfFehlt(z, l), m = mobilSoll(z, true) - z.eigeneMobil;
  if (k > 0) fehlt += k;
  if (m > 0) fehlt += m;
  const int stueck = (fehlt + wert - 1) / wert;
  if (stueck <= 1) return false;
  const int kosten = fi.GetOreCost(z.spieler) + wi.GetOreCost(z.spieler);
  // Erz-Zeit: was ueber den Vorrat hinaus fehlt, kommt mit dem Zufluss (je 1000 Ticks)
  // Zufluss: der Netto-Fluss enthaelt die laufenden Ausgaben -- mindestens der Ertrag der Routen (wie die Bilanz)
  const Bilanz bl = bilanz(z, l);
  double zufluss = erzRouten(z) * (double)bl.routeErz * 1000.0 / PAR(z).horizont;
  if (z.zuflussErz > zufluss) zufluss = z.zuflussErz;
  if (zufluss < 1) zufluss = 1;
  const int fehltErz = stueck * kosten - l.erz;
  const double tErz = fehltErz > 0 ? fehltErz * 1000.0 / zufluss : 0;
  auto dauer = [&](int fabriken) { const double tf = stueck * (double)BAU_FZ / fabriken; return tf > tErz ? tf : tErz; };
  const double jetzt = dauer(f), dann = dauer(f + 1);
  if (jetzt <= PAR(z).reaktion || dann > jetzt * PAR(z).fabrikGewinn) return false;      // schnell genug, oder das Erz begrenzt
  // Arbeiter nur aus freien oder ueberzaehligen Stellen (Rang < 50)
  UnitInfo vi(mapVehicleFactory);
  if (vi.GetWorkersRequired(z.spieler) > l.arbFrei + l.arbStilllegbar[5] - beansprucht(l, M_ARB, rangNeu(z, l, mapVehicleFactory, f), mapVehicleFactory)) return false;
  std::snprintf(text, n, "Fahrzeugfabrik %d: %d %s fehlen, mit %d Fabrik(en) %d Ticks, mit einer mehr %d (Erz-Grenze %d)",
                f + 1, stueck, name(fz), f, (int)jetzt, (int)dann, (int)tErz);
  return true;
}

// Kann gebaut UND betrieben werden? Liefert 0 = ja, sonst Grund (Text in `grund`).
// Arbeiter: frei, oder stilllegbar bei einem unwichtigeren Gebaeude (arbeit() macht das, sobald es steht).
inline int baubar(const Zustand& z, const Lage& l, int typ, bool darfVorgemerktes, char* grund, int n) {
  if (!kitMoeglich(z, typ)) { std::snprintf(grund, n, "Technik %d fehlt", techFuer(typ)); return 1; }
  UnitInfo info((map_id)typ);
  const int erz = info.GetOreCost(z.spieler), sel = info.GetRareOreCost(z.spieler);
  // Wichtigere Vorhaben halten ihr Erz und ihre Arbeiter fest (Ansprueche); das Gebaeude bekommt nur den Rest.
  int r = rangNeu(z, l, typ, hatGeb(z, l, typ));
  if (r < 0) r = 0;
  const int frei = (darfVorgemerktes ? l.erz : l.erz - l.vorErz) - beansprucht(l, M_ERZ, r, typ);
  const int freiSel = (darfVorgemerktes ? l.selten : l.selten - l.vorSelten) - beansprucht(l, M_SELTEN, r, typ);
  // Seltenerz nur pruefen, wenn das Gebaeude welches kostet (Kampf 14: Vormerkung 500 > Vorrat 0 ergab "frei -500"
  // und blockierte Wohnhaeuser ohne Seltenerz-Kosten)
  if (erz > frei || (sel > 0 && sel > freiSel)) {
    std::snprintf(grund, n, "Erz %d/%d, Seltenerz %d/%d (Rest nach %sAnspruechen ab Rang %d)", frei, erz, freiSel, sel, darfVorgemerktes ? "" : "Vormerkung und ", r);
    return 2;
  }
  if (standardVolk(l)) return 0;
  // Steht schon eins dieses Typs ohne Personal da, bringt ein weiteres nichts (Wohnraum-Schleife, Lauf 15)
  if (typ < 128 && l.unbesetzt[typ] > 0) { std::snprintf(grund, n, "schon %d %s ohne Personal", l.unbesetzt[typ], name(typ)); return 4; }
  // Arbeiter: frei + stilllegbar (unwichtigere Gebaeude) - was gleich wichtige oder wichtigere Ansprueche brauchen
  const int arb = info.GetWorkersRequired(z.spieler);
  const int still = l.arbStilllegbar[r / 10 > 10 ? 10 : r / 10];
  const int belegt = beansprucht(l, M_ARB, r, typ);
  const int verfuegbar = l.arbFrei + still - belegt;
  if (arb > verfuegbar) {
    std::snprintf(grund, n, "fehlen %d Arbeiter (frei %d, stilllegbar %d, beansprucht ab Rang %d: %d)", arb - verfuegbar,
                  l.arbFrei, still, r, belegt);
    return 3;
  }
  return 0;
}

// Vorhaben merken: das Gebaeude waere jetzt dran, es fehlen nur Erz oder Arbeiter. Bis zur naechsten Bauwahl haelt es seinen
// Bedarf als Anspruch fest -- unwichtigere Bauten und die Uni-Ausbildung nehmen ihm nichts weg. Keep a blocked plan's needs.
inline void vorhabenMerken(const Zustand& z, const Lage& l, int typ) {
  const int sp = z.spieler & 7;
  UnitInfo info((map_id)typ);
  Anspruch a; a.typ = typ; a.vorhaben = true; a.rang = rangNeu(z, l, typ, hatGeb(z, l, typ));
  a.menge[M_ARB] = info.GetWorkersRequired(z.spieler); a.menge[M_WISS] = info.GetScientistsRequired(z.spieler);
  a.menge[M_ERZ] = info.GetOreCost(z.spieler); a.menge[M_SELTEN] = info.GetRareOreCost(z.spieler);
  int k = -1;
  for (int i = 0; i < g_nVorhaben[sp]; ++i) if (g_vorhaben[sp][i].typ == typ) return;   // schon gemerkt
  if (g_nVorhaben[sp] < MAX_VORHABEN) k = g_nVorhaben[sp]++;
  else for (int i = 0; i < MAX_VORHABEN; ++i) if (g_vorhaben[sp][i].rang < a.rang && (k < 0 || g_vorhaben[sp][i].rang < g_vorhaben[sp][k].rang)) k = i;
  if (k >= 0) g_vorhaben[sp][k] = a;
}

inline bool waehle(const Zustand& z, const Lage& l, int typ, int bereich, bool vorgemerkt, Wahl& w, const char* fmt, ...) {
  char grund[160];
  const int nicht = baubar(z, l, typ, vorgemerkt, grund, sizeof(grund));
  if (nicht == 2 || nicht == 3) vorhabenMerken(z, l, typ);
  if (nicht != 0) {
    // Den Grund des wichtigsten Kandidaten merken, auch wenn er nicht baubar ist (fuer die Spur).
    if (!w.warum[0]) std::snprintf(w.warum, sizeof(w.warum), "%s %s wartet: %s", BEREICH[bereich], name(typ), grund);
    return false;
  }
  va_list ap; va_start(ap, fmt);
  char text[160];
  std::vsnprintf(text, sizeof(text), fmt, ap);
  va_end(ap);
  w.typ = typ; w.bereich = bereich;
  std::snprintf(w.warum, sizeof(w.warum), "%s: %s", BEREICH[bereich], text);
  return true;
}

inline int moralZahl(const Zustand& z);

// Welches Wohnhaus: mit Erz im Ueberfluss das groesste (spart Bauzeit und Fahrten der ConVecs), sonst das billigste je Platz.
// Which residence: the largest when ore is plentiful (saves build time), else the cheapest per place.
inline int wohnTyp(const Zustand& z, const Lage& l) {
  int beste = mapResidence;
  UnitInfo basis(mapResidence);
  int besteKap = basis.GetProductionCapacity(z.spieler);
  double besteJePlatz = besteKap > 0 ? (double)basis.GetOreCost(z.spieler) / besteKap : 1e9;
  static const int GROESSER[] = { mapReinforcedResidence, mapAdvancedResidence };
  for (int t : GROESSER) {
    if (!hatTech(z, techFuer(t))) continue;
    UnitInfo i((map_id)t);
    const int kap = i.GetProductionCapacity(z.spieler), erz = i.GetOreCost(z.spieler);
    if (kap <= 0) continue;
    const double jePlatz = (double)erz / kap;
    if (l.erz - l.vorErz >= 2 * erz ? kap > besteKap : jePlatz < besteJePlatz) { beste = t; besteKap = kap; besteJePlatz = jePlatz; }
  }
  return beste;
}

// Kandidaten eines Bereichs in Reihenfolge ihres Nutzens; true, sobald einer baubar ist.
inline bool bereichWahl(const Zustand& z, const Lage& l, int b, Wahl& w) {
  const int bew = l.menschen;
  const bool volk = standardVolk(l);
  if (b == B_KOLONIE) {
    // Fehlt Kindergarten oder Uni die Technik und gibt es noch kein Labor, ist das Labor der erste
    // Kolonie-Schritt -- noch vor Wohnraum (Hinweis des Spielers: "hier waere Labor vor Wohnraum").
    // If nursery/university lack their tech and there is no lab yet, the lab comes first -- before housing.
    const bool wachstumBrauchtLabor = (!hatGeb(z, l, mapNursery) && !kitMoeglich(z, mapNursery)) ||
                                      (!hatGeb(z, l, mapUniversity) && !kitMoeglich(z, mapUniversity));
    if (wachstumBrauchtLabor && !hatGeb(z, l, mapStandardLab) && !hatGeb(z, l, mapBasicLab) &&
        waehle(z, l, mapStandardLab, b, false, w, "Labor zuerst -- Kindergarten/Uni brauchen erst Forschung")) return true;
    // Kindergarten vor Uni (ohne ihn keine Geburten), dann nach groesster Luecke: Wohnraum, Medizin,
    // Freizeit, Forum. Abdeckung = Kapazitaet laufender Gebaeude / Bewohner (wie die %-Anzeige im Spiel).
    // Nursery before university, then by largest gap: housing, medicine, recreation, forum (coverage %).
    if (!hatGeb(z, l, mapNursery) && waehle(z, l, mapNursery, b, false, w, "Kindergarten -- ohne ihn keine neuen Kinder")) return true;
    if (!hatGeb(z, l, mapUniversity) && waehle(z, l, mapUniversity, b, false, w, "Universitaet -- bildet Arbeiter und Wissenschaftler aus")) return true;
    if (volk) return false;
    struct K { int typ, wert; const char* text; };
    K k[6]; int nk = 0;
    const int med = bew > 0 ? l.medKap * 100 / bew : 100;
    const int frz = bew > 0 ? l.freizeitKap * 100 / bew : 100;
    const int frm = bew > 0 ? l.forumKap * 100 / bew : 100;
    bool wohnUnterwegs = false;
    static const int HAEUSER[] = { mapResidence, mapReinforcedResidence, mapAdvancedResidence };
    for (int t : HAEUSER)
      if (l.imBau[t] + geplant(z, t) + l.kitsImSpiel[t] > 0) wohnUnterwegs = true;
    const int wohnArt = wohnTyp(z, l);
    // Wohnraum mit 10 Plaetzen Luft; ueberzaehlige Haeuser legt die Arbeiter-Rangliste still (siehe rang()).
    // Wohnraum-Puffer: 10 Plaetze, beim Bevoelkerungsziel so viel, wie die Kolonie gerade waechst (ein Haus mehr)
    const int puffer = z.ziel == 2 && !z.zielErreicht ? 25 : 10;
    if (bew + puffer > l.wohnraum && !wohnUnterwegs) k[nk++] = { wohnArt, 60 + bew - l.wohnraum, "Wohnraum" };
    if (med < 100 && l.imBau[mapMedicalCenter] == 0) k[nk++] = { mapMedicalCenter, 100 - med, "Medizin" };
    // Freizeitzentrum nach Luecke, wie Medizin (Spieler: "Nimm die Freizeitanlagen mit rein")
    if (frz < 100 && l.imBau[mapRecreationFacility] == 0) k[nk++] = { mapRecreationFacility, 100 - frz, "Freizeit" };
    // Forum nur, wenn seine Technik fuer diese Kolonie erreichbar ist (Eden: nie -- Duell 8 wartete darauf)
    const tb::Tech* forumTech = tb::tech(techFuer(mapForum));
    const bool forumGeht = !forumTech || eigeneKostenT(z, *forumTech) >= 0;
    if (forumGeht && frm < 100 && bew > 60 && l.imBau[mapForum] == 0) k[nk++] = { mapForum, (100 - frm) / 2, "Forum" };
    // GORF und DIRT: erforscht, aber nie gebaut (2026-11-07). Gewicht aus der Moral: laeuft sie gut, wartet das Gebaeude.
    {
      // nach Auslastung (Spieler): GORF genau eins in Betrieb, DIRT bis die angeschlossenen Gebaeude abgedeckt sind
      if (hatTech(z, techFuer(mapGORF)) && !l.gorfLaeuft && !hatGeb(z, l, mapGORF) && !l.imBau[mapGORF]) k[nk++] = { mapGORF, 60, "GORF (keins in Betrieb)" };
      if (hatTech(z, techFuer(mapDIRT)) && l.dirtAbdeckung < 100 && !l.imBau[mapDIRT] && geplant(z, mapDIRT) + l.kitsImSpiel[mapDIRT] == 0)
        k[nk++] = { mapDIRT, 100 - l.dirtAbdeckung, "DIRT-Abdeckung" };
    }
    for (int i = 0; i < nk; ++i) for (int j = i + 1; j < nk; ++j) if (k[j].wert > k[i].wert) { K t = k[i]; k[i] = k[j]; k[j] = t; }
    for (int i = 0; i < nk; ++i) {
      // Medizin/Freizeit: neues Gebaeude nur bei grosser Luecke oder freien Arbeitern -- sonst lieber die
      // Upgrade-Forschung (mehr Kapazitaet ohne zusaetzliches Personal, siehe forschung()).
      // Kleine Luecken (unter 20 %) schliesst die Upgrade-Forschung -- kein zweites Medical Center bei 98 %
      // (Lauf 17). Small gaps are left to the upgrade research.
      if (k[i].typ != wohnArt && (k[i].wert < 20 || (k[i].wert < 30 && l.arbFrei < 2))) continue;
      if (waehle(z, l, k[i].typ, b, false, w, "%s (Wohnraum %d, Medizin %d %%, Freizeit %d %%, Forum %d %% bei %d Bewohnern)",
                 k[i].text, l.wohnraum, med, frz, frm, bew)) return true;
    }
    return false;
  }
  if (b == B_ZIEL) {
    if (z.ziel != 1 || z.zielErreicht) return false;
    // Die Seltenerz-Schmelze ist kein fester Schritt: das Ziel erzeugt Seltenerz-Bedarf, und die Rohstoff-
    // Bilanz entscheidet, ob Seltenerz oder Erz gerade der groessere Engpass ist (Spieler: keine festen Regeln).
    static const int REIHE[] = { mapStandardLab, mapAdvancedLab, mapSpaceport };
    static const char* const WARUM[] = { "Labor fuer die Forschungskette", "Space Program braucht ein Advanced Lab", "Spaceport" };
    for (int i = 0; i < 3; ++i) {
      if (hatGeb(z, l, REIHE[i])) continue;
      if (waehle(z, l, REIHE[i], b, true, w, "%s", WARUM[i])) return true;
      if (kitMoeglich(z, REIHE[i])) return false;      // naechster Schritt baubar, aber noch nicht leistbar: warten
    }
    return false;
  }
  if (b == B_VERTEIDIGUNG) {
    const int fehlt = kampfFehlt(z, l);
    const int aussen = aussenTurmFehlt(z);
    if (aussen >= 0 && hatGeb(z, l, mapGuardPost) < MAX_WT + 2 &&
        waehle(z, l, mapGuardPost, b, z.bedrohung >= 50, w, "Wachturm fuer den Aussenposten bei %d,%d (Bedrohung %d)",
               z.aussenX[aussen], z.aussenY[aussen], z.bedrohung)) return true;
    if (fehlt <= 0) return false;
    // Lynx erforscht, aber keine Fahrzeugfabrik: die zuerst (Kampf 2: bei Welle 1 fehlte sie)
    if (hatTech(z, 3851) && !hatGeb(z, l, mapVehicleFactory) &&
        waehle(z, l, mapVehicleFactory, b, z.bedrohung >= 50, w, "Fahrzeugfabrik fuer Lynx (fehlt %d Kampfwert)", fehlt)) return true;
    // Weitere Fahrzeugfabrik, wenn die vorhandenen zu langsam nachruesten und Erz und Arbeiter sie tragen
    {
      char text[160];
      // Raumhafen fuer die EMP-Rakete: Technik da, Seltenerz-Mine laeuft, und die Abwehr des Feindes ist die Rakete wert
      int empWert = 0;
      if (empMoeglich(z) && !hatGeb(z, l, mapSpaceport) && l.geb[mapRareOreMine] > 0 && empLohnt(z, &empWert) &&
          waehle(z, l, mapSpaceport, b, false, w, "Raumhafen fuer EMP-Raketen (dichteste Feindabwehr Wert %d, Rakete kostet ~%d)",
                 empWert, empKostenWert(z))) return true;
      if (weitereFabrikLohnt(z, l, text, sizeof text) &&
          waehle(z, l, mapVehicleFactory, b, false, w, "%s", text)) return true;
    }
    if (hatGeb(z, l, mapGuardPost) >= MAX_WT) return false;
    // Weniger Tuerme (Spieler: "zu unflexibel"): Tuerme nur, solange noch kein Kampffahrzeug gebaut werden kann, oder
    // bis zum Turm-Anteil am Soll -- den Rest decken Fahrzeuge (fahrzeuge()).
    // Fewer towers: only while no combat vehicle can be built, or up to the tower share of the target.
    {
      const bool fahrzeugGeht = fahrgestellWahl(z, 1 << 20) && hatGeb(z, l, mapVehicleFactory);
      const int turmWert = z.eigeneWTWert + (geplant(z, mapGuardPost) + l.kitsImSpiel[mapGuardPost] + l.imBau[mapGuardPost]) * WERT_WT;
      if (fahrzeugGeht && turmWert >= z.sollArmee * z.turmAnteil) return false;
    }
    // Unter Druck darf die Verteidigung auch vorgemerktes Erz nehmen
    return waehle(z, l, mapGuardPost, b, z.bedrohung >= 50, w, "Wachturm (Feind Wert %d, eigener %d, Soll %d, fehlt %d, Bedrohung %d)",
                  z.feindArmee, z.eigeneArmee, z.sollArmee, fehlt, z.bedrohung);
  }
  // Wirtschaft
  // Labor, das die Forschung braucht (Duell 1: Lynx braucht ein Advanced Lab -- das kam nur fuer das Satellitenziel)
  if (z.laborWunsch && !hatGeb(z, l, z.laborWunsch) &&
      waehle(z, l, z.laborWunsch, b, false, w, "%s -- die Forschung Richtung %d braucht es (Dringlichkeit %d)",
             name(z.laborWunsch), z.laborFuer, z.laborWert)) return true;
  if (!hatGeb(z, l, mapVehicleFactory) && waehle(z, l, mapVehicleFactory, b, false, w, "Fahrzeuge nachbauen")) return true;
  // Garage: wenn die Ersparnis an den beschaedigten Fahrzeugen in Basisnaehe die Baukosten erreicht (Reparatur spart die
  // Haelfte des Neupreises je HP). Garage once the savings on damaged vehicles near base reach its cost.
  if (!hatGeb(z, l, mapGarage) && kitMoeglich(z, mapGarage)) {
    double ersparnis = 0;
    PlayerVehicleEnum e(z.spieler); UnitEx v;
    while (e.GetNext(v)) {
      if (!v.IsLive() || !kaempfer(v) || v.GetType() == mapScorpion) continue;
      const LOCATION lo = v.Location();
      if (abstand2(edX(z, lo), edY(z, lo), z.bx, z.by) > 35 * 35) continue;
      UnitInfo c((map_id)v.GetType()), wf((map_id)v.GetWeapon()); const int hp = c.GetHitPoints(z.spieler);
      if (hp > 0) ersparnis += 0.5 * (c.GetOreCost(z.spieler) + wf.GetOreCost(z.spieler)) * v.GetDamage() / hp;
    }
    UnitInfo gi(mapGarage);
    if (ersparnis >= gi.GetOreCost(z.spieler) && waehle(z, l, mapGarage, b, false, w, "Garage -- beschaedigte Fahrzeuge, Ersparnis %d Erz", (int)ersparnis)) return true;
  }
  // Robot Command Center: schaltet die bessere Wegfindung frei (Spielerfeld 0x1B8; gemessen auf on4_03: Fahrten 3-6 %
  // schneller). Lohnt, wenn der Zeitgewinn der Lastwagen die Baukosten noch einbringt. RCC: better pathfinding, worth it
  // once ore income pays the cost back in the remaining game.
  if (!hatGeb(z, l, mapRobotCommand) && erzRouten(z) > 0 && hatGeb(z, l, mapVehicleFactory) && kitMoeglich(z, mapRobotCommand)) {
    const Bilanz blr = bilanz(z, l);
    double fluss = erzRouten(z) * (double)blr.routeErz * 1000.0 / PAR(z).horizont;      // Erz je 1000 Ticks
    if (z.zuflussErz > fluss) fluss = z.zuflussErz;
    UnitInfo ri(mapRobotCommand);
    const double gewinnJeTick = 0.03 * fluss / 1000;
    const int tr = TethysGame::Tick(), rest = WERTUNG_TICK - tr > 40000 ? WERTUNG_TICK - tr : 40000;
    if (gewinnJeTick > 0 && ri.GetOreCost(z.spieler) / gewinnJeTick < rest &&
        waehle(z, l, mapRobotCommand, b, false, w, "Robot Command Center -- bessere Wegfindung, bringt die Kosten in %d Ticks ein", (int)(ri.GetOreCost(z.spieler) / gewinnJeTick))) return true;
  }
  // Erz oder Seltenerz: aus der Rohstoff-Bilanz, ohne feste Reihenfolge. Nur, was sich im Horizont bezahlt.
  // Ore or rare ore from the resource balance; only what pays for itself within the horizon.
  {
    const Bilanz bl = bilanz(z, l);
    const ErzWahl ew = erzWahl(z, l, bl);
    if (ew.typ != mapNone && ew.nutzen >= PAR(z).schmelzeNutzen &&
        waehle(z, l, ew.typ, b, (ew.typ == mapRareOreSmelter && z.ziel == 1 && !z.zielErreicht), w,
               "%s -- Bilanz: Erz fehlt %d, Seltenerz fehlt %d (Treiber %s), Nutzen %.1f (Route bringt %d bzw. %d im Horizont)",
               name(ew.typ), bl.engpassErz, bl.engpassSelten, bl.treiber, ew.nutzen, bl.routeErz, bl.routeSelten)) return true;
  }
  // Lager nur, wenn ein vorgemerktes Vorhaben mehr Erz braucht, als ins Lager passt -- ein volles Lager
  // ist kein Grund (Spieler: "Ein volles Lager ist nicht schlimm", jedes Lager kostet einen Arbeiter).
  // Storage only if a reserved project costs more than the store holds -- a full store is no reason.
  const int lager = hatGeb(z, l, mapCommonStorage);
  if (lager < 2 && l.erzMax > 0 && l.vorErz > l.erzMax && l.imBau[mapCommonStorage] == 0 &&
      waehle(z, l, mapCommonStorage, b, false, w, "%s braucht %d Erz, Lager fasst %d (Lager %d von max. 2)",
             l.vorFuer, l.vorErz, l.erzMax, lager)) return true;
  return false;
}

// ---------------------------------------------------------------------------------------------------
// Dringlichkeit je Bereich (0..100), nur aus Messwerten -- Grundlage fuer Bau- und Forschungskuchen
// (Spieler: keine festen Regeln, alles aus der Situation).
//  Kolonie: groesste Luecke aus Wachstum (Kindergarten, Uni), Wohnraum-Auslastung (ab 85 % steigend),
//           Medizin-Abdeckung und Moral (100 - Moralwert).
//  Wirtschaft: Engpass der Rohstoff-Bilanz in Prozent des Bedarfs, fehlende Fahrzeugfabrik bei
//           Fahrzeugbedarf, zu kleines Lager fuer ein Vorhaben.
//  Verteidigung: Bedrohung x Anteil des fehlenden Kampfwerts am Soll; ungeschuetzter Aussenposten.
//  Ziel: was uebrig bleibt, wenn die anderen nicht draengen -- kein Zeitdruck (Spieler).
// Urgency per area from measurements only; the goal gets what is left.
inline int moralZahl(const Zustand& z) {
  if (isInited && playerArray) {
    const int* r = reinterpret_cast<const int*>(reinterpret_cast<const char*>(playerArray) + z.spieler * 3108);
    if (r[16] >= 0 && r[16] <= 100) return r[16];               // genauer Moralwert (gemessen, Index 16)
  }
  return 100 - 25 * (int)Player[z.spieler].MoraleLevel();
}

struct Dringlichkeit { int kolonie, ziel, wirtschaft, verteidigung, kampfForschung; const char* kolonieGrund; const char* wirtschaftGrund; };
inline int fahrzeugBedarf(const Zustand& z, const Lage& l, const char** warum);   // weiter unten

inline Dringlichkeit dringlichkeit(const Zustand& z, const Lage& l) {
  Dringlichkeit d = { 0, 0, 0, 0, 0, "-", "-" };
  auto grenze = [](int v) { return v < 0 ? 0 : v > 100 ? 100 : v; };
  auto kol = [&](int v, const char* g) { v = grenze(v); if (v > d.kolonie) { d.kolonie = v; d.kolonieGrund = g; } };
  auto wir = [&](int v, const char* g) { v = grenze(v); if (v > d.wirtschaft) { d.wirtschaft = v; d.wirtschaftGrund = g; } };
  const int bew = l.menschen > 0 ? l.menschen : 1;
  if (!hatGeb(z, l, mapNursery)) kol(100, "kein Kindergarten -- keine Geburten");
  if (!hatGeb(z, l, mapUniversity)) kol(Player[z.spieler].Kids() > 0 ? 80 : 40, "keine Universitaet -- Kinder werden nicht ausgebildet");
  if (!standardVolk(l)) {
    if (l.wohnraum <= 0) kol(100, "kein Wohnraum");
    else { const int g = (int)PAR(z).wohnraumGrenze; kol((l.menschen * 100 / l.wohnraum - g) * 100 / (100 - g > 1 ? 100 - g : 1), "Wohnraum-Auslastung"); }
    kol(100 - l.medKap * 100 / bew, "Medizin-Abdeckung");
    if (kitMoeglich(z, mapRecreationFacility) || hatGeb(z, l, mapRecreationFacility))
      kol(100 - l.freizeitKap * 100 / bew, "Freizeit-Abdeckung");
    kol(100 - moralZahl(z), "Moral");
  }
  {
    // Ein Engpass zaehlt nur, wenn es eine Massnahme dagegen gibt (Schmelze, Seltenerz-Schmelze, neue Mine) --
    // sonst haette die Wirtschaft Dringlichkeit, ohne etwas tun zu koennen, und das Ziel bekaeme nie etwas.
    // A shortage only counts if something can be done about it.
    const Bilanz bl = bilanz(z, l);
    const ErzWahl ew = erzWahl(z, l, bl);
    // ... und nur, wenn sich die Massnahme im Horizont bezahlt (dasselbe Kriterium wie beim Bauen)
    if (ew.nutzen >= PAR(z).schmelzeNutzen && ew.typ == mapCommonOreSmelter && bl.bedarfErz > 0) wir(bl.engpassErz * 100 / bl.bedarfErz, "Erz-Engpass, Schmelze lohnt sich");
    if (ew.nutzen >= PAR(z).schmelzeNutzen && ew.typ == mapRareOreSmelter && bl.bedarfSelten > 0) wir(bl.engpassSelten * 100 / bl.bedarfSelten, "Seltenerz-Engpass, Seltenerz-Schmelze lohnt sich");
    // Neue Mine: ab PAR schmelzenJeMine Schmelzen je Mine (Test: die unterlegene Seite blieb bei 1 Mine -- sie kam nie
    // auf 2 Schmelzen). New mine once smelters per mine reach the parameter.
    if (l.geb[mapCommonOreMine] > 0 && l.geb[mapCommonOreSmelter] >= PAR(z).schmelzenJeMine * l.geb[mapCommonOreMine] && l.geb[mapCommonOreMine] < MAX_MINEN &&
        bl.engpassErz > 0 && bl.bedarfErz > 0 && l.vf.unitID)
      wir(bl.engpassErz * 100 / bl.bedarfErz / 2, "Erz-Engpass, Minen ausgelastet -- neue Mine");
  }
  if (!hatGeb(z, l, mapVehicleFactory) && kitMoeglich(z, mapVehicleFactory)) {
    const char* w = "";
    wir(fahrzeugBedarf(z, l, &w), "Fahrzeuge gebraucht, keine Fahrzeugfabrik");
  }
  if (l.erzMax > 0 && l.vorErz > l.erzMax) wir((l.vorErz - l.erzMax) * 100 / l.vorErz, "Lager zu klein fuer ein Vorhaben");
  if (z.laborWunsch && !hatGeb(z, l, z.laborWunsch) && kitMoeglich(z, z.laborWunsch)) wir(z.laborWert, "Labor fehlt fuer die Forschung");
  {
    const int fehlt = kampfFehlt(z, l);
    if (fehlt > 0 && kitMoeglich(z, mapGuardPost)) {
      const int anteil = fehlt * 100 / (z.sollArmee > 0 ? z.sollArmee : fehlt);
      d.verteidigung = grenze(z.bedrohung * (anteil > 100 ? 100 : anteil) / 100);
    }
    if (aussenTurmFehlt(z) >= 0 && kitMoeglich(z, mapGuardPost) && z.bedrohung > d.verteidigung) d.verteidigung = z.bedrohung;
  }
  // Militaerforschung: Bedrohung -- oder Blockade: Feind bekannt, Kampfwert fehlt, aber weder Wachturm noch
  // Kampffahrzeug ist erforscht. Dann waechst sie mit der Luecke und mit der Naehe der naechsten Welle (gemessener
  // Abstand); vor der ersten Welle: voll, wenn der Feind Fahrzeugfabriken hat (er kann jederzeit kommen), sonst halb.
  // Military research: threat, or blockade (enemy known, value missing, nothing researched to build).
  d.kampfForschung = z.bedrohung;
  // Qualitaet (Spieler: "Wettruesten ... mit entsprechend besseren Waffen"): ist das staerkste gesehene Feindfahrzeug
  // besser als das beste, das wir bauen koennten, waechst die Militaerforschung mit der Luecke (gleich = 0,
  // doppelt so stark = 100). Daraus folgen Panther/Tiger-Forschung und damit die Seltenerz-Mine (seltenAktiv).
  // Quality gap: research grows with how much better the enemy's best vehicle is than ours.
  // Vorsorge: jede bekannte Feindkolonie kann eine Fahrzeugfabrik bauen -- ohne erforschtes Kampffahrzeug bekommt die
  // Militaerforschung einen Mindestanteil (Test: P1 forschte bis Tick 28000 nur Kolonie, der erste Angriff kam bei
  // 34000, Lynx war nicht erforscht -- 8 von 10 Kolonien verloren). Precaution: minimum share until a combat
  // vehicle is researched, once an enemy colony is known.
  if (z.feindGeb > 0 && !fahrgestellWahl(z, 1 << 20) && PAR(z).vorsorge > d.kampfForschung) d.kampfForschung = (int)PAR(z).vorsorge;
  {
    const int fz = fahrgestellWahl(z, 1 << 20);
    const int eigen = fz ? sdkWertT(fz, waffeWahl(z, fz, 1 << 20)) : 0;
    if (z.feindBestWert > eigen) {
      const int v = eigen > 0 ? (z.feindBestWert - eigen) * 100 / eigen : 100;
      if (v > d.kampfForschung) d.kampfForschung = v > 100 ? 100 : v;
    }
  }
  // Proaktiv aufruesten: Anteil des Feind-Potenzials an der Summe aus Potenzial und eigener Staerke -- bei Gleichstand
  // 50, je staerker der Feind werden kann, desto mehr. So hoert die Militaerforschung nicht auf, nur weil das Soll
  // gerade erfuellt ist. Proactive: research grows with enemy potential vs own strength.
  if (z.feindPotenzial > 0) {
    const int v = z.feindPotenzial * 100 / (z.feindPotenzial + z.eigeneArmee);
    if (v > d.kampfForschung) d.kampfForschung = v;
  }
  bool blockiert = false;
  {
    const int fehlt = kampfFehlt(z, l);
    if (fehlt > 0 && !kitMoeglich(z, mapGuardPost) && !fahrgestellWahl(z, 1 << 20)) {
      blockiert = true;
      const int luecke = grenze(fehlt * 100 / (z.sollArmee > 0 ? z.sollArmee : fehlt));
      // Hat der Feind Fabriken oder schon eine Armee, kann er jederzeit kommen
      const int naehe = (z.feindFabriken || z.feindArmee) ? 100 : 50;
      const int v = luecke * naehe / 100;
      if (v > d.kampfForschung) d.kampfForschung = v;
    }
  }
  if (z.ziel >= 1 && !z.zielErreicht) {
    int m = d.kolonie; if (d.wirtschaft > m) m = d.wirtschaft; if (d.verteidigung > m) m = d.verteidigung;
    if (blockiert && d.kampfForschung > m) m = d.kampfForschung;  // Blockade der Verteidigung geht dem Ziel vor
    const int rest = 100 - m;
    // Satellit: eigener Bereich. Bevoelkerung: der Rest geht an die Kolonie (Wachstum). Vernichtung: an Militaer und
    // Angriffstruppe. The leftover goes to the goal's own area.
    if (z.ziel == 1) d.ziel = rest;
    else if (z.ziel == 2) { if (rest > d.kolonie) { d.kolonie = rest; d.kolonieGrund = "Ziel Bevoelkerung"; } }
    else if (z.ziel == 3) { if (rest > d.kampfForschung) d.kampfForschung = rest; if (rest > d.verteidigung && z.feindGeb) d.verteidigung = rest / 2; }
  }
  return d;
}

// Bau-Kuchen: Bedarf je Bereich aus der Lage, normiert auf 100. Building pie.
inline void bauKuchen(const Zustand& z, const Lage& l, int* anteil) {
  // Anteile = Dringlichkeiten (siehe dringlichkeit()), normiert auf 100 -- keine Grundanteile mehr
  const Dringlichkeit d = dringlichkeit(z, l);
  int roh[B_ANZAHL] = { d.kolonie, d.ziel, d.wirtschaft, d.verteidigung };
  int summe = roh[0] + roh[1] + roh[2] + roh[3];
  for (int i = 0; i < B_ANZAHL; ++i) anteil[i] = summe > 0 ? roh[i] * 100 / summe : 0;
}

// Liefert die Wahl (typ = mapNone: nichts). Returns the choice.
// Liegt Forschung id auf dem Weg zu ziel (ziel selbst oder eine Voraussetzung davon)? / Is id on the path to ziel?
inline bool aufWegZu(int ziel, int id, int tiefe) {
  if (ziel == id) return true;
  const int zi = techIndex(ziel); if (zi < 0 || tiefe > 6) return false;
  TechInfo ti = Research::GetTechInfo(zi);
  for (int r = 0; r < ti.GetNumRequiredTechs(); ++r)
    if (aufWegZu(Research::GetTechInfo(ti.GetRequiredTechIndex(r)).GetTechID(), id, tiefe + 1)) return true;
  return false;
}

// Erwarteter Bedarf: die erste Fahrzeugfabrik wird gebraucht, sobald ihre Technik da ist und der Weg zum Lynx (3851 oder eine
// Voraussetzung) begonnen hat -- einmal begonnen, bleibt es so, auch wenn das Labor zwischendurch anderes erforscht.
// Expected need: the first vehicle factory, once its tech is there and the way to the Lynx has begun.
inline int g_waffeBegonnen[8];
inline bool fabrikErwartet(const Zustand& z, const Lage& l) {
  if (hatGeb(z, l, mapVehicleFactory) || l.imBau[mapVehicleFactory] || !kitMoeglich(z, mapVehicleFactory)) return false;
  int& begonnen = g_waffeBegonnen[z.spieler & 7];
  if (TethysGame::Tick() < 10) begonnen = 0;                 // neues Spiel
  if (!begonnen && hatTech(z, 3851)) begonnen = 1;
  static const int LABS[] = { mapStandardLab, mapAdvancedLab };
  for (int typ : LABS) {
    PlayerBuildingEnum e(z.spieler, (map_id)typ); UnitEx lab;
    while (!begonnen && e.GetNext(lab)) {
      if (!lab.IsLive()) continue;
      const int th = lab.GetLabCurrentTopic(); if (th <= 0) continue;
      const int ii = th < Research::GetTechCount() ? th : techIndex(th); if (ii < 0) continue;
      const int id = Research::GetTechInfo(ii).GetTechID();
      if (!hatTech(z, id) && aufWegZu(3851, id, 0)) begonnen = 1;
    }
  }
  return begonnen != 0;
}

// Erwartete Seltenerz-Schmelze: Technik da, eine Seltenerz-Bake bekannt, eine Fahrzeugfabrik laeuft und Fahrzeuge mit Seltenerz
// bringen genug mehr Kampfkraft je Erz (seltenGewinn). Expected rare-ore smelter.
inline bool seltenErwartet(const Zustand& z, const Lage& l) {
  if (hatGeb(z, l, mapRareOreSmelter) || l.imBau[mapRareOreSmelter] || !l.geb[mapVehicleFactory]) return false;
  if (!hatTech(z, TECH_SELTENERZ) || !kitMoeglich(z, mapRareOreSmelter) || !seltenGewollt(z)) return false;
  return seltenBakeDa(z);
}

inline Wahl bauWahl(const Zustand& z, const Lage& l) {
  Wahl w; w.typ = mapNone; w.bereich = -1; w.warum[0] = 0;
  g_nVorhaben[z.spieler & 7] = 0; g_vorhabenTick[z.spieler & 7] = TethysGame::Tick();   // die Vorhaben dieser Wahl ersetzen die alten
  auto notfall = [&](int typ, const char* fmt, int a, int b) {
    w.typ = typ; w.bereich = -1;
    char text[120]; std::snprintf(text, sizeof(text), fmt, a, b);
    std::snprintf(w.warum, sizeof(w.warum), "Notfall: %s", text);
  };
  // 1. Notfaelle
  if (!hatGeb(z, l, mapCommandCenter)) { notfall(mapCommandCenter, "keine Kommandozentrale", 0, 0); return w; }
  if (!hatGeb(z, l, mapStructureFactory)) { notfall(mapStructureFactory, "keine Strukturfabrik", 0, 0); return w; }
  if (!hatGeb(z, l, mapTokamak)) { notfall(mapTokamak, "kein Kraftwerk", 0, 0); return w; }
  if (!hatGeb(z, l, mapCommonOreSmelter)) { notfall(mapCommonOreSmelter, "keine Erzschmelze", 0, 0); return w; }
  if (!hatGeb(z, l, mapAgridome)) { notfall(mapAgridome, "keine Nahrung", 0, 0); return w; }
  const int tokamakUnterwegs = geplant(z, mapTokamak) + l.kitsImSpiel[mapTokamak] + l.imBau[mapTokamak];
  const int agriUnterwegs = geplant(z, mapAgridome) + l.kitsImSpiel[mapAgridome] + l.imBau[mapAgridome];
  // Strompuffer (Eden-Analyse 2026-11-07: 75-103 % Auslastung, Wohnhaeuser und Agrardome ohne Strom, Moral 80 -> 46; Plymouth
  // nutzte nur ~50 %). Reserve ein Fuenftel der Erzeugung, damit ein verlorenes Kraftwerk nicht sofort die Kolonie abschaltet.
  const int stromPuffer = 25 + ExtPlayer[z.spieler].GetAmountPowerGenerated() / 5;
  if (l.stromFrei < stromPuffer && tokamakUnterwegs == 0) { notfall(mapTokamak, "nur %d Strom frei (Puffer %d)", l.stromFrei, stromPuffer); return w; }
  const int reichweite = nahrungsReichweite(l);
  if ((reichweite < 3000 || l.nahrung < 300) && l.nahrungNetto <= 0 && agriUnterwegs == 0) {
    notfall(mapAgridome, "Nahrung %d, netto %+d", l.nahrung, l.nahrungNetto); return w;
  }
  // Fabrik frueh (Spieler 2026-11-08): sobald ihre Technik da ist und die Waffenforschung begonnen hat, die erste
  // Fahrzeugfabrik bauen -- nicht erst nach dem Lynx. Build the first vehicle factory once weapon research has begun.
  if (fabrikErwartet(z, l)) {
    if (waehle(z, l, mapVehicleFactory, B_VERTEIDIGUNG, z.bedrohung >= 50, w, "Fabrik frueh: Waffenforschung begonnen, Fahrzeugfabrik soll bereitstehen")) return w;
  }
  // Seltenerz-Schmelze, sobald Seltenerz-Fahrzeuge sich lohnen (Parameter seltenGewinn). Rare-ore smelter once it pays.
  if (seltenErwartet(z, l) &&
      waehle(z, l, mapRareOreSmelter, B_WIRTSCHAFT, false, w, "Seltenerz-Schmelze: Fahrzeuge mit Seltenerz bringen %d %% mehr Kampfkraft je Erz (Schwelle %d %%)",
             (int)(100 * seltenVorteil(z)), (int)(100 * PAR(z).seltenGewinn))) return w;
  // 2. Bau-Kuchen: Bereiche nach Anteil absteigend probieren
  int anteil[B_ANZAHL];
  bauKuchen(z, l, anteil);
  int reihe[B_ANZAHL] = { 0, 1, 2, 3 };
  for (int i = 0; i < B_ANZAHL; ++i)
    for (int j = i + 1; j < B_ANZAHL; ++j) if (anteil[reihe[j]] > anteil[reihe[i]]) { int t = reihe[i]; reihe[i] = reihe[j]; reihe[j] = t; }
  for (int k = 0; k < B_ANZAHL; ++k) {
    if (anteil[reihe[k]] == 0) break;
    if (bereichWahl(z, l, reihe[k], w)) {
      char text[200];
      std::snprintf(text, sizeof(text), "%s [Kuchen K%d Z%d W%d V%d]", w.warum, anteil[0], anteil[1], anteil[2], anteil[3]);
      std::snprintf(w.warum, sizeof(w.warum), "%s", text);
      return w;
    }
  }
  // 3. Lueckenfueller: nur mit Erz, das nicht vorgemerkt ist
  char grund[120];
  if (l.stromFrei < 150 && tokamakUnterwegs == 0 && baubar(z, l, mapTokamak, false, grund, sizeof(grund)) == 0) {
    w.typ = mapTokamak; w.bereich = -1;
    std::snprintf(w.warum, sizeof(w.warum), "Lueckenfueller: nur %d Strom frei", l.stromFrei); return w;
  }
  if (l.nahrungNetto < 0 && agriUnterwegs == 0 && baubar(z, l, mapAgridome, false, grund, sizeof(grund)) == 0) {
    w.typ = mapAgridome; w.bereich = -1;
    std::snprintf(w.warum, sizeof(w.warum), "Lueckenfueller: Nahrung netto %+d, reicht %d Ticks", l.nahrungNetto, reichweite); return w;
  }
  w.typ = mapNone;      // w.warum enthaelt ggf. den Grund des wichtigsten wartenden Kandidaten
  return w;
}

// Lage um Arbeiter-Rangliste und Vormerkungen ergaenzen (braucht rang(), darum nach erhebe()).
// Adds worker ranking and reservations to the situation.
inline void arbeitErheben(const Zustand& z, Lage& l) {
  for (int t : GEBAEUDE) {
    PlayerBuildingEnum e(z.spieler, (map_id)t);
    UnitEx u;
    const int r = rang(z, l, t);
    UnitInfo info((map_id)t);
    const int arb = info.GetWorkersRequired(z.spieler);
    const int wiss = info.GetScientistsRequired(z.spieler);
    if (arb <= 0 && wiss <= 0) continue;
    while (e.GetNext(u)) {
      // Im Bau: braucht sein Personal, sobald es steht (Anspruch). Under construction: claims its staff.
      if (u.GetCurAction() == moObjWait) { anspruchDazu(l, t, rangNeu(z, l, t, l.geb[t] - 1), arb, wiss, 0, 0); continue; }
      if (u.GetCurAction() != moDone || u.IsBusy()) continue;       // im Bau, stillgelegt oder arbeitet
      if (arb > 0 && !u.HasWorkers()) { l.arbFehlt += arb; if (t < 128) l.unbesetzt[t]++; anspruchDazu(l, t, r, arb, 0, 0, 0); continue; }
      if (wiss > 0 && !u.HasScientists()) { l.wissFehlt += wiss; if (t < 128) l.unbesetzt[t]++; anspruchDazu(l, t, r, 0, wiss, 0, 0); continue; }
      if (arb > 0) for (int i = 0; i <= 10; ++i) if (r < i * 10) l.arbStilllegbar[i] += arb;
    }
  }
  // Bausaetze unterwegs zum Bauplatz (schon bezahlt): Personal. Kits on the way: their staff.
  for (int i = 0; i < z.anzahlPlaetze; ++i) {
    const Platz& q = z.plaetze[i];
    if (q.zustand != P_FAHRT && q.zustand != P_BAUT) continue;
    UnitInfo info((map_id)q.typ);
    anspruchDazu(l, q.typ, rangNeu(z, l, q.typ, l.geb[q.typ]), info.GetWorkersRequired(z.spieler), info.GetScientistsRequired(z.spieler), 0, 0);
  }
  // Vorhaben der letzten Bauwahl und Weck-Wunsch (Fahrzeugfabrik, Strukturfabrik). Plans and wake wishes.
  const int sp = z.spieler & 7;
  if (TethysGame::Tick() - g_vorhabenTick[sp] < 6000)
    for (int i = 0; i < g_nVorhaben[sp]; ++i) {
      const Anspruch& a = g_vorhaben[sp][i];
      anspruchDazu(l, a.typ, a.rang, a.menge[M_ARB], a.menge[M_WISS], a.menge[M_ERZ], a.menge[M_SELTEN], true);
    }
  if (z.wunschArbeiter > 0 && TethysGame::Tick() - z.wunschSeit < 1000) anspruchDazu(l, -2, z.wunschRang, z.wunschArbeiter, 0, 0, 0);
  // Erwartete erste Fahrzeugfabrik: Arbeiter und Erz, sobald der Weg zum Lynx begonnen hat (sonst bildete die Uni die Arbeiter
  // aus, bevor die Bauwahl die Fabrik ueberhaupt pruefte -- Eden 2026-11-09: Uni 20 Ticks nach Forschungsbeginn, Fabrik erst 18000).
  // Expected first vehicle factory: its workers and ore are claimed as soon as the way to the Lynx has begun.
  if (fabrikErwartet(z, l)) {
    bool schon = false;
    if (TethysGame::Tick() - g_vorhabenTick[sp] < 6000) for (int i = 0; i < g_nVorhaben[sp]; ++i) if (g_vorhaben[sp][i].typ == mapVehicleFactory) schon = true;
    UnitInfo vi(mapVehicleFactory);
    if (!schon) anspruchDazu(l, mapVehicleFactory, rang(z, l, mapVehicleFactory), vi.GetWorkersRequired(z.spieler), 0, vi.GetOreCost(z.spieler), vi.GetRareOreCost(z.spieler), true);
  }
  // Erwartete Seltenerz-Schmelze: Arbeiter und Erz; solange das Erz fehlt, warten neue Kampffahrzeuge (Fabrikgruppe haelt ihr Soll).
  // Expected rare-ore smelter: its workers and ore; while ore is short, new combat vehicles wait.
  if (seltenErwartet(z, l)) {
    bool schon = false;
    if (TethysGame::Tick() - g_vorhabenTick[sp] < 6000) for (int i = 0; i < g_nVorhaben[sp]; ++i) if (g_vorhaben[sp][i].typ == mapRareOreSmelter) schon = true;
    UnitInfo si(mapRareOreSmelter);
    if (!schon) anspruchDazu(l, mapRareOreSmelter, rang(z, l, mapRareOreSmelter), si.GetWorkersRequired(z.spieler), 0, si.GetOreCost(z.spieler), si.GetRareOreCost(z.spieler), true);
    l.seltenSpart = l.erz < si.GetOreCost(z.spieler);
  }
  vormerkungen(z, l, l.vorErz, l.vorSelten, &l.vorFuer);
}

// Arbeiter-Rangliste / worker ranking (alle 200 Ticks, eine Aenderung je Runde):
// - Ein wichtiges Gebaeude steht fertig, aber ohne Arbeiter -> das unwichtigste laufende stilllegen (DoIdle).
// - Sonst, wenn Arbeiter frei sind -> das wichtigste stillgelegte wieder einschalten (DoUnIdle).
// Gemessen (Messlauf): stillgelegt = Aktion 0, beschaeftigt, ohne Strom und Arbeiter; die BuildingGroup
// schaltet es nicht von sich aus wieder ein. Laufende Labore (Aktion 12) und Fabriken (7) bleiben unberuehrt.
// Idle the least important running building for a more important unstaffed one; unidle when workers free up.
inline int themaGruppe(int techID);
inline int techIndex(int techID);
inline void arbeit(Zustand& z, const Lage& l) {
  const int t = TethysGame::Tick();
  if (t < z.naechsteArbeit || standardVolk(l)) return;
  z.naechsteArbeit = t + 200;
  // Liste bereinigen: zerstoerte oder schon wieder laufende Gebaeude entfernen
  int k = 0;
  for (int i = 0; i < z.anzahlGeparkt; ++i) {
    UnitEx u; u.unitID = z.geparkt[i];
    if (u.IsLive() && u.IsBusy() && u.GetCurAction() == moDone) { z.geparktGrund[k] = z.geparktGrund[i]; z.geparkt[k++] = z.geparkt[i]; }
  }
  z.anzahlGeparkt = k;
  // Haengende Stilllegungen: ein Gebaeude steht stillgelegt da (Aktion 0, beschaeftigt, ohne Strom und
  // Personal), ohne dass die KI es geparkt hat -- z.B. weil DoUnIdle nicht gegriffen hat (Lauf 21: die
  // Fahrzeugfabrik blieb nach DoUnIdle aus, der Robo-Miner fuer Seltenerz kam nie). Erneut einschalten.
  // Stuck idle: re-issue DoUnIdle for buildings idle without being parked by the AI.
  for (int typ : GEBAEUDE) {
    // Nur Gebaeude, die Strom UND Personal brauchen -- Minen, Kraftwerke, Lager sehen im Betrieb sonst
    // genauso aus (Lauf 22: Erzmine faelschlich "eingeschaltet"). Only buildings needing power and staff.
    UnitInfo info((map_id)typ);
    if (info.GetPowerRequired(z.spieler) <= 0 || (info.GetWorkersRequired(z.spieler) <= 0 && info.GetScientistsRequired(z.spieler) <= 0)) continue;
    PlayerBuildingEnum e(z.spieler, (map_id)typ);
    UnitEx u;
    while (e.GetNext(u)) {
      if (u.GetCurAction() != moDone || !u.IsBusy() || u.HasPower() || u.HasWorkers()) continue;
      if (u.unitID == z.fabrikStill) continue;
      bool weitereStill = false;
      for (int i = 0; i < MAX_VF; ++i) if (z.fabrikStillW[i] == u.unitID) weitereStill = true;
      if (weitereStill) continue;
      bool geparktHier = false;
      for (int i = 0; i < z.anzahlGeparkt; ++i) if (z.geparkt[i] == u.unitID) geparktHier = true;
      if (geparktHier) continue;
      if (l.arbFrei - beansprucht(l, M_ARB, rang(z, l, typ) + 1) < info.GetWorkersRequired(z.spieler)) continue;  // DoUnIdle wirkt nur mit genug Arbeitern
      u.DoUnIdle();
      melde(z, "arbeit", typ, edX(z, u.Location()), edY(z, u.Location()), "hing stillgelegt (nicht geparkt) -- erneut eingeschaltet");
      return;
    }
  }
  // Strommangel: ein wichtiges Gebaeude steht ohne Strom, ein unwichtigeres laeuft -> das unwichtigere abschalten, damit der Strom
  // dorthin geht (Eden: Wohnhaeuser 46 %, Agrardome 56 % laufend, Wachtuerme 84 %). Power shortage: idle a lesser consumer.
  if (z.anzahlGeparkt < 16) {
    UnitEx ohne, weg; int rOhne = -1, rWeg = 1000;
    for (int typ : GEBAEUDE) {
      UnitInfo info((map_id)typ);
      if (info.GetPowerRequired(z.spieler) <= 0 || typ == mapCommandCenter) continue;
      const int r = rang(z, l, typ);
      PlayerBuildingEnum e(z.spieler, (map_id)typ);
      UnitEx u;
      while (e.GetNext(u)) {
        if (!u.IsLive() || u.GetCurAction() != moDone || u.IsBusy()) continue;
        if (!u.HasPower()) { if (r > rOhne) { rOhne = r; ohne = u; } }
        else if (r < rWeg && typ != mapGuardPost) { rWeg = r; weg = u; }   // Tuerme halten die Kolonie, sie bleiben an
      }
    }
    if (ohne.unitID && weg.unitID && rWeg + 5 < rOhne) {
      weg.DoIdle();
      z.geparktGrund[z.anzahlGeparkt] = 0;
      z.geparkt[z.anzahlGeparkt++] = weg.unitID;
      melde(z, "arbeit", weg.GetType(), edX(z, weg.Location()), edY(z, weg.Location()), "stillgelegt (Rang %d) -- Strom fuer %s (Rang %d)", rWeg, name(ohne.GetType()), rOhne);
      return;
    }
  }
  // Wichtigstes fertiges Gebaeude ohne Arbeiter, und unwichtigstes laufendes mit Arbeitern
  UnitEx fehlt, opfer, fehltWiss; int rFehlt = -1, rOpfer = 1000, rFehltWiss = -1;
  for (int typ : GEBAEUDE) {
    UnitInfo info((map_id)typ);
    const int arbN = info.GetWorkersRequired(z.spieler), wissN = info.GetScientistsRequired(z.spieler);
    if (arbN <= 0 && wissN <= 0) continue;
    const int r = rang(z, l, typ);
    PlayerBuildingEnum e(z.spieler, (map_id)typ);
    UnitEx u;
    while (e.GetNext(u)) {
      if (u.GetCurAction() != moDone || u.IsBusy()) continue;          // im Bau, stillgelegt oder arbeitet
      if (arbN > 0 && !u.HasWorkers()) { if (r > rFehlt) { rFehlt = r; fehlt = u; } }
      else if (wissN > 0 && !u.HasScientists()) { if (r > rFehltWiss) { rFehltWiss = r; fehltWiss = u; } }
      else if (arbN > 0 && r < rOpfer && typ != mapCommandCenter) { rOpfer = r; opfer = u; }
    }
  }
  // Gebaeude ohne Wissenschaftler (Kindergarten, Medizin, Uni ...): die unwichtigste laufende Forschung gibt
  // welche ab (Personal geht vor Forschung). Buildings lacking scientists take them from the least important research.
  if (fehltWiss.unitID && !fehlt.unitID) {
    UnitEx quelle; int gQ = 99, aQ = 0;
    const int labTypen[] = { mapBasicLab, mapStandardLab, mapAdvancedLab };
    for (int lt : labTypen) {
      PlayerBuildingEnum e(z.spieler, (map_id)lt);
      UnitEx lab;
      while (e.GetNext(lab)) {
      if (!lab.IsLive()) continue;   // gerade zerstoert (Duell 19: 85x Absturz 00472D78 in forschung)
        const int th = lab.GetLabCurrentTopic();
        const int anz = lab.GetLabScientistCount();
        if (th <= 0 || anz <= 1) continue;
        const int i2 = th < Research::GetTechCount() ? th : techIndex(th);
        const int g = i2 >= 0 ? themaGruppe(Research::GetTechInfo(i2).GetTechID()) : 0;
        if (g < gQ || (g == gQ && anz > aQ)) { gQ = g; aQ = anz; quelle = lab; }
      }
    }
    if (quelle.unitID) {
      UnitInfo info((map_id)fehltWiss.GetType());
      int ab = info.GetScientistsRequired(z.spieler);
      if (ab > aQ - 1) ab = aQ - 1;
      quelle.SetLabScientistCount(aQ - ab);
      ExtPlayer[z.spieler].RecalculateValues();
      melde(z, "arbeit", fehltWiss.GetType(), edX(z, fehltWiss.Location()), edY(z, fehltWiss.Location()),
            "ohne Wissenschaftler -- Forschung im Labor gibt %d ab (Personal vor Forschung)", ab);
      return;
    }
  }
  // Arbeiter-Wunsch (z.B. Fahrzeugfabrik fuer den Seltenerz-Miner) wie ein fehlendes Gebaeude behandeln
  const bool wunsch = z.wunschArbeiter > 0 && TethysGame::Tick() - z.wunschSeit < 1000 && l.arbFrei < z.wunschArbeiter;
  // Fuer Wuensche nur Unwichtiges stilllegen (Rang < 50: Lager, Ueberzaehliges) -- nie Wohnraum, Nahrung, Medizin
  if (!fehlt.unitID && wunsch && opfer.unitID && rOpfer < 50 && rOpfer + 5 < z.wunschRang && z.anzahlGeparkt < 16) {
    opfer.DoIdle();
    z.geparktGrund[z.anzahlGeparkt] = 0;
    z.geparkt[z.anzahlGeparkt++] = opfer.unitID;
    melde(z, "arbeit", opfer.GetType(), edX(z, opfer.Location()), edY(z, opfer.Location()),
          "stillgelegt (Rang %d), Arbeiter fuer einen Wunsch (Rang %d, %d Arbeiter)", rOpfer, z.wunschRang, z.wunschArbeiter);
    return;
  }
  if (fehlt.unitID && opfer.unitID && rOpfer + 5 < rFehlt && z.anzahlGeparkt < 16) {
    opfer.DoIdle();
    z.geparktGrund[z.anzahlGeparkt] = 0;
    z.geparkt[z.anzahlGeparkt++] = opfer.unitID;
    melde(z, "arbeit", opfer.GetType(), edX(z, opfer.Location()), edY(z, opfer.Location()),
          "stillgelegt (Rang %d), Arbeiter fuer %s (Rang %d)", rOpfer, name(fehlt.GetType()), rFehlt);
    return;
  }
  if (fehlt.unitID || wunsch || l.arbFrei <= 0 || !z.anzahlGeparkt) return;
  // Wichtigstes stillgelegtes, das mit den freien Arbeitern laufen kann, wieder einschalten
  int beste = -1, rBeste = -1;
  for (int i = 0; i < z.anzahlGeparkt; ++i) {
    if (z.geparktGrund[i] != 0) continue;                              // Leerlauf: weckt der zustaendige Bereich
    UnitEx u; u.unitID = z.geparkt[i];
    UnitInfo info((map_id)u.GetType());
    const int r = rang(z, l, u.GetType());
    if (info.GetPowerRequired(z.spieler) > l.stromFrei) continue;   // erst, wenn Strom da ist
    if (info.GetWorkersRequired(z.spieler) <= l.arbFrei - beansprucht(l, M_ARB, r + 1) && r > rBeste) { rBeste = r; beste = i; }
  }
  if (beste < 0) return;
  UnitEx u; u.unitID = z.geparkt[beste];
  u.DoUnIdle();
  melde(z, "arbeit", u.GetType(), edX(z, u.Location()), edY(z, u.Location()), "wieder eingeschaltet (Rang %d, %d Arbeiter frei)", rBeste, l.arbFrei);
  --z.anzahlGeparkt;
  z.geparkt[beste] = z.geparkt[z.anzahlGeparkt]; z.geparktGrund[beste] = z.geparktGrund[z.anzahlGeparkt];
}

// Leerlauf / idle: Gebaeude, die laengere Zeit nichts zu tun haben, bei Personalmangel stilllegen (Spieler:
// "Ein Gebaeude, das langfristig nichts zu tun hat, kann stillgelegt werden"). Labore ohne Thema,
// Strukturfabrik ohne Auftrag, Spaceport ohne Auftrag. Geweckt werden sie vom zustaendigen Bereich
// (leerlaufWecken): Labor, wenn ein Thema da ist; Fabrik, wenn gebaut werden soll; Spaceport fuer das Ziel.
// Idle buildings with nothing to do for a long time when staff is short; the owning area wakes them.
constexpr int LEERLAUF_TICKS = 1500;

inline bool istGeparkt(const Zustand& z, int id) {
  for (int i = 0; i < z.anzahlGeparkt; ++i) if (z.geparkt[i] == id) return true;
  return false;
}

inline void leerlaufWecken(Zustand& z, int id, const char* warum) {
  for (int i = 0; i < z.anzahlGeparkt; ++i) {
    if (z.geparkt[i] != id) continue;
    UnitEx u; u.unitID = id;
    UnitInfo info((map_id)u.GetType());
    const int braucht = info.GetWorkersRequired(z.spieler);
    if (ExtPlayer[z.spieler].GetNumAvailableWorkers() < braucht) {
      // DoUnIdle wirkt nur mit genug freien Arbeitern -> Wunsch an die Rangliste, geparkt lassen
      z.wunschArbeiter = braucht; z.wunschRang = 70; z.wunschSeit = TethysGame::Tick();
      return;
    }
    u.DoUnIdle();
    melde(z, "arbeit", u.GetType(), edX(z, u.Location()), edY(z, u.Location()), "wieder eingeschaltet: %s", warum);
    // Leerlauf-Zaehler zuruecksetzen (sonst gilt es sofort wieder als "seit langem untaetig")
    for (int j = 0; j < z.anzahlLeer; ++j) if (z.leerId[j] == id) { z.leerId[j] = z.leerId[--z.anzahlLeer]; z.leerSeit[j] = z.leerSeit[z.anzahlLeer]; break; }
    --z.anzahlGeparkt;
    z.geparkt[i] = z.geparkt[z.anzahlGeparkt]; z.geparktGrund[i] = z.geparktGrund[z.anzahlGeparkt];
    return;
  }
}

inline void leerlauf(Zustand& z, const Lage& l) {
  if (standardVolk(l)) return;
  const int t = TethysGame::Tick();
  const bool mangel = l.arbFrei <= 0 || l.arbFehlt > 0 || l.wissFehlt > 0;
  // Untaetige Gebaeude erfassen
  int ids[16], seit[16], n = 0;
  auto merke = [&](UnitEx& u) {
    if (n >= 16) return;
    int s0 = t;
    for (int i = 0; i < z.anzahlLeer; ++i) if (z.leerId[i] == u.unitID) s0 = z.leerSeit[i];
    ids[n] = u.unitID; seit[n] = s0; n++;
  };
  const int typen[] = { mapBasicLab, mapStandardLab, mapAdvancedLab, mapStructureFactory, mapSpaceport };
  for (int typ : typen) {
    PlayerBuildingEnum e(z.spieler, (map_id)typ);
    UnitEx u;
    while (e.GetNext(u)) {
      if (u.GetCurAction() != moDone || u.IsBusy() || istGeparkt(z, u.unitID)) continue;   // arbeitet / im Bau / geparkt
      // Mit bekanntem Feind bleibt die Strukturfabrik an: Tuerme und Schmelzen muessen sofort nachgebaut werden
      // koennen (Kampf 8: Fabrik stillgelegt, Wecken scheiterte an Arbeitern, zweite Schmelze kam erst spaet)
      if (typ == mapStructureFactory && z.bedrohung > 0) continue;
      if (typ == mapStructureFactory) {
        bool offen = false;
        for (int i = 0; i < z.anzahlPlaetze; ++i) if (z.plaetze[i].zustand == P_FAHRT || z.plaetze[i].zustand == P_BAUT) offen = true;
        bool kits = false;
        for (int bay = 0; bay < 6; ++bay) { const int k = u.GetFactoryCargo(bay); if (k > 0 && k != mapNone) kits = true; }
        if (offen || kits) continue;
      }
      if (typ == mapSpaceport && z.ziel == 1 && !z.zielErreicht && hatTech(z, TECH_RAUMFAHRT)) continue;   // gleich gebraucht
      if (typ == mapSpaceport && !Player[z.spieler].IsEden() && hatTech(z, 12201)) continue;               // baut EMP-Raketen
      if ((typ == mapBasicLab || typ == mapStandardLab || typ == mapAdvancedLab) && u.GetLabCurrentTopic() > 0) continue;
      // Ein Labor, das die Forschung braucht, ist nicht "untaetig" -- es wartet auf Wissenschaftler (Duell 9: Advanced Lab
      // pendelte zwischen Stilllegen und Wecken)
      if ((typ == mapStandardLab || typ == mapAdvancedLab) && z.laborNutzen[typ == mapAdvancedLab ? 3 : 2] > 0) continue;
      merke(u);
    }
  }
  for (int i = 0; i < n; ++i) { z.leerId[i] = ids[i]; z.leerSeit[i] = seit[i]; }
  z.anzahlLeer = n;
  if (!mangel || z.anzahlGeparkt >= 16) return;
  for (int i = 0; i < n; ++i) {
    if (t - seit[i] < LEERLAUF_TICKS) continue;
    UnitEx u; u.unitID = ids[i];
    u.DoIdle();
    z.geparktGrund[z.anzahlGeparkt] = 1;
    z.geparkt[z.anzahlGeparkt++] = u.unitID;
    melde(z, "arbeit", u.GetType(), edX(z, u.Location()), edY(z, u.Location()), "Leerlauf seit %d Ticks bei Personalmangel -- stillgelegt", t - seit[i]);
    return;                                                     // eins je Runde
  }
}

inline bool kitGesperrt(const Zustand& z, int typ) {
  const int t = TethysGame::Tick();
  for (int i = 0; i < 8; ++i) if (z.kitGesperrtTyp[i] == typ && z.kitGesperrtBis[i] > t) return true;
  return false;
}
inline void kitSperren(Zustand& z, int typ, int dauer) {
  int frei = 0;
  for (int i = 0; i < 8; ++i) if (z.kitGesperrtBis[i] < z.kitGesperrtBis[frei]) frei = i;
  z.kitGesperrtTyp[frei] = typ;
  z.kitGesperrtBis[frei] = TethysGame::Tick() + dauer;
}

// ---------------------------------------------------------------------------------------------------
// Bauen ueber eine BuildingGroup der Spiel-KI / building via the game's own BuildingGroup
//
// Die KI entscheidet nur noch WAS (bedarf) und WO (platzWaehlen) gebaut wird. Den Rest macht die
// BuildingGroup der Engine: die Strukturfabrik der Gruppe stellt den Bausatz her, ein ConVec der Gruppe
// holt ihn und baut ihn an der aufgezeichneten Stelle (RecordBuilding mit der Gebaeudemitte, wie
// CreateUnit). Bausaetze, die ConVecs schon tragen (Startausruestung), verbaut die Gruppe direkt.
// Die Gruppe arbeitet der Reihe nach -- die KI zeichnet ein neues Gebaeude erst auf, wenn das vorige steht.
// The AI decides only WHAT and WHERE; the engine's BuildingGroup produces the kit in its structure
// factory, fetches it with a ConVec and builds it at the recorded location (building centre).

constexpr int GEDULD_GRUPPE = 6000;   // so lange darf ein aufgezeichnetes Gebaeude dauern, bevor die Spur mahnt

inline bool stehtDa(const Zustand& z, const Platz& p) {
  const Groesse g = grundflaeche(p.typ);
  MAP_RECT r(engine(z, p.x0, p.y0), engine(z, p.x0 + g.w - 1, p.y0 + g.h - 1));
  InRectEnumerator e(r);
  Unit u;
  while (e.GetNext(u)) if (u.IsBuilding() && u.GetType() == p.typ && u.OwnerID() == z.spieler) return true;
  return false;
}

// Fertig und ruhig? Ein TakeUnit auf ein Gebaeude im Bau (moDevelop) oder kurz davor macht es kaputt.
// Finished and idle? TakeUnit on a building under construction corrupts it.
inline bool bereit(UnitEx u) { return u.unitID && u.IsLive() && u.GetCurAction() == moDone && !u.IsBusy(); }

inline bool inGruppe(BuildingGroup& g, int id) {
  GroupEnumerator ge(g);
  Unit v;
  while (ge.GetNext(v)) if (v.unitID == id) return true;
  return false;
}

// Gebaeude in der Gruppe aufzeichnen (Platz waehlen, RecordBuilding). Liefert true bei Erfolg.
constexpr int SCHUSSFELD = 6;      // so weit muss es vor einem Wachturm frei von Gebaeuden sein


// Wachtuerme verteidigen die Kolonie als Ganzes (Spieler: "Die Verteidigung soll sich auf die Kolonie im
// Ganzen konzentrieren, weniger auf einzelne Gebaeude"): Ring um die Basis in 16 Richtungen, je 3 Kacheln
// vor dem aeussersten Gebaeude in dieser Richtung. Bevorzugt die Seite zum Feind, dann Richtungen, in denen
// noch kein Turm steht. Davor muss das Schussfeld frei sein (Laser schiesst nicht ueber Mauern/Gebaeude).
// Abseits stehende Minen (Aussenposten) bekommen ihren eigenen Turm, siehe aussenPlatz().
// Guard posts defend the colony as a whole: a ring 3 tiles beyond the outermost building per direction.
inline bool schussfeldFrei(const Rechteck* hin, int nh, int x, int y, double ux, double uy) {
  for (int d = 1; d <= SCHUSSFELD; ++d) {
    const int fx = (int)(x + ux * d + 0.5), fy = (int)(y + uy * d + 0.5);
    for (int h = 0; h < nh; ++h)
      if (fx >= hin[h].x0 && fx < hin[h].x0 + hin[h].w && fy >= hin[h].y0 && fy < hin[h].y0 + hin[h].h) return false;
  }
  return true;
}

inline int koloniePlatz(Zustand& z) {
  // Front statt Ring (Kampf 6: der gleichmaessige Ring verlor Turm um Turm einzeln). Die Tuerme stehen in einer
  // Linie quer zur Feindrichtung, 3 Kacheln vor dem aeussersten Gebaeude, je 2 Kacheln auseinander -- so
  // ueberlappen sich die Schussfelder (Laser-Reichweite 4). Ist die Linie voll, kommt eine zweite Reihe.
  // Weitere Seiten bekommen erst Tuerme, wenn von dort schon Feinde an die Kolonie kamen (angriffSektoren).
  // Front instead of ring: a line across the enemy direction, 2 tiles apart so fields of fire overlap.
  static Rechteck hin[160];
  const int nh = hindernisse(z, hin, 160);
  double eux, euy;
  feindRichtung(z, z.bx, z.by, eux, euy);
  struct R { double ux, uy, gewicht; };
  R rt[9]; int nr = 0;
  rt[nr++] = { eux, euy, 2.0 };
  for (int sk = 0; sk < 8; ++sk) {
    if (!(z.angriffSektoren & (1 << sk))) continue;
    const double w = sk * 3.14159265358979 / 4;
    const double ux = std::cos(w), uy = std::sin(w);
    if (ux * eux + uy * euy > 0.7) continue;                        // liegt schon in der Front
    rt[nr++] = { ux, uy, 1.0 };
  }
  auto ausdehnung = [&](double ux, double uy) {
    double ext = 2;
    for (int h = 0; h < nh; ++h) {
      if (hin[h].w == 1 && hin[h].h == 1) continue;
      if (abstand2(hin[h].x0 + hin[h].w / 2, hin[h].y0 + hin[h].h / 2, z.bx, z.by) > 20 * 20) continue;
      for (int cx = 0; cx < 2; ++cx)
        for (int cy = 0; cy < 2; ++cy) {
          const double pr = (hin[h].x0 + cx * hin[h].w - z.bx) * ux + (hin[h].y0 + cy * hin[h].h - z.by) * uy;
          if (pr > ext) ext = pr;
        }
    }
    return ext;
  };
  // Richtung mit den wenigsten Tuermen je Gewicht zuerst
  int reihe[9]; double last[9];
  for (int i = 0; i < nr; ++i) {
    const double r = ausdehnung(rt[i].ux, rt[i].uy) + 3;
    last[i] = tuermeBei(z, (int)(z.bx + rt[i].ux * r + 0.5), (int)(z.by + rt[i].uy * r + 0.5), 9) / rt[i].gewicht;
    reihe[i] = i;
  }
  for (int i = 0; i < nr; ++i) for (int j = i + 1; j < nr; ++j) if (last[reihe[j]] < last[reihe[i]]) { int t = reihe[i]; reihe[i] = reihe[j]; reihe[j] = t; }
  static const int VERSATZ[9] = { 0, 1, -1, 2, -2, 3, -3, 4, -4 };
  for (int q = 0; q < nr; ++q) {
    const R& d = rt[reihe[q]];
    const double r0 = ausdehnung(d.ux, d.uy) + 3;
    for (int zeile = 0; zeile < 3; ++zeile)
      for (int k = 0; k < 9; ++k) {
        const double r = r0 + zeile * 2;
        const int x = (int)(z.bx + d.ux * r - d.uy * VERSATZ[k] * 2 + 0.5);
        const int y = (int)(z.by + d.uy * r + d.ux * VERSATZ[k] * 2 + 0.5);
        const char* g = "";
        if (!platzFrei(z, x, y, 1, 1, hin, nh, &g) || !schussfeldFrei(hin, nh, x, y, d.ux, d.uy)) continue;
        const int idx = platzEintrag(z, mapGuardPost, x, y);
        if (idx >= 0)
          melde(z, "kampf", mapGuardPost, x, y, "Wachturm %s (Richtung %d Grad, Reihe %d, Platz %d der Linie)",
                reihe[q] == 0 ? "in der Front zum Feind" : "auf einer Seite, von der schon angegriffen wurde",
                (int)(std::atan2(d.uy, d.ux) * 180 / 3.14159265358979), zeile + 1, k + 1);
        return idx;
      }
  }
  return -1;
}

// Turm fuer einen Aussenposten: an der Feindseite der Mine, 3-5 Kacheln, mit freiem Schussfeld.
inline int aussenPlatz(Zustand& z, int a) {
  static Rechteck hin[160];
  const int nh = hindernisse(z, hin, 160);
  double ux, uy;
  feindRichtung(z, z.aussenX[a], z.aussenY[a], ux, uy);
  for (int ring = 0; ring < 4; ++ring)
    for (int s2 = 0; s2 < 5; ++s2) {
      const int seite = (s2 % 2 ? 1 : -1) * ((s2 + 1) / 2);
      const double r = 3 + ring;
      const int x = (int)(z.aussenX[a] + ux * r - uy * seite * 1.5 + 0.5);
      const int y = (int)(z.aussenY[a] + uy * r + ux * seite * 1.5 + 0.5);
      const char* g = "";
      if (!platzFrei(z, x, y, 1, 1, hin, nh, &g) || !schussfeldFrei(hin, nh, x, y, ux, uy)) continue;
      const int idx = platzEintrag(z, mapGuardPost, x, y);
      if (idx >= 0) melde(z, "kampf", mapGuardPost, x, y, "Wachturm fuer den Aussenposten bei %d,%d", z.aussenX[a], z.aussenY[a]);
      return idx;
    }
  return -1;
}

// Turm an eine Schmelze ohne Schutz: sie haelt den ganzen Erzvorrat, und eine verlorene letzte Schmelze kam nie zurueck
// (0 von 11 Faellen, 2026-11-07). Feindseite, wie beim Aussenposten. Guard the smelter that holds the ore stock.
inline int schmelzPlatz(Zustand& z) {
  static Rechteck hin[160];
  // auch Kraftwerke (Eden verlor sie oft; ohne Strom fielen Wohnhaeuser und Agrardome aus)
  const int schutzTypen[2] = { mapCommonOreSmelter, mapTokamak };
  for (int st = 0; st < 2; ++st) {
  if (st == 1 && schutzTypen[1] == mapCommonOreSmelter) break;
  PlayerBuildingEnum e(z.spieler, (map_id)schutzTypen[st]);
  UnitEx s;
  while (e.GetNext(s)) {
    if (!s.IsLive()) continue;
    const LOCATION ls = s.Location();
    const int sx = edX(z, ls), sy = edY(z, ls);
    if (tuermeBei(z, sx, sy) > 0) continue;
    const int nh = hindernisse(z, hin, 160);
    double ux, uy;
    feindRichtung(z, sx, sy, ux, uy);
    for (int ring = 0; ring < 4; ++ring)
      for (int s2 = 0; s2 < 5; ++s2) {
        const int seite = (s2 % 2 ? 1 : -1) * ((s2 + 1) / 2);
        const double r = 4 + ring;
        const int x = (int)(sx + ux * r - uy * seite * 1.5 + 0.5);
        const int y = (int)(sy + uy * r + ux * seite * 1.5 + 0.5);
        const char* g = "";
        if (!platzFrei(z, x, y, 1, 1, hin, nh, &g) || !schussfeldFrei(hin, nh, x, y, ux, uy)) continue;
        const int idx = platzEintrag(z, mapGuardPost, x, y);
        if (idx >= 0) melde(z, "kampf", mapGuardPost, x, y, "Wachturm fuer %s bei %d,%d", st ? "das Kraftwerk" : "die Schmelze", sx, sy);
        return idx;
      }
  }
  }
  return -1;
}

inline int wachtPlatz(Zustand& z) {
  const int a = aussenTurmFehlt(z);
  if (a >= 0) return aussenPlatz(z, a);
  { const int i = schmelzPlatz(z); if (i >= 0) return i; }
  return koloniePlatz(z);
}

// Mauern neben einem Wachturm: je 2 Kacheln links und rechts, quer zur Feindrichtung, auf Hoehe des Turms.
// Nicht davor -- ein Laser schiesst nicht ueber Mauern (Spieler). So bleibt das Schussfeld nach vorn frei
// und die Flanken sind gedeckt. Betonmauern (Tech 5106) gibt es ab Start; der Earthworker der BuildingGroup
// baut sie. Nur auf freien Kacheln mit Abstand zu allem Geplanten (kein Roehrenstutzen zugemauert).
// Walls beside a guard post (left/right), never in front: lasers cannot fire over walls.
// Mauer vor einem wichtigen Gebaeude zur Feindseite (Spieler): 2 Kacheln vor seiner Front, so breit wie das Gebaeude plus je eine
// Kachel. Laser, Mikrowelle und Railgun schiessen nicht darueber; Angreifer muessen herum. Wall in front of a key building.
inline void mauerVor(Zustand& z, int x0, int y0, int w, int h) {
  static Rechteck hin[160];
  const int nh = hindernisse(z, hin, 160);
  const double cx = x0 + w / 2.0, cy = y0 + h / 2.0;
  double ux, uy;
  feindRichtung(z, (int)cx, (int)cy, ux, uy);
  const double vorn = (std::fabs(ux) * w + std::fabs(uy) * h) / 2.0 + 2;
  const int halb = (int)((std::fabs(uy) * w + std::fabs(ux) * h) / 2.0) + 1;
  int n = 0;
  for (int q = -halb; q <= halb; ++q) {
    const int x = (int)(cx + ux * vorn - uy * q + 0.5), y = (int)(cy + uy * vorn + ux * q + 0.5);
    const char* g = "";
    if (!platzFrei(z, x, y, 1, 1, hin, nh, &g)) continue;
    LOCATION l = engine(z, x, y); z.bau.RecordWall(l, mapWall); n++;
  }
  if (n) melde(z, "kampf", mapWall, (int)cx, (int)cy, "%d Mauerkacheln vor %d,%d zur Feindseite", n, x0, y0);
}

inline void mauerNeben(Zustand& z, int tx, int ty) {
  static Rechteck hin[160];
  const int nh = hindernisse(z, hin, 160);
  double ux, uy;
  feindRichtung(z, tx, ty, ux, uy);
  int gesetzt = 0;
  static const int SEITE[6] = { -2, -3, 2, 3, -4, 4 };
  for (int si = 0; si < 6; ++si) {
    const int seite = SEITE[si];
    const int x = (int)(tx - uy * seite + 0.5);
    const int y = (int)(ty + ux * seite + 0.5);
    const char* g = "";
    if (!platzFrei(z, x, y, 1, 1, hin, nh, &g)) continue;
    LOCATION l = engine(z, x, y);
    z.bau.RecordWall(l, mapWall);
    gesetzt++;
  }
  // Mauern davor (Spieler): Laser, Mikrowelle und Railgun schiessen nicht ueber Mauern, Thors Hammer, EMP, Klebeschaum, ESG,
  // Saeure und RPG schon; Klippen wirken vermutlich genauso. Lohnt also nur, wenn der eigene Turm darueber schiessen kann
  // und mindestens die Haelfte der feindlichen Feuerkraft nicht. Walls in front only if our tower fires over them and
  // at least half of the enemy firepower cannot.
  {
    const int wf = eigeneWaffe(z);
    int direkt = 0, gesamt = 0;
    for (int p = 0; p < TethysGame::NoPlayers(); ++p) {
      if (p == z.spieler || ExtPlayer[z.spieler].IsAlliedTo(p)) continue;
      PlayerVehicleEnum e(p); UnitEx u;
      while (e.GetNext(u)) {
        if (!u.IsLive() || kampfWert(u) <= 0) continue;
        const int k = kampfWert(u), w = (int)u.GetWeapon(); gesamt += k;
        if (w == mapLaser || w == mapMicrowave || w == mapRailGun) direkt += k;
      }
    }
    if (wf != mapLaser && wf != mapMicrowave && wf != mapRailGun && gesamt > 0 && direkt * 2 >= gesamt) {
      int vorn = 0;
      for (int q = -1; q <= 1; ++q) {
        const int x = (int)(tx + ux * 2 - uy * q + 0.5), y = (int)(ty + uy * 2 + ux * q + 0.5);
        const char* g = "";
        if (!platzFrei(z, x, y, 1, 1, hin, nh, &g)) continue;
        LOCATION l = engine(z, x, y); z.bau.RecordWall(l, mapWall); vorn++;
      }
      if (vorn) melde(z, "kampf", mapWall, tx, ty, "%d Mauerkacheln vor dem Wachturm (%s schiesst darueber, Feind zu %d %% direkt)", vorn, name(wf), direkt * 100 / gesamt);
    }
  }
  if (gesetzt) melde(z, "kampf", mapWall, tx, ty, "%d Mauerkacheln neben dem Wachturm aufgezeichnet (Schussfeld nach vorn frei)", gesetzt);
}

inline bool bauAufzeichnen(Zustand& z, int typ, const char* warum) {
  const int i = typ == mapGuardPost ? wachtPlatz(z) : platzWaehlen(z, typ);
  if (i < 0) return false;
  Platz& p = z.plaetze[i];
  const Groesse g = grundflaeche(typ);
  LOCATION ort = engine(z, p.x0 + g.w / 2, p.y0 + g.h / 2);
  z.bau.RecordBuilding(ort, (map_id)typ, typ == mapGuardPost ? (map_id)eigeneWaffe(z) : mapNone);
  if (typ == mapGuardPost) mauerNeben(z, p.x0, p.y0);
  if (z.feindGemeldet && (typ == mapTokamak || typ == mapCommonOreSmelter || typ == mapCommandCenter || typ == mapVehicleFactory)) {
    const Groesse gg = grundflaeche(typ); mauerVor(z, p.x0, p.y0, gg.w, gg.h);
  }
  p.zustand = P_FAHRT; p.seit = TethysGame::Tick(); p.convec = 0;
  melde(z, "bau", typ, p.x0, p.y0, "BuildingGroup baut: %s (Platz %dx%d, Mitte %d,%d)", warum, g.w, g.h, edX(z, ort), edY(z, ort));
  return true;
}

// Freier Warteplatz (2026-11-09, Bildschirmfotos: Wache bzw. Sammelgruppe parkten an der Schmelze und versperrten den Lastwagen die
// Anlegestelle -- Erz stand bis zu 16000 Ticks auf 0): nahe dem Wunschpunkt ein Feld von (2h+1) x (2h+1) Kacheln, auf dem samt
// 2 Kacheln Rand kein eigenes Gebaeude steht, das nicht auf dem Weg einer Erzroute (Mine -> Schmelze) liegt, befahrbar ist und --
// fuer die Wache -- keinen Turm in der Naehe hat (ein zerstoerter Turm zerstoert Einheiten daneben). Ringsuche, aber hoechstens 11 Kacheln
// von der Basismitte: weiter draussen haelt der Gegner die Truppe fuer eine anrueckende Welle (feindUnterwegs, ab 15 Kacheln) und greift
// nie an (10 Duelle 2026-11-09 ohne jeden Kampf). Gibt es so nichts, ohne Turmbedingung; sonst bleibt der Wunschpunkt.
// Free waiting area near the wish point: clear of buildings (+2 tiles), ore routes and, for the guard, towers.
inline void freierWarteplatz(const Zustand& z, int& x, int& y, int h, bool turmAbstand) {
  static int rx0[160], ry0[160], rx1[160], ry1[160];
  int nr = 0;
  for (int t : GEBAEUDE) {
    PlayerBuildingEnum e(z.spieler, (map_id)t); UnitEx b;
    while (e.GetNext(b) && nr < 160) {
      if (!b.IsLive()) continue;
      const LOCATION lo = b.Location(); const Groesse g = grundflaeche(t);
      rx0[nr] = edX(z, lo) - g.w / 2 - 2; ry0[nr] = edY(z, lo) - g.h / 2 - 2; rx1[nr] = rx0[nr] + g.w + 3; ry1[nr] = ry0[nr] + g.h + 3; nr++;
    }
  }
  double sx0[8], sy0[8], sx1[8], sy1[8]; int ns = 0;   // Erzrouten als Strecken
  auto strecke = [&](int mine, int schmelze) {
    if (!mine || !schmelze || ns >= 8) return;
    UnitEx m, s2; m.unitID = mine; s2.unitID = schmelze;
    if (!m.IsLive() || !s2.IsLive()) return;
    const LOCATION a = m.Location(), b = s2.Location();
    sx0[ns] = edX(z, a); sy0[ns] = edY(z, a); sx1[ns] = edX(z, b); sy1[ns] = edY(z, b); ns++;
  };
  for (int i = 0; i < MAX_MINEN; ++i) strecke(z.minenId[i], z.schmelzeId[i]);
  for (int i = 0; i < 2; ++i) strecke(z.seltenMineId[i], z.seltenSchmelzeId[i]);
  auto frei = [&](int cx, int cy) {
    if (!aufKarte(z, cx - h, cy - h, 1) || !aufKarte(z, cx + h, cy + h, 1)) return false;
    if (!zelleBefahrbar(zelle(z, cx, cy))) return false;
    for (int i = 0; i < nr; ++i) if (cx + h >= rx0[i] && cx - h <= rx1[i] && cy + h >= ry0[i] && cy - h <= ry1[i]) return false;
    for (int i = 0; i < ns; ++i) {
      const double vx = sx1[i] - sx0[i], vy = sy1[i] - sy0[i], ll = vx * vx + vy * vy;
      double u = ll > 0 ? ((cx - sx0[i]) * vx + (cy - sy0[i]) * vy) / ll : 0; if (u < 0) u = 0; if (u > 1) u = 1;
      const double dx = cx - (sx0[i] + u * vx), dy = cy - (sy0[i] + u * vy);
      if (dx * dx + dy * dy < (h + 2.5) * (h + 2.5)) return false;
    }
    if (turmAbstand && tuermeBei(z, cx, cy, h + 3) > 0) return false;
    return true;
  };
  for (int durchgang = 0; durchgang < 2; ++durchgang) {
    if (durchgang == 1) { if (!turmAbstand) break; turmAbstand = false; }
    for (int r = 0; r <= 14; ++r) {
      int bx = 0, by = 0; double bd = 1e9;
      for (int dy = -r; dy <= r; ++dy) for (int dx = -r; dx <= r; ++dx) {
        if ((dx < 0 ? -dx : dx) != r && (dy < 0 ? -dy : dy) != r) continue;
        const double d = dx * dx + dy * dy;
        if (abstand2(x + dx, y + dy, z.bx, z.by) > 11 * 11) continue;       // in der Basis bleiben
        if (d < bd && frei(x + dx, y + dy)) { bd = d; bx = dx; by = dy; }
      }
      if (bd < 1e9) { x += bx; y += by; return; }
    }
  }
}
// Gemerkte Platzmitten je Spieler: 0 Wache, 1 Sammelgruppe, 2 Reserve daheim, 3 Baugruppe, 4 Fabrikgruppe. Remembered centres.
inline int g_platz[8][5][2];

inline void bauGruppe(Zustand& z, const Lage& l) {
  const int p = z.spieler;
  const int t = TethysGame::Tick();
  // Warteplatz der Gruppe: unter der Basis, frei von Gebaeuden und Erzwegen; alle 2000 Ticks neu pruefen (die Basis waechst)
  if (!z.bauAn || t % 2000 < TAKT) {
    int rx = z.bx, ry = z.by + 9;
    freierWarteplatz(z, rx, ry, 2, false);
    int* gp = g_platz[p & 7][3];
    MAP_RECT ruhe(engine(z, rx - 3, ry - 1), engine(z, rx + 3, ry + 2));
    if (!z.bauAn) {
      z.bau = CreateBuildingGroup(Player[p]);
      z.bau.SetRect(ruhe);
      // Soll der Gruppe: ohne angemeldeten Earthworker baut sie keine Roehren (Lauf 7)
      z.bau.SetTargCount(mapEarthworker, mapNone, 1);
      z.bauAn = 1;
      melde(z, "info", mapStructureFactory, rx, ry, "BuildingGroup angelegt (Warteplatz %d,%d, frei von Gebaeuden und Erzwegen)", rx, ry);
    } else if (gp[0] != rx || gp[1] != ry) {
      z.bau.SetRect(ruhe);
      melde(z, "info", mapStructureFactory, rx, ry, "Warteplatz der BuildingGroup verlegt nach %d,%d (alter Platz belegt)", rx, ry);
    }
    gp[0] = rx; gp[1] = ry;
  }
  // Einheiten aufnehmen: fertige, ruhige Strukturfabriken und alle ConVecs
  {
    PlayerBuildingEnum e(p, mapStructureFactory);
    UnitEx u;
    while (e.GetNext(u))
      if (bereit(u) && !inGruppe(z.bau, u.unitID)) {
        z.bau.TakeUnit(u);
        melde(z, "info", mapStructureFactory, edX(z, u.Location()), edY(z, u.Location()), "Strukturfabrik %d in die BuildingGroup", u.unitID);
      }
  }
  int getragen[128];
  std::memset(getragen, 0, sizeof(getragen));
  {
    PlayerVehicleEnum e(p);
    UnitEx u;
    while (e.GetNext(u)) {
      if (u.GetType() == mapEarthworker && !inGruppe(z.bau, u.unitID)) { z.bau.TakeUnit(u); continue; }  // baut die Roehren
      if (u.GetType() != mapConVec) continue;
      if (!inGruppe(z.bau, u.unitID)) z.bau.TakeUnit(u);
      const int k = u.GetCargo();
      // Nur ruhige ConVecs zaehlen: waehrend des Baus traegt der ConVec den Bausatz noch.
      if (k > 0 && k < 128 && k != mapNone && !u.IsBusy() && u.GetCurAction() == moDone) getragen[k]++;
    }
  }
  // Aufgezeichnete Plaetze nachfuehren
  int offen = 0;
  for (int i = 0; i < z.anzahlPlaetze; ++i) {
    Platz& q = z.plaetze[i];
    if (q.zustand != P_FAHRT && q.zustand != P_BAUT) continue;
    if (stehtDa(z, q)) {
      q.zustand = P_STEHT; q.seit = t;
      melde(z, "fertig", q.typ, q.x0, q.y0, "Gebaeude steht (im Bau oder fertig)");
      continue;
    }
    offen++;
    if (t - q.seit > GEDULD_GRUPPE && selten(z, 1, 3000))
      melde(z, "warte", q.typ, q.x0, q.y0, "BuildingGroup baut seit %d Ticks noch nicht (Erz %d, Seltenerz %d, Strukturfabrik %s)",
            t - q.seit, l.erz, l.selten, l.sf.unitID ? "da" : "fehlt");
  }
  // Bausaetze, die ConVecs schon tragen (Startausruestung), sofort aufzeichnen
  // (Kommandozentrale zuerst: an ihr haengen die Anschlussplaetze aller anderen)
  for (int j = 0; j < 128; ++j) {
    const int k = j == 0 ? (int)mapCommandCenter : (j == mapCommandCenter ? 0 : j);
    if (k <= 0) continue;
    while (getragen[k] > geplant(z, k)) {
      if (!bauAufzeichnen(z, k, "Bausatz liegt schon auf einem ConVec")) { getragen[k] = 0; break; }
      offen++;
    }
  }
  // Neues Gebaeude erst, wenn nichts mehr offen ist und eine Strukturfabrik in der Gruppe arbeitet
  if (offen > 0 || !l.sf.unitID || !inGruppe(z.bau, l.sf.unitID)) return;
  const Wahl w = bauWahl(z, l);
  if (w.typ != mapNone && w.typ != mapCommandCenter && istGeparkt(z, l.sf.unitID)) {
    // Nur wecken, wenn der Bau auch mit den Arbeitern geht, die die Fabrik selbst wieder braucht -- sonst
    // pendelt sie (Lauf 23: Stilllegen machte 4 Arbeiter frei, das machte den Bau "baubar", Wecken nahm sie wieder).
    // Wake only if the build still works once the factory takes its own workers back.
    UnitInfo sfInfo(mapStructureFactory), bauInfo((map_id)w.typ);
    if (l.arbFrei - sfInfo.GetWorkersRequired(p) >= bauInfo.GetWorkersRequired(p) || standardVolk(l)) {
      leerlaufWecken(z, l.sf.unitID, w.warum);                // Leerlauf vorbei: erst einschalten, naechster Schritt baut
      return;
    }
    // Arbeiter-Wunsch: Fabrik + Bau zusammen; die Rangliste legt dafuer Unwichtigeres still
    z.wunschArbeiter = sfInfo.GetWorkersRequired(p) + bauInfo.GetWorkersRequired(p);
    z.wunschRang = rang(z, l, w.typ) + (w.bereich == B_ZIEL ? 10 : 0); z.wunschSeit = TethysGame::Tick();
    if (selten(z, 24, 3000)) melde(z, "warte", w.typ, z.bx, z.by, "Strukturfabrik bleibt still: %s braucht Arbeiter, die die Fabrik selbst bindet", name(w.typ));
    return;
  }
  if (w.typ == mapNone || w.typ == mapCommandCenter) {
    if (w.warum[0] && selten(z, 0, 1500)) melde(z, "warte", mapNone, z.bx, z.by, "%s", w.warum);
    return;
  }
  // Notfaelle kommen ohne Kostenpruefung aus bauWahl -- hier nachholen (nicht leistbar = warten)
  UnitInfo info((map_id)w.typ);
  const int kosten = info.GetOreCost(p), kostenSelten = info.GetRareOreCost(p);
  if (kosten > l.erz || kostenSelten > l.selten) {
    if (selten(z, 0, 1500))
      melde(z, "warte", w.typ, z.bx, z.by, "Erz %d/%d, Seltenerz %d/%d (%s)", l.erz, kosten, l.selten, kostenSelten, w.warum);
    return;
  }
  if (bauAufzeichnen(z, w.typ, w.warum)) z.letzterBau = TethysGame::Tick();
}

// ---------------------------------------------------------------------------------------------------
// Roehren / tubes: der Earthworker arbeitet die Auftragsliste ab

inline bool roehreFertig(const Zustand& z, const Roehre& r) {
  for (int y = r.y1; y <= r.y2; ++y)
    for (int x = r.x1; x <= r.x2; ++x) if (!zelleRoehre(zelle(z, x, y))) return false;
  return true;
}

inline void roehren(Zustand& z) {
  const int t = TethysGame::Tick();
  UnitEx ew;
  {
    PlayerVehicleEnum e(z.spieler);
    UnitEx u;
    while (e.GetNext(u)) if (u.GetType() == mapEarthworker) { ew = u; break; }
  }
  bool beschaeftigt = false;
  for (int i = 0; i < z.anzahlRoehren; ++i) {
    Roehre& r = z.roehren[i];
    if (r.zustand == 2) continue;
    if (roehreFertig(z, r)) { r.zustand = 2; continue; }
    if (r.zustand == 1) {
      if (ew.unitID && (ew.IsBusy() || t - r.seit < 150)) { beschaeftigt = true; continue; }
      r.versuche++;
      if (r.versuche >= 3) {
        melde(z, "fehler", mapNone, r.x1, r.y1, "Roehre bis %d,%d nicht gebaut -- aufgegeben", r.x2, r.y2);
        r.zustand = 2;
        continue;
      }
      r.zustand = 0;
    }
    if (beschaeftigt || !ew.unitID || ew.IsBusy()) continue;
    // Nur die Einzelkacheln bauen, die noch keine Roehre sind? Ein Auftrag fuer das ganze Stueck ist
    // einfacher; der Earthworker ueberspringt fertige Kacheln.
    MAP_RECT rect(engine(z, r.x1, r.y1), engine(z, r.x2, r.y2));
    ew.DoBuildWall(mapTube, rect);
    r.zustand = 1; r.seit = t;
    beschaeftigt = true;
    melde(z, "roehre", mapNone, r.x1, r.y1, "Earthworker %d legt Roehre bis %d,%d", ew.unitID, r.x2, r.y2);
  }
  if (!ew.unitID) {
    for (int i = 0; i < z.anzahlRoehren; ++i)
      if (z.roehren[i].zustand != 2) {
        if (selten(z, 1, 3000)) melde(z, "warte", mapEarthworker, z.bx, z.by, "Roehren offen, aber kein Earthworker");
        break;
      }
  }
}

// ---------------------------------------------------------------------------------------------------
// Anschluss / connection: Gebaeude ohne Verbindung zur Kommandozentrale per Roehre anschliessen.
//
// Erkennen: fertiges, ruhiges Gebaeude (moDone) mit Strombedarf hat keinen Strom, obwohl genug frei ist.
// Vorsicht: Gebaeude im Bau haben auch keinen Strom (Aktion 6, beschaeftigt) -- die zaehlen nicht.
// Dann sucht die KI den kuerzesten Weg ueber freie Kacheln zu einem angeschlossenen Gebaeude (oder der Kommandozentrale) und zeichnet jede
// Roehrenkachel in der BuildingGroup auf (RecordTube); deren Earthworker baut sie.
// Buildings without power despite free power are disconnected: the AI finds the shortest free path to a
// connected building and records each tube tile in the BuildingGroup (its Earthworker builds them).

constexpr int MAX_WEG = 400;

inline bool angeschlossen(UnitEx u) { return u.HasPower() != 0; }

// Belegte Kacheln: Grundflaechen aller eigenen Gebaeude und geplanten Plaetze
inline bool belegt(const Rechteck* hin, int nh, int x, int y) {
  for (int i = 0; i < nh; ++i)
    if (x >= hin[i].x0 && x < hin[i].x0 + hin[i].w && y >= hin[i].y0 && y < hin[i].y0 + hin[i].h) return true;
  return false;
}

// Kachel direkt an einer Seite der Grundflaeche (ohne Ecken). Tile directly beside a side (no corners).
inline bool anGrundflaeche(const Rechteck& r, int x, int y) {
  const bool innenX = x >= r.x0 && x < r.x0 + r.w, innenY = y >= r.y0 && y < r.y0 + r.h;
  return (innenX && (y == r.y0 - 1 || y == r.y0 + r.h)) || (innenY && (x == r.x0 - 1 || x == r.x0 + r.w));
}

// Kuerzester Weg (Breitensuche) vom Rand von `von` zum Rand eines Ziel-Rechtecks. Liefert die Laenge,
// die Kacheln stehen in wx/wy (vom Start zum Ziel). 0 = kein Weg.
inline int wegSuchen(const Zustand& z, const Rechteck& von, const Rechteck* ziele, int nz,
                     const Rechteck* hin, int nh, short* wx, short* wy, bool ohneRoehren = false) {
  static int vor[256 * 256];
  static short qx[256 * 256], qy[256 * 256];
  const int B = z.breite < 256 ? z.breite : 256, H = z.hoehe < 256 ? z.hoehe : 256;
  for (int i = 0; i < B * H; ++i) vor[i] = -2;
  int kopf = 0, ende = 0;
  auto frei = [&](int x, int y) {
    if (!aufKarte(z, x, y, 1)) return false;
    const int c = zelle(z, x, y);
    if (ohneRoehren && zelleRoehre(c)) return false;
    return (zelleBebaubar(c) || zelleRoehre(c)) && !belegt(hin, nh, x, y);
  };
  for (int y = von.y0 - 1; y <= von.y0 + von.h; ++y)
    for (int x = von.x0 - 1; x <= von.x0 + von.w; ++x)
      if (anGrundflaeche(von, x, y) && frei(x, y) && vor[y * B + x] == -2) { vor[y * B + x] = -1; qx[ende] = x; qy[ende] = y; ende++; }
  static const int DX[4] = { 1, -1, 0, 0 }, DY[4] = { 0, 0, 1, -1 };
  while (kopf < ende) {
    const int x = qx[kopf], y = qy[kopf]; kopf++;
    for (int k = 0; k < nz; ++k)
      if (anGrundflaeche(ziele[k], x, y)) {
        int n = 0, cx = x, cy = y;
        while (n < MAX_WEG) {
          wx[n] = (short)cx; wy[n] = (short)cy; n++;
          const int v = vor[cy * B + cx];
          if (v < 0) break;
          cx = v % B; cy = v / B;
        }
        return n;
      }
    for (int d = 0; d < 4; ++d) {
      const int nx = x + DX[d], ny = y + DY[d];
      if (nx < 0 || ny < 0 || nx >= B || ny >= H || vor[ny * B + nx] != -2 || !frei(nx, ny)) continue;
      vor[ny * B + nx] = y * B + x;
      qx[ende] = nx; qy[ende] = ny; ende++;
    }
  }
  return 0;
}

inline void anschluss(Zustand& z, const Lage& l) {
  const int t = TethysGame::Tick();
  if (t < z.naechsterAnschluss || !z.bauAn) return;
  z.naechsterAnschluss = t + 200;
  // Laufende Roehrenauftraege: fertig, sobald die Kachel eine Roehre ist
  int offen = 0;
  for (int i = 0; i < z.anzahlRoehren; ++i) {
    Roehre& r = z.roehren[i];
    if (r.zustand != 1) continue;
    if (zelleRoehre(zelle(z, r.x1, r.y1))) {
      r.zustand = 2;
      melde(z, "fertig", mapEarthworker, r.x1, r.y1, "Roehre gebaut (%s, nach %d Ticks)", r.versuche ? "Rueckfall Earthworker" : "BuildingGroup", t - r.seit);
      continue;
    }
    offen++;
    // Rueckfall: baut die Gruppe die Kachel nicht, bekommt der Earthworker den direkten Befehl.
    if (t - r.seit > 1500 && r.versuche == 0) {
      PlayerVehicleEnum e(z.spieler); UnitEx ew;
      while (e.GetNext(ew)) {
        if (ew.GetType() != mapEarthworker || ew.IsBusy()) continue;
        MAP_RECT rect(engine(z, r.x1, r.y1), engine(z, r.x2, r.y2));
        ew.DoBuildWall(mapTube, rect);
        r.versuche = 1; r.seit = t;
        melde(z, "fehler", mapEarthworker, r.x1, r.y1, "BuildingGroup baut die Roehre nicht -- Earthworker %d direkt (Rueckfall)", ew.unitID);
        break;
      }
    }
  }
  if (offen) {
    if (selten(z, 19, 3000)) melde(z, "warte", mapEarthworker, z.bx, z.by, "%d Roehrenkacheln aufgezeichnet, noch nicht gebaut (Erz %d)", offen, l.erz);
    return;
  }
  static Rechteck hin[160];
  const int nh = hindernisse(z, hin, 160);
  Rechteck ziele[64]; int nz = 0;
  UnitEx lose[8]; Rechteck loseR[8]; int nl = 0;
  for (int typ : GEBAEUDE) {
    PlayerBuildingEnum e(z.spieler, (map_id)typ);
    UnitEx u;
    while (e.GetNext(u)) {
      const LOCATION lo = u.Location();
      const Rechteck r = rechteckUm(typ, edX(z, lo), edY(z, lo));
      if (typ == mapCommandCenter || (hatStutzen(typ) && angeschlossen(u))) { if (nz < 64) ziele[nz++] = r; continue; }
      if (!hatStutzen(typ) || nl >= 8) continue;
      // Nur fertige, ruhige Gebaeude: im Bau stehen sie auf Aktion 6, beschaeftigt und ohne Strom -- das
      // ist KEIN fehlender Anschluss (Lauf 13 gemessen: Standard Lab ~2700 Ticks im Bau).
      // Only finished idle buildings: under construction they show action 6, busy, no power.
      if (u.GetCurAction() != moDone || u.IsBusy()) continue;
      UnitInfo info((map_id)typ);
      const int bedarfStrom = info.GetPowerRequired(z.spieler);
      if (bedarfStrom <= 0) continue;                                            // Lager u.ae.: nie Strom, nicht erkennbar
      if (l.stromFrei < bedarfStrom) continue;                                   // fehlt wirklich Strom
      lose[nl] = u; loseR[nl] = r; nl++;
    }
  }
  if (nl == 0 || nz == 0) return;
  // Alle unverbundenen Gebaeude in einer Runde anschliessen
  for (int j = 0; j < nl; ++j) {
    short wx[MAX_WEG], wy[MAX_WEG];
    int n = wegSuchen(z, loseR[j], ziele, nz, hin, nh, wx, wy);
    // Liegt der Weg schon ganz auf Roehrenkacheln und das Gebaeude hat trotzdem keinen Strom, einen
    // zweiten Weg ueber freie Kacheln suchen (Rueckfall, falls eine Roehre selbst nicht am Netz haengt).
    // Path already all tubes but still unpowered: try a second path over free tiles (fallback).
    bool alleRoehre = n > 0;
    for (int i = 0; i < n && alleRoehre; ++i) if (!zelleRoehre(zelle(z, wx[i], wy[i]))) alleRoehre = false;
    if (alleRoehre) {
      const int n2 = wegSuchen(z, loseR[j], ziele, nz, hin, nh, wx, wy, true);
      if (n2 > 0) n = n2;
      else { n = 0; }
    }
    const int typ = lose[j].GetType();
    if (n == 0) {
      if (selten(z, 20, 3000)) melde(z, "fehler", typ, loseR[j].x0, loseR[j].y0, "nicht angeschlossen, kein freier Weg fuer eine Roehre");
      continue;
    }
    int neu = 0;
    for (int i = 0; i < n; ++i) {
      if (zelleRoehre(zelle(z, wx[i], wy[i]))) continue;
      bool schon = false;
      for (int k = 0; k < z.anzahlRoehren; ++k)
        if (z.roehren[k].zustand == 1 && z.roehren[k].x1 == wx[i] && z.roehren[k].y1 == wy[i]) schon = true;
      if (schon) continue;
      LOCATION ort = engine(z, wx[i], wy[i]);
      z.bau.RecordTube(ort);
      if (z.anzahlRoehren < MAX_ROEHREN) {
        Roehre& r = z.roehren[z.anzahlRoehren++];
        r.x1 = r.x2 = wx[i]; r.y1 = r.y2 = wy[i]; r.zustand = 1; r.versuche = 0; r.seit = t;
      }
      neu++;
    }
    if (neu == 0) {
      // Weg liegt schon ganz auf Roehren und das Gebaeude hat trotzdem keinen Strom -- die Roehre haengt
      // selbst nicht am Netz (Lauf 9). Nur selten melden.
      if (selten(z, 21, 4000))
        melde(z, "warte", typ, loseR[j].x0, loseR[j].y0, "ohne Strom, obwohl der Weg %d,%d -> %d,%d schon Roehre ist", wx[0], wy[0], wx[n - 1], wy[n - 1]);
      continue;
    }
    melde(z, "roehre", typ, loseR[j].x0, loseR[j].y0, "nicht angeschlossen (kein Strom bei %d frei) -- %d Roehrenkacheln aufgezeichnet, Weg %d,%d -> %d,%d",
          l.stromFrei, neu, wx[0], wy[0], wx[n - 1], wy[n - 1]);
  }
  // Erledigte Eintraege vorn entfernen, damit die Liste nicht voll laeuft
  int k = 0;
  for (int i = 0; i < z.anzahlRoehren; ++i) if (z.roehren[i].zustand != 2) z.roehren[k++] = z.roehren[i];
  z.anzahlRoehren = k;
}

// ---------------------------------------------------------------------------------------------------
// Erz / ore: Bake finden, vermessen, Mine bauen, Lastwagen zuteilen

struct BakenFund { UnitEx beste; int gesehen, gewoehnlich, frei, einheiten; int balken; double wert; UnitEx unvermessen; };

// Hoechste belegte Einheitennummer. HFL legt die Kartenstruktur des Spiels offen (`mapObj`, siehe
// HFL/Source/GameMapEx.cpp, struct OP2Map); `lastUsedUnitIndex` liegt dort bei Byte 64.
// Highest used unit index, read from the game's map object as described by HFL.
inline int hoechsteEinheit() {
  if (!isInited || !mapObj) return 0;
  const int n = *reinterpret_cast<const int*>(reinterpret_cast<const char*>(mapObj) + 64);
  return (n > 0 && n < 4096) ? n : 0;
}

// Steht auf dieser Kachel eine Mine (irgendeines Spielers)? Ueber die Gebaeudelisten: der LocationEnumerator des Spiels
// liefert eine Mine erst nach einer unbekannten Weile (gemessen 2026-11-12: 840 Ticks lang nicht).
inline bool mineAuf(const LOCATION& l) {
  static const int MINEN_T[2] = { mapCommonOreMine, mapRareOreMine };
  for (int p = 0; p < TethysGame::NoPlayers(); ++p)
    for (int typ : MINEN_T) {
      PlayerBuildingEnum e(p, (map_id)typ); UnitEx u;
      while (e.GetNext(u)) if (u.IsLive()) { const LOCATION m = u.Location(); if (m.x == l.x && m.y == l.y) return true; }
    }
  return false;
}

// Ertrag einer Bake in Balken (1..3), wie ein Spieler ihn kennt: erst nach dem Vermessen; vorher die Erwartung 2.
// Engine: barYield 0 = Bar3, 1 = Bar2, 2 = Bar1 (wie im Editor, codegen_groups.py). Yield as a player knows it.
inline int bakenBalken(const Zustand& z, UnitEx& b, bool* bekannt = nullptr) {
  const bool vermessen = (b.GetSurveyedBy() & (1 << z.spieler)) != 0;
  if (bekannt) *bekannt = vermessen;
  if (!vermessen) return 2;
  const int y = b.GetBarYield();
  return y == 0 ? 3 : y == 1 ? 2 : y == 2 ? 1 : 2;
}

// Beste freie Erz-Bake der Sorte `erzTyp` (0 gewoehnlich, 1 Seltenerz) nach erwartetem Ertrag (Spieler: "wie reichhaltig
// ist die Erzmine -- anhand dessen weiss man, wie viel Ressourcen zu erwarten sind"): Balken x Lastwagenertrag ueber
// die Entfernung (eine Fahrt ~300 + 60 Ticks je Kachel, Erztest), abgewertet, wenn die Bake naeher am Feind liegt als
// an der eigenen Basis. Dazu die naechste unvermessene Bake -- der Surveyor vermisst sie, damit die Wahl besser wird.
// Drei Wege, weil nicht sicher ist, welcher Aufzaehler Baken liefert: alle Einheitennummern, Rechteck, Umkreis.
// Best free beacon by expected yield (bars x truck yield over distance, devalued near the enemy).
inline BakenFund bakeSuchen(const Zustand& z, int erzTyp = 0) {
  BakenFund f; f.gesehen = f.gewoehnlich = f.frei = f.einheiten = 0; f.balken = 0; f.wert = -1;
  int ids[64]; int nids = 0;
  int naechsteUnverm = 1 << 30;
  auto pruefe = [&](UnitEx& u) {
    if (u.GetType() != mapMiningBeacon) return;
    for (int i = 0; i < nids; ++i) if (ids[i] == u.unitID) return;
    if (nids < 64) ids[nids++] = u.unitID;
    f.gesehen++;
    if (u.GetOreType() != erzTyp) return;                              // 0 gewoehnlich, 1 Seltenerz
    f.gewoehnlich++;
    const LOCATION l = u.Location();
    if (mineAuf(l)) return;
    for (int i = 0; i < 8; ++i) if (z.sperrBake[i] == u.unitID && TethysGame::Tick() < z.sperrBis[i]) return;
    f.frei++;
    bool bekannt = false;
    const int balken = bakenBalken(z, u, &bekannt);
    const int bx = edX(z, l), by = edY(z, l);
    const double d = std::sqrt((double)abstand2(bx, by, z.bx, z.by));
    double wert = balken * 1000.0 / (300 + 60 * d);                   // Erz je Lastwagen und Zeit, relativ
    if (z.feindGeb) {
      const double df = std::sqrt((double)abstand2(bx, by, z.feindX, z.feindY));
      if (df < d) wert *= df / (d > 1 ? d : 1);                         // naeher am Feind: gefaehrdet
    }
    if (wert > f.wert) { f.wert = wert; f.beste = u; f.balken = balken; }
    // vermessen nur, was nicht naeher am Feind liegt als an der eigenen Basis (Test: Surveyor fuhr zur Bake des Gegners)
    const bool feindSeite = z.feindGeb && abstand2(bx, by, z.feindX, z.feindY) < abstand2(bx, by, z.bx, z.by);
    if (!bekannt && !feindSeite && abstand2(bx, by, z.bx, z.by) < naechsteUnverm) { naechsteUnverm = abstand2(bx, by, z.bx, z.by); f.unvermessen = u; }
  };
  // Die Aufzaehler der Engine liefern Baken offenbar nicht (zweiter Lauf: 0 gesehen, auch auf der
  // eigenen Kachel). Darum zuerst alle Einheitennummern durchgehen.
  // The engine's enumerators apparently skip beacons, so walk all unit indices first.
  {
    const int bis = hoechsteEinheit();
    f.einheiten = bis;
    for (int id = 1; id <= bis; ++id) {
      UnitEx u; u.unitID = id;
      if (!u.IsLive()) continue;
      pruefe(u);
    }
  }
  {
    MAP_RECT alles(engine(z, 1, 1), engine(z, z.breite - 2, z.hoehe - 2));
    InRectEnumerator e(alles);
    UnitEx u;
    while (e.GetNext(u)) pruefe(u);
  }
  {
    InRangeEnumerator e(engine(z, z.bx, z.by), 60);
    UnitEx u;
    while (e.GetNext(u)) pruefe(u);
  }
  for (int i = 0; i < z.anzahlBaken; ++i) {
    LocationEnumerator e(engine(z, z.bakenX[i], z.bakenY[i]));
    UnitEx u;
    while (e.GetNext(u)) pruefe(u);
  }
  return f;
}

// Faehrt der Lastwagen schon in einer Erzgruppe (gewoehnlich oder Seltenerz)?
// Is the truck already in an ore group (common or rare)?
inline bool seltenBakeDa(const Zustand& z) {
  if (seltenRouten(z)) return true;
  PlayerBuildingEnum e(z.spieler, mapRareOreMine);
  UnitEx u;
  if (e.GetNext(u)) return true;
  return bakeSuchen(z, 1).beste.unitID != 0;
}

// Lastwagen je Erzroute aus der Entfernung Mine-Schmelze (Erztest, je Messung eigene Mission, 2026-11-05):
// eine Fahrt (Laden, Hin, Abladen, Zurueck) dauert etwa 300 + 60 Ticks je Kachel, eine Ladung bringt ~500 Erz,
// die Schmelze verarbeitet hoechstens ~1400 Erz je 1000 Ticks. Gemessen je 1000 Ticks:
//   Abstand  5: 1 LKW 800,  2 LKW 1346, 3 LKW 1373 (voll)      Abstand 15: 434 / 813 / 1069 / 1375
//   Abstand 10: 534 / 1051 / 1390 (voll mit 3)                Abstand 20: 344 / 714 / 920 / 1213
//   Abstand 25: 264 / 526 / 779 / 994
// Soll = so viele, bis die Schmelze ausgelastet ist; mehr bringt nichts. Trucks per route from the measured trip time.
inline int lastwagenFuerAbstand(double kacheln) {
  const double fahrt = 300 + 60 * kacheln;                  // Ticks je Fahrt
  const double jeLkw = 500 * 1000 / fahrt;                  // Erz je 1000 Ticks und Lastwagen
  int n = (int)(1400 / jeLkw + 0.85);                        // bis die Schmelze (fast) ausgelastet ist
  return n < 1 ? 1 : n > 8 ? 8 : n;
}
inline int lastwagenSoll(const Zustand& z, int i) {
  if (!z.minenId[i] || !z.schmelzeId[i]) return 2;
  UnitEx m, s; m.unitID = z.minenId[i]; s.unitID = z.schmelzeId[i];
  if (!m.IsLive() || !s.IsLive()) return 2;
  const LOCATION ml = m.Location(), sl = s.Location();
  return lastwagenFuerAbstand(std::sqrt((double)abstand2(ml.x, ml.y, sl.x, sl.y)));
}

inline bool lastwagenVergeben(Zustand& z, int id) {
  for (int i = 0; i < MAX_MINEN; ++i) {
    if (!z.minenId[i]) continue;
    GroupEnumerator ge(z.minenGruppe[i]);
    Unit v;
    while (ge.GetNext(v)) if (v.unitID == id) return true;
  }
  for (int i = 0; i < 2; ++i) {
    if (!z.seltenMineId[i]) continue;
    GroupEnumerator ge(z.seltenGruppe[i]);
    Unit v;
    while (ge.GetNext(v)) if (v.unitID == id) return true;
  }
  return false;
}

inline void erz(Zustand& z, const Lage& l) {
  const int p = z.spieler;
  const int t = TethysGame::Tick();
  // Ziel: naechste freie Bake. Der Surveyor faehrt hin und vermisst (wie ein Spieler es tun wuerde);
  // der Miner baut, sobald sie vermessen ist -- oder ohne, wenn es keinen Surveyor gibt oder er zu
  // lange braucht.
  // Target: nearest free beacon. The surveyor goes there first (human-like); the miner builds once it
  // is surveyed -- or without, if there is no surveyor or it takes too long.
  if (l.geb[mapCommonOreMine] > z.minenGesehen) {
    melde(z, "fertig", mapCommonOreMine, z.bx, z.by, "Mine steht (%d) -- Weg %d hat funktioniert", l.geb[mapCommonOreMine], z.minerWeg);
  }
  z.minenGesehen = l.geb[mapCommonOreMine];
  if (!z.kbStand || t - z.kbStand > 5000) {
    z.kbStand = t > 0 ? t : 1; z.kbN = 0;
    const int bis = hoechsteEinheit();
    for (int id = 1; id <= bis && z.kbN < 32; ++id) {
      UnitEx u; u.unitID = id;
      if (!u.IsLive() || u.GetType() != mapMiningBeacon) continue;
      const LOCATION bl = u.Location();
      z.kbX[z.kbN] = (short)edX(z, bl); z.kbY[z.kbN] = (short)edY(z, bl); z.kbN++;
    }
  }
  if (l.geb[mapCommonOreMine] < MAX_MINEN) {
    BakenFund f = bakeSuchen(z);
    if (!f.beste.unitID) {
      if (selten(z, 8, 5000))
        melde(z, "warte", mapMiningBeacon, z.bx, z.by, "keine freie Erz-Bake (gesehen %d, gewoehnlich %d, frei %d, Einheiten bis %d)",
              f.gesehen, f.gewoehnlich, f.frei, f.einheiten);
    } else {
      UnitEx b = f.beste;
      const LOCATION bl = b.Location();
      if (z.bakeZiel != b.unitID) {
        z.bakeZiel = b.unitID; z.bakeSeit = t;
        bool bek = false; const int bal = bakenBalken(z, b, &bek);
        melde(z, "info", mapMiningBeacon, edX(z, bl), edY(z, bl), "Bake %d gewaehlt: %d Balken%s, Wert %.2f (gesehen %d, frei %d)",
              b.unitID, bal, bek ? "" : " (erwartet, unvermessen)", f.wert, f.gesehen, f.frei);
      }
      const bool vermessen = (b.GetSurveyedBy() & (1 << p)) != 0;
      // Surveyor
      UnitEx s;
      bool surveyor = false;
      { PlayerVehicleEnum e(p); UnitEx u; while (e.GetNext(u)) if (u.GetType() == mapRoboSurveyor) { s = u; surveyor = true; break; } }
      // Ziel schon vermessen: der Surveyor vermisst die naechste unvermessene Bake (Ertrag kennen, bevor gebaut wird)
      if (vermessen && surveyor && f.unvermessen.unitID && !s.IsBusy() && s.GetCurAction() == moDone && selten(z, 11, 600)) {
        const LOCATION ul = f.unvermessen.Location(), sl2 = s.Location();
        LOCATION hin = ul;
        if (abstand2(sl2.x, sl2.y, ul.x, ul.y) > 2) hin.x += 1;         // erst daneben, dann drauf (wie oben)
        s.DoMove(hin);
        melde(z, "vermessen", mapMiningBeacon, edX(z, ul), edY(z, ul), "Surveyor %d vermisst die naechste Bake (Ertrag noch unbekannt)", s.unitID);
      }
      if (!vermessen && surveyor && !s.IsBusy() && s.GetCurAction() == moDone && selten(z, 9, 400)) {
        const LOCATION sl = s.Location();
        const bool dort = abstand2(sl.x, sl.y, bl.x, bl.y) <= 2;
        LOCATION ziel = bl;
        if (!dort) ziel.x += 1;
        s.DoMove(ziel);
        melde(z, "vermessen", mapMiningBeacon, edX(z, bl), edY(z, bl),
              dort ? "Surveyor %d steht an der Bake, noch nicht vermessen -- faehrt drauf" : "Surveyor %d faehrt zur Bake", s.unitID);
      }
      // Miner. HFLs DoDeployMiner faehrt nur hin (Lauf 4); DoBuild mit der beim ConVec gemessenen Lesart
      // baut die Mine (Lauf 5, Weg 1). Die BuildingGroup bleibt als Rueckfall.
      // How to make the miner build is not settled; the AI tries DoBuild, then a BuildingGroup.
      UnitEx m; m.unitID = z.bakeMiner;
      const bool minerLebt = z.bakeMiner && m.IsLive() && m.GetType() == mapRoboMiner;
      const bool minerAmWerk = minerLebt && (m.IsBusy() || m.GetCurAction() != moDone);
      const bool darf = vermessen || !surveyor || t - z.bakeSeit > 1200;
      const bool gruppeArbeitet = z.minerWeg == 2 && z.minenBauAn && z.minenBau.TotalUnitCount() > 0;
      if (gruppeArbeitet && t - z.minerSeit > 3000) {
        // Weg 1 und Weg 2 erfolglos: an dieser Bake geht es nicht (Platz verbaut, Untergrund) -- Miner frei, Bake sperren
        if (minerLebt) z.minenBau.RemoveUnit(m);
        z.minenBau.Destroy(); z.minenBauAn = 0;                          // alter Bauauftrag darf nicht mitwandern
        int k = 0;
        for (int i = 1; i < 8; ++i) if (z.sperrBis[i] < z.sperrBis[k]) k = i;
        z.sperrBake[k] = b.unitID; z.sperrBis[k] = t + 40000;
        melde(z, "fehler", mapCommonOreMine, edX(z, bl), edY(z, bl), "Mine an Bake %d kommt nicht zustande -- Bake bis Tick %d gesperrt, naechste Bake",
              b.unitID, z.sperrBis[k]);
        z.minerWeg = 1; z.bakeZiel = 0; z.bakeMiner = 0;
        if (minerLebt) m.DoStop();
      } else if (gruppeArbeitet) {
        if (selten(z, 10, 2000))
          melde(z, "warte", mapCommonOreMine, edX(z, bl), edY(z, bl), "BuildingGroup soll den Miner einsetzen (%d Einheiten)",
                z.minenBau.TotalUnitCount());
      } else if (darf && !minerAmWerk) {
        if (minerLebt && t - z.minerSeit > 300) {
          melde(z, "fehler", mapCommonOreMine, edX(z, bl), edY(z, bl), "Robo-Miner %d steht, Mine nicht gebaut (Weg %d, vermessen: %s)",
                m.unitID, z.minerWeg, vermessen ? "ja" : "nein");
          z.minerWeg = z.minerWeg == 1 ? 2 : z.minerWeg == 2 ? 0 : 1;
        }
        PlayerVehicleEnum e(p);
        UnitEx u;
        while (e.GetNext(u)) {
          if (u.GetType() != mapRoboMiner || u.IsBusy()) continue;
          if (u.unitID == z.seltenMiner) continue;                         // fuer Seltenerz reserviert
          if (u.unitID == z.bakeMiner && t - z.minerSeit < 300) break;      // gleicher Auftrag, kurz warten
          const char* weg = "";
          if (z.minerWeg == 1) {
            // wie beim ConVec: Zielpunkt = Ecke unten rechts der Grundflaeche (2x1, Bake rechts) mit Rand
            u.DoBuild(mapCommonOreMine, LOCATION(bl.x + 1, bl.y + 1));
            weg = "Weg 1: DoBuild";
          } else if (z.minerWeg == 2) {
            if (!z.minenBauAn) {
              z.minenBau = CreateBuildingGroup(Player[p]);
              MAP_RECT ruhe(engine(z, z.bx - 3, z.by + 6), engine(z, z.bx + 3, z.by + 9));
              z.minenBau.SetRect(ruhe);
              z.minenBauAn = 1;
            }
            z.minenBau.TakeUnit(u);
            LOCATION ort = bl;
            z.minenBau.RecordBuilding(ort, mapCommonOreMine, mapNone);
            weg = "Weg 2: BuildingGroup";
          } else {
            u.DoDeployMiner(bl);
            weg = "Weg 0: HFL DoDeployMiner";
          }
          z.bakeMiner = u.unitID; z.minerSeit = t;
          melde(z, "mine", mapCommonOreMine, edX(z, bl), edY(z, bl), "Robo-Miner %d baut Mine (%s, %s)", u.unitID, weg,
                vermessen ? "Bake vermessen" : surveyor ? "Surveyor braucht zu lange" : "ohne Surveyor");
          break;
        }
      }
    }
  }
  // 3) Erzrouten: jede fertige Schmelze bekommt eine Mine, eine Mine versorgt bis zu 2 Schmelzen (Spieler:
  //    "Eine Mine kann 2 Smelter betreiben"). Je Route eine MiningGroup mit zwei Lastwagen.
  // Ore routes: every finished smelter gets a mine; one mine feeds up to two smelters.
  UnitEx schmelzen[MAX_MINEN];
  int ns = 0;
  { PlayerBuildingEnum e(p, mapCommonOreSmelter); UnitEx u;
    // nur lebende: eine gerade zerstoerte Schmelze steht noch in der Liste (HFL setzt dann beim Abfragen unitID 0)
    while (e.GetNext(u) && ns < MAX_MINEN) { const int id = u.unitID; if (u.IsLive() && u.GetCurAction() != moObjWait && u.GetCurAction() != moDevelop) { u.unitID = id; schmelzen[ns++] = u; } } }
  UnitEx minen[MAX_MINEN];
  int nm = 0;
  { PlayerBuildingEnum e(p, mapCommonOreMine); UnitEx u;
    while (e.GetNext(u) && nm < MAX_MINEN) { const int id = u.unitID; if (u.IsLive() && u.GetCurAction() != moDevelop && u.GetCurAction() != moObjWait) { u.unitID = id; minen[nm++] = u; } } }
  // Routen, deren Mine oder Schmelze nicht mehr steht, aufloesen -- die Lastwagen vorher aus der Gruppe nehmen,
  // damit keine Gruppe mit zerstoerter Mine/Schmelze weiterlebt (Absturz-Verdacht Duell 4/7 bei 00429B49).
  hinweis(z, "ki::erz/routen-pruefen");
  for (int i = 0; i < MAX_MINEN; ++i) {
    if (!z.minenId[i]) continue;
    bool m = false, sm = false;
    for (int k = 0; k < nm; ++k) if (minen[k].unitID == z.minenId[i]) m = true;
    for (int k = 0; k < ns; ++k) if (schmelzen[k].unitID == z.schmelzeId[i]) sm = true;
    if (!m || !sm) {
      hinweis(z, "ki::erz/route-aufloesen");
      UnitEx raus[8]; int nr = 0;
      // nur lebende Lastwagen herausnehmen (Duell 15: Absturz 00475EF5 hier, Schmelze gerade zerstoert)
      { GroupEnumerator ge(z.minenGruppe[i]); UnitEx v; while (ge.GetNext(v) && nr < 8) if (v.IsLive() && v.GetType() == mapCargoTruck) raus[nr++] = v; }
      for (int k = 0; k < nr; ++k) z.minenGruppe[i].RemoveUnit(raus[k]);
      z.minenId[i] = 0; z.schmelzeId[i] = 0;
      melde(z, "mine", mapCommonOreMine, z.bx, z.by, "Erzroute aufgeloest (%s zerstoert), %d Lastwagen frei", !m ? "Mine" : "Schmelze", nr);
    }
  }
  if (!ns || !nm) return;   // erst nach dem Aufloesen: sonst bliebe die Route der letzten zerstoerten Schmelze stehen
  for (int k = 0; k < ns; ++k) {
    int g = -1;
    for (int i = 0; i < MAX_MINEN; ++i) if (z.minenId[i] && z.schmelzeId[i] == schmelzen[k].unitID) g = i;
    if (g < 0) {
      // Mine mit den wenigsten Routen (hoechstens 2), bei Gleichstand die naechste
      int beste = -1, besteN = 3, besteD = 1 << 30;
      if (!schmelzen[k].IsLive()) continue;              // gerade zerstoert (Duell 18: Absturz 00475EF5 nach route-aufloesen)
      hinweis(z, "ki::erz/route-suchen");
      LOCATION sl = schmelzen[k].Location();
      for (int m = 0; m < nm; ++m) {
        int n = 0;
        for (int i = 0; i < MAX_MINEN; ++i) if (z.minenId[i] == minen[m].unitID) n++;
        if (!minen[m].IsLive()) continue;
        const LOCATION ml = minen[m].Location();
        const int d = abstand2(ml.x, ml.y, sl.x, sl.y);
        if (n < 2 && (n < besteN || (n == besteN && d < besteD))) { beste = m; besteN = n; besteD = d; }
      }
      if (beste < 0) continue;                                     // jede Mine hat schon zwei Schmelzen
      // Nicht mitten im Gefecht: Mine/Schmelze schwer beschaedigt oder Feind in der Naehe -> spaeter (Duell 8: Absturz
      // in CreateMiningGroup/Setup bei 00429B49, waehrend die Schmelze zerstoert wurde)
      {
        bool gefahr = false;
        UnitEx pr[2] = { minen[beste], schmelzen[k] };
        for (UnitEx& u2 : pr) {
          UnitInfo ui((map_id)u2.GetType());
          if (!u2.IsLive() || (ui.GetHitPoints(p) > 0 && u2.GetDamage() * 2 > ui.GetHitPoints(p))) gefahr = true;
          const LOCATION lo = u2.Location();
          for (int f = 0; f < z.feindPosN; ++f) if (abstand2(z.feindPosX[f], z.feindPosY[f], edX(z, lo), edY(z, lo)) < 12 * 12) gefahr = true;
        }
        if (gefahr) continue;
      }
      for (int i = 0; i < MAX_MINEN; ++i) if (!z.minenId[i]) { g = i; break; }
      if (g < 0) break;
      hinweis(z, "ki::erz/route-anlegen");
      z.minenGruppe[g] = CreateMiningGroup(Player[p]);
      z.minenId[g] = minen[beste].unitID;
      z.schmelzeId[g] = schmelzen[k].unitID;
      LOCATION ml = minen[beste].Location();
      MAP_RECT bereich(LOCATION(sl.x - 3, sl.y - 3), LOCATION(sl.x + 3, sl.y + 3));
      z.minenGruppe[g].Setup(ml, sl, bereich);
      melde(z, "mine", mapCommonOreMine, edX(z, ml), edY(z, ml), "Erzroute Mine %d,%d -> Schmelze %d,%d (%d. Route dieser Mine)",
            edX(z, ml), edY(z, ml), edX(z, sl), edY(z, sl), besteN + 1);
    }
    // Lastwagen je Route nach Entfernung (gemessen, lastwagenSoll)
    hinweis(z, "ki::erz/lastwagen");
    if (z.minenGruppe[g].TotalUnitCount() < lastwagenSoll(z, g)) {
      PlayerVehicleEnum e(p);
      UnitEx u;
      while (e.GetNext(u)) {
        if (u.GetType() != mapCargoTruck || u.IsBusy() || !u.IsLive()) continue;
        if (lastwagenVergeben(z, u.unitID)) continue;
        z.minenGruppe[g].TakeUnit(u);
        melde(z, "mine", mapCargoTruck, edX(z, u.Location()), edY(z, u.Location()), "Lastwagen %d faehrt Erz", u.unitID);
        break;
      }
    }
  }
}

// ---------------------------------------------------------------------------------------------------
// Seltenerz / rare ore (nur fuers Raumfahrtziel): Bake, Mine, Route zur Seltenerz-Schmelze.
// Spaceport, SULV und EDWARD kosten zusammen 1200 Seltenerz. Ohne Seltenerz-Schmelze (oder -Lager)
// kann die Kolonie keins speichern.
// Rare ore (space goal only): beacon, mine, route to the rare ore smelter.

// Seltenerz abbauen: fuers Satellitenziel oder sobald das gewaehlte Kampffahrzeug Seltenerz kostet (Panther, Tiger,
// bessere Waffen -- Spieler: "Wettruesten ... bessere Panther oder Tiger mit besseren Waffen"). Vorher nur Ziel 1.
// Mine rare ore for the satellite goal or as soon as the chosen combat vehicle needs it.
inline bool seltenAktiv(const Zustand& z) {
  if (!hatTech(z, TECH_SELTENERZ)) return false;
  if (z.ziel == 1 && !z.zielErreicht) return true;
  if (empMoeglich(z) && empLohnt(z)) return true;                 // EMP-Rakete kostet 1500 Seltenerz
  return seltenGewollt(z);
}

inline void seltenErz(Zustand& z, const Lage& l) {
  if (l.geb[mapRareOreSmelter] == 0) return;                      // Mine erst, wenn die Schmelze steht -- steht sie, wird sie betrieben
  const int p = z.spieler;
  const int t = TethysGame::Tick();
  if (l.geb[mapRareOreMine] == 0) {
    BakenFund f = bakeSuchen(z, 1);
    if (!f.beste.unitID) {
      if (selten(z, 11, 5000))
        melde(z, "warte", mapMiningBeacon, z.bx, z.by, "keine freie Seltenerz-Bake (gesehen %d, Seltenerz %d, frei %d)",
              f.gesehen, f.gewoehnlich, f.frei);
    } else {
      UnitEx b = f.beste;
      const LOCATION bl = b.Location();
      if (z.seltenBake != b.unitID) {
        z.seltenBake = b.unitID; z.seltenBakeSeit = t;
        melde(z, "info", mapMiningBeacon, edX(z, bl), edY(z, bl), "Seltenerz-Bake %d wird erschlossen", b.unitID);
      }
      const bool vermessen = (b.GetSurveyedBy() & (1 << p)) != 0;
      const bool surveyorDa = l.fz[mapRoboSurveyor] > 0;
      // Bauen, sobald vermessen -- oder ohne, wenn es keinen Surveyor gibt oder er zu lange braucht.
      const bool darf = vermessen || !surveyorDa || t - z.seltenBakeSeit > 1200;
      // Surveyor hinschicken, wenn er frei ist (wie beim gewoehnlichen Erz)
      if (!vermessen && selten(z, 14, 400)) {
        PlayerVehicleEnum e(p); UnitEx s;
        while (e.GetNext(s)) {
          if (s.GetType() != mapRoboSurveyor || s.IsBusy() || s.GetCurAction() != moDone) continue;
          LOCATION ziel = bl;
          if (abstand2(s.Location().x, s.Location().y, bl.x, bl.y) > 2) ziel.x += 1;
          s.DoMove(ziel);
          melde(z, "vermessen", mapMiningBeacon, edX(z, bl), edY(z, bl), "Surveyor %d faehrt zur Seltenerz-Bake", s.unitID);
          break;
        }
      }
      UnitEx m; m.unitID = z.seltenMiner;
      const bool lebt = z.seltenMiner && m.IsLive() && m.GetType() == mapRoboMiner && m.OwnerID() == p;
      if (!lebt) {
        z.seltenMiner = 0; z.seltenMinerSeit = 0;
        PlayerVehicleEnum e(p); UnitEx u;
        while (e.GetNext(u)) {
          if (u.GetType() != mapRoboMiner || u.IsBusy() || u.GetCurAction() != moDone) continue;
          if (u.unitID == z.bakeMiner && t - z.minerSeit < 300) continue;   // baut gerade gewoehnliches Erz
          z.seltenMiner = u.unitID;
          melde(z, "info", mapRoboMiner, edX(z, u.Location()), edY(z, u.Location()), "Robo-Miner %d fuer Seltenerz reserviert", u.unitID);
          break;
        }
        if (!z.seltenMiner && selten(z, 12, 3000))
          melde(z, "warte", mapRareOreMine, edX(z, bl), edY(z, bl), "kein freier Robo-Miner fuer die Seltenerz-Mine");
      } else if (darf && z.bauAn && z.seltenMinerSeit == 0) {
        // BuildingGroup setzt den Robo-Miner ein (Ort = Bake)
        z.bau.TakeUnit(m);
        LOCATION ort = bl;
        z.bau.RecordBuilding(ort, mapRareOreMine, mapNone);
        z.seltenMinerSeit = t;
        melde(z, "mine", mapRareOreMine, edX(z, bl), edY(z, bl), "BuildingGroup: Robo-Miner %d baut Seltenerz-Mine (%s)", m.unitID,
              vermessen ? "Bake vermessen" : "unvermessen");
      } else if (z.seltenMinerSeit && t - z.seltenMinerSeit > 3000 && !m.IsBusy() && m.GetCurAction() == moDone) {
        // Rueckfall: die Gruppe hat ihn nicht eingesetzt -> direkter Befehl wie beim gewoehnlichen Erz
        m.DoBuild(mapRareOreMine, LOCATION(bl.x + 1, bl.y + 1));
        z.seltenMinerSeit = t - 2000;
        melde(z, "fehler", mapRareOreMine, edX(z, bl), edY(z, bl), "BuildingGroup setzt Robo-Miner %d nicht ein -- direkter DoBuild", m.unitID);
      }
    }
  }
  // Mine -> Seltenerz-Schmelze, zwei Lastwagen je Mine
  UnitEx schmelze;
  { PlayerBuildingEnum e(p, mapRareOreSmelter); e.GetNext(schmelze); }
  if (!schmelze.unitID) return;
  PlayerBuildingEnum em(p, mapRareOreMine);
  UnitEx mine;
  while (em.GetNext(mine)) {
    int g = -1;
    for (int i = 0; i < 2; ++i) if (z.seltenMineId[i] == mine.unitID) g = i;
    if (g < 0) {
      for (int i = 0; i < 2; ++i) if (!z.seltenMineId[i]) { g = i; break; }
      if (g < 0) break;
      z.seltenGruppe[g] = CreateMiningGroup(Player[p]);
      z.seltenMineId[g] = mine.unitID;
      z.seltenSchmelzeId[g] = 0;
      melde(z, "fertig", mapRareOreMine, edX(z, mine.Location()), edY(z, mine.Location()), "Seltenerz-Mine steht");
    }
    if (z.seltenSchmelzeId[g] != schmelze.unitID) {
      LOCATION ml = mine.Location(), sl = schmelze.Location();
      MAP_RECT bereich(LOCATION(sl.x - 3, sl.y - 3), LOCATION(sl.x + 3, sl.y + 3));
      z.seltenGruppe[g].Setup(ml, sl, bereich);
      z.seltenSchmelzeId[g] = schmelze.unitID;
      melde(z, "mine", mapRareOreMine, edX(z, ml), edY(z, ml), "Seltenerz-Route Mine -> Schmelze %d,%d", edX(z, sl), edY(z, sl));
    }
    if (z.seltenGruppe[g].TotalUnitCount() < 2) {
      PlayerVehicleEnum e(p);
      UnitEx u;
      while (e.GetNext(u)) {
        if (u.GetType() != mapCargoTruck || u.IsBusy() || !u.IsLive()) continue;
        if (lastwagenVergeben(z, u.unitID)) continue;
        z.seltenGruppe[g].TakeUnit(u);
        melde(z, "mine", mapCargoTruck, edX(z, u.Location()), edY(z, u.Location()), "Lastwagen %d faehrt Seltenerz", u.unitID);
        break;
      }
      // Kein freier Lastwagen und keiner baubar (ein Lastwagen kostet im Multiplayer-Techbaum 100 Seltenerz -- ohne
      // Seltenerz-Route also nie): eine Erzroute mit mindestens zwei Lastwagen gibt einen ab.
      // No free truck (a truck costs rare ore): a common-ore route with two or more trucks hands one over.
      if (z.seltenGruppe[g].TotalUnitCount() == 0) {
        for (int i = 0; i < MAX_MINEN; ++i) {
          if (!z.minenId[i] || z.minenGruppe[i].TotalUnitCount() < 2) continue;
          UnitEx abgeben; { GroupEnumerator ge(z.minenGruppe[i]); UnitEx v; while (ge.GetNext(v)) if (v.IsLive() && v.GetType() == mapCargoTruck) abgeben = v; }
          if (!abgeben.unitID) continue;
          z.minenGruppe[i].RemoveUnit(abgeben);
          z.seltenGruppe[g].TakeUnit(abgeben);
          melde(z, "mine", mapCargoTruck, edX(z, abgeben.Location()), edY(z, abgeben.Location()),
                "Lastwagen %d wechselt von der Erzroute zur Seltenerz-Route (kein Lastwagen baubar ohne Seltenerz)", abgeben.unitID);
          break;
        }
      }
    }
  }
}

// ---------------------------------------------------------------------------------------------------
// Fahrzeugfabrik / vehicle factory

// Fahrzeuge nach Dringlichkeit (Spieler: keine feste Liste): jeder Typ bekommt einen Wert aus offenen Aufgaben,
// gebaut wird der hoechste. 0 = nichts noetig (dann legt fahrzeuge() die Fabrik still).
// Vehicles by urgency from open tasks; highest wins, 0 = nothing needed.
inline int fahrzeugWahl(const Zustand& z, const Lage& l, const char** warum) {
  struct K { int typ, wert; const char* warum; };
  K beste = { mapNone, 0, "" };
  // Was Seltenerz kostet, das gerade fehlt, kommt nicht in Frage -- sonst blockiert es alles dahinter (Eskalationstest:
  // Lastwagen braucht 100 Seltenerz, wartete ewig, der Robo-Miner fuer die Seltenerz-Mine kam nie).
  // Skip vehicles whose rare-ore cost cannot be paid now; otherwise they block everything behind them.
  auto pruefe = [&](int typ, int wert, const char* w) {
    if (wert <= beste.wert) return;
    UnitInfo ui((map_id)typ);
    if (ui.GetRareOreCost(z.spieler) > l.selten) return;
    beste = { typ, wert, w };
  };
  int offen = 0;
  for (int i = 0; i < z.anzahlPlaetze; ++i) if (z.plaetze[i].zustand == P_FAHRT || z.plaetze[i].zustand == P_BAUT) offen++;
  // ConVecs: ohne keinen Bau; zwei, damit einer Bausaetze holt, waehrend der andere baut; ein dritter, wenn
  // Erz uebrig ist und Bauten anstehen
  if (l.fz[mapConVec] == 0) pruefe(mapConVec, 100, "kein ConVec -- ohne ihn wird nichts gebaut");
  else if (l.fz[mapConVec] < 2) pruefe(mapConVec, 60, "zweiter ConVec: einer holt Bausaetze, einer baut");
  else if (l.fz[mapConVec] < 3 && offen > 0 && z.erzUeberschuss) pruefe(mapConVec, 20, "dritter ConVec: Bauten stehen an, Erz reicht");
  // Earthworker: Roehren (BuildingGroup) und Mauern
  if (l.fz[mapEarthworker] == 0) pruefe(mapEarthworker, 70, "Roehren und Mauern brauchen einen Earthworker");
  // Lastwagen: zwei je Erz- und Seltenerz-Route; dringender, je groesser der Engpass
  {
    const Bilanz bl = bilanz(z, l);
    const int routen = (l.geb[mapCommonOreSmelter] < 2 * l.geb[mapCommonOreMine] ? l.geb[mapCommonOreSmelter] : 2 * l.geb[mapCommonOreMine])
                       + (l.geb[mapRareOreSmelter] > 0 ? l.geb[mapRareOreMine] : 0);
    // Soll je angelegter Erzroute nach Entfernung, sonst (Route noch nicht angelegt, Seltenerz) zwei
    int soll = 0, angelegt = 0;
    for (int i = 0; i < MAX_MINEN; ++i) if (z.minenId[i]) { soll += lastwagenSoll(z, i); angelegt++; }
    soll += 2 * (routen > angelegt ? routen - angelegt : 0);
    const int fehlen = soll - l.fz[mapCargoTruck];
    if (fehlen > 0) {
      int eng = 0;
      if (bl.bedarfErz > 0 && bl.engpassErz > 0) eng = bl.engpassErz * 100 / bl.bedarfErz;
      if (bl.bedarfSelten > 0 && bl.engpassSelten > 0 && bl.engpassSelten * 100 / bl.bedarfSelten > eng) eng = bl.engpassSelten * 100 / bl.bedarfSelten;
      // Ohne Engpass reicht ein Lastwagen je Route -- weitere erst, wenn Erz fehlt (volles Lager braucht keinen)
      if (eng > 0 || l.fz[mapCargoTruck] < routen)
        pruefe(mapCargoTruck, 40 + (eng > 100 ? 100 : eng) / 2, "Lastwagen fehlen auf den Erzrouten");
    }
    // Neue gewoehnliche Mine: Schmelzen-Kapazitaet der Minen voll und Erz fehlt
    if (l.fz[mapRoboMiner] == 0 && l.geb[mapCommonOreMine] < MAX_MINEN && l.geb[mapCommonOreSmelter] >= PAR(z).schmelzenJeMine * l.geb[mapCommonOreMine] &&
        bl.engpassErz > 0 && bl.bedarfErz > 0)
      pruefe(mapRoboMiner, 30 + (bl.engpassErz * 100 / bl.bedarfErz > 100 ? 100 : bl.engpassErz * 100 / bl.bedarfErz) / 2,
             "neue Mine -- Bilanz: Erz fehlt, Minen ausgelastet");
    // Seltenerz-Mine, sobald die Seltenerz-Schmelze steht
    if (l.geb[mapRareOreSmelter] > 0 && l.geb[mapRareOreMine] == 0 && l.fz[mapRoboMiner] == 0)
      pruefe(mapRoboMiner, 50, "Seltenerz-Mine fuer die Seltenerz-Schmelze");
  }
  if (l.fz[mapRoboSurveyor] == 0 && l.geb[mapCommonOreMine] < MAX_MINEN) pruefe(mapRoboSurveyor, 15, "Baken vermessen");
  // Kampffahrzeuge: so dringend wie die Verteidigung (oder die Aussenposten/Angriffstruppe)
  {
    const int fz = fahrgestellWahl(z, l.selten);
    const int zahl = l.fz[mapLynx] + l.fz[mapPanther] + l.fz[mapTiger];
    const int fehlt = kampfFehlt(z, l), mob = mobilSoll(z) - z.eigeneMobil - (istKampfFz(z.fzTyp) ? WERT_LYNX : 0);
    (void)zahl;
    if (fz && (fehlt > 0 || mob > 0)) {
      int wert = 0;
      if (fehlt > 0) wert = z.bedrohung * (fehlt * 100 / (z.sollArmee > 0 ? z.sollArmee : fehlt) > 100 ? 100 : fehlt * 100 / (z.sollArmee > 0 ? z.sollArmee : fehlt)) / 100;
      if (mob > 0 && wert < 25) wert = 25;
      static char t[140];
      std::snprintf(t, sizeof(t), "Kampffahrzeug: Feind Wert %d (+%.1f je 1000 Ticks), eigener %d, Soll %d, mobil %d/%d",
                    z.feindArmee, z.feindWachstum, z.eigeneArmee, z.sollArmee, z.eigeneMobil, mobilSoll(z));
      pruefe(fz, wert, t);
    }
  }
  // Reparatur nach Erwartung (Test 2026-11-09: repariert der ConVec Gebaeude sofort, sah die KI keinen Schaden mehr und baute nie
  // ein Reparaturfahrzeug -- Edens Lynx blieben kaputt): gebraucht, sobald ein Feind bekannt ist (Kampffahrzeuge werden Schaden
  // nehmen) oder etwas beschaedigt ist; ein weiteres je 15 Kampffahrzeuge. Expected need, not the damage of the moment.
  if (hatTech(z, 5051)) {
    int schaden = 0, kampfFz = 0;
    for (int ty : GEBAEUDE) {
      PlayerBuildingEnum e(z.spieler, (map_id)ty);
      UnitEx b;
      while (e.GetNext(b)) { const int a2 = b.GetCurAction(); if (a2 != moObjWait && a2 != moDevelop && b.GetDamage() > 0) schaden++; }
    }
    {
      PlayerVehicleEnum e(z.spieler); UnitEx v;
      while (e.GetNext(v)) if (v.IsLive() && kaempfer(v) && v.GetType() != mapScorpion) {
        kampfFz++;
        UnitInfo i((map_id)v.GetType()); const int hp = i.GetHitPoints(z.spieler); if (hp > 0 && v.GetDamage() * 100 / hp > 50) schaden++;
      }
    }
    const int rep = l.fz[mapRepairVehicle] + l.fz[mapSpider];
    if (rep == 0 && (schaden > 0 || z.feindGemeldet || z.bedrohung > 0)) pruefe(reparaturTyp(z), 50, "Reparaturfahrzeug: Feind bekannt oder etwas beschaedigt");
    else if (rep > 0 && rep < 1 + kampfFz / 15) pruefe(reparaturTyp(z), 30, "weiteres Reparaturfahrzeug: je 15 Kampffahrzeuge eins");
    // Spinnen fuer die Uebernahme: nur Plymouth, nur wenn EMP im Einsatz ist (Truppe oder Rakete)
    if (!Player[z.spieler].IsEden() && (z.zielEmp > 0 || z.empRakete) && l.fz[mapSpider] < 1 + (z.zielEmp > 0 ? z.zielEmp : 2) / 2 + 1)
      pruefe(mapSpider, 35, "Spinnen: uebernehmen betaeubte Feindfahrzeuge");
  }
  *warum = beste.warum;
  return beste.typ;
}

inline int fahrzeugBedarf(const Zustand& z, const Lage& l, const char** warum) {
  struct K { int typ, wert; };
  // Dringlichkeit des wichtigsten Fahrzeugs (fuer die Wirtschafts-Dringlichkeit, wenn die Fabrik fehlt)
  const char* w = "";
  const int typ = fahrzeugWahl(z, l, &w);
  *warum = w;
  if (typ == mapNone) return 0;
  if (typ == mapConVec && l.fz[mapConVec] == 0) return 100;
  return typ == mapCargoTruck || typ == mapRoboMiner ? 60 : istKampfFz(typ) ? z.bedrohung : 40;
}

// Weitere Fahrzeugfabriken bauen nur Kampffahrzeuge (Fabrikgruppe): ohne Kampfbedarf stilllegen (Arbeiter frei),
// mit Bedarf wieder einschalten. Further factories: idle without combat demand, unidle with it.
inline void weitereFabriken(Zustand& z, const Lage& l) {
  for (int j = 1; j < l.vfAnzahl; ++j) {
    UnitEx v = l.vfs[j];
    if (!v.IsLive() || v.GetCurAction() != moDone) continue;
    int still = -1;
    for (int i = 0; i < MAX_VF; ++i) if (z.fabrikStillW[i] == v.unitID) still = i;
    if (still < 0 && !z.gruppenBedarf && !standardVolk(l) && !v.IsBusy() && v.HasWorkers()) {
      for (int i = 0; i < MAX_VF; ++i) {
        UnitEx alt; alt.unitID = z.fabrikStillW[i];
        if (z.fabrikStillW[i] && alt.IsLive() && alt.GetType() == mapVehicleFactory) continue;
        v.DoIdle();
        z.fabrikStillW[i] = v.unitID;
        melde(z, "arbeit", mapVehicleFactory, edX(z, v.Location()), edY(z, v.Location()), "weitere Fahrzeugfabrik stillgelegt -- Kampfgruppen voll");
        break;
      }
    } else if (still >= 0 && z.gruppenBedarf) {
      if (!v.IsBusy()) { z.fabrikStillW[still] = 0; continue; }      // laeuft schon wieder
      UnitInfo vi(mapVehicleFactory);
      if (l.arbFrei < vi.GetWorkersRequired(z.spieler)) continue;     // Arbeiter fehlen: erst die erste Fabrik
      v.DoUnIdle();
      z.fabrikStillW[still] = 0;
      melde(z, "arbeit", mapVehicleFactory, edX(z, v.Location()), edY(z, v.Location()), "weitere Fahrzeugfabrik eingeschaltet -- Kampfgruppen brauchen Fahrzeuge");
    }
  }
}

inline void fahrzeuge(Zustand& z, const Lage& l) {
  if (!l.vf.unitID) return;
  weitereFabriken(z, l);
  UnitEx vf = l.vf;
  const int t = TethysGame::Tick();
  if (z.fzTyp) {
    if (t - z.fzSeit > 100 && t - z.fzSeit < 100 + TAKT + 1 && vf.GetCurAction() == moDone && !vf.IsBusy()) {
      melde(z, "warte", z.fzTyp, edX(z, vf.Location()), edY(z, vf.Location()),
            "Fahrzeugfabrik nimmt den Auftrag nicht an (Strom %d, Arbeiter %d) -- in 500 Ticks erneut",
            vf.HasPower(), vf.HasWorkers());
      z.fzTyp = 0; z.fzPauseBis = t + 500;
      return;
    }
    if (vf.GetCurAction() == moDevelop || vf.IsBusy()) {
      if (t - z.fzSeit > GEDULD_KIT) { melde(z, "fehler", z.fzTyp, edX(z, vf.Location()), edY(z, vf.Location()), "Fahrzeug dauert zu lange"); z.fzTyp = 0; }
      return;
    }
    z.fzTyp = 0;
  }
  const char* warum = "";
  int typ = fahrzeugWahl(z, l, &warum);
  // Kampffahrzeuge baut die Fabrikgruppe fuer Wache und Sammelgruppe (RecordVehReinforceGroup) -- hier nur die
  // Wirtschaftsfahrzeuge. Combat vehicles come from the factory group's reinforcement.
  if (istKampfFz(typ)) typ = mapNone;
  // Keine neuen Fahrzeuge noetig: Fabrik stilllegen (gibt 3 Arbeiter frei), bei Bedarf wieder einschalten.
  // Spieler: "Fahrzeugfabriken stilllegen, wenn keine neuen Fahrzeuge benoetigt werden."
  // No vehicles needed: idle the factory (frees 3 workers); unidle when needed.
  if (z.fabrikStill == vf.unitID) {
    if (typ == mapNone && !z.gruppenBedarf) return;
    if (typ == mapNone) { typ = mapLynx; warum = "Kampfgruppen brauchen Fahrzeuge (Fabrikgruppe)"; }   // nur zum Wecken
    if (!vf.IsBusy()) { z.fabrikStill = 0; return; }           // laeuft wieder -- naechster Schritt produziert
    UnitInfo vfInfo(mapVehicleFactory);
    const int braucht = vfInfo.GetWorkersRequired(z.spieler);
    if (l.arbFrei < braucht) {
      // DoUnIdle wirkt nur mit genug freien Arbeitern -> Wunsch an die Rangliste
      z.wunschArbeiter = braucht; z.wunschRang = rang(z, l, mapVehicleFactory) + (seltenAktiv(z) ? 10 : 0); z.wunschSeit = TethysGame::Tick();
      if (selten(z, 25, 2000))
        melde(z, "warte", mapVehicleFactory, edX(z, vf.Location()), edY(z, vf.Location()), "Fahrzeugfabrik gebraucht (%s), aber nur %d von %d Arbeitern frei", warum, l.arbFrei, braucht);
      return;
    }
    vf.DoUnIdle();
    z.wunschArbeiter = 0;
    melde(z, "arbeit", mapVehicleFactory, edX(z, vf.Location()), edY(z, vf.Location()), "Fahrzeugfabrik wird eingeschaltet: %s", warum);
    return;
  }
  if (vf.IsBusy() || t < z.fzPauseBis) return;
  if (typ == mapNone) {
    if (z.gruppenBedarf) return;                                   // Fabrikgruppe soll Kampffahrzeuge bauen
    if (!standardVolk(l) && vf.GetCurAction() == moDone && vf.HasWorkers()) {
      vf.DoIdle();
      z.fabrikStill = vf.unitID;
      melde(z, "arbeit", mapVehicleFactory, edX(z, vf.Location()), edY(z, vf.Location()), "Fahrzeugfabrik stillgelegt -- keine neuen Fahrzeuge noetig");
    }
    return;
  }
  UnitInfo info((map_id)typ);
  const int waffe = istKampfFz(typ) ? waffeWahl(z, typ, l.selten - info.GetRareOreCost(z.spieler)) : (int)mapNone;
  int kosten = info.GetOreCost(z.spieler), kostenSelten = info.GetRareOreCost(z.spieler);
  if (waffe != mapNone) { UnitInfo wi((map_id)waffe); kosten += wi.GetOreCost(z.spieler); kostenSelten += wi.GetRareOreCost(z.spieler); }
  const int erzFrei = l.erz;
  if (kosten > erzFrei || kostenSelten > l.selten) {
    if (selten(z, 2, 1500)) melde(z, "warte", typ, edX(z, vf.Location()), edY(z, vf.Location()), "Erz %d/%d, Seltenerz %d/%d fuer %s", erzFrei, kosten, l.selten, kostenSelten, name(typ));
    return;
  }
  vf.DoProduce((map_id)typ, (map_id)waffe);
  z.fzTyp = typ; z.fzSeit = t;
  if (istKampfFz(typ)) z.letztesKampfFz = t;
  if (waffe != mapNone)
    melde(z, "fahrzeug", typ, edX(z, vf.Location()), edY(z, vf.Location()), "%s mit %s (SDK-Wert %d): %s", name(typ), name(waffe), sdkWertT(typ, waffe), warum);
  else
    melde(z, "fahrzeug", typ, edX(z, vf.Location()), edY(z, vf.Location()), "%s", warum);
}

// Kampffuehrung: eigene Kampffahrzeuge in Gruppen. Die Wache (FightGroup) haelt die Kolonie; Aussenposten
// bekommen kleine eigene Gruppen. Die Wache hat drei Modi:
//   Wache    -- bewacht die Basis (DoGuardRect)
//   Abfangen -- eine anrueckende Welle, die die Wache schlagen kann, unterwegs stellen (Spieler: "Beide greifen
//               an und treffen sich unterwegs")
//   Angriff  -- nach dem Ziel, ohne Feind unterwegs und mit doppelter Ueberlegenheit gegen die Wachtuerme:
//               auf die Feindbasis. Abbruch, wenn die Truppe auf die Haelfte faellt.
// Combat command: guard group holds the colony, outposts get small groups; modes guard / intercept / attack.
// Wachen: DoGuardRect LOESCHT alle Zonen der Gruppe -- darum zuerst DoGuardRect, dann Wartezone (SetRect) und
// bewachte Zonen (AddGuardedRect). Andersherum ist die Wartezone (0,0,0,0) und die Fahrzeuge fahren nach oben links
// (Forum outpost2.net, Test KI_TestZR). Bei mehreren angegriffenen Zonen faehrt die ganze Gruppe nacheinander hin.
// Guarding: DoGuardRect clears all zones, so call it first, then SetRect and AddGuardedRect.
// Eine Gruppe fasst hoechstens 32 Einheiten. TakeUnit und TakeAllUnits in eine volle Gruppe loeschen den Ueberschuss ohne
// Meldung aus dem Spiel (Test KI_TestGruppe 2026-11-07; in den Duellen 64 -> 32 Fahrzeuge beim Losschlagen).
// A group holds 32 units; taking more silently deletes the surplus from the game.
constexpr int GRUPPE_MAX = 32;
inline bool sicherNehmen(const Zustand& z, ScGroup& g, Unit u) {
  if (g.TotalUnitCount() >= GRUPPE_MAX) return false;
  g.TakeUnit(u);
  return true;
}
// Alles aus quelle, soweit Platz ist; der Rest bleibt in quelle. Liefert, wie viele dort bleiben.
inline int sicherAlle(const Zustand& z, ScGroup& ziel, ScGroup& quelle) {
  if (ziel.TotalUnitCount() + quelle.TotalUnitCount() <= GRUPPE_MAX) { ziel.TakeAllUnits(quelle); return 0; }
  UnitEx v[GRUPPE_MAX]; int n = 0;
  { GroupEnumerator ge(quelle); UnitEx u; while (ge.GetNext(u) && n < GRUPPE_MAX) if (u.IsLive()) v[n++] = u; }
  for (int i = 0; i < n && ziel.TotalUnitCount() < GRUPPE_MAX; ++i) { quelle.RemoveUnit(v[i]); ziel.TakeUnit(v[i]); }
  return quelle.TotalUnitCount();
}

// Alle Fahrzeuge des Angriffs, ueber beide Gruppen. All attack vehicles across both groups.
struct AngriffEnum {
  GroupEnumerator a, b; bool zweite; const bool hatZweite;
  explicit AngriffEnum(Zustand& z) : a(z.angriff), b(z.angriff2), zweite(false), hatZweite(z.angriff2An != 0) {}
  bool GetNext(UnitEx& u) {
    if (!zweite) { if (a.GetNext(u)) return true; zweite = true; }
    return hatZweite && b.GetNext(u);
  }
};
inline int gruppenWert(ScGroup& g, int& n);
inline int angriffWert(Zustand& z, int& n) {
  int n1 = 0, n2 = 0;
  const int w = gruppenWert(z.angriff, n1) + (z.angriff2An ? gruppenWert(z.angriff2, n2) : 0);
  n = n1 + n2;
  return w;
}
// Aus der Angriffsgruppe entfernen, in der die Einheit wirklich ist: RemoveUnit fuer ein fremdes Mitglied stuerzte im Test ab.
inline bool inGruppeG(ScGroup& g, int id);
inline void angriffRaus(Zustand& z, Unit u) {
  if (inGruppeG(z.angriff, u.unitID)) z.angriff.RemoveUnit(u);
  else if (z.angriff2An && inGruppeG(z.angriff2, u.unitID)) z.angriff2.RemoveUnit(u);
}

inline void wachen(FightGroup& g, MAP_RECT warte, const MAP_RECT* bewacht, int n) {
  g.DoGuardRect();
  g.SetRect(warte);
  for (int i = 0; i < n; ++i) { MAP_RECT r = bewacht[i]; g.AddGuardedRect(r); }
}

inline int gruppenWert(ScGroup& g, int& n);
// --- Angriffsbilanz: was hat ein Angriff gekostet (Einsatz in Erz und Seltenerz), was kam zurueck, was wurde
// zerstoert -- je Zusammenstellung (Fahrgestell + Waffe) beider Seiten. Nur Messung fuer die spaetere Rangliste
// "was hilft gegen was"; aendert kein Verhalten. Attack balance sheet, measurement only.
inline void mixDazu(short* typ, short* wf, short* anz, int& n, int t, int w) {
  for (int i = 0; i < n; ++i) if (typ[i] == t && wf[i] == w) { anz[i]++; return; }
  if (n < 12) { typ[n] = (short)t; wf[n] = (short)w; anz[n] = 1; n++; }
}
inline void fzKosten(int spieler, UnitEx& v, int& erz, int& sel) {
  UnitInfo c((map_id)v.GetType());
  erz += c.GetOreCost(spieler); sel += c.GetRareOreCost(spieler);
  const int w = (int)v.GetWeapon();
  if (w > 0 && w != mapNone && w < 128) { UnitInfo wi((map_id)w); erz += wi.GetOreCost(spieler); sel += wi.GetRareOreCost(spieler); }
}
// Wert und Zusammenstellung der Angriffsgruppe (jede Einheit einmal, alle Fahrzeuge -- auch Spinnen)
inline void angriffsGruppe(Zustand& z, int& erz, int& sel, short* typ, short* wf, short* anz, int& n) {
  erz = sel = 0; n = 0;
  int ids[64]; int ni = 0;
  AngriffEnum ge(z);
  UnitEx v;
  while (ge.GetNext(v)) {
    if (!v.IsLive() || !v.IsVehicle()) continue;
    bool schon = false;
    for (int i = 0; i < ni; ++i) if (ids[i] == v.unitID) { schon = true; break; }
    if (schon) continue;
    if (ni < 64) ids[ni++] = v.unitID;
    fzKosten(z.spieler, v, erz, sel);
    if (typ) mixDazu(typ, wf, anz, n, v.GetType(), (int)v.GetWeapon());
  }
}
inline void bilanzStart(Zustand& z) {
  z.bilStart = TethysGame::Tick(); z.bilEmp = z.empRakete ? 1 : 0;
  angriffsGruppe(z, z.bilEinsatzErz, z.bilEinsatzSelten, z.bilTyp, z.bilWaffe, z.bilAnz, z.bilN);
  { int nn = 0; z.bilE = angriffWert(z, nn); z.bilGruppe = nn; }
  z.bilArmee = z.feindArmee; z.bilKontakt = 0; z.bilErste = 0; z.bilNach = -1;
  z.bilZerHeim = z.zerstoertHeim; z.bilVerHeim = z.verlorenHeim;
  for (int i = 0; i < 6; ++i) z.bilK[i] = -1; for (int i = 0; i < 3; ++i) z.bilK2[i] = -1; z.bilKTick = 0;
  z.bilRueck = 0; z.bilRueckErz = 0; z.bilUeber = 0;
  z.bilD = z.feindArmee;
  z.bilFN = 0; z.bilFeindWT = 0;
  for (int p = 0; p < TethysGame::NoPlayers(); ++p) {
    if (p == z.spieler || ExtPlayer[z.spieler].IsAlliedTo(p)) continue;
    { PlayerVehicleEnum e(p); UnitEx v;
      while (e.GetNext(v)) if (v.IsLive() && kampfWert(v) > 0) mixDazu(z.bilFTyp, z.bilFWaffe, z.bilFAnz, z.bilFN, v.GetType(), (int)v.GetWeapon()); }
    { PlayerBuildingEnum e(p, mapGuardPost); UnitEx g; while (e.GetNext(g)) if (g.IsLive()) { z.bilFeindWT++; z.bilD += kampfWert(g); } }
  }
}
// Ankunft der Truppe am Ziel: wie viele sind beim ersten Kontakt in 9 Kacheln, wie viele 200 Ticks spaeter?
inline void bilanzAnkunft(Zustand& z) {
  if (!z.bilStart || !z.angriffZiel) return;
  UnitEx ziel; ziel.unitID = z.angriffZiel;
  if (!ziel.IsLive()) return;
  const LOCATION zl = ziel.Location();
  int nah = 0; AngriffEnum ge(z); UnitEx v;
  while (ge.GetNext(v)) if (v.IsLive() && v.IsVehicle()) { const LOCATION l = v.Location(); if (abstand2(l.x, l.y, zl.x, zl.y) <= 9 * 9) nah++; }
  const int t = TethysGame::Tick();
  if (!z.bilKontakt) { if (nah) { z.bilKontakt = t; z.bilErste = nah; } }
  else if (z.bilNach < 0 && t - z.bilKontakt >= 200) z.bilNach = nah;
}
// Erster Kontakt mit Feindeinheiten: wie viel der Truppe ist dann in der Naehe, wie stark ist der Feind dort?
// First contact: how much of the group is close, and how strong is the local enemy?
inline void kontaktStaerken(Zustand& z, int kx, int ky, int& nah, int& alle, int& e, int& d) {
  nah = alle = e = d = 0;
  int ids[64], ni = 0; AngriffEnum ge(z); UnitEx v;
  while (ge.GetNext(v)) {
    if (!v.IsLive() || !v.IsVehicle()) continue;
    bool schon = false; for (int i = 0; i < ni; ++i) if (ids[i] == v.unitID) schon = true;
    if (schon) continue;
    if (ni < 64) ids[ni++] = v.unitID;
    alle++; const LOCATION l = v.Location();
    if (abstand2(l.x, l.y, kx, ky) <= 12 * 12) { nah++; e += kampfWert(v); }
  }
  for (int p = 0; p < TethysGame::NoPlayers(); ++p) {
    if (p == z.spieler || ExtPlayer[z.spieler].IsAlliedTo(p)) continue;
    { PlayerVehicleEnum en(p); UnitEx u; while (en.GetNext(u)) if (u.IsLive() && kampfWert(u) > 0) { const LOCATION l = u.Location(); if (abstand2(l.x, l.y, kx, ky) <= 12 * 12) d += kampfWert(u); } }
    { PlayerBuildingEnum en(p, mapGuardPost); UnitEx u; while (en.GetNext(u)) if (u.IsLive()) { const LOCATION l = u.Location(); if (abstand2(l.x, l.y, kx, ky) <= 12 * 12) d += kampfWert(u); } }
  }
}
inline void bilanzKontakt(Zustand& z) {
  if (!z.bilStart || z.bilK2[0] >= 0) return;
  const int t = TethysGame::Tick();
  if (z.bilK[0] >= 0) {
    if (t - z.bilKTick < 300) return;
    int nah, alle, e, d; kontaktStaerken(z, z.bilK[4] >> 16, z.bilK[4] & 0xffff, nah, alle, e, d);
    z.bilK2[0] = nah; z.bilK2[1] = e; z.bilK2[2] = d;
    return;
  }
  AngriffEnum ge(z); UnitEx v; UnitEx fund; bool gef = false;
  while (ge.GetNext(v) && !gef) {
    if (!v.IsLive() || !v.IsVehicle()) continue;
    const LOCATION l = v.Location();
    for (int p = 0; p < TethysGame::NoPlayers() && !gef; ++p) {
      if (p == z.spieler || ExtPlayer[z.spieler].IsAlliedTo(p)) continue;
      { PlayerVehicleEnum en(p); UnitEx u; while (en.GetNext(u) && !gef) if (u.IsLive() && kampfWert(u) > 0) { const LOCATION m = u.Location(); if (abstand2(l.x, l.y, m.x, m.y) <= 9 * 9) { gef = true; fund = v; } } }
      { PlayerBuildingEnum en(p, mapGuardPost); UnitEx u; while (en.GetNext(u) && !gef) if (u.IsLive()) { const LOCATION m = u.Location(); if (abstand2(l.x, l.y, m.x, m.y) <= 9 * 9) { gef = true; fund = v; } } }
    }
  }
  if (!gef) return;
  const LOCATION fl = fund.Location();
  int nah, alle, e, d; kontaktStaerken(z, fl.x, fl.y, nah, alle, e, d);
  z.bilK[0] = t - z.bilStart; z.bilK[1] = z.kampfModus; z.bilK[2] = nah; z.bilK[3] = alle; z.bilK[4] = (fl.x << 16) | (fl.y & 0xffff); z.bilK[5] = e | (d << 16);
  z.bilKTick = t;
}
// Rueckzug (Spieler: nur leicht Beschaedigte -- schwer Beschaedigte sind zu langsam, Fahrzeit ~ 1 + Schadensanteil, und
// kaempfen besser weiter). Wenn die Truppe nach dem ersten Kontakt vor Ort unterliegt, fahren die mit >= 50 % HP heim.
// Retreat: once locally outmatched, vehicles with at least half their HP go home; the rest fights on.
inline void rueckzugPruefen(Zustand& z, FightGroup& wache) {
  if (z.bilRueck || !z.bilStart || z.bilK[0] < 0) return;
  const int t = TethysGame::Tick();
  if (t % 100 >= TAKT) return;
  int mx = 0, my = 0, k = 0;
  { AngriffEnum ge(z); UnitEx v; while (ge.GetNext(v)) if (v.IsLive() && v.IsVehicle()) { const LOCATION l = v.Location(); mx += l.x; my += l.y; k++; } }
  if (!k) return;
  int nah, alle, e, d; kontaktStaerken(z, mx / k, my / k, nah, alle, e, d);
  if (d <= 0 || e >= d * PAR(z).rueckzugAb) return;
  z.bilRueck = 1;
  UnitEx raus[64]; int n = 0;
  { AngriffEnum ge(z); UnitEx v;
    while (ge.GetNext(v) && n < 64) {
      if (!v.IsLive() || !v.IsVehicle() || !kaempfer(v)) continue;
      bool schon = false; for (int i = 0; i < n; ++i) if (raus[i].unitID == v.unitID) schon = true;
      if (schon) continue;
      UnitInfo c((map_id)v.GetType()); const int hp = c.GetHitPoints(z.spieler);
      if (hp > 0 && v.GetDamage() * 2 <= hp) raus[n++] = v;
    } }
  const LOCATION heim = engine(z, z.bx, z.by + 4);
  for (int i = 0; i < n; ++i) {
    int erz = 0, sel = 0; fzKosten(z.spieler, raus[i], erz, sel); z.bilRueckErz += erz;
    if (wache.TotalUnitCount() >= GRUPPE_MAX) { raus[i].DoMove(heim); continue; }   // bleibt in der Angriffsgruppe
    angriffRaus(z, raus[i]); wache.TakeUnit(raus[i]); raus[i].DoMove(heim);
  }
  melde(z, "kampf", mapLynx, mx / k - z.ux, my / k - z.uy, "Rueckzug: %d von %d Fahrzeugen fahren heim (vor Ort eigene Staerke %d gegen %d)", n, alle, e, d);
}
// Verfolgung: beschaedigte Feinde in Basisnaehe (zurueckweichend oder nicht) ausserhalb ihrer Tuerme werden von wenigen
// gesunden Wachfahrzeugen gejagt -- beschaedigte Fahrzeuge sind langsam. Pursuit of damaged enemies near the base.
inline void verfolgen(Zustand& z) {
  const int t = TethysGame::Tick();
  if (t - z.jagdTakt < 100) return;
  z.jagdTakt = t;
  UnitEx ziel; bool hat = false; int best = 1 << 30;
  for (int p = 0; p < TethysGame::NoPlayers(); ++p) {
    if (p == z.spieler || ExtPlayer[z.spieler].IsAlliedTo(p)) continue;
    PlayerVehicleEnum e(p); UnitEx u;
    while (e.GetNext(u)) {
      if (!u.IsLive() || kampfWert(u) <= 0) continue;
      UnitInfo c((map_id)u.GetType()); const int hp = c.GetHitPoints(p);
      if (hp <= 0 || u.GetDamage() * 10 < hp * 4) continue;              // unter 60 % HP
      const LOCATION l = u.Location(); const int x = edX(z, l), y = edY(z, l);
      const int d = abstand2(x, y, z.bx, z.by);
      if (d > 40 * 40 || d >= best) continue;
      bool turm = false;
      { PlayerBuildingEnum g(p, mapGuardPost); UnitEx w; while (g.GetNext(w) && !turm) if (w.IsLive()) { const LOCATION wl = w.Location(); if (abstand2(wl.x, wl.y, l.x, l.y) <= 8 * 8) turm = true; } }
      if (turm) continue;
      best = d; ziel = u; hat = true;
    }
  }
  if (!hat) { z.jagdZiel = 0; return; }
  if (z.jagdZiel != ziel.unitID) { z.jagdZiel = ziel.unitID; z.jagdSeit = t; }
  const LOCATION zl = ziel.Location();
  // bis zu 4 gesunde, freie Wachfahrzeuge, die naechsten zuerst
  UnitEx jg[4]; int dj[4], nj = 0;
  GroupEnumerator ge(z.wache); UnitEx v;
  while (ge.GetNext(v)) {
    if (!v.IsLive() || !kaempfer(v)) continue;
    UnitInfo c((map_id)v.GetType()); const int hp = c.GetHitPoints(z.spieler);
    if (hp <= 0 || v.GetDamage() * 2 > hp) continue;
    const LOCATION l = v.Location(); const int d = abstand2(l.x, l.y, zl.x, zl.y);
    if (nj == 4 && dj[3] <= d) continue;                // volle Liste: nur Naehere (vorher las das Fuellen von Platz 3 dj[3] ungeschrieben)
    int k = nj < 4 ? nj++ : 3;
    jg[k] = v; dj[k] = d;
    while (k > 0 && dj[k] < dj[k - 1]) { UnitEx hu = jg[k]; int hd = dj[k]; jg[k] = jg[k - 1]; dj[k] = dj[k - 1]; jg[k - 1] = hu; dj[k - 1] = hd; --k; }
  }
  for (int i = 0; i < nj; ++i) {
    UnitInfo wi((map_id)jg[i].GetWeapon()); const int rw = wi.GetWeaponRange(z.spieler) > 0 ? wi.GetWeaponRange(z.spieler) : 4;
    if (dj[i] <= rw * rw) jg[i].Unit::DoAttack(ziel); else jg[i].DoMove(zl);
  }
}
// Spinnen (Test KI_TestSpinne): DoReprogram geht nur auf EMP-Gelaehmte und direkt daneben, ~120 Ticks, immun gegen EMP. Sie
// fahren mit der Truppe, sobald EMP dabei ist; die erste bleibt zur Reparatur daheim. Spiders follow an EMP attack and
// take over stunned enemies.
inline void spinnenEinsatz(Zustand& z) {
  if (Player[z.spieler].IsEden()) return;
  const int t = TethysGame::Tick();
  if (t - z.spinneTakt < 40) return;
  z.spinneTakt = t;
  // Uebernahmen zaehlen
  for (int i = 0; i < 8; ++i) if (z.spinneZiel[i]) {
    UnitEx u; u.unitID = z.spinneZiel[i];
    if (u.IsLive() && u.IsVehicle() && u.OwnerID() == z.spieler) { z.bilUeber++; z.spinneZiel[i] = 0; melde(z, "kampf", mapSpider, z.bx, z.by, "Spinne hat ein Feindfahrzeug uebernommen (%d in diesem Angriff)", z.bilUeber); }
    else if (!u.IsLive() || !u.IsVehicle()) z.spinneZiel[i] = 0;
  }
  UnitEx sp[8]; int ns = 0;
  { PlayerVehicleEnum e(z.spieler); UnitEx u; while (e.GetNext(u) && ns < 8) if (u.IsLive() && u.GetType() == mapSpider) sp[ns++] = u; }
  if (ns < 2) return;
  const bool angriff = z.kampfModus >= 2 && (z.zielEmp > 0 || z.empRakete || z.kampfModus == 4);
  int mx = 0, my = 0, k = 0;
  if (angriff) { AngriffEnum ge(z); UnitEx v; while (ge.GetNext(v)) if (v.IsLive() && v.IsVehicle()) { const LOCATION l = v.Location(); mx += l.x; my += l.y; k++; } }
  for (int i = 1; i < ns; ++i) {
    UnitEx& s = sp[i];
    if (s.GetCurAction() != moDone || s.IsBusy()) continue;
    const LOCATION sl = s.Location();
    UnitEx ziel; int best = 12 * 12 + 1;
    for (int p = 0; p < TethysGame::NoPlayers(); ++p) {
      if (p == z.spieler || ExtPlayer[z.spieler].IsAlliedTo(p)) continue;
      PlayerVehicleEnum e(p); UnitEx u;
      while (e.GetNext(u)) { if (!u.IsLive() || !u.isEMPed()) continue; const LOCATION l = u.Location(); const int d = abstand2(sl.x, sl.y, l.x, l.y); if (d < best) { best = d; ziel = u; } }
    }
    if (ziel.unitID) { Unit z2; z2.unitID = ziel.unitID; s.DoReprogram(z2); z.spinneZiel[i] = ziel.unitID; continue; }
    if (angriff && k) { LOCATION hin(mx / k, my / k); if (abstand2(sl.x, sl.y, hin.x, hin.y) > 6 * 6) s.DoMove(hin); }
    else if (!angriff && abstand2(edX(z, sl), edY(z, sl), z.bx, z.by) > 25 * 25) s.DoMove(engine(z, z.bx, z.by + 5));
  }
}
inline void mixSchreiben(Puffer& p, const short* typ, const short* wf, const short* anz, int n) {
  p.roh("[");
  for (int i = 0; i < n; ++i) {
    if (i) p.roh(",");
    p.roh("["); p.str(name(typ[i])); p.roh(","); p.str(wf[i] > 0 && wf[i] != mapNone ? name(wf[i]) : "-"); p.f(",%d]", anz[i]);
  }
  p.roh("]");
}
inline void bilanzEnde(Zustand& z, int zer, int ver, const char* abbruch) {
  if (!z.bilStart) return;
  int erz = 0, sel = 0, n = 0;
  angriffsGruppe(z, erz, sel, nullptr, nullptr, nullptr, n);
  erz += z.bilRueckErz;                                   // heimgekehrte zaehlen als zurueck
  Puffer p;
  p.f("{\"t\":%d,\"art\":\"angriff\",\"p\":%d,\"start\":%d,\"dauer\":%d,\"angriffArt\":%d,\"emp\":%d,"
      "\"einsatz\":[%d,%d],\"zurueck\":[%d,%d],\"zerstoert\":%d,\"verloren\":%d,\"uebernommen\":%d,\"feindWT\":%d,\"E\":%d,\"D\":%d,\"armee\":[%d,%d],\"ankunft\":[%d,%d,%d,%d],\"heim\":[%d,%d],\"kontakt\":[%d,%d,%d,%d,%d,%d,%d,%d,%d],\"eigene\":",
      TethysGame::Tick(), z.spieler, z.bilStart, TethysGame::Tick() - z.bilStart, z.angriffArt, z.bilEmp,
      z.bilEinsatzErz, z.bilEinsatzSelten, erz, sel, zer, ver, z.bilUeber, z.bilFeindWT, z.bilE, z.bilD, z.bilArmee, z.feindArmee,
      z.bilKontakt ? z.bilKontakt - z.bilStart : -1, z.bilErste, z.bilNach, z.bilGruppe, z.zerstoertHeim - z.bilZerHeim, z.verlorenHeim - z.bilVerHeim,
      z.bilK[0], z.bilK[1], z.bilK[2], z.bilK[3], z.bilK[0] >= 0 ? (z.bilK[5] & 0xffff) : -1, z.bilK[0] >= 0 ? (z.bilK[5] >> 16) : -1, z.bilK2[0], z.bilK2[1], z.bilK2[2]);
  mixSchreiben(p, z.bilTyp, z.bilWaffe, z.bilAnz, z.bilN);
  p.roh(",\"feind\":");
  mixSchreiben(p, z.bilFTyp, z.bilFWaffe, z.bilFAnz, z.bilFN);
  p.roh(",\"ende\":"); p.str(abbruch);
  p.roh("}");
  p.schreiben();
  z.bilStart = 0;
}

inline int gruppenWert(ScGroup& g, int& n) {
  // Jede Einheit nur einmal (Kampf 7: "27 Lynx" bei 3 echten -- eine Einheit stand mehrfach in der Gruppe)
  int w = 0; n = 0;
  int ids[64]; int ni = 0;
  GroupEnumerator ge(g);
  UnitEx v;
  while (ge.GetNext(v)) {
    if (!v.IsLive() || !kaempfer(v)) continue;
    bool schon = false;
    for (int i = 0; i < ni; ++i) if (ids[i] == v.unitID) { schon = true; break; }
    if (schon) continue;
    if (ni < 64) ids[ni++] = v.unitID;
    w += kampfWert(v); n++;
  }
  return w;
}
inline bool inGruppeG(ScGroup& g, int id) {
  GroupEnumerator ge(g);
  Unit v;
  while (ge.GetNext(v)) if (v.unitID == id) return true;
  return false;
}

// Angriffsziel nach Funktion (Spieler: "nach Standort in der Basis -- Energie, Kraftwerke, CC und Strukturfabrik,
// keine wahllosen Gebaeude"): Kraftwerke zuerst (ohne Strom steht alles), dann Kommandozentrale, Strukturfabrik,
// Fahrzeugfabrik, dann der Rest -- je Stufe das naechste zur Truppe. Attack target by function.
// Zielreihenfolge (Basis-Test 3, 2026-11-09, 144 Kaempfe): Zentrale und Strukturfabrik zuerst -- das ist die Siegbedingung; bei
// gleicher Staerke blieb so am meisten uebrig (Restwert 58 statt 38 %) und der Angriff dauerte halb so lange wie "Kraftwerk zuerst".
// Target order: command centre and structure factory first (the win condition), then power, then the vehicle factory.
inline UnitEx angriffZielWaehlen(const Zustand& z, int vonX, int vonY, int& stufe) {
  static const int STUFE[][4] = {
    { mapCommandCenter, mapNone, mapNone, mapNone },
    { mapStructureFactory, mapNone, mapNone, mapNone },
    { mapTokamak, mapMHDGenerator, mapSolarPowerArray, mapGeothermalPlant },
    { mapVehicleFactory, mapArachnidFactory, mapNone, mapNone },
  };
  UnitEx beste; int besterD = 1 << 30; stufe = -1;
  for (int s2 = 0; s2 < 5 && !beste.unitID; ++s2) {
    for (int p = 0; p < TethysGame::NoPlayers(); ++p) {
      if (p == z.spieler || ExtPlayer[z.spieler].IsAlliedTo(p)) continue;
      if (s2 < 4) {
        for (int k = 0; k < 4; ++k) {
          if (STUFE[s2][k] == mapNone) continue;
          PlayerBuildingEnum e(p, (map_id)STUFE[s2][k]);
          UnitEx b;
          while (e.GetNext(b)) {
            const LOCATION lo = b.Location();
            const int d = abstand2(edX(z, lo), edY(z, lo), vonX, vonY);
            if (d < besterD) { besterD = d; beste = b; stufe = s2; }
          }
        }
      } else {
        for (int ty : GEBAEUDE) {
          PlayerBuildingEnum e(p, (map_id)ty);
          UnitEx b;
          while (e.GetNext(b)) {
            const LOCATION lo = b.Location();
            const int d = abstand2(edX(z, lo), edY(z, lo), vonX, vonY);
            if (d < besterD) { besterD = d; beste = b; stufe = 4; }
          }
        }
      }
    }
  }
  return beste;
}

inline void angriffZielSetzen(Zustand& z, int vonX, int vonY) {
  int stufe = -1;
  UnitEx ziel;
  // Ziel- und Zermuerbungsangriff: erst das gewaehlte Gebaeude, danach weiter nach Funktion
  { UnitEx w; w.unitID = z.angriffWunsch; if (z.angriffWunsch && w.IsLive()) { ziel = w; stufe = 4; } else z.angriffWunsch = 0; }
  if (!ziel.unitID) ziel = angriffZielWaehlen(z, vonX, vonY, stufe);
  if (!ziel.unitID) return;
  if (ziel.unitID == z.angriffZiel) {
    UnitEx alt; alt.unitID = z.angriffZiel;
    if (alt.IsLive()) {
      // Ziel steht noch: untaetige Fahrzeuge bekommen den Befehl neu (Duell 4: Truppe stand still vor dem Ziel)
      AngriffEnum ge(z);
      UnitEx v;
      // DoAttack bewegt nicht in Reichweite (Test KI_TestS: Lynx 15 Kacheln entfernt blieben stehen) -- ausser
      // Reichweite erst heranfahren. DoAttack does not move units into range: move first.
      const LOCATION zl = alt.Location();
      // Kampfmodell: wer eine Bedrohung in Reichweite hat, schiesst auf sie, nicht auf das Gebaeude (Arena: jeder greift den
      // Naechsten an, nur so stimmt die Rechnung). Nearest threat first, then the building.
      UnitEx bedr[128]; int nb = 0;
      for (int p = 0; p < TethysGame::NoPlayers(); ++p) {
        if (p == z.spieler || ExtPlayer[z.spieler].IsAlliedTo(p)) continue;
        { PlayerVehicleEnum e(p); UnitEx u; while (e.GetNext(u) && nb < 128) if (u.IsLive() && kampfWert(u) > 0) bedr[nb++] = u; }
        { PlayerBuildingEnum e(p, mapGuardPost); UnitEx u; while (e.GetNext(u) && nb < 128) if (u.IsLive()) bedr[nb++] = u; }
      }
      while (ge.GetNext(v)) {
        if (!v.IsLive() || !v.IsVehicle()) continue;
        const int akt = v.GetCurAction();
        if (nb && (akt == moDone || akt == moMove)) {
          UnitInfo wi0((map_id)v.GetWeapon());
          const int rw0 = wi0.GetWeaponRange(z.spieler) > 0 ? wi0.GetWeaponRange(z.spieler) : 4;
          const LOCATION vl0 = v.Location();
          int best = (rw0 + 1) * (rw0 + 1) + 1, bi = -1;
          for (int i = 0; i < nb; ++i) { if (!bedr[i].IsLive()) continue; const LOCATION bl = bedr[i].Location(); const int d = abstand2(vl0.x, vl0.y, bl.x, bl.y); if (d < best) { best = d; bi = i; } }
          if (bi >= 0) { if (akt == moMove || !v.IsBusy()) v.Unit::DoAttack(bedr[bi]); continue; }
        }
        // In Reichweite: angreifen, auch wenn sie noch "faehrt" (Duell 20: 9 Lynx standen 60000 Ticks im Zustand
        // moMove neben dem Ziel -- der Fahrbefehl auf das Gebaeudefeld selbst kommt nie an).
        // In range: attack even while still "moving" (a move onto the building tile never completes).
        if (akt != moDone && akt != moMove) continue;
        UnitInfo wi((map_id)v.GetWeapon());
        const int rw = wi.GetWeaponRange(z.spieler) > 0 ? wi.GetWeaponRange(z.spieler) : 4;
        const LOCATION vl = v.Location();
        const int d2 = abstand2(vl.x, vl.y, zl.x, zl.y);
        if (d2 <= (rw + 1) * (rw + 1)) { if (akt == moMove || !v.IsBusy()) v.Unit::DoAttack(alt); continue; }
        if (akt != moDone || v.IsBusy()) continue;
        // Heranfahren bis knapp in Reichweite, nicht auf das Gebaeude selbst
        const double d = std::sqrt((double)d2), f = (d - (rw - 1)) / d;
        LOCATION hin(vl.x + (int)((zl.x - vl.x) * f + 0.5), vl.y + (int)((zl.y - vl.y) * f + 0.5));
        v.DoMove(hin);
      }
      return;
    }
  }
  const LOCATION lo = ziel.Location();
  const int x = edX(z, lo), y = edY(z, lo);
  // Wartezone (SetRect) neben dem Ziel, auf unserer Seite: die Engine faehrt die Truppe hin. Bewachte Zone ueber der
  // Feindbasis (DoGuardRect): feindliche Fahrzeuge dort bekaempft die Truppe selbst -- Gebaeude nicht (Test KI_TestG),
  // die bekommen den Einzelbefehl, sobald ein Fahrzeug in Reichweite steht (siehe oben, Auffrischen).
  // Idle zone next to the target, guarded zone over the enemy base; buildings via per-unit DoAttack when in range.
  {
    double ux, uy;
    feindRichtung(z, z.bx, z.by, ux, uy);                       // Richtung Feind -> Wartezone davor (unsere Seite)
    const int wx = (int)(x - ux * 3 + 0.5), wy = (int)(y - uy * 3 + 0.5);
    MAP_RECT r(engine(z, wx - 1, wy - 1), engine(z, wx + 1, wy + 1));
    const MAP_RECT feind(engine(z, z.feindX - 12, z.feindY - 12), engine(z, z.feindX + 12, z.feindY + 12));
    wachen(z.angriff, r, &feind, 1);
    if (z.angriff2An) wachen(z.angriff2, r, &feind, 1);
  }
  z.angriffZiel = ziel.unitID;
  static const char* const ART[] = { "Kommandozentrale", "Strukturfabrik", "Kraftwerk", "Fahrzeugfabrik", "sonstiges Gebaeude" };
  melde(z, "kampf", ziel.GetType(), x, y, "Angriffsziel: %s %s bei %d,%d", ART[stufe < 0 ? 4 : stufe], name(ziel.GetType()), x, y);
}

// Merkt sich, ob kampfFuehrung gerade laeuft: bei einem abgefangenen Absturz bleibt das Flag stehen (der Destruktor
// laeuft dann nicht), und der naechste Aufruf legt die Gruppen neu an.
struct KampfLaeuft { int& f; explicit KampfLaeuft(int& x) : f(x) { f = 1; } ~KampfLaeuft() { f = 0; } };

// Kampfgruppen (Spieler: "Kampfgruppen unterteilt in Verteidigung und Angriff ... in einer Gruppe werden die Fahrzeuge
// gebaut ... wenn die Gruppe voll ist, werden alle Einheiten mit einem Befehl in die Angriffsgruppe geschoben"):
//  - Fabrikgruppe (BuildingGroup mit der Fahrzeugfabrik) verstaerkt per RecordVehReinforceGroup die Wache (hohe
//    Prioritaet) und die Sammelgruppe; die Sollzahlen setzt SetTargCount -- die Engine baut und setzt die Fahrzeuge
//    selbst in die Gruppe (kein TakeUnit fuer frische Fahrzeuge mehr, Duell 9).
//  - Wache: bewacht immer die Kolonie. Sammelgruppe: wartet an der Kolonie, bis die Angriffstruppe voll ist.
//  - Angriffsgruppe: bekommt die volle Sammelgruppe per TakeAllUnits und greift an (Ziele nach Funktion); Ueberlebende
//    gehen beim Abbruch per TakeAllUnits zur Wache.
// Combat groups: a factory group reinforces guard and staging group; a full staging group moves into the attack group.
inline bool netzAngriff(const Zustand& z, const Lage& l, int art, int fahrzeuge, int wertTruppe, float& zer, float& ver);   // weiter unten

// EMP-Rakete bauen: im Raumhafen, sobald Erz und Seltenerz reichen und sie sich gegen die jetzige Abwehr lohnt.
// Die fertige Rakete steht auf der Rampe (empRakete), bis ein Angriff sie braucht. Build the missile when worth it.
inline void empRaketeBauen(Zustand& z, const Lage& l) {
  const int t = TethysGame::Tick();
  if (!empMoeglich(z) || t % 200 >= TAKT) return;
  UnitEx sp;
  { PlayerBuildingEnum e(z.spieler, mapSpaceport); UnitEx u; while (e.GetNext(u)) if (u.IsLive() && u.GetCurAction() != moObjWait) { sp = u; break; } }
  if (!sp.unitID) { z.empRakete = 0; z.empAuftrag = 0; return; }   // kein (fertiger) Raumhafen: auch keine Rakete
  if (z.empRakete) return;
  if (z.empAuftrag) {
    // fertig, sobald der Raumhafen nicht mehr baut
    if (t - z.empAuftragSeit > 200 && sp.GetCurAction() != moDevelop && !sp.IsBusy()) {
      z.empAuftrag = 0; z.empRakete = 1;
      melde(z, "kampf", mapEMPMissile, edX(z, sp.Location()), edY(z, sp.Location()), "EMP-Rakete steht auf der Rampe");
    }
    return;
  }
  if (sp.IsBusy() || sp.GetCurAction() != moDone) return;
  UnitInfo r(mapEMPMissile);
  if (l.erz < r.GetOreCost(z.spieler) || l.selten < r.GetRareOreCost(z.spieler)) return;
  int wert = 0;
  if (!empLohnt(z, &wert)) return;
  sp.DoProduce(mapEMPMissile, mapNone);
  z.empAuftrag = 1; z.empAuftragSeit = t;
  melde(z, "kampf", mapEMPMissile, edX(z, sp.Location()), edY(z, sp.Location()), "EMP-Rakete wird gebaut (%d Erz, %d Seltenerz; dichteste Abwehr Wert %d, Rakete ~%d)",
        r.GetOreCost(z.spieler), r.GetRareOreCost(z.spieler), wert, empKostenWert(z));
}
// Feindliche Kampfstaerke um einen Punkt (Fahrzeuge). Enemy combat vehicles' strength around a point.
inline int feindStaerkeBei(const Zustand& z, int x, int y, int r) {
  int d = 0;
  for (int p = 0; p < TethysGame::NoPlayers(); ++p) {
    if (p == z.spieler || ExtPlayer[z.spieler].IsAlliedTo(p)) continue;
    PlayerVehicleEnum en(p); UnitEx u;
    while (en.GetNext(u)) if (u.IsLive() && kampfWert(u) > 0) { const LOCATION l = u.Location(); if (abstand2(edX(z, l), edY(z, l), x, y) <= r * r) d += kampfWert(u); }
  }
  return d;
}
inline int eigeneGruppeBei(const Zustand& z, ScGroup& g, int x, int y, int r) {
  int e = 0; GroupEnumerator ge(g); UnitEx v;
  while (ge.GetNext(v)) if (v.IsLive() && v.IsVehicle()) { const LOCATION l = v.Location(); if (abstand2(edX(z, l), edY(z, l), x, y) <= r * r) e += kampfWert(v); }
  return e;
}
// Wert eines Bereichs: Baukosten der eigenen Gebaeude darin (was hier verloren ginge). Value at stake: building cost nearby.
inline int bereichWert(const Zustand& z, int x, int y, int r) {
  int w = 0; PlayerBuildingEnum e(z.spieler, mapNone); UnitEx b;
  while (e.GetNext(b)) if (b.IsLive()) { const LOCATION l = b.Location(); if (abstand2(edX(z, l), edY(z, l), x, y) <= r * r) { UnitInfo i(b.GetType()); w += i.GetOreCost(z.spieler); } }
  return w;
}
// Eingreifreserve: Basis und Aussenposten sind Bereiche. Die Reserve geht dorthin, wo Luecke (Feind minus Wache und Tuerme vor Ort)
// mal Wert am groessten ist -- aber nur, wenn sie die Luecke schlagen kann; sonst bleibt sie daheim und der Bereich wird aufgegeben.
// Reserve goes where gap x value is largest, only if it can beat the gap; otherwise it stays home.
inline void reserveFuehren(Zustand& z) {
  if (!z.reserveAn) return;
  const int t = TethysGame::Tick();
  int nr = 0; const int er = gruppenWert(z.eingreif, nr);
  if (!nr) { z.reserveOrt = -2; return; }
  const int R = 14;
  int besterOrt = -1, bx = z.bx, by = z.by; double beste = 0;
  for (int i = -1; i < 6; ++i) {
    if (i >= 0 && !z.aussenId[i]) continue;
    const int x = i < 0 ? z.bx : z.aussenX[i], y = i < 0 ? z.by : z.aussenY[i];
    const int d = feindStaerkeBei(z, x, y, R);
    if (!d) continue;
    int vorOrt = eigeneGruppeBei(z, z.wache, x, y, R);
    if (i >= 0 && z.aussenAn[i]) vorOrt += eigeneGruppeBei(z, z.aussenGruppe[i], x, y, R);
    { PlayerBuildingEnum e(z.spieler, mapGuardPost); UnitEx g; while (e.GetNext(g)) if (g.IsLive()) { const LOCATION l = g.Location(); if (abstand2(edX(z, l), edY(z, l), x, y) <= R * R) vorOrt += kampfWert(g); } }
    const int luecke = d - vorOrt;
    if (luecke <= 0 || er < luecke) continue;
    const double score = (double)luecke * (bereichWert(z, x, y, R) + 1);
    if (score > beste) { beste = score; besterOrt = i; bx = x; by = y; }
  }
  // Nicht bei jedem Schwanken umdrehen: mindestens 300 Ticks bleiben, solange der Bereich noch eine Luecke hat
  if (besterOrt < 0 && z.reserveOrt >= -1 && t - z.reserveSeit < 300) return;
  const int ort = besterOrt >= 0 ? besterOrt : -3;
  if (ort == z.reserveOrt) return;
  if (ort == -3) { bx = z.bx; by = z.by + 4; freierWarteplatz(z, bx, by, 2, false); }   // daheim: frei von Gebaeuden und Erzwegen
  MAP_RECT warte(engine(z, bx - 2, by - 2), engine(z, bx + 2, by + 2));
  const MAP_RECT schutz(engine(z, bx - 8, by - 8), engine(z, bx + 8, by + 8));
  wachen(z.eingreif, warte, &schutz, 1);
  { GroupEnumerator ge(z.eingreif); UnitEx v; const LOCATION zl = engine(z, bx, by); while (ge.GetNext(v)) if (v.IsLive() && v.IsVehicle()) v.DoMove(zl); }
  z.reserveOrt = ort; z.reserveSeit = t;
  melde(z, "kampf", mapLynx, bx, by, ort == -3 ? "Reserve (%d Fahrzeuge) zurueck in die Basis" : "Reserve (%d Fahrzeuge) verteidigt den Bereich bei %d,%d", nr, bx, by);
}

inline void kampfFuehrung(Zustand& z, const Lage& l) {
  const int t = TethysGame::Tick();
  if (t % SCAN_TAKT >= TAKT) return;                    // einmal je SCAN_TAKT, egal mit welchem Versatz der Schritt laeuft
  // Selbstheilung (Duell 9): ist der letzte Durchlauf abgestuerzt, alle Gruppen neu anlegen
  if (z.kampfLaeuft) {
    z.wacheAn = 0; z.sammelAn = 0; z.angriffAn = 0; z.fabrikAn = 0; z.kampfModus = 0; z.wacheAnzahl = 0;
  z.angriff2An = 0; z.reserveAn = 0; z.reserveZiel = 0; z.reserveOrt = -2; z.reserveSeit = 0;
    for (int i = 0; i < 6; ++i) z.aussenAn[i] = 0;
    melde(z, "diagnose", mapLynx, z.bx, z.by, "Kampffuehrung war abgestuerzt -- alle Kampfgruppen werden neu angelegt");
  }
  KampfLaeuft laeuft(z.kampfLaeuft);
  hinweis(z, "ki::kampfFuehrung/gruppen");
  double eux, euy;
  feindRichtung(z, z.bx, z.by, eux, euy);
  const int wx = (int)(z.bx + eux * 8 + 0.5), wy = (int)(z.by + euy * 8 + 0.5);   // Warteplatz der Wache zum Feind hin
  const int sx = (int)(z.bx - eux * 6 + 0.5), sy = (int)(z.by - euy * 6 + 0.5);   // Sammelplatz hinter der Kolonie
  const MAP_RECT basis(engine(z, z.bx - 16, z.by - 14), engine(z, z.bx + 16, z.by + 14));
  int* const platz = &g_platz[z.spieler & 7][0][0];
  auto heim = [&](const char* warum) {
    // Warteplatz zum Feind hin, frei von Gebaeuden, Erzwegen und Tuermen (freierWarteplatz; ein zerstoerter Turm zerstoert Einheiten
    // daneben). Frueher wich die Wache bei Tuermen zur Basis hin aus und stand dann auf der Anlegestelle der Schmelze.
    int hx = wx, hy = wy;
    freierWarteplatz(z, hx, hy, 3, true);
    platz[0] = hx; platz[1] = hy;
    MAP_RECT warte(engine(z, hx - 3, hy - 3), engine(z, hx + 3, hy + 3));
    wachen(z.wache, warte, &basis, 1);
    if (warum) melde(z, "kampf", mapLynx, hx, hy, "Wache an der Kolonie (Warteplatz %d,%d): %s", hx, hy, warum);
  };
  auto sammelPlatz = [&]() {
    int qx = sx, qy = sy;
    freierWarteplatz(z, qx, qy, 2, false);
    platz[2] = qx; platz[3] = qy;
    MAP_RECT s(engine(z, qx - 2, qy - 2), engine(z, qx + 2, qy + 2));
    wachen(z.sammel, s, &basis, 1);
  };
  if (!z.wacheAn) { z.wache = CreateFightGroup(Player[z.spieler]); z.wacheAn = 1; heim(nullptr); }
  if (!z.sammelAn) { z.sammel = CreateFightGroup(Player[z.spieler]); z.sammelAn = 1; sammelPlatz(); }
  // Die Basis waechst: alle 2000 Ticks pruefen, ob die Plaetze noch frei sind, sonst verlegen. Re-check the areas as the base grows.
  if (TethysGame::Tick() % 2000 < TAKT) {
    int hx = wx, hy = wy; freierWarteplatz(z, hx, hy, 3, true);
    if (hx != platz[0] || hy != platz[1]) heim("Warteplatz verlegt (alter Platz belegt)");
    int qx = sx, qy = sy; freierWarteplatz(z, qx, qy, 2, false);
    if (qx != platz[2] || qy != platz[3]) { sammelPlatz(); melde(z, "kampf", mapLynx, qx, qy, "Sammelplatz verlegt nach %d,%d (alter Platz belegt)", qx, qy); }
    if (z.fabrikAn) {
      int fx = z.bx, fy = z.by + 7; freierWarteplatz(z, fx, fy, 2, false);
      if (fx != platz[8] || fy != platz[9]) { platz[8] = fx; platz[9] = fy; MAP_RECT r(engine(z, fx - 3, fy - 1), engine(z, fx + 3, fy + 2)); z.fabrik.SetRect(r); }
    }
  }
  if (!z.angriffAn) { z.angriff = CreateFightGroup(Player[z.spieler]); z.angriffAn = 1; }
  if (!z.angriff2An) { z.angriff2 = CreateFightGroup(Player[z.spieler]); z.angriff2An = 1; }
  if (!z.reserveAn) { z.eingreif = CreateFightGroup(Player[z.spieler]); z.reserveAn = 1; z.reserveOrt = -2; }
  // Fabrikgruppe mit der (fertigen, ruhigen) Fahrzeugfabrik
  if (l.vf.unitID && (!z.fabrikAn || z.fabrikVf != l.vf.unitID) && bereit(l.vf)) {
    hinweis(z, "ki::kampfFuehrung/fabrikgruppe");
    z.fabrik = CreateBuildingGroup(Player[z.spieler]);
    int fx = z.bx, fy = z.by + 7; freierWarteplatz(z, fx, fy, 2, false);   // frei von Gebaeuden und Erzwegen
    platz[8] = fx; platz[9] = fy;
    MAP_RECT r(engine(z, fx - 3, fy - 1), engine(z, fx + 3, fy + 2));
    z.fabrik.SetRect(r);
    z.fabrik.TakeUnit(l.vf);
    z.fabrik.RecordVehReinforceGroup(z.wache, 3000);
    z.fabrik.RecordVehReinforceGroup(z.sammel, 1000);
    z.fabrik.RecordVehReinforceGroup(z.eingreif, 500);
    z.fabrikAn = 1; z.fabrikVf = l.vf.unitID;
    for (int i = 0; i < MAX_VF; ++i) z.fabrikDrin[i] = 0;
    z.fabrikDrin[0] = l.vf.unitID;
    melde(z, "kampf", mapVehicleFactory, edX(z, l.vf.Location()), edY(z, l.vf.Location()),
          "Fabrikgruppe angelegt: die Fahrzeugfabrik verstaerkt Wache (Prioritaet 3000) und Sammelgruppe (1000)");
  }
  // Weitere Fahrzeugfabriken (fertig, ruhig) in dieselbe Fabrikgruppe -- sie bauen nur Kampffahrzeuge
  // Further factories join the same factory group; they only build combat vehicles.
  if (z.fabrikAn) {
    for (int j = 1; j < l.vfAnzahl; ++j) {
      UnitEx v = l.vfs[j];
      if (!bereit(v)) continue;
      int frei = -1; bool drin = false;
      for (int i = 0; i < MAX_VF; ++i) {
        if (z.fabrikDrin[i] == v.unitID) drin = true;
        UnitEx alt; alt.unitID = z.fabrikDrin[i];
        if (frei < 0 && (!z.fabrikDrin[i] || !alt.IsLive() || alt.GetType() != mapVehicleFactory)) frei = i;
      }
      if (drin || frei < 0) continue;
      hinweis(z, "ki::kampfFuehrung/fabrik-dazu");
      z.fabrik.TakeUnit(v);
      z.fabrikDrin[frei] = v.unitID;
      melde(z, "kampf", mapVehicleFactory, edX(z, v.Location()), edY(z, v.Location()),
            "Weitere Fahrzeugfabrik in die Fabrikgruppe (%d Fabriken)", l.vfAnzahl);
    }
  }
  // Sollzahlen: Wache deckt den fehlenden Kampfwert (ohne Tuerme) plus die Aussenposten, Sammelgruppe die Angriffstruppe
  hinweis(z, "ki::kampfFuehrung/soll");
  const int fz = fahrgestellWahl(z, l.selten);
  int wacheZiel = 0, sammelZiel = 0, waffe = mapNone;
  if (fz) {
    UnitInfo fi((map_id)fz);
    waffe = waffeWahl(z, fz, l.selten - fi.GetRareOreCost(z.spieler));
    const int wert = sdkWertT(fz, waffe) > 0 ? sdkWertT(fz, waffe) : WERT_LYNX;
    const int turm = z.eigeneWTWert + (geplant(z, mapGuardPost) + l.kitsImSpiel[mapGuardPost] + l.imBau[mapGuardPost]) * WERT_WT;
    const int fehlt = z.sollArmee - turm;
    wacheZiel = fehlt > 0 ? (fehlt + wert - 1) / wert : 0;
    for (int i = 0; i < 6; ++i) wacheZiel += aussenFahrzeuge(z, i);
    const int angriffSoll = mobilSoll(z, true) - mobilSoll(z, false);
    sammelZiel = angriffSoll > 0 ? (angriffSoll + wert - 1) / wert : 0;
    // Keine feste Obergrenze (Duell 14: Wache 14 + Sammelgruppe 14 erreicht, Fabrik stillgelegt, 29000 Erz im Lager,
    // Soll weit offen). Grenze ist, was sich bezahlen laesst: vorhandene Fahrzeuge plus was Vorrat und Zufluss in der
    // Reaktionszeit hergeben. No fixed cap: existing vehicles plus what stock and income pay for within REAKTION.
    const int wacheOhne = wacheZiel, sammelOhne = sammelZiel;
    UnitInfo wi((map_id)waffe);
    const int kosten = fi.GetOreCost(z.spieler) + wi.GetOreCost(z.spieler) > 0 ? fi.GetOreCost(z.spieler) + wi.GetOreCost(z.spieler) : 1;
    const Bilanz bl = bilanz(z, l);
    double zufluss = erzRouten(z) * (double)bl.routeErz * 1000.0 / PAR(z).horizont;
    if (z.zuflussErz > zufluss) zufluss = z.zuflussErz;
    const int neu = (int)((l.erz + zufluss * PAR(z).reaktion / 1000) / kosten);
    int nw = 0, ns = 0;
    gruppenWert(z.wache, nw); gruppenWert(z.sammel, ns);
    if (wacheZiel > nw + neu) wacheZiel = nw + neu;
    const int rest = neu - (wacheZiel > nw ? wacheZiel - nw : 0);
    if (sammelZiel > ns + (rest > 0 ? rest : 0)) sammelZiel = ns + (rest > 0 ? rest : 0);
    // Die Erz-Grenze bremst nur das Erhoehen: ein schon gesetztes Ziel faellt nicht, nur weil gerade Erz ausgegeben
    // wurde (Duell 15: Sollzahlen sprangen alle 200 Ticks). Only raising is limited by ore; a set target stays.
    if (wacheZiel < z.wacheZiel && wacheOhne >= z.wacheZiel) wacheZiel = z.wacheZiel;
    if (sammelZiel < z.sammelZiel && sammelOhne >= z.sammelZiel) sammelZiel = z.sammelZiel;
    // Seltenerz-Schmelze geht vor neuen Kampffahrzeugen, solange keine akute Bedrohung besteht: das Soll bleibt beim Bestand,
    // bis ihr Erz zusammen ist (sonst gab die Fabrikgruppe jedes Erz fuer Lynx aus, 2026-11-09: nie Seltenerz, nur Lynx).
    // Rare smelter before new combat vehicles unless under acute threat: targets stay at the current count.
    const bool sparen = l.seltenSpart && z.bedrohung < 50;
    if (sparen) { if (wacheZiel > nw) wacheZiel = nw; if (sammelZiel > ns) sammelZiel = ns; }
    // Reserve: erst wenn Wache und Sammelgruppe stehen, und nur so viel, wie Vorrat und Zufluss hergeben
    {
      int nr = 0; gruppenWert(z.eingreif, nr);
      int rz = nr;
      if (!sparen && nw >= wacheZiel && ns >= sammelZiel) { rz = nr + neu; if (rz > GRUPPE_MAX) rz = GRUPPE_MAX; }
      if (rz != z.reserveZiel) { z.eingreif.ClearTargCount(); if (rz > 0) z.eingreif.SetTargCount((map_id)fz, (map_id)waffe, rz); z.reserveZiel = rz; }
    }
    // Eine Gruppe fasst 32, die Fabrik haelt dort an (Test KI_TestGruppe: Soll 40, bei 32 Stillstand, nichts verloren).
    { if (wacheZiel > GRUPPE_MAX) wacheZiel = GRUPPE_MAX; if (sammelZiel > GRUPPE_MAX) sammelZiel = GRUPPE_MAX; }
  }
  // EMP-Beimischung: 25 % der Angriffstruppe, wenn die gemessene Verstaerkung (Reihe 2: Faktor 1,06 bis Stufe 8, 1,12 ab Tiger)
  // den Aufpreis uebersteigt. EMP mix if the measured gain exceeds the extra cost.
  int empN = 0;
  if (fz && sammelZiel >= 4 && waffe != mapEMP && hatTech(z, 5508)) {
    UnitInfo fi((map_id)fz), wi((map_id)waffe), ei(mapEMP);
    const double pm = fi.GetOreCost(z.spieler) + wi.GetOreCost(z.spieler), ph = fi.GetOreCost(z.spieler) + ei.GetOreCost(z.spieler);
    const double faktor = einheitErforscht(z, mapTiger) ? 1.12 : 1.06;
    if (pm > 0 && faktor > (3 * pm + ph) / (4 * pm)) empN = sammelZiel / 4;
  }
  if (fz != z.zielFz || waffe != z.zielWaffe || wacheZiel != z.wacheZiel || sammelZiel != z.sammelZiel || empN != z.zielEmp) {
    z.wache.ClearTargCount(); z.sammel.ClearTargCount();
    if (fz && wacheZiel > 0) z.wache.SetTargCount((map_id)fz, (map_id)waffe, wacheZiel);
    if (fz && sammelZiel > 0) z.sammel.SetTargCount((map_id)fz, (map_id)waffe, sammelZiel - empN);
    if (fz && (empN > 0 || z.zielEmp > 0)) z.sammel.SetTargCount((map_id)fz, mapEMP, empN);
    if (empN != z.zielEmp) melde(z, "kampf", fz, z.bx, z.by, "EMP-Beimischung: %d von %d Fahrzeugen der Angriffstruppe", empN, sammelZiel);
    z.zielEmp = empN;
    melde(z, "kampf", fz ? fz : mapLynx, z.bx, z.by, "Sollzahlen: Wache %d, Sammelgruppe %d %s mit %s (Soll %d, Tuerme %d, Angriffstruppe %d)",
          wacheZiel, sammelZiel, name(fz), name(waffe), z.sollArmee, z.eigeneWTWert, mobilSoll(z, true) - mobilSoll(z, false));
    z.zielFz = fz; z.zielWaffe = waffe; z.wacheZiel = wacheZiel; z.sammelZiel = sammelZiel;
  }
  hinweis(z, "ki::kampfFuehrung/reserve"); reserveFuehren(z);
  // 1. Aussenposten: Fahrzeuge aus der Wache abziehen bzw. zurueckgeben (nur lebende)
  hinweis(z, "ki::kampfFuehrung/aussenposten");
  for (int i = 0; i < 6; ++i) {
    const int soll = aussenFahrzeuge(z, i);
    if (!z.aussenAn[i] && soll == 0) continue;
    if (!z.aussenAn[i]) { z.aussenGruppe[i] = CreateFightGroup(Player[z.spieler]); z.aussenAn[i] = 1; }
    int n = 0, nw = 0;
    gruppenWert(z.aussenGruppe[i], n);
    gruppenWert(z.wache, nw);
    bool geaendert = false;
    while (n > soll) {
      GroupEnumerator ge(z.aussenGruppe[i]); UnitEx v; bool da = false;
      while (ge.GetNext(v)) if (v.IsLive() && kaempfer(v)) { da = true; break; }
      if (!da) break;
      if (z.wache.TotalUnitCount() >= GRUPPE_MAX) break;
      z.aussenGruppe[i].RemoveUnit(v); z.wache.TakeUnit(v); n--; geaendert = true;
    }
    while (n < soll && nw > 1) {
      GroupEnumerator ge(z.wache); UnitEx v; bool da = false;
      while (ge.GetNext(v)) if (v.IsLive() && kaempfer(v)) { da = true; break; }
      if (!da) break;
      if (z.aussenGruppe[i].TotalUnitCount() >= GRUPPE_MAX) break;
      z.wache.RemoveUnit(v); z.aussenGruppe[i].TakeUnit(v); n++; nw--; geaendert = true;
    }
    if (geaendert) {
      MAP_RECT r(engine(z, z.aussenX[i] - 2, z.aussenY[i] - 2), engine(z, z.aussenX[i] + 2, z.aussenY[i] + 2));
      const MAP_RECT g(engine(z, z.aussenX[i] - 7, z.aussenY[i] - 7), engine(z, z.aussenX[i] + 7, z.aussenY[i] + 7));
      wachen(z.aussenGruppe[i], r, &g, 1);
      heim(nullptr);
      melde(z, "kampf", mapLynx, z.aussenX[i], z.aussenY[i], "Aussenposten bei %d,%d: %d Fahrzeug(e) (%s)", z.aussenX[i], z.aussenY[i], n,
            z.aussenAngriff[i] && t - z.aussenAngriff[i] < 10000 ? "wurde angegriffen" : soll ? "naeher am Feind als die Basis" : "keine Gefahr mehr");
    }
    if (!z.aussenId[i] && n == 0) z.aussenAn[i] = 0;
  }
  // 2. Kampffahrzeuge ohne Gruppe (z.B. nach Neuanlage) zur Wache -- nur fertige, ruhige
  hinweis(z, "ki::kampfFuehrung/streuner");
  int neu = 0, alle = 0;
  {
    PlayerVehicleEnum e(z.spieler);
    UnitEx u;
    while (e.GetNext(u)) {
      if (!u.IsLive() || !kaempfer(u)) continue;
      alle++;
      if (u.unitID == z.garDran || u.unitID == z.garRaus) continue;   // gehoert gerade der Garage
      if (inGruppeG(z.wache, u.unitID) || inGruppeG(z.sammel, u.unitID) || inGruppeG(z.angriff, u.unitID) || inGruppeG(z.angriff2, u.unitID) || inGruppeG(z.eingreif, u.unitID)) continue;
      bool aussen = false;
      for (int i = 0; i < 6 && !aussen; ++i) if (z.aussenAn[i] && inGruppeG(z.aussenGruppe[i], u.unitID)) aussen = true;
      if (aussen || u.GetCurAction() != moDone || u.IsBusy()) continue;
      bool aufGebaeude = false;
      { LocationEnumerator le(u.Location()); UnitEx o; while (le.GetNext(o)) if (o.IsBuilding()) { aufGebaeude = true; break; } }
      if (aufGebaeude) continue;
      if (sicherNehmen(z, z.wache, u) || sicherNehmen(z, z.eingreif, u)) neu++;
    }
  }
  if (alle > z.kampfFzAnzahl) z.letztesKampfFz = t;                 // fuer die Selbstpruefung
  z.kampfFzAnzahl = alle;
  if (neu) heim("Kampffahrzeuge ohne Gruppe aufgenommen");
  int nw = 0, ns = 0, na = 0;
  gruppenWert(z.wache, nw);
  gruppenWert(z.sammel, ns);
  const int wa = angriffWert(z, na);
  z.wacheAnzahl = nw;
  { int nr = 0; gruppenWert(z.eingreif, nr); z.gruppenBedarf = (fz && (nw < wacheZiel || ns < sammelZiel || nr < z.reserveZiel)) ? 1 : 0; }
  // 3. Angriff
  hinweis(z, "ki::kampfFuehrung/angriff");
  int feindWTWert = z.feindWT * WERT_WT;
  {                                                  // echte Turmwerte statt Schaetzung
    feindWTWert = 0;
    for (int p = 0; p < TethysGame::NoPlayers(); ++p) {
      if (p == z.spieler || ExtPlayer[z.spieler].IsAlliedTo(p)) continue;
      PlayerBuildingEnum e(p, mapGuardPost); UnitEx g; while (e.GetNext(g)) if (g.IsLive()) feindWTWert += kampfWert(g);
    }
  }
  hinweis(z, "ki::kampfFuehrung/verfolgen"); verfolgen(z);
  hinweis(z, "ki::kampfFuehrung/spinnen"); spinnenEinsatz(z);
  if (z.kampfModus >= 2) { hinweis(z, "ki::kampfFuehrung/kontakt"); bilanzKontakt(z); hinweis(z, "ki::kampfFuehrung/rueckzug"); rueckzugPruefen(z, z.wache); hinweis(z, "ki::kampfFuehrung/angriff"); }
  if (z.kampfModus == 3 || z.kampfModus == 4) {
    // Aufmarsch / Rakete im Flug. Bricht etwas weg (Truppe aufgerieben, Kolonie angegriffen, Raumhafen weg), wird
    // daraus ein normaler Angriff (Modus 2 regelt Abbruch und Lernen). Staging / missile in flight.
    const bool kolonieInNot = z.feindNah > z.eigeneWTWert + gruppenWert(z.wache, nw) + gruppenWert(z.eingreif, nw);
    if (na == 0 || !z.feindGeb || kolonieInNot) { z.kampfModus = 2; return; }
    if (z.kampfModus == 3) {
      int dort = 0;
      { AngriffEnum ge(z); UnitEx v;
        while (ge.GetNext(v)) if (v.IsLive()) { const LOCATION lo = v.Location(); if (abstand2(edX(z, lo), edY(z, lo), z.aufmarschX, z.aufmarschY) <= 5 * 5) dort++; } }
      // Frist nach der Entfernung (Messung: Lynx ~50 Ticks je Kachel im Pulk; feste 3000 Ticks waren zu kurz, 4 von 17 standen)
      const int aufFrist = 1000 + (int)(70 * std::sqrt((double)abstand2(z.bx, z.by, z.aufmarschX, z.aufmarschY)));
      if (dort * 10 >= na * 7 || t - z.aufmarschSeit > aufFrist) {
        UnitEx sp; { PlayerBuildingEnum e(z.spieler, mapSpaceport); UnitEx u; while (e.GetNext(u)) if (u.IsLive()) { sp = u; break; } }
        if (!z.empRakete) {                                            // ohne Rakete: die Truppe steht, geschlossen los
          melde(z, "kampf", mapLynx, z.aufmarschX, z.aufmarschY, "Truppe steht (%d von %d nach %d Ticks) -- Angriff", dort, na, t - z.aufmarschSeit);
          z.kampfModus = 2; z.angriffZiel = 0; angriffZielSetzen(z, z.aufmarschX, z.aufmarschY); return;
        }
        const int wert = empZiel(z, z.empZielX, z.empZielY);         // Ziel neu bestimmen: die Abwehr kann gewandert sein
        // Start nur, wenn das Ziel die Rakete noch wert ist -- sonst ohne EMP angreifen, die Rakete bleibt fuer spaeter
        // (EMP-Test: eine Rakete flog auf eine Abwehr vom Wert 8, die Rakete kostet ~40)
        if (!sp.unitID || !z.empRakete || wert < empKostenWert(z) * PAR(z).empSchwelle) {
          melde(z, "kampf", mapLynx, z.aufmarschX, z.aufmarschY, "Aufmarsch steht -- Ziel ist die Rakete nicht wert (Abwehr %d), Angriff ohne EMP", wert);
          z.kampfModus = 2; angriffZielSetzen(z, z.aufmarschX, z.aufmarschY); return;
        }
        const LOCATION ziel = engine(z, z.empZielX, z.empZielY);
        sp.DoLaunch(ziel.x * 32 + 16, ziel.y * 32 + 16, 1);
        z.empRakete = 0; z.empStart = t; z.kampfModus = 4;
        melde(z, "kampf", mapEMPMissile, z.empZielX, z.empZielY, "EMP-Rakete gestartet auf %d,%d (Abwehr Wert %d, %d von %d Fahrzeugen am Aufmarschpunkt)",
              z.empZielX, z.empZielY, wert, dort, na);
      }
      return;
    }
    // Modus 4: nach dem Einschlag (gemessen 200-300 Ticks) angreifen, waehrend die Abwehr ~700 Ticks gelaehmt ist
    if (t - z.empStart >= EMP_FLUG) {
      int gelaehmt = 0;
      for (int p = 0; p < TethysGame::NoPlayers(); ++p) {
        if (p == z.spieler || ExtPlayer[z.spieler].IsAlliedTo(p)) continue;
        PlayerVehicleEnum e(p); UnitEx u; while (e.GetNext(u)) if (u.IsLive() && u.isEMPed()) gelaehmt++;
      }
      melde(z, "kampf", mapLynx, z.empZielX, z.empZielY, "EMP eingeschlagen (%d feindliche Fahrzeuge gelaehmt) -- Angriff", gelaehmt);
      z.kampfModus = 2; z.angriffZiel = 0;
      angriffZielSetzen(z, z.aufmarschX, z.aufmarschY);
    }
    return;
  }
  if (z.kampfModus == 2) {
    bilanzAnkunft(z);
    // kein Abbruch bei halber Staerke (Spieler) -- gekaempft wird bis zum Ende, auch beim Zermuerben
    (void)wa;
    const char* abbruch = nullptr;
    if (!z.feindGeb) abbruch = "Feindbasis vernichtet";
    else if (na == 0) abbruch = "Angriffsgruppe aufgerieben";
    else if (z.feindNah > z.eigeneWTWert + gruppenWert(z.wache, nw) + gruppenWert(z.eingreif, nw)) abbruch = "Kolonie wird angegriffen, Wache und Tuerme zu schwach";
    if (abbruch) {
      // Lernen: was hat dieser Angriff gebracht (zerstoerter Feindwert) und gekostet (eigene Verluste)?
      const int zer = z.zerstoertWert - z.angriffZerstoertStart, ver = z.verlorenWert - z.angriffVerlorenStart;
      const double quote = (double)zer / (ver > 0 ? ver : 1);
      const int art = z.angriffArt;
      z.angriffQuote[art] = z.angriffQuote[art] * 0.7 + (quote > 5 ? 5 : quote) * 0.3;
      if (art == 2) {
        const int alt = z.zermuerbAbstand;
        z.zermuerbAbstand = z.angriffQuote[2] < 0.7 ? z.zermuerbAbstand * 3 / 2 : z.zermuerbAbstand * 4 / 5;
        if (z.zermuerbAbstand < 4000) z.zermuerbAbstand = 4000;
        if (z.zermuerbAbstand > 30000) z.zermuerbAbstand = 30000;
        melde(z, "lernen", mapLynx, z.bx, z.by, "Zermuerben: zerstoert %d, verloren %d (Quote %.2f, gleitend %.2f) -- Abstand %d -> %d Ticks",
              zer, ver, quote, z.angriffQuote[2], alt, z.zermuerbAbstand);
      } else
        melde(z, "lernen", mapLynx, z.bx, z.by, "%s: zerstoert %d, verloren %d (Quote %.2f, gleitend %.2f)",
              art == 1 ? "Zielangriff" : "Angriff auf die Basis", zer, ver, quote, z.angriffQuote[art]);
      hinweis(z, "ki::kampfFuehrung/bilanzEnde"); bilanzEnde(z, zer, ver, abbruch);
      z.naechstesSpeichern = 0;
      if (na && sicherAlle(z, z.wache, z.angriff) + sicherAlle(z, z.wache, z.angriff2) > 0) {              // kein Platz in der Wache: heim, bleiben in der Angriffsgruppe
        const LOCATION daheim = engine(z, z.bx, z.by + 4);
        AngriffEnum ge(z); UnitEx v;
        while (ge.GetNext(v)) if (v.IsLive() && v.IsVehicle()) v.DoMove(daheim);
      }
      z.kampfModus = 0; z.angriffWunsch = 0;
      heim(abbruch);
      return;
    }
    int mx = 0, my = 0, k = 0;
    { AngriffEnum ge(z); UnitEx v; while (ge.GetNext(v)) if (v.IsLive()) { const LOCATION lo = v.Location(); mx += edX(z, lo); my += edY(z, lo); k++; } }
    if (k) angriffZielSetzen(z, mx / k, my / k);
    return;
  }
  // Sammelgruppe voll und kein Feind zu uns unterwegs: alle mit einem Befehl in die Angriffsgruppe
  int sw = 0;
  { int dummy; sw = gruppenWert(z.sammel, dummy); }
  // Umverteilung Wache -> Angriff aus der Lage (Spieler, Duell 16: 65 Fahrzeuge, angegriffen wurde mit 7):
  // Daheim bleiben muss nur, was der Feind waehrend des Angriffs neu aufstellen koennte (Potenzial ueber seine
  // jetzige Armee -- die steht in seiner Basis und kaempft gegen unsere Truppe), abzueglich der eigenen Tuerme.
  // Alles darueber hinaus in der Wache ist Ueberschuss und greift mit an.
  // Redistribution: home keeps only what the enemy could newly field during the attack, minus own towers.
  int ueber = 0;
  {
    int dummy;
    const int ww = gruppenWert(z.wache, dummy);
    int halten = (int)((z.feindPotenzial - z.feindArmee) * PAR(z).heimHalten) - z.eigeneWTWert;
    if (halten < 0) halten = 0;
    ueber = ww - halten;
    // Spieler 2026-11-07: riskanter, mehr aus der Verteidigung greift mit an (gemessen +915 +-399)
    if (ueber < ww * PAR(z).risikoAnteil) ueber = (int)(ww * PAR(z).risikoAnteil);
    if (ueber < 0 || z.feindUnterwegs > 0) ueber = 0;
  }
  int gesamt = sw + ueber;
  // Nur zaehlen, was wirklich mitfaehrt: Angriffsgruppe fasst 32 (Reste vom letzten Angriff, Sammelgruppe, Rest aus der Wache).
  // Bisher zaehlte der ganze Wache-Ueberschuss: 32 Fahrzeuge, E 220, gegen 300-400 Abwehr. Count only what fits in the group.
  {
    int nA = 0, nS = 0;
    const int wA = angriffWert(z, nA), wS = gruppenWert(z.sammel, nS);
    const int plaetze = 2 * GRUPPE_MAX;
    int platz = plaetze - nA - nS;
    if (platz <= 0) gesamt = nA + nS > 0 ? (wA + wS) * plaetze / (nA + nS) : 0;
    else {
      gesamt = wA + wS;
      int bew = 0;
      GroupEnumerator ge(z.wache); UnitEx v; int seen[64], ns2 = 0;
      while (ge.GetNext(v) && platz > 0 && bew < ueber) {
        if (!v.IsLive() || !kaempfer(v)) continue;
        bool schon = false; for (int i = 0; i < ns2; ++i) if (seen[i] == v.unitID) schon = true;
        if (schon) continue;
        if (ns2 < 64) seen[ns2++] = v.unitID;
        const int kw = kampfWert(v);
        if (bew > 0 && bew + kw > ueber + kw / 2) continue;
        bew += kw; --platz;
      }
      gesamt += bew;
    }
  }
  auto losschlagen = [&](int art, int wunsch, const char* text) {
    if (sicherAlle(z, z.angriff, z.sammel) > 0) sicherAlle(z, z.angriff2, z.sammel);
    // Ueberschuss der Wache dazu: fertige, ruhige Kaempfer, bis der Wert erreicht ist
    int bewegt = 0, nb = 0;
    if (ueber > 0) {
      UnitEx raus[64]; int nr = 0;
      { GroupEnumerator ge(z.wache); UnitEx v;
        while (ge.GetNext(v) && nr < 64) {
          if (!v.IsLive() || !kaempfer(v)) continue;
          bool schon = false; for (int i = 0; i < nr; ++i) if (raus[i].unitID == v.unitID) schon = true;
          if (!schon) raus[nr++] = v;
        } }
      for (int i = 0; i < nr && bewegt < ueber; ++i) {
        const int kw = kampfWert(raus[i]);
        if (bewegt > 0 && bewegt + kw > ueber + kw / 2) continue;
        const bool ersteVoll = z.angriff.TotalUnitCount() >= GRUPPE_MAX;
        if (ersteVoll && z.angriff2.TotalUnitCount() >= GRUPPE_MAX) break;
        z.wache.RemoveUnit(raus[i]);
        (ersteVoll ? z.angriff2 : z.angriff).TakeUnit(raus[i]);
        bewegt += kw; nb++;
      }
      if (nb) melde(z, "kampf", fz ? fz : mapLynx, z.bx, z.by, "Wache gibt %d Fahrzeuge (Wert %d) an den Angriff ab -- daheim bleibt, was der Feind neu aufstellen koennte",
                    nb, bewegt);
    }
    sw += bewegt; ns += nb;
    if (ns <= 0) { z.angriffWunsch = 0; return; }        // nichts zum Losschicken (Duell 18: Angriffe mit 0 Fahrzeugen)
    z.angriffZiel = 0; z.angriffWunsch = wunsch; z.angriffArt = art;
    // Mit EMP-Rakete: erst Aufmarsch knapp ausserhalb des EMP-Radius vor der dichtesten Abwehr (Modus 3), dann Start
    // (Modus 4), Angriff nach dem Einschlag. With a missile: stage just outside the EMP radius, launch, then attack.
    bool mitEmp = false;
    if (!(z.empRakete && empZiel(z, z.empZielX, z.empZielY) > 0)) {
      // Sammeln vor jedem Angriff (Messung: beim ersten Kontakt waren nur 4-35 % der Truppe am Ziel): auf der Linie zur
      // eigenen Basis, ausserhalb der Reichweite aller Tuerme; Modus 3 wartet, bis die Truppe steht. Stage every attack.
      int zx = z.feindX, zy = z.feindY;
      { UnitEx w; w.unitID = wunsch; if (wunsch && w.IsLive()) { const LOCATION l0 = w.Location(); zx = edX(z, l0); zy = edY(z, l0); } }
      double rx = z.bx - zx, ry = z.by - zy;
      const double d = std::sqrt(rx * rx + ry * ry) > 1 ? std::sqrt(rx * rx + ry * ry) : 1;
      z.aufmarschX = z.bx; z.aufmarschY = z.by;
      for (int abst = 10; abst <= 40 && abst < d; ++abst) {
        const int ax = (int)(zx + rx / d * abst + 0.5), ay = (int)(zy + ry / d * abst + 0.5);
        if (!aufKarte(z, ax - 2, ay - 2, 2) || !aufKarte(z, ax + 2, ay + 2, 2) || !zelleBefahrbar(zelle(z, ax, ay))) continue;
        bool frei = true;
        for (int p = 0; p < TethysGame::NoPlayers() && frei; ++p) {
          if (p == z.spieler || ExtPlayer[z.spieler].IsAlliedTo(p)) continue;
          PlayerBuildingEnum e(p, mapGuardPost); UnitEx g;
          while (e.GetNext(g) && frei) { if (!g.IsLive()) continue; const LOCATION gl = g.Location(); if (abstand2(edX(z, gl), edY(z, gl), ax, ay) <= 9 * 9) frei = false; }
        }
        if (frei) { z.aufmarschX = ax; z.aufmarschY = ay; break; }
      }
      z.aufmarschSeit = t; mitEmp = true;
      MAP_RECT warte(engine(z, z.aufmarschX - 2, z.aufmarschY - 2), engine(z, z.aufmarschX + 2, z.aufmarschY + 2));
      const MAP_RECT schutz(engine(z, z.aufmarschX - 5, z.aufmarschY - 5), engine(z, z.aufmarschX + 5, z.aufmarschY + 5));
      wachen(z.angriff, warte, &schutz, 1);
      if (z.angriff2An) wachen(z.angriff2, warte, &schutz, 1);
      melde(z, "kampf", fz ? fz : mapLynx, z.aufmarschX, z.aufmarschY, "Sammeln bei %d,%d vor dem Angriff auf %d,%d", z.aufmarschX, z.aufmarschY, zx, zy);
    } else if (z.empRakete && empZiel(z, z.empZielX, z.empZielY) > 0) {
      double rx = z.bx - z.empZielX, ry = z.by - z.empZielY;
      const double d = std::sqrt(rx * rx + ry * ry) > 1 ? std::sqrt(rx * rx + ry * ry) : 1;
      // so nah wie moeglich, aber ausserhalb der Reichweite feindlicher Tuerme, die die Rakete nicht trifft
      // (EMP-Test im Spiel: zweimal wurde die Truppe am Aufmarschpunkt aufgerieben, bevor die Rakete startete)
      for (int abst = EMP_RADIUS + EMP_AUFMARSCH; abst <= EMP_RADIUS + EMP_AUFMARSCH + 12; ++abst) {
        z.aufmarschX = (int)(z.empZielX + rx / d * abst + 0.5);
        z.aufmarschY = (int)(z.empZielY + ry / d * abst + 0.5);
        bool frei = true;
        for (int p = 0; p < TethysGame::NoPlayers() && frei; ++p) {
          if (p == z.spieler || ExtPlayer[z.spieler].IsAlliedTo(p)) continue;
          PlayerBuildingEnum e(p, mapGuardPost); UnitEx g;
          while (e.GetNext(g) && frei) {
            if (!g.IsLive()) continue;
            const LOCATION gl = g.Location(); const int gx = edX(z, gl), gy = edY(z, gl);
            const bool getroffen = abstand2(gx, gy, z.empZielX, z.empZielY) <= EMP_RADIUS * EMP_RADIUS;
            if (!getroffen && abstand2(gx, gy, z.aufmarschX, z.aufmarschY) <= 9 * 9) frei = false;   // Turmreichweite + Rand
          }
        }
        if (frei) break;
      }
      z.aufmarschSeit = t; mitEmp = true;
      // Wartezone der Angriffsgruppe = Aufmarschpunkt (sonst zieht sie die alte Wartezone des letzten Angriffs an);
      // bewacht wird der Aufmarschpunkt selbst -- wer dort angegriffen wird, wehrt sich
      MAP_RECT warte(engine(z, z.aufmarschX - 2, z.aufmarschY - 2), engine(z, z.aufmarschX + 2, z.aufmarschY + 2));
      const MAP_RECT schutz(engine(z, z.aufmarschX - 5, z.aufmarschY - 5), engine(z, z.aufmarschX + 5, z.aufmarschY + 5));
      wachen(z.angriff, warte, &schutz, 1);
      if (z.angriff2An) wachen(z.angriff2, warte, &schutz, 1);
      melde(z, "kampf", mapEMPMissile, z.aufmarschX, z.aufmarschY, "Aufmarsch bei %d,%d vor der Abwehr bei %d,%d -- EMP-Rakete startet, wenn die Truppe steht",
            z.aufmarschX, z.aufmarschY, z.empZielX, z.empZielY);
    } else angriffZielSetzen(z, z.bx, z.by);
    z.angriffStart = sw; z.kampfModus = mitEmp ? 3 : 2; z.kampfModusSeit = t; z.letzterAngriffStart = t;
    z.angriffZerstoertStart = z.zerstoertWert; z.angriffVerlorenStart = z.verlorenWert;
    hinweis(z, "ki::kampfFuehrung/bilanzStart"); bilanzStart(z);
    float nz = 0, nv = 0;
    if (netzAngriff(z, l, art, ns, sw, nz, nv))
      melde(z, "kampf", fz ? fz : mapLynx, z.feindX, z.feindY, "%s (%d Fahrzeuge, Wert %d) [Netz: zerstoert ~%.0f, verloren ~%.0f]", text, ns, sw, nz, nv);
    else
      melde(z, "kampf", fz ? fz : mapLynx, z.feindX, z.feindY, "%s (%d Fahrzeuge, Wert %d)", text, ns, sw);
  };
  // "bereit": Sammelgruppe voll, oder die Wache hat Ueberschuss (Basis- und Zielangriff pruefen zusaetzlich die Staerke)
  // -- und die Truppe ist staerker als beim letzten Angriff (Eskalation, PAR steigerung). Wer zu lange nicht
  // losschlaegt, darf mit gleicher Staerke (nach einem Zermuerbungsabstand) bzw. ohne diese Bedingung (nach zweien).
  // Escalation: each attack must exceed the last one; relaxed after one / two harassment intervals of waiting.
  int eskalation = 0;
  if (z.angriffStart > 0) {
    const int warten = t - z.letzterAngriffStart;
    eskalation = warten > 2 * z.zermuerbAbstand ? 0 : warten > z.zermuerbAbstand ? z.angriffStart : (int)(z.angriffStart * PAR(z).steigerung);
  }
  const bool bereit = (ns >= sammelZiel || ueber > 0) && gesamt >= eskalation;
  // Steht eine EMP-Rakete bereit, zaehlt die Abwehr, die sie laehmt, beim Kraftvergleich nicht mit
  int empAbzug = 0;
  if (z.empRakete) { int ex, ey; empAbzug = empZiel(z, ex, ey); }
  if (z.feindUnterwegs == 0 && z.feindGeb && gesamt > 0 && (ns + (ueber > 0 ? 2 : 0)) >= 2) {
    // a) ganze Feindbasis, wenn die Truppe Tuerme plus Armee mit 50 % Reserve schlaegt
    // Kampfmodell: wer staerker ist als die ganze Abwehr, gewinnt -- Ueberlegenheit unter 1 ergibt dann keinen Sinn
    const double ueberleg = PAR(z).ueberlegenheit < 1 ? 1.0 : PAR(z).ueberlegenheit;
    if (bereit && gesamt >= (feindWTWert + z.feindArmee - empAbzug) * ueberleg) {
      char text[160]; std::snprintf(text, sizeof(text), "Angriff auf die Feindbasis bei %d,%d gegen %d Wachtuerme und Armee %d", z.feindX, z.feindY, z.feindWT, z.feindArmee);
      losschlagen(0, 0, text);
      return;
    }
    // b) Zermuerben (Spieler: "einfach angreifen und den Feind zermuerben, auch wenn der Angriff keinen Erfolg hat"): das
    //    Feindgebaeude mit der schwaechsten oertlichen Abwehr (Tuerme in 7 Kacheln plus die ganze Feindarmee). Der eigene
    //    Zielangriff des alten Kampfmodells ist entfallen. Harassment of the least defended building.
    UnitEx schwaechstes; int schwachAbwehr = 1 << 30;
    for (int p = 0; p < TethysGame::NoPlayers(); ++p) {
      if (p == z.spieler || ExtPlayer[z.spieler].IsAlliedTo(p)) continue;
      for (int ty : GEBAEUDE) {
        if (ty == mapGuardPost) continue;
        PlayerBuildingEnum e(p, (map_id)ty);
        UnitEx b;
        while (e.GetNext(b)) {
          if (!b.IsLive()) continue;
          const LOCATION lo = b.Location();
          const int bx = edX(z, lo), by = edY(z, lo);
          int abwehr = 0;
          { PlayerBuildingEnum eg(p, mapGuardPost); UnitEx g;
            while (eg.GetNext(g)) if (g.IsLive()) { const LOCATION gl = g.Location(); if (abstand2(edX(z, gl), edY(z, gl), bx, by) <= 7 * 7) abwehr += kampfWert(g); } }
          // Die ganze Feindarmee zaehlt: sie steht in der Basis und ist in Sekunden da (Duell 17: nur Fahrzeuge in
          // 10 Kacheln gezaehlt -> Angriffe mit 1-4 Fahrzeugen alle 2000 Ticks, beide Seiten verbluteten).
          // The whole enemy army counts as defence; it is home and arrives within seconds.
          abwehr += z.feindArmee;
          if (abwehr < schwachAbwehr) { schwachAbwehr = abwehr; schwaechstes = b; }
        }
      }
    }
    // Zermuerben: Sammelgruppe voll (oder am Deckel) und der gelernte Abstand seit dem letzten Angriff ist um
    // Zermuerben nur mit voller Sammelgruppe -- der Wache-Ueberschuss allein reicht nicht (sonst jedes neue Fahrzeug)
    // Neu: nur, wenn sich der Angriff laut Kampfmodell lohnt. Lanchester (Staerke E gegen Abwehr D): der Schwaechere geht
    // ganz verloren, der Staerkere verliert den Anteil 1 - Wurzel(1 - (schwach/stark)^2). Gewertet wie die Punkte: Zerstoerung
    // /10, Verluste /20. Die Bilanz zeigte: fast jeder Zermuerbungsangriff wurde aufgerieben, bei negativer Rendite.
    // Harassment only if the expected exchange is worth it (Lanchester, weighted like the score).
    double zermuerbGewinn = 1;
    if (schwaechstes.unitID) {
      const double e = gesamt, d = feindWTWert + z.feindArmee - empAbzug > 0 ? feindWTWert + z.feindArmee - empAbzug : 0;   // die ganze Abwehr: der Weg fuehrt durch die Tuerme
      const double zer = e >= d ? d : d * (1 - std::sqrt(1 - (e / (d > 0 ? d : 1)) * (e / (d > 0 ? d : 1))));
      const double ver = e >= d ? e * (1 - std::sqrt(1 - (d / (e > 0 ? e : 1)) * (d / (e > 0 ? e : 1)))) : e;
      zermuerbGewinn = zer / 10 - ver / 20;
    }
    const double gewinnMin = PAR(z).angriffMarge * gesamt;   // knapper Gewinn: nur die Streuung
    if (schwaechstes.unitID && gesamt > 0 && zermuerbGewinn > gewinnMin && (ns >= sammelZiel || ueber > 0) && gesamt >= eskalation && t - z.letzterAngriffStart > z.zermuerbAbstand) {
      char text[160]; std::snprintf(text, sizeof(text), "Zermuerben: Angriff auf %s (oertliche Abwehr %d, gelernter Abstand %d Ticks)",
                                    name(schwaechstes.GetType()), schwachAbwehr, z.zermuerbAbstand);
      losschlagen(2, schwaechstes.unitID, text);
      return;
    }
  }
}

// Garage (gemessen 2026-11-06): Fahrzeug per DoDock aufs Dock, dort DoTransferCargo(Bucht) -> faehrt hinein, wird von selbst
// repariert (~0,87 HP je Tick, halber Neupreis je HP, auch unter 50 % Schaden); DoTransferCargo(Bucht) holt es wieder aufs
// Dock, von dort muss es weg, sonst kommt nichts mehr heraus. PutInGarage verschluckt das Fahrzeug -- nicht benutzen.
// Spinnen und Skorpione passen nicht hinein. Garage: dock, park in a bay, auto repair, take out, clear the dock.
inline void garageAblauf(Zustand& z, const Lage& l) {
  const int t = TethysGame::Tick();
  UnitEx g; bool da = false;
  { PlayerBuildingEnum e(z.spieler, mapGarage); UnitEx u;
    while (e.GetNext(u)) if (u.IsLive() && u.GetCurAction() != moObjWait && u.GetCurAction() != moDevelop) { g = u; da = true; break; } }
  if (!da) { for (int i = 0; i < 6; ++i) { z.garBucht[i] = 0; z.garFrei[i] = 0; } z.garDran = z.garRaus = 0; return; }
  if (!g.HasPower() || !g.HasWorkers()) return;
  const LOCATION dock = g.GetDockLocation();
  // 1. Frisch herausgeholt: vom Dock weg und zurueck zur Wache
  hinweis(z, "ki::garage/1");
  if (z.garRaus) {
    // Erst wenn es wirklich wieder auf der Karte steht: waehrend der Ausfahrt ist es nicht in der Fahrzeugliste, ein
    // Befehl dann stuerzt ab (00475EF5). Only once it is back in the vehicle list; orders during the exit animation crash.
    bool draussen = false; UnitEx v;
    { PlayerVehicleEnum e(z.spieler); UnitEx u; while (e.GetNext(u)) if (u.unitID == z.garRaus) { v = u; draussen = true; break; } }
    // Ausfahrt dauert: frueher angefasst stuerzt GetType ab (00475EF5), obwohl IsLive schon stimmt. Marker-Test 2026-11-06.
    const LOCATION rl = draussen ? v.Location() : dock;
    if (draussen && t - z.garRausSeit >= 100 && v.IsLive() && v.GetCurAction() == moDone && rl.x == dock.x && rl.y == dock.y) {
      z.garRaus = 0;                                                  // zuerst: ein Absturz darf den Ablauf nicht festhalten
      LOCATION hin = engine(z, z.bx, z.by + 4);
      hinweis(z, "ki::garage/1b");
      v.DoMove(hin); sicherNehmen(z, z.wache, v);
      hinweis(z, "ki::garage/1c");
      melde(z, "reparatur", mapGarage, edX(z, dock), edY(z, dock), "Garage: Fahrzeug repariert (Schaden jetzt %d) zurueck zur Wache", v.GetDamage());
    } else if (t - z.garRausSeit > 800) z.garRaus = 0;
    return;
  }
  hinweis(z, "ki::garage/2");
  // Die Reparatur braucht Erz: ohne Erz steht sie still, die Frist laeuft dann nicht weiter (Test: Lynx kam unrepariert heraus)
  if (l.erz < 50) for (int i = 0; i < 6; ++i) if (z.garBucht[i]) z.garFrei[i] += TAKT;
  // 2. Fertig reparierte Bucht leeren, wenn das Dock frei ist
  if (!z.garDran) for (int i = 0; i < 6; ++i) if (z.garBucht[i] && t >= z.garFrei[i]) {
    g.DoTransferCargo(i); z.garRaus = z.garBucht[i]; z.garRausSeit = t; z.garBucht[i] = 0;
    return;
  }
  // 3. Unterwegs zum Dock: dort angekommen in eine freie Bucht
  hinweis(z, "ki::garage/3");
  if (z.garDran) {
    UnitEx v; v.unitID = z.garDran;
    if (!v.IsLive() || !v.IsVehicle() || t - z.garDranSeit > 2500) {
      if (v.IsLive() && v.IsVehicle()) sicherNehmen(z, z.wache, v);
      z.garDran = 0; return;
    }
    const LOCATION vl = v.Location();
    if (v.GetCurAction() == moDone && vl.x == dock.x && vl.y == dock.y) {
      for (int i = 0; i < 6; ++i) if (!z.garBucht[i]) {
        const int d = v.GetDamage();
        g.DoTransferCargo(i); z.garBucht[i] = v.unitID; z.garFrei[i] = t + d * 100 / 70 + 200;
        z.garDran = 0; z.garZaehl++;
        melde(z, "reparatur", v.GetType(), edX(z, dock), edY(z, dock), "Garage: %s (Schaden %d) in Bucht %d, frei ab Tick %d", name(v.GetType()), d, i, z.garFrei[i]);
        return;
      }
    } else if (v.GetCurAction() == moDone) v.DoDock(dock);
    return;
  }
  hinweis(z, "ki::garage/4");
  // 4. Naechstes Fahrzeug: am staerksten beschaedigt, aus der Wache, Erz fuer die Reparatur da, kein Feind in der Naehe
  if (t % 100 >= TAKT || z.feindUnterwegs > 0 || z.feindNah > 0) return;
  bool freiB = false; for (int i = 0; i < 6; ++i) if (!z.garBucht[i]) freiB = true;
  if (!freiB) return;
  GroupEnumerator ge(z.wache); UnitEx v, beste; int besteD = -1;
  while (ge.GetNext(v)) {
    if (!v.IsLive() || !kaempfer(v) || v.GetType() == mapScorpion || v.GetCurAction() != moDone) continue;
    UnitInfo i((map_id)v.GetType()); const int hp = i.GetHitPoints(z.spieler);
    if (hp <= 0 || v.GetDamage() * 100 / hp < 30) continue;
    if (v.GetDamage() > besteD) { besteD = v.GetDamage(); beste = v; }
  }
  if (besteD < 0 || l.erz < 200) return;
  hinweis(z, "ki::garage/5");
  z.wache.RemoveUnit(beste); beste.DoDock(dock); z.garDran = beste.unitID; z.garDranSeit = t;
}

// Reparatur (Spieler: "Reparatur ist wichtig"; Regel des Spielers 2026-11-09): ein freies Reparaturfahrzeug (Eden) bzw.
// eine Spinne (Plymouth) wird genommen; ist gerade keins frei, hilft ein ConVec aus -- der repariert nur Gebaeude, und nur,
// wenn ihn kein wartender Bau braucht (Anspruch der Bauauftraege). Tokamaks ab 5 % Schaden (sie verlieren von selbst
// 1 HP je 100 Ticks, die Explosionsgefahr steigt mit dem Schaden), alle anderen Gebaeude sofort. Nicht mitten im Gefecht.
// Repair: a free repair unit first, else a ConVec (buildings only, not when a build needs it). Tokamaks from 5 %, others at once.
inline void reparatur(Zustand& z, const Lage& l) {
  const int t = TethysGame::Tick();
  if (t % 100 >= TAKT) return;
  UnitEx frei[6];
  int nf = 0;
  {
    PlayerVehicleEnum e(z.spieler);
    UnitEx u;
    while (e.GetNext(u)) {
      const int ty = u.GetType();
      if (ty != mapRepairVehicle && ty != mapSpider) continue;
      if (u.GetCurAction() == moDone && !u.IsBusy() && nf < 6) frei[nf++] = u;
    }
  }
  struct Ziel { UnitEx u; int wert, proz; bool fz; };
  Ziel beste[6];
  int nb = 0;
  for (int ty : GEBAEUDE) {
    PlayerBuildingEnum e(z.spieler, (map_id)ty);
    UnitEx b;
    while (e.GetNext(b)) {
      // Im Bau (auch Minen, die der Robo-Miner gerade setzt) zaehlt die fehlende Bauleistung als Schaden --
      // das ist keine Reparatur (Kampf 6: ConVec zur halb fertigen Mine geschickt).
      const int akt = b.GetCurAction();
      if (akt == moObjWait || akt == moDevelop || akt == moOperationalWait) continue;
      const int dmg = b.GetDamage();
      if (dmg <= 0) continue;
      UnitInfo info((map_id)ty);
      const int hp = info.GetHitPoints(z.spieler);
      const int proz = hp > 0 ? dmg * 100 / hp : 0;
      if (ty == mapTokamak && proz < 5) continue;
      const LOCATION lo = b.Location();
      const int bx = edX(z, lo), by = edY(z, lo);
      bool gefecht = false;
      for (int f = 0; f < z.feindPosN && !gefecht; ++f) if (abstand2(z.feindPosX[f], z.feindPosY[f], bx, by) < 8 * 8) gefecht = true;
      if (gefecht) continue;
      const int wert = proz * 2 + rang(z, l, ty) / 2 + (ty == mapGuardPost ? 40 : 0);
      if (nb == 6 && beste[5].wert >= wert) continue;      // volle Liste: nur Bessere (vorher las das Fuellen von Platz 5 beste[5] ungeschrieben)
      int k = nb < 6 ? nb++ : 5;
      beste[k] = { b, wert, proz, false };
      while (k > 0 && beste[k].wert > beste[k - 1].wert) { Ziel tmp = beste[k]; beste[k] = beste[k - 1]; beste[k - 1] = tmp; --k; }
    }
  }
  // Kampffahrzeuge in Basisnaehe: Reparaturfahrzeug und Spinne schaffen nur bis 50 % Schaden (gemessen), Preis je HP der
  // halbe Neupreis. Nur ueber 50 %, nur mit echtem Reparaturgeraet, nicht im Gefecht. Vehicles above 50 % damage near base.
  if (nf) {
    PlayerVehicleEnum e(z.spieler); UnitEx v;
    while (e.GetNext(v)) {
      if (!v.IsLive() || !kaempfer(v) || v.GetType() == mapScorpion) continue;
      UnitInfo info((map_id)v.GetType()); const int hp = info.GetHitPoints(z.spieler);
      const int proz = hp > 0 ? v.GetDamage() * 100 / hp : 0;
      if (proz <= 50) continue;
      const LOCATION lo = v.Location(); const int vx = edX(z, lo), vy = edY(z, lo);
      if (abstand2(vx, vy, z.bx, z.by) > 30 * 30) continue;
      bool gefecht = false;
      for (int f = 0; f < z.feindPosN && !gefecht; ++f) if (abstand2(z.feindPosX[f], z.feindPosY[f], vx, vy) < 10 * 10) gefecht = true;
      if (gefecht) continue;
      const int wert = (proz - 50) * 2 + 40;
      if (nb == 6 && beste[5].wert >= wert) continue;      // volle Liste: nur Bessere (vorher las das Fuellen von Platz 5 beste[5] ungeschrieben)
      int k = nb < 6 ? nb++ : 5;
      beste[k] = { v, wert, proz, true };
      while (k > 0 && beste[k].wert > beste[k - 1].wert) { Ziel tmp = beste[k]; beste[k] = beste[k - 1]; beste[k - 1] = tmp; --k; }
    }
  }
  if (!nb) return;
  if (!nf) {
    // ConVec: nur Gebaeude, und nur, wenn mehr ConVecs frei sind, als Bauauftraege (offene Plaetze, fertige Bausaetze) warten
    int g = -1;
    for (int i = 0; i < nb && g < 0; ++i) if (!beste[i].fz) g = i;
    if (g < 0) return;
    beste[0] = beste[g]; nb = 1;
    int auftraege = l.kitsFabrik, ruhig = 0;
    for (int i = 0; i < z.anzahlPlaetze; ++i) if (z.plaetze[i].zustand == P_FAHRT && !z.plaetze[i].convec) auftraege++;
    PlayerVehicleEnum e(z.spieler);
    UnitEx u, kandidat;
    while (e.GetNext(u))
      if (u.GetType() == mapConVec && u.GetCurAction() == moDone && !u.IsBusy() && (u.GetCargo() <= 0 || u.GetCargo() == mapNone)) { ruhig++; if (!kandidat.unitID) kandidat = u; }
    if (!kandidat.unitID || ruhig <= auftraege) return;
    frei[nf++] = kandidat;
  }
  for (int i = 0; i < nf && i < nb; ++i) {
    frei[i].DoRepair(beste[i].u);
    const LOCATION lo = beste[i].u.Location();
    if (selten(z, 26 + (i & 1), 300))
      melde(z, "reparatur", beste[i].u.GetType(), edX(z, lo), edY(z, lo), "%s hat %d %% Schaden -- %s repariert", name(beste[i].u.GetType()),
            beste[i].proz, frei[i].GetType() == mapConVec ? "ConVec" : "Reparaturfahrzeug");
  }
}

// ---------------------------------------------------------------------------------------------------
// Forschung / research: freie Labore suchen sich ein Thema
// Ob DoResearch die techID oder den Tabellenindex will, ist nicht sicher dokumentiert -- die KI prueft
// nach dem ersten Auftrag, ob das Labor arbeitet, und wechselt sonst die Lesart (steht in der Spur).

inline int laborTyp(int unitTyp) {
  return unitTyp == mapBasicLab ? ltBasic : unitTyp == mapStandardLab ? ltStandard : unitTyp == mapAdvancedLab ? ltAdvanced : 0;
}

// Technik, die Einheiten ploetzlich Seltenerz kosten laesst (z.B. 5317 Verstaerkte Fahrzeugkonstruktion: ConVec 150,
// Lastwagen 100, Evakuierung 100 Seltenerz), erst forschen, wenn eine Seltenerz-Mine steht (Spieler). Ohne Mine
// sperrt sie den Nachschub an Lastwagen und ConVecs -- und ohne Lastwagen kommt nie Seltenerz.
// A tech that adds a rare-ore cost to units that had none waits until a rare ore mine stands.
inline bool hatTechId(const Zustand& z, int id);
inline bool techVerteuert(const Zustand& z, int id) {
  { PlayerBuildingEnum e(z.spieler, mapRareOreMine); UnitEx m; while (e.GetNext(m)) if (m.IsLive()) return false; }
  const tb::Tech* t = tb::tech(id);
  if (!t) return false;
  for (int i = 0; i < t->nw; ++i) {
    const tb::Wirkung& w = tb::WIRKUNG[t->w0 + i];
    if (w.eig != tb::E_RARE_REQUIRED || w.wert <= 0 || w.typ <= 0 || w.typ >= 128) continue;
    UnitInfo ui((map_id)w.typ);
    if (ui.GetRareOreCost(z.spieler) == 0) return true;
  }
  return false;
}
// ... auch, wenn eine noch fehlende Voraussetzung so eine Technik ist (rekursiv). Also if a missing prerequisite is.
inline bool wegVerteuert(const Zustand& z, int id, int tiefe = 0) {
  if (tiefe > 12 || hatTechId(z, id)) return false;
  if (techVerteuert(z, id)) return true;
  const int ti = techIndex(id);
  if (ti < 0) return false;
  TechInfo info = Research::GetTechInfo(ti);
  for (int r = 0; r < info.GetNumRequiredTechs(); ++r)
    if (wegVerteuert(z, Research::GetTechInfo(info.GetRequiredTechIndex(r)).GetTechID(), tiefe + 1)) return true;
  return false;
}

inline int themaWaehlen(const Zustand& z, int lab, char* name, int n) {
  static const int vorrang[] = { tcFood, tcMetals, tcPower, tcPopulation, tcMorale, tcBasic, tcVehicles, tcDefense, tcDisaster, tcWeapons, tcSpace };
  const int anzahl = Research::GetTechCount();
  int besteIdx = -1, besterRang = 99, besteKosten = 1 << 30;
  for (int i = 0; i < anzahl; ++i) {
    TechInfo ti = Research::GetTechInfo(i);
    if (!ti.IsValid()) continue;
    if (ti.GetPlayerHasTech() & (1 << z.spieler)) continue;
    if ((int)ti.GetLab() != lab) continue;
    if (techVerteuert(z, ti.GetTechID())) continue;           // wartet auf die Seltenerz-Mine (Lueckenfueller nahm 5317)
    { const int id = ti.GetTechID(); if (id == 3301 || id == 3302 || id == 3306 || id == 3406) continue; }   // Moraltest: kostet Moral
    const int kosten = Player[z.spieler].IsEden() ? ti.GetEdenCost() : ti.GetPlymouthCost();
    if (kosten <= 0) continue;                                     // 0 = frei, -1 = nicht erforschbar
    bool voraussetzungen = true;
    for (int r = 0; r < ti.GetNumRequiredTechs() && voraussetzungen; ++r) {
      TechInfo req = Research::GetTechInfo(ti.GetRequiredTechIndex(r));
      if (!req.IsValid() || !(req.GetPlayerHasTech() & (1 << z.spieler))) voraussetzungen = false;
    }
    if (!voraussetzungen) continue;
    int rang = 50;
    for (int k = 0; k < (int)(sizeof(vorrang) / sizeof(vorrang[0])); ++k) if (vorrang[k] == (int)ti.GetCategory()) { rang = k; break; }
    if (rang < besterRang || (rang == besterRang && kosten < besteKosten)) { besteIdx = i; besterRang = rang; besteKosten = kosten; }
  }
  if (besteIdx >= 0) {
    const char* nm = Research::GetTechInfo(besteIdx).GetTechName();
    std::snprintf(name, n, "%s", nm ? nm : "?");
  }
  return besteIdx;
}

inline bool hatIdx(const Zustand& z, int idx) {
  TechInfo ti = Research::GetTechInfo(idx);
  return ti.IsValid() && (ti.GetPlayerHasTech() & (1 << z.spieler)) != 0;
}

inline int techIndex(int techID) {
  // Zuordnung techID -> Tabellenindex einmal aufbauen (die Vorhersage fragt sehr oft)
  static int ids[512], anzahl = -1;
  if (anzahl < 0) {
    anzahl = Research::GetTechCount();
    if (anzahl > 512) anzahl = 512;
    for (int i = 0; i < anzahl; ++i) { TechInfo ti = Research::GetTechInfo(i); ids[i] = ti.IsValid() ? ti.GetTechID() : -1; }
  }
  for (int i = 0; i < anzahl; ++i) if (ids[i] == techID) return i;
  return -1;
}

// Naechstes erforschbares Thema auf dem Weg zu `zielTech` fuer diesen Labortyp: alle Voraussetzungen
// erfuellt, kostet etwas, nicht schon in einem anderen Labor. Billigstes zuerst. -1 = keins.
// Next researchable topic on the way to `zielTech` for this lab type (cheapest first), -1 = none.
inline int zielThema(const Zustand& z, int zielTech, int lab, const int* belegt, int nb, char* name, int n) {
  const int start = techIndex(zielTech);
  if (start < 0 || hatIdx(z, start)) return -1;
  int stapel[64], gesehen[64];
  int ns = 0, ng = 0;
  stapel[ns++] = start;
  int beste = -1, besteKosten = 1 << 30;
  while (ns) {
    const int i = stapel[--ns];
    bool schon = false;
    for (int k = 0; k < ng; ++k) if (gesehen[k] == i) { schon = true; break; }
    if (schon) continue;
    if (ng < 64) gesehen[ng++] = i;
    if (hatIdx(z, i)) continue;
    TechInfo ti = Research::GetTechInfo(i);
    if (!ti.IsValid()) continue;
    bool alle = true;
    for (int r = 0; r < ti.GetNumRequiredTechs(); ++r) {
      const int req = ti.GetRequiredTechIndex(r);
      if (!hatIdx(z, req)) { alle = false; if (ns < 64) stapel[ns++] = req; }
    }
    if (!alle) continue;
    const int kosten = Player[z.spieler].IsEden() ? ti.GetEdenCost() : ti.GetPlymouthCost();
    if (kosten <= 0) continue;                                    // frei: kommt mit den Voraussetzungen
    if (techVerteuert(z, ti.GetTechID())) continue;           // wartet auf die Seltenerz-Mine
    // GORF-Forschung raus (Spieler): bei Eden schon um Tick 6400, danach GORF-Bau und Recycling vor dem Lynx; ohne sie auch kein
    // Moral-Malus fuer ein fehlendes GORF. No GORF research: it came early and pulled the GORF and recycling ahead.
    { const int id = ti.GetTechID(); if (id == 3301 || id == 3302 || id == 3306 || id == 3406) continue; }   // Moraltest: kostet Moral
    if ((int)ti.GetLab() != lab) continue;
    bool laeuft = false;
    for (int k = 0; k < nb; ++k) if (belegt[k] == ti.GetTechID() || belegt[k] == i) laeuft = true;
    if (laeuft) continue;
    if (kosten < besteKosten) { beste = i; besteKosten = kosten; }
  }
  if (beste >= 0) {
    const char* nm = Research::GetTechInfo(beste).GetTechName();
    std::snprintf(name, n, "%s", nm ? nm : "?");
  }
  return beste;
}

// Vorrang eines Forschungsthemas: 2 Ziel, 1 Kolonie, 0 Lueckenfueller/sonstiges
inline int themaGruppe(int techID) {
  switch (techID) {
    case 3305: case 3401: case 3402: case 3405: case 5110: case 5405: case 3502: case 5104: return 2;
    case 3407: case 3851: case 5051: return 2;                                 // Militaer (Laser/Wachturm, Lynx)
    case 3304: case 3303: case 3406: case 5307: case 3403: return 1;
    default: return 0;
  }
}

// ---------------------------------------------------------------------------------------------------
// Techbaum-Vorhersage / tech tree prediction
//
// Die KI soll den ganzen Baum verstehen (Spieler): Fuer jede Technik sagt sie vorher, was sie besser macht,
// in den Bereichen Kolonie, Kampf, Wirtschaft, Space (Tabelle aus op2_ki_techbaum.hpp, erzeugt aus dem
// Techbaum und den sheets). Gewaehlt wird nach Nutzen pro Forschungskosten -- einschliesslich aller noch
// fehlenden Voraussetzungen auf dem Weg dorthin.
//
// Kampf: Nutzen in SDK-Kampfwert.
//  - Upgrades (Schaden, Feuerrate, Lebenspunkte, Panzerung): Kampfkraft-Modell vorher/nachher aus den
//    aktuellen Werten (UnitInfo je Spieler, enthaelt schon Erforschtes) -- Kraft = Wurzel(Schaden pro Zeit
//    x Haltbarkeit). Der Gewinn ist SDK-Wert der betroffenen eigenen Einheiten x relative Steigerung.
//  - Freischaltungen: Unterschied im SDK-Wert (Panther 7 statt Lynx 5, andere Waffe +1 ...) x Zahl der
//    Einheiten, die davon profitieren. EMP betaeubt nur ein paar Sekunden (Spieler): zaehlt wie eine
//    normale Waffe, kein Multiplikator.
// Wirtschaft: Nutzen in Punkten aus Kapazitaet, Ertrag, Strombedarf und Tempo der eigenen Gebaeude/Fahrzeuge,
// gewichtet nach Bedarf (Strom knapp, Erz knapp).
//
// The AI predicts for every tech what it improves (combat in SDK value, economy in points) and picks by
// benefit per research cost including missing prerequisites.

inline int eigeneKosten(const Zustand& z, const tb::Tech& t) { return Player[z.spieler].IsEden() ? t.eden : t.ply; }

// Nummer 0 ist der Platzhalter "keine Voraussetzung" (Tabellenindex 0): zaehlt als vorhanden. Seit der Absicherung von hatTech
// galt er als fehlend -- wegKosten lieferte -1, alle Techniken ohne Voraussetzung fielen aus der Forschungsplanung, der
// Kindergarten kam erst um Tick 25000 statt ab Tick 3. Tech 0 is the "no prerequisite" placeholder: counts as researched.
inline bool hatTechId(const Zustand& z, int id) { return id == 0 || hatTech(z, id); }

// Kosten aller noch fehlenden Techniken bis einschliesslich `id`; -1 = fuer diese Kolonie nicht erreichbar.
inline int wegKosten(const Zustand& z, int id) {
  int stapel[64], gesehen[96];
  int ns = 0, ng = 0, summe = 0;
  stapel[ns++] = id;
  while (ns) {
    const int t = stapel[--ns];
    bool schon = false;
    for (int k = 0; k < ng; ++k) if (gesehen[k] == t) { schon = true; break; }
    if (schon || hatTechId(z, t)) continue;
    if (ng < 96) gesehen[ng++] = t;
    const tb::Tech* tt = tb::tech(t);
    if (!tt) return -1;
    const int k = eigeneKosten(z, *tt);
    if (k < 0) return -1;
    summe += k;
    const int idx = techIndex(t);
    if (idx < 0) return -1;
    TechInfo ti = Research::GetTechInfo(idx);
    for (int r = 0; r < ti.GetNumRequiredTechs(); ++r) {
      TechInfo rq = Research::GetTechInfo(ti.GetRequiredTechIndex(r));
      if (ns < 64) stapel[ns++] = rq.GetTechID();
    }
  }
  return summe;
}

inline bool istWaffe(int typ) {
  switch (typ) {
    case mapLaser: case mapMicrowave: case mapStickyfoam: case mapRailGun: case mapEMP: case mapRPG: case mapStarflare:
    case mapStarflare2: case mapESG: case mapSupernova: case mapSupernova2: case mapAcidCloud: case mapThorsHammer:
    case mapEnergyCannon: return true;
    default: return false;
  }
}
inline bool istFahrgestell(int typ) { return typ == mapLynx || typ == mapPanther || typ == mapTiger || typ == mapScorpion; }

// SDK-Wert fuer Fahrgestell/Turm + Waffe (wie kampfWert, aber ohne Einheit)
inline int sdkWert(int traeger, int waffe) {
  const int stufe = waffe == mapThorsHammer ? 2 : (waffe == mapLaser || waffe == mapMicrowave || waffe == mapNone) ? 0 : 1;
  switch (traeger) {
    case mapSpider: case mapScorpion: return 4;
    case mapLynx: return 5 + stufe;
    case mapPanther: return 7 + stufe;
    case mapTiger: return 8 + stufe;
    case mapGuardPost: return stufe == 2 ? 7 : 4 + stufe;
    default: return 0;
  }
}


// Eigene Kampfeinheiten nach Traeger+Waffe zusammengefasst (Bestand + was das Soll noch verlangt; mit
// bekanntem Feind mindestens 4 Tuerme und 4 Lynx, damit Upgrades schon vorher einen Wert haben).
struct Trupp { int traeger, waffe, anzahl; };
inline int truppen(const Zustand& z, Trupp* tr, int max) {
  int n = 0;
  auto dazu = [&](int t, int w, int k) {
    for (int i = 0; i < n; ++i) if (tr[i].traeger == t && tr[i].waffe == w) { tr[i].anzahl += k; return; }
    if (n < max) tr[n++] = { t, w, k };
  };
  { PlayerVehicleEnum e(z.spieler); UnitEx u; while (e.GetNext(u)) if (kaempfer(u)) dazu(u.GetType(), u.GetWeapon(), 1); }
  { PlayerBuildingEnum e(z.spieler, mapGuardPost); UnitEx b; while (e.GetNext(b)) dazu(mapGuardPost, b.GetWeapon() == mapNone ? eigeneWaffe(z) : (int)b.GetWeapon(), 1); }
  if (z.bedrohung > 0) {
    int wt = 0, ly = 0;
    for (int i = 0; i < n; ++i) { if (tr[i].traeger == mapGuardPost) wt += tr[i].anzahl; else ly += tr[i].anzahl; }
    if (wt < 4) dazu(mapGuardPost, eigeneWaffe(z), 4 - wt);
    int mehr = (z.sollArmee + mobilSoll(z) - z.eigeneArmee) / WERT_LYNX;
    if (ly + mehr < 4) mehr = 4 - ly;
    if (mehr > 0) dazu(mapLynx, eigeneWaffe(z), mehr);
  }
  return n;
}

inline Kraftwerte werteJetzt(const Zustand& z, int traeger, int waffe) {
  UnitInfo c((map_id)traeger), w((map_id)waffe);
  return { c.GetHitPoints(z.spieler), c.GetArmor(z.spieler), w.GetPenetrationDamage(z.spieler), w.GetConcussionDamage(z.spieler), w.GetReloadTime(z.spieler) };
}

// Kampf-Nutzen einer Technik (ohne Voraussetzungen), in SDK-Kampfwert
inline double kampfNutzen(const Zustand& z, const tb::Tech& t, const Trupp* tr, int nt) {
  double g = 0;
  for (int i = 0; i < t.nw; ++i) {
    const tb::Wirkung& w = tb::WIRKUNG[t.w0 + i];
    for (int k = 0; k < nt; ++k) {
      if (w.typ != tr[k].traeger && w.typ != tr[k].waffe) continue;
      Kraftwerte vor = werteJetzt(z, tr[k].traeger, tr[k].waffe), nach = vor;
      if (w.typ == tr[k].traeger && w.eig == tb::E_HIT_POINTS) nach.hp = w.wert;
      else if (w.typ == tr[k].traeger && w.eig == tb::E_ARMOR) nach.panzer = w.wert;
      else if (w.typ == tr[k].waffe && w.eig == tb::E_RATE_OF_FIRE) nach.reload = w.wert;
      else if (w.typ == tr[k].waffe && w.eig == tb::E_PENETRATION_DAMAGE) nach.pen = w.wert;
      else if (w.typ == tr[k].waffe && w.eig == tb::E_CONCUSSION_DAMAGE) nach.kon = w.wert;
      else continue;
      const double a = kraft(vor, tr[k].waffe), b = kraft(nach, tr[k].waffe);
      if (a > 0 && b > a) g += tr[k].anzahl * sdkWert(tr[k].traeger, tr[k].waffe) * (b / a - 1);
    }
  }
  int besterTraeger = 0, besteWaffe = eigeneWaffe(z);
  for (int k = 0; k < nt; ++k)
    if (tr[k].traeger != mapGuardPost && sdkWert(tr[k].traeger, tr[k].waffe) > sdkWert(besterTraeger, besteWaffe)) { besterTraeger = tr[k].traeger; besteWaffe = tr[k].waffe; }
  int fz = 0, wt = 0;
  for (int k = 0; k < nt; ++k) { if (tr[k].traeger == mapGuardPost) wt += tr[k].anzahl; else fz += tr[k].anzahl; }
  for (int i = 0; i < t.nf; ++i) {
    const int u = tb::FREI[t.f0 + i];
    if (istWaffe(u)) {
      for (int k = 0; k < nt; ++k) {
        const int d = sdkWert(tr[k].traeger, u) - sdkWert(tr[k].traeger, tr[k].waffe);
        if (d > 0) g += d * tr[k].anzahl;
      }
    } else if (istFahrgestell(u)) {
      // Spieler 2026-11-07: ein besseres Fahrgestell ist nichts wert, solange es das erste (Lynx) noch nicht gibt -- Supraleiter,
      // Fahrzeugkraftwerk und Seltenerz kamen bei Eden so vor der Lynx-Forschung. No better chassis before the first one exists.
      const int d = sdkWert(u, besteWaffe) - (besterTraeger ? sdkWert(besterTraeger, besteWaffe) : 0);
      if (d > 0) g += d * (fz > 4 ? fz : 4);
    } else if (u == mapGuardPost && wt == 0) g += 4 * WERT_WT;
    else if (u == mapRepairVehicle && z.bedrohung > 0) g += 6;   // haelt Tuerme und Fahrzeuge im Kampf
    else if (u == mapWall && z.bedrohung > 0) g += 2;
  }
  return g;
}

// Wirtschafts-Nutzen in Punkten: Kapazitaet/Ertrag/Strombedarf/Tempo eigener Einheiten, nach Bedarf gewichtet
inline double wirtschaftNutzen(const Zustand& z, const Lage& l, const tb::Tech& t) {
  double g = 0;
  const bool stromKnapp = l.stromFrei < 150, erzKnapp = l.erz < 3000;
  for (int i = 0; i < t.nw; ++i) {
    const tb::Wirkung& w = tb::WIRKUNG[t.w0 + i];
    if (w.typ < 0 || w.typ >= 128) continue;
    const int n = l.geb[w.typ] + l.fz[w.typ];
    if (!n) continue;
    UnitInfo info((map_id)w.typ);
    switch (w.eig) {
      case tb::E_PRODUCTION_CAPACITY: {
        const int alt = info.GetProductionCapacity(z.spieler);
        if (alt > 0 && w.wert > alt) {
          const bool strom = w.typ == mapTokamak || w.typ == mapMHDGenerator || w.typ == mapSolarPowerArray || w.typ == mapGeothermalPlant;
          g += n * 100.0 * (w.wert - alt) / alt * (strom ? (stromKnapp ? 1.0 : 0.3) : 0.5);
        }
        break;
      }
      case tb::E_IMPROVED:
        if (w.typ == mapCommonOreMine || w.typ == mapCommonOreSmelter) g += n * (erzKnapp ? 40 : 20);
        else g += n * 10;
        break;
      case tb::E_POWER_REQUIRED: {
        const int alt = info.GetPowerRequired(z.spieler);
        if (w.wert < alt) g += n * (alt - w.wert) * (stromKnapp ? 0.5 : 0.2);
        break;
      }
      case tb::E_PRODUCTION_RATE: g += n * 5; break;
      case tb::E_MOVE_SPEED: if (w.typ == mapCargoTruck || w.typ == mapConVec) g += n * 4; break;
      case tb::E_HIT_POINTS: if (w.typ == mapConVec || w.typ == mapCargoTruck) g += n * 2; break;
      default: break;
    }
  }
  return g;
}

// Kolonie-Nutzen in "Bewohnern, denen etwas fehlt": Kindergarten (Geburten), Uni (Ausbildung), Medizin,
// Wohnraum-, Freizeit- und Nahrungskapazitaet nach Luecke. Katastrophenforschung
// ohne messbare Wirkung: 1 (Lueckenfueller).
inline double kolonieNutzen(const Zustand& z, const Lage& l, const tb::Tech& t) {
  const int bew = l.menschen > 0 ? l.menschen : 1;
  double g = 0;
  for (int i = 0; i < t.nf; ++i) {
    const int u = tb::FREI[t.f0 + i];
    if (u == mapNursery && !l.geb[mapNursery]) g += bew;
    else if (u == mapUniversity && !l.geb[mapUniversity]) g += Player[z.spieler].Kids() + 10;
    else if (u == mapMedicalCenter && !l.geb[mapMedicalCenter]) g += bew;
    else if (u == mapRecreationFacility && !l.geb[mapRecreationFacility]) g += bew;
    else if ((u == mapAdvancedResidence || u == mapReinforcedResidence) && l.wohnraum > 0 && l.menschen * 100 / l.wohnraum > 85) g += 10;
    else if (u == mapDIRT || u == mapGORF || u == mapMeteorDefense) g += 2;
  }
  for (int i = 0; i < t.nw; ++i) {
    const tb::Wirkung& w = tb::WIRKUNG[t.w0 + i];
    if (w.typ < 0 || w.typ >= 128 || w.eig != tb::E_PRODUCTION_CAPACITY) continue;
    const int n = l.geb[w.typ];
    if (!n) continue;
    UnitInfo info((map_id)w.typ);
    const int plus = (w.wert - info.GetProductionCapacity(z.spieler)) * n;
    if (plus <= 0) continue;
    if (w.typ == mapResidence) { const int luecke = l.menschen + 10 - l.wohnraum; g += luecke > 0 ? (plus < luecke ? plus : luecke) : plus * 0.2; }
    else if (w.typ == mapMedicalCenter) { const int luecke = bew - l.medKap; g += luecke > 0 ? (plus < luecke ? plus : luecke) : plus * 0.1; }
    else if (w.typ == mapRecreationFacility) { const int luecke = bew - l.freizeitKap; g += luecke > 0 ? (plus < luecke ? plus : luecke) : plus * 0.1; }
    else if (w.typ == mapAgridome) g += l.nahrungNetto < 20 ? plus : plus * 0.1;
    else g += plus * 0.1;
  }
  if (g == 0 && t.kat == 10) g = 1;                                // Katastrophen: Wirkung nicht messbar
  return g;
}
// Space-Nutzen: was das Ziel noch freischalten muss (Spaceport, SULV, EDWARD)
inline double spaceNutzen(const Zustand& z, const Lage& l, const tb::Tech& t) {
  if (z.ziel != 1 || z.zielErreicht) return 0;
  double g = 0;
  for (int i = 0; i < t.nf; ++i) {
    const int u = tb::FREI[t.f0 + i];
    if ((u == mapSpaceport && !l.geb[mapSpaceport]) || u == mapSULV || u == mapEDWARDSatellite) g += 100;
  }
  return g;
}

struct Vorhersage { int id; double nutzen; int kosten; };

// Beste Ziele eines Bereichs (1 Kampf, 2 Wirtschaft) nach Nutzen pro Wegkosten, absteigend. Liefert Anzahl.
inline int bereichZiele(const Zustand& z, const Lage& l, int bereich, Vorhersage* out, int max) {
  Trupp tr[16];
  const int nt = bereich == 1 ? truppen(z, tr, 16) : 0;
  int n = 0;
  for (int i = 0; i < tb::ANZ_TECH; ++i) {
    const tb::Tech& t = tb::TECH[i];
    if (t.bereich != bereich || hatTechId(z, t.id)) continue;
    const int eigen = eigeneKosten(z, t);
    if (eigen < 0) continue;
    if (wegVerteuert(z, t.id)) continue;                    // wartet auf die Seltenerz-Mine
    auto nutzen = [&](const tb::Tech& tt) { return bereich == 1 ? kampfNutzen(z, tt, tr, nt) : bereich == 2 ? wirtschaftNutzen(z, l, tt)
                                                    : bereich == 0 ? kolonieNutzen(z, l, tt) : spaceNutzen(z, l, tt); };
    double v = nutzen(t);
    // Freie Folgetechniken (Kosten 0), die mit dieser fertig werden, zaehlen mit (z.B. 3407 -> Laser, Wachturm)
    for (int j = 0; j < tb::ANZ_TECH; ++j) {
      const tb::Tech& f = tb::TECH[j];
      if (eigeneKosten(z, f) != 0 || hatTechId(z, f.id)) continue;
      const int fi = techIndex(f.id);
      if (fi < 0) continue;
      TechInfo ti = Research::GetTechInfo(fi);
      bool braucht = false, rest = true;
      for (int r = 0; r < ti.GetNumRequiredTechs(); ++r) {
        TechInfo rq = Research::GetTechInfo(ti.GetRequiredTechIndex(r));
        const int rid = rq.GetTechID();
        if (rid == t.id) braucht = true;
        else if (!hatTechId(z, rid)) {
          const tb::Tech* rt = tb::tech(rid);
          if (!rt || eigeneKosten(z, *rt) != 0) rest = false;      // andere Kolonie-Variante (Kosten 0) zaehlt als da
        }
      }
      if (braucht && rest) v += nutzen(f);
    }
    // Forschung fuer Forschung (Spieler): ein Teil des Nutzens der Techniken, die diese direkt voraussetzen, zaehlt
    // mit -- Wegbereiter, die viele Optionen oeffnen, kommen frueher. Enabling value: 30 % of direct dependents.
    for (int j = 0; j < tb::ANZ_TECH; ++j) {
      const tb::Tech& f = tb::TECH[j];
      if (f.id == t.id || hatTechId(z, f.id) || eigeneKosten(z, f) <= 0 || f.bereich != bereich) continue;
      const int fi = techIndex(f.id);
      if (fi < 0) continue;
      TechInfo ti = Research::GetTechInfo(fi);
      for (int r = 0; r < ti.GetNumRequiredTechs(); ++r)
        if (Research::GetTechInfo(ti.GetRequiredTechIndex(r)).GetTechID() == t.id) { v += PAR(z).freischaltWert * nutzen(f); break; }
    }
    if (v <= 0.5) continue;
    const int k = wegKosten(z, t.id);
    if (k <= 0) continue;
    Vorhersage neu = { t.id, v, k };
    // Volle Liste: nur besser als der letzte Eintrag kommt hinein. (Frueher wurde auch beim Fuellen des letzten Platzes mit
    // out[max-1] verglichen -- noch nie geschrieben, Stapelreste: das Ergebnis hing vom Build ab.)
    const bool voll = n >= max;
    if (voll && out[max - 1].nutzen / out[max - 1].kosten >= v / k) continue;
    int pos = voll ? max - 1 : n++;
    out[pos] = neu;
    while (pos > 0 && out[pos].nutzen / out[pos].kosten > out[pos - 1].nutzen / out[pos - 1].kosten) {
      Vorhersage tmp = out[pos]; out[pos] = out[pos - 1]; out[pos - 1] = tmp; --pos;
    }
  }
  return n;
}

// Welcher Labortyp fehlt auf dem Weg zu `id`? 0 = keiner (alle noetigen Labore stehen).
// Which lab type is missing on the way to a tech?
inline int laborImWeg(const Zustand& z, const Lage& l, int id) {
  int stapel[64], gesehen[96]; int ns = 0, ng = 0, fehlt = 0;
  stapel[ns++] = id;
  while (ns) {
    const int t = stapel[--ns];
    bool schon = false;
    for (int k = 0; k < ng; ++k) if (gesehen[k] == t) { schon = true; break; }
    if (schon || hatTechId(z, t)) continue;
    if (ng < 96) gesehen[ng++] = t;
    const tb::Tech* tt = tb::tech(t);
    if (tt && eigeneKosten(z, *tt) > 0) {
      const int typ = tt->lab == 3 ? mapAdvancedLab : tt->lab == 2 ? mapStandardLab : tt->lab == 1 ? mapBasicLab : 0;
      if (typ && !l.geb[typ] && !geplant(z, typ) && (fehlt == 0 || typ == mapStandardLab)) fehlt = typ;   // Standard zuerst
    }
    const int idx = techIndex(t);
    if (idx < 0) continue;
    TechInfo ti = Research::GetTechInfo(idx);
    for (int r = 0; r < ti.GetNumRequiredTechs(); ++r) if (ns < 64) stapel[ns++] = Research::GetTechInfo(ti.GetRequiredTechIndex(r)).GetTechID();
  }
  if (fehlt == mapBasicLab) fehlt = mapStandardLab;                  // Basic Lab gibt es nicht (02106 "NOT AVAILABLE")
  return fehlt;
}

// Labortypen (Bitmaske 1<<lab), die auf dem Weg zu `id` noch forschen muessen (fehlend oder vorhanden)
inline int laborTypenImWeg(const Zustand& z, int id) {
  int stapel[64], gesehen[96]; int ns = 0, ng = 0, maske = 0;
  stapel[ns++] = id;
  while (ns) {
    const int t = stapel[--ns];
    bool schon = false;
    for (int k = 0; k < ng; ++k) if (gesehen[k] == t) { schon = true; break; }
    if (schon || hatTechId(z, t)) continue;
    if (ng < 96) gesehen[ng++] = t;
    const tb::Tech* tt = tb::tech(t);
    if (tt && eigeneKosten(z, *tt) > 0 && tt->lab >= 1 && tt->lab <= 3) maske |= 1 << tt->lab;
    const int idx = techIndex(t);
    if (idx < 0) continue;
    TechInfo ti = Research::GetTechInfo(idx);
    for (int r = 0; r < ti.GetNumRequiredTechs(); ++r) if (ns < 64) stapel[ns++] = Research::GetTechInfo(ti.GetRequiredTechIndex(r)).GetTechID();
  }
  return maske;
}

// Alle 1000 Ticks: fuer jeden Bereich mit Forschungsanteil die besten Ziele pruefen -- fehlt ein Labor auf ihrem Weg,
// bekommt dessen Bau die Dringlichkeit des Bereichs. Every 1000 ticks: labs needed by the best research targets.
inline void laborBedarf(Zustand& z, const Lage& l) {
  const int t = TethysGame::Tick();
  if (t < z.naechsterLaborCheck) return;
  z.naechsterLaborCheck = t + 1000;
  const Dringlichkeit d = dringlichkeit(z, l);
  const int anteil[4] = { d.kolonie, d.kampfForschung, d.wirtschaft, d.ziel };
  int wunsch = 0, wert = 0, fuer = 0;
  int nutzen[4] = { 0, 0, 0, 0 };
  for (int b = 0; b < 4; ++b) {
    if (anteil[b] <= 0) continue;
    Vorhersage v[3];
    const int n = bereichZiele(z, l, b, v, 3);
    for (int i = 0; i < n; ++i) {
      const int maske = laborTypenImWeg(z, v[i].id);
      for (int lt = 1; lt <= 3; ++lt) if ((maske & (1 << lt)) && anteil[b] > nutzen[lt]) nutzen[lt] = anteil[b];
      if (anteil[b] > wert) {
        const int lab = laborImWeg(z, l, v[i].id);
        if (lab) { wunsch = lab; wert = anteil[b]; fuer = v[i].id; }
      }
    }
  }
  for (int lt = 0; lt < 4; ++lt) z.laborNutzen[lt] = nutzen[lt];
  if (wunsch != z.laborWunsch && wunsch)
    melde(z, "info", wunsch, z.bx, z.by, "Forschung braucht ein %s (Ziel %d, Dringlichkeit %d)", name(wunsch), fuer, wert);
  z.laborWunsch = wunsch; z.laborWert = wert; z.laborFuer = fuer;
}

// ---------------------------------------------------------------------------------------------------
// Selbstpruefung (Spieler: "versuche sowas vorzeitig zu erkennen"): Widersprueche zwischen "wird dringend gebraucht"
// und "tut sich nichts" -- mit der Ursache, soweit sie sich aus der Lage ablesen laesst. Meldet als "diagnose".
// Self-check: contradictions between urgent need and no progress, with the cause where it can be read off.
inline const char* stillstandGrund(UnitEx u) {
  if (!u.unitID || !u.IsLive()) return "fehlt";
  if (u.GetCurAction() == moObjWait || u.GetCurAction() == moDevelop) return "im Bau";
  // Gebaeude ohne Strom- und Personalbedarf (Minen, ...) koennen nicht stillstehen (Fehlalarm Duell 9)
  { UnitInfo ui((map_id)u.GetType()); if (ui.GetPowerRequired(0) <= 0 && ui.GetWorkersRequired(0) <= 0 && ui.GetScientistsRequired(0) <= 0) return nullptr; }
  if (u.IsBusy() && !u.HasPower() && !u.HasWorkers() && u.GetCurAction() == moDone) return "stillgelegt";
  if (!u.HasPower()) return "ohne Strom";
  if (!u.HasWorkers()) return "ohne Personal";
  return nullptr;
}

inline void selbstpruefung(Zustand& z, const Lage& l) {
  const int t = TethysGame::Tick();
  if (t < z.naechstePruefung) return;
  z.naechstePruefung = t + 2000;
  // 1. Labore, die die Forschung braucht
  static const int LABTYP[4] = { 0, mapBasicLab, mapStandardLab, mapAdvancedLab };
  for (int lt = 2; lt <= 3; ++lt) {
    if (z.laborNutzen[lt] < 30) continue;
    PlayerBuildingEnum e(z.spieler, (map_id)LABTYP[lt]);
    UnitEx lab;
    const char* grund = "fehlt";
    while (e.GetNext(lab)) {
      if (!lab.IsLive()) continue;   // gerade zerstoert
      grund = stillstandGrund(lab); if (!grund) break;
    }
    if (grund && std::strcmp(grund, "im Bau") != 0)
      melde(z, "diagnose", LABTYP[lt], z.bx, z.by, "Blockade: die Forschung braucht ein %s (Dringlichkeit %d), es %s",
            name(LABTYP[lt]), z.laborNutzen[lt], grund);
  }
  // 2. Kampfwert fehlt, aber seit langem kein Kampffahrzeug
  {
    const int fehlt = kampfFehlt(z, l), mob = mobilSoll(z) - z.eigeneMobil;
    if (fehlt > 0 || mob > 0) { if (!z.kampfBedarfSeit) z.kampfBedarfSeit = t; }
    else z.kampfBedarfSeit = 0;
    const int seit = z.letztesKampfFz > z.kampfBedarfSeit ? z.letztesKampfFz : z.kampfBedarfSeit;
    if (z.kampfBedarfSeit && t - seit > 5000) {
      char grund[200] = "";
      const int fz = fahrgestellWahl(z, 1 << 20);
      if (!fz) {
        const int lab = laborImWeg(z, l, 3851);
        UnitEx al; { PlayerBuildingEnum e(z.spieler, mapAdvancedLab); e.GetNext(al); }
        const char* lg = stillstandGrund(al);
        std::snprintf(grund, sizeof(grund), "kein Kampffahrzeug erforscht (Lynx 3851)%s%s%s%s", lab ? ", es fehlt ein " : "", lab ? name(lab) : "",
                      !lab && lg ? ", Advanced Lab " : "", !lab && lg ? lg : "");
      } else if (!l.vf.unitID) std::snprintf(grund, sizeof(grund), "keine Fahrzeugfabrik");
      else if (const char* vg = stillstandGrund(l.vf)) std::snprintf(grund, sizeof(grund), "Fahrzeugfabrik %s", vg);
      else std::snprintf(grund, sizeof(grund), "%s baubar, Erz %d / Seltenerz %d -- anderes Fahrzeug hatte Vorrang?", name(fz), l.erz, l.selten);
      melde(z, "diagnose", fz ? fz : mapLynx, z.bx, z.by, "Blockade: Kampfwert fehlt seit %d Ticks (Soll %d, eigener %d, mobil %d/%d), kein Kampffahrzeug -- %s",
            t - z.kampfBedarfSeit, z.sollArmee, z.eigeneArmee, z.eigeneMobil, mobilSoll(z), grund);
    }
  }
  // 3. Hohe Bau-Dringlichkeit, aber seit langem kein Bau
  {
    const Dringlichkeit d = dringlichkeit(z, l);
    int m = d.kolonie; const char* b = "Kolonie";
    if (d.wirtschaft > m) { m = d.wirtschaft; b = "Wirtschaft"; }
    if (d.verteidigung > m) { m = d.verteidigung; b = "Verteidigung"; }
    if (m >= 60) { if (!z.bauDrangSeit) z.bauDrangSeit = t; } else z.bauDrangSeit = 0;
    const int seit = z.letzterBau > z.bauDrangSeit ? z.letzterBau : z.bauDrangSeit;
    if (z.bauDrangSeit && t - seit > 5000) {
      const Wahl w = bauWahl(z, l);
      melde(z, "diagnose", w.typ, z.bx, z.by, "Blockade: %s hat Dringlichkeit %d seit %d Ticks, nichts gebaut -- %s",
            b, m, t - z.bauDrangSeit, w.warum[0] ? w.warum : "kein baubarer Kandidat");
    }
  }
  // 4. Wichtige Gebaeude (Rang >= 60), die stillstehen
  for (int ty : GEBAEUDE) {
    if (ty == mapGuardPost) continue;
    const int r = rang(z, l, ty);
    if (r < 60) continue;
    PlayerBuildingEnum e(z.spieler, (map_id)ty);
    UnitEx u;
    while (e.GetNext(u)) {
      const char* g = stillstandGrund(u);
      if (g && std::strcmp(g, "im Bau") != 0)
        melde(z, "diagnose", ty, edX(z, u.Location()), edY(z, u.Location()), "Stillstand: %s (Rang %d) %s", name(ty), r, g);
    }
  }
}

// Spur: die besten Vorhersagen je Bereich (alle 10000 Ticks), damit man sieht, wie die KI den Baum liest
inline void vorhersageMelden(Zustand& z, const Lage& l) {
  const int t = TethysGame::Tick();
  if (t < z.naechsteVorhersage) return;
  z.naechsteVorhersage = t + 10000;
  for (int b = 0; b <= 2; ++b) {
    Vorhersage v[4];
    const int n = bereichZiele(z, l, b, v, 4);
    char text[400]; int pos = 0;
    pos += std::snprintf(text + pos, sizeof(text) - pos, "Vorhersage %s:", tb::BEREICH_NAME[b]);
    for (int i = 0; i < n && pos < 330; ++i) {
      const int idx = techIndex(v[i].id);
      const char* nm = idx >= 0 ? Research::GetTechInfo(idx).GetTechName() : "?";
      pos += std::snprintf(text + pos, sizeof(text) - pos, " %d %.24s +%.1f fuer %d;", v[i].id, nm ? nm : "?", v[i].nutzen, v[i].kosten);
    }
    if (!n) std::snprintf(text + pos, sizeof(text) - pos, " nichts mit Nutzen");
    melde(z, "vorhersage", mapNone, z.bx, z.by, "%s", text);
  }
}

inline void forschung(Zustand& z, const Lage& l) {
  const int t = TethysGame::Tick();
  // Spur: abgeschlossene Forschung (bisher stand nur der Beginn drin). Trace finished research.
  if (t >= z.naechsteTechPruefung) {
    z.naechsteTechPruefung = t + 200;
    for (int i = 0; i < tb::ANZ_TECH && i < 512; ++i)
      if (!z.techGemeldet[i] && hatTech(z, tb::TECH[i].id)) {
        z.techGemeldet[i] = 1;
        if (t > 0) melde(z, "techFertig", mapNone, z.bx, z.by, "Tech %d abgeschlossen", tb::TECH[i].id);
      }
  }
  // Kalibrierung: hat der letzte Auftrag ein Thema gestartet?
  if (z.forschungLab && t - z.forschungSeit > 60) {
    UnitEx lab; lab.unitID = z.forschungLab;
    if (lab.IsLive() && !lab.HasPower()) {
      melde(z, "info", lab.GetType(), z.bx, z.by, "Labor ohne Strom -- Lesart von DoResearch noch offen");
    } else if (lab.IsLive()) {
      const int thema = lab.GetLabCurrentTopic();
      // Nicht mehr umschalten: DoResearch nimmt den Tabellenindex (in allen Laeufen "Lesart Index"). Mit einer techID
      // liest das Spiel hinter die Techtabelle -- das war der seltene Absturz 00472D78 in ki::forschung, wenn ein Labor
      // aus anderem Grund (Strom, Wissenschaftler, Leerlauf) nicht anfing und die Lesart kippte.
      // Keep the table index: a techID makes the engine read past the tech table (crash 00472D78).
      if (thema <= 0 && lab.GetCurAction() != moDoResearch) {
        melde(z, "info", lab.GetType(), z.bx, z.by, "Labor hat nicht angefangen (Strom, Wissenschaftler oder Leerlauf)");
      } else {
        melde(z, "info", lab.GetType(), z.bx, z.by, "Forschung laeuft (Thema %d, Lesart %s)", thema, z.forschungModus ? "Index" : "techID");
      }
    }
    z.forschungLab = 0;
  }
  const int typen[] = { mapBasicLab, mapStandardLab, mapAdvancedLab };
  // Stillgelegte Labore (Leerlauf) wecken, sobald es fuer ihren Labortyp ein Ziel- oder Kolonie-Thema gibt
  // Wake idled labs as soon as there is a goal or colony topic for their lab type.
  // Laeuft eine Forschung unter ihrem Maximum, darf kein weiteres Labor starten (Konzentration) -- dann
  // auch keins wecken, sonst pendelt es zwischen Wecken und Leerlauf.
  bool konzentriert = false;
  if (!standardVolk(l))
    for (int typ : typen) {
      PlayerBuildingEnum e(z.spieler, (map_id)typ);
      UnitEx lab;
      while (e.GetNext(lab)) {
      if (!lab.IsLive()) continue;   // gerade zerstoert (Duell 19: 85x Absturz 00472D78 in forschung)
        const int th = lab.GetLabCurrentTopic();
        if (th <= 0) continue;
        const int i2 = th < Research::GetTechCount() ? th : techIndex(th);
        if (i2 >= 0 && lab.GetLabScientistCount() < Research::GetTechInfo(i2).GetMaxScientists()) konzentriert = true;
      }
    }
  for (int typ : typen) {
    PlayerBuildingEnum e(z.spieler, (map_id)typ);
    UnitEx lab;
    while (e.GetNext(lab)) {
      if (!lab.IsLive()) continue;   // gerade zerstoert (Duell 19: 85x Absturz 00472D78 in forschung)
      if (!istGeparkt(z, lab.unitID) || konzentriert) continue;
      // Braucht die Forschung diesen Labortyp (beste Ziele irgendeines Bereichs)? Dann wecken.
      if (z.laborNutzen[laborTyp(typ)] > 0) {
        char warum[140]; std::snprintf(warum, sizeof(warum), "die Forschung braucht dieses Labor (Dringlichkeit %d)", z.laborNutzen[laborTyp(typ)]);
        leerlaufWecken(z, lab.unitID, warum);
        return;
      }
      static const int WECK_T[] = { 5104, TECH_RAUMFAHRT, 3502, 3304, 3305, 3303, 3406, 5307, 3403 };
      char nm[96]; int leer[1] = { 0 };
      for (int zt : WECK_T) {
        if ((zt == 5104 || zt == TECH_RAUMFAHRT || zt == 3502) && (z.ziel != 1 || z.zielErreicht)) continue;
        if (zielThema(z, zt, laborTyp(typ), leer, 0, nm, sizeof(nm)) >= 0) {
          char warum[140]; std::snprintf(warum, sizeof(warum), "Thema fuer dieses Labor: %s", nm);
          leerlaufWecken(z, lab.unitID, warum);
          return;
        }
      }
    }
  }
  // Themen, an denen gerade ein Labor arbeitet (nicht doppelt vergeben)
  int belegt[16]; int nb = 0;
  for (int typ : typen) {
    PlayerBuildingEnum e(z.spieler, (map_id)typ);
    UnitEx lab;
    while (e.GetNext(lab)) {
      if (!lab.IsLive()) continue;   // gerade zerstoert
      const int th = lab.GetLabCurrentTopic(); if (th > 0 && nb < 16) belegt[nb++] = th;
    }
  }
  for (int typ : typen) {
    PlayerBuildingEnum e(z.spieler, (map_id)typ);
    UnitEx lab;
    while (e.GetNext(lab)) {
      if (!lab.IsLive()) continue;   // gerade zerstoert (Duell 19: 85x Absturz 00472D78 in forschung)
      // Grosslabor fuer den Lynx (Eden-Analyse 2026-11-07): Mobile Waffenplattform braucht das Grosslabor; dort liefen erst die
      // Supraleiter, der Lynx kam um Tick 22000. Fehlt er bei bekanntem Feind, startet hier nichts anderes, Laufendes wird gestoppt.
      // Keep the advanced lab for the Lynx once an enemy is known.
      if (typ == mapAdvancedLab && z.feindGemeldet && !hatTechId(z, 3851) && !standardVolk(l)) {
        const tb::Tech* lt = tb::tech(3851);
        const int li = techIndex(3851);
        if (lt && li >= 0 && eigeneKosten(z, *lt) >= 0) {
          TechInfo lti = Research::GetTechInfo(li);
          bool bereit = true;
          for (int r = 0; r < lti.GetNumRequiredTechs(); ++r)
            if (!hatTechId(z, Research::GetTechInfo(lti.GetRequiredTechIndex(r)).GetTechID())) bereit = false;
          const int th = lab.GetLabCurrentTopic();
          const int ti2 = th > 0 ? (th < Research::GetTechCount() ? th : techIndex(th)) : -1;
          if (ti2 == li) continue;                                   // forscht schon den Lynx
          if (th > 0 || lab.IsBusy() || lab.GetCurAction() == moDoResearch) {
            if (bereit) {
              lab.DoStop();
              melde(z, "forschung", typ, edX(z, lab.Location()), edY(z, lab.Location()), "Grosslabor gestoppt (Thema %d) -- der Lynx hat Vorrang", ti2 >= 0 ? Research::GetTechInfo(ti2).GetTechID() : th);
            }
            continue;
          }
          if (!bereit) {
            if (selten(z, 24, 3000)) melde(z, "warte", typ, edX(z, lab.Location()), edY(z, lab.Location()), "Grosslabor wartet auf den Lynx (Voraussetzungen fehlen noch)");
            continue;
          }
          if (!lab.HasPower()) continue;
          int wiss = ExtPlayer[z.spieler].GetNumAvailableScientists();
          // Zu wenige frei (Test: Start mit 1 Wissenschaftler): andere Labore geben bis auf einen ab, dann einmal kurz warten
          if (wiss < lti.GetMaxScientists() && !z.lynxFreiSeit) {
            int frei = 0;
            for (int typ2 : typen) {
              PlayerBuildingEnum e2(z.spieler, (map_id)typ2); UnitEx l2;
              while (e2.GetNext(l2)) if (l2.IsLive() && l2.unitID != lab.unitID && l2.GetLabCurrentTopic() > 0 && l2.GetLabScientistCount() > 1) {
                frei += l2.GetLabScientistCount() - 1; l2.SetLabScientistCount(1);
              }
            }
            z.lynxFreiSeit = t;
            if (frei) { ExtPlayer[z.spieler].RecalculateValues(); melde(z, "wiss", typ, edX(z, lab.Location()), edY(z, lab.Location()), "andere Labore geben %d Wissenschaftler fuer den Lynx ab", frei); continue; }
          }
          if (wiss > lti.GetMaxScientists()) wiss = lti.GetMaxScientists();
          if (wiss <= 0) continue;
          lab.DoResearch(li, wiss);
          z.forschungLab = lab.unitID; z.forschungWert = li; z.forschungSeit = t;
          melde(z, "forschung", typ, edX(z, lab.Location()), edY(z, lab.Location()), "Kampf (Lynx-Vorrang): Mobile Waffenplattform (Tech 3851, %d Wissenschaftler)", wiss);
          return;
        }
      }
      if (lab.IsBusy() || lab.GetCurAction() == moDoResearch || lab.GetLabCurrentTopic() > 0) continue;
      if (!lab.HasPower()) {
        if (selten(z, 7, 3000)) melde(z, "warte", typ, edX(z, lab.Location()), edY(z, lab.Location()), "Labor ohne Strom");
        continue;
      }
      char nm[96];
      // Forschungs-Kuchen: Kolonie (Kindergarten 3304, Uni 3305, Medizin 3303 -- wiegt schwer, solange die
      // Kolonie klein ist) gegen Ziel (Advanced Lab 5104, Space Program, Vehicle Factory 3502). Der groessere
      // Anteil wird zuerst probiert; die Kette sucht sich das billigste erforschbare Thema selbst.
      // Katastrophen (Seismologie ... Stuerme) sind Lueckenfueller: nur, wenn nichts davon passt.
      // Research pie: colony vs goal, larger share first; disaster research only as gap filler.
      // Kindergarten-Technik zuerst: ohne Kindergarten werden keine Kinder geboren (Hinweis des Spielers),
      // dann Uni, dann Medizin. Danach Kapazitaets-Upgrades -- nur, wenn ihre Luecke besteht: mehr Platz
      // ohne zusaetzliches Personal (Wohnhaus 25->35: 3406, Freizeit 3306 und 40->60: 8307,
      // Medizin 50->75: 5307, Agridome 40->50: 3403).
      // Nursery tech first; then capacity upgrades only where the gap exists (more capacity, no extra staff).
      int KOLONIE_T[10]; int nKol = 0;
      KOLONIE_T[nKol++] = 3304; KOLONIE_T[nKol++] = 3305; KOLONIE_T[nKol++] = 3303;
      if (!standardVolk(l)) {
        const int bew = l.menschen > 0 ? l.menschen : 1;
        const int med = l.medKap * 100 / bew, frz = l.freizeitKap * 100 / bew;
        if (bew + 10 > l.wohnraum) KOLONIE_T[nKol++] = 3406;       // Freizeit (3306/8307) vorerst nicht
        (void)frz;
        if (med < 100) KOLONIE_T[nKol++] = 5307;
        if (l.nahrungNetto < 20) KOLONIE_T[nKol++] = 3403;
      }
      static const int ZIEL_T[] = { 5104, TECH_RAUMFAHRT, 3502 };
      static const int FUELLER_T[] = { 3201, 3202, 5302, 5303 };
      const bool zielOffen = z.ziel == 1 && !z.zielErreicht;
      // Moral nicht "Great" und Kolonie klein: Kolonie-Forschung vor dem Ziel (Spieler)
      const Dringlichkeit dr = dringlichkeit(z, l);
      const int anteilKolonie = dr.kolonie;
      const int anteilZiel = zielOffen ? dr.ziel : 0;                 // bekommt, was uebrig bleibt (kein Zeitdruck)
      int idx = -1;
      const char* art = "";
      static char artText[96];
      auto liste = [&](const int* tl, int n, const char* name) {
        for (int i = 0; i < n && idx < 0; ++i) {
          idx = zielThema(z, tl[i], laborTyp(typ), belegt, nb, nm, sizeof(nm));
          if (idx >= 0) {
            art = name;
            if (Research::GetTechInfo(idx).GetTechID() != tl[i]) {               // Voraussetzung auf dem Weg
              std::snprintf(artText, sizeof(artText), "%sauf dem Weg zu %d -- ", name, tl[i]);
              art = artText;
            }
          }
        }
      };
      // Militaer: Laser + Wachturm (3407), dann Lynx (3851). Anteil = Bedrohung -- ohne bekannten Feind 0.
      // Military: laser + guard post, then Lynx; share = threat, zero without a known enemy.
      // Kampf und Wirtschaft aus der Techbaum-Vorhersage (Nutzen pro Wegkosten); Kampf-Anteil = Bedrohung,
      // Wirtschaft 15, mehr bei Erz- oder Strommangel.
      // Combat and economy topics come from the tech tree prediction.
      const int anteilMilitaer = dr.kampfForschung;
      const int anteilWirtschaft = dr.wirtschaft;
      int KAMPF_T[7], WIRT_T[6], KOL_V[6], RAUM_T[6]; int nKampf = 0, nWirt = 0, nKolV = 0, nRaum = 0;
      bool empHinten = false;
      {
        Vorhersage v[6];
        // Noch gar kein Kampffahrzeug: das erste ist mehr wert als jede Verbesserung -- Mobile Waffenplattform (Lynx)
        // zuerst (Test: Kampfforschung zielte auf den Panther, der erste Angriff traf eine Kolonie ohne Lynx).
        // No combat vehicle yet: the first one (Lynx platform) comes before any upgrade.
        if (anteilMilitaer > 0 && !fahrgestellWahl(z, 1 << 20) && !hatTechId(z, 3851)) KAMPF_T[nKampf++] = 3851;
        // EMP-Rakete (nur Plymouth): lohnt sie gegen die jetzige Abwehr, kommt ihre Technik (mit Weg ueber 5405, 5508)
        // in die Kampfliste -- vorne, wenn die Abwehr doppelt so viel wert ist wie die Rakete kostet, sonst hinten.
        if (anteilMilitaer > 0 && !Player[z.spieler].IsEden() && !hatTechId(z, EMP_TECH) && fahrgestellWahl(z, 1 << 20)) {
          int ew = 0;
          if (empLohnt(z, &ew)) {
            if (ew >= 2 * empKostenWert(z) * PAR(z).empSchwelle) { for (int i = nKampf; i > 0; --i) KAMPF_T[i] = KAMPF_T[i - 1]; KAMPF_T[0] = EMP_TECH; nKampf++; }
            else empHinten = true;
          }
        }
        if (anteilMilitaer > 0) { const int n = bereichZiele(z, l, 1, v, 4); for (int i = 0; i < n; ++i) KAMPF_T[nKampf++] = v[i].id; }
        if (empHinten && nKampf < 6) KAMPF_T[nKampf++] = EMP_TECH;
        int n2 = bereichZiele(z, l, 2, v, 6);
        for (int i = 0; i < n2; ++i) WIRT_T[nWirt++] = v[i].id;
        n2 = bereichZiele(z, l, 0, v, 6);
        for (int i = 0; i < n2; ++i) KOL_V[nKolV++] = v[i].id;
        if (zielOffen) { n2 = bereichZiele(z, l, 3, v, 6); for (int i = 0; i < n2; ++i) RAUM_T[nRaum++] = v[i].id; }
      }
      // Uni vor (Spieler 2026-11-07): Eden begann die Uni-Forschung ~2100 Ticks spaeter, weil der Kuchen nach dem Kindergarten
      // reihum zum Kampf (3401) wechselte; Uni 2200 Ticks spaeter, weniger Arbeiter. Ohne akute Bedrohung kommt sie zuerst.
      // Research the university before combat/economy topics unless under acute threat.
      // erst nach dem Kindergarten: der erste Test zog die Uni davor, Kindergarten erst Tick 10000 statt 4000, weniger Kinder
  const bool uniZuerst = hatTechId(z, 3304) && !hatTechId(z, 3305) && z.bedrohung < 50;
      if (uniZuerst) {
        if (nKolV >= 6) nKolV = 5;
        for (int i = nKolV; i > 0; --i) KOL_V[i] = KOL_V[i - 1];
        KOL_V[0] = 3305; nKolV++;
      }
      struct Anteil { const int* tl; int n; const char* name; int wert; int bereich; double rang; };
      (void)KOLONIE_T; (void)nKol; (void)ZIEL_T;
      Anteil reihe[4] = { { KOL_V, nKolV, "Kolonie (Vorhersage): ", anteilKolonie, 0, 0 },
                          { RAUM_T, nRaum, "Ziel Satellit (Vorhersage): ", anteilZiel, 1, 0 },
                          { KAMPF_T, nKampf, "Kampf (Vorhersage): ", anteilMilitaer, 2, 0 },
                          { WIRT_T, nWirt, "Wirtschaft (Vorhersage): ", anteilWirtschaft, 3, 0 } };
      // Kuchen anteilig (Spieler: Kuchen): jeder Bereich bekommt Themen im Verhaeltnis seines Anteils -- wer im
      // Verhaeltnis zu seinem Anteil am wenigsten Themen bekommen hat, ist dran. Vorher strenge Reihenfolge: Kolonie
      // 100 schlug Vorsorge 60 immer, der Lynx kam nie rechtzeitig. Proportional pie: least served area goes first.
      for (Anteil& a : reihe) a.rang = a.wert > 0 ? a.wert / (1.0 + z.forschGestartet[a.bereich]) : -1;
      // Aufbauphase (Spieler 2026-11-07): solange Uni, Medizin oder -- bei bekanntem Feind -- der Lynx fehlen, kommt Wirtschaftsforschung
      // erst danach; im Kampf zaehlt dann nur der Weg zum Lynx. Growth phase: economy research waits, combat only towards the Lynx.
      if (uniZuerst) reihe[0].rang = 1e9;
      for (int a = 0; a < 4; ++a)
        for (int b2 = a + 1; b2 < 4; ++b2) if (reihe[b2].rang > reihe[a].rang) { Anteil tmp = reihe[a]; reihe[a] = reihe[b2]; reihe[b2] = tmp; }
      int gewaehlt = -1;
      for (int a = 0; a < 4; ++a) {
        const int vorher = idx;
        liste(reihe[a].tl, reihe[a].n, reihe[a].name);
        if (vorher < 0 && idx >= 0) gewaehlt = reihe[a].bereich;
      }
      const bool zielweg = idx >= 0;
      // Lueckenfueller Katastrophen -- nur mit genug freien Wissenschaftlern (die Zielkette geht vor)
      // Lueckenfueller (Katastrophen) nur, wenn das Ziel erreicht ist oder genug Wissenschaftler da sind --
      // sonst bindet z.B. Seismologie die Wissenschaftler, die der Zielkette fehlen (Spieler).
      if (idx < 0 && (!zielOffen || standardVolk(l))) liste(FUELLER_T, 4, "Lueckenfueller Katastrophen: ");
      // Bei echter Bevoelkerung keine weiteren Nebenthemen, solange das Ziel offen ist (Wissenschaftler freihalten)
      if (idx < 0 && zielOffen && Player[z.spieler].Scientists() < 100) {
        if (selten(z, 22, 4000))
          melde(z, "warte", typ, edX(z, lab.Location()), edY(z, lab.Location()),
                "kein Ziel- oder Kolonie-Thema fuer dieses Labor -- haelt %d Wissenschaftler frei", Player[z.spieler].Scientists());
        continue;
      }
      if (idx < 0) idx = themaWaehlen(z, laborTyp(typ), nm, sizeof(nm));
      if (idx < 0) {
        if (selten(z, 3, 6000)) melde(z, "warte", typ, edX(z, lab.Location()), edY(z, lab.Location()), "kein erforschbares Thema fuer dieses Labor");
        continue;
      }
      TechInfo ti = Research::GetTechInfo(idx);
      // Konzentrieren: lieber eine Forschung auf Maximum als zwei halbe (Spieler). Laeuft eine gleich wichtige
      // oder wichtigere unter ihrem Maximum, startet hier nichts. Ist dieses Thema wichtiger als eine laufende,
      // bekommt es deren Wissenschaftler (bis auf einen) -- Start dann im naechsten Schritt.
      // Concentrate: one topic at max rather than two halves.
      if (!standardVolk(l)) {
        const int gNeu = themaGruppe(ti.GetTechID());
        bool warten = false;
        UnitEx abgeben; int abAnz = 0, abTech = 0;
        for (int typ2 : typen) {
          PlayerBuildingEnum e2(z.spieler, (map_id)typ2);
          UnitEx lab2;
          while (e2.GetNext(lab2)) {
            if (!lab2.IsLive() || lab2.unitID == lab.unitID) continue;   // zerstoert (Absturz 00472D78)
            const int th = lab2.GetLabCurrentTopic();
            if (th <= 0) continue;
            const int i2 = th < Research::GetTechCount() ? th : techIndex(th);
            if (i2 < 0) continue;
            TechInfo t2 = Research::GetTechInfo(i2);
            const int g2 = themaGruppe(t2.GetTechID()), anz = lab2.GetLabScientistCount();
            if (g2 >= gNeu && anz < t2.GetMaxScientists()) warten = true;
            if (g2 < gNeu && anz > 1 && anz > abAnz) { abgeben = lab2; abAnz = anz; abTech = t2.GetTechID(); }
          }
        }
        if (warten) {
          if (selten(z, 23, 3000))
            melde(z, "warte", typ, edX(z, lab.Location()), edY(z, lab.Location()), "konzentriert: %s wartet, bis die laufende Forschung voll besetzt ist", nm);
          continue;
        }
        if (l.wissFrei < ti.GetMaxScientists() && abgeben.unitID) {
          abgeben.SetLabScientistCount(1);
          ExtPlayer[z.spieler].RecalculateValues();
          z.forschungSeit = t;                                      // Umverteilung wartet die Neuberechnung ab
          melde(z, "wiss", typ, edX(z, lab.Location()), edY(z, lab.Location()), "Tech %d gibt %d Wissenschaftler ab fuer %s (wichtiger)",
                abTech, abAnz - 1, nm);
          return;
        }
      }
      int wiss = ExtPlayer[z.spieler].GetNumAvailableScientists();
      if (wiss > ti.GetMaxScientists()) wiss = ti.GetMaxScientists();
      if (wiss <= 0) {
        if (selten(z, 4, 3000)) melde(z, "warte", typ, edX(z, lab.Location()), edY(z, lab.Location()), "keine freien Wissenschaftler fuer %s", nm);
        continue;
      }
      const int wert = idx;                                    // immer Tabellenindex (siehe Kalibrierung oben)
      lab.DoResearch(wert, wiss);
      if (gewaehlt >= 0) z.forschGestartet[gewaehlt]++;
      z.forschungLab = lab.unitID; z.forschungWert = wert; z.forschungSeit = t;
      if (nb < 16) belegt[nb++] = ti.GetTechID();
      melde(z, "forschung", typ, edX(z, lab.Location()), edY(z, lab.Location()), "%s%s (Tech %d, %d Wissenschaftler) [Kuchen K%d Z%d M%d W%d]",
            art, nm, ti.GetTechID(), wiss, anteilKolonie, anteilZiel, anteilMilitaer, anteilWirtschaft);
      (void)zielweg;
      return;                                                         // ein Auftrag je Schritt
    }
  }
}

// Universitaet: Wissenschaftler ausbilden, wenn ein Labor auf Wissenschaftler wartet und Arbeiter uebrig
// sind (die Uni macht aus Arbeitern Wissenschaftler; passiv bildet sie Kinder zu Arbeitern aus).
// University: train scientists when labs lack them and workers are spare.
// Wissenschaftler in laufenden Forschungen umverteilen (HFL Get/SetLabScientistCount, dann
// PlayerEx::RecalculateValues). Hinweis des Spielers: Wissenschaftler lassen sich einer laufenden Forschung
// nachtraeglich hinzufuegen oder abziehen. Vorrang: Ziel > Kolonie > Lueckenfueller/Sonstiges.
// Ob die Engine den direkt gesetzten Zaehler uebernimmt, steht in der Spur (frei vorher/nachher).
// Move scientists between running research topics: goal > colony > filler.

inline void wissenschaftlerVerteilen(Zustand& z, const Lage& l) {
  const int t = TethysGame::Tick();
  if (t < z.naechsteVerteilung || standardVolk(l)) return;
  // Nach einem Forschungsstart erst neu rechnen lassen: sonst zaehlen dessen Wissenschaftler noch als frei und
  // werden doppelt vergeben (Lauf 18: 13 benoetigt bei 10 vorhanden). Wait one recalculation after DoResearch.
  if (t - z.forschungSeit < 60) return;
  z.naechsteVerteilung = t + 200;
  struct Lab { UnitEx u; int anz, max, gruppe, tech; };
  Lab labs[8]; int n = 0;
  const int typen[] = { mapBasicLab, mapStandardLab, mapAdvancedLab };
  const int anzahlTechs = Research::GetTechCount();
  for (int typ : typen) {
    PlayerBuildingEnum e(z.spieler, (map_id)typ);
    UnitEx lab;
    while (e.GetNext(lab) && n < 8) {
      if (!lab.IsLive()) continue;
      const int th = lab.GetLabCurrentTopic();
      if (th <= 0) continue;
      const int idx = th < anzahlTechs ? th : techIndex(th);       // Index oder techID -- beides abfangen
      if (idx < 0) continue;
      TechInfo ti = Research::GetTechInfo(idx);
      if (!ti.IsValid()) continue;
      labs[n].u = lab; labs[n].anz = lab.GetLabScientistCount(); labs[n].max = ti.GetMaxScientists();
      labs[n].gruppe = themaGruppe(ti.GetTechID()); labs[n].tech = ti.GetTechID();
      n++;
    }
  }
  if (n == 0) return;
  // Wichtigste Forschung unter ihrem Maximum
  int ziel = -1;
  for (int i = 0; i < n; ++i)
    if (labs[i].anz < labs[i].max && (ziel < 0 || labs[i].gruppe > labs[ziel].gruppe)) ziel = i;
  if (ziel < 0) return;
  Lab& zl = labs[ziel];
  const int freiVorher = ExtPlayer[z.spieler].GetNumAvailableScientists();
  // Personal-Luecken gehen vor; ausserdem 1 Wissenschaftler Reserve, sonst pendelt er zwischen Medizin und
  // Forschung (Lauf 21). Staff gaps first, plus one scientist in reserve.
  const int wirklichFrei = freiVorher - beansprucht(l, M_WISS, 0) - 1;
  int dazu = 0, quelle = -1;
  if (wirklichFrei > 0) {
    dazu = wirklichFrei < zl.max - zl.anz ? wirklichFrei : zl.max - zl.anz;
  } else {
    // Unwichtigste Forschung mit mehr als einem Wissenschaftler als Quelle
    for (int i = 0; i < n; ++i)
      if (i != ziel && labs[i].gruppe < zl.gruppe && labs[i].anz > 1 && (quelle < 0 || labs[i].gruppe < labs[quelle].gruppe)) quelle = i;
    if (quelle < 0) return;
    const int abgebbar = labs[quelle].anz - 1;
    dazu = abgebbar < zl.max - zl.anz ? abgebbar : zl.max - zl.anz;
    if (dazu <= 0) return;
    labs[quelle].u.SetLabScientistCount(labs[quelle].anz - dazu);
  }
  if (dazu <= 0) return;
  zl.u.SetLabScientistCount(zl.anz + dazu);
  ExtPlayer[z.spieler].RecalculateValues();
  melde(z, "wiss", zl.u.GetType(), edX(z, zl.u.Location()), edY(z, zl.u.Location()),
        "Tech %d: %d -> %d Wissenschaftler (max %d), %s %d, frei vorher %d (Personal fehlt %d)",
        zl.tech, zl.anz, zl.anz + dazu, zl.max, quelle >= 0 ? "abgezogen von Tech" : "freie, Quelle", quelle >= 0 ? labs[quelle].tech : 0,
        freiVorher, l.wissFehlt);
}

// Forschungs-Luecke: freie Forscherplaetze der laufenden Themen (Maximum minus besetzt) und alle Plaetze, dazu der hoechste
// Laborrang. Research gap: open researcher slots of running topics, all slots, and the highest lab rank.
inline void forschungsLuecke(const Zustand& z, const Lage& l, int& luecke, int& plaetze, int& rangLabor) {
  luecke = 0; plaetze = 0; rangLabor = 0;
  const int typen[] = { mapBasicLab, mapStandardLab, mapAdvancedLab };
  const int anzahlTechs = Research::GetTechCount();
  for (int typ : typen) {
    PlayerBuildingEnum e(z.spieler, (map_id)typ);
    UnitEx lab;
    while (e.GetNext(lab)) {
      if (!lab.IsLive()) continue;
      const int th = lab.GetLabCurrentTopic();
      if (th <= 0) continue;
      const int idx = th < anzahlTechs ? th : techIndex(th);
      if (idx < 0) continue;
      TechInfo ti = Research::GetTechInfo(idx);
      if (!ti.IsValid()) continue;
      const int mx = ti.GetMaxScientists(), anz = lab.GetLabScientistCount();
      plaetze += mx; luecke += mx > anz ? mx - anz : 0;
      const int r = rang(z, l, typ); if (r > rangLabor) rangLabor = r;
    }
  }
}

inline void ausbildung(Zustand& z, const Lage& l) {
  if (standardVolk(l)) return;
  // Bedarf: Gebaeude brauchen Wissenschaftler (fehlend, im Bau oder als Vorhaben: Kindergarten, Medizin, Uni, Spaceport)
  // oder die Forschung hat keine freien mehr. Die Ausbildung ist so wichtig wie das, wofuer sie ausbildet: das wichtigste
  // Gebaeude, dem Wissenschaftler fehlen; fuer die Forschung der Laborrang mal Anteil der offenen Forscherplaetze (ein volles
  // Labor gewinnt durch mehr Wissenschaftler wenig -- der Laborrang allein (90) stach 2026-11-09 die erste Fahrzeugfabrik).
  // Arbeiter, die gleich wichtige oder wichtigere Ansprueche brauchen, bleiben frei -- ersetzt die Regel uniReserve.
  // Training ranks like what it trains for (research: lab rank x share of open slots); claimed workers stay put.
  const int wissBedarf = beansprucht(l, M_WISS, 0);
  const bool fuerGebaeude = wissBedarf > l.wissFrei;
  int luecke = 0, plaetze = 0, rangLabor = 0;
  forschungsLuecke(z, l, luecke, plaetze, rangLabor);
  const int rangForschung = plaetze > 0 ? rangLabor * luecke / plaetze : 0;
  const bool bedarf = fuerGebaeude || (l.wissFrei == 0 && luecke > 0);
  const int rangAusb = fuerGebaeude && hoechsterAnspruch(l, M_WISS) > rangForschung ? hoechsterAnspruch(l, M_WISS) : rangForschung;
  const int uebrig = l.arbFrei - beansprucht(l, M_ARB, rangAusb);
  if (!bedarf || uebrig < 3) return;
  PlayerBuildingEnum e(z.spieler, mapUniversity);
  UnitEx uni;
  while (e.GetNext(uni)) {
    if (uni.GetCurAction() != moDone || uni.IsBusy() || !uni.HasScientists() || uni.GetWorkersInTraining() > 0) continue;
    int n = uebrig - 2;
    if (n > 5) n = 5;
    if (n <= 0) return;
    uni.DoTrainScientists(n);
    melde(z, "ausbildung", mapUniversity, edX(z, uni.Location()), edY(z, uni.Location()),
          "%d Arbeiter werden zu Wissenschaftlern (Rang %d; frei: %d Arbeiter, %d Wissenschaftler; Gebaeude brauchen %d Wiss.)",
          n, rangAusb, l.arbFrei, l.wissFrei, wissBedarf);
    return;
  }
}

// ---------------------------------------------------------------------------------------------------
// Raumfahrt / space: EDWARD-Satellit auf einer SULV-Rakete starten.
//
// Wie die Engine Satellit und Rakete zusammenbringt, ist nicht dokumentiert. Die KI geht schrittweise
// vor und schreibt jeden Zustand des Spaceports (Buchten, Startrampe, Aktion) in die Spur:
//   0 EDWARD herstellen -> 1 SULV herstellen -> 2 EDWARD auf die Startrampe (TransferCargo, nach
//   200 Ticks direkt setzen) -> 3 starten (DoLaunch) -> 4 pruefen, ob der Satellit oben ist.
// Erfolg misst sie an der Satellitenzahl des Spielers (HFL PlayerEx::GetSatelliteCount).
// Launch an EDWARD satellite on a SULV, step by step, logging the spaceport state at every step.

inline void raumAuftrag(Zustand& z, UnitEx& sp, const Lage& l, int typ, int schritt) {
  UnitInfo info((map_id)typ);
  const int kosten = info.GetOreCost(z.spieler), kostenSelten = info.GetRareOreCost(z.spieler);
  const LOCATION sl = sp.Location();
  if (kosten > l.erz || kostenSelten > l.selten) {
    if (selten(z, 15, 1500))
      melde(z, "warte", typ, edX(z, sl), edY(z, sl), "Ziel Satellit: Erz %d/%d, Seltenerz %d/%d", l.erz, kosten, l.selten, kostenSelten);
    return;
  }
  sp.DoProduce((map_id)typ, mapNone);
  z.raumAuftrag = typ; z.raumSeit = TethysGame::Tick(); z.raumSchritt = schritt;
  melde(z, "raum", typ, edX(z, sl), edY(z, sl), "Ziel Satellit: Spaceport stellt her (Erz %d, Seltenerz %d)", kosten, kostenSelten);
}

inline void raumfahrt(Zustand& z, const Lage& l) {
  if (z.ziel != 1 || z.zielErreicht) return;
  const int p = z.spieler;
  const int t = TethysGame::Tick();
  const int sats = ExtPlayer[p].GetSatelliteCount(mapEDWARDSatellite);
  if (sats > 0) {
    z.zielErreicht = t;
    melde(z, "ziel", mapEDWARDSatellite, z.bx, z.by, "ZIEL ERREICHT: EDWARD-Satellit im All (%d), Startversuch %d", sats, z.raumVersuche);
    op2::log::linef("Aufbau-KI Spieler %d: ZIEL ERREICHT -- EDWARD-Satellit im All (Tick %d)", p, t);
    return;
  }
  if (!hatTech(z, TECH_RAUMFAHRT)) return;
  UnitEx sp;
  { PlayerBuildingEnum e(p, mapSpaceport); e.GetNext(sp); }
  if (!sp.unitID) return;
  if (istGeparkt(z, sp.unitID)) { leerlaufWecken(z, sp.unitID, "Ziel Satellit braucht den Spaceport"); return; }
  const LOCATION sl = sp.Location();
  const int sx = edX(z, sl), sy = edY(z, sl);
  int buchtEdward = -1;
  char buchten[96]; int nbu = 0; buchten[0] = 0;
  for (int bay = 0; bay < 6; ++bay) {
    const int k = sp.GetFactoryCargo(bay);
    if (k == mapEDWARDSatellite && buchtEdward < 0) buchtEdward = bay;
    nbu += std::snprintf(buchten + nbu, sizeof(buchten) - nbu, "%s%s", bay ? "," : "", (k > 0 && k != mapNone) ? name(k) : "-");
    if (nbu >= (int)sizeof(buchten)) nbu = sizeof(buchten) - 1;
  }
  const int rampe = sp.GetLaunchPadCargo();
  const bool arbeitet = sp.IsBusy() || sp.GetCurAction() == moDevelop;
  if (selten(z, 13, 400))
    melde(z, "raum", mapSpaceport, sx, sy, "Schritt %d: Rampe %s (%d), Buchten [%s], Aktion %d, arbeitet %d, Strom %d, Arbeiter %d, Satelliten %d",
          z.raumSchritt, (rampe > 0 && rampe != mapNone) ? name(rampe) : "-", rampe, buchten, (int)sp.GetCurAction(), arbeitet ? 1 : 0,
          sp.HasPower(), sp.HasWorkers(), sats);
  if (arbeitet) {
    if (z.raumAuftrag && t - z.raumSeit > 2 * GEDULD_KIT && selten(z, 16, 2000))
      melde(z, "fehler", z.raumAuftrag, sx, sy, "Spaceport braucht schon %d Ticks", t - z.raumSeit);
    return;
  }
  if (z.raumAuftrag) {                       // Auftrag beendet: festhalten, wo das Ergebnis liegt
    melde(z, "raum", z.raumAuftrag, sx, sy, "fertig nach %d Ticks: Rampe %s, Buchten [%s]", t - z.raumSeit,
          (rampe > 0 && rampe != mapNone) ? name(rampe) : "-", buchten);
    z.raumAuftrag = 0;
    z.raumSeit = t;                          // Wartezeiten der Folgeschritte ab hier zaehlen
  }
  if (!sp.HasPower() || !sp.HasWorkers()) {
    if (selten(z, 17, 1500)) melde(z, "warte", mapSpaceport, sx, sy, "Spaceport ohne Strom oder Arbeiter");
    return;
  }
  const bool edwardDa = buchtEdward >= 0 || rampe == mapEDWARDSatellite;
  switch (z.raumSchritt) {
    case 0:                                   // EDWARD herstellen
      if (edwardDa) { z.raumSchritt = 1; break; }
      raumAuftrag(z, sp, l, mapEDWARDSatellite, 0);
      break;
    case 1:                                   // SULV herstellen
      if (!edwardDa) { z.raumSchritt = 0; break; }
      raumAuftrag(z, sp, l, mapSULV, 2);
      break;
    case 2:                                   // EDWARD auf die Startrampe
      if (rampe == mapEDWARDSatellite || buchtEdward < 0) { z.raumSchritt = 3; z.raumSeit = t; break; }
      if (t - z.raumSeit < 200) {
        sp.DoTransferCargo(buchtEdward);
        if (selten(z, 18, 200)) melde(z, "raum", mapEDWARDSatellite, sx, sy, "EDWARD aus Bucht %d auf die Startrampe (TransferCargo)", buchtEdward);
      } else {
        sp.SetLaunchPadCargo(mapEDWARDSatellite);
        sp.SetFactoryCargo(buchtEdward, mapNone, mapNone);
        melde(z, "raum", mapEDWARDSatellite, sx, sy, "TransferCargo greift nicht -- EDWARD aus Bucht %d direkt auf die Rampe gesetzt", buchtEdward);
        z.raumSchritt = 3; z.raumSeit = t;
      }
      break;
    case 3: {                                 // starten
      const int px = sl.x * 32 + 16, py = sl.y * 32 + 16;
      z.raumVersuche++;
      if (z.raumVersuche % 2) sp.DoLaunch(px, py, 1);
      else sp.DoLaunch(0, 0, 0);
      melde(z, "raum", mapSULV, sx, sy, "Start! DoLaunch (Versuch %d, %s), Rampe %s", z.raumVersuche,
            z.raumVersuche % 2 ? "Ziel Spaceport, erzwungen" : "ohne Ziel", (rampe > 0 && rampe != mapNone) ? name(rampe) : "-");
      z.raumSchritt = 4; z.raumSeit = t;
      break;
    }
    case 4:                                   // warten, ob er oben ankommt
      if (t - z.raumSeit > 600) {
        melde(z, "fehler", mapSULV, sx, sy, "nach 600 Ticks kein Satellit im All -- Rampe %s, Buchten [%s]",
              (rampe > 0 && rampe != mapNone) ? name(rampe) : "-", buchten);
        // Rakete noch da (EDWARD auf der Rampe)? Erneut starten, sonst von vorn.
        z.raumSchritt = (rampe == mapEDWARDSatellite && z.raumVersuche < 6) ? 3 : 0;
      }
      break;
  }
}

// ---------------------------------------------------------------------------------------------------
// Momentaufnahme / snapshot

// ---------------------------------------------------------------------------------------------------
// Neuronales Netz / neural net (Vorbereitung: rechnet mit, entscheidet noch nichts)
//
// Gewichte aus logs/KI-Netz-<name>.txt (ki-test/netz_training.py, reines numpy). Die Merkmale muessen GENAU wie in
// ki-test/merkmale.py berechnet werden (MERKMAL_VERSION). Wertnetz: Lage + Stellschrauben -> erwartete Punkte bei
// Tick 80000. Angriffsnetz: Lage + Angriff -> erwartete Zerstoerung und Verluste. Ohne Datei: kein Netz, nichts passiert.
// Weights from logs/KI-Netz-<name>.txt; features must match merkmale.py exactly. Logged only, no decisions yet.
inline void wissenPfad(char* out, int n, const char* datei);
namespace netz {
constexpr int MERKMAL_VERSION = 1;
constexpr int MAX_EIN = 96, MAX_BREIT = 64, MAX_SCHICHT = 4, MAX_AUS = 4;
struct Netz {
  int geladen = 0, versucht = 0, ein = 0, aus = 0, schichten = 0;
  int zeilen[MAX_SCHICHT], spalten[MAX_SCHICHT], tanh_[MAX_SCHICHT];
  float mx[MAX_EIN], sx[MAX_EIN], my[MAX_AUS], sy[MAX_AUS];
  float W[MAX_SCHICHT][MAX_EIN * MAX_BREIT];
  float b[MAX_SCHICHT][MAX_BREIT];
};
inline Netz wert, angriff;

inline bool laden(Netz& n, const char* name) {
  if (n.versucht) return n.geladen != 0;
  n.versucht = 1;
  char datei[64], pfad[600];
  std::snprintf(datei, sizeof datei, "KI-Netz-%s.txt", name);
  wissenPfad(pfad, sizeof pfad, datei);
  FILE* f = std::fopen(pfad, "r");
  if (!f) return false;
  char kopf[16], nm[32]; int ver = 0;
  bool ok = std::fscanf(f, "%15s %31s %d %d %d %d", kopf, nm, &ver, &n.ein, &n.aus, &n.schichten) == 6 &&
            std::strcmp(kopf, "KI-NETZ") == 0 && ver == MERKMAL_VERSION && n.ein > 0 && n.ein <= MAX_EIN &&
            n.aus > 0 && n.aus <= MAX_AUS && n.schichten > 0 && n.schichten <= MAX_SCHICHT;
  for (int i = 0; ok && i < n.ein; ++i) ok = std::fscanf(f, "%f", &n.mx[i]) == 1;
  for (int i = 0; ok && i < n.ein; ++i) ok = std::fscanf(f, "%f", &n.sx[i]) == 1;
  for (int i = 0; ok && i < n.aus; ++i) ok = std::fscanf(f, "%f", &n.my[i]) == 1;
  for (int i = 0; ok && i < n.aus; ++i) ok = std::fscanf(f, "%f", &n.sy[i]) == 1;
  for (int k = 0; ok && k < n.schichten; ++k) {
    char akt[16];
    ok = std::fscanf(f, "%d %d %15s", &n.zeilen[k], &n.spalten[k], akt) == 3 && n.zeilen[k] <= MAX_EIN &&
         n.spalten[k] <= MAX_BREIT && n.zeilen[k] * n.spalten[k] <= MAX_EIN * MAX_BREIT;
    if (!ok) break;
    n.tanh_[k] = std::strcmp(akt, "tanh") == 0;
    for (int i = 0; ok && i < n.zeilen[k] * n.spalten[k]; ++i) ok = std::fscanf(f, "%f", &n.W[k][i]) == 1;
    for (int i = 0; ok && i < n.spalten[k]; ++i) ok = std::fscanf(f, "%f", &n.b[k][i]) == 1;
  }
  std::fclose(f);
  n.geladen = ok ? 1 : 0;
  return ok;
}

// Vorwaertsrechnung; out bekommt die Ausgaenge in Originalskala. Forward pass, outputs de-normalised.
inline bool rechnen(const Netz& n, const float* x, int anzahl, float* out) {
  if (!n.geladen || anzahl != n.ein) return false;
  float a[MAX_BREIT > MAX_EIN ? MAX_BREIT : MAX_EIN], neu[MAX_BREIT];
  // begrenzt: Lagen weit ausserhalb der Trainingsdaten sollen das Netz nicht in die Saettigung treiben
  for (int i = 0; i < n.ein; ++i) { const float v = (x[i] - n.mx[i]) / n.sx[i]; a[i] = v > 6 ? 6 : v < -6 ? -6 : v; }
  int breite = n.ein;
  for (int k = 0; k < n.schichten; ++k) {
    for (int j = 0; j < n.spalten[k]; ++j) {
      float s = n.b[k][j];
      for (int i = 0; i < breite; ++i) s += a[i] * n.W[k][i * n.spalten[k] + j];
      neu[j] = n.tanh_[k] ? std::tanh(s) : s;
    }
    breite = n.spalten[k];
    for (int j = 0; j < breite; ++j) a[j] = neu[j];
  }
  for (int j = 0; j < n.aus; ++j) out[j] = a[j] * n.sy[j] + n.my[j];
  return true;
}
}  // namespace netz

// Merkmale wie ki-test/merkmale.py (Reihenfolge und Skalen gleich), dann die Stellschrauben auf 0..1 wie
// ki-test/optimierer.py (PARAMETER). Liefert die Anzahl. Features as in merkmale.py, then normalised parameters.
inline int netzMerkmale(const Zustand& z, const Lage& l, float* v) {
  const int pl = z.spieler;
  int n = 0;
  const int kinder = Player[pl].Kids(), arbeiter = Player[pl].Workers(), wiss = Player[pl].Scientists();
  v[n++] = TethysGame::Tick() / 100000.f; v[n++] = (kinder + arbeiter + wiss) / 100.f;
  v[n++] = kinder / 100.f; v[n++] = arbeiter / 100.f; v[n++] = wiss / 100.f;
  v[n++] = l.erz / 10000.f; v[n++] = Player[pl].RareOre() / 10000.f; v[n++] = l.nahrung / 10000.f; v[n++] = l.nahrungNetto / 100.f;
  v[n++] = moralZahl(z) / 100.f;
  v[n++] = (ExtPlayer[pl].GetAmountPowerGenerated() - ExtPlayer[pl].GetAmountPowerConsumed()) / 1000.f;
  static const int GEB[] = { mapCommonOreSmelter, mapCommonOreMine, mapRareOreSmelter, mapRareOreMine, mapVehicleFactory,
                             mapStandardLab, mapAdvancedLab, mapResidence, mapGuardPost, mapAgridome, mapTokamak,
                             mapNursery, mapUniversity, mapMedicalCenter };
  for (int t : GEB) v[n++] = l.geb[t] / 10.f;
  static const int FZ[] = { mapLynx, mapPanther, mapTiger, mapCargoTruck, mapConVec, mapRoboMiner };
  for (int t : FZ) v[n++] = l.fz[t] / 20.f;
  v[n++] = z.feindArmee / 100.f; v[n++] = z.feindNah / 100.f; v[n++] = z.feindGeb / 20.f; v[n++] = z.eigeneArmee / 100.f;
  v[n++] = z.sollArmee / 100.f; v[n++] = z.bedrohung / 100.f; v[n++] = z.eigeneMobil / 100.f; v[n++] = (float)z.kampfModus;
  v[n++] = z.ziel == 2 ? 1.f : 0.f; v[n++] = z.ziel == 3 ? 1.f : 0.f;
  return n;
}
inline int netzParameter(const Zustand& z, float* v) {
  const Parameter& q = PAR(z);
  struct G { double w, lo, hi; };
  const G g[] = { { q.reaktion, 2000, 15000 }, { q.horizont, 5000, 50000 }, { q.ueberlegenheit, 0.8, 3.0 },
                  { q.fabrikGewinn, 0.4, 1.0 }, { q.reserveStart, 0.8, 3.0 }, { q.reserveLernrate, 0, 1 },
                  { q.reserveRueckgang, 0.8, 1.0 }, { q.reserveMax, 1, 4 }, { q.turmAnteil, 0, 0.8 },
                  { q.zermuerbAbstand, 2000, 30000 }, { q.schmelzeNutzen, 0.3, 3.0 }, { q.wohnraumGrenze, 60, 98 },
                  { q.freischaltWert, 0, 1 }, { 10, 2, 60 }, { 40, 10, 90 },   // entfallene Reparatur-Schwellen: alte Standardwerte, Netz-Format bleibt
                  { q.nahRadius, 12, 40 }, { q.heimHalten, 0, 2 }, { q.steigerung, 1.0, 1.5 }, { q.vorsorge, 0, 100 },
                  { q.schmelzenJeMine, 0.5, 2.0 }, { q.empSchwelle, 0.1, 5.0 } };
  int n = 0;
  for (const G& e : g) v[n++] = (float)((e.w - e.lo) / (e.hi - e.lo));
  return n;
}
// Erwartete Punkte bei Tick 80000 laut Wertnetz (oder -1e9 ohne Netz). Value-net prediction.
inline float netzWert(const Zustand& z, const Lage& l) {
  if (!netz::laden(netz::wert, "wert")) return -1e9f;
  float v[netz::MAX_EIN]; int n = netzMerkmale(z, l, v); n += netzParameter(z, v + n);
  float out[netz::MAX_AUS];
  return netz::rechnen(netz::wert, v, n, out) ? out[0] : -1e9f;
}
// Erwartete Zerstoerung/Verluste eines Angriffs laut Angriffsnetz. Attack-net prediction.
inline bool netzAngriff(const Zustand& z, const Lage& l, int art, int fahrzeuge, int wertTruppe, float& zer, float& ver) {
  if (!netz::laden(netz::angriff, "angriff")) return false;
  float v[netz::MAX_EIN]; int n = netzMerkmale(z, l, v);
  for (int k = 0; k < 3; ++k) v[n++] = art == k ? 1.f : 0.f;
  v[n++] = fahrzeuge / 50.f; v[n++] = wertTruppe / 300.f;
  n += netzParameter(z, v + n);
  float out[netz::MAX_AUS];
  if (!netz::rechnen(netz::angriff, v, n, out)) return false;
  zer = out[0]; ver = out[1];
  return true;
}

// Lastwagen-Diagnose (2026-11-09: Edens Erz stand ab Tick 23000 auf 81, obwohl Schmelze, Mine und 3 Lastwagen da waren): alle
// 500 Ticks die Erzrouten (Mine, Schmelze, Lastwagen in der Gruppe) und jeder Lastwagen mit Ort, Aktion, Ladung, bisherigen Fahrten
// und Route als Zeile "art":"lkw". Truck diagnostics: routes and every truck (place, action, cargo, loads so far, route).
inline void lkwDiagnose(Zustand& z) {
  Puffer p;
  p.f("{\"t\":%d,\"art\":\"lkw\",\"p\":%d,\"routen\":[", TethysGame::Tick(), z.spieler);
  bool erst = true;
  for (int i = 0; i < MAX_MINEN; ++i) {
    if (!z.minenId[i]) continue;
    UnitEx m, s; m.unitID = z.minenId[i]; s.unitID = z.schmelzeId[i];
    const LOCATION ml = m.IsLive() ? m.Location() : LOCATION(0, 0), sl = s.unitID && s.IsLive() ? s.Location() : LOCATION(0, 0);
    p.f("%s[%d,%d,%d,%d,%d,%d,%d,%d]", erst ? "" : ",", i, m.IsLive() ? 1 : 0, edX(z, ml), edY(z, ml), s.unitID && s.IsLive() ? 1 : 0,
        edX(z, sl), edY(z, sl), z.minenGruppe[i].TotalUnitCount());
    erst = false;
  }
  p.roh("],\"lkw\":[");
  erst = true;
  PlayerVehicleEnum e(z.spieler); UnitEx u;
  while (e.GetNext(u)) {
    if (u.GetType() != mapCargoTruck || !u.IsLive()) continue;
    int route = -1;
    for (int i = 0; i < MAX_MINEN && route < 0; ++i) {
      if (!z.minenId[i]) continue;
      GroupEnumerator ge(z.minenGruppe[i]); UnitEx v;
      while (ge.GetNext(v)) if (v.unitID == u.unitID) { route = i; break; }
    }
    const LOCATION lo = u.Location();
    // [unitID, x, y, Aktion, beschaeftigt, Ladungsart, Menge, Fahrten bisher, Route, Schaden]
    p.f("%s[%d,%d,%d,%d,%d,%d,%d,%d,%d,%d]", erst ? "" : ",", u.unitID, edX(z, lo), edY(z, lo), (int)u.GetCurAction(), u.IsBusy() ? 1 : 0,
        (int)u.GetCargoType(), u.GetCargoAmount(), u.GetNumTruckLoadsSoFar(), route, u.GetDamage());
    erst = false;
  }
  p.roh("]}");
  p.schreiben();
  // Stau erkennen: beladener Lastwagen einer Route steht bei 3 Messungen (3000 Ticks) am selben Fleck nahe seiner Schmelze. Dann
  // einmal melden, was auf der Anlegestelle steht und wie die Schmelze steht, und die Ansicht dorthin (fuer ein Bildschirmfoto).
  // Stall: a loaded truck stands still near its smelter for 3 checks; report the dock and centre the local view on it.
  static int altId[8][16], altX[8][16], altY[8][16], zaehler[8][16], gemeldet[8];
  const int sp = z.spieler & 7;
  int neuId[16], neuX[16], neuY[16], neuZ[16], nn = 0, stauSchmelze = -1;
  PlayerVehicleEnum e2(z.spieler); UnitEx v;
  while (e2.GetNext(v) && nn < 16) {
    if (v.GetType() != mapCargoTruck || !v.IsLive() || v.GetCargoAmount() <= 0) continue;
    int route = -1;
    for (int i = 0; i < MAX_MINEN && route < 0; ++i) {
      if (!z.minenId[i] || !z.schmelzeId[i]) continue;
      GroupEnumerator ge(z.minenGruppe[i]); UnitEx w;
      while (ge.GetNext(w)) if (w.unitID == v.unitID) { route = i; break; }
    }
    if (route < 0) continue;
    UnitEx sm; sm.unitID = z.schmelzeId[route];
    if (!sm.IsLive()) continue;
    const LOCATION vl = v.Location(), sl = sm.Location();
    if (abstand2(vl.x, vl.y, sl.x, sl.y) > 6 * 6) continue;
    int zz = 1;
    for (int k = 0; k < 16; ++k) if (altId[sp][k] == v.unitID && altX[sp][k] == vl.x && altY[sp][k] == vl.y) zz = zaehler[sp][k] + 1;
    neuId[nn] = v.unitID; neuX[nn] = vl.x; neuY[nn] = vl.y; neuZ[nn] = zz; nn++;
    if (zz >= 3) stauSchmelze = z.schmelzeId[route];
  }
  for (int k = 0; k < 16; ++k) { altId[sp][k] = k < nn ? neuId[k] : 0; altX[sp][k] = k < nn ? neuX[k] : 0; altY[sp][k] = k < nn ? neuY[k] : 0; zaehler[sp][k] = k < nn ? neuZ[k] : 0; }
  if (stauSchmelze < 0) { gemeldet[sp] = 0; return; }
  if (gemeldet[sp]) return;
  gemeldet[sp] = 1;
  UnitEx sm; sm.unitID = stauSchmelze;
  const LOCATION sl = sm.Location(), dl = sm.GetDockLocation();
  Puffer q;
  q.f("{\"t\":%d,\"art\":\"lkwstau\",\"p\":%d,\"schmelze\":[%d,%d],\"dock\":[%d,%d],\"zustand\":[%d,%d,%d,%d,%d],\"aufDock\":[",
      TethysGame::Tick(), z.spieler, edX(z, sl), edY(z, sl), edX(z, dl), edY(z, dl), sm.IsBusy() ? 1 : 0, (int)sm.GetCurAction(),
      sm.HasPower() ? 1 : 0, sm.HasWorkers() ? 1 : 0, sm.GetDamage());
  // alles, was auf der Anlegestelle und den Kacheln daneben steht: [x, y, Typ, Besitzer, Aktion]
  bool erst2 = true;
  for (int dy = -1; dy <= 1; ++dy) for (int dx = -1; dx <= 1; ++dx) {
    LocationEnumerator le(LOCATION(dl.x + dx, dl.y + dy)); UnitEx o;
    while (le.GetNext(o)) {
      q.f("%s[%d,%d,%d,%d,%d]", erst2 ? "" : ",", edX(z, o.Location()), edY(z, o.Location()), o.GetType(), o.OwnerID(), (int)o.GetCurAction());
      erst2 = false;
    }
  }
  q.f("],\"zelleDock\":%d}", GameMap::GetCellType(dl));
  q.schreiben();
  Player[TethysGame::LocalPlayer()].CenterViewOn(dl.x, dl.y);
}

inline void spur(const Zustand& z, const Lage& l) {
  Puffer p;
  const int pl = z.spieler;
  p.f("{\"t\":%d,\"art\":\"stand\",\"p\":%d,\"erz\":%d,\"erzMax\":%d,\"selten\":%d,\"nahrung\":%d,\"nahrungNetto\":%d,"
      "\"strom\":[%d,%d],\"kinder\":%d,\"arbeiter\":%d,\"wiss\":%d,\"wohnraum\":%d,\"moral\":%d",
      TethysGame::Tick(), pl, l.erz, l.erzMax, Player[pl].RareOre(), l.nahrung, l.nahrungNetto,
      ExtPlayer[pl].GetAmountPowerGenerated(), ExtPlayer[pl].GetAmountPowerConsumed(),
      Player[pl].Kids(), Player[pl].Workers(), Player[pl].Scientists(), l.wohnraum, (int)Player[pl].MoraleLevel());
  { const float nw = netzWert(z, l); if (nw > -1e8f) p.f(",\"netzWert\":%.0f", nw); }   // Wertnetz (falls geladen)
  p.f(",\"ziel\":%d,\"zielErreicht\":%d,\"raumSchritt\":%d,\"satelliten\":%d,\"seltenMax\":%d",
      z.ziel, z.zielErreicht, z.raumSchritt, ExtPlayer[pl].GetSatelliteCount(mapEDWARDSatellite), l.seltenMax);
  // Bevoelkerung und Arbeit: frei/benoetigt, stillgelegte/aktive Gebaeude, Kapazitaeten (Medizin, Freizeit, Forum)
  p.f(",\"arbFrei\":%d,\"arbBedarf\":%d,\"wissFrei\":%d,\"wissBedarf\":%d,\"wissForschung\":%d,"
      "\"stillgelegt\":%d,\"aktiv\":%d,\"ohneStrom\":%d,\"kap\":[%d,%d,%d]",
      ExtPlayer[pl].GetNumAvailableWorkers(), ExtPlayer[pl].GetNumWorkersRequired(),
      ExtPlayer[pl].GetNumAvailableScientists(), ExtPlayer[pl].GetNumScientistsRequired(),
      ExtPlayer[pl].GetNumScientistsAssignedToResearch(),
      ExtPlayer[pl].GetNumIdleBuildings(), ExtPlayer[pl].GetNumActiveBuildings(), ExtPlayer[pl].GetNumUnpoweredStructures(),
      ExtPlayer[pl].GetTotalMedCenterCapacity(), ExtPlayer[pl].GetTotalRecreationFacilityCapacity(),
      ExtPlayer[pl].GetTotalForumCapacity());
  p.f(",\"dirt\":[%d,%d,%d]", l.dirtAbdeckung, l.gorfLaeuft, l.angeschlossen);   // DIRT-Abdeckung %, GORF in Betrieb, angeschlossene Gebaeude
  // Unbekannte Felder der Spielerstruktur (HFL OP2Player: moraleLevel an Index 13, danach unk1[5] und nach
  // foodSupply unk2[17]) -- hier steckt vermutlich der genaue Moralwert. Messung, um ihn zu finden.
  // Unknown player-struct fields (likely the exact morale value) -- logged to find it.
  hinweis(z, "ki::spur/1");
  if (isInited && playerArray) {
    const int* roh = reinterpret_cast<const int*>(reinterpret_cast<const char*>(playerArray) + pl * 3108);
    p.roh(",\"roh\":[");
    for (int i = 0; i <= 36; ++i) p.f("%s%d", i ? "," : "", roh[i]);   // [0] = Spielernummer, [13] = Moralstufe (Pruefung)
    p.roh("]");
  }
  // Tafel: Bau-Kuchen, Vormerkung und Stand der Teilziele (ok / bau / frei = baubar / Tech = Technik fehlt)
  // Board: building pie, reservation and sub-goal status.
  hinweis(z, "ki::spur/2");
  {
    int anteil[B_ANZAHL];
    bauKuchen(z, l, anteil);
    p.f(",\"kuchen\":{\"Kolonie\":%d,\"Ziel\":%d,\"Wirtschaft\":%d,\"Verteidigung\":%d},\"vorgemerkt\":[%d,%d,\"%s\"]",
        anteil[0], anteil[1], anteil[2], anteil[3], l.vorErz, l.vorSelten, l.vorFuer);
    // Kampf: [Feind Armee, davon nahe, Feind-Gebaeude, eigener Wert, Soll, Bedrohung, Angriffe, Wache]
    // dazu: groesste Feindarmee, mobil eigen/soll, Modus (0 Wache 1 Abfangen 2 Angriff), Aussenposten mit Gruppe
    int aussen = 0;
    for (int i = 0; i < 6; ++i) if (z.aussenAn[i]) aussen++;
    {
      const Bilanz bl = bilanz(z, l);
      // Bilanz: [Bedarf Erz, Bedarf Seltenerz, Engpass Erz, Engpass Seltenerz, Zufluss Erz, Zufluss Seltenerz je 1000 Ticks]
      p.f(",\"bilanz\":[%d,%d,%d,%d,%d,%d]", bl.bedarfErz, bl.bedarfSelten, bl.engpassErz, bl.engpassSelten,
          (int)z.zuflussErz, (int)z.zuflussSelten);
      p.f(",\"seltenVorteil\":%d,\"seltenSpart\":%d", (int)(100 * seltenVorteil(z)), l.seltenSpart ? 1 : 0);   // % mehr Kampfkraft je Erz
    }
    {
      const Dringlichkeit d = dringlichkeit(z, l);
      p.f(",\"potenzial\":%d", z.feindPotenzial);
      p.f(",\"lernen\":{\"lauf\":%d,\"reserve\":%.2f,\"unterwegs\":%d,\"punkte\":%d,\"zerstoert\":%d,\"verloren\":%d}", z.laufNr, z.reserve,
          z.feindUnterwegs, laufPunkte(z), z.zerstoertWert, z.verlorenWert);
      p.f(",\"dringlich\":{\"Kolonie\":%d,\"Ziel\":%d,\"Wirtschaft\":%d,\"Verteidigung\":%d,\"Militaerforschung\":%d,\"Wellenabstand\":%d,\"warumKolonie\":\"%s\",\"warumWirtschaft\":\"%s\"}",
          d.kolonie, d.ziel, d.wirtschaft, d.verteidigung, d.kampfForschung, z.wellenAbstand, d.kolonieGrund, d.wirtschaftGrund);
    }
    p.f(",\"kampf\":[%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d,%d]", z.feindArmee, z.feindNah, z.feindGeb, z.eigeneArmee, z.sollArmee,
        z.bedrohung, z.angriffe, z.wacheAnzahl, z.staerksterAngriff, z.eigeneMobil, mobilSoll(z), z.kampfModus, aussen);
    static const int TEILE[] = { mapNursery, mapUniversity, mapMedicalCenter, mapStandardLab, mapAdvancedLab,
                                 mapVehicleFactory, mapRareOreSmelter, mapRareOreMine, mapSpaceport };
    p.roh(",\"tafel\":{");
    bool erst = true;
    for (int t : TEILE) {
      const char* st = l.geb[t] > l.imBau[t] ? "ok" : (l.imBau[t] || geplant(z, t) || l.kitsImSpiel[t]) ? "bau"
                     : kitMoeglich(z, t) ? "frei" : "Tech";
      p.f("%s\"%s\":\"%s\"", erst ? "" : ",", name(t), st);
      erst = false;
    }
    p.f(",\"Satellit\":\"%s\"}", z.zielErreicht ? "ok" : z.ziel == 1 ? "offen" : "-");
  }
  // Kontrolle: fertige Gebaeude, die nicht laufen (ohne Arbeiter, Wissenschaftler oder Strom), ausser den
  // bewusst stillgelegten. Check: finished buildings that are not running, except deliberately idled ones.
  hinweis(z, "ki::spur/3");
  {
    p.roh(",\"inaktiv\":[");
    bool erst = true;
    for (int t : GEBAEUDE) {
      UnitInfo info((map_id)t);
      const int arbN = info.GetWorkersRequired(pl), wissN = info.GetScientistsRequired(pl);
      PlayerBuildingEnum e(pl, (map_id)t);
      UnitEx u;
      while (e.GetNext(u)) {
      if (!u.IsLive()) continue;
        if (u.GetCurAction() != moDone || u.IsBusy()) continue;
        const char* grund = (arbN > 0 && !u.HasWorkers()) ? "Arbeiter" : (wissN > 0 && !u.HasScientists()) ? "Wissenschaftler"
                          : (info.GetPowerRequired(pl) > 0 && !u.HasPower()) ? "Strom" : nullptr;
        if (!grund) continue;
        p.f("%s[\"%s\",\"%s\"]", erst ? "" : ",", name(t), grund);
        erst = false;
      }
    }
    p.roh("]");
  }
  p.roh(",\"geb\":[");
  hinweis(z, "ki::spur/4");
  {
    bool erst = true; int k = 0;
    for (int t : GEBAEUDE) {
      PlayerBuildingEnum e(pl, (map_id)t);
      UnitEx u;
      while (e.GetNext(u) && k++ < 120) {
        const LOCATION lo = u.Location();
        // [Name, x, y, beschaeftigt, Aktion, Strom, Arbeiter, unitID] -- fertig, angeschlossen, besetzt?
        p.f("%s[\"%s\",%d,%d,%d,%d,%d,%d,%d]", erst ? "" : ",", name(t), edX(z, lo), edY(z, lo), u.IsBusy() ? 1 : 0,
            (int)u.GetCurAction(), u.HasPower() ? 1 : 0, u.HasWorkers() ? 1 : 0, u.unitID);
        erst = false;
      }
    }
  }
  p.roh("],\"fz\":[");
  hinweis(z, "ki::spur/5");
  {
    PlayerVehicleEnum e(pl);
    UnitEx u; bool erst = true; int k = 0;
    while (e.GetNext(u) && k++ < 120) {
      // Zerstoerte Fahrzeuge koennen noch in der Liste stehen (Duell 13: 3x Absturz 00475EF5 hier, immer im Gefecht)
      if (!u.IsLive()) continue;
      const LOCATION lo = u.Location();
      p.f("%s[\"%s\",%d,%d,\"%s\",%d,%d]", erst ? "" : ",", name(u.GetType()), edX(z, lo), edY(z, lo),
          name(u.GetCargo()), (int)u.GetCurAction(), u.unitID);
      erst = false;
    }
  }
  p.roh("],\"plaetze\":[");
  hinweis(z, "ki::spur/6");
  for (int i = 0; i < z.anzahlPlaetze; ++i) {
    const Platz& q = z.plaetze[i];
    const Groesse g = grundflaeche(q.typ);
    p.f("%s[\"%s\",%d,%d,%d,%d,%d]", i ? "," : "", name(q.typ), q.x0, q.y0, g.w, g.h, (int)q.zustand);
  }
  p.roh("],\"roehren\":[");
  hinweis(z, "ki::spur/7");
  for (int i = 0; i < z.anzahlRoehren; ++i) {
    const Roehre& r = z.roehren[i];
    p.f("%s[%d,%d,%d,%d,%d]", i ? "," : "", r.x1, r.y1, r.x2, r.y2, (int)r.zustand);
  }
  p.roh("]}");
  p.schreiben();
}

// ---------------------------------------------------------------------------------------------------
// Oeffentliche Schnittstelle / public interface

// Startausruestung "von null": ConVecs mit Bausaetzen, Bergbau- und Hilfsfahrzeuge um (x, y).
// `stufe`: 0 knapp (wie bisher), 1 normal (+1 Wohnhaus), 2 komfortabel (+2 Wohnhaeuser, +1 Agridome).
// Ohne Wohnhaus bricht die Moral am Start ein, bis Wohnraum steht (Lauf 18: Great -> Rotten in 2000 Ticks).
inline void startausruestung(const Zustand& z, int x, int y, int stufe = 0) {
  struct Teil { int typ, fracht, dx, dy; };
  static const Teil teile[] = {
    { mapConVec, mapCommandCenter, 0, 0 },      { mapConVec, mapStructureFactory, 1, 0 },
    { mapConVec, mapTokamak, 2, 0 },            { mapConVec, mapCommonOreSmelter, -1, 0 },
    { mapConVec, mapAgridome, 0, 1 },           { mapRoboMiner, mapNone, 1, 1 },
    { mapRoboSurveyor, mapNone, 2, 1 },         { mapCargoTruck, mapNone, -1, 1 },
    { mapCargoTruck, mapNone, -2, 1 },          { mapEarthworker, mapNone, -2, 0 },
  };
  static const Teil extra[] = {
    { mapConVec, mapResidence, 3, 0 }, { mapConVec, mapResidence, 3, 1 }, { mapConVec, mapAgridome, -3, 0 },
  };
  Unit u;
  int convecs = 0;
  for (const Teil& t : teile) {
    // Stufe 3 ("Basis steht"): die Gebaeude stehen schon (basisAufbauen) -- nur zwei leere ConVecs
    if (stufe >= 3 && t.typ == mapConVec && ++convecs > 2) continue;
    const int fracht = stufe >= 3 && t.typ == mapConVec ? (int)mapNone : t.fracht;
    // Abseits der Basis parken: stehende Fahrzeuge blockieren Bauplaetze.
    TethysGame::CreateUnit(u, (map_id)t.typ, engine(z, x + 2 * t.dx, y + 9 + t.dy), z.spieler, (map_id)fracht, 0);
  }
  const int mehr = stufe <= 0 ? 0 : stufe == 1 ? 1 : stufe == 2 ? 3 : 0;
  for (int i = 0; i < mehr; ++i)
    TethysGame::CreateUnit(u, (map_id)extra[i].typ, engine(z, x + 2 * extra[i].dx, y + 9 + extra[i].dy), z.spieler, (map_id)extra[i].fracht, 0);
}

// Stufe 3 ("Basis steht", fuer kurze Tests -- Spieler): die Gebaeude der Startausruestung plus Wohnhaus, Standard
// Lab und ein zweites Agridome stehen schon, an den Plaetzen, die die KI selbst gewaehlt haette (Roehrenstutzen
// verbinden sie). Stage 3: start buildings plus lab and second agridome are already built.
inline void basisAufbauen(Zustand& z) {
  static const int GEB[] = { mapCommandCenter, mapStructureFactory, mapTokamak, mapCommonOreSmelter, mapAgridome,
                             mapResidence, mapStandardLab, mapAgridome };
  for (int typ : GEB) {
    const int i = platzWaehlen(z, typ);
    if (i < 0) { melde(z, "fehler", typ, z.bx, z.by, "Basis steht: kein Platz fuer %s", name(typ)); continue; }
    Platz& p = z.plaetze[i];
    const Groesse g = grundflaeche(typ);
    Unit u;
    TethysGame::CreateUnit(u, (map_id)typ, engine(z, p.x0 + g.w / 2, p.y0 + g.h / 2), z.spieler, mapNone, 0);
    p.zustand = P_STEHT; p.seit = TethysGame::Tick();
    melde(z, "info", typ, p.x0, p.y0, "Basis steht: %s gesetzt (Platz %dx%d)", name(typ), g.w, g.h);
  }
}

// Ohne gewoehnliche Erz-Bake in Reichweite der Basis zwei anlegen (Teil der Startausruestung --
// Baken kommen sonst aus dem Missionsskript, die Karte selbst hat keine).
// Without a common ore beacon near the base, create two (part of the start equipment).
inline void bakenSichern(Zustand& z) {
  BakenFund f = bakeSuchen(z);
  UnitEx b = f.beste;
  if (b.unitID && abstand2(edX(z, b.Location()), edY(z, b.Location()), z.bx, z.by) < 40 * 40) return;
  int gesetzt = 0;
  for (int r = 12; r <= 30 && gesetzt < 2; r += 3)
    for (int k = 0; k < 16 && gesetzt < 2; ++k) {
      static const int RX[16] = { 1, 1, 0, -1, -1, -1, 0, 1, 1, 0, -1, 0, 1, -1, 1, -1 };
      static const int RY[16] = { 0, 1, 1, 1, 0, -1, -1, -1, 0, 1, 0, -1, 1, 1, -1, -1 };
      const int x = z.bx + RX[k] * r, y = z.by + RY[k] * r + 2;
      if (!aufKarte(z, x - 2, y - 2, 3) || !aufKarte(z, x + 2, y + 2, 3)) continue;
      bool frei = true;
      for (int yy = y - 1; yy <= y + 1 && frei; ++yy)
        for (int xx = x - 2; xx <= x + 2 && frei; ++xx) if (!zelleBebaubar(zelle(z, xx, yy))) frei = false;
      if (!frei) continue;
      const LOCATION l = engine(z, x, y);
      const int rueck = TethysGame::CreateBeacon(mapMiningBeacon, l.x, l.y, OreTypeCommon, gesetzt == 0 ? Bar2 : Bar3, VariantRandom);
      if (z.anzahlBaken < 4) { z.bakenX[z.anzahlBaken] = x; z.bakenY[z.anzahlBaken] = y; z.anzahlBaken++; }
      UnitEx neu; neu.unitID = rueck;
      melde(z, "info", mapMiningBeacon, x, y, "keine Erz-Bake nahe der Basis -- Bake angelegt (Rueckgabe %d, Typ dort %d, Bake ist Typ %d)",
            rueck, rueck > 0 && rueck < 4096 ? (int)neu.GetType() : -1, (int)mapMiningBeacon);
      gesetzt++;
      k += 7;                                                   // zweite Bake in eine andere Richtung
    }
}

// ---------------------------------------------------------------------------------------------------
// Langzeit-Wissen / long-term knowledge (KI-Wissen.txt im Log-Ordner, ueber alle Laeufe und Missionen)
//
// Gelernt wird, was sich aus einem Lauf messen laesst: die Reserve auf die erwartete Welle (aus den Verlusten) und
// der Erzertrag je Route. Dazu eine Bewertung jedes Laufs (Ziel, Wellen, Verluste, Einwohner, Moral), die letzten 20
// stehen in der Datei. Die Datei wird beim Start gelesen und alle 5000 Ticks sowie nach jeder Welle neu geschrieben
// (das Spiel wird von aussen beendet -- es gibt kein sicheres "Ende").
// Learned across runs: wave reserve, ore per route; plus a score table of the last 20 runs.
inline void wissenPfad(char* out, int n, const char* datei) {
  const char* k = op2::log::kiPath();
  int ende = -1;
  for (int i = 0; k[i]; ++i) if (k[i] == '\\' || k[i] == '/') ende = i;
  if (ende < 0) { std::snprintf(out, n, "%s", datei); return; }
  std::snprintf(out, n, "%.*s\\%s", ende, k, datei);
}

// Parameter aus logs/KI-Parameter-P<n>.txt (name=wert je Zeile, # = Kommentar). Unbekannte Namen werden gemeldet.
// Parameters from logs/KI-Parameter-P<n>.txt.
inline void parameterLaden(Zustand& z) {
  Parameter& q = PAR(z);
  q = Parameter();
  char pfad[600], datei[48];
  std::snprintf(datei, sizeof(datei), "KI-Parameter-P%d.txt", z.spieler);
  wissenPfad(pfad, sizeof(pfad), datei);
  FILE* f = std::fopen(pfad, "r");
  const char* text = parameterText[z.spieler & 7];
  if (!f && !text) return;
  struct Feld { const char* name; double* wert; };
  const Feld felder[] = {
    { "reaktion", &q.reaktion }, { "horizont", &q.horizont }, { "ueberlegenheit", &q.ueberlegenheit },
    { "fabrikGewinn", &q.fabrikGewinn }, { "reserveStart", &q.reserveStart }, { "reserveLernrate", &q.reserveLernrate },
    { "reserveRueckgang", &q.reserveRueckgang }, { "reserveMax", &q.reserveMax }, { "turmAnteil", &q.turmAnteil },
    { "zermuerbAbstand", &q.zermuerbAbstand }, { "schmelzeNutzen", &q.schmelzeNutzen }, { "wohnraumGrenze", &q.wohnraumGrenze },
    { "freischaltWert", &q.freischaltWert },
    { "nahRadius", &q.nahRadius }, { "heimHalten", &q.heimHalten }, { "steigerung", &q.steigerung }, { "vorsorge", &q.vorsorge }, { "seltenGewinn", &q.seltenGewinn }, { "schmelzenJeMine", &q.schmelzenJeMine }, { "empSchwelle", &q.empSchwelle }, { "risikoAnteil", &q.risikoAnteil }, { "angriffMarge", &q.angriffMarge }, { "rueckzugAb", &q.rueckzugAb }, { "lernen", &q.lernen } };
  char zeile[200]; int n = 0;
  // naechste Zeile aus der Datei oder aus dem eingebauten Text
  auto naechste = [&]() -> bool {
    if (f) return std::fgets(zeile, sizeof(zeile), f) != nullptr;
    if (!*text) return false;
    int k = 0;
    while (*text && *text != '\n' && k < (int)sizeof(zeile) - 2) zeile[k++] = *text++;
    if (*text == '\n') ++text;
    zeile[k++] = '\n'; zeile[k] = 0;
    return true;
  };
  while (naechste()) {
    if (zeile[0] == '#' || zeile[0] == '\n') continue;
    char name[64]; double w;
    if (std::sscanf(zeile, "%63[^=]=%lf", name, &w) != 2) continue;
    bool bekannt = false;
    for (const Feld& fe : felder) if (std::strcmp(fe.name, name) == 0) { *fe.wert = w; bekannt = true; n++; }
    if (!bekannt) melde(z, "info", mapNone, z.bx, z.by, "Parameter unbekannt: %s", name);
  }
  if (f) std::fclose(f);
  else std::snprintf(datei, sizeof(datei), "eingebaut (Mission)");
  // Grenzen, damit kein Satz die KI lahmlegt / sane bounds
  auto grenze = [](double& v, double lo, double hi) { if (v < lo) v = lo; if (v > hi) v = hi; };
  grenze(q.reaktion, 1000, 30000); grenze(q.horizont, 2000, 80000); grenze(q.ueberlegenheit, 0.5, 4);
  grenze(q.fabrikGewinn, 0.2, 1); grenze(q.reserveStart, 0.5, 4); grenze(q.reserveLernrate, 0, 2);
  grenze(q.reserveRueckgang, 0.5, 1); grenze(q.reserveMax, 1, 6); grenze(q.turmAnteil, 0, 1);
  grenze(q.zermuerbAbstand, 1000, 60000); grenze(q.schmelzeNutzen, 0.1, 10); grenze(q.wohnraumGrenze, 50, 99);
  grenze(q.freischaltWert, 0, 2);
  grenze(q.nahRadius, 8, 60); grenze(q.heimHalten, 0, 4); grenze(q.steigerung, 1, 2); grenze(q.vorsorge, 0, 100); grenze(q.seltenGewinn, 0, 5); grenze(q.schmelzenJeMine, 0.5, 2); grenze(q.empSchwelle, 0.1, 5);
  melde(z, "lernen", mapNone, z.bx, z.by, "%d Parameter aus %s geladen (reaktion %.0f, ueberlegenheit %.2f, reserveStart %.2f, lernen %.0f)",
        n, datei, q.reaktion, q.ueberlegenheit, q.reserveStart, q.lernen);
}

// Ergebnis eines Optimierer-Laufs: logs/KI-Ergebnis-P<n>.txt (eine Zeile, wird ueberschrieben).
// Result line for the optimizer.
inline int laufPunkte(const Zustand& z);
inline void ergebnisSchreiben(Zustand& z) {
  char pfad[600], datei[48];
  std::snprintf(datei, sizeof(datei), "KI-Ergebnis-P%d.txt", z.spieler);
  wissenPfad(pfad, sizeof(pfad), datei);
  FILE* f = std::fopen(pfad, "w");
  if (!f) return;
  std::fprintf(f, "tick=%d punkte=%d zielArt=%d ziel=%d zerstoert=%d verloren=%d maxBew=%d kolonie=%d sieg=%d niederlage=%d verlustGeb=%d verlustEinh=%d schmelze2=%d\n",
               TethysGame::Tick(), laufPunkte(z), z.ziel, z.zielErreicht, z.zerstoertWert, z.verlorenWert, z.maxBew,
               l_kolonieLebt(z) ? 1 : 0, z.siegTick, z.niederlageTick, z.verlustGebaeude, z.verlustEinheiten, z.zweiteSchmelzeTick);
  std::fclose(f);
}

// Wertung eines Laufs (Spieler: feste Laufzeit, Angriff nach angerichteter Zerstoerung). Gewertet wird bis
// WERTUNG_TICK, danach ist der Wert eingefroren. Erzwerte /10 als Punkte.
//  Ziel 3 (Gegner vernichten): zerstoerter Feindwert, kleiner Zusatz fuer vollstaendige Vernichtung.
//  Ziel 2 (Bevoelkerung): Anteil am Ziel (1000 bei erreicht), dazu frueher = besser.
//  Ziel 1 (Satellit): erreicht 1000 minus Zeit, sonst 0; dazu die Kolonie (hoechste Einwohnerzahl).
//  Immer: eigene Verluste negativ, Kolonie am Ende noch da +200.
// Scoring up to a fixed tick: attackers by destruction dealt, others by goal progress; own losses count against.
// WERTUNG_TICK: siehe oben bei den Kampfwerten
// Neue Wertung (Spieler 2026-11-07), Ziel 3: Sieg = Kommandozentrale und Strukturfabrik des Gegners zerstoert. Je frueher, desto
// mehr; eine Niederlage kostet entsprechend, ein Unentschieden liegt bei null. Dazu gedeckelt +-1000: Zerstoerung gegen Verluste,
// Gebaeude (ausser Wachtuerme) viermal, Einheiten und Wachtuerme halb so schwer wie bisher (/20).
// Win/loss and time decide; a capped term weighs destruction against losses, buildings far heavier than units.
inline int laufPunkte(const Zustand& z) {
  int p = 0;
  if (z.ziel == 3) {
    int k = z.zerstoertWert / 10 - z.verlustEinheiten / 40 - z.verlustGebaeude / 5;
    if (k > 1000) k = 1000; if (k < -1000) k = -1000;
    const auto zeitWert = [](int t) { const int rest = WERTUNG_TICK - t > 0 ? WERTUNG_TICK - t : 0; return 2000 + (int)(8000.0 * rest / WERTUNG_TICK); };
    if (z.siegTick) return zeitWert(z.siegTick) + k;
    if (z.niederlageTick) return -zeitWert(z.niederlageTick) + k;
    return k;
  }
  if (z.ziel == 2) {
    // Bevoelkerung: Anteil am Ziel, dazu die selbst angerichtete Zerstoerung (Gegenangriffe lohnen sich) und Verluste
    // nur halb so schwer wie beim Angreifer (Spieler, 2026-11-05: Vorschlag fuer den Optimierer angenommen).
    // Population goal: share of target plus own destruction dealt; losses weigh half.
    const int anteil = z.zielWert > 0 ? (z.maxBew * 1000 / z.zielWert > 1000 ? 1000 : z.maxBew * 1000 / z.zielWert) : 0;
    p += anteil + z.zerstoertWert / 10;
    if (z.zielErreicht) p += (WERTUNG_TICK - z.zielErreicht) / 200;
    p += z.verlorenWert / 40;                       // gleicht den allgemeinen Abzug (/20) auf /40 aus
  } else {
    if (z.zielErreicht) p += 1000 - z.zielErreicht / 200;
    p += z.maxBew;
  }
  p -= z.verlorenWert / 20;
  if (l_kolonieLebt(z)) p += 200;
  return p;
}

// Zerstoerung zaehlen: alle Einheiten (Gebaeude und Fahrzeuge) der Gegner bzw. die eigenen mit ihrem Erzwert. Was
// beim naechsten Scan fehlt, wurde zerstoert (andere Wege, eine Einheit zu verlieren, gibt es hier nicht).
// Count destruction: units missing since the last scan were destroyed; their ore value is summed.
inline int einheitWert(UnitEx u) {
  UnitInfo i((map_id)u.GetType());
  int w = i.GetOreCost(0);
  const int waffe = u.IsVehicle() || u.GetType() == mapGuardPost ? (int)u.GetWeapon() : (int)mapNone;
  if (waffe > 0 && waffe != mapNone && waffe < 128) { UnitInfo wi((map_id)waffe); w += wi.GetOreCost(0); }
  return w > 0 ? w : 100;
}
inline void verlusteZaehlen(int* ids, int* werte, int* pos, int& anz, int& summe, int& heim, bool feind, const Zustand& z, int* summeGeb = nullptr, int* summeEinh = nullptr) {
  int neuId[192], neuWert[192], neuPos[192], n = 0;
  for (int p = 0; p < TethysGame::NoPlayers(); ++p) {
    if (feind ? (p == z.spieler || ExtPlayer[z.spieler].IsAlliedTo(p)) : p != z.spieler) continue;
    PlayerUnitEnum e(p);
    UnitEx u;
    while (e.GetNext(u) && n < 192) {
      if (!u.IsLive()) continue;
      const int t = u.GetType();
      // Robo-Miner/GeoCon verwandeln sich in Minen -- kein Verlust
      if (t == mapNone || t >= 128 || t == mapTube || t == mapWall || t == mapRoboMiner || t == mapGeoCon) continue;
      const LOCATION lo = u.Location();
      neuPos[n] = (edX(z, lo) & 0xffff) | (edY(z, lo) << 16);
      neuId[n] = u.unitID | (t << 16); neuWert[n] = einheitWert(u); n++;   // gleiche ID mit anderem Typ = neue Einheit
    }
    // Gebaeude ausdruecklich: in der Wertung 2026-11-07 blieb der Gebaeudeverlust trotz Niederlagen immer 0 (PlayerUnitEnum).
    for (int gt : GEBAEUDE) {
      PlayerBuildingEnum be(p, (map_id)gt); UnitEx g;
      while (be.GetNext(g) && n < 192) {
        if (!g.IsLive()) continue;
        const int kennung = g.unitID | (gt << 16);
        bool schon = false; for (int k = 0; k < n; ++k) if (neuId[k] == kennung) schon = true;
        if (schon) continue;
        const LOCATION lo = g.Location();
        neuPos[n] = (edX(z, lo) & 0xffff) | (edY(z, lo) << 16);
        neuId[n] = kennung; neuWert[n] = einheitWert(g); n++;
      }
    }
  }
  for (int i = 0; i < anz; ++i) {
    bool da = false;
    for (int k = 0; k < n; ++k) if (neuId[k] == ids[i]) { da = true; break; }
    if (!da) {
      summe += werte[i];
      if (summeGeb) {
        const int typ = (ids[i] >> 16) & 0xffff;
        bool geb = false;
        if (typ != mapGuardPost) for (int t : GEBAEUDE) if (t == typ) geb = true;
        if (geb) *summeGeb += werte[i]; else *summeEinh += werte[i];
      }
      const int x = (short)(pos[i] & 0xffff), y = pos[i] >> 16;
      if (abstand2(x, y, z.bx, z.by) <= 25 * 25) heim += werte[i];      // an der eigenen Basis gefallen
    }
  }
  std::memcpy(ids, neuId, sizeof(int) * n); std::memcpy(werte, neuWert, sizeof(int) * n); std::memcpy(pos, neuPos, sizeof(int) * n); anz = n;
}

inline void wissenLaden(Zustand& z) {
  char pfad[600];
  char datei[40]; std::snprintf(datei, sizeof(datei), "KI-Wissen-P%d.txt", z.spieler);
  wissenPfad(pfad, sizeof(pfad), datei);
  FILE* f = std::fopen(pfad, "r");
  int laeufe = 0, beste = -(1 << 30), besterLauf = 0;
  if (f) {
    char zeile[400];
    while (std::fgets(zeile, sizeof(zeile), f)) {
      double d; int i;
      // Reserve aus frueheren Laeufen: um 20 % Richtung Ausgangswert 1,3 zurueckbilden, hoechstens 2,5
      if (std::sscanf(zeile, "reserve=%lf", &d) == 1 && d >= 1.0 && d <= 4.0) {
        z.reserve = PAR(z).reserveStart + (d - PAR(z).reserveStart) * 0.8;
        if (z.reserve > PAR(z).reserveMax) z.reserve = PAR(z).reserveMax;
      }
      else if (std::sscanf(zeile, "erzJeRoute=%lf", &d) == 1 && d > 0) z.erzJeRouteGelernt = d;
      else if (std::sscanf(zeile, "turmAnteil=%lf", &d) == 1 && d >= 0 && d <= 1) z.turmAnteil = d;
      else if (std::sscanf(zeile, "zermuerbAbstand=%d", &i) == 1 && i >= 4000 && i <= 30000) z.zermuerbAbstand = i;
      else if (std::sscanf(zeile, "quoteZiel=%lf", &d) == 1) z.angriffQuote[1] = d;
      else if (std::sscanf(zeile, "quoteZermuerben=%lf", &d) == 1) z.angriffQuote[2] = d;
      else if (std::sscanf(zeile, "quoteBasis=%lf", &d) == 1) z.angriffQuote[0] = d;
      else if (std::sscanf(zeile, "laeufe=%d", &i) == 1) laeufe = i;
      else if (std::strncmp(zeile, "lauf=", 5) == 0) {
        int nr = 0, pk = 0;
        const char* pp = std::strstr(zeile, "punkte=");
        std::sscanf(zeile + 5, "%d", &nr);
        if (pp && std::sscanf(pp + 7, "%d", &pk) == 1 && pk > beste) { beste = pk; besterLauf = nr; }
      }
    }
    std::fclose(f);
  }
  z.laufNr = laeufe + 1;
  if (laeufe)
    melde(z, "lernen", mapNone, z.bx, z.by, "Wissen geladen: %d Laeufe bisher, bester Lauf %d mit %d Punkten -- Reserve %.2f, Erz je Route %.0f",
          laeufe, besterLauf, beste, z.reserve, z.erzJeRouteGelernt);
  else
    melde(z, "lernen", mapNone, z.bx, z.by, "Noch kein Wissen (%s) -- Reserve %.2f", pfad, z.reserve);
}

inline void wissenSpeichern(Zustand& z) {
  if (PAR(z).lernen == 0) { ergebnisSchreiben(z); return; }
  char pfad[600];
  char datei[40]; std::snprintf(datei, sizeof(datei), "KI-Wissen-P%d.txt", z.spieler);
  wissenPfad(pfad, sizeof(pfad), datei);
  // Alte Laufzeilen behalten (ohne diesen Lauf), hoechstens 19
  static char alt[20][400];
  int na = 0;
  FILE* f = std::fopen(pfad, "r");
  if (f) {
    char zeile[400];
    while (std::fgets(zeile, sizeof(zeile), f)) {
      if (std::strncmp(zeile, "lauf=", 5) != 0) continue;
      int nr = 0; std::sscanf(zeile + 5, "%d", &nr);
      if (nr == z.laufNr) continue;
      if (na == 19) { for (int i = 1; i < 19; ++i) std::memcpy(alt[i - 1], alt[i], 400); na = 18; }
      std::snprintf(alt[na++], 400, "%s", zeile);
    }
    std::fclose(f);
  }
  f = std::fopen(pfad, "w");
  if (!f) return;
  std::fprintf(f, "# KI-Wissen der Aufbau-KI (op2_ki.hpp) -- wird automatisch geschrieben\n");
  std::fprintf(f, "laeufe=%d\nreserve=%.3f\nturmAnteil=%.3f\n", z.laufNr, z.reserve, z.turmAnteil);
  std::fprintf(f, "zermuerbAbstand=%d\nquoteBasis=%.3f\nquoteZiel=%.3f\nquoteZermuerben=%.3f\n", z.zermuerbAbstand,
               z.angriffQuote[0], z.angriffQuote[1], z.angriffQuote[2]);
  // Erzertrag je Route: gelernter Wert, gleitend mit diesem Lauf
  const int re = erzRouten(z);
  if (re > 0 && z.zuflussErz / re > 0) {
    const double jetzt = z.zuflussErz / re;
    z.erzJeRouteGelernt = z.erzJeRouteGelernt > 0 ? z.erzJeRouteGelernt * 0.8 + jetzt * 0.2 : jetzt;
  }
  if (z.erzJeRouteGelernt > 0) std::fprintf(f, "erzJeRoute=%.1f\n", z.erzJeRouteGelernt);
  for (int i = 0; i < na; ++i) std::fputs(alt[i], f);
  std::fprintf(f, "lauf=%d tick=%d zielArt=%d ziel=%d gefechte=%d ueberstanden=%d zerstoert=%d verloren=%d maxBew=%d moral=%d feindbasis=%d kolonie=%d reserve=%.2f punkte=%d\n",
               z.laufNr, TethysGame::Tick() < WERTUNG_TICK ? TethysGame::Tick() : WERTUNG_TICK, z.ziel, z.zielErreicht, z.anzWellen, z.ueberstanden,
               z.zerstoertWert, z.verlorenWert, z.maxBew,
               z.moralN ? (int)(z.moralSumme / z.moralN) : 0, z.feindVernichtet, l_kolonieLebt(z) ? 1 : 0, z.reserve, laufPunkte(z));
  std::fclose(f);
}

// Einmal in InitProc. `ursprung` = Engine-Koordinate der Editor-Kachel (0,0), z.B. MkXY(1, 1).
// `ziel`: 1 = EDWARD-Satellit ins All schicken (Standard), 0 = nur die Kolonie aufbauen.
// `startStufe`: Umfang der Startausruestung (siehe startausruestung()).
// Fremde KI auch beim Start (Rust-Fassung baut die Basis selbst): ki::start ruft dann nur sie auf.
// Foreign AI hook for start as well.
typedef void (*FremdStart)(Zustand&, int, LOCATION, int, int, int, int, bool, int, int, int);
static FremdStart fremdStart[8] = { 0 };
inline void start(Zustand& z, int spieler, LOCATION ursprung, int breite, int hoehe, int x, int y, bool ausruestung,
                  int ziel = 1, int startStufe = 0, int zielWert = 0) {
  if (spieler >= 0 && spieler < 8 && fremdStart[spieler]) {
    z.aktiv = 1; z.spieler = spieler;
    fremdStart[spieler](z, spieler, ursprung, breite, hoehe, x, y, ausruestung, ziel, startStufe, zielWert);
    return;
  }
  std::memset(&z.plaetze, 0, sizeof(z.plaetze));
  std::memset(&z.roehren, 0, sizeof(z.roehren));
  z.aktiv = 1; z.spieler = spieler;
  tempo::an();                                   // nur mit OP2_TEMPO (Testlaeufe)
  parameterLaden(z);                             // Stellschrauben (Standard oder KI-Parameter-P<n>.txt)
  z.ux = ursprung.x; z.uy = ursprung.y;
  z.breite = breite; z.hoehe = hoehe;
  z.bx = x; z.by = y;
  z.naechsterSchritt = 0; z.naechsteSpur = 0;
  // DoResearch nimmt den Tabellenindex: mit der techID (z.B. 3305) stuerzt die Engine ab (Lauf 8,
  // Zugriffsfehler in Outpost2.exe bei jedem Auftrag). DoResearch takes the table index.
  z.forschungModus = 1; z.forschungLab = 0;
  z.kitTyp = 0; z.fzTyp = 0; z.anzahlPlaetze = 0; z.anzahlRoehren = 0;
  z.kitPauseBis = 0; z.fzPauseBis = 0;
  for (int i = 0; i < 8; ++i) { z.kitGesperrtTyp[i] = 0; z.kitGesperrtBis[i] = 0; }
  for (int i = 0; i < 32; ++i) z.meldungenSeit[i] = 0;
  for (int i = 0; i < MAX_MINEN; ++i) { z.minenId[i] = 0; z.schmelzeId[i] = 0; }
  z.bakeZiel = 0; z.bakeMiner = 0; z.bakeSeit = 0; z.minerSeit = 0; z.anzahlBaken = 0;
  z.minerWeg = 1; z.minenGesehen = 0; z.minenBauAn = 0;
  z.kbN = 0; z.kbStand = 0;
  for (int i = 0; i < 8; ++i) { z.sperrBake[i] = 0; z.sperrBis[i] = 0; }
  z.ladeConvec = 0; z.ladeSeit = 0;
  z.ziel = ziel; z.zielErreicht = 0;
  z.raumSchritt = 0; z.raumSeit = 0; z.raumVersuche = 0; z.raumAuftrag = 0;
  for (int i = 0; i < 2; ++i) { z.seltenMineId[i] = 0; z.seltenSchmelzeId[i] = 0; }
  z.seltenBake = 0; z.seltenBakeSeit = 0; z.seltenMiner = 0; z.seltenMinerSeit = 0;
  z.bauAn = 0; z.naechsterAnschluss = 0;
  z.anzahlGeparkt = 0; z.naechsteArbeit = 0; z.anzahlLeer = 0;
  std::memset(z.techGemeldet, 0, sizeof(z.techGemeldet)); z.naechsteTechPruefung = 0;
  z.moralFrei = 0; z.naechsteVerteilung = 0; z.fabrikStill = 0;
  for (int i = 0; i < MAX_VF; ++i) { z.fabrikStillW[i] = 0; z.fabrikDrin[i] = 0; }
  z.wunschArbeiter = 0; z.wunschRang = 0; z.wunschSeit = 0;
  z.naechsterScan = 0; z.feindArmee = 0; z.feindNah = 0; z.feindGeb = 0; z.feindX = 0; z.feindY = 0;
  z.feindBestWert = 0; z.feindBestTyp = 0; z.feindBestWaffe = 0;
  for (int i = 0; i < 4; ++i) z.forschGestartet[i] = 0;
  z.empRakete = z.empAuftrag = z.empAuftragSeit = z.empStart = z.empZielX = z.empZielY = 0;
  z.aufmarschX = z.aufmarschY = z.aufmarschSeit = 0;
  z.feindFabriken = 0; z.feindWT = 0; z.eigeneArmee = 0; z.sollArmee = 0; z.bedrohung = 0;
  z.letzterAngriff = 0; z.angriffe = 0; z.feindGemeldet = 0; z.wacheAn = 0; z.wacheAnzahl = 0;
  z.staerksterAngriff = 0; z.eigeneGeb = 0;
  z.feindPosN = 0; z.feindMinD2 = 1 << 30; z.feindNX = 0; z.feindNY = 0; z.eigeneMobil = 0; z.eigeneWTWert = 0;
  z.kampfModus = 0; z.kampfModusSeit = 0; z.angriffStart = 0;
  z.angriffSektoren = 0; z.naechsteVorhersage = 0;
  z.messErz = 0; z.messSelten = 0; z.messTick = 0; z.zuflussErz = 0; z.zuflussSelten = 0;
  for (int i = 0; i < 8; ++i) z.wellen[i] = 0;
  z.anzWellen = 0; z.welleSpitze = 0; z.erzUeberschuss = 0; z.welleStart = 0; z.wellenAbstand = 0;
  z.feindUnterwegs = 0; z.feindWachstum = 0; z.feindArmeeVorher = 0; z.turmAnteil = PAR(z).turmAnteil; z.reserve = PAR(z).reserveStart; z.welleEigenStart = 0; z.welleEigenMin = 0; z.laufNr = 0; z.naechstesSpeichern = 0;
  z.erzJeRouteGelernt = 0; z.verlustSumme = 0; z.maxBew = 0; z.ueberstanden = 0; z.feindVernichtet = 0;
  z.moralSumme = 0; z.moralN = 0;
  z.zweiteSchmelzeTick = 0; z.lynxFreiSeit = 0; z.siegTick = 0; z.niederlageTick = 0; z.verlustGebaeude = 0; z.verlustEinheiten = 0; z.feindHatteBasis = 0;
  z.anzFeindId = 0; z.anzEigenId = 0; z.zerstoertWert = 0; z.verlorenWert = 0; z.bewertet = 0; z.zerstoertHeim = 0; z.verlorenHeim = 0;
  z.laborWunsch = 0; z.laborWert = 0; z.laborFuer = 0; z.naechsterLaborCheck = 0;
  for (int i = 0; i < 4; ++i) z.laborNutzen[i] = 0;
  z.letztesKampfFz = 0; z.letzterBau = 0; z.kampfBedarfSeit = 0; z.bauDrangSeit = 0; z.naechstePruefung = 0; z.kampfLaeuft = 0;
  z.sammelAn = 0; z.angriffAn = 0; z.fabrikAn = 0; z.fabrikVf = 0;
  z.angriff2An = 0; z.reserveAn = 0; z.reserveZiel = 0; z.reserveOrt = -2; z.reserveSeit = 0;
  z.zielFz = 0; z.zielWaffe = 0; z.wacheZiel = 0; z.sammelZiel = 0; z.gruppenBedarf = 0; z.kampfFzAnzahl = 0;
  z.angriffArt = 0; z.angriffWunsch = 0; z.angriffZerstoertStart = 0; z.angriffVerlorenStart = 0; z.letzterAngriffStart = 0;
  z.bilStart = 0; z.bilN = 0; z.bilFN = 0; z.bilRueck = z.bilRueckErz = z.jagdZiel = z.jagdSeit = z.jagdTakt = 0;
  z.zielEmp = z.bilUeber = z.spinneTakt = 0; for (int i = 0; i < 8; ++i) { z.spinneZiel[i] = 0; z.spinneId[i] = 0; }
  z.garDran = z.garDranSeit = z.garRaus = z.garRausSeit = z.garZaehl = 0; for (int i = 0; i < 6; ++i) { z.garBucht[i] = 0; z.garFrei[i] = 0; }
  z.bilE = z.bilD = z.bilArmee = z.bilKontakt = z.bilErste = 0; z.bilNach = -1; z.bilKTick = 0; for (int i = 0; i < 6; ++i) z.bilK[i] = -1; for (int i = 0; i < 3; ++i) z.bilK2[i] = -1; z.bilGruppe = 0; z.bilZerHeim = z.bilVerHeim = 0;
  for (int i = 0; i < 3; ++i) z.angriffQuote[i] = 1.0;
  z.zermuerbAbstand = (int)PAR(z).zermuerbAbstand; z.angriffZiel = 0; z.feindPotenzial = 0;
  for (int i = 0; i < 6; ++i) { z.aussenId[i] = 0; z.aussenX[i] = 0; z.aussenY[i] = 0; z.aussenAngriff[i] = 0; z.aussenAn[i] = 0; }
  z.startErz = Player[spieler].Ore(); z.startSelten = Player[spieler].RareOre();
  Puffer p;
  p.f("{\"t\":%d,\"art\":\"start\",\"p\":%d,\"breite\":%d,\"hoehe\":%d,\"bx\":%d,\"by\":%d,\"ux\":%d,\"uy\":%d,\"eden\":%d,\"ziel\":%d,\"startStufe\":%d}",
      TethysGame::Tick(), spieler, breite, hoehe, x, y, z.ux, z.uy, Player[spieler].IsEden() ? 1 : 0, ziel, startStufe);
  p.schreiben();
  z.zielWert = zielWert > 0 ? zielWert : 500;
  if (PAR(z).lernen != 0) wissenLaden(z);
  else melde(z, "lernen", mapNone, z.bx, z.by, "Langzeit-Wissen aus (lernen=0) -- Lauf mit festen Parametern");
  if (ausruestung) {
    startausruestung(z, x, y, startStufe);
    if (startStufe >= 3) basisAufbauen(z);
    bakenSichern(z);
  }
}

#ifdef KI_MESSUNG
// Nur im Messbuild (-DKI_MESSUNG): ein fertiges Agridome bei Tick ~9000 stilllegen (DoIdle) und bei
// ~10600 wieder einschalten (DoUnIdle). Die Momentaufnahmen zeigen, wie es dabei aussieht (Aktion,
// Strom, Arbeiter) und ob die BuildingGroup es von sich aus wieder einschaltet.
// Measurement build only: idle a finished Agridome at ~9000, unidle at ~10600, watch the snapshots.
inline void messung(Zustand& z) {
  static int id = 0;
  const int t = TethysGame::Tick();
  if (!id && t >= 9000) {
    PlayerBuildingEnum e(z.spieler, mapAgridome);
    UnitEx u;
    while (e.GetNext(u)) {
      if (u.GetCurAction() != moDone || u.IsBusy()) continue;
      id = u.unitID;
      melde(z, "messung", mapAgridome, edX(z, u.Location()), edY(z, u.Location()), "vor DoIdle: Aktion %d, Strom %d, Arbeiter %d, stillgelegt %d",
            (int)u.GetCurAction(), u.HasPower() ? 1 : 0, u.HasWorkers() ? 1 : 0, ExtPlayer[z.spieler].GetNumIdleBuildings());
      u.DoIdle();
      break;
    }
    return;
  }
  if (!id) return;
  UnitEx u; u.unitID = id;
  if (!u.IsLive()) return;
  if (t % 200 == 3 || (t >= 10600 && t < 10620)) {
    melde(z, "messung", mapAgridome, edX(z, u.Location()), edY(z, u.Location()), "Agridome %d: Aktion %d, beschaeftigt %d, Strom %d, Arbeiter %d, stillgelegt %d, Arbeiter frei %d",
          id, (int)u.GetCurAction(), u.IsBusy() ? 1 : 0, u.HasPower() ? 1 : 0, u.HasWorkers() ? 1 : 0,
          ExtPlayer[z.spieler].GetNumIdleBuildings(), ExtPlayer[z.spieler].GetNumAvailableWorkers());
  }
  if (t >= 10600 && t < 10620) {
    u.DoUnIdle();
    melde(z, "messung", mapAgridome, edX(z, u.Location()), edY(z, u.Location()), "DoUnIdle gegeben");
  }
}
#endif

// Jeden AIProc-Aufruf; arbeitet intern im Takt.
// Fremde KI fuer einen Spieler (z. B. die Rust-KI): ki::start baut die Basis wie sonst, ki::schritt ruft dann nur sie auf.
// Foreign AI hook for a player (e.g. the Rust AI): start() builds the base as usual, schritt() only calls the hook.
typedef void (*FremdSchritt)(Zustand&);
static FremdSchritt fremdSchritt[8] = { 0 };
inline void schritt(Zustand& z) {
  if (!z.aktiv) return;
  if (z.spieler >= 0 && z.spieler < 8 && fremdSchritt[z.spieler]) { fremdSchritt[z.spieler](z); return; }
  const int t = TethysGame::Tick();
  if (t < z.naechsterSchritt) return;
  z.naechsterSchritt = t + TAKT;
  g_kraftSpieler = z.spieler;
  // Moral freigeben: der Codegen setzt am Ende von InitProc ForceMoraleGood fuer alle Spieler -- damit
  // haette die KI keinen Einfluss auf die Moral. FreeMoraleLevel darf jederzeit gerufen werden (kein
  // "Cheated Game"). Release morale: the codegen forces it to Good in InitProc.
  if (!z.moralFrei) {
    TethysGame::FreeMoraleLevel(z.spieler);
    z.moralFrei = 1;
    melde(z, "info", mapNone, z.bx, z.by, "Moral freigegeben (FreeMoraleLevel), Stufe jetzt %d", (int)Player[z.spieler].MoraleLevel());
    if (tempo::faktor > 1) melde(z, "info", mapNone, z.bx, z.by, "Testbeschleunigung: Spieluhr laeuft %.1f-fach (OP2_TEMPO), Bild %s gedrosselt",
                                 tempo::faktor, tempo::echtBitBlt ? "" : "NICHT");
  }
  Lage l;
  hinweis(z, "ki::erhebe"); erhebe(z, l);
  // Startvorrat einlagern, sobald es Lagerplatz gibt (siehe Zustand::startErz)
  if (z.startErz > 0 && l.erzMax > 0) {
    const int neu = z.startErz < l.erzMax ? z.startErz : l.erzMax;
    if (neu > l.erz) Player[z.spieler].SetOre(neu);
    melde(z, "info", mapCommonOreSmelter, z.bx, z.by, "Startvorrat eingelagert: %d Erz (Lager %d)", neu, l.erzMax);
    z.startErz = 0;
    hinweis(z, "ki::erhebe"); erhebe(z, l);
  }
  if (z.startSelten > 0 && l.seltenMax > 0) {
    const int neu = z.startSelten < l.seltenMax ? z.startSelten : l.seltenMax;
    if (neu > l.selten) Player[z.spieler].SetRareOre(neu);
    melde(z, "info", mapRareOreSmelter, z.bx, z.by, "Startvorrat eingelagert: %d Seltenerz (Lager %d)", neu, l.seltenMax);
    z.startSelten = 0;
    hinweis(z, "ki::erhebe"); erhebe(z, l);
  }
  hinweis(z, "ki::karteScannen"); karteScannen(z);          // Gegner-Lagebild (alle SCAN_TAKT Ticks), vor allem, was die Bedrohung liest
  if (!z.zielErreicht && z.ziel == 2 && z.zielWert > 0 && l.menschen >= z.zielWert) {
    z.zielErreicht = t;
    melde(z, "ziel", mapResidence, z.bx, z.by, "ZIEL ERREICHT: %d Einwohner (Ziel %d)", l.menschen, z.zielWert);
    op2::log::linef("Aufbau-KI Spieler %d: ZIEL ERREICHT -- %d Einwohner (Tick %d)", z.spieler, l.menschen, t);
  }
  if (!z.zielErreicht && z.ziel == 3 && z.feindGemeldet && !z.feindGeb) {
    z.zielErreicht = t;
    melde(z, "ziel", mapNone, z.bx, z.by, "ZIEL ERREICHT: Gegner vernichtet");
    op2::log::linef("Aufbau-KI Spieler %d: ZIEL ERREICHT -- Gegner vernichtet (Tick %d)", z.spieler, t);
  }
  if (t % 200 < TAKT) {                                    // einmal je 200 Ticks (Versatz-unabhaengig, Duell 8)
    if (l.menschen > z.maxBew) z.maxBew = l.menschen;
    if (!z.zweiteSchmelzeTick && l.geb[mapCommonOreSmelter] >= 2) z.zweiteSchmelzeTick = t;
    z.moralSumme += moralZahl(z); z.moralN++;
    if (!z.feindVernichtet && z.feindGemeldet && !z.feindGeb) z.feindVernichtet = t;
  }
  // auch nach der Wertung weiter zaehlen: die Angriffsquoten lernen daraus (die Wertung selbst ist eingefroren)
  if (t % SCAN_TAKT < TAKT) {
    hinweis(z, "ki::verlusteZaehlen");
    verlusteZaehlen(z.feindId, z.feindWert, z.feindPos, z.anzFeindId, z.zerstoertWert, z.zerstoertHeim, true, z);
    verlusteZaehlen(z.eigenId, z.eigenWert, z.eigenPos, z.anzEigenId, z.verlorenWert, z.verlorenHeim, false, z, &z.verlustGebaeude, &z.verlustEinheiten);
    // Sieg/Niederlage: Kommandozentrale und Strukturfabrik weg. Einmal gesetzt, bleibt es.
    if (t <= WERTUNG_TICK) {
      bool feindBasis = false, eigeneBasis = false;
      for (int q = 0; q < TethysGame::NoPlayers(); ++q) {
        const bool f = q != z.spieler && !ExtPlayer[z.spieler].IsAlliedTo(q);
        if (!f && q != z.spieler) continue;
        bool da = false;
        { PlayerBuildingEnum e(q, mapCommandCenter); UnitEx u; while (e.GetNext(u)) if (u.IsLive()) da = true; }
        { PlayerBuildingEnum e(q, mapStructureFactory); UnitEx u; while (e.GetNext(u)) if (u.IsLive()) da = true; }
        if (f) feindBasis = feindBasis || da; else eigeneBasis = da;
      }
      if (feindBasis) z.feindHatteBasis = 1;
      if (!z.siegTick && !z.niederlageTick && z.feindHatteBasis && !feindBasis) { z.siegTick = t; ergebnisSchreiben(z); melde(z, "lernen", mapNone, z.bx, z.by, "SIEG bei Tick %d", t); }
      if (!z.siegTick && !z.niederlageTick && t > 500 && !eigeneBasis) { z.niederlageTick = t; ergebnisSchreiben(z); melde(z, "lernen", mapNone, z.bx, z.by, "NIEDERLAGE bei Tick %d", t); }
    }
  }
  if (!z.bewertet && t >= WERTUNG_TICK) {
    z.bewertet = 1;
    melde(z, "lernen", mapNone, z.bx, z.by, "Wertung bei Tick %d: %d Punkte (Ziel %d %s, zerstoert %d, verloren %d, hoechstens %d Einwohner)",
          WERTUNG_TICK, laufPunkte(z), z.ziel, z.zielErreicht ? "erreicht" : "offen", z.zerstoertWert, z.verlorenWert, z.maxBew);
    hinweis(z, "ki::wissenSpeichern"); wissenSpeichern(z);
  }
  if (t >= z.naechstesSpeichern && !z.bewertet) { hinweis(z, "ki::wissenSpeichern"); wissenSpeichern(z); z.naechstesSpeichern = t + 5000; }
  // Zufluss messen: Netto-Aenderung des Vorrats (Einnahmen minus Ausgaben) je 1000 Ticks, gleitend ueber etwa
  // 4000 Ticks gemittelt. Nur Zunahmen zu zaehlen war zu optimistisch (Kampf 12: Lastwagen liefern in grossen
  // Portionen, die Bilanz zeigte Ueberschuss bei leerem Lager). Bei vollem Lager zaehlt die Messung nicht.
  // Net change of stock per 1000 ticks, smoothed; counting only increases was too optimistic.
  if (t - z.messTick >= 200) {
    if (z.messTick) {
      const double f = 1000.0 / (t - z.messTick);
      const int de = l.erz - z.messErz, ds = l.selten - z.messSelten;
      if (l.erz < l.erzMax) z.zuflussErz = z.zuflussErz * 0.95 + de * f * 0.05;
      if (l.selten < l.seltenMax) z.zuflussSelten = z.zuflussSelten * 0.95 + ds * f * 0.05;
    }
    z.messErz = l.erz; z.messSelten = l.selten; z.messTick = t;
  }
  hinweis(z, "ki::arbeitErheben"); arbeitErheben(z, l);      // nach dem letzten erhebe(): Arbeiter-Rangliste und Vormerkungen
  z.erzUeberschuss = bilanz(z, l).engpassErz <= 0 ? 1 : 0;
  // Ohne Kommandozentrale: der ConVec mit dem CC-Bausatz baut auf dem Mittelplatz.
  hinweis(z, "ki::bauGruppe"); bauGruppe(z, l);
  hinweis(z, "ki::arbeit"); arbeit(z, l);            // Arbeiter-Rangliste: stilllegen / einschalten
  if (t % 200 < TAKT) { hinweis(z, "ki::leerlauf"); leerlauf(z, l); }   // untaetige Gebaeude bei Personalmangel stilllegen
  hinweis(z, "ki::fahrzeuge"); fahrzeuge(z, l);
  hinweis(z, "ki::kampfFuehrung"); kampfFuehrung(z, l);
  hinweis(z, "ki::empRakete"); empRaketeBauen(z, l);     // Plymouth: EMP-Rakete im Raumhafen         // Wache, Aussenposten, Abfangen, Angriff
  hinweis(z, "ki::garage"); garageAblauf(z, l);
  hinweis(z, "ki::reparatur"); reparatur(z, l);          // beschaedigte Gebaeude reparieren
  hinweis(z, "ki::seltenErz"); seltenErz(z, l);          // vor erz(): reserviert den Robo-Miner fuer Seltenerz
  hinweis(z, "ki::erz"); erz(z, l);
  hinweis(z, "ki::anschluss"); anschluss(z, l);         // Roehren ueber die BuildingGroup (ersetzt den alten Earthworker-Weg)
  hinweis(z, "ki::vorhersageMelden"); vorhersageMelden(z, l);   // Techbaum-Vorhersage in die Spur (alle 10000 Ticks)
  hinweis(z, "ki::laborBedarf"); laborBedarf(z, l);        // fehlt der Forschung ein Labor? (alle 1000 Ticks)
  hinweis(z, "ki::selbstpruefung"); selbstpruefung(z, l);   // Widersprueche erkennen (alle 2000 Ticks)
  hinweis(z, "ki::forschung"); forschung(z, l);
  hinweis(z, "ki::ausbildung"); ausbildung(z, l);        // Uni: Wissenschaftler ausbilden, wenn die Forschung sie braucht
  hinweis(z, "ki::wissenschaftlerVerteilen"); wissenschaftlerVerteilen(z, l);   // laufende Forschungen: Ziel > Kolonie > Lueckenfueller
  hinweis(z, "ki::raumfahrt"); raumfahrt(z, l);
  if (t >= z.naechsteSpur) {
    z.naechsteSpur = t + SPUR_TAKT;
    hinweis(z, "ki::spur"); spur(z, l);
    if (TethysGame::Tick() % 1000 < SPUR_TAKT) { hinweis(z, "ki::lkwDiagnose"); lkwDiagnose(z); }
  }
#ifdef KI_MESSUNG
  messung(z);
#endif
  op2::crash::g_hinweis = nullptr;
}

} // namespace ki
