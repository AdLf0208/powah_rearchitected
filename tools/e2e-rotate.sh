#!/usr/bin/env bash
# Bounded in-game leg for slice 20 (the wrench's ROTATE mode). Same game, control
# and fail-capability discipline as tools/e2e-wrench.sh, but the asserted state is
# the machine's `facing` block property, not a side-config face:
#   * `/powah wrench mode rotate` then a right-click advances a hopper's
#     `facing=north` to the jar's next direction (`south`) with a fresh t=;
#   * a CONFIG-mode wrench leaves `facing` unchanged (differential);
#   * a machine with no `facing` (a cell) refuses cleanly and keeps its readout;
#   * a stone (foreign block) refuses cleanly and keeps the item;
#   * SELFTEST=rotate inverts the changed half and must end EXIT:1;
#   * SELFTEST=1 is slice 1's still-live flip; LAUNCH_GAME=$CONTROL_GAME is the
#     unmodded control, which fails at the first powah block.
# Every leg prints EXIT:<status> from its cleanup trap (check-e2e-logs --strict).
# The delivered package bytes are not touched by this script.
#
#   GAME=<powah game> CTL=<ctl> RES=<prepared 26.3> E2E_TMP=<scratch root> tools/e2e-rotate.sh
#   LAUNCH_GAME=$CONTROL_GAME ... tools/e2e-rotate.sh     # negative leg
#   SELFTEST=rotate ... tools/e2e-rotate.sh               # must end FAIL + EXIT:1
#   SELFTEST=1 ...      tools/e2e-rotate.sh               # slice-1 flip, EXIT:1
set -uo pipefail

CTL=${CTL:?horizon-ctl is required}
GAME=${GAME:?a launcher-built game carrying powah is required}
RES=${RES:-${HORIZON_RES:-/home/adlf/Desktop/horizon/.local/horizon-client/26.3}}
E2E_TMP=${E2E_TMP:?E2E_TMP must be set (never /tmp)}
case "$E2E_TMP" in /tmp) echo "E2E_TMP must not be /tmp" >&2; exit 1 ;; esac
SELFTEST=${SELFTEST:-0}
LAUNCH_GAME=${LAUNCH_GAME:-$GAME}
SOCK=${SOCK:-$E2E_TMP/powah-rotate.sock}
WORLD=PowahRotateE2E
PORT=$(cd "$(dirname "$0")/.." && pwd)

[ -x "$CTL" ] || { echo "missing horizon-ctl: $CTL" >&2; exit 1; }
[ -x "$GAME" ] || { echo "missing game: $GAME" >&2; exit 1; }
[ -d "$RES/assets" ] || { echo "missing prepared resources: $RES" >&2; exit 1; }

SCRATCH=$(mktemp -d "$E2E_TMP/powah-rotate-home.XXXXXX")
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
facing_field() { sed -nE 's/.*, facing ([a-z]+),.*/\1/p' <<<"$1"; }

info() { # info X Y Z — a fresh reply, never a stale chat line
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

mode() { # the opt-in wrench-mode readout, fresh
    local out line before
    before=$(ctl messages 20 | jq -r '.chat[]' | grep -E '^Wrench mode:' | head -1)
    for tries in 1 2 3 4 5; do
        say "powah wrench"
        for _ in $(seq 1 12); do
            ctl wait 4 >/dev/null
            out=$(ctl messages 20)
            line=$(jq -r '.chat[]' <<<"$out" | grep -E '^Wrench mode:' | head -1)
            [ -n "$line" ] && [ "$line" != "$before" ] && { echo "$line"; return; }
        done
        before=$line
    done
    echo "NO-MODE $(jq -c '{chat,action_bar}' <<<"$out")"
}

cd "$SCRATCH"
echo "  launch: $(basename "$LAUNCH_GAME") (E2E_TMP=$E2E_TMP)"
ctl launch --game "$LAUNCH_GAME" --world "$WORLD" --seed 20261105 \
    --creative --offscreen --wait --resources "$RES" >/dev/null
ctl wait-ready 120
ctl wait 40

if [ "$SELFTEST" = 1 ]; then
    echo "== slice 1: an unfuelled furnator makes no FE =="
    ctl command "setblock 10 100 60 powah:furnator_starter" >/dev/null
    ctl wait 6
    before=$(info 10 100 60)
    ctl wait 20
    after=$(info 10 100 60)
    before_e=$(sed -nE 's/.*: ([0-9]+)\/20000 FE.*/\1/p' <<<"$before")
    after_e=$(sed -nE 's/.*: ([0-9]+)\/20000 FE.*/\1/p' <<<"$after")
    echo "  unfuelled furnator: ${before_e:-?} -> ${after_e:-?} FE"
    [ "${after_e:-0}" -gt 0 ] ||
        fail "SELFTEST flipped: the unfuelled furnator made no FE, as it should"
fi

echo "== slice 20: a hopper facing north and a held wrench =="
ctl command "setblock 6 100 60 powah:energy_hopper_starter[facing=north]" >/dev/null
ctl command "setblock 8 100 60 minecraft:stone" >/dev/null
ctl command "setblock 10 100 60 powah:energy_cell_starter" >/dev/null
ctl wait 6
# The whole-case control: an unmodded game knows none of the ids, so the block is air.
block=$(ctl block 6 100 60 | jq -r '.block // empty')
case "$block" in
    powah:energy_hopper_starter) ;;
    *) fail "expected powah:energy_hopper_starter at 6,100,60, got '$block'" ;;
esac
ctl give powah:wrench 1 >/dev/null
ctl wait 3
ctl select powah:wrench >/dev/null
ctl wait 2
held=$(ctl inventory | jq -r '.counts["powah:wrench"] // 0')
[ "$held" -ge 1 ] || fail "the wrench was not given into the hotbar: counts $held"

before=$(info 6 100 60)
echo "  before: $before"
[ "$(facing_field "$before")" = north ] || fail "the hopper did not start facing north: $before"
before_t=$(time_of "$before")
[ -n "$before_t" ] || fail "the info readout carries no clock: $before"

echo "== slice 20: ROTATE advances north to the jar's next direction (south) =="
say "powah wrench mode rotate"
echo "  mode: $(mode)"
# A fresh wrench is CONFIG; the operator-only command is the mode selector's
# replacement (the hook carries no shift-key state).
case "$(mode)" in
    *rotate*) ;;
    *) fail "the mode command did not select rotate: $(mode)" ;;
esac
# `interact X Y Z north` aims from the player's eye and refuses a face the ray
# cannot reach: stand NORTH of the hopper (z < 60) so the ray hits its north face.
ctl command "tp @s 6.5 100 56.5" >/dev/null
ctl wait 2
echo "  interact: $(ctl interact 6 100 60 north)"
ctl wait 6
after=$(info 6 100 60)
echo "  after the rotate click: $after"
after_t=$(time_of "$after")
[ -n "$after_t" ] && [ "$after_t" -gt "$before_t" ] ||
    fail "the rotate readout is stale: $before_t -> $after_t"
after_facing=$(facing_field "$after")
if [ "$SELFTEST" = rotate ]; then
    # The inverted half: on a correct package facing DID advance to south, so
    # demanding it stayed north must fail here (EXIT:1).
    [ "$after_facing" = north ] ||
        fail "SELFTEST flipped: the wrench left facing at $after_facing, as it should"
else
    [ "$after_facing" = south ] ||
        fail "the wrench did not rotate north -> south: $after"
fi

if [ "$SELFTEST" != rotate ]; then
    echo "== slice 20: a CONFIG-mode wrench leaves facing alone =="
    ctl command "setblock 6 100 60 powah:energy_hopper_starter[facing=north]" >/dev/null
    ctl wait 6
    cfg_before=$(info 6 100 60)
    cfg_before_t=$(time_of "$cfg_before")
    say "powah wrench mode config"
    echo "  mode: $(mode)"
    case "$(mode)" in
        *config*) ;;
        *) fail "the mode command did not select config: $(mode)" ;;
    esac
    ctl command "tp @s 6.5 100 56.5" >/dev/null
    ctl wait 2
    echo "  config interact: $(ctl interact 6 100 60 north)"
    ctl wait 6
    cfg_after=$(info 6 100 60)
    echo "  config before: $cfg_before"
    echo "  config after:  $cfg_after"
    [ "$(facing_field "$cfg_after")" = north ] ||
        fail "a CONFIG-mode wrench changed facing: $cfg_after"
    cfg_after_t=$(time_of "$cfg_after")
    [ -n "$cfg_after_t" ] && [ "$cfg_after_t" -gt "$cfg_before_t" ] ||
        fail "the config readout is stale: $cfg_before_t -> $cfg_after_t"

    echo "== slice 20: a block with no facing refuses cleanly =="
    say "powah wrench mode rotate"
    case "$(mode)" in
        *rotate*) ;;
        *) fail "the mode command did not reselect rotate: $(mode)" ;;
    esac
    noface_before=$(info 10 100 60)
    ctl command "tp @s 10.5 100 56.5" >/dev/null
    ctl wait 2
    echo "  cell interact: $(ctl interact 10 100 60 north)"
    ctl wait 6
    noface_after=$(info 10 100 60)
    echo "  cell before: $noface_before"
    echo "  cell after:  $noface_after"
    case "$noface_after" in
        *t=*) ;;
        *) fail "the no-facing machine stopped answering after a rotate click: $noface_after" ;;
    esac
    [ -z "$(facing_field "$noface_after")" ] ||
        fail "a cell acquired a facing from a rotate click: $noface_after"

    echo "== slice 20: a stone keeps the item and is untouched =="
    ctl command "tp @s 8.5 100 56.5" >/dev/null
    ctl wait 2
    echo "  stone interact: $(ctl interact 8 100 60 north)"
    ctl wait 6
    stone_after=$(ctl block 8 100 60 | jq -r '.block // empty')
    [ "$stone_after" = minecraft:stone ] ||
        fail "a stone changed under a rotate click: $stone_after"
    held=$(ctl inventory | jq -r '.counts["powah:wrench"] // 0')
    [ "$held" -ge 1 ] || fail "rotating a stone consumed the item: counts $held"
fi

echo "PASS: the operator set ROTATE mode, a held wrench advanced a hopper's facing north -> south with a fresh t=, a CONFIG-mode wrench left facing north, a cell with no facing refused cleanly, a stone kept its block and the item, and the jar's rotate order is the next Direction.values() entry"
