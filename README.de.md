# OP2-KI

*English version: [README.md](README.md)*

Eine KI für **Outpost 2** in Rust. Sie baut eine Kolonie auf, forscht, baut Armeen und greift an – wie ein
Mitspieler. Sie läuft im Original-Spiel als `op2ki.dll` und wird aus einer Missions-DLL geladen.

Dieses Repository enthält nur das, was zum Spielen und Zuschauen gebraucht wird: die KI, ihre Schnittstelle zum
Spiel und die Missionen. Simulator, Lern- und Messwerkzeuge gehören nicht dazu.

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
Mission, spielt eine ältere, eingebaute C++-KI (nur bei „Du gegen die KI“; beim Zuschauen bleiben die KIs dann
untätig).

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

Die KI ist aus einer C++-KI (`missionen/gemeinsam/op2_ki.hpp`) Funktion für Funktion nach Rust übertragen und dann
weiterentwickelt worden. Die Missionen enthalten die C++-Fassung noch als Rückfall.

| Bereich | Wo | Kurz |
|---|---|---|
| Ablauf | `kern/src/cpp/ablauf` | Start, Schritt je Tick, Stellschrauben, Spur |
| Planung | `kern/src/cpp/planung` | Lagebild, Dringlichkeiten, was als Nächstes gebaut wird |
| Bau | `kern/src/cpp/bau`, `vorplan.rs` | Bauplätze, Bebauungsplan, Röhren, Arbeiter |
| Erz | `kern/src/cpp/erz` | Baken, Minen, Lastwagen-Routen, Seltenerz, Fahrzeugwahl |
| Forschung | `kern/src/cpp/forschung` | Techbaum-Vorhersage (Nutzen je Kosten), Labore, Ausbildung |
| Kampf | `kern/src/cpp/kampf`, `paarung.rs` | Armee-Soll, Waffenwahl nach Paarungen, Zermürben, Basisangriff |
| Gefecht | `gefecht.rs`, `gefecht_netz.rs` | Nahkampf mit einem gelernten Netz (`gefecht_netz.txt`) |

Die KI sieht das Spiel nur über den Trait `Welt` (`kern/src/cpp/welt.rs`). In der DLL setzt `dll/src/extern_welt.rs`
ihn auf die Funktionszeiger der Mission um; jede Mission bringt dafür die OP2-API als Brücke mit.

## Bauen

Siehe [BAUEN.md](BAUEN.md). Lizenz: MIT ([LICENSE](LICENSE)) für den Code hier; die Missionen bauen gegen das OP2MissionSDK von Outpost Universe.
