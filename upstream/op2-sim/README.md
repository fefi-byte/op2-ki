# op2-sim fixes for op2-rs

Five commits for [op2-rs](https://gitlab.com/bufke/op2-rs), against `main` at `4843d057`. To apply them:

```
cd op2-rs
git am /path/to/op2-ki/upstream/op2-sim/*.patch
```

## Path context of pending orders, self-Guard, group Produce, earthworker turn

op2-ki (an AI for Outpost 2, https://github.com/fefi-byte/op2-ki) trains and tests on op2-sim. To get the same games
in op2-sim as in the original, we compared AI-vs-AI games tick by tick against the original in the Wine harness. Where
they split, we fixed op2-sim. These are those fixes, rebased onto `main` at `4843d057` (they were made on `d0f5980`).

Five commits, one behaviour each:

1. **A path order waiting for the unit's turn leaves the old path context in place.** The packet fills +0x38 and the
   unit's own context (+0x34) stays until its turn swaps them (0x44B640). Until then:
   - the bump reads the old path's next step and turning state
   - a Stop restores the old waypoints and keeps the steps
   - a side-step taken before the turn goes with the old context
   - an idle unit whose Move context is still there goes back to its waypoint

   Measured: mp2_05 seed 1 (tanks 54, 65, 66, 67), mp2_08 seed 1 (Lynx 57, Spider 49), mp4_12 seed 3 (Earthworker 53).
2. **A Guard order on the unit itself clears its targets, and an Attack without a target ends** (0x43AA10 at 0x40FC71;
   0x43D28A..0x43D2D0). Measured: mp2_05 seed 1 tick 30207, on4_03 seed 1 tick 38528.
3. **A building group's Produce and cargo transfer go through the repeated structure packet check**
   (0x42A4A0 -> 0x40E300, 0x40E87B..0x40E90D). Measured: mp2_11 seed 1 tick 19215.
4. **A turn on the spot ends when an earthwork order arrives.** Measured: mp2_08 seed 1, Earthworker 11, tick 1319.
5. **`Sim::last_index`**: the highest object index handed out (lastUsedUnitIndex, HFL +64), for an AI bridge.

Every comment names the map, seed, tick and unit of its measurement, and the address it relies on.

We also had a fix for `is_x87` (Fxch/Fxam), missing kernel32/gdi32 calls in op2-missionhost and a lab-team setter.
`main` has all three now, so they are not part of these patches.

## How it was tested
- `cargo test -p op2-sim` (with `OP2_DIR`): passes with these commits, same results as on `main`.
- `cargo clippy -p op2-sim --all-targets -- -D warnings`: clean.
- `cargo check -p op2-sim`: passes after every commit.
- The measurements above were made on `d0f5980`: our AI games in the original (Wine harness) against op2-sim, compared
  tick by tick. After the rebase onto current `main` we have not measured them again.
- `make accuracy` not run: the recorded runs of `harness/oracle/runs` aren't available to us. Please run it. If a
  fix lowers a score, tell us which run and we'll look at it.

## AI use

Written with Claude Code (Anthropic). The measurements are from our own harness runs.

`sim/op2-sim.patch` in this repository holds the same fixes in their first form, against `d0f5980`, the base `sim/` is built and
measured on.
