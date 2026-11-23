//! The Powah machines' glue: the three block entity Parts, the level-tick queue,
//! the block energy buffers and the `/powah` command bodies.
//!
//! Every rule lives in [`crate::runtime`] (the retired bundle's `runtime.rs`,
//! byte-identical); this file reads the game, calls it and writes the result
//! back. Its tests pin what it *declares* (one fuel slot, two buffers and the
//! cable's none), that the saved compound reads back, and the exact lines
//! `/powah info` prints.
//!
//! What the retired bundle's `block_entity_energy` hook did through a shared
//! output is a declaration now (`Registrar::block_energy`, keyed by the block),
//! so the buffer's numbers have one home, [`battery_of`], which the cable's
//! relay also reads when it plans a move.
use std::collections::{BTreeSet, HashMap, VecDeque};
use std::sync::{Arc, Mutex, OnceLock, RwLock};

use mod_api::HostBlockStates;
use mod_api::{
    self, BlockDropsEvent, BlockEntityPart, BlockEntityTickEvent, BlockEntityTickReply,
    BlockEntityTicksEvent, BlockEntityTypeKey, BlockFace, BlockPart, Command, CommandArgumentValue,
    Component, ComponentId, Compound, Direction, ENERGY, EnergyHandler, Flow, Host,
    HostTransaction, Identifier, ItemComponentHandler, ItemKey, ItemPart, ItemPlace, LevelHost,
    LevelRef, ModContainer, ModEnergyBuffer, OnBlockPlacedByEvent, Registrar, ServerHost, Tag,
    Transaction, move_energy,
};

use crate::runtime::{
    CABLE_TIER_COUNT, CELL_TIER_COUNT, CableTier, CellTier, DISCHARGER_TIER_COUNT, DischargerTier,
    EnergizingRecipe, FURNATOR_TIER_COUNT, Furnator, FurnatorTier, HOPPER_TIER_COUNT, HopperTier,
    Neighbour, ORB_MAX_ENERGY, ORB_OUTPUT_SLOT, ORB_SLOTS, ROD_TIER_COUNT, RodTier, cable_tier,
    cell_tier, clamp_energy, comparator_strength, discharger_tier, furnator_tier, network_moves,
    push_amount, rod_tier,
};

/// The energizing orb block and its item.
pub(crate) const ORB: &str = "powah:energizing_orb";
/// The orb's block entity type path, in this mod's namespace.
pub(crate) const ORB_TYPE: &str = "energizing_orb";

/// Every energizing rod tier's block id and block entity type name, in the
/// ladder's order (the index [`rod_tier`] takes).
pub(crate) const RODS: [(&str, &str); ROD_TIER_COUNT] = [
    ("powah:energizing_rod_starter", "energizing_rod_starter"),
    ("powah:energizing_rod_basic", "energizing_rod_basic"),
    ("powah:energizing_rod_hardened", "energizing_rod_hardened"),
    ("powah:energizing_rod_blazing", "energizing_rod_blazing"),
    ("powah:energizing_rod_niotic", "energizing_rod_niotic"),
    ("powah:energizing_rod_spirited", "energizing_rod_spirited"),
    ("powah:energizing_rod_nitro", "energizing_rod_nitro"),
];

/// The starter furnator block and its item (the ladder's first row).
pub(crate) const FURNATOR: &str = "powah:furnator_starter";
/// The starter energy cell block and its item.
pub(crate) const CELL: &str = "powah:energy_cell_starter";
/// The starter energy cable block and its item.
pub(crate) const CABLE: &str = "powah:energy_cable_starter";

/// The block entity type paths, in this mod's namespace.
pub(crate) const FURNATOR_TYPE: &str = "furnator_starter";
pub(crate) const CELL_TYPE: &str = "energy_cell_starter";
pub(crate) const CABLE_TYPE: &str = "energy_cable_starter";

/// Every furnator tier's block id and block entity type name, in the ladder's
/// order (the index [`furnator_tier`] takes).
pub(crate) const FURNATORS: [(&str, &str); FURNATOR_TIER_COUNT] = [
    (FURNATOR, FURNATOR_TYPE),
    ("powah:furnator_basic", "furnator_basic"),
    ("powah:furnator_hardened", "furnator_hardened"),
    ("powah:furnator_blazing", "furnator_blazing"),
    ("powah:furnator_niotic", "furnator_niotic"),
    ("powah:furnator_spirited", "furnator_spirited"),
    ("powah:furnator_nitro", "furnator_nitro"),
];

/// Every energy cable tier's block id and block entity type name, in the
/// ladder's order (the index [`cable_tier`] takes).
pub(crate) const CABLES: [(&str, &str); CABLE_TIER_COUNT] = [
    (CABLE, CABLE_TYPE),
    ("powah:energy_cable_basic", "energy_cable_basic"),
    ("powah:energy_cable_hardened", "energy_cable_hardened"),
    ("powah:energy_cable_blazing", "energy_cable_blazing"),
    ("powah:energy_cable_niotic", "energy_cable_niotic"),
    ("powah:energy_cable_spirited", "energy_cable_spirited"),
    ("powah:energy_cable_nitro", "energy_cable_nitro"),
];

/// Every energy cell tier's block id and block entity type name, in the ladder's
/// order (the index [`cell_tier`] takes). A type name is its block id without this
/// mod's namespace, which is what [`declare`] attaches.
pub(crate) const CELLS: [(&str, &str); CELL_TIER_COUNT] = [
    (CELL, CELL_TYPE),
    ("powah:energy_cell_basic", "energy_cell_basic"),
    ("powah:energy_cell_hardened", "energy_cell_hardened"),
    ("powah:energy_cell_blazing", "energy_cell_blazing"),
    ("powah:energy_cell_niotic", "energy_cell_niotic"),
    ("powah:energy_cell_spirited", "energy_cell_spirited"),
    ("powah:energy_cell_nitro", "energy_cell_nitro"),
];

/// Every energy discharger tier's block id and block entity type name, in the
/// ladder's order (the index [`discharger_tier`] takes).
pub(crate) const DISCHARGERS: [(&str, &str); DISCHARGER_TIER_COUNT] = [
    (
        "powah:energy_discharger_starter",
        "energy_discharger_starter",
    ),
    ("powah:energy_discharger_basic", "energy_discharger_basic"),
    (
        "powah:energy_discharger_hardened",
        "energy_discharger_hardened",
    ),
    (
        "powah:energy_discharger_blazing",
        "energy_discharger_blazing",
    ),
    ("powah:energy_discharger_niotic", "energy_discharger_niotic"),
    (
        "powah:energy_discharger_spirited",
        "energy_discharger_spirited",
    ),
    ("powah:energy_discharger_nitro", "energy_discharger_nitro"),
];

/// Every energy hopper tier's block id and block entity type name, in the ladder's order (the
/// index [`hopper_of`] takes).
pub(crate) const HOPPERS: [(&str, &str); HOPPER_TIER_COUNT] = [
    ("powah:energy_hopper_starter", "energy_hopper_starter"),
    ("powah:energy_hopper_basic", "energy_hopper_basic"),
    ("powah:energy_hopper_hardened", "energy_hopper_hardened"),
    ("powah:energy_hopper_blazing", "energy_hopper_blazing"),
    ("powah:energy_hopper_niotic", "energy_hopper_niotic"),
    ("powah:energy_hopper_spirited", "energy_hopper_spirited"),
    ("powah:energy_hopper_nitro", "energy_hopper_nitro"),
];

/// The furnator's fuel slot: upstream's slot 1 of 2, and the charge slot is 0.
const FUEL_SLOT: usize = 1;
/// The charge slot: upstream's slot 0 of the furnator, and the first of a cell's two.
pub(crate) const CHARGE_SLOT: usize = 0;
/// A furnator's two slots (`FurnatorTile`: chargeables then fuel).
const FURNATOR_SLOTS: usize = 2;
/// A cell's two slot slots (`EnergyCellTile` charges `chargeItems(2)`).
const CELL_SLOTS: usize = 2;
/// A discharger's seven slots (`EnergyDischargerTile`'s `inv.add(7)`, slot limit 1,
/// dischargeable items only).
const DISCHARGER_SLOTS: usize = 7;

/// What one of this mod's machine items holds, as upstream's `EnergyBlockItem`
/// `Info` gives it: the machine tier's capacity, and the per-call insert and
/// extract limits its `getTransferType` allows (a furnator only gives).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ItemBuffer {
    /// The tier's capacity.
    pub capacity: i32,
    /// The most one insert moves; 0 for a machine that never receives.
    pub max_insert: i32,
    /// The most one extract moves.
    pub max_extract: i32,
}

/// The item buffer of the machine `block` names, `None` for a block whose item
/// holds none (the cable stores nothing and the orb is not an energy block).
pub(crate) fn item_buffer(block: &str) -> Option<ItemBuffer> {
    match Kind::of(block)? {
        // `AbstractEnergyProvider.getTransferType` is `Transfer.EXTRACT`: a furnator's
        // own item gives and never takes.
        Kind::Furnator(tier) => Some(ItemBuffer {
            capacity: tier.capacity,
            max_insert: 0,
            max_extract: tier.transfer,
        }),
        // `EnergyDischargerBlock.getTransferType` is `Transfer.EXTRACT` too: a
        // discharger item only gives.
        Kind::Discharger(tier) => Some(ItemBuffer {
            capacity: tier.capacity,
            max_insert: 0,
            max_extract: tier.transfer,
        }),
        Kind::Cell(tier) => Some(ItemBuffer {
            capacity: tier.capacity,
            max_insert: tier.transfer,
            max_extract: tier.transfer,
        }),
        // `AbstractEnergyBlock.getTransferType()` is the default `Transfer.ALL`: a hopper's
        // own item both gives and takes at its tier's transfer.
        Kind::Hopper(tier) => Some(ItemBuffer {
            capacity: tier.capacity,
            max_insert: tier.transfer,
            max_extract: tier.transfer,
        }),
        Kind::Rod(tier) => Some(ItemBuffer {
            capacity: tier.capacity,
            max_insert: tier.transfer,
            max_extract: tier.transfer,
        }),
        Kind::Orb | Kind::Cable(_) => None,
    }
}

/// The item slots a machine kind holds energy items in: the furnator's one charge slot
/// (`FurnatorTile`'s `chargeItems(1)`), a cell's two (`EnergyCellTile`'s `chargeItems(2)`),
/// a discharger's seven (`EnergyDischargerTile`'s `inv.add(7)`, which its
/// `dischargeItemsInInventory` drains), everything else none.
pub(crate) fn kind_item_slots(kind: Kind) -> usize {
    match kind {
        Kind::Furnator(_) => 1,
        Kind::Cell(_) => CELL_SLOTS,
        Kind::Discharger(_) => DISCHARGER_SLOTS,
        // `EnergyHopperTile` has no slots of its own (`getSlotLimit` 0, `canInsert` and
        // `canExtract` false): it charges the container it faces, never one it holds.
        Kind::Hopper(_) => 0,
        Kind::Orb | Kind::Cable(_) | Kind::Rod(_) => 0,
    }
}

/// How many item slots the machine `block` names holds energy items in: a furnator one, a
/// cell two, a discharger seven, everything else none.
pub(crate) fn item_slots(block: &str) -> usize {
    Kind::of(block).map_or(0, kind_item_slots)
}

/// The buffer a machine's own item carries when the machine is broken
/// (upstream's `keepEnergy`): its tier's capacity, `None` when the machine keeps
/// nothing (the cable, the orb).
pub(crate) fn keep_capacity(block: &str) -> Option<i32> {
    match Kind::of(block)? {
        Kind::Furnator(tier) => Some(tier.capacity),
        Kind::Cell(tier) | Kind::Discharger(tier) => Some(tier.capacity),
        Kind::Hopper(tier) => Some(tier.capacity),
        Kind::Rod(tier) => Some(tier.capacity),
        Kind::Orb | Kind::Cable(_) => None,
    }
}

/// A cell's container: upstream's two item slots (`EnergyCellTile`), every face
/// reaching both, no menu (this port has no GUI).
pub(crate) fn cell_container() -> ModContainer {
    let slots: Vec<usize> = (0..CELL_SLOTS).collect();
    ModContainer {
        slots: CELL_SLOTS,
        faces: std::array::from_fn(|_| slots.clone()),
        inputs: slots.clone(),
        outputs: slots,
        menu: None,
    }
}

/// A discharger's container: upstream's seven slots (`EnergyDischargerTile`'s
/// `inv.add(7)`, slot limit 1), every face reaching all seven, no menu. Upstream's
/// `canInsert` takes only dischargeable items there; a container declaration cannot
/// express that at this pin, so the tick leaves an item with no buffer alone.
pub(crate) fn discharger_container() -> ModContainer {
    let slots: Vec<usize> = (0..DISCHARGER_SLOTS).collect();
    ModContainer {
        slots: DISCHARGER_SLOTS,
        faces: std::array::from_fn(|_| slots.clone()),
        inputs: slots.clone(),
        outputs: slots,
        menu: None,
    }
}

/// The vanilla component that moves a block entity's saved data on its item
/// (`BlockEntity.applyComponentsFromItemStack`), tagged with the type's id.
pub(crate) const BLOCK_ENTITY_DATA: &str = "minecraft:block_entity_data";

/// One charge slot as a readout shows it: the item's id, what it holds and its
/// capacity.
pub(crate) type ChargeReadout<'a> = Option<(&'a str, i32, i32)>;

/// `minecraft:block_entity_data`, as a component a stack's patch is read from.
struct BlockEntityData;

impl Component for BlockEntityData {
    type Value = Compound;

    fn id(&self) -> &str {
        BLOCK_ENTITY_DATA
    }

    fn decode(&self, value: &Tag) -> Option<Compound> {
        match value {
            Tag::Compound(data) => Some(data.clone()),
            _ => None,
        }
    }

    fn encode(&self, value: Compound) -> Tag {
        Tag::Compound(value)
    }
}

/// The `minecraft:block_entity_data` patch a broken machine's own item carries:
/// the buffer, tagged with the block entity type's id (the engine loads a payload
/// only when its `id` names the block entity it is placed as). `None` when the
/// buffer is empty, which is upstream's "write the storable compound only when
/// it is not empty".
///
/// **Only the buffer.** The engine spills a mod container's stacks as their own
/// item entities when the block is removed, so a payload that also carried
/// `Items` would put every stack in the world twice (HANDOVER §9m).
#[must_use]
pub(crate) fn carried_patch(energy: i32, capacity: i32, type_id: &str) -> Option<Compound> {
    let energy = crate::runtime::carried_energy(energy, capacity)?;
    let mut payload = Compound::new();
    payload.insert("id".into(), Tag::String(type_id.into()));
    payload.insert(KEY_ENERGY.into(), Tag::Int(energy));
    let mut patch = Compound::new();
    patch.insert(BLOCK_ENTITY_DATA.into(), Tag::Compound(payload));
    Some(patch)
}

/// The buffer a stack's `minecraft:block_entity_data` payload asks a placed
/// machine to hold, exactly as the payload states it (a `/give`d item may name
/// any amount); `None` when the payload carries none at all.
#[must_use]
pub(crate) fn asked_energy(payload: Option<&Compound>) -> Option<i32> {
    match payload?.get(&KEY_ENERGY.into()) {
        Some(Tag::Int(energy)) => Some(*energy),
        // A payload a player wrote may name a long; it is clamped where it is used.
        Some(Tag::Long(energy)) => {
            Some(i32::try_from(*energy).unwrap_or(if *energy < 0 { i32::MIN } else { i32::MAX }))
        }
        _ => None,
    }
}

/// The `minecraft:block_entity_data` payload a stack carries, if it carries one.
fn payload_of(stack: &mod_api::ItemStack) -> Option<Compound> {
    stack.get(&BlockEntityData)
}

/// The FE `stack`'s payload says the machine it becomes holds: 0 when it carries
/// no payload or one without a buffer.
fn held_energy(stack: &mod_api::ItemStack) -> i32 {
    asked_energy(payload_of(stack).as_ref()).unwrap_or(0)
}

/// `stack` rewritten as holding `energy` FE in its own
/// `minecraft:block_entity_data` payload (so placing it gives that machine that
/// FE), clamped into the item's own buffer. `None` when the stack is not one of
/// this mod's machine items.
///
/// The payload is built fresh — the type id and the buffer, nothing else — so a
/// doctored stack's other keys (`Items` above all) never travel on.
fn with_payload_energy(
    host: &dyn Host,
    stack: &mod_api::ItemStack,
    energy: i32,
) -> Option<mod_api::ItemStack> {
    let block = stack.item().id(host);
    let buffer = item_buffer(&block)?;
    let mut payload = Compound::new();
    payload.insert("id".into(), Tag::String(block.as_str().into()));
    payload.insert(
        KEY_ENERGY.into(),
        Tag::Int(crate::runtime::clamp_energy(energy, buffer.capacity)),
    );
    let mut stack = stack.clone();
    stack.set(&BlockEntityData, payload).ok()?;
    Some(stack)
}

/// Upstream's saved keys (`saveAdditional`).
const KEY_ENERGY: &str = "energy";
const KEY_CARBON: &str = "carbon";
/// The orb's matched recipe (its output id) and the rod's linked orb position.
const KEY_RECIPE: &str = "recipe";
const KEY_ORB: &str = "orb";
/// A rod's warm-up counter (`Ticker(20)`), kept across a reload so a reloaded rod
/// does not restart its 20 ticks.
const KEY_COOLDOWN: &str = "cool_down";

/// The three machines, as a readout has to tell them apart.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    /// A furnator, carrying its tier's generation, capacity and transfer.
    Furnator(FurnatorTier),
    /// An energy cell, carrying its tier's capacity and transfer.
    Cell(CellTier),
    /// An energy cable, carrying its tier's rate.
    Cable(CableTier),
    /// An energy discharger, carrying its tier's capacity and transfer.
    Discharger(DischargerTier),
    /// An energy hopper, carrying its tier's capacity, transfer and charging rate.
    Hopper(HopperTier),
    /// The energizing orb: a recipe-gated buffer over seven slots.
    Orb,
    /// An energizing rod, carrying its tier's capacity and transfer.
    Rod(RodTier),
}

impl Kind {
    /// The machine a block id names, `None` for anything else.
    pub(crate) fn of(block: &str) -> Option<Kind> {
        match block {
            ORB => Some(Kind::Orb),
            _ => furnator_of(block)
                .map(Kind::Furnator)
                .or_else(|| cell_of(block).map(Kind::Cell))
                .or_else(|| cable_of(block).map(Kind::Cable))
                .or_else(|| rod_of(block).map(Kind::Rod))
                .or_else(|| discharger_of(block).map(Kind::Discharger))
                .or_else(|| hopper_of(block).map(Kind::Hopper)),
        }
    }
}

/// The furnator tier a block id names, with its numbers; `None` for any other
/// block. The ids are the content's own (`data/powah/horizon/block/furnator_*`).
pub(crate) fn furnator_of(block: &str) -> Option<FurnatorTier> {
    let index = FURNATORS.iter().position(|(id, _)| *id == block)?;
    furnator_tier(index)
}

/// The energy cable tier a block id names, with its rate; `None` for any other
/// block. The ids are the content's own (`data/powah/horizon/block/energy_cable_*`).
pub(crate) fn cable_of(block: &str) -> Option<CableTier> {
    let index = CABLES.iter().position(|(id, _)| *id == block)?;
    cable_tier(index)
}

/// The energy cell a block id names, with its tier's numbers; `None` for any other
/// block. The ids are the content's own (`data/powah/horizon/block/energy_cell_*`).
pub(crate) fn cell_of(block: &str) -> Option<CellTier> {
    let index = CELLS.iter().position(|(id, _)| *id == block)?;
    cell_tier(index)
}

/// The energy discharger a block id names, with its tier's numbers; `None` for any other
/// block. The ids are the content's own (`data/powah/horizon/block/energy_discharger_*`).
pub(crate) fn discharger_of(block: &str) -> Option<DischargerTier> {
    let index = DISCHARGERS.iter().position(|(id, _)| *id == block)?;
    discharger_tier(index)
}

/// The energy hopper a block id names, with its tier's numbers; `None` for any other block. The
/// ids are the content's own (`data/powah/horizon/block/energy_hopper_*`).
pub(crate) fn hopper_of(block: &str) -> Option<HopperTier> {
    let index = HOPPERS.iter().position(|(id, _)| *id == block)?;
    crate::runtime::hopper_tier(index)
}

/// What `/powah set` did, so the command can answer honestly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SetOutcome {
    /// The buffer now holds this much.
    Set(i32),
    /// No Powah machine stands there.
    NoBlock,
    /// The cable stores nothing, so there is nothing to set.
    Cable,
}

/// One block entity type, built in `Hooks::register`: the type's id is this mod's
/// namespace plus `name`, and `name` is the one block allowed to hold it.
pub(crate) fn declare(registrar: &mut dyn Registrar, name: &str) -> BlockEntityTypeKey {
    registrar.block_entity_type(name).blocks(&[name]).build()
}

/// The furnator's container: the charge slot then the fuel slot, every face
/// reaching both, no menu (this port has no GUI). Upstream's own `canInsert`
/// gates the slots (fuel in 1, chargeables in 0); a container declaration cannot
/// express a per-slot rule at this pin — a refusal on one slot cancels a click on
/// another (HANDOVER §9l) — so the tick decides instead, as it already did for
/// the fuel.
pub(crate) fn container() -> ModContainer {
    ModContainer {
        slots: FURNATOR_SLOTS,
        faces: std::array::from_fn(|_| vec![CHARGE_SLOT, FUEL_SLOT]),
        inputs: vec![CHARGE_SLOT, FUEL_SLOT],
        outputs: vec![CHARGE_SLOT, FUEL_SLOT],
        menu: None,
    }
}

/// A furnator's buffer (`AbstractEnergyProvider`): the tier's capacity, filled by
/// its own burn only, given out at the tier's transfer a side.
pub(crate) fn furnator_battery(tier: FurnatorTier) -> ModEnergyBuffer {
    ModEnergyBuffer {
        capacity: tier.capacity,
        max_insert: 0,
        max_extract: tier.transfer,
        insert_faces: [false; 6],
        extract_faces: [true; 6],
    }
}

/// The cell's buffer (`AbstractEnergyStorage`): the tier's capacity, its transfer a
/// side both ways.
pub(crate) fn cell_battery(tier: CellTier) -> ModEnergyBuffer {
    ModEnergyBuffer {
        capacity: tier.capacity,
        max_insert: tier.transfer,
        max_extract: tier.transfer,
        insert_faces: [true; 6],
        extract_faces: [true; 6],
    }
}

/// A discharger's buffer (`EnergyDischargerBlock` over `AbstractEnergyStorage` with
/// `getTransferType()` = `Transfer.EXTRACT`): the tier's capacity, giving its transfer
/// a side and taking nothing through any face — its FE comes from the items in its
/// slots, never from a neighbour.
pub(crate) fn discharger_battery(tier: DischargerTier) -> ModEnergyBuffer {
    ModEnergyBuffer {
        capacity: tier.capacity,
        max_insert: 0,
        max_extract: tier.transfer,
        insert_faces: [false; 6],
        extract_faces: [true; 6],
    }
}

/// The buffer a block declares; the cable stores nothing (upstream's capacity is
/// 0), so it declares none and is only ever a relay.
pub(crate) fn battery_of(block: &str) -> Option<ModEnergyBuffer> {
    match Kind::of(block) {
        Some(Kind::Furnator(tier)) => Some(furnator_battery(tier)),
        Some(Kind::Cell(tier)) => Some(cell_battery(tier)),
        Some(Kind::Discharger(tier)) => Some(discharger_battery(tier)),
        Some(Kind::Hopper(tier)) => Some(hopper_battery(tier)),
        Some(Kind::Orb) => Some(orb_battery()),
        Some(Kind::Rod(tier)) => Some(rod_battery(tier)),
        _ => None,
    }
}

/// An energy hopper's buffer (`EnergyHopperTile` over `AbstractEnergyStorage` with the default
/// `Transfer.ALL`): the tier's capacity, giving and taking its transfer on every face. It never
/// pushes on its own (`postTick` has no `extractFromSides`); a neighbour pulls, as a cell's would.
pub(crate) fn hopper_battery(tier: HopperTier) -> ModEnergyBuffer {
    ModEnergyBuffer {
        capacity: tier.capacity,
        max_insert: tier.transfer,
        max_extract: tier.transfer,
        insert_faces: [true; 6],
        extract_faces: [true; 6],
    }
}

/// The furnator saved in a block entity's compound, clamped into its tier; a
/// fresh one is empty.
pub(crate) fn read(saved: &Tag, tier: FurnatorTier) -> Furnator {
    Furnator {
        energy: clamp_energy(int(saved, KEY_ENERGY), tier.capacity),
        carbon: int(saved, KEY_CARBON).max(0),
    }
}

/// Write the tick's result back under upstream's keys: the buffer's amount
/// (`energy`, the int the energy capability reads) and the carbon left.
pub(crate) fn write(saved: &mut Tag, state: &Furnator) {
    let Tag::Compound(nbt) = saved else {
        return;
    };
    nbt.insert(KEY_ENERGY.into(), Tag::Int(state.energy));
    nbt.insert(KEY_CARBON.into(), Tag::Int(state.carbon));
}

/// A saved int, 0 when absent or of another type.
fn int(saved: &Tag, key: &str) -> i32 {
    match saved.get(key) {
        Some(Tag::Int(value)) => *value,
        _ => 0,
    }
}

/// The bookmark this mod writes for "no recipe". Empty rather than absent because
/// a [`Compound`]'s keys are Java strings this module cannot name, so a removal
/// would need one.
const NO_RECIPE: &str = "";

/// A saved string, `None` when absent, of another type or the empty marker.
fn string(saved: &Tag, key: &str) -> Option<String> {
    match saved.get(key) {
        Some(Tag::String(value)) => {
            let value = value.to_string_lossy();
            (!value.is_empty()).then_some(value)
        }
        _ => None,
    }
}

/// A saved position (an int array of three), `None` when absent or malformed.
fn read_pos(saved: &Tag, key: &str) -> Option<[i32; 3]> {
    match saved.get(key) {
        Some(Tag::IntArray(pos)) if pos.len() == 3 => Some([pos[0], pos[1], pos[2]]),
        _ => None,
    }
}

/// Write an int under `key`.
fn write_int(saved: &mut Tag, key: &str, value: i32) {
    if let Tag::Compound(nbt) = saved {
        nbt.insert(key.into(), Tag::Int(value));
    }
}

/// Write a string under `key`.
fn write_string(saved: &mut Tag, key: &str, value: &str) {
    if let Tag::Compound(nbt) = saved {
        nbt.insert(key.into(), Tag::String(value.into()));
    }
}

/// The orb's state as its tick and `/powah info` read it.
pub(crate) struct OrbState {
    pub energy: i32,
    /// The input slots' item ids, in slot order.
    pub inputs: Vec<String>,
    pub recipe: Option<&'static EnergizingRecipe>,
}

/// The filled input slots of `items` (slot 0 is the result, 1..6 the inputs).
fn input_ids(host: &dyn Host, items: &[Option<mod_api::ItemStack>]) -> Vec<String> {
    items
        .iter()
        .skip(1)
        .flatten()
        .map(|stack| stack.item().id(host))
        .collect()
}

/// A loaded orb's state: its buffer and the recipe its six input slots match.
pub(crate) fn read_orb(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
) -> Option<OrbState> {
    let entity = host.block_entity(level, pos)?;
    let saved = Tag::Compound(entity.data(&*host)?);
    let mut inputs = Vec::new();
    for slot in 1..ORB_SLOTS as u32 {
        // A slot the entity does not have yet (this pin grows a mod entity's
        // `items` with its container, but a freshly placed orb answers before its
        // first tick) reads as empty rather than as a missing orb.
        if let Ok(Some(stack)) = entity.item_slot(&*host, slot) {
            inputs.push(stack.item().id(&*host));
        }
    }
    let recipe = {
        let ids: Vec<&str> = inputs.iter().map(String::as_str).collect();
        crate::runtime::energize(&ids)
    };
    Some(OrbState {
        energy: int(&saved, KEY_ENERGY),
        inputs,
        recipe,
    })
}

/// The orb's tick (`EnergizingOrbTile`): record the recipe its inputs match, keep
/// the buffer the rods fill, and produce when it reaches the recipe's energy. The
/// payload is left `Changed` only when something was written, so a waiting orb
/// does not dirty its chunk every tick and a rod's fill survives it.
fn orb_tick(host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) -> bool {
    let resized = event.entity.items.len() != ORB_SLOTS;
    if resized {
        event.entity.items.resize(ORB_SLOTS, None);
    }
    let mut saved = Tag::Compound(std::mem::take(&mut event.entity.nbt));
    let energy = int(&saved, KEY_ENERGY);
    let recorded = string(&saved, KEY_RECIPE);
    let inputs = input_ids(&*host, &event.entity.items);
    let ids: Vec<&str> = inputs.iter().map(String::as_str).collect();
    let step = crate::runtime::orb_step(&ids, energy, recorded.as_deref());
    let mut changed = resized;

    match step.recipe {
        Some(recipe) if step.done => {
            // Upstream's `inv.clear()` then slot 0: the inputs become the result.
            let output = host
                .item_key(&Identifier::parse(recipe.output).expect("a registry id"))
                .and_then(|item| mod_api::ItemStack::new(item, recipe.count.unsigned_abs()));
            event.entity.items.fill(None);
            event.entity.items[ORB_OUTPUT_SLOT] = output;
            write_string(&mut saved, KEY_RECIPE, NO_RECIPE);
            changed = true;
        }
        Some(recipe) => {
            if recorded.as_deref() != Some(recipe.output) {
                write_string(&mut saved, KEY_RECIPE, recipe.output);
                changed = true;
            }
        }
        None => {
            if recorded.is_some() {
                write_string(&mut saved, KEY_RECIPE, NO_RECIPE);
                changed = true;
            }
        }
    }
    if step.energy != energy {
        write_int(&mut saved, KEY_ENERGY, step.energy);
        changed = true;
    }
    if let Tag::Compound(nbt) = saved {
        event.entity.nbt = nbt;
    }
    changed
}

/// One rod tick's move into its orb (`EnergizingRodTile.postTick`): find the orb,
/// warm up for 20 ticks while the orb has a recipe, then move
/// `min(stored, transfer)` into the orb's remaining room and take exactly what
/// the orb accepted. Answers whether anything was written.
fn rod_tick(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3], tier: RodTier) -> bool {
    let Some(rod) = host.block_entity(level, pos) else {
        return false;
    };
    let Some(saved) = rod.data(&*host) else {
        return false;
    };
    let saved = Tag::Compound(saved);
    let mut patch = Compound::new();

    // The link: upstream sets it on placement and clears it when the orb goes, so
    // a missing, broken or out-of-range orb is searched for again here.
    let recorded = read_pos(&saved, KEY_ORB);
    let linked = recorded.filter(|orb| {
        in_range(pos, *orb)
            && block_at(&*host, level, *orb).as_deref().and_then(Kind::of) == Some(Kind::Orb)
    });
    let orb_pos = linked.or_else(|| find_orb(host, level, pos));
    if orb_pos != recorded {
        patch.insert(
            KEY_ORB.into(),
            Tag::IntArray(orb_pos.unwrap_or([0, 0, 0]).to_vec()),
        );
    }

    // A rod only gives to an orb that holds a recipe (`containRecipe`), and that
    // is what warms its 20-tick counter up; without one the counter cools.
    let awaiting = orb_pos.and_then(|orb| {
        let state = read_orb(host, level, orb)?;
        Some((orb, state.recipe?))
    });
    let cooldown = crate::runtime::rod_cooldown(awaiting.is_some(), int(&saved, KEY_COOLDOWN));
    if int(&saved, KEY_COOLDOWN) != cooldown {
        patch.insert(KEY_COOLDOWN.into(), Tag::Int(cooldown));
    }

    let mut applied = false;
    if let Some((orb, recipe)) = awaiting
        && cooldown >= crate::runtime::ROD_WARMUP
    {
        // Both sides are read again here, after every earlier move of this phase
        // has run, so a cable or a cell that touched either one is not undone.
        let rod_energy = rod
            .data(&*host)
            .map_or(0, |nbt| int(&Tag::Compound(nbt), KEY_ENERGY));
        let Some(orb_entity) = host.block_entity(level, orb) else {
            return false;
        };
        let Some(orb_saved) = orb_entity.data(&*host) else {
            return false;
        };
        let orb_energy = int(&Tag::Compound(orb_saved), KEY_ENERGY);
        let amount = crate::runtime::orb_fill(rod_energy, tier.transfer, orb_energy, recipe.energy);
        if amount > 0 {
            let mut rod_patch = Compound::new();
            rod_patch.insert(KEY_ENERGY.into(), Tag::Int(rod_energy - amount));
            let mut orb_patch = Compound::new();
            orb_patch.insert(KEY_ENERGY.into(), Tag::Int(orb_energy + amount));
            applied = host
                .transaction(|host, _tx| -> Result<bool, ()> {
                    rod.merge_data(host, rod_patch.clone()).map_err(|_| ())?;
                    orb_entity
                        .merge_data(host, orb_patch.clone())
                        .map_err(|_| ())?;
                    Ok(true)
                })
                .unwrap_or(false);
        }
    }

    if !patch.is_empty() {
        let _ = rod.merge_data(host, patch);
        return true;
    }
    applied
}

/// The nearest orb within `energizing_range` of `pos`, in `BlockPos.betweenClosed`
/// order (the order upstream's scan takes and keeps the first of).
fn find_orb(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3]) -> Option<[i32; 3]> {
    for y in -crate::runtime::ENERGIZING_RANGE..=crate::runtime::ENERGIZING_RANGE {
        for x in -crate::runtime::ENERGIZING_RANGE..=crate::runtime::ENERGIZING_RANGE {
            for z in -crate::runtime::ENERGIZING_RANGE..=crate::runtime::ENERGIZING_RANGE {
                let candidate = [pos[0] + x, pos[1] + y, pos[2] + z];
                if candidate == pos || candidate == [0, 0, 0] {
                    continue;
                }
                if block_at(&*host, level, candidate)
                    .as_deref()
                    .and_then(Kind::of)
                    == Some(Kind::Orb)
                {
                    return Some(candidate);
                }
            }
        }
    }
    None
}

/// Whether `orb` stands within `energizing_range` of `rod` (Chebyshev, as
/// `BlockPos.betweenClosed` boxes it).
fn in_range(rod: [i32; 3], orb: [i32; 3]) -> bool {
    let range = crate::runtime::ENERGIZING_RANGE;
    (0..3).all(|axis| (rod[axis] - orb[axis]).abs() <= range)
}

/// One machine's readout line, exactly as 0.1.0 printed it (the line the
/// in-game check's assertions match, so it must not drift). The charge slots a
/// machine has are named after its fuel (a furnator's) or its comparator (a
/// cell's), so every earlier assertion still matches its prefix.
pub(crate) fn status_line(
    kind: Kind,
    pos: [i32; 3],
    energy: i32,
    carbon: i32,
    fuel: Option<(&str, u32)>,
    charge: &[ChargeReadout<'_>],
    time: i64,
) -> String {
    let [x, y, z] = pos;
    match kind {
        Kind::Furnator(tier) => {
            let fuel = match fuel {
                Some((item, count)) => format!("{item} x{count}"),
                None => "empty".to_owned(),
            };
            format!(
                "Furnator at {x},{y},{z}: {energy}/{} FE (comparator {}, \
                 carbon {carbon}, burning {}, fuel {fuel}, charge {}, {} FE/t a side, t={time})",
                tier.capacity,
                comparator_strength(energy, tier.capacity),
                carbon > 0,
                charge_text(charge),
                tier.transfer,
            )
        }
        Kind::Cell(tier) => format!(
            "Energy Cell at {x},{y},{z}: {energy}/{} FE (comparator {}, charge {}, {} FE/t a \
             side, t={time})",
            tier.capacity,
            comparator_strength(energy, tier.capacity),
            charge_text(charge),
            tier.transfer,
        ),
        // A discharger is the cell's mirror: the same buffer, and its seven slots are
        // the ones it drains, so the readout names them in the same `charge` field.
        Kind::Discharger(tier) => format!(
            "Energy Discharger at {x},{y},{z}: {energy}/{} FE (comparator {}, charge {}, {} FE/t \
             a side, t={time})",
            tier.capacity,
            comparator_strength(energy, tier.capacity),
            charge_text(charge),
            tier.transfer,
        ),
        Kind::Cable(tier) => format!(
            "Energy Cable at {x},{y},{z}: no storage, moves up to {} FE/t (t={time})",
            tier.transfer
        ),
        // The hopper's readout names the face it charges through, which only [`info_line`] can
        // read from the block state; this arm is the type's exhaustiveness.
        Kind::Hopper(tier) => hopper_line(pos, energy, tier, None, time),
        // The orb and a rod have readouts of their own ([`orb_line`], [`rod_line`]);
        // [`info_line`] gives them those, so these arms are only the type's
        // exhaustiveness.
        Kind::Orb => orb_line(pos, energy, None, 0, time),
        Kind::Rod(tier) => rod_line(pos, energy, tier, None, time),
    }
}

/// An energy hopper's readout: its buffer, the face it charges through, its tier's charging rate
/// and the transfer every face gives and takes. `facing` is the block state's own value — [`None`]
/// only in [`status_line`]'s exhaustiveness arm, which has no block state to read.
pub(crate) fn hopper_line(
    pos: [i32; 3],
    energy: i32,
    tier: HopperTier,
    facing: Option<Direction>,
    time: i64,
) -> String {
    let [x, y, z] = pos;
    let facing = facing.map_or("unknown", facing_name);
    format!(
        "Energy Hopper at {x},{y},{z}: {energy}/{} FE (comparator {}, facing {facing}, charging \
         {} FE/t, {} FE/t a side, t={time})",
        tier.capacity,
        comparator_strength(energy, tier.capacity),
        tier.charging_rate,
        tier.transfer,
    )
}

/// The `facing` block state's value for a direction, as the block definition spells it.
pub(crate) fn facing_name(direction: Direction) -> &'static str {
    match direction {
        Direction::Down => "down",
        Direction::Up => "up",
        Direction::North => "north",
        Direction::South => "south",
        Direction::West => "west",
        Direction::East => "east",
    }
}

/// The direction a block definition's `facing` value names, `None` for anything else.
pub(crate) fn facing_of(name: &str) -> Option<Direction> {
    Direction::ALL
        .into_iter()
        .find(|direction| facing_name(*direction) == name)
}

/// The direction a clicked block face names, `None` for a variant this pin does not have (the
/// placed block then keeps its definition's default `facing`).
fn face_direction(face: BlockFace) -> Option<Direction> {
    Some(match face {
        BlockFace::Down => Direction::Down,
        BlockFace::Up => Direction::Up,
        BlockFace::North => Direction::North,
        BlockFace::South => Direction::South,
        BlockFace::West => Direction::West,
        BlockFace::East => Direction::East,
        _ => return None,
    })
}

/// The charge slots of a readout, comma-separated: `<item> <held>/<capacity>`, or
/// `empty` for a slot with nothing in it.
fn charge_text(charge: &[ChargeReadout<'_>]) -> String {
    if charge.is_empty() {
        return "none".to_owned();
    }
    charge
        .iter()
        .map(|slot| match slot {
            Some((item, held, capacity)) => format!("{item} {held}/{capacity}"),
            None => "empty".to_owned(),
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// The energizing rod tier a block id names, with its numbers; `None` for any
/// other block. The ids are the content's own
/// (`data/powah/horizon/block/energizing_rod_*`).
pub(crate) fn rod_of(block: &str) -> Option<RodTier> {
    let index = RODS.iter().position(|(id, _)| *id == block)?;
    rod_tier(index)
}

/// The orb's container: seven slots, slot 0 the result and 1..6 the inputs
/// (`this.inv.set(7)`, slot limit 1 on the inputs). Automation may fill any
/// input slot and take from the result slot, as upstream's `IInventoryHolder`
/// allows; there is no menu (no GUI in this port).
pub(crate) fn orb_container() -> ModContainer {
    ModContainer {
        slots: ORB_SLOTS,
        faces: std::array::from_fn(|_| (0..ORB_SLOTS).collect()),
        inputs: (1..ORB_SLOTS).collect(),
        outputs: vec![ORB_OUTPUT_SLOT],
        menu: None,
    }
}

/// The orb's buffer: it takes FE and gives none (`fillEnergy`), up to the largest
/// energizing recipe.
pub(crate) fn orb_battery() -> ModEnergyBuffer {
    ModEnergyBuffer {
        capacity: ORB_MAX_ENERGY,
        max_insert: ORB_MAX_ENERGY,
        max_extract: 0,
        insert_faces: [true; 6],
        extract_faces: [false; 6],
    }
}

/// A rod's buffer (`AbstractEnergyStorage`): its tier's capacity, taking and
/// giving its tier's transfer a side. Upstream lets a rod reach FE only on the
/// face its `facing` state names; a declaration cannot vary by state, so every
/// face is open (recorded as a deviation in `notes.md`).
pub(crate) fn rod_battery(tier: RodTier) -> ModEnergyBuffer {
    ModEnergyBuffer {
        capacity: tier.capacity,
        max_insert: tier.transfer,
        max_extract: tier.transfer,
        insert_faces: [true; 6],
        extract_faces: [true; 6],
    }
}

/// The orb's readout: its buffer, the recipe its inputs match (or none) and how
/// many input slots are filled.
pub(crate) fn orb_line(
    pos: [i32; 3],
    energy: i32,
    recipe: Option<&'static EnergizingRecipe>,
    inputs: usize,
    time: i64,
) -> String {
    let [x, y, z] = pos;
    match recipe {
        Some(recipe) => format!(
            "Energizing Orb at {x},{y},{z}: {energy}/{} FE (recipe {} x{}, inputs {inputs}, \
             t={time})",
            recipe.energy, recipe.output, recipe.count
        ),
        None => format!("Energizing Orb at {x},{y},{z}: 0/0 FE (no recipe, t={time})"),
    }
}

/// A rod's readout: its buffer, its tier's transfer and the orb it is linked to.
pub(crate) fn rod_line(
    pos: [i32; 3],
    energy: i32,
    tier: RodTier,
    orb: Option<[i32; 3]>,
    time: i64,
) -> String {
    let [x, y, z] = pos;
    let orb = match orb {
        Some([ox, oy, oz]) => format!("orb {ox},{oy},{oz}"),
        None => "no orb".to_owned(),
    };
    format!(
        "Energizing Rod at {x},{y},{z}: {energy}/{} FE ({} FE/t a side, {orb}, t={time})",
        tier.capacity, tier.transfer
    )
}

/// The machines whose block entity ticked this tick, per level, drained at the
/// end of that level's tick. The SDK cannot enumerate a level's block entities
/// and a ticking one is out of the world, so the tick leaves its position here
/// and the moves run where every machine is back in the world.
#[derive(Default)]
pub(crate) struct Queue {
    pending: HashMap<LevelRef, BTreeSet<[i32; 3]>>,
}

impl Queue {
    /// Queue `pos` for the level the machine's tick ran in.
    pub(crate) fn push(&mut self, level: LevelRef, pos: [i32; 3]) {
        self.pending.entry(level).or_default().insert(pos);
    }

    /// Everything queued for `level`, in position order (empty afterwards).
    pub(crate) fn take(&mut self, level: LevelRef) -> BTreeSet<[i32; 3]> {
        self.pending.remove(&level).unwrap_or_default()
    }

    /// Forget every queued position: a world opened or closed.
    pub(crate) fn clear(&mut self) {
        self.pending.clear();
    }
}

/// The level the server ticks now: the one a block-entity tick runs in.
pub(crate) fn level_now(host: &dyn ServerHost) -> Option<LevelRef> {
    host.level(host.dimension())
}

/// The id of the block at `pos`, if there is one.
pub(crate) fn block_at(host: &dyn Host, level: LevelRef, pos: [i32; 3]) -> Option<String> {
    Some(level.block(host, pos)?.id(host))
}

/// The two usages `/powah` declares, all operator-only (level 2). They are
/// administrative: `set` writes a machine's stored FE, `item` writes the charge
/// of the item in its charge slot, and `info` reads its slots, at coordinates the
/// caller chooses, so they are operator diagnostics rather than gameplay (review
/// finding 2026-11-18).
pub(crate) fn declare_commands(commands: &mut Vec<Command>) {
    commands.push(Command::op("powah info <pos:block_pos>", 2));
    commands.push(Command::op("powah set <amount:integer> <pos:block_pos>", 2));
    commands.push(Command::op(
        "powah item <amount:integer> <pos:block_pos>",
        2,
    ));
}

/// A machine's charge slots as its readout shows them: each slot's item (with the
/// buffer it carries) or an empty slot.
fn charge_readout(
    host: &mut dyn ServerHost,
    entity: mod_api::BlockEntityRef,
    slots: usize,
) -> Vec<Option<(String, i32, i32)>> {
    (0..slots)
        .map(|slot| {
            let stack = entity
                .item_slot(&*host, u32::try_from(slot).unwrap_or(u32::MAX))
                .ok()
                .flatten()?;
            let item = stack.item().id(&*host).to_owned();
            let capacity = item_buffer(&item).map_or(0, |buffer| buffer.capacity);
            Some((
                item,
                crate::runtime::clamp_energy(held_energy(&stack), capacity),
                capacity,
            ))
        })
        .collect()
}

/// A machine's readout, or `None` when no Powah machine stands there. The block id
/// is the test, so a chest at that position is "no Powah block", not a machine at
/// 0 FE.
pub(crate) fn info_line(host: &mut dyn ServerHost, pos: [i32; 3]) -> Option<String> {
    let level = level_now(host)?;
    let block = block_at(&*host, level, pos)?;
    let kind = Kind::of(&block)?;
    let slots = item_slots(&block);
    let time = level.game_time(&*host).unwrap_or(0);
    let (energy, carbon, fuel, charge) = match kind {
        Kind::Furnator(tier) => {
            let entity = host.block_entity(level, pos)?;
            let state = read(&Tag::Compound(entity.data(&*host)?), tier);
            let fuel = entity
                .item_slot(&*host, FUEL_SLOT as u32)
                .ok()
                .flatten()
                .map(|stack| (stack.item().id(&*host), stack.count()));
            let charge = charge_readout(host, entity, slots);
            (state.energy, state.carbon, fuel, charge)
        }
        Kind::Cell(tier) | Kind::Discharger(tier) => {
            let entity = host.block_entity(level, pos)?;
            let saved = Tag::Compound(entity.data(&*host)?);
            let charge = charge_readout(host, entity, slots);
            (
                clamp_energy(int(&saved, KEY_ENERGY), tier.capacity),
                0,
                None,
                charge,
            )
        }
        Kind::Cable(_) => (0, 0, None, Vec::new()),
        Kind::Hopper(tier) => {
            let entity = host.block_entity(level, pos)?;
            let saved = Tag::Compound(entity.data(&*host)?);
            // `EnergyHopperTile.postTick` reads the container on `state.facing`'s neighbour, so the
            // readout names that face too.
            let facing = level.block_state(&*host, pos).and_then(|state| {
                state
                    .property(&host.block_states(), "facing")
                    .ok()
                    .flatten()
                    .and_then(facing_of)
            });
            return Some(hopper_line(
                pos,
                clamp_energy(int(&saved, KEY_ENERGY), tier.capacity),
                tier,
                facing,
                time,
            ));
        }
        Kind::Orb => {
            let orb = read_orb(host, level, pos)?;
            return Some(orb_line(
                pos,
                clamp_energy(orb.energy, orb.recipe.map_or(0, |recipe| recipe.energy)),
                orb.recipe,
                orb.inputs.len(),
                time,
            ));
        }
        Kind::Rod(tier) => {
            let entity = host.block_entity(level, pos)?;
            let saved = Tag::Compound(entity.data(&*host)?);
            let orb = read_pos(&saved, KEY_ORB);
            return Some(rod_line(
                pos,
                clamp_energy(int(&saved, KEY_ENERGY), tier.capacity),
                tier,
                orb,
                time,
            ));
        }
    };
    let fuel = fuel.as_ref().map(|(item, count)| (item.as_str(), *count));
    let charge: Vec<ChargeReadout<'_>> = charge
        .iter()
        .map(|slot| {
            slot.as_ref()
                .map(|(item, held, capacity)| (item.as_str(), *held, *capacity))
        })
        .collect();
    Some(status_line(kind, pos, energy, carbon, fuel, &charge, time))
}

/// Set a furnator's, a cell's, the orb's or a rod's stored FE (`/powah set`): the
/// harness control the in-game check uses. Writing an `energy` key into a foreign
/// block's compound would be a foreign key in someone else's data, so only this
/// mod's blocks take it.
pub(crate) fn set_energy(host: &mut dyn ServerHost, pos: [i32; 3], amount: i32) -> SetOutcome {
    let Some(level) = level_now(host) else {
        return SetOutcome::NoBlock;
    };
    let Some(block) = block_at(&*host, level, pos) else {
        return SetOutcome::NoBlock;
    };
    let capacity = match Kind::of(&block) {
        Some(Kind::Furnator(tier)) => tier.capacity,
        Some(Kind::Cell(tier)) => tier.capacity,
        Some(Kind::Discharger(tier)) => tier.capacity,
        Some(Kind::Hopper(tier)) => tier.capacity,
        Some(Kind::Rod(tier)) => tier.capacity,
        // The orb's buffer is the recipe's own energy: with no recipe it stores
        // nothing, exactly as upstream's `onSlotChanged` leaves it.
        Some(Kind::Orb) => read_orb(host, level, pos)
            .and_then(|orb| orb.recipe)
            .map_or(0, |recipe| recipe.energy),
        Some(Kind::Cable(_)) => return SetOutcome::Cable,
        None => return SetOutcome::NoBlock,
    };
    let energy = clamp_energy(amount, capacity);
    let Some(entity) = host.block_entity(level, pos) else {
        return SetOutcome::NoBlock;
    };
    let mut patch = Compound::new();
    patch.insert(KEY_ENERGY.into(), Tag::Int(energy));
    // `merge_data` is `/data merge block`: it loads the patch and marks the chunk
    // changed, which is the `setChanged` the retired bundle had to fake.
    match entity.merge_data(host, patch) {
        Ok(_) => SetOutcome::Set(energy),
        Err(_) => SetOutcome::NoBlock,
    }
}

/// A furnator tier: a block entity that burns its fuel and ticks at its own
/// generation into its own buffer.
pub(crate) struct FurnatorPart {
    queue: Arc<Mutex<Queue>>,
    tier: FurnatorTier,
}

impl FurnatorPart {
    pub(crate) fn new(queue: Arc<Mutex<Queue>>, tier: FurnatorTier) -> FurnatorPart {
        FurnatorPart { queue, tier }
    }
}

#[mod_api::horizon_part]
impl BlockPart for FurnatorPart {}

#[mod_api::horizon_part]
impl BlockEntityPart for FurnatorPart {
    /// `EntityBlock.getTicker`: it ticks. Never clear a value an earlier mod set.
    fn block_entity_ticks(&self, _host: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    /// One server tick: load the fuel upstream would, burn one tick of carbon,
    /// then queue this position for the level's move phase. The payload is left
    /// `Changed` only when something moved, so an idle furnator does not dirty its
    /// chunk every tick.
    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        let changed = burn(host, event, self.tier);
        self.enqueue(host, event.pos);
        if !changed {
            *event.reply = BlockEntityTickReply::Unchanged;
        }
    }
}

impl FurnatorPart {
    /// Queue `pos` for the level this tick runs in; outside a ticking level there
    /// is nothing to move and nothing is queued.
    fn enqueue(&self, host: &dyn ServerHost, pos: [i32; 3]) {
        if let Some(level) = level_now(host) {
            // Only this expression runs under the lock: one insert on the port's own
            // state, which cannot panic and so cannot poison it.
            self.queue
                .lock()
                .expect("the powah queue lock is held for one insert")
                .push(level, pos);
        }
    }
}

/// What a stored machine's own tick does besides queue: clamp its buffer into its
/// tier, and — for a cell — charge the items in its slots.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct IdleRules {
    /// The tier's capacity: a buffer above it is clamped back on its own tick, so a
    /// doctored payload or a `/data merge block` cannot turn the machine into an
    /// endless source.
    pub capacity: i32,
    /// The tier's per-side transfer, which is also the per-slot charge cap
    /// (`ChargeUtil.chargeItemsInInventory`'s `maxPerSlot`).
    pub transfer: i32,
    /// The item slots it charges (`EnergyCellTile`'s two; a rod none).
    pub slots: usize,
}

/// The cell, the cable and the rod: a block entity that only queues itself for the
/// level tick's end, where the moves run (a rod's move into its orb is one of the
/// level tick's jobs; a cell's charge of its two item slots is here, before the
/// moves, as upstream's `postTick` charges before `extractFromSides`).
///
/// A cell and a rod also clamp their saved buffer on every tick when it is out of
/// their tier; a cable has no buffer and passes [`None`].
pub(crate) struct IdlePart {
    queue: Arc<Mutex<Queue>>,
    rules: Option<IdleRules>,
}

impl IdlePart {
    pub(crate) fn new(queue: Arc<Mutex<Queue>>, rules: Option<IdleRules>) -> IdlePart {
        IdlePart { queue, rules }
    }
}

#[mod_api::horizon_part]
impl BlockPart for IdlePart {}

#[mod_api::horizon_part]
impl BlockEntityPart for IdlePart {
    fn block_entity_ticks(&self, _host: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    /// The cell and the rod store their FE in their saved compound and the cable
    /// stores nothing: their only job is the move phase, plus the cell's item
    /// charge and the clamp of a buffer that is out of its tier.
    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        if let Some(level) = level_now(host) {
            self.queue
                .lock()
                .expect("the powah queue lock is held for one insert")
                .push(level, event.pos);
        }
        let Some(rules) = self.rules else {
            *event.reply = BlockEntityTickReply::Unchanged;
            return;
        };
        let mut saved = Tag::Compound(std::mem::take(&mut event.entity.nbt));
        let before = int(&saved, KEY_ENERGY);
        let mut energy = crate::runtime::clamp_energy(before, rules.capacity);
        let charged = charge_items(
            &*host,
            rules.slots,
            energy,
            rules.transfer,
            &mut event.entity.items,
        );
        energy -= charged;
        let changed = energy != before;
        if changed {
            write_int(&mut saved, KEY_ENERGY, energy);
        }
        if let Tag::Compound(nbt) = saved {
            event.entity.nbt = nbt;
        }
        if !changed {
            *event.reply = BlockEntityTickReply::Unchanged;
        }
    }
}

/// An energy hopper tier: a block entity with no slots of its own that queues its position for the
/// level tick's end, where it charges the container on the face its `facing` names, and clamps its
/// saved buffer into its tier.
pub(crate) struct HopperPart {
    queue: Arc<Mutex<Queue>>,
    tier: HopperTier,
}

impl HopperPart {
    pub(crate) fn new(queue: Arc<Mutex<Queue>>, tier: HopperTier) -> HopperPart {
        HopperPart { queue, tier }
    }
}

#[mod_api::horizon_part]
impl BlockPart for HopperPart {}

#[mod_api::horizon_part]
impl BlockEntityPart for HopperPart {
    fn block_entity_ticks(&self, _host: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    /// One server tick: queue this position for the level's move phase (the faced container is
    /// read there, where every block entity is back in the world) and clamp a doctored buffer.
    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        if let Some(level) = level_now(host) {
            self.queue
                .lock()
                .expect("the powah queue lock is held for one insert")
                .push(level, event.pos);
        }
        let mut saved = Tag::Compound(std::mem::take(&mut event.entity.nbt));
        let before = int(&saved, KEY_ENERGY);
        let energy = crate::runtime::clamp_energy(before, self.tier.capacity);
        let changed = energy != before;
        if changed {
            write_int(&mut saved, KEY_ENERGY, energy);
        }
        if let Tag::Compound(nbt) = saved {
            event.entity.nbt = nbt;
        }
        if !changed {
            *event.reply = BlockEntityTickReply::Unchanged;
        }
    }
}

/// The orb: a block entity with the seven slots and the recipe-gated buffer, which
/// ticks to fill itself and to turn a full buffer into its result.
pub(crate) struct OrbPart;
#[mod_api::horizon_part]
impl BlockPart for OrbPart {}

#[mod_api::horizon_part]
impl BlockEntityPart for OrbPart {
    fn block_entity_ticks(&self, _host: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    /// One server tick: the recipe the inputs match, the buffer's value and the
    /// result, as `EnergizingOrbTile` decides them.
    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        if !orb_tick(host, event) {
            *event.reply = BlockEntityTickReply::Unchanged;
        }
    }
}

/// The furnator's tick: `initBurn` (`AbstractGeneratorBlock`), one tick of
/// production, the item charge (`postTick`'s `chargeItems(1)`) and the state
/// written back. Answers whether anything changed.
fn burn(
    host: &mut dyn ServerHost,
    event: &mut BlockEntityTickEvent<'_>,
    tier: FurnatorTier,
) -> bool {
    let resized = event.entity.items.len() != FURNATOR_SLOTS;
    if resized {
        event.entity.items.resize(FURNATOR_SLOTS, None);
    }
    let mut saved = Tag::Compound(std::mem::take(&mut event.entity.nbt));
    let before = read(&saved, tier);
    let mut state = before;
    let mut took = false;
    if state.wants_fuel() {
        let mut fuel = FuelSlot {
            host: &*host,
            entity: &mut event.entity,
        };
        let burn_time = fuel.peek();
        if state.load_fuel(burn_time) {
            fuel.take();
            took = true;
        }
    }
    let produced = state.tick(tier) > 0;
    // `postTick` charges the charge slot after producing, so a tick's production can
    // leave on the same tick; the buffer is debited by exactly what the item took.
    let charged = charge_items(
        &*host,
        kind_item_slots(Kind::Furnator(tier)),
        state.energy,
        tier.transfer,
        &mut event.entity.items,
    );
    state.energy -= charged;
    write(&mut saved, &state);
    if let Tag::Compound(nbt) = saved {
        event.entity.nbt = nbt;
    }
    resized || took || produced || charged > 0
}

/// The machine's item-charging slots, one step per slot as upstream's
/// `ChargeUtil.transferSlotList` takes them: each item takes
/// `min(the machine's transfer, what is left of its buffer, the item's own room)`,
/// the buffer is debited by what the item took, and a slot whose item holds no
/// buffer (a stone in a charge slot) is left alone. Answers the FE that left the
/// buffer.
fn charge_items(
    host: &dyn Host,
    slots: usize,
    stored: i32,
    transfer: i32,
    items: &mut [Option<mod_api::ItemStack>],
) -> i32 {
    let mut budget = stored.max(0);
    let mut charged = 0;
    for slot in 0..slots.min(items.len()) {
        if budget <= 0 {
            break;
        }
        let Some(stack) = items[slot].as_ref() else {
            continue;
        };
        let id = stack.item().id(host);
        let Some(buffer) = item_buffer(&id) else {
            continue;
        };
        let held = held_energy(stack);
        let room = buffer.capacity.saturating_sub(held.max(0));
        // The item's own capability caps one call too (`Info.maxInsert`).
        let moved = crate::runtime::charge_step(budget, transfer.min(buffer.max_insert), room);
        if moved <= 0 {
            continue;
        }
        let Some(charged_stack) = with_payload_energy(host, stack, held.saturating_add(moved))
        else {
            continue;
        };
        items[slot] = Some(charged_stack);
        budget -= moved;
        charged += moved;
    }
    charged
}

/// The machine items, resolved once at registration: `block_drops` runs with a
/// read-only level host, which cannot name an item, so the drop is matched by the
/// keys remembered here.
static ITEMS: OnceLock<RwLock<HashMap<&'static str, ItemKey>>> = OnceLock::new();

/// The block keys' table, filled by [`remember_item`].
fn items() -> &'static RwLock<HashMap<&'static str, ItemKey>> {
    ITEMS.get_or_init(|| RwLock::new(HashMap::new()))
}

/// Remember the item a machine block's own drop resolves to (registration time).
pub(crate) fn remember_item(block: &'static str, item: ItemKey) {
    items()
        .write()
        .expect("the powah item table is written at registration only")
        .insert(block, item);
}

/// The item key of the machine `block` names, once [`remember_item`] ran.
fn item_of(block: &str) -> Option<ItemKey> {
    items()
        .read()
        .expect("the powah item table is read for one lookup")
        .get(block)
        .copied()
}

/// `block_drops`: a broken machine's own item carries its buffer, so placing the
/// item puts the FE back (upstream's `playerDestroy` → `storeToStack`, and
/// `keepEnergy` for every furnator, cell and rod).
///
/// The block entity is out of the world by now; the engine holds its saved data
/// for this event (`level.block_entity_data`). The payload is built from the
/// buffer alone: the engine spills a container's stacks itself, so carrying
/// `Items` would duplicate them.
pub(crate) fn on_block_drops(level: &dyn LevelHost, event: &mut BlockDropsEvent<'_>) -> Flow {
    let Some(block) = level.state_block(event.state) else {
        return Flow::Continue;
    };
    let Some(capacity) = keep_capacity(block) else {
        return Flow::Continue;
    };
    let Some(entity) = event.block_entity else {
        return Flow::Continue;
    };
    let Some(saved) = level.block_entity_data(entity) else {
        return Flow::Continue;
    };
    let energy = int(&Tag::Compound(saved), KEY_ENERGY);
    let Some(patch) = carried_patch(energy, capacity, block) else {
        return Flow::Continue;
    };
    let Some(item) = item_of(block) else {
        return Flow::Continue;
    };
    for drop in event.drops.iter_mut() {
        if drop.stack.item() == item {
            let _ = drop.stack.apply_components(&patch);
        }
    }
    Flow::Continue
}

/// The item half of every machine: a placed machine takes the buffer its item
/// carried, clamped into the tier's capacity.
///
/// The engine has already applied the payload to the new block entity, so a
/// doctored `/give` (2,000,000,000 FE on a 20,000 FE furnator) is clamped here;
/// a machine placed by a dispenser reaches here with no placer, and every
/// machine's own tick clamps again.
pub(crate) struct MachineItem;

#[mod_api::horizon_part]
impl ItemPart for MachineItem {
    fn on_block_placed_by(&self, host: &mut dyn ServerHost, event: &mut OnBlockPlacedByEvent<'_>) {
        let Some(level) = level_now(host) else {
            return;
        };
        // `AbstractBlock`'s `Facing.ALL` placement: an energy hopper's `facing` is the face it was
        // placed against — `getStateForPlacement`'s `getNearestLookingDirection().getOpposite()`
        // for every face-aligned placement. The pin exposes no player rotation, so the clicked face
        // is the value; a hopper a dispenser places keeps the block's default `north`.
        if let Some(block) = block_at(&*host, level, event.pos)
            && matches!(Kind::of(&block), Some(Kind::Hopper(_)))
            && let Some(facing) = face_direction(event.face)
        {
            host.set_block_property(event.pos, "facing", facing_name(facing));
        }
        let Some(asked) = asked_energy(payload_of(event.stack).as_ref()) else {
            return;
        };
        let Some(block) = block_at(&*host, level, event.pos) else {
            return;
        };
        let Some(capacity) = keep_capacity(&block) else {
            return;
        };
        // A payload is stack-scoped: a stack of more than one (unreachable for this
        // mod's unstackable machine items, reachable for a doctored `/give`) splits
        // it instead of giving every item the whole buffer.
        let count = i32::try_from(event.stack.count()).unwrap_or(1).max(1);
        let wanted = crate::runtime::restored_energy(asked / count, capacity);
        let Some(entity) = host.block_entity(level, event.pos) else {
            return;
        };
        let current = entity
            .data(&*host)
            .map_or(0, |nbt| int(&Tag::Compound(nbt), KEY_ENERGY));
        if current == wanted {
            return;
        }
        let mut patch = Compound::new();
        patch.insert(KEY_ENERGY.into(), Tag::Int(wanted));
        let _ = entity.merge_data(host, patch);
    }
}

/// One of this mod's machine items as `Capabilities.Energy.ITEM`: the buffer in
/// the stack's own `minecraft:block_entity_data` payload, which is also what
/// placement reads — one store, so a machine item charged in a slot gives the
/// machine that charge (see `notes.md`, upstream defect fixed).
pub(crate) struct MachineEnergy {
    place: ItemPlace,
    block: &'static str,
    buffer: ItemBuffer,
}

impl MachineEnergy {
    pub(crate) fn new(place: ItemPlace, block: &'static str, buffer: ItemBuffer) -> MachineEnergy {
        MachineEnergy {
            place,
            block,
            buffer,
        }
    }

    /// What the item at the place holds, clamped into its buffer.
    fn held(&self, host: &mut dyn ServerHost) -> i32 {
        match self.handler().read(host) {
            Some(Tag::Compound(payload)) => crate::runtime::clamp_energy(
                asked_energy(Some(&payload)).unwrap_or(0),
                self.buffer.capacity,
            ),
            _ => 0,
        }
    }

    /// The component handler of the item at the place.
    fn handler(&self) -> ItemComponentHandler {
        ItemComponentHandler::new(
            self.place.clone(),
            ComponentId::new(BLOCK_ENTITY_DATA)
                .expect("`minecraft:block_entity_data` is a namespaced component id"),
        )
    }

    /// Rewrites the item as holding `energy`: `moved` when that went through.
    fn rewrite(&self, host: &mut dyn ServerHost, tx: &Transaction, energy: i32, moved: i32) -> i32 {
        let mut payload = Compound::new();
        payload.insert("id".into(), Tag::String(self.block.into()));
        payload.insert(KEY_ENERGY.into(), Tag::Int(energy));
        if self.handler().write(host, tx, Some(Tag::Compound(payload))) {
            moved
        } else {
            0
        }
    }
}

impl EnergyHandler for MachineEnergy {
    /// What the item holds (`EnergyBlockItem`'s `Info` over the payload).
    fn amount(&self, host: &mut dyn ServerHost) -> u32 {
        u32::try_from(self.held(host)).unwrap_or(0)
    }

    fn capacity(&self, _: &mut dyn ServerHost) -> u32 {
        u32::try_from(self.buffer.capacity).unwrap_or(0)
    }

    fn insert(&self, host: &mut dyn ServerHost, tx: &Transaction, amount: u32) -> u32 {
        let held = self.held(host);
        let room = self.buffer.capacity.saturating_sub(held);
        let moved = crate::runtime::charge_step(
            i32::try_from(amount).unwrap_or(i32::MAX),
            self.buffer.max_insert,
            room,
        );
        match moved {
            0 => 0,
            moved => u32::try_from(self.rewrite(host, tx, held.saturating_add(moved), moved))
                .unwrap_or(0),
        }
    }

    fn extract(&self, host: &mut dyn ServerHost, tx: &Transaction, amount: u32) -> u32 {
        let held = self.held(host);
        let moved = i32::try_from(amount)
            .unwrap_or(i32::MAX)
            .min(self.buffer.max_extract)
            .min(held);
        match moved {
            0 => 0,
            moved => u32::try_from(self.rewrite(host, tx, held.saturating_sub(moved), moved))
                .unwrap_or(0),
        }
    }
}

/// What `/powah item` did, so the command can answer honestly.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SetItemOutcome {
    /// The charge slot's item now holds this much.
    Set(String, i32),
    /// The machine has no charge slot (the cable, the orb, a rod).
    NoSlot,
    /// The charge slot holds no item, or one that carries no buffer.
    NotChargeable,
    /// No Powah machine stands there.
    NoBlock,
}

/// Set the FE the item in a machine's charge slot carries (`/powah item`): the
/// harness control the in-game check uses to leave an item an exact room. The
/// item is rewritten through its own place's component handler, the write a
/// charge takes, so a stack splits one off.
pub(crate) fn set_item_energy(
    host: &mut dyn ServerHost,
    pos: [i32; 3],
    amount: i32,
) -> SetItemOutcome {
    let Some(level) = level_now(host) else {
        return SetItemOutcome::NoBlock;
    };
    let Some(block) = block_at(&*host, level, pos) else {
        return SetItemOutcome::NoBlock;
    };
    if Kind::of(&block).is_none() {
        return SetItemOutcome::NoBlock;
    }
    if item_slots(&block) == 0 {
        return SetItemOutcome::NoSlot;
    }
    let Some(entity) = host.block_entity(level, pos) else {
        return SetItemOutcome::NoBlock;
    };
    let Ok(Some(stack)) = entity.item_slot(&*host, CHARGE_SLOT as u32) else {
        return SetItemOutcome::NotChargeable;
    };
    let item = stack.item().id(&*host).to_owned();
    let Some(buffer) = item_buffer(&item) else {
        return SetItemOutcome::NotChargeable;
    };
    let energy = crate::runtime::clamp_energy(amount, buffer.capacity);
    let mut payload = Compound::new();
    payload.insert("id".into(), Tag::String(item.as_str().into()));
    payload.insert(KEY_ENERGY.into(), Tag::Int(energy));
    let place = ItemPlace::BlockEntity {
        level,
        pos,
        side: None,
        slot: CHARGE_SLOT,
    };
    let handler = ItemComponentHandler::new(
        place,
        ComponentId::new(BLOCK_ENTITY_DATA)
            .expect("`minecraft:block_entity_data` is a namespaced component id"),
    );
    let written = host
        .transaction(|host, tx| {
            Ok::<bool, std::convert::Infallible>(handler.write(
                host,
                tx,
                Some(Tag::Compound(payload)),
            ))
        })
        .unwrap_or(false);
    if written {
        SetItemOutcome::Set(item, energy)
    } else {
        SetItemOutcome::NotChargeable
    }
}

/// The fuel slot, as `AbstractGeneratorBlock.initBurn` reads it: the item's
/// smelting burn time and the shrink (or the crafting remainder) that takes it.
struct FuelSlot<'a, 'g> {
    host: &'a dyn ServerHost,
    entity: &'a mut mod_api::ModBlockEntityView<'g>,
}

impl FuelSlot<'_, '_> {
    /// The burn time the item in the fuel slot gives, 0 when there is none.
    fn peek(&self) -> u32 {
        let Some(Some(stack)) = self.entity.items.get(FUEL_SLOT) else {
            return 0;
        };
        burn_time(self.host, stack)
    }

    /// `ItemStack.shrink(1)` plus `getCraftingRemainingItem`: a lava bucket leaves
    /// its bucket.
    fn take(&mut self) {
        let Some(Some(stack)) = self.entity.items.get(FUEL_SLOT) else {
            return;
        };
        let left = stack.clone().shrink(1);
        self.entity.items[FUEL_SLOT] = left.or_else(|| remaining(self.host, stack));
    }
}

/// The engine's fuel value (`minecraft:cooking_fuel.burn_time`), 0 if none.
fn burn_time(host: &dyn Host, stack: &mod_api::ItemStack) -> u32 {
    host.item_burn_time(stack.item()).unwrap_or(0).max(0) as u32
}

/// `ItemStack.getCraftingRemainingItem`: the bucket a lava bucket leaves.
fn remaining(host: &dyn Host, stack: &mod_api::ItemStack) -> Option<mod_api::ItemStack> {
    host.item_crafting_remainder(stack)
}

/// After the level's tick: run the queued moves. A block entity is out of the
/// world during its own tick, so every machine reads its neighbours here, where
/// they are all back in the world.
pub(crate) fn on_level_tick_post(
    host: &mut dyn ServerHost,
    level: LevelRef,
    queued: BTreeSet<[i32; 3]>,
) {
    // The cables already covered by a network this tick: every cable in a network
    // ticks, so without this a run of N cables would plan the same relay N times.
    let mut planned: BTreeSet<[i32; 3]> = BTreeSet::new();
    for pos in queued {
        let Some(kind) = block_at(&*host, level, pos).as_deref().and_then(Kind::of) else {
            continue;
        };
        match kind {
            Kind::Furnator(tier) => push_out(host, level, pos, tier.transfer),
            Kind::Cell(tier) => push_out(host, level, pos, tier.transfer),
            // A discharger fills from its own slots first, then pushes: upstream's
            // `postTick` discharges and then runs `extractFromSides`.
            Kind::Discharger(tier) => {
                drain_items(host, level, pos, tier);
                push_out(host, level, pos, tier.transfer);
            }
            // A rod gives to its orb in the same phase; the orb itself moves
            // nothing (it has no neighbours to give to).
            Kind::Rod(tier) => {
                rod_tick(host, level, pos, tier);
            }
            // A hopper charges the container on the face its `facing` names, out of its own
            // buffer (`EnergyHopperTile.postTick`), and pushes nothing anywhere.
            Kind::Hopper(tier) => {
                charge_container(host, level, pos, tier);
            }
            Kind::Orb => {}
            Kind::Cable(tier) => {
                if planned.contains(&pos) {
                    continue;
                }
                planned.extend(relay_network(host, level, pos, tier));
            }
        }
    }
}

/// Upstream's `extractFromSides`: at most the transfer and what is stored, into
/// every side whose neighbour can take it. One transaction per side, so a refused
/// insert creates no energy.
fn push_out(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3], transfer: i32) {
    let Some(own) = level.capability(&mut *host, &ENERGY, pos, Some(Direction::Up)) else {
        return;
    };
    let stored = i32::try_from(own.amount(host)).unwrap_or(i32::MAX);
    let amount = push_amount(stored, transfer);
    if amount == 0 {
        return;
    }
    for side in Direction::ALL {
        let neighbour = offset(pos, side);
        // Upstream's `extractFromSides` skips a side whose neighbour refuses a fill; this
        // mod's own blocks say so in their declaration (`furnator_battery` has no insert
        // face), and a foreign block is tried, its handler refusing.
        let Some(target_block) = block_at(&*host, level, neighbour) else {
            continue;
        };
        if battery_of(&target_block)
            .is_some_and(|buffer| !buffer.insert_faces[side.opposite() as usize])
        {
            continue;
        }
        let Some(target) = level.capability(&mut *host, &ENERGY, neighbour, Some(side.opposite()))
        else {
            continue;
        };
        let room = target.capacity(host).saturating_sub(target.amount(host));
        let want = u32::try_from(amount).unwrap_or(0).min(room);
        if want == 0 {
            continue;
        }
        let _ = host.transaction(|host, tx| -> Result<u32, ()> {
            let Some(own) = level.capability(&mut *host, &ENERGY, pos, Some(side)) else {
                return Ok(0);
            };
            let Some(target) =
                level.capability(&mut *host, &ENERGY, neighbour, Some(side.opposite()))
            else {
                return Ok(0);
            };
            move_energy(host, &own, &target, want, tx).map_err(|_| ())
        });
    }
}

/// A discharger's item pass, in the level's move phase where every block entity is back in
/// the world: `EnergyDischargerTile.postTick` runs
/// `dischargeItemsInInventory(inv, transfer, capacity - stored)` before `extractFromSides`,
/// so one pass spends the machine's transfer out of each slot against one room budget.
///
/// Each slot's FE comes out of the item's own `Capabilities.Energy.ITEM` handler in a
/// transaction — for this mod's machines that is [`MachineEnergy::extract`], so a machine
/// item dropped in a discharger gives the buffer its charge, and a foreign energy item
/// answers for itself. An item with no handler (a stone) is left alone, and the buffer is
/// credited by exactly what moved.
fn drain_items(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3], tier: DischargerTier) {
    let Some(entity) = host.block_entity(level, pos) else {
        return;
    };
    let Some(saved) = entity.data(&*host) else {
        return;
    };
    let stored = clamp_energy(int(&Tag::Compound(saved), KEY_ENERGY), tier.capacity);
    let mut room = tier.capacity - stored;
    let mut taken = 0;
    for slot in 0..DISCHARGER_SLOTS {
        if room <= 0 {
            break;
        }
        let place = ItemPlace::BlockEntity {
            level,
            pos,
            side: None,
            slot,
        };
        let Some(item) = place.capability(host, &mod_api::ITEM_ENERGY) else {
            continue;
        };
        let held = i32::try_from(item.amount(host)).unwrap_or(i32::MAX);
        let want = crate::runtime::discharge_step(held, tier.transfer, room);
        let want = u32::try_from(want).unwrap_or(0);
        if want == 0 {
            continue;
        }
        let moved = host
            .transaction(|host, tx| -> Result<u32, ()> { Ok(item.extract(host, tx, want)) })
            .unwrap_or(0);
        let moved = i32::try_from(moved.min(want)).unwrap_or(0);
        if moved == 0 {
            continue;
        }
        room -= moved;
        taken += moved;
    }
    if taken == 0 {
        return;
    }
    let mut patch = Compound::new();
    patch.insert(KEY_ENERGY.into(), Tag::Int(stored + taken));
    let _ = entity.merge_data(host, patch);
}

/// A hopper's item pass, in the level's move phase where every block entity is back in the world:
/// `EnergyHopperTile.postTick` reads the block entity on the face its `facing` names, and — when
/// that block holds a container — spends `ChargeUtil.chargeItemsInContainer(container,
/// charging_rates.get(variant), energy.getStored())` on it, then consumes what went out of its own
/// buffer. It never pushes anything (`postTick` has no `extractFromSides`).
///
/// Each item's FE goes in through its own `Capabilities.Energy.ITEM` handler in a transaction, so
/// this mod's machine items are charged by [`MachineEnergy::insert`] and a foreign battery answers
/// for itself; a slot with no energy item (a stone) has no handler and is left alone.
fn charge_container(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3], tier: HopperTier) {
    let Some(state) = level.block_state(&*host, pos) else {
        return;
    };
    let facing = state
        .property(&host.block_states(), "facing")
        .ok()
        .flatten()
        .and_then(facing_of);
    let Some(facing) = facing else {
        return;
    };
    let container = offset(pos, facing);
    // The `ITEMS` capability is the game's own container read, as `world.getBlockEntity(...)
    // instanceof Container` is upstream's test.
    let Some(handler) = level.capability(host, &mod_api::ITEMS, container, None) else {
        return;
    };
    let size = handler.size(host);
    if size == 0 {
        return;
    }
    let Some(entity) = host.block_entity(level, pos) else {
        return;
    };
    let Some(saved) = entity.data(&*host) else {
        return;
    };
    let stored = clamp_energy(int(&Tag::Compound(saved), KEY_ENERGY), tier.capacity);
    let mut rooms = Vec::with_capacity(size);
    for slot in 0..size {
        let place = ItemPlace::BlockEntity {
            level,
            pos: container,
            side: None,
            slot,
        };
        let room = place
            .capability(host, &mod_api::ITEM_ENERGY)
            .map_or(0, |item| {
                let held = i32::try_from(item.amount(host)).unwrap_or(i32::MAX);
                let capacity = i32::try_from(item.capacity(host)).unwrap_or(i32::MAX);
                capacity.saturating_sub(held).max(0)
            });
        rooms.push(room);
    }
    let mut charged = 0;
    for (slot, asked) in crate::runtime::hopper_charges(stored, tier.charging_rate, &rooms)
        .into_iter()
        .enumerate()
    {
        let asked = u32::try_from(asked).unwrap_or(0);
        if asked == 0 {
            continue;
        }
        let place = ItemPlace::BlockEntity {
            level,
            pos: container,
            side: None,
            slot,
        };
        let Some(item) = place.capability(host, &mod_api::ITEM_ENERGY) else {
            continue;
        };
        let moved = host
            .transaction(|host, tx| -> Result<u32, ()> { Ok(item.insert(host, tx, asked)) })
            .unwrap_or(0);
        charged += i32::try_from(moved.min(asked)).unwrap_or(0);
    }
    if charged == 0 {
        return;
    }
    let mut patch = Compound::new();
    patch.insert(KEY_ENERGY.into(), Tag::Int(stored - charged));
    let _ = entity.merge_data(host, patch);
}

/// The largest cable network one tick walks. Upstream's `CableNet` is unbounded;
/// this cap keeps one very long run from costing a tick, and is far past any
/// pack's cable run (recorded as a deviation from upstream's unbounded walk).
const NETWORK_MAX_CABLES: usize = 1024;

/// One external energy side of a cable network: the block the relay reaches and
/// the side of the network cable it hangs off, so the face the capability is read
/// and moved on is `cable_side.opposite()`.
#[derive(Clone, Copy)]
struct Port {
    pos: [i32; 3],
    cable_side: Direction,
}

/// The cables one network holds: every same-tier cable reachable from `start`
/// through cables only, in position order. A network stops at the first block
/// that is not a cable, which is what makes a run of cables relay while a wall
/// of stone does not.
fn discover_cables(
    host: &mut dyn ServerHost,
    level: LevelRef,
    start: [i32; 3],
    tier: CableTier,
) -> BTreeSet<[i32; 3]> {
    let mut cables = BTreeSet::from([start]);
    let mut queue = VecDeque::from([start]);
    while let Some(pos) = queue.pop_front() {
        for side in Direction::ALL {
            if cables.len() >= NETWORK_MAX_CABLES {
                break;
            }
            let next = offset(pos, side);
            if cables.contains(&next) {
                continue;
            }
            // Upstream's `CableNet` joins cables of one tier only: a basic run
            // does not relay into a starter one (`different tiers never connect`).
            if block_at(&*host, level, next).as_deref().and_then(Kind::of)
                == Some(Kind::Cable(tier))
            {
                cables.insert(next);
                queue.push_back(next);
            }
        }
    }
    cables
}

/// A cable's tick: gather the whole network, then plan and run its moves once.
/// Upstream's `CableNet` forwards one insertion through every connected cable of
/// the same tier; the port pulls, so one plan for the whole network per tick is
/// the same relay — a run of two or more cables now moves FE, where slice 1
/// moved it only between the block entities beside one cable.
///
/// Answers the network it covered, so the level tick does not plan it again.
fn relay_network(
    host: &mut dyn ServerHost,
    level: LevelRef,
    start: [i32; 3],
    tier: CableTier,
) -> BTreeSet<[i32; 3]> {
    let cables = discover_cables(host, level, start, tier);
    let mut ports: Vec<Neighbour> = Vec::new();
    let mut places: Vec<Port> = Vec::new();
    for cable in &cables {
        for side in Direction::ALL {
            let pos = offset(*cable, side);
            let Some(block) = block_at(&*host, level, pos) else {
                continue;
            };
            if matches!(Kind::of(&block), Some(Kind::Cable(_))) {
                continue; // inside the network: neither a source nor a sink
            }
            // A block touching two cables of one network is one place: reading it
            // through both cables would plan the same energy twice.
            if places.iter().any(|place| place.pos == pos) {
                continue;
            }
            let Some(buffer) = neighbour(host, level, pos, side.opposite()) else {
                continue;
            };
            ports.push(buffer);
            places.push(Port {
                pos,
                cable_side: side,
            });
        }
    }
    let rotation = level.game_time(&*host).unwrap_or(0).max(0) as usize;
    for step in network_moves(&ports, tier.transfer, rotation) {
        let from = places[step.from];
        let to = places[step.to];
        let want = u32::try_from(step.amount).unwrap_or(0);
        let _ = host.transaction(|host, tx| -> Result<u32, ()> {
            let Some(source) = level.capability(
                &mut *host,
                &ENERGY,
                from.pos,
                Some(from.cable_side.opposite()),
            ) else {
                return Ok(0);
            };
            let Some(target) =
                level.capability(&mut *host, &ENERGY, to.pos, Some(to.cable_side.opposite()))
            else {
                return Ok(0);
            };
            move_energy(host, &source, &target, want, tx).map_err(|_| ())
        });
    }
    cables
}

/// A neighbour's buffer as the cable reaches it. The capability answers `amount`,
/// `capacity` and the moves themselves; the per-call limits and the faces are this
/// mod's own declaration, read back through [`battery_of`], so its own three blocks
/// are planned with the numbers they really enforce. A foreign block's limits are
/// not visible through the capability, so its handler caps the move instead.
fn neighbour(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
    side: Direction,
) -> Option<Neighbour> {
    let block = block_at(&*host, level, pos)?;
    let handler = level.capability(&mut *host, &ENERGY, pos, Some(side))?;
    let energy = i32::try_from(handler.amount(host)).unwrap_or(i32::MAX);
    let capacity = i32::try_from(handler.capacity(host)).unwrap_or(i32::MAX);
    let buffer = battery_of(&block);
    let face = side as usize;
    Some(Neighbour {
        energy,
        capacity,
        max_insert: buffer.as_ref().map_or(i32::MAX, |b| b.max_insert),
        max_extract: buffer.as_ref().map_or(i32::MAX, |b| b.max_extract),
        can_insert: buffer.as_ref().is_none_or(|b| b.insert_faces[face]),
        can_extract: buffer.as_ref().is_none_or(|b| b.extract_faces[face]),
    })
}

/// The neighbouring position one side leads to.
fn offset(pos: [i32; 3], side: Direction) -> [i32; 3] {
    let [dx, dy, dz] = side.normal();
    [pos[0] + dx, pos[1] + dy, pos[2] + dz]
}

/// `/powah`'s block position argument, as the dispatcher resolved it.
pub(crate) fn block_pos(argument: Option<&CommandArgumentValue>) -> Option<[i32; 3]> {
    match argument? {
        CommandArgumentValue::BlockPos(pos) => Some(*pos),
        _ => None,
    }
}

/// `/powah`'s amount argument.
pub(crate) fn integer(argument: Option<&CommandArgumentValue>) -> Option<i32> {
    match argument? {
        CommandArgumentValue::Integer(amount) => Some(*amount),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::{CELL_CAPACITY, CELL_TRANSFER, FURNATOR_CAPACITY, FURNATOR_TRANSFER};

    /// The review finding (2026-11-18): both usages act on any loaded machine at
    /// player-chosen coordinates, so a level-0 registration lets a non-operator
    /// create or drain FE and read another player's fuel. Operator-only is the
    /// fix; this test fails on a `Command::new` registration (level 0).
    #[test]
    fn the_harness_commands_are_operator_only() {
        let mut commands = Vec::new();
        declare_commands(&mut commands);
        assert_eq!(
            commands.len(),
            3,
            "every usage is declared: `info`, `set` and the item charge control"
        );
        let usages: Vec<&str> = commands
            .iter()
            .map(|command| command.usage.as_str())
            .collect();
        assert!(
            usages.contains(&"powah item <amount:integer> <pos:block_pos>"),
            "the item charge control is a usage of its own: {usages:?}"
        );
        for command in &commands {
            assert!(
                command.usage.starts_with("powah "),
                "{} is this mod's",
                command.usage
            );
            assert_eq!(
                command.level, 2,
                "{} is an administrative harness control",
                command.usage
            );
        }
    }

    #[test]
    fn every_energy_cell_tier_is_one_of_the_ladder() {
        let ids = [
            ("powah:energy_cell_starter", 1_000_000, 1_000),
            ("powah:energy_cell_basic", 4_000_000, 4_000),
            ("powah:energy_cell_hardened", 10_000_000, 10_000),
            ("powah:energy_cell_blazing", 40_000_000, 40_000),
            ("powah:energy_cell_niotic", 100_000_000, 100_000),
            ("powah:energy_cell_spirited", 400_000_000, 400_000),
            ("powah:energy_cell_nitro", 2_000_000_000, 2_000_000),
        ];
        for (index, (id, capacity, transfer)) in ids.into_iter().enumerate() {
            let cell = cell_of(id).unwrap_or_else(|| panic!("{id} is an energy cell"));
            assert_eq!(
                (cell.capacity, cell.transfer),
                (capacity, transfer),
                "{id} is the ladder's tier {index}"
            );
            assert_eq!(
                cell,
                cell_tier(index).expect("the same tier in the rule's table")
            );
        }
        assert_eq!(
            cell_of("powah:energy_cell_creative"),
            None,
            "the creative cell is not carried"
        );
        assert_eq!(cell_of(FURNATOR), None, "a furnator is not a cell");
        assert_eq!(cell_of("minecraft:chest"), None);
    }

    #[test]
    fn a_block_id_maps_to_its_machine_and_nothing_else_does() {
        assert_eq!(
            Kind::of(FURNATOR),
            Some(Kind::Furnator(furnator_tier(0).expect("the starter tier")))
        );
        assert_eq!(
            Kind::of(CELL),
            Some(Kind::Cell(cell_tier(0).expect("the starter tier")))
        );
        assert_eq!(
            Kind::of(CABLE),
            Some(Kind::Cable(cable_tier(0).expect("the starter tier")))
        );
        assert_eq!(Kind::of("minecraft:chest"), None);
        assert_eq!(
            Kind::of("powah:furnator_basic"),
            Some(Kind::Furnator(furnator_tier(1).expect("the basic tier"))),
            "a later furnator tier"
        );
        assert_eq!(
            Kind::of("powah:energy_cable_basic"),
            Some(Kind::Cable(cable_tier(1).expect("the basic cable"))),
            "a later cable tier"
        );
    }

    #[test]
    fn every_furnator_tier_is_one_of_the_ladder_with_its_own_buffer() {
        let expected = [
            ("powah:furnator_starter", 20, 20_000, 80),
            ("powah:furnator_basic", 80, 80_000, 320),
            ("powah:furnator_hardened", 200, 200_000, 800),
            ("powah:furnator_blazing", 800, 800_000, 3_200),
            ("powah:furnator_niotic", 2_000, 2_000_000, 8_000),
            ("powah:furnator_spirited", 8_000, 8_000_000, 32_000),
            ("powah:furnator_nitro", 40_000, 40_000_000, 160_000),
        ];
        for (index, (id, generation, capacity, transfer)) in expected.into_iter().enumerate() {
            let tier = furnator_of(id).unwrap_or_else(|| panic!("{id} is a furnator"));
            assert_eq!(
                (tier.generation, tier.capacity, tier.transfer),
                (generation, capacity, transfer),
                "{id} is the ladder's tier {index}"
            );
            assert_eq!(
                tier,
                furnator_tier(index).expect("the same tier in the rule's table")
            );
            let buffer = battery_of(id).unwrap_or_else(|| panic!("{id} declares a buffer"));
            assert_eq!(buffer.capacity, capacity, "{id}'s capacity");
            assert_eq!(buffer.max_insert, 0, "{id} makes its own FE");
            assert_eq!(buffer.max_extract, transfer, "{id}'s extract cap");
            assert_eq!(buffer.insert_faces, [false; 6], "{id} takes no FE in");
            assert_eq!(
                buffer.extract_faces, [true; 6],
                "{id} gives FE on every face"
            );
        }
        assert_eq!(furnator_of(CELL), None, "a cell is not a furnator");
        assert_eq!(furnator_of("minecraft:furnace"), None);
    }

    #[test]
    fn every_cable_tier_is_one_of_the_ladder_and_declares_no_buffer() {
        let expected = [
            ("powah:energy_cable_starter", 500),
            ("powah:energy_cable_basic", 2_000),
            ("powah:energy_cable_hardened", 5_000),
            ("powah:energy_cable_blazing", 20_000),
            ("powah:energy_cable_niotic", 50_000),
            ("powah:energy_cable_spirited", 200_000),
            ("powah:energy_cable_nitro", 1_000_000),
        ];
        for (index, (id, transfer)) in expected.into_iter().enumerate() {
            let tier = cable_of(id).unwrap_or_else(|| panic!("{id} is an energy cable"));
            assert_eq!(tier.transfer, transfer, "{id} is the ladder's tier {index}");
            assert_eq!(
                tier,
                cable_tier(index).expect("the same tier in the rule's table")
            );
            assert_eq!(battery_of(id), None, "{id} stores nothing");
        }
        assert_eq!(cable_of(FURNATOR), None, "a furnator is not a cable");
        assert_eq!(cable_of("minecraft:chest"), None);
    }

    #[test]
    fn the_furnator_declares_an_extract_only_buffer() {
        let buffer = furnator_battery(furnator_tier(0).expect("the starter tier"));
        assert_eq!(buffer.capacity, FURNATOR_CAPACITY);
        assert_eq!(buffer.max_insert, 0, "it makes its own FE");
        assert_eq!(buffer.max_extract, FURNATOR_TRANSFER);
        assert_eq!(buffer.insert_faces, [false; 6]);
        assert_eq!(buffer.extract_faces, [true; 6]);
    }

    #[test]
    fn the_cell_declares_a_buffer_both_ways() {
        let starter = cell_tier(0).expect("the starter tier");
        let buffer = cell_battery(starter);
        assert_eq!(buffer.capacity, CELL_CAPACITY);
        assert_eq!(buffer.max_insert, CELL_TRANSFER);
        assert_eq!(buffer.max_extract, CELL_TRANSFER);
        assert_eq!(buffer.insert_faces, [true; 6]);
        assert_eq!(buffer.extract_faces, [true; 6]);
        assert_eq!(battery_of(CELL), Some(buffer));
    }

    #[test]
    fn every_cell_tier_declares_its_own_buffer_and_nothing_else_does() {
        for (index, (block, _)) in CELLS.iter().enumerate() {
            let tier = cell_tier(index).expect("a tier on the ladder");
            assert_eq!(
                battery_of(block),
                Some(cell_battery(tier)),
                "{block} declares tier {index}'s numbers"
            );
        }
        assert_eq!(battery_of("powah:energy_cell_creative"), None);
    }

    #[test]
    fn the_cable_declares_no_buffer_because_it_stores_nothing() {
        assert_eq!(battery_of(CABLE), None);
        assert_eq!(battery_of("minecraft:chest"), None);
    }

    #[test]
    fn a_fresh_compound_is_an_empty_furnator() {
        assert_eq!(
            read(
                &Tag::Compound(Compound::new()),
                furnator_tier(0).expect("the starter tier")
            ),
            Furnator::new()
        );
    }

    #[test]
    fn the_tick_s_state_reads_back_from_the_compound_it_wrote() {
        let mut saved = Tag::Compound(Compound::new());
        let state = Furnator {
            energy: 20_000,
            carbon: 46_800,
        };
        write(&mut saved, &state);
        assert_eq!(
            read(&saved, furnator_tier(0).expect("the starter tier")),
            state
        );
        assert_eq!(
            saved.get(KEY_ENERGY),
            Some(&Tag::Int(20_000)),
            "the buffer is the int `energy` the energy capability reads"
        );
        assert_eq!(saved.get(KEY_CARBON), Some(&Tag::Int(46_800)));
    }

    #[test]
    fn every_energizing_rod_tier_is_one_of_the_ladder_with_its_own_buffer() {
        let expected = [
            ("powah:energizing_rod_starter", 10_000, 100),
            ("powah:energizing_rod_basic", 40_000, 400),
            ("powah:energizing_rod_hardened", 100_000, 1_000),
            ("powah:energizing_rod_blazing", 400_000, 4_000),
            ("powah:energizing_rod_niotic", 1_000_000, 10_000),
            ("powah:energizing_rod_spirited", 4_000_000, 40_000),
            ("powah:energizing_rod_nitro", 20_000_000, 200_000),
        ];
        for (index, (id, capacity, transfer)) in expected.into_iter().enumerate() {
            let tier = rod_of(id).unwrap_or_else(|| panic!("{id} is an energizing rod"));
            assert_eq!(
                (tier.capacity, tier.transfer),
                (capacity, transfer),
                "{id} is the ladder's tier {index}"
            );
            assert_eq!(
                tier,
                rod_tier(index).expect("the same tier in the rule's table")
            );
            let buffer = battery_of(id).unwrap_or_else(|| panic!("{id} declares a buffer"));
            assert_eq!(buffer.capacity, capacity, "{id}'s capacity");
            assert_eq!(buffer.max_insert, transfer, "{id}'s insert cap");
            assert_eq!(buffer.max_extract, transfer, "{id}'s extract cap");
            assert_eq!(
                buffer.insert_faces, [true; 6],
                "{id} takes FE on every face"
            );
            assert_eq!(
                buffer.extract_faces, [true; 6],
                "{id} gives FE on every face"
            );
        }
        assert_eq!(rod_of(ORB), None, "an orb is not a rod");
        assert_eq!(rod_of("powah:energy_cell_starter"), None);
    }

    #[test]
    fn the_orb_declares_a_seven_slot_container_and_a_take_only_buffer() {
        let container = orb_container();
        assert_eq!(container.slots, ORB_SLOTS, "slot 0 plus six inputs");
        assert_eq!(
            container.inputs,
            (1..ORB_SLOTS).collect::<Vec<_>>(),
            "automation fills the six input slots"
        );
        assert_eq!(
            container.outputs,
            vec![ORB_OUTPUT_SLOT],
            "automation takes the result slot"
        );
        assert_eq!(
            container.faces,
            std::array::from_fn(|_| (0..ORB_SLOTS).collect::<Vec<_>>()),
            "every face reaches all seven slots"
        );
        assert_eq!(container.menu, None, "this port has no GUI");

        let buffer = orb_battery();
        assert_eq!(buffer.capacity, ORB_MAX_ENERGY);
        assert_eq!(
            buffer.max_insert, ORB_MAX_ENERGY,
            "upstream's transfer is its capacity"
        );
        assert_eq!(buffer.max_extract, 0, "the orb never gives FE back");
        assert_eq!(buffer.insert_faces, [true; 6]);
        assert_eq!(buffer.extract_faces, [false; 6]);
        assert_eq!(battery_of(ORB), Some(buffer));
    }

    #[test]
    fn the_orb_and_the_rod_print_the_lines_the_e2e_asserts() {
        let steel = crate::runtime::energizing()
            .iter()
            .find(|recipe| recipe.output == "powah:steel_energized")
            .expect("the steel recipe");
        assert_eq!(
            orb_line([0, 100, 20], 0, Some(steel), 2, 120),
            "Energizing Orb at 0,100,20: 0/10000 FE (recipe powah:steel_energized x2, inputs 2, \
             t=120)"
        );
        assert_eq!(
            orb_line([0, 100, 20], 0, None, 0, 40),
            "Energizing Orb at 0,100,20: 0/0 FE (no recipe, t=40)"
        );
        let starter = rod_tier(0).expect("the starter rod");
        assert_eq!(
            rod_line([0, 101, 20], 10_000, starter, Some([0, 100, 20]), 120),
            "Energizing Rod at 0,101,20: 10000/10000 FE (100 FE/t a side, orb 0,100,20, t=120)"
        );
        let nitro = rod_tier(6).expect("the nitro rod");
        assert_eq!(
            rod_line([0, 101, 20], 0, nitro, None, 40),
            "Energizing Rod at 0,101,20: 0/20000000 FE (200000 FE/t a side, no orb, t=40)"
        );
    }

    #[test]
    fn the_info_lines_are_the_ones_the_e2e_asserts() {
        let starter_furnator = Kind::Furnator(furnator_tier(0).expect("the starter tier"));
        assert_eq!(
            status_line(starter_furnator, [0, 100, 0], 0, 0, None, &[None], 150),
            "Furnator at 0,100,0: 0/20000 FE (comparator 0, carbon 0, burning false, fuel empty, \
             charge empty, 80 FE/t a side, t=150)"
        );
        assert_eq!(
            status_line(
                starter_furnator,
                [0, 100, 0],
                320,
                46_800,
                Some(("minecraft:coal", 4)),
                &[None],
                282
            ),
            "Furnator at 0,100,0: 320/20000 FE (comparator 0, carbon 46800, burning true, fuel \
             minecraft:coal x4, charge empty, 80 FE/t a side, t=282)"
        );
        let nitro_furnator = Kind::Furnator(furnator_tier(6).expect("the nitro tier"));
        assert_eq!(
            status_line(
                nitro_furnator,
                [0, 100, 8],
                40_000_000,
                8_000,
                None,
                &[None],
                500
            ),
            "Furnator at 0,100,8: 40000000/40000000 FE (comparator 15, carbon 8000, burning \
             true, fuel empty, charge empty, 160000 FE/t a side, t=500)"
        );
        let starter = Kind::Cell(cell_tier(0).expect("the starter tier"));
        assert_eq!(
            status_line(starter, [2, 100, 0], 1_360, 0, None, &[None, None], 290),
            "Energy Cell at 2,100,0: 1360/1000000 FE (comparator 0, charge empty, empty, 1000 FE/t a \
             side, t=290)"
        );
        assert_eq!(
            status_line(starter, [2, 100, 0], 1_000_000, 0, None, &[None, None], 380),
            "Energy Cell at 2,100,0: 1000000/1000000 FE (comparator 15, charge empty, empty, 1000 FE/t \
             a side, t=380)"
        );
        let nitro = Kind::Cell(cell_tier(6).expect("the nitro tier"));
        assert_eq!(
            status_line(
                nitro,
                [2, 100, 0],
                2_000_000_000,
                0,
                None,
                &[None, None],
                380
            ),
            "Energy Cell at 2,100,0: 2000000000/2000000000 FE (comparator 15, charge empty, empty, \
             2000000 FE/t a side, t=380)"
        );
        assert_eq!(
            status_line(
                Kind::Cable(cable_tier(0).expect("the starter cable")),
                [1, 100, 0],
                0,
                0,
                None,
                &[],
                188
            ),
            "Energy Cable at 1,100,0: no storage, moves up to 500 FE/t (t=188)"
        );
        assert_eq!(
            status_line(
                Kind::Cable(cable_tier(1).expect("the basic cable")),
                [1, 100, 0],
                0,
                0,
                None,
                &[],
                188
            ),
            "Energy Cable at 1,100,0: no storage, moves up to 2000 FE/t (t=188)"
        );
    }

    #[test]
    fn every_furnator_container_is_the_charge_slot_then_the_fuel_slot() {
        let container = container();
        assert_eq!(
            (CHARGE_SLOT, FUEL_SLOT),
            (0, 1),
            "upstream's `FurnatorTile`: 0 chargeables, 1 fuel"
        );
        assert_eq!(container.slots, 2, "`FurnatorTile` has two slots");
        assert_eq!(container.inputs, vec![CHARGE_SLOT, FUEL_SLOT]);
        assert_eq!(
            container.outputs,
            vec![CHARGE_SLOT, FUEL_SLOT],
            "everything may be taken out (`canExtract` is true for both)"
        );
        assert_eq!(
            container.faces,
            std::array::from_fn(|_| vec![CHARGE_SLOT, FUEL_SLOT]),
            "every face reaches both slots"
        );
        assert_eq!(container.menu, None, "this port has no GUI");
    }

    #[test]
    fn every_cell_container_is_its_two_item_slots() {
        let container = cell_container();
        assert_eq!(container.slots, 2, "`EnergyCellTile` charges two slots");
        assert_eq!(container.inputs, vec![0, 1]);
        assert_eq!(container.outputs, vec![0, 1]);
        assert_eq!(container.faces, std::array::from_fn(|_| vec![0, 1]));
        assert_eq!(container.menu, None, "this port has no GUI");
    }

    #[test]
    fn a_machine_item_carries_its_tier_s_buffer() {
        // upstream's `EnergyBlockItem.getEnergyInfo` over the block's config and
        // its `getTransferType`.
        assert_eq!(
            item_buffer(FURNATOR),
            Some(ItemBuffer {
                capacity: 20_000,
                max_insert: 0,
                max_extract: 80,
            }),
            "a furnator gives and never receives (`Transfer.EXTRACT`)"
        );
        assert_eq!(
            item_buffer(CELL),
            Some(ItemBuffer {
                capacity: 1_000_000,
                max_insert: 1_000,
                max_extract: 1_000,
            }),
            "a cell takes and gives its tier's transfer (`Transfer.ALL`)"
        );
        assert_eq!(
            item_buffer("powah:energizing_rod_starter"),
            Some(ItemBuffer {
                capacity: 10_000,
                max_insert: 100,
                max_extract: 100,
            }),
            "a rod is an `AbstractEnergyStorage` like a cell"
        );
        assert_eq!(
            item_buffer(ORB),
            None,
            "the orb is not an energy block item"
        );
        assert_eq!(item_buffer(CABLE), None, "a cable stores nothing");
        assert_eq!(item_buffer("minecraft:stone"), None);
    }

    #[test]
    fn a_charge_slot_exists_only_on_the_machines_that_charge_items() {
        assert_eq!(item_slots(FURNATOR), 1, "the furnator charges slot 0");
        assert_eq!(item_slots(CELL), 2, "a cell charges two slots");
        assert_eq!(item_slots("powah:energy_cell_nitro"), 2);
        assert_eq!(item_slots(CABLE), 0);
        assert_eq!(
            item_slots(ORB),
            0,
            "the orb's seven slots are a recipe, not a charger"
        );
        assert_eq!(item_slots("powah:energizing_rod_starter"), 0);
    }

    #[test]
    fn a_keep_energy_machine_is_the_one_that_carries_its_buffer_on_its_item() {
        for (index, (block, _)) in FURNATORS.into_iter().enumerate() {
            let tier = furnator_tier(index).expect("the ladder's row");
            assert_eq!(
                keep_capacity(block),
                Some(tier.capacity),
                "a furnator keeps its buffer (`keepEnergy` is true)"
            );
        }
        for (index, (block, _)) in CELLS.into_iter().enumerate() {
            let tier = cell_tier(index).expect("the ladder's row");
            assert_eq!(keep_capacity(block), Some(tier.capacity));
        }
        for (index, (block, _)) in RODS.into_iter().enumerate() {
            let tier = rod_tier(index).expect("the ladder's row");
            assert_eq!(keep_capacity(block), Some(tier.capacity));
        }
        assert_eq!(
            keep_capacity(ORB),
            None,
            "the orb keeps nothing: upstream's default is false"
        );
        assert_eq!(keep_capacity(CABLE), None, "a cable has no buffer to keep");
        assert_eq!(keep_capacity("minecraft:stone"), None);
    }

    #[test]
    fn the_readout_names_every_charge_slot() {
        let furnator = Kind::Furnator(furnator_tier(0).expect("the starter tier"));
        let empty = status_line(furnator, [0, 100, 0], 0, 0, None, &[None], 150);
        assert!(
            empty.contains("fuel empty, charge empty, 80 FE/t a side"),
            "an empty charge slot reads `empty`: {empty}"
        );
        let charged = status_line(
            furnator,
            [0, 100, 0],
            40,
            0,
            Some(("minecraft:coal", 1)),
            &[Some(("powah:energy_cell_starter", 40, 1_000_000))],
            282,
        );
        assert!(
            charged.contains("charge powah:energy_cell_starter 40/1000000, 80 FE/t a side"),
            "the charge slot names its item and what it holds: {charged}"
        );
        let cell = Kind::Cell(cell_tier(0).expect("the starter tier"));
        let two = status_line(
            cell,
            [2, 100, 0],
            1_500,
            0,
            None,
            &[
                Some(("powah:energy_cell_starter", 1_000, 1_000_000)),
                Some(("powah:energy_cell_starter", 500, 1_000_000)),
            ],
            300,
        );
        assert!(
            two.contains(
                "charge powah:energy_cell_starter 1000/1000000, \
                 powah:energy_cell_starter 500/1000000, 1000 FE/t a side"
            ),
            "a cell names both of its charge slots: {two}"
        );
        let stone = status_line(
            furnator,
            [0, 100, 0],
            0,
            0,
            None,
            &[Some(("minecraft:stone", 0, 0))],
            150,
        );
        assert!(
            stone.contains("charge minecraft:stone 0/0"),
            "an item without a buffer reads as one with none: {stone}"
        );
    }

    #[test]
    fn every_discharger_tier_is_one_of_the_ladder_with_an_extract_only_buffer() {
        // `PowahConfig.EnergyDevices`: `dischargers = new EnergyConfig(batteryCapacity(),
        // batteryTransfer())`, and `EnergyDischargerBlock.getTransferType()` is EXTRACT.
        let expected = [
            ("powah:energy_discharger_starter", 1_000_000, 1_000),
            ("powah:energy_discharger_basic", 4_000_000, 4_000),
            ("powah:energy_discharger_hardened", 10_000_000, 10_000),
            ("powah:energy_discharger_blazing", 40_000_000, 40_000),
            ("powah:energy_discharger_niotic", 100_000_000, 100_000),
            ("powah:energy_discharger_spirited", 400_000_000, 400_000),
            ("powah:energy_discharger_nitro", 2_000_000_000, 2_000_000),
        ];
        for (index, (id, capacity, transfer)) in expected.into_iter().enumerate() {
            let tier = discharger_of(id).unwrap_or_else(|| panic!("{id} is an energy discharger"));
            assert_eq!(
                (tier.capacity, tier.transfer),
                (capacity, transfer),
                "{id} is the ladder's tier {index}"
            );
            assert_eq!(
                Kind::of(id),
                Some(Kind::Discharger(
                    crate::runtime::discharger_tier(index)
                        .expect("the same tier in the rule's table")
                )),
                "{id} maps to its tier"
            );
            let buffer = battery_of(id).unwrap_or_else(|| panic!("{id} declares a buffer"));
            assert_eq!(buffer.capacity, capacity, "{id}'s capacity");
            assert_eq!(
                buffer.max_insert, 0,
                "{id} takes no FE through any face: `Transfer.EXTRACT`"
            );
            assert_eq!(buffer.max_extract, transfer, "{id}'s extract cap");
            assert_eq!(
                buffer.insert_faces, [false; 6],
                "{id} fills from its items, never from a neighbour"
            );
            assert_eq!(
                buffer.extract_faces, [true; 6],
                "{id} gives FE on every face"
            );
            assert_eq!(
                item_buffer(id),
                Some(ItemBuffer {
                    capacity,
                    max_insert: 0,
                    max_extract: transfer,
                }),
                "{id}'s own item only gives, as the block does"
            );
            assert_eq!(
                keep_capacity(id),
                Some(capacity),
                "{id} keeps its buffer (`keepEnergy` is true)"
            );
            assert_eq!(
                item_slots(id),
                DISCHARGER_SLOTS,
                "{id} holds its seven discharge slots"
            );
        }
        assert_eq!(discharger_of(CELL), None, "a cell is not a discharger");
        assert_eq!(discharger_of("powah:energy_discharger_creative"), None);
        assert_eq!(discharger_of("minecraft:chest"), None);
    }

    #[test]
    fn the_discharger_container_is_its_seven_discharge_slots() {
        let container = discharger_container();
        assert_eq!(
            container.slots, DISCHARGER_SLOTS,
            "`EnergyDischargerTile`: `inv.add(7)`, slot limit 1"
        );
        let all: Vec<usize> = (0..DISCHARGER_SLOTS).collect();
        assert_eq!(container.inputs, all, "automation fills any slot");
        assert_eq!(
            container.outputs, all,
            "everything may be taken out (`canExtract` is true)"
        );
        assert_eq!(
            container.faces,
            std::array::from_fn(|_| all.clone()),
            "every face reaches all seven slots"
        );
        assert_eq!(container.menu, None, "this port has no GUI");
    }

    #[test]
    fn the_discharger_readout_names_its_buffer_and_its_seven_slots() {
        let starter = Kind::Discharger(cell_tier(0).expect("the starter tier"));
        let empty = [None; DISCHARGER_SLOTS];
        assert_eq!(
            status_line(starter, [20, 100, 34], 0, 0, None, &empty, 2600),
            "Energy Discharger at 20,100,34: 0/1000000 FE (comparator 0, charge empty, empty, \
             empty, empty, empty, empty, empty, 1000 FE/t a side, t=2600)"
        );
        let mut charged = empty;
        charged[0] = Some(("powah:energy_cell_starter", 5_000, 1_000_000));
        assert_eq!(
            status_line(starter, [20, 100, 34], 5_000, 0, None, &charged, 2612),
            "Energy Discharger at 20,100,34: 5000/1000000 FE (comparator 0, charge \
             powah:energy_cell_starter 5000/1000000, empty, empty, empty, empty, empty, empty, \
             1000 FE/t a side, t=2612)"
        );
        let stone = status_line(
            starter,
            [20, 100, 34],
            0,
            0,
            None,
            &[Some(("minecraft:stone", 0, 0)); DISCHARGER_SLOTS],
            2620,
        );
        assert!(
            stone.contains("charge minecraft:stone 0/0, minecraft:stone 0/0"),
            "an item with no buffer names none: {stone}"
        );
    }

    #[test]
    fn a_carried_payload_is_the_machine_s_block_entity_data_and_carries_no_items() {
        let patch =
            carried_patch(5_000, 20_000, FURNATOR).expect("a charged furnator carries its buffer");
        let Some(Tag::Compound(payload)) = patch.get(&BLOCK_ENTITY_DATA.into()) else {
            panic!("the payload is the block entity data component: {patch:?}");
        };
        assert_eq!(
            payload.get(&"id".into()),
            Some(&Tag::String(FURNATOR.into())),
            "the engine loads a payload only when its id names the block entity it is placed as"
        );
        assert_eq!(payload.get(&KEY_ENERGY.into()), Some(&Tag::Int(5_000)));
        assert_eq!(
            payload.get(&"Items".into()),
            None,
            "the engine spills a container's stacks itself: a payload with Items would duplicate \
             every stack (HANDOVER §9m)"
        );
        assert_eq!(
            payload.len(),
            2,
            "the payload is the type id and the buffer, nothing else"
        );
        assert_eq!(
            carried_patch(0, 20_000, FURNATOR),
            None,
            "an empty machine carries nothing: upstream writes the storable compound only when it \
             is not empty"
        );
        assert_eq!(
            carried_patch(2_000_000_000, 20_000, FURNATOR),
            carried_patch(20_000, 20_000, FURNATOR),
            "a buffer above the tier is carried clamped"
        );
    }

    #[test]
    fn a_payload_is_read_back_and_believed_only_within_the_tier() {
        let patch = carried_patch(5_000, 1_000_000, CELL).expect("a charged cell");
        let Some(Tag::Compound(payload)) = patch.get(&BLOCK_ENTITY_DATA.into()) else {
            panic!("the payload is the block entity data component: {patch:?}");
        };
        assert_eq!(asked_energy(Some(payload)), Some(5_000));
        let mut doctored = Compound::new();
        doctored.insert("id".into(), Tag::String(CELL.into()));
        doctored.insert(KEY_ENERGY.into(), Tag::Int(2_000_000_000));
        assert_eq!(
            asked_energy(Some(&doctored)),
            Some(2_000_000_000),
            "a payload is read exactly as it states it"
        );
        assert_eq!(
            crate::runtime::restored_energy(2_000_000_000, 1_000_000),
            1_000_000,
            "and clamped into the tier of the machine it is placed as"
        );
        let mut as_long = Compound::new();
        as_long.insert(KEY_ENERGY.into(), Tag::Long(5_000));
        assert_eq!(
            asked_energy(Some(&as_long)),
            Some(5_000),
            "a payload a player wrote as a long is read too"
        );
        assert_eq!(asked_energy(Some(&Compound::new())), None);
        assert_eq!(asked_energy(None), None);
    }

    #[test]
    fn every_hopper_tier_is_one_of_the_ladder_with_a_two_way_buffer_and_no_slots() {
        // `PowahConfig.EnergyDevices`: `hoppers = new ChargingConfig(batteryCapacity(),
        // batteryTransfer(), chargingTransfer())`, and `AbstractEnergyBlock.getTransferType()`
        // is the default `Transfer.ALL`.
        let expected = [
            ("powah:energy_hopper_starter", 1_000_000, 1_000, 500),
            ("powah:energy_hopper_basic", 4_000_000, 4_000, 2_000),
            ("powah:energy_hopper_hardened", 10_000_000, 10_000, 5_000),
            ("powah:energy_hopper_blazing", 40_000_000, 40_000, 20_000),
            ("powah:energy_hopper_niotic", 100_000_000, 100_000, 50_000),
            (
                "powah:energy_hopper_spirited",
                400_000_000,
                400_000,
                200_000,
            ),
            (
                "powah:energy_hopper_nitro",
                2_000_000_000,
                2_000_000,
                1_000_000,
            ),
        ];
        for (index, (id, capacity, transfer, charging_rate)) in expected.into_iter().enumerate() {
            let tier = hopper_of(id).unwrap_or_else(|| panic!("{id} is an energy hopper"));
            assert_eq!(
                (tier.capacity, tier.transfer, tier.charging_rate),
                (capacity, transfer, charging_rate),
                "{id} is the ladder's tier {index}"
            );
            assert_eq!(
                Kind::of(id),
                Some(Kind::Hopper(
                    crate::runtime::hopper_tier(index).expect("the same tier in the rule's table")
                )),
                "{id} maps to its tier"
            );
            let buffer = battery_of(id).unwrap_or_else(|| panic!("{id} declares a buffer"));
            assert_eq!(buffer.capacity, capacity, "{id}'s capacity");
            assert_eq!(
                buffer.max_insert, transfer,
                "{id} takes its transfer through a face: `Transfer.ALL`"
            );
            assert_eq!(buffer.max_extract, transfer, "{id}'s extract cap");
            assert_eq!(
                buffer.insert_faces, [true; 6],
                "{id} takes FE on every face"
            );
            assert_eq!(
                buffer.extract_faces, [true; 6],
                "{id} gives FE on every face"
            );
            assert_eq!(
                item_buffer(id),
                Some(ItemBuffer {
                    capacity,
                    max_insert: transfer,
                    max_extract: transfer,
                }),
                "{id}'s own item both gives and takes, as the block does"
            );
            assert_eq!(
                keep_capacity(id),
                Some(capacity),
                "{id} keeps its buffer (`keepEnergy` is true)"
            );
            assert_eq!(
                item_slots(id),
                0,
                "{id} has no slots of its own: it charges the container it faces"
            );
        }
        assert_eq!(
            hopper_of("powah:energy_hopper_creative"),
            None,
            "there is no hopper past the ladder"
        );
        assert_eq!(
            Kind::of("minecraft:chest"),
            None,
            "a container it faces is not one of its machines"
        );
    }

    #[test]
    fn the_hopper_readout_names_its_buffer_its_facing_and_its_charging_rate() {
        let starter = Kind::Hopper(crate::runtime::hopper_tier(0).expect("the starter tier"));
        assert_eq!(
            hopper_line(
                [20, 100, 40],
                0,
                crate::runtime::hopper_tier(0).unwrap(),
                None,
                2600
            ),
            "Energy Hopper at 20,100,40: 0/1000000 FE (comparator 0, facing unknown, charging \
             500 FE/t, 1000 FE/t a side, t=2600)"
        );
        assert_eq!(
            hopper_line(
                [20, 100, 40],
                5_000,
                crate::runtime::hopper_tier(0).unwrap(),
                Some(Direction::North),
                2612,
            ),
            "Energy Hopper at 20,100,40: 5000/1000000 FE (comparator 0, facing north, charging \
             500 FE/t, 1000 FE/t a side, t=2612)"
        );
        // `status_line` gets the same line for the type's exhaustiveness, with no state to read.
        assert_eq!(
            status_line(starter, [20, 100, 40], 0, 0, None, &[], 2600),
            hopper_line(
                [20, 100, 40],
                0,
                crate::runtime::hopper_tier(0).unwrap(),
                None,
                2600
            )
        );
        let nitro = crate::runtime::hopper_tier(6).expect("the nitro tier");
        assert!(
            hopper_line([1, 2, 3], 2_000_000_000, nitro, Some(Direction::Down), 7)
                .contains("2000000000/2000000000 FE (comparator 15, facing down, charging 1000000 FE/t, 2000000 FE/t a side, t=7)"),
            "the nitro tier names its own numbers"
        );
    }

    #[test]
    fn the_facing_property_names_every_direction_and_nothing_else() {
        for direction in Direction::ALL {
            assert_eq!(
                facing_of(facing_name(direction)),
                Some(direction),
                "{direction:?} is spelled and read back"
            );
        }
        assert_eq!(facing_name(Direction::North), "north");
        assert_eq!(facing_name(Direction::Up), "up");
        assert_eq!(facing_of("NORTH"), None, "the property is lower case");
        assert_eq!(
            facing_of("upward"),
            None,
            "a value the block has not is no facing"
        );
    }
}
