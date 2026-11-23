#!/usr/bin/env python3
"""Carry slice 10's content out of Powah 6.2.10's ATM10 jar into this package.

Slice 10 carries the **solar panel**, the seven tiers of it: a generator whose only fuel is the sky
above it. Its blockstates, models, item models, textures and loot are in the raw carry; its recipes
are read from the jar itself (the carry's raw import keeps them in the jar's object-ingredient
shape, which this pin's loader drops), and the starter panel's recipe needs the jar's
`photoelectric_pane`, so that item and its recipe come along.

    python3 tools/carry_slice10.py           # write the files
    python3 tools/carry_slice10.py --check   # verify the tree already matches

What is patched, and why:

* `data/powah/horizon/block/*.json` — the carry's guess is `minecraft:oak_fence`, a full-size fence
  with a post's collision. Upstream's `SolarBlock` is a 1-pixel plate (`box(0,0,0,16,1,16)`) with
  four horizontal booleans (`north`/`east`/`south`/`west`, `true` when no panel of the same tier is
  attached, so the frame is drawn) and `Properties.metal(2.0, 20.0)`. The definition declares those
  four on the plain `minecraft:iron_block` template and its 1-pixel outline and collision.
* `data/powah/recipe/crafting/*.json` — the pin's loader needs the string ingredient form, so the
  jar's object ingredients are rewritten; the jar's one NeoForge tag (`#c:glass_panes` in the
  photoelectric pane) becomes the vanilla member it resolves to (`minecraft:glass_pane`), in the
  recipe and in its unlock advancement.
* tags and the creative tab — the ids join the families they belong to.

No asset or model is invented: every copied file is byte-for-byte the carry's, and the carry's own
blockstate already draws the four frames.
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
PANELS = [f"solar_panel_{tier}" for tier in TIERS]

TAB = "data/powah/horizon/creative_tab/machines.json"
MINEABLE = "data/minecraft/tags/block/mineable/pickaxe.json"
NEEDS_IRON = "data/minecraft/tags/block/needs_iron_tool.json"
TAGS = [
    "data/powah/tags/block/solar_panels.json",
    "data/powah/tags/item/solar_panels.json",
]
# The models every tier's blockstate and item model hang off (the starter has its own frame model;
# the other six share `solar_panel_frame_`, which is what their blockstates name).
BLOCK_MODELS = [
    "solar_panel",
    "solar_panel_frame",
    "solar_panel_frame_",
    "solar_panel_starter_frame",
]
ITEM_MODELS = ["solar_panel", "solar_panel_"]
# Upstream's `SolarBlock.SHAPE`: `box(0,0,0,16,1,16)`.
PLATE = [[0.0, 0.0, 0.0, 1.0, 0.0625, 1.0]]
# The jar's one tag ingredient this slice meets, and the vanilla member it resolves to in 26.3.
TAGS_26_3 = {"#c:glass_panes": "minecraft:glass_pane"}


def ingredient(value) -> str:
    """One recipe ingredient in the pin's string form."""
    if isinstance(value, str):
        if value in TAGS_26_3:
            return TAGS_26_3[value]
        if value.startswith("#"):
            raise SystemExit(f"a tag ingredient this slice does not carry: {value}")
        return value
    if "item" in value:
        return value["item"]
    if "tag" in value:
        tag = value["tag"]
        if tag in TAGS_26_3:
            return TAGS_26_3[tag]
        if f"#{tag}" in TAGS_26_3:
            return TAGS_26_3[f"#{tag}"]
        raise SystemExit(f"a tag ingredient this slice does not carry: {tag}")
    raise SystemExit(f"an ingredient shape this slice does not carry: {value}")


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
    """The package's own solar panel block: a one-pixel plate on the plain metal template.

    Upstream's `Blcks.SOLAR_PANEL` is `Properties.metal(2.0f, 20.0f)` with the four frame booleans
    `SolarBlock` creates; a lone panel has no attached neighbour, so every side starts framed.
    """
    references = {
        "north": {"type": "boolean"},
        "south": {"type": "boolean"},
        "west": {"type": "boolean"},
        "east": {"type": "boolean"},
    }
    return {
        "template": "minecraft:iron_block",
        "properties": {
            "hardness": 2.0,
            "explosion_resistance": 20.0,
            "sounds": "minecraft:iron_block",
            "requires_tool": True,
            "occludes": False,
        },
        "state_properties": references,
        "default_state": {name: "true" for name in references},
        "shapes": {"default": PLATE},
        "collision_shapes": {"default": PLATE},
        "item": True,
        "components": {"minecraft:max_stack_size": 1},
    }


def replaced(value, mapping: dict[str, str]):
    """`value` with every string the mapping names replaced."""
    if isinstance(value, str):
        return mapping.get(value, value)
    if isinstance(value, list):
        return [replaced(entry, mapping) for entry in value]
    if isinstance(value, dict):
        return {key: replaced(entry, mapping) for key, entry in value.items()}
    return value


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

    for texture in (
        "solar_panel_top",
        "solar_panel_starter_top",
        *(f"solar_panel_{tier}_frame" for tier in TIERS),
    ):
        copy(f"assets/powah/textures/block/{texture}.png")
    for model in BLOCK_MODELS:
        copy(f"assets/powah/models/block/{model}.json")
    for model in ITEM_MODELS:
        copy(f"assets/powah/models/item/{model}.json")
    for panel in PANELS:
        copy(f"assets/powah/models/block/{panel}.json")
        copy(f"assets/powah/models/item/{panel}.json")
        copy(f"assets/powah/items/{panel}.json")
        copy(f"assets/powah/blockstates/{panel}.json")
        copy(f"data/powah/loot_table/blocks/{panel}.json")
        copy(f"data/powah/advancement/recipes/misc/crafting/{panel}.json")
        write(f"data/powah/horizon/block/{panel}.json", definition())
        # The recipe's own id is the block's, as the jar has it.
        write(f"data/powah/recipe/crafting/{panel}.json", jar_recipe(panel))

    # The starter panel's `p` is the photoelectric pane, and its recipe is the only place this
    # slice meets a NeoForge `c:` tag; the pane's own unlock advancement names the same tag.
    copy("assets/powah/models/item/photoelectric_pane.json")
    copy("assets/powah/items/photoelectric_pane.json")
    copy("data/powah/horizon/item/photoelectric_pane.json")
    write("data/powah/recipe/crafting/photoelectric_pane.json", jar_recipe("photoelectric_pane"))
    advancement = json.loads(
        (CARRY / "data/powah/advancement/recipes/misc/crafting/photoelectric_pane.json").read_text()
    )
    write(
        "data/powah/advancement/recipes/misc/crafting/photoelectric_pane.json",
        replaced(advancement, TAGS_26_3),
    )
    copy("assets/powah/textures/item/photoelectric_pane.png")

    for relative in TAGS:
        copy(relative)

    # The creative tab: the seven panels and the pane that feeds them follow the hopper row, beside
    # the generators they belong with.
    tab = json.loads((PORT / TAB).read_text())
    anchor = tab["items"].index("powah:energy_hopper_nitro")
    for index, entry in enumerate([*(f"powah:{panel}" for panel in PANELS),
                                  "powah:photoelectric_pane"]):
        if entry not in tab["items"]:
            tab["items"].insert(anchor + 1 + index, entry)
    write(TAB, tab)

    for relative in (MINEABLE, NEEDS_IRON):
        tag = json.loads((PORT / relative).read_text())
        for panel in PANELS:
            entry = f"powah:{panel}"
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
            print(f"slice 10 content is stale ({len(stale)} file(s)):", file=sys.stderr)
            for line in stale:
                print(f"  {line}", file=sys.stderr)
            return 1
        print(f"slice 10 content is reproducible: {len(files)} files match")
        return 0
    print(f"wrote {len(files)} files for slice 10")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
