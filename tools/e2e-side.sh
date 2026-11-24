#!/usr/bin/env bash
# Bounded in-game leg for slice 16 (the side config) — the same assertions as the
# slice-16 phase of tools/e2e.sh, without replaying slices 1-15 (each full replay
# is a ~20-minute game session; this leg is the one the orchestrator re-runs for
# the control and the fail-capable SELFTEST=side run).
#
#   GAME=<powah game> CTL=<ctl> RES=<prepared 26.3> E2E_TMP=<scratch root> tools/e2e-side.sh
#   LAUNCH_GAME=$CONTROL_GAME ... tools/e2e-side.sh     # the negative leg
#   SELFTEST=side ... tools/e2e-side.sh                 # must end FAIL + EXIT:1
#
# Every leg prints EXIT:<status> from its cleanup trap, as check-e2e-logs --strict
# requires. The delivered package bytes are not touched by this script.
set -uo pipefail

CTL=${CTL:?horizon-ctl is required}
GAME=${GAME:?a launcher-built game carrying powah is required}
RES=${RES:-${HORIZON_RES:-/home/adlf/Desktop/horizon/.local/horizon-client/26.3}}
E2E_TMP=${E2E_TMP:?E2E_TMP must be set (never /tmp)}
case "$E2E_TMP" in /tmp) echo "E2E_TMP must not be /tmp" >&2; exit 1 ;; esac
SELFTEST=${SELFTEST:-0}
LAUNCH_GAME=${LAUNCH_GAME:-$GAME}
SOCK=${SOCK:-$E2E_TMP/powah-side.sock}
WORLD=PowahSideE2E

[ -x "$CTL" ] || { echo "missing horizon-ctl: $CTL" >&2; exit 1; }
[ -x "$GAME" ] || { echo "missing game: $GAME" >&2; exit 1; }
[ -d "$RES/assets" ] || { echo "missing prepared resources: $RES" >&2; exit 1; }

SCRATCH=$(mktemp -d "$E2E_TMP/powah-side-home.XXXXXX")
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
# A face's type, parsed from the readout. The faces after `down` are comma-separated,
# so the separator is a space OR a comma (the first e2e version required a space and
# false-failed on `up=all` — kept here as the harness finding for this slice).
side_field() { sed -nE "s/.*[ ,]$2=([a-z]+).*/\1/p" <<<"$1"; }

info() { # info X Y Z — a fresh reply, never a stale one from the chat window
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

side() { # side X Y Z — the opt-in six-face readout, same fresh-reply guard
    local out line before before_t line_t tries
    before=$(ctl messages 20 | jq -r '.chat[]' | grep -E " at $1,$2,$3 \(side " | head -1)
    before_t=$(time_of "$before")
    for tries in 1 2 3 4 5; do
        say "powah side $1 $2 $3"
        for _ in $(seq 1 12); do
            ctl wait 4 >/dev/null
            out=$(ctl messages 20)
            line=$(jq -r '.chat[]' <<<"$out" |
                grep -E " at $1,$2,$3 \(side |no side config|No Powah block" | head -1)
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
    echo "NO-SIDE $(jq -c '{chat,action_bar}' <<<"$out")"
}

side_reply() { # side_reply X Y Z FACE TYPE SUBSTRING — the wrench button's replacement
    local out line=""
    say "powah side $4 $5 $1 $2 $3"
    for _ in $(seq 1 30); do
        ctl wait 10
        out=$(ctl messages 20)
        line=$(jq -r '.chat[]' <<<"$out" |
            grep -E "Side (down|up|north|south|west|east) set to|no side config|No Powah block" | head -1)
        case "$line" in
            *"$6"*) echo "  side($1,$2,$3) reply: $line"; return 0 ;;
        esac
    done
    fail "expected /powah side $4 $5 at $1,$2,$3 to answer '$6', got '$line'"
}

set_reply() { # set_reply X Y Z AMOUNT SUBSTRING — no equality poll on a draining cell
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

set_energy() { # X Y Z AMOUNT — only for a cell that is not currently draining
    local want=$4 line
    say "powah set $want $1 $2 $3"
    for _ in $(seq 1 10); do
        line=$(info "$1" "$2" "$3")
        [ "$(energy_of "$line")" = "$want" ] && { echo "  set($1,$2,$3) = $line"; return; }
        say "powah set $want $1 $2 $3"
    done
    fail "could not set $1,$2,$3 to $want FE: $line"
}

cd "$SCRATCH"
echo "  launch: $(basename "$LAUNCH_GAME") (E2E_TMP=$E2E_TMP)"
ctl launch --game "$LAUNCH_GAME" --world "$WORLD" --seed 20261105 \
    --creative --offscreen --wait --resources "$RES" >/dev/null
ctl wait-ready 120
ctl wait 40

if [ "$SELFTEST" = 1 ]; then
    # The slice-1 flip, kept fail-capable here: an unfuelled furnator must make no FE.
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

echo "== slice 16: the six per-face transfer types =="
ctl command "setblock -4 100 58 powah:energy_cell_starter" >/dev/null
ctl command "setblock -4 100 59 powah:energy_cable_starter" >/dev/null
ctl command "setblock -4 100 60 powah:energy_cell_starter" >/dev/null
ctl command "setblock -4 100 61 powah:energy_cable_starter" >/dev/null
ctl command "setblock -4 100 62 powah:energy_cell_starter" >/dev/null
ctl command "setblock 6 100 60 powah:furnator_starter" >/dev/null
ctl wait 6
# The whole-case control: the same game without the package knows none of the ids,
# so its setblock is refused and the block stays air.
block=$(ctl block -4 100 60 | jq -r '.block // empty')
case "$block" in
    powah:energy_cell_starter) ;;
    *) fail "expected powah:energy_cell_starter at -4,100,60, got '$block'" ;;
esac

fresh=$(side -4 100 60)
echo "  fresh cell: $fresh"
for face in down up north south west east; do
    [ "$(side_field "$fresh" "$face")" = all ] ||
        fail "a fresh cell's $face face is not all: $fresh"
done
# The defaults are NOT all the same: a furnator (AbstractEnergyProvider) gives only.
furn=$(side 6 100 60)
echo "  fresh furnator: $furn"
[ "$(side_field "$furn" north)" = extract ] ||
    fail "a furnator defaults to give only, not all: $furn"
side_reply -4 100 59 north none "no side config"

side_reply -4 100 60 north none "Side north set to none."
blocked=$(side -4 100 60)
echo "  after north=none: $blocked"
[ "$(side_field "$blocked" north)" = none ] ||
    fail "the readout did not show the parsed north=none: $blocked"
[ "$(side_field "$blocked" south)" = all ] ||
    fail "setting north must not change south: $blocked"
[ -n "$(time_of "$blocked")" ] || fail "the side readout carries no fresh t= clock: $blocked"

# Phase A — a charged source on the blocked face, over 20 ticks.
set_reply -4 100 58 200000 "Energy set to"
set_energy -4 100 60 0
set_energy -4 100 62 0
ctl wait 20
north_line=$(info -4 100 60)
south_line=$(info -4 100 62)
echo "  blocked over 20 ticks: cell $(energy_of "$north_line") FE, sink $(energy_of "$south_line") FE"
if [ "$SELFTEST" = side ]; then
    [ "$(energy_of "$north_line")" -gt 0 ] ||
        fail "SELFTEST flipped: a none face kept its sink at 0 FE, as it should"
else
    [ "$(energy_of "$north_line")" = 0 ] ||
        fail "a cell whose north face is none took FE through that face: $north_line"
    [ "$(energy_of "$south_line")" = 0 ] ||
        fail "the sink behind a blocked face gained FE: $south_line"
fi

# Phase B — the south face still gives exactly 500 x dt out of its own buffer.
set_reply -4 100 60 1000000 "Energy set to"
read_a=$(info -4 100 60)
a_t=$(time_of "$read_a"); a_e=$(energy_of "$read_a")
ctl wait 10
read_b=$(info -4 100 60)
b_t=$(time_of "$read_b"); b_e=$(energy_of "$read_b")
dt=$((b_t - a_t)); lost=$((a_e - b_e)); want=$((500 * dt))
echo "  south face: $a_e FE at t=$a_t -> $b_e FE at t=$b_t, lost $lost over dt=$dt (want $want)"
if [ "$SELFTEST" != side ]; then
    [ "$dt" -gt 0 ] || fail "the two rate readouts carry no distinct t= clock"
    [ "$lost" -eq "$want" ] ||
        fail "the south face did not deliver exactly 500 x dt: lost $lost over $dt ticks"
fi
# Freeze, then check conservation and the untouched source.
ctl command "setblock -4 100 61 minecraft:air" >/dev/null
ctl wait 4
held=$(energy_of "$(info -4 100 60)")
gained=$(energy_of "$(info -4 100 62)")
src=$(energy_of "$(info -4 100 58)")
echo "  frozen: cell $held + sink $gained = $((held + gained)), source $src"
if [ "$SELFTEST" != side ]; then
    [ $((held + gained)) -eq 1000000 ] ||
        fail "the south transfer created or lost FE: $held + $gained != 1000000"
    [ "$gained" -gt 0 ] || fail "the south face gave nothing"
    [ "$src" -eq 200000 ] || fail "a blocked north face let the source lose FE: $src"
fi

# Phase C — flipping the north face back starts the charge.
set_energy -4 100 60 0
side_reply -4 100 60 north all "Side north set to all."
opened=$(side -4 100 60)
echo "  after north=all: $opened"
[ "$(side_field "$opened" north)" = all ] ||
    fail "the readout did not show north=all after the flip: $opened"
charged=0
for _ in $(seq 1 15); do
    charged=$(energy_of "$(info -4 100 60)")
    [ "$charged" -gt 0 ] && break
    ctl wait 10
done
echo "  after the flip: cell $charged FE"
if [ "$SELFTEST" != side ]; then
    [ "$charged" -gt 0 ] || fail "opening the north face again did not start the charge"
fi

echo "PASS: a fresh cell read all on six faces and a furnator extract, /powah side parsed all six values, a north face set to none kept the cell and its sink at 0 FE over 20 ticks with the source still charged, the south face delivered exactly 500 x dt without creating FE, and opening the north face again started the charge"
