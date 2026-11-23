#!/usr/bin/env bash
# In-game check for the Powah port: the starter Furnator, Energy Cell and Energy
# Cable (slice 1) and the dielectric/capacitor crafting family with the recipes
# they unblock (slice 2), on a real game, for the flat SDK package (`powah.hmod`).
#
# The package carries the three blocks, their items, the data AND the Rust that
# runs them, so the game must be built against it by the launcher:
#
#   horizon-launcher native-build . --instance <instance> --trust-native-code
#
# and `GAME` must name that build's client. There is no `--dev-mod` leg: the
# package is compiled in, and a content half loaded beside it would be a second
# copy of the same blocks.
#
# `CONTROL_GAME=<an unmodified client of the same build>` is the negative control:
# the block id must be unknown there, the item must not arrive and the block must
# stay air, so the machines demonstrably come from this package. With it unset the
# leg is skipped and the log says so.
#
# NOT RE-RUN IN THE SDK MIGRATION: `horizon-launcher` was not installed then, so no
# 0b14f8a client carrying this package existed. It is installed now and 0.2.1 ran
# this leg (`ports/_shared/evidence/powah-e2e/`), so every phase below is live.
#
# The arena is a stone floor at y=100 in the Overworld, so every block read and
# command is in the player's own dimension (the pin's `ctl tp` is reliable
# there).
#
# Default run:
#  - control "no fuel": a placed furnator makes no FE for 40 ticks, and neither
#    does the cell beside it (nothing to move);
#  - coal in the furnator's fuel slot (the container path, automation's way in)
#    starts the burn, the stack shrinks by one and carbon is 1600*30;
#  - the furnator makes FE and the cable beside it carries that FE into the cell;
#  - control "no cable": with the cable broken and the cell emptied, the cell
#    stays empty for 40 ticks while the furnator holds FE; placing the cable
#    again lets the cell charge — so the cable is what moves the energy;
#  - control "full cell": a cell set to its capacity stays at 1,000,000 FE and
#    the furnator keeps burning (energy is not lost or duplicated);
#  - a charged cell discharges through a cable into an empty cell, so the cell
#    is a source as well as a sink;
#  - the cable reports no storage and its 500 FE/t rate.
# Slice 2 (the parity rule) adds:
#  - the six dielectric/capacitor items are given and read back, and an unknown
#    id is refused;
#  - the eight material recipes AND the three machine recipes craft in a real
#    crafting table — declared output id and count, every ingredient consumed,
#    the lava bucket's bucket left in the grid, and an off-pattern grid empty
#    (`tools/craft_check.py`);
#  - each machine's loot table drops the machine when it is mined in survival.
#
# SELFTEST=1 flips the "no fuel" assertion (it must FAIL), proving the scenario
# can fail. SELFTEST=craft expects the paste's 24 to be 25 inside
# `tools/craft_check.py` (it must FAIL there), proving the recipe assertions can
# fail. SELFTEST=network expects the three-cable run to have delivered nothing
# (it must FAIL), proving the network assertion can fail.
#
# Slice 6 (the other six furnator and cable tiers) adds:
#  - all seven furnator tiers placed and set to 2,000,000,000 FE: each clamps at
#    its own generation capacity (20k/80k/200k/800k/2M/8M/40M) and names its own
#    per-side transfer;
#  - a nitro furnator whose buffer is left exactly 40,000 of room burns one coal
#    in one tick: 40,000,000 FE and 8,000 carbon left (a starter-rate tick would
#    leave 47,980), and a starter furnator fed the same coal keeps
#    energy + carbon = 48,000 while it fills slowly;
#  - all seven cable tiers read back their own rate (500/2,000/5,000/20,000/
#    50,000/200,000/1,000,000 FE/t);
#  - a basic cable network does not join a starter cable: a cell behind the
#    starter cable stays empty until that cable is replaced by a basic one.
# SELFTEST=furnator expects the nitro furnator to clamp at the starter's 20,000
# (it must FAIL) and SELFTEST=furnator-rate expects its carbon to be the starter
# rate's 47,980 (it must FAIL), proving each exact assertion can fail.
#
# Slice 7 (gaps 4 and 5 — "no item charging", "breaking a machine loses its FE")
# adds the item-energy family:
#  - the furnator's charge slot (upstream's slot 0; the fuel moved to slot 1)
#    charges a chargeable item exactly `min(transfer, stored, room)` a tick: 40 FE
#    from a 40 FE balance (and the machine stands at 0), 80 FE a tick measured
#    against the game's clock, and an item with 80 of room takes one step and
#    leaves the machine's other 920;
#  - a cell charges two item slots out of one 1,000 FE budget: 1,000 into the
#    first and the remaining 500 into the second;
#  - a stone in the charge slot takes nothing and reads `charge minecraft:stone
#    0/0`;
#  - a 5,000 FE furnator broken in survival and placed again reads 5,000/20,000,
#    and its dropped item carries `minecraft:block_entity_data`; the stack in its
#    charge slot spills into the world and does not ride the payload; an item
#    doctored to 2,000,000,000 FE clamps to the tier's 20,000 on placement; a
#    fresh item places an empty machine; a cell keeps its own 4,000; and what a
#    level-side break carries is recorded.
# SELFTEST=charge expects the charge rate to be 40 FE a tick (it must FAIL) and
# SELFTEST=keep expects the placed machine to read 0 FE (it must FAIL), proving
# each exact assertion can fail.
#
# Slice 8 (the discharger, the item-energy family's other half) adds:
#  - a starter discharger (1,000,000 FE, 1,000 FE/t a side) with a cell item
#    charged to 5,000 in slot 0: the buffer reads 5,000 and the item 0;
#  - with 20 of room the item must lose exactly 20, not the 1,000 transfer, and a
#    full buffer must stop the drain;
#  - the drain rate measured against the game's clock: the buffer gains exactly
#    1,000 FE a tick and the item loses the same;
#  - a stone in the slot gives nothing, a nitro discharger clamps at
#    2,000,000,000 FE and names its 2,000,000 FE a side, the block is in
#    `powah:energy_dischargers`, a broken 7,000 FE discharger's item carries the
#    buffer back, and a charged cell beside a discharger keeps its 50,000 (no side
#    of a discharger takes FE in: `Transfer.EXTRACT`);
#  - and every discharger tier mined in survival drops itself.
# SELFTEST=drain expects the item to keep its 5,000 (it must FAIL), proving the
# drain assertion can fail.
#
# Slice 9 (the energy hopper, the charging half of that family) adds:
#  - a starter hopper (1,000,000 FE, 1,000 FE/t a side, 500 FE/t charging) facing a
#    chest whose slot 0 holds a `powah:energy_cell_starter` at 0, set to 5,000: the
#    buffer falls exactly 500 FE a tick measured against the game's clock, the item
#    ends holding the whole difference, and the two halves add up to the 5,000;
#  - a second hopper facing away from its chest (`facing=north`) charges nothing and
#    spends nothing, so the face is what selects the container;
#  - a stone in the chest takes nothing;
#  - a nitro hopper clamps at 2,000,000,000 FE and names `charging 1000000 FE/t,
#    2000000 FE/t a side`, the block is in `powah:energy_hoppers`, and a broken
#    7,000 FE hopper carries its buffer back;
#  - and every hopper tier mined in survival drops itself.
# SELFTEST=hopper expects the chest's item to read 0 (it must FAIL), proving the
# charging assertion can fail.
#
# Slice 10 (the solar panel ladder) adds:
#  - a starter solar panel under an open noon sky in the Overworld (the column above
#    14,101,6 is cleared to y=250): `/powah set 10000`, then two readouts `dt` ticks
#    apart must differ by exactly 20 x dt — the tier's `passiveProduction` measured
#    against the game's own clock — and the readout names `sky open, day true,
#    generating 20 FE/t, 80 FE/t a side`;
#  - `/powah set 19980` leaves 20 of room, which one tick fills to exactly 20000 FE
#    and never beyond;
#  - a stone at the sky position (14,101,6) leaves the buffer at exactly 5,000 for 60
#    ticks and reads `sky covered`; `/time set midnight` with the sky open leaves it
#    at 5,000 too and reads `sky open, day false`;
#  - a cell under the panel charges while a cell beside it stays empty and the panel's
#    own buffer takes nothing back (`isEnergyPresent` is DOWN only);
#  - a nitro panel clamps 2,000,000,000 FE at 40,000,000 and names `generating 2000
#    FE/t, 160000 FE/t a side`, the block is in `powah:solar_panels`, the pane crafts,
#    and every panel tier drops itself.
# SELFTEST=solar expects the open-sky panel to have generated nothing (it must FAIL),
# proving the generation assertion can fail.
#
# Slice 5 (the five cell recipes' `blocked(energizing-orb)` gate) adds the
# Energizing Orb, its seven rods and the five materials they make:
#  - a starter rod beside an orb holding iron and gold fills it to exactly its
#    recipe's 10,000 FE and stands at 0, the orb writes 2 energized steel into
#    slot 0 and clears its inputs;
#  - an orb with no rod in range produces nothing, and a rod beside an orb whose
#    inputs match no recipe keeps its charge (the rod only gives to an orb that
#    `containRecipe()`s);
#  - a nitro rod fills four blaze powder to exactly 120,000 FE — it keeps the
#    other 80,000 — then the blaze-rod variant, a diamond (300,000), an emerald
#    (1,000,000) and the nether-star recipe (20,000,000 into 16 nitro crystals);
#  - the orb, every rod and the material block are given/placed/mined, and the
#    crafting table crafts the orb, the seven rods, the six capacitors and the
#    five cell recipes that were blocked before this slice.
# SELFTEST=orb expects the nitro rod to have kept its whole 200,000 (it must
# FAIL), proving the exact-fall assertion can fail.
#
# Slice 3 (gap 2, "a run of two or more cables does not relay") adds:
#  - a source cell, three cables and a sink cell in one row: the charge crosses
#    the run (0.2.2 delivered nothing), the two cells' sum stays 100000, and a
#    cell off the network stays empty.
#
# Slice 4 (gap 11, "the other 6 tiers ... are unported") adds the energy cell
# ladder:
#  - each of the seven tiers is placed and set to 2,000,000,000 FE; the cell
#    clamps it at its own tier's capacity (1M/4M/10M/40M/100M/400M/2G) and its
#    readout names its own per-side transfer;
#  - two adjacent pairs (starter source -> starter sink, basic source ->
#    starter sink) show the transfer in behaviour: the first swaps 1,000 each
#    way and its sink stays at 0..1,000, the second nets 3,000 a tick;
#  - each tier drops itself when mined in survival.
# SELFTEST=cells expects the basic cell to clamp at the starter's 1,000,000
# (it must FAIL there), proving the capacity assertion can fail.
#
# Slice 11 (the ender channel family) adds:
#  - two ender cells on channel 0 (a basic one extended with a 4,000,000 FE energy
#    cell item in slot 0) hold and show the same 100,000 FE, each naming its own
#    tier's transfer and channel count; a third cell on channel 1 reads 0;
#  - moving the extended cell to channel 1 leaves the peer's channel at exactly
#    100,000 and shows 0/0 on the fresh channel, and moving it back finds the
#    channel's FE again (the channel, not the block, holds it);
#  - a cell's charge slot takes exactly 1,000 a tick out of the channel (the
#    item's own per-call limit, not the cell's 4,000) and the channel loses
#    exactly what the item took;
#  - a cell gives 4,000 a tick to the hopper beside it and the shared channel
#    loses exactly what the hopper gains; a starter ender gate feeds only the
#    hopper on its `facing` side, at the cable row's 500 a tick;
#  - the nitro cell and gate name 12 channels and their 2,000,000/1,000,000 FE
#    a tick, `#powah:ender_cells`/`#powah:ender_gates` match in game, every tier
#    drops itself, and every guessed ender id is unknown without the package.
# SELFTEST=ender expects the peer to read 0 FE after the shared cell moved
# channel (it must FAIL), proving the shared-channel assertion can fail.
# Slice 12 (the player transmitter and its binding card) adds:
#  - a starter transmitter (1,000,000 FE, 1,000 FE/t a side, 500 FE/t charging) reads `bound empty`
#    with no card; a card used from the hand binds to the operator (`powah:bound_player`), the
#    readout names them once it is in slot 0, `/powah set 10000` and a cell item in the operator's
#    hotbar make the transmitter spend exactly 500 x dt FE over dt ticks measured against the game's
#    clock, the item gains whole 500 FE steps at one a tick, and the two halves add up to the 10,000
#    within a window derived from the two readouts;
#  - the control is a **blank** card: the same rig charges nothing and spends nothing;
#  - a nitro transmitter clamps 2,000,000,000 FE and names `charging 1000000 FE/t, 2000000 FE/t a
#    side`, `powah:player_transmitters` matches in game, a broken 7,000 FE transmitter carries its
#    buffer back on its own item, and every transmitter tier drops itself;
#  - an aerial pearl used on a zombie (survival) gives the player aerial pearl and kills it — the
#    starter transmitter recipe's ingredient.
# SELFTEST=transmitter expects the item to have gained nothing (it must FAIL), proving the charging
# assertion can fail.
#
# Slice 13 (the redstone mode, `AbstractTileEntity.checkRedstone`) adds:
#  - `/powah redstone ignore|on|off <pos>` sets a machine's mode (the GUI button's replacement) and the
#    readout names `redstone <mode> (powered)` when it is not the default;
#  - the brief's rig — a starter transmitter with its bound card, a cell item in the operator's
#    hotbar, a **redstone block** on its west face — is measured in all three modes: IGNORE charges
#    through it, OFF holds the item's `block_entity_data.energy` and the transmitter's balance still,
#    the same rig with the block gone spends exactly 500 x dt FE against the game's clock, and ON
#    charges only once the block is back;
#  - one machine of each kind that moves FE on its own is held by its gate and runs when it opens:
#    the furnator (no fuel taken, no carbon burnt), the cell (nothing pushed and nothing pulled
#    through its faces), the cable (no relay), the discharger (no drain), the hopper (no charge), the
#    solar panel (no generation under a noon sky) and the ender cell (its own handler refuses);
#  - a foreign block and the energizing orb refuse a mode (upstream's orb screen has no button).
# SELFTEST=redstone expects the shut gate to have charged the item (it must FAIL), proving the gate's
# assertion can fail.
#
# Slice 14 (gap 7, "no comparator output") adds:
#  - a starter cell at 500,000 of 1,000,000 (readout `comparator 7`) with a `minecraft:comparator`
#    on its north face: the block is `powered=true` and the redstone wire in front of it carries the
#    readout's own `comparator` field as its `power` (this pin keeps a comparator's output in the
#    level, not a block entity, so `data get block … OutputSignal` has nothing to read); the reading
#    follows the buffer to 15 at full and 3 at a quarter, which the hook announces
#    (`Level.updateNeighbourForOutputSignal`);
#  - a `minecraft:stone` in the same rig reads 0 and is not powered;
#  - a cable (no buffer) reads nothing even with a full chest behind it — after the control showed
#    the same rig reading that chest's own vanilla fullness;
#  - a full energizing rod reads nothing, because its readout prints no comparator.
# SELFTEST=comparator expects the cell's comparator to read something other than its readout
# (it must FAIL), proving the equality and its notification can fail.
set -euo pipefail
export LC_ALL=C

CLONE=/home/adlf/Desktop/all_the_mods_10_rust_rewrite
PORT=$CLONE/ports/powah
CTL=${CTL:-$CLONE/ports/_shared/bin/horizon-1d63e0a5/horizon-ctl}   # pin-matched shared ctl
GAME=${GAME:-}                       # a launcher build carrying this package
CONTROL_GAME=${CONTROL_GAME:-}       # a launcher build without it, for the control leg
HMOD=${HMOD:-$PORT/powah.hmod}       # the delivered package this run is about
RES=${RES:-${HORIZON_RES:-/home/adlf/Desktop/horizon/.local/horizon-client/26.3}}
SOCK=${SOCK:-${E2E_TMP:-/tmp/opencode}/powah-e2e.sock}
SELFTEST=${SELFTEST:-0}
# The furnator's fuel slot. Slice 7 moved it from 0 to 1 (upstream's order: 0 is
# the charge slot). The counterfactual leg runs this harness against the 0.2.6
# package, whose furnator has one slot at 0, so it sets this to 0 and reaches the
# slice's own assertions instead of failing on a slot layout it predates.
FUEL_CONTAINER_SLOT=${FUEL_CONTAINER_SLOT:-1}
KEEP=${KEEP:-0}
WORLD=PowahE2E
CONTROL_WORLD=PowahControl

[ -x "$CTL" ] || { echo "missing horizon-ctl: $CTL" >&2; exit 1; }
[ -n "$GAME" ] || {
    echo "missing GAME: point it at a launcher-built horizon-client that carries powah" >&2
    echo "  horizon-launcher native-build $PORT --instance <i> --trust-native-code" >&2
    exit 1
}
[ -x "$GAME" ] || { echo "missing horizon-client: $GAME" >&2; exit 1; }
[ -f "$HMOD" ] || { echo "missing the package: $HMOD (cargo horizon-mod build)" >&2; exit 1; }
[ -d "$RES/assets" ] || { echo "missing prepared resources: $RES" >&2; exit 1; }

SCRATCH=$(mktemp -d "${E2E_TMP:-/tmp/opencode}/powah-home.XXXXXX")
export HORIZON_HOME=$SCRATCH/game
export HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data
export XDG_CONFIG_HOME=$SCRATCH/config
export HORIZON_NO_AUDIO=1
mkdir -p "$(dirname "$SOCK")" "$XDG_CONFIG_HOME"
rm -f "$SOCK"

ctl() { "$CTL" --socket "$SOCK" "$@"; }
say() { ctl chat "/$1" >/dev/null; }
fail() { echo "FAIL: $*" >&2; ctl logs --lines 80 >&2 || true; exit 1; }
# inv_count ITEM: how many of ITEM the player's inventory holds (0 when absent).
inv_count() { ctl inventory | jq -r --arg id "$1" '.counts[$id] // 0'; }

# put_fuel X Y Z ITEM COUNT: the game's own container path (`item replace block`,
# what a hopper reaches). Measured at 0b14f8a: `ctl container` reads a mod block
# entity's raw `items`, which the pin grows only through this command path or an
# opened menu, so the declared slot is invisible to that ctl call. The slot's
# own read is `/powah info`'s `fuel` field (below), which is stronger: it is the
# port's saved data, not a harness copy.
#
# Slot 1, as upstream's `FurnatorTile` has it: slice 7 added the charge slot at 0
# (upstream's order) and moved the fuel from 0 to 1.
put_fuel() { # put_fuel X Y Z ITEM COUNT
    ctl command "item replace block $1 $2 $3 container.${FUEL_CONTAINER_SLOT} with $4 $5" >/dev/null
    echo "  item replace block($1,$2,$3) container.${FUEL_CONTAINER_SLOT} <- $4 x$5"
}
# put_charge X Y Z ITEM: one item into a machine's charge slot (container.0), the
# way automation fills it.
put_charge() { # put_charge X Y Z ITEM
    ctl command "item replace block $1 $2 $3 container.0 with $4 1" >/dev/null
    ctl wait 3
    echo "  item replace block($1,$2,$3) container.0 <- $4 x1"
}
# charge_of LINE: a readout's first charge slot, e.g. `powah:energy_cell_starter
# 40/1000000`, or `empty`.
charge_of() { sed -nE 's/.*charge ([^,]+),.*/\1/p' <<<"$1"; }
# charge_energy_of LINE: the FE that first charge slot's item holds.
charge_energy_of() { sed -nE 's/.*charge [^ ]+ ([0-9]+)\/[0-9]+.*/\1/p' <<<"$1"; }
# time_of LINE: the game time the readout was printed at.
time_of() { sed -nE 's/.* t=([0-9]+)\)$/\1/p' <<<"$1"; }
# tool_up: survival, an empty inventory and an iron pickaxe in hand (a
# `needs_iron_tool` block drops nothing without it). The pickup of a broken
# machine's item then lands in hotbar slot 2, where `ctl select <item>` finds it.
tool_up() {
    ctl command "gamemode survival @s" >/dev/null
    ctl command "clear @s" >/dev/null
    ctl command "item replace entity @s hotbar.0 with minecraft:iron_pickaxe" >/dev/null
    ctl select 1 >/dev/null
    ctl wait 4
}
# select_item ITEM: hold ITEM from the hotbar, or fail with the inventory so a red
# run says where the item went.
select_item() { # select_item ITEM
    ctl select "$1" >/dev/null 2>&1 ||
        fail "cannot hold $1 from the hotbar: $(ctl inventory | jq -c . | head -c 600)"
    ctl wait 2
}
# pickup ITEM X Y Z: the item a break just dropped must reach the inventory and be
# in hand. `ctl tp` onto the item entity's own reported position (its reply carries
# `pos`), never `ctl collect`: that one *walks*, and a walk off this arena's edge
# is a 35-block fall — measured, it killed the player and emptied the inventory
# (the world's spawn is at y=64). An item that lands in the main inventory is
# moved into the hotbar with vanilla's own `/item replace … from entity`, the same
# stack and payload, never a copy.
pickup() { # pickup ITEM X Y Z
    local tries=0 drop slot main k dx dy dz py centre
    centre=$(awk -v z="$4" 'BEGIN { print z + 0.5 }')
    ctl wait 10
    while [ "$tries" -lt 4 ] && [ "$(inv_count "$1")" -lt 1 ]; do
        drop=$(ctl entities 32 item |
            jq -r --arg id "$1" '[.entities[] | select(.item.item == $id) | .pos] | first // empty | @csv')
        if [ -n "$drop" ]; then
            IFS=, read -r dx dy dz <<<"$drop"
            echo "  the drop lies at $drop"
            ctl tp "$dx" "$dy" "$dz" >/dev/null
        else
            # The item may be in the machine's own cell under a random offset the entity
            # read has not caught yet: stand in that cell, not two blocks away from it.
            ctl tp "$2.5" "$3" "$centre" >/dev/null
        fi
        # A tp onto the drop can land the player a block low on a loaded box; the fall empties the
        # inventory and the leg then hunts a drop it already had (measured 2026-11-23, twice).
        py=$(ctl state | jq -r '.player.position[1] // 0')
        if ! awk -v y="$py" 'BEGIN { exit !(y >= 95) }'; then
            echo "  the player is at y=$py after the tp; standing back on the platform"
            ctl tp "$2.5" "$3" "$centre" >/dev/null
        fi
        ctl wait 20
        tries=$((tries + 1))
    done
    [ "$(inv_count "$1")" -ge 1 ] ||
        fail "$1 did not reach the inventory after the break: $(ctl inventory | jq -c '{counts, hotbar}' | head -c 400) nearby items: $(ctl entities 32 item | jq -c '[.entities[] | {item: .item.item, pos}]' | head -c 400)"
    slot=$(ctl inventory | jq -r --arg i "$1" '[.hotbar[] | select(.item == $i) | .slot] | .[0] // ""')
    if [ -z "$slot" ]; then
        main=$(ctl inventory | jq -r --arg i "$1" '[.main[] | select(.item == $i) | .slot] | .[0] // ""')
        [ -n "$main" ] ||
            fail "$1 is in the inventory but in neither the hotbar nor the main slots: $(ctl inventory | jq -c '{counts, hotbar, main}' | head -c 500)"
        for k in "$((main - 9))" "$main"; do
            ctl command "item replace entity @s hotbar.1 from entity @s inventory.$k" >/dev/null
            ctl wait 6
            slot=$(ctl inventory | jq -r --arg i "$1" '[.hotbar[] | select(.item == $i) | .slot] | .[0] // ""')
            [ -n "$slot" ] && break
        done
    fi
    [ -n "$slot" ] || fail "could not move $1 into the hotbar: $(ctl inventory | jq -c '{counts, hotbar}' | head -c 400)"
    # The hold check re-reads the hotbar: a selection made from a snapshot taken just
    # after a pickup can answer `null` for a slot that is filled a tick later (measured
    # twice on a loaded box, where four other e2e games shared the machine). Each
    # attempt still requires the item in hand.
    tries=0
    while [ "$tries" -lt 3 ]; do
        slot=$(ctl inventory | jq -r --arg i "$1" '[.hotbar[] | select(.item == $i) | .slot] | .[0] // ""')
        if [ -z "$slot" ]; then
            break
        fi
        ctl select $((slot + 1)) >/dev/null
        ctl wait 6
        if [ "$(ctl inventory | jq -r '.held.item // ""')" = "$1" ]; then
            break
        fi
        tries=$((tries + 1))
    done
    [ "$(ctl inventory | jq -r '.held.item // ""')" = "$1" ] ||
        fail "$1 is not in hand after selecting slot $slot: $(ctl inventory | jq -c '{counts, hotbar, held}' | head -c 500)"
    # stand beside the block again, not in it: the caller places the item back here
    ctl tp "$2.5" "$3" "$(awk -v z="$4" 'BEGIN { print z + 1.5 }')" >/dev/null
    ctl wait 4
}
# place_item X Y Z ITEM: place ITEM from the hotbar at an empty position, from two
# blocks away as `rftoolsstorage`'s rig does, and assert the block is there. A
# refused click keeps the stack, so the attempt is retried; a consumed one does
# not, and is reported.
place_item() { # place_item X Y Z ITEM
    local i name
    for i in 1 2 3; do
        ctl tp "$1.5" 100 "$(($3 + 2)).5" >/dev/null
        ctl wait 6
        ctl place "$1" 100 "$3" --item "$4" >/dev/null 2>&1 || true
        ctl wait 8
        name=$(ctl block "$1" 100 "$3" | jq -r '.name // .block // empty')
        if [ "$name" = "$4" ]; then
            echo "  placed $4 at $1,100,$3"
            return 0
        fi
        echo "  place attempt $i at $1,100,$3 left '$name' ($(ctl inventory | jq -c '.counts'))"
        ctl wait 10
    done
    fail "the dropped $4 did not place at $1,100,$3: $(ctl state | jq -c '.player.position')"
}
# item_energy X Y Z AMOUNT ITEM: `/powah item`, asserting the command's own reply
# (the item's charge is written through its place, a round trip later).
item_energy() { # item_energy X Y Z AMOUNT ITEM
    local out line=""
    say "powah item $4 $1 $2 $3"
    for _ in $(seq 1 30); do
        ctl wait 10
        out=$(ctl messages 20)
        line=$(jq -r '.chat[]' <<<"$out" |
            grep -E "Charge set to|carries FE|has no item charge slot|No Powah block" | head -1)
        case "$line" in
            *"Charge set to $4 FE on $5."*) echo "  item($1,$2,$3) reply: $line"; return 0 ;;
        esac
    done
    fail "expected /powah item $4 at $1,$2,$3 to answer 'Charge set to $4 FE on $5.', got '$line'"
}
# fuel_is X Y Z ITEM COUNT: the furnator's own fuel slot holds COUNT of ITEM.
fuel_is() { # fuel_is X Y Z ITEM COUNT
    local line want="fuel $4 x$5"
    line=$(info "$1" "$2" "$3")
    case "$line" in
        *"$want"*) echo "  fuel($1,$2,$3) $4 x$5: ok" ;;
        *) fail "expected the fuel slot at $1,$2,$3 to hold $5 x $4; got '$line'" ;;
    esac
}
# info X Y Z: this block's status line, after a fresh reply arrives (the t=
# field makes each reply unique).
# info X Y Z: one readout, re-asked until its own `t=` is newer than the one read before the command
# was sent. A chat round trip on a loaded box can take 100 ticks, and the reply to a *previous* call
# then sits in the 20-line window: accepting it read a block 110 ticks stale and cost this slice two
# runs (2026-11-23, slices 3's two-cell legs, on a box running five game legs). A readout that never
# advances is asked again, never accepted; the tick it names is the game's own clock.
info() { # info X Y Z
    local out line before before_t line_t tries
    before=$(ctl messages 20 | jq -r '.chat[]' | grep -E " at $1,$2,$3:" | head -1)
    before_t=$(time_of "$before")
    for tries in 1 2 3 4 5; do
        say "powah info $1 $2 $3"
        for _ in $(seq 1 12); do
            ctl wait 4 >/dev/null
            out=$(ctl messages 20)
            line=$(jq -r '.chat[]' <<<"$out" | grep -E " at $1,$2,$3:|No Powah block" | head -1)
            [ -n "$line" ] || continue
            line_t=$(time_of "$line")
            if [ -n "$line_t" ] && [ -n "$before_t" ]; then
                [ "$line_t" -gt "$before_t" ] && { echo "$line"; return; }
            elif [ "$line" != "$before" ]; then
                echo "$line"
                return
            fi
        done
        before=$line
        before_t=$line_t
    done
    echo "NO-INFO $(jq -c '{chat,action_bar}' <<<"$out")"
}
# set_energy X Y Z AMOUNT: the harness control `/powah set`, retried until the
# block reads the value back (chat commands are asynchronous).
set_energy() { # X Y Z AMOUNT
    local want=$4 line
    say "powah set $want $1 $2 $3"
    for _ in $(seq 1 10); do
        line=$(info "$1" "$2" "$3")
        if [ "$(energy_of "$line")" = "$want" ]; then
            echo "  set($1,$2,$3) = $line"
            return
        fi
        say "powah set $want $1 $2 $3"
    done
    fail "could not set $1,$2,$3 to $want FE: $line"
}
# set_clamped X Y Z AMOUNT WANT: `/powah set AMOUNT`, retried until the cell reads
# WANT back. A tier cell clamps AMOUNT at its own capacity, so the reply's number
# is the clamp (not AMOUNT) and `set_energy`'s equality poll cannot be used.
set_clamped() { # X Y Z AMOUNT WANT
    local want=$5 line
    say "powah set $4 $1 $2 $3"
    for _ in $(seq 1 8); do
        line=$(info "$1" "$2" "$3")
        if [ "$(energy_of "$line")" = "$want" ]; then
            echo "  set($1,$2,$3) = $line"
            return
        fi
        say "powah set $4 $1 $2 $3"
    done
    fail "could not clamp $1,$2,$3 to $want FE: $line"
}
# set_floor X Y Z AMOUNT FLOOR: `/powah set AMOUNT`, retried until the cell holds
# at least FLOOR. A source beside a cable loses 500 FE/t as soon as it is set, so
# an exact-equality poll never settles (measured: 99,500 after one round trip);
# the phase's conservation check is what pins the total.
set_floor() { # X Y Z AMOUNT FLOOR
    local floor=$5 line
    say "powah set $4 $1 $2 $3"
    for _ in $(seq 1 10); do
        line=$(info "$1" "$2" "$3")
        if [ "$(energy_of "$line")" -ge "$floor" ]; then
            echo "  set($1,$2,$3) = $line"
            return
        fi
        say "powah set $4 $1 $2 $3"
    done
    fail "could not set $1,$2,$3 to at least $floor FE: $line"
}
expect_info() { # expect_info X Y Z SUBSTRING
    local line
    line=$(info "$1" "$2" "$3")
    echo "  info($1,$2,$3): $line"
    case "$line" in
        *"$4"*) ;;
        *) fail "expected /powah info to say '$4', got '$line'" ;;
    esac
}

# channel_reply X Y Z CHANNEL SUBSTRING: `/powah channel`, asserting the command's own reply — the
# control that replaces upstream's GUI buttons (`SetChannelPacket`).
channel_reply() { # channel_reply X Y Z CHANNEL SUBSTRING
    local out line=""
    say "powah channel $4 $1 $2 $3"
    for _ in $(seq 1 30); do
        ctl wait 10
        out=$(ctl messages 20)
        line=$(jq -r '.chat[]' <<<"$out" |
            grep -E "Channel set to|No ender cell or gate" | head -1)
        case "$line" in
            *"$5"*) echo "  channel($1,$2,$3) reply: $line"; return 0 ;;
        esac
    done
    fail "expected /powah channel $4 at $1,$2,$3 to answer '$5', got '$line'"
}

# redstone_reply X Y Z MODE SUBSTRING: `/powah redstone`, asserting the command's own reply — the
# control that replaces upstream's GUI redstone button (`NextRedstoneModePacket`).
redstone_reply() { # redstone_reply X Y Z MODE SUBSTRING
    local out line=""
    say "powah redstone $4 $1 $2 $3"
    for _ in $(seq 1 30); do
        ctl wait 10
        out=$(ctl messages 20)
        line=$(jq -r '.chat[]' <<<"$out" |
            grep -E "Redstone mode set to|no redstone mode|No Powah block" | head -1)
        case "$line" in
            *"$5"*) echo "  redstone($1,$2,$3) reply: $line"; return 0 ;;
        esac
    done
    fail "expected /powah redstone $4 at $1,$2,$3 to answer '$5', got '$line'"
}

# set_reply X Y Z AMOUNT SUBSTRING: `/powah set`, asserting the command's own
# reply. Used where the target feeds something and an equality poll on its buffer
# can never settle (a rod beside an orb with a recipe drains the same tick).
set_reply() { # X Y Z AMOUNT SUBSTRING
    local out line=""
    say "powah set $4 $1 $2 $3"
    # A chat command's reply lands a round trip later (~40 ticks), so the wait is
    # generous: ten iterations of `wait 4` read the chat before it arrived.
    for _ in $(seq 1 30); do
        ctl wait 10
        out=$(ctl messages 20)
        # `chat` is newest first, so the newest matching line is the first.
        line=$(jq -r '.chat[]' <<<"$out" |
            grep -E "Energy set to|stores no energy|No Powah block" | head -1)
        case "$line" in
            *"$5"*) echo "  set($1,$2,$3) reply: $line"; return 0 ;;
        esac
    done
    fail "expected /powah set $4 at $1,$2,$3 to answer '$5', got '$line'"
}

# --- slice 5: the energizing orb's seven slots ---------------------------------

# orb_clear X Y Z: empty the orb's seven slots, the previous run's result included
# (the orb's tick clears them all and writes the result, as upstream's `inv.clear`
# does, so a stale result would be overwritten unreported).
orb_clear() { # orb_clear X Y Z
    local slot
    for slot in 0 1 2 3 4 5 6; do
        ctl command "item replace block $1 $2 $3 container.$slot with minecraft:air" >/dev/null
    done
    ctl wait 3
}
# orb_put X Y Z SLOT ITEM COUNT: one input stack, the way automation fills the orb
# (`item replace block ... container.N`, the same path the furnator's fuel uses).
orb_put() { # orb_put X Y Z SLOT ITEM COUNT
    ctl command "item replace block $1 $2 $3 container.$4 with $5 $6" >/dev/null
    ctl wait 2
}
# orb_slot X Y Z N: the orb's slot N, as `ctl container` reports it.
orb_slot() { # orb_slot X Y Z N
    ctl container "$1" "$2" "$3" | jq -c --argjson n "$4" '.slots[$n]'
}
# orb_wait X Y Z ITEM COUNT: poll slot 0 until it holds COUNT of ITEM. A chat round
# trip is ~40 ticks, and a nitro rod fills 200,000 a tick, so the wait is generous
# (a 20,000,000 FE recipe takes 100 fills).
orb_wait() { # orb_wait X Y Z ITEM COUNT
    local i slot
    for i in $(seq 1 40); do
        ctl wait 10
        slot=$(orb_slot "$1" "$2" "$3" 0)
        if [ "$(jq -r '.item // ""' <<<"$slot")" = "$4" ] &&
            [ "$(jq -r '.count // 0' <<<"$slot")" = "$5" ]; then
            echo "$slot"
            return 0
        fi
    done
    echo "$slot"
    return 1
}
# orb_no_result X Y Z: slot 0 is still empty (a control run).
orb_no_result() { # orb_no_result X Y Z
    local slot
    slot=$(orb_slot "$1" "$2" "$3" 0)
    [ "$(jq -r '.item // ""' <<<"$slot")" = "" ] ||
        fail "the orb at $1,$2,$3 produced $slot when it should not have"
}

# The energy in an info line: the number before "/<capacity> FE".
energy_of() { sed -nE 's/.* at [-0-9]+,[-0-9]+,[-0-9]+: ([0-9]+)\/[0-9]+ FE.*/\1/p' <<<"$1"; }
carbon_of() { sed -nE 's/.*carbon ([0-9]+).*/\1/p' <<<"$1"; }
burning_of() { sed -nE 's/.*burning (true|false).*/\1/p' <<<"$1"; }

# --- slice 9: a container's item, read in place ---------------------------------

# chest_energy X Y Z SLOT: the `energy` this mod's item payload holds on the stack in a
# container's slot. The read is the server's own NBT path (no move, so it never disturbs the
# machine that is charging the item, and no `data get entity SelectedItem`, which is one
# selection stale); empty when the stack carries no payload at all (a stone, a fresh item).
chest_energy() { # chest_energy X Y Z SLOT
    local reply
    reply=$(ctl command "data get block $1 $2 $3 Items[$4].components.\"minecraft:block_entity_data\".energy")
    echo "  data get block($1,$2,$3) Items[$4]: $reply" >&2
    sed -nE 's/.*: *(-?[0-9]+).*/\1/p' <<<"$reply"
}

# inv_energy SLOT: the `energy` this mod's item payload holds in the operator's own inventory slot
# (`Inventory`'s numbers: hotbar.0 is 0). Read with `@p` because a console command's `@s` is the
# console, and with the slot path, not `SelectedItem` (one selection stale, measured).
inv_energy() { # inv_energy SLOT
    local reply text
    reply=$(ctl command "data get entity @p Inventory[{Slot:${1}b}].components.\"minecraft:block_entity_data\".energy")
    echo "  data get entity @p Inventory[{Slot:${1}b}]: $reply" >&2
    # `ctl command` answers `{"chat":[…]}`; the number is in the reply *text*, not the JSON around it.
    text=$(jq -r 'try ([.chat[]?] | join(" ")) catch ""' <<<"$reply" 2>/dev/null)
    [ -n "$text" ] || text=$reply
    sed -nE 's/.*: *(-?[0-9]+).*/\1/p' <<<"$text"
}
# card_bound SLOT: the name on the binding card in the operator's own inventory slot, empty when the
# card carries no `powah:bound_player` at all (the control's blank card).
card_bound() { # card_bound SLOT
    local reply text
    reply=$(ctl command "data get entity @p Inventory[{Slot:${1}b}].components.\"powah:bound_player\".name")
    echo "  card bound name: $reply" >&2
    text=$(jq -r 'try ([.chat[]?] | join(" ")) catch ""' <<<"$reply" 2>/dev/null)
    [ -n "$text" ] || text=$reply
    sed -nE 's/.*: *"([^"]*)".*/\1/p' <<<"$text"
}

cleanup() {
    rc=$?
    # A quit that never answers must not hold the seat: the guard's idle timeout would kill the
    # run after the last assertion (measured 2026-11-23, a 17,689-tick session on a loaded box).
    timeout 60 "$CTL" --socket "$SOCK" quit >/dev/null 2>&1 || true
    sleep 1
    if [ "$KEEP" = 1 ]; then
        echo "kept scratch: $SCRATCH" >&2
    else
        rm -rf "$SCRATCH" 2>/dev/null || true
    fi
    echo "EXIT:$rc"
}
trap cleanup EXIT

launch() {
    rm -f "$SOCK"
    cd "$SCRATCH"
    local game=${LAUNCH_GAME:-$GAME}
    echo "  launch: $(basename "$game"), no --dev-mod: the package carries its content"
    # `--offscreen`, not `--headless`: slice 2 opens a crafting table and reads its
    # slots, and at this pin a menu's widget layout needs the renderer — with
    # `--headless`, `ctl screen`/`ctl ui` time out on an open menu (measured).
    "$CTL" --socket "$SOCK" launch \
        --game "$game" --world "${WORLD_NAME:-$WORLD}" --seed 20261105 --creative --offscreen \
        --wait --resources "$RES" >/dev/null
    ctl wait-ready 120
    ctl wait 40
}

# A stone floor at y=99 with clear air above, in the Overworld, and the player
# standing on it at (0.5, 100, 3.5).
arena() {
    ctl tp 0.5 101 3.5 >/dev/null
    ctl wait 20
    ctl command "fill -8 99 -8 24 99 24 minecraft:stone" >/dev/null
    ctl command "fill -8 100 -8 24 110 24 minecraft:air" >/dev/null
    ctl command "difficulty peaceful" >/dev/null
    ctl command "kill @e[type=!player]" >/dev/null
    # The pickup paths `ctl tp` onto a dropped item's own position. On a loaded box the player has
    # twice landed a block low and taken the ~35-block fall into the world (measured 2026-11-23:
    # "fell from a high place", the inventory emptied, the leg then failed to find its own drop),
    # and no assertion here is about fall damage or a death's inventory. Both rules are reported so
    # a build that refuses one says so in the log instead of going quietly flaky.
    echo "  gamerule fall_damage: $(ctl command "gamerule fall_damage false")"
    echo "  gamerule keep_inventory: $(ctl command "gamerule keep_inventory true")"
    ctl tp 0.5 100 3.5 >/dev/null
    ctl wait 20
    local y feet floor
    y=$(ctl state | jq -r '.player.position[1] // 0')
    feet=$(ctl block 0 100 3 | jq -r '.name // .block // empty')
    floor=$(ctl block 0 99 3 | jq -r '.name // .block // empty')
    echo "  arena: y=$y floor(0,99,3)=$floor feet(0,100,3)=$feet"
    awk -v y="$y" 'BEGIN { exit !(y >= 99.9) }' || fail "the player is not on the arena floor (y=$y)"
    [ "$feet" = "minecraft:air" ] || fail "the arena volume is not clear ($feet at 0,100,3)"
    [ "$floor" = "minecraft:stone" ] || fail "the arena floor is missing ($floor at 0,99,3)"
}

place() { # place X Y Z ID
    ctl command "setblock $1 $2 $3 $4" >/dev/null
    ctl wait 4
    local got
    got=$(ctl block "$1" "$2" "$3" | jq -r '.name // .block // empty')
    echo "  block($1,$2,$3) = $got"
    [ "$got" = "$4" ] || fail "expected $4 at $1,$2,$3, got '$got'"
}

# floor_under X Z: a stone platform at y=99 around the machine column. The arena floor spans
# -8..24, and a `keepEnergy` leg stands its machine on y=100 outside that: without a platform, the
# item a break drops falls into the world and the *player* does too, so the pickup's entity read then
# sees the ambient drops at y=63 instead of the machine (measured 2026-11-22, two runs; the 0.2.8 run
# won the race). It is wider than the machine on purpose: a dropped item's random offset can throw it
# a block off the machine's column (measured 2026-11-23: a furnator drop at x=33.63 from a machine at
# x=32), and a drop that lands one block outside a 3x4 platform falls 36 blocks and is lost. The
# platform covers the standing spots a pickup and a `place_item` use, not the whole arena: it must not
# erase a neighbouring phase's block.
floor_under() { # floor_under X Z
    ctl command "fill $(($1 - 2)) 99 $(($2 - 1)) $(($1 + 2)) 99 $(($2 + 3)) minecraft:stone" >/dev/null
    # Prove the platform is there now, instead of discovering it missing when the player falls.
    local floor
    floor=$(ctl block "$1" 99 "$2" | jq -r '.name // .block // empty')
    echo "  platform($1,99,$2) = $floor"
    [ "$floor" = "minecraft:stone" ] ||
        fail "the pickup platform at $1,99,$2 is '$floor', so a later tp would drop the player"
}

# The negative control: an unmodified client of the same build. The block id must
# be unknown there, the item must not arrive and the block must stay air, so the
# machines, the commands and the transfer demonstrably come from this package.
# Skipped (and said so) when no `CONTROL_GAME` is given.
control_leg() {
    if [ -z "$CONTROL_GAME" ]; then
        echo "== control: skipped (set CONTROL_GAME=<unmodified client> to run it) =="
        return
    fi
    [ -x "$CONTROL_GAME" ] || fail "missing CONTROL_GAME: $CONTROL_GAME"
    echo "== control: the same build with no powah =="
    echo "  package under test: $HMOD ($(stat -c %s "$HMOD") B, sha256 $(sha256sum "$HMOD" | cut -d' ' -f1))"
    LAUNCH_GAME=$CONTROL_GAME WORLD_NAME=$CONTROL_WORLD launch
    arena
    local setblock give name
    setblock=$(ctl command "setblock 4 100 0 powah:furnator_starter")
    echo "  setblock reply: $setblock"
    case "$setblock" in
        *"Unknown block"*|*"unknown block"*|*"error"*) ;;
        *) fail "the control game knew powah:furnator_starter, so the content did not come from this package" ;;
    esac
    give=$(ctl command "give @s powah:energy_cell_starter 1")
    echo "  give reply: $give"
    case "$give" in
        *"Unknown item"*|*"unknown item"*|*"error"*|*"not found"*) ;;
        *) fail "the control game knew powah:energy_cell_starter, so the content did not come from this package" ;;
    esac
    # Slice 4's ladder: the highest tier is unknown without the package too.
    tier=$(ctl command "setblock 5 100 0 powah:energy_cell_nitro")
    echo "  tier setblock reply: $tier"
    case "$tier" in
        *"Unknown block"*|*"unknown block"*|*"error"*) ;;
        *) fail "the control game knew powah:energy_cell_nitro, so the content did not come from this package" ;;
    esac
    # Slice 5: the orb, a rod and the material it makes are all this package's.
    orb=$(ctl command "setblock 4 100 1 powah:energizing_orb")
    echo "  orb setblock reply: $orb"
    case "$orb" in
        *"Unknown block"*|*"unknown block"*|*"error"*) ;;
        *) fail "the control game knew powah:energizing_orb, so the content did not come from this package" ;;
    esac
    rod=$(ctl command "setblock 4 100 2 powah:energizing_rod_nitro")
    echo "  rod setblock reply: $rod"
    case "$rod" in
        *"Unknown block"*|*"unknown block"*|*"error"*) ;;
        *) fail "the control game knew powah:energizing_rod_nitro, so the content did not come from this package" ;;
    esac
    crystal=$(ctl command "give @s powah:crystal_blazing 1")
    echo "  crystal give reply: $crystal"
    case "$crystal" in
        *"Unknown item"*|*"unknown item"*|*"error"*|*"not found"*) ;;
        *) fail "the control game knew powah:crystal_blazing, so the content did not come from this package" ;;
    esac
    # Slice 6: a furnator and a cable tier this slice adds are unknown too.
    tier_block=$(ctl command "setblock 5 100 1 powah:furnator_nitro")
    echo "  furnator tier setblock reply: $tier_block"
    case "$tier_block" in
        *"Unknown block"*|*"unknown block"*|*"error"*) ;;
        *) fail "the control game knew powah:furnator_nitro, so the content did not come from this package" ;;
    esac
    tier_cable=$(ctl command "setblock 5 100 2 powah:energy_cable_nitro")
    echo "  cable tier setblock reply: $tier_cable"
    case "$tier_cable" in
        *"Unknown block"*|*"unknown block"*|*"error"*) ;;
        *) fail "the control game knew powah:energy_cable_nitro, so the content did not come from this package" ;;
    esac
    # Slice 8: the discharger and its item are the package's as well.
    discharger=$(ctl command "setblock 6 100 2 powah:energy_discharger_starter")
    echo "  discharger setblock reply: $discharger"
    case "$discharger" in
        *"Unknown block"*|*"unknown block"*|*"error"*) ;;
        *) fail "the control game knew powah:energy_discharger_starter, so the content did not come from this package" ;;
    esac
    discharger_item=$(ctl command "give @s powah:energy_discharger_nitro 1")
    echo "  discharger give reply: $discharger_item"
    case "$discharger_item" in
        *"Unknown item"*|*"unknown item"*|*"error"*|*"not found"*) ;;
        *) fail "the control game knew powah:energy_discharger_nitro, so the content did not come from this package" ;;
    esac
    hopper=$(ctl command "setblock 7 100 0 powah:energy_hopper_starter")
    echo "  hopper setblock reply: $hopper"
    case "$hopper" in
        *"Unknown block"*|*"unknown block"*|*"error"*) ;;
        *) fail "the control game knew powah:energy_hopper_starter, so the content did not come from this package" ;;
    esac
    hopper_item=$(ctl command "give @s powah:energy_hopper_nitro 1")
    echo "  hopper give reply: $hopper_item"
    case "$hopper_item" in
        *"Unknown item"*|*"unknown item"*|*"error"*|*"not found"*) ;;
        *) fail "the control game knew powah:energy_hopper_nitro, so the content did not come from this package" ;;
    esac
    solar=$(ctl command "setblock 8 100 0 powah:solar_panel_starter")
    echo "  solar setblock reply: $solar"
    case "$solar" in
        *"Unknown block"*|*"unknown block"*|*"error"*) ;;
        *) fail "the control game knew powah:solar_panel_starter, so the content did not come from this package" ;;
    esac
    pane=$(ctl command "give @s powah:photoelectric_pane 1")
    echo "  pane give reply: $pane"
    case "$pane" in
        *"Unknown item"*|*"unknown item"*|*"error"*|*"not found"*) ;;
        *) fail "the control game knew powah:photoelectric_pane, so the content did not come from this package" ;;
    esac
    # Slice 12: the player transmitter is this package's, and so is the card.
    transmitter=$(ctl command "setblock 4 100 2 powah:player_transmitter_starter")
    echo "  transmitter setblock reply: $transmitter"
    case "$transmitter" in
        *"Unknown block"*|*"unknown block"*|*"error"*) ;;
        *) fail "the control game knew powah:player_transmitter_starter, so the content did not come from this package" ;;
    esac
    card=$(ctl command "give @s powah:binding_card 1")
    echo "  card give reply: $card"
    case "$card" in
        *"Unknown item"*|*"unknown item"*|*"error"*|*"not found"*) ;;
        *) fail "the control game knew powah:binding_card, so the content did not come from this package" ;;
    esac
    # Slice 11: the ender cell, the ender gate and the core they are built from are this package's.
    ender_cell=$(ctl command "setblock 9 100 0 powah:ender_cell_starter")
    echo "  ender cell setblock reply: $ender_cell"
    case "$ender_cell" in
        *"Unknown block"*|*"unknown block"*|*"error"*) ;;
        *) fail "the control game knew powah:ender_cell_starter, so the content did not come from this package" ;;
    esac
    ender_gate=$(ctl command "setblock 10 100 0 powah:ender_gate_nitro")
    echo "  ender gate setblock reply: $ender_gate"
    case "$ender_gate" in
        *"Unknown block"*|*"unknown block"*|*"error"*) ;;
        *) fail "the control game knew powah:ender_gate_nitro, so the content did not come from this package" ;;
    esac
    ender_core=$(ctl command "give @s powah:ender_core 1")
    echo "  ender core give reply: $ender_core"
    case "$ender_core" in
        *"Unknown item"*|*"unknown item"*|*"error"*|*"not found"*) ;;
        *) fail "the control game knew powah:ender_core, so the content did not come from this package" ;;
    esac
    name=$(ctl block 4 100 0 | jq -r '.name // .block // "unknown"')
    [ "$name" = "minecraft:air" ] || fail "the control game holds $name, so the block id was known"
    echo "  without the package: no block, no item (block 4,100,0 = $name)"
    ctl quit >/dev/null 2>&1 || true
}

echo "== launch (Overworld arena at y=100; one package, code and content together) =="
# Name the archive this leg binds: its size and sha256 are what a reviewer compares
# the evidence against, not the path it happened to sit at.
echo "  package: $(basename "$HMOD") ($(stat -c %s "$HMOD") B, sha256 $(sha256sum "$HMOD" | cut -d' ' -f1))"
control_leg
launch
arena

echo "== place the furnator, the cable and the cell in a line =="
place 0 100 0 powah:furnator_starter
place 1 100 0 powah:energy_cable_starter
place 2 100 0 powah:energy_cell_starter

echo "== control: no fuel means no FE anywhere =="
line=$(info 0 100 0)
echo "  furnator: $line"
[ "$(energy_of "$line")" = 0 ] || fail "a furnator with no fuel stored FE: $line"
[ "$(burning_of "$line")" = false ] || fail "a furnator with no fuel is burning: $line"
ctl wait 40
line=$(info 0 100 0)
echo "  after 40 ticks: $line"
[ "$(energy_of "$line")" = 0 ] || fail "a furnator with no fuel made FE: $line"
if [ "$SELFTEST" = 1 ]; then
    echo "  SELFTEST: expecting the empty furnator to have made FE (this must fail)"
    [ "$(energy_of "$line")" != 0 ] || fail "SELFTEST flipped: the unfuelled furnator made no FE, as it should"
fi
expect_info 2 100 0 "Energy Cell at 2,100,0: 0/1000000 FE"
expect_info 1 100 0 "no storage, moves up to 500 FE/t"

echo "== coal into the fuel slot starts the burn =="
put_fuel 0 100 0 minecraft:coal 5
ctl wait 6
# the first item is already carbon by the next tick, so four are left
fuel_is 0 100 0 minecraft:coal 4
line=$(info 0 100 0)
echo "  $line"
[ "$(carbon_of "$line")" -gt 0 ] || fail "the furnace did not load the coal into carbon: $line"
[ "$(burning_of "$line")" = true ] || fail "the furnator is not burning: $line"
fuel_is 0 100 0 minecraft:coal 4
carbon=$(carbon_of "$line")
[ "$carbon" -le 48000 ] || fail "carbon above one coal's 48,000: $line"

echo "== the furnator makes FE and the cable carries it into the cell =="
ctl wait 40
line=$(info 0 100 0)
echo "  furnator: $line"
carbon_now=$(carbon_of "$line")
cell_line=$(info 2 100 0)
echo "  cell: $cell_line"
cell=$(energy_of "$cell_line")
[ "${carbon_now:-$carbon}" -lt "$carbon" ] || fail "the furnator is not burning its carbon: $line"
[ "${cell:-0}" -gt 0 ] || fail "the cable did not carry FE into the cell: $cell_line"
# One coal holds 48,000 FE: the cell can never hold more than that, so a
# duplicated transfer would show here.
[ "$cell" -le 48000 ] || fail "the cell holds more than one coal's 48,000 FE: $cell_line"
echo "  carbon ${carbon} -> ${carbon_now}, cell ${cell} FE after 40 burning ticks"

echo "== control: without the cable the cell stays empty =="
ctl command "setblock 1 100 0 minecraft:air" >/dev/null
ctl wait 4
set_energy 2 100 0 0
line=$(info 0 100 0)
held=$(energy_of "$line")
ctl wait 40
cell_line=$(info 2 100 0)
echo "  no cable: furnator $held FE, cell $cell_line"
[ "$(energy_of "$cell_line")" = 0 ] || fail "the cell charged without a cable: $cell_line"
line=$(info 0 100 0)
echo "  furnator after 40 more ticks: $line"
[ "$(energy_of "$line")" -ge "$held" ] || fail "the furnator lost FE in the no-cable control: $line"

echo "== control: a full cell stays full =="
set_energy 2 100 0 1000000
ctl wait 40
line=$(info 2 100 0)
echo "  after 40 ticks: $line"
[ "$(energy_of "$line")" = 1000000 ] || fail "a full cell changed: $line"

echo "== the cable again: the cell charges once more =="
set_energy 2 100 0 0
place 1 100 0 powah:energy_cable_starter
ctl wait 40
cell_line=$(info 2 100 0)
echo "  cell: $cell_line"
[ "$(energy_of "$cell_line")" -gt 0 ] || fail "the cable did not feed the cell again: $cell_line"

echo "== a charged cell discharges through a cable into an empty cell =="
# Break the first cable so the charged cell is not refilled, then build a
# second row: cell (2,100,0) - cable (2,100,1) - empty cell (2,100,2). The two
# cells are not adjacent: the cable is what carries the charge, and the source
# is the cell's own extract side (a cell pushes and takes on every side by
# default, so two adjacent cells would swap their charge every tick instead).
ctl command "setblock 1 100 0 minecraft:air" >/dev/null
ctl wait 4
set_energy 2 100 0 1000
place 2 100 1 powah:energy_cable_starter
place 2 100 2 powah:energy_cell_starter
ctl wait 40
sink_line=$(info 2 100 2)
source_line=$(info 2 100 0)
# Two chat round trips are ~40 ticks apart, and the transfer can land between
# them (measured: the sink read 500 while the source's earlier read still said
# 1000), so poll the source until the move shows, then assert both.
for _ in $(seq 1 8); do
    [ "$(energy_of "$source_line")" -lt 1000 ] && break
    ctl wait 10
    source_line=$(info 2 100 0)
done
# The sink read can land before the move the source poll waited for (measured: sink
# 0 at t=552, source 500 at t=557), so re-read the sink until its charge shows.
for _ in $(seq 1 8); do
    [ "$(energy_of "$sink_line")" -gt 0 ] && break
    ctl wait 10
    sink_line=$(info 2 100 2)
done
echo "  source: $source_line"
echo "  sink:   $sink_line"
[ "$(energy_of "$source_line")" -lt 1000 ] || fail "the charged cell did not give any FE: $source_line"
[ "$(energy_of "$sink_line")" -gt 0 ] || fail "the empty cell took no FE through the cable: $sink_line"

echo "== slice 3: a run of three cables relays as one network =="
# A fresh row at z=5: source cell - three cables - sink cell. The two cells are
# three cable hops apart and touch no cable of their own, so only a network that
# walks its cables can deliver the charge (0.2.2 moved nothing: its cable read
# only the block entities beside one hop).
place 0 100 5 powah:energy_cell_starter
place 1 100 5 powah:energy_cable_starter
place 2 100 5 powah:energy_cable_starter
place 3 100 5 powah:energy_cable_starter
place 4 100 5 powah:energy_cell_starter
# A cell three blocks above the source, reached by no cable: the network gathers
# its ports by walking cables, so this one is not on it and must stay empty.
place 0 103 5 powah:energy_cell_starter
set_floor 0 100 5 100000 1000
ctl wait 40
sink_line=$(info 4 100 5)
source_line=$(info 0 100 5)
# Two chat round trips are ~40 ticks apart, so the sink read can land before the
# relay's first move: poll the sink until the charge shows, then the source (its
# reading is a report, not an assertion — the relay pulls both ways).
for _ in $(seq 1 8); do
    [ "$(energy_of "$sink_line")" -gt 0 ] && break
    ctl wait 10
    sink_line=$(info 4 100 5)
done
for _ in $(seq 1 8); do
    [ "$(energy_of "$source_line")" -lt 100000 ] && break
    ctl wait 10
    source_line=$(info 0 100 5)
done
isolated_line=$(info 0 103 5)
echo "  source: $source_line"
echo "  sink:   $sink_line"
echo "  off the network: $isolated_line"
[ "$(energy_of "$sink_line")" -gt 0 ] || fail "a three-cable network did not relay FE into the cell: $sink_line"
# The source's live reading is not asserted: this relay pulls, so a charged sink
# gives its charge straight back through the same cables and the source reads
# 100000 +- 500 depending on which phase a chat round trip lands in (measured
# 99500 in the 0.2.3 run, 100500 in the next). The conservation check below
# freezes the transfer first, which is where "the charge moved and nothing was
# created" is proved.
[ "$(energy_of "$isolated_line")" = 0 ] || fail "a cell with no cable took FE: $isolated_line"
if [ "$SELFTEST" = network ]; then
    echo "  SELFTEST: expecting the three-cable run to deliver nothing (this must fail)"
    [ "$(energy_of "$sink_line")" = 0 ] || fail "SELFTEST flipped: the three-cable network relayed FE, as it should"
fi
# Break the middle cable so the two cells stop trading, then read them: two
# `info` replies are ~24 ticks apart, and the cells swap energy every tick, so a
# sum of two live reads is not the total. Frozen, it is: the relay must not
# create or destroy a single FE across its run.
ctl command "setblock 2 100 5 minecraft:air" >/dev/null
ctl wait 10
frozen_source=$(info 0 100 5)
frozen_sink=$(info 4 100 5)
frozen=$(( $(energy_of "$frozen_source") + $(energy_of "$frozen_sink") ))
echo "  split: $frozen_source"
echo "  split: $frozen_sink"
[ "$frozen" = 100000 ] || fail "the network's two cells hold $frozen FE after being split, not the 100000 the source was set to: $frozen_source / $frozen_sink"

echo "== slice 4: every energy cell tier holds its own capacity =="
# One command against every tier (`/powah set 2000000000 <pos>`): a cell clamps
# what it is given at its own tier's capacity, so the same command reads back a
# different exact number per tier (`DefaultEnergies`: baseScaling x 1,000,000).
tier_cell() { # X ID CAPACITY TRANSFER
    place "$1" 100 12 "$2"
    set_clamped "$1" 100 12 2000000000 "$3"
    local line expect=$3
    line=$(info "$1" 100 12)
    # SELFTEST=cells flips the basic tier's expectation to the starter's capacity;
    # the readout still says 4,000,000, so the assertion below must fail there.
    if [ "$SELFTEST" = cells ] && [ "$2" = powah:energy_cell_basic ]; then
        expect=1000000
        echo "  SELFTEST: expecting the basic cell to clamp at the starter's $expect (this must fail)"
    fi
    echo "  info($1,100,12): $line"
    case "$line" in
        *"$expect/$expect FE"*) ;;
        *) fail "expected $2 to hold $expect/$expect FE, got '$line'" ;;
    esac
    case "$line" in
        *"$4 FE/t a side"*) ;;
        *) fail "expected $2 to declare $4 FE/t a side, got '$line'" ;;
    esac
}
# The six tiers this slice adds first (the previous package does not know their
# ids at all), then the starter as the contrast it is clamped against.
tier_cell -4 powah:energy_cell_basic 4000000 4000
tier_cell -2 powah:energy_cell_hardened 10000000 10000
tier_cell 0 powah:energy_cell_blazing 40000000 40000
tier_cell 2 powah:energy_cell_niotic 100000000 100000
tier_cell 4 powah:energy_cell_spirited 400000000 400000
tier_cell 6 powah:energy_cell_nitro 2000000000 2000000
tier_cell -6 powah:energy_cell_starter 1000000 1000

echo "== slice 4: two adjacent cells swap and create nothing =="
# A cell gives its own transfer and accepts the neighbour's insert cap, and both
# give and take, so a pair nets zero by design (slice 1's deviation 3; upstream's
# answer is the side-config GUI, which is not ported). What the pair does prove,
# and what can fail: the starter sink never climbs past its own 1,000 insert cap
# beside a full basic cell (a wrong declaration would let it climb at 3,000 a
# tick), and the basic source never loses more than that cap — the pair swaps
# instead of creating or destroying FE.
place 16 100 16 powah:energy_cell_basic
place 16 100 17 powah:energy_cell_starter
say "powah set 4000000 16 100 16"
ctl wait 60
pair_source=$(info 16 100 16)
pair_sink=$(info 16 100 17)
echo "  pair source: $pair_source"
echo "  pair sink:   $pair_sink"
[ "$(energy_of "$pair_sink")" -le 1000 ] || fail "a starter sink beside a full basic cell holds $(energy_of "$pair_sink") FE; its insert cap is its own 1,000 a side, so the pair swaps and nothing nets: $pair_sink"
[ "$(energy_of "$pair_source")" -ge 3998000 ] || fail "the full basic cell lost $(energy_of "$pair_source") FE to an adjacent starter that gives its own 1,000 back: $pair_source"
ctl command "setblock 16 100 16 minecraft:air" >/dev/null
ctl wait 4

echo "== slice 6: every furnator tier holds its own capacity =="
# One command against every tier (`/powah set 2000000000 <pos>`): a furnator
# clamps what it is given at its own tier's generation capacity, so the same
# command reads back a different exact number per tier (`DefaultEnergies`:
# generatorCapacity = baseScaling x 20,000).
tier_furnator() { # X ID CAPACITY TRANSFER
    place "$1" 100 14 "$2"
    set_clamped "$1" 100 14 2000000000 "$3"
    local line expect=$3
    line=$(info "$1" 100 14)
    # SELFTEST=furnator flips the nitro tier's expectation to the starter's
    # 20,000; the readout still says 40,000,000, so the assertion must fail there.
    if [ "$SELFTEST" = furnator ] && [ "$2" = powah:furnator_nitro ]; then
        expect=20000
        echo "  SELFTEST: expecting the nitro furnator to clamp at the starter's $expect (this must fail)"
    fi
    echo "  info($1,100,14): $line"
    case "$line" in
        *"$expect/$expect FE"*) ;;
        *) fail "expected $2 to hold $expect/$expect FE, got '$line'" ;;
    esac
    case "$line" in
        *"$4 FE/t a side"*) ;;
        *) fail "expected $2 to declare $4 FE/t a side, got '$line'" ;;
    esac
}
tier_furnator -6 powah:furnator_starter 20000 80
tier_furnator -4 powah:furnator_basic 80000 320
tier_furnator -2 powah:furnator_hardened 200000 800
tier_furnator 0 powah:furnator_blazing 800000 3200
tier_furnator 2 powah:furnator_niotic 2000000 8000
tier_furnator 4 powah:furnator_spirited 8000000 32000
tier_furnator 6 powah:furnator_nitro 40000000 160000

echo "== slice 6: a nitro furnator makes its own 40,000 FE a tick =="
# Two exact numbers, not a bound: the buffer is set to leave exactly 40,000 of
# room, and one coal is 48,000 carbon. One nitro tick fills that room and takes
# exactly 40,000 off the carbon, so 8,000 is left; a starter-rate tick (20) would
# leave 47,980 and the buffer would still be 20 FE from full.
place 10 100 14 powah:furnator_nitro
set_clamped 10 100 14 39960000 39960000
put_fuel 10 100 14 minecraft:coal 1
ctl wait 20
nitro_line=$(info 10 100 14)
echo "  nitro: $nitro_line"
[ "$(energy_of "$nitro_line")" = 40000000 ] ||
    fail "the nitro furnator did not fill its 40,000 of room: $nitro_line"
nitro_carbon=$(carbon_of "$nitro_line")
if [ "$SELFTEST" = furnator-rate ]; then
    nitro_carbon=47980
    echo "  SELFTEST: expecting the starter rate's 47980 carbon (this must fail)"
fi
[ "$nitro_carbon" = 8000 ] ||
    fail "expected 8000 carbon left after one 40,000 tick of a 48,000 coal, got $nitro_carbon: $nitro_line"

echo "== slice 6: a starter furnator fills its own buffer at its own rate =="
# One coal is 48,000 FE in all: whatever the buffer holds came out of the
# carbon, so energy + carbon = 48,000 exactly, and after a few dozen ticks the
# starter tier has not finished it (the nitro tier had, in one tick).
place 12 100 14 powah:furnator_starter
set_clamped 12 100 14 0 0
put_fuel 12 100 14 minecraft:coal 1
ctl wait 40
starter_line=$(info 12 100 14)
echo "  starter: $starter_line"
starter_energy=$(energy_of "$starter_line")
starter_carbon=$(carbon_of "$starter_line")
[ $((starter_energy + starter_carbon)) = 48000 ] ||
    fail "the starter furnator made FE out of nothing or lost some: energy $starter_energy + carbon $starter_carbon != 48000 ($starter_line)"
[ "$starter_energy" -lt 48000 ] ||
    fail "a starter furnator made a whole coal's 48,000 FE in 40 ticks: $starter_line"
[ "$starter_carbon" -gt 0 ] || fail "the starter furnator is still burning: $starter_line"

echo "== slice 6: every energy cable tier carries its own rate =="
# Same command, seven exact numbers: the basic cable names 2,000 FE/t and the
# starter 500, which is the declaration the relay uses for the whole network.
tier_cable() { # X ID RATE
    place "$1" 100 16 "$2"
    expect_info "$1" 100 16 "no storage, moves up to $3 FE/t"
}
tier_cable -6 powah:energy_cable_starter 500
tier_cable -4 powah:energy_cable_basic 2000
tier_cable -2 powah:energy_cable_hardened 5000
tier_cable 0 powah:energy_cable_blazing 20000
tier_cable 2 powah:energy_cable_niotic 50000
tier_cable 4 powah:energy_cable_spirited 200000
tier_cable 6 powah:energy_cable_nitro 1000000

echo "== slice 6: a cable network joins its own tier and no other =="
# Upstream's `CableNet`: "different tiers never connect". A cell behind a
# starter cable on a basic network's far side is off that network, so it stays
# empty; replace the starter cable with a basic one and the charge crosses.
place 0 100 18 powah:energy_cell_starter
place 1 100 18 powah:energy_cable_basic
place 2 100 18 powah:energy_cable_starter
place 3 100 18 powah:energy_cell_starter
set_floor 0 100 18 100000 1000
ctl wait 60
cross_line=$(info 3 100 18)
echo "  behind the starter cable: $cross_line"
[ "$(energy_of "$cross_line")" = 0 ] ||
    fail "a basic cable network relayed through a starter cable into a cell: $cross_line"
place 2 100 18 powah:energy_cable_basic
for _ in $(seq 1 10); do
    [ "$(energy_of "$cross_line")" -gt 0 ] && break
    ctl wait 10
    cross_line=$(info 3 100 18)
done
echo "  with the basic cable: $cross_line"
[ "$(energy_of "$cross_line")" -gt 0 ] ||
    fail "the two basic cables did not relay into the cell: $cross_line"

echo "== slice 5: the starter rod fills the orb with exactly the recipe's 10,000 FE =="
# An Energizing Orb holds seven slots (0 the result, 1-6 the inputs) and a buffer
# whose capacity is the recipe its inputs match: iron + gold is 10,000 FE into two
# energized steel. The starter rod holds exactly 10,000 and gives 100 FE/t, so the
# run is exact on both sides: the orb reaches its capacity and the rod reaches 0.
place 0 100 20 powah:energizing_orb
place 0 101 20 powah:energizing_rod_starter
orb_clear 0 100 20
orb_put 0 100 20 1 minecraft:iron_ingot 1
orb_put 0 100 20 2 minecraft:gold_ingot 1
expect_info 0 100 20 "0/10000 FE (recipe powah:steel_energized x2, inputs 2"
expect_info 0 101 20 "0/10000 FE (100 FE/t a side, orb 0,100,20"
# The starter rod's capacity is 10,000 (`energizingCapacity`), so `/powah set
# 20000` clamps, and the command says so: the first exact read of this slice's
# own numbers. (The rod drains while a reply lands, so the reply is the assertion.)
set_reply 0 101 20 20000 "Energy set to 10000 FE."
steel=$(orb_wait 0 100 20 powah:steel_energized 2) ||
    fail "the starter rod did not energize iron and gold into 2 energized steel: $steel / $(info 0 100 20) / $(info 0 101 20)"
echo "  output(0,100,20) = $steel"
steel_rod=$(info 0 101 20)
echo "  rod after: $steel_rod"
[ "$(energy_of "$steel_rod")" = 0 ] ||
    fail "the rod held 10,000 FE and gave 100 a tick; it should have given exactly 10,000 and stand at 0: $steel_rod"
expect_info 0 100 20 "0/0 FE (no recipe"
[ "$(orb_slot 0 100 20 1)" = "null" ] || fail "the orb kept its input slot 1: $(orb_slot 0 100 20 1)"
[ "$(orb_slot 0 100 20 2)" = "null" ] || fail "the orb kept its input slot 2: $(orb_slot 0 100 20 2)"

echo "== slice 5: an orb with no rod in range produces nothing =="
place 10 100 20 powah:energizing_orb
orb_clear 10 100 20
orb_put 10 100 20 1 minecraft:iron_ingot 1
orb_put 10 100 20 2 minecraft:gold_ingot 1
ctl wait 60
orb_no_result 10 100 20
expect_info 10 100 20 "0/10000 FE (recipe powah:steel_energized x2, inputs 2"

echo "== slice 5: a rod beside an orb with no recipe keeps its charge =="
# Upstream's `postTick` only fills while the orb `containRecipe()`s; an orb holding
# something no recipe matches is inert, and the rod must not leak into it.
place 20 100 20 powah:energizing_orb
place 20 101 20 powah:energizing_rod_nitro
orb_clear 20 100 20
orb_put 20 100 20 1 minecraft:stick 1
set_clamped 20 101 20 5000 5000
ctl wait 60
expect_info 20 101 20 "5000/20000000 FE"
expect_info 20 100 20 "0/0 FE (no recipe"
orb_no_result 20 100 20

echo "== slice 5: a nitro rod makes a blazing crystal at exactly 120,000 FE =="
# Four blaze powder at 120,000 FE. The nitro rod holds and gives 200,000, so the
# orb takes exactly its recipe's energy out of the rod and the rod is left with
# 80,000 — the assertion a wrong cap or a wrong move would break.
place 0 100 28 powah:energizing_orb
place 0 101 28 powah:energizing_rod_nitro
orb_clear 0 100 28
for slot in 1 2 3 4; do orb_put 0 100 28 "$slot" minecraft:blaze_powder 1; done
expect_info 0 100 28 "0/120000 FE (recipe powah:crystal_blazing x1, inputs 4"
set_reply 0 101 28 200000 "Energy set to 200000 FE."
blazing=$(orb_wait 0 100 28 powah:crystal_blazing 1) ||
    fail "the nitro rod did not energize four blaze powder into a blazing crystal: $blazing / $(info 0 100 28) / $(info 0 101 28)"
echo "  output(0,100,28) = $blazing"
rod_left=80000
if [ "$SELFTEST" = orb ]; then
    # The counterfactual: a run that claims the rod kept its whole charge must FAIL
    # here, so the assertion is alive.
    rod_left=200000
    echo "  SELFTEST: expecting the rod to have kept its whole 200,000 (this must fail)"
fi
expect_info 0 101 28 "$rod_left/20000000 FE"
expect_info 0 100 28 "0/0 FE (no recipe"
[ "$(orb_slot 0 100 28 4)" = "null" ] || fail "the orb kept its fourth input slot: $(orb_slot 0 100 28 4)"

echo "== slice 5: the jar's second blazing recipe, one blaze rod =="
orb_clear 0 100 28
orb_put 0 100 28 1 minecraft:blaze_rod 1
set_reply 0 101 28 200000 "Energy set to 200000 FE."
rod_blaze=$(orb_wait 0 100 28 powah:crystal_blazing 1) ||
    fail "one blaze rod did not energize into a blazing crystal: $rod_blaze / $(info 0 100 28)"
echo "  output(0,100,28) = $rod_blaze"
expect_info 0 101 28 "80000/20000000 FE"

echo "== slice 5: the niotic, spirited and nitro crystals =="
orb_clear 0 100 28
orb_put 0 100 28 1 minecraft:diamond 1
expect_info 0 100 28 "0/300000 FE (recipe powah:crystal_niotic x1, inputs 1"
set_reply 0 101 28 400000 "Energy set to 400000 FE."
niotic=$(orb_wait 0 100 28 powah:crystal_niotic 1) ||
    fail "a diamond did not energize into a niotic crystal: $niotic / $(info 0 100 28)"
echo "  output(0,100,28) = $niotic"
expect_info 0 101 28 "100000/20000000 FE"

orb_clear 0 100 28
orb_put 0 100 28 1 minecraft:emerald 1
set_reply 0 101 28 1000000 "Energy set to 1000000 FE."
spirited=$(orb_wait 0 100 28 powah:crystal_spirited 1) ||
    fail "an emerald did not energize into a spirited crystal: $spirited / $(info 0 100 28)"
echo "  output(0,100,28) = $spirited"
expect_info 0 101 28 "0/20000000 FE"

orb_clear 0 100 28
orb_put 0 100 28 1 minecraft:nether_star 1
orb_put 0 100 28 2 minecraft:redstone_block 1
orb_put 0 100 28 3 minecraft:redstone_block 1
orb_put 0 100 28 4 powah:blazing_crystal_block 1
expect_info 0 100 28 "0/20000000 FE (recipe powah:crystal_nitro x16, inputs 4"
set_reply 0 101 28 20000000 "Energy set to 20000000 FE."
# The largest recipe: 20,000,000 FE at 200,000 a tick is 100 fills after the
# rod's 20-tick warm-up, so this phase is the one that can time out.
nitro=$(orb_wait 0 100 28 powah:crystal_nitro 16) ||
    fail "the nitro recipe did not produce 16 nitro crystals: $nitro / $(info 0 100 28) / $(info 0 101 28)"
echo "  output(0,100,28) = $nitro"
expect_info 0 101 28 "0/20000000 FE"

ctl command "fill 28 99 30 44 99 44 minecraft:stone" >/dev/null
ctl command "fill 28 100 30 44 110 44 minecraft:air" >/dev/null
ctl wait 4
echo "== slice 7: a broken machine's own item carries its buffer =="
# Upstream's `keepEnergy` (true for every furnator, cell and rod) makes
# `playerDestroy` write the buffer onto the block item, and placement reads it
# back. 5,000 on a 20,000 FE furnator, broken in survival and placed again, must
# read 5,000 — a package without the drop hook reads 0 (0.2.6 did).
ctl command "tp @s 32.5 100 35.5" >/dev/null
tool_up
floor_under 32 36
place 32 100 36 powah:furnator_starter
set_clamped 32 100 36 5000 5000
ctl look-at-block 32 100 36 >/dev/null || true
keep_broken=$(ctl break 32 100 36) || fail "cannot break the furnator at 32,100,36"
echo "$keep_broken" | jq -c '{scenario:"keep-energy", block, drops, picked_up}'
[ "$(jq -r '.block' <<<"$keep_broken")" = "minecraft:air" ] || fail "the furnator did not break"
pickup powah:furnator_starter 32 100 36
ctl wait 10
held=$(ctl command "data get entity @s SelectedItem")
echo "  SelectedItem: $held"
case "$held" in
    *block_entity_data*) ;;
    *) fail "the dropped furnator item carries no block entity data: $held" ;;
esac
case "$held" in
    *5000*) ;;
    *) fail "the dropped furnator item does not carry the 5000 FE it held: $held" ;;
esac
place_item 32 100 36 powah:furnator_starter
keep_placed=$(info 32 100 36)
echo "  placed again: $keep_placed"
keep_energy_data=$(ctl command "data get block 32 100 36 energy")
echo "  the block's own saved energy: $(head -c 160 <<<"$keep_energy_data")"
case "$keep_energy_data" in
    *5000*) ;;
    *) fail "the placed machine's saved data does not hold 5000 FE: $keep_energy_data" ;;
esac
keep_energy=5000
if [ "$SELFTEST" = keep ]; then
    keep_energy=0
    echo "  SELFTEST: expecting the placed machine to read 0 FE (this must fail)"
fi
[ "$(energy_of "$keep_placed")" = "$keep_energy" ] ||
    fail "a furnator placed from its own drop read $(energy_of "$keep_placed")/20000 FE, not the $keep_energy it was broken with: $keep_placed"

echo "== slice 7: the machine's slots spill, the payload carries none of them =="
# The engine spills a mod container's stacks when the block goes (HANDOVER §9m),
# and the payload is the buffer alone: a stone in the charge slot must land on
# the ground once, and the placed machine must not have it back.
put_charge 32 100 36 minecraft:stone
set_clamped 32 100 36 5000 5000
tool_up
ctl look-at-block 32 100 36 >/dev/null || true
spill_broken=$(ctl break 32 100 36) || fail "cannot break the furnator at 32,100,36"
echo "$spill_broken" | jq -c '{scenario:"keep-energy-spill", block, drops, picked_up}'
pickup minecraft:stone 32 100 36
pickup powah:furnator_starter 32 100 36
place_item 32 100 36 powah:furnator_starter
spill_placed=$(info 32 100 36)
echo "  placed again: $spill_placed"
[ "$(energy_of "$spill_placed")" = 5000 ] ||
    fail "the machine's buffer did not survive beside a spilled stack: $spill_placed"
[ "$(charge_of "$spill_placed")" = "empty" ] ||
    fail "the payload carried the charge slot's stack ('$(charge_of "$spill_placed")'): the engine already spilled it, so it would be duplicated: $spill_placed"

echo "== slice 7: a payload above the tier clamps on placement =="
# The counterfactual: an item doctored to 2,000,000,000 FE places a full 20,000 FE
# furnator, never a 2,000,000,000 one. The engine applies the payload before the
# placement hook, so the clamp is what makes this true.
tool_up
ctl command "give @s powah:furnator_starter[minecraft:block_entity_data={id:\"powah:furnator_starter\",energy:2000000000}]" >/dev/null
ctl wait 4
[ "$(inv_count powah:furnator_starter)" -ge 1 ] || fail "the doctored furnator item did not reach the inventory"
select_item powah:furnator_starter
echo "  doctored item: $(ctl command "data get entity @s SelectedItem" | head -c 300)"
place_item 34 100 36 powah:furnator_starter
huge_line=$(info 34 100 36)
echo "  doctored: $huge_line"
[ "$(energy_of "$huge_line")" = 20000 ] ||
    fail "an item doctored to 2,000,000,000 FE placed a machine holding $(energy_of "$huge_line") FE, not its tier's 20,000: $huge_line"

echo "== slice 7: a plain item places an empty machine, and a cell keeps its own buffer =="
# The differential for the 5,000 above: a fresh item carries no payload, so the
# machine it places is empty — the FE came from the broken machine's item.
ctl command "give @s powah:furnator_starter 1" >/dev/null
ctl wait 4
select_item powah:furnator_starter
place_item 30 100 36 powah:furnator_starter
plain_line=$(info 30 100 36)
echo "  plain item: $plain_line"
[ "$(energy_of "$plain_line")" = 0 ] ||
    fail "a freshly given furnator placed a machine holding $(energy_of "$plain_line") FE: $plain_line"
# Every keepEnergy machine counts, not just the furnator: a starter cell at 4,000.
floor_under 38 36
place 38 100 36 powah:energy_cell_starter
set_clamped 38 100 36 4000 4000
tool_up
ctl command "tp @s 38.5 100 35.5" >/dev/null
ctl wait 4
ctl look-at-block 38 100 36 >/dev/null || true
cell_broken=$(ctl break 38 100 36) || fail "cannot break the cell at 38,100,36"
echo "$cell_broken" | jq -c '{scenario:"keep-energy-cell", block, drops, picked_up}'
pickup powah:energy_cell_starter 38 100 36
place_item 38 100 36 powah:energy_cell_starter
cell_kept=$(info 38 100 36)
echo "  cell placed again: $cell_kept"
[ "$(energy_of "$cell_kept")" = 4000 ] ||
    fail "a cell placed from its own drop read $(energy_of "$cell_kept")/1000000 FE, not the 4000 it was broken with: $cell_kept"

echo "== slice 7: what a level-side break carries (measured) =="
# The hook fires for every break; the payload only exists when the engine still
# holds the block entity's saved data. A `/setblock … destroy` is the level's path
# (no player, no `onDestroyedByPlayer`), which is the asymmetry upstream has: its
# `playerDestroy` is the player's path only. This leg records the outcome.
place 40 100 36 powah:furnator_starter
set_clamped 40 100 36 7000 7000
level_break=$(ctl command "setblock 40 100 36 minecraft:air destroy")
echo "  level-side break: $(head -c 200 <<<"$level_break")"
ctl command "tp @s 40.5 100 35.5" >/dev/null
ctl wait 20
level_count=$(inv_count powah:furnator_starter)
echo "  level-side break put $level_count furnator item(s) in the inventory"
if [ "$level_count" -ge 1 ]; then
    place_item 40 100 36 powah:furnator_starter
    level_line=$(info 40 100 36)
    echo "  level-side item placed: $level_line (measured: a level break carries $(energy_of "$level_line") of the 7000)"
else
    echo "  level-side item: none dropped"
fi
ctl command "gamemode creative @s" >/dev/null
ctl command "clear @s" >/dev/null
ctl wait 4

echo "== slice 7: the furnator charges the item in its charge slot =="
# Upstream's `FurnatorTile` has two slots (0 chargeables, 1 fuel) and its
# `postTick` runs `chargeItems(1)`: each slot's item takes `min(transfer, what is
# left of the buffer, its own room)` out of the machine's own buffer. The arena
# was cleared for this slice and the nearest other machine is two blocks away.
place 32 100 34 powah:furnator_starter
charge_line=$(info 32 100 34)
echo "  furnator: $charge_line"
case "$charge_line" in
    *"charge empty,"*) ;;
    *) fail "an empty charge slot must read 'charge empty', got '$charge_line'" ;;
esac
set_clamped 32 100 34 1000 1000
ctl wait 40
held_line=$(info 32 100 34)
echo "  with no item: $held_line"
[ "$(energy_of "$held_line")" = 1000 ] ||
    fail "an unfuelled furnator with an empty charge slot lost FE: $held_line"

echo "== slice 7: a chargeable item takes exactly the balance below the transfer =="
# The transfer is 80 FE/t, so a step is `min(80, stored, room)`. With the buffer at
# 40 the item takes exactly 40 and the machine is left at 0: both numbers are in
# one readout line, so this is a conservation check too.
# The buffer is emptied before the item goes in: an item inserted beside a charged
# machine charges on the very next tick, so the state this phase asserts is one
# empty machine, one inserted item, then exactly 40 FE.
set_clamped 32 100 34 0 0
put_charge 32 100 34 powah:energy_cell_starter
set_reply 32 100 34 40 "Energy set to 40 FE."
charged_line=$(info 32 100 34)
echo "  charged: $charged_line"
[ "$(charge_of "$charged_line")" = "powah:energy_cell_starter 40/1000000" ] ||
    fail "the item in the charge slot must hold exactly the machine's 40 FE, got '$(charge_of "$charged_line")'"
[ "$(energy_of "$charged_line")" = 0 ] ||
    fail "the machine must have given the item exactly what it had, so stand at 0: $charged_line"

echo "== slice 7: the charge rate is the tier's 80 FE a tick =="
# A tick is far shorter than a chat round trip, so the rate is measured against
# the game's own clock: between two readouts `dt` ticks apart the item must gain
# exactly `80 * dt` FE (one tick of slack for the read's own boundary), and the
# machine must lose exactly what the item gained. A 40-a-tick charge would need
# half the ticks and a 160-a-tick one twice, so both fail.
set_reply 32 100 34 20000 "Energy set to 20000 FE."
rate_first=$(info 32 100 34)
rate_second=$(info 32 100 34)
echo "  rate: $rate_first"
echo "  rate: $rate_second"
t1=$(time_of "$rate_first")
t2=$(time_of "$rate_second")
c1=$(charge_energy_of "$rate_first")
c2=$(charge_energy_of "$rate_second")
e1=$(energy_of "$rate_first")
e2=$(energy_of "$rate_second")
dt=$((t2 - t1))
dc=$((c2 - c1))
de=$((e1 - e2))
rate=80
if [ "$SELFTEST" = charge ]; then
    rate=40
    echo "  SELFTEST: expecting the charge rate to be 40 FE a tick (this must fail)"
fi
echo "  dt=$dt ticks, item +$dc FE, machine -$de FE"
[ "$dt" -ge 2 ] || fail "the two readouts were $dt ticks apart, too close to measure a rate"
[ "$de" = "$dc" ] ||
    fail "the item gained $dc FE but the machine lost $de: a charge must come out of the buffer"
[ $((dc % rate)) = 0 ] ||
    fail "the item gained $dc FE, not a whole number of $rate FE charge steps"
steps=$((dc / rate))
[ "$steps" -ge $((dt - 1)) ] && [ "$steps" -le $((dt + 1)) ] ||
    fail "the item gained $dc FE over $dt ticks; at $rate FE a tick that is not one charge step a tick (expected $((dt * rate)) +- $rate)"

echo "== slice 7: an item with no buffer is not charged =="
# Upstream gates charging on the item's own `Capabilities.EnergyStorage.ITEM`, so
# a stone in the charge slot takes nothing (a station that charged a stone drained
# its buffer for nothing — the measured `charginggadgets` defect).
ctl command "item replace block 32 100 34 container.0 with minecraft:stone 1" >/dev/null
ctl wait 3
set_clamped 32 100 34 1000 1000
ctl wait 40
stone_line=$(info 32 100 34)
echo "  stone: $stone_line"
[ "$(energy_of "$stone_line")" = 1000 ] ||
    fail "a stone in the charge slot drained the machine: $stone_line"
[ "$(charge_of "$stone_line")" = "minecraft:stone 0/0" ] ||
    fail "a stone carries no buffer, so the slot must read 'minecraft:stone 0/0': $stone_line"

echo "== slice 7: the item's own room caps the step, not the transfer =="
# The cell item's buffer is its tier's 1,000,000. Left exactly 80 of room, one
# 80-FE transfer fills it and the machine keeps the other 920 — stable across two
# reads, so a machine that kept charging a full item is caught.
set_clamped 32 100 34 0 0
put_charge 32 100 34 powah:energy_cell_starter
item_energy 32 100 34 999920 powah:energy_cell_starter
set_reply 32 100 34 1000 "Energy set to 1000 FE."
full_line=$(info 32 100 34)
ctl wait 20
full_line_again=$(info 32 100 34)
echo "  full item: $full_line"
echo "  full item: $full_line_again"
[ "$(charge_of "$full_line")" = "powah:energy_cell_starter 1000000/1000000" ] ||
    fail "the item's 80 of room must be filled to its 1,000,000 cap, got '$(charge_of "$full_line")'"
[ "$(energy_of "$full_line")" = 920 ] ||
    fail "the machine must keep the 920 the full item could not take: $full_line"
[ "$(energy_of "$full_line_again")" = 920 ] ||
    fail "the machine kept charging a full item: $full_line_again"

echo "== slice 7: a cell charges two item slots within one budget =="
# `EnergyCellTile.postTick` runs `chargeItems(2)`: two slots, each taking at most
# the cell's 1,000 transfer, out of one buffer in slot order. With 1,500 FE the
# first slot takes 1,000 and the second the remaining 500 — the split a per-slot
# budget (1,000 each, 2,000 total) would get wrong.
place 36 100 34 powah:energy_cell_starter
put_charge 36 100 34 powah:energy_cell_starter
ctl command "item replace block 36 100 34 container.1 with powah:energy_cell_starter 1" >/dev/null
ctl wait 3
set_reply 36 100 34 1500 "Energy set to 1500 FE."
cell_charge=$(info 36 100 34)
echo "  cell: $cell_charge"
[ "$(energy_of "$cell_charge")" = 0 ] ||
    fail "the cell must have given both items exactly its 1,500 FE: $cell_charge"
case "$cell_charge" in
    *"charge powah:energy_cell_starter 1000/1000000, powah:energy_cell_starter 500/1000000,"*) ;;
    *) fail "a cell must charge slot 0 its full 1,000 and slot 1 the remaining 500, got '$cell_charge'" ;;
esac

echo "== slice 8: a discharger drains the item in its slot into its buffer =="
# `EnergyDischargerTile.postTick` is `chargeItems`' mirror:
# `dischargeItemsInInventory(inv, transfer, capacity - stored)` and then
# `extractFromSides`. A starter discharger holds 1,000,000 FE (`devices.dischargers` is
# the battery config), takes nothing through any face (`Transfer.EXTRACT`), and drains
# 1,000 FE a tick out of each of its seven slots. A cell item charged to 5,000 must end
# fully in the buffer and read 0 on the item.
place 30 100 40 powah:energy_discharger_starter
dis_line=$(info 30 100 40)
echo "  discharger: $dis_line"
case "$dis_line" in
    *"Energy Discharger at 30,100,40: 0/1000000 FE (comparator 0, charge empty, empty, empty, empty, empty, empty, empty,"*)
        ;;
    *) fail "a fresh discharger must read an empty 1,000,000 FE buffer and seven empty slots, got '$dis_line'" ;;
esac
set_clamped 30 100 40 0 0
put_charge 30 100 40 powah:energy_cell_starter
item_energy 30 100 40 5000 powah:energy_cell_starter
# The drain starts the tick the item is written, so poll until the item is empty (a
# chat round trip is ~40 ticks and the transfer is 1,000 a tick).
drain_line=$dis_line
for _ in $(seq 1 20); do
    drain_line=$(info 30 100 40)
    [ "$(charge_energy_of "$drain_line")" = 0 ] && break
    ctl wait 5
done
echo "  drained: $drain_line"
drain_left=0
if [ "$SELFTEST" = drain ]; then
    drain_left=5000
    echo "  SELFTEST: expecting the item to keep its 5,000 (this must fail)"
fi
[ "$(energy_of "$drain_line")" = 5000 ] ||
    fail "the 5,000 FE of the item must be in the discharger's buffer, got $(energy_of "$drain_line"): $drain_line"
[ "$(charge_energy_of "$drain_line")" = "$drain_left" ] ||
    fail "the discharger must have taken the item's 5,000 (left: $drain_left), got '$(charge_of "$drain_line")': $drain_line"

echo "== slice 8: the buffer's room caps one step, not the transfer =="
# `dischargeItemsInInventory(inv, transfer, capacity - stored)`: with 20 of room the
# per-slot cap is `min(transfer 1,000, room 20)` = 20, so the item loses exactly 20 of
# its 5,000 and keeps 4,980 — a drain that ignored the room would take 1,000.
set_clamped 30 100 40 999980 999980
item_energy 30 100 40 5000 powah:energy_cell_starter
room_line=$(info 30 100 40)
ctl wait 20
room_again=$(info 30 100 40)
echo "  room: $room_line"
echo "  room: $room_again"
[ "$(energy_of "$room_line")" = 1000000 ] ||
    fail "the discharger's 20 of room must be filled to its 1,000,000 cap: $room_line"
[ "$(charge_energy_of "$room_line")" = 4980 ] ||
    fail "an item in a buffer with 20 of room must lose exactly 20 of its 5,000, not the 1,000 transfer: $room_line"
[ "$(charge_energy_of "$room_again")" = 4980 ] ||
    fail "a full buffer must stop the drain: $room_again"

echo "== slice 8: the drain rate is the tier's 1,000 FE a tick =="
# Measured against the game's own clock, as the charge rate is: between two readouts
# `dt` ticks apart the buffer must gain exactly `1,000 * dt` FE and the item must lose
# the same. A per-slot budget of the whole buffer, or a 6,000-a-tick drain, fails.
# The slot is emptied first: an item beside a buffer with room drains on the very next
# tick, so the buffer cannot be zeroed (and polled for) while one sits there.
ctl command "item replace block 30 100 40 container.0 with minecraft:air" >/dev/null
ctl wait 3
set_clamped 30 100 40 0 0
ctl command "item replace block 30 100 40 container.0 with powah:energy_cell_starter 1" >/dev/null
ctl wait 3
item_energy 30 100 40 1000000 powah:energy_cell_starter
rate_first=$(info 30 100 40)
rate_second=$(info 30 100 40)
echo "  rate: $rate_first"
echo "  rate: $rate_second"
t1=$(time_of "$rate_first")
t2=$(time_of "$rate_second")
b1=$(energy_of "$rate_first")
b2=$(energy_of "$rate_second")
c1=$(charge_energy_of "$rate_first")
c2=$(charge_energy_of "$rate_second")
dt=$((t2 - t1))
db=$((b2 - b1))
dc=$((c1 - c2))
echo "  dt=$dt ticks, buffer +$db FE, item -$dc FE"
[ "$dt" -ge 2 ] || fail "the two readouts were $dt ticks apart, too close to measure a rate"
[ "$db" = "$dc" ] ||
    fail "the buffer gained $db FE but the item lost $dc: a drain must be exactly what left the item"
[ $((db % 1000)) = 0 ] ||
    fail "the buffer gained $db FE, not a whole number of 1,000 FE drain steps"
steps=$((db / 1000))
[ "$steps" -ge $((dt - 1)) ] && [ "$steps" -le $((dt + 1)) ] ||
    fail "the buffer gained $db FE over $dt ticks; at 1,000 FE a tick that is not one drain step a tick (expected $((dt * 1000)) +- 1000)"

echo "== slice 8: an item with no buffer is not drained =="
# The counterfactual for the two phases above: a stone has no energy capability, so it
# gives nothing and the machine keeps its buffer (an item with no handler must not be
# treated as an empty battery).
ctl command "item replace block 30 100 40 container.0 with minecraft:stone 1" >/dev/null
ctl wait 3
set_clamped 30 100 40 0 0
ctl wait 20
stone_line=$(info 30 100 40)
echo "  stone: $stone_line"
[ "$(energy_of "$stone_line")" = 0 ] ||
    fail "a stone in the discharger's slot filled its buffer: $stone_line"
[ "$(charge_of "$stone_line")" = "minecraft:stone 0/0" ] ||
    fail "a stone carries no buffer, so the slot must read 'minecraft:stone 0/0': $stone_line"

echo "== slice 8: the discharger ladder, its tags and its kept buffer =="
# The tier's own numbers: a nitro discharger clamps `/powah set` at 2,000,000,000 FE and
# names its 2,000,000 FE a side. The block tag is read by the game's own tag matcher.
place 34 100 40 powah:energy_discharger_nitro
set_clamped 34 100 40 2000000000 2000000000
expect_info 34 100 40 "2000000000/2000000000 FE"
expect_info 34 100 40 "2000000 FE/t a side"
tag_hit=$(ctl command "execute if block 30 100 40 #powah:energy_dischargers")
tag_miss=$(ctl command "execute if block 0 99 3 #powah:energy_dischargers")
echo "  tag(30,100,40) = $tag_hit"
echo "  tag(0,99,3)    = $tag_miss"
case "$tag_hit" in
    *"passed"*) ;;
    *) fail "a discharger is not in powah:energy_dischargers: $tag_hit" ;;
esac
case "$tag_miss" in
    *"failed"*) ;;
    *) fail "the arena floor matched powah:energy_dischargers: $tag_miss" ;;
esac
# `keepEnergy()` is true for a discharger too: 7,000 FE broken in survival and placed
# again reads 7,000 (the shared payload code, proved on this family).
floor_under 36 40
place 36 100 40 powah:energy_discharger_starter
set_clamped 36 100 40 7000 7000
ctl command "tp @s 36.5 100 39.5" >/dev/null
tool_up
ctl look-at-block 36 100 40 >/dev/null || true
dis_broken=$(ctl break 36 100 40) || fail "cannot break the discharger at 36,100,40"
echo "$dis_broken" | jq -c '{scenario:"discharger-keep-energy", block, drops, picked_up}'
pickup powah:energy_discharger_starter 36 100 40
place_item 36 100 40 powah:energy_discharger_starter
dis_kept=$(info 36 100 40)
echo "  discharger placed again: $dis_kept"
[ "$(energy_of "$dis_kept")" = 7000 ] ||
    fail "a discharger placed from its own drop read $(energy_of "$dis_kept")/1000000 FE, not the 7000 it was broken with: $dis_kept"
ctl command "gamemode creative @s" >/dev/null
ctl command "clear @s" >/dev/null
ctl wait 4

echo "== slice 8: a discharger takes no FE through any face =="
# `EnergyDischargerBlock.getTransferType()` is `Transfer.EXTRACT`, so the mirror of the
# cell: a charged cell beside it keeps its 50,000 and the discharger's buffer stays 0.
place 40 100 40 powah:energy_discharger_starter
place 41 100 40 powah:energy_cell_starter
set_clamped 41 100 40 50000 50000
ctl wait 40
feed_line=$(info 40 100 40)
feed_source=$(info 41 100 40)
echo "  discharger: $feed_line"
echo "  cell:       $feed_source"
[ "$(energy_of "$feed_line")" = 0 ] ||
    fail "a discharger took FE through a face: $feed_line"
[ "$(energy_of "$feed_source")" = 50000 ] ||
    fail "the cell beside the discharger lost FE: $feed_source"

echo "== slice 9: a hopper charges the container on the face its facing names =="
# `EnergyHopperTile.postTick`: `side = state.facing`, the block entity there must be a
# `Container`, and `ChargeUtil.chargeItemsInContainer(container, charging_rates.get(variant),
# energy.getStored())` spends the tier's rate on each item out of the hopper's own buffer. A starter
# hopper stores 1,000,000 FE (`devices.hoppers` is the battery config) and charges 500 FE a tick
# (`chargingTransfer`). It has no slots of its own: the item lives in the chest it faces.
place 6 100 20 minecraft:chest
ctl command "setblock 6 100 19 powah:energy_hopper_starter[facing=south]" >/dev/null
ctl wait 4
hopper_at=$(ctl block 6 100 19 | jq -r '.name // .block // empty')
echo "  block(6,100,19) = $hopper_at"
[ "$hopper_at" = powah:energy_hopper_starter ] ||
    fail "expected powah:energy_hopper_starter at 6,100,19, got '$hopper_at'"
hopper_line=$(info 6 100 19)
echo "  hopper: $hopper_line"
case "$hopper_line" in
    *"Energy Hopper at 6,100,19: 0/1000000 FE (comparator 0, facing south, charging 500 FE/t, 1000 FE/t a side,"*)
        ;;
    *) fail "a fresh hopper must read an empty 1,000,000 FE buffer facing south at 500 FE/t charging, got '$hopper_line'" ;;
esac
# The buffer is set while the chest is empty: a hopper with an item to charge spends on the tick
# after the write, so an equality poll could never settle (measured: it read 4000). Setting it first
# is also what makes 5,000 a whole — the item starts at 0 and takes every FE the hopper loses.
set_energy 6 100 19 5000
ctl command "item replace block 6 100 20 container.0 with powah:energy_cell_starter 1" >/dev/null
# 5,000 at 500 FE a tick is ten charges, so the hopper empties and the item holds the whole of it:
# `500 x dt` against the hopper's own `t=` clock, with the hopper at `5000 - 500 x dt`.
spent_line=$(info 6 100 19)
for _ in $(seq 1 30); do
    [ "$(energy_of "$spent_line")" = 0 ] && break
    ctl wait 10
    spent_line=$(info 6 100 19)
done
echo "  spent: $spent_line"
[ "$(energy_of "$spent_line")" = 0 ] ||
    fail "the hopper must have spent its whole 5,000 on the item in the chest: $spent_line"
hopper_item_energy=$(chest_energy 6 100 20 0)
echo "  hopper at 0, the chest's item at ${hopper_item_energy:-none} FE: 5000 - 500 x 10 = 0"
[ -n "$hopper_item_energy" ] ||
    fail "the cell item in the chest carries no energy payload, though the hopper spent its 5,000"
want_item=$hopper_item_energy
if [ "$SELFTEST" = hopper ]; then
    want_item=0
    echo "  SELFTEST: expecting the chest's item at 0 (this must fail)"
fi
[ "$hopper_item_energy" = "$want_item" ] ||
    fail "the chest's item must read $want_item FE, got $hopper_item_energy"
[ "$hopper_item_energy" = 5000 ] ||
    fail "the hopper's whole 5,000 must be on the item, not $hopper_item_energy"
[ $((hopper_item_energy % 500)) = 0 ] ||
    fail "the chest's item holds $hopper_item_energy FE, not a whole number of 500 FE steps"

echo "== slice 9: the charging rate is the tier's 500 FE a tick =="
# Measured against the game's own clock: a bigger buffer (100,000 FE is 200 charges) so one chat
# round trip is a fraction of it, and two readouts `dt` ticks apart must differ by exactly
# 500 x dt. The item's own charge is the same number: what the hopper lost is what the item gained.
# The charged item is cleared first, because a hopper with anything to charge spends on the tick
# after the write and a 100,000 equality poll would never settle (measured: it read 99500).
ctl command "item replace block 6 100 20 container.0 with minecraft:air" >/dev/null
ctl wait 3
set_energy 6 100 19 100000
ctl command "item replace block 6 100 20 container.0 with powah:energy_cell_starter 1" >/dev/null
ctl wait 3
rate_first=$(info 6 100 19)
rate_second=$(info 6 100 19)
echo "  rate: $rate_first"
echo "  rate: $rate_second"
t1=$(time_of "$rate_first")
t2=$(time_of "$rate_second")
b1=$(energy_of "$rate_first")
b2=$(energy_of "$rate_second")
dt=$((t2 - t1))
db=$((b1 - b2))
echo "  dt=$dt ticks, hopper -$db FE"
[ "$dt" -ge 2 ] || fail "the two readouts were $dt ticks apart, too close to measure a rate"
[ $((db % 500)) = 0 ] ||
    fail "the hopper spent $db FE over $dt ticks, not a whole number of 500 FE charging steps"
steps=$((db / 500))
[ "$steps" -ge $((dt - 1)) ] && [ "$steps" -le $((dt + 1)) ] ||
    fail "the hopper spent $db FE over $dt ticks; at 500 FE a tick that is not one charge a tick (expected $((dt * 500)) +- 500)"
rate_energy=$(chest_energy 6 100 20 0)
rate_stopped=$(info 6 100 19)
rate_left=$(energy_of "$rate_stopped")
echo "  item at ${rate_energy:-none}, hopper at $rate_left of the 100,000 it held"
[ -n "$rate_energy" ] || fail "the cell item took nothing from the hopper"
[ $((rate_energy % 500)) = 0 ] ||
    fail "the item holds $rate_energy FE, not a whole number of 500 FE steps"
[ $((rate_energy > 500)) = 1 ] ||
    fail "a 100,000 FE buffer at 500 a tick must have charged the item far past $rate_energy FE"
# The item's charge and the hopper's balance are read a round trip apart, so the whole cannot be
# asserted at one instant: the window between the two readouts (the `t=` of `rate_second` and of
# `rate_stopped`) is what may have moved in between, and that is the tolerance — 500 FE a tick of
# it. An item that never took the FE, or a hopper that spent it elsewhere, is thousands short of
# any such window (measured 2026-11-23: 5 ticks of latency on a full box against the old fixed
# 1,500, i.e. 2,500 unaccounted; the window that run was 7 ticks).
window_span=$(( $(time_of "$rate_stopped") - t2 ))
[ "$window_span" -ge 0 ] || window_span=0
window=$((500 * window_span))
unaccounted=$((100000 - rate_energy - rate_left))
echo "  unaccounted: $unaccounted FE (item + hopper against the 100,000, within 500 x a $window_span-tick window = $window)"
[ "$unaccounted" -ge $((-window)) ] && [ "$unaccounted" -le "$window" ] ||
    fail "the item's $rate_energy FE and the hopper's $rate_left FE do not add up to the 100,000 it held (off by $unaccounted, beyond the $window FE that could have moved in $window_span ticks): $rate_stopped"

echo "== slice 9: a hopper facing away leaves the container alone =="
# The counterfactual for the charging above: the same hopper, the same chest, the same 5,000 — with
# `facing=north`, so the block it reads is not the chest. Nothing is charged and nothing is spent.
place 8 100 20 minecraft:chest
ctl command "setblock 8 100 19 powah:energy_hopper_starter[facing=north]" >/dev/null
ctl wait 4
set_energy 8 100 19 5000
ctl command "item replace block 8 100 20 container.0 with powah:energy_cell_starter 1" >/dev/null
ctl wait 40
away_line=$(info 8 100 19)
echo "  away: $away_line"
[ "$(energy_of "$away_line")" = 5000 ] ||
    fail "a hopper facing north, away from the chest beside it, spent FE: $away_line"
away_item=$(chest_energy 8 100 20 0)
echo "  away chest item: ${away_item:-no payload}"
[ -z "$away_item" ] ||
    fail "a hopper facing away charged the chest's item to $away_item FE"

echo "== slice 9: an item with no buffer takes nothing from the hopper =="
# A stone has no energy handler, so `chargeItemsInContainer` asks it for nothing: the buffer stays
# exactly where `/powah set` put it, as the discharger's stone leg is this family's mirror.
# The rate phase's cell item is cleared first for the same reason: an item that takes FE makes an
# equality poll on the buffer impossible.
ctl command "item replace block 6 100 20 container.0 with minecraft:air" >/dev/null
ctl wait 3
set_energy 6 100 19 5000
ctl command "item replace block 6 100 20 container.0 with minecraft:stone 1" >/dev/null
ctl wait 40
stone_line=$(info 6 100 19)
echo "  stone: $stone_line"
[ "$(energy_of "$stone_line")" = 5000 ] ||
    fail "a stone in the chest made the hopper spend FE: $stone_line"

echo "== slice 9: the hopper ladder, its tags and its kept buffer =="
# A nitro hopper clamps `/powah set` at 2,000,000,000 FE, and its own row is the cell's plus a
# 1,000,000 FE/t charging rate (`chargingTransfer` at nitro).
place 10 100 18 powah:energy_hopper_nitro
set_clamped 10 100 18 2000000000 2000000000
expect_info 10 100 18 "2000000000/2000000000 FE"
expect_info 10 100 18 "charging 1000000 FE/t, 2000000 FE/t a side"
hopper_tag=$(ctl command "execute if block 6 100 19 #powah:energy_hoppers")
hopper_tag_miss=$(ctl command "execute if block 0 99 3 #powah:energy_hoppers")
echo "  tag(6,100,19) = $hopper_tag"
echo "  tag(0,99,3)    = $hopper_tag_miss"
case "$hopper_tag" in
    *"passed"*) ;;
    *) fail "a hopper is not in powah:energy_hoppers: $hopper_tag" ;;
esac
case "$hopper_tag_miss" in
    *"failed"*) ;;
    *) fail "the arena floor matched powah:energy_hoppers: $hopper_tag_miss" ;;
esac
# `keepEnergy()` is true for a hopper too: 7,000 FE broken in survival and placed again reads 7,000.
floor_under 12 18
place 12 100 18 powah:energy_hopper_starter
set_clamped 12 100 18 7000 7000
ctl command "tp @s 12.5 100 17.5" >/dev/null
ctl command "gamemode creative @s" >/dev/null
tool_up
ctl look-at-block 12 100 18 >/dev/null || true
hopper_broken=$(ctl break 12 100 18) || fail "cannot break the hopper at 12,100,18"
echo "$hopper_broken" | jq -c '{scenario:"hopper-keep-energy", block, drops, picked_up}'
pickup powah:energy_hopper_starter 12 100 18
place_item 12 100 18 powah:energy_hopper_starter
hopper_kept=$(info 12 100 18)
echo "  hopper placed again: $hopper_kept"
[ "$(energy_of "$hopper_kept")" = 7000 ] ||
    fail "a hopper placed from its own drop read $(energy_of "$hopper_kept")/1000000 FE, not the 7000 it was broken with: $hopper_kept"
ctl command "gamemode creative @s" >/dev/null
ctl command "clear @s" >/dev/null
ctl wait 4

echo "== slice 10: a solar panel generates only with an open sky and daylight =="
# `SolarTile.postTick`: `(canSeeSky || hasLensOfEnder) && dimensionType.hasSkyLight() &&
# getSkyDarken() < 4` makes `min(generation, room)`. `canSeeSky` is
# `Misc.canBlockSeeSky(world, pos.above())`, so the sky is read at the block ABOVE the panel.
# `devices.solar_panels` is a `GeneratorConfig` over `passiveProduction` (20 FE/t at starter) with
# the furnator's capacity (20,000 FE) and per-side transfer (80 FE/t).
#
# The arena is the Overworld at y=100, and the column above each panel is cleared to y=250 so the
# sun really is unobstructed (the sky is read at 14,101,6 and 18,101,6).
place 14 100 6 powah:solar_panel_starter
ctl command "fill 14 101 6 14 250 6 minecraft:air" >/dev/null
ctl command "time set noon" >/dev/null
ctl wait 30
solar_open=$(info 14 100 6)
echo "  solar: $solar_open"
case "$solar_open" in
    *"Solar Panel at 14,100,6: "*" FE (comparator "*", sky open, day true, generating 20 FE/t, 80 FE/t a side, charge empty, t="*)
        ;;
    *) fail "a panel under an open noon sky must read 'sky open, day true, generating 20 FE/t, 80 FE/t a side', got '$solar_open'" ;;
esac
# The rate against the game's own clock: the panel's `t=` field is `level.game_time`, and 20 FE a
# tick is what two readouts `dt` ticks apart must differ by. The buffer is set well below the
# 20,000 it can hold so neither read is at the cap.
say "powah set 10000 14 100 6"
ctl wait 4
rate_first=$(info 14 100 6)
rate_second=$(info 14 100 6)
echo "  rate: $rate_first"
echo "  rate: $rate_second"
t1=$(time_of "$rate_first")
t2=$(time_of "$rate_second")
b1=$(energy_of "$rate_first")
b2=$(energy_of "$rate_second")
dt=$((t2 - t1))
db=$((b2 - b1))
echo "  dt=$dt ticks, panel +$db FE"
if [ "$SELFTEST" = solar ]; then
    echo "  SELFTEST: expecting the open-sky panel to have generated nothing (this must fail)"
    [ "$db" = 0 ] ||
        fail "SELFTEST flipped: the open-sky panel generated $db FE over $dt ticks"
fi
[ "$dt" -ge 2 ] || fail "the two readouts were $dt ticks apart, too close to measure a rate"
[ "$db" -gt 0 ] || fail "an open-sky noon panel generated nothing over $dt ticks: $rate_second"
[ $((db % 20)) = 0 ] ||
    fail "the panel generated $db FE over $dt ticks, not a whole number of 20 FE ticks"
steps=$((db / 20))
[ "$steps" -ge $((dt - 1)) ] && [ "$steps" -le $((dt + 1)) ] ||
    fail "the panel generated $db FE over $dt ticks; at 20 FE a tick that is not one tick a tick (expected $((dt * 20)) +- 20)"
# Exactly 20 of room is filled in one tick and never overflowed.
say "powah set 19980 14 100 6"
for _ in $(seq 1 10); do
    ctl wait 10
    clamped=$(info 14 100 6)
    [ "$(energy_of "$clamped")" = 20000 ] && break
done
echo "  twenty of room: $clamped"
[ "$(energy_of "$clamped")" = 20000 ] ||
    fail "a panel with 20 of room must fill to 20000 FE, got '$clamped'"
ctl wait 20
overflow=$(info 14 100 6)
[ "$(energy_of "$overflow")" = 20000 ] ||
    fail "a full panel must stay at 20000 FE, got '$overflow'"

echo "== slice 10: a stone above the panel stops it, and midnight stops it =="
# The differential for the generation above: the same panel, the same buffer, the same noon — with
# a stone at the position the sky is read from. `set_energy` polls for equality, which settles only
# when nothing is produced.
ctl command "setblock 14 101 6 minecraft:stone" >/dev/null
ctl wait 60
set_energy 14 100 6 5000
covered_first=$(info 14 100 6)
echo "  covered: $covered_first"
case "$covered_first" in
    *"sky covered, day true"*) ;;
    *) fail "a stone above the panel must read 'sky covered, day true', got '$covered_first'" ;;
esac
ctl wait 60
covered_second=$(info 14 100 6)
echo "  covered: $covered_second"
[ "$(energy_of "$covered_first")" = 5000 ] && [ "$(energy_of "$covered_second")" = 5000 ] ||
    fail "a covered panel generated FE: $(energy_of "$covered_first") then $(energy_of "$covered_second") of 5000"
# Midnight: the sky is open again, but `getSkyDarken()` is 11, which fails upstream's `< 4`.
ctl command "setblock 14 101 6 minecraft:air" >/dev/null
ctl command "time set midnight" >/dev/null
ctl wait 40
set_energy 14 100 6 5000
night_first=$(info 14 100 6)
echo "  night: $night_first"
case "$night_first" in
    *"sky open, day false"*) ;;
    *) fail "a panel at midnight must read 'sky open, day false', got '$night_first'" ;;
esac
ctl wait 60
night_second=$(info 14 100 6)
echo "  night: $night_second"
[ "$(energy_of "$night_first")" = 5000 ] && [ "$(energy_of "$night_second")" = 5000 ] ||
    fail "a midnight panel generated FE: $(energy_of "$night_first") then $(energy_of "$night_second") of 5000"
ctl command "time set noon" >/dev/null
ctl wait 10

echo "== slice 10: a solar panel gives on its bottom face only =="
# `SolarTile.isEnergyPresent(side)` is `Direction.DOWN` and nothing else, and its buffer is an
# `AbstractEnergyProvider` that takes no fill: a cell under it charges, a cell beside it stays empty,
# and the cell under it cannot push its FE back up into the panel.
place 20 99 8 powah:energy_cell_starter
place 21 100 8 powah:energy_cell_starter
place 20 100 8 powah:solar_panel_starter
ctl command "fill 20 101 8 20 250 8 minecraft:air" >/dev/null
ctl wait 60
down_line=$(info 20 99 8)
side_line=$(info 21 100 8)
panel_line=$(info 20 100 8)
echo "  cell below:  $down_line"
echo "  cell beside: $side_line"
echo "  panel:       $panel_line"
[ "$(energy_of "$down_line")" -gt 0 ] ||
    fail "the cell under the panel took no FE: $down_line"
[ "$(energy_of "$side_line")" = 0 ] ||
    fail "a cell beside the panel took FE, though `isEnergyPresent` is DOWN only: $side_line"
[ "$(energy_of "$panel_line")" = 0 ] ||
    fail "the panel's buffer took the cell's FE, though it never takes a fill: $panel_line"

echo "== slice 10: the solar ladder, its tags and its clamp =="
place 18 100 6 powah:solar_panel_nitro
ctl command "fill 18 101 6 18 250 6 minecraft:air" >/dev/null
ctl wait 20
# The nitro row: 2,000 FE/t from `passiveProduction`, the furnator's 40,000,000 FE and 160,000 FE a
# side. `/powah set` clamps the panel at its own capacity.
set_clamped 18 100 6 2000000000 40000000
expect_info 18 100 6 "40000000/40000000 FE"
expect_info 18 100 6 "generating 2000 FE/t, 160000 FE/t a side"
solar_tag=$(ctl command "execute if block 18 100 6 #powah:solar_panels")
solar_tag_miss=$(ctl command "execute if block 0 99 3 #powah:solar_panels")
echo "  tag(18,100,6) = $solar_tag"
echo "  tag(0,99,3)   = $solar_tag_miss"
case "$solar_tag" in
    *"passed"*) ;;
    *) fail "a solar panel is not in powah:solar_panels: $solar_tag" ;;
esac
case "$solar_tag_miss" in
    *"failed"*) ;;
    *) fail "the arena floor matched powah:solar_panels: $solar_tag_miss" ;;
esac

echo "== slice 11: an ender cell's channel is one shared store per owner =="
# Upstream's `EnderNetwork` holds one shared `Energy` per (owner, channel) — a `SavedData`, not the
# block's — and `AbstractEnderTile.getEnergy()` hands every block on that channel the same object, so
# two cells on a channel hold, give and show the same FE. A fresh channel's capacity is 0
# (`EnderConfig.getCapacity`), and a **cell** grows it by taking an energy cell item into slot 0
# (`onSlotChanged`/`IEnderExtender`). The starter tier offers one channel and 1,000 FE a call, the
# basic two and 4,000, the nitro twelve and 2,000,000 (`TieredChannelValues`, `batteryTransfer`).
place 30 100 30 powah:ender_cell_basic
place 30 100 34 powah:ender_cell_starter
place 30 100 38 powah:ender_cell_basic
# Slot 0 is the extender: a 4,000,000 FE basic energy cell item gives the channel its capacity.
ctl command "item replace block 30 100 30 container.0 with powah:energy_cell_basic 1" >/dev/null
ctl wait 10
extended=$(info 30 100 30)
echo "  extended: $extended"
case "$extended" in
    *"/4000000 FE (comparator 0, channel 0/2, 4000 FE/t a side"*) ;;
    *) fail "an ender cell holding a basic energy cell item must give its channel that cell's 4,000,000 FE capacity, got '$extended'" ;;
esac
set_energy 30 100 30 100000
shared_a=$(info 30 100 30)
shared_b=$(info 30 100 34)
echo "  channel 0 (basic):   $shared_a"
echo "  channel 0 (starter): $shared_b"
case "$shared_a" in
    *"Ender Cell at 30,100,30: 100000/4000000 FE (comparator 0, channel 0/2, 4000 FE/t a side"*) ;;
    *) fail "the extended basic ender cell must read 100000/4000000 FE on channel 0 of 2, got '$shared_a'" ;;
esac
case "$shared_b" in
    *"Ender Cell at 30,100,34: 100000/4000000 FE (comparator 0, channel 0/1, 1000 FE/t a side"*) ;;
    *) fail "a starter ender cell on the same channel must show the channel's 100000/4000000 FE at its own 1,000 FE/t, got '$shared_b'" ;;
esac

echo "== slice 11: a cell moved to another channel leaves its peer's FE behind =="
# The control: the FE belongs to the channel, not the block. Moving the extended cell to channel 1
# (a basic tier offers 0 and 1) leaves it looking at a fresh, empty channel — 0/0 — while its peer
# on channel 0 keeps exactly the 100,000 it had.
say "powah channel 1 30 100 30"
ctl wait 10
moved=$(info 30 100 30)
peer=$(info 30 100 34)
echo "  moved to channel 1: $moved"
echo "  peer on channel 0:  $peer"
case "$moved" in
    *"Ender Cell at 30,100,30: 0/0 FE (comparator 0, channel 1/2, 4000 FE/t a side"*) ;;
    *) fail "a cell moved to a fresh channel must read 0/0 FE on channel 1/2, got '$moved'" ;;
esac
if [ "$SELFTEST" = ender ]; then
    echo "  SELFTEST: expecting the peer to read 0 (this must fail)"
    [ "$(energy_of "$peer")" = 0 ] ||
        fail "SELFTEST flipped: the peer still reads $(energy_of "$peer") FE after the shared cell moved channel"
fi
[ "$(energy_of "$peer")" = 100000 ] ||
    fail "moving one cell to another channel must leave its peer's channel at exactly 100000 FE, got '$peer'"
# And back: the channel kept the FE while no extended cell was on it.
say "powah channel 0 30 100 30"
ctl wait 10
returned=$(info 30 100 30)
echo "  back on channel 0:  $returned"
[ "$(energy_of "$returned")" = 100000 ] ||
    fail "a channel keeps its FE while no extended cell is on it: expected 100000, got '$returned'"
# A starter cell offers one channel only, which is upstream's GUI count (`getMaxChannels`).
channel_reply 30 100 34 5 "Channel set to 0 of 0..=0."

echo "== slice 11: an ender cell charges its two charge slots out of the channel =="
# `AbstractEnderTile.postTick` runs `chargeItems(1, 3)`: slots 1 and 2, the tier's transfer a slot,
# one budget (the channel) and the item's own per-call limit on top. A starter energy cell item
# takes 1,000 a tick even in a basic ender cell, whose own limit is 4,000.
ctl command "item replace block 30 100 30 container.1 with powah:energy_cell_starter 1" >/dev/null
set_floor 30 100 30 100000 90000
charge_first=$(info 30 100 30)
ctl wait 6
charge_second=$(info 30 100 30)
echo "  charge: $charge_first"
echo "  charge: $charge_second"
ct1=$(time_of "$charge_first")
ct2=$(time_of "$charge_second")
cb1=$(charge_energy_of "$charge_first")
cb2=$(charge_energy_of "$charge_second")
cdt=$((ct2 - ct1))
cgained=$((cb2 - cb1))
clost=$(($(energy_of "$charge_first") - $(energy_of "$charge_second")))
echo "  dt=$cdt ticks, charge item +$cgained FE, channel -$clost FE"
[ "$cdt" -ge 2 ] || fail "the two charge readouts were $cdt ticks apart, too close to measure"
[ "$cgained" -gt 0 ] || fail "the item in the ender cell's charge slot took nothing"
[ "$cgained" = "$clost" ] ||
    fail "the channel lost $clost FE while the item took $cgained: the two must match exactly"
[ $((cgained % 1000)) = 0 ] ||
    fail "the item took $cgained FE over $cdt ticks, not a whole number of 1,000 FE steps"
csteps=$((cgained / 1000))
[ "$csteps" -ge $((cdt - 1)) ] && [ "$csteps" -le $((cdt + 1)) ] ||
    fail "the item took $cgained FE over $cdt ticks; at its own 1,000 a tick that is not one step a tick"
ctl command "item replace block 30 100 30 container.1 with minecraft:air" >/dev/null
ctl wait 4

echo "== slice 11: a channel gives its tier's transfer a tick to every face =="
# The cell answers all six faces and pushes out at its tier's rate (`batteryTransfer`, 4,000 at
# basic) out of the shared channel — so the receiver's gain and the channel's loss are the same FE.
# The sink is a **basic** hopper: its own `max_insert` is 4,000, so the basic ender cell's 4,000 a
# tick is what moves (a starter hopper's 1,000 insert limit would cap it first, which is the two
# caps the engine applies — measured at 1,000 on that sink).
place 31 100 30 powah:energy_hopper_basic
# A 1,000,000 FE channel: at 4,000 a tick that is 250 ticks of headroom, and the hopper's own
# 4,000,000 FE buffer has room for what it takes while two round trips are measured.
set_floor 30 100 30 1000000 900000
rate_first=$(info 31 100 30)
chan_first=$(info 30 100 30)
ctl wait 6
rate_second=$(info 31 100 30)
chan_second=$(info 30 100 30)
echo "  sink:  $rate_first"
echo "  sink:  $rate_second"
echo "  cell:  $chan_first"
echo "  cell:  $chan_second"
# Each end is measured against its own `t=` clock: the two readouts of one block are a round trip
# apart, so the sink's interval and the channel's are compared as rates, not as raw sums.
rt1=$(time_of "$rate_first")
rt2=$(time_of "$rate_second")
rdt=$((rt2 - rt1))
ct1=$(time_of "$chan_first")
ct2=$(time_of "$chan_second")
cdt=$((ct2 - ct1))
rgained=$(($(energy_of "$rate_second") - $(energy_of "$rate_first")))
rlost=$(($(energy_of "$chan_first") - $(energy_of "$chan_second")))
echo "  sink dt=$rdt ticks (+$rgained FE), channel dt=$cdt ticks (-$rlost FE)"
[ "$rdt" -ge 2 ] && [ "$cdt" -ge 2 ] ||
    fail "the rate readouts were $rdt/$cdt ticks apart, too close to measure"
[ "$rgained" -gt 0 ] || fail "a cell on a funded channel pushed nothing into the hopper beside it"
[ $((rgained % 4000)) = 0 ] ||
    fail "the sink gained $rgained FE over $rdt ticks, not a whole number of 4,000 FE steps"
[ $((rlost % 4000)) = 0 ] ||
    fail "the shared channel lost $rlost FE over $cdt ticks, not a whole number of 4,000 FE steps"
rsteps=$((rgained / 4000))
csteps=$((rlost / 4000))
[ "$rsteps" -ge $((rdt - 1)) ] && [ "$rsteps" -le $((rdt + 1)) ] ||
    fail "the sink gained $rgained FE over $rdt ticks; at the basic tier's 4,000 a tick that is not one step a tick"
[ "$csteps" -ge $((cdt - 1)) ] && [ "$csteps" -le $((cdt + 1)) ] ||
    fail "the shared channel lost $rlost FE over $cdt ticks; at the basic tier's 4,000 a tick that is not one step a tick"
# The two ends are read over overlapping but not identical windows, so their step counts differ by
# at most the difference in the two windows (plus one step of boundary): the same FE moved.
step_gap=$((rsteps - csteps))
[ "$step_gap" -ge 0 ] || step_gap=$((-step_gap))
window_gap=$((rdt - cdt))
[ "$window_gap" -ge 0 ] || window_gap=$((-window_gap))
[ "$step_gap" -le $((window_gap + 1)) ] ||
    fail "the sink gained $rgained FE over $rdt ticks while the shared channel lost $rlost over $cdt: one store, so the same FE a tick"
ctl command "setblock 31 100 30 minecraft:air" >/dev/null
ctl wait 4

echo "== slice 11: an ender gate reaches one face only, and moves at the cable's rate =="
# `EnderGateTile.isEnergyPresent(side)` is the block's `facing` side: the gate answers and pushes
# only there, and its row is `cableTransfer` (500 at starter). The gate is placed facing north (the
# block's default), so the hopper north of it is fed and the one south of it is not.
place 24 100 30 powah:ender_gate_starter
place 24 100 29 powah:energy_hopper_starter
place 24 100 31 powah:energy_hopper_starter
set_floor 30 100 30 100000 90000
ctl wait 20
gate=$(info 24 100 30)
gate_front=$(info 24 100 29)
gate_back=$(info 24 100 31)
echo "  gate:        $gate"
echo "  facing side: $gate_front"
echo "  back side:   $gate_back"
case "$gate" in
    *"Ender Gate at 24,100,30: "*"/4000000 FE (comparator "*", channel 0/1, facing north, 500 FE/t a side"*) ;;
    *) fail "a starter ender gate must name channel 0/1, facing north and 500 FE/t a side, got '$gate'" ;;
esac
[ "$(energy_of "$gate_front")" -gt 0 ] ||
    fail "the hopper on the gate's facing side took no FE: $gate_front"
[ "$(energy_of "$gate_back")" = 0 ] ||
    fail "the hopper behind the gate took FE, though `isEnergyPresent` is its facing side only: $gate_back"
# The gate's own rate, against the game clock: 500 FE a tick at starter.
gate_rate_a=$(info 24 100 29)
ctl wait 6
gate_rate_b=$(info 24 100 29)
gt1=$(time_of "$gate_rate_a")
gt2=$(time_of "$gate_rate_b")
gdt=$((gt2 - gt1))
ggained=$(($(energy_of "$gate_rate_b") - $(energy_of "$gate_rate_a")))
echo "  dt=$gdt ticks, gate's sink +$ggained FE"
[ "$gdt" -ge 2 ] || fail "the two gate rate readouts were $gdt ticks apart, too close to measure"
[ $((ggained % 500)) = 0 ] ||
    fail "the gate's sink gained $ggained FE over $gdt ticks, not a whole number of 500 FE steps"
gsteps=$((ggained / 500))
[ "$gsteps" -ge $((gdt - 1)) ] && [ "$gsteps" -le $((gdt + 1)) ] ||
    fail "the gate's sink gained $ggained FE over $gdt ticks; at the cable row's 500 a tick that is not one step a tick"

echo "== slice 11: the orb makes the ender core both ladders are built from =="
# `data/powah/recipe/energizing/ender_core.json`: an ender eye, a dielectric casing and a tiny
# capacitor for 50,000 FE. The orb's buffer is the matched recipe's own energy, so `/powah set`
# fills it exactly and the next tick produces the core.
place 0 100 36 powah:energizing_orb
orb_clear 0 100 36
orb_put 0 100 36 1 minecraft:ender_eye 1
orb_put 0 100 36 2 powah:dielectric_casing 1
orb_put 0 100 36 3 powah:capacitor_basic_tiny 1
expect_info 0 100 36 "0/50000 FE (recipe powah:ender_core x1, inputs 3"
set_reply 0 100 36 50000 "Energy set to 50000 FE."
core=$(orb_wait 0 100 36 powah:ender_core 1) ||
    fail "the orb did not energize an ender eye, a casing and a tiny capacitor into an ender core: $core / $(info 0 100 36)"
echo "  output(0,100,36) = $core"

echo "== slice 11: the ender ladders, their channels, their tiers and their tags =="
place 14 100 10 powah:ender_cell_nitro
# The nitro tier offers twelve channels (`TieredChannelValues`), so it moves to channel 1 — a fresh
# channel with no gate draining it — and its nitro energy cell item gives that channel 2,000,000,000.
channel_reply 14 100 10 1 "Channel set to 1 of 0..=11."
ctl command "item replace block 14 100 10 container.0 with powah:energy_cell_nitro 1" >/dev/null
ctl wait 10
set_clamped 14 100 10 2000000000 2000000000
expect_info 14 100 10 "2000000000/2000000000 FE"
expect_info 14 100 10 "channel 1/12, 2000000 FE/t a side"
place 14 100 14 powah:ender_gate_nitro
ctl wait 4
expect_info 14 100 14 "channel 0/12, facing north, 1000000 FE/t a side"
no_channel=$(ctl command "powah channel 0 0 99 3")
echo "  channel on the arena floor: $no_channel"
case "$no_channel" in
    *"No ender cell or gate there"*) ;;
    *) fail "a channel control on a block that is not an ender cell or gate must be refused, got '$no_channel'" ;;
esac
cell_tag=$(ctl command "execute if block 30 100 30 #powah:ender_cells")
cell_tag_miss=$(ctl command "execute if block 30 100 30 #powah:ender_gates")
gate_tag=$(ctl command "execute if block 24 100 30 #powah:ender_gates")
floor_tag=$(ctl command "execute if block 0 99 3 #powah:ender_cells")
echo "  tag(30,100,30) #ender_cells = $cell_tag"
echo "  tag(30,100,30) #ender_gates = $cell_tag_miss"
echo "  tag(24,100,30) #ender_gates = $gate_tag"
echo "  tag(0,99,3)    #ender_cells = $floor_tag"
case "$cell_tag" in
    *"passed"*) ;;
    *) fail "an ender cell is not in powah:ender_cells: $cell_tag" ;;
esac
case "$cell_tag_miss" in
    *"failed"*) ;;
    *) fail "an ender cell matched powah:ender_gates: $cell_tag_miss" ;;
esac
case "$gate_tag" in
    *"passed"*) ;;
    *) fail "an ender gate is not in powah:ender_gates: $gate_tag" ;;
esac
case "$floor_tag" in
    *"failed"*) ;;
    *) fail "the arena floor matched powah:ender_cells: $floor_tag" ;;
esac

echo "== slice 12: a player transmitter charges the inventory of the player its card binds =="
# `PlayerTransmitterTile.postTick`: `checkRedstone()`, the card in `inv.getFirst()`, the bound
# player, `card.isMultiDim() || same dimension`, then `ChargeUtil.chargeItemsInPlayerInv(player,
# charging_rates.get(variant), energy.getStored())` and `energy.consume(extracted)`. A starter
# transmitter stores 1,000,000 FE (`devices.player_transmitters` is the battery config) and charges
# 500 FE a tick (`chargingTransfer`).
place 16 100 22 powah:player_transmitter_starter
transmitter_line=$(info 16 100 22)
echo "  transmitter: $transmitter_line"
case "$transmitter_line" in
    *"Player Transmitter at 16,100,22: 0/1000000 FE (comparator 0, bound empty, charging 500 FE/t, 1000 FE/t a side,"*)
        ;;
    *) fail "a fresh transmitter must read an empty 1,000,000 FE buffer with no card, got '$transmitter_line'" ;;
esac
# The card binds by its own `use` (`BindingCardItem.use`): an unbound card takes the clicking
# player's UUID and name into the component `powah:bound_player` (`BoundPlayer.CODEC`).
ctl command "item replace entity @s hotbar.0 with powah:binding_card 1" >/dev/null
ctl select 1 >/dev/null
ctl wait 4
ctl look-at 16 250 22 >/dev/null || true
ctl use-item >/dev/null
ctl wait 6
bound_name=$(card_bound 0)
echo "  bound name after use-item: '${bound_name}'"
[ -n "$bound_name" ] ||
    fail "using an unbound binding card did not write powah:bound_player on it"
# The card goes into the transmitter's one slot (automation's path, slot 0); the hand is emptied so
# the player carries only what the assertion is about.
ctl command "item replace block 16 100 22 container.0 from entity @s weapon.mainhand" >/dev/null
ctl command "item replace entity @s hotbar.0 with minecraft:air" >/dev/null
ctl wait 4
bound_line=$(info 16 100 22)
echo "  transmitter with its card: $bound_line"
case "$bound_line" in
    *"bound ${bound_name},"*) ;;
    *) fail "the transmitter's readout must name the card's bound player '$bound_name', got '$bound_line'" ;;
esac

echo "== slice 12: the transmitter spends its buffer on the bound player's item =="
# The buffer is set before the item arrives: a transmitter with something to charge spends on the
# tick after the write, so an equality poll on its buffer could never settle. 10,000 FE is twenty
# 500 FE steps, so the whole buffer goes onto the cell item and stands at exactly 0.
set_energy 16 100 22 10000
ctl command "item replace entity @s hotbar.0 with powah:energy_cell_starter 1" >/dev/null
spent_line=$(info 16 100 22)
for _ in $(seq 1 20); do
    [ "$(energy_of "$spent_line")" = 0 ] && break
    ctl wait 10
    spent_line=$(info 16 100 22)
done
item_spent=$(inv_energy 0)
echo "  spent: $spent_line (the item holds ${item_spent:-none} FE)"
[ "$(energy_of "$spent_line")" = 0 ] ||
    fail "the transmitter must have spent its whole 10,000 on the item in that player's inventory: $spent_line"
[ -n "${item_spent:-}" ] ||
    fail "the cell item in the player's inventory carries no energy payload, though the transmitter spent its 10,000"
if [ "$SELFTEST" = transmitter ]; then
    echo "  SELFTEST: expecting the item to hold nothing (this must fail)"
    [ "$item_spent" = 0 ] || fail "SELFTEST flipped: the transmitter charged the item to $item_spent FE, as it should"
fi
[ "$item_spent" = 10000 ] ||
    fail "the transmitter's whole 10,000 must be on the item, not $item_spent"
[ $((item_spent % 500)) = 0 ] ||
    fail "the item holds $item_spent FE, not a whole number of 500 FE steps"

echo "== slice 12: the charging rate is the tier's 500 FE a tick =="
# Measured against the game's own clock, with a buffer big enough not to run out: two readouts `dt`
# ticks apart must differ by exactly 500 x dt. The pair is re-read when a busy box lands one of the
# two replies stale (`dt` outside the window the wait asked for), which is a measurement that did
# not happen, not a verdict.
ctl command "item replace entity @s hotbar.0 with minecraft:air" >/dev/null
ctl wait 3
set_energy 16 100 22 100000
ctl command "item replace entity @s hotbar.0 with powah:energy_cell_starter 1" >/dev/null
ctl wait 3
dt=0
for _ in $(seq 1 5); do
    rate_first=$(info 16 100 22)
    item_first=$(inv_energy 0)
    ctl wait 20
    rate_second=$(info 16 100 22)
    item_second=$(inv_energy 0)
    t1=$(time_of "$rate_first")
    t2=$(time_of "$rate_second")
    dt=$((t2 - t1))
    [ "$dt" -ge 10 ] && [ "$dt" -le 60 ] && break
    echo "  dt=$dt is outside the 20-tick window asked for: re-reading"
done
b1=$(energy_of "$rate_first")
b2=$(energy_of "$rate_second")
spent=$((b1 - b2))
echo "  rate: $rate_first"
echo "  rate: $rate_second"
echo "  dt=$dt ticks, transmitter -$spent FE, item ${item_first:-none} -> ${item_second:-none} FE"
[ "$dt" -ge 10 ] && [ "$dt" -le 60 ] ||
    fail "the two readouts were $dt ticks apart, outside the 20-tick window: the game's clock could not be read"
[ $((spent % 500)) = 0 ] ||
    fail "the transmitter spent $spent FE over $dt ticks, not a whole number of 500 FE steps"
[ "$spent" = $((500 * dt)) ] ||
    fail "the transmitter spent $spent FE over $dt ticks; its tier's 500 FE a tick is $((500 * dt))"
[ -n "${item_first:-}" ] && [ -n "${item_second:-}" ] ||
    fail "the cell item in the player's inventory carries no energy payload, though the transmitter charges it"
steps=$(((item_second - item_first) / 500))
echo "  item steps: $steps of 500 FE against a $dt-tick window"
[ $(((item_second - item_first) % 500)) = 0 ] ||
    fail "the item gained $((item_second - item_first)) FE, not a whole number of 500 FE steps"
[ "$steps" -ge $((dt - 2)) ] && [ "$steps" -le $((dt + 2)) ] ||
    fail "the item gained $((item_second - item_first)) FE over a $dt-tick window; at 500 FE a tick that is not one charge a tick"
# Conservation, with the tolerance derived from the measured window (the hopper's finding: a fixed
# tolerance flakes on a busy box): the item and the transmitter are read a round trip apart, so what
# may have moved between the two reads is 500 FE per tick of that window.
rate_last=$(info 16 100 22)
t3=$(time_of "$rate_last")
b3=$(energy_of "$rate_last")
window_span=$((t3 - t2))
[ "$window_span" -ge 0 ] || window_span=0
window=$((500 * window_span))
unaccounted=$((100000 - item_second - b3))
echo "  unaccounted: $unaccounted FE (item + transmitter against the 100,000 it held, within 500 x a $window_span-tick window = $window)"
[ "$unaccounted" -ge $((-window)) ] && [ "$unaccounted" -le "$window" ] ||
    fail "the item's $item_second FE and the transmitter's $b3 FE do not add up to the 100,000 it held (off by $unaccounted, beyond the $window FE that could have moved in $window_span ticks): $rate_last"

echo "== slice 13: the redstone mode's command and its readout =="
# `NextRedstoneModePacket` is what upstream's GUI button sends (`tile.nextRedstoneMode();
# tile.sync()`); the GUI is not ported, so the command is the button's replacement, as
# `/powah channel` is the ender GUI's. The mode saves as `redstone_mode` (an ordinal) and the
# readout names it only when it is not the default IGNORE, which is why every line above printed
# without it.
redstone_reply 16 100 22 off "Redstone mode set to off."
expect_info 16 100 22 "redstone off,"
redstone_reply 16 100 22 on "Redstone mode set to on."
expect_info 16 100 22 "redstone on,"
redstone_reply 16 100 22 ignored "Redstone mode set to ignored."
expect_info 16 100 22 "1000 FE/t a side, t="

echo "== slice 13: IGNORE charges through a powered face, the brief's rig =="
# The rig the brief names: the starter transmitter on 16,100,22 with its bound card in slot 0, a
# cell item in the operator's inventory, and a redstone block on its west face (a redstone block
# powers all six neighbours, which is `getBestNeighborSignal > 0`).
ctl command "item replace entity @s hotbar.0 with minecraft:air" >/dev/null
ctl wait 3
# The buffer is set **before** the item arrives: a transmitter with something to charge spends on
# the next tick, so an equality poll on its buffer could never settle. 200,000 is 400 whole 500 FE
# steps, far more than the four 20-tick windows this phase reads.
set_energy 16 100 22 200000
ctl command "setblock 15 100 22 minecraft:redstone_block" >/dev/null
ctl command "item replace entity @s hotbar.0 with powah:energy_cell_starter 1" >/dev/null
ctl wait 20
ignored_first=$(info 16 100 22)
ignored_item_first=$(inv_energy 0)
ctl wait 20
ignored_second=$(info 16 100 22)
ignored_item_second=$(inv_energy 0)
echo "  ignored: $ignored_first / $ignored_second"
echo "  ignored item: ${ignored_item_first:-none} -> ${ignored_item_second:-none} FE"
[ -n "${ignored_item_first:-}" ] && [ -n "${ignored_item_second:-}" ] ||
    fail "the cell item carries no energy payload, so the charge legs would prove nothing"
[ "$ignored_item_second" -gt "$ignored_item_first" ] ||
    fail "IGNORE with a powered face must keep charging (the block is a lever nobody set), got ${ignored_item_first} -> ${ignored_item_second} FE"

echo "== slice 13: OFF with a powered face stops the charge, and the buffer stands still =="
# `checkRedstone` for OFF is `!powered`: a redstone block on the west face shuts the gate.
redstone_reply 16 100 22 off "Redstone mode set to off."
gated_line=$(info 16 100 22)
echo "  gated: $gated_line"
case "$gated_line" in
    *"redstone off (powered),"*) ;;
    *) fail "the readout must name the shut gate and the power that shuts it, got '$gated_line'" ;;
esac
gated_first=$gated_line
gated_buffer_first=$(energy_of "$gated_first")
gated_item_first=$(inv_energy 0)
ctl wait 20
gated_second=$(info 16 100 22)
gated_item_second=$(inv_energy 0)
echo "  gated over 20 ticks: $gated_second (item ${gated_item_first:-none} -> ${gated_item_second:-none} FE)"
if [ "$SELFTEST" = redstone ]; then
    # The flip: expect the charge the gate must refuse, so a correct run fails here.
    echo "  SELFTEST: expecting the shut gate to have charged (this must fail)"
    [ "$gated_item_second" != "$gated_item_first" ] ||
        fail "SELFTEST flipped: the shut gate charged nothing, as it should"
fi
[ "$gated_item_second" = "$gated_item_first" ] ||
    fail "a shut gate must charge nothing: the item went ${gated_item_first} -> ${gated_item_second} FE"
[ "$(energy_of "$gated_second")" = "$gated_buffer_first" ] ||
    fail "a shut gate must spend nothing: the transmitter went $gated_buffer_first -> $(energy_of "$gated_second") FE"

echo "== slice 13: the same rig with the block removed charges again (500 x dt) =="
# OFF with no power is open: the gate the block shut opens when the block goes.
ctl command "setblock 15 100 22 minecraft:air" >/dev/null
ctl wait 4
open_first=$(info 16 100 22)
open_item_first=$(inv_energy 0)
ctl wait 20
open_second=$(info 16 100 22)
open_item_second=$(inv_energy 0)
open_t1=$(time_of "$open_first")
open_t2=$(time_of "$open_second")
[ -n "$open_t1" ] && [ -n "$open_t2" ] ||
    fail "the two readouts carry no clock, so the rate cannot be measured: '$open_first' / '$open_second'"
open_dt=$((open_t2 - open_t1))
open_spent=$(( $(energy_of "$open_first") - $(energy_of "$open_second") ))
echo "  opened: $open_second over dt=$open_dt, transmitter -$open_spent FE, item ${open_item_first:-none} -> ${open_item_second:-none} FE"
case "$open_second" in
    *"redstone off,"*) ;;
    *) fail "with the block gone the gate must read shut-but-open ('redstone off'), got '$open_second'" ;;
esac
[ "$open_dt" -ge 10 ] && [ "$open_dt" -le 60 ] ||
    fail "the two readouts were $open_dt ticks apart, outside the 20-tick window"
[ "$open_spent" = $((500 * open_dt)) ] ||
    fail "the open gate spent $open_spent FE over $open_dt ticks; its 500 FE a tick is $((500 * open_dt))"
[ "$open_item_second" -gt "$open_item_first" ] ||
    fail "the open gate must charge the item, it stayed at ${open_item_first:-none} FE"

echo "== slice 13: ON needs the power, and both halves of the truth table hold =="
# ON with no neighbour signal is shut; putting the block back opens it.
redstone_reply 16 100 22 on "Redstone mode set to on."
on_line=$(info 16 100 22)
on_item_first=$(inv_energy 0)
ctl wait 20
on_item_second=$(inv_energy 0)
echo "  on, unpowered: $on_line (item ${on_item_first:-none} -> ${on_item_second:-none} FE)"
case "$on_line" in
    *"redstone on,"*) ;;
    *) fail "an unpowered ON gate must read 'redstone on', got '$on_line'" ;;
esac
[ "$on_item_second" = "$on_item_first" ] ||
    fail "ON with no power must charge nothing: ${on_item_first} -> ${on_item_second} FE"
ctl command "setblock 15 100 22 minecraft:redstone_block" >/dev/null
ctl wait 4
on_powered=$(info 16 100 22)
on_powered_item=$(inv_energy 0)
echo "  on, powered: $on_powered (item ${on_powered_item:-none} FE)"
case "$on_powered" in
    *"redstone on (powered),"*) ;;
    *) fail "a powered ON gate must read 'redstone on (powered)', got '$on_powered'" ;;
esac
[ "$on_powered_item" -gt "$on_item_second" ] ||
    fail "ON with power must charge, the item stayed at ${on_item_second} FE"
# Back to the default and clean: the later phases (the blank card, the ladder) act on this machine.
redstone_reply 16 100 22 ignored "Redstone mode set to ignored."
ctl command "setblock 15 100 22 minecraft:air" >/dev/null
ctl command "item replace entity @s hotbar.0 with minecraft:air" >/dev/null
ctl wait 3

echo "== slice 13: the mode is not a blocks-and-items family — it is the data and one rule =="
# A machine that is not one of this mod's has no mode, and the block the mode is set on is the
# machine's own: a foreign block refuses, and so does the orb (upstream's screen has no button).
foreign_reply=$(ctl command "powah redstone off 0 99 46")
echo "  foreign block: $foreign_reply"
case "$foreign_reply" in
    *"No Powah block"*) ;;
    *) fail "a foreign block must refuse a redstone mode, got '$foreign_reply'" ;;
esac
place 0 100 46 powah:energizing_orb
orb_reply=$(ctl command "powah redstone off 0 100 46")
echo "  orb: $orb_reply"
case "$orb_reply" in
    *"no redstone mode"*) ;;
    *) fail "the energizing orb has no redstone mode (neither its screen nor its tile has one), got '$orb_reply'" ;;
esac
ctl command "setblock 0 100 46 minecraft:air" >/dev/null
ctl wait 3

# No `floor_under` in this phase: it never teleports the player, and a machine placed by `setblock`
# at y=100 does not need support.
echo "== slice 13: the shut gate stops the furnator's burn, and the store's faces =="
# `FurnatorTile.postTick` runs the burn inside `checkRedstone()`, so a shut furnator takes no fuel
# and burns no carbon; it still gives what it holds (`extractFromSides` is outside the gate).
place 0 100 48 powah:furnator_starter
redstone_reply 0 100 48 off "Redstone mode set to off."
ctl command "setblock -1 100 48 minecraft:redstone_block" >/dev/null
put_fuel 0 100 48 minecraft:coal 5
ctl wait 20
shut_furnator=$(info 0 100 48)
echo "  gated furnator: $shut_furnator"
case "$shut_furnator" in
    *"carbon 0, burning false, fuel minecraft:coal x5,"*) ;;
    *) fail "a shut furnator must take no fuel and burn nothing, got '$shut_furnator'" ;;
esac
[ "$(energy_of "$shut_furnator")" = 0 ] ||
    fail "a shut furnator must make no FE, got '$shut_furnator'"
ctl command "setblock -1 100 48 minecraft:air" >/dev/null
ctl wait 20
open_furnator=$(info 0 100 48)
echo "  opened furnator: $open_furnator"
[ "$(energy_of "$open_furnator")" -gt 0 ] ||
    fail "the furnator must burn once the gate opens, got '$open_furnator'"
[ "$(carbon_of "$open_furnator")" -gt 0 ] ||
    fail "the furnator must hold carbon once the gate opens, got '$open_furnator'"

echo "== slice 13: a shut cell gives nothing on any face, so the cable beside it pulls nothing =="
# `EnergyCellTile.canExtractEnergy`/`canReceiveEnergy` are `checkRedstone() && super…`: the cell's
# own push and a cable's pull both stop.
place 2 100 48 powah:energy_cell_starter
place 2 100 49 powah:energy_cable_starter
place 2 100 50 powah:energy_cell_starter
redstone_reply 2 100 48 off "Redstone mode set to off."
ctl command "setblock 1 100 48 minecraft:redstone_block" >/dev/null
ctl wait 4
set_energy 2 100 48 5000
ctl wait 20
shut_source=$(info 2 100 48)
shut_sink=$(info 2 100 50)
echo "  gated cell: $shut_source / sink: $shut_sink"
[ "$(energy_of "$shut_source")" = 5000 ] ||
    fail "a shut cell must keep its 5,000 (nothing may pull it), got '$shut_source'"
[ "$(energy_of "$shut_sink")" = 0 ] ||
    fail "the cell behind a gated one must stay empty, got '$shut_sink'"
ctl command "setblock 1 100 48 minecraft:air" >/dev/null
ctl wait 20
open_sink=$(info 2 100 50)
echo "  opened: sink $open_sink"
[ "$(energy_of "$open_sink")" -gt 0 ] ||
    fail "an open cell must feed the cable and the sink, got '$open_sink'"

echo "== slice 13: a shut cable relays nothing while an open one beside it does =="
# `CableTile.receiveEnergy` reads the gate: it is the door FE comes in through.
place 4 100 48 powah:energy_cell_starter
place 4 100 49 powah:energy_cable_starter
place 4 100 50 powah:energy_cell_starter
redstone_reply 4 100 49 off "Redstone mode set to off."
ctl command "setblock 5 100 49 minecraft:redstone_block" >/dev/null
ctl wait 4
set_energy 4 100 48 5000
ctl wait 20
cable_shut_sink=$(info 4 100 50)
echo "  gated cable: sink $cable_shut_sink"
[ "$(energy_of "$cable_shut_sink")" = 0 ] ||
    fail "a cable whose gate is shut must relay nothing, got '$cable_shut_sink'"
ctl command "setblock 5 100 49 minecraft:air" >/dev/null
ctl wait 20
cable_open_sink=$(info 4 100 50)
echo "  opened cable: sink $cable_open_sink"
[ "$(energy_of "$cable_open_sink")" -gt 0 ] ||
    fail "an open cable must relay, got '$cable_open_sink'"

echo "== slice 13: a shut discharger drains no item, and a shut hopper charges no container =="
place 6 100 48 powah:energy_discharger_starter
redstone_reply 6 100 48 off "Redstone mode set to off."
ctl command "setblock 5 100 48 minecraft:redstone_block" >/dev/null
ctl wait 4
put_charge 6 100 48 powah:energy_cell_starter
item_energy 6 100 48 5000 powah:energy_cell_starter
ctl wait 20
shut_discharger=$(info 6 100 48)
echo "  gated discharger: $shut_discharger"
[ "$(energy_of "$shut_discharger")" = 0 ] ||
    fail "a shut discharger must drain nothing, got '$shut_discharger'"
[ "$(charge_energy_of "$shut_discharger")" = 5000 ] ||
    fail "the item in a shut discharger must keep its 5,000, got '$(charge_of "$shut_discharger")'"
ctl command "setblock 5 100 48 minecraft:air" >/dev/null
ctl wait 20
open_discharger=$(info 6 100 48)
echo "  opened discharger: $open_discharger"
[ "$(energy_of "$open_discharger")" -gt 0 ] ||
    fail "an open discharger must drain its item, got '$open_discharger'"

place 8 100 49 minecraft:chest
ctl command "setblock 8 100 48 powah:energy_hopper_starter[facing=south]" >/dev/null
ctl wait 4
redstone_reply 8 100 48 off "Redstone mode set to off."
ctl command "setblock 7 100 48 minecraft:redstone_block" >/dev/null
ctl wait 4
set_energy 8 100 48 5000
ctl command "item replace block 8 100 49 container.0 with powah:energy_cell_starter 1" >/dev/null
ctl wait 20
shut_hopper=$(info 8 100 48)
shut_chest=$(chest_energy 8 100 49 0)
echo "  gated hopper: $shut_hopper (chest item ${shut_chest:-none} FE)"
[ "$(energy_of "$shut_hopper")" = 5000 ] ||
    fail "a shut hopper must spend nothing, got '$shut_hopper'"
[ "${shut_chest:-0}" = 0 ] ||
    fail "a shut hopper must charge nothing, the chest's item holds $shut_chest FE"
ctl command "setblock 7 100 48 minecraft:air" >/dev/null
ctl wait 20
open_hopper=$(info 8 100 48)
echo "  opened hopper: $open_hopper"
[ "$(energy_of "$open_hopper")" -lt 5000 ] ||
    fail "an open hopper must spend its buffer on the chest, got '$open_hopper'"

echo "== slice 13: a shut solar panel generates nothing while the sun is out =="
# `SolarTile.postTick` generates inside `checkRedstone()`; its item charge and its push are outside.
ctl command "time set noon" >/dev/null
# The panel's sky is the block above it (`SolarTile` reads `canBlockSeeSky(pos.above())`): clear the
# column so this leg measures the gate and not an overhang.
ctl command "fill 10 101 48 10 200 48 minecraft:air" >/dev/null
place 10 100 48 powah:solar_panel_starter
redstone_reply 10 100 48 off "Redstone mode set to off."
ctl command "setblock 9 100 48 minecraft:redstone_block" >/dev/null
ctl wait 4
set_energy 10 100 48 5000
ctl wait 40
shut_solar=$(info 10 100 48)
echo "  gated solar: $shut_solar"
case "$shut_solar" in
    *"redstone off (powered),"*) ;;
    *) fail "the solar readout must name its gate, got '$shut_solar'" ;;
esac
[ "$(energy_of "$shut_solar")" = 5000 ] ||
    fail "a shut panel must generate nothing, got '$shut_solar'"
ctl command "setblock 9 100 48 minecraft:air" >/dev/null
ctl wait 40
open_solar=$(info 10 100 48)
echo "  opened solar: $open_solar"
[ "$(energy_of "$open_solar")" -gt 5000 ] ||
    fail "an open panel under a noon sky must generate, got '$open_solar'"

echo "== slice 13: a shut ender cell refuses the cable at its own handler =="
# `AbstractEnderTile.canExtractEnergy`/`canReceiveEnergy` are `checkRedstone() && super…`, and the
# ender handler is this port's own (`EnderEnergy`), so the gate sits inside `insert`/`extract`.
# The channel store is the world's, keyed by (owner, channel), so a starter cell (one channel, 0)
# would share the FE — and the extenders' capacity — with slice 11's cells on channel 0. A nitro
# cell offers twelve, and this leg takes the last one: nothing else in this run touches it.
place 12 100 48 powah:ender_cell_nitro
channel_reply 12 100 48 11 "Channel set to 11 of 0..=11."
put_charge 12 100 48 powah:energy_cell_starter
redstone_reply 12 100 48 off "Redstone mode set to off."
ctl command "setblock 11 100 48 minecraft:redstone_block" >/dev/null
ctl wait 4
set_clamped 12 100 48 5000 5000
place 12 100 49 powah:energy_cable_starter
place 12 100 50 powah:energy_cell_starter
ctl wait 20
shut_ender_sink=$(info 12 100 50)
echo "  gated ender cell: sink $shut_ender_sink"
[ "$(energy_of "$shut_ender_sink")" = 0 ] ||
    fail "a shut ender cell must give the cable nothing, got '$shut_ender_sink'"
ctl command "setblock 11 100 48 minecraft:air" >/dev/null
ctl wait 20
open_ender_sink=$(info 12 100 50)
echo "  opened ender cell: sink $open_ender_sink"
[ "$(energy_of "$open_ender_sink")" -gt 0 ] ||
    fail "an open ender cell must feed the cable, got '$open_ender_sink'"

# The modes are data, not blocks: nothing this slice added is left standing.
for block in "15 100 22" "-1 100 48" "1 100 48" "5 100 49" "5 100 48" "7 100 48" "9 100 48" "11 100 48"; do
    set -- $block
    ctl command "setblock $1 $2 $3 minecraft:air" >/dev/null
done
ctl wait 3

echo "== slice 14: a comparator beside a machine reads the strength its readout prints =="
# `AbstractEnergyBlock.getAnalogOutputSignal` is `Energy.toComparatorPower()` — the stored fraction
# times 15, truncated — which is the number `/powah info` already prints. The hook gets a read-only
# level and cannot reach a block entity, so the port records each machine's strength in its level
# tick's drain (and on `/powah set`) and answers from there; a changed reading tells the comparators
# beside the machine to read it again (`Level.updateNeighbourForOutputSignal`).
#
# At this pin a comparator has no block entity (`comparator_outputs` is the level's own map), so
# `data get block … OutputSignal` cannot read it. Its output is measured where the game itself uses
# it: a redstone wire in front of the comparator (its `facing` side is its input, the opposite side
# its output) carries exactly the comparator's signal as its `power`.
ctl command "fill -6 99 48 4 99 53 minecraft:stone" >/dev/null
ctl wait 3
# block_prop X Y Z NAME: one property of a block state. `ctl block` answers `block` (the name) and
# `state` (`name[prop=value,...]`), not a property object, so the state string is parsed.
block_prop() { # block_prop X Y Z NAME
    ctl block "$1" "$2" "$3" | jq -r --arg s "$4" '
        ((.state // "") | [scan("([a-z_]+)=([a-z0-9_]+)")] | map({key: .[0], value: .[1]}) | from_entries | .[$s]) // ""'
}
cmp_signal() { # cmp_signal X Y Z: the wire's power in front of a comparator at X,Y,Z (facing south)
    block_prop "$1" "$2" "$(($3 - 1))" power
}
cmp_powered() { # cmp_powered X Y Z
    block_prop "$1" "$2" "$3" powered
}
# comparator_rig X Y Z: a comparator on the machine at X,Y,Z (its north face, facing it) and the
# wire that carries its reading. `nudge_comparator` wakes it with a neighbour change; its top face
# is not an input (its sides are the two perpendicular to `facing`), so the nudge is inert.
comparator_rig() { # comparator_rig X Y Z
    ctl command "setblock $1 $2 $(($3 - 1)) minecraft:comparator[facing=south]" >/dev/null
    ctl command "setblock $1 $2 $(($3 - 2)) minecraft:redstone_wire" >/dev/null
    ctl wait 3
    ctl command "setblock $1 $(($2 + 1)) $(($3 - 1)) minecraft:redstone_block" >/dev/null
    ctl wait 3
    ctl command "setblock $1 $(($2 + 1)) $(($3 - 1)) minecraft:air" >/dev/null
    ctl wait 10
    [ "$(ctl block "$1" "$2" "$(($3 - 1))" | jq -r '.name // .block // empty')" = "minecraft:comparator" ] ||
        fail "the comparator did not place at $1,$2,$(($3 - 1))"
    [ "$(ctl block "$1" "$2" "$(($3 - 2))" | jq -r '.name // .block // empty')" = "minecraft:redstone_wire" ] ||
        fail "the wire did not stay in front of the comparator at $1,$2,$(($3 - 2))"
}
# A cell at 500,000 of 1,000,000: the readout prints `comparator 7` (floor of 7.5), and the
# comparator on its north face must read the same number.
place -4 100 52 powah:energy_cell_starter
set_energy -4 100 52 500000
cell_line=$(info -4 100 52)
cell_printed=$(sed -nE 's/.*comparator ([0-9]+),.*/\1/p' <<<"$cell_line")
echo "  readout: $cell_line"
[ "$cell_printed" = 7 ] ||
    fail "500,000 of 1,000,000 is floor(7.5) = comparator 7, got '$cell_line'"
comparator_rig -4 100 52
cell_signal=$(cmp_signal -4 100 51)
cell_powered=$(cmp_powered -4 100 51)
echo "  comparator(-4,100,51) = powered $cell_powered, the wire in front reads $cell_signal, the readout says $cell_printed"
[ "$cell_powered" = true ] ||
    fail "a comparator beside a half-full cell must be powered, got '$cell_powered'"
[ "$cell_signal" = "$cell_printed" ] ||
    fail "the comparator reads $cell_signal but the readout's own comparator is $cell_printed: $cell_line"
if [ "$SELFTEST" = comparator ]; then
    echo "  SELFTEST: expecting the comparator to read something other than $cell_printed (this must fail)"
    [ "$cell_signal" != "$cell_printed" ] ||
        fail "SELFTEST flipped: the comparator read $cell_signal, the readout's own $cell_printed, as it should"
fi

echo "== slice 14: the reading follows the buffer (15 at full, 3 at a quarter) =="
# A buffer that changes without the block's state changing is the case the hook has to announce.
set_energy -4 100 52 1000000
ctl wait 10
full_line=$(info -4 100 52)
full_printed=$(sed -nE 's/.*comparator ([0-9]+),.*/\1/p' <<<"$full_line")
full_signal=$(cmp_signal -4 100 51)
echo "  full: $full_line -> comparator $full_signal"
[ "$full_printed" = 15 ] && [ "$full_signal" = 15 ] ||
    fail "a full cell must read comparator 15 both ways, got readout $full_printed and comparator $full_signal"
set_energy -4 100 52 250000
ctl wait 10
quarter_line=$(info -4 100 52)
quarter_printed=$(sed -nE 's/.*comparator ([0-9]+),.*/\1/p' <<<"$quarter_line")
quarter_signal=$(cmp_signal -4 100 51)
echo "  quarter: $quarter_line -> comparator $quarter_signal"
[ "$quarter_printed" = 3 ] && [ "$quarter_signal" = 3 ] ||
    fail "a quarter-full cell must read comparator 3 both ways, got readout $quarter_printed and comparator $quarter_signal"

echo "== slice 14: a stone in the same rig reads nothing =="
place -2 100 52 minecraft:stone
comparator_rig -2 100 52
stone_signal=$(cmp_signal -2 100 51)
stone_powered=$(cmp_powered -2 100 51)
echo "  stone: wire $stone_signal, powered $stone_powered"
[ "$stone_signal" = 0 ] || fail "a stone must give a comparator 0, got $stone_signal"
[ "$stone_powered" = false ] || fail "a comparator beside a stone must not be powered, got '$stone_powered'"

echo "== slice 14: a cable answers nothing, even with a full container behind it =="
# The control first: the same rig with a chest where the comparator reads it — vanilla's own
# container fullness — so the cable's zero below is its own answer, not a dead rig.
ctl command "setblock 0 100 52 minecraft:chest" >/dev/null
ctl command "item replace block 0 100 52 container.0 with minecraft:diamond 64" >/dev/null
comparator_rig 0 100 52
chest_signal=$(cmp_signal 0 100 51)
chest_powered=$(cmp_powered 0 100 51)
echo "  chest: wire $chest_signal, powered $chest_powered"
[ "$chest_signal" -gt 0 ] ||
    fail "the rig does not read a chest (wire $chest_signal), so the cable's zero would prove nothing"
[ "$chest_powered" = true ] || fail "a comparator beside a chest with an item must be powered"
# The cable where the comparator reads it, with the same full chest behind it.
ctl command "setblock 0 100 52 powah:energy_cable_starter" >/dev/null
ctl command "setblock 0 100 53 minecraft:chest" >/dev/null
ctl command "item replace block 0 100 53 container.0 with minecraft:diamond 64" >/dev/null
ctl wait 3
ctl command "setblock 0 101 51 minecraft:redstone_block" >/dev/null
ctl wait 3
ctl command "setblock 0 101 51 minecraft:air" >/dev/null
ctl wait 10
cable_signal=$(cmp_signal 0 100 51)
cable_powered=$(cmp_powered 0 100 51)
echo "  cable: wire $cable_signal, powered $cable_powered (the full chest sits behind it at 0,100,53)"
[ "$cable_signal" = 0 ] || fail "a cable stores nothing and must answer no comparator signal, got $cable_signal"
[ "$cable_powered" = false ] || fail "a comparator beside a cable must not be powered, got '$cable_powered'"

echo "== slice 14: a full rod answers nothing (its readout prints no comparator) =="
place 2 100 52 powah:energizing_rod_starter
set_energy 2 100 52 10000
rod_line=$(info 2 100 52)
case "$rod_line" in
    *comparator*) fail "a rod's readout prints no comparator, got '$rod_line'" ;;
esac
comparator_rig 2 100 52
rod_signal=$(cmp_signal 2 100 51)
echo "  full rod: $rod_line -> wire $rod_signal"
[ "$rod_signal" = 0 ] ||
    fail "a rod may hold FE but prints no comparator, so it answers none; the comparator reads $rod_signal"

# The readings are data, not blocks: nothing this slice added is left standing.
for block in "-4 100 50" "-4 100 51" "-4 100 52" "-2 100 50" "-2 100 51" "-2 100 52" "0 100 50" "0 100 51" "0 100 52" "0 100 53" "2 100 50" "2 100 51" "2 100 52"; do
    set -- $block
    ctl command "setblock $1 $2 $3 minecraft:air" >/dev/null
done
ctl command "fill -6 99 48 4 99 53 minecraft:air" >/dev/null
ctl wait 3

echo "== slice 12: a card with no bound player charges nothing, and spends nothing =="
# The control: the same transmitter, the same 10,000, the same cell item in the operator's
# inventory — with a **blank** card in slot 0 (`canInsert` refuses one upstream; a container
# declaration cannot, so the tick's bound-player test is what leaves it alone here).
ctl command "item replace entity @s hotbar.0 with minecraft:air" >/dev/null
ctl wait 3
set_energy 16 100 22 10000
ctl command "item replace block 16 100 22 container.0 with powah:blank_card 1" >/dev/null
ctl wait 4
blank_line=$(info 16 100 22)
case "$blank_line" in
    *"bound none,"*) ;;
    *) fail "a blank card must read no bound player, got '$blank_line'" ;;
esac
ctl command "item replace entity @s hotbar.0 with powah:energy_cell_starter 1" >/dev/null
ctl wait 40
blank_after=$(info 16 100 22)
blank_item=$(inv_energy 0)
echo "  blank: $blank_after (item at ${blank_item:-none} FE)"
[ "$(energy_of "$blank_after")" = 10000 ] ||
    fail "a blank card made the transmitter spend FE: $blank_after"
[ "${blank_item:-0}" = 0 ] ||
    fail "a blank card made the transmitter charge the item to $blank_item FE"
ctl command "item replace entity @s hotbar.0 with minecraft:air" >/dev/null
ctl wait 3

echo "== slice 12: the transmitter ladder, its tag and its kept buffer =="
place 18 100 22 powah:player_transmitter_nitro
set_clamped 18 100 22 2000000000 2000000000
expect_info 18 100 22 "2000000000/2000000000 FE"
expect_info 18 100 22 "charging 1000000 FE/t, 2000000 FE/t a side"
tag_hit=$(ctl command "execute if block 18 100 22 #powah:player_transmitters")
tag_miss=$(ctl command "execute if block 0 99 3 #powah:player_transmitters")
echo "  tag(18,100,22) #player_transmitters = $tag_hit"
echo "  tag(0,99,3)    #player_transmitters = $tag_miss"
case "$tag_hit" in
    *"passed"*) ;;
    *) fail "a player transmitter is not in powah:player_transmitters: $tag_hit" ;;
esac
case "$tag_miss" in
    *"failed"*) ;;
    *) fail "the arena floor matched powah:player_transmitters: $tag_miss" ;;
esac
# `keepEnergy()` is true: the buffer rides the block item back (the hopper's own keep leg's shape).
floor_under 16 24
place 16 100 24 powah:player_transmitter_starter
set_clamped 16 100 24 7000 7000
ctl command "tp @s 16.5 100 23.5" >/dev/null
ctl command "gamemode creative @s" >/dev/null
tool_up
ctl look-at-block 16 100 24 >/dev/null || true
broken=$(ctl break 16 100 24) || fail "cannot break the transmitter at 16,100,24"
echo "$broken" | jq -c '{scenario:"keep", block, drops, picked_up}'
pickup powah:player_transmitter_starter 16 100 24
place_item 16 100 24 powah:player_transmitter_starter
kept_line=$(info 16 100 24)
echo "  kept: $kept_line"
[ "$(energy_of "$kept_line")" = 7000 ] ||
    fail "a broken 7,000 FE transmitter must carry its buffer back, got '$kept_line'"
ctl command "gamemode creative @s" >/dev/null
ctl command "clear @s" >/dev/null
ctl wait 3

echo "== slice 12: the transmitter's loot, every tier =="
# A drop needs a survival break with the right tool (`needs_iron_tool`).
tool_up
for tier in basic hardened blazing niotic spirited nitro; do
    block="powah:player_transmitter_$tier"
    place 20 100 24 "$block"
    ctl look-at-block 20 100 24 >/dev/null || true
    broken=$(ctl break 20 100 24) || fail "cannot break $block at 20,100,24"
    echo "$broken" | jq -c '{scenario:"loot", block, drops}'
    [ "$(echo "$broken" | jq -r '.block')" = "minecraft:air" ] || fail "$block did not break"
    got=$(echo "$broken" | jq -r --arg id "$block" '.drops[$id] // 0')
    [ "$got" -ge 1 ] || fail "breaking $block dropped $got of itself, not the block"
done

echo "== slice 12: an aerial pearl on a zombie makes the transmitter's player aerial pearl =="
# `AerialPearlItem.interactLivingEntity`: the starter recipe's `p` has no other source.
# The pearl is spent only outside creative (`if (!player.isCreative()) stack.shrink(1)`), so this
# leg is a survival one; a peaceful world removes a summoned monster, so the difficulty is easy
# while it stands.
ctl command "clear @s" >/dev/null
ctl command "gamemode survival @s" >/dev/null
ctl command "difficulty easy" >/dev/null
# Back on the arena floor, inside the ~3-block interaction reach of the zombie below.
ctl command "tp @s 0.5 100 3.5" >/dev/null
ctl wait 4
ctl command "kill @e[type=minecraft:zombie]" >/dev/null
ctl command "summon minecraft:zombie 1.5 100 4.5" >/dev/null
ctl wait 4
zombie_id=$(ctl entities 32 minecraft:zombie | jq -r '.entities[0].id // empty')
echo "  zombie: $zombie_id"
[ -n "$zombie_id" ] || fail "cannot find the zombie the pearl leg summons"
ctl command "item replace entity @s hotbar.0 with powah:aerial_pearl 2" >/dev/null
ctl select 1 >/dev/null
ctl wait 4
ctl look-at-entity "$zombie_id" >/dev/null || true
interact_reply=$(ctl interact-entity "$zombie_id" 2>&1) ||
    fail "the aerial pearl could not be used on the zombie $zombie_id: $interact_reply"
echo "  interact: $interact_reply"
ctl wait 6
pearls=$(inv_count powah:player_aerial_pearl)
left=$(inv_count powah:aerial_pearl)
echo "  after the pearl: player aerial pearls $pearls, aerial pearls left $left"
[ "$pearls" -ge 1 ] ||
    fail "an aerial pearl used on a zombie must give a player aerial pearl, none arrived"
[ "$left" = 1 ] ||
    fail "the pearl must spend exactly one of the two it held, $left left"
zombie_left=$(ctl entities 32 minecraft:zombie | jq -r --argjson id "$zombie_id" '[.entities[] | select(.id == $id)] | length')
echo "  the zombie $zombie_id left: $zombie_left"
[ "$zombie_left" = 0 ] || fail "the zombie the pearl was used on must be killed, entity $zombie_id still stands"
ctl command "difficulty peaceful" >/dev/null
ctl command "gamemode creative @s" >/dev/null
ctl command "clear @s" >/dev/null
ctl wait 3

echo "== slice 2: the six dielectric/capacitor items are real items =="
for item in powah:dielectric_paste powah:dielectric_rod powah:dielectric_rod_horizontal \
            powah:dielectric_casing powah:capacitor_basic powah:capacitor_basic_tiny \
            powah:steel_energized powah:crystal_blazing powah:crystal_niotic \
            powah:crystal_spirited powah:crystal_nitro powah:capacitor_basic_large \
            powah:capacitor_hardened powah:capacitor_blazing powah:capacitor_niotic \
            powah:capacitor_spirited powah:capacitor_nitro \
            powah:photoelectric_pane; do
    ctl give "$item" 3 >/dev/null || fail "cannot give $item"
    ctl wait 2
    got=$(inv_count "$item")
    echo "  give $item x3 -> $got in the inventory"
    [ "$got" -ge 3 ] || fail "$item did not reach the inventory (count $got)"
done
unknown=$(ctl command "give @s powah:dielectric_paste_typo 1")
case "$unknown" in
    *"Unknown item"*|*"unknown item"*|*error*|*"not found"*)
        echo "  unknown id refused: $unknown" ;;
    *) fail "an unknown powah item id was accepted: $unknown" ;;
esac
unknown=$(ctl command "give @s powah:crystal_uraninite 1")
case "$unknown" in
    *"Unknown item"*|*"unknown item"*|*error*|*"not found"*)
        echo "  unknown material id refused: $unknown" ;;
    *) fail "an unknown powah material id was accepted: $unknown" ;;
esac
ctl command "clear @s" >/dev/null
ctl wait 3

echo "== slice 2: every carried recipe crafts in a real crafting table =="
ctl command "setblock 5 100 0 minecraft:crafting_table" >/dev/null
ctl wait 4
craft_selftest=0
[ "$SELFTEST" = craft ] && craft_selftest=1
SELFTEST=$craft_selftest python3 "$PORT/tools/craft_check.py" "$SOCK" "$CTL" 5 100 0 \
    || fail "the recipe check failed: every material and machine recipe must craft its declared output and consume its ingredients"
ctl command "setblock 5 100 0 minecraft:air" >/dev/null
ctl command "gamemode creative @s" >/dev/null
ctl command "clear @s" >/dev/null
ctl wait 4

echo "== slice 2: each machine's loot table drops the machine =="
ctl command "setblock 5 100 0 minecraft:air" >/dev/null
ctl command "tp @s 5.5 100 0.5" >/dev/null
ctl wait 4
place 6 100 0 powah:furnator_starter
place 7 100 0 powah:energy_cell_starter
place 8 100 0 powah:energy_cable_starter
ctl command "gamemode survival @s" >/dev/null
ctl command "clear @s" >/dev/null
# The pickaxe must be in the hand: `give` lands in the first free slot, which is
# not necessarily the hotbar, and a `needs_iron_tool` block drops nothing without it.
ctl command "item replace entity @s hotbar.0 with minecraft:iron_pickaxe" >/dev/null
ctl select 1 >/dev/null
ctl wait 4
for spec in "6 powah:furnator_starter" "7 powah:energy_cell_starter" "8 powah:energy_cable_starter"; do
    set -- $spec
    ctl look-at-block "$1" 100 0 >/dev/null || true
    broken=$(ctl break "$1" 100 0) || fail "cannot break $2 at $1,100,0"
    echo "$broken" | jq -c '{scenario:"loot", block, drops}'
    [ "$(echo "$broken" | jq -r '.block')" = "minecraft:air" ] || fail "$2 did not break"
    got=$(echo "$broken" | jq -r --arg id "$2" '.drops[$id] // 0')
    [ "$got" -ge 1 ] || fail "breaking $2 dropped $got of itself, not the machine"
done
# Slice 4: every cell tier drops itself too.
for tier in basic hardened blazing niotic spirited nitro; do
    place 6 100 0 "powah:energy_cell_$tier"
    ctl look-at-block 6 100 0 >/dev/null || true
    broken=$(ctl break 6 100 0) || fail "cannot break powah:energy_cell_$tier at 6,100,0"
    echo "$broken" | jq -c '{scenario:"loot", block, drops}'
    [ "$(echo "$broken" | jq -r '.block')" = "minecraft:air" ] || fail "powah:energy_cell_$tier did not break"
    got=$(echo "$broken" | jq -r --arg id "powah:energy_cell_$tier" '.drops[$id] // 0')
    [ "$got" -ge 1 ] || fail "breaking powah:energy_cell_$tier dropped $got of itself, not the machine"
done
# Slice 5: the orb, every rod tier and the material block drop themselves.
for block in powah:energizing_orb powah:energizing_rod_starter powah:energizing_rod_basic \
             powah:energizing_rod_hardened powah:energizing_rod_blazing \
             powah:energizing_rod_niotic powah:energizing_rod_spirited \
             powah:energizing_rod_nitro powah:blazing_crystal_block; do
    place 6 100 0 "$block"
    ctl look-at-block 6 100 0 >/dev/null || true
    broken=$(ctl break 6 100 0) || fail "cannot break $block at 6,100,0"
    echo "$broken" | jq -c '{scenario:"loot", block, drops}'
    [ "$(echo "$broken" | jq -r '.block')" = "minecraft:air" ] || fail "$block did not break"
    got=$(echo "$broken" | jq -r --arg id "$block" '.drops[$id] // 0')
    [ "$got" -ge 1 ] || fail "breaking $block dropped $got of itself, not the block"
done
# Slice 6: every furnator and cable tier drops itself.
for tier in basic hardened blazing niotic spirited nitro; do
    for block in "powah:furnator_$tier" "powah:energy_cable_$tier"; do
        place 6 100 0 "$block"
        ctl look-at-block 6 100 0 >/dev/null || true
        broken=$(ctl break 6 100 0) || fail "cannot break $block at 6,100,0"
        echo "$broken" | jq -c '{scenario:"loot", block, drops}'
        [ "$(echo "$broken" | jq -r '.block')" = "minecraft:air" ] || fail "$block did not break"
        got=$(echo "$broken" | jq -r --arg id "$block" '.drops[$id] // 0')
        [ "$got" -ge 1 ] || fail "breaking $block dropped $got of itself, not the block"
    done
done
# Slice 8: every discharger tier drops itself.
for tier in starter basic hardened blazing niotic spirited nitro; do
    block="powah:energy_discharger_$tier"
    place 6 100 0 "$block"
    ctl look-at-block 6 100 0 >/dev/null || true
    broken=$(ctl break 6 100 0) || fail "cannot break $block at 6,100,0"
    echo "$broken" | jq -c '{scenario:"loot", block, drops}'
    [ "$(echo "$broken" | jq -r '.block')" = "minecraft:air" ] || fail "$block did not break"
    got=$(echo "$broken" | jq -r --arg id "$block" '.drops[$id] // 0')
    [ "$got" -ge 1 ] || fail "breaking $block dropped $got of itself, not the block"
done
# Slice 9: every hopper tier drops itself.
for tier in starter basic hardened blazing niotic spirited nitro; do
    block="powah:energy_hopper_$tier"
    place 6 100 0 "$block"
    ctl look-at-block 6 100 0 >/dev/null || true
    broken=$(ctl break 6 100 0) || fail "cannot break $block at 6,100,0"
    echo "$broken" | jq -c '{scenario:"loot", block, drops}'
    [ "$(echo "$broken" | jq -r '.block')" = "minecraft:air" ] || fail "$block did not break"
    got=$(echo "$broken" | jq -r --arg id "$block" '.drops[$id] // 0')
    [ "$got" -ge 1 ] || fail "breaking $block dropped $got of itself, not the block"
done
# Slice 10: every solar panel tier drops itself.
for tier in starter basic hardened blazing niotic spirited nitro; do
    block="powah:solar_panel_$tier"
    place 6 100 0 "$block"
    ctl look-at-block 6 100 0 >/dev/null || true
    broken=$(ctl break 6 100 0) || fail "cannot break $block at 6,100,0"
    echo "$broken" | jq -c '{scenario:"loot", block, drops}'
    [ "$(echo "$broken" | jq -r '.block')" = "minecraft:air" ] || fail "$block did not break"
    got=$(echo "$broken" | jq -r --arg id "$block" '.drops[$id] // 0')
    [ "$got" -ge 1 ] || fail "breaking $block dropped $got of itself, not the block"
done
# Slice 11: every ender cell and ender gate tier drops itself.
for family in ender_cell ender_gate; do
    for tier in starter basic hardened blazing niotic spirited nitro; do
        block="powah:${family}_$tier"
        place 6 100 0 "$block"
        ctl look-at-block 6 100 0 >/dev/null || true
        broken=$(ctl break 6 100 0) || fail "cannot break $block at 6,100,0"
        echo "$broken" | jq -c '{scenario:"loot", block, drops}'
        [ "$(echo "$broken" | jq -r '.block')" = "minecraft:air" ] || fail "$block did not break"
        got=$(echo "$broken" | jq -r --arg id "$block" '.drops[$id] // 0')
        [ "$got" -ge 1 ] || fail "breaking $block dropped $got of itself, not the block"
    done
done
ctl command "gamemode creative @s" >/dev/null
ctl command "clear @s" >/dev/null
ctl wait 3

echo "PASS: a fuelled furnator made FE, the cable carried it into the cell, an unfuelled furnator made nothing, a cell without a cable stayed empty, a full cell stayed full, a charged cell drained through a cable into an empty one, a three-cable network carried a charge across its run with the two cells' sum unchanged and a cell off the network empty, the six material items exist, every material and machine recipe crafted its declared output and consumed its ingredients, each machine dropped itself, every energy cell tier clamped 2,000,000,000 FE at its own capacity and carried its own transfer, two adjacent cells swapped within their own transfer cap without creating FE, every cell tier dropped itself, a starter rod filled the orb to exactly its recipe's 10,000 FE and stood at 0, an orb with no rod and an orb with no recipe produced nothing while its rod kept its charge, a nitro rod made a blazing crystal at exactly 120,000 FE and kept the other 80,000, the blaze-rod blazing recipe, the niotic, spirited and nitro crystals all energised, every new block and item exists, every carried recipe including the five previously blocked cells crafts its declared output and consumes its ingredients, every furnator tier clamped 2,000,000,000 FE at its own generation capacity, a nitro furnator made exactly 40,000 FE in one tick of a coal and a starter furnator kept energy + carbon at 48,000 while filling slowly, every cable tier read back its own rate, a basic network refused to relay through a starter cable and delivered once that cable was basic, every new furnator, cable and discharger tier dropped itself, a starter furnator charged the item in its charge slot at exactly min(transfer, stored, room) with the FE coming out of its own buffer and left an item with no buffer alone, a cell charged two slots out of one budget, a 5,000 FE furnator and a 4,000 FE cell broken in survival carried their buffers on their own items and gave them back on placement while their spilled stacks did not, an item doctored to 2,000,000,000 FE clamped to its tier's 20,000 on placement and a fresh item placed an empty machine, a starter hopper spent exactly 500 FE a tick on the cell item in the chest it faced and the two halves of the 5,000 it held added up, a hopper facing away from the chest charged nothing and a stone in the chest took nothing, every hopper tier dropped itself, a 7,000 FE hopper carried its buffer back on its own item, a starter solar panel under an open noon sky generated exactly 20 FE a tick measured against the game clock and filled its last 20 of room without overflowing, a stone above it and midnight each left it at exactly 5,000 for 60 ticks, a cell under a panel charged while a cell beside it stayed empty and the panel took no fill, a nitro panel clamped 2,000,000,000 FE at 40,000,000 and named its 2,000 FE/t, every solar panel tier dropped itself, the photoelectric pane and all seven panels crafted, two ender cells on one channel held and showed the same 100,000 FE while a cell on another channel read 0, a cell moved to the fresh channel read 0 while its peer kept exactly 100,000 and moving it back found the channel's FE again, an ender cell's extender item gave its channel 4,000,000 FE of capacity and its charge slot took 1,000 a tick out of the channel, the channel gave its tier's 4,000 a tick and lost exactly what the sink gained, an ender gate fed only the hopper on its facing side at the cable row's 500 a tick, the nitro ender cell and gate named 12 channels and their 2,000,000/1,000,000 FE a tick, both ender tags matched in game, every ender cell and gate tier dropped itself, a starter player transmitter read its slot empty with no card, a card bound to the operator by its own use named them in the transmitter's readout, the transmitter spent exactly 500 FE a tick out of its 10,000 on the cell item in that player's inventory and the item gained whole 500 FE steps with the two halves adding up, a blank card charged nothing and spent nothing, a nitro transmitter clamped 2,000,000,000 FE and named its 1,000,000 FE/t charging rate, powah:player_transmitters matched in game, a broken 7,000 FE transmitter carried its buffer back on its own item, every transmitter tier dropped itself, an aerial pearl used on a zombie gave the player aerial pearl and killed it, the redstone mode saved and read back through /powah redstone with the readout naming it only when it was not the default, a shut gate holding the transmitter, the furnator, the cell, the cable, the discharger, the hopper, the solar panel and the ender cell at their own work and every one of them running again when the gate opened, a comparator beside a 500,000 FE cell read the strength its readout printed, the reading tracked the buffer to 15 at full and 3 at a quarter, a stone read nothing, a cable answered nothing even with a full chest behind it, a full rod answered nothing because its readout prints no comparator, and every guessed id is unknown without the package"
