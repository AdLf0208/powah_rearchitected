#!/usr/bin/env bash
# Bounded in-game leg for slice 17 (the creative energy cell). The same game and
# control as tools/e2e-side.sh, but only the creative-cell assertions: a full
# replay of slices 1-16 is a ~20-minute session and this slice changed no earlier
# family. Every verdict leg prints EXIT:<status> from its cleanup trap.
#
#   GAME=<powah game> CTL=<ctl> RES=<prepared 26.3> E2E_TMP=<scratch root> tools/e2e-creative.sh
#   LAUNCH_GAME=$CONTROL_GAME ... tools/e2e-creative.sh     # negative leg
#   SELFTEST=creative ... tools/e2e-creative.sh             # must end FAIL + EXIT:1
#   SELFTEST=1 ...        tools/e2e-creative.sh             # slice-1 flip, EXIT:1
set -uo pipefail

CTL=${CTL:?horizon-ctl is required}
GAME=${GAME:?a launcher-built game carrying powah is required}
RES=${RES:-${HORIZON_RES:-/home/adlf/Desktop/horizon/.local/horizon-client/26.3}}
E2E_TMP=${E2E_TMP:?E2E_TMP must be set (never /tmp)}
case "$E2E_TMP" in /tmp) echo "E2E_TMP must not be /tmp" >&2; exit 1 ;; esac
SELFTEST=${SELFTEST:-0}
LAUNCH_GAME=${LAUNCH_GAME:-$GAME}
SOCK=${SOCK:-$E2E_TMP/powah-creative.sock}
WORLD=PowahCreativeE2E
CAP=2147483647

[ -x "$CTL" ] || { echo "missing horizon-ctl: $CTL" >&2; exit 1; }
[ -x "$GAME" ] || { echo "missing game: $GAME" >&2; exit 1; }
[ -d "$RES/assets" ] || { echo "missing prepared resources: $RES" >&2; exit 1; }

SCRATCH=$(mktemp -d "$E2E_TMP/powah-creative-home.XXXXXX")
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

info() { # info X Y Z — a fresh reply, never a stale line from the chat window
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

set_reply() { # X Y Z AMOUNT SUBSTRING
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

echo "== slice 17: a creative cell feeds an empty cell through a cable =="
ctl command "setblock -8 100 60 powah:energy_cell_creative" >/dev/null
ctl command "setblock -7 100 60 powah:energy_cable_starter" >/dev/null
ctl command "setblock -6 100 60 powah:energy_cell_starter" >/dev/null
ctl wait 6
block=$(ctl block -8 100 60 | jq -r '.block // empty')
case "$block" in
    powah:energy_cell_creative) ;;
    *) fail "expected powah:energy_cell_creative at -8,100,60, got '$block'" ;;
esac

src_a=$(info -8 100 60)
echo "  source: $src_a"
[ "$(energy_of "$src_a")" = "$CAP" ] ||
    fail "a fresh creative cell did not read its full capacity: $src_a"
case "$src_a" in
    *"${CAP} FE/t a side"*) ;;
    *) fail "the creative cell's readout does not name its tier's transfer: $src_a" ;;
esac
case "$src_a" in
    *creative*) ;;
    *) fail "the creative cell's readout has no creative field: $src_a" ;;
esac

set_reply -6 100 60 0 "Energy set to"
sink_a=$(info -6 100 60)
a_t=$(time_of "$sink_a"); a_e=$(energy_of "$sink_a")
ctl wait 12
sink_b=$(info -6 100 60)
b_t=$(time_of "$sink_b"); b_e=$(energy_of "$sink_b")
dt=$((b_t - a_t)); gain=$((b_e - a_e)); want=$((500 * dt))
echo "  sink: $a_e FE at t=$a_t -> $b_e FE at t=$b_t, gained $gain over dt=$dt (want $want)"
[ "$dt" -gt 0 ] || fail "the two sink readouts carry no distinct t= clock: $sink_a / $sink_b"
[ "$gain" -eq "$want" ] ||
    fail "the cable did not deliver exactly 500 x dt: gained $gain over $dt ticks"

src_b=$(info -8 100 60)
echo "  source after: $src_b"
if [ "$SELFTEST" = creative ]; then
    [ "$(energy_of "$src_b")" -lt "$CAP" ] ||
        fail "SELFTEST flipped: the creative source never dropped, as it should"
else
    [ "$(energy_of "$src_b")" = "$CAP" ] ||
        fail "the creative source dropped to $(energy_of "$src_b") FE: $src_b"
fi

# Differential: a stone in the source's place transfers nothing at all.
ctl command "setblock -8 100 60 minecraft:stone" >/dev/null
set_reply -6 100 60 0 "Energy set to"
ctl wait 20
stone_sink=$(info -6 100 60)
echo "  stone source, sink after 20 ticks: $(energy_of "$stone_sink") FE"
if [ "$SELFTEST" != creative ]; then
    [ "$(energy_of "$stone_sink")" = 0 ] ||
        fail "a stone in the source's place transferred FE: $stone_sink"
fi

echo "PASS: a creative cell read its full 2,147,483,647 FE with its tier's transfer and a creative field, an empty cell behind a cable gained exactly 500 x dt from the replies' own clocks while the source never dropped, and a stone in its place transferred nothing"
