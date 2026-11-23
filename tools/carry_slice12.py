#!/usr/bin/env python3
"""Carry slice 12's content out of Powah 6.2.10's ATM10 jar into this package.

Slice 12 carries the **player transmitter** (seven tiers), the **binding card** it takes, and the
three items its recipes are built from (`blank_card`, `aerial_pearl`, `player_aerial_pearl`): the
blockstates, models, item models, textures, loot and unlock advancements are in the raw carry; the
recipes come from the carry too (its importer already rewrote them into the pin's string-ingredient
form), and the jar's own `player_tranmitter_*` spelling is kept — that is the file name upstream's
datagen wrote, and renaming it would make the recipe a different one.

    python3 tools/carry_slice12.py           # write the files
    python3 tools/carry_slice12.py --check   # verify the tree already matches

What is written rather than carried, and why:

* `data/powah/horizon/block/player_transmitter_*.json` — the carry's guess is a bare
  `minecraft:stone` definition. Upstream's `Blcks.PLAYER_TRANSMITTER` is
  `Properties.metalNoSolid(2.0, 20.0)` with a `top` state of its own, and this slice carries the
  **bottom half only** (one block, no state): the definition is the package's plain metal block.
* `assets/powah/blockstates/player_transmitter_*.json` — the carry's file is a `top=false` /
  `top=true` pair; with no `top` state the state's name is empty, so the blockstate is one variant.
* `assets/powah/models/item/player_transmitter_*.json` — upstream's item model is the two-block
  structure (`powah:item/player_transmitter`, elements up to y=28). With one block placed, the item
  shows that block's own model: the tier's block model, with upstream's own `display` transforms
  (read from the carry's `models/item/player_transmitter.json`) so it still sits in the hand as it
  did.
* `assets/powah/models/item/binding_card.json` — the carry's file carries a `powah:bound` model
  `overrides` entry, which 26.3 moved to an item model property this slice does not register (the
  client half is a named gap). The model is the generated one without that override, and
  `binding_card_bound` is not carried.

No asset is invented: every copied file is byte-for-byte the carry's.

The recipes are the one thing read from the jar rather than the carry: the carry's importer skipped
this family's files (their `_2`-less names are new), and the jar's own recipe JSON is NeoForge's
`{"item": "…"}` ingredient form, which 26.3 replaced with the id string. `recipe_from_jar` reads the
jar's file and rewrites each ingredient object to the id it names, which is exactly the rewrite the
carry made for every earlier slice; `--check` proves the result is stable.
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
TRANSMITTERS = [f"player_transmitter_{tier}" for tier in TIERS]
# The jar writes the recipes with that spelling (no second `s`), and the advancement beside them.
RECIPE_NAME = "player_tranmitter_{tier}"

TAB = "data/powah/horizon/creative_tab/machines.json"
MINEABLE = "data/minecraft/tags/block/mineable/pickaxe.json"
NEEDS_IRON = "data/minecraft/tags/block/needs_iron_tool.json"
TAGS = [
    "data/powah/tags/block/player_transmitters.json",
    "data/powah/tags/item/player_transmitters.json",
]
# The card and the three items its recipes are built from: id, item definition, model texture.
CARDS = [
    ("binding_card", {"template": "minecraft:stick", "max_stack_size": 1}),
    ("blank_card", {"template": "minecraft:stick"}),
    ("aerial_pearl", {"template": "minecraft:stick"}),
    ("player_aerial_pearl", {"template": "minecraft:stick"}),
]


def metal() -> dict:
    """Upstream's `Properties.metalNoSolid(2.0, 20.0)`: a metal block that does not occlude."""
    return {
        "hardness": 2.0,
        "explosion_resistance": 20.0,
        "sounds": "minecraft:iron_block",
        "requires_tool": True,
        "occludes": False,
    }


def transmitter_definition() -> dict:
    """A player transmitter: the plain metal block, one slot, no state of its own."""
    return {
        "template": "minecraft:iron_block",
        "properties": metal(),
        "item": True,
        "components": {"minecraft:max_stack_size": 1},
    }


def item_model(tier: str) -> dict:
    """The item shows the placed block's own model, with upstream's display transforms."""
    carried = json.loads((CARRY / "assets/powah/models/item/player_transmitter.json").read_text())
    return {
        "parent": f"powah:block/player_transmitter_{tier}",
        "display": carried["display"],
    }


def card_model(name: str) -> dict:
    """The generated item model of a plain item: its own texture, no model override."""
    return {
        "parent": "item/generated",
        "textures": {"layer0": f"powah:item/{name}"},
    }


def ingredient(value):
    """A jar ingredient in 26.3's form: the id string its `{"item": …}` names."""
    if isinstance(value, dict):
        if set(value) == {"item"}:
            return value["item"]
        raise SystemExit(f"an ingredient this slice does not know: {value!r}")
    if isinstance(value, list):
        return [ingredient(entry) for entry in value]
    return value


def recipe_from_jar(path: str) -> dict:
    """The jar's recipe at `path`, with every ingredient rewritten into the pin's dialect."""
    with zipfile.ZipFile(JAR) as jar:
        recipe = json.loads(jar.read(path))
    recipe["key"] = {symbol: ingredient(value) for symbol, value in recipe["key"].items()}
    return recipe


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

    # The shared transmitter body: the block model every tier's model parents, its particle
    # texture and the core every tier's `core` face draws.
    copy("assets/powah/models/block/player_transmitter.json")
    copy("assets/powah/textures/block/player_transmitter.png")
    copy("assets/powah/textures/block/player_transmitter_core.png")

    for tier, block in zip(TIERS, TRANSMITTERS):
        copy(f"assets/powah/models/block/{block}.json")
        copy(f"assets/powah/textures/block/{block}_var.png")
        copy(f"assets/powah/items/{block}.json")
        copy(f"data/powah/loot_table/blocks/{block}.json")
        copy(f"data/powah/advancement/recipes/misc/crafting/{RECIPE_NAME.format(tier=tier)}.json")
        write(
            f"data/powah/recipe/crafting/{RECIPE_NAME.format(tier=tier)}.json",
            recipe_from_jar(
                f"data/powah/recipe/crafting/{RECIPE_NAME.format(tier=tier)}.json"
            ),
        )
        write(
            f"assets/powah/blockstates/{block}.json",
            {"variants": {"": {"model": f"powah:block/{block}"}}},
        )
        write(f"assets/powah/models/item/{block}.json", item_model(tier))
        write(f"data/powah/horizon/block/{block}.json", transmitter_definition())

    for name, definition in CARDS:
        copy(f"assets/powah/items/{name}.json")
        copy(f"assets/powah/textures/item/{name}.png")
        write(f"assets/powah/models/item/{name}.json", card_model(name))
        write(f"data/powah/horizon/item/{name}.json", definition)
    for name in ("binding_card", "blank_card", "aerial_pearl"):
        write(
            f"data/powah/recipe/crafting/{name}.json",
            recipe_from_jar(f"data/powah/recipe/crafting/{name}.json"),
        )
        copy(f"data/powah/advancement/recipes/misc/crafting/{name}.json")

    for relative in TAGS:
        copy(relative)

    # The creative tab: the ladder follows the hoppers (the storage devices), and the card and its
    # ingredient beside them.
    tab = json.loads((PORT / TAB).read_text())
    anchor = tab["items"].index("powah:energy_hopper_nitro")
    for index, entry in enumerate(
        [*(f"powah:{block}" for block in TRANSMITTERS), *(f"powah:{name}" for name, _ in CARDS)]
    ):
        if entry not in tab["items"]:
            tab["items"].insert(anchor + 1 + index, entry)
    write(TAB, tab)

    for relative in (MINEABLE, NEEDS_IRON):
        tag = json.loads((PORT / relative).read_text())
        for block in TRANSMITTERS:
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
            print(f"slice 12 content is stale ({len(stale)} file(s)):", file=sys.stderr)
            for line in stale:
                print(f"  {line}", file=sys.stderr)
            return 1
        print(f"slice 12 content is reproducible: {len(files)} files match")
        return 0
    print(f"wrote {len(files)} files for slice 12")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
