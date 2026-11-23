# Validation — Powah slices 1 to 11

> **0.2.11 (slice 11, the Ender Cell and Ender Gate)** is recorded in
> [`notes.md`](notes.md#slice-11--the-ender-cell-and-the-ender-gate-2026-11-23): the slice and the
> gap lines it comes from (`FINDINGS.md`'s "Ender Cells/Gates" and slice 10's "Still open"), the
> upstream facts it is built on (`devices.ender_cells`/`ender_gates` = `EnderConfig` over
> `batteryTransfer()`/`cableTransfer()` and `TieredChannelValues` = 1/2/3/5/7/9/12, capacity **0**
> until an extender, the shared per-owner `EnderNetwork` `SavedData`, `keepEnergy()` **false**, the
> cell's extender slot, the gate's `isEnergyPresent(facing)` and `chargeItems(1, 3)`), the red proof
> (seven `todo!()` rules, 73 passed / 6 failed, quoted verbatim), the ladder at pin `0a875da`
> (84 tests, clippy 0 warnings, `fmt --check` clean, two `cmp`-identical builds of **482,716 B**,
> `check`, `fits` against the pin **and** against `0b14f8a`, gate 1/1 with **770 entries**, dialect
> 239/239) and the in-game legs (main `PASS`+`EXIT:0`; `SELFTEST=1` and `SELFTEST=ender` at
> `EXIT:1`; the control game knows none of the ender ids; the log gate). Package: `powah.hmod`,
> 482,716 B, sha256 `49ba3462341a8460bd020a5e5816113a974be3fee78555d226eab4923b71612f` (the in-game legs ran the 482,574 B archive of the same
> sources, whose only difference is `README.md`). Launcher
> build: `Compiling powah (1 of 1)`, `native-0a875da-32e311705549`, instance `powah-e2e`. What it
> proves: two ender cells on one channel hold and show the same 100,000/4,000,000 FE with their own
> tiers' transfer (4,000 and 1,000 a side) printed beside it; a cell moved to channel 1 reads 0/0
> while its peer keeps exactly 100,000, and moving it back finds the channel's FE again (the store is
> the world's, not the block's); an energy cell item in the extender slot gives the channel
> 4,000,000 FE of capacity; the charge slot takes 1,000 a tick (the item's own limit) while the
> channel loses exactly the same; the channel gives its tier's 4,000 a tick and loses exactly what
> the sink gains; a starter ender gate feeds only the hopper on its `facing` side, at the cable
> row's 500 a tick, and the one behind it stays 0; the nitro cell and gate name 12 channels and
> their 2,000,000/1,000,000 FE a tick; the orb makes the ender core at 50,000 FE; both ender tags
> match in game; all fourteen tiers drop themselves; and every guessed id is unknown without the
> package.
> **0.2.10 (slice 10, the Solar Panel)** is recorded in
> [`notes.md`](notes.md#slice-10--the-solar-panel-ladder-2026-11-23): the slice and the gap lines it
> comes from (`FINDINGS.md`'s "Magmator/Thermo/Solar/Reactors" and slice 9's own "the generators need
> a mod fluid tank and sky visibility", which this slice shows is wrong for solar — the pin reads the
> sky natively), the upstream facts it is built on (`devices.solar_panels` = `passiveProduction` over
> the furnator's capacity/transfer, `postTick`'s sky + daylight + room rule, `canSeeSky` at
> `pos.above()`, `getSkyDarken() < 4`, `isEnergyPresent(DOWN)`, one charge slot, `keepEnergy`), the
> red proof (five `todo!()` rules, 69 passed / 5 failed, quoted verbatim), the ladder at pin
> `0a875da` (74 tests, clippy 0 warnings, `fmt --check` clean, two `cmp`-identical builds of
> **388,951 B**, `check`, `fits` against the pin **and** against `0b14f8a`, gate 1/1 with **624
> entries**, dialect 197/197) and the in-game legs (main `PASS`+`EXIT:0`; control and
> `SELFTEST=solar` at `EXIT:1`; the **0.2.9 package red** at the slice's own block id after every
> earlier phase passed; the log gate 37/37). Package: `powah.hmod`, 388,951 B, sha256
> `13f0e7be10390536b39ae315d81878f257e5a5921f2cf7c071b45c60d6cfc9b4` (rebuilt from the final
> tree: the first archive packed the pre-slice-10 `README.md`; the packed one is now byte-identical to
> the tree's and every packed tree file matches). Launcher build:
> `Compiling powah (1 of 1)`, `native-0a875da-eb0b2f34c233`, instance `powah-e2e`. What it proves: a
> starter panel under an open noon sky generates exactly 20 FE a tick measured against the game's
> clock (`+120` over `dt=6`) and fills its last 20 of room to exactly 20,000 without overflowing; a
> stone at the block above it and `/time set midnight` each hold it at exactly 5,000 for 60 ticks
> (`sky covered, day true` / `sky open, day false`); a cell under a panel charges while a cell beside
> it stays empty and the panel takes no fill; the nitro tier clamps 2,000,000,000 FE at 40,000,000
> and names `generating 2000 FE/t, 160000 FE/t a side`; `powah:solar_panels` matches in game; the
> photoelectric pane and all seven panels craft; and every panel tier drops itself.
> **0.2.9 (slice 9, the Energy Hopper)** is recorded in
> [`notes.md`](notes.md#slice-9--the-energy-hopper-2026-11-23): the slice and the gap lines it comes
> from (item 11's unported machines, `FINDINGS.md`'s "Discharger/Hopper/Player Transmitter" and the
> 0.2.8 section's own "left for the next slice"), the upstream facts it is built on
> (`devices.hoppers` = `batteryCapacity`/`batteryTransfer`/`chargingTransfer(500)`, no slots of its
> own, `postTick` = `chargeItemsInContainer` on `state.facing`, `Transfer.ALL`, `keepEnergy`), the red
> proof (four `todo!()` rules, 64 passed / 4 failed, quoted verbatim), the ladder at pin `0a875da`
> (68 tests, clippy 0 warnings, `fmt --check` clean, two `cmp`-identical builds of **356,866 B**,
> `check`, `fits` against the pin **and** against `0b14f8a`, gate 1/1 with **545 entries**, dialect
> 174/174) and the in-game legs (main `PASS`+`EXIT:0`; control and `SELFTEST=hopper` at `EXIT:1`; the
> **0.2.8 package red** at the slice's own block id after every earlier phase passed; the log gate
> 28/28). Package: `powah.hmod`, 356,866 B, sha256
> `387259ea8a3b6ce0975279cd4f4d67620d8c2fc11d1c634a76cc8113e32045f3`. Launcher build:
> `Compiling powah (1 of 1)`, `native-0a875da-6653b4fc239a`, instance `powah-e2e`. What it proves: a
> starter hopper with a cell item at 0 in the chest it faces spends exactly its 5,000 and leaves the
> item at 5,000 (10 charges of 500); the rate is 500 FE a tick measured against the game's clock
> (`3,500` over `dt=7`, whole 500 steps); a hopper facing away from the chest spends nothing and a
> stone in the chest takes nothing; the nitro hopper clamps 2,000,000,000 FE and names
> `charging 1000000 FE/t, 2000000 FE/t a side`; `powah:energy_hoppers` matches in game; a broken 7,000
> FE hopper carries its buffer back on its own item; and every hopper tier drops itself.
> **0.2.8 (slice 8, the Energy Discharger)** is recorded in
> [`notes.md`](notes.md#slice-8--the-energy-discharger-2026-11-22): the slice and the gap lines it comes
> from (item 11's unported machines and `FINDINGS.md`'s "Discharger", named as the item-energy family's
> mirror by the 0.2.7 report), the upstream facts it is built on (`devices.dischargers` =
> `batteryCapacity`/`batteryTransfer`, seven slots, `postTick` = drain then push, `Transfer.EXTRACT`), the
> red proof (four `todo!()` rules, 59 passed / 4 failed, quoted verbatim), the ladder at pin `0a875da`
> (63 tests, clippy 0 warnings, `fmt --check` clean, two `cmp`-identical builds of **330,025 B**, `check`,
> `fits` against the pin **and** against `0b14f8a`, gate 1/1 with **483 entries**, dialect 153/153) and the
> in-game legs (main `PASS`+`EXIT:0`; control and `SELFTEST=drain` at `EXIT:1`; the **0.2.7 package red**
> at the slice's own block id after every earlier phase passed; `check-e2e-logs --strict` 24/24).
> Package: `powah.hmod`, 330,025 B, sha256
> `d152d396ff809f291b8bee6043f78ac1f1a23771a1b44dcfb6fb00c6f3b43950`. Launcher build:
> `Compiling powah (1 of 1)`, instance `powah-e2e`. What it proves: a cell item charged to 5,000 in a
> starter discharger ends fully in the 1,000,000 FE buffer and reads 0; 20 of room takes exactly 20 of a
> 5,000 item (not the 1,000 transfer) and a full buffer stops it; the drain is 1,000 FE a tick measured
> against the game clock; a stone gives nothing; a nitro discharger clamps at 2,000,000,000 FE and names
> 2,000,000 FE a side; `powah:energy_dischargers` matches in game; a broken 7,000 FE discharger carries
> its buffer back; and a charged cell beside one keeps its 50,000 (`Transfer.EXTRACT`).
> The legs ran the pack of the same sources whose only other difference is `README.md` (329,812 B,
> `99bc3239…`); all 483 entries are otherwise byte-identical to the installed copy the legs used.
> **0.2.7 (slice 7, the item-energy family: `keepEnergy` and the item charge slots)** is recorded in
> [`notes.md`](notes.md#slice-7--the-item-energy-family-keepenergy-and-the-item-charge-slots-2026-11-22):
> the slice and its gap lines (items 4 and 5: "no item charging", "breaking a machine loses its FE"),
> the red proof (three `todo!()` rules, 51 passed / 7 failed, quoted verbatim), the ladder at pin
> `0a875da` (58 tests, clippy 0 warnings, `fmt --check` clean, two `cmp`-identical builds of
> **305,679 B**, `check`, `fits` against the pin **and** against `0b14f8a`, gate 1/1 with **422
> entries**, dialect 132/132) and the in-game legs (main `PASS`+`EXIT:0`; control, `SELFTEST=1`,
> `SELFTEST=keep` and `SELFTEST=charge` at `EXIT:1`; the **0.2.6 package red** at the slice's own
> payload assertion; `check-e2e-logs --strict` 20/20). Package: `powah.hmod`, 305,679 B, sha256
> `8cf04f34e5135e4e07a66a516c42da11c2841e82f82458d63b3ae29a480269d3` (the legs ran the pack of
the same sources whose only other difference is `README.md`, 305,399 B `91cca602…`). Launcher build:
> `Compiling powah (1 of 1)`, `native-0a875da-d4ab2c99bd07`. What it proves: a 5,000 FE furnator and a
> 4,000 FE cell broken in survival carry their buffer on their own item (`minecraft:block_entity_data`
> `{id, energy}`, no `Items`), place back at exactly that value (`/data get block … energy` too), a
> 2,000,000,000 FE payload clamps to the tier's 20,000, a furnator charges its charge slot at exactly
> `min(transfer, stored, room)` out of its own buffer (measured against the game clock: `+480` over
> `dt=6`), a cell splits one 1,000 FE budget over two slots, and a stone in the charge slot takes
> nothing.
> **0.2.6 (slice 6, the Furnator and Energy Cable ladders)** is recorded in
> [`notes.md`](notes.md#slice-6--the-furnator-and-energy-cable-tier-ladders-2026-11-21): the
> slice and its gap line (the package's own "not yet ported: the other six furnator and
> cable tiers"), the red proof (four `todo!()` functions: `furnator_tier`, `cable_tier`,
> `furnator_of`, `cable_of`; 28 passed / 20 failed), the ladder at pin `0a875da` (48 tests,
> clippy 0 warnings, `fmt --check` clean, two byte-identical builds of 293,821 B, `check`,
> `fits` against the pin **and** against `0b14f8a`, gate 1/1 with **422 entries**, dialect
> 132/132) and the in-game legs (main `PASS`+`EXIT:0`, control / `SELFTEST=furnator` /
> `SELFTEST=furnator-rate` at `EXIT:1`, `check-e2e-logs --strict` 14/14). Package:
> `powah.hmod`, 293,821 B, sha256 `3ab1b5b46df0285f14f5d8fd47367e1cd053cbf96e671bfe8e3bb2eca30ed358`.
> Launcher build: `Compiling powah (1 of 1)`, `native-0a875da-9952c6153ae1`.
> **0.2.5 (slice 5, the Energizing Orb, its seven rods and the materials they make)** is recorded in
> [`notes.md`](notes.md#slice-5--the-energizing-orb-the-rod-ladder-and-the-materials-2026-11-21):
> the slice and its gap line (slice 4's five `blocked(energizing-orb)` cells), the red proof
> (seven `todo!()` tests, 34 passed / 7 failed, plus a second red for the orb's own rule at
> 39 passed / 3 failed), the ladder at pin `0a875da` (42 tests, clippy 0 warnings, `fmt --check`
> clean, two byte-identical builds of 210,786 B, `check`, `fits` against the pin **and** against
> `0b14f8a`, gate 1/1 with **292 entries**) and the in-game legs (main `PASS`+`EXIT:0`, control /
> `SELFTEST=1` / `SELFTEST=orb` at `EXIT:1`, `check-e2e-logs --strict` 12/12). Package:
> `powah.hmod`, 210,853 B, sha256 `7aef65e337192bb0caff50946ceb889e5f40bfcc10892654e766d46d78e697c3`.
> **0.2.4 (slice 4, the energy cell ladder)** is recorded in
> [`notes.md`](notes.md#slice-4--the-energy-cell-ladder-2026-11-21): the slice and its gap line, the red
> proof (three `todo!()` tests, 28 passed / 3 failed, plus the 0.2.3 package failing at the first new
> tier), the ladder at pin `0a875da` (32 tests, clippy 0 warnings, `fmt --check` clean, two
> byte-identical builds of 119,200 B, `check`, `fits` against the pin **and** against `0b14f8a`, gate
> 1/1 with 134 entries) and the in-game legs (main `PASS`+`EXIT:0`, control / `SELFTEST=1` /
> `SELFTEST=cells` at `EXIT:1`). Package: `powah.hmod`, 119,200 B, sha256
> `663465ee8b93e5f8be371537f6515bd7252651e462a57f477faedd82ffcd3e34`.
> **0.2.3 (slice 3, the cable network)** is recorded in [`notes.md`](notes.md#slice-3--the-cable-network-2026-11-21):
> the slice and its gap line, the red proof (five `todo!()` tests, 23 passed / 5 failed), the
> ladder at pin `0a875da` (28 tests, clippy 0 warnings, `fmt --check` clean, two byte-identical
> builds of 85,383 B, `check`, `fits` against the pin **and** against `0b14f8a`, gate 1/1) and the
> in-game legs (main `PASS`+`EXIT:0`, control / `SELFTEST=network` / `SELFTEST=1` at `EXIT:1`,
> the 0.2.2 build red at `EXIT:1`, and `check-e2e-logs.py --strict` 9/9).
> **0.2.2 (slice 2, the dielectric/capacitor family) and 0.2.1 (the review fixes) are recorded in
> [`notes.md`](notes.md)**: [`## Slice 2 …`](notes.md#slice-2--the-dielectriccapacitor-crafting-family-2026-11-20)
> carries its ladder, its in-game legs and the coverage table, and
> [`## Verification (0.2.1)`](notes.md#verification-021) the review-fix run.
> **0.2.0 (the SDK package)** is under
> [`## Migration to the SDK format`](notes.md#migration-to-the-sdk-format): its ladder
> (`cargo test` 22 passed, clippy 0 warnings, `fmt --check` clean, `cargo horizon-mod
> build/check/fits`, two byte-identical builds) and its logs (`sdk-*.log`) are there.
> Everything below is the **0.1.0** ladder of the retired `.hnmod`, kept as history: those
> commands (`tools/horizon.py`, `tools/package.py`, `--dev-mod`) no longer exist in this port.

Every check below was run in this session on the campaign pin
**`1d63e0a5a37a7a708eb0b32b49bf0dfeb28ed8a9`** / mixins
**`5a64af938ec86b00d53d16a408aabc0038fb3e57`** (schema 6), through
`ports/tools/build-slot.sh` with `CARGO_INCREMENTAL=0`. Logs:
`ports/_shared/evidence/powah/`.

| Check | Command | Result |
|---|---|---|
| format | `cargo fmt --all -- --check` + `rustfmt --check mixins/powah/*.rs` | clean |
| lints | `cargo clippy --all-targets --locked --offline -- -D warnings` | exit 0, 0 warnings |
| runtime tests | `cargo test --locked --offline` | **14 passed**, 0 failed (written first: 11 failed on the stubs) |
| staged check | `python3 tools/horizon.py check --horizon …/pins/horizon-1d63e0a` | `cargo_exit: 0`, **0 warnings** |
| embedded tests | `python3 tools/horizon.py test …` | `14 passed; 0 failed; 1240 filtered out` |
| fast build | `python3 tools/horizon.py build …` | `cargo_exit: 0` |
| e2e (one delivered file) | `EMBEDDED=1 tools/e2e.sh` on the port fast game, no `--dev-mod` | **PASS**, `EXIT:0` (content from the `.hnmod`) |
| fail-capability | `env EMBEDDED=1 SELFTEST=1 tools/e2e.sh` | exit **1** at the flipped assertion |
| content package | `cargo horizon-mod build` + `check` | 39,262 B, `7d344174…`, 42 entries, content only |
| native package | `tools/package.sh` (content → `data/` → `.hnmod`) | 33,899 B, `0030419c…`, 15 entries incl. the embedded content |
| load-check + negative | `tools/hmod-load.sh` | **PASS**; without `--dev-mod` the ids are unknown |

## Template tools not applicable at this pin

The scaffold ships two fixture tools from the native-mod template. Both are
written for the template's example bundle and its old `ports/horizon` layout, so
they do not validate a real named-hook port here; the real ladder above is the
evidence:

- `python3 tools/test_template.py` → `E_HOOK_CATALOG`: its tiny scratch
  workspace has no `native-hooks.json`, and a real bundle's injections name
  catalog hooks (`block_entity`, …). Its fixture also expects exactly one
  injection and one module.
- `python3 tools/test_stage.py` → `git -C ports/horizon status` fails: this
  campaign's pinned checkout is `ports/_shared/pins/horizon-1d63e0a`.

Both files are kept unmodified so the scaffold stays comparable with
`ports/_template`.

## What the e2e proves, and what it cannot

The run proves the in-game chain (fuel → furnator FE → cable → cell FE), the
controls (no fuel, no cable, full cell) and a cell discharging through a cable,
with a `ctl` transcript in `e2e.log`. It does **not** pin the exact rates: a
chat round trip in this environment is ~2 s (≈40 ticks), far longer than a tick,
so the numeric assertions are bounds and the unit tests carry the exact numbers
(20 FE/t, 30 carbon per burn tick, 48,000 FE per coal, 500/1,000/80 transfers).
