#!/usr/bin/env python3
"""Carry slice 11's content out of Powah 6.2.10's ATM10 jar into this package.

Slice 11 carries the **ender cell** and the **ender gate**, seven tiers of each: two block families
whose buffer is not their own but one shared channel per owner (`EnderNetwork`). The blockstates,
models, item models, textures, loot and unlock advancements are in the raw carry; the 14 recipes
are carried from the carry too (its importer already rewrote them into the pin's string-ingredient
form), and the cell and gate recipes' `ender_core` is an **energizing** recipe this package makes
in its orb, so that item comes along.

    python3 tools/carry_slice11.py           # write the files
    python3 tools/carry_slice11.py --check   # verify the tree already matches

What is written rather than carried, and why:

* `data/powah/horizon/block/ender_cell_*.json` — the carry's guess is a bare `minecraft:stone`
  definition. Upstream's `Blcks.ENDER_CELL` is `Properties.metalNoSolid(2.0, 20.0)` with no state
  of its own, so the definition is the package's plain metal block (2.0 / 20.0, iron sounds,
  `occludes: false`, stack 1).
* `data/powah/horizon/block/ender_gate_*.json` — the same metal block with upstream's six-value
  `facing` (`EnderGateBlock.getFacing()` is `Facing.ALL`), default `north`, a 4x4x0.5-pixel plate
  on the face it names (`EnderGateBlock.SHAPES`) and **no collision** (`noCollission()`).
* the `ender_cell_*_2` / `ender_gate_*_2` recipes are **not** carried: each is the jar's own
  datagen self-recipe (one cell into one cell), which changes nothing — the ender item carries no
  data for it to preserve (`keepEnergy()` is false), so crafting it returns the same stack. See
  `notes.md`, slice 11.

No asset or model is invented: every copied file is byte-for-byte the carry's, and the carry's own
blockstates already draw both families.
"""
from __future__ import annotations

import json
import sys
from pathlib import Path

PORT = Path(__file__).resolve().parents[1]
CARRY = PORT.parent / "_shared" / "carry-work" / "powah" / "carry"

TIERS = ["starter", "basic", "hardened", "blazing", "niotic", "spirited", "nitro"]
CELLS = [f"ender_cell_{tier}" for tier in TIERS]
GATES = [f"ender_gate_{tier}" for tier in TIERS]

TAB = "data/powah/horizon/creative_tab/machines.json"
MINEABLE = "data/minecraft/tags/block/mineable/pickaxe.json"
NEEDS_IRON = "data/minecraft/tags/block/needs_iron_tool.json"
TAGS = [
    "data/powah/tags/block/ender_cells.json",
    "data/powah/tags/item/ender_cells.json",
    "data/powah/tags/block/ender_gates.json",
    "data/powah/tags/item/ender_gates.json",
]
# The models both families' blockstates and item models hang off: the cell's and the gate's shared
# bases plus one model per tier.
BLOCK_MODELS = ["ender_cell", "ender_gate"]
ITEM_MODELS = ["ender_core"]
# Upstream's `EnderGateBlock.SHAPES`, in pixels of sixteen: a 4x4x0.5 plate on the face it names.
PLATE = 0.25  # 4 / 16
HALF_PIXEL = 0.03125  # 0.5 / 16
FACINGS = {
    "down": [[0.375, 0.0, 0.375, 0.625, HALF_PIXEL, 0.625]],
    "up": [[0.375, 1.0 - HALF_PIXEL, 0.375, 0.625, 1.0, 0.625]],
    "north": [[0.375, 0.375, 0.0, 0.625, 0.625, HALF_PIXEL]],
    "south": [[0.375, 0.375, 1.0 - HALF_PIXEL, 0.625, 0.625, 1.0]],
    "west": [[0.0, 0.375, 0.375, HALF_PIXEL, 0.625, 0.625]],
    "east": [[1.0 - HALF_PIXEL, 0.375, 0.375, 1.0, 0.625, 0.625]],
}


def metal() -> dict:
    """Upstream's `Properties.metalNoSolid(2.0, 20.0)`: a metal block that does not occlude."""
    return {
        "hardness": 2.0,
        "explosion_resistance": 20.0,
        "sounds": "minecraft:iron_block",
        "requires_tool": True,
        "occludes": False,
    }


def cell_definition() -> dict:
    """An ender cell: the plain metal block, no state of its own (`EnderCellBlock`)."""
    return {
        "template": "minecraft:iron_block",
        "properties": metal(),
        "item": True,
        "components": {"minecraft:max_stack_size": 1},
    }


def gate_definition() -> dict:
    """An ender gate: the metal block with `Facing.ALL`, a plate shape per face and no collision."""
    return {
        "template": "minecraft:iron_block",
        "properties": metal(),
        "state_properties": {
            "facing": {
                "type": "enum",
                "values": ["down", "up", "north", "south", "west", "east"],
            }
        },
        "default_state": {"facing": "north"},
        "shapes": {"property": "facing", "values": FACINGS},
        "collision_shapes": {"default": []},
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

    # The shared bases and the per-tier block models.
    for model in BLOCK_MODELS:
        copy(f"assets/powah/models/block/{model}.json")
    for model in ITEM_MODELS:
        copy(f"assets/powah/models/item/{model}.json")

    # The cell and gate textures' shared pieces.
    copy("assets/powah/textures/block/ender_cell_core.png")
    copy("assets/powah/textures/block/ender_gate.png")
    copy("assets/powah/textures/item/ender_core.png")
    copy("assets/powah/textures/item/ender_core.png.mcmeta")

    # `powah:ender_core`: the energizing recipe's item (slice 11 adds its row to `runtime::energizing`).
    copy("assets/powah/items/ender_core.json")
    copy("data/powah/horizon/item/ender_core.json")

    for block, definition in [
        *((cell, cell_definition()) for cell in CELLS),
        *((gate, gate_definition()) for gate in GATES),
    ]:
        tier = block.rsplit("_", 1)[1]
        family = "ender_cell" if block.startswith("ender_cell") else "ender_gate"
        copy(f"assets/powah/blockstates/{block}.json")
        copy(f"assets/powah/models/block/{block}.json")
        copy(f"assets/powah/models/item/{block}.json")
        copy(f"assets/powah/items/{block}.json")
        copy(f"data/powah/loot_table/blocks/{block}.json")
        copy(f"data/powah/advancement/recipes/misc/crafting/{block}.json")
        copy(f"data/powah/recipe/crafting/{block}.json")
        # A cell tier has its own animated block texture; a gate's body is the shared
        # `ender_gate.png` and its tier is the `_ov` overlay every machine tier already carried.
        if family == "ender_cell":
            copy(f"assets/powah/textures/block/{family}_{tier}.png")
            copy(f"assets/powah/textures/block/{family}_{tier}.png.mcmeta")
        else:
            copy(f"assets/powah/textures/item/ender_gate_{tier}.png")
        write(f"data/powah/horizon/block/{block}.json", definition)

    for relative in TAGS:
        copy(relative)

    # The creative tab: the two ladders and the core they are built from follow the solar panels,
    # beside the storage blocks they belong with.
    tab = json.loads((PORT / TAB).read_text())
    anchor = tab["items"].index("powah:solar_panel_nitro")
    for index, entry in enumerate(
        [*(f"powah:{cell}" for cell in CELLS), *(f"powah:{gate}" for gate in GATES), "powah:ender_core"]
    ):
        if entry not in tab["items"]:
            tab["items"].insert(anchor + 1 + index, entry)
    write(TAB, tab)

    for relative in (MINEABLE, NEEDS_IRON):
        tag = json.loads((PORT / relative).read_text())
        for block in [*CELLS, *GATES]:
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
            print(f"slice 11 content is stale ({len(stale)} file(s)):", file=sys.stderr)
            for line in stale:
                print(f"  {line}", file=sys.stderr)
            return 1
        print(f"slice 11 content is reproducible: {len(files)} files match")
        return 0
    print(f"wrote {len(files)} files for slice 11")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
