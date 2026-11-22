# OP2-KI

*English version: [README.md](README.md)*

Eine KI für **Outpost 2** in Rust. Sie baut eine Kolonie auf, forscht, baut Armeen und greift an – wie ein
Mitspieler. Sie läuft im Original-Spiel als `op2ki.dll` und wird aus einer Missions-DLL geladen.

Dieses Repository enthält, was zum Spielen und Zuschauen gebraucht wird: die KI, ihre Schnittstelle zum Spiel und die
Missionen, dazu [`sim/`](sim/), das die KI ohne Spiel auf op2-sim (dem nativen Engine-Nachbau aus op2-rs) gegen sich
selbst spielen lässt. Lern- und Messwerkzeuge gehören nicht dazu.

## Spielen

Fertige Dateien liegen in [`fertig/`](fertig/). In den Ordner `OPU` der Outpost-2-Installation (mit OPU-Update)
kopieren:

- `op2ki.dll` – die KI selbst, immer nötig
- `cDuGegenKI211.dll` – **Du gegen die KI** auf mp2_11
- `cDuGegenKI.dll` – **Du gegen die KI** auf mp2_08
- `cKIKRR<karte>.dll` – **KI gegen KI** zum Zuschauen, eine je Karte: mp2_05, mp2_08, mp2_11, mp4_05, mp4_08,
  mp4_12, on4_03, on6_01, on6_03

Die Mission über den Launcher starten (`OPU\op2launcher.exe cDuGegenKI211.dll`). Die Karten gehören zum normalen
Spiel und sind hier nicht enthalten.

**Du gegen die KI:** Du spielst Eden, die KI Plymouth. Beide starten mit derselben Basis: Kommandozentrale,
Strukturfabrik, Tokamak, Erzschmelze, 2 Agridome, Wohnhaus, Standard-Labor, 2 ConVecs, Robo-Miner, Robo-Surveyor,
2 Lastwagen und ein Earthworker. Dazu 26 Arbeiter, 13 Wissenschaftler, 15 Kinder, 8000 Erz und 3000 Nahrung.
Welche Ecke du bekommst, entscheidet der Zufall. Wer keine Kerngebäude mehr hat (Zentrale, Fabriken, Schmelze,
Tokamak, Agridome, Wohnhaus, Labore), hat verloren.

Beim Start muss im Nachrichtenfenster **„KI: Rust-KI (op2ki.dll) geladen“** stehen. Fehlt `op2ki.dll` neben der
Mission, gibt es keinen Gegner (und bei „Du gegen die KI“ auch keine Startbasis); das Nachrichtenfenster sagt es.

**Zuschauen:** Die Missionen `cKIKRR*.dll` lassen zwei KIs gegeneinander spielen. Mit der Umgebungsvariable
`OP2KI_KAMERA=1` folgt die Ansicht den Kämpfen.

**Spielstand laden** wird nicht unterstützt: die KI behält ihren Zustand nur im laufenden Spiel.

## Aufzeichnung

Jede Partie schreibt nach `OPU\logs`:

- `<Mission>-KI.jsonl` – jede Entscheidung der KI mit Begründung (was, wo, warum)
- `Aufzeichnung-<Datum>-<Zeit>.jsonl` – bei „Du gegen die KI“ alles, was du tust (Einheiten, Kolonie, Spielende)

Liegen in `OPU\logs` Dateien `KI-Parameter-P1.txt` oder `KI-Parameter-P2.txt`, verändern sie die KI. Für ein
normales Spiel entfernen.

## Was die KI tut

Die KI ist aus einer früheren C++-KI Funktion für Funktion nach Rust übertragen und dann weiterentwickelt worden.
Rust ist jetzt ihre einzige Quelle; die Missionen enthalten keine eigene KI.

| Bereich | Wo | Kurz |
|---|---|---|
| Ablauf | `kern/src/cpp/ablauf` | Start, Schritt je Tick, Stellschrauben, Spur |
| Planung | `kern/src/cpp/planung` | Lagebild, Dringlichkeiten, was als Nächstes gebaut wird |
| Bau | `kern/src/cpp/bau`, `vorplan.rs` | Bauplätze, Bebauungsplan, Röhren, Arbeiter |
| Erz | `kern/src/cpp/erz` | Baken, Minen, Lastwagen-Routen, Seltenerz, Fahrzeugwahl |
| Forschung | `kern/src/cpp/forschung` | feste Eröffnung (Kindergarten, Universität, Weg zum Lynx), danach Techbaum-Vorhersage (Nutzen je Kosten, Kampf-Anteil zehnfach, Panzerung gegen die Feindwaffen, Tempo, Voraussetzungen für Baubares, eine Linie halten), Labore, Ausbildung |
| Kampf | `kern/src/cpp/kampf`, `paarung.rs` | frühe Wehr (frühester Angriff aus Lynx-Forschung, Bauzeit und Weg zur nächsten Feindbasis), Armee-Soll, Wachgebiet um alle Gebäude der Basis, Waffenwahl nach Paarungen, Zermürben, Basisangriff |
| Gefecht | `gefecht.rs`, `gefecht_netz.rs` | Nahkampf mit einem gelernten Netz (`gefecht_netz.txt`) |
| Zwei Wege | `parallel.rs`, `gelaende.rs` | Angriff über zwei getrennte Wege, wenn das Gelände sie hat |
| Zweitbasis | `zweitbasis.rs` | Außenposten mit Kommandozentrale und Schmelze an einem fernen Bakenfeld, sobald die Hauptbasis steht, Erz übrig und Personal frei ist; im Missionsmodus eine unabhängige Basis mit eigenen Fabriken, Kolonie und Wache |
| Gelände | `erweiterung.rs`, `kampf/marsch.rs` | Wache hinter dem Pass und Türme an Engstellen; die Angriffstruppe sammelt sich auf dem Marsch |
| Missionsmodus | `modus.rs`, `bezirke.rs` | Schwierigkeit, Rollen, Außenposten auf Befehl; Bezirke, Röhren zuerst, Raumhafen-Festung |
| Mauern | `mauer.rs` | merkt sich aufgezeichnete Mauern, damit dort nichts anderes geplant wird; Bau-Reihenfolge: erst Gebäude mit Personal anschließen, dann Wachtürme (Röhre zur Kommandozentrale), zuletzt Mauern |

Die KI sieht das Spiel nur über den Trait `Welt` (`kern/src/cpp/welt.rs`). In der DLL setzt `dll/src/extern_welt.rs`
ihn auf eine Rufschnittstelle um, die die Mission übergibt; `missionen/gemeinsam/op2ki_bruecke.hpp` beantwortet die
Rufe mit der OP2-API. Beide Dateien erzeugt `werkzeug/bruecke.py` aus dem Trait; eine Mission ist nur noch Kartendaten
plus `op2ki::start` und `op2ki::schritt` (`missionen/gemeinsam/op2ki.hpp`).

## Missionsmodus und eigene Missionen

Mit `missionsModus=1` spielt die KI eine Mission statt eines Mehrspieler-Spiels: Schwierigkeit aus dem Spiel, Rollen
(aggressiv, defensiv, Kolonie), Außenposten oder eine unabhängige zweite Basis auf Befehl, Bezirke und Mauern aus dem
Gelände, Armee mit Patrouille vor der Basis, Spinnen, Aufstieg zu Panther und Tiger, MHD- und Solarkraftwerk. Für eine
eigene Mission (oder aus einem Missions-Editor) braucht man nur drei Header und `op2ki.dll`, siehe
[EINBINDEN.md](EINBINDEN.md).

## Simulator

[`sim/`](sim/) spielt op2-ki gegen op2-ki auf op2-sim in Sekunden und schreibt dieselbe Spur wie das Spiel. Es ist auch
die Beispiel-Umsetzung des `Welt`-Traits für eine andere Engine. Einrichten und Schalter in [sim/README.md](sim/README.md).

## Bauen

Siehe [BAUEN.md](BAUEN.md). Lizenz: GPL-3.0-or-later ([LICENSE](LICENSE)) für den Code hier; die Missionen bauen gegen das OP2MissionSDK von Outpost Universe.
