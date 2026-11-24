#!/usr/bin/env bash
# Bounded in-game leg for slice 21 (the wrench's LINK mode, the two-click rod<->orb link).
# Same game, control and fail-capability discipline as tools/e2e-rotate.sh, but the
# asserted state is the rod's saved `orb` key and the wrench's two stored click positions:
#   * LINK mode, orb out of range: click rod then orb -> the rod's `orb` field stays empty
#     and the stored rod position is consumed (the range refusal);
#   * LINK mode, two orbs in range: the placement search auto-links the FIRST one, and the
#     wrench's rod->(second orb) click must OVERRIDE it with the exact clicked position,
#     after which the rod fills that orb (the fail-capable half: 0.2.20 has no link write);
#   * a rod clicked twice stores its own position (the first half of the state machine);
#   * an unmatched orb click changes nothing on the rod;
#   * CONFIG and ROTATE still work (no regression in the shared dispatch);
#   * SELFTEST=link inverts the linked half and ends EXIT:1;
#   * SELFTEST=1 is slice 1's still-live flip; LAUNCH_GAME=$CONTROL_GAME is the
#     unmodded control, which fails at the first powah block.
# Every leg prints EXIT:<status> from its cleanup trap (check-e2e-logs --strict).
# The delivered package bytes are not touched by this script.
#
#   GAME=<powah game> CTL=<ctl> RES=<prepared 26.3> E2E_TMP=<scratch root> tools/e2e-link.sh
#   LAUNCH_GAME=$CONTROL_GAME ... tools/e2e-link.sh     # negative leg
#   SELFTEST=link ... tools/e2e-link.sh                 # must end FAIL + EXIT:1
#   SELFTEST=1 ...    tools/e2e-link.sh                 # slice-1 flip, EXIT:1
set -uo pipefail

CTL=${CTL:?horizon-ctl is required}
GAME=${GAME:?a launcher-built game carrying powah is required}
RES=${RES:-${HORIZON_RES:-/home/adlf/Desktop/horizon/.local/horizon-client/26.3}}
E2E_TMP=${E2E_TMP:?E2E_TMP must be set (never /tmp)}
case "$E2E_TMP" in /tmp) echo "E2E_TMP must not be /tmp" >&2; exit 1 ;; esac
SELFTEST=${SELFTEST:-0}
LAUNCH_GAME=${LAUNCH_GAME:-$GAME}
SOCK=${SOCK:-$E2E_TMP/powah-link.sock}
WORLD=PowahLinkE2E
PORT=$(cd "$(dirname "$0")/.." && pwd)

[ -x "$CTL" ] || { echo "missing horizon-ctl: $CTL" >&2; exit 1; }
[ -x "$GAME" ] || { echo "missing game: $GAME" >&2; exit 1; }
[ -d "$RES/assets" ] || { echo "missing prepared resources: $RES" >&2; exit 1; }

SCRATCH=$(mktemp -d "$E2E_TMP/powah-link-home.XXXXXX")
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
# The rod's link is printed as `orb x,y,z` or `no orb` just before its clock.
orb_field() { sed -nE 's/.*, (orb [-0-9]+,[-0-9]+,[-0-9]+|no orb), t=[0-9]+\)$/\1/p' <<<"$1"; }
facing_field() { sed -nE 's/.*, facing ([a-z]+),.*/\1/p' <<<"$1"; }
side_field() { sed -nE "s/.*[ ,]$2=([a-z]+).*/\1/p" <<<"$1"; }

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

# The opt-in LINK wrench readout carries the level's clock, so freshness is the same
# strict `t=` test `info()` uses (a stale chat line can never satisfy it).
wrench_t() { sed -nE 's/.*, t=([0-9]+)\).*$/\1/p' <<<"$1"; }
wrench() {
    local out line before before_t line_t tries
    # `ctl messages` lists newest first, so `head -1` is the latest readout.
    before=$(ctl messages 200 | jq -r '.chat[]' | grep -E '^Wrench mode:.* t=[0-9]+\)' | head -1)
    before_t=$(wrench_t "$before")
    for tries in 1 2 3 4 5; do
        say "powah wrench"
        for _ in $(seq 1 12); do
            ctl wait 4 >/dev/null
            out=$(ctl messages 200)
            line=$(jq -r '.chat[]' <<<"$out" | grep -E '^Wrench mode:.* t=[0-9]+\)' | head -1)
            [ -n "$line" ] || continue
            line_t=$(wrench_t "$line")
            if [ -n "$line_t" ] && [ -n "$before_t" ]; then
                [ "$line_t" -gt "$before_t" ] && { echo "$line"; return; }
            elif [ -n "$line_t" ] && [ "$line" != "$before" ]; then
                echo "$line"
                return
            fi
        done
        before=$line
        before_t=$line_t
    done
    echo "NO-WRENCH $(jq -c '{chat,action_bar}' <<<"$out")"
}

# A mode-only read (CONFIG/ROTATE lines have no clock). Fresh by message count,
# which is unambiguous once the LINK phase's clocked replies have settled.
mode_line() {
    local out count before_count tries
    before_count=$(ctl messages 200 | jq -r '.chat[]' | grep -c '^Wrench mode:')
    for tries in 1 2 3; do
        say "powah wrench"
        for _ in $(seq 1 12); do
            ctl wait 4 >/dev/null
            out=$(ctl messages 200)
            count=$(jq -r '.chat[]' <<<"$out" | grep -c '^Wrench mode:')
            if [ "$count" -gt "$before_count" ]; then
                # newest first: `head -1` is this call's reply.
                jq -r '.chat[]' <<<"$out" | grep '^Wrench mode:' | head -1
                return
            fi
        done
        before_count=$count
    done
    echo "NO-MODE $(jq -c '{chat,action_bar}' <<<"$out")"
}

side() { # side X Y Z — the opt-in six-face readout, fresh
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

click_north() { # click_north X Y Z — stand north of the block and hit its north face
    ctl command "tp @s $(( $1 )).5 $(( $2 )) $(( $3 - 4 )).5" >/dev/null
    ctl wait 2
    ctl interact "$1" "$2" "$3" north
}

cd "$SCRATCH"
echo "  launch: $(basename "$LAUNCH_GAME") (E2E_TMP=$E2E_TMP)"
ctl launch --game "$LAUNCH_GAME" --world "$WORLD" --seed 20261105 \
    --creative --offscreen --wait --resources "$RES" >/dev/null
ctl wait-ready 120
ctl wait 40

if [ "$SELFTEST" = 1 ]; then
    echo "== slice 1: an unfuelled furnator makes no FE =="
    ctl command "setblock 14 100 60 powah:furnator_starter" >/dev/null
    ctl wait 6
    before=$(info 14 100 60)
    ctl wait 20
    after=$(info 14 100 60)
    echo "  unfuelled furnator: $(energy_of "$before") -> $(energy_of "$after") FE"
    [ "$(energy_of "$after")" -gt 0 ] ||
        fail "SELFTEST flipped: the unfuelled furnator made no FE, as it should"
fi

echo "== slice 21: a rod, an out-of-range orb and a held wrench =="
ctl command "setblock 0 100 60 powah:energizing_rod_starter" >/dev/null
ctl command "setblock 0 100 66 powah:energizing_orb" >/dev/null
ctl wait 6
# The whole-case control: an unmodded game knows none of the ids, so the block is air.
block=$(ctl block 0 100 60 | jq -r '.block // empty')
case "$block" in
    powah:energizing_rod_starter) ;;
    *) fail "expected powah:energizing_rod_starter at 0,100,60, got '$block'" ;;
esac
ctl give powah:wrench 1 >/dev/null
ctl wait 3
ctl select powah:wrench >/dev/null
ctl wait 2
held=$(ctl inventory | jq -r '.counts["powah:wrench"] // 0')
[ "$held" -ge 1 ] || fail "the wrench was not given into the hotbar: counts $held"

echo "== slice 21: LINK mode, an out-of-range orb is refused =="
say "powah wrench mode link"
lmode=$(wrench)
echo "  mode: $lmode"
case "$lmode" in
    *"mode: link"*) ;;
    *) fail "the mode command did not select link: $lmode" ;;
esac
before=$(info 0 100 60)
echo "  rod before: $before"
[ "$(orb_field "$before")" = "no orb" ] || fail "the far orb auto-linked the rod: $before"
before_t=$(time_of "$before")
[ -n "$before_t" ] || fail "the info readout carries no clock: $before"

echo "  rod click: $(click_north 0 100 60)"
ctl wait 6
first=$(wrench)
echo "  cursor after the first click: $first"
case "$first" in
    *"rod 0,100,60"*) ;;
    *) fail "the first rod click did not store the rod position: $first" ;;
esac

echo "  far orb click: $(click_north 0 100 66)"
ctl wait 6
consumed=$(wrench)
echo "  cursor after the refused click: $consumed"
case "$consumed" in
    *"rod none"*) ;;
    *) fail "the refused click did not consume the stored rod position: $consumed" ;;
esac
refused=$(info 0 100 60)
echo "  rod after the refused link: $refused"
[ "$(orb_field "$refused")" = "no orb" ] ||
    fail "an out-of-range orb linked the rod: $refused"
[ "$(time_of "$refused")" -gt "$before_t" ] ||
    fail "the refusal readout is stale: $before_t -> $(time_of "$refused")"

echo "== slice 21: the placement search picks the first orb; LINK overrides it =="
# Orb A is first in `BlockPos.betweenClosed` order (x=0 before the clicked orb's x=1);
# the wrench must still link to orb B, the one actually clicked.
ctl command "setblock 0 100 66 minecraft:air" >/dev/null
ctl command "setblock 0 100 61 powah:energizing_orb" >/dev/null
ctl command "setblock 1 100 60 powah:energizing_orb" >/dev/null
ctl command "setblock 0 100 60 minecraft:air" >/dev/null
ctl command "setblock 0 100 60 powah:energizing_rod_starter" >/dev/null
ctl wait 8
auto=$(info 0 100 60)
auto_t=$(time_of "$auto")
echo "  placement-search link: $auto"
[ "$(orb_field "$auto")" = "orb 0,100,61" ] ||
    fail "the placement search did not pick the first orb: $auto"

echo "  rod click: $(click_north 0 100 60)"
ctl wait 4
echo "  clicked orb click: $(click_north 1 100 60)"
ctl wait 8
linked=$(info 0 100 60)
echo "  rod after the link: $linked"
linked_t=$(time_of "$linked")
[ -n "$linked_t" ] && [ "$linked_t" -gt "$auto_t" ] ||
    fail "the linked readout is stale: $auto_t -> $linked_t"
linked_orb=$(orb_field "$linked")
if [ "$SELFTEST" = link ]; then
    # The inverted half: on a correct package the override names orb B, so demanding
    # the placement search's orb A must fail here (EXIT:1).
    [ "$linked_orb" = "orb 0,100,61" ] ||
        fail "SELFTEST flipped: the wrench left the rod on $linked_orb, as it should"
else
    [ "$linked_orb" = "orb 1,100,60" ] ||
        fail "the wrench did not override the placement search's orb: $linked"
fi

if [ "$SELFTEST" != link ]; then
    echo "== slice 21: the rod then fills the orb it was linked to =="
    say "powah set 10000 0 100 60"
    ctl command "item replace block 1 100 60 container.1 with minecraft:iron_ingot" >/dev/null
    ctl command "item replace block 1 100 60 container.2 with minecraft:gold_ingot" >/dev/null
    # The 100 FE/t rod needs up to 100 ticks for the 10,000 FE recipe; a chat round trip
    # is ~20-40 ticks, so poll the orb until it completes rather than guessing a wait.
    orb_after=$(info 1 100 60)
    for _ in 1 2 3 4 5 6; do
        case "$orb_after" in
            *"no recipe"*) break ;;
        esac
        ctl wait 30
        orb_after=$(info 1 100 60)
    done
    filled=$(info 0 100 60)
    echo "  rod after filling: $filled"
    echo "  linked orb after filling: $orb_after"
    [ "$(energy_of "$filled")" -lt 10000 ] ||
        fail "the linked rod did not spend FE into its orb: $filled"
    case "$orb_after" in
        *"no recipe"*) ;;
        *) fail "the linked orb did not complete its recipe: $orb_after" ;;
    esac

    echo "== slice 21: a rod clicked twice still stores its own position =="
    click_north 0 100 60 >/dev/null
    ctl wait 3
    click_north 0 100 60 >/dev/null
    ctl wait 6
    twice=$(wrench)
    echo "  cursor after two rod clicks: $twice"
    case "$twice" in
        *"rod 0,100,60"*) ;;
        *) fail "two rod clicks did not store the rod position: $twice" ;;
    esac

    echo "== slice 21: an unmatched orb click changes nothing on the rod =="
    # The two rod clicks left a rod position stored, so clear the cursor with a matched
    # orb click first; then an orb click with nothing stored is upstream's LINK_ORB_POS
    # first half: it stores the orb on the wrench and leaves the rod's link alone.
    click_north 1 100 60 >/dev/null
    ctl wait 4
    unmatched_before=$(info 0 100 60)
    click_north 1 100 60 >/dev/null
    ctl wait 6
    unmatched_cursor=$(wrench)
    unmatched_after=$(info 0 100 60)
    echo "  cursor after the unmatched orb click: $unmatched_cursor"
    echo "  rod before: $unmatched_before"
    echo "  rod after:  $unmatched_after"
    case "$unmatched_cursor" in
        *"orb 1,100,60"*) ;;
        *) fail "the unmatched orb click did not store the orb position: $unmatched_cursor" ;;
    esac
    [ "$(orb_field "$unmatched_after")" = "$(orb_field "$unmatched_before")" ] ||
        fail "an unmatched orb click changed the rod's link: $unmatched_after"
    [ "$(time_of "$unmatched_after")" -gt "$(time_of "$unmatched_before")" ] ||
        fail "the unmatched readout is stale"

    echo "== slice 21: CONFIG and ROTATE still work =="
    say "powah wrench mode config"
    cmode=$(mode_line)
    case "$cmode" in
        *"mode: config"*) ;;
        *) fail "CONFIG was not reselected: $cmode" ;;
    esac
    ctl command "setblock 4 100 60 powah:energy_cell_starter" >/dev/null
    ctl wait 6
    cfg_before=$(side 4 100 60)
    click_north 4 100 60 >/dev/null
    ctl wait 6
    cfg_after=$(side 4 100 60)
    echo "  cell north before/after: $(side_field "$cfg_before" north) -> $(side_field "$cfg_after" north)"
    [ "$(side_field "$cfg_after" north)" = extract ] ||
        fail "the CONFIG face cycle regressed: $cfg_after"

    say "powah wrench mode rotate"
    rmode=$(mode_line)
    case "$rmode" in
        *"mode: rotate"*) ;;
        *) fail "ROTATE was not reselected: $rmode" ;;
    esac
    ctl command "setblock 6 100 60 powah:energy_hopper_starter[facing=north]" >/dev/null
    ctl wait 6
    rot_before=$(info 6 100 60)
    click_north 6 100 60 >/dev/null
    ctl wait 6
    rot_after=$(info 6 100 60)
    echo "  hopper facing before/after: $(facing_field "$rot_before") -> $(facing_field "$rot_after")"
    [ "$(facing_field "$rot_after")" = south ] ||
        fail "the ROTATE mode regressed: $rot_after"
fi

held=$(ctl inventory | jq -r '.counts["powah:wrench"] // 0')
[ "$held" -ge 1 ] || fail "the wrench was consumed: counts $held"

echo "PASS: the operator selected LINK; an out-of-range orb was refused and consumed the click; a rod clicked twice stored its own position; an unmatched orb click left the rod's link alone; the wrench overrode the placement search with the exact clicked orb which the rod then filled; CONFIG and ROTATE still work; the item was kept"
