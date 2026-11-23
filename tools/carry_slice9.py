#!/usr/bin/env python3
"""Carry slice 9's content out of Powah 6.2.10's ATM10 jar into this package.

Slice 9 carries the **energy hopper**, the seven tiers of it: the charging half of the item-energy
family — a storage block that charges the items in the container on the face its `facing` names. Its
assets, models and loot are in the raw carry; its recipes are read from the jar itself (the carry's raw
import keeps them in the jar's object-ingredient shape, which this pin's loader drops).

    python3 tools/carry_slice9.py           # write the files
    python3 tools/carry_slice9.py --check   # verify the tree already matches

What is patched, and why:

* `data/powah/horizon/block/*.json` — the carry's guess is `minecraft:carved_pumpkin`, a four-facing
  block; upstream's `AbstractBlock` places an energy hopper with `Facing.ALL` (six values, default
  `north`) and `Properties.metalNoSolid(2.0f, 20.0f)`, so the definition declares its own `facing`
  enum of six values on the plain `minecraft:iron_block` template, as the orb's does.
* `data/powah/recipe/crafting/*.json` — the pin's loader needs the string ingredient form, so the
  jar's object ingredients are rewritten.
* tags and the creative tab — the seven ids join the families they belong to.

No asset or model is invented: every copied file is byte-for-byte the carry's, and the carry's own
blockstate already draws all six facings.
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
HOPPERS = [f"energy_hopper_{tier}" for tier in TIERS]

TAB = "data/powah/horizon/creative_tab/machines.json"
MINEABLE = "data/minecraft/tags/block/mineable/pickaxe.json"
NEEDS_IRON = "data/minecraft/tags/block/needs_iron_tool.json"
TAGS = [
    "data/powah/tags/block/energy_hoppers.json",
    "data/powah/tags/item/energy_hoppers.json",
]
# The block model blockstates are drawn against, and the three textures every tier's model names
# beside its own `<tier>_ov.png` overlay (already carried).
BASE_MODEL = "assets/powah/models/block/energy_hopper.json"
TEXTURES = [
    "assets/powah/textures/block/energy_hopper_side.png",
    "assets/powah/textures/block/energy_hopper_back.png",
    "assets/powah/textures/block/energy_hopper_pointer.png",
]
# `Tier.getNormalVariants`, the six directions `Facing.ALL` gives the block state.
FACINGS = ["down", "up", "north", "south", "west", "east"]


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


def definition() -> dict:
    """The package's own hopper block: the orb's plain template with this block's own `facing`.

    Upstream's `Blcks.ENERGY_HOPPER` is `Properties.metalNoSolid(2.0f, 20.0f)` and
    `EnergyHopperBlock.getFacing()` is `Facing.ALL`; `AbstractBlock.registerDefaultState` sets
    `facing` to `north`, and `postTick` reads it to pick the container it charges.
    """
    return {
        "template": "minecraft:iron_block",
        "properties": {
            "hardness": 2.0,
            "explosion_resistance": 20.0,
            "sounds": "minecraft:iron_block",
            "requires_tool": True,
            "occludes": False,
        },
        "state_properties": {
            "facing": {"type": "enum", "values": FACINGS},
        },
        "default_state": {"facing": "north"},
        "item": True,
        "components": {"minecraft:max_stack_size": 1},
    }


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

    copy(BASE_MODEL)
    for texture in TEXTURES:
        copy(texture)
    for block in HOPPERS:
        copy(f"assets/powah/models/block/{block}.json")
        copy(f"assets/powah/models/item/{block}.json")
        copy(f"assets/powah/items/{block}.json")
        copy(f"assets/powah/blockstates/{block}.json")
        copy(f"data/powah/loot_table/blocks/{block}.json")
        copy(f"data/powah/advancement/recipes/misc/crafting/{block}.json")
        write(f"data/powah/horizon/block/{block}.json", definition())
        # The recipe's own id is the block's, as the jar has it.
        write(f"data/powah/recipe/crafting/{block}.json", jar_recipe(block))

    for relative in TAGS:
        copy(relative)

    # The creative tab: the seven hoppers follow the discharger row, beside the machines they
    # belong with.
    tab = json.loads((PORT / TAB).read_text())
    anchor = tab["items"].index("powah:energy_discharger_nitro")
    for index, block in enumerate(HOPPERS):
        entry = f"powah:{block}"
        if entry not in tab["items"]:
            tab["items"].insert(anchor + 1 + index, entry)
    write(TAB, tab)

    for relative in (MINEABLE, NEEDS_IRON):
        tag = json.loads((PORT / relative).read_text())
        for block in HOPPERS:
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
            print(f"slice 9 content is stale ({len(stale)} file(s)):", file=sys.stderr)
            for line in stale:
                print(f"  {line}", file=sys.stderr)
            return 1
        print(f"slice 9 content is reproducible: {len(files)} files match")
        return 0
    print(f"wrote {len(files)} files for slice 9")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
