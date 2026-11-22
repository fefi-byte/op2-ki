# op2-ki in eine eigene Mission einbinden

*English summary at the end.*

Die KI ist eine eigene DLL (`op2ki.dll`). Eine Mission lädt sie beim Spielstart und ruft sie jeden Tick auf. Zum
**Bauen** einer Mission braucht man deshalb keinen KI-Code, nur drei Header aus
[`missionen/gemeinsam/`](missionen/gemeinsam/):

| Datei | Zweck |
|---|---|
| `op2ki.hpp` | `op2ki.dll` laden, Stellschrauben übergeben, starten, je Tick einen Schritt |
| `op2ki_bruecke.hpp` | beantwortet die Rufe der KI mit der OP2-API (OP2MissionSDK/HFL); von `werkzeug/bruecke.py` erzeugt |
| `op2_log.hpp` | Log und KI-Spur (`op2::log::line`, `linef`, `kiRaw`, `kiPath`, `setTickSource`) |

Eine eigene `op2_log.hpp` (z. B. die eines Missions-Editors) geht auch, wenn sie dieselben Funktionen hat.

Zum **Spielen** muss `op2ki.dll` neben der Missions-DLL im `OPU`-Ordner liegen. Fehlt sie, läuft die Mission
trotzdem, der KI-Spieler tut dann nichts; `op2ki::laden()` liefert `false` (Meldung ins Spiel schreiben).

## Aufrufe

```cpp
#include <HFL/Source/HFL.h>
#include "op2_log.hpp"
#include "op2ki.hpp"

// InitProc, nachdem die Spieler eingerichtet sind:
op2ki::parameterText[1] = "missionsModus=1\nrolle=2\n";              // Stellschrauben fuer Spieler 1 (name=wert je Zeile)
if (!op2ki::laden()) AddGameMessage("op2ki.dll fehlt");
op2ki::start(1, MkXY(1, 1), breite, hoehe, x, y, true, 3, 3, 500);   // Spieler, Ursprung, Kartengroesse, Startplatz,
                                                                      // Startbasis anlegen, Ziel, Stufe, Wert
// AIProc, jeden Tick:
op2ki::schritt(1);
```

- `MkXY(1, 1)`: Engine-Koordinate der Editor-Kachel (0,0); Kartengröße und Startplatz in Kacheln.
- Startbasis anlegen (`true`): die KI legt ihre Startbasis selbst an (Stufe 3 wie im Mehrspieler). Mit `false` übernimmt
  sie, was die Mission an Gebäuden und Fahrzeugen gesetzt hat.
- `op2::log::setTickSource([]{ return TethysGame::Tick(); })` in `DllMain` gibt den Log-Zeilen den Tick.
- Eine Datei `OPU\logs\KI-Parameter-P<n>.txt` überschreibt einzelne Stellschrauben (zum Ausprobieren).

Die Rufnummern der Brücke sind fest: eine ältere Missions-DLL läuft mit einer neueren `op2ki.dll` (neue Rufe liefern
dort 0, die KI kommt damit zurecht). Für neue Funktionen (z. B. die Schwierigkeit des Spiels, Ruf 142) die Mission mit
dem neuen `op2ki_bruecke.hpp` neu bauen.

## Missionsmodus

`missionsModus=1` macht aus dem Mehrspieler-Gegner einen Missions-Spieler. Die Schwierigkeit kommt aus dem Spiel
(leicht/normal/schwer, ohne Boni, nur Verhalten) oder aus `stufe=0..2`.

| Stellschraube | Bedeutung |
|---|---|
| `rolle` | 1 aggressiv, 2 defensiv (greift nicht selbst an, schlägt zurück), 3 Kolonie (baut aus, keine Angriffe) |
| `eigeneAngriffe` | 1/0: eigene Angriffe an/aus (überschreibt die Rolle) |
| `testPosten`, `testPostenX/Y`, `testPostenGroesse`, `testBakeX/Y` | ab diesem Tick eine Außenbasis bei X/Y; Größe 4 = unabhängige zweite Basis (Konvoi mit Bausätzen, eigene Fabriken, Wohnen, Medizin, Forum, Türme, Wache) |
| `zbWache`, `zbWacheDruck` | Kampffahrzeuge der zweiten Basis (4, unter Bedrohung 6) |
| `zbVorrang` | Ticks, die das Erz für den nächsten Bau der zweiten Basis vorgemerkt bleibt (3000) |
| `kigaAus`, `kigaAn` | Kindergarten aus über so vielen freien Arbeitern, wieder an darunter (20/15) |
| `minenUmkreis`, `minenJeSorte` | Minen nur im Umkreis einer Basis (35), höchstens eine je Erzsorte (1) |

Im Missionsmodus sind außerdem an (jeweils mit `-1` abschaltbar): Bezirke und Röhren zuerst (`bezirke`,
`roehrenVorrang`, `roehreAnNetz`), Gebiet und Zugangsmauern aus dem Gelände (`gebietAuto`, `zugangMauer`), Armee vor
der Basis mit Patrouille (`wacheVorBasis`, `wachePatrouille`), Kindergarten-Bremse (`kigaBremse`), Raumhafen in der
Mitte mit Festung (`raumhafen`), Spinnen für die Reparatur (`spinnen`, Plymouth), Aufstieg zu Panther und Tiger mit der
Waffe aus der Paarungs-Arena (`aufstieg`), MHD-Generator mit Seltenerz (`mhd`), Solarsatellit und Solarkraftwerk
(`solar`).

## English summary

To build a mission that uses op2-ki you only need `op2ki.hpp`, `op2ki_bruecke.hpp` and `op2_log.hpp` from
`missionen/gemeinsam/`. Call `op2ki::laden()`, set `op2ki::parameterText[player]`, call `op2ki::start(...)` once in
InitProc and `op2ki::schritt(player)` every AIProc. Ship `op2ki.dll` next to the mission DLL. `missionsModus=1` turns
the AI into a mission player (roles, outposts, second base, difficulty from the game); see the table above.
