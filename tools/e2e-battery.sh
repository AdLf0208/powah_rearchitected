#!/usr/bin/env bash
# Bounded in-game leg for slice 18 (the energy battery ladder). Same game and
# control as tools/e2e-creative.sh, but only the battery assertions: the seven
# tiers' capacity/transfer parsed from /powah item, a starter battery in a
# starter discharger giving exactly 1000 x dt to an adjacent starter cell while
# it loses the same, the stone differential, the seven carried recipes, a
# control and two selftests. A full replay of slices 1-17 is a ~20-minute
# session and this slice changed no earlier family. Every verdict leg prints
# EXIT:<status> from its cleanup trap.
#
#   GAME=<powah game> CTL=<ctl> RES=<prepared 26.3> E2E_TMP=<scratch root> tools/e2e-battery.sh
#   LAUNCH_GAME=$CONTROL_GAME ... tools/e2e-battery.sh     # negative leg
#   SELFTEST=battery ... tools/e2e-battery.sh              # must end FAIL + EXIT:1
#   SELFTEST=1 ...        tools/e2e-battery.sh             # slice-1 flip, EXIT:1
set -uo pipefail

CTL=${CTL:?horizon-ctl is required}
GAME=${GAME:?a launcher-built game carrying powah is required}
RES=${RES:-${HORIZON_RES:-/home/adlf/Desktop/horizon/.local/horizon-client/26.3}}
E2E_TMP=${E2E_TMP:?E2E_TMP must be set (never /tmp)}
case "$E2E_TMP" in /tmp) echo "E2E_TMP must not be /tmp" >&2; exit 1 ;; esac
SELFTEST=${SELFTEST:-0}
LAUNCH_GAME=${LAUNCH_GAME:-$GAME}
SOCK=${SOCK:-$E2E_TMP/powah-battery.sock}
WORLD=PowahBatteryE2E
PORT=$(cd "$(dirname "$0")/.." && pwd)

[ -x "$CTL" ] || { echo "missing horizon-ctl: $CTL" >&2; exit 1; }
[ -x "$GAME" ] || { echo "missing game: $GAME" >&2; exit 1; }
[ -d "$RES/assets" ] || { echo "missing prepared resources: $RES" >&2; exit 1; }

SCRATCH=$(mktemp -d "$E2E_TMP/powah-battery-home.XXXXXX")
export HORIZON_HOME=$SCRATCH/game HORIZON_LAUNCHER_HOME=$SCRATCH/launcher
export XDG_DATA_HOME=$SCRATCH/data XDG_CONFIG_HOME=$SCRATCH/config HORIZON_NO_AUDIO=1
mkdir -p "$(dirname "$SOCK")" "$XDG_CONFIG_HOME"
rm -f "$SOCK"

ctl() { "$CTL" --socket "$SOCK" "$@"; }
say() { ctl chat "/$1" >/dev/null; }
fail() { echo "FAIL: $*" >&2; ctl logs --lines 40 >&2 || true; exit 1; }
cleanup() {
    rc=$?
    timeout 60 "$CTL" --socket "$SOCK" quit >/dev/null 2>&1 || true
    sleep 1
    rm -rf "$SCRATCH" 2>/dev/null || true
    echo "EXIT:$rc"
}
trap cleanup EXIT

time_of() { sed -nE 's/.* t=([0-9]+)\)$/\1/p' <<<"$1"; }
energy_of() { sed -nE 's/.* at [-0-9]+,[-0-9]+,[-0-9]+: ([0-9]+)\/[0-9]+ FE.*/\1/p' <<<"$1"; }
charge_of() { sed -nE 's/^Charge on ([a-z0-9_]+:[a-z0-9_]+): ([0-9]+)\/([0-9]+) FE \(([0-9]+) FE\/t, t=[0-9]+\)$/\1 \2 \3 \4/p' <<<"$1"; }

# A fresh /powah info reply (its own t= clock), never a stale chat line.
info() {
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

# A fresh /powah item readout (slice 18's opt-in line), guarded by its own t=.
charge() {
    local out line before before_t line_t tries
    before=$(ctl messages 20 | jq -r '.chat[]' | grep -E "^Charge on .* t=" | head -1)
    before_t=$(time_of "$before")
    for tries in 1 2 3 4 5; do
        say "powah item $1 $2 $3"
        for _ in $(seq 1 12); do
            ctl wait 4 >/dev/null
            out=$(ctl messages 20)
            line=$(jq -r '.chat[]' <<<"$out" | grep -E "^Charge on |holds no item that carries FE|has no item charge slot|No Powah block" | head -1)
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
    echo "NO-CHARGE $(jq -c '{chat,action_bar}' <<<"$out")"
}

set_reply() {
    local out line=""
    say "powah set $4 $1 $2 $3"
    for _ in $(seq 1 30); do
        ctl wait 10
        out=$(ctl messages 20)
        line=$(jq -r '.chat[]' <<<"$out" | grep -E "Energy set to|stores no energy|No Powah block" | head -1)
        case "$line" in
            *"$5"*) echo "  set($1,$2,$3) reply: $line"; return 0 ;;
        esac
    done
    fail "expected /powah set $4 at $1,$2,$3 to answer '$5', got '$line'"
}

item_set_capped() { # X Y Z AMOUNT ITEM — the reply names what the tier clamped to.
    local out line=""
    say "powah item $4 $1 $2 $3"
    for _ in $(seq 1 30); do
        ctl wait 10
        out=$(ctl messages 20)
        line=$(jq -r '.chat[]' <<<"$out" | grep -E "Charge set to [0-9]+ FE on $5\." | head -1)
        [ -n "$line" ] && { echo "  item($1,$2,$3) reply: $line"; return 0; }
    done
    fail "expected /powah item $4 at $1,$2,$3 to answer 'Charge set to <tier cap> FE on $5.', got '$line'"
}

item_set() { # X Y Z AMOUNT ITEM — the command's own reply is the exact read.
    local out line=""
    say "powah item $4 $1 $2 $3"
    for _ in $(seq 1 30); do
        ctl wait 10
        out=$(ctl messages 20)
        line=$(jq -r '.chat[]' <<<"$out" | grep -E "Charge set to|holds no item that carries FE|has no item charge slot|No Powah block" | head -1)
        case "$line" in
            *"Charge set to $4 FE on $5."*) echo "  item($1,$2,$3) reply: $line"; return 0 ;;
        esac
    done
    fail "expected /powah item $4 at $1,$2,$3 to answer 'Charge set to $4 FE on $5.', got '$line'"
}

cd "$SCRATCH"
echo "  launch: $(basename "$LAUNCH_GAME") (E2E_TMP=$E2E_TMP)"
ctl launch --game "$LAUNCH_GAME" --world "$WORLD" --seed 20261105 \
    --creative --offscreen --wait --resources "$RES" >/dev/null
ctl wait-ready 120
ctl wait 40

if [ "$SELFTEST" = 1 ]; then
    echo "== slice 1: an unfuelled furnator makes no FE =="
    ctl command "setblock 9 100 60 powah:furnator_starter" >/dev/null
    ctl wait 6
    before=$(info 9 100 60)
    ctl wait 20
    after=$(info 9 100 60)
    echo "  unfuelled furnator: $(energy_of "$before") -> $(energy_of "$after") FE"
    [ "$(energy_of "$after")" -gt 0 ] ||
        fail "SELFTEST flipped: the unfuelled furnator made no FE, as it should"
fi

echo "== slice 18: a starter battery gives 1000 FE/t into an adjacent starter cell =="
ctl command "setblock 30 100 60 powah:energy_discharger_starter" >/dev/null
ctl command "setblock 31 100 60 powah:energy_cell_starter" >/dev/null
ctl wait 6
block=$(ctl block 30 100 60 | jq -r '.block // empty')
case "$block" in
    powah:energy_discharger_starter) ;;
    *) fail "expected powah:energy_discharger_starter at 30,100,60, got '$block'" ;;
esac
ctl command "item replace block 30 100 60 container.0 with powah:battery_starter 1" >/dev/null
ctl wait 4
set_reply 31 100 60 0 "Energy set to"
# A 5,000 FE battery empties in five ticks — shorter than one chat round trip,
# so the exact-rate leg uses the full starter capacity (a recorded adjustment
# to the brief's example number, which cannot span two replies' own clocks).
item_set 30 100 60 1000000 powah:battery_starter
bat_a=$(charge 30 100 60)
cell_a=$(info 31 100 60)
read -r id_a held_a cap_a rate_a <<<"$(charge_of "$bat_a")"
a_bt=$(time_of "$bat_a"); a_ct=$(time_of "$cell_a"); a_ce=$(energy_of "$cell_a")
echo "  battery before: $bat_a"
echo "  cell before: $cell_a"
[ "$id_a" = "powah:battery_starter" ] || fail "the discharger slot did not hold a starter battery: $bat_a"
[ "$cap_a" = "1000000" ] && [ "$rate_a" = "1000" ] ||
    fail "the starter battery's capacity/transfer were not 1,000,000 / 1,000: $bat_a"
ctl wait 12
bat_b=$(charge 30 100 60)
cell_b=$(info 31 100 60)
read -r id_b held_b cap_b rate_b <<<"$(charge_of "$bat_b")"
b_bt=$(time_of "$bat_b"); b_ct=$(time_of "$cell_b"); b_ce=$(energy_of "$cell_b")
echo "  battery after: $bat_b"
echo "  cell after: $cell_b"
dt_b=$((b_bt - a_bt)); loss=$((held_a - held_b))
dt_c=$((b_ct - a_ct)); gain=$((b_ce - a_ce))
want_b=$((1000 * dt_b)); want_c=$((1000 * dt_c))
echo "  battery: $held_a -> $held_b, lost $loss over dt=$dt_b (want $want_b)"
echo "  cell: $a_ce -> $b_ce, gained $gain over dt=$dt_c (want $want_c)"
[ "$dt_b" -gt 0 ] && [ "$dt_c" -gt 0 ] ||
    fail "the two readouts carry no distinct t= clock: $bat_a / $cell_a"
if [ "$SELFTEST" = battery ]; then
    [ "$loss" -eq 0 ] ||
        fail "SELFTEST flipped: the battery kept its charge, as it should"
else
    [ "$loss" -eq "$want_b" ] ||
        fail "the battery did not lose exactly 1000 x dt: lost $loss over $dt_b ticks"
    [ "$gain" -eq "$want_c" ] ||
        fail "the cell did not gain exactly 1000 x dt: gained $gain over $dt_c ticks"
fi

echo "== slice 18: the brief's 5,000 FE case conserves exactly =="
ctl command "item replace block 30 100 60 container.0 with minecraft:air" >/dev/null
set_reply 31 100 60 0 "Energy set to"
ctl command "item replace block 30 100 60 container.0 with powah:battery_starter 1" >/dev/null
ctl wait 4
item_set 30 100 60 5000 powah:battery_starter
ctl wait 20
bat_empty=$(charge 30 100 60); cell_full=$(info 31 100 60)
read -r _ held_empty _ _ <<<"$(charge_of "$bat_empty")"
echo "  5000 FE case: battery $held_empty FE, cell $(energy_of "$cell_full") FE"
if [ "$SELFTEST" != battery ]; then
    [ "$held_empty" = 0 ] || fail "a 5,000 FE battery did not fully discharge: $bat_empty"
    [ "$(energy_of "$cell_full")" = 5000 ] ||
        fail "the cell did not gain exactly the battery's 5,000 FE: $cell_full"
fi

echo "== slice 18: a stone in the slot transfers nothing =="
ctl command "item replace block 30 100 60 container.0 with minecraft:stone 1" >/dev/null
set_reply 31 100 60 0 "Energy set to"
ctl wait 20
stone_cell=$(info 31 100 60)
echo "  stone slot, cell after 20 ticks: $(energy_of "$stone_cell") FE"
if [ "$SELFTEST" != battery ]; then
    [ "$(energy_of "$stone_cell")" = 0 ] ||
        fail "a stone in the slot transferred FE: $stone_cell"
fi

echo "== slice 18: every battery tier's capacity and transfer in game =="
ctl command "setblock 34 100 60 powah:energy_cell_starter" >/dev/null
ctl wait 4
tiers="starter:1000000:1000 basic:4000000:4000 hardened:10000000:10000 blazing:40000000:40000 niotic:100000000:100000 spirited:400000000:400000 nitro:2000000000:2000000"
for row in $tiers; do
    IFS=: read -r tier cap rate <<<"$row"
    ctl command "item replace block 34 100 60 container.0 with powah:battery_$tier 1" >/dev/null
    ctl wait 3
    item_set_capped 34 100 60 2000000000 "powah:battery_$tier"
    line=$(charge 34 100 60)
    read -r got_id got_held got_cap got_rate <<<"$(charge_of "$line")"
    echo "  battery_$tier: $line"
    [ "$got_id" = "powah:battery_$tier" ] || fail "wrong battery id in the readout: $line"
    [ "$got_cap" = "$cap" ] || fail "battery_$tier's capacity was $got_cap, want $cap: $line"
    [ "$got_rate" = "$rate" ] || fail "battery_$tier's transfer was $got_rate, want $rate: $line"
    [ "$got_held" = "$cap" ] || fail "battery_$tier did not clamp at $cap: $line"
done

echo "== slice 18: every carried battery recipe crafts in a real table =="
# The full replay has an arena floor at y=99; a bounded leg must lay one under
# the table or the player falls away between `tp` and `interact` (the menu then
# reads "playing", which looks exactly like a broken recipe).
ctl command "fill 0 99 65 9 99 75 minecraft:stone" >/dev/null
ctl command "setblock 5 100 70 minecraft:crafting_table" >/dev/null
ctl wait 4
CRAFT_ONLY=battery SELFTEST=0 python3 "$PORT/tools/craft_check.py" "$SOCK" "$CTL" 5 100 70 \
    || fail "the battery recipe check failed: each must craft its id and count and consume its ingredients"

echo "PASS: seven battery tiers read their exact capacity and transfer, a starter battery gave exactly 1000 x dt into an adjacent cell while losing the same (and the 5,000 FE case conserved exactly), a stone transferred nothing, and all seven carried battery recipes crafted in a real table"
