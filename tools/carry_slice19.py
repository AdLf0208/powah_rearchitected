#!/usr/bin/env python3
"""Carry slice 19's wrench out of Powah 6.2.10 into this package.

    python3 tools/carry_slice19.py           # write the files
    python3 tools/carry_slice19.py --check   # verify the tree already matches

The texture, item model, 26.3 item definition, shaped recipe and recipe advancement
are copied byte-for-byte from the raw carry. The horizon item declaration is
*written*: upstream's `Itms.WRENCH = new WrenchItem(new Item.Properties().stacksTo(1))`,
so the carry's plain `{"template": "minecraft:stick"}` guess (a 64-stack) must carry
`max_stack_size: 1` instead. Lang already names `item.powah.wrench`; the id joins the
one Powah creative tab. The `c:tools/wrench` tag is deliberately not carried: it is a
mod-compatibility tag with no effect in this port (no other carried recipe or hook
reads it), so carrying it would add an unprovable tag family. No model, number or
recipe is invented.
"""
from __future__ import annotations

import sys
from pathlib import Path

PORT = Path(__file__).resolve().parents[1]
CARRY = PORT.parent / "_shared" / "carry-work" / "powah" / "carry"

CARRIED = [
    "assets/powah/textures/item/wrench.png",
    "assets/powah/models/item/wrench.json",
    "assets/powah/items/wrench.json",
    "data/powah/recipe/crafting/wrench.json",
    "data/powah/advancement/recipes/misc/crafting/wrench.json",
]
ITEM = "data/powah/horizon/item/wrench.json"
ITEM_TEXT = """{
  "template": "minecraft:stick",
  "max_stack_size": 1
}
"""
TAB = "data/powah/horizon/creative_tab/machines.json"


def expected(path: str) -> bytes:
    if path == ITEM:
        return ITEM_TEXT.encode()
    return (CARRY / path).read_bytes()


def with_tab() -> bytes:
    """The creative tab with the wrench id appended once, formatting untouched."""
    text = (PORT / TAB).read_text()
    lines = text.splitlines(keepends=True)
    for i in range(len(lines) - 1, -1, -1):
        if lines[i].lstrip().startswith('"'):
            comma = "" if lines[i].rstrip().endswith(",") else ","
            indent = lines[i][: len(lines[i]) - len(lines[i].lstrip())]
            lines.insert(i + 1, f'{indent}"powah:wrench"\n')
            lines[i] = lines[i].rstrip("\n") + comma + "\n"
            break
    else:
        raise SystemExit(f"no array entries in {TAB}")
    return "".join(lines).encode()


def main() -> None:
    check = "--check" in sys.argv
    bad = []
    paths = CARRIED + [ITEM]
    for path in paths:
        want = expected(path)
        got = (PORT / path).read_bytes() if (PORT / path).exists() else b""
        if got != want:
            bad.append(path)
            if not check:
                (PORT / path).write_bytes(want)
    text = (PORT / TAB).read_text()
    if check:
        if text.count('"powah:wrench"') != 1:
            bad.append(f"{TAB}: wrench")
        if bad:
            raise SystemExit(f"slice 19 tree does not match: {', '.join(bad)}")
        print(f"slice 19: {len(paths)} carried/written files and the creative tab match")
    else:
        if text.count('"powah:wrench"') == 0:
            (PORT / TAB).write_bytes(with_tab())
        print(f"slice 19: wrote {len(paths)} files, updated the creative tab")


if __name__ == "__main__":
    main()
