#!/usr/bin/env python3
"""Carry slice 18's seven battery items out of Powah 6.2.10 into this package.

    python3 tools/carry_slice18.py           # write the files
    python3 tools/carry_slice18.py --check   # verify the tree already matches

Five files per tier (texture, item model, 26.3 item definition, shaped recipe,
recipe advancement) are copied byte-for-byte from the raw carry. The horizon
item declaration is *written*: upstream's `Itms.BATTERY` is
`new BatteryItem(new Item.Properties().stacksTo(1), variant)`, so the carry's
plain `{"template": "minecraft:stick"}` guess (a 64-stack) must carry
`max_stack_size: 1` instead. No asset, model, number or recipe is invented.
Lang already names all seven (`item.powah.battery_*`); the ids join the one
Powah creative tab. No tool tags: a battery is an item, not a block.
"""
from __future__ import annotations

import sys
from pathlib import Path

PORT = Path(__file__).resolve().parents[1]
CARRY = PORT.parent / "_shared" / "carry-work" / "powah" / "carry"

TIERS = ["starter", "basic", "hardened", "blazing", "niotic", "spirited", "nitro"]
CARRIED = [
    f"assets/powah/textures/item/battery_{tier}.png" for tier in TIERS
] + [
    f"assets/powah/models/item/battery_{tier}.json" for tier in TIERS
] + [
    f"assets/powah/items/battery_{tier}.json" for tier in TIERS
] + [
    f"data/powah/recipe/crafting/battery_{tier}.json" for tier in TIERS
] + [
    f"data/powah/advancement/recipes/misc/crafting/battery_{tier}.json"
    for tier in TIERS
]
ITEM = "data/powah/horizon/item/battery_{}.json"
ITEM_TEXT = """{
  "template": "minecraft:stick",
  "max_stack_size": 1
}
"""
TAB = "data/powah/horizon/creative_tab/machines.json"


def expected(path: str) -> bytes:
    if path.startswith("data/powah/horizon/item/battery_"):
        return ITEM_TEXT.encode()
    return (CARRY / path).read_bytes()


def with_tabs() -> bytes:
    """The creative tab with each battery id appended once, formatting untouched."""
    text = (PORT / TAB).read_text()
    for tier in TIERS:
        entry = f'"powah:battery_{tier}"'
        if entry in text:
            continue
        lines = text.splitlines(keepends=True)
        for i in range(len(lines) - 1, -1, -1):
            if lines[i].lstrip().startswith('"'):
                comma = "" if lines[i].rstrip().endswith(",") else ","
                indent = lines[i][: len(lines[i]) - len(lines[i].lstrip())]
                lines.insert(i + 1, f"{indent}{entry}\n")
                lines[i] = lines[i].rstrip("\n") + comma + "\n"
                break
        else:
            raise SystemExit(f"no array entries in {TAB}")
        text = "".join(lines)
    return text.encode()


def main() -> None:
    check = "--check" in sys.argv
    bad = []
    paths = CARRIED + [ITEM.format(tier) for tier in TIERS]
    for path in paths:
        want = expected(path)
        got = (PORT / path).read_bytes() if (PORT / path).exists() else b""
        if got != want:
            bad.append(path)
            if not check:
                (PORT / path).write_bytes(want)
    text = (PORT / TAB).read_text()
    if check:
        for tier in TIERS:
            if text.count(f'"powah:battery_{tier}"') != 1:
                bad.append(f"{TAB}: battery_{tier}")
        if bad:
            raise SystemExit(f"slice 18 tree does not match: {', '.join(bad)}")
        print(f"slice 18: {len(paths)} carried/written files and the creative tab match")
    else:
        want = with_tabs()
        if text.encode() != want:
            (PORT / TAB).write_bytes(want)
        print(f"slice 18: wrote {len(paths)} files, updated the creative tab")


if __name__ == "__main__":
    main()
