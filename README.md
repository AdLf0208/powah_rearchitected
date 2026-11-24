# Powah for Horizon — slices 1–16

A clean-room Rust rewrite of **Powah! (Rearchitected) 6.2.10** (`Technici4n/Powah`,
**LGPL-3.0-only**) for Horizon 26.3, as one flat SDK package (`api = "0.2"`): the
carried content (`assets/`, `data/`) and the behaviour (`src/`) ship in the same
`.hmod`. Slice 1 is the starter tier of the energy chain:

| Block | What it does |
|---|---|
| `powah:furnator_starter` | burns furnace fuel into a carbon buffer and makes 20 FE/t into a 20,000 FE buffer (48,000 FE per coal) |
| `powah:energy_cell_starter` | a 1,000,000 FE capacitor, 1,000 FE/t per side |
| `powah:energy_cable_starter` | moves FE between the block entities beside it, 500 FE/t, storing nothing |

Slice 2 is the **dielectric/capacitor crafting family** — `powah:dielectric_paste`,
`dielectric_rod`, `dielectric_rod_horizontal`, `dielectric_casing`, `capacitor_basic`
and `capacitor_basic_tiny`, with their eight recipes — which is what the three
machine recipes above need, so all three now craft.

Slice 3 is the **cable network**: every cable connected to a cable through cables is
one relay (upstream's `CableNet`), so a charge crosses a run of three cables into a
cell at the far end (0.2.2 moved nothing across more than one hop). The network
carries 500 FE/t — one cable's rate for the whole run — and creates or loses no FE
across it.

Slice 4 is the **energy cell ladder**: the basic, hardened, blazing, niotic, spirited
and nitro cells, each with its own capacity (4M/10M/40M/100M/400M/2G FE) and per-side
transfer (4,000 up to 2,000,000 FE/t), so `/powah set` clamps at the tier's own number.

Slice 5 is the **Energizing Orb**, its **seven rods** and the **materials** they make:
an orb's seven slots (0 result, 1–6 inputs) match one of six energizing recipes — iron
and gold into energized steel at 10,000 FE, four blaze powder (or one blaze rod) into a
blazing crystal at 120,000, a diamond at 300,000, an emerald at 1,000,000 and the
nether-star recipe at 20,000,000 into sixteen nitro crystals — and a rod within four
blocks fills the orb's buffer at up to its tier's transfer (100 up to 200,000 FE/t),
losing exactly what the orb took. That is what makes the five higher cell tiers, the
large capacitor and the higher capacitor and rod tiers craftable.

Slice 6 completes the two **furnator and cable ladders**: the basic, hardened, blazing,
niotic, spirited and nitro furnators (80/200/800/2,000/8,000/40,000 FE/t into their own
80k/200k/800k/2M/8M/40M FE buffers, each giving at its own 320…160,000 FE/t a side) and the
six matching cables (2,000 up to 1,000,000 FE/t). A cable network joins only its own
tier's cables, which is upstream's `CableNet` rule.

Slice 7 is the **item-energy family**: a machine's own item carries its buffer when the
machine is broken (upstream's `keepEnergy`, true for every furnator, cell and rod), as
`minecraft:block_entity_data` `{id, energy}` — the engine spills a container's stacks
itself, so the payload never carries `Items` — and placing the item restores the FE
clamped to the tier (an item doctored to 2,000,000,000 FE places a full 20,000 FE
furnator). A furnator's charge slot (upstream's slot 0; the fuel moved to slot 1) and a
cell's two slots take `min(transfer, stored, room)` out of the machine's own buffer each
tick, and a machine's item answers `Capabilities.Energy.ITEM` from that same payload.

Slice 8 is the **Energy Discharger**, the cell's mirror: seven tiers (1,000,000 FE /
1,000 FE a side up to 2,000,000,000 / 2,000,000) whose buffer takes nothing through any
face (`Transfer.EXTRACT`) and fills from the energy items in its seven slots — each step
`min(the item's charge, the tier's transfer, the buffer's room)`, read through the item's
own `Capabilities.Energy.ITEM` handler, so a cell item placed there gives up its charge
and a stone gives nothing. It keeps its buffer on its item like the other machines.

Slice 9 is the **Energy Hopper**, the discharger's charging mirror: seven tiers
(1,000,000 FE / 500 FE a tick charging up to 2,000,000,000 / 1,000,000) with **no slots
of its own** — its tick reads the container on the face its `facing` names and spends
`min(the tier's charging rate, the buffer's balance, the item's own room)` on each
stack through that item's `Capabilities.Energy.ITEM` handler, then debits its own
buffer by exactly what went out. It never pushes through a face (`postTick` has no
`extractFromSides`), it takes and gives its tier's transfer on every face, a stone
takes nothing, and it keeps its buffer on its item like the other machines.

Slice 10 is the **Solar Panel**: seven tiers (20 FE a tick into a 20,000 FE buffer up
to 2,000 into 40,000,000) whose only fuel is the sky. Each tick the panel reads
`can_see_sky` at the block above it (upstream's `Misc.canBlockSeeSky(world,
pos.above())`), its dimension's sky light and `get_sky_darken()`, and makes
`min(generation, room)` only when it sees the sky and it is day — so a stone overhead
or midnight stops it, exactly as upstream. It charges one item in its slot, gives FE on
its bottom face only (`isEnergyPresent` is `Direction.DOWN`), takes no fill through any
face, and keeps its buffer on its item. The **photoelectric pane** the starter panel is
built from comes with it, so all eight recipes craft.

Slice 11 is the **Ender Cell** and the **Ender Gate**, seven tiers each: two block
families whose buffer is not their own. Upstream's `EnderNetwork` holds one shared
channel per owner, and a block's `ENERGY` capability is that channel — so two cells on
one channel hold, give and show the same FE from anywhere in the level, and the charge
outlives breaking either block and relaunching the world (the item carries nothing:
`keepEnergy()` is false). A **cell** answers all six faces and takes an **energy cell
item** into its extender slot, which adds that cell's capacity and its own charge to the
channel; a **gate** answers and pushes only on the face its `facing` names, moves at the
cable's rate (`cableTransfer`) and cannot extend the network. The starter tier offers one
channel, the nitro twelve (`TieredChannelValues`), and `/powah channel` is the GUI
packet's replacement.

Slice 12 is the **Player Transmitter**, seven tiers (a 1,000,000 FE buffer up to
2,000,000,000, giving and taking 1,000…2,000,000 FE a side), and the **Binding Card** it
takes: the card names the player it is bound to (`powah:bound_player`, bound by using it),
and the transmitter spends its tier's **charging rate** (500…1,000,000 FE a tick) on that
player's inventory — every slot through its own energy handler, so an energy cell the
player carries takes the charge. `keepEnergy()` is true, so the buffer rides the block
item. The card is crafted from a blank card, and the starter transmitter's `p` is the
**player aerial pearl** an aerial pearl makes of a zombie, so the whole chain is
obtainable in survival.

Slice 13 is the **redstone mode** upstream's GUI button cycles: `IGNORE` (the default, always
running), `ON` (only while a neighbour powers the block) and `OFF` (only while none does), saved as
`redstone_mode` in the block entity's own data and asked once per tick through the level's own
neighbour-signal read. A machine whose gate is shut stops its own work — a furnator's burn, a solar
panel's generation, a discharger's drain, a hopper's charge, a transmitter's charge — and a cell or
an ender block also stops answering energy on its faces, as upstream's `canExtractEnergy` and
`canReceiveEnergy` do. `/powah redstone <ignore|on|off> <pos>` is the GUI button's replacement.

Slice 14 is the **comparator reading**: a vanilla comparator beside a buffered machine now reads the
fraction of its buffer (upstream's `Energy.toComparatorPower()`, the number `/powah info` prints), not
container fullness, and the reading follows the buffer without the block's state changing — a changed
strength tells the comparators beside the machine to read it again. It is declared for the machines
whose readout prints a comparator (every furnator, cell, discharger, hopper, solar panel, ender cell,
ender gate and player transmitter) only: a cable stores nothing, and the orb and the rods print no
comparator, so they answer none rather than a wrong number.

Slice 15 finishes that family: the orb answers the **number of its non-empty stacks** (capped at
15) and a rod the fraction of its own buffer (a half-full starter rod reads 7, a full one 15),
while a cable with a full container behind it still reads 0.

Slice 16 is the **side config**: every machine with a `SideConfig` saves six per-face transfer
types — `all`, `extract` (give only), `receive` (take only) and `none` — and the port's own
per-face movers honour them. The defaults are upstream's own: a storage is `all` on six faces,
while a furnator, a solar panel and a discharger default to `extract`. The pin's `ModEnergyBuffer`
can only declare face booleans, so the four-type config is carried as six saved ordinals under
`side_transfer_type`. `/powah side <pos>` reads all six faces (with a fresh `t=` clock, so no
existing `/powah info` line changed) and `/powah side <face> <type> <pos>` replaces the wrench
button: a face set to `none` takes nothing, and opening it again starts the transfer.

Slice 17 adds the **creative energy cell** (`powah:energy_cell_creative`). Upstream's
`EnergyCellTile.isCreative()` makes it a source: its first tick fills it, its transfer type is
`EXTRACT` (give only), and its `extractEnergy` is simulated so giving FE never drops the buffer.
Upstream's `Energy.MAX` (9,000,000,000,000,000,000) is clamped into the SDK's `i32` energy buffer,
so the source holds and offers 2,147,483,647 FE. It has the cell's two charge slots, an idle tick
that keeps it full, and a `/powah info` line that keeps the existing cell formatting byte-for-byte
with a trailing `creative` field; a cable carries 500 FE/t out of it into an empty cell while the
source's own readout never drops. A stone in its place transfers nothing.

Every machine declares a real `Capabilities.Energy.BLOCK` buffer (the amount is the int
`energy` in the block entity's saved compound), so other machines can fill and drain a
furnator, a cell, a discharger, a hopper, a solar panel or a transmitter; an ender cell or
gate answers that capability with its owner's channel, and a cable is a pure relay that
moves FE from the fuller store to the emptier one (a network never takes back what it gave).

- Scope, behaviour table, the hook table, deviations and the gap list:
  [`notes.md`](notes.md); upstream analysis: [`FINDINGS.md`](FINDINGS.md);
  content carry and the patches: [`CONTENT.md`](CONTENT.md).
- Verification: [`VALIDATION.md`](VALIDATION.md).
- Licence: **LGPL-3.0-only** (rank 1, copyleft — publishable under the same
  licence with its complete source; local-only until the maintainer approves
  publishing, ARCHITECTURE.md §8).

## Layout

```text
powah.hmod            the delivered package (horizon-mod.toml + Cargo.toml + src/ + assets/ + data/)
horizon-mod.toml      the only manifest and identity: id, version 0.2.17, api 0.2, [native] pins
Cargo.toml            one crate; its only dependency is horizon-mod-api from .horizon/sdk/
src/lib.rs            the one `#[mod_api::horizon_mod] impl Hooks`
src/powah.rs          the glue: the block entity Parts, the queue, the tier tables, the buffers, the cable network, the command
src/runtime.rs        the pure machine rules and their tests (no mod_api types)
assets/powah/         the carried blockstates, models, items, textures and lang
data/powah/           the carried blocks, items, loot, recipes, tags, the creative tab
tools/craft_check.py  the recipe leg: every material, machine, discharger, hopper, solar, ender and transmitter recipe in a real table
tools/e2e.sh          the in-game scenario (needs a launcher-built game, see below)
```

## Build and check

The SDK is fetched into `.horizon/sdk/` (gitignored) by the first
`cargo horizon-mod` command; a port that is offline can copy it from another
migrated port at the same pin.

```sh
cd ports/powah
CARGO_INCREMENTAL=0 cargo test                        # the pure rules and the glue's declarations
CARGO_INCREMENTAL=0 cargo clippy --all-targets -- -D warnings
cargo fmt --check
rm -f Cargo.lock                                      # a packed lock changes the artifact
CARGO_INCREMENTAL=0 cargo horizon-mod build           # -> target/horizon-mod/powah.hmod
cargo horizon-mod check powah.hmod                    # the registry's own check
cargo horizon-mod fits powah.hmod --build nightly     # does it still fit that build
```

## In-game observability

Slice 1 has no GUI, so the port adds these commands (recorded as harness additions,
not upstream gameplay). `/powah channel` is slice 11's replacement for the ender
block's GUI channel buttons (`SetChannelPacket`):

```
/powah info <x> <y> <z>            # FE, carbon, burn state, capacity, comparator, an ender block's channel
/powah set <amount> <x> <y> <z>    # set a furnator's or a cell's stored FE (an ender block's channel)
/powah item <amount> <x> <y> <z>   # set the charge of the item in a machine's first charge slot
/powah channel <n> <x> <y> <z>     # move an ender cell or gate to channel n (clamped to its tier)
/powah redstone <ignore|on|off> <x> <y> <z>  # the GUI redstone button (slice 13)
/powah side <x> <y> <z>            # the six face types, with their own t= clock (slice 16)
/powah side <face> <type> <x> <y> <z>  # set one face to all|extract|receive|none (the wrench button)
```

## Running the in-game check

A native package is compiled **into** the client by the launcher, so the check
needs a game built with this package:

```sh
horizon-launcher native-build ports/powah --instance <instance> --trust-native-code
GAME=<that instance's horizon-client> tools/e2e.sh
GAME=<that game> CONTROL_GAME=<an unmodified build> tools/e2e.sh   # the negative leg
SELFTEST=1 GAME=<that game> tools/e2e.sh                           # the fail-capable leg
SELFTEST=craft GAME=<that game> tools/e2e.sh                       # the recipe leg's fail-capable run
SELFTEST=network GAME=<that game> tools/e2e.sh                     # the cable network's fail-capable run
SELFTEST=orb GAME=<that game> tools/e2e.sh                         # the rod's exact fall
SELFTEST=furnator GAME=<that game> tools/e2e.sh                    # the furnator tier clamp
SELFTEST=furnator-rate GAME=<that game> tools/e2e.sh               # the nitro furnator's exact tick
SELFTEST=drain GAME=<that game> tools/e2e.sh                       # the discharger's drain
SELFTEST=hopper GAME=<that game> tools/e2e.sh                      # the hopper's charge rate
SELFTEST=solar GAME=<that game> tools/e2e.sh                       # the panel's sky reads
SELFTEST=ender GAME=<that game> tools/e2e.sh                       # the shared channel (a peer left behind)
SELFTEST=redstone GAME=<that game> tools/e2e.sh                    # the shut gate charged nothing
SELFTEST=comparator GAME=<that game> tools/e2e.sh                  # the comparator's reading
SELFTEST=orb-comparator GAME=<that game> tools/e2e.sh              # the orb's stack count
SELFTEST=side GAME=<that game> tools/e2e.sh                        # the none face stayed at 0 FE
```

Give the port **its own launcher instance** (e.g. `--instance powah-e2e`, and the
same in `HORIZON_E2E_INSTANCE`): an instance that already holds an older install
of the port answers *"was made by an older tool and doesn't say what it uses"* and
leaves the mod out of the build (measured 2026-11-21, `notes.md` §Pin findings 2).
