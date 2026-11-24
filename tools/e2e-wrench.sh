#!/usr/bin/env bash
# Bounded in-game leg for slice 19 (the wrench and its side-config action). Same
# game and control as tools/e2e-side.sh, but only the wrench assertions: a fresh
# cell reads `all` on six faces, a right-click with powah:wrench on its north face
# advances that face to the jar's next type (`extract`) with a fresh t= while the
# other five stay put, a stone keeps its "no side config" readout and the item,
# the carried wrench recipe crafts, a control and two fail-capable selftests.
# Every leg prints EXIT:<status> from its cleanup trap (check-e2e-logs --strict).
# The delivered package bytes are not touched by this script.
#
#   GAME=<powah game> CTL=<ctl> RES=<prepared 26.3> E2E_TMP=<scratch root> tools/e2e-wrench.sh
#   LAUNCH_GAME=$CONTROL_GAME ... tools/e2e-wrench.sh     # the negative leg
#   SELFTEST=wrench ... tools/e2e-wrench.sh               # must end FAIL + EXIT:1
#   SELFTEST=1 ...      tools/e2e-wrench.sh               # slice-1 flip, EXIT:1
set -uo pipefail

CTL=${CTL:?horizon-ctl is required}
GAME=${GAME:?a launcher-built game carrying powah is required}
RES=${RES:-${HORIZON_RES:-/home/adlf/Desktop/horizon/.local/horizon-client/26.3}}
E2E_TMP=${E2E_TMP:?E2E_TMP must be set (never /tmp)}
case "$E2E_TMP" in /tmp) echo "E2E_TMP must not be /tmp" >&2; exit 1 ;; esac
SELFTEST=${SELFTEST:-0}
LAUNCH_GAME=${LAUNCH_GAME:-$GAME}
SOCK=${SOCK:-$E2E_TMP/powah-wrench.sock}
WORLD=PowahWrenchE2E
PORT=$(cd "$(dirname "$0")/.." && pwd)

[ -x "$CTL" ] || { echo "missing horizon-ctl: $CTL" >&2; exit 1; }
[ -x "$GAME" ] || { echo "missing game: $GAME" >&2; exit 1; }
[ -d "$RES/assets" ] || { echo "missing prepared resources: $RES" >&2; exit 1; }

SCRATCH=$(mktemp -d "$E2E_TMP/powah-wrench-home.XXXXXX")
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
# A face's type, parsed from the readout (faces after `down` are comma-separated,
# so the separator is a space OR a comma).
side_field() { sed -nE "s/.*[ ,]$2=([a-z]+).*/\1/p" <<<"$1"; }

side() { # side X Y Z — the opt-in six-face readout, with a fresh-reply guard
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
    echo "  unfuelled furnator: $(energy_of "$before") -> $(energy_of "$after") FE"
    [ "$(energy_of "$after")" -gt 0 ] ||
        fail "SELFTEST flipped: the unfuelled furnator made no FE, as it should"
fi

echo "== slice 19: a fresh cell and a held wrench =="
ctl command "setblock 6 100 60 powah:energy_cell_starter" >/dev/null
ctl command "setblock 8 100 60 minecraft:stone" >/dev/null
ctl wait 6
# The whole-case control: an unmodded game knows none of the ids, so the cell is air.
block=$(ctl block 6 100 60 | jq -r '.block // empty')
case "$block" in
    powah:energy_cell_starter) ;;
    *) fail "expected powah:energy_cell_starter at 6,100,60, got '$block'" ;;
esac
ctl give powah:wrench 1 >/dev/null
ctl wait 3
ctl select powah:wrench >/dev/null
ctl wait 2
held=$(ctl inventory | jq -r '.counts["powah:wrench"] // 0')
[ "$held" -ge 1 ] || fail "the wrench was not given into the hotbar: inventory counts $held"

fresh=$(side 6 100 60)
echo "  fresh cell: $fresh"
for face in down up north south west east; do
    [ "$(side_field "$fresh" "$face")" = all ] ||
        fail "a fresh cell's $face face is not all: $fresh"
done

echo "== slice 19: wrenching the north face advances it once =="
# `interact X Y Z north` aims from the player's eye and refuses a face the ray
# cannot reach: stand NORTH of the cell (z < 60) so the ray hits its north face.
ctl command "tp @s 6.5 100 56.5" >/dev/null
ctl wait 2
echo "  interact: $(ctl interact 6 100 60 north)"
ctl wait 6
after=$(side 6 100 60)
echo "  after the wrench click: $after"
north=$(side_field "$after" north)
if [ "$SELFTEST" = wrench ]; then
    # The inverted half: on a correct package the face DID change to extract, so
    # demanding it stayed `all` must fail here (EXIT:1).
    [ "$north" = all ] ||
        fail "SELFTEST flipped: the wrench left the north face at $north, as it should"
else
    [ "$north" = extract ] ||
        fail "the wrench did not advance the north face all -> extract: $after"
    for face in down up south west east; do
        [ "$(side_field "$after" "$face")" = all ] ||
            fail "wrenching north must not change $face: $after"
    done
    [ -n "$(time_of "$after")" ] || fail "the side readout carries no fresh t= clock: $after"
fi

echo "== slice 19: a stone takes no change and keeps the item =="
stone_before=$(side 8 100 60)
ctl command "tp @s 8.5 100 56.5" >/dev/null
ctl wait 2
echo "  stone interact: $(ctl interact 8 100 60 north)"
ctl wait 6
stone_after=$(side 8 100 60)
echo "  stone before: $stone_before"
echo "  stone after:  $stone_after"
[ "$stone_before" = "$stone_after" ] ||
    fail "a stone's side readout changed after wrenching: $stone_before -> $stone_after"
case "$stone_after" in
    *"no side config"*) ;;
    *) fail "a stone should report no side config, got: $stone_after" ;;
esac
held=$(ctl inventory | jq -r '.counts["powah:wrench"] // 0')
[ "$held" -ge 1 ] || fail "wrenching a stone consumed the item: inventory counts $held"

echo "== slice 19: the carried wrench recipe crafts in a real table =="
ctl command "fill 0 99 65 9 99 75 minecraft:stone" >/dev/null
ctl command "setblock 5 100 70 minecraft:crafting_table" >/dev/null
ctl wait 4
CRAFT_ONLY=wrench SELFTEST=0 python3 "$PORT/tools/craft_check.py" "$SOCK" "$CTL" 5 100 70 \
    || fail "the wrench recipe check failed: it must craft powah:wrench and consume its ingredients"

echo "PASS: a fresh cell read all on six faces, the wrench advanced its north face all -> extract with a fresh t= and left the other five alone, a stone kept its no-side-config readout and the item, and the jar's wrench recipe crafted its id and count with every ingredient consumed"
