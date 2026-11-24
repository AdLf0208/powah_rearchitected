# Powah → Horizon — slices 1–3: Furnator, Energy Cell, the Energy Cable network, the dielectric family

Ported from **Powah 6.2.10** (`Technici4n/Powah`, **LGPL-3.0-only**), the exact
ATM10 NeoForge 1.21.1 jar `Powah-6.2.10.jar` (Modrinth SHA-1
`f134ab3e0ace3793abf93129e318eb817898a4e1`, byte-identical to the pack's
`modlist.json` entry; provenance, sizes and hashes in
[`ports/_shared/carry-work/powah/SOURCE.md`](../_shared/carry-work/powah/SOURCE.md)).
Upstream behaviour was read from tag `v6.2.10` (commit
`a5a8f25b5bafb58b694dc44ef0e4c0f14d9e4c11`) — analysis only, this is a
clean-room Rust rewrite. What was read and the numbers extracted:
[`FINDINGS.md`](FINDINGS.md).

**0.2.0 — the SDK package (2026-11-18).** Horizon's mod-API merge retired the `.hnmod`
container, so this port is now **one flat `.hmod`** (mod API 0.2): the carried content
(`assets/`, `data/`) and the Rust (`src/`) ship together, and closed PR **#806** is
superseded. `src/runtime.rs` is **byte-identical** to the pre-migration
`mixins/powah/runtime.rs` (sha256
`91df374d5b925d94fdefe23c4f8b01f164a481cb783b510dcfc50df2284921dc`, including the file inside
the built package). [`## Migration to the SDK format`](#migration-to-the-sdk-format) has the
file map, the injection → shape table, the gaps, the measured ladder and the traps; the retired
tree is `ports/_shared/evidence/powah/retired-hnmod-0.1.0/`. Everything below the migration
section is the 0.1.0 record and stays as the behaviour's history.

**Licence: LGPL-3.0-only → rank 1 (copyleft, publishable).** A port may be
redistributed under the same licence with its complete corresponding source;
the campaign still keeps every port local-only until the maintainer approves
publishing (ARCHITECTURE.md §8). Ported now because it is the next tractable
`content/machines` core mod at licence rank 1 with no unported dependency
(internally it needs nothing but Horizon's own API).

**Class + tier:** `content` · **machines** (`ports/ledger.csv`, mirrored from
`ports/_shared/source-map/atm10-sources.csv`). Approach per the pin's
`docs/modpacks.md` ("What packs need and what runs today"): a **machines**
shape — block entities with ticks, a declared energy capability (API 142), a
container and a command — not a data-only carry. The mod's own tier is
`machines`; nothing here is `deep` or `client`.

## Slice 1 scope

The **starter tier** of the energy chain, one of each:

- **Furnator** (`powah:furnator_starter`): burns furnace fuel into a carbon
  buffer and produces FE into a 20,000 FE buffer;
- **Energy Cell** (`powah:energy_cell_starter`): a 1,000,000 FE capacitor that
  takes and gives FE on every side;
- **Energy Cable** (`powah:energy_cable_starter`): moves FE between the block
  entities beside it, 500 FE/t, storing nothing.

Everything else in Powah (other tiers, creative cell, Magmator, Thermo, Solar,
Reactors, Ender Cells/Gates, Energizing Orb/Rods, Discharger, Hopper, Player
Transmitter, upgrades, GUIs, side config, worldgen) is a recorded gap below.

## Behaviour (upstream → this port)

| Machine | Upstream | This port |
|---|---|---|
| **Furnator** | `AbstractEnergyProvider`: buffer `generatorCapacity` 20,000, transfer 80 a side, generation 20 FE/t | same constants; `runtime::Furnator` |
| fuel | `carbon = burnTime * energy_per_fuel_tick` (**30**), one item at a time, only while carbon is empty; a crafting remainder replaces the stack | same (`load_fuel`, `consume_fuel`, including the remainder — a lava bucket leaves a bucket); the burn time is the engine's `minecraft:cooking_fuel` (`Providers::burn_time`), so modded fuels work. The remainder is not exercised by the e2e |
| production | `min(space, min(generation, carbon))`, and nothing while the buffer is full (carbon untouched) | same (`Furnator::tick`), unit-tested; one coal = 48,000 carbon = 2,400 ticks × 20 FE = **48,000 FE** |
| output | `extractFromSides`: `min(transfer, stored)` into every side that takes it | same, at `level_tick_post` (`push_out`) |
| slot | 2 (0 chargeables, 1 fuel) | 1 (fuel at index 0): charging items needs `item_energy_buffer`, deferred — a recorded deviation |
| **Energy Cell** | `AbstractEnergyStorage`: capacity 1,000,000, transfer 1,000 a side, `Transfer.ALL` | same; the buffer is API 142 (`energy` in the BE's compound) |
| cell tick | push to every side + charge its 2 item slots | push only (no item charging, no slots) |
| **Cable** | no storage (capacity 0); `receiveEnergy` forwards an insertion through the same-tier network, round-robin, skipping the side it came from | no buffer at all; on its tick it moves FE straight from each adjacent giver into each adjacent taker, up to 500 FE/t, with the round-robin start rotating by the game time (`runtime::cable_moves`) |
| cable sides | a side joins a neighbour that is not a cable and has FE | the same rule via `native_energy`; the six visual sides come from the content tag |
| redstone | `checkRedstone()` — default mode `IGNORE`, so always on | slice 13: the three modes, saved as `redstone_mode` and read once per tick; `/powah redstone` replaces the button |
| comparator | `hasAnalogOutputSignal` from the FE fraction | computed and printed by the command, and answered through `has_analog_output`/`analog_output` since slice 14; the orb's stack-count rule and the rods are still `not carried` |
| keep energy | `keepEnergy()`: the block item carries the buffer | a broken machine loses its FE — gap |

## Hooks (0.1.0, kept as history)

*The retired bundle's wiring. The 0.2.0 mapping is the injection → shape table in
[`## Migration to the SDK format`](#migration-to-the-sdk-format); these injections and helper
files no longer exist.*

| Hook (API) | Helper | What it does |
|---|---|---|
| `block_entity` (15) | `block_entity.rs` | The three blocks get a block entity. |
| `block_entity_ticks` (15) | `ticks.rs` | All three tick on the server. |
| `block_entity_container` (16) | `container.rs` | The furnator's one fuel slot. |
| `block_entity_energy` (142) | `energy.rs` | Furnator 20,000 / 80 out; cell 1,000,000 / 1,000 both ways; the cable none. |
| `block_entity_tick` (17) | `tick.rs` | The furnator burns; every block queues its position for the move phase. |
| `level_tick_post` (23) | `level_tick_post.rs` | The queued moves (`native_move_energy`): a block is out of the world during its own tick. |
| `register_commands` (8) | `register.rs` | `/powah info`, `/powah set`. |
| `perform_command` (8) | `perform.rs` | The command bodies. |
| `world_lifecycle` (v1) | `world.rs` | Drop the queued positions per world. |

Two pin facts that shaped the design (both recorded by earlier ports and
re-confirmed here):

- API **142** gives a real `Capabilities.Energy.BLOCK` buffer whose amount is
  the int **`energy`** in the BE's saved compound, with `native_energy`,
  `native_energy_insert`/`native_energy_extract` and `native_move_energy`
  (`crates/horizon-client/src/game/machines/native_energy.rs:136,160,174,190`).
  This is what makes the cell a real FE store for other mods.
- `native_energy_at` refuses a block entity **that is ticking at that moment**
  (`native_energy.rs:61-78`), so cross-block moves run at `level_tick_post`,
  where every entity is back in the world.

## Command (harness addition)

`/powah info <x> <y> <z>` prints the block's state, and `/powah set <amount> <x>
<y> <z>` sets a furnator's or a cell's stored FE. They exist because slice 1
has no GUI and no in-game FE readout:

```
Furnator at 0,100,0: 0/20000 FE (comparator 0, carbon 46780, burning true, fuel minecraft:coal x4, t=256)
Energy Cell at 2,100,0: 1380/1000000 FE (comparator 0, t=264)
Energy Cable at 1,100,0: no storage, moves up to 500 FE/t (t=188)
```

`t=` is the world game time, so the e2e can tell a fresh reply from a stale one.
`set` is ungated, like `mekanism energy`'s harness control; it is recorded here
as an addition, not upstream gameplay. Upstream's commands are its config/GUI
GUI actions and the guide book; none are ported.

## Content (`.hmod`)

*0.1.0 assembled this into `content/` with `tools/assemble_content.py`; since 0.2.0 the same files
are the package's `assets/` and `data/`, moved with their paths, sizes and hashes unchanged, and
the script is retired with the rest of the old tooling. The patch table below is still the record
of what the carry got wrong; [`CONTENT.md`](CONTENT.md) is the current summary.*

Carried byte-for-byte: the three blockstates, the six models (three variants +
the `furnator`/`energy_cell`/`cable`/`cable_multipart` parents), item models and
26.3 `items/` definitions, seven textures, `en_us.json`, the three loot tables,
the furnator/cell recipes and advancements.

Patched, because the carry cannot get them right (each documented in
`content/README.md`):

| File | Patch |
|---|---|
| `horizon/block/furnator_starter.json` | the carry guessed `minecraft:carved_pumpkin`; the real block is `AbstractGeneratorBlock` with a horizontal `facing` → `minecraft:furnace`, 2.0/20.0, metal sounds, `occludes: false`, stack 1 |
| `horizon/block/energy_cell_starter.json` | the carry guessed `minecraft:stone` (no states); → `minecraft:iron_block` with the upstream numbers, `occludes: false`, stack 1 |
| `horizon/block/energy_cable_starter.json` | the carry **left the cable out** ("no template for properties [down,east,north,south,up,west]") → `minecraft:chorus_plant` + `connects_to: #powah:energy_cable_connects` + `thickness: 4`, empty collision, `occludes: false` |
| `blockstates/furnator_starter.json` | `lollipop:block/empty` (another mod's model) for `facing=up/down`, which the furnace template does not have; now 4 facings × `lit` (never set) |
| `models/block/energy_cell.json` | parent `block` (1.21.1's `models/block.json`) → `block/block`, which 26.3 has |
| `data/minecraft/tags/block/{mineable/pickaxe,needs_iron_tool}.json` | the carry lists all 92 blocks; only the three exist |
| `data/powah/tags/block/energy_cable_connects.json` | new: upstream joins any FE block; a content tag can only name blocks, so the slice's three |
| `data/powah/recipe/crafting/cable_starter.json` + its advancement | the carry skipped them with the block; copied from the jar in the carry's upgraded shape (12 cables) |
| `horizon/creative_tab/machines.json` | upstream's tab is code; one tab with the three items |

The three recipes ship but **do not craft yet**: their ingredients
(`powah:capacitor_basic_tiny`, `powah:dielectric_casing`,
`powah:dielectric_paste`, `powah:dielectric_rod_horizontal`) are later slices.

## Verification (0.1.0, the retired `.hnmod` — historical)

*Everything in this section is the retired bundle's record. The 0.2.0 ladder is in
[`## Migration to the SDK format`](#migration-to-the-sdk-format); the logs it names stay under
`ports/_shared/evidence/powah/` as history and are not the current artifact's evidence.*

Pin **`1d63e0a5a37a7a708eb0b32b49bf0dfeb28ed8a9`**, mixins
**`5a64af938ec86b00d53d16a408aabc0038fb3e57`** (schema 6); checkouts
`ports/_shared/pins/horizon-1d63e0a` and `.../horizon-mixins-5a64af9`. Every
build ran through `ports/tools/build-slot.sh` with `CARGO_INCREMENTAL=0`, never
a bare cargo; the e2e used the pin-matched shared `horizon-ctl`
(`ports/_shared/bin/horizon-1d63e0a5/horizon-ctl`) and the port's own fast game.

### Ladder

| Step | Command | Result |
|---|---|---|
| format | `cargo fmt` (through the same rustfmt the ladder uses) | clean |
| runtime tests | `cargo test --locked --offline` (standalone lib) | **14 passed**, 0 failed; TDD: the same 14 were watched failing first (11 failed, 3 "nothing happens" assertions trivially held on stubs) |
| staged check | `python3 tools/horizon.py check --horizon …/pins/horizon-1d63e0a` | `cargo_exit: 0`, **0 warnings** |
| embedded tests | `python3 tools/horizon.py test …` | `14 passed; 0 failed; 1240 filtered out` |
| fast build | `python3 tools/horizon.py build …` | `cargo_exit: 0`, binary in the slot target |
| e2e (one delivered file) | `EMBEDDED=1 tools/e2e.sh` on the port's fast game, `--dev-mod` omitted | **PASS**, `EXIT:0`; the content came from the `.hnmod`'s embedded copy, not `content/` |
| fail-capability | `env EMBEDDED=1 SELFTEST=1 tools/e2e.sh` | **exit 1** at the flipped no-fuel assertion |
| content `.hmod` | `cargo horizon-mod build` + `check` | **39,262 B**, SHA-256 `7d344174…26c1`, **42 entries**, content only, API 0.2, `side = both` |
| native `.hnmod` | `tools/package.sh` (content → `mixins/powah/data/powah-content.hmod` → `.hnmod`) | **33,899 B**, SHA-256 `0030419c…a9cdd`, **15 entries**, including the embedded content |
| load-check | `tools/hmod-load.sh` (shared 1d63e0a5 game) | **PASS** with `--dev-mod <the .hmod>` (three blocks + three items); without it the same ids are **unknown** (`Unknown block type 'powah:furnator_starter'`, block stays bedrock) |

Evidence: `ports/_shared/evidence/powah/{e2e,e2e-selftest,hmod-load,horizon-check,horizon-test,horizon-build}.log`.

**One delivered file.** The registry stores one file per project, so the built
content mod is carried inside the bundle at
`mixins/powah/data/powah-content.hmod` and the `embedded-content` injection at
`content_root` writes it to `.horizon-cache/` and appends it to the dev-mod list
(the AE2 pattern, `docs/native-mods.md` "Data files"). The e2e above runs with
no `--dev-mod`, so it proves the delivered `.hnmod` installs both halves;
`content/powah.hmod` stays as the content input and as the standalone
`hmod-load.sh` check.

### In-game scenario (quoted from `e2e.log`)

```
  arena: y=100.0 floor(0,99,3)=minecraft:stone feet(0,100,3)=minecraft:air
  block(0,100,0) = powah:furnator_starter
  block(1,100,0) = powah:energy_cable_starter
  block(2,100,0) = powah:energy_cell_starter
  furnator: Furnator at 0,100,0: 0/20000 FE (comparator 0, carbon 0, burning false, fuel empty, t=150)
  after 40 ticks: Furnator at 0,100,0: 0/20000 FE (comparator 0, carbon 0, burning false, fuel empty, t=199)
  has(0,100,0) minecraft:coal x4: ok
  has(0,100,0) minecraft:coal x4: ok
  furnator: Furnator at 0,100,0: 0/20000 FE (comparator 0, carbon 46800, burning true, fuel minecraft:coal x4, t=282)
  cell: Energy Cell at 2,100,0: 1360/1000000 FE (comparator 0, t=290)
  carbon 47800 -> 46800, cell 1360 FE after 40 burning ticks
  set(2,100,0) = Energy Cell at 2,100,0: 0/1000000 FE (comparator 0, t=305)
  no cable: furnator 320 FE, cell Energy Cell at 2,100,0: 0/1000000 FE (comparator 0, t=363)
  set(2,100,0) = Energy Cell at 2,100,0: 1000000/1000000 FE (comparator 15, t=380)
  after 40 ticks: Energy Cell at 2,100,0: 1000000/1000000 FE (comparator 15, t=429)
  set(2,100,0) = Energy Cell at 2,100,0: 0/1000000 FE (comparator 0, t=438)
  block(1,100,0) = powah:energy_cable_starter
  cell: Energy Cell at 2,100,0: 3920/1000000 FE (comparator 0, t=494)
  set(2,100,0) = Energy Cell at 2,100,0: 1000/1000000 FE (comparator 0, t=509)
  block(2,100,1) = powah:energy_cable_starter
  block(2,100,2) = powah:energy_cell_starter
  source: Energy Cell at 2,100,0: 500/1000000 FE (comparator 0, t=572)
  sink:   Energy Cell at 2,100,2: 500/1000000 FE (comparator 0, t=580)
PASS: a fuelled furnator made FE, the cable carried it into the cell, an unfuelled furnator made nothing, a cell without a cable stayed empty, a full cell stayed full, and a charged cell drained through a cable into an empty one
EXIT:0
```

The run's controls: a **no-fuel** furnator stores 0 FE for 40 ticks (and the
cell beside it stays 0); the same furnator with the **cable broken** holds its
FE while the cell stays 0 (so the cable is what moves it); a **full cell**
stays at 1,000,000 FE while the furnator keeps burning; a charged cell is
drained through a cable into a second cell. `SELFTEST=1` flips the no-fuel
assertion and the run fails at it (`FAIL: SELFTEST flipped: the unfuelled
furnator made no FE, as it should`, exit 1), so the scenario can fail. The
48,000-FE-per-coal figure is pinned by the unit test, not by the e2e (the chat
round trip in this environment is ~2 s, far longer than a tick).

## Deviations and gaps (slice 1)

1. **No GUI.** Upstream opens a container screen for the furnator/cell/cable;
   this slice inserts fuel through the container (automation's path, what the
   e2e uses) and reads state with `/powah info`.
2. **No cable network** — **closed by slice 3** (`## Slice 3 — the cable network`); the
   slice's gap line, kept verbatim: Upstream's cables share one buffer across a connected
   network and forward an insertion through it in one call. The port moves FE
   one cable hop per tick between *adjacent block entities*, so a run of two or
   more cables does not relay. The e2e's chain is exactly one cable (the slice's
   brief).
3. **Two adjacent cells swap energy.** Both push on every side by default (as
   upstream's `Transfer.ALL` does), so a pair exchanges its charge each tick and
   the later tick holds it. The port is faithful; upstream's answer is the side
   config GUI, which is not ported. The e2e avoids the shape (cell → cable →
   cell).
4. **No item charging.** Upstream's furnator charges slot 0 and a cell charges
   two slots through `item_energy_buffer`; not ported, so the furnator has one
   slot, not two, and the cell none. — **closed by slice 7**
   ([`## Slice 7`](#slice-7--the-item-energy-family-keepenergy-and-the-item-charge-slots-2026-11-22)):
   the furnator has upstream's two slots (charge 0, fuel 1) and a cell its two, and the per-tick step
   is `min(transfer, stored, room)` out of the machine's own buffer.
5. **Breaking a machine loses its FE.** Upstream's `keepEnergy()` makes the
   block item carry the buffer; no energy component/item buffer is registered
   here, and the loot tables drop the plain block. — **closed by slice 7**
   ([`## Slice 7`](#slice-7--the-item-energy-family-keepenergy-and-the-item-charge-slots-2026-11-22)):
   every furnator, cell and rod carries its buffer as `minecraft:block_entity_data` `{id, energy}` and
   gives it back on placement, clamped to its tier.
6. **No side config or redstone modes.** Upstream's six per-side transfer types
   and redstone IGNORE/ON/OFF default to `ALL`/`IGNORE`; this slice ships only
   those defaults. — the **redstone** half is **closed by slice 13**
   ([`## Slice 13`](#slice-13--the-redstone-mode-abstracttileentitycheckredstone-2026-11-23)):
   the mode saves as `redstone_mode`, one rule reads it in every machine's own tick, and
   `/powah redstone <mode> <pos>` is the GUI button's replacement. The **side config** stays open:
   a `ModEnergyBuffer` declares six face booleans, not six per-side transfer types.
7. **No comparator output.** `comparator_strength` is computed and printed, but
   no `has_analog_output` hook is declared, so a vanilla comparator beside a
   machine still reads container fullness. — **closed by slice 14**
   ([`## Slice 14 — the comparator reading`](#slice-14--the-comparator-reading-analog_output-2026-11-23)):
   every machine whose readout prints a comparator answers `has_analog_output`/`analog_output`
   with that number; a cable, the orb and the rods answer none (recorded in that slice's
   deviations).
8. **`lit`/`burning` is not a block state** (upstream's `burning` is a client
   sync flag for particles and sound), so nothing is drawn differently while
   burning, and no particles/sound play.
9. **No item count/stack parity for the cable's visual sides**: the content tag
   names the slice's three blocks; upstream also draws arms to any other mod's
   FE block.
10. **Crafting is not possible yet**: the recipes need later-slice items.
11. **The other 6 tiers, the creative cell and ~15 machines** are unported. — the
    **energy cell** tiers are closed by [`## Slice 4 — the energy cell ladder`](#slice-4--the-energy-cell-ladder-2026-11-21)
    and the **furnator and cable** tiers by [`## Slice 6 — the Furnator and Energy Cable tier
    ladders`](#slice-6--the-furnator-and-energy-cable-tier-ladders-2026-11-21); the **discharger** by
    slice 8 and the **hopper** by slice 9; the creative cell, the generators, the reactors (which are
    `blocked(uraninite-worldgen)`), the ender gates, the player transmitter and the batteries remain.
12. **`/powah set` is a harness control**, not upstream gameplay (same precedent
    as `/mekanism energy`).

## Pin watch (2026-11-09 ruling)

`python3 ports/tools/check-pin-updates.py` reports the pin **moved** (Horizon
`7d0f8b956b09`, 2,439 commits ahead; catalog +2 hooks `world_hold`,
`world_warp`, +3 chunk helpers). None of them touches this slice's surface
(API 8/15/16/17/23/142 are unchanged), and the brief pins `1d63e0a`, so the
port stays on it; the repin decision is the orchestrator's.

## Harness findings (for the next machine port)

- `NativeCommand` argument kinds are `word`/`text`/`integer`/`number`/
  `boolean`/`player`/`item`/`block_pos` (`docs/native-mods.md:2173`); `int` is
  refused with "unknown kind `int`" and the whole usage is dropped.
- The container path (`ctl container X Y Z item n`) inserts into `inputs` on
  the next tick, and the furnator consumes the first fuel item immediately, so
  read `n-1` back, not `n`.
- Two mod block entities tick in the same level tick; a cross-block energy move
  must be queued per position and run at `level_tick_post` — a per-tick read of
  a neighbour inside a BE tick returns nothing (`native_energy_at` refuses the
  ticking entity).
- `ctl chat` replies take ~2 s (~40 ticks) in this environment; assert on a
  fresh `t=` field, and for a value you set, poll until it reads back.
- The `.hmod` load-check must launch the **shared unmodded game** for the
  negative leg; the port's own build already has the native half.
- A model parent of `block` (1.21.1's `models/block.json`) is missing in 26.3
  and logs "missing block model minecraft:block"; use `block/block`.

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices

## Review repairs (PR #554)

The reviewer's blocker is staging (the pinned artifact absent from the registry
release/staging), which is the orchestrator's (horizon-mods#686). The licence is
LGPL-3.0-only, the original's, with the complete corresponding source in this
repository — the registry policy accepts copyleft; the reviewer's inability to
read LGPL source under CONTRIBUTING is a reviewer-permission matter, not a
listing defect. This touch adds the notices to the artifact
(`mixins/powah/LICENSE` and `NOTICE.md`, and the same at the embedded content
root) and repackages it.

### Verification after the repairs

- Artifact: `powah.hnmod`, 40,622 B, SHA-256 `2dabb27b01f31e201903fd5f55d42d86d92aed816d3967701d23e06f134d2b42` (rebuilt twice, identical bytes; `tools/package.py`; the embedded content equals `content/powah.hmod`).
- `upstream_ready.py powah --copyleft` exits 0 (READY, LGPL-3.0-only). `check-skills.py --notes ports/powah` exits 0.

## Migration to the SDK format

*Recipe followed from `ports/imfast/notes.md` → `## Migration to the SDK format` (the first
migration), with `ports/charginggadgets/notes.md` (a block entity, a container, an energy buffer,
typed commands) and `ports/laserio/notes.md` (the per-tick queue drained at `level_tick_post`)
for the parts this port shares. 2026-11-18. Version **0.1.0 → 0.2.0**; **PR #806** (the
`.hnmod`) is superseded.*

**Why.** Horizon's mod-API merge (#16941, launcher ≥ 0.2.70, PR #17992) retired the `.hnmod`
container, and the registry closed every `.hnmod` submission on 2026-11-18 with one reason:
*"republish this as a native `.hmod` with the SDK from the next Nightly"*. The unit is **one flat
`.hmod`** — content, native code, or both — so this port no longer needs the AE2 embedded-content
trick, its `content_root` injection or its `.horizon-cache` write: `assets/` and `data/` sit
beside `src/` in the same package.

**Pins.** SDK nightly **`0b14f8acfd97fa2c750e42795249dcc981178ca4`** (surface + `api-source`
cached at `ports/_shared/sdk/0b14f8a/`, copied into `.horizon/sdk/` for the offline builds),
mixins **`e59da9094f0c2fdda6c627fe9eff22dec6a906f0`** (was `5a64af93…`), `package_format` 6,
`content_api` 0.2. The old campaign pin `1d63e0a` does not apply to a package build.

**The retired tree was moved before the first build** (trap 1): `mixins/`, `powah.hnmod`,
`compatibility.json`, `rust-toolchain.toml`, the port's old `AGENTS.md` (it told the reader to
run `tools/horizon.py` and read `compatibility.json`), `.github/`, `content/` (its
`horizon-mod.toml`, `powah.hmod`, `README.md` and `target/`) and the eight retired tools
(`horizon.py`, `package.py`, `package.sh`, `assemble_content.py`, `add-notices.py`,
`test_stage.py`, `test_template.py`, `hmod-load.sh`, `tools/audit/`) are in
`ports/_shared/evidence/powah/retired-hnmod-0.1.0/`. `Cargo.lock` was deleted, not kept.

### File map (0.2.0)

| 0.1.0 | 0.2.0 | Evidence |
|---|---|---|
| `mixins/powah/runtime.rs` | `src/runtime.rs`, copied with `cp` and proved with `cmp` | retired copy kept; all three copies sha256 `91df374d…21dc` |
| `mixins/powah/support.rs` (`powah_state`, 451 lines) | `src/powah.rs` — the same read/write/push/relay/readout logic over `&mut dyn ServerHost` | — |
| `mixins/powah/horizon-mixin.toml` (2 modules, 10 injections) | no plan: `Registrar` declarations, two Part types and four `Hooks` methods | retired |
| `mixins/powah/*.rs` (10 helper files) | the `Hooks`/Part methods and the declarations below; no per-hook files | retired |
| `content/assets/`, `content/data/` | `assets/`, `data/` at the package root | 41 files, byte-identical (`carried-content-sha256.txt`) |
| `content/horizon-mod.toml` | merged into the root `horizon-mod.toml` (`[native]` added; version 0.2.0) | retired |
| `content/powah.hmod` (39,262 B content carry) | not needed: the one package carries the content | retired |
| `mixins/powah/data/powah-content.hmod` + `embedded_content.rs` | gone: nothing to embed | retired |
| `compatibility.json`, `rust-toolchain.toml`, `horizon-native.json` | `[native]` in `horizon-mod.toml`; the pinned commit decides the toolchain | retired |
| `powah.hnmod` (40,622 B) | `powah.hmod` (68,403 B, native + content) | retired |
| `tools/horizon.py`, `package.*`, `assemble_content.py`, `hmod-load.sh`, `tools/audit/`, `.github/` | retired; the content carry is recorded in `CONTENT.md` | retired |

**Kept unchanged:** `FINDINGS.md`, `CONTENT.md`, `VALIDATION.md`, `LICENSE`,
`notes.md` (this section added), and `tools/e2e.sh`'s *scenario* — its assertions are
format-independent; only the launch line changed (a launcher-built `GAME=`, an optional
`CONTROL_GAME=`, no `--dev-mod`).

**New:** `horizon-mod.toml`, `Cargo.toml` (one crate, `edition 2024`, the only dependency
`horizon-mod-api` from `.horizon/sdk/`), `src/lib.rs`, `src/powah.rs`, `.gitignore`
(`/target`, `/.horizon`, `/Cargo.lock`), a rewritten `README.md`, an updated
`LICENSE-NOTICE.md`.

### Manifest mapping

| 0.1.0 (plan + content manifest) | 0.2.0 (`horizon-mod.toml`) |
|---|---|
| `[mod] id = "powah"` | `id = "powah"` (also the resource namespace) |
| `[mod] version = "0.1.0"` and content `6.2.10` | `version = "0.2.0"` — the port's version, one for the one package |
| `[mod] priority = 100` | gone; ordering would be `#[listener(priority = …)]` |
| `[compatibility] horizon_revision = "1d63e0a5…"` | `[native] horizon = "0b14f8ac…"` |
| mixins `5a64af938ec8…` (the bundle) | `[native] mixins = "e59da9094f0c…"` — **it moved** |
| game 26.3 (content manifest) | `[native] game = "26.3"` |
| `[[module]] runtime` + `state` | ordinary modules: `pub mod runtime;`, `mod powah;` |
| 10 `[[injection]]` | 4 `Hooks` methods + 3 `BlockEntityPart` methods + 3 declarations (table below) |
| — | `api = "0.2"`, `side = "both"`, `capabilities = ["registries"]` |

### Injection / helper → shape (every one mapped, none silently dropped)

| 0.1.0 injection (API) | 0.2.0 shape | Kept / gap |
|---|---|---|
| `embedded-content` (`content_root`, raw target, HEAD, cancellable) | — | **gone by design**, not a gap: one `.hmod` carries `assets/`, `data/` and `src/`, so there is nothing to write out or append to the dev list |
| `block-entity` (`block_entity` 15) | the three **types** declared in `Hooks::register`: `block_entity_type("furnator_starter"/"energy_cell_starter"/"energy_cable_starter").blocks(&[..]).build()` | **kept as a declaration**: a declared type starts with an empty compound, so no Part method is needed (`BlockEntityPart::block_entity` stays unset) |
| `block-entity-ticks` (15) | `BlockEntityPart::block_entity_ticks` on both Part types → `event.ticks = true` | **kept** |
| `block-entity-container` (16) | `Registrar::block_container(&furnator, container())` — 1 slot, every face in and out, `menu: None` | **kept** |
| `block-entity-energy` (142) | `Registrar::block_energy` for the furnator (20,000; insert 0; extract 80; extract faces only) and the cell (1,000,000; 1,000 both ways, all faces); **the cable declares none** | **kept**, keyed by the block instead of a shared `Option` |
| `block-entity-tick` (17) | `FurnatorPart::block_entity_tick` (load fuel, burn, write, queue) and `IdlePart::block_entity_tick` (queue only) | **kept**; the reply is `Unchanged` when nothing moved |
| `level-tick-post` (23) | `Hooks::level_tick_post` over the `LevelRef`-keyed queue: `push_out` for the furnator/cell, `relay` for the cable | **kept** |
| `register-commands` (8) | `Hooks::register_commands` with typed usages (`<pos:block_pos>`, `<amount:integer>`) | **kept** |
| `perform-command` (8) | `Hooks::run_mod_command` reading `literals` and the named arguments | **kept**, minus the free-text parsing (trap 9) |
| `world-lifecycle` (v1) | `Hooks::world_lifecycle` → `Queue::clear` | **kept** |

So: **9 named hooks + 1 raw target → 4 `Hooks` methods, 3 Part methods and 3 declarations; 10
mapped, 0 recorded as gaps.**

Host calls, 0.1.0 → 0.2.0:

| 0.1.0 (`Game<'_>`) | 0.2.0 |
|---|---|
| `game.res.providers().burn_time(items, stack, false)` | `host.item_burn_time(stack.item())` |
| `items().get(&stack.item).remainder` | `host.item_crafting_remainder(stack)` (then `ItemStack::shrink(1)`) |
| `game.native_energy(dimension, pos, side)` | `level.capability(host, &ENERGY, pos, Some(side))` (`amount`, `capacity`; the moves themselves are `insert`/`extract`) |
| `game.native_move_energy(..)` | `move_energy(host, &source, &target, max, tx)` in `host.transaction(..)` |
| `game.native_with_mod_block_entity(pos, f)` | `host.block_entity(level, pos)` + `BlockEntityRef::{data, item_slot, merge_data}` |
| `game.world.game_time` | `level.game_time(host)` |
| `game.world.modified.insert(chunk)` | nothing: the tick reply dirties the chunk, `Changed` only when something moved |
| `game.native_feedback(Text)` | `host.commands().feedback(Text)` |
| `game.native_dimension()` | the queue is keyed by `LevelRef` (Copy + Hash + Eq), so no dimension strings |
| `game.world.block_entities.get(&key)` | `host.block_entity(level, pos)` / `level.block(host, pos)?.id(host)` |

### Ladder measured for 0.2.0

Every step ran in `ports/powah`, `CARGO_INCREMENTAL=0`, inside a
`ports/tools/build-slot.sh run --job powah-sdk --port ports/powah --step … -- bash -c 'cd
ports/powah && …'` lease (the runner does not `cd`, and an SDK package has no staged tree). Logs in
`ports/_shared/evidence/powah/sdk-*.log`.

| Step | Command | Result |
|---|---|---|
| runtime carried over | `cp` + `cmp` + `sha256sum` | `91df374d…21dc` in the retired copy, `src/runtime.rs` and inside the package |
| runtime tests before any glue | `cargo test` with `src/lib.rs` = `pub mod runtime;` | **14 passed**, 0 failed (`sdk-runtime-first.log`) |
| the glue's tests can fail | 8 new tests, every body `todo!()` | **14 passed, 8 failed**, each `not yet implemented` (`sdk-glue-red.log`) |
| all tests | `cargo test` | **22 passed**, 0 failed — 14 runtime + 8 glue (`sdk-test.log`) |
| lints | `cargo clippy --all-targets -- -D warnings` | exit 0, **0 warnings** (`sdk-clippy.log`) |
| formatting | `cargo fmt --check` (never `--all`: trap 6) | exit 0, empty output (`sdk-fmt.log`) |
| package | `rm -f Cargo.lock; cargo horizon-mod build` | `target/horizon-mod/powah.hmod` (`sdk-build.log`) |
| determinism | `rm -f Cargo.lock; cargo horizon-mod build` again | **byte-identical**, same 68,403 B and sha256 (`sdk-build-second.log`) |
| a clean tree packs the same bytes | `rm -rf target`, then `rm -f Cargo.lock; cargo horizon-mod build` | same 68,403 B, same sha256 (`sdk-fresh-tree.log`) |
| registry check | `cargo horizon-mod check powah.hmod` | see the verbatim output below, **EXIT 0** (`sdk-check.log`) |
| fits the pinned build | `cargo horizon-mod fits powah.hmod --build nightly` | `Powah: 0b14f8a  made for`, **EXIT 0** (`sdk-fits.log`) |
| what it uses | the package's `uses.json` | **310** entries (`sdk-uses-summary.log`): 25 hook, 33 method, 24 trait, 146 type, 71 handle, 5 part, 2 capability, 2 macro, 2 fn |
| package contents | `unzip -l powah.hmod` | **50 entries**: 6 root documents (`horizon-mod.toml`, `Cargo.toml`, `LICENSE`, `LICENSE-NOTICE.md`, `README.md`, `uses.json`), 3 `src/`, 25 `assets/`, 16 `data/`; **no `Cargo.lock`** (`sdk-entries.log`) |
| carried content | `sha256sum` of `assets/` + `data/` before and after the move | **41 files, unchanged** (`carried-content-sha256.txt`) |

`cargo horizon-mod check powah.hmod`, verbatim:

```text
powah 0.2.0 (native-mod, format native-1/0b14f8acfd97fa2c750e42795249dcc981178ca4/e59da9094f0c2fdda6c627fe9eff22dec6a906f0, for 26.3)
  68403 bytes, SHA-256 11824c7cc17003caf71f1ee9eef63bf1fdfc81c1c2535b7fa54463ce3166455d, unsigned
  native source: unsandboxed; not executed by the registry check. Compile and test against the exact pinned Horizon and Mixins revisions before review.
```

`cargo horizon-mod fits powah.hmod --build nightly`, verbatim:

```text
Powah:
0b14f8a  made for
```

### Packages

| File | Size | SHA-256 | Entries |
|---|---:|---|---:|
| `powah.hmod` (0.2.0, the delivered package) | 68,403 B | `11824c7cc17003caf71f1ee9eef63bf1fdfc81c1c2535b7fa54463ce3166455d` | 50 |
| `powah.hnmod` (0.1.0, retired) | 40,622 B | `2dabb27b01f31e201903fd5f55d42d86d92aed816d3967701d23e06f134d2b42` | 13 |
| `content/powah.hmod` (0.1.0 content carry, retired) | 39,262 B | `7d344174…26c1` | 42 |

The manifest the tool packs is the one written (no field was stripped): the package's
`horizon-mod.toml` is byte-identical to the port's, comments included.

### Gaps this migration adds

1. **No in-game run at 0.2.0.** `horizon-launcher` is not installed here, so no `0b14f8a` game
   carrying this package exists (an SDK package is compiled **into** the client; there is no
   staging path). `tools/e2e.sh` was rewritten for a launcher-built `GAME=`, an optional
   `CONTROL_GAME=` (an unmodified build of the same version, where all three ids and `/powah`
   must be unknown) and the same fail-capable `SELFTEST=1`; it refuses to start without `GAME`
   (measured: `missing GAME: …`, exit 1) and was **not run**. The 0.1.0 runs remain the
   behaviour's evidence; the byte-identical `runtime.rs`, the same saved keys (`energy`,
   `carbon`), the same numbers and the same readout lines make it the same behaviour **except**
   what only a game can show: (a) a declared block entity type gives the furnator its one
   container slot and the energy capability reads the compound's `energy`; (b) a move between two
   block entities works at `level_tick_post`; (c) the cable's 500 FE/t budget and round-robin are
   unchanged; (d) `/powah info|set` print and act as before.
2. **`/powah set` no longer explains a bad amount.** `<amount:integer>` is parsed (and refused) by
   the dispatcher, so 0.1.0's *"The amount must be a whole number of FE."* line is gone.
3. **A command acts on the level in place.** `/powah info|set` read `host.level(host.dimension())`,
   as 0.1.0 read `game.world`; a command sent with `execute in <other dimension>` still acts on the
   ticked level. Single player and a LAN host are unaffected; recorded rather than worked around.
4. **A foreign FE block's per-call limits and faces are invisible.** `EnergyHandler` exposes
   `amount`/`capacity` and the moves only, so the cable's planner reads the limits back from this
   mod's own declaration and treats a foreign block as unbounded; its handler then caps the move
   (and `move_energy` retries with what it took, so nothing is lost). No other FE block exists in
   this slice, so nothing observable changes.
5. `/powah set` is operator-only since the 2026-11-18 review (see `## Review fixes`), as
   `laserio`'s mutating commands became at review; `info` too, because its fuel readout
   discloses another player's machine contents.
6. Breaking a machine still loses its FE (0.1.0 gap 5), and the slice boundary is unchanged: no
   GUI, no cable network, no item charging, no side config, no comparator hook, the other six
   tiers unported.

### Traps this migration hit (1–15 are the reference's; 16–25 are this port's)

1. **A leftover old path refuses the package.** The retired tree must be out of the port root
   **before** the first build; here it was `mixins/`, `content/`, `powah.hnmod`, the old
   `Cargo.lock` and the old tools.
2. A short pin reads as *missing*: both pins are 40 hex, and the **mixins commit moved**
   (`5a64af93…` → `e59da9094f0c…`).
3. `cargo horizon-mod check` with **no file** checks the wasm target; the package check is
   `check <file.hmod>`.
4. `cargo fmt --all` reaches into `.horizon/sdk/`; use `cargo fmt --check`.
5. `rm -f Cargo.lock` immediately before `cargo horizon-mod build` — any `cargo test`/`clippy`
   writes one and a packed lock changes the artifact.
6. `runtime.rs` must stay byte-identical (`cmp` **and** `unzip -p … src/runtime.rs | sha256sum`);
   `pub mod runtime;` is required or every `pub` item dead-codes.
7. `side = "both"` (an integrated server is part of the client installation) and
   `capabilities = ["registries"]` are mandatory values, not claims.
8. **The three types get two Part types, not three:** the furnator burns, the cell and the cable
   only queue. Nothing in the glue reads `event.entity.id()`: the attached Part decides what the
   tick does, and the move phase asks the **block** id, which is the content's own.
9. **`perform_command` is gone → `run_mod_command` with typed arguments.** The old
   `split_whitespace` parse (and its two usage lines) goes with it; the **readout lines stay
   byte-identical** so the 0.1.0 e2e assertions still match.
10. **`host.transaction` comes from `HostTransaction`**, not from `ServerHost` itself: without
    `use mod_api::HostTransaction;` the call is E0599 (`no method named transaction`).
11. **`move_energy` is the primitive, not a hand-rolled pair.** It extracts, inserts, and when the
    target takes only part it aborts that transaction and retries with the accepted amount — so a
    per-call cap (the furnator's 80, a cell's 1,000) never loses energy and nothing needs the
    insert-then-extract dance `bigreactors` does.
12. **The energy capability has no limit or face getters.** `EnergyHandler` is
    `amount`/`capacity`/`insert`/`extract`; the cable's planner needs `max_insert`,
    `max_extract`, `can_insert` and `can_extract`, so they come from the mod's own declaration
    (`battery_of`), the same table `Registrar::block_energy` uses. One table, so the declaration
    and the relay cannot disagree; a foreign block is planned as unbounded (its handler caps).
    Without this, a cable between a cell and a furnator would spend its budget planning a move
    *into* the furnator (which refuses inserts) and carry less than it should.
13. **A cable that stores nothing still needs a declared type and a tick.** The SDK cannot
    enumerate a level's block entities and a content block has no server tick, so the queue is the
    only way to find the cable — and the queue is keyed by `LevelRef`, not by a dimension string.
14. **`BlockEntityTickReply` arrives as `Changed`.** The dispatch sets `Changed` before each Part,
    so a Part that changed nothing must set `Unchanged` itself, or every machine dirties its chunk
    every tick (and clients get an update every tick).
15. **A declared type starts with an empty compound**, so `BlockEntityPart::block_entity` is not
    needed (`charginggadgets` returns `Some(Compound::new())` redundantly); the saved keys appear
    on the first tick.
16. **The old `Cargo.toml` cannot be carried over.** It had `[workspace] members = ["tools/audit"]`,
    `[lib] path = "mixins/…"` and a `serde_json` dependency — a path that no longer exists, a
    package path the tool refuses, and a registry dependency that would need
    `horizon-launcher native-lock`. The new crate has one dependency: the SDK.
17. **`cargo test` needs `.horizon/sdk/` before the first `cargo horizon-mod` command.** Copied
    from a port at the same pin (`cp -a ports/framedblocks/.horizon/sdk ports/powah/.horizon/sdk`);
    the SDK also fetches itself on an online build.
18. **`BlockEntityTypeBuilder::blocks` takes bare names.** `.blocks(&["furnator_starter"])` names
    `powah:furnator_starter`; an id without its namespace would silently attach nothing.

Skills: superpowers/using-superpowers,writing-plans,test-driven-development,systematic-debugging,verification-before-completion (the recipe is fixed and no design choice was open, so `brainstorming` did not apply to this migration; the 0.1.0 record above lists it); BeautifulRust/rust-best-practices,rust-port

### Changelog / supersede text (for the resubmission that replaces #806)

> **0.2.0 — republished as a native `.hmod` (the SDK format).** Horizon's mod-API merge retired
> the `.hnmod` container, so this release replaces the closed submission #806 with one flat
> `.hmod` that carries the starter Furnator, Energy Cell and Energy Cable — their blocks, items,
> models, textures, recipes, loot, tags and lang — **together with the code**. The embedded-content
> workaround of 0.1.0 (a second `.hmod` inside the bundle, written to `.horizon-cache` at start) is
> gone: one file installs both halves. Built against SDK nightly `0b14f8a` (mixins `e59da90`, api
> 0.2, format native-1). Behaviour is unchanged: a fuelled furnator burns furnace fuel into carbon
> and makes 20 FE/t into a 20,000 FE buffer (one coal is 48,000 FE), the energy cell is a
> 1,000,000 FE capacitor that takes and gives 1,000 FE a side, and the energy cable moves up to 500
> FE/t between the block entities beside it, storing nothing; the buffered machines declare a real
> `Capabilities.Energy.BLOCK` buffer that other machines can fill and drain, and `/powah info|set`
> print and set the same lines. What changed is the wiring: the ten injected helpers became four
> `Hooks` methods, three `BlockEntityPart` methods and three declarations (the block entity types,
> the furnator's container, the two energy buffers), and a declared buffer plus a typed command
> pair replace the old hooks' shared outputs. `src/runtime.rs` is byte-identical to 0.1.0 and keeps
> its 14 tests. Version `0.1.0` → `0.2.0`.

## Review fixes (2026-11-18)

Source review of PR **#1222** (`var/review/1222-powah.md`), both findings fixed in **0.2.1**.
Both are the same defect: the two harness usages are administrative — they act on any loaded
machine at coordinates the caller chooses — and 0.2.0 declared them with `Command::new` (level 0).

| # | Finding (`file:line`) | What a player could do | Fix | Test that pins it |
|---|---|---|---|---|
| 1 | `src/lib.rs:121` — `set` mutates any loaded cell/furnator | a non-operator overwrote a machine's FE at any coordinates in the level: create energy without fuel, drain another player's machine | `powah::declare_commands` registers `powah set <amount> <pos>` with `Command::op(…, 2)`; the server's dispatcher refuses it below game-master permission, before any listener runs | `powah::tests::the_harness_commands_are_operator_only` (`src/powah.rs`) — RED on the 0.2.0 registration (`left: 0, right: 2`, log `evidence/powah/review-test-red.log`), GREEN after |
| 2 | `src/powah.rs:239` — `info` discloses another player's fuel | a non-operator read the fuel item and stack count of any loaded furnator at any coordinates | same `Command::op(…, 2)` for `powah info <pos>`; it is operator diagnostics, not gameplay (upstream's own readout is its GUI, which is not ported) | the same test iterates **both** declared usages, so dropping `info` back to level 0 fails it |

No upstream defect was involved; this is a **port** defect (the commands are the harness surface
0.1.0 added, and 0.2.0 kept their level at 0). The commands' words, argument kinds and reply lines
are unchanged, so every readout the e2e asserts is byte-identical.

**The in-game scenario changed in one place, and got stronger.** At `0b14f8a`, `ctl container`
reads a mod block entity's raw `items` vector, which the engine grows only through the game's own
container command path (`item replace block … container.N …`, the path a hopper uses) or an opened
menu — so `ctl container 0 100 0 minecraft:coal 5` reported *"container is full"* with no slots
(first run: `evidence/powah/first-run-container.log`). The e2e now inserts through
`item replace block 0 100 0 container.0 with minecraft:coal 5` and reads the slot back from
`/powah info`'s own `fuel <item> x<count>` field (`fuel_is`), i.e. the port's saved data rather
than a harness copy. Every assertion it had is kept, and the stack-shrink check is now read from
the machine itself.

### Verification (0.2.1)

| Step | Command | Result |
|---|---|---|
| unit tests | `cargo test` | **23 passed**, 0 failed (`evidence/powah/sdk-test.log`) |
| lints | `cargo clippy --all-targets -- -D warnings` | exit 0, no warning (`evidence/powah/sdk-clippy.log`) |
| formatting | `cargo fmt --check` | exit 0, empty (`evidence/powah/sdk-fmt.log`) |
| package, twice | `rm -f Cargo.lock; cargo horizon-mod build` ×2, `cmp` | **byte-identical**, 68,970 B (`evidence/powah/sdk-build{,-second}.log`) |
| registry check | `cargo horizon-mod check powah.hmod` | **exit 0**, `powah 0.2.1 … 68970 bytes, SHA-256 698b63fb…981670` (`evidence/powah/sdk-check.log`) |
| fits the build | `cargo horizon-mod fits powah.hmod --build nightly` | `Powah: / 0b14f8a  made for`, **exit 0** (`evidence/powah/sdk-fits.log`) |
| in-game | `ports/tools/launcher-game.sh run powah` | `PASS: …` + `EXIT:0` (`evidence/powah-e2e/e2e-launcher.log`) |
| control (same Nightly, no package) | `GAME=$CONTROL_GAME … bash ports/powah/tools/e2e.sh` | `FAIL: expected powah:furnator_starter at 0,100,0, got 'minecraft:air'` + `EXIT:1` (`…-control.log`) |
| selftest | `SELFTEST=1 …` | `FAIL: SELFTEST flipped: the unfuelled furnator made no FE, as it should` + `EXIT:1` (`…-selftest.log`) |
| log gate | `python3 ports/tools/check-e2e-logs.py powah-e2e --strict` | `logs=3 ok=3 missing-exit=0 bad-exit=0`, exit 0 |

The e2e now runs on a launcher-built game carrying this package (the gap the 0.2.0 notes
recorded), including the **control** leg: the same Nightly without `powah` knows neither block id,
so the behaviour is the package's. Package: `powah.hmod`, **68,970 B**, sha256
`698b63fb7880f84729dda23d0c9a14ccca091ebd595b97a7c56b977efc981670`.

## Slice 2 — the dielectric/capacitor crafting family (2026-11-20)

### The slice, quoted from this port's own gaps

`## Deviations and gaps (slice 1)`, item 10 — the notes' own next-slice authority:

> **10. Crafting is not possible yet**: the recipes need later-slice items.

and `## Content (`.hmod`)`, the sentence that names them by id:

> The three recipes ship but **do not craft yet**: their ingredients
> (`powah:capacitor_basic_tiny`, `powah:dielectric_casing`, `powah:dielectric_paste`,
> `powah:dielectric_rod_horizontal`) are later slices.

Item 11 ("The other 6 tiers, the creative cell and ~15 machines are unported") names no single
next machine, and every one of those tiers' recipes needs this same material family first, so
the **smallest complete family the notes name concretely** is the one in gap 10: the
dielectric/capacitor items. This slice ships that family and, with it, makes the three carried
machine recipes craft — which is what the 2026-11-20 parity rule (AGENTS.md) requires of every
carried recipe.

**Chosen:** the crafting-material family (six items, eight recipes). **Left:** the six machine
tiers (`furnator_basic` … `nitro`), the creative cell, the cable network (gap 2), item charging
(gap 4), side config/redstone (gap 6), the comparator hook (gap 7), the GUIs (gap 1) and
`powah:capacitor_basic_large` (its consumers are `battery_basic`, `reactor_basic` and the higher
capacitor tiers — none of them in this slice).

### What it ships

| Family | Files |
|---|---|
| items (6) | `data/powah/horizon/item/{dielectric_paste,dielectric_rod,dielectric_rod_horizontal,dielectric_casing,capacitor_basic,capacitor_basic_tiny}.json`, each `{"template": "minecraft:stick"}` (upstream's plain 64-stack items) |
| item models (6) | `assets/powah/models/item/<id>.json` carried from the jar (`dielectric_casing` is its Blockbench 3D model on `block/block`) |
| item definitions (6) | `assets/powah/items/<id>.json` in the 26.3 shape the three existing items already use |
| textures (6) | `assets/powah/textures/item/<id>.png`, carried byte-for-byte |
| recipes (8) | `data/powah/recipe/crafting/{dielectric_paste,dielectric_paste_2,dielectric_rod,dielectric_rod_2,dielectric_rod_h,dielectric_rod_h_2,capacitor_basic,capacitor_basic_tiny}.json` |
| advancements (8) | the jar's recipe-unlock files under `data/powah/advancement/recipes/misc/crafting/` |
| creative tab | the six items join `data/powah/horizon/creative_tab/machines.json` (upstream has one Powah tab) |
| lang | already carried: `item.powah.*` names are in `assets/powah/lang/en_us.json` |

Upstream's numbers (jar `data/powah/recipe/crafting/`, read at tag `v6.2.10`):

| Recipe | Shape | Out |
|---|---|---|
| `dielectric_paste` | shapeless: 3 × `#minecraft:coals` + 2 × `minecraft:clay_ball` + `minecraft:lava_bucket` | 24 |
| `dielectric_paste_2` | shapeless: 2 × `#minecraft:coals` + `minecraft:clay_ball` + `minecraft:blaze_powder` | 16 |
| `dielectric_rod` | shaped `pip/pip/pip` (`p` paste, `i` iron bars) | 8 |
| `dielectric_rod_h` | shaped `ppp/iii/ppp` | 8 |
| `dielectric_rod_2` | shapeless: one horizontal rod → one vertical rod | 1 |
| `dielectric_rod_h_2` | shapeless: one vertical rod → one horizontal rod | 1 |
| `capacitor_basic` | shaped ` ip/iri/pi ` (`i` iron ingot, `p` paste, `r` redstone block) | 4 |
| `capacitor_basic_tiny` | shapeless: one `capacitor_basic` | 2 |

No Rust changes: these are plain items, and a plain item is a `horizon/item` declaration plus a
model, exactly as `docs/mods.md` §"Content mods (API 0.2)" says. `src/runtime.rs` and
`src/powah.rs` stay byte-identical to 0.2.1, so their 23 tests are unchanged.

### How it is proven (parity rule)

`tools/craft_check.py` drives a real crafting table through `horizon-ctl` (`screen` slots
100–108 row-major, result 200, the `additional_lights` pattern): for **each of the eight
material recipes and the three machine recipes** it gives the declared ingredients, places them
in the grid the way the pattern declares, asserts the result slot's **id and count**, takes it,
and asserts the **inventory counts changed by exactly the recipe's amounts** (each ingredient
down by what it used, the output up by its count). `dielectric_paste` also asserts the
`minecraft:lava_bucket` leaves a `minecraft:bucket` in the grid — the crafting remainder.
Two controls run in the same leg: an off-pattern grid (a stick where the casing belongs) leaves
the result empty, and `SELFTEST=1` expects a wrong output count so the run must fail.

`notes.md` carries the coverage table at the end of this section.

### Results (2026-11-20)

**TDD order as run.** The carry and the recipe leg were written first, then the same
`tools/craft_check.py` was run against the **0.2.1** package: the first two cases reported
`got None xNone` (the paste did not craft) and the third could not find `powah:dielectric_paste`
in the inventory at all; the run ended `EXIT:1`
(`evidence/powah-e2e/e2e-selftest-oldpackage.log`). The 0.2.2 package then made the same script
green, and the whole leg was run four times (main, control, two selftests).

Ladder (every step in `ports/powah`, `CARGO_INCREMENTAL=0`, inside a
`ports/tools/build-slot.sh run --job powah-ladder --port ports/powah --step build` lease;
logs in `ports/_shared/evidence/powah/`):

| Step | Command | Result |
|---|---|---|
| unit tests | `cargo test --offline` | **23 passed**, 0 failed (`sdk-test.log`) |
| lints | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, 0 warnings (`sdk-clippy.log`) |
| formatting | `cargo fmt --check` | exit 0, empty (`sdk-fmt.log`) |
| package, twice | `rm -f Cargo.lock; cargo horizon-mod build` ×2, `cmp` | **byte-identical**, 83,289 B (`sdk-build{,-second}.log`) |
| registry check | `cargo horizon-mod check powah.hmod` | **exit 0**, `powah 0.2.2 … 83289 bytes, SHA-256 9b600446…14bcdf3` (`sdk-check.log`) |
| fits the build | `cargo horizon-mod fits powah.hmod --build 0b14f8a` | `Powah: / 0b14f8a  made for`, **exit 0** (`sdk-fits.log`) |
| package contents | `unzip -l powah.hmod` | **90 entries**, no `Cargo.lock`, version 0.2.2 |

In-game (launcher build at `0b14f8a` carrying this package; logs in
`ports/_shared/evidence/powah-e2e/`):

| Leg | Command | Result |
|---|---|---|
| main | `launcher-game.sh build powah` + `build-slot.sh run --step e2e -- bash ports/powah/tools/e2e.sh` | **PASS** + `EXIT:0` (`e2e-launcher.log`) |
| control (same Nightly, no package) | `GAME=$CONTROL_GAME …` | `FAIL: expected powah:furnator_starter at 0,100,0, got 'minecraft:air'` + `EXIT:1` (`e2e-control.log`) |
| selftest (slice 1's flip) | `SELFTEST=1 …` | `FAIL: SELFTEST flipped: the unfuelled furnator made no FE, as it should` + `EXIT:1` (`e2e-selftest.log`) |
| selftest (slice 2's flip) | `SELFTEST=craft …` | `FAIL: … expected 25 powah:dielectric_paste, got powah:dielectric_paste x24` + `EXIT:1` (`e2e-selftest-craft.log`) |
| old package | `craft_check.py` against the 0.2.1 build | every case `got None xNone` + `EXIT:1` (`e2e-selftest-oldpackage.log`) |
| log gate | `python3 ports/tools/check-e2e-logs.py powah-e2e --strict` | `logs=7 ok=7 missing-exit=0 bad-exit=0`, exit 0 |

Quoted from `e2e-launcher.log` (the recipe leg, then the loot leg):

```
3 coal + 2 clay + lava bucket -> 24 dielectric paste -> {"count": 24, "item": "powah:dielectric_paste", "slot": 200}
2 coal + clay + blaze powder -> 16 dielectric paste -> {"count": 16, "item": "powah:dielectric_paste", "slot": 200}
paste + iron bars -> 8 dielectric rods -> {"count": 8, "item": "powah:dielectric_rod", "slot": 200}
paste + iron bars -> 8 horizontal dielectric rods -> {"count": 8, "item": "powah:dielectric_rod_horizontal", "slot": 200}
one horizontal rod -> one vertical rod -> {"count": 1, "item": "powah:dielectric_rod", "slot": 200}
one vertical rod -> one horizontal rod -> {"count": 1, "item": "powah:dielectric_rod_horizontal", "slot": 200}
iron + paste + redstone block -> 4 basic capacitors -> {"count": 4, "item": "powah:capacitor_basic", "slot": 200}
one basic capacitor -> two tiny capacitors -> {"count": 2, "item": "powah:capacitor_basic_tiny", "slot": 200}
3 paste + 2 tiny capacitors + casing + furnace -> a furnator -> {"count": 1, "item": "powah:furnator_starter", "slot": 200}
4 iron + 4 tiny capacitors + casing -> an energy cell -> {"count": 1, "item": "powah:energy_cell_starter", "slot": 200}
6 horizontal rods + 2 nuggets + tiny capacitor -> 12 cables -> {"count": 12, "item": "powah:energy_cable_starter", "slot": 200}
furnator shape with a stick -> {"item": null, "slot": 200}
recipe PASS: the eight material recipes and the three machine recipes craft their declared output id and count,
every ingredient is consumed, the lava bucket leaves its bucket, and the off-pattern grid leaves the result empty
  block(6,100,0) = powah:furnator_starter
  block(7,100,0) = powah:energy_cell_starter
  block(8,100,0) = powah:energy_cable_starter
```

The consumption half is asserted inside `craft_check.py` against `ctl inventory`: each case reads
the counts after `give`, crafts, takes the result into an empty slot, and fails unless every
ingredient fell by exactly the number the pattern used and the output rose by its count; the
paste case also reads the grid back and requires the `minecraft:bucket` remainder there. The
loot leg switches to survival with an iron pickaxe in hand and asserts `drops` contains the
machine.

Package: `powah.hmod`, **83,097 B**, sha256
`9b600446d7457809af1583b3f478881de5b066add4035c0d18ba14ae414bcdf3`.

### Coverage (parity rule, every family the package ships)

| Family (from the jar) | Status |
|---|---|
| `furnator_starter` block + item | **tested** — placed (`place`), given, `info` readout, fuel burned, FE produced (`e2e-launcher.log` phases 1–5) |
| `energy_cell_starter` block + item | **tested** — placed, `set`, full/empty controls, source and sink of a transfer |
| `energy_cable_starter` block + item | **tested** — placed, `no storage, moves up to 500 FE/t`, carries FE, and the no-cable control |
| 3 block loot tables | **tested** — mined in survival, each drops itself (loot leg) |
| `dielectric_paste`, `dielectric_rod`, `dielectric_rod_horizontal`, `dielectric_casing`, `capacitor_basic`, `capacitor_basic_tiny` | **tested** — `ctl give` + `inventory` counts, and each is the output of a crafted recipe |
| 8 material recipes | **tested** — crafted in a real crafting table, output id + count, ingredients consumed, bucket remainder |
| 3 machine recipes | **tested** — same leg; they were dead data in 0.2.1 |
| creative tab (9 items) | **not asserted in game** — no `ctl` command reads a creative tab; `cargo horizon-mod check` validates the tab file and its ids, and the leg proves each id is giveable |
| `powah:capacitor_basic_large` | **not ported** — its consumers (`battery_basic`, `reactor_basic`, the higher capacitor tiers) are not in this slice |
| the six machine tiers, creative cell, Magmator, Thermo, Solar, Reactors, Ender cells/gates, Energizing, Discharger, Hopper, Player Transmitter, batteries, wrench, book, upgrades, GUIs, side config, redstone modes, worldgen | **not ported** — the recorded gaps of slice 1 (items 1–9, 11) |

### Pin findings this slice measured (for the next port)

1. **A menu's slots need `--offscreen`.** With `--headless`, `ctl screen` and `ctl ui` time out on
   an open menu (measured twice: no menu and a crafting table); with `--offscreen` they answer.
   The e2e now launches `--offscreen`.
2. **`ctl interact` refuses a face the player's eye cannot see** (`no face of block … is visible
   from here`): stand ~2.5 blocks away and `ctl look-at-block` first. Standing right beside the
   block puts the eye above it.
3. **Shapeless matching counts grid cells, not items** — `crafting.rs:2013`
   (`matches_all(ctx.context, ingredients, &input.stacks())`) with
   `crafting/shapeless_matching.rs` (`ingredients.len() != stacks.len()` → false). Three coals in
   one cell never match three coal ingredients; one coal in each of three cells does. The port's
   shapeless recipes stay upstream's; the leg places one item per cell. **Worth an upstream
   report**: vanilla's `StackedContents` counts items, so a vanilla-shaped pack behaves
   differently.
4. **A recipe ingredient must be the string form** (`"minecraft:iron_ingot"`,
   `"#minecraft:coals"`); an object (`{"item": …}`) is logged as *"recipes: invalid ingredient
   (unresolved tag or malformed input)"* and the whole recipe is dropped. Other ports carry the
   object form (e.g. `enderstorage`), so their recipes may be silently absent — not this port's
   file to change, recorded here.
5. **Two chat round trips are ~40 ticks apart**, so a two-block assertion can read the source
   before the transfer and the sink after it (measured: source 1000 at t=508, sink 500 at t=513).
   The slice-1 discharge phase now polls the source until the move shows; it was flaky before.

### Deviations slice 2 adds

1. **`capacitor_basic_large` is not carried** (see the coverage table: nothing in this slice uses it).
2. **The paste recipes are matched one-cell-per-item** (finding 3): upstream's shapeless shape is
   kept, but a player stacking three coals in one cell will not match until the pin matches items.
3. **The recipes are written in the 26.3 string form** (finding 4), which is a format, not a
   behaviour difference: the ids, counts, shapes and ingredients are the jar's.

Version: **0.2.1 → 0.2.2** (one patch). The published `[[versions]]` record keeps 0.2.1 and gains
0.2.2 at publish time — this port does not touch the registry entry.

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, verification-before-completion; BeautifulRust/none (no Rust change; `rust-best-practices` loaded and the Rust tree is untouched)

## Slice 3 — the cable network (2026-11-21)

### The slice, quoted from this port's own gaps

`## Deviations and gaps (slice 1)`, item 2 (the notes' own next-slice authority):

> **No cable network.** Upstream's cables share one buffer across a connected
> network and forward an insertion through it in one call. The port moves FE one
> cable hop per tick between *adjacent block entities*, so a run of two or more
> cables does not relay. The e2e's chain is exactly one cable (the slice's brief).

`FINDINGS.md` §Energy Cable reads the same mechanic upstream (`CableTile` /
`CableNet`: walk the connected same-tier cables round-robin and push into each
cable's energy side). So this is a family in the jar, named in the port's gaps.

**Why it fits `machines`:** it is a block-entity relay — the cable's
`BlockEntityPart` tick plus the level's energy capability and `move_energy`, the
same three API pieces slice 1 already declares; no renderer, no GUI, no new item,
no client half. And it is implementable at the pin: `LevelRef::block(host,
pos)?.id(host)` reads a neighbour's block id, which is all a network walk needs.

**Not chosen (and why):** item charging (gap 4) needs `item_energy_buffer` *and*
a chargeable item — Powah's batteries are unported, so nothing in the package can
be charged and the assertion could not fail; the comparator (gap 7) and side
config/redstone (gap 6) are one-hook families; the other six tiers (gap 11) name
no single next machine and need a full content carry first.

### What it ships

No content file changed: the package still carries the same **90 entries** (43
`assets/`, 38 `data/`, 3 `src/`, the 6 root documents). All of slice 3 is Rust.

| File | Change |
|---|---|
| `src/runtime.rs` | The cable planner's loop became a private `plan(sides, transfer, rotation)`; `cable_moves` (six sides) and the new **`network_moves(ports, transfer, rotation)`** both delegate to it. 5 new tests. |
| `src/powah.rs` | `discover_cables` (breadth-first walk over same-tier cable blocks, capped at `NETWORK_MAX_CABLES = 1024`), `relay_network` (gathers the network's non-cable energy sides once per position, plans once, one transaction per move), and `on_level_tick_post` now plans each network once per tick. The single-hop `relay` is gone. |

The machine numbers are unchanged (furnator 20,000 / 80 / 20 FE/t, cell
1,000,000 / 1,000 a side, cable 500 FE/t). The **network spends one cable's 500
FE/t per tick**, not one per cable: the readout's "moves up to 500 FE/t" stays true
for the whole run, and a longer run is not faster. Upstream caps *each* energy
side at a cable's transfer; this port pulls, so the network is one relay and its
budget is one cable's rate. That is a recorded deviation (below), not a defect —
it keeps slice 1's measured number.

### TDD: the red proof, quoted

`network_moves` was declared with a `todo!()` body and the five tests written
against it, then run (`ports/_shared/evidence/powah/slice3-test-red.log`):

```text
---- runtime::tests::a_network_relays_across_the_cables_between_the_source_and_the_sink stdout ----

thread 'runtime::tests::a_network_relays_across_the_cables_between_the_source_and_the_sink' (1797) panicked at src/runtime.rs:209:5:
not yet implemented: the cable network planner
...
test result: FAILED. 23 passed; 5 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

All five failed for the same reason (the planner was not written), and the five
names were:

- `a_network_relays_across_the_cables_between_the_source_and_the_sink`
- `a_network_spends_one_cable_s_rate_not_a_rate_per_cable`
- `one_cable_is_the_same_plan_as_its_six_sides` (the generalization is
  behaviour-preserving; it cross-checks `cable_moves` against `network_moves`)
- `a_network_without_a_giver_or_a_taker_moves_nothing`
- `the_network_rotation_starts_at_another_port`

After the implementation the same command printed **28 passed, 0 failed**
(`slice3-test-green.log`), 23 slice-1/2 tests plus the five.

### In-game (parity rule)

The leg is a **red→green against the previous package**: the 0.2.2 build (made for
`0b14f8a`, kept at `ports/_shared/evidence/powah/powah-0.2.2.hmod`) run with the
new scenario delivers **0 FE** across the three-cable row and fails there:

```text
  source: Energy Cell at 0,100,5: 100000/1000000 FE (comparator 1, t=758)
  sink:   Energy Cell at 4,100,5: 0/1000000 FE (comparator 0, t=624)
  off the network: Energy Cell at 0,103,5: 0/1000000 FE (comparator 0, t=763)
FAIL: a three-cable network did not relay FE into the cell: Energy Cell at 4,100,5: 0/1000000 FE (comparator 0, t=624)
```

(`ports/_shared/evidence/powah-e2e/e2e-slice3-oldpackage.log`, `EXIT:1`.) 0.2.3,
the same scenario (`e2e-launcher.log`, `EXIT:0`):

```text
== slice 3: a run of three cables relays as one network ==
  block(0,100,5) = powah:energy_cell_starter
  block(1,100,5) = powah:energy_cable_starter
  block(2,100,5) = powah:energy_cable_starter
  block(3,100,5) = powah:energy_cable_starter
  block(4,100,5) = powah:energy_cell_starter
  block(0,103,5) = powah:energy_cell_starter
  set(0,100,5) = Energy Cell at 0,100,5: 100000/1000000 FE (comparator 1, t=698)
  source: Energy Cell at 0,100,5: 99500/1000000 FE (comparator 1, t=757)
  sink:   Energy Cell at 4,100,5: 500/1000000 FE (comparator 0, t=747)
  off the network: Energy Cell at 0,103,5: 0/1000000 FE (comparator 0, t=765)
  split: Energy Cell at 0,100,5: 99500/1000000 FE (comparator 1, t=784)
  split: Energy Cell at 4,100,5: 500/1000000 FE (comparator 0, t=794)
```

The three assertions, each able to fail:

1. **the run relays** — the sink holds > 0 (0 on 0.2.2, 0 on `SELFTEST=network`);
2. **nothing is created or lost** — the middle cable is broken so the two cells
   stop trading, and then the two readings must sum to exactly the 100,000 the
   source was set to (99,500 + 500);
3. **the walk is what carries it** — a cell three blocks above the source, reached
   by no cable, stays at 0, so the charge did not teleport.

### Ladder (0.2.3, pin `0a875da`)

Every compile step ran in `ports/powah`, `CARGO_INCREMENTAL=0`, inside a
`ports/tools/build-slot.sh run --job W1-powah --port ports/powah --step …` lease
with `BUILD_LOCAL=1`; logs in `ports/_shared/evidence/powah/`.

| Step | Command | Result |
|---|---|---|
| unit tests | `cargo test --offline` | **28 passed**, 0 failed (`sdk-test.log`) |
| lints | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, 0 warnings (`sdk-clippy.log`) |
| formatting | `cargo fmt --check` | exit 0, empty (`sdk-fmt.log`) |
| package, twice | `rm -f Cargo.lock; cargo horizon-mod build` ×2, `cmp` | **byte-identical, 85,383 B** (`sdk-build{,-second}.log`) |
| registry check | `cargo horizon-mod check powah.hmod` | **exit 0**, `powah 0.2.3 (native-mod, format native-1/0a875da…/e59da90…)`, `85383 bytes, SHA-256 e9583d43…9b3b9c` (`sdk-check.log`) |
| fits the pin | `cargo horizon-mod fits powah.hmod --build 0a875da` | `0a875da  made for`, exit 0 (`sdk-fits.log`) |
| fits **another** build | `cargo horizon-mod fits powah.hmod --build 0b14f8a` | `0b14f8a  fits`, exit 0 (`sdk-fits-nightly.log`) |
| what it uses | `cargo horizon-mod check --uses` | 312 items, made for `0a875da` |
| package gate | `python3 ports/tools/verify-sdk-package.py powah` | **1/1**, runtime byte-identical, no packed `Cargo.lock`, 90 entries |

The `fits` pair matters: `--build 0a875da` is the pin and only prints `made for`
(a tautology); the same package against the previous pin `0b14f8a` says **fits**,
i.e. every one of its 312 API items still resolves there.

In-game legs (launcher build `native-0a875da-e8d17b7a402b`, instance
`powah-e2e`; logs in `ports/_shared/evidence/powah-e2e/`):

| Leg | Command | Result |
|---|---|---|
| main | `launcher-game.sh build powah` + `build-slot.sh run --step e2e -- bash tools/e2e.sh` | **PASS** + `EXIT:0` (`e2e-launcher.log`) |
| control (same Nightly, no package) | `GAME=$CONTROL_GAME …` | `FAIL: expected powah:furnator_starter at 0,100,0, got 'minecraft:air'` + `EXIT:1` (`e2e-control.log`) |
| selftest (slice 3's flip) | `SELFTEST=network …` | `FAIL: SELFTEST flipped: the three-cable network relayed FE, as it should` + `EXIT:1` (`e2e-selftest-network.log`) |
| selftest (slice 1's flip) | `SELFTEST=1 …` | `FAIL: SELFTEST flipped: the unfuelled furnator made no FE, as it should` + `EXIT:1` (`e2e-selftest.log`) |
| the previous package | the 0.2.2 build + the same scenario | `EXIT:1` at the network assertion (`e2e-slice3-oldpackage.log`) |
| log gate | `python3 ports/tools/check-e2e-logs.py powah-e2e --strict` | `logs=9 ok=9 missing-exit=0 bad-exit=0`, exit 0 |

The main log also carries the in-script control leg (the same Nightly without
powah knows neither block id and neither item).

### Coverage (parity rule, the family this slice adds)

| Family (from the jar) | Status |
|---|---|
| cable **network** (`CableNet`: connected same-tier cables forward one insertion) | **tested** — a source cell, three cables and a sink cell in one row: the charge crosses the run (0 on 0.2.2), the split cells sum to the 100,000 set, and a cell off the network stays 0; the planner's five rules are unit-tested |
| single-cable relay (slice 1) | **tested** — unchanged: `one_cable_is_the_same_plan_as_its_six_sides` compares the new network plan with the six-side plan, and the slice-1 phases still pass in the same run |
| every other family | as slice 2's table: ported and tested, or `not ported` with the gap that names it |

### Deviations slice 3 adds

1. **One cable's rate per network per tick** (500 FE/t), not one per cable: the
   port pulls, so the network is one relay; upstream pushes into each energy side
   at up to a cable's transfer. A ten-cable run therefore carries the same 500
   FE/t as a one-cable run.
2. **The walk is capped at 1,024 cables** per network (`NETWORK_MAX_CABLES`).
   Upstream's `CableNet` is unbounded; the cap bounds one tick's work and is far
   past any pack's run.
3. **A block touching two cables of one network is one port** (deduplicated by
   position), so a cell flanked by two cables of the same network is filled once
   per plan rather than twice. With deviation 1's single budget this changes no
   observable rate.
4. **Per-tier isolation is by block id**: only `powah:energy_cable_starter` exists
   in this slice, so the walk treats every cable it finds as the same network. When
   the other six tiers arrive, the membership test becomes same-tier.
5. The slice-1 gaps that remain are unchanged: no GUI, no item charging, no side
   config/redstone modes, no comparator hook, breaking a machine loses its FE, the
   other six tiers and ~15 machines unported, `/powah set` is a harness control.

**Upstream defects fixed:** none — the single-hop relay was the *port's*
limitation (slice 1's recorded scope), not a defect in Powah 6.2.10, so the
upstream-defect procedure did not apply.

### Pin findings this slice measured (for the next port)

1. **The pin moved mid-slice** (`0b14f8a` → `0a875da`, mixins unchanged). Every
   number above is at `0a875da`; the package declares it, and the launcher's
   `check_pins` refuses a package pinned elsewhere. `fits --build <the previous
   pin>` is the run that proves something.
2. **A launcher instance that already holds an older install of the port refuses
   the new package.** `native-build --instance e2e` (an instance shared with a
   previous `powah` install from `0b14f8a`) answered *"powah's package was made by
   an older tool and doesn't say what it uses, so it works on the Nov 20 update
   only"*, although the same folder carries a valid `uses.json` (312 entries,
   `made_for` = the pin) and `cargo horizon-mod fits` on the folder is green. With
   a dedicated instance (`HORIZON_E2E_INSTANCE=powah-e2e`) the identical folder
   built in 7 s. **Give each port its own instance**, which is also what stops two
   agents rebuilding each other's instance.
3. **Two `info` reads are ~20–40 ticks apart, and adjacent cells swap energy every
   tick**, so a sum of two live readings is *not* the total: the first version of
   the conservation assertion read 100,500 (source at `t=757`, sink at `t=733`) on
   a balanced relay. Freeze the transfer (break a cable) before summing.
4. **`BUILD_LOCAL=1` for evidence integrity.** A forwarded step runs on the far
   box and then `remote-slot.sh` pulls `ports/_shared/evidence/<mod>/` back, which
   **overwrote a fresh local `sdk-test.log` with the far box's older file of the
   same name** (mtime and content from the previous day). Local runs keep each log
   an artefact of the run it names.

Version: **0.2.2 → 0.2.3** (one patch), pin `0b14f8a` → `0a875da`. The published
`[[versions]]` record keeps 0.2.2 and gains 0.2.3 at publish time — this port does
not touch the registry entry.

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, verification-before-completion; BeautifulRust/rust-best-practices

## Slice 4 — the energy cell ladder (2026-11-21)

### The slice, quoted from this port's own gaps

`## Deviations and gaps (slice 1)`, item 11 — the notes' own next-slice authority:

> **11. The other 6 tiers, the creative cell and ~15 machines** are unported.

and `FINDINGS.md` §"Tier numbering", whose derived table names the numbers this slice
carries:

> cell 1M/4M/10M/40M/100M/400M/2G

**Chosen:** the six remaining **energy cell** tiers (`basic`, `hardened`, `blazing`,
`niotic`, `spirited`, `nitro`) as storage blocks with their own capacity and per-side
transfer — the ladder the starter cell's row already begins. **Why it fits
`machines`:** a cell is the tier's shape exactly — a declared block energy capability
(`Registrar::block_energy`) over the saved int `energy`, one block entity type and an
idle tick, and the `push_out` the starter cell already runs; no new hook family, no
renderer, no GUI, no item behaviour. Its rule is the most directly assertable in the
package: `/powah set 2000000000` against each tier must clamp at **that tier's**
capacity, so one command reads back a different exact number per tier (a
before → after `ctl` readout that the 0.2.3 package fails outright: it does not know
the ids).

**Not chosen (and why):** the reactors (`FINDINGS.md` names them) need
`powah:uraninite`, whose every source is the **unported Energizing Orb** or worldgen,
so no reactor recipe could be crafted; the Magmator/Thermo generators need a fluid
tank (`fluid_insert` cannot fill a mod BE tank at this pin, measured) and the Solar
panel needs sky visibility; the other six **furnator/cable** tiers are the same shape
as this slice but their rule is a rate into a buffer (a bound), not an exact readout.

**Recipes:** the jar's `energy_cell_basic` recipe is craftable from carried items
(`4 × capacitor_basic`, `dielectric_casing`, `4 × iron_ingot`), so it is carried and
crafted in game. The other five cell recipes take `powah:steel_energized` /
`powah:crystal_blazing|niotic|spirited|nitro`, which are only made by the unported
**Energizing Orb**; those recipe files are **not carried** (a dead recipe is worse
than a named gap) and the coverage table records `blocked(energizing-orb)` with the
exact missing ids. Every tier block itself is placed, set and read in game.

### What it ships

| Family | Files |
|---|---|
| blocks (6) | `data/powah/horizon/block/energy_cell_{basic,hardened,blazing,niotic,spirited,nitro}.json` (the starter's patched def: `minecraft:iron_block`, 2.0/20.0, metal, `requires_tool`, `occludes: false`, stack 1) |
| blockstates (6) | `assets/powah/blockstates/energy_cell_<tier>.json`, carried |
| block models (6) | `assets/powah/models/block/energy_cell_<tier>.json` (parent `powah:block/energy_cell`, the tier's texture) |
| item models (6) + item defs (6) | `assets/powah/models/item/…`, `assets/powah/items/…`, carried |
| textures (6) | `assets/powah/textures/block/energy_cell_<tier>.png`, byte-for-byte |
| loot (6) | `data/powah/loot_table/blocks/energy_cell_<tier>.json` (the jar's 26.3 shape) |
| tags | the six join `minecraft:mineable/pickaxe` and `needs_iron_tool` |
| creative tab | the six join `powah:horizon/creative_tab/machines.json` |
| recipe + advancement (1 + 1) | `energy_cell_basic` only (see above) |

Rust, all in the port's own tree:

| File | Change |
|---|---|
| `src/runtime.rs` | `CellTier { capacity, transfer }`, `CELL_TIER_COUNT`, `cell_tier(index)` derived from upstream's `baseScaling` (`× 1_000_000` capacity, `× 1_000` transfer); 2 new tests |
| `src/powah.rs` | `CELLS` (the seven ids + type names), `cell_of(id)`, `Kind::Cell(CellTier)`, `cell_battery(tier)`, per-tier `battery_of`/readout/set clamp/push cap; 2 new tests |
| `src/lib.rs` | one `block_energy` declaration and one idle block entity type per cell tier (7 buffers, 9 types in total) |

The readout gained the tier's transfer so the in-game leg can read it back:
`Energy Cell at x,y,z: 4000000/4000000 FE (comparator 15, 4000 FE/t a side, t=739)`.
The starter's line keeps its prefix (every 0.1.0/0.2.x assertion still matches) and
the unit test that pins the exact line was updated.

### TDD: the red proof, quoted

`cell_tier` and `cell_of` were declared with `todo!()` and the tests written against
them, then run (`ports/_shared/evidence/powah/slice4-test-red.log`):

```text
thread 'powah::tests::every_energy_cell_tier_is_one_of_the_ladder' panicked at src/powah.rs:74:5:
not yet implemented: the energy cell block ids

thread 'runtime::tests::every_energy_cell_tier_scales_upstreams_battery_numbers' panicked at src/runtime.rs:44:5:
not yet implemented: the energy cell tier ladder

thread 'runtime::tests::the_starter_cell_keeps_slice_1_s_numbers' panicked at src/runtime.rs:44:5:
not yet implemented: the energy cell tier ladder

test result: FAILED. 28 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

After the implementation: **32 passed, 0 failed**
(`slice4-test-green.log`; the same 32 in `sdk-test.log`). The fourth new test,
`every_cell_tier_declares_its_own_buffer_and_nothing_else_does`, was added with the
wiring after that run (it pins `battery_of` to the same table); the wiring's red
proof is the in-game one below.

The in-game red is the same scenario against the **0.2.3** package
(`ports/_shared/evidence/powah/powah-0.2.3.hmod`, run on a launcher build of that
package's own tree, `evidence/powah-e2e/e2e-slice4-oldpackage.log`):

```text
== slice 4: every energy cell tier holds its own capacity ==
FAIL: expected powah:energy_cell_basic at -4,100,12, got 'minecraft:air'
```
(`Unknown block type 'powah:energy_cell_basic'` in the game log; `EXIT:1`.)

### Ladder (0.2.4, pin `0a875da`)

Every compile step ran in `ports/powah`, `CARGO_INCREMENTAL=0`, inside a
`ports/tools/build-slot.sh run --job W1-powah --port ports/powah --step …` lease with
`BUILD_LOCAL=1`; logs in `ports/_shared/evidence/powah/`.

| Step | Command | Result |
|---|---|---|
| unit tests | `cargo test --offline` | **32 passed**, 0 failed (`sdk-test.log`) |
| lints | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, 0 warnings (`sdk-clippy.log`) |
| formatting | `cargo fmt --check` | exit 0, empty (`sdk-fmt.log`) |
| package, twice | `rm -f Cargo.lock; cargo horizon-mod build` ×2, `cmp` | **byte-identical, 119,200 B** (`sdk-build{,-second}.log`) |
| registry check | `cargo horizon-mod check powah.hmod` | **exit 0**, `powah 0.2.4 (native-mod, format native-1/0a875da…/e59da90…, for 26.3)`, `119200 bytes, SHA-256 663465ee8b93e5f8be371537f6515bd7252651e462a57f477faedd82ffcd3e34, unsigned` (`sdk-check.log`) |
| fits the pin | `cargo horizon-mod fits powah.hmod --build 0a875da` | `0a875da  made for`, exit 0 (`sdk-fits.log`) |
| fits **another** build | `cargo horizon-mod fits powah.hmod --build 0b14f8a` | `0b14f8a  fits`, exit 0 (`sdk-fits-nightly.log`) |
| package gate | `python3 ports/tools/verify-sdk-package.py powah` | **1/1**, runtime byte-identical (`e49a2edda151`), no packed `Cargo.lock`, **134 entries** |
| carried-data dialect | `python3 ports/tools/check_261_dialect.py powah.hmod` | exit 0: 33 recipe/loot files, every one in the 26.3 dialect, every `requires_tool` block in a mineable tag |

In-game legs (launcher build `native-0a875da-f6fbf1a8142b`, instance `powah-e2e`,
`Compiling powah (1 of 1)` in `evidence/powah-e2e/launcher-build-0.2.4.log`; logs in
`ports/_shared/evidence/powah-e2e/`):

| Leg | Command | Result |
|---|---|---|
| main | `launcher-game.sh build powah` + `build-slot.sh run --step e2e -- bash tools/e2e.sh` | **PASS** + `EXIT:0` (`e2e-launcher.log`) |
| control (inside the main run; same Nightly, no package) | `CONTROL_GAME` | `setblock` → `Unknown block type 'powah:energy_cell_nitro'`, `give` → `Unknown item 'powah:energy_cell_starter'`, block stays air |
| control (whole scenario, no package) | `GAME=$CONTROL_GAME …` | `FAIL: expected powah:furnator_starter at 0,100,0, got 'minecraft:air'` + `EXIT:1` (`e2e-control.log`) |
| selftest (slice 1's flip) | `SELFTEST=1 …` | `FAIL: SELFTEST flipped: the unfuelled furnator made no FE, as it should` + `EXIT:1` (`e2e-selftest.log`) |
| selftest (this slice's flip) | `SELFTEST=cells …` | `FAIL: expected powah:energy_cell_basic to hold 1000000/1000000 FE, got 'Energy Cell at -4,100,12: 4000000/4000000 FE (comparator 15, 4000 FE/t a side, t=766)'` + `EXIT:1` (`e2e-selftest-cells.log`) |
| the previous package | 0.2.3 + this scenario | `EXIT:1` at the first new tier (`e2e-slice4-oldpackage.log`) |
| log gate | `python3 ports/tools/check-e2e-logs.py powah-e2e --strict` | see below |

Quoted from `e2e-launcher.log` (one command per tier; every tier clamps at its own
capacity and names its own transfer):

```text
== slice 4: every energy cell tier holds its own capacity ==
  block(-4,100,12) = powah:energy_cell_basic
  set(-4,100,12) = Energy Cell at -4,100,12: 4000000/4000000 FE (comparator 15, 4000 FE/t a side, t=714)
  block(-2,100,12) = powah:energy_cell_hardened
  set(-2,100,12) = Energy Cell at -2,100,12: 10000000/10000000 FE (comparator 15, 10000 FE/t a side, t=728)
  block(0,100,12) = powah:energy_cell_blazing
  set(0,100,12) = Energy Cell at 0,100,12: 40000000/40000000 FE (comparator 15, 40000 FE/t a side, t=744)
  block(2,100,12) = powah:energy_cell_niotic
  set(2,100,12) = Energy Cell at 2,100,12: 100000000/100000000 FE (comparator 15, 100000 FE/t a side, t=762)
  block(4,100,12) = powah:energy_cell_spirited
  set(4,100,12) = Energy Cell at 4,100,12: 400000000/400000000 FE (comparator 15, 400000 FE/t a side, t=778)
  block(6,100,12) = powah:energy_cell_nitro
  set(6,100,12) = Energy Cell at 6,100,12: 2000000000/2000000000 FE (comparator 15, 2000000 FE/t a side, t=794)
  block(-6,100,12) = powah:energy_cell_starter
  set(-6,100,12) = Energy Cell at -6,100,12: 1000000/1000000 FE (comparator 15, 1000 FE/t a side, t=810)
== slice 4: two adjacent cells swap and create nothing ==
  pair source: Energy Cell at 16,100,16: 4000000/4000000 FE (comparator 15, 4000 FE/t a side, t=892)
  pair sink:   Energy Cell at 16,100,17: 0/1000000 FE (comparator 0, 1000 FE/t a side, t=899)
  {"scenario":"loot","block":"minecraft:air","drops":{"powah:energy_cell_basic":1}}
  {"scenario":"loot","block":"minecraft:air","drops":{"powah:energy_cell_nitro":1}}
```

### What the in-game leg can and cannot assert about the transfer

The **capacity** is asserted behaviourally and exactly: `2000000000` is above every
tier's capacity, so the clamp is the tier's number, and it differs per tier.

The **transfer** is asserted as the declaration the readout reads back, and pinned by
`every_cell_tier_declares_its_own_buffer_and_nothing_else_does` (the same `cell_tier`
row feeds `Registrar::block_energy`). A *net* flow between two cells is not
observable at this pin, and this is upstream's behaviour rather than a port gap: a
cell gives `min(transfer, stored)` and the neighbour accepts only
`min(the neighbour's own insert cap, room)`, and both push every tick, so any adjacent
pair swaps and nets zero — measured both ways (basic source → starter sink and
starter → starter), and the pair phase asserts exactly that (the sink never climbs
past its own 1,000 insert cap beside a 4,000/t source, and the source never loses more
than the same cap, so the pair swaps rather than creating FE).

### Coverage (parity rule, the family this slice adds)

| Family (from the jar) | Status |
|---|---|
| `energy_cell_basic` block + item | **tested** — placed, `/powah set 2000000000` clamps at 4,000,000, readout names 4,000/t; mined in survival, drops itself |
| `energy_cell_hardened` | **tested** — same legs; clamps 10,000,000, 10,000/t |
| `energy_cell_blazing` | **tested** — clamps 40,000,000, 40,000/t |
| `energy_cell_niotic` | **tested** — clamps 100,000,000, 100,000/t |
| `energy_cell_spirited` | **tested** — clamps 400,000,000, 400,000/t |
| `energy_cell_nitro` | **tested** — clamps 2,000,000,000, 2,000,000/t (the largest value the integer command accepts) |
| `energy_cell_starter` (contrast) | **tested** — the same command clamps at 1,000,000, 1,000/t; slice 1's legs unchanged |
| `energy_cell_basic` recipe | **tested** — crafted in a real table (`craft_check.py`): id + count, every ingredient consumed |
| `energy_cell_{hardened,blazing,niotic,spirited,nitro}` recipes | **blocked(energizing-orb)** — they need `powah:steel_energized` / `powah:crystal_{blazing,niotic,spirited,nitro}`, all made by the unported Energizing Orb; the files are not carried |
| the six cells' loot tables | **tested** — mined in survival, each drops itself (6 rows in the loot leg) |
| creative tab (15 items now) | **not asserted in game** — no `ctl` command reads a creative tab; `cargo horizon-mod check` validates the tab, and the leg proves each id is giveable/placeable |
| `energy_cell_creative` | **not ported** — a source, not a storage tier (gap 11) |
| the other tiers/machines (furnator, cable, generators, reactors, Energizing, batteries, GUIs, side config, redstone modes, item charging, worldgen) | **not ported** — slice 1's recorded gaps 1–9, 11; `## Next slice` names the Energizing Orb as the one that unblocks the five recipes above |

### Deviations slice 4 adds

1. **Five of the six cell recipes are not carried** (see above): the blocks exist and
   are provable, but in survival only `energy_cell_basic` can be reached until the
   Energizing Orb ships. Recorded as `blocked(energizing-orb)`, not hidden.
2. **The readout gained the tier's transfer** (`… FE/t a side`) so the in-game leg can
   read the declaration back. The starter's line keeps its prefix.
3. **The side config, redstone modes, comparator hook, GUI and item charging** remain
   slice 1's gaps for every tier, the starter included.

**Upstream defects fixed:** none — the tier numbers are upstream's
(`DefaultEnergies`), and the swap between adjacent cells is upstream's own
`Transfer.ALL` behaviour, documented (slice 1 deviation 3), not a defect. The only
defect found while measuring was in **this port's harness**, not the mod: three
assertions read a sink once before the relay had moved (slice 1's discharge phase,
slice 3's network sink, and `set_energy`'s exact-equality poll against a cell that a
cable drains every tick) — all three are now poll-based (`set_floor`, sink re-reads),
and the two flaky runs that exposed them are kept in
`evidence/powah-e2e/` history only through the logs they overwrote.

### Pin findings this slice measured (for the next port)

1. **A cell pair always nets zero.** A cell gives `min(transfer, stored)` and accepts
   only `min(its own insert cap, room)`, and both push; a higher-tier source's extra
   transfer is truncated by the lower-tier sink's insert cap and returned the same
   tick. Any assertion of a tier's transfer must therefore be a declaration test (the
   readout and a unit test on `battery_of`), never a net flow between two cells.
2. **`/powah set` on a source that feeds something cannot be polled for equality** —
   the drain starts the same tick (measured 99,500 by the next chat round trip). A
   `set_floor` (≥ a floor) poll is the right shape; the conservation assertion carries
   the exact total.
3. **A sink read can precede the move the source poll waited for** by one round trip
   (measured: sink 0 at t=552, source 500 at t=557). Poll the sink too.
4. `/powah info`'s cell line now carries the declared transfer, so a harness can
   assert a tier's declaration without a second command.

Version: **0.2.3 → 0.2.4** (one patch). The published `[[versions]]` record keeps
0.2.3 and gains 0.2.4 at publish time — this port does not touch the registry entry.

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices

## Slice 5 — the Energizing Orb, the rod ladder and the materials (2026-11-21)

### The slice, quoted from this port's own gaps

`## Slice 4 — the energy cell ladder`, §"What it ships":

> The five other recipes need `powah:steel_energized` / `powah:crystal_*`, which only the Energizing Orb
> makes; they are not carried and are recorded `blocked(energizing-orb)` in `notes.md`.

and its coverage row, which every one of the five cells carries:

> **blocked(energizing-orb)** — they need `powah:steel_energized` /
> `powah:crystal_{blazing,niotic,spirited,nitro}`, all made by the unported Energizing Orb

**Chosen:** the **Energizing Orb** (a block entity with seven slots and a recipe-gated buffer), the
**seven energizing rods** (each an energy capability whose tick fills an orb within
`energizing_range`) and the **six energizing recipes** that make the five materials, plus the
crafting recipes they unblock: the five cell tiers slice 4 left blocked, the large capacitor, the
five capacitor tiers and the rod ladder's own recipes. **Why it fits `machines`:** every piece is
a shape the port already has — a block entity type with an idle tick, a declared block container,
a declared block energy buffer, and the `level_tick_post` queue for a move one block entity makes
into another. No new hook family, no renderer, no GUI, no entity, no worldgen.

**One brief number does not survive the artefact.** The brief asked for a *starter* rod charged to
20,000 FE to fill a 120,000 FE recipe and then fall by exactly 120,000. The jar's own numbers
(`DefaultEnergies`) make the starter rod **10,000 FE and 100 FE/t** — it cannot hold, let alone
give, 120,000 — so the port carries the **whole rod ladder** (the same shape as slice 4's cells)
and the crystal_blazing leg uses the **nitro** rod: charged to 200,000 it gives 200,000, the orb
takes exactly its recipe's 120,000, and the rod is left at 80,000 — the exact fall the brief asked
for, at the tier that can express it. The starter rod keeps its own exact leg (10,000 → 0), which
is also the first exact read of the ladder (`/powah set 20000` clamps at 10,000).

### What it ships

Rust:

| File | Change |
|---|---|
| `src/runtime.rs` | `RodTier` + `rod_tier` (× 10,000 capacity, × 100 transfer), `EnergizingRecipe` + `energizing()` (the jar's six rows), `matches`/`energize` (a bag of filled slots), `orb_step` (the recipe-gated buffer: adopt, clear on a changed recipe, done at the capacity), `orb_fill`, `rod_cooldown`; `ENERGIZING_RANGE` 4, `ORB_SLOTS` 7, `ORB_MAX_ENERGY` 20,000,000, `ROD_WARMUP` 20. 9 new tests |
| `src/powah.rs` | `Kind::Orb` + `Kind::Rod(tier)`, `RODS`/`ORB`, `orb_container` (7 slots: 0 out, 1–6 in), `orb_battery` (take-only 20M), `rod_battery`, `orb_line`/`rod_line`, `orb_tick`, `rod_tick` (link search + warm-up + exact move), `OrbPart`; `/powah info` and `/powah set` learn both |
| `src/lib.rs` | the orb's container and buffer, seven rod buffers, the orb's type with `OrbPart`, seven idle rod types (17 block entity types in all) |

Content: see [`CONTENT.md`](CONTENT.md) §"Slice 5" — 9 blocks (orb, seven rods, blazing crystal
block), 11 plain items (five materials, six capacitors), 21 recipes with their advancements, the
loot/tags/tab entries, and `tools/carry_slice5.py`, which reproduces the 160 files from the raw
carry (`--check` green).

### The readouts this slice adds

```text
Energizing Orb at 0,100,20: 0/10000 FE (recipe powah:steel_energized x2, inputs 2, t=120)
Energizing Orb at 0,100,20: 0/0 FE (no recipe, t=140)
Energizing Rod at 0,101,20: 10000/10000 FE (100 FE/t a side, orb 0,100,20, t=120)
Energizing Rod at 0,101,20: 0/20000000 FE (200000 FE/t a side, no orb, t=40)
```

### TDD: the red proof, quoted

The rod ladder, the recipe table, the matcher, `orb_step`, `orb_fill` and `rod_cooldown` were
declared `todo!()` (with `rod_of`/`orb_container`/`orb_battery`/`rod_battery`/`orb_line`/
`rod_line` in the glue) and the tests written against them, then run
(`ports/_shared/evidence/powah/slice5-test-red.log`):

```text
---- runtime::tests::every_energizing_rod_tier_scales_upstreams_numbers stdout ----
thread 'runtime::tests::every_energizing_rod_tier_scales_upstreams_numbers' (296590) panicked at src/runtime.rs:299:5:
not yet implemented: the energizing rod tier ladder

---- runtime::tests::every_energizing_recipe_is_the_jars_own_row stdout ----
thread 'runtime::tests::every_energizing_recipe_is_the_jars_own_row' (296589) panicked at src/runtime.rs:322:5:
not yet implemented: the energizing recipe table
...
---- powah::tests::every_energizing_rod_tier_is_one_of_the_ladder_with_its_own_buffer stdout ----
thread 'powah::tests::every_energizing_rod_tier_is_one_of_the_ladder_with_its_own_buffer' (296562) panicked at src/powah.rs:238:5:
not yet implemented: the energizing rod block ids

test result: FAILED. 34 passed; 7 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

The orb's own rule had a second red of its own (`not yet implemented: the orb's recipe-gated
buffer rule`, `39 passed; 3 failed`) taken before `orb_step` was written. After the
implementation: **42 passed, 0 failed** (`sdk-test.log`).

### Coverage (parity rule, the family this slice adds)

| Family (from the jar) | Status |
|---|---|
| the orb's seven slots (0 result + 6 inputs) | **tested** — `orb_container`'s declaration is pinned by a unit test; in game `item replace block … container.1..4` fills them, the inputs are consumed, and slot 0 holds the result |
| the orb's recipe-gated buffer | **tested** — the buffer's capacity is the matched recipe's energy, read back by `/powah info` (`0/10000`, `0/120000`, `0/300000`, `0/1000000`, `0/20000000`); a changed recipe clears it (`orb_step` unit test) |
| the orb with no rod / with no recipe | **tested** — the no-rod orb (10,100,20) and the stick-fed orb (20,100,20) produce nothing after 60 ticks, and the second orb's rod keeps its 5,000 FE |
| `energizing_rod_{starter..nitro}` | **tested** — the starter tier's exact leg; the nitro tier's four legs; all seven placed and mined for their drop, their buffers pinned by unit tests |
| a rod's energy capability | **tested** — `/powah set` writes and `/powah info` reads each tier's capacity (`20000` clamps at 10,000) |
| the energizing recipes (6) | **tested** — steel (10,000 → 2), blazing from powder and from a blaze rod (120,000 → 1), niotic (300,000), spirited (1,000,000), nitro (20,000,000 → 16); the rod's exact fall is asserted in each |
| the orb, every rod and the material block | **tested** — placed, read back, mined in survival, each drops itself |
| the orb, seven rods, the large capacitor, five capacitor tiers, five cells, the material block and its 9-back recipe | **tested** — `tools/craft_check.py` crafts all 21 in a real table: output id + count, every ingredient consumed, two off-pattern controls |
| the orb's GUI/particles, the rod's wrench link, `keepEnergy()` block items, a rod's facing-only energy side, the client renderers | **not ported** — recorded in `## Deviations slice 5 adds` |
| the `powah:energizing` recipe *files* | **not carried** — a custom recipe type the pin cannot load; the six rows live in `runtime::energizing()` and are matched per filled slot |

### In-game (parity rule)

The launcher build (`ports/_shared/evidence/powah-e2e/launcher-build-0.2.5.log`) shows
`Compiling powah (1 of 1)`; the main leg is `ports/_shared/evidence/powah-e2e/e2e-launcher.log`,
`PASS` + `EXIT:0`, quoted (the `{"ok":true…}` lines are the `ctl` acknowledgements, dropped):

```text
== slice 5: the starter rod fills the orb with exactly the recipe's 10,000 FE ==
  block(0,100,20) = powah:energizing_orb
  block(0,101,20) = powah:energizing_rod_starter
  info(0,100,20): Energizing Orb at 0,100,20: 0/10000 FE (recipe powah:steel_energized x2, inputs 2, t=856)
  info(0,101,20): Energizing Rod at 0,101,20: 0/10000 FE (100 FE/t a side, orb 0,100,20, t=861)
  set(0,101,20) reply: Energy set to 10000 FE.
  output(0,100,20) = {"count":2,"item":"powah:steel_energized"}
  rod after: Energizing Rod at 0,101,20: 0/10000 FE (100 FE/t a side, orb 0,100,20, t=980)
  info(0,100,20): Energizing Orb at 0,100,20: 0/0 FE (no recipe, t=988)
== slice 5: an orb with no rod in range produces nothing ==
  info(10,100,20): Energizing Orb at 10,100,20: 0/10000 FE (recipe powah:steel_energized x2, inputs 2, t=1080)
== slice 5: a rod beside an orb with no recipe keeps its charge ==
  set(20,101,20) = Energizing Rod at 20,101,20: 5000/20000000 FE (200000 FE/t a side, orb 20,100,20, t=1108)
  info(20,101,20): Energizing Rod at 20,101,20: 5000/20000000 FE (200000 FE/t a side, orb 20,100,20, t=1174)
  info(20,100,20): Energizing Orb at 20,100,20: 0/0 FE (no recipe, t=1181)
== slice 5: a nitro rod makes a blazing crystal at exactly 120,000 FE ==
  info(0,100,28): Energizing Orb at 0,100,28: 0/120000 FE (recipe powah:crystal_blazing x1, inputs 4, t=1224)
  set(0,101,28) reply: Energy set to 200000 FE.
  output(0,100,28) = {"count":1,"item":"powah:crystal_blazing"}
  info(0,101,28): Energizing Rod at 0,101,28: 80000/20000000 FE (200000 FE/t a side, orb 0,100,28, t=1255)
  info(0,100,28): Energizing Orb at 0,100,28: 0/0 FE (no recipe, t=1261)
== slice 5: the jar's second blazing recipe, one blaze rod ==
  set(0,101,28) reply: Energy set to 200000 FE.
  output(0,100,28) = {"count":1,"item":"powah:crystal_blazing"}
  info(0,101,28): Energizing Rod at 0,101,28: 80000/20000000 FE (200000 FE/t a side, orb 0,100,28, t=1305)
== slice 5: the niotic, spirited and nitro crystals ==
  info(0,100,28): Energizing Orb at 0,100,28: 0/300000 FE (recipe powah:crystal_niotic x1, inputs 1, t=1324)
  set(0,101,28) reply: Energy set to 400000 FE.
  output(0,100,28) = {"count":1,"item":"powah:crystal_niotic"}
  info(0,101,28): Energizing Rod at 0,101,28: 100000/20000000 FE (200000 FE/t a side, orb 0,100,28, t=1354)
  set(0,101,28) reply: Energy set to 1000000 FE.
  output(0,100,28) = {"count":1,"item":"powah:crystal_spirited"}
  info(0,101,28): Energizing Rod at 0,101,28: 0/20000000 FE (200000 FE/t a side, orb 0,100,28, t=1398)
  info(0,100,28): Energizing Orb at 0,100,28: 0/20000000 FE (recipe powah:crystal_nitro x16, inputs 4, t=1429)
  set(0,101,28) reply: Energy set to 20000000 FE.
  output(0,100,28) = {"count":16,"item":"powah:crystal_nitro"}
  info(0,101,28): Energizing Rod at 0,101,28: 0/20000000 FE (200000 FE/t a side, orb 0,100,28, t=1554)
```

The exact numbers are all there: the starter rod gives exactly its 10,000 and stands at 0 (the
recipe is 10,000), the nitro rod falls 200,000 → 80,000 on a 120,000 recipe (exactly the recipe's
energy), 400,000 → 100,000 on niotic (300,000), 1,000,000 → 0 on spirited, and 20,000,000 → 0 on
the nitro recipe, which produced its 16 crystals. Every completion clears the inputs (`0/0 FE (no
recipe)`), and the two controls are silent where they must be.

Legs (all in `ports/_shared/evidence/powah-e2e/`):

| Leg | Command | Result |
|---|---|---|
| main | `build-slot.sh run --step e2e -- bash tools/e2e.sh` | **PASS** + `EXIT:0` (`e2e-launcher.log`) |
| control (no package) | `GAME=$CONTROL_GAME …` | `FAIL: expected powah:furnator_starter at 0,100,0, got 'minecraft:air'` + `EXIT:1` (`e2e-control.log`) |
| control (ids), inside the main run | the control leg | `Unknown block type 'powah:energizing_orb'`, `'powah:energizing_rod_nitro'`, `Unknown item 'powah:crystal_blazing'` |
| selftest (slice 1's flip) | `SELFTEST=1 …` | `FAIL: SELFTEST flipped: the unfuelled furnator made no FE, as it should` + `EXIT:1` (`e2e-selftest.log`) |
| selftest (this slice's flip) | `SELFTEST=orb …` | `FAIL: expected /powah info to say '200000/20000000 FE', got 'Energizing Rod at 0,101,28: 80000/20000000 FE …'` + `EXIT:1` (`e2e-selftest-orb.log`) |
| log gate | `check-e2e-logs.py powah-e2e --strict` | `logs=12 ok=12`, exit 0 |

`SELFTEST=orb` is the counterfactual of the exact-fall assertion: a run that claims the rod kept
its whole charge must fail, and does.

### Deviations slice 5 adds

1. **The `c:` tags become the vanilla member they resolve to.** 26.3 has no NeoForge common
   tags, so `c:glass_blocks` is `minecraft:glass`, `c:storage_blocks/quartz` is
   `minecraft:quartz_block`, `c:rods/blaze` is `minecraft:blaze_rod`, and the six energizing
   recipes accept the vanilla ids (`minecraft:iron_ingot`, `minecraft:gold_ingot`,
   `minecraft:diamond`, `minecraft:emerald`, `minecraft:nether_star`,
   `minecraft:redstone_block`, `minecraft:blaze_powder`). A modded iron ingot therefore does not
   energize; the tags are recorded here so a later slice can declare `powah:` tags and read them.
2. **The orb's buffer capacity is a declaration, not per-recipe.** The capability is declared once
   at 20,000,000 (the largest recipe) because a block's buffer is static; the *rule* still uses
   the matched recipe's energy, and the tick clears anything a foreign insert left in an orb with
   no recipe (`onSlotChanged`).
3. **A rod's energy side is every face.** Upstream's `isEnergyPresent` answers only for the face
   its `facing` state names; a `block_energy` declaration cannot vary by state, so all six faces
   are open.
4. **The orb completes on its own tick, one tick after the fill.** Upstream's `fillEnergy`
   produces inside the rod's call; the port's rod writes the orb's buffer at `level_tick_post` and
   the orb produces on its next tick (invisible through a poll, but it is one tick of latency).
5. **An input slot has no stack limit of 1.** `ModContainer` declares slots, not per-slot limits,
   so four blaze powder in one slot are one filled slot (which matches one ingredient and
   therefore no recipe) — the same recipe result as upstream's slot limit, reached from the other
   side.
6. **`/powah set` on the orb clamps at the matched recipe's energy**, and at 0 with no recipe:
   upstream's buffer capacity *is* the recipe's energy, so this is the faithful clamp rather than
   a harness invention.
7. **No `keepEnergy()`:** breaking an orb or a rod loses its buffer, as slice 1's machines do.

**Upstream defects fixed:** none found — the energizing numbers, the bag match and the warm-up
ticker are internally consistent. Two *port* hazards were found and closed while measuring: the
recipe-change reset would have destroyed a fill written in the same tick the inputs arrived
(hence `orb_step`'s "no recorded recipe adopts the energy"), and a rod's whole-compound write
would have clobbered a cable's move into it in the same post phase (the rod now writes only the
keys it changed).

### Pin findings this slice measured (for the next port)

1. **A mod block entity's container is grown by the command path.** `item replace block …
   container.N` fills slot N and grows the entity's `items`; `ctl container` then reads it back.
   Seven slots work as declared.
2. **A rod may fill on the tick its orb's inputs arrive** if its warm-up is already at 20 —
   anything reading the orb's buffer must treat "energy but no recorded recipe" as the recipe's,
   not as a mismatch.
3. **A 20,000,000 FE recipe is 100 fills at a nitro rod's 200,000 a tick**, i.e. ~120 ticks after
   the warm-up: a poll budget under 150 ticks would time out.
4. `/powah info`'s orb line carries the recipe **and** the input count, so a harness can assert
   the match before any energy moves.
5. **`/powah set` on a machine that feeds something cannot be polled for equality** — the drain
   starts the same tick. For a rod the command's own reply (`Energy set to 10000 FE.`) is the
   exact read; `ctl`'s `messages` list is **newest first**, so the newest matching line is the
   first, not the last (a `tail -1` reads the oldest and cost one run its assertion).
6. **A `max_stack_size: 1` item cannot fill two crafting cells from one pickup.** Right-clicking
   with an empty cursor places nothing, and the cells, the rods and the orb all stack to one, so
   a checker must pick the item up once per cell (measured: the hardened cell's grid had slot 105
   empty and read as "the recipe does not craft").
7. **`interact` on a crafting table can silently miss** ("out of reach", "no face … visible"),
   leaving the player's inventory open; every click then lands in the wrong slots and the result
   slot reads empty — which looks exactly like a broken recipe. A checker must assert the table's
   result slot exists before it fills the grid, and retry the interact (this cost the first two
   runs of this slice 12 and 8 false recipe failures).

Version: **0.2.4 → 0.2.5** (one patch). The published `[[versions]]` record keeps 0.2.4 and gains
0.2.5 at publish time — this port does not touch the registry entry.

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices

## Slice 6 — the Furnator and Energy Cable tier ladders (2026-11-21)

### The slice, quoted from this port's own gaps

`horizon-mod.toml` (0.2.5), the `description` the package ships:

> Not yet ported: the other six
> furnator and cable tiers, the creative cell, the generators, reactors, item
> charging, side config, redstone modes and the GUIs.

and `## Slice 4 — the energy cell ladder` §"The slice, quoted from this port's own gaps",
which names the two ladders and why they were left:

> the other six **furnator/cable** tiers are the same shape as this slice but their rule
> is a rate into a buffer (a bound), not an exact readout.

**Chosen:** the six remaining **Furnator** tiers (`basic`, `hardened`, `blazing`,
`niotic`, `spirited`, `nitro`) and the six remaining **Energy Cable** tiers — the
completion of the two ladders slice 1 opened, and the last families the package's own
description listed before the machines it does not carry. **Why it fits `machines`:** a
furnator is the shape the port already runs (one fuel slot, a declared block energy
buffer, a tick that burns carbon into a buffer) with the tier's numbers; a cable tier is
the relay the port already runs with a different rate and no connection across tiers. No
new hook family, no renderer, no GUI, no entity, no worldgen.

**Not chosen (and why):** the **reactors** need `powah:uraninite`, and every source of it
is worldgen the port does not carry (`#c:ores/uraninite`) or an energizing recipe whose
input is uraninite — so no reactor content is reachable in a 26.3 world; recorded
`blocked(uraninite-worldgen)`. The generators (Magmator/Thermo/Solar) still need a mod
block entity fluid tank the pin cannot fill and sky visibility, and the creative
cell/ender gates/item charging/side config/GUIs remain slice 1's recorded gaps.

### What it ships

Rust:

| File | Change |
|---|---|
| `src/runtime.rs` | `FurnatorTier { generation, capacity, transfer }` + `furnator_tier` (× 20 / × 20,000 / × 80), `CableTier { transfer }` + `cable_tier` (× 500); `Furnator::tick(tier)` takes the tier; 5 new tests |
| `src/powah.rs` | `FURNATORS`/`CABLES` (seven ids + type names each), `furnator_of`/`cable_of`, `Kind::Furnator(tier)`/`Kind::Cable(tier)`, per-tier `furnator_battery`/read/set clamp/push cap, `FurnatorPart` carries its tier, the network walk joins one tier only, the furnator readout names its per-side transfer; 4 new tests |
| `src/lib.rs` | one container + energy declaration and one burning block entity type per furnator tier, one idle type per cable tier (29 block entity types in all) |

Content: see [`CONTENT.md`](CONTENT.md) §"Slice 6" — 12 blocks (6 furnators, 6 cables),
their blockstates/models/item definitions/textures/loot, the jar's 6 furnator recipes and
12 cable recipes (both of its paths per tier) with their advancements, the jar's
`powah:furnators` / `powah:energy_cables` block and item tags, the pickaxe/needs-iron
entries, the cable's visual-arm tag and the creative tab; `tools/carry_slice6.py`
reproduces the 134 files from the raw carry and the jar (`--check` green).

### The readouts this slice adds

```text
Furnator at 6,100,14: 40000000/40000000 FE (comparator 15, carbon 0, burning false, fuel empty, 160000 FE/t a side, t=1144)
Furnator at 10,100,14: 40000000/40000000 FE (comparator 15, carbon 8000, burning true, fuel empty, 160000 FE/t a side, t=1187)
Energy Cable at -4,100,16: no storage, moves up to 2000 FE/t (t=1276)
Energy Cable at -6,100,16: no storage, moves up to 500 FE/t (t=1263)
```

The furnator line gained its per-side transfer (the cells gained theirs in slice 4), so a
tier's declaration is readable in game and pinned by the e2e; the starter's line keeps
its prefix, so every earlier assertion still matches.

### TDD: the red proof, quoted

`furnator_tier`, `cable_tier`, `furnator_of` and `cable_of` were declared with `todo!()`
and the tests written against them, then run
(`ports/_shared/evidence/powah/slice6-test-red.log`):

```text
---- runtime::tests::every_furnator_tier_scales_upstreams_numbers stdout ----
thread 'runtime::tests::every_furnator_tier_scales_upstreams_numbers' panicked at src/runtime.rs:47:5:
not yet implemented: the furnator tier ladder

---- runtime::tests::every_cable_tier_scales_upstreams_numbers stdout ----
thread 'runtime::tests::every_cable_tier_scales_upstreams_numbers' panicked at src/runtime.rs:65:5:
not yet implemented: the energy cable tier ladder

---- powah::tests::every_furnator_tier_is_one_of_the_ladder_with_its_own_buffer stdout ----
thread 'powah::tests::every_furnator_tier_is_one_of_the_ladder_with_its_own_buffer' panicked at src/powah.rs:144:5:
not yet implemented: the furnator block ids

---- powah::tests::every_cable_tier_is_one_of_the_ladder_and_declares_no_buffer stdout ----
thread 'powah::tests::every_cable_tier_is_one_of_the_ladder_and_declares_no_buffer' panicked at src/powah.rs:150:5:
not yet implemented: the energy cable block ids

test result: FAILED. 28 passed; 20 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

(The 20 include the existing tests that read a machine's kind or a furnator's compound
through the new tier, which is the same missing table.) After the implementation:
**48 passed, 0 failed** (`slice6-test-green.log`, the same 48 in `sdk-test.log`).

### Ladder (0.2.6, pin `0a875da`)

Every compile step ran in `ports/powah`, `CARGO_INCREMENTAL=0`, inside a
`ports/tools/build-slot.sh run --job W1-powah --port ports/powah --step …` lease. Logs in
`ports/_shared/evidence/powah/`.

| Step | Command | Result |
|---|---|---|
| unit tests | `cargo test --offline` | **48 passed**, 0 failed (`sdk-test.log`) |
| lints | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, 0 warnings (`sdk-clippy.log`) |
| formatting | `cargo fmt --all -- --check` | exit 0, empty (`sdk-fmt.log`) |
| package, twice | `rm -f Cargo.lock; cargo horizon-mod build` ×2, `cmp` | **byte-identical, 293,821 B** (`sdk-build{,-second}.log`) |
| registry check | `cargo horizon-mod check powah.hmod` | **exit 0**, `powah 0.2.6 (native-mod, format native-1/0a875da…/e59da90…, for 26.3)`, `293821 bytes, SHA-256 3ab1b5b46df0285f14f5d8fd47367e1cd053cbf96e671bfe8e3bb2eca30ed358, unsigned` (`sdk-check.log`) |
| fits the pin | `cargo horizon-mod fits powah.hmod --build 0a875da` | `0a875da  made for`, exit 0 (`sdk-fits.log`) |
| fits **another** build | `cargo horizon-mod fits powah.hmod --build 0b14f8a` | `0b14f8a  fits`, exit 0 (`sdk-fits-nightly.log`) |
| package gate | `python3 ports/tools/verify-sdk-package.py powah` | **1/1**, runtime byte-identical (`b076ea834d71`), no packed `Cargo.lock`, **422 entries** (`sdk-verify.log`) |
| carried-data dialect | `python3 ports/tools/check_261_dialect.py powah.hmod` | exit 0: 132 recipe/loot files, every one in the 26.3 dialect, every `requires_tool` block in a mineable tag |

In-game legs (launcher build `native-0a875da-9952c6153ae1`, `Compiling powah (1 of 1)` in
`evidence/powah-e2e/launcher-build-0.2.6.log`; logs in
`ports/_shared/evidence/powah-e2e/`):

| Leg | Command | Result |
|---|---|---|
| main | `launcher-game.sh build powah` + `build-slot.sh run --step e2e` | **PASS** + `EXIT:0` (`e2e-launcher.log`) |
| control (whole scenario, no package) | `GAME=$CONTROL_GAME …` | `FAIL: expected powah:furnator_starter at 0,100,0, got 'minecraft:air'` + `EXIT:1` (`e2e-control.log`) |
| control (ids), inside the main run | the control leg | `Unknown block type 'powah:furnator_nitro'`, `'powah:energy_cable_nitro'`, and the slice-5 ids |
| selftest (slice 1's flip, re-run on this package) | `SELFTEST=1 …` | `FAIL: SELFTEST flipped: the unfuelled furnator made no FE, as it should` + `EXIT:1` (`e2e-selftest.log`) |
| selftest (this slice's clamp) | `SELFTEST=furnator …` | `FAIL: expected powah:furnator_nitro to hold 20000/20000 FE, got 'Furnator at 6,100,14: 40000000/40000000 FE …'` + `EXIT:1` (`e2e-selftest-furnator.log`) |
| selftest (this slice's rate) | `SELFTEST=furnator-rate …` | `FAIL: expected 8000 carbon left after one 40,000 tick of a 48,000 coal, got 47980: Furnator at 10,100,14: 40000000/40000000 FE (… carbon 8000 …)` + `EXIT:1` (`e2e-selftest-furnator-rate.log`) |
| log gate | `check-e2e-logs.py powah-e2e --strict` | `logs=14 ok=14`, exit 0 |

Quoted from `e2e-launcher.log` (one command per tier; every tier clamps at its own
capacity and names its own per-side transfer):

```text
== slice 6: every furnator tier holds its own capacity ==
  info(-6,100,14): Furnator at -6,100,14: 20000/20000 FE (comparator 15, carbon 0, burning false, fuel empty, 80 FE/t a side, t=1032)
  info(-4,100,14): Furnator at -4,100,14: 80000/80000 FE (comparator 15, carbon 0, burning false, fuel empty, 320 FE/t a side, t=1049)
  info(-2,100,14): Furnator at -2,100,14: 200000/200000 FE (comparator 15, carbon 0, burning false, fuel empty, 800 FE/t a side, t=1067)
  info(0,100,14): Furnator at 0,100,14: 800000/800000 FE (comparator 15, carbon 0, burning false, fuel empty, 3200 FE/t a side, t=1084)
  info(2,100,14): Furnator at 2,100,14: 2000000/2000000 FE (comparator 15, carbon 0, burning false, fuel empty, 8000 FE/t a side, t=1102)
  info(4,100,14): Furnator at 4,100,14: 8000000/8000000 FE (comparator 15, carbon 0, burning false, fuel empty, 32000 FE/t a side, t=1123)
  info(6,100,14): Furnator at 6,100,14: 40000000/40000000 FE (comparator 15, carbon 0, burning false, fuel empty, 160000 FE/t a side, t=1144)
== slice 6: a nitro furnator makes its own 40,000 FE a tick ==
  set(10,100,14) = Furnator at 10,100,14: 39960000/40000000 FE (comparator 14, carbon 0, burning false, fuel empty, 160000 FE/t a side, t=1158)
  item replace block(10,100,14) container.0 <- minecraft:coal x1
  nitro: Furnator at 10,100,14: 40000000/40000000 FE (comparator 15, carbon 8000, burning true, fuel empty, 160000 FE/t a side, t=1187)
== slice 6: a starter furnator fills its own buffer at its own rate ==
  starter: Furnator at 12,100,14: 860/20000 FE (comparator 0, carbon 47140, burning true, fuel empty, 80 FE/t a side, t=1250)
== slice 6: every energy cable tier carries its own rate ==
  info(-6,100,16): Energy Cable at -6,100,16: no storage, moves up to 500 FE/t (t=1263)
  info(-4,100,16): Energy Cable at -4,100,16: no storage, moves up to 2000 FE/t (t=1276)
  info(-2,100,16): Energy Cable at -2,100,16: no storage, moves up to 5000 FE/t (t=1290)
  info(0,100,16): Energy Cable at 0,100,16: no storage, moves up to 20000 FE/t (t=1304)
  info(2,100,16): Energy Cable at 2,100,16: no storage, moves up to 50000 FE/t (t=1317)
  info(4,100,16): Energy Cable at 4,100,16: no storage, moves up to 200000 FE/t (t=1330)
  info(6,100,16): Energy Cable at 6,100,16: no storage, moves up to 1000000 FE/t (t=1343)
== slice 6: a cable network joins its own tier and no other ==
  behind the starter cable: Energy Cell at 3,100,18: 0/1000000 FE (comparator 0, 1000 FE/t a side, t=1445)
  with the basic cable: Energy Cell at 3,100,18: 1000/1000000 FE (comparator 0, 1000 FE/t a side, t=1469)
```

The rate assertion is exact, not a bound: the buffer is left exactly **40,000** of room
(`/powah set 39960000`) and one coal is 48,000 carbon, so one nitro tick fills the room and
leaves **exactly 8,000** carbon — a 20 FE/t tick would leave 47,980 and the buffer would
still be 20 short. The starter's line is the conservation counterpart: `860 + 47,140 =
48,000` — every FE in the buffer came out of the carbon, and the starter tier has not
finished the coal in 42 ticks. The cable's cross-tier phase is fail-capable both ways: the
sink is **0** while a starter cable sits between two basic ones, and **1,000** once that
cable is basic.

### Coverage (parity rule, the family this slice adds)

| Family (from the jar) | Status |
|---|---|
| `furnator_{basic,hardened,blazing,niotic,spirited,nitro}` block + item | **tested** — each placed, `/powah set 2000000000` clamps at its own capacity (80k…40M) and the readout names its per-side transfer (320…160,000 FE/t); each mined in survival, drops itself |
| the six new furnator recipes | **tested** — crafted in a real table: output id + count, every ingredient consumed (`craft_check.py`) |
| a tier's generation | **tested** — the nitro tier's exact 40,000 FE tick (carbon 48,000 → 8,000 with exactly 40,000 of room) and the starter tier's 840 FE in 42 ticks with `energy + carbon = 48,000` |
| `energy_cable_{basic,hardened,blazing,niotic,spirited,nitro}` block + item | **tested** — each placed and `/powah info` reads back its own rate (2,000…1,000,000 FE/t); each mined in survival, drops itself |
| the cable network's tier rule (`CableNet`: different tiers never connect) | **tested** — a cell behind a starter cable on a basic run stays at 0; replacing that cable with a basic one delivers 1,000 FE |
| both of the jar's cable recipes per tier (12) | **tested** — crafted in a real table: 6-cable upgrade path and 12-cable alternate, output id + count, every ingredient consumed; the nitro cable's shape with a stick in place of its capacitor crafts nothing |
| `powah:furnators` / `powah:energy_cables` block + item tags | **tested** — carried from the jar, validated by `cargo horizon-mod check`; the e2e gives, places and mines every member |
| the creative tab (47 items) | **not asserted in game** — no `ctl` command reads a tab; `check` validates it and every id is giveable/placeable |
| reactors (`uraninite`) | **blocked(uraninite-worldgen)** — `powah:uraninite` has no source: its ore tag `#c:ores/uraninite` is worldgen this port does not carry, and its energizing recipe takes uraninite itself, so no reactor recipe can be reached |
| generators (Magmator/Thermo/Solar), creative cell, ender gates, item charging, side config, redstone modes, GUIs | **not ported** — slice 1's recorded gaps 1–9, 11 |

### Deviations slice 6 adds

1. **The furnator readout gained its per-side transfer** so the declared rate is readable
   in game (the cells' line gained theirs in slice 4). The starter's prefix is unchanged.
2. **`needs_iron_tool` gains the twelve new blocks.** Upstream's own file is only
   `#c:ores/uraninite`; the port's starter machines were already in it (slice 1), so the
   tiers follow their starter rather than upstream's tag.
3. **The cable's visual-arm tag gains the new furnators and cables.** Upstream joins any
   FE block; a content tag can only name blocks, and the higher cell tiers were not added
   in slice 4 either — their arms stay unrendered, a visual gap only.
4. **A cable network still spends one cable's rate, not one per cable** (slice 3's
   deviation). Upstream's `CableNet` forwards one insertion through the run; the port
   plans the whole network with the tier's single rate.

**Upstream defects fixed:** none found — the tier tables are internally consistent with
`DefaultEnergies`, and the "different tiers never connect" rule is the artefact's own. The
only defect found while measuring was in **this port's harness**: the slice-1 discharge
phase asserts a *source* cell that oscillates (0/1000 with its sink every two ticks) by
reading it eight times; one run read the same stale `t=662` line eight times after a
dropped chat reply and failed a phase this slice did not touch. The main leg and both
selftests passed it on re-run; the phase is left as it is (its own log is unchanged) and
this is recorded as a known harness flake, not a mod defect.

### Pin findings this slice measured (for the next port)

1. **A tier clamp is read with the same command as another tier's** — the furnator's
   `capacity` comes straight from `generatorCapacity`, so `/powah set 2000000000` on
   seven blocks gives seven exact readouts in one phase.
2. **A rate is provable in game when the buffer is given exactly the rate's room.** A
   chat round trip (~40 ticks) is far longer than a nitro tick, so a rate cannot be
   measured by Δenergy/Δt; leave room = generation and read the carbon the one tick took.
3. **`/powah set` on a furnator that then burns cannot be polled for the set value** —
   the burn starts the same tick; read the post-burn line instead (the carbon field).
4. **A cross-tier cable row is a clean control**: the sink is exactly 0 until the tier
   matches, so the join rule is assertable with one `info`.

Version: **0.2.5 → 0.2.6** (one patch). The published `[[versions]]` record keeps 0.2.5
and gains 0.2.6 at publish time — this port does not touch the registry entry.

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices

### Coverage table in the checker's shape (`check-coverage.py`)

| Family | Status | Evidence |
|---|---|---|
| `block` | tested | every block placed and read back, from the three slice-1 machines to the seven furnator tiers, the seven cell tiers, the seven cable tiers, the energizing orb, the seven rods and the blazing crystal block, and their loot |
| `item` | tested | every item given and counted; the machines', cables' and furnators' items placed |
| `block_entity` | tested | each furnator tier burns at its own generation into its own capacity (the nitro tier's exact 40,000 tick); a cell holds its tier's capacity; each cable tier relays at its own rate and a network joins one tier only; the orb holds seven slots and a recipe-gated buffer, and a rod fills it exactly (energy only moves through the block entities' ticks) |
| `loot_table` | tested | every machine, furnator tier, cell, cable tier, orb, rod and material block mined in survival in the loot leg and asserted to drop itself |
| `recipe:minecraft:crafting_shaped` | tested | `craft_check.py` crafts each of the 43 shaped recipes in a real table (the five previously blocked cells, the six furnator tiers and the twelve cable recipes included): output id + count, ingredients consumed |
| `recipe:minecraft:crafting_shapeless` | tested | same leg, 8 cases; the paste's lava bucket leaves its bucket, and the material block and its 9-back recipe both work |
| `tag:item` | tested | `powah:energizing_rods`, `powah:furnators` and `powah:energy_cables` name their seven members (the jar's own tags); the e2e gives, places, crafts and mines each of those ids, and `cargo horizon-mod check` validates the tags |
| `tag:block` | tested | the cable's `powah:energy_cable_connects` is the visual-arms tag (the relay is the mechanic and the three-cable run plus the cross-tier control are asserted); the pickaxe/needs-iron tags are what makes the loot leg's drops real |

## Slice 7 — the item-energy family: `keepEnergy` and the item charge slots (2026-11-22)

### The slice, quoted from this port's own gaps

`## Deviations and gaps (slice 1)`, items **4** and **5** — the two lines the 0.2.6 report named as the
next slice:

> **4. No item charging.** Upstream's furnator charges slot 0 and a cell charges
> two slots through `item_energy_buffer`; not ported, so the furnator has one
> slot, not two, and the cell none.

> **5. Breaking a machine loses its FE.** Upstream's `keepEnergy()` makes the
> block item carry the buffer; no energy component/item buffer is registered
> here, and the loot tables drop the plain block.

and the 0.2.6 `description` ("Not yet ported: … item charging …"), which this slice removes from the
list.

**Chosen:** the **item-energy family** — the buffer a machine's own item carries when the machine is
broken (`keepEnergy`: every furnator, cell and rod) and the item-charging slots (`chargeItems`: the
furnator's slot 0, a cell's two). It is the one family in the gap list that needs **no new content**
(every item is already carried) and whose rule is one rule for 21 blocks: a machine's item is an
energy item, and a machine's slots take FE out of its own buffer. **Why it fits `machines`:** it is
the block entity's own state (its saved buffer, its declared container, its tick) plus the item
capability this pin exposes; no renderer, no GUI, no entity and no worldgen.

**Not chosen (and why):** the creative cell, the generators and the reactors remain out (uraninite has
no source in a 26.3 world — `blocked(uraninite-worldgen)`; the generators need a mod fluid tank the
pin cannot fill); side config, redstone modes and the GUIs stay slice 1's recorded gaps.

### What upstream does (read from tag `v6.2.10`, commit `a5a8f25`)

| Fact | Where |
|---|---|
| `keepEnergy()` is true for `FurnatorTile`, `EnergyCellTile` (`!isCreative`) and `EnergizingRodTile`; the orb's and the cable's are false | `block/*/*Tile.java` |
| `writeStorable` writes the buffer **only** when `keepEnergy()`; `storeToStack` puts that compound on the block item as `PowahComponents.STORED_BLOCK_ENTITY_STATE`, and `onPlaced` reads it back | `lib/block/AbstractEnergyStorage.java:62,70`, `lib/block/AbstractTileEntity.java:157,174` |
| the drop is the `playerDestroy` path — `storeToStack(new ItemStack(this))` popped instead of the loot table — and only `keepInventory()` stacks merge (false for these machines, so **no `Items`**) | `lib/block/AbstractBlock.java:113`, `lib/block/AbstractTileEntity.java:167,176` |
| every `EnergyBlockItem` is an energy item (`IEnergyContainingItem`), its `Info` the tier's capacity and the `getTransferType()` limits; the value lives in a **different** component, `PowahComponents.ENERGY_STORED` (`Energy.Item`) | `lib/item/EnergyBlockItem.java`, `lib/logistics/energy/Energy.java:251-305`, `Powah.java:137` |
| the furnator has two slots — 0 chargeables (`Energy.chargeable`), 1 fuel (burn time > 0) — and runs `chargeItems(1)`; a cell has two slots, limit 1, all chargeable, and runs `chargeItems(2)` | `block/furnator/FurnatorTile.java:96,105,114`, `block/energycell/EnergyCellTile.java:56,73` |
| `chargeItems` = `ChargeUtil.chargeItemsInInventory(inv, 0, n, getEnergyTransfer(), energy.getStored())`: per slot `min(transfer, budget left)`, the item's own room applies, and the buffer is debited by exactly what the item took | `lib/block/AbstractEnergyStorage.java:103`, `util/ChargeUtil.java:60` |
| a furnator's transfer type is `EXTRACT`, so its own item only gives; a cell's and a rod's are `ALL` | `lib/block/AbstractEnergyProvider.java`, `AbstractEnergyBlock.java:75` |

### What it ships

Rust only — no content change (every id was already carried):

| File | Change |
|---|---|
| `src/runtime.rs` | `charge_step(stored, transfer, room)`, `carried_energy(energy, capacity) -> Option<i32>`, `restored_energy(asked, capacity)`; 3 tests |
| `src/powah.rs` | `ItemBuffer` + `item_buffer`, `charge_slots`/`kind_charge_slots`, `keep_capacity`, `cell_container` (two slots), the `BlockEntityData` component + `carried_patch` + `asked_energy` + `with_payload_energy`, `charge_items` (the per-slot step), the `ITEM_ENERGY` handler `MachineEnergy`, `on_block_drops`, the `MachineItem` Part (`on_block_placed_by`), `IdleRules` (a cell's two charge slots and the tier clamp on its own tick), `/powah item`; the readout's `charge` field; 7 tests |
| `src/lib.rs` | every machine item registered as an `ItemPart` and — where it holds a buffer — an `ITEM_ENERGY` provider, every cell's two-slot container, `Hooks::block_drops` |
| `horizon-mod.toml`, `Cargo.toml` | version 0.2.7, the description's "item charging" moved from gaps to the slice list |

### The readouts this slice adds

```text
Furnator at 32,100,34: 0/20000 FE (comparator 0, carbon 0, burning false, fuel empty, charge powah:energy_cell_starter 40/1000000, 80 FE/t a side, t=2590)
Energy Cell at 36,100,34: 0/1000000 FE (comparator 0, charge powah:energy_cell_starter 1000/1000000, powah:energy_cell_starter 500/1000000, 1000 FE/t a side, t=…)
```

The `charge` field goes after the fuel (a furnator) or the comparator (a cell), so every earlier
assertion keeps matching its prefix; `/powah item <amount> <pos>` is the operator-only (level 2)
control that leaves an item an exact charge.

### TDD: the red proof, quoted

`charge_step`, `carried_energy` and `restored_energy` were declared with `todo!()`, and the tests were
written against them before any glue
(`ports/_shared/evidence/powah/slice7-test-red.log`):

```text
thread 'runtime::tests::a_charge_step_moves_the_transfer_the_balance_or_the_room' panicked at src/runtime.rs:159:5:
not yet implemented: the item charge step
thread 'runtime::tests::a_broken_machine_carries_its_buffer_only_when_it_holds_one' panicked at src/runtime.rs:168:5:
not yet implemented: the carried buffer of a broken machine
thread 'runtime::tests::a_placed_machine_takes_its_item_s_buffer_clamped_to_its_tier' panicked at src/runtime.rs:175:5:
not yet implemented: the restored buffer of a placed machine
thread 'powah::tests::a_carried_payload_is_the_machine_s_block_entity_data_and_carries_no_items' panicked at src/runtime.rs:168:5:
not yet implemented: the carried buffer of a broken machine
thread 'powah::tests::a_payload_is_read_back_and_believed_only_within_the_tier' panicked at src/runtime.rs:168:5:
not yet implemented: the carried buffer of a broken machine
thread 'powah::tests::the_harness_commands_are_operator_only' panicked at src/powah.rs:1809:9:
... (the third usage, `/powah item`, is not declared yet)
thread 'powah::tests::the_info_lines_are_the_ones_the_e2e_asserts' panicked at src/powah.rs:2118:9:
... (the readout has no `charge` field yet)
test result: FAILED. 51 passed; 7 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

After the implementation: **58 passed, 0 failed** (`sdk-test.log`, the same 58 in `slice7-test-green.log`).

### Ladder (0.2.7, pin `0a875da`)

Every compile step ran in `ports/powah`, `CARGO_INCREMENTAL=0`, inside a
`ports/tools/build-slot.sh run --job W1-powah --port ports/powah --step …` lease. Logs in
`ports/_shared/evidence/powah/`.

| Step | Command | Result |
|---|---|---|
| unit tests | `cargo test --offline` | **58 passed**, 0 failed (`sdk-test.log`) |
| lints | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, 0 warnings (`sdk-clippy.log`) |
| formatting | `cargo fmt --all -- --check` | exit 0, empty (`sdk-fmt.log`) |
| package, twice | `rm -f Cargo.lock; cargo horizon-mod build` ×2, `cmp` | **byte-identical, 305,679 B** (`sdk-build{,-second}.log`) |
| registry check | `cargo horizon-mod check powah.hmod` | **exit 0**, `powah 0.2.7 (native-mod, format native-1/0a875da…/e59da90…, for 26.3)`, `305679 bytes, SHA-256 8cf04f34e5135e4e07a66a516c42da11c2841e82f82458d63b3ae29a480269d3, unsigned` (`sdk-check.log`) |
| fits the pin | `cargo horizon-mod fits powah.hmod --build 0a875da` | `0a875da  made for`, exit 0 (`sdk-fits.log`) |
| fits **another** build | `cargo horizon-mod fits powah.hmod --build 0b14f8a` | `0b14f8a  fits`, exit 0 (`sdk-fits-nightly.log`) |
| package gate | `python3 ports/tools/verify-sdk-package.py powah` | **1/1**, runtime byte-identical (`b5de81d0b618`), no packed `Cargo.lock`, **422 entries** |

The in-game legs below ran the package as packed **before** this slice's `README.md` was given its
slice-7 paragraph (305,399 B, sha256 `91cca602…`). The shipped artifact is 305,679 B
`8cf04f34…`; compared entry by entry against the launcher's installed copy of the leg's package, the
two are identical in all 422 entries except `README.md` (`src/`, `assets/`, `data/`, the manifest and
the licences are byte-identical, and both pack `src/runtime.rs` `b5de81d0b618…`), so the legs prove
the shipped code.
| carried-data dialect | `python3 ports/tools/check_261_dialect.py powah.hmod` | exit 0: 132 recipe/loot files in the 26.3 dialect |

### In-game legs

Launcher build `native-0a875da-d4ab2c99bd07` (`Compiling powah (1 of 1)` in
`evidence/powah-e2e/launcher-build-0.2.7.log`); logs in `ports/_shared/evidence/powah-e2e/`.

| Leg | Command | Result |
|---|---|---|
| main | `launcher-game.sh build powah` + `build-slot.sh run --step e2e` | **PASS** + `EXIT:0` (`e2e-launcher-0.2.7.log`) |
| control (whole scenario, no package) | `GAME=$CONTROL_GAME …` | `FAIL: expected powah:furnator_starter at 0,100,0, got 'minecraft:air'` + `EXIT:1` (`e2e-control-0.2.7.log`) |
| selftest (slice 1's flip) | `SELFTEST=1 …` | `FAIL: SELFTEST flipped: the unfuelled furnator made no FE, as it should` + `EXIT:1` (`e2e-selftest-0.2.7.log`) |
| selftest (this slice's drop) | `SELFTEST=keep …` | `FAIL: a furnator placed from its own drop read 5000/20000 FE, not the 0 it was broken with` + `EXIT:1` (`e2e-selftest-keep.log`) |
| selftest (this slice's rate) | `SELFTEST=charge …` | `FAIL: the item gained 480 FE over 6 ticks; at 40 FE a tick that is not one charge step a tick (expected 240 +- 40)` + `EXIT:1` (`e2e-selftest-charge.log`) |
| **the previous package** (0.2.6, the parity rule's red) | `GAME=…/powah-0.2.6/horizon-client FUEL_CONTAINER_SLOT=0 …` | `FAIL: the dropped furnator item carries no block entity data: … {count: 1, id: "powah:furnator_starter"}` + `EXIT:1` (`e2e-oldpackage-0.2.6.log`) |
| log gate | `check-e2e-logs.py powah-e2e --strict` | `logs=20 ok=20`, exit 0 |

Quoted from `e2e-launcher-0.2.7.log` (the family's own assertions):

```text
== slice 7: a broken machine's own item carries its buffer ==
  scenario: {"block":"minecraft:air","drops":{"powah:furnator_starter":1},"picked_up":{"powah:furnator_starter":1}}
  SelectedItem: … {components: {"minecraft:block_entity_data": {id: "powah:furnator_starter", energy: 5000}}, count: 1, id: "powah:furnator_starter"}
  placed powah:furnator_starter at 32,100,36
  placed again: Furnator at 32,100,36: 5000/20000 FE (comparator 3, carbon 0, burning false, fuel empty, charge empty, 80 FE/t a side, t=2073)
  the block's own saved energy: 32, 100, 36 has the following block data: 5000
== slice 7: the machine's slots spill, the payload carries none of them ==
  scenario: {"block":"minecraft:air","drops":{"minecraft:stone":1,"powah:furnator_starter":1},"picked_up":{}}
  placed again: Furnator at 32,100,36: 5000/20000 FE (… charge empty, …)
== slice 7: a payload above the tier clamps on placement ==
  doctored item: … {minecraft:block_entity_data": {id: "powah:furnator_starter", energy: 2000000000}} …
  doctored: Furnator at 34,100,36: 20000/20000 FE (comparator 15, …)
== slice 7: a plain item places an empty machine, and a cell keeps its own buffer ==
  plain item: Furnator at 30,100,36: 0/20000 FE (comparator 0, …)
  scenario: {"block":"minecraft:air","drops":{"powah:energy_cell_starter":1},"picked_up":{"powah:energy_cell_starter":1}}
  cell placed again: Energy Cell at 38,100,36: 4000/1000000 FE (comparator 0, charge empty, empty, 1000 FE/t a side, t=…)
== slice 7: what a level-side break carries (measured) ==
  level-side break put 0 furnator item(s) in the inventory
== slice 7: a chargeable item takes exactly the balance below the transfer ==
  charged: Furnator at 32,100,34: 0/20000 FE (… charge powah:energy_cell_starter 40/1000000, 80 FE/t a side, t=2590)
== slice 7: the charge rate is the tier's 80 FE a tick ==
  rate: … 18960/20000 FE (… charge powah:energy_cell_starter 1080/1000000, … t=2608)
  rate: … 18480/20000 FE (… charge powah:energy_cell_starter 1560/1000000, … t=2614)
  dt=6 ticks, item +480 FE, machine -480 FE
== slice 7: an item with no buffer is not charged ==
  stone: Furnator at 32,100,34: 1000/20000 FE (… charge minecraft:stone 0/0, …)
== slice 7: the item's own room caps the step, not the transfer ==
  item(32,100,34) reply: Charge set to 999920 FE on powah:energy_cell_starter.
  full item: … charge powah:energy_cell_starter 1000000/1000000 … 920/20000 FE …
== slice 7: a cell charges two item slots within one budget ==
  cell: Energy Cell at 36,100,34: 0/1000000 FE (comparator 0, charge powah:energy_cell_starter 1000/1000000, powah:energy_cell_starter 500/1000000, 1000 FE/t a side, t=…)
```

Each number is exact and its leg can fail: the 40 is the whole balance (a machine that kept it reads
40, a machine that charged more than it had cannot), `+480` over `dt=6` is 80 a tick against the
game's own clock (the `SELFTEST=charge` run expects 40 and fails), `1000000` with `920` left is the
item's room capping one 80-FE step (a machine that kept charging a full item fails the second read),
`1000`/`500` is one 1,000 FE budget spent in slot order (a per-slot budget would give 1000/1000 and a
machine at 0 still), and `20000` from a 2,000,000,000 payload is the placement clamp.

### Coverage (parity rule, the family this slice adds)

| Family (from the jar) | Status |
|---|---|
| `keepEnergy` on the seven furnators | **tested** — a starter furnator at 5,000 FE broken in survival drops itself carrying `minecraft:block_entity_data {id, energy: 5000}`, placed back reads `5000/20000` and `/data get block … energy` reads 5000; its own charge slot's stone spilled as a separate item, and the placed machine reads `charge empty` |
| `keepEnergy` on the seven cells | **tested** — a starter cell at 4,000 FE broken and placed again reads `4000/1000000` |
| `keepEnergy` on the seven rods | **rule-tested** (`keep_capacity`, `carried_patch`) — the rod's payload and clamp are the cell's; no rod is broken in game this slice |
| the placement clamp | **tested** — an item doctored to 2,000,000,000 FE places a `20000/20000` furnator, and a fresh item places `0/20000` |
| the furnator's charge slot (`chargeItems(1)`, upstream's slot 0) | **tested** — 40 FE from a 40 FE balance, 80 FE a tick over `dt=6`, an item with 80 of room takes one step and leaves 920, a stone takes nothing |
| a cell's two charge slots (`chargeItems(2)`) | **tested** — 1,500 FE split 1,000 / 500 in slot order, machine at 0 |
| the fuel slot at upstream's index 1 | **tested** — the whole slice-1 and slice-6 fuel path runs on `container.1` |
| the machine item as `Capabilities.Energy.ITEM` | **rule-tested** (`item_buffer`, `MachineEnergy` registered per item) — no other mod's charger exists in this leg to drive it |
| reactors / generators / creative cell / side config / redstone / GUIs | `not ported` — slice 1's gaps; the reactors stay `blocked(uraninite-worldgen)` |

### Deviations and upstream defects this slice records

1. **Upstream defect fixed: two stores for one buffer.** Upstream's `EnergyBlockItem` keeps an item's
   charge in `PowahComponents.ENERGY_STORED` (`Energy.Item`), while placement reads only
   `STORED_BLOCK_ENTITY_STATE` — so a machine item charged in a slot (or by a battery) gives the
   machine it becomes **nothing**. The port has one store, the item's
   `minecraft:block_entity_data {id, energy}`: the drop writes it, the `ITEM_ENERGY` handler reads and
   writes it, and placement restores it. A user notices that charging a cell item in a furnator and
   placing it now works.
2. **Upstream defect fixed: a payload is believed only within its tier, and split across a stack.** A
   `/give`d item naming 2,000,000,000 FE on a 20,000 FE furnator places a full machine (upstream's
   `readStorable` restores whatever the compound says), and a stack-scoped payload is divided by the
   stack's count so placing a doctored stack cannot mint FE per item (upstream gives every placed item
   the whole value). This port's machine items stack to 1, so only the doctored case is reachable.
3. **The container declares no per-slot rule.** Upstream's `canInsert` gates slot 0 to chargeables and
   slot 1 to fuel; a container declaration cannot express that at this pin, and a refusal on one slot
   cancels a click on another (HANDOVER §9l, measured in `rftoolsstorage`). The tick decides instead,
   as the fuel slot already did: a slot whose item holds no buffer is left alone (measured: a stone in
   the charge slot takes nothing and the machine keeps its 1,000 FE).
4. **Cells declare a container where they had none** (two slots, every face, no menu) — upstream's
   `EnergyCellTile` has the same two slots.
5. **The furnator's fuel moved from slot 0 to slot 1**, upstream's order; `notes.md`'s slice-1 record
   and the harness's `put_fuel` say so. The harness keeps `FUEL_CONTAINER_SLOT` for the counterfactual
   leg against the 0.2.6 package, whose furnator has one slot.
6. **A cell and a rod clamp their saved buffer on their own tick.** A doctored payload, or a
   `/data merge block`, would otherwise leave a 2,000,000,000 FE cell that the energy capability
   reports raw; the tick writes it back clamped. The furnator already clamped on read.
7. **`/powah item` is a harness control**, operator-only (level 2), like `/powah set`/`info`: it writes
   the charge of the item in a machine's charge slot so a leg can leave an item an exact room.
   Upstream has no such command.
8. **A level-side break drops nothing at this pin.** `/setblock … destroy` on a `needs_iron_tool` block
   gives no tool to the level path, so no item appears at all; the payload's differential is therefore
   a freshly given item (no payload) placing an empty machine, which is the same statement.

### Harness findings this slice measured (for the next machine port)

1. **`ctl collect` walks, and a walk off the arena is a 35-block fall.** Two runs died there: the
   player respawned at the world spawn (`y=64`) with an empty inventory, and every later assertion saw
   `counts: {}`. Walk to the drop's own reported `pos` with `ctl tp` instead.
2. **An item entity's reply carries `pos` (not `position`) and its stack under `item.item`** —
   `ctl break`'s `on_ground` uses `pos` too.
3. **A break's own drop is not reliably picked up**: it lands up to ~1.2 blocks off the block
   (measured positions `32.68,100,35.87` and `-0.08,100,-0.04`), sometimes inside the player's reach
   (`picked_up` set) and sometimes not. `pickup` reads `ctl entities 12 item`, tps onto the item's
   `pos`, then selects its hotbar slot.
4. **`/data get entity @s SelectedItem` is one selection stale**; read `ctl inventory`'s `.held` for
   the held stack, and `/data get block <pos> energy` for the machine's own saved buffer (no mod code
   in the path — the strongest read of the restored value).
5. **`/item replace entity @s hotbar.1 from entity @s inventory.N` moves a stack between the player's
   own slots**, the same stack and payload (a drop that lands in the main inventory is not reachable by
   `ctl select` or `ctl place --item`).
6. **An item inserted beside a charged machine charges on the very next tick**: a leg that wants an
   exact item value must empty the machine first, insert, then set the buffer.
7. **`/powah set` on a machine that charges an item cannot be polled for its own value** (the charge
   starts the same tick); assert the command's reply and read the pair afterwards.

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices

## Slice 8 — the Energy Discharger (2026-11-22)

### The slice, quoted from this port's own gaps

`## Deviations and gaps (slice 1)`, item 11 — the notes' own next-slice authority:

> **11. The other 6 tiers, the creative cell and ~15 machines** are unported.

and `FINDINGS.md` §"Slice 1 boundary", which names the machine:

> Discharger/Hopper/Player Transmitter; item charging (`item_energy_buffer`) and
> energy-carrying block items; side-config GUI and redstone modes (defaults only);

The 0.2.7 report's next-slice line named it as the item-energy family's **mirror**: `EnergyDischargerTile`
is `AbstractEnergyStorage` with `keepEnergy()` and `chargeItems`' inverse, and slice 7's `ITEM_ENERGY`
handler had an `extract` side with no driver. That is this slice.

**Why it fits `machines`:** a discharger is a storage block entity — a declared block energy buffer
(extract-only), a seven-slot container and one tick (`dischargeItemsInInventory` → `extractFromSides`);
no renderer, no GUI, no entity, no worldgen.

**Not chosen (and why):** the creative cell, the generators and the reactors stay out (uraninite has no
source in a 26.3 world — `blocked(uraninite-worldgen)`; the generators need a mod fluid tank the pin
cannot fill); the Energy Hopper (`ChargingConfig`, a charging station over a container) is the other
half of the same family and is left for the next slice; side config, redstone modes and the GUIs stay
slice 1's recorded gaps.

### What upstream does (read from tag `v6.2.10`, commit `a5a8f25`)

| Fact | Where |
|---|---|
| capacity/transfer are `batteryCapacity()`/`batteryTransfer()` — the battery config: starter **1,000,000 FE / 1,000 FE a side**, nitro 2,000,000,000 / 2,000,000 | `config/v2/PowahConfig.java:95` (`devices.dischargers`), `DefaultEnergies` |
| seven slots (`inv.add(7)`), slot limit **1**, insert only a dischargeable item, extract always | `block/discharger/EnergyDischargerTile.java:24,44,49` |
| `postTick`: `dischargeItemsInInventory(inv, transfer, capacity - stored)`, then `energy.produce(extracted)`, then `extractFromSides` | `EnergyDischargerTile.java:29-37` |
| `dischargeItemsInInventory` = one pass over the slots, each taking `min(transfer, budget left)` through the item's own `Capabilities.EnergyStorage.ITEM.extractEnergy`; the budget is the buffer's **room** | `util/ChargeUtil.java:66-78` |
| `keepEnergy()` is true, so the block item carries the buffer | `EnergyDischargerTile.java:41` |
| `getTransferType()` is `Transfer.EXTRACT` → the side config takes nothing on any face, and the block item only gives | `block/discharger/EnergyDischargerBlock.java:57` |

### What it ships

Content, carried by `tools/carry_slice8.py` (64 files, `--check` reproducible): seven block
definitions (`minecraft:furnace`, horizontal `facing`, 2.0/20.0, metal, `occludes: false`, stack 1),
seven blockstates (four facings × `lit`, the carry's `lollipop:block/empty` for `facing=up/down`
replaced — the block is horizontal), the base model and seven tier models, seven item models and 26.3
item definitions, the two textures, seven loot tables, seven recipes read from the jar in the pin's
string form, their advancements, the `powah:energy_dischargers` block and item tags, the pickaxe and
needs-iron tags, and the creative tab.

Rust:

| File | Change |
|---|---|
| `src/runtime.rs` | `DischargerTier` (`= CellTier`, the battery config), `DISCHARGER_TIER_COUNT`, `discharger_tier(index)`, `discharge_step(held, transfer, room)`; 2 tests |
| `src/powah.rs` | `DISCHARGERS` (seven ids + type names), `Kind::Discharger`, `discharger_of`, `discharge_step`'s glue (`drain_items`), the extract-only battery, the seven-slot container, the item buffer (`max_insert` 0), `keep_capacity`, the readout line; `charge_slots`/`kind_charge_slots` renamed `item_slots`/`kind_item_slots` (a discharger's slots are not charge slots); 3 tests |
| `src/lib.rs` | one container, one buffer, one idle type and one machine item per tier |
| `horizon-mod.toml`, `Cargo.toml` | version 0.2.8, the description's slice list |

**The drain goes through the item capability, not a payload rewrite.** At the level tick's end (where a
block entity is back in the world) each of the seven slots is asked for its `ITEM_ENERGY` handler with
`ItemPlace::BlockEntity{level, pos, side: None, slot}` and its `extract` runs inside a transaction: for
this mod's machines that is slice 7's `MachineEnergy::extract`, so the handler's extract side has a real
driver, and a foreign energy item answers for itself. The buffer is credited with exactly what moved.

### The readouts this slice adds

```text
Energy Discharger at 30,100,40: 0/1000000 FE (comparator 0, charge empty, empty, empty, empty, empty, empty, empty, 1000 FE/t a side, t=2985)
Energy Discharger at 30,100,40: 5000/1000000 FE (comparator 0, charge powah:energy_cell_starter 0/1000000, empty, empty, empty, empty, empty, empty, 1000 FE/t a side, t=3015)
```

The `charge` field names the seven slots in slot order, so `charge_of`/`charge_energy_of` read the
first one exactly as they do on a furnator or a cell.

### TDD: the red proof, quoted

`discharger_tier`, `discharge_step`, the readout's arm and the discharger table were left unfinished
(`todo!()`), and the two runtime tests and the three glue tests were written against them before any
glue (`ports/_shared/evidence/powah/slice8-test-red.log`):

```text
thread 'runtime::tests::a_discharge_step_takes_the_item_s_charge_the_transfer_or_the_room' panicked at src/runtime.rs:184:5:
not yet implemented: the item discharge step
thread 'runtime::tests::the_discharger_ladder_is_the_battery_config' panicked at src/runtime.rs:144:5:
not yet implemented: the discharger ladder
thread 'powah::tests::every_discharger_tier_is_one_of_the_ladder_with_an_extract_only_buffer' panicked at src/runtime.rs:144:5:
not yet implemented: the discharger ladder
thread 'powah::tests::the_discharger_readout_names_its_buffer_and_its_seven_slots' panicked at src/powah.rs:811:35:
not yet implemented: the discharger readout
test result: FAILED. 59 passed; 4 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

(The fifth new test, the container's seven slots, passes on the first run because `discharger_container`
was already written; the four above are the rules.) After the implementation: **63 passed, 0 failed**
(`slice8-test-green.log`; 58 slice-1…7 tests plus the five).

### Ladder (0.2.8, pin `0a875da`)

Every compile step ran in `ports/powah`, `CARGO_INCREMENTAL=0`, inside a
`ports/tools/build-slot.sh run --job W1-powah --port ports/powah --step …` lease. Logs in
`ports/_shared/evidence/powah/`.

| Step | Command | Result |
|---|---|---|
| unit tests | `cargo test --offline` | **63 passed**, 0 failed (`slice8-test-green.log`) |
| lints | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, 0 warnings (`slice8-clippy-fmt.log`) |
| formatting | `cargo fmt --check` | exit 0, empty (`slice8-clippy-fmt.log`) |
| package, twice | `rm -f Cargo.lock; cargo horizon-mod build` ×2, `cmp` | **byte-identical, 330,025 B**, sha256 `d152d396ff809f291b8bee6043f78ac1f1a23771a1b44dcfb6fb00c6f3b43950` (`slice8-build-twice.log`); `src/runtime.rs` packs as `733ae14c45a1…` |
| registry check | `cargo horizon-mod check powah.hmod` | **exit 0**, `powah 0.2.8 (native-mod, format native-1/0a875da…/e59da90…, for 26.3)`, `330025 bytes, SHA-256 d152d396…f3b43950, unsigned` (`slice8-check-fits.log`) |
| fits the pin | `cargo horizon-mod fits powah.hmod --build 0a875da` | `0a875da  made for`, exit 0 (`slice8-check-fits.log`) |
| fits **another** build | `cargo horizon-mod fits powah.hmod --build 0b14f8a` | `0b14f8a  fits`, exit 0 (`slice8-check-fits.log`) |
| dialect | `python3 ports/tools/check_261_dialect.py powah.hmod` | exit 0: **153** recipe/loot files in the 26.3 dialect (`slice8-check-fits.log`) |
| carried content | `python3 tools/carry_slice8.py --check` | 64 files match |
| package gate | `python3 ports/tools/verify-sdk-package.py powah` | **1/1**, runtime byte-identical, no packed `Cargo.lock`, **483 entries** |

### In-game legs

Launcher build `Compiling powah (1 of 1)` (`evidence/powah-e2e/launcher-build-0.2.8.log`, instance
`powah-e2e`); logs in `ports/_shared/evidence/powah-e2e/`.

| Leg | Command | Result |
|---|---|---|
| main | `launcher-game.sh build powah` + `build-slot.sh run --step e2e` | **PASS** + `EXIT:0` (`e2e-launcher-0.2.8.log`) |
| control (whole scenario, no package) | `GAME=$CONTROL_GAME …` | `FAIL: expected powah:furnator_starter at 0,100,0, got 'minecraft:air'` + `EXIT:1` (`e2e-control-0.2.8.log`) |
| selftest (this slice's flip) | `SELFTEST=drain …` | `FAIL: the discharger must have taken the item's 5,000 (left: 5000), got 'powah:energy_cell_starter 0/1000000'` + `EXIT:1` (`e2e-selftest-drain.log`) |
| **the previous package** (0.2.7, the parity rule's red) | the 0.2.7 game + the same scenario | `FAIL: expected powah:energy_discharger_starter at 30,100,40, got 'minecraft:air'` + `EXIT:1`, after every slice 1–7 phase passed (`e2e-oldpackage-0.2.7.log`) |
| log gate | `check-e2e-logs.py powah-e2e --strict` | `logs=24 ok=24 missing-exit=0 bad-exit=0`, exit 0 |

The legs above ran the package as packed **before** this slice's `README.md` paragraphs
(329,812 B, sha256 `99bc3239…`). The shipped artifact is 330,025 B `d152d396…`; compared entry by
entry against the launcher's installed copy of the leg's package, all 483 entries are identical
except `README.md` (`uses.json` is generated at build time and is not installed), and `src/`,
`assets/`, `data/`, the manifest and the licences are byte-identical — so the legs prove the shipped
code.

Quoted from `e2e-launcher-0.2.8.log` (the family's own assertions):

```text
== slice 8: a discharger drains the item in its slot into its buffer ==
  discharger: Energy Discharger at 30,100,40: 0/1000000 FE (comparator 0, charge empty, empty, empty, empty, empty, empty, empty, 1000 FE/t a side, t=2985)
  item(30,100,40) reply: Charge set to 5000 FE on powah:energy_cell_starter.
  drained: Energy Discharger at 30,100,40: 5000/1000000 FE (comparator 0, charge powah:energy_cell_starter 0/1000000, empty, empty, empty, empty, empty, empty, 1000 FE/t a side, t=3015)
== slice 8: the buffer's room caps one step, not the transfer ==
  room: Energy Discharger at 30,100,40: 1000000/1000000 FE (comparator 15, charge powah:energy_cell_starter 4980/1000000, empty, empty, empty, empty, empty, empty, 1000 FE/t a side, t=3041)
  room: Energy Discharger at 30,100,40: 1000000/1000000 FE (comparator 15, charge powah:energy_cell_starter 4980/1000000, empty, empty, empty, empty, empty, empty, 1000 FE/t a side, t=3069)
== slice 8: the drain rate is the tier's 1,000 FE a tick ==
  rate: Energy Discharger at 30,100,40: 14000/1000000 FE (comparator 0, charge powah:energy_cell_starter 986000/1000000, …)
  rate: Energy Discharger at 30,100,40: 21000/1000000 FE (comparator 0, charge powah:energy_cell_starter 979000/1000000, …)
  dt=7 ticks, buffer +7000 FE, item -7000 FE
== slice 8: an item with no buffer is not drained ==
  stone: Energy Discharger at 30,100,40: 0/1000000 FE (comparator 0, charge minecraft:stone 0/0, …)
== slice 8: the discharger ladder, its tags and its kept buffer ==
  set(34,100,40) = Energy Discharger at 34,100,40: 2000000000/2000000000 FE (comparator 15, … 2000000 FE/t a side, t=3166)
  tag(30,100,40) = {"chat":["Test passed"],"ok":true}
  tag(0,99,3)    = {"chat":["Test failed"],"ok":true}
  discharger placed again: Energy Discharger at 36,100,40: 7000/1000000 FE (comparator 0, charge empty, …)
== slice 8: a discharger takes no FE through any face ==
  discharger: Energy Discharger at 40,100,40: 0/1000000 FE (…)
  cell:       Energy Cell at 41,100,40: 50000/1000000 FE (…)
```

Every number is exact and its leg can fail: `5000` in the buffer with `0` on the item is what
`dischargeItemsInInventory` does with a 5,000 FE item under a 1,000 FE transfer (the `SELFTEST=drain`
run expects the item to keep its 5,000 and fails), `4980` from `5,000` is the **20 of room** capping one
step (a drain that ignored the room would leave 4,000), `+7000` over `dt=7` is 1,000 a tick against the
game's own clock, the stone's `0/0` is the item-capability gate, `2,000,000,000` with `2,000,000 FE/t`
is the nitro tier, `7000` back from a break is `keepEnergy`, and the cell beside a discharger keeping
its 50,000 is `Transfer.EXTRACT` (the cell's mirror).

### Coverage (parity rule, the family this slice adds)

| Family (from the jar) | Status |
|---|---|
| the seven `energy_discharger_*` blocks and items | **tested** — every tier's item crafted, the starter and nitro placed, `/powah set` clamped at each tier's own capacity, the readout's capacity and transfer read back, the block in `powah:energy_dischargers`, and the 0.2.7 package red at the starter's own id |
| `dischargeItemsInInventory` (item → buffer) | **tested** — a cell item charged to 5,000 ends fully in the buffer and reads 0; the buffer's 20 of room takes exactly 20 of a 5,000 item; 1,000 FE a tick measured against the game clock; a stone gives nothing |
| `keepEnergy` on a discharger | **tested** — 7,000 FE broken in survival and placed again reads 7,000/1,000,000 |
| `Transfer.EXTRACT` (no face takes FE) | **tested** — a charged cell beside it keeps its 50,000 while the discharger stays 0 |
| the seven recipes + advancements | **tested** — `craft_check.py` crafts all seven (output id + count, ingredients consumed) |
| the `powah:energy_dischargers` block/item tags | **tested in game** — `execute if block … #powah:energy_dischargers` passes on a discharger and fails on the arena floor; the item tag is the same list, validated by `check` |
| loot | **tested** — the loot leg mines the starter and the nitro discharger in survival and asserts each drops itself |
| the discharger GUI, its side-config/redstone modes, the Energy Hopper | `not ported` — slice 1's GUI/side-config gaps and the next slice (see `## Next slice` in the report) |

### Deviations and upstream defects this slice records

1. **No per-slot insert gate on the container.** Upstream's `canInsert` takes only a dischargeable item;
   a container declaration cannot express that at this pin (a refusal on one slot cancels a click on
   another, HANDOVER §9l — measured in `rftoolsstorage`), so the drain decides instead: an item whose
   `ITEM_ENERGY` handler answers nothing is left alone (measured: a stone gives 0 and the buffer stays
   0). The same shape slice 7 recorded for the charge slots.
2. **The drain runs at the level tick's end, not inside the block entity's own tick.** A block entity is
   out of the world during its tick, so an item capability there answers nothing; the position is
   queued by its tick and the pass runs at `level_tick_post`, which is also where `extractFromSides`
   already runs. The order upstream has (drain, then push) is kept.
3. **The item's own room/transfer caps are the handler's.** The pass caps one step at the machine's
   transfer and the buffer's room (`discharge_step`); the item's own `extractEnergy` caps whatever it
   holds. A foreign energy item therefore works without this mod knowing its numbers, as upstream's
   capability path does.
4. **A discharger clamps its saved buffer on its own tick** (the shared `IdleRules` clamp), so a
   doctored `/data merge block` cannot leave a buffer above the tier.
5. **No upstream defect fixed.** Everything the artefact states was expressible at the pin; the one
   mismatch found was in the brief, not the mod (the brief's `5000/10000` buffer line is the starter
   **rod**'s capacity: the starter discharger is `batteryCapacity` = 1,000,000 FE, the cell's tier-0
   number, as `PowahConfig.EnergyDevices.dischargers` says; the artefact wins and the measured line is
   `5000/1000000`).

### Harness findings this slice measured (for the next machine port)

1. **`item replace block … container.N` reaches a seven-slot declaration** exactly as it does a two-slot
   one, and `/powah item` writes slot 0 of a discharger (its first discharge slot).
2. **An item inserted beside a buffer with room drains on the very next tick**, so a leg that wants an
   exact *buffer* value must empty the slot first, set the buffer, then write the item's charge (a
   `set_energy 0` poll cannot settle while an item with charge sits in a slot).
3. **`execute if block <pos> #tag` is a reliable block-tag assertion at this pin** (`"Test passed"` /
   `"Test failed"` in the command's chat reply) — cheaper than a recipe that accepts the tag.
4. **A full scenario on a loaded box can lose a phase to timing, not to code.** Three attempts were
   needed: one failed at slice 1's discharge poll (the source read 1,000 while the sink already had 500,
   so the transfer itself was fine), two at slice 7's pickup (`held` empty right after `ctl select`);
   the same log then passed with every phase. The `pickup` helper now re-reads the hotbar and retries
   the selection (`tools/e2e.sh`), and a failing pickup dumps `{counts, hotbar, held}`.

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices

### Coverage table in the checker's shape (`check-coverage.py`)

| Family | Status | Evidence |
|---|---|---|
| `block` | tested | every block placed and read back, from the three slice-1 machines through the seven furnator/cell/cable/discharger tiers, the orb, the rods and the material block, and their loot |
| `item` | tested | every item given and counted; every machine item placed; a machine item carries its buffer (`minecraft:block_entity_data`) and places it back |
| `block_entity` | tested | each furnator tier burns at its own generation into its own capacity; a cell holds its tier's capacity and charges two item slots; each discharger tier holds its own capacity and drains its seven slots into it, clamped to the tier; each cable tier relays at its own rate and a network joins one tier; the orb holds seven slots and a recipe-gated buffer; a broken furnator/cell/rod/discharger carries its buffer on its item, clamped to the tier on placement |
| `container` | tested | the furnator's charge slot (0) and fuel slot (1), a cell's two slots and a discharger's seven, each asserted through the machine's own readout; a stone in a charge or discharge slot is left alone |
| `loot_table` | tested | every machine, furnator tier, cell, cable tier, discharger tier, orb, rod and material block mined in survival in the loot leg and asserted to drop itself |
| `recipe:minecraft:crafting_shaped` | tested | `craft_check.py` crafts each of the 50 shaped recipes in a real table: output id + count, ingredients consumed |
| `recipe:minecraft:crafting_shapeless` | tested | same leg, 8 cases; the paste's lava bucket leaves its bucket |
| `tag:item` | tested | `powah:energizing_rods`, `powah:furnators`, `powah:energy_cables` and `powah:energy_dischargers` name their seven members |
| `tag:block` | tested | the cable's `powah:energy_cable_connects` visual-arms tag and `powah:energy_dischargers` (read by `execute if block` in game); the pickaxe/needs-iron tags are what makes the loot leg's drops real |

## Slice 9 — the Energy Hopper (2026-11-23)

### The slice, quoted from this port's own gaps

`## Deviations and gaps (slice 1)`, item 11 — the notes' own next-slice authority:

> **11. The other 6 tiers, the creative cell and ~15 machines** are unported.

and `FINDINGS.md` §"Slice 1 boundary", which names the machine:

> Discharger/**Hopper**/Player Transmitter; item charging (`item_energy_buffer`) and
> energy-carrying block items; side-config GUI and redstone modes (defaults only);

and the 0.2.8 section's own "Not chosen", which named it as the next slice:

> the Energy Hopper (`ChargingConfig`, a charging station over a container) is the other
> half of the same family and is left for the next slice

**Chosen:** the **Energy Hopper**, all seven tiers — the charging half of the item-energy family, the
mirror of slice 8's discharger: a storage block with a keep-reserve and **no slots of its own** whose
tick takes the container on the face its `facing` names and charges the items in it.

**Why it fits `machines`:** it is a block entity with a declared block energy buffer (two-way), a
`keepEnergy` payload on its item, no container of its own, and one tick that reads the block state's
`facing`, the neighbour's `ITEMS` capability and each item's `ITEM_ENERGY` handler. No renderer, no
GUI, no entity, no worldgen.

**Not chosen (and why):** the creative cell (a source, not a tier) and the generators (a mod block
entity fluid tank the pin cannot fill) remain out; the reactors stay `blocked(uraninite-worldgen)`;
side config, redstone modes and the GUIs stay slice 1's recorded gaps.

### What upstream does (read from tag `v6.2.10`, commit `a5a8f25`)

| Fact | Where |
|---|---|
| `devices.hoppers = new ChargingConfig(batteryCapacity(), batteryTransfer(), chargingTransfer())`: the **cell** ladder for storage (starter 1,000,000 FE / 1,000 FE a side) | `config/v2/PowahConfig.java:95`, `DefaultEnergies` |
| `chargingTransfer() = cableTransfer() = baseScaling * 500` — **500 FE a tick at starter**, 1,000,000 at nitro | `config/v2/DefaultEnergies.java` |
| `postTick`: `side = state.facing`; the block entity there must be a `Container`; `ChargeUtil.chargeItemsInContainer(container, charging_rates.get(variant), energy.getStored())`, then `energy.consume(extracted)`; **no `extractFromSides`** | `block/hopper/EnergyHopperTile.java:29-40` |
| `chargeItemsInContainer` is one pass over `0..containerSize`, an empty slot skipped, each stack taking `min(rate, total - charged)` through its own `Capabilities.EnergyStorage.ITEM` | `util/ChargeUtil.java:60-80` |
| `keepEnergy()` true; `getSlotLimit(0)`, `canInsert` and `canExtract` false — **no slots of its own** | `EnergyHopperTile.java:44-56` |
| `getTransferType()` is `AbstractEnergyBlock`'s default `Transfer.ALL`: takes and gives its transfer on every face | `lib/block/AbstractEnergyBlock.java:75` |
| `Blcks.ENERGY_HOPPER`: `Properties.metalNoSolid(2.0, 20.0)`, `Facing.ALL` (six values, default `north`), seven tiers, stack 1 | `block/Blcks.java`, `lib/block/AbstractBlock.java` |

### What it ships

Rust:

| File | Change |
|---|---|
| `src/runtime.rs` | `HOPPER_TIER_COUNT`, `HopperTier { capacity, transfer, charging_rate }`, `hopper_tier(index)` (the cell row × the cable's 500), `hopper_charges(stored, rate, rooms)` (one pass, one budget); 2 tests |
| `src/powah.rs` | `HOPPERS`, `Kind::Hopper(tier)`, `hopper_of`, `hopper_battery` (`Transfer.ALL`), the two-way `ItemBuffer`, `keep_capacity`, `item_slots` = 0, the `/powah set` clamp, `hopper_line`, `HopperPart` (queue + clamp), `charge_container` at `level_tick_post`, placement sets `facing` from the clicked face, the `/powah info` arm; 3 tests |
| `src/lib.rs` | one buffer, one block entity type and one machine item per hopper tier |

Content: see [`CONTENT.md`](CONTENT.md) §"Slice 9" — 7 blocks, 7 blockstates, a base model and
seven tier overlays, 7 item models and 26.3 item definitions, 3 textures, 7 loot tables, 7 recipes
with their advancements, the `powah:energy_hoppers` block and item tags, the pickaxe/needs-iron
entries and the creative tab; `tools/carry_slice9.py` reproduces the **65** files from the raw carry
and the jar (`--check` green).

### The readouts this slice adds

```text
Energy Hopper at 6,100,19: 0/1000000 FE (comparator 0, facing south, charging 500 FE/t, 1000 FE/t a side, t=3232)
Energy Hopper at 10,100,18: 2000000000/2000000000 FE (comparator 15, facing north, charging 1000000 FE/t, 2000000 FE/t a side, t=3447)
Energy Hopper at 12,100,18: 7000/1000000 FE (comparator 0, facing up, charging 500 FE/t, 1000 FE/t a side, t=3575)
```

The line names the face the hopper charges through and the tier's charging rate beside its buffer and
per-side transfer. A hopper's own container is empty by design, so there is no `charge` field; the
container it faces is read with `/data get block`.

### TDD: the red proof, quoted

`hopper_tier`, `hopper_charges`, `hopper_of` and `hopper_line` were declared unfinished (`todo!()`,
or the readout's arm absent) and the tests were written against them before any glue
(`ports/_shared/evidence/powah/slice9-test-red.log`):

```text
---- powah::tests::every_hopper_tier_is_one_of_the_ladder_with_a_two_way_buffer_and_no_slots stdout ----

thread 'powah::tests::every_hopper_tier_is_one_of_the_ladder_with_a_two_way_buffer_and_no_slots' (3802252) panicked at src/runtime.rs:171:5:
not yet implemented: the hopper ladder

---- powah::tests::the_hopper_readout_names_its_buffer_its_facing_and_its_charging_rate stdout ----

thread 'powah::tests::the_hopper_readout_names_its_buffer_its_facing_and_its_charging_rate' (3802260) panicked at src/runtime.rs:171:5:
not yet implemented: the hopper ladder

---- runtime::tests::a_hopper_pass_charges_each_item_at_its_rate_within_one_budget stdout ----

thread 'runtime::tests::a_hopper_pass_charges_each_item_at_its_rate_within_one_budget' (3802275) panicked at src/runtime.rs:181:5:
not yet implemented: the hopper charge pass

---- runtime::tests::the_hopper_ladder_is_the_battery_config_with_its_own_charging_rate stdout ----

thread 'runtime::tests::the_hopper_ladder_is_the_battery_config_with_its_own_charging_rate' (3802299) panicked at src/runtime.rs:171:5:
not yet implemented: the hopper ladder

test result: FAILED. 64 passed; 4 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

(The red run was taken while the crate still read `0.2.8` — the version bump came with the packaging
step.) After the implementation: **68 passed, 0 failed** (`slice9-test-green.log`, the same 68 in
`sdk-test.log`; 63 slice-1…8 tests plus the five).

### Ladder (0.2.9, pin `0a875da`)

Every compile step ran in `ports/powah`, `CARGO_INCREMENTAL=0`, inside a
`ports/tools/build-slot.sh run --job W1-powah --port ports/powah --step …` lease with
`BUILD_LOCAL=1`; logs in `ports/_shared/evidence/powah/`.

| Step | Command | Result |
|---|---|---|
| unit tests | `cargo test --offline` | **68 passed**, 0 failed (`sdk-test.log`) |
| lints | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, 0 warnings (`slice9-clippy-fmt.log`) |
| formatting | `cargo fmt --check` | exit 0, empty (`slice9-clippy-fmt.log`) |
| package, twice | `rm -f Cargo.lock; cargo horizon-mod build` ×2, `cmp` | **byte-identical, 356,866 B**, sha256 `387259ea8a3b6ce0975279cd4f4d67620d8c2fc11d1c634a76cc8113e32045f3` (`slice9-build-twice.log`) |
| registry check | `cargo horizon-mod check powah.hmod` | **exit 0**, `powah 0.2.9 (native-mod, format native-1/0a875da…/e59da90…, for 26.3)`, `356866 bytes, SHA-256 387259ea…32045f3, unsigned` (`slice9-check-fits.log`) |
| fits the pin | `cargo horizon-mod fits powah.hmod --build 0a875da` | `0a875da  made for`, exit 0 (`slice9-check-fits.log`) |
| fits **another** build | `cargo horizon-mod fits powah.hmod --build 0b14f8a` | `0b14f8a  fits`, exit 0 (`slice9-check-fits.log`) |
| dialect | `python3 ports/tools/check_261_dialect.py powah.hmod` | exit 0: **174** recipe/loot files in the 26.3 dialect (`slice9-check-fits.log`) |
| carried content | `python3 tools/carry_slice9.py --check` | 65 files match |
| package gate | `python3 ports/tools/verify-sdk-package.py powah` | **1/1**, runtime byte-identical (`25359ccdb7d4`), no packed `Cargo.lock`, **545 entries** (`sdk-verify.log`) |

### In-game legs

Launcher build `Compiling powah (1 of 1)`, `native-0a875da-6653b4fc239a`, instance `powah-e2e`
(`evidence/powah-e2e/launcher-build-0.2.9.log`); logs in `ports/_shared/evidence/powah-e2e/`.

| Leg | Command | Result |
|---|---|---|
| main | `launcher-game.sh build powah` + `build-slot.sh run --step e2e` | **PASS** + `EXIT:0` (`e2e-launcher-0.2.9.log`) |
| control (whole scenario, no package) | `GAME=$CONTROL_GAME …` | `FAIL: expected powah:furnator_starter at 0,100,0, got 'minecraft:air'` + `EXIT:1`; the control pre-check says `Unknown block type 'powah:energy_hopper_starter'` (`e2e-control-0.2.9.log`) |
| selftest (this slice's flip) | `SELFTEST=hopper …` | `FAIL: the chest's item must read 0 FE, got 5000` + `EXIT:1` (`e2e-selftest-hopper.log`) |
| **the previous package** (0.2.8, the parity rule's red) | the 0.2.8 game + the same scenario | `FAIL: expected powah:energy_hopper_starter at 6,100,19, got 'minecraft:air'` + `EXIT:1`, after every slice 1–8 phase passed (`e2e-oldpackage-0.2.8.log`) |
| log gate | `check-e2e-logs.py powah-e2e --strict` | `logs=28 ok=28 missing-exit=0 bad-exit=0`, exit 0 |

Quoted from `e2e-launcher-0.2.9.log` (the family's own assertions):

```text
== slice 9: a hopper charges the container on the face its facing names ==
  block(6,100,20) = minecraft:chest
  block(6,100,19) = powah:energy_hopper_starter
  hopper: Energy Hopper at 6,100,19: 0/1000000 FE (comparator 0, facing south, charging 500 FE/t, 1000 FE/t a side, t=3232)
  set(6,100,19) = Energy Hopper at 6,100,19: 5000/1000000 FE (comparator 0, facing south, charging 500 FE/t, 1000 FE/t a side, t=3239)
  spent: Energy Hopper at 6,100,19: 0/1000000 FE (comparator 0, facing south, charging 500 FE/t, 1000 FE/t a side, t=3263)
  data get block(6,100,20) Items[0]: {"chat":["6, 100, 20 has the following block data: 5000"],"ok":true}
  hopper at 0, the chest's item at 5000 FE: 5000 - 500 x 10 = 0
== slice 9: the charging rate is the tier's 500 FE a tick ==
  set(6,100,19) = Energy Hopper at 6,100,19: 100000/1000000 FE (comparator 1, facing south, charging 500 FE/t, 1000 FE/t a side, t=3277)
  rate: Energy Hopper at 6,100,19: 97500/1000000 FE (comparator 1, facing south, charging 500 FE/t, 1000 FE/t a side, t=3287)
  rate: Energy Hopper at 6,100,19: 94000/1000000 FE (comparator 1, facing south, charging 500 FE/t, 1000 FE/t a side, t=3294)
  dt=7 ticks, hopper -3500 FE
  data get block(6,100,20) Items[0]: {"chat":["6, 100, 20 has the following block data: 8500"],"ok":true}
  item at 8500, hopper at 90500 of the 100,000 it held
  unaccounted: 1000 FE (item + hopper against the 100,000)
== slice 9: a hopper facing away leaves the container alone ==
  block(8,100,20) = minecraft:chest
  set(8,100,19) = Energy Hopper at 8,100,19: 5000/1000000 FE (comparator 0, facing north, charging 500 FE/t, 1000 FE/t a side, t=3322)
  away: Energy Hopper at 8,100,19: 5000/1000000 FE (comparator 0, facing north, charging 500 FE/t, 1000 FE/t a side, t=3374)
  data get block(8,100,20) Items[0]: {"chat":["Found no elements matching Items[0].components"],"ok":true}
  away chest item: no payload
== slice 9: an item with no buffer takes nothing from the hopper ==
  set(6,100,19) = Energy Hopper at 6,100,19: 5000/1000000 FE (comparator 0, facing south, charging 500 FE/t, 1000 FE/t a side, t=3387)
  stone: Energy Hopper at 6,100,19: 5000/1000000 FE (comparator 0, facing south, charging 500 FE/t, 1000 FE/t a side, t=3434)
== slice 9: the hopper ladder, its tags and its kept buffer ==
  block(10,100,18) = powah:energy_hopper_nitro
  set(10,100,18) = Energy Hopper at 10,100,18: 2000000000/2000000000 FE (comparator 15, facing north, charging 1000000 FE/t, 2000000 FE/t a side, t=3447)
  tag(6,100,19) = {"chat":["Test passed"],"ok":true}
  tag(0,99,3)    = {"chat":["Test failed"],"ok":true}
  block(12,100,18) = powah:energy_hopper_starter
  set(12,100,18) = Energy Hopper at 12,100,18: 7000/1000000 FE (comparator 0, facing north, charging 500 FE/t, 1000 FE/t a side, t=3477)
  {"scenario":"hopper-keep-energy","block":"minecraft:air","drops":{"powah:energy_hopper_starter":1},"picked_up":{"powah:energy_hopper_starter":1}}
  placed powah:energy_hopper_starter at 12,100,18
  hopper placed again: Energy Hopper at 12,100,18: 7000/1000000 FE (comparator 0, facing up, charging 500 FE/t, 1000 FE/t a side, t=3575)
```

Every number is exact and its leg can fail: the 5,000 set on the hopper ends **at 0** while the cell
item in the chest it faces ends **at exactly 5,000** (10 charges of 500; the `SELFTEST=hopper` run
expects 0 on the item and fails), `dt=7` against the hopper's own `t=` clock costs exactly
**3,500 = 7 × 500** (a 40-FE or a per-slot-multiplied rate fails the whole-step test), the facing-away
control spends **nothing** and leaves the chest item without a payload, a stone leaves the buffer at
**5,000**, the nitro tier clamps 2,000,000,000 at its own capacity and names `charging 1000000 FE/t,
2000000 FE/t a side`, the tag passes on a hopper and fails on the arena floor, and a 7,000 FE hopper
broken in survival and placed again reads **7,000** (its placement also set `facing up` from the
clicked face). The conservation sum of the item's charge and the hopper's balance is the 100,000 it
held within **1,000 FE** — two ticks of lag between the item read and the hopper read, the tolerance
the leg documents.

### Coverage (parity rule, the family this slice adds)

| Family (from the jar) | Status |
|---|---|
| the seven `energy_hopper_*` blocks and items | **tested** — the starter and the nitro placed and read back, `/powah set 2000000000` clamped at the nitro tier's 2,000,000,000, each readout naming its buffer, face, charging rate and per-side transfer; the 0.2.8 package red at the starter's own id; the control game knows neither id |
| `chargeItemsInContainer` (hopper → the faced container's items) | **tested** — a cell item in a chest at 0 ends at exactly 5,000 while the hopper falls 5,000 → 0; the rate is 500 FE a tick measured against the game clock (`3,500` over `dt=7`, whole 500 steps) |
| the face rule (`state.facing` picks the container) | **tested** — the same hopper at `facing=north` beside a chest at its south spends nothing and charges the item to nothing; placement from an item sets `facing` from the clicked face (read back `facing up`) |
| an item with no energy buffer | **tested** — a stone in the faced chest leaves the hopper at exactly 5,000 |
| `keepEnergy` on a hopper | **tested** — a 7,000 FE hopper broken in survival and placed again reads 7,000/1,000,000 |
| `Transfer.ALL` on every face | **rule-tested** — `hopper_battery`/`item_buffer` are pinned per tier by `every_hopper_tier_is_one_of_the_ladder_with_a_two_way_buffer_and_no_slots`; the hopper never pushes in its tick (`postTick` has no `extractFromSides`) |
| **no slots of its own** | **rule-tested and observed** — `item_slots` = 0 per tier; the readout has no `charge` field and the e2e only ever fills the chest |
| the seven recipes + advancements | **tested** — `craft_check.py` crafts all seven (output id + count, every ingredient consumed) |
| the `powah:energy_hoppers` block/item tags | **tested in game** — `execute if block … #powah:energy_hoppers` passes on a hopper and fails on the arena floor |
| loot | **tested** — the loot leg mines all seven tiers in survival and asserts each drops itself |
| the hopper GUI, its side-config/redstone modes | `not ported` — slice 1's GUI/side-config gaps |
| the creative cell, the generators, the reactors, the ender gates, the player transmitter, the batteries | `not ported` — the reactors stay `blocked(uraninite-worldgen)`; the generators need a mod fluid tank and sky visibility |

### Deviations and upstream defects this slice records

1. **The charge pass runs at the level tick's end, not inside the block entity's own tick.** A block
   entity is out of the world during its tick, so the neighbour's `ITEMS` capability and each item's
   `ITEM_ENERGY` handler are read in the queue phase (`level_tick_post`), the same deviation slice 8
   records for the drain.
2. **`facing` on placement is the clicked face.** `getStateForPlacement` uses
   `getNearestLookingDirection().getOpposite()`; the pin exposes no player rotation, so the face the
   engine reports on placement is used — identical for every face-aligned placement. A hopper a
   dispenser places keeps `north`.
3. **No per-slot gate on the faced container.** A container declaration cannot express upstream's
   `canInsert`; the pass decides instead: a slot whose item answers no `ITEM_ENERGY` handler is left
   alone (measured: a stone gives nothing and the buffer stays 5,000). The same shape slices 7/8
   record for their own slots.
4. **The hopper's readout names its face and charging rate, not the container it faces.** The
   container's stacks are read with `/data get block … Items[N]` in the leg; `/powah info` prints the
   hopper's own state (`Energy Hopper at …: 0/1000000 FE (comparator 0, facing south, charging 500
   FE/t, 1000 FE/t a side, t=…)`).
5. **The tier clamp runs on the hopper's own tick** (the shared clamp in `HopperPart`), so a doctored
   `/data merge block` cannot leave a buffer above the tier.

**Upstream defects fixed:** none — the hopper's numbers (`ChargingConfig` over the battery row, the
cable's rate for charging), the face rule and the item pass are internally consistent. The only
mismatch found while measuring was in **this slice's first harness shape**: an equality poll on the
buffer can never settle while an item with room sits in the faced chest (measured: it read 4,000
instead of 5,000 after the set), so the leg clears the slot, sets the buffer, then inserts the item —
the same ordering slice 8 recorded.

### Harness findings this slice measured (for the next machine port)

1. **A chest's item energy is read with the path form.** `/data get block X Y Z
   Items[0].components."minecraft:block_entity_data".energy` answers the exact int (the same
   component slice 7 writes on a machine's own item); it is the only read of an item's charge that
   does not need the item in hand.
2. **`setblock … [facing=south]` sets the state the tick reads**, and a hopper placed from its own
   item takes the clicked face (read back `facing up` when placed on the floor) — so both placement
   paths are observable through `/powah info`.
3. **`execute if block <pos> #tag` is a reliable block-tag assertion for the new tag too** (`"Test
   passed"` / `"Test failed"`), and a tag miss on the arena floor is the negative half.
4. **A hopper's buffer cannot be polled for equality while it has something to charge** (the spend
   starts the tick after the write): clear the faced slot, write the buffer, then insert the item.
5. `/powah set` stays an operator-only (level 2) harness control; it clamps a hopper at its tier's
   capacity because the buffer *is* the tier's.

Version: **0.2.8 → 0.2.9** (one patch). The published `[[versions]]` record keeps 0.2.8 and gains
0.2.9 at publish time — this port does not touch the registry entry.

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices

## Slice 10 — the Solar Panel ladder (2026-11-23)

### The slice, quoted from this port's own gaps

`FINDINGS.md` §"Slice 1 boundary", the line naming the family:

> Gaps (recorded, not claimed): the other tiers and the creative cell; the
> Magmator/Thermo/**Solar**/Reactors; Ender Cells/Gates; Energizing Orb/Rods; …

and slice 9's coverage row, which is what kept it out of the last two waves:

> the creative cell, the generators, the reactors, the ender gates, the player transmitter, the
> batteries | `not ported` — the reactors stay `blocked(uraninite-worldgen)`; **the generators need
> a mod fluid tank and sky visibility**

and the 0.2.9 report's own "Next slice", which named it:

> the Solar Panel ladder (`powah:solar_panel_{starter…nitro}`, seven tiers), because the pin
> exposes the two reads upstream's `SolarTile` needs — `level.can_see_sky(host, pos)` and
> `level.get_sky_darken(host)` — so the "sky visibility is unreachable" note was wrong for this
> family.

**Chosen:** the **Solar Panel**, all seven tiers — the one generator family whose only input is
the sky, and the family that proves the sky reads work in game.

**Why it fits `machines`:** a block entity with a declared block energy buffer (extract-only, on
its bottom face), a `keepEnergy` payload on its item, one charge slot, and a tick that reads the
level's sky (`can_see_sky` at the block above, the dimension's sky light, `get_sky_darken`). No
renderer, no GUI, no entity, no worldgen, no fluid tank.

**Not chosen (and why):** the Magmator and the Thermo Generator need a mod block entity fluid tank
the pin cannot fill (the recorded gap), and the reactors stay `blocked(uraninite-worldgen)`; the
creative cell (a source, not a tier), the ender gates and the player transmitter remain out. The
Lens of Ender — upstream's bypass for the sky — is a named gap (below).

### What upstream does (read from tag `v6.2.10`, commit `a5a8f25`)

| Fact | Where |
|---|---|
| `devices.solar_panels = new GeneratorConfig(generatorCapacity(), generatorTransfer(), passiveProduction())` — the **passive** row for generation and the furnator's rows for the buffer and the per-side transfer | `config/v2/PowahConfig.java:79`, `DefaultEnergies` |
| `passiveProduction = baseSlowScaling * 20` = **20/60/100/200/400/800/2,000 FE/t**; `generatorCapacity = baseScaling * 20,000` (20k…40M); `generatorTransfer = baseScaling * 80` (80…160,000) | `config/v2/DefaultEnergies.java` |
| `postTick`: `(canSeeSky \|\| hasLensOfEnder) && dimensionType().hasSkyLight() && getSkyDarken() < 4` makes `min(generation, room)` while the buffer is not full | `block/solar/SolarTile.java:44-62` |
| `canSeeSky` is refreshed every 40 ticks as `Misc.canBlockSeeSky(world, pos.above())`, i.e. **the block above the panel**: the panel's own `propagatesSkylightDown` is false, so the sky is read one block up (upstream's own comment says exactly this) | `SolarTile.java:49-56`, `util/Misc.java:7-14`, `SolarBlock.propagatesSkylightDown` |
| `isEnergyPresent(side)` is `Direction.DOWN` only — the panel gives FE on its bottom face and no other | `SolarTile.java:78-81` |
| `keepEnergy()` is true; one slot (`inv.add(1)`, slot limit 1, chargeables only); `chargeItems(1)` runs before the production | `SolarTile.java:34, 83-100` |
| `Blcks.SOLAR_PANEL`: seven tiers, `Properties.metal(2.0, 20.0)`, a `SimpleWaterloggedBlock` whose shape is `box(0,0,0,16,1,16)` with four horizontal booleans (`true` = no panel attached, so the frame is drawn) | `block/Blcks.java`, `SolarBlock.java` |

### What it ships

Rust:

| File | Change |
|---|---|
| `src/runtime.rs` | `SOLAR_TIER_COUNT`, `SolarTier { generation, capacity, transfer }`, `solar_tier` (the passive row × the active capacity/transfer), `Sky { can_see_sky, has_skylight, darken }`, `Sky::daylight`, `SOLAR_DAY_MAX_DARKEN`, `solar_production` (the `min(generation, room)` rule); 3 tests |
| `src/powah.rs` | `SOLARS`, `Kind::Solar(tier)`, `solar_of`, `solar_battery` (extract on `Direction::Down` only), `solar_container` (one charge slot), `SOLAR_SLOTS`, `sky_at`, `solar_line`, `SolarPart` + `solar_tick`, `push_out_on` (the faces a machine gives on; `push_out` is it over all six), the `/powah set` and `/powah info` arms, and the four frame booleans on placement; 3 tests |
| `src/lib.rs` | one container, one buffer, one block entity type and one machine item per solar tier |

Content: see [`CONTENT.md`](CONTENT.md) §"Slice 10" — 7 blocks with their own definition and a
1-pixel outline/collision, 7 blockstates, 11 block models, 9 item models, 7 item definitions,
9 textures, 7 loot tables, 8 recipes with their advancements (the seven panels and the
photoelectric pane the starter needs), the `powah:solar_panels` block and item tags, the
pickaxe/needs-iron entries and the creative tab; `tools/carry_slice10.py` reproduces the **82**
files from the raw carry and the jar (`--check` green).

### The readouts this slice adds

```text
Solar Panel at 14,100,6: 860/20000 FE (comparator 0, sky open, day true, generating 20 FE/t, 80 FE/t a side, charge empty, t=3708)
Solar Panel at 14,100,6: 5000/20000 FE (comparator 3, sky covered, day true, generating 20 FE/t, 80 FE/t a side, charge empty, t=3861)
Solar Panel at 14,100,6: 5000/20000 FE (comparator 3, sky open, day false, generating 20 FE/t, 80 FE/t a side, charge empty, t=3995)
Solar Panel at 18,100,6: 40000000/40000000 FE (comparator 15, sky open, day true, generating 2000 FE/t, 160000 FE/t a side, charge empty, t=4232)
```

The line names the two sky reads the tick makes (`sky open|covered` is `can_see_sky`, `day
true|false` is `has_skylight && get_sky_darken() < 4`), the tier's generation and per-side
transfer, and its one charge slot.

### TDD: the red proof, quoted

`solar_tier`, `Sky::daylight`, `solar_production`, `solar_battery`, `solar_container` and
`solar_line` were declared unfinished (`todo!()`) and the five tests were written against them
before any glue (`ports/_shared/evidence/powah/slice10-test-red.log`):

```text
---- powah::tests::every_solar_tier_is_one_of_the_ladder_with_a_down_only_buffer stdout ----

thread 'powah::tests::every_solar_tier_is_one_of_the_ladder_with_a_down_only_buffer' (945896) panicked at src/runtime.rs:225:5:
not yet implemented: the solar ladder

---- powah::tests::the_solar_readout_names_its_sky_its_generation_and_its_charge_slot stdout ----

thread 'powah::tests::the_solar_readout_names_its_sky_its_generation_and_its_charge_slot' (945911) panicked at src/runtime.rs:225:5:
not yet implemented: the solar ladder

---- runtime::tests::a_solar_panel_generates_only_with_sky_daylight_and_room stdout ----

thread 'runtime::tests::a_solar_panel_generates_only_with_sky_daylight_and_room' (945931) panicked at src/runtime.rs:225:5:
not yet implemented: the solar ladder

---- runtime::tests::every_solar_tier_scales_upstreams_passive_numbers stdout ----

thread 'runtime::tests::every_solar_tier_scales_upstreams_passive_numbers' (945941) panicked at src/runtime.rs:225:5:
not yet implemented: the solar ladder

---- runtime::tests::the_solar_daylight_test_is_upstreams_two_reads stdout ----

thread 'runtime::tests::the_solar_daylight_test_is_upstreams_two_reads' (945951) panicked at src/runtime.rs:243:9:
not yet implemented: the solar daylight test


test result: FAILED. 69 passed; 5 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

(The red run was taken while the crate still read `0.2.9` — the version bump came with the packaging
step.) After the implementation: **74 passed, 0 failed** (`slice10-test-green.log`, the same 74 in
`sdk-test.log`; 69 slice-1…9 tests plus the five).

### Ladder (0.2.10, pin `0a875da`)

Every compile step ran in `ports/powah`, `CARGO_INCREMENTAL=0`, inside a
`ports/tools/build-slot.sh run --job W1-powah --port ports/powah --step …` lease with
`BUILD_LOCAL=1`; logs in `ports/_shared/evidence/powah/`.

| Step | Command | Result |
|---|---|---|
| unit tests | `cargo test --offline` | **74 passed**, 0 failed (`sdk-test.log`) |
| lints | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, 0 warnings (`slice10-clippy-fmt.log`) |
| formatting | `cargo fmt --check` | exit 0, empty (`slice10-clippy-fmt.log`) |
| package, twice | `rm -f Cargo.lock; cargo horizon-mod build` ×2, `cmp` | **byte-identical, 388,951 B**, sha256 `13f0e7be10390536b39ae315d81878f257e5a5921f2cf7c071b45c60d6cfc9b4` (`slice10-build-twice.log`); the packed `README.md` is byte-identical to the tree's |
| registry check | `cargo horizon-mod check powah.hmod` | **exit 0**, `powah 0.2.10 (native-mod, format native-1/0a875da…/e59da90…, for 26.3)`, `388951 bytes, SHA-256 13f0e7be…cfc9b4, unsigned` (`slice10-check-fits.log`) |
| fits the pin | `cargo horizon-mod fits powah.hmod --build 0a875da` | `0a875da  made for`, exit 0 (`slice10-check-fits.log`) |
| fits **another** build | `cargo horizon-mod fits powah.hmod --build 0b14f8a` | `0b14f8a  fits`, exit 0 (`slice10-check-fits.log`) |
| dialect | `python3 ports/tools/check_261_dialect.py powah.hmod` | exit 0: **197** recipe/loot files in the 26.3 dialect (`slice10-check-fits.log`) |
| carried content | `python3 tools/carry_slice10.py --check` | 82 files match |
| package gate | `python3 ports/tools/verify-sdk-package.py powah` | **1/1**, runtime byte-identical (`3aaf9dfd3796`), no packed `Cargo.lock`, **624 entries** (`sdk-verify.log`) |

### In-game legs

Launcher build `Compiling powah (1 of 1)`, `native-0a875da-eb0b2f34c233`, instance `powah-e2e`
(`evidence/powah-e2e/launcher-build-0.2.10.log`); logs in `ports/_shared/evidence/powah-e2e/`.
The legs below ran the 388,638 B archive (`696850bc…`) of these same sources, which differs from the
delivered archive only in `README.md` — a document, with nothing compiled or carried changed between
them. The delivered 0.2.10 archive is **`13f0e7be…`** (388,951 B), and it predates the ender-channel
slice's tree edits (`src/runtime.rs` was edited at 07:18 for that slice, after this package was
built), so the `README.md`-only difference is the **only** difference between the bytes the legs ran
and the bytes that ship. A later main leg and `SELFTEST=solar` were also re-run on the delivered
archive itself (`e2e-launcher-0.2.10-readme.log`, `e2e-selftest-solar-readme.log`, whose own
`package:` line names `13f0e7be…` and 388,951 B), against the same `native-0a875da-eb0b2f34c233`
build of 0.2.10's code — the ender edits were never compiled into it.

| Leg | Command | Result |
|---|---|---|
| main | `launcher-game.sh build powah` + `build-slot.sh run --step e2e` | **PASS** + `EXIT:0` (`e2e-launcher-0.2.10.log`) |
| control (whole scenario, no package) | `GAME=$CONTROL_GAME …` | `FAIL: expected powah:furnator_starter at 0,100,0, got 'minecraft:air'` + `EXIT:1` (`e2e-control-0.2.10.log`) |
| selftest (this slice's flip) | `SELFTEST=solar …` | `FAIL: SELFTEST flipped: the open-sky panel generated 140 FE over 7 ticks` + `EXIT:1` (`e2e-selftest-solar.log`) |
| **the previous package** (0.2.9, the parity rule's red) | the 0.2.9 game + the same scenario | `FAIL: expected powah:solar_panel_starter at 14,100,6, got 'minecraft:air'` + `EXIT:1`, after every slice 1–9 phase passed (`e2e-oldpackage-0.2.9.log`) |
| log gate | `check-e2e-logs.py powah-e2e --strict` | `logs=37 ok=37 missing-exit=0 bad-exit=0`, exit 0 |

Quoted from `e2e-launcher-0.2.10.log` (the family's own assertions):

```text
== slice 10: a solar panel generates only with an open sky and daylight ==
  solar: Solar Panel at 14,100,6: 860/20000 FE (comparator 0, sky open, day true, generating 20 FE/t, 80 FE/t a side, charge empty, t=3708)
  rate: Solar Panel at 14,100,6: 10140/20000 FE (comparator 7, sky open, day true, generating 20 FE/t, 80 FE/t a side, charge empty, t=3722)
  rate: Solar Panel at 14,100,6: 10260/20000 FE (comparator 7, sky open, day true, generating 20 FE/t, 80 FE/t a side, charge empty, t=3728)
  dt=6 ticks, panel +120 FE
  twenty of room: Solar Panel at 14,100,6: 20000/20000 FE (comparator 15, sky open, day true, generating 20 FE/t, 80 FE/t a side, charge empty, t=3749)
== slice 10: a stone above the panel stops it, and midnight stops it ==
  covered: Solar Panel at 14,100,6: 5000/20000 FE (comparator 3, sky covered, day true, generating 20 FE/t, 80 FE/t a side, charge empty, t=3861)
  covered: Solar Panel at 14,100,6: 5000/20000 FE (comparator 3, sky covered, day true, generating 20 FE/t, 80 FE/t a side, charge empty, t=3929)
  night: Solar Panel at 14,100,6: 5000/20000 FE (comparator 3, sky open, day false, generating 20 FE/t, 80 FE/t a side, charge empty, t=3995)
  night: Solar Panel at 14,100,6: 5000/20000 FE (comparator 3, sky open, day false, generating 20 FE/t, 80 FE/t a side, charge empty, t=4066)
== slice 10: a solar panel gives on its bottom face only ==
  cell below:  Energy Cell at 20,99,8: 1420/1000000 FE (comparator 0, charge empty, empty, 1000 FE/t a side, t=4169)
  cell beside: Energy Cell at 21,100,8: 0/1000000 FE (comparator 0, charge empty, empty, 1000 FE/t a side, t=4175)
  panel:       Solar Panel at 20,100,8: 0/20000 FE (comparator 0, sky open, day true, generating 20 FE/t, 80 FE/t a side, charge empty, t=4182)
== slice 10: the solar ladder, its tags and its clamp ==
  set(18,100,6) = Solar Panel at 18,100,6: 40000000/40000000 FE (comparator 15, sky open, day true, generating 2000 FE/t, 160000 FE/t a side, charge empty, t=4221)
  info(18,100,6): Solar Panel at 18,100,6: 40000000/40000000 FE (comparator 15, sky open, day true, generating 2000 FE/t, 160000 FE/t a side, charge empty, t=4232)
  tag(18,100,6) = {"chat":["Test passed"],"ok":true}
  tag(0,99,3)   = {"chat":["Test failed"],"ok":true}
```

Every number is exact and its leg can fail: two readouts **6 ticks apart differ by exactly 120 =
6 × 20** (a 40-FE or a 10-FE tier fails the whole-step test; `SELFTEST=solar` expects 0 and fails),
a panel with exactly 20 of room fills to **20,000 and stays there**, a stone at the sky position
leaves **5,000** for two reads 68 ticks apart while the same panel under an open sky generated
120 in 6 (`sky covered` in the readout names why), `/time set midnight` with the sky open leaves
**5,000** for 71 ticks (`sky open, day false`), the panel's bottom face charges the cell under it
(**1,420**) while a cell on its side stays **0** and the panel's own buffer stays **0** (it takes no
fill), the nitro tier clamps 2,000,000,000 at **40,000,000** and names `generating 2000 FE/t,
160000 FE/t a side`, the tag passes on a panel and fails on the arena floor, and every panel tier
drops itself.

### Coverage (parity rule, the family this slice adds)

| Family (from the jar) | Status |
|---|---|
| the seven `solar_panel_*` blocks and items | **tested** — the starter placed and read back (its column cleared to y=250), the nitro placed and clamped at 40,000,000; the 0.2.9 package red at the starter's own id; the control game knows neither id |
| `postTick`'s production rule (sky + daylight + room) | **tested** — 20 FE/t measured against the game clock (`+120` over `dt=6`), the last 20 of room filled exactly, a stone above and midnight each holding 5,000 for 60 ticks; the counterfactual `SELFTEST=solar` fails |
| `canSeeSky` at `pos.above()` | **tested** — the stone at 14,101,6 (the block above the panel) is what stops it, and the readout says `sky covered` |
| `getSkyDarken() < 4` | **tested** — `/time set midnight` stops an open-sky panel and the readout says `day false`; `/time set noon` restarts it (the rate leg) |
| `dimensionType().hasSkyLight()` | **rule-tested** — `the_solar_daylight_test_is_upstreams_two_reads`; no in-game leg (the arena is the Overworld; the End has no sky to test — see the harness note) |
| `isEnergyPresent(DOWN)` | **tested** — a cell under the panel charges to 1,420 while a cell beside it stays 0, and the panel takes no fill |
| `keepEnergy` on a panel | **rule-tested** — `keep_capacity`/`item_buffer` pinned per tier; the payload path itself is proved in game on the furnator, cell, rod, discharger and hopper families |
| the one charge slot (`chargeItems(1)`) | **rule-tested** — `solar_container` and `item_slots` = 1 per tier; the readout's `charge` field prints it (empty in every leg) |
| the eight recipes + advancements | **tested** — `craft_check.py` crafts the pane and all seven panels (output id + count, every ingredient consumed) |
| the `powah:solar_panels` block/item tags | **tested in game** — `execute if block … #powah:solar_panels` passes on a panel and fails on the arena floor |
| loot | **tested** — the loot leg mines all seven tiers in survival and asserts each drops itself |
| the Lens of Ender | `not ported` — the jar has no recipe for it (it is got by right-clicking an enderman with a photoelectric pane, `general.lens_of_ender`, and the panel's lens slot is a GUI upgrade); `solar_production` keeps upstream's `lens_of_ender` parameter and its test pins the rule |
| waterlogging | `not ported` — cosmetic (`SimpleWaterloggedBlock`); the carried blockstate has no waterlogged variant |
| the panel's GUI, side config and redstone modes | `not ported` — slice 1's GUI/side-config gaps |

### Deviations and upstream defects this slice records

1. **`canSeeSky` is read every tick, not every 40.** Upstream refreshes it on `ticks % 40 == 0`
   (a cache to spare a light lookup); the port asks the level each tick, so a placed panel produces
   from its first tick and a stone overhead stops it at once. Measured: the panel read 860 FE 30
   ticks after placement.
2. **Production runs in the block entity's own tick; the push down runs at the level tick's end.**
   A block entity is out of the world during its own tick, so a neighbour's `ENERGY` capability is
   read in the queue phase — the same phase rule slices 7–9 record. Upstream's `postTick` does both
   in one call.
3. **The four frame booleans are set on placement only.** Upstream's `updateShape` also recomputes
   them when a neighbour changes; they are cosmetic (the carried blockstate's multipart) and no
   assertion depends on them.
4. **Waterlogging is not ported** (cosmetic, as above).
5. **The solar item's `minecraft:max_stack_size` is 1** as upstream's `stacksTo(1)`; the block is a
   1-pixel plate (outline and collision) rather than the carry's `oak_fence` guess, and the panel
   takes no FE in through any face (upstream's `AbstractEnergyProvider`).

**Upstream defects fixed:** none. `Misc.canBlockSeeSky(world, pos.above())` looks like an
off-by-one until `SolarBlock.propagatesSkylightDown` is read: the panel's own position never has
full sky light, so upstream reads the block above it, and the port does the same. The one mismatch
found in this slice was in **this slice's own first in-game leg**: the panel produced (1,640 FE in
its buffer) but pushed nothing down, because `push_out_on` read the machine's buffer through
`Direction::Up` — a face a solar panel does not extract on, so the capability answered nothing.
Reading it through `faces[0]` fixed it; the leg that caught it is the "cell under the panel took no
FE" assertion, which is kept.

### Harness findings this slice measured (for the next machine port)

1. **The sky is read at the block above the panel**, so the differential is a block at
   `(x, y+1, z)` — not one at the panel's own position — and a column cleared to y=250 is what
   makes `can_see_sky` true at the arena's y=100 (the arena's own fill stops at y=110).
2. **`set_energy` cannot poll a producing machine for equality**; the rate leg writes the buffer
   and takes two `info` readouts instead, comparing `b - b'` against `t' - t` (both in the same
   `t=` clock). Every earlier exact-value poll in this harness is on a machine that is idle at that
   moment.
3. **`/time set midnight` takes effect on the next tick** and `get_sky_darken` follows it; the leg
   waits 40 ticks before setting the buffer so the equality poll can settle.
4. **The `keepEnergy` pickup's tp can land the player a block low.** Three legs of this wave (two on
   the 0.2.10 game, one on the 0.2.9 game) failed in slice 7's spill phase with an empty inventory
   and a `fell from a high place` line: `ctl tp` onto a dropped item's own position had put the
   player into the 35-block fall the `floor_under` comment describes. The harness now turns
   `fall_damage` off and `keep_inventory` on for the run (no assertion is about either), proves
   each `floor_under` platform is stone, and re-stands the player if a tp lands below y=95. With
   those, the main leg passed and the box has stayed green.
5. **A recipe case must name the previous tier's item in the top row.** The first `craft_check`
   attempt wrote `photoelectric_pane` in all seven panel recipes; the table refused the six
   upgrades and the check failed on the real grid, which is what a fail-capable recipe assertion
   looks like (the fix is `panel(capacitor, top, bottom)`).

The first 0.2.10 archive (388,638 B, `696850bc…`) was built before this section's `README.md` prose
landed, so the archive and the tree differed in exactly that one file. It was rebuilt from the final
tree: the delivered archive is **`13f0e7be…`, 388,951 B, 624 entries**, the packed `README.md` is
byte-identical to the tree's, the two builds are `cmp`-identical, and every one of its 623 tree files
matches the tree byte for byte (`uses.json` is the build's own API-uses report, as in every earlier
package). **PACKAGE_MATCHES_TREE** holds. The delivered archive predates the ender-channel slice's
tree edits, so against what was run in game the `README.md`-only difference is the only difference.

Version: **0.2.9 → 0.2.10** (one patch). The published `[[versions]]` record keeps 0.2.9 and gains
0.2.10 at publish time — this port does not touch the registry entry.

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices

## Slice 11 — the Ender Cell and the Ender Gate (2026-11-23)

### The slice, quoted from this port's own gaps

`FINDINGS.md` §"Slice 1 boundary", the line naming the family:

> Gaps (recorded, not claimed): the other tiers and the creative cell; the
> Magmator/Thermo/Solar/Reactors; **Ender Cells/Gates**; Energizing Orb/Rods; …

and §"Slice 10"'s closing list:

> **Still open:** the creative cell, the Magmator and Thermo Generator …, the reactors
> (`blocked(uraninite-worldgen)`), the ender cells/gates, the player transmitter, side config,
> redstone modes, the GUIs, particles, worldgen.

and the 0.2.10 report's own "Next slice", which named it:

> the Ender Cell and Ender Gate channel family (`powah:ender_cell_*` and `powah:ender_gate_*`,
> seven tiers each) — the last machines-tier gap that needs neither a mod fluid tank nor worldgen.

**Chosen:** the **Ender Cell** and the **Ender Gate**, all seven tiers of each.

**Why it fits `machines`:** two block entities with a declared energy capability, a saved channel
value, an item slot that takes an energy cell as an upgrade, two charge slots, and a level-tick
move; the buffer itself is one shared store per owner and channel, which is what makes two blocks
in different places the same storage. No renderer, no GUI, no entity, no worldgen, no fluid tank.

**Not chosen (and why):** the Magmator and Thermo Generator need a mod block entity fluid tank the
pin cannot fill, the reactors stay `blocked(uraninite-worldgen)`, the creative cell is a source and
not a tier, and the player transmitter is the next item-energy slice (it charges a player's
inventory, not a block's).

### What upstream does (read from tag `v6.2.10`, commit `a5a8f25`)

| Fact | Where |
|---|---|
| `devices.ender_cells = new EnderConfig(batteryTransfer(), TieredChannelValues.getDefault())` and `devices.ender_gates = new EnderConfig(cableTransfer(), …)`: the cell moves at the energy cell's per-side row, the gate at the cable's | `config/v2/PowahConfig.java:98-105`, `config/v2/types/EnderConfig.java` |
| `TieredChannelValues.getDefault()` = **1, 2, 3, 5, 7, 9, 12** channels for starter…nitro; `EnderNetwork.MAX_CHANNELS = 12`, and the tile saves its index in a `RangedInt(12)` (0..11) | `config/v2/values/TieredChannelValues.java`, `block/ender/EnderNetwork.java:22`, `AbstractEnderTile.java:31,44,52` |
| `EnderConfig.getCapacity` is **0**: the network starts empty and is grown only by an extender item | `config/v2/types/EnderConfig.java:15-24` |
| **The buffer is not the block's**: `getEnergy()` returns `EnderNetwork.get(serverLevel).getEnergy(this, channel).setTransfer(getEnergyTransfer())` — one shared `Energy` per (owner UUID, channel), a `SavedData` named `powah_network` on the overworld, so the value survives a block being broken and the world being reloaded | `block/ender/AbstractEnderTile.java:154`, `EnderNetwork.java` |
| `keepEnergy()` is **false** (no override anywhere in the cell/gate path): nothing is written to the block item — there is nothing in the tile to keep | `lib/block/AbstractEnergyStorage.writeStorable`, `AbstractEnderTile` |
| Cell: `isExtender()` true → slot 0 takes an `IEnderExtender` (an **energy cell item** or a battery); `onSlotChanged` adds `getExtendedCapacity` to the channel and `getExtendedEnergy` to its stored value, then consumes one item and plays the ender-eye sound | `AbstractEnderTile.java:66-84`, `item/EnergyCellItem.java`, `item/BatteryItem.java` |
| Gate: `isExtender()` false, `canInsert` only slots > 0, and `isEnergyPresent(side)` is **the block's `facing` side only** — a gate is a tap between one face and a channel | `block/ender/EnderGateTile.java` |
| Both: `postTick` = `chargeItems(1, 3)` (slots 1 and 2) then `extractFromSides` (six faces for a cell, one for a gate); `RangedInt channel` is set by `SetChannelPacket` | `AbstractEnderTile.postTick`, `network/packet/SetChannelPacket.java` |
| Blocks: `Blcks.ENDER_CELL` / `ENDER_GATE` = `Properties.metalNoSolid(2.0f, 20.0f)`, seven normal tiers; the gate is `noCollission()` with `Facing.ALL` and a 4×4×0.5-pixel plate per face | `block/Blcks.java:32-38`, `EnderGateBlock.java` |

The guide book says the same in words: "store energy (FE) to a specific channel of the ender
network of the owner … if the Ender Cell … have an active channel with a valid capacity", and
"Unlike the Ender cell you can not Upgrade the network from it"
(`assets/powah/guides/powah/book/storage_transfer/ender_{cell,gate}.md`).

### What it ships

Rust:

| File | Change |
|---|---|
| `src/runtime.rs` | `MAX_CHANNELS` (12), `ENDER_TIER_COUNT`, `EnderTier { transfer, channels }`, `ender_cell_tier` (`batteryTransfer` = `baseScaling × 1,000`), `ender_gate_tier` (`cableTransfer` = `baseScaling × 500`), `channel_clamped` (upstream's `getMaxChannels`), `Channel { stored, capacity }` with `insert`/`extract`/`extended`, `charge_slots` (the pass `hopper_charges` now delegates to); the `energizing()` table gains the jar's `ender_core` row; 5 tests |
| `src/powah.rs` | `ENDER_CELLS`/`ENDER_GATES`, `Kind::EnderCell`/`Kind::EnderGate`, `ender_container` (three slots), `ChannelKey`/`ChannelStore` with its `AttachmentType` codec, `EnderEnergy` (the `ENERGY` capability that *is* the channel), `ender_capability` (a gate answers only its `facing` side), `EnderPart` (absorb an extender, charge 1-2, queue), `absorb_extender`, `charge_from_channel`, `facing_at`, the move-phase arms, `ender_line`, `/powah channel`, the `set_energy`/`info_line` arms, owner and gate facing on placement; 5 tests |
| `src/lib.rs` | 14 declarations: a container, the `ENERGY` capability and its Part per tier, plus the item; `/powah channel` in the command body |

Content: see [`CONTENT.md`](CONTENT.md) §"Slice 11" — the 14 blocks with written definitions (the
cell's plain metal block; the gate's six-value `facing`, facing-plate shape and empty collision),
14 blockstates, 16 block models, 15 item models and 15 item definitions, 25 textures (the seven
cells' animated ones with their `.mcmeta`, the gate's shared body and seven item textures, the
core), 14 loot tables, the 14 recipes and their unlock advancements, the `powah:ender_core` the
recipes are built from, the `powah:ender_cells`/`powah:ender_gates` block and item tags, the
pickaxe/needs-iron entries and the creative tab; `tools/carry_slice11.py` reproduces the **149**
files from the raw carry (`--check` green). The fourteen `_2` recipes (one cell into one cell) are
**not** carried, and why is in §"Deviations" below.

### The readouts this slice adds

```text
Ender Cell at 30,100,30: 100000/4000000 FE (comparator 0, channel 0/2, 4000 FE/t a side, charge empty, empty, t=4298)
Ender Cell at 30,100,34: 100000/4000000 FE (comparator 0, channel 0/1, 1000 FE/t a side, charge empty, empty, t=4304)
Ender Cell at 30,100,30: 0/0 FE (comparator 0, channel 1/2, 4000 FE/t a side, charge empty, empty, t=4321)
Ender Gate at 24,100,30: 86000/4000000 FE (comparator 0, channel 0/1, facing north, 500 FE/t a side, charge empty, empty, t=4436)
Ender Cell at 14,100,10: 2000000000/2000000000 FE (comparator 15, channel 1/12, 2000000 FE/t a side, charge empty, empty, t=4571)
Ender Gate at 14,100,14: 7000/4000000 FE (comparator 0, channel 0/12, facing north, 1000000 FE/t a side, charge empty, empty, t=4594)
```

The line names the **channel's own** stored and capacity (which is why two blocks on one channel
print the same numbers), the channel index of the tier's count, the tier's transfer a call, and —
for a gate — the face it reaches energy on. The two charge slots print after `charge`, as the
hoppers' and panels' do.

### TDD: the red proof, quoted

`ender_cell_tier`, `ender_gate_tier`, `channel_clamped`, `Channel::insert`, `Channel::extract`,
`Channel::extended` and `charge_slots` were declared unfinished (`todo!()`) and the tests written
against them before any glue (`ports/_shared/evidence/powah/slice11-test-red.log`):

```text
---- runtime::tests::every_ender_tier_is_one_of_the_ladder_with_its_own_transfer_and_channels stdout ----

thread 'runtime::tests::every_ender_tier_is_one_of_the_ladder_with_its_own_transfer_and_channels' (1170713) panicked at src/runtime.rs:301:5:
not yet implemented: the ender cell ladder

---- runtime::tests::a_channel_takes_and_gives_at_most_its_transfer stdout ----

thread 'runtime::tests::a_channel_takes_and_gives_at_most_its_transfer' (1170689) panicked at src/runtime.rs:339:5:
not yet implemented: a channel insert

---- runtime::tests::a_channel_index_is_clamped_to_the_tiers_own_offering stdout ----

thread 'runtime::tests::a_channel_index_is_clamped_to_the_tiers_own_offering' (1170688) panicked at src/runtime.rs:321:5:
not yet implemented: the channel clamp

---- runtime::tests::an_extender_adds_its_capacity_and_its_own_charge stdout ----

thread 'runtime::tests::an_extender_adds_its_capacity_and_its_own_charge' (1170708) panicked at src/runtime.rs:353:5:
not yet implemented: a channel extension

---- runtime::tests::an_ender_pass_charges_its_two_slots_out_of_one_budget stdout ----

thread 'runtime::tests::an_ender_pass_charges_its_two_slots_out_of_one_budget' (1170707) panicked at src/runtime.rs:364:5:
not yet implemented: a charge pass

---- runtime::tests::a_hopper_pass_charges_each_item_at_its_rate_within_one_budget stdout ----

thread 'runtime::tests::a_hopper_pass_charges_each_item_at_its_rate_within_one_budget' (1170694) panicked at src/runtime.rs:364:5:
not yet implemented: a charge pass


test result: FAILED. 73 passed; 6 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

(The hopper's pass test failed with the ender's because `hopper_charges` now delegates to the new
`charge_slots`, which is the one rule both use. A first green attempt then failed my own arithmetic
— I had written `4,000,001` for a 1,000-capacity channel grown by a 4,000,000 item, which is
`4,001,000`; the expectation was wrong, not the code, and it was corrected before the green run.)
After the implementation: **84 passed, 0 failed** (`slice11-test-green.log`, and the same 84 in
`sdk-test.log`: 74 slice-1…10 tests plus the ten this slice adds).

### Ladder (0.2.11, pin `0a875da`)

Every compile step ran in `ports/powah`, `CARGO_INCREMENTAL=0`, inside a
`ports/tools/build-slot.sh run --job W2-powah --port ports/powah --step …` lease with
`BUILD_LOCAL=1`; logs in `ports/_shared/evidence/powah/`.

| Step | Command | Result |
|---|---|---|
| unit tests | `cargo test --offline` | **84 passed**, 0 failed (`sdk-test.log`, `slice11-test-green.log`) |
| lints | `cargo clippy --all-targets --offline -- -D warnings` | exit 0, 0 warnings (`slice11-clippy-fmt.log`) |
| formatting | `cargo fmt --check` | exit 0, empty (`slice11-clippy-fmt.log`) |
| package, twice | `rm -f Cargo.lock; cargo horizon-mod build` ×2, `cmp` | **byte-identical, 482,716 B**, sha256 `49ba3462341a8460bd020a5e5816113a974be3fee78555d226eab4923b71612f` (`slice11-build-twice.log`); `BUILD1_EXIT:0`, `BUILD2_EXIT:0`; the packed `README.md` is byte-identical to the tree's. The first archive of this slice was **482,574 B**, sha256 `bf99ad985baff71204ad707e6d89a63470655791e81277537ebedff2366e8e4f`: the legs below ran it, and it differs from the delivered one only in `README.md` (this section's command list landed after the run, as slice 7's and slice 10's README edits did) |
| registry check | `cargo horizon-mod check powah.hmod` | **exit 0**, `powah 0.2.11 (native-mod, format native-1/0a875da…/e59da90…, for 26.3)`, `482716 bytes, SHA-256 49ba3462…1612f, unsigned` (`slice11-check-fits.log`) |
| fits the pin | `cargo horizon-mod fits powah.hmod --build 0a875da` | `0a875da  made for`, exit 0 (`slice11-check-fits.log`) |
| fits **another** build | `cargo horizon-mod fits powah.hmod --build 0b14f8a` | `0b14f8a  fits`, exit 0 (`slice11-check-fits.log`) |
| dialect | `python3 ports/tools/check_261_dialect.py powah.hmod` | exit 0: **239** recipe/loot files in the 26.3 dialect (`slice11-check-fits.log`) |
| carried content | `python3 tools/carry_slice11.py --check` | 149 files match |
| package gate | `python3 ports/tools/verify-sdk-package.py powah` | **1/1**, runtime byte-identical (`cfdc211d1106`), no packed `Cargo.lock`, **770 entries**, the package matches the tree (`sdk-verify.log`) |

The first `cargo horizon-mod build` of the bumped tree failed at the gate's shape (`invalid type:
floating point 0.375, expected an array of length 6` — a box is a six-number *list*, and I had
written a flat array); the stale `target/horizon-mod/powah.hmod` made the `cmp` still say
"identical", which is why the second run prints `BUILDn_EXIT` and the exported `version`, and why
the artifact was deleted before rebuilding. The trap is recorded below.

### In-game legs

Launcher build `Compiling powah (1 of 1)`, `native-0a875da-32e311705549`, instance `powah-e2e`
(`evidence/powah-e2e/launcher-build-0.2.11.log`; the re-issued run reports *Up to date* because the
sources it compiled are the sources on disk, and the exported
`horizon-native-mods/powah/horizon-mod.toml` carries `version = "0.2.11"`); logs in
`ports/_shared/evidence/powah-e2e/`.

| Leg | Command | Result |
|---|---|---|
| main | `launcher-game.sh env powah` + `build-slot.sh run --step e2e` | **PASS** + `EXIT:0` (`e2e-launcher-0.2.11.log`) |
| control (no package, inside the main run) | the control game, `Unknown block`/`Unknown item` for the ender ids | the three ender ids are unknown there and the arena stays air (`e2e-launcher-0.2.11.log` lines 30–31) |
| control (whole scenario, no package) | `GAME=$CONTROL_GAME …` | `FAIL: expected powah:furnator_starter at 0,100,0, got 'minecraft:air'` + `EXIT:1` (`e2e-control-0.2.11.log`) |
| selftest (the pre-existing flip) | `SELFTEST=1 …` | `FAIL: SELFTEST flipped: the unfuelled furnator made no FE, as it should` + `EXIT:1` (`e2e-selftest-0.2.11.log`) |
| selftest (this slice's flip) | `SELFTEST=ender …` | `FAIL: SELFTEST flipped: the peer still reads 100000 FE after the shared cell moved channel` + `EXIT:1` (`e2e-selftest-ender.log`) |
| **the previous package** (0.2.10, the parity rule's red) | the 0.2.10 archive in a `powah-0210` instance + the same scenario | `FAIL: expected powah:ender_cell_basic at 30,100,30, got 'minecraft:air'` + `EXIT:1`, after every slice 1–10 phase passed (`e2e-oldpackage-0.2.10.log`) |
| log gate | `check-e2e-logs.py powah-e2e --strict` | `logs=43 ok=43 missing-exit=0 bad-exit=0`, exit 0 |

Quoted from `e2e-launcher-0.2.11.log` (the family's own assertions):

```text
== slice 11: an ender cell's channel is one shared store per owner ==
  extended: Ender Cell at 30,100,30: 0/4000000 FE (comparator 0, channel 0/2, 4000 FE/t a side, charge empty, empty, t=4281)
  channel 0 (basic):   Ender Cell at 30,100,30: 100000/4000000 FE (comparator 0, channel 0/2, 4000 FE/t a side, charge empty, empty, t=4298)
  channel 0 (starter): Ender Cell at 30,100,34: 100000/4000000 FE (comparator 0, channel 0/1, 1000 FE/t a side, charge empty, empty, t=4304)
== slice 11: a cell moved to another channel leaves its peer's FE behind ==
  moved to channel 1: Ender Cell at 30,100,30: 0/0 FE (comparator 0, channel 1/2, 4000 FE/t a side, charge empty, empty, t=4321)
  peer on channel 0:  Ender Cell at 30,100,34: 100000/4000000 FE (comparator 0, channel 0/1, 1000 FE/t a side, charge empty, empty, t=4329)
  back on channel 0:  Ender Cell at 30,100,30: 100000/4000000 FE (comparator 0, channel 0/2, 4000 FE/t a side, charge empty, empty, t=4348)
  channel(30,100,34) reply: Channel set to 0 of 0..=0.
== slice 11: an ender cell charges its two charge slots out of the channel ==
  charge: Ender Cell at 30,100,30: 92000/4000000 FE (comparator 0, channel 0/2, 4000 FE/t a side, charge powah:energy_cell_starter 9000/1000000, empty, t=4311)
  charge: Ender Cell at 30,100,30: 80000/4000000 FE (comparator 0, channel 0/2, 4000 FE/t a side, charge powah:energy_cell_starter 21000/1000000, empty, t=4323)
  dt=12 ticks, charge item +12000 FE, channel -12000 FE
== slice 11: a channel gives its tier's transfer a tick to every face ==
  sink:  Energy Hopper at 31,100,30: 56000/4000000 FE (comparator 0, facing north, charging 2000 FE/t, 4000 FE/t a side, t=4349)
  sink:  Energy Hopper at 31,100,30: 132000/4000000 FE (comparator 0, facing north, charging 2000 FE/t, 4000 FE/t a side, t=4368)
  cell:  Ender Cell at 30,100,30: 948000/4000000 FE (comparator 3, channel 0/2, 4000 FE/t a side, charge empty, empty, t=4355)
  cell:  Ender Cell at 30,100,30: 872000/4000000 FE (comparator 3, channel 0/2, 4000 FE/t a side, charge empty, empty, t=4374)
  sink dt=19 ticks (+76000 FE), channel dt=19 ticks (-76000 FE)
== slice 11: an ender gate reaches one face only, and moves at the cable's rate ==
  gate:        Ender Gate at 24,100,30: 86000/4000000 FE (comparator 0, channel 0/1, facing north, 500 FE/t a side, charge empty, empty, t=4436)
  facing side: Energy Hopper at 24,100,29: 24000/1000000 FE (comparator 0, facing north, charging 500 FE/t, 1000 FE/t a side, t=4442)
  back side:   Energy Hopper at 24,100,31: 0/1000000 FE (comparator 0, facing north, charging 500 FE/t, 1000 FE/t a side, t=4449)
  dt=13 ticks, gate's sink +6500 FE
== slice 11: the orb makes the ender core both ladders are built from ==
  output(0,100,36) = {"count": 1, "item": "powah:ender_core", "slot": 200}
== slice 11: the ender ladders, their channels, their tiers and their tags ==
  channel(14,100,10) reply: Channel set to 1 of 0..=11.
  set(14,100,10) = Ender Cell at 14,100,10: 2000000000/2000000000 FE (comparator 15, channel 1/12, 2000000 FE/t a side, charge empty, empty, t=4565)
  info(14,100,14): Ender Gate at 14,100,14: 7000/4000000 FE (comparator 0, channel 0/12, facing north, 1000000 FE/t a side, charge empty, empty, t=4594)
  tag(30,100,30) #ender_cells = {"chat":["Test passed"],"ok":true}
  tag(30,100,30) #ender_gates = {"chat":["Test failed"],"ok":true}
  tag(24,100,30) #ender_gates = {"chat":["Test passed"],"ok":true}
  tag(0,99,3)    #ender_cells = {"chat":["Test failed"],"ok":true}
```

Every number is exact and its leg can fail: the two cells on one channel read the **same**
100,000/4,000,000 with their own tier's transfer printed beside it (a starter tier reading 4,000,
or a peer reading 0, fails), the cell moved to channel 1 reads **0/0** while its peer stays at
exactly **100,000** (`SELFTEST=ender` expects 0 and fails), moving it back finds the channel's FE
(the store outlives the block on it), the charge item takes **12,000 over dt=12** while the channel
loses the same (1,000 a tick is the *item's* own limit, not the cell's 4,000 — 4,000 would fail),
the sink gains **76,000 over dt=19** and the channel loses **76,000 over dt=19** (4,000 a tick, one
store), the gate's sink gains **6,500 over dt=13** (500 a tick, the cable row) while the hopper
behind the gate stays **0**, the nitro cell clamps 2,000,000,000 on its own 2,000,000,000 capacity
and names 12 channels, both tags match on their own blocks and fail on the arena floor, and all
fourteen tiers drop themselves.

### Coverage (parity rule, the family this slice adds)

| Family (from the jar) | Status |
|---|---|
| the seven `ender_cell_*` and seven `ender_gate_*` blocks and items | **tested** — the starter, basic, nitro cells and gates placed and read back; the 0.2.10 package knows none of them (the control game's `Unknown block type`), and every tier drops itself |
| the shared channel (one store per owner and channel) | **tested** — two cells on channel 0 read the same 100,000/4,000,000; a cell moved to channel 1 reads 0/0 while its peer keeps exactly 100,000, and moving it back finds the FE; `SELFTEST=ender` fails |
| the extender item (`onSlotChanged`, `IEnderExtender`) | **tested** — a basic energy cell item in slot 0 gives the channel 4,000,000 FE of capacity and is consumed (the readout goes from 0/0 to 0/4000000) |
| `postTick`'s `chargeItems(1, 3)` | **tested** — the starter cell item in slot 1 takes 1,000 a tick (its own per-call limit) while the channel loses exactly the same; the two slots share one budget (rule test) |
| the channel's per-call transfer (`batteryTransfer` / `cableTransfer`) | **tested** — a basic cell gives 4,000 a tick and a starter gate 500 a tick, both against the game's clock, with the sink's gain equal to the channel's loss |
| `EnderGateTile.isEnergyPresent(facing)` | **tested** — the hopper on the gate's `facing` side is fed, the one behind it stays exactly 0 |
| `keepEnergy` is false (nothing on the item) | **tested in game** — the cells and gates dropped carry no payload and read their channel; the payload family itself is slice 7's |
| the channel index and the tier's count (`TieredChannelValues`, `getMaxChannels`) | **tested** — a basic cell's `/powah channel 5` answers `Channel set to 0 of 0..=1.`, the nitro's `1` answers `1 of 0..=11.`, and a non-ender block is refused |
| the fifteen recipes (`ender_core` in the orb, 7 cells, 7 gates) | **tested** — the orb energizes an ender eye + casing + tiny capacitor into the core at 50,000 FE, and `craft_check.py` crafts all fourteen in a real table (output id + count, every ingredient consumed) |
| the `powah:ender_cells`/`powah:ender_gates` block and item tags | **tested in game** — each passes on its own family and fails on the other and on the arena floor |
| loot | **tested** — all fourteen tiers mined in survival, each dropping itself |
| the GUI, the channel buttons, particles and the waterlogged gate | `not ported` — slice 1's GUI gap; `/powah channel` is the packet's replacement and the shape is the block definition's |
| the battery items as extenders | `not ported` — no slice carries `powah:battery_*`; the seven energy cell items are the extenders this package has |

### Deviations and upstream defects this slice records

1. **The channel store is one per level, not one per server.** Upstream's `EnderNetwork` is a
   `SavedData` on the overworld, so a channel reaches every dimension; the pin has no `SavedData`,
   so the store is a level attachment (`Holder::Level`, the `transfer_labels` pattern) and a
   channel is per level. A cell in the Nether and one in the Overworld do not share a channel; the
   figure the two would share is the same store's job in a later slice, if the pin grows one.
2. **The `ENERGY` capability is the channel, registered with `block_capability_before_game`.** A
   declared `ModEnergyBuffer` carries a static capacity and reads its amount from the block
   entity's own `energy` NBT, which is exactly what a shared channel is not; the accumulator's
   pattern (a handler that reads and writes the store) is what expresses upstream's single `Energy`
   object, and it makes two cells on a channel answer the same amount with no mirror to keep.
3. **`/powah channel` clamps to the tier's count.** Upstream's tile saves a `RangedInt(12)` (0..11)
   and its GUI offers `getMaxChannels()` buttons (1/2/3/5/7/9/12); a crafted `SetChannelPacket`
   could put a starter cell on any 0..11 its own GUI would never show. The port's command is the
   GUI's replacement, so it offers what the GUI offers, and a doctored saved index is still clamped
   to 0..11 on read.
4. **The extender is absorbed one item a tick.** Upstream's slot limit 1 refuses a second item in
   slot 0; a container declaration cannot express a per-slot limit at this pin (the same finding as
   the hopper's), so a stack automation puts there is absorbed one item a tick, each adding its own
   capacity and charge.
5. **The ender-eye sound of an extender insertion is not played** (cosmetic; no sound calls in this
   port), and the gate's waterlogging is not carried (cosmetic, as slice 10's panel).
6. **The fourteen `_2` recipes are not carried.** Each is upstream's datagen self-recipe — one
   ender cell into one ender cell, one gate into one gate — which changes nothing: `keepEnergy()` is
   false, so the item holds no data for a recipe to preserve, and the output is the same stack the
   input was. Carrying them would add fourteen no-op recipes and fourteen no-op assertions.
7. **A doctored `stored` above the capacity reads as stored.** The channel's entry is written by
   this port only (there is no block `energy` key to doctor), and `Channel::extended` clamps; the
   readout prints what the store holds.

**Upstream defects fixed:** none. The one inconsistency found is item 3 (the saved range wider than
the tier's GUI), and it is recorded as a deviation with the reason rather than "fixed", because
which of the two is upstream's intent is not decidable from the artefact.

### Harness findings this slice measured (for the next machine port)

1. **A channel's readout is the channel's, so both ends of a transfer print the same number.** The
   rate leg measures the sink and the channel separately and compares *rates* per tick, because two
   readouts of one block are a chat round trip apart (measured 5 ticks of latency on a loaded box)
   and a shared store's number moves under both ends.
2. **The receiver's `max_insert` caps a push before the giver's transfer does.** A basic ender cell
   pushing at 4,000 into a *starter* hopper (insert limit 1,000) delivers 1,000 a tick; the leg
   uses a basic hopper (4,000) to measure the cell's own rate. Same rule as slice 9's charging.
3. **`/powah set` on an ender block writes the channel, not the block.** An equality poll on a cell
   that is feeding a neighbour never settles, so the legs use `set_floor`; a fresh channel with no
   extender clamps everything to 0, which is what makes the extender leg's first read a hard 0/0.
4. **The channel is the world's, not the block's**: breaking the block or moving the cell to
   another channel leaves the FE where it was, which is the assertion `SELFTEST=ender` inverts.
5. **A fixed tolerance on a two-readout sum is a flake on a busy box.** Slice 9's "the two halves
   add up" leg used ±1,500 FE and measured 2,500 unaccounted when five build slots were busy; the
   tolerance is now derived from the measured window (`500 × the ticks between the two readouts`),
   which is the FE that could legitimately have moved between them. Recorded here because it was
   the only pre-existing leg this slice had to touch.

The `target/horizon-mod/powah.hmod` trap: a failed `cargo horizon-mod build` leaves the *previous*
archive in place, so a `cmp` between two runs still says "identical" and a copy of it looks like a
successful build. The build script now deletes the artifact first, prints `BUILDn_EXIT`, and reads
the exported `version` back from the archive.

Version: **0.2.10 → 0.2.11** (one patch). The published `[[versions]]` record keeps 0.2.10 and
gains 0.2.11 at publish time — this port does not touch the registry entry.

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices

## Slice 12 — the Player Transmitter and its Binding Card (2026-11-23)

### The slice, quoted from this port's own gaps

`notes.md` slice 11's coverage row named the family:

> the creative cell, the generators, the reactors, the ender gates, the player transmitter, the
> batteries | `not ported` — the reactors stay `blocked(uraninite-worldgen)`; the generators need a
> mod fluid tank and sky visibility

slice 11's closing list:

> **Still open:** the creative cell, the Magmator and Thermo Generator …, the reactors
> (`blocked(uraninite-worldgen)`), the player transmitter, the batteries

and the 0.2.11 report's own "Next slice", which named it:

> the Player Transmitter (`powah:player_transmitter_*`, seven tiers) with its binding card
> (`powah:binding_card`, a `BOUND_PLAYER` component) — the last item-energy family: a block entity
> with a declared FE buffer, one slot holding a card that names a player, `keepEnergy()` true, and a
> tick that charges that player's inventory at `chargingTransfer` out of its own buffer.

**Chosen:** the **Player Transmitter** — all seven tiers — and the **Binding Card** it takes, with
the three items its recipes are built from (`blank_card`, `aerial_pearl`, `player_aerial_pearl`).

**Why it fits `machines`:** a block entity with a declared block energy buffer (a `ChargingConfig`
over `AbstractEnergyStorage`), one slot whose item is a card, a `keepEnergy` payload and a tick that
charges another *inventory*'s items through each item's own energy handler. No renderer, no GUI, no
entity, no worldgen, no fluid tank.

**Not chosen (and why):** the Magmator and Thermo Generator need a mod block entity fluid tank the
pin cannot fill, the reactors stay `blocked(uraninite-worldgen)`, and the creative cell is a source,
not a tier. The **creative cell** was the fallback the brief named if the card's bind step turned out
to be more than one slice; it is not: the bind is one `use_item` hook plus one registered component,
both of which this pin has (measured before the slice started).

### What upstream does (read from tag `v6.2.10`, commit `a5a8f25`)

| Fact | Where |
|---|---|
| `devices.player_transmitters = new ChargingConfig(batteryCapacity(), batteryTransfer(), chargingTransfer())`: the energy cell's storage row and `chargingTransfer` (= `cableTransfer`, `baseScaling * 500`) as the charge rate — starter **1,000,000 FE / 1,000 a side / 500 a tick**, nitro 2,000,000,000 / 2,000,000 / 1,000,000 | `config/v2/PowahConfig.java:114-117`, `config/v2/DefaultEnergies.java` |
| `postTick`: `checkRedstone()`; `stack = inv.getFirst()`; if it is a `BindingCardItem` and `card.getPlayer(level, stack)` finds the bound `ServerPlayer`, and `card.isMultiDim() || player.level().dimensionType().equals(world.dimensionType())`, then `ChargeUtil.chargeItemsInPlayerInv(player, getChargingSpeed(variant), energy.getStored())` and `energy.consume(extracted)` | `block/transmitter/PlayerTransmitterTile.java:47-63` |
| `ChargeUtil.chargeItemsInPlayerInv` = `transferSlotList` over `Player.invStacks` (`inventory.items` 36 + `armor` 4 + `offhand` 1 = **41 slots**), each item's `Capabilities.EnergyStorage.ITEM.receiveEnergy(min(rate, total - charged))` | `util/ChargeUtil.java:27-33`, `util/Player.java:48-53` |
| `getSlotLimit(0) = 1`, `canInsert(slot, stack) = stack.has(BOUND_PLAYER)`, `canExtract` true, `keepEnergy()` true | `PlayerTransmitterTile.java:65-79` |
| The block is `Properties.metalNoSolid(2.0, 20.0)` with a `top` state and a second, decorative block above; the block entity exists only in the bottom half (`newBlockEntity` returns `null` for `top`) | `block/Blcks.java:52`, `PlayerTransmitterBlock.java` |
| `BindingCardItem.use`: unbound → `stack.set(BOUND_PLAYER, new BoundPlayer(uuid, displayName))`; bound to someone else → a chat line and `FAIL`; on an EnderMan/Endermite it becomes `binding_card_dim` | `item/BindingCardItem.java:52-72` |
| `BoundPlayer.CODEC`: `{gameProfileId: [I;4] (UUIDUtil.CODEC), name: String}` under the registered component `powah:bound_player` | `components/BoundPlayer.java`, `components/PowahComponents.java:39` |
| `AerialPearlItem.interactLivingEntity`: an aerial pearl on a Zombie/Husk/ZombieVillager gives the player a `player_aerial_pearl` and kills it — the starter transmitter recipe's `p` | `item/AerialPearlItem.java:23-45` |

### What it ships

Rust:

| File | Change |
|---|---|
| `src/runtime.rs` | `TRANSMITTER_TIER_COUNT`, `TransmitterTier` (= `HopperTier`, the same three rows), `transmitter_tier`, `PLAYER_INVENTORY_SLOTS`, `player_slot` (the `ItemAccess.forPlayerSlot` numbers), `player_charges`; **3 tests** |
| `src/powah.rs` | `TRANSMITTERS`, `BINDING_CARD`, `AERIAL_PEARL`, `PLAYER_AERIAL_PEARL`, `BOUND_PLAYER`, `PLAYER_SLOTS`; `Kind::Transmitter`; `item_buffer`/`kind_item_slots`/`keep_capacity`/`battery_of` arms; `transmitter_battery`, `transmitter_container`; the `BoundPlayerComponent` (`powah:bound_player`); `CardPart` (`use_item`), `PearlPart` (`item_interact_mob`); `TransmitterPart` with `charge_bound_player` + `charge_player_inventory`; `CardReadout`, `transmitter_line`, the `/powah info` and `/powah set` arms; **4 tests** |
| `src/lib.rs` | seven transmitters (a container, a buffer, a type, a Part and a machine item each), the card's `item_part`, the pearl's `item_part` and the pearl item key, and `component_types` registering `powah:bound_player` |
| `src/runtime.rs` (network) | the relay's plan now moves FE only from the fuller **storage** to the emptier one; a **provider** (`can_insert` false: a furnator, a solar panel, a discharger) always gives — see "upstream defects fixed" |
| `tools/carry_slice12.py` | 7 block definitions/blockstates/models/textures/loot/recipes/tags, the transmitter's shared body, the card, blank card, aerial pearl and player aerial pearl, the creative tab and the mineable tags — 93 files, `--check` green |
| `tools/craft_check.py` | the ten new recipes (7 transmitters, binding card, blank card, aerial pearl) |
| `tools/e2e.sh` | the transmitter phase, its blank-card control, the ladder/tag/keep/loot legs, the aerial-pearl leg and `SELFTEST=transmitter`; plus three harness fixes (below) |

Content (93 files, all reproducible with `python3 tools/carry_slice12.py --check`): the seven
`player_transmitter_*` blocks (metal, 2.0/20.0, stack 1) and their items, models, textures, loot and
`powah:player_transmitters` block and item tags; `powah:binding_card` (stack 1), `powah:blank_card`,
`powah:aerial_pearl` and `powah:player_aerial_pearl`; the ten recipes (the jar's own
`player_tranmitter_*` spelling kept) and their unlock advancements.

### The red proof

**1. The family's rules, before any of them existed** (`cargo test`, 84 passed / 2 failed):

```
---- runtime::tests::every_player_transmitter_tier_is_the_battery_config_with_its_own_charging_rate stdout ----
thread '...' panicked at src/runtime.rs:202:5:
not yet implemented: slice 12: the player transmitter tier

---- runtime::tests::a_player_pass_charges_each_inventory_item_at_its_rate_within_one_budget stdout ----
thread '...' panicked at src/runtime.rs:213:5:
not yet implemented: slice 12: the transmitter's pass over a player's inventory

test result: FAILED. 84 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

**2. The glue's three tests**, against a plausible wrong implementation each (the tests were written
with the glue; each was then run against a mutation and failed, then the mutation reverted):

```
assertion `left == right` failed: powah:player_transmitter_starter keeps its buffer on its own item
  left: None
 right: Some(1000000)

assertion `left == right` failed: a bound card names the player the tick charges
  left: "Player Transmitter at 3,100,5: 5000/1000000 FE (comparator 0, bound AdLf0208, charging 1000 FE/t, 1000 FE/t a side, t=1234)"
 right: "Player Transmitter at 3,100,5: 5000/1000000 FE (comparator 0, bound AdLf0208, charging 500 FE/t, 1000 FE/t a side, t=1234)"

test powah::tests::a_binding_card_component_is_the_uuid_and_the_name_and_nothing_else_is_a_player ... FAILED
```

**3. The relay's ping-pong, before the fix** (`a_network_equalizes_two_storages_and_never_takes_back_what_it_gave`):

```
thread '...' panicked at src/runtime.rs:1314:9:
assertion `left == right` failed: the rotation starts at the receiver; a network never takes back what it gave
  left: [Transfer { from: 1, to: 0, amount: 500 }]
 right: []
```

**4. The item-place slot numbering** (`a_player_slot_number_is_the_item_places_not_the_inventorys`):

```
thread '...' panicked at src/runtime.rs:218:5:
not yet implemented: slice 12: the item place's player slot number
```

**5. In game**, four runs before the green one, each catching a real defect (logs kept):

| Run | Verbatim |
|---|---|
| `e2e-0.2.12h-glue-before-player-slot.log` | `FAIL: the transmitter must have spent its whole 10,000 on the item in that player's inventory: Player Transmitter at 16,100,22: 10000/1000000 FE (comparator 0, bound Player10726, …)` — the inventory slot was read through `ItemPlace::EntitySlot`, which does not resolve a player; `PlayerSlot` (and its own numbering) is what the pin reads |
| `e2e-0.2.12j-recipes-wrong-grid.log` | `FAIL: blank card + 2 redstone + rotten flesh -> a binding card: expected 1 powah:binding_card, got None xNone` — a 2×2 pattern sits at the grid's top left (100/101, 103/104), not at 101/106/107 |
| `e2e-0.2.12e.log` (earlier) | `FAIL: using an unbound binding card did not write powah:bound_player on it` — `ctl command` answers `{"chat":[…]}`; the reader took the JSON, not the reply text |

### Green

| Check | Result |
|---|---|
| `cargo test --offline` | **91 passed, 0 failed** (`sdk-test.log`) |
| `cargo clippy --all-targets --offline -- -D warnings` | 0 warnings, `CLIPPY_EXIT:0` (`slice12-clippy-fmt.log`) |
| `cargo fmt --check` | clean, `FMT_EXIT:0` |
| two builds | `BUILD1_EXIT:0`, `BUILD2_EXIT:0`, **byte-identical**, 525,573 B, 860 entries, `version = "0.2.12"`, no packed `Cargo.lock`, the packed `README.md` byte-identical to the tree's (`slice12-build-twice.log`) |
| `cargo horizon-mod check powah.hmod` | `powah 0.2.12 (native-mod, format native-1/0a875da…/e59da909…, for 26.3)` · `525573 bytes, SHA-256 8b264c5d2e9a3da6073c9dc17229cd474d625ed63aabb56d6900922df5be951a`, `CHECK_EXIT:0` |
| `cargo horizon-mod fits powah.hmod --build 0a875da` | `Powah:` / `0a875da  made for` (`FITS_PIN_EXIT:0`) |
| `cargo horizon-mod fits powah.hmod --build 0b14f8a` | `Powah:` / `0b14f8a  fits` (`FITS_OTHER_EXIT:0`) |
| `python3 ports/tools/check-recipe-types.py powah` | `powah: ok`, exit 0 |
| `python3 ports/tools/verify-sdk-package.py powah` | **1/1**, 860 entries, every packed file the tree's bytes (`slice12-check-fits.log`) |
| launcher build | `Compiling powah (1 of 1)`, `native-0a875da-2174565b7b48`, instance `powah-e2e` (`launcher-build-0.2.12.log`) |
| in-game main leg | `PASS` + `EXIT:0` (`e2e-launcher-0.2.12.log`); the control game (the same Nightly without the package) knows none of the ids, the transmitter's included |
| `SELFTEST=1` | `EXIT:1` — `FAIL: SELFTEST flipped: the unfuelled furnator made no FE, as it should` |
| `SELFTEST=transmitter` | `EXIT:1` — `FAIL: SELFTEST flipped: the transmitter charged the item to 10000 FE, as it should` |
| `check-e2e-logs.py powah-e2e --strict` | `logs=50  ok=50  missing-exit=0  bad-exit=0` |

What the in-game leg proves, in order: a fresh transmitter reads
`Player Transmitter at 16,100,22: 0/1000000 FE (comparator 0, bound empty, charging 500 FE/t, 1000 FE/t a side)`;
a card held and used (`ctl use-item`) gains `powah:bound_player` naming the operator; moved into the
slot it makes the readout say `bound Player35726`; `/powah set 10000` then a `powah:energy_cell_starter`
in the operator's own inventory spends the whole 10,000 onto that item and leaves the transmitter at
exactly 0; with 100,000 in the buffer two readouts 29 ticks apart differ by exactly **14,500 = 500 × 29**
while the item gains **29 whole 500 FE steps** (`5500 → 20000`), and item + buffer add up to the
100,000 within the 4,000 FE window the 8-tick read gap allows; a **blank** card in the slot reads `bound none` and
spends nothing while the item stays at 0; the nitro tier clamps 2,000,000,000 FE and names
`charging 1000000 FE/t, 2000000 FE/t a side`; `powah:player_transmitters` passes on a transmitter and
fails on the arena floor; a 7,000 FE transmitter broken in survival carries its buffer back on its own
item; every tier drops itself; and an aerial pearl used on a zombie turns into a player aerial pearl,
spends exactly one of the two carried, and kills the zombie.

### Coverage (parity rule, the family this slice adds)

| Family (from the jar) | Status |
|---|---|
| the seven `player_transmitter_*` blocks and items | **tested** — the starter and the nitro placed and read back, `/powah set 2000000000` clamped at the nitro's 2,000,000,000, both readouts naming their own charging rate and transfer; the control game knows neither the block nor the card |
| the buffer (`ChargingConfig` = `batteryCapacity`/`batteryTransfer`) | **tested** — the starter reads `0/1000000`, `/powah set 10000` holds, the nitro clamps at 2,000,000,000; the tier table pinned per tier against the cell's storage row (rule test) |
| the tick (`ChargeUtil.chargeItemsInPlayerInv`) | **tested** — 10,000 FE of buffer lands on the cell item in the bound player's inventory (transmitter at exactly 0), and at 500 × dt against the game's clock (14,500 over 29 ticks) with whole 500 FE steps and conservation |
| the card, its `use` and its component (`BoundPlayer.CODEC`) | **tested** — `ctl use-item` on a held card writes `powah:bound_player` (name read back = the operator's), the readout names them, and the component round-trips the UUID as four ints (rule test) |
| `canInsert` only a bound card (slot limit 1) | **tested in game as the control** — a blank card sits in the slot (a container declaration cannot refuse it), the tick charges nothing and spends nothing |
| `keepEnergy()` | **tested** — a 7,000 FE transmitter broken in survival and placed again reads 7,000/1,000,000 |
| the ten recipes (7 transmitters, binding card, blank card, aerial pearl) | **tested** — `craft_check.py` crafts all ten in a real table: output id + count, every ingredient consumed |
| `AerialPearlItem.interactLivingEntity` (the starter recipe's `p`) | **tested** — an aerial pearl on a zombie gives a player aerial pearl, spends one of two and kills the zombie |
| the `powah:player_transmitters` block and item tags | **tested in game** — passes on a transmitter, fails on the arena floor |
| loot | **tested** — all seven tiers mined in survival, each dropping itself |
| the decorative `top` half of the block | `not ported` — one block (see deviations); it is a shape and a model, no behaviour |
| `binding_card_dim` and its EnderMan/Endermite upgrade | `not ported` — the dimensional card bypasses the same-dimension test, which with no dim card is always applied |
| the `powah:bound` item model property (`binding_card_bound`) | `not ported` — a client model property, not a machine behaviour; a bound card keeps the plain model |
| the GUI, the JEI page and the sounds | `not ported` — slice 1's GUI gap; the ender-eye and zombie-death sounds are not played (this port has no sound calls) |

### Deviations and upstream defects this slice records

1. **The transmitter is one block, not two.** Upstream's block has a `top` state and places a second,
   decorative block above the one holding the block entity (`PlayerTransmitterBlock.onPlace`), with
   `updateShape` removing both when either goes. This slice carries the **bottom** block only: its
   model, its state-less definition and its shape. A user sees a shorter device in the world than
   upstream's; the behaviour is untouched. (Carrying the top would need `updateShape`/neighbour
   removal hooks and a two-block break path, which is shape work, not machine work.)
2. **`powah:bound_player` is an NBT component, not a typed one.** The pin registers component *names*
   (`Hooks::component_types`) and reads/writes them as NBT; there is no typed codec registry. The
   value is upstream's own compound (`{gameProfileId: [I;4], name}`), so a card written here reads in
   upstream's codec and vice versa.
3. **A blank card may sit in the slot.** Upstream's per-slot `canInsert` refuses a card without
   `BOUND_PLAYER`; a container declaration cannot express a per-slot rule at this pin, so the tick's
   bound-player test is what leaves it alone (and the in-game control proves it spends nothing).
4. **The dimensional card is not carried**, so the same-dimension test is always applied. Upstream's
   `card.isMultiDim() ||` arm has no counterpart to bypass.
5. **The card's bound look is not drawn.** The `powah:bound` item model property is client-side and
   not registered; a bound card keeps the unbound model (`binding_card_bound` is not carried).
6. **No chat line for a card bound to someone else** (`chat.powah.no.binding`): the `use` answer is
   upstream's `FAIL` but this port sends no message (no client text channel is opened here).
7. **The player-slot numbering is the item place's, not `Inventory`'s.** `Player.invStacks` walks
   `Inventory`'s numbers (36 main, the armour at 36–39, the off hand at 40); the pin's
   `ItemPlace::PlayerSlot` takes `ItemAccess.forPlayerSlot`'s (0–35, the armour at 100–103, the off
   hand at −106). `runtime::player_slot` is that map, and the first in-game run proved it matters
   (the `EntitySlot` form read nothing at all).

**Upstream defects fixed: one, in the cable network (a pre-existing behaviour, not this slice's
family).** A network's plan rotated its starting port, so two **storage** blocks on one cable — both
of which answer an extract — ping-ponged the same 500 FE: the receiver gave back next tick what it
had just been given, and the pair never equalized. Measured in game on 2026-11-23 with a trace:
`move 0→1 want=500 got=Ok(500)` then `move 1→0 want=500 got=Ok(500)`, alternating for hundreds of
ticks, with a cell source holding 100,000 and its peer never keeping a charge. The rule is now
upstream's: **a storage gives only to a storage that holds less; a provider (`can_insert` false — a
furnator, a solar panel, a discharger) always gives**, because its buffer is filled by its own tick
and not by the network. Two cells on a cable now equalize, a furnator still feeds a cell through a
run of three cables, and the two cells' sum never changes. This also removed the flake the slice-3
leg had carried since 0.2.3: that leg read the sink on whichever tick its chat reply landed, and the
sink held 500 on one tick and 0 on the next (`e2e-0.2.12f.log`: sink `t=672` at 0 while the source
held 100,000; `e2e-launcher-0.2.11.log`: sink 500).

### Harness findings this slice measured (for the next port)

1. **`ctl command` answers `{"chat":[…]}`.** A `data get` reply is inside the JSON, with the quotes
   escaped, so a `sed` over the raw output reads nothing (it cost this slice a run). Extract with
   `jq -r '[.chat[]?] | join(" ")'` first.
2. **A chat reply can be 100+ ticks late on a busy box, and the 20-line window then holds the
   *previous* answer.** `info` accepted the stale line and read a machine 110 ticks back (slices 3's
   two-cell legs failed on three different runs for that reason alone). It now re-asks until the
   readout's own ` t=` is **newer** than the one read before the command, and gives up with `NO-INFO`
   rather than a number.
3. **A cell-to-cell relay needs the sink read on a real tick, not a lucky one.** See the defect
   above: the assertion was `> 0` on a value that alternated 500/0, so the leg's pass depended on the
   parity of the command's tick.
4. **A player's inventory slot is `ItemPlace::PlayerSlot`, not `ItemPlace::EntitySlot`.** The latter
   resolves a chest minecart or a chested mount, and silently reads nothing for a player (the first
   run's 10,000 stayed in the buffer).
5. **`/data get entity @p Inventory[{Slot:0b}]…` is the stable read of an item the player carries.**
   `@p` is the console's nearest player (the only one in this harness); `@s` on the console path is
   the console, and `SelectedItem` is one selection stale.
6. **A chat reply is not a place to put backticks.** A `read \`bound empty\`` inside the PASS string
   ran a command substitution (`bound: command not found`) — the symptom was a PASS line with a hole
   in it and a hang in `cleanup`.
7. **`ctl quit` can hang on a long session.** After a 17,689-tick run on a loaded box the guard
   killed the leg at its 900 s idle timeout, *after* the PASS line, and the log reported `EXIT:124`.
   `cleanup` now bounds the quit with `timeout 60`.
8. **A two-row pattern sits at the grid's top left** (`100,101 / 103,104`), not one row down; a case
   written as `101/106/107` crafts nothing and reports `got None xNone`.

The in-game legs ran the launcher build of the same sources (`native-0a875da-2174565b7b48`) while
the delivered archive was rebuilt from the final tree after this section's `README.md` prose landed;
the two differ in `README.md` alone, which is not in the game.

Version: **0.2.11 → 0.2.12** (one patch). The published `[[versions]]` record keeps 0.2.11 and gains
0.2.12 at publish time — this port does not touch the registry entry.

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices

## Slice 13 — the redstone mode: `AbstractTileEntity.checkRedstone` (2026-11-23)

### The slice, quoted from this port's own gaps

`## Deviations and gaps (slice 1)`, item 6 — the port's own gap list, unchanged through twelve slices:

> **6. No side config or redstone modes.** Upstream's six per-side transfer types
> and redstone IGNORE/ON/OFF default to `ALL`/`IGNORE`; this slice ships only
> those defaults.

the `## Behaviour` table's row:

> | redstone | `checkRedstone()` — default mode `IGNORE`, so always on | no redstone gate (defaults only); a recorded gap |

slices 8, 9 and 10 repeated it ("the discharger GUI, its side-config/redstone modes, the Energy
Hopper | `not ported` — slice 1's GUI/side-config gaps"), slice 12's diagrams name it
(`postTick`: `checkRedstone()`), and 0.2.12's own next-slice proposal named it as the family that
sits inside the transmitter's existing tick. The **side config** is *not* in this slice: it is the
same gap item's other half, and it needs a per-side declaration the port cannot express (a
`ModEnergyBuffer` carries six face booleans only, not six transfer types). Redstone only needs the
tick the machines already have, which is why it comes first.

### What the artefact says (tag `v6.2.10`, commit `a5a8f25`)

**The mode set is exactly three** — `lib/logistics/Redstone.java`:

| Ordinal (saved) | Mode | `checkRedstone()` returns |
|---:|---|---|
| 0 | `IGNORE` (the default) | `true` — the signal is never read |
| 1 | `ON` | `powered` |
| 2 | `OFF` | `!powered` |

`next()` is `ordinal + 1`, wrapping past `OFF` back to `IGNORE` (the GUI button's cycle).
`powered` is `level.getBestNeighborSignal(pos) > 0` — vanilla's own read, in `Redstone.java`'s
comment "avoid checking redstone if mode is IGNORE: `getBestNeighborSignal` is relatively expensive".

**The rule** — `lib/block/AbstractTileEntity.java:202-214`:

```java
if (Redstone.IGNORE.equals(redstoneMode)) return true;
boolean power = this.level != null && this.level.getBestNeighborSignal(this.worldPosition) > 0;
return power && Redstone.ON.equals(redstoneMode) || !power && Redstone.OFF.equals(redstoneMode);
```

**Which machines honour it** — every `checkRedstone()` call site in the jar, and what each gates:

| Machine | Site | What the gate stops |
|---|---|---|
| Energy Cable | `CableTile.receiveEnergy` | an insertion into the cable's network (the door FE comes in through) |
| Energy Discharger | `EnergyDischargerTile.postTick` | the **item drain** only — `extractFromSides` runs outside the gate |
| Energy Cell | `EnergyCellTile.canExtractEnergy`/`canReceiveEnergy` | every face, both ways (the capability itself) |
| Ender Cell, Ender Gate | `AbstractEnderTile.canExtractEnergy`/`canReceiveEnergy` | every face, both ways (the channel handler) |
| Furnator | `FurnatorTile.postTick` | the **burn** only (the fuel load included) — `super.postTick`'s `extractFromSides` runs outside it |
| Energy Hopper | `EnergyHopperTile.postTick` | the charge it spends on its faced container |
| Solar Panel | `SolarTile.postTick` | the **generation** only — the item charge and `extractFromSides` run first |
| Player Transmitter | `PlayerTransmitterTile.postTick` | the whole pass (upstream returns before it reads the card) |
| Magmator, Thermo Generator, Reactor | their `postTick` | the generation — **not ported** (unported machines) |

**The energizing orb and the rods have no redstone mode at all**: neither calls `checkRedstone()`
and neither screen is an `AbstractEnergyScreen`, so neither offers the button. **The control**
upstream is that GUI button: `AbstractEnergyScreen.init` calls `addRedstoneButton` when
`hasRedstoneButton()`, and the button sends `NextRedstoneModePacket(pos)` →
`tile.nextRedstoneMode(); tile.sync()`. The set of machines with the button is exactly the set with
the call site. **The saved key** is `redstone_mode`, an int ordinal, in the block entity's *sync*
compound (`writeSync`/`readSync`) — **not** in `writeStorable`, so a broken machine's item carries
its buffer and never its mode.

### What it ships

| File | Change |
|---|---|
| `src/runtime.rs` | `RedstoneMode` (`ALL`, `ordinal`, `from_ordinal`, `name`, `command_word`, `named`, `next`) and `check_redstone(mode, powered)`; assertions added to the red test below |
| `src/powah.rs` | `KEY_REDSTONE_MODE`, `has_redstone_mode`, `redstone_target`, `saved_mode`, `powered`, `gate_open`, `gate_open_at`, `faces_open`, `RedstoneField`/`redstone_field`; the gate inside the furnator's, the solar panel's and the transmitter's own tick, inside the level-tick phase for the cell, the discharger, the hopper and the cable, and inside `EnderEnergy::insert`/`extract`; the readout's `redstone <mode>[ (powered)]` field; `set_redstone_mode` and `/powah redstone ignore|ignored|on|off <pos>`; **3 tests** |
| `src/lib.rs` | the `redstone` command arm |
| `tools/e2e.sh` | the slice-13 phase (the brief's rig in all three modes, then one machine of each kind) and `SELFTEST=redstone` |
| `horizon-mod.toml`, `Cargo.toml`, `README.md` | version 0.2.13 and the slice's paragraph |

No content change: the mode is data and one rule, not blocks or items.

### The rule in the port

`gate_open(host, level, pos, saved)` = `runtime::check_redstone(saved_mode(saved), powered)` where
`saved_mode` reads upstream's key (absent → `IGNORE`) and `powered` is
`LevelRef::best_neighbor_signal(host, pos) > 0` — the pin's own `SignalGetter.getBestNeighborSignal`
read, so a lever, a repeater, a torch and a redstone block all answer exactly as they do upstream.
The default short-circuits before the signal read, as upstream's does.

Where each machine's gate sits, in the port's own structure:

| Kind | Gate site | Matches upstream |
|---|---|---|
| Furnator | the tick's `burn` call | yes — the push at the level-tick end is outside it |
| Solar | the tick's `solar_production` | yes — the item charge runs first, the push is outside |
| Transmitter | the tick's `charge_bound_player` | yes |
| Cell | the level-tick `push_out`, and `faces_open` on every port this port's movers see | yes, both faces |
| Ender cell / gate | inside `EnderEnergy::insert`/`extract`, and `faces_open` at the relay's ports | yes, both faces, for a foreign mover too |
| Discharger | the level-tick `drain_items` | yes — `push_out` still runs |
| Hopper | the level-tick `charge_container` | yes |
| Cable | the level-tick `relay_network` for that cable | the cable's own relay, not only the insertion path (see deviations) |
| Orb, rods | none — `has_redstone_mode` is false and the command refuses with "no redstone mode" | yes (upstream has neither) |

### The red proof

**1. The mode's rules, before either existed** (`cargo test`, **91 passed / 2 failed**;
`evidence/powah/slice13-red-runtime.log`):

```
---- runtime::tests::the_three_redstone_modes_are_upstreams_ordinals_and_cycle_in_its_order stdout ----

thread '...' panicked at src/runtime.rs:952:9:
not yet implemented: slice 13: a redstone mode's ordinal

---- runtime::tests::a_machine_runs_unless_its_mode_names_the_other_power_state stdout ----

thread '...' panicked at src/runtime.rs:987:5:
not yet implemented: slice 13: whether a machine's redstone gate is open

test result: FAILED. 91 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

**2. In game, five runs before the green one**, each catching a real defect:

| Run | Verbatim |
|---|---|
| first | `FAIL: expected /powah redstone ignored at 16,100,22 to answer 'Redstone mode set to ignore.', got 'Redstone mode set to on.'` — the usage was declared as `ignore` (the enum's own spelling) while the command sent `ignored`; the dispatcher refused the word and no mod command ran. Fixed by `RedstoneMode::command_word` (`ignored`, with `ignore` still accepted). |
| second | the same two words one step on (`got 'Redstone mode set to ignored.'`) — the *test* still expected `ignore.` |
| third | `FAIL: IGNORE with a powered face must keep charging … got 10000 -> 10000 FE` — the leg set the buffer to 10,000, which 500 FE a tick spends in 20 ticks, so both reads saw the item already full. Fixed by setting 200,000 **before** the item arrives (a transmitter with something to charge never settles an equality poll). |
| fourth | `FAIL: a foreign block must refuse a redstone mode, got '{"chat":["That block has no redstone mode."],"ok":true}'` — `set_redstone_mode` asked `has_redstone_mode` before asking whether the block was this mod's, so a stone answered "no mode" where upstream's packet (which only acts on an `AbstractTileEntity`) answers nothing. Fixed with `redstone_target` (`NoBlock` before `NoMode`), pinned by the test. |
| fifth | `FAIL: could not clamp 12,100,48 to 5000 FE: Ender Cell at 12,100,48: 4500/5000000 FE (… channel 0/1 …)` — the ender channel store is **the world's**, keyed by (owner, channel), so a starter cell (one channel) shared channel 0 with slice 11's extended cells: their extenders' 4,000,000 capacity showed and a neighbour still on that channel took the 500. Fixed by giving the leg a **nitro** ender cell on channel 11. |

One more in the same wave: `floor_under 6 48`'s `/fill` answered *"No blocks were filled"* and the
next read found air, so the platform assertion stopped the leg before the discharger. The phase
never teleports the player, and a machine placed by `setblock` needs no support, so the phase's
`floor_under` calls were removed.

### Green

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices

| Check | Result |
|---|---|
| `cargo test --offline` | **96 passed, 0 failed** (`sdk-test.log`) |
| `cargo clippy --all-targets --offline -- -D warnings` | 0 warnings, `CLIPPY_EXIT:0` |
| `cargo fmt --check` | clean, exit 0 |
| two builds | `BUILD1_EXIT:0`, `BUILD2_EXIT:0`, **`cmp`-identical** (533,187 B, 860 entries, `version = "0.2.13"`, no packed `Cargo.lock`) — `slice13-build-twice.log` |
| `cargo horizon-mod check powah.hmod` | `powah 0.2.13 (native-mod, format native-1/0a875da…/e59da909…, for 26.3)` · `533187 bytes, SHA-256 70f218d63974277c44b1d98fe6fd2a90b3fb131e45ecc212972ff532e3d9cf04`, `CHECK_EXIT:0` |
| `cargo horizon-mod fits powah.hmod --build 0a875da` | `Powah:` / `0a875da  made for`, `FITS_PIN_EXIT:0` |
| `cargo horizon-mod fits powah.hmod --build 0b14f8a` | `Powah:` / `0b14f8a  fits`, `FITS_OTHER_EXIT:0` |
| `python3 ports/tools/check-recipe-types.py powah` | `powah: ok`, exit 0 (no recipe changed) |
| `python3 ports/tools/verify-sdk-package.py powah` | **1/1**, 860 entries, every packed file the tree's bytes |
| launcher build | `Compiling powah (1 of 1)`, `native-0a875da-13e86a810b6a`, instance `powah-e2e` (`launcher-build-0.2.13.log`) |
| in-game main leg | **`PASS` + `EXIT:0`** (`e2e-launcher-0.2.13.log`), on the delivered archive (`package: powah.hmod (533187 B, sha256 70f218d6…)`); the control game (the same Nightly without the package) knows none of the ids |
| `SELFTEST=1` | `EXIT:1` — `FAIL: SELFTEST flipped: the unfuelled furnator made no FE, as it should` (`e2e-selftest-0.2.13.log`) |
| `SELFTEST=redstone` | `EXIT:1` — `FAIL: SELFTEST flipped: the shut gate charged nothing, as it should` (`e2e-selftest-redstone-0.2.13.log`) |
| `check-e2e-logs.py powah-e2e --strict` | `logs=53  ok=53  missing-exit=0  bad-exit=0`, exit 0 |

### The in-game transcript (the brief's assertion, quoted from `e2e-launcher-0.2.13.log`)

The rig: the starter transmitter at **16,100,22** with its bound card in slot 0, a
`powah:energy_cell_starter` in the operator's hotbar, and a **redstone block at 15,100,22** (its
west face; a redstone block powers all six neighbours).

```
== slice 13: the redstone mode's command and its readout ==
  redstone(16,100,22) reply: Redstone mode set to off.
  info(16,100,22): Player Transmitter at 16,100,22: 66000/1000000 FE (comparator 0, bound Player85052, charging 500 FE/t, 1000 FE/t a side, redstone off, t=4804)
  redstone(16,100,22) reply: Redstone mode set to on.
  info(16,100,22): Player Transmitter at 16,100,22: 61500/1000000 FE (comparator 0, bound Player85052, charging 500 FE/t, 1000 FE/t a side, redstone on, t=4828)
  redstone(16,100,22) reply: Redstone mode set to ignored.
  info(16,100,22): Player Transmitter at 16,100,22: 53000/1000000 FE (comparator 0, bound Player85052, charging 500 FE/t, 1000 FE/t a side, t=4855)
== slice 13: IGNORE charges through a powered face, the brief's rig ==
  set(16,100,22) = Player Transmitter at 16,100,22: 200000/1000000 FE (comparator 3, bound Player85052, charging 500 FE/t, 1000 FE/t a side, t=4870)
  ignored item: 14000 -> 28000 FE
== slice 13: OFF with a powered face stops the charge, and the buffer stands still ==
  gated: Player Transmitter at 16,100,22: 171500/1000000 FE (comparator 2, bound Player85052, charging 500 FE/t, 1000 FE/t a side, redstone off (powered), t=4946)
  gated over 20 ticks: Player Transmitter at 16,100,22: 171500/1000000 FE (comparator 2, bound Player85052, charging 500 FE/t, 1000 FE/t a side, redstone off (powered), t=4973) (item 28500 -> 28500 FE)
== slice 13: the same rig with the block removed charges again (500 x dt) ==
  opened: Player Transmitter at 16,100,22: 153500/1000000 FE (comparator 2, bound Player85052, charging 500 FE/t, 1000 FE/t a side, redstone off, t=5017) over dt=29, transmitter -14500 FE, item 35000 -> 49500 FE
== slice 13: ON needs the power, and both halves of the truth table hold ==
  on, unpowered: Player Transmitter at 16,100,22: 150000/1000000 FE (comparator 2, bound Player85052, charging 500 FE/t, 1000 FE/t a side, redstone on, t=5037) (item 50000 -> 50000 FE)
  on, powered: Player Transmitter at 16,100,22: 147000/1000000 FE (comparator 2, bound Player85052, charging 500 FE/t, 1000 FE/t a side, redstone on (powered), t=5071) (item 56000 FE)
```

The rest of the phase, one machine of each kind that moves FE on its own:

```
== slice 13: a shut cell gives nothing on any face, so the cable beside it pulls nothing ==
  gated cell: Energy Cell at 2,100,48: 5000/1000000 FE (comparator 0, charge empty, empty, 1000 FE/t a side, redstone off (powered), t=5399) / sink: Energy Cell at 2,100,50: 0/1000000 FE (… t=5406)
  opened: sink Energy Cell at 2,100,50: 2500/1000000 FE (… t=5434)
== slice 13: a shut cable relays nothing while an open one beside it does ==
  gated cable: sink Energy Cell at 4,100,50: 0/1000000 FE (… t=5506)
  opened cable: sink Energy Cell at 4,100,50: 2500/1000000 FE (… t=5534)
```

(the furnator's leg reads `carbon 0, burning false, fuel minecraft:coal x5` while shut and makes FE
once opened; the discharger keeps its item's 5,000 and drains it once opened; the hopper keeps its
5,000 and spends it on the chest's item once opened; the solar panel holds 5,000 under a noon sky
and generates once opened; the nitro ender cell on channel 11 gives the cable nothing while shut and
feeds it once opened.)

### Coverage (parity rule, the family this slice adds)

| Family (from the jar) | Status |
|---|---|
| the three modes and `checkRedstone`'s truth table | **tested** — the unit test pins the ordinals (0/1/2), the cycle, the words and all six mode×powered answers; in game all three modes × both power states are measured on the transmitter |
| `/powah redstone <mode> <pos>` (the GUI button's `NextRedstoneModePacket`) | **tested** — three replies asserted (off/on/ignored), a foreign block answers "No Powah block there." and the orb "That block has no redstone mode." |
| the readout's `redstone <mode>[ (powered)]` | **tested** — on the transmitter, the cell and the solar panel; absent on the default, which keeps every earlier line byte-identical (unit test + every earlier leg green) |
| the transmitter's `postTick` gate | **tested** — the brief's rig: item and balance frozen over 20 ticks with the block on the west face, `500 × dt` once it is removed, ON needing the power |
| the furnator's burn | **tested** — no fuel taken, no carbon burnt, no FE while shut |
| the cell's faces | **tested** — its 5,000 stays put and the cable behind it delivers nothing |
| the cable's relay | **tested** — a gated cable relays nothing, the same cable relays once opened |
| the discharger's item drain | **tested** — the item keeps its 5,000 |
| the hopper's charge | **tested** — the buffer keeps its 5,000 and the chest's item stays empty |
| the solar panel's generation | **tested** — 5,000 under a noon sky, unchanged for 40 ticks |
| the ender cell's (and gate's) handler | **tested** — the cell refuses the cable at `EnderEnergy`; the gate shares the same handler and gate |
| the magmator, the thermo generator, the reactor | `not ported` — the machines themselves are unported (slice 1's gap 11), so their gates have nothing to gate |
| the GUI's redstone button | `not ported` — slice 1's GUI gap; `/powah redstone` is its replacement, as `/powah channel` is the ender GUI's |
| a **foreign** mover's view of a gated cell | `blocked(pin: block_energy has no per-instance hook)` — see deviation 2 |
| the mode on a broken machine's item | `not carried` — upstream's own `writeStorable` omits `redstone_mode`, so the item carries the buffer and never the mode |

### Deviations and upstream defects this slice records

1. **The button is a command.** Upstream's mode is set from the machine's screen and any player may
   press it; this port has no GUI (slice 1's gap 1), so `/powah redstone <mode> <pos>` sets it, and
   it is operator-only (level 2) like every other harness control here, because it acts at
   coordinates the caller chooses.
2. **A foreign mover still sees a gated cell's declared buffer.** The pin's
   `Registrar::block_energy` declaration is per *block*, not per instance: a cell's `ENERGY`
   capability cannot ask its own saved mode, so a block from another mod that pulls on a gated cell
   still gets FE. This port's own movers (a push, a cable's relay) ask `faces_open` first, and the
   ender cell and gate — whose handler is this port's own `EnderEnergy` — refuse inside `insert` and
   `extract` even to a foreign mover. Expressed with the closest supported API, recorded rather than
   faked.
3. **A cable's gate closes that cable's whole relay**, where upstream's gate closes only
   `CableTile.receiveEnergy` (the door an insertion comes in through; a network already inside still
   relays). The port's cable *is* its network's entry point, so a gated cable relays nothing — and it
   does not claim the network, so another cable of the same network that is open still relays.
4. **The readout names the gate only when it is not the default.** `IGNORE` is what a placed machine
   has, and an untouched machine must print the line twelve slices of legs assert; the field appears
   from `, redstone on`/`off` on. The `t=` clock stays the last field, which is what the harness's
   `time_of` anchors on.
5. **`ignored` is the command's word, `ignore` the readout's.** Upstream's enum is `IGNORE`; both
   parse, the usage declares `ignored` (the brief's and the notes' word), and the reply repeats it.
6. **The mode is saved but not carried.** Faithful: upstream writes `redstone_mode` in `writeSync`
   only, so a broken machine is placed with `IGNORE` again. Nothing to do.

**Upstream defects fixed: none.** The five in-game failures above were this port's own defects (a
command word, two test expectations, a short buffer, an ordering bug in `set_redstone_mode` and a
shared ender channel in the leg), not the mod's.

### Harness findings this slice measured (for the next port)

1. **A command's usage word and the word the test sends must be one value.** `MODE` was declared as
   `ignore` while the leg sent `ignored`: the dispatcher refused it and the only reply was the
   *previous* command's, which a polling helper reads as a stale answer. Pin both from one function
   (`RedstoneMode::command_word`).
2. **The ender channel store is world data.** A leg sharing channel 0 with an earlier slice's
   extended cells inherits *their* capacity (4,000,000 + 1,000,000 = 5,000,000 in the first run) and
   can lose FE to a neighbour they left standing. Use a tier with spare channels (the nitro cell's
   twelve) and a channel no other leg touches.
3. **`IGNORE` must short-circuit before the signal read**, or every machine pays a
   `best_neighbor_signal` per tick for a gate nobody set (upstream says so in a comment; the port
   does the same).
4. **A redstone block powers all six neighbours** (`getBestNeighborSignal` reads the block's own
   signal), so one block on any face is the whole rig — no lever, no dust.
5. **A `SELFTEST` flip is `[ wrong ] || fail`.** Writing `[ right ] || fail` under `SELFTEST=…`
   makes the leg *pass* when the behaviour is correct (this happened once: the redstone selftest
   exited 0 before the flip was inverted).
6. **`floor_under`'s `/fill` can answer "No blocks were filled"** for a region the platform check
   then reads as air — a phase that never teleports the player needs no platform at all.

Version: **0.2.12 → 0.2.13** (one patch). The published `[[versions]]` record keeps 0.2.12 and
gains 0.2.13 at publish time — this port does not touch the registry entry.

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices

## Slice 14 — the comparator reading (`analog_output`, 2026-11-23)

### The slice, quoted from this port's own gaps

`## Deviations and gaps (slice 1)`, item 7 — unchanged through thirteen slices:

> **7. No comparator output.** `comparator_strength` is computed and printed, but
> no `has_analog_output` hook is declared, so a vanilla comparator beside a
> machine still reads container fullness.

and the `## Behaviour` table's row:

> | comparator | `hasAnalogOutputSignal` from the FE fraction | computed and printed by the command; no
> `has_analog_output` hook declared — gap |

The brief for this slice fixes the shape: declare it only for the blocks that have a buffer **and**
a `comparator_strength` today, and answer no signal for the rest.

### What the artefact says (tag `v6.2.10`, commit `a5a8f25`)

- `lib/block/AbstractEnergyBlock.hasAnalogOutputSignal` is `true` for every machine that extends it,
  and `getAnalogOutputSignal` returns `AbstractEnergyStorage.getEnergy().toComparatorPower()` —
  `lib/logistics/energy/Energy.java:227-233`, `(int)(stored / capacity * 15)` with `subSized()` 0 at
  capacity 0. That is exactly `runtime::comparator_strength`, the number the readout prints.
- The energizing orb overrides both with `EnergizingOrbBlock`'s non-empty stack count (a different
  rule, not this slice), and a cable and a rod inherit `AbstractEnergyBlock` — the cable's storage is
  0 (so 0), the rod computes its own fraction.

### What it ships

| File | Change |
|---|---|
| `src/powah.rs` | `has_comparator` and `comparator_of` (the set and the number), `Outputs` (record/get/clear), the `OUTPUTS` cache, `strength_at`/`saved_energy`, `publish_outputs` at the level tick's end, `publish_strength` on `/powah set`, `notify_comparators`, the two hooks on the six machine Parts; **3 tests** |
| `src/lib.rs` | the comparator flag on `IdlePart` (a cell and a discharger answer, a rod and a cable do not); `clear_outputs` on the world lifecycle |
| `tools/e2e.sh` | the slice-14 phase and `SELFTEST=comparator` |
| `horizon-mod.toml`, `Cargo.toml`, `README.md` | version 0.2.14 and the slice's paragraph |

No content change: the reading is data and one rule, not blocks or items.

### The rule in the port

`has_comparator(kind)` is exactly the machines whose readout prints a `comparator` field — every
furnator, cell, discharger, hopper, solar panel, transmitter, ender cell and ender gate.
`comparator_of(kind, stored, capacity)` returns `comparator_strength(clamp_energy(stored,
capacity), capacity)`, i.e. the line's own number, or `None` for a cable, the orb and the rods.

The comparator hooks (`BlockPart::has_analog_output`/`analog_output`, API 64) receive only a
read-only `LevelHost`, which cannot reach a block entity (`LevelHost::block_entity_data` needs a
`BlockEntityRef`, and only `Host::block_entity`/`block_entity_identity` make one). So the value is
recorded, not read live: after the level tick's move phase, every queued machine's strength is
written into `OUTPUTS`, keyed by the level's dimension key and the position; `/powah set` records its
value at once. A **changed** reading calls `LevelRef::update_neighbors(host, pos, state,
NeighbourUpdate::OutputSignal)` (`Level.updateNeighbourForOutputSignal`), which is what a buffer
changing without its block state changing needs — vanilla's `setChanged` path. `analog_output`
answers `OUTPUTS[dimension][pos]`, 0 where nothing recorded one. The cache is cleared when a world
opens or closes.

### The red proof

**The rules, before either existed** (`cargo test`, **96 passed / 3 failed**;
`evidence/powah/slice14-test-red.log`):

```
---- powah::tests::only_the_machines_with_a_printed_comparator_answer_one stdout ----
thread '...' panicked at src/powah.rs:663:5:
not yet implemented: slice 14: whether a machine answers a comparator

---- powah::tests::a_cable_the_orb_and_the_rods_answer_no_comparator stdout ----
thread '...' panicked at src/powah.rs:663:5:
not yet implemented: slice 14: whether a machine answers a comparator

---- powah::tests::a_comparators_reading_is_per_position_and_per_dimension stdout ----
thread '...' panicked at src/powah.rs:2174:9:
not yet implemented: slice 14: read a machine's comparator reading

test result: FAILED. 96 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

**In game, two runs before the green one:**

| Run | Verbatim |
|---|---|
| first | `FAIL: data get block -4 100 51 OutputSignal answered '{"chat":["The target block is not a block entity"],"ok":true}'` — at this pin a comparator has **no block entity**: its output lives in the level's `comparator_outputs` map, so the brief's `data get block … OutputSignal` has nothing to read. Replaced by the game's own path: a redstone wire in front of the comparator (its `facing` side is its input, the opposite side its output) carries exactly its signal as `power`. |
| second | `FAIL: a comparator beside a half-full cell must be powered, got ''` — `ctl block` answers `block` and `state` (`name[prop=value,…]`), not a `properties` object (the pneumaticcraft harness's jq already knew); the helper now parses the state string. A one-minute probe (`probe-slice14.log`) then showed `powered=true` and `wire … power=7` before a 15-minute leg depended on it. |

### Green

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices

| Check | Result |
|---|---|
| `cargo test --offline` | **99 passed, 0 failed** (`sdk-test.log`, regenerated for 0.2.14) |
| `cargo clippy --all-targets --offline -- -D warnings` | 0 warnings, exit 0 (`slice14-clippy.log`) |
| `cargo fmt --check` | clean, exit 0 (`slice14-fmt.log`) |
| two builds | `BUILD1_EXIT:0`, `BUILD2_EXIT:0`, **`cmp`-identical** — **538,155 B**, sha256 `c6f1e813acea2b4891f40eebfc1e49e5ee889449715458206939234fb53f0ac2`, 860 entries, `version = "0.2.14"`, `LOCKLINE:0`, `WORKSPACE:0` (`slice14-build-twice.log`) |
| `cargo horizon-mod check powah.hmod` | `powah 0.2.14 (native-mod, format native-1/0a875da…/e59da909…, for 26.3)` · `538155 bytes, SHA-256 c6f1e813…`, `CHECK_EXIT:0` |
| `cargo horizon-mod fits powah.hmod --build 0a875da` | `Powah:` / `0a875da  made for`, `FITS_PIN_EXIT:0` |
| `cargo horizon-mod fits powah.hmod --build 0b14f8a` | `Powah:` / `0b14f8a  fits`, `FITS_OTHER_EXIT:0` |
| `python3 ports/tools/verify-sdk-package.py powah` | **1/1**, `package matches the tree`, `no packed Cargo.lock  860 entries`, `SDK-only dependencies (R9)`, `tests 99 passed`, `skills gate PASS` (`slice14-verify.log`) |
| `python3 ports/tools/check-recipe-types.py powah` | `powah: ok`, exit 0 (no recipe changed) |
| launcher build | `Compiling powah (1 of 1)`, tag `native-0a875da-2417ecf27009`, instance `powah-e2e` (`launcher-build-0.2.14.log`) |
| in-game main leg | **`PASS` + `EXIT:0`** on the delivered archive (`package: powah.hmod (538155 B, sha256 c6f1e813…)`) |
| in-game control | the whole leg on the same Nightly **without** the package: `FAIL: expected powah:furnator_starter at 0,100,0, got 'minecraft:air'` + `EXIT:1` (`e2e-control-0.2.14.log`); the main log's own control phase shows every powah id unknown there |
| `SELFTEST=1` | `EXIT:1` — `FAIL: SELFTEST flipped: the unfuelled furnator made no FE, as it should` (`e2e-selftest-0.2.14.log`) |
| `SELFTEST=comparator` | `EXIT:1` — `FAIL: SELFTEST flipped: the comparator read 7, the readout's own 7, as it should` (`e2e-selftest-comparator-0.2.14.log`) |
| `check-e2e-logs.py powah-e2e --strict` | `logs=57  ok=57  missing-exit=0  bad-exit=0`, exit 0 |

### The in-game transcript (quoted from `e2e-launcher-0.2.14.log`)

The rig: a starter cell at **-4,100,52** set to 500,000, a `minecraft:comparator[facing=south]` at
**-4,100,51** (a comparator's `facing` side is its input, so it reads the cell) and a
`minecraft:redstone_wire` at **-4,100,50**, the comparator's output side.

```
== slice 14: a comparator beside a machine reads the strength its readout prints ==
  block(-4,100,52) = powah:energy_cell_starter
  set(-4,100,52) = Energy Cell at -4,100,52: 500000/1000000 FE (comparator 7, charge empty, empty, 1000 FE/t a side, t=5847)
  readout: Energy Cell at -4,100,52: 500000/1000000 FE (comparator 7, charge empty, empty, 1000 FE/t a side, t=5854)
  comparator(-4,100,51) = powered true, the wire in front reads 7, the readout says 7
== slice 14: the reading follows the buffer (15 at full, 3 at a quarter) ==
  full: Energy Cell at -4,100,52: 1000000/1000000 FE (comparator 15, …) -> comparator 15
  quarter: Energy Cell at -4,100,52: 250000/1000000 FE (comparator 3, …) -> comparator 3
== slice 14: a stone in the same rig reads nothing ==
  stone: wire 0, powered false
== slice 14: a cable answers nothing, even with a full container behind it ==
  chest: wire 1, powered true
  cable: wire 0, powered false (the full chest sits behind it at 0,100,53)
== slice 14: a full rod answers nothing (its readout prints no comparator) ==
  full rod: Energizing Rod at 2,100,52: 10000/10000 FE (100 FE/t a side, no orb, t=6032) -> wire 0
```

The 500,000 case prints and reads **7**, not the brief's parenthetical 8: upstream's
`toComparatorPower()` is `(int)(7.5)`, truncating, and the readout has always printed 7. The
assertion is the equality with the readout, which is what the brief calls for; the number itself is
the artefact's.

### Coverage (parity rule, the family this slice adds)

| Family (from the jar) | Status |
|---|---|
| the reading (`Energy.toComparatorPower`, all 8 machine kinds) | **tested** — the unit test pins every kind's reading against its own printed line at one value, at full, and at twice capacity; in game 7, 15 and 3 on a cell |
| `has_analog_output` (the declaration) | **tested** — a comparator beside a cell is `powered=true`; beside a stone it is not |
| `analog_output` (the number and its updates) | **tested** — the wire in front reads 7, then 15 and 3 after `/powah set` (the notification path) |
| a block that answers no signal | **tested** — a cable reads 0 with a full chest behind it after the same rig read that chest as 1; a full rod reads 0 |
| the orb's own comparator (non-empty stack count) | `not ported` — a different rule from the FE fraction, and its readout prints no comparator (this slice's brief excludes it) |
| a rod's fraction, a cable's 0 | `not carried` — upstream declares both through `AbstractEnergyBlock`; the brief limits the hook to the blocks whose readout prints a comparator, so both answer none (deviation 1) |

### Deviations and upstream defects this slice records

1. **Only the printed-comparator machines answer one.** Upstream's `AbstractEnergyBlock` also answers
   for a cable (0, no storage) and a rod (its own fraction), and the orb answers its stack count. The
   brief restricts the hook to the blocks whose `/powah info` line prints a comparator, so a
   comparator beside a full rod reads 0 here where upstream would read 15, and the orb's stack count
   is not carried. Recorded rather than faked; the orb's and rods' readings are the family's next
   slice if it is asked for.
2. **The reading is the end of the last tick, not a mid-tick live read.** The hook's level is
   read-only, so the number is recorded at the level tick's end (and on `/powah set`) and announced
   when it changes. A comparator therefore reads the buffer as of the machine's last completed move
   phase — the same value the machine's own readout shows, one tick's movement behind a foreign mover
   that inserted FE earlier in the same tick.
3. **The brief's `data get block … OutputSignal` is not available at this pin.** A comparator has no
   block entity (its output is `Level.ticks.comparator_outputs`), so the in-game assertion reads the
   game's own consumer: the redstone wire in front of it carries exactly its signal as `power`. The
   comparator's `powered` property is asserted as the brief asks.
4. **The brief's "(8)" is not what the artefact computes.** `comparator_strength(500000,
   1000000)` — and upstream's `(int)(subSized()*15)` — is 7, the number the readout prints; the
   assertion is equality with that field.

**Upstream defects fixed: none.** Neither in-game failure was the mod's: the pin has no comparator
block entity, and `ctl block` has no `properties` object.

### Harness findings this slice measured (for the next port)

1. **`ctl block` answers `block` (a name) and `state` (`name[prop=value,…]`), not a `properties`
   object.** Parse `.state`; a `.properties.powered` read answers `null`/empty and looks like a dead
   block.
2. **`data get block … <key>` needs a block entity.** At this pin a comparator has none, so a
   comparator's output must be measured at its consumer (a redstone wire's `power`); the same holds
   for any vanilla block whose data lives in a level map.
3. **A comparator's `facing` names its input side, not its output side** (`diode_input_signal`: target
   = pos + facing); its output goes the opposite way. A rig placed "on the north face" must
   `facing=south` to read the machine, with the wire to the north.
4. **Redstone dust needs a floor.** A wire `setblock`-ed above air pops off on the next update, so the
   phase lays a stone platform under the rig first (the arena's own floor stops at z=24).

Version: **0.2.13 → 0.2.14** (one patch). The published `[[versions]]` record keeps 0.2.13 and
gains 0.2.14 at publish time — this port does not touch the registry entry.

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices

## Slice 15 — finishing the comparator family (the orb's stack count and a rod's fraction, 2026-11-24)

### The slice, quoted from this port's own gaps

`## Slice 14 — the comparator reading`, `### Deviations and upstream defects this slice records`,
item 1 (the notes' own next-slice authority):

> **1. Only the printed-comparator machines answer one.** Upstream's `AbstractEnergyBlock` also
> answers for a cable (0, no storage) and a rod (its own fraction), and the orb answers its stack
> count. The brief restricts the hook to the blocks whose `/powah info` line prints a comparator, so
> a comparator beside a full rod reads 0 here where upstream would read 15, and the orb's stack count
> is not carried. Recorded rather than faked; the orb's and rods' readings are the family's next
> slice if it is asked for.

Slice 14's coverage table has the same two rows: `the orb's own comparator (non-empty stack count)`
and `a rod's fraction, a cable's 0` marked `not carried`. This slice asks for them, so it closes both.

**Chosen:** the orb's non-empty-stack count and a rod's own buffer fraction. **Why it fits
`machines`:** no new content, no new block entity, no renderer or GUI — the orb's `Part` and both
rods' block entities already tick; only one pure rule and two readings are new. The cable is left at
zero (it stores nothing), so the family is complete without inventing a signal for it.

### The two upstream reading rules (tag `v6.2.10`, commit `a5a8f25`)

1. **`EnergizingOrbBlock.getAnalogOutputSignal`** overrides `AbstractEnergyBlock`: it returns
   `Math.min(15, getInventory().getNonEmptyStacks().size())` — a **count**, not a fraction of the
   buffer. Read in `FINDINGS.md` §Slice 5 (`block/energizing/EnergizingOrbBlock.java`,
   `EnergizingOrbTile.java`; line 146: *"Comparator output is the count of non-empty stacks."*). The
   orb's seven slots are slot 0 (result) and 1–6 (inputs), so all seven are counted.
2. **A rod inherits `AbstractEnergyBlock`**, whose `getAnalogOutputSignal` is `Energy.toComparatorPower()`
   — `(int)(stored / capacity * 15)`, **truncating** (`lib/logistics/energy/Energy.java:227-233`).
   This is the same arithmetic slice 14 already uses (`runtime::comparator_strength`,
   `src/runtime.rs:467-473`): a starter rod at 5,000 of 10,000 reads **7**, not 8; a full rod reads 15.
   `EnergizingRodTile` is an `AbstractEnergyStorage` over `devices.energizing_rods`.
3. **A cable's storage is 0**, so upstream's fraction would be 0. The port keeps its 0.2.14 behaviour
   (no declaration, so a comparator beside a cable still reads the vanilla zero) rather than adding a
   faked signal to make the family look complete.

### What it ships

| File | Change |
|---|---|
| `src/runtime.rs` | `stack_comparator(non_empty_stacks) = min(15, count)` and 2 tests |
| `src/powah.rs` | `analog_reading(kind, stored, capacity, stacks)` (orb count / rod fraction / 8 machines / cable `None`), `orb_stack_count` over all seven slots, `strength_at` uses it, `OrbPart` queues its position and answers `has_analog_output`/`analog_output`, `/powah set` announces a rod's changed fraction; 1 rewritten test |
| `src/lib.rs` | the orb's Part gets the two comparator hooks; a rod's `IdlePart` is declared `comparator = true` |
| `tools/e2e.sh` | the slice-15 phase, `SELFTEST=orb-comparator`, and the full-rod leg's assertion changed from 0 to 15 |
| `horizon-mod.toml`, `Cargo.toml`, `README.md` | version 0.2.15 and the slice's paragraph |

No content change: the two readings are data and one rule, not blocks or items.

### TDD: the red proof, quoted

`stack_comparator`, `analog_reading` and the rewritten test were declared `todo!()` first; `cargo test`
then printed (`ports/_shared/evidence/powah/slice15-test-red.log`, **99 passed / 2 failed**):

```
---- powah::tests::the_orb_and_the_rods_answer_a_comparator_and_the_cable_does_not stdout ----

thread 'powah::tests::the_orb_and_the_rods_answer_a_comparator_and_the_cable_does_not' (20065) panicked at src/powah.rs:701:5:
not yet implemented

---- runtime::tests::the_orb_comparator_counts_non_empty_stacks_capped_at_15 stdout ----

thread 'runtime::tests::the_orb_comparator_counts_non_empty_stacks_capped_at_15' (20119) panicked at src/runtime.rs:480:5:
not yet implemented

failures:
    powah::tests::the_orb_and_the_rods_answer_a_comparator_and_the_cable_does_not
    runtime::tests::the_orb_comparator_counts_non_empty_stacks_capped_at_15

test result: FAILED. 99 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

The rod's arithmetic test (`a_rods_comparator_is_its_own_buffer_fraction_truncated`) passed as written
because `comparator_strength` already computes the truncated fraction; it pins the 5,000→7 value the
in-game leg asserts. After implementation: **101 passed, 0 failed**.

### The rule in the port

The comparator hooks get a read-only `LevelHost` and cannot reach the block entity, so — as in slice
14 — the number is recorded in `OUTPUTS` and announced on change. `strength_at` now reads the orb's
seven item slots (`orb_stack_count`) and a rod's saved `energy` with its tier's capacity, and passes
them to `analog_reading`; `publish_outputs` at the level tick's end records and notifies. `/powah set`
publishes a rod's new fraction immediately (an orb's set changes FE, not the stack count, so its
cached count is left alone). `has_comparator` is unchanged — it is the *readout* set of eight machines,
not the set of blocks that answer an analog signal.

### Ladder (0.2.15, pin `0a875da`)

Every compile step ran in `ports/powah`, `CARGO_INCREMENTAL=0`, inside a
`ports/tools/build-slot.sh run --job W1-powah --port ports/powah --step …` lease with `BUILD_LOCAL=1`;
logs in `ports/_shared/evidence/powah/`.

| Step | Result |
|---|---|
| `cargo test --offline` | **101 passed, 0 failed** (`sdk-test.log`, regenerated for 0.2.15) |
| `cargo clippy --all-targets --offline -- -D warnings` | 0 warnings, exit 0 (`slice15-clippy.log`) |
| `cargo fmt --check` | clean, exit 0 (`slice15-fmt.log`) |
| two builds, `rm -f Cargo.lock` before each | `BUILD1_EXIT:0`, `BUILD2_EXIT:0`, **`cmp`-identical** — **539,968 B**, sha256 `f954aa9099f1ab550176602d406c1a71cba78fde765f881a65c75e58206cbd40`, 860 entries, `version = "0.2.15"`, `LOCKLINE:0`, `WORKSPACE:0` (`slice15-build-twice.log`) |
| `cargo horizon-mod check powah.hmod` | `powah 0.2.15 (native-mod, format native-1/0a875da…/e59da909…, for 26.3)` · `539968 bytes, SHA-256 f954aa90…`, `CHECK_EXIT:0` (`slice15-check.log`) |
| `fits --build 0a875da` | `0a875da  made for`, `FITS_PIN_EXIT:0` (`slice15-fits.log`) |
| `fits --build 0b14f8a` | `0b14f8a  fits`, `FITS_OTHER_EXIT:0` (`slice15-fits-nightly.log`) |
| `python3 ports/tools/verify-sdk-package.py powah` | **1/1**, `package matches the tree`, `SDK-only dependencies (R9)`, `no packed Cargo.lock  860 entries`, `tests 101 passed`, `skills gate PASS` (`slice15-verify.log`) |
| `python3 ports/tools/check-recipe-types.py powah` | `powah: ok`, exit 0 |
| log gate | `python3 ports/tools/check-e2e-logs.py powah-e2e --strict` → `logs=62 ok=62 missing-exit=0 bad-exit=0`, exit 0 |

The delivered `powah.hmod` is **539,968 B**, sha256
`f954aa9099f1ab550176602d406c1a71cba78fde765f881a65c75e58206cbd40`, **860 entries**, no packed
`Cargo.lock`. Both fits lines are real: `--build 0a875da` prints `made for` (a tautology), while
`--build 0b14f8a` prints `fits` — every API item still resolves on the other pinned build. No local
type or variant named `Block`/`Item` was introduced, so `fits` had no false failure.

### In-game (parity rule)

Launcher build `native-0a875da-8d7947c713ed`, instance `powah-e2e`, `Compiling powah (1 of 1)`
(`ports/_shared/evidence/powah-e2e/launcher-build-0.2.15.log`). Every `native-*` tag carrying this mod
was removed first (`879d8e38abee` 0.2.8, `2417ecf27009` 0.2.14, `02302726280a` 0.2.10, then the first
build of this slice) so the launcher recompiled instead of reusing an older install. The main leg is
`ports/_shared/evidence/powah-e2e/e2e-launcher-0.2.15.log`, **`PASS` + `EXIT:0`** on the delivered
archive (`package: powah.hmod (539968 B, sha256 f954aa90…)`), quoted:

```
== slice 14: a comparator answers nothing, even with a full container behind it ==
  chest: wire 1, powered true
  cable: wire 0, powered false (the full chest sits behind it at 0,100,53)
== slice 15: a full rod answers its own buffer fraction (15) ==
  set(2,100,52) = Energizing Rod at 2,100,52: 10000/10000 FE (100 FE/t a side, no orb, t=5664)
  full rod: Energizing Rod at 2,100,52: 10000/10000 FE (100 FE/t a side, no orb, t=5671) -> wire 15
== slice 15: an orb's comparator reads its non-empty stack count ==
  block(-4,100,52) = powah:energizing_orb
  orb: Energizing Orb at -4,100,52: 0/0 FE (no recipe, t=5741)
  comparator(-4,100,51) = powered true, the wire in front reads 3
== slice 15: a half-full rod reads 7 on the wire ==
  set(2,100,52) = Energizing Rod at 2,100,52: 5000/10000 FE (100 FE/t a side, no orb, t=5779)
  half rod: Energizing Rod at 2,100,52: 5000/10000 FE (100 FE/t a side, no orb, t=5786) -> wire 7
```

Three blaze powder are three of the blazing recipe's **four** inputs, so the orb holds no complete
recipe (`0/0 FE`) — the signal is the stack count, not the buffer, which is the point of the rule.
The readout at each rig shows no comparator field for an orb or a rod (0.2.14's assertion stays
byte-identical); the readings are taken at the consumer wire, since a comparator has no block entity
at this pin (slice 14 deviation 3).

| Leg | Result |
|---|---|
| main | `PASS` + `EXIT:0` (`e2e-launcher-0.2.15.log`) |
| control (same Nightly, no package) | `FAIL: expected powah:furnator_starter at 0,100,0, got 'minecraft:air'` + `EXIT:1` (`e2e-control-0.2.15.log`); the main log's own control phase shows every powah id unknown |
| `SELFTEST=1` | `FAIL: SELFTEST flipped: the unfuelled furnator made no FE, as it should` + `EXIT:1` (`e2e-selftest-0.2.15.log`) |
| `SELFTEST=comparator` | `FAIL: SELFTEST flipped: the comparator read 7, the readout's own 7, as it should` + `EXIT:1` (`e2e-selftest-comparator-0.2.15.log`) |
| `SELFTEST=orb-comparator` | `FAIL: SELFTEST flipped: the orb comparator read 3, the three non-empty stacks, as it should` + `EXIT:1` (`e2e-selftest-orb-comparator-0.2.15.log`) |

The first `SELFTEST=orb-comparator` attempt ran in parallel with another game and the client exited
during slice 13 (`{"error":"game exited"}`) before reaching the new assertion; it was re-run alone
under its own lease and failed at the orb assertion above. The discarded run is not kept as evidence.

### Coverage (parity rule, the family this slice closes)

| Family (from the jar) | Status |
|---|---|
| orb comparator (`EnergizingOrbBlock.getAnalogOutputSignal`) | **tested** — three blaze powder in three input slots read 3 on the wire, comparator `powered=true`; unit test pins 0/3/7/20→0/3/7/15 |
| rod comparator (`AbstractEnergyStorage` fraction) | **tested** — a full starter rod reads 15 and a 5,000/10,000 rod reads 7; unit test pins the truncation and 0 |
| cable (0, no storage) | **tested** — unchanged slice-14 rig: a full chest reads 1, the cable in the same spot reads 0 |
| the eight machines' printed fraction (slice 14) | **tested** — unchanged: 7 at half a cell, 15 at full, 3 at a quarter |
| orb's GUI/particles, rod's wrench link, client renderers | **not ported** — recorded slice-5 gaps, unchanged by this data-and-rule slice |

### Deviations and upstream defects this slice records

1. **The cable still answers nothing** rather than declaring upstream's explicit zero. Observably the
   same (a comparator beside one reads 0), and it keeps 0.2.14's measured rig byte-identical.
2. **Orb and rod readings are end-of-tick values**, like slice 14's machines: the hook's level is
   read-only, so an item insert or a rod's fill is recorded at the level tick's end and announced on
   change. A comparator reads what the block entity held at the last completed move phase.
3. **The orb counts all seven slots**, including slot 0 (the result). Upstream's
   `getNonEmptyStacks()` does the same; the in-game rig has no result yet, so its three inputs are the
   whole count.

**Upstream defects fixed: none.** The two readings are upstream's own rules; no defect was found
while measuring them.

### Harness findings this slice measured (for the next port)

1. **Two headless games in parallel on a busy box can exit mid-leg** (`{"error":"game exited"}` at
   tick ~4,900 with five slots occupied by other ports' e2e legs). Run each leg under its own lease,
   one at a time when `MemAvailable` is low; a slot is a concurrency permit, not a promise of RAM.
2. `SELFTEST` modes must invert the assertion they are meant to prove *at the assertion's own phase*;
   the orb mode branches around the normal equality so the run reaches the orb rig before failing.

Version: **0.2.14 → 0.2.15** (one patch). The published `[[versions]]` record keeps 0.2.14 and gains
0.2.15 at publish time — this port does not touch the registry entry. Title: `Update powah 0.2.15`
(the entry is ours on `main`).

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices

## Slice 16 — the side config (2026-11-24)

### The slice, quoted from this port's own gaps

`## Deviations and gaps (slice 1)`, item 6, the half slice 13 did not take:

> **6. No side config or redstone modes.** Upstream's six per-side transfer types
> and redstone IGNORE/ON/OFF default to `ALL`/`IGNORE`; this slice ships only
> those defaults.

and the sentence this port's own `## Next slice` carried forward:

> The side config stays open: a `ModEnergyBuffer` declares six face booleans, not six per-side transfer types.

Slice 13 closed the redstone half and slices 14–15 the comparator family; this closes the last
half of gap 6. **Why it fits `machines`:** six saved face values plus one gate in the per-face
transfer path the port already has, the same shape as `redstone_mode` — no new block, item,
renderer, hook family or content file.

### The artefact (upstream tag `v6.2.10`, commit `a5a8f25`)

Read from `ports/_shared/upstream/powah/src/main/java/owmii/powah/`:

- `lib/logistics/Transfer.java`: `ALL(true,true)`, `EXTRACT(true,false)`, `RECEIVE(false,true)`,
  `NONE(false,false)`, ordinals 0…3. `next(ALL)` cycles 0→1→2→3→0; `next(EXTRACT)` and
  `next(RECEIVE)` toggle that type with `NONE`.
- `lib/logistics/energy/SideConfig.java`: six `Transfer`s, saved under **`side_transfer_type`**
  as an int array of ordinals (in `Direction.get3DDataValue()` order — the SDK's `Direction`
  `Down=0, Up=1, North=2, South=3, West=4, East=5` is the same order). `init()` fills all six
  faces from `storage.getTransferType()`; `setType` refuses a side where `!isEnergyPresent(side)`.
- `lib/block/AbstractEnergyStorage.java:143-149`: `canExtractEnergy(side)` /
  `canReceiveEnergy(side)` are `isEnergyPresent(side) && sideConfig.getType(side).canExtract/canReceive`.
- **The six defaults are not all the same** (the brief's explicit warning): `AbstractEnergyBlock`
  and `AbstractEnergyStorage`'s `getTransferType()` is `ALL`; `AbstractEnergyProvider` (furnator),
  `AbstractGeneratorBlock` (solar) and `EnergyDischargerTile` return `EXTRACT` — give only.
  `isEnergyPresent` is `DOWN` only for a solar panel, and the block's `facing` only for a rod and
  an ender gate (already carried in `solar_battery`, `rod_battery` and the gate declaration).
  The energizing orb owns its own energy field (no `SideConfig`) and a cable stores nothing.

### The six defaults, as read from the jar/source (not assumed)

| Machine | Default on all six faces | Faces that exist (`isEnergyPresent`) |
|---|---|---|
| `energy_cell_*` | `all` (give and take) | all six |
| `energy_hopper_*`, `player_transmitter_*`, `ender_cell_*`, `energizing_rod_*` | `all` | all six (rod: `facing` only) |
| `furnator_*` | `extract` (give only) | all six |
| `solar_panel_*` | `extract` | `down` only |
| `energy_discharger_*` | `extract` | all six |
| `energizing_orb`, `energy_cable_*` | no side config | — |

### What it ships

| File | Change |
|---|---|
| `src/runtime.rs` | `TransferType` (four ordinals, `can_extract`/`can_receive`, `next(default)`, `named`, `default_sides`) + 3 tests |
| `src/powah.rs` | `KEY_SIDE_TRANSFER`, `has_side_config`, `default_transfer`, `side_types`, `face_allows`/`can_extract_at`/`can_receive_at` wired into `push_out_on` and the cable's `neighbour`, `side_text`/`side_line`, `SideOutcome`/`set_side_type`, `word`, 2 new usages, 4 tests (incl. the operator-only count 7→9) |
| `src/lib.rs` | `powah side <pos>` (opt-in readout) and `powah side <face:word> <type:word> <pos>` |
| `tools/e2e.sh` | `side`/`side_field`/`side_reply` helpers and the slice-16 phase with `SELFTEST=side` |
| `horizon-mod.toml`, `Cargo.toml`, `README.md` | version 0.2.16 and the slice's paragraph |

No content file changed: the config is data and one rule, not blocks or items. **No existing
`/powah info` line changed** — the six faces are a separate opt-in readout, so all earlier legs
stay byte-identical.

### TDD: the red proof, quoted

`TransferType`'s methods and `default_sides` were declared `todo!()` and three tests written
against them; `cargo test` printed (`ports/_shared/evidence/powah/slice16-test-red.log`):

```text
---- runtime::tests::the_four_transfer_types_are_upstreams_ordinals_flags_and_names stdout ----
thread 'runtime::tests::the_four_transfer_types_are_upstreams_ordinals_flags_and_names' panicked at src/runtime.rs:1058:9:
not yet implemented: slice 16: a transfer type's saved ordinal

---- runtime::tests::a_face_cycles_its_type_in_upstreams_order_for_each_default stdout ----
thread 'runtime::tests::a_face_cycles_its_type_in_upstreams_order_for_each_default' (149562) panicked at src/runtime.rs:1090:9:
not yet implemented: slice 16: a face's next transfer type

---- runtime::tests::a_fresh_machines_six_faces_all_start_at_its_own_default_type stdout ----
thread 'runtime::tests::a_fresh_machines_six_faces_all_start_at_its_own_default_type' (149563) panicked at src/runtime.rs:1098:5:
not yet implemented: slice 16: a fresh machine's six face types

test result: FAILED. 101 passed; 3 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s
```

After the implementation the same command printed **107 passed, 0 failed**
(`sdk-test.log`, regenerated for 0.2.16).

### The rule in the port

`saved side_transfer_type` is upstream's int array; a machine with no valid array uses
`default_sides(default_transfer(kind))`. `face_allows` reads it for one face and answers
`Transfer.can_extract`/`can_receive`; it is called for the giver's face in `push_out_on`, for the
taker's opposite face there, and through `neighbour`'s `can_insert`/`can_extract` in the cable
planner. A block entity that cannot be read, and a block with no side config, never blocks a move.

**The pin cannot express the config in the declaration.** `ModEnergyBuffer` has
`insert_faces`/`extract_faces` only — six face booleans per direction, shared by every instance of
the block — so the four-type config is carried as six saved ordinals and enforced by the port's own
movers. Exactly as slice 13 recorded for redstone, a *foreign* mover still sees the declared
buffer; this port's own push and cable relay ask the config first.

### Ladder (0.2.16, pin `0a875da`)

Every compile step ran in `ports/powah`, `CARGO_INCREMENTAL=0`, through
`ports/tools/build-slot.sh run --job W1-powah --port ports/powah --step …` (compile steps
forwarded to `AdLf-PC`); logs in `ports/_shared/evidence/powah/`.

| Check | Result |
|---|---|
| `cargo test --offline` | **107 passed, 0 failed** (`sdk-test.log`, regenerated for 0.2.16) |
| `cargo clippy --all-targets --offline -- -D warnings` | `CLIPPY_EXIT:0`, 0 warnings (`slice16-clippy.log`) |
| `cargo fmt --check` | `FMT_EXIT:0`, clean (`slice16-fmt.log`) |
| two builds, `rm -f Cargo.lock` before each | `BUILD1_EXIT:0`, `BUILD2_EXIT:0`, **`cmp`-identical**, 545,626 B (`slice16-build{,-second}.log`) |
| `cargo horizon-mod check powah.hmod` | `powah 0.2.16 (native-mod, format native-1/0a875da…/e59da90…, for 26.3)` · `545626 bytes, SHA-256 41c3456e…`, `CHECK_EXIT:0` |
| `cargo horizon-mod fits powah.hmod --build 0a875da` | `Powah:` / `0a875da  made for`, `FITS_PIN_EXIT:0` |
| `cargo horizon-mod fits powah.hmod --build 0b14f8a` | `Powah:` / `0b14f8a  fits`, `FITS_OTHER_EXIT:0` |
| `python3 ports/tools/verify-sdk-package.py powah` | **1/1**, `package matches the tree`, `no packed Cargo.lock  860 entries`, `SDK-only dependencies (R9)`, `tests 107 passed`, `skills gate PASS` |

The delivered `powah.hmod` is **545,626 B**, sha256
`41c3456e743ad4cf85d68812a086532f1afc0429cb0c1033f1bbcc415f90a225`, **860 entries**, no packed
`Cargo.lock`. Both fits lines are real: the pin prints `made for` (a tautology), the other pinned
build prints `fits`. No local type or variant named `Block`/`Item` was introduced, so `fits` had no
false failure.

### In-game (parity rule)

Launcher build `native-0a875da-29829d6c3057`, instance `powah-e2e`, `Compiling powah (1 of 1)`
(`launcher-build-0.2.16.log`); the single stale tag carrying this mod
(`native-0a875da-8d7947c713ed`, 0.2.15) was removed first so the launcher recompiled.

**The full 62-log replay ran once** and reached slice 16 with every earlier phase green; it then
`FAIL`ed at this slice's *own* readout parse, `a fresh cell's up face is not all`, because the
harness's `side_field` helper required a space before the face word while the readout separates
faces with commas (`…down=all,up=all,…`). The readout itself showed `up=all`. That run is kept as
`discarded-side-field-harness-bug-0.2.16.log` (outside the `e2e*.log` strict set) and is **not
evidence**. The helper was fixed to accept `[ ,]` as the separator, and the three verdict legs
then ran as a **bounded side-config scenario** (`tools/e2e-side.sh`) rather than three more
~20-minute full replays: it launches the same game and asserts exactly the slice-16 family. The
earlier families' evidence is the unchanged 0.2.15 replay; this slice changed only Rust logic in
the side gate and the two new commands, and the full replay proved those earlier phases green on
0.2.16 before the harness bug.

| Leg | Command | Result |
|---|---|---|
| main | `build-slot.sh run --step e2e -- bash tools/e2e-side.sh` | **`PASS` + `EXIT:0`** (`e2e-side-0.2.16.log`) |
| control (same Nightly, no package) | `LAUNCH_GAME=$CONTROL_GAME …` | `FAIL: expected powah:energy_cell_starter at -4,100,60, got 'minecraft:air'` + `EXIT:1` (`e2e-side-control-0.2.16.log`) |
| `SELFTEST=side` | `SELFTEST=side …` | `FAIL: SELFTEST flipped: a none face kept its sink at 0 FE, as it should` + `EXIT:1` (`e2e-selftest-side-0.2.16.log`) |
| `SELFTEST=1` (slice-1 flip) | `SELFTEST=1 …` | `FAIL: SELFTEST flipped: the unfuelled furnator made no FE, as it should` + `EXIT:1` (`e2e-selftest-1-0.2.16.log`) |
| log gate | `python3 ports/tools/check-e2e-logs.py powah-e2e --strict` | `logs=66 ok=66 missing-exit=0 bad-exit=0`, exit 0 |

Quoted from `e2e-side-0.2.16.log` (the per-face readouts are parsed from the reply, never "no
error"):

```text
  fresh cell: Machine at -4,100,60 (side down=all,up=all,north=all,south=all,west=all,east=all, t=77)
  fresh furnator: Machine at 6,100,60 (side down=extract,up=extract,north=extract,south=extract,west=extract,east=extract, t=83)
  side(-4,100,59) reply: That block has no side config.
  side(-4,100,60) reply: Side north set to none.
  after north=none: Machine at -4,100,60 (side down=all,up=all,north=none,south=all,west=all,east=all, t=111)
  set(-4,100,58) reply: Energy set to 200000 FE.
  blocked over 20 ticks: cell 0 FE, sink 0 FE
  south face: 994000 FE at t=181 -> 986000 FE at t=197, lost 8000 over dt=16 (want 8000)
  frozen: cell 983500 + sink 16500 = 1000000, source 200000
  after north=all: Machine at -4,100,60 (side down=all,up=all,north=all,south=all,west=all,east=all, t=241)
  after the flip: cell 8500 FE
```

The `500 × dt` leg lost **8,000 FE over dt=16** exactly; the broken-cable freeze shows the cell's
983,500 plus the sink's 16,500 equals the 1,000,000 set, with the source behind the blocked face
untouched at 200,000; and opening the face again charged 8,500 FE.

### Deviations this slice records

1. **The GUI wrench button is a command.** Upstream cycles a face with
   `NextEnergyConfigPacket` from a screen; this port has no GUI (slice 1's gap 1), so
   `/powah side <face> <type> <pos>` sets one face and `/powah side <pos>` reads all six. Both are
   operator-only (level 2), like the other harness controls, because they act at chosen coordinates.
2. **A foreign mover still sees a declared face.** The pin's `block_energy` declaration is per
   block, not per instance, so another mod's mover can still pull through a face this port set to
   `none`. Expressed with the closest supported API, recorded rather than faked (slice 13's
   deviation 2).
3. **The config lives in the sync compound.** Upstream writes `side_transfer_type` in
   `writeSync`, not `writeStorable`, so a broken machine's item carries its buffer but never its
   face types — faithful to the mod. A picked-up/placed machine starts at its default types again.

**Upstream defects fixed: none.** The four types, the cycle and the per-machine defaults are the
artefact's own; no defect was found while measuring them.

### Coverage (parity rule, the family this slice adds)

| Family (from the jar) | Status |
|---|---|
| four transfer types and the ordinals/flags/cycle | **tested** — unit tests pin 0/1/2/3, the two flags and both cycle rules; in game all six faces are parsed from `/powah side`'s reply |
| the six defaults (storage `all`, furnator/solar/discharger `extract`) | **tested** — unit test per machine kind; in game a fresh cell reads `all` on six faces and a fresh furnator `extract` |
| `side_transfer_type` saved compound | **tested** — unit test of the int array and a fresh/malformed fallback; the in-game phase writes `north=none` and reads it back, then flips it to `all` |
| the per-face gate on a cell's receive face | **tested** — a charged source on the `none` face leaves the cell and its sink at 0 FE over 20 ticks; opening the face starts the charge |
| the per-face gate on the give face | **tested** — the cell's south face delivers exactly `500 × dt` out of its own buffer while north is blocked; broken-cable conservation shows no FE created or lost |
| the GUI's wrench button | `not ported` — slice 1's GUI gap; the two `/powah side` commands are its replacement |
| a foreign mover's view of a `none` face | `blocked(pin: block_energy has no per-instance hook)` — the declaration is per block (deviation 2) |

### Harness findings this slice measured (for the next port)

1. The side readout carries its own `t=` clock so the harness's fresh-reply guard works; without
   it a stale chat line from an earlier `/powah side` would be accepted as the new state.
2. `/powah set` on a cell that is actively pushing cannot be polled for exact equality (slice 4's
   finding); use the command's reply and take the next readout's own value as the starting point.
3. **A readout parser must accept the separators the readout actually uses.** The first `side_field`
   matched only `" <face>="`, so `down=all` parsed while `,up=all` and `,north=all` did not — a
   harness false failure whose message showed the correct value in the very line it rejected. The
   regex is now `[ ,]<face>=`, and the false failure is kept in a discarded (non-`e2e*.log`) file.

Version: **0.2.15 → 0.2.16** (one patch). The published `[[versions]]` record keeps 0.2.15 and
gains 0.2.16 at publish time — this port does not touch the registry entry. Title:
`Update powah 0.2.16` (the entry is ours on `main`).

Skills: superpowers/using-superpowers, brainstorming, writing-plans, test-driven-development, systematic-debugging, verification-before-completion; BeautifulRust/rust-best-practices

## Next slice

**Family:** the creative energy cell. `## Slice 4 — the energy cell ladder`'s coverage row is the
gap line, verbatim:

> `energy_cell_creative` | **not ported** — a source, not a storage tier (gap 11)

and `## Deviations and gaps (slice 1)`, item 11 names it: *"The other 6 tiers, the creative cell
and ~15 machines are unported."* Slice 16 closed the last half of gap 6 and slices 13–15 the
redstone/comparator families, so this is the next machine in the port's own list. **Why it fits
`machines`:** one storage block with the cell's declared `block_energy` buffer and idle tick, a
source whose `getTransferType()` is `EXTRACT` (upstream `EnergyCellTile.isCreative()`), carried
content from the same jar, no renderer, GUI, entity or new hook. **In-game assertion:** place
`powah:energy_cell_creative`, put an empty starter cell behind a cable, read the source's
`/powah info` (its `FE/t a side` names the creative tier) at t1 and the sink at t2, and assert the
sink gains exactly **500 × dt** while the source's readout still shows its full 2,147,483,647 FE
and never drops — a `SELFTEST=creative` leg must invert the "never drops" half and end `EXIT:1`.
The generators (Magmator/Thermo) remain `blocked(mod-fluid-tank)`, the reactors
`blocked(uraninite-worldgen)` and the GUIs out of the machines tier.

