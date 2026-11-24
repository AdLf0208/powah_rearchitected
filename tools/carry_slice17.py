#!/usr/bin/env python3
"""Carry slice 17's creative energy cell out of Powah 6.2.10 into this package.

    python3 tools/carry_slice17.py           # write the files
    python3 tools/carry_slice17.py --check   # verify the tree already matches

The six carried files (blockstate, block/item models, item definition, block
texture, loot table) are copied byte-for-byte from the raw carry. The horizon
block definition is *written*: the carry's guess is `minecraft:stone`, while an
energy cell is upstream's `AbstractEnergyBlock` — `minecraft:iron_block` with
2.0/20.0 metal, `requires_tool`, `occludes: false`, stack 1, exactly the shape
the port's other seven cells use. The id joins the two vanilla tool tags and
the creative tab. There is no recipe: the jar has none for the creative tier.
No asset or model is invented.
"""
from __future__ import annotations

import sys
from pathlib import Path

PORT = Path(__file__).resolve().parents[1]
CARRY = PORT.parent / "_shared" / "carry-work" / "powah" / "carry"

ID = "energy_cell_creative"
CARRIED = [
    f"assets/powah/blockstates/{ID}.json",
    f"assets/powah/models/block/{ID}.json",
    f"assets/powah/models/item/{ID}.json",
    f"assets/powah/items/{ID}.json",
    f"assets/powah/textures/block/{ID}.png",
    f"data/powah/loot_table/blocks/{ID}.json",
]
BLOCK = f"data/powah/horizon/block/{ID}.json"
BLOCK_TEXT = """{
  "template": "minecraft:iron_block",
  "properties": {
    "hardness": 2.0,
    "explosion_resistance": 20.0,
    "sounds": "minecraft:iron_block",
    "requires_tool": true,
    "occludes": false
  },
  "item": true,
  "components": {
    "minecraft:max_stack_size": 1
  }
}
"""
LISTS = [
    "data/minecraft/tags/block/mineable/pickaxe.json",
    "data/minecraft/tags/block/needs_iron_tool.json",
    "data/powah/horizon/creative_tab/machines.json",
]
ENTRY = f'"powah:{ID}"'


def expected(path: str) -> bytes:
    if path == BLOCK:
        return BLOCK_TEXT.encode()
    return (CARRY / path).read_bytes()


def with_entry(path: str) -> bytes:
    """The list file with the creative cell appended once, formatting untouched."""
    text = (PORT / path).read_text()
    if ENTRY in text:
        return text.encode()
    lines = text.splitlines(keepends=True)
    for i in range(len(lines) - 1, -1, -1):
        if lines[i].lstrip().startswith('"'):
            comma = "" if lines[i].rstrip().endswith(",") else ","
            indent = lines[i][: len(lines[i]) - len(lines[i].lstrip())]
            lines.insert(i + 1, f"{indent}{ENTRY}\n")
            # The previous last entry now needs a comma.
            lines[i] = lines[i].rstrip("\n") + comma + "\n"
            break
    else:
        raise SystemExit(f"no array entries in {path}")
    return "".join(lines).encode()


def main() -> None:
    check = "--check" in sys.argv
    bad = []
    for path in CARRIED + [BLOCK]:
        want = expected(path)
        got = (PORT / path).read_bytes() if (PORT / path).exists() else b""
        if got != want:
            bad.append(path)
            if not check:
                (PORT / path).write_bytes(want)
    for path in LISTS:
        text = (PORT / path).read_text()
        if check:
            if text.count(ENTRY) != 1:
                bad.append(path)
            continue
        want = with_entry(path)
        if text.encode() != want:
            bad.append(path)
            (PORT / path).write_bytes(want)
    if check:
        if bad:
            raise SystemExit(f"slice 17 tree does not match: {', '.join(bad)}")
        print(f"slice 17: {len(CARRIED) + 1} carried/written files and {len(LISTS)} lists match")
    else:
        print(f"slice 17: wrote {len(CARRIED) + 1} files, updated {len(LISTS)} lists")


if __name__ == "__main__":
    main()
