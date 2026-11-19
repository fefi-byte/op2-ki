# OP2-KI

*Deutsche Fassung: [README.de.md](README.de.md)*

An AI opponent for **Outpost 2**, written in Rust. It builds a colony, researches, raises armies and attacks like
a fellow player. It runs inside the original game as `op2ki.dll`, loaded by a mission DLL.

This repository holds only what is needed to play and watch: the AI, its interface to the game and the missions.
The simulator and the training and measuring tools are not part of it. Code comments and in-game messages are in
German.

## Playing

Ready-built files are in [`fertig/`](fertig/) ("ready"). Copy them into the `OPU` folder of an Outpost 2
installation with the OPU update:

- `op2ki.dll` – the AI itself, always required
- `cDuGegenKI211.dll` – **you against the AI** on mp2_11
- `cDuGegenKI.dll` – **you against the AI** on mp2_08
- `cKIKRR<map>.dll` – **AI against AI** to watch, one per map: mp2_05, mp2_08, mp2_11, mp4_05, mp4_08, mp4_12,
  on4_03, on6_01, on6_03

Start a mission through the launcher (`OPU\op2launcher.exe cDuGegenKI211.dll`). The maps ship with the game and
are not included here.

**You against the AI:** you play Eden, the AI plays Plymouth. Both start with the same base: command center,
structure factory, tokamak, ore smelter, 2 agridomes, residence, standard lab, 2 ConVecs, robo-miner,
robo-surveyor, 2 cargo trucks and an earthworker, plus 26 workers, 13 scientists, 15 children, 8000 ore and 3000
food. Chance decides which corner you get. Whoever has no core buildings left (command center, factories, smelter,
tokamak, agridomes, residence, labs) loses.

At the start the message window must show **"KI: Rust-KI (op2ki.dll) geladen"** (Rust AI loaded). Without
`op2ki.dll` next to the mission there is no opponent (and in "you against the AI" no start base); the message window
says so.

**Watching:** the `cKIKRR*.dll` missions let two AIs play each other. With the environment variable
`OP2KI_KAMERA=1` the view follows the fighting.

**Saved games** are not supported: the AI keeps its state only in the running game.

## Logs

Every game writes to `OPU\logs`:

- `<mission>-KI.jsonl` – every decision of the AI with its reason (what, where, why)
- `Aufzeichnung-<date>-<time>.jsonl` – in "you against the AI", everything you do (units, colony, end of game)

Files `KI-Parameter-P1.txt` or `KI-Parameter-P2.txt` in `OPU\logs` change the AI; remove them for a normal game.

## How the AI works

The AI was translated function by function from an earlier C++ AI into Rust and then developed further. Rust is
now its only source; the missions contain no AI of their own.

| Area | Where | In short |
|---|---|---|
| Flow | `kern/src/cpp/ablauf` | start, step per tick, parameters, decision log |
| Planning | `kern/src/cpp/planung` | situation picture, urgencies, what to build next |
| Building | `kern/src/cpp/bau`, `vorplan.rs` | building sites, layout plan, tubes, workers |
| Ore | `kern/src/cpp/erz` | beacons, mines, truck routes, rare ore, vehicle choice |
| Research | `kern/src/cpp/forschung` | tech tree forecast (benefit per cost), labs, training |
| Military | `kern/src/cpp/kampf`, `paarung.rs` | army target, weapon choice by matchups, raids, base attack |
| Combat | `gefecht.rs`, `gefecht_netz.rs` | close combat by a learned network (`gefecht_netz.txt`) |

The AI sees the game only through the `Welt` ("world") trait in `kern/src/cpp/welt.rs`. In the DLL,
`dll/src/extern_welt.rs` maps it onto a call interface the mission passes in, and
`missionen/gemeinsam/op2ki_bruecke.hpp` answers those calls with the OP2 API. Both files are generated from the trait
by `werkzeug/bruecke.py`; a mission itself is only map data plus a call to `op2ki::start` and `op2ki::schritt`
(`missionen/gemeinsam/op2ki.hpp`).

## Building

Both are 32-bit Windows DLLs. Details (also cross-building on Linux) in [BAUEN.md](BAUEN.md) (German).

```
rustup target add i686-pc-windows-msvc
cargo build --release -p op2ki-dll --target i686-pc-windows-msvc

git clone https://github.com/OutpostUniverse/OP2MissionSDK.git
cmake -S missionen -B build -A Win32 -DOP2SDK_DIR=<path>/OP2MissionSDK
cmake --build build --config Release
```

## License

GPL-3.0-or-later, see [LICENSE](LICENSE), for the code in this repository (copyright 2026 fefi-byte). The missions are built against the
[OP2MissionSDK](https://github.com/OutpostUniverse/OP2MissionSDK) of Outpost Universe, whose OP2Helper and HFL parts
are compiled into the mission DLLs in `fertig/`. Outpost 2 itself is not included.
