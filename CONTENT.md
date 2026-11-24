# Powah — content (slices 1 and 2)

**Slice 21 (the wrench's LINK mode, 0.2.21) changed no file here**: it is Rust only — the
wrench, the rod and the orb all shipped in earlier slices (19 and 5).

The carried content of the starter Furnator, Energy Cell and Energy Cable (slice 1) and of the
dielectric/capacitor crafting family (slice 2). Since 0.2.0 the package root **is** the mod root
(`horizon-mod.toml` + `assets/` + `data/` beside `src/`), and `powah.hmod` is the one built
artifact.

**Slice 3 (the cable network, 0.2.3) changed no file here**: it is Rust only.
**Slice 4 (the energy cell ladder, 0.2.4)** adds the six higher tiers' blocks, blockstates, block and
item models, item definitions, textures, loot tables, the pickaxe/needs-iron tag entries, their
creative-tab rows, and the one craftable recipe (`energy_cell_basic`) with its advancement — 44 new
entries, for **134** in the package. The five higher cells' recipes are deliberately absent: their
inputs come from the unported Energizing Orb (see `notes.md`, `blocked(energizing-orb)`).

Every file comes from the raw carry of the ATM10 jar `Powah-6.2.10.jar`
(Crash Assistant `modlist.json`; Modrinth SHA-1
`f134ab3e0ace3793abf93129e318eb817898a4e1`, 2,737,991 B) in
`ports/_shared/carry-work/powah/carry/`, produced by `horizon-port-pack check`
at the campaign pin. `tools/assemble_content.py` (in the port root) reproduces
this folder and applies the patches below; **no asset or model is invented**.

| Path | What |
|---|---|
| `data/powah/horizon/block/furnator_starter.json` | **Patched** from the carry's `minecraft:carved_pumpkin` guess to `minecraft:furnace` (the block has a horizontal `facing`); hardness 2.0, resistance 20.0, `minecraft:iron_block` sounds, tool required, `occludes: false` (upstream `metalNoSolid`), stack 1 |
| `data/powah/horizon/block/energy_cell_starter.json` | **Patched** from the carry's `minecraft:stone` to `minecraft:iron_block` (no states) with the same numbers |
| `data/powah/horizon/block/energy_cable_starter.json` | **New**: the carry left the cable out (`no template for properties [down,east,north,south,up,west]`). `minecraft:chorus_plant` + `connects_to: "#powah:energy_cable_connects"` + `thickness: 4.0`, empty collision (upstream `noCollission`) |
| `assets/powah/blockstates/furnator_starter.json` | **Patched**: dropped `lollipop:block/empty` (another mod's model) for `facing=up/down`; 4 facings × `lit` (never set; upstream's `burning` is a sync flag, not a state) |
| `assets/powah/models/block/energy_cell.json` | **Patched**: parent `block` (1.21.1's `models/block.json`) → `block/block`, the only one 26.3 ships |
| `data/minecraft/tags/block/mineable/pickaxe.json`, `needs_iron_tool.json` | **Patched** to the three ids that exist (the carry lists all 92) |
| `data/powah/tags/block/energy_cable_connects.json` | **New**: the cable's neighbours for its visual arms (upstream joins any FE block; a content tag can only name blocks) |
| `data/powah/recipe/crafting/cable_starter.json` + `data/powah/advancement/recipes/misc/crafting/cable_starter.json` | **From the jar**: the carry skipped them with the block. 12 cables per recipe |
| `data/powah/horizon/creative_tab/machines.json` | **New**: upstream's tab is code; one tab with the three items (title key `itemGroup.powah.tab` from the jar's lang) |
| blockstates, models, `items/`, textures, lang, loot tables, two recipes/advancements | The carry's 26.3 output, byte-for-byte |

The three recipes need `powah:capacitor_basic_tiny`, `powah:dielectric_casing`,
`powah:dielectric_paste` and `powah:dielectric_rod_horizontal`. **Slice 2 carries them** (below),
so all three craft.

## Slice 2 — the dielectric/capacitor family (2026-11-20)

Six plain items, carried from the same jar and declared with the same shapes the three machines
use:

| Family | Files |
|---|---|
| items (6) | `data/powah/horizon/item/{dielectric_paste,dielectric_rod,dielectric_rod_horizontal,dielectric_casing,capacitor_basic,capacitor_basic_tiny}.json`, each `{"template": "minecraft:stick"}` (upstream's plain 64-stack items) |
| item definitions (6) | `assets/powah/items/<id>.json`, the 26.3 item-model definitions the three existing items already use |
| models (6) | `assets/powah/models/item/<id>.json`, byte-for-byte from the jar (`dielectric_casing` is its Blockbench 3D model on `block/block`) |
| textures (6) | `assets/powah/textures/item/<id>.png`, byte-for-byte |
| recipes (8) | `data/powah/recipe/crafting/{dielectric_paste,dielectric_paste_2,dielectric_rod,dielectric_rod_2,dielectric_rod_h,dielectric_rod_h_2,capacitor_basic,capacitor_basic_tiny}.json`, from the jar's recipes in the 26.3 shape |
| advancements (8) | the jar's recipe-unlock files under `data/powah/advancement/recipes/misc/crafting/` |
| creative tab | the six items join `data/powah/horizon/creative_tab/machines.json` (upstream has one Powah tab) |
| lang | unchanged: the `item.powah.*` names were already carried |

Two patches, both from the pin's recipe loader (proved in game, see `notes.md`):

1. **Ingredients are the string form** (`"minecraft:iron_ingot"`, `"#minecraft:coals"`), not the
   object form (`{"item": …}`). The loader rejects an object with *"recipes: invalid ingredient
   (unresolved tag or malformed input)"* and drops the whole file, so a carried 1.21.1 recipe
   written that way would never register.
2. **`dielectric_paste` and `dielectric_paste_2` are shapeless with repeated ingredients**, and
   this pin matches a shapeless recipe's ingredients against the **grid cells**, one stack per
   cell (`crafting.rs` `input.stacks()`, `crafting/shapeless_matching.rs`
   `ingredients.len() != stacks.len()`): a player who puts three coals in one cell matches
   nothing. The recipes stay upstream's (shapeless); the in-game leg places one item per cell,
   and the deviation is recorded in `notes.md` as a pin finding, not a port change.

## Slice 4 — the energy cell tiers (2026-11-21)

Six blocks and their items, carried from the same jar with the starter cell's patched block
definition (upstream's `AbstractEnergyBlock`: 2.0/20.0, metal sounds, `occludes: false`, stack 1):

| Family | Files |
|---|---|
| blocks (6) | `data/powah/horizon/block/energy_cell_{basic,hardened,blazing,niotic,spirited,nitro}.json` |
| blockstates / block models / item models / item definitions (6 each) | `assets/powah/…` under `blockstates/`, `models/block/`, `models/item/`, `items/` |
| textures (6) | `assets/powah/textures/block/energy_cell_<tier>.png` |
| loot tables (6) | `data/powah/loot_table/blocks/energy_cell_<tier>.json` |
| tags | the six join `minecraft:mineable/pickaxe` and `minecraft:needs_iron_tool` |
| creative tab | the six join `data/powah/horizon/creative_tab/machines.json` |
| recipe + advancement | `data/powah/recipe/crafting/energy_cell_basic.json` only (`ici/cdc/ici` with `capacitor_basic`, `dielectric_casing`, iron) |

The five other recipes need `powah:steel_energized` / `powah:crystal_*`, which only the Energizing Orb
makes; they are not carried and are recorded `blocked(energizing-orb)` in `notes.md`.

## Slice 5 — the Energizing Orb, the rod ladder and the materials (2026-11-21)

The gate slice 4 hit: the orb makes `powah:steel_energized` and the four crystals, and without
it five of six cell recipes and the whole capacitor ladder above `capacitor_basic` are
unreachable.

| Family | Files |
|---|---|
| blocks (9) | `data/powah/horizon/block/{energizing_orb,energizing_rod_starter,basic,hardened,blazing,niotic,spirited,nitro,blazing_crystal_block}.json` — the orb and the rods declare the six-value `facing` state upstream's `Facing.ALL` gives them (`minecraft:iron_block` + `state_properties`, default `down`, `occludes: false`, stack 1); the material block is `minecraft:stone` with upstream's 2.0/20.0 metal |
| blockstates (9) | `assets/powah/blockstates/<block>.json`, carried (six `facing` variants each, from the jar) |
| block models (10) | `assets/powah/models/block/<block>.json` plus the rods' shared `energizing_rod.json` parent |
| item models (20) + item definitions (20) | `assets/powah/models/item/…`, `assets/powah/items/…` for the nine blocks and the eleven plain items |
| item definitions (11) | `data/powah/horizon/item/<item>.json` — `minecraft:stick` (64-stack), as slice 2's |
| textures (23) | the orb's three, the rod's shared one and seven gems, the material block, the five materials (three animated, `.mcmeta` carried) and the six capacitors |
| loot (9) | `data/powah/loot_table/blocks/<block>.json` (the jar's 26.3 shape) |
| recipes (21) + advancements (21) | the orb, seven rods, `capacitor_basic_large`, five capacitor tiers, the five previously blocked cells, `blazing_crystal_block` and its 9-back shapeless |
| tags | the nine join `minecraft:mineable/pickaxe`; the jar's `powah:energizing_rods` block and item tags are carried |
| creative tab | the twenty new items join `powah:horizon/creative_tab/machines.json` |
| lang | unchanged: the carry's `en_us.json` already has every key |

**Not carried:** the jar's `data/powah/recipe/energizing/*.json` — `powah:energizing` is a
custom recipe type the pin has no loader for, so the six rows live in `src/runtime.rs` as a
table (`energizing()`), matched per filled slot. `blazing_crystal_2`'s ingredient is
`c:rods/blaze`, which has no 26.3 equivalent, so it is carried as `minecraft:blaze_rod`
(the tag's vanilla member) instead.

Two patches, both from the pin's loader and recorded in `notes.md`:

1. **Ingredients are the string form** (`"minecraft:quartz_block"`), not `{"item": …}`;
2. **NeoForge `c:` tags become the vanilla member they resolve to** — 26.3 has no `c:` tags:
   `c:glass_blocks` → `minecraft:glass`, `c:storage_blocks/quartz` → `minecraft:quartz_block`.
   The same replacement is applied in the recipe-unlock advancements, or a recipe would never
   unlock.

`tools/carry_slice5.py` (in the port root) reproduces this slice's 160 files from the raw carry
and `--check` verifies the tree still matches; **no asset or model is invented**.

## Slice 6 — the Furnator and Energy Cable ladders (2026-11-21)

The two ladders slice 1 opened, completed: six more furnators and six more cables, with
the jar's own recipes for each tier.

| Family | Files |
|---|---|
| blocks (12) | `data/powah/horizon/block/{furnator,energy_cable}_{basic,hardened,blazing,niotic,spirited,nitro}.json` — a furnator is the starter's `minecraft:furnace` definition (horizontal `facing`, 2.0/20.0 metal, `occludes: false`, stack 1) and a cable is the starter's `minecraft:chorus_plant` relay (`connects_to`, `thickness: 4.0`, empty collision); a tier differs from the starter only in its id |
| blockstates (12) | `assets/powah/blockstates/…` — the cables carried; a furnator's is the package's own starter blockstate with its model renamed (the carry's still names `lollipop:block/empty` and has no `lit` variant) |
| block models (18) | `assets/powah/models/block/…` — six furnators on the shared `powah:block/furnator` parent with their `<tier>_ov` texture, six cables and their six multipart arms |
| item models (12) + item definitions (12) | `assets/powah/models/item/…`, `assets/powah/items/…` |
| textures (12) | six `<tier>_ov.png` (the furnator body is shared) and six `energy_cable_<tier>.png` |
| loot tables (12) | `data/powah/loot_table/blocks/…` (the jar's 26.3 shape) |
| recipes (18) + advancements (18) | the six `furnator_<tier>` recipes from the carry, and **both** of the jar's cable recipes per tier (`cable_<tier>` upgrading the previous cable, `cable_<tier>_2` from the tier's material) — the carry had skipped the cable recipe files with the block |
| tags | the jar's `powah:furnators` and `powah:energy_cables` block and item tags; the twelve join `minecraft:mineable/pickaxe` and `minecraft:needs_iron_tool` and `powah:energy_cable_connects` |
| creative tab | the twelve join `data/powah/horizon/creative_tab/machines.json` (47 items) |
| lang | unchanged: the carry's `en_us.json` already has `block.powah.furnator_<tier>` and `block.powah.energy_cable_<tier>` |

`tools/carry_slice6.py` reproduces this slice's 134 files from the raw carry and the jar
(the six cable recipes and their advancements are read from `Powah-6.2.10.jar` itself) and
`--check` verifies the tree still matches; **no asset or model is invented**. The two
patches are the ones every slice applies: the horizon block definition is the package's own
starter shape, and a recipe's ingredients are the pin's bare-string form.

## Slice 8 — the energy discharger ladder (2026-11-22)

The cell's mirror, seven tiers of it, with the jar's own recipes.

| Family | Files |
|---|---|
| blocks (7) | `data/powah/horizon/block/energy_discharger_<tier>.json` — the package's `minecraft:furnace` shape (horizontal `facing`, 2.0/20.0 metal, `occludes: false`, stack 1), as upstream's `Facing.HORIZONTAL` and `Properties.metalNoSolid(2.0, 20.0)` say |
| blockstates (7) | `assets/powah/blockstates/energy_discharger_<tier>.json` — **written**, not carried: the carry names `lollipop:block/empty` for `facing=up/down` and has no `lit` variant; four horizontal facings × `lit`, exactly as the furnator's is |
| block models (8) | `assets/powah/models/block/energy_discharger.json` (the jar's Blockbench base, parent `block/block`) and the seven tiers' `<tier>_ov` overlays |
| item models (7) + item definitions (7) | `assets/powah/models/item/…`, `assets/powah/items/…` |
| textures (2) | `energy_discharger_side.png`, `energy_discharger_comps.png` (the seven `<tier>_ov.png` were already carried) |
| loot tables (7) | `data/powah/loot_table/blocks/energy_discharger_<tier>.json` |
| recipes (7) + advancements (7) | `data/powah/recipe/crafting/energy_discharger_<tier>.json` — `pcp/pdp/pcp` (six paste, two capacitors of the tier, one casing); read from the jar in the pin's string form |
| tags | the jar's `powah:energy_dischargers` block and item tags; the seven join `minecraft:mineable/pickaxe` and `minecraft:needs_iron_tool` |
| creative tab | the seven join `data/powah/horizon/creative_tab/machines.json`, after the cable row |
| lang | unchanged: the carry's `en_us.json` already has `block.powah.energy_discharger_<tier>` |

`tools/carry_slice8.py` reproduces the slice's 64 files from the raw carry and the jar and
`--check` verifies the tree still matches; **no asset or model is invented**. Every tier's
recipe is craftable from carried items (the capacitor tiers come from slice 5's materials), so
all seven are carried and all seven are crafted in the recipe leg.

## Slice 9 — the energy hopper ladder (2026-11-23)

The discharger's mirror: seven tiers of a storage block that charges the container it faces. No
asset or model is invented; the carry's own blockstate already draws all six facings.

| Family | Files |
|---|---|
| blocks (7) | `data/powah/horizon/block/energy_hopper_<tier>.json` — the plain `minecraft:iron_block` template with upstream's numbers (`Properties.metalNoSolid(2.0, 20.0)`), a six-value `facing` enum and default `north`, item stack 1 |
| blockstates (7) | `assets/powah/blockstates/energy_hopper_<tier>.json`, carried byte-for-byte (six facings, each with its model rotation) |
| block models (8) | `assets/powah/models/block/energy_hopper.json` (the jar's base model, parent `block/block`) and the seven tiers' `<tier>_ov` overlays |
| item models (7) + item definitions (7) | `assets/powah/models/item/…`, `assets/powah/items/…` |
| textures (3) | `energy_hopper_side.png`, `energy_hopper_back.png`, `energy_hopper_pointer.png` (the tiers' `<tier>_ov.png` were already carried) |
| loot tables (7) | `data/powah/loot_table/blocks/energy_hopper_<tier>.json` |
| recipes (7) + advancements (7) | `data/powah/recipe/crafting/energy_hopper_<tier>.json`, read from the jar in the pin's string form — starter `ppp/cdc/php` (three paste, two tiny capacitors, a casing, a vanilla hopper); each tier upgrades the previous tier's hopper |
| tags | the jar's `powah:energy_hoppers` block and item tags; the seven join `minecraft:mineable/pickaxe` and `minecraft:needs_iron_tool` |
| creative tab | the seven join `data/powah/horizon/creative_tab/machines.json`, after the discharger row |
| lang | unchanged: the carry's `en_us.json` already has `block.powah.energy_hopper_<tier>` |

`tools/carry_slice9.py` reproduces the slice's 65 files from the raw carry and the jar and
`--check` verifies the tree still matches. Every recipe is craftable from carried items, so all
seven are carried and all seven are crafted in the recipe leg.

## Slice 10 — the Solar Panel ladder (2026-11-23)

The generator whose only input is the sky, and the pane it is built from. No asset or model is
invented; two files are written because the carry's guesses would be wrong.

| Family | Files |
|---|---|
| blocks (7) | `data/powah/horizon/block/solar_panel_<tier>.json` — **written, not carried**: the carry's guess is `minecraft:oak_fence` (a full-size post), while upstream's `SolarBlock` is a 1-pixel plate (`box(0,0,0,16,1,16)`) with four horizontal booleans (`north`/`east`/`south`/`west`, `true` when no panel of the same tier is attached, so the frame is drawn). The definition is `minecraft:iron_block` with `Properties.metal(2.0, 20.0)`, **its own** four booleans (default `true`), a `shapes` and a `collision_shapes` at `[0,0,0,1,0.0625,1]`, and stack 1 |
| blockstates (7) | `assets/powah/blockstates/solar_panel_<tier>.json`, carried byte-for-byte (four frames and the panel) |
| block models (11) | `solar_panel` (the base), `solar_panel_frame`, `solar_panel_frame_` (the six higher tiers' shared frame) and `solar_panel_starter_frame`, plus the seven tiers' own models |
| item models (9) + item definitions (7) | `assets/powah/models/item/solar_panel…`, `assets/powah/items/solar_panel_<tier>.json` |
| textures (9) | `solar_panel_top.png`, `solar_panel_starter_top.png` and the seven `<tier>_frame.png` |
| loot tables (7) | `data/powah/loot_table/blocks/solar_panel_<tier>.json` (the jar's 26.3 shape) |
| the pane (1 item + 1 recipe + 1 advancement + 1 texture + 2 models) | `powah:photoelectric_pane` — the starter panel's top row; its recipe is the jar's `dld/lpl/dld` (paste, lapis, a glass pane) and its unlock advancement comes with it |
| recipes (8) + advancements (8) | the seven `solar_panel_<tier>` recipes from the jar (the starter `ppp/cdc/eee`, each tier `ppp/cdc/<material>` over the previous tier's panel) and the pane's; read from the jar in the pin's string form |
| tags | the jar's `powah:solar_panels` block and item tags; the seven join `minecraft:mineable/pickaxe` and `minecraft:needs_iron_tool` |
| creative tab | the seven panels and the pane join `data/powah/horizon/creative_tab/machines.json`, after the hopper row (69 items) |
| lang | unchanged: the carry's `en_us.json` already has `block.powah.solar_panel_<tier>` and `item.powah.photoelectric_pane` |

Two patches, both the ones earlier slices apply:

1. **The horizon block definition is written** (see above) rather than the carry's fence guess.
2. **The jar's one NeoForge `c:` tag becomes its vanilla member**: the pane's `#c:glass_panes` is
   `minecraft:glass_pane` in the recipe **and** in its unlock advancement (26.3 has no `c:` tags).

`tools/carry_slice10.py` reproduces the slice's **82** files from the raw carry and the jar and
`--check` verifies the tree still matches. Every recipe is craftable from carried items (the
capacitor tiers come from slice 5's materials, the pane from slice 2's paste and vanilla lapis), so
all eight are carried and all eight are crafted in the recipe leg. The Lens of Ender is **not**
carried: the jar has no recipe for it (it is a right-click interaction on an enderman with a pane,
config-gated) and the panel's lens slot is a GUI upgrade — a named gap in `notes.md`.

## Slice 11 — the ender cell and ender gate ladders (2026-11-23)

Two block families whose buffer is one shared channel per owner, seven tiers of each. The carry's
blockstates, models and textures draw both; the two block definitions are written, and the fourteen
recipes are carried from the carry's already-string-form import.

| Family | Files |
|---|---|
| blocks (14) | `data/powah/horizon/block/ender_cell_<tier>.json` (**written**: the plain `minecraft:iron_block` metal block upstream's `Properties.metalNoSolid(2.0, 20.0)` describes, no state of its own, stack 1) and `ender_gate_<tier>.json` (**written**: the same metal block with the six-value `facing` upstream's `Facing.ALL` gives it, default `north`, a 4×4×0.5-pixel plate per face (`EnderGateBlock.SHAPES`) and **empty collision** (`noCollission()`)) |
| blockstates (14) | `assets/powah/blockstates/…`, carried byte-for-byte (the cells' single state, the gates' six facings) |
| block models (16) | `assets/powah/models/block/{ender_cell,ender_gate}.json` (the jar's Blockbench bases, parent `block/block`) and the fourteen tiers' models — a gate tier's `ov` is the same `<tier>_ov` overlay every machine tier already carried |
| item models (15) + item definitions (15) | `assets/powah/models/item/…` and `assets/powah/items/…` for the fourteen blocks and `powah:ender_core` |
| textures (25) | the seven cells' animated `ender_cell_<tier>.png` with their `.mcmeta`, `ender_cell_core.png`, `ender_gate.png`, the seven gate item textures, and `ender_core.png` with its `.mcmeta` |
| the core (1 item) | `powah:ender_core` — the item both ladders are built from; its recipe is an **energizing** row (`data/powah/recipe/energizing/ender_core.json`: an ender eye, a dielectric casing, a tiny capacitor, 50,000 FE), which lives in `src/runtime.rs`'s `energizing()` table because 26.3 has no loader for `powah:energizing` |
| loot tables (14) | `data/powah/loot_table/blocks/…` (the jar's 26.3 shape) |
| recipes (14) + advancements (14) | `data/powah/recipe/crafting/ender_{cell,gate}_<tier>.json` — `oio/ici/oio` (four obsidian, four of the tier's material or cable, an ender core; the starter cell's material is an iron **nugget**, a gate craft is four) — read from the carry in the pin's string form, with the jar's unlock advancements |
| tags | the jar's `powah:ender_cells` and `powah:ender_gates` block and item tags; the fourteen join `minecraft:mineable/pickaxe` and `minecraft:needs_iron_tool` |
| creative tab | the fourteen and the core join `data/powah/horizon/creative_tab/machines.json`, after the solar panels (84 items) |
| lang | unchanged: the carry's `en_us.json` already has every `block.powah.ender_*` and `item.powah.ender_core` key |

Two notes on what is **not** here:

1. The jar's fourteen `ender_{cell,gate}_<tier>_2` recipes are not carried: each is the jar's own
   datagen self-recipe (one cell into one cell), a no-op because `keepEnergy()` is false and the
   item carries no data for it to preserve (see `notes.md`, slice 11, deviation 6).
2. No asset or model is invented: everything except the two written block definitions and the core's
   energizing row is byte-for-byte the carry's, which is what `tools/carry_slice11.py --check`
   proves for all **149** files.

## Where this content lives now (0.2.0)

Since the SDK migration the carried tree **is the package root**: `assets/powah/` and
`data/powah/` sit beside `src/`, and `horizon-mod.toml` at the root carries the identity plus
`[native]`. There is no `content/` folder and no separate content `.hmod` any more — one
`powah.hmod` (68,403 B, SHA-256 `11824c7c…455d`, 50 entries) ships this content **with** the
code. The 41 carried files were moved with their paths, sizes and SHA-256 unchanged
(`../_shared/evidence/powah/carried-content-sha256.txt`).

The build of the 0.1.0 content-only archive, kept as history:

Built with (inside a build slot, from this folder):

```sh
CARGO_TARGET_DIR=<slot target>/cargo-horizon-mod cargo run --locked \
  --manifest-path ../../_shared/pins/horizon-1d63e0a/crates/cargo-horizon-mod/Cargo.toml \
  -- horizon-mod build
```

then copied beside the folder as `content/powah.hmod` (39,262 B; SHA-256
`7d344174…`; 42 entries) — retired to
`../_shared/evidence/powah/retired-hnmod-0.1.0/content-old/`.

## Slice 12 — the player transmitter and the card family (93 files)

`tools/carry_slice12.py` writes all of it and `--check` reproves it. Carried byte-for-byte from the
jar/carry: the transmitter body model (`models/block/player_transmitter.json`), its particle and core
textures, the seven tier models, the seven `_var` tier textures, the seven `items/*.json`, the seven
loot tables and unlock advancements, the `powah:player_transmitters` block and item tags, the seven
recipes' advancements, `binding_card`/`blank_card`/`aerial_pearl`/`player_aerial_pearl` items,
textures and items JSON, and their recipes' advancements.

Written rather than carried, each with its reason in the script's docstring: the seven block
definitions (the metal block upstream's `Blcks.PLAYER_TRANSMITTER` declares — the carry's guess is a
bare stone block), the seven blockstates (one variant: this slice carries the bottom half only), the
seven item models (the block model plus upstream's own `display` transforms, because the carried item
model is the two-block structure) and `models/item/binding_card.json` (the carried one's `powah:bound`
override is 26.3 item model property this slice does not register).

The ten recipes are read from the jar's own files and their `{"item": …}` ingredients rewritten to
the pin's id-string form (`recipe_from_jar`), because the carry's importer skipped this family; the
jar's `player_tranmitter_*` spelling is kept, since renaming an upstream recipe changes its id.

## Slice 17 — the creative energy cell (2026-11-24)

Seven files for one block, reproduced by `tools/carry_slice17.py` (`--check` green). Six are
carried **byte-for-byte** from the same raw carry as every earlier slice:

| Path | What |
|---|---|
| `assets/powah/blockstates/energy_cell_creative.json` | the jar's single-variant blockstate |
| `assets/powah/models/block/energy_cell_creative.json` | parent `powah:block/energy_cell` with the creative texture |
| `assets/powah/models/item/energy_cell_creative.json` | the jar's item model |
| `assets/powah/items/energy_cell_creative.json` | the 26.3 item definition |
| `assets/powah/textures/block/energy_cell_creative.png` | the jar's 3,250 B texture |
| `data/powah/loot_table/blocks/energy_cell_creative.json` | self-drop with `survives_explosion` |

One is **written**, not carried:

| Path | Patch |
|---|---|
| `data/powah/horizon/block/energy_cell_creative.json` | the carry guesses `minecraft:stone`; a cell is upstream's `AbstractEnergyBlock` — `minecraft:iron_block`, 2.0/20.0 metal, `requires_tool`, `occludes: false`, stack 1 — exactly the port's other seven cells |

The id joins `minecraft:mineable/pickaxe`, `minecraft:needs_iron_tool` and the
`powah:horizon/creative_tab/machines` list. **No recipe is carried:** the jar has no recipe for the creative tier (it is creative-only). The already-carried
`assets/powah/lang/en_us.json` has `block.powah.energy_cell_creative` ("Energy Cell (Creative)").
No asset or model is invented.

## Slice 18 — the seven battery items (2026-11-24)

Forty-two files, reproduced by `tools/carry_slice18.py` (`--check` green). Five per tier are
carried **byte-for-byte** from the raw carry:

| Path | What |
|---|---|
| `assets/powah/textures/item/battery_<tier>.png` | the jar's battery texture |
| `assets/powah/models/item/battery_<tier>.json` | `item/generated` with the texture as `layer0` |
| `assets/powah/items/battery_<tier>.json` | the 26.3 item definition |
| `data/powah/recipe/crafting/battery_<tier>.json` | the jar's shaped recipe (starter's `oio/ici/oio`, the six others' `oko/iri/oco`) |
| `data/powah/advancement/recipes/misc/crafting/battery_<tier>.json` | the jar's unlock file |

Seven are **written**, not carried:

| Path | Patch |
|---|---|
| `data/powah/horizon/item/battery_<tier>.json` | the carry guesses a 64-stack `{"template": "minecraft:stick"}`; `Itms.BATTERY` is `new BatteryItem(new Item.Properties().stacksTo(1), variant)`, so the declaration carries `max_stack_size: 1` |

The seven ids join `powah:horizon/creative_tab/machines`. The already-carried
`assets/powah/lang/en_us.json` names every `item.powah.battery_<tier>`. No tool tags: a battery is
an item, not a block. No asset, model, number or recipe is invented; `craft_check.py` crafts all
seven in game.

## Slice 19 — the wrench (2026-11-24)

Six files, reproduced by `tools/carry_slice19.py` (`--check` green). Five are carried
**byte-for-byte** from the raw carry:

| Path | What |
|---|---|
| `assets/powah/textures/item/wrench.png` | the jar's wrench texture (368 B) |
| `assets/powah/models/item/wrench.json` | `item/generated` with that texture as `layer0` |
| `assets/powah/items/wrench.json` | the 26.3 item definition |
| `data/powah/recipe/crafting/wrench.json` | the jar's shaped recipe (`" ip"/" pi"/"p  "`: iron 101/105, paste 102/104/106) |
| `data/powah/advancement/recipes/misc/crafting/wrench.json` | the jar's unlock file |

One is **written**, not carried:

| Path | Patch |
|---|---|
| `data/powah/horizon/item/wrench.json` | the carry guesses a 64-stack `{"template": "minecraft:stick"}`; `Itms.WRENCH = new WrenchItem(new Item.Properties().stacksTo(1))`, so the declaration carries `max_stack_size: 1` |

The id joins `powah:horizon/creative_tab/machines`. The already-carried
`assets/powah/lang/en_us.json` names `item.powah.wrench` and its three mode strings. The
`c:tools/wrench` item tag is deliberately not carried: it is a mod-compatibility tag with no
effect in this port (no carried recipe or hook reads it). No asset, model, number or recipe is
invented; `craft_check.py` crafts the wrench in game.
