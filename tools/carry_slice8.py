#!/usr/bin/env python3
"""Carry slice 8's content out of Powah 6.2.10's ATM10 jar into this package.

Slice 8 carries the **energy discharger**, the seven tiers of it: the mirror of the
energy cell the item-energy slice left as its other half. Its assets, models and
loot are in the raw carry; its recipes are read from the jar itself (the carry's
raw import keeps them in the jar's object-ingredient shape, which this pin's
loader drops).

    python3 tools/carry_slice8.py           # write the files
    python3 tools/carry_slice8.py --check   # verify the tree already matches

What is patched, and why:

* `data/powah/horizon/block/*.json` — the carry's guess is not this port's shape:
  a discharger is a `minecraft:furnace` definition (horizontal `facing`, 2.0/20.0
  metal, `occludes: false`, stack 1), as upstream's `Facing.HORIZONTAL` and
  `Properties.metalNoSolid(2.0f, 20.0f)` say.
* `assets/powah/blockstates/*.json` — the carry's file names
  `lollipop:block/empty` for `facing=up/down` (another mod's model) and leaves no
  room for the `lit` state the furnace template adds; the four horizontal facings
  x `lit` are written here, exactly as the port's furnator blockstate is.
* `data/powah/recipe/crafting/*.json` — the pin's loader needs the string
  ingredient form, so the jar's object ingredients are rewritten.
* tags and the creative tab — the seven ids join the families they belong to.

No asset or model is invented: every copied file is byte-for-byte the carry's.
"""
from __future__ import annotations

import json
import sys
import zipfile
from pathlib import Path

PORT = Path(__file__).resolve().parents[1]
CARRY = PORT.parent / "_shared" / "carry-work" / "powah" / "carry"
JAR = PORT.parent / "_shared" / "carry-work" / "powah" / "Powah-6.2.10.jar"

TIERS = ["starter", "basic", "hardened", "blazing", "niotic", "spirited", "nitro"]
DISCHARGERS = [f"energy_discharger_{tier}" for tier in TIERS]

TAB = "data/powah/horizon/creative_tab/machines.json"
MINEABLE = "data/minecraft/tags/block/mineable/pickaxe.json"
NEEDS_IRON = "data/minecraft/tags/block/needs_iron_tool.json"
TAGS = [
    "data/powah/tags/block/energy_dischargers.json",
    "data/powah/tags/item/energy_dischargers.json",
]
# The block model blockstates are drawn against, and the two textures every tier's
# model names beside its own `<tier>_ov.png` overlay (already carried).
BASE_MODEL = "assets/powah/models/block/energy_discharger.json"
TEXTURES = [
    "assets/powah/textures/block/energy_discharger_side.png",
    "assets/powah/textures/block/energy_discharger_comps.png",
]


def ingredient(value) -> str:
    """One recipe ingredient in the pin's string form."""
    if isinstance(value, str):
        if value.startswith("#"):
            raise SystemExit(f"a tag ingredient this slice does not carry: {value}")
        return value
    if "item" in value:
        return value["item"]
    raise SystemExit(f"a tag ingredient this slice does not carry: {value}")


def shaped(raw: dict) -> dict:
    """The jar's recipe re-shaped for this pin's loader."""
    out = {"category": raw.get("category", "misc"), "type": raw["type"]}
    out["key"] = {symbol: ingredient(value) for symbol, value in raw["key"].items()}
    out["pattern"] = raw["pattern"]
    out["result"] = raw["result"]
    return out


def jar_recipe(recipe_id: str) -> dict:
    with zipfile.ZipFile(JAR) as jar:
        raw = json.loads(jar.read(f"data/powah/recipe/crafting/{recipe_id}.json"))
    return shaped(raw)


def blockstate(block: str) -> dict:
    """The four horizontal facings the furnace template has, each with `lit` (never set:
    upstream's discharger has no burning state, so both variants are the same model)."""
    variants = {}
    for facing, y in (("north", 0), ("south", 180), ("west", 270), ("east", 90)):
        for lit in ("false", "true"):
            model = {"model": f"powah:block/{block}"}
            if y:
                model["y"] = y
            variants[f"facing={facing},lit={lit}"] = model
    return {"variants": variants}


def outputs() -> dict[str, bytes]:
    """Every file this slice writes, as `path -> bytes`."""
    files: dict[str, bytes] = {}

    def copy(relative: str) -> None:
        source = CARRY / relative
        if not source.is_file():
            raise SystemExit(f"missing in the carry: {relative}")
        files[relative] = source.read_bytes()

    def write(relative: str, value) -> None:
        files[relative] = (json.dumps(value, indent=2) + "\n").encode()

    # The package's own furnator definition: the same template (horizontal facing,
    # metal, no occlusion, stack 1) with the discharger's id.
    definition = json.loads(
        (PORT / "data/powah/horizon/block/furnator_starter.json").read_text()
    )

    copy(BASE_MODEL)
    for texture in TEXTURES:
        copy(texture)
    for block in DISCHARGERS:
        copy(f"assets/powah/models/block/{block}.json")
        copy(f"assets/powah/models/item/{block}.json")
        copy(f"assets/powah/items/{block}.json")
        copy(f"data/powah/loot_table/blocks/{block}.json")
        copy(f"data/powah/advancement/recipes/misc/crafting/{block}.json")
        write(f"data/powah/horizon/block/{block}.json", definition)
        write(f"assets/powah/blockstates/{block}.json", blockstate(block))
        # The recipe's own id is the block's, as the jar has it.
        write(f"data/powah/recipe/crafting/{block}.json", jar_recipe(block))

    for relative in TAGS:
        copy(relative)

    # The creative tab: the seven dischargers follow the cable row, beside the cells
    # and cables they belong with.
    tab = json.loads((PORT / TAB).read_text())
    anchor = tab["items"].index("powah:energy_cable_nitro")
    for index, block in enumerate(DISCHARGERS):
        entry = f"powah:{block}"
        if entry not in tab["items"]:
            tab["items"].insert(anchor + 1 + index, entry)
    write(TAB, tab)

    for relative in (MINEABLE, NEEDS_IRON):
        tag = json.loads((PORT / relative).read_text())
        for block in DISCHARGERS:
            entry = f"powah:{block}"
            if entry not in tag["values"]:
                tag["values"].append(entry)
        write(relative, tag)
    return files


def main() -> int:
    check = "--check" in sys.argv[1:]
    files = outputs()
    stale = []
    for relative, content in sorted(files.items()):
        path = PORT / relative
        if check:
            if not path.is_file():
                stale.append(f"missing {relative}")
            elif path.read_bytes() != content:
                stale.append(f"differs {relative}")
        else:
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(content)
    if check:
        if stale:
            print(f"slice 8 content is stale ({len(stale)} file(s)):", file=sys.stderr)
            for line in stale:
                print(f"  {line}", file=sys.stderr)
            return 1
        print(f"slice 8 content is reproducible: {len(files)} files match")
        return 0
    print(f"wrote {len(files)} files for slice 8")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
