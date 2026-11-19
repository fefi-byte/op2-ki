# op2ki-sim

*Deutsch unten.*

Runs op2-ki against op2-ki on **op2-sim**, the native Outpost 2 engine rebuild of
[op2-rs](https://gitlab.com/bufke/op2-rs). There is no game, no Wine and no graphics. A whole multiplayer game of
100,000 ticks takes about 10 s, and the AI behaves the same as in the original game as far as op2-sim does.

This is also the reference for plugging op2-ki into another engine: `src/welt_sim.rs` implements the `Welt` trait
(`kern/src/cpp/welt.rs`, one method per OP2 API call, engine coordinates) on top of op2-sim, and `src/partie2.rs`
starts a game and calls the AI each tick:

```rust
op2ki::cpp::globale::zuruecksetzen();                         // once per game
op2ki::cpp::start(&mut welt, &mut zustand, player, origin, width, height, base_x, base_y, true, 3, 3, 500);
loop { op2ki::cpp::schritt(&mut welt, &mut zustand, player); /* advance the engine one tick */ }
```

Tuning goes in as text, `name=value` per line, per player (`cpp::globale::parameterText`).

## Setup

op2-sim needs a few local fixes that are not upstream yet (path handling during pending orders, the last object
index). They are in `op2-sim.patch`, against op2-rs commit `d0f5980`:

```
git clone https://gitlab.com/bufke/op2-rs.git      # next to this repository: ../op2-rs
cd op2-rs && git checkout d0f5980 && git apply ../op2-ki/sim/op2-sim.patch && cd ..
cd op2-ki/sim && cargo build --release
```

## Running

```
target/release/op2ki-sim --game-dir "<Outpost 2 folder>" --szenario daten/szenarien/scn_ml2_11.dll.json \
    --seed 1 --ticks 400000 --ende --p1 "name=value;..." --p2 "..." --spur game.jsonl
```

- `--game-dir`: an Outpost 2 installation (for the map, the tech tree and the unit data)
- `--szenario`: start state of a map (beacons, start positions), recorded from the multiplayer missions by
  op2-missionhost; nine maps are in `daten/szenarien`
- `--ende`: stop 2500 ticks after one side has lost its command center and structure factory
- `--spur`: the AI's decision trace, one JSON line per decision (what, where, why), the same as `<mission>-KI.jsonl` in
  the game

---

# op2ki-sim (deutsch)

op2-ki gegen op2-ki auf **op2-sim**, dem nativen Nachbau der Outpost-2-Engine aus
[op2-rs](https://gitlab.com/bufke/op2-rs) -- ohne Spiel, ohne Wine. Eine Mehrspieler-Partie mit 100.000 Ticks dauert
etwa 10 s. `src/welt_sim.rs` setzt das `Welt`-Trait auf op2-sim um und ist damit das Beispiel, wie man op2-ki an eine
andere Engine haengt.

Einrichten: op2-rs neben dieses Repo klonen, auf Commit `d0f5980` stellen, `op2-sim.patch` anwenden (lokale
Korrekturen an op2-sim), dann `cargo build --release` in `sim/`. Aufruf und Schalter wie oben.
