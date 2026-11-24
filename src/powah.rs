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
use std::collections::{BTreeMap, BTreeSet, HashMap, VecDeque};
use std::sync::{Arc, Mutex, OnceLock, RwLock};

use mod_api::HostBlockStates;
use mod_api::{
    self, AnalogOutputEvent, AttachmentCodec, AttachmentError, AttachmentType, BlockDropsEvent,
    BlockEntityPart, BlockEntityTickEvent, BlockEntityTickReply, BlockEntityTicksEvent,
    BlockEntityTypeKey, BlockFace, BlockPart, BlockSide, Command, CommandArgumentValue, Component,
    ComponentId, Compound, Direction, ENERGY, EnergyHandler, Flow, Hand, HasAnalogOutputEvent,
    Holder, Host, HostSide, HostTransaction, Identifier, InteractionResult, ItemComponentHandler,
    ItemInteractMobEvent, ItemKey, ItemPart, ItemPlace, LevelDimension, LevelHost, LevelKey,
    LevelRef, ModContainer, ModEnergyBuffer, NeighbourUpdate, OnBlockPlacedByEvent, PlayerRef,
    Registrar, ServerHost, Tag, Transaction, UseItemEvent, move_energy,
};

use crate::runtime::{
    BATTERY_TIER_COUNT, BatteryTier, CABLE_TIER_COUNT, CELL_TIER_COUNT, CableTier, CellTier,
    Channel, CreativeCellTier, DISCHARGER_TIER_COUNT, DischargerTier, ENDER_TIER_COUNT, EnderTier,
    EnergizingRecipe, FURNATOR_TIER_COUNT, Furnator, FurnatorTier, HOPPER_TIER_COUNT, HopperTier,
    MAX_CHANNELS, Neighbour, ORB_MAX_ENERGY, ORB_OUTPUT_SLOT, ORB_SLOTS, ROD_TIER_COUNT,
    RedstoneMode, RodTier, SOLAR_TIER_COUNT, Sky, SolarTier, TRANSMITTER_TIER_COUNT, TransferType,
    battery_tier, cable_tier, cell_tier, check_redstone, clamp_energy, comparator_strength,
    creative_after_give, creative_cell_tier, default_sides, discharger_tier, ender_cell_tier,
    ender_gate_tier, furnator_tier, network_moves, push_amount, rod_tier,
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
/// The creative energy cell block and its item: upstream's `EnergyCellTile` at
/// `Tier.CREATIVE`, a source whose `getTransferType()` is `Transfer.EXTRACT`.
pub(crate) const CREATIVE_CELL: &str = "powah:energy_cell_creative";
/// The creative cell's block entity type path, in this mod's namespace.
pub(crate) const CREATIVE_CELL_TYPE: &str = "energy_cell_creative";
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

/// Every battery item's id, in the ladder's order (the index [`battery_tier`]
/// takes). A battery is a plain `ITEM_ENERGY` item, not a block: it has no
/// block entity type, unlike the machine arrays below.
pub(crate) const BATTERIES: [&str; BATTERY_TIER_COUNT] = [
    "powah:battery_starter",
    "powah:battery_basic",
    "powah:battery_hardened",
    "powah:battery_blazing",
    "powah:battery_niotic",
    "powah:battery_spirited",
    "powah:battery_nitro",
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

/// Every solar panel tier's block id and block entity type name, in the ladder's order (the index
/// [`solar_of`] takes).
pub(crate) const SOLARS: [(&str, &str); SOLAR_TIER_COUNT] = [
    ("powah:solar_panel_starter", "solar_panel_starter"),
    ("powah:solar_panel_basic", "solar_panel_basic"),
    ("powah:solar_panel_hardened", "solar_panel_hardened"),
    ("powah:solar_panel_blazing", "solar_panel_blazing"),
    ("powah:solar_panel_niotic", "solar_panel_niotic"),
    ("powah:solar_panel_spirited", "solar_panel_spirited"),
    ("powah:solar_panel_nitro", "solar_panel_nitro"),
];

/// Every ender cell tier's block id and block entity type name, in the ladder's order (the index
/// [`ender_cell_of`] takes).
pub(crate) const ENDER_CELLS: [(&str, &str); ENDER_TIER_COUNT] = [
    ("powah:ender_cell_starter", "ender_cell_starter"),
    ("powah:ender_cell_basic", "ender_cell_basic"),
    ("powah:ender_cell_hardened", "ender_cell_hardened"),
    ("powah:ender_cell_blazing", "ender_cell_blazing"),
    ("powah:ender_cell_niotic", "ender_cell_niotic"),
    ("powah:ender_cell_spirited", "ender_cell_spirited"),
    ("powah:ender_cell_nitro", "ender_cell_nitro"),
];

/// Every ender gate tier's block id and block entity type name, in the ladder's order (the index
/// [`ender_gate_of`] takes).
pub(crate) const ENDER_GATES: [(&str, &str); ENDER_TIER_COUNT] = [
    ("powah:ender_gate_starter", "ender_gate_starter"),
    ("powah:ender_gate_basic", "ender_gate_basic"),
    ("powah:ender_gate_hardened", "ender_gate_hardened"),
    ("powah:ender_gate_blazing", "ender_gate_blazing"),
    ("powah:ender_gate_niotic", "ender_gate_niotic"),
    ("powah:ender_gate_spirited", "ender_gate_spirited"),
    ("powah:ender_gate_nitro", "ender_gate_nitro"),
];

/// Every player transmitter tier's block id and block entity type name, in the ladder's order (the
/// index [`transmitter_tier`](crate::runtime::transmitter_tier) takes).
pub(crate) const TRANSMITTERS: [(&str, &str); TRANSMITTER_TIER_COUNT] = [
    (
        "powah:player_transmitter_starter",
        "player_transmitter_starter",
    ),
    ("powah:player_transmitter_basic", "player_transmitter_basic"),
    (
        "powah:player_transmitter_hardened",
        "player_transmitter_hardened",
    ),
    (
        "powah:player_transmitter_blazing",
        "player_transmitter_blazing",
    ),
    (
        "powah:player_transmitter_niotic",
        "player_transmitter_niotic",
    ),
    (
        "powah:player_transmitter_spirited",
        "player_transmitter_spirited",
    ),
    ("powah:player_transmitter_nitro", "player_transmitter_nitro"),
];

/// The binding card a transmitter's one slot takes (`Itms.BINDING_CARD`).
pub(crate) const BINDING_CARD: &str = "powah:binding_card";
/// The aerial pearl, and the pearl an `AerialPearlItem` makes of a zombie (`Itms.PLAYER_AERIAL_PEARL`):
/// the starter transmitter recipe's `p`.
pub(crate) const AERIAL_PEARL: &str = "powah:aerial_pearl";
pub(crate) const PLAYER_AERIAL_PEARL: &str = "powah:player_aerial_pearl";

/// The component upstream registers for a card's bound player (`PowahComponents.BOUND_PLAYER`):
/// `{gameProfileId: [I; 4], name: "..."}`, as its `BoundPlayer.CODEC` writes it.
pub(crate) const BOUND_PLAYER: &str = "powah:bound_player";

/// A player transmitter's one slot (`PlayerTransmitterTile`'s `inv.add(1)`): the binding card.
const TRANSMITTER_SLOTS: usize = 1;
/// The slot the card sits in (`inv.getFirst()`).
const TRANSMITTER_CARD_SLOT: usize = 0;
/// The slots `Player.invStacks` walks: the main inventory (0..36), the armour (36..40) and the off
/// hand (40), in `Inventory`'s own numbers — the `ENTITY_ITEMS` indices a player answers.
pub(crate) const PLAYER_SLOTS: usize = 41;
/// The off hand's `Inventory` number (40), which a `Hand::Off` bind writes.
const OFF_HAND_SLOT: usize = 40;

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
/// A solar panel's one slot (`SolarTile`'s `inv.add(1)`, slot limit 1, chargeables only).
const SOLAR_SLOTS: usize = 1;
/// An ender cell's and an ender gate's three slots (`AbstractEnderTile`'s `inv.add(3)` — the cell
/// on its own, the gate in `EnderGateTile`): slot 0 the extender, slots 1 and 2 the charge slots.
const ENDER_SLOTS: usize = 3;
/// The extender slot (`AbstractEnderTile.canInsert` slot 0: `IEnderExtender` items only).
const ENDER_EXTENDER_SLOT: usize = 0;
/// How many slots an ender block charges (`AbstractEnderTile.postTick`'s `chargeItems(1, 3)`).
const ENDER_CHARGE_SLOTS: usize = ENDER_SLOTS - ENDER_EXTENDER_SLOT - 1;

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
    // A battery is a plain item (`Itms.BATTERY`), not a machine block: its id
    // never names a `Kind`, so it is looked up before the block table.
    if let Some(buffer) = battery_item_buffer(block) {
        return Some(buffer);
    }
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
        // A solar panel is an `AbstractEnergyProvider` too: its own item gives and never takes.
        Kind::Solar(tier) => Some(ItemBuffer {
            capacity: tier.capacity,
            max_insert: 0,
            max_extract: tier.transfer,
        }),
        // An ender cell and an ender gate keep nothing in their own item (upstream's `keepEnergy`
        // is false): the FE is in the owner's channel, which the channel store keeps.
        Kind::EnderCell(_) | Kind::EnderGate(_) => None,
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
        // A transmitter keeps its buffer (`keepEnergy()` true) and is an `AbstractEnergyStorage`
        // with the default `Transfer.ALL`: its own item both gives and takes at its transfer.
        Kind::Transmitter(tier) => Some(ItemBuffer {
            capacity: tier.capacity,
            max_insert: tier.transfer,
            max_extract: tier.transfer,
        }),
        // `keepEnergy()` is false for the creative cell, so its item carries no buffer.
        Kind::CreativeCell(_) | Kind::Orb | Kind::Cable(_) => None,
    }
}

/// The item slots a machine kind holds energy items in: the furnator's one charge slot
/// (`FurnatorTile`'s `chargeItems(1)`), a cell's two (`EnergyCellTile`'s `chargeItems(2)`),
/// a discharger's seven (`EnergyDischargerTile`'s `inv.add(7)`, which its
/// `dischargeItemsInInventory` drains), everything else none.
pub(crate) fn kind_item_slots(kind: Kind) -> usize {
    match kind {
        Kind::Furnator(_) => 1,
        Kind::Cell(_) | Kind::CreativeCell(_) => CELL_SLOTS,
        Kind::Discharger(_) => DISCHARGER_SLOTS,
        // `EnergyHopperTile` has no slots of its own (`getSlotLimit` 0, `canInsert` and
        // `canExtract` false): it charges the container it faces, never one it holds.
        Kind::Hopper(_) => 0,
        // `SolarTile`'s one charge slot (`inv.add(1)`, `chargeItems(1)`).
        Kind::Solar(_) => SOLAR_SLOTS,
        // `AbstractEnderTile.postTick`'s `chargeItems(1, 3)`: an ender cell and an ender gate each
        // charge the two slots after the extender.
        Kind::EnderCell(_) | Kind::EnderGate(_) => ENDER_CHARGE_SLOTS,
        // A transmitter's one slot holds a binding card, not an energy item: the pass over it is
        // the bound player's inventory (`postTick`), so it charges no slot of its own.
        Kind::Transmitter(_) => 0,
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
        // Upstream's `EnergyCellTile.keepEnergy()` is `!isCreative()`: a creative
        // cell's block item carries nothing back on break.
        Kind::CreativeCell(_) => None,
        Kind::Hopper(tier) => Some(tier.capacity),
        Kind::Solar(tier) => Some(tier.capacity),
        Kind::Rod(tier) => Some(tier.capacity),
        // `PlayerTransmitterTile.keepEnergy()` is true: the buffer rides the block item.
        Kind::Transmitter(tier) => Some(tier.capacity),
        // Upstream's `keepEnergy()` is false for both ender blocks: the FE is in the owner's
        // channel, not in the tile, so the item carries nothing and keeps nothing.
        Kind::EnderCell(_) | Kind::EnderGate(_) | Kind::Orb | Kind::Cable(_) => None,
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

/// A solar panel's container: upstream's one charge slot (`SolarTile`'s `inv.add(1)`, slot limit
/// 1, chargeables only), every face reaching it, no menu. The slot rule is the tick's, as a
/// declaration cannot express a per-slot `canInsert` at this pin.
pub(crate) fn solar_container() -> ModContainer {
    let slots: Vec<usize> = (0..SOLAR_SLOTS).collect();
    ModContainer {
        slots: SOLAR_SLOTS,
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
/// An ender block's channel index and owner (`AbstractEnderTile`'s `channel` and `owner_id`).
const KEY_CHANNEL: &str = "channel";
const KEY_OWNER: &str = "owner";
/// The mode a machine's redstone gate runs from (`AbstractTileEntity.writeSync`'s
/// `redstone_mode`, an int ordinal) — the sync compound's key, so a broken machine's item
/// carries its buffer but never its mode, as upstream's `writeStorable` leaves it out too.
const KEY_REDSTONE_MODE: &str = "redstone_mode";
/// The six per-face transfer types (`SideConfig.write`'s `side_transfer_type`, an int
/// array of `Transfer` ordinals, in `Direction.get3DDataValue()` order) — the sync
/// compound's key, as upstream's is, so a broken machine's item carries its buffer
/// but never its side config.
const KEY_SIDE_TRANSFER: &str = "side_transfer_type";

/// The three machines, as a readout has to tell them apart.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Kind {
    /// A furnator, carrying its tier's generation, capacity and transfer.
    Furnator(FurnatorTier),
    /// An energy cell, carrying its tier's capacity and transfer.
    Cell(CellTier),
    /// The creative energy cell: a source with upstream's `Energy.MAX` clamped
    /// into the SDK's `i32`, `Transfer.EXTRACT`, and a buffer that never drops.
    CreativeCell(CreativeCellTier),
    /// An energy cable, carrying its tier's rate.
    Cable(CableTier),
    /// An energy discharger, carrying its tier's capacity and transfer.
    Discharger(DischargerTier),
    /// An energy hopper, carrying its tier's capacity, transfer and charging rate.
    Hopper(HopperTier),
    /// A solar panel, carrying its tier's generation, capacity and transfer.
    Solar(SolarTier),
    /// The energizing orb: a recipe-gated buffer over seven slots.
    Orb,
    /// An energizing rod, carrying its tier's capacity and transfer.
    Rod(RodTier),
    /// An ender cell, carrying its tier's transfer and channel count.
    EnderCell(EnderTier),
    /// An ender gate, carrying its tier's transfer and channel count.
    EnderGate(EnderTier),
    /// A player transmitter, carrying its tier's capacity, transfer and charging rate.
    Transmitter(crate::runtime::TransmitterTier),
}

impl Kind {
    /// The machine a block id names, `None` for anything else.
    pub(crate) fn of(block: &str) -> Option<Kind> {
        match block {
            ORB => Some(Kind::Orb),
            CREATIVE_CELL => Some(Kind::CreativeCell(creative_cell_tier())),
            _ => furnator_of(block)
                .map(Kind::Furnator)
                .or_else(|| cell_of(block).map(Kind::Cell))
                .or_else(|| cable_of(block).map(Kind::Cable))
                .or_else(|| rod_of(block).map(Kind::Rod))
                .or_else(|| discharger_of(block).map(Kind::Discharger))
                .or_else(|| hopper_of(block).map(Kind::Hopper))
                .or_else(|| solar_of(block).map(Kind::Solar))
                .or_else(|| ender_cell_of(block).map(Kind::EnderCell))
                .or_else(|| ender_gate_of(block).map(Kind::EnderGate))
                .or_else(|| transmitter_of(block).map(Kind::Transmitter)),
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

/// The battery tier an **item** id names, with its numbers; `None` for any other
/// item. Batteries are plain items (`Itms.BATTERY`), so their ids are not blocks.
pub(crate) fn battery_item_of(item: &str) -> Option<BatteryTier> {
    let index = BATTERIES.iter().position(|id| *id == item)?;
    battery_tier(index)
}

/// The `ITEM_ENERGY` buffer of a battery item: upstream's `EnergyItem.getEnergyInfo()`
/// gives it the tier's capacity and the transfer as both its insert and extract
/// limit (`isChargeable()` is true for every battery).
pub(crate) fn battery_item_buffer(item: &str) -> Option<ItemBuffer> {
    battery_item_of(item).map(|tier| ItemBuffer {
        capacity: tier.capacity,
        max_insert: tier.transfer,
        max_extract: tier.transfer,
    })
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

/// The solar panel a block id names, with its tier's numbers; `None` for any other block. The ids
/// are the content's own (`data/powah/horizon/block/solar_panel_*`).
pub(crate) fn solar_of(block: &str) -> Option<SolarTier> {
    let index = SOLARS.iter().position(|(id, _)| *id == block)?;
    crate::runtime::solar_tier(index)
}

/// The ender cell a block id names, with its tier's transfer and channel count; `None` for any
/// other block. The ids are the content's own (`data/powah/horizon/block/ender_cell_*`).
pub(crate) fn ender_cell_of(block: &str) -> Option<EnderTier> {
    let index = ENDER_CELLS.iter().position(|(id, _)| *id == block)?;
    ender_cell_tier(index)
}

/// The ender gate a block id names, with its tier's transfer and channel count; `None` for any
/// other block. The ids are the content's own (`data/powah/horizon/block/ender_gate_*`).
pub(crate) fn ender_gate_of(block: &str) -> Option<EnderTier> {
    let index = ENDER_GATES.iter().position(|(id, _)| *id == block)?;
    ender_gate_tier(index)
}

/// The player transmitter a block id names, with its tier's numbers; `None` for any other block.
/// The ids are the content's own (`data/powah/horizon/block/player_transmitter_*`).
pub(crate) fn transmitter_of(block: &str) -> Option<crate::runtime::TransmitterTier> {
    let index = TRANSMITTERS.iter().position(|(id, _)| *id == block)?;
    crate::runtime::transmitter_tier(index)
}

// ---------------------------------------------------------------------------
// The redstone gate: `AbstractTileEntity.checkRedstone`
// ---------------------------------------------------------------------------

/// Whether the machine `block` names has a redstone mode at all: every machine whose upstream
/// screen offers the button (`AbstractEnergyScreen.init` → `addRedstoneButton`) — and that is
/// exactly the set that calls `checkRedstone()`. The energizing orb and the rods have neither.
pub(crate) fn has_redstone_mode(block: &str) -> bool {
    matches!(
        Kind::of(block),
        Some(
            Kind::Furnator(_)
                | Kind::Cell(_)
                | Kind::CreativeCell(_)
                | Kind::Cable(_)
                | Kind::Discharger(_)
                | Kind::Hopper(_)
                | Kind::Solar(_)
                | Kind::EnderCell(_)
                | Kind::EnderGate(_)
                | Kind::Transmitter(_)
        )
    )
}

/// Whether the machine `kind`'s `/powah info` line prints a `comparator` field: the eight buffered
/// machines, whose reading is `runtime::comparator_strength`. This is the *readout* set, not the
/// full set of blocks that answer a comparator — since slice 15 the orb (stack count) and a rod
/// (its own fraction) answer through [`analog_reading`] too, and a cable still answers nothing.
pub(crate) fn has_comparator(kind: Kind) -> bool {
    matches!(
        kind,
        Kind::Furnator(_)
            | Kind::Cell(_)
            | Kind::CreativeCell(_)
            | Kind::Discharger(_)
            | Kind::Hopper(_)
            | Kind::Solar(_)
            | Kind::Transmitter(_)
            | Kind::EnderCell(_)
            | Kind::EnderGate(_)
    )
}

/// The reading a comparator gets from a machine `kind` whose buffer holds `stored` of `capacity`:
/// the same number its readout prints, upstream's `Energy.toComparatorPower()` (the fraction of the
/// buffer times 15, truncated), clamped into the buffer and to 0..=15. `None` for a block whose
/// readout prints no comparator (see [`has_comparator`]).
pub(crate) fn comparator_of(kind: Kind, stored: i32, capacity: i32) -> Option<i32> {
    if !has_comparator(kind) {
        return None;
    }
    Some(comparator_strength(
        clamp_energy(stored, capacity),
        capacity,
    ))
}

/// The analog signal a comparator gets from a Powah block, closing slice 14's deviation 1.
/// Upstream declares an output on `AbstractEnergyBlock` for the buffered machines, for a rod
/// (its own fraction) and for a cable (zero, no storage); `EnergizingOrbBlock` overrides with its
/// non-empty stack count. `None` means the block declares no signal, so a comparator beside it
/// keeps vanilla's reading instead of a faked number.
pub(crate) fn analog_reading(
    kind: Kind,
    stored: i32,
    capacity: i32,
    non_empty_stacks: usize,
) -> Option<i32> {
    match kind {
        Kind::Orb => Some(crate::runtime::stack_comparator(non_empty_stacks)),
        Kind::Rod(_) => Some(crate::runtime::comparator_strength(stored, capacity)),
        // Upstream's cable does answer through `AbstractEnergyBlock`, but its storage is 0, so the
        // number is 0. Declaring no signal keeps vanilla's own zero rather than faking a reading,
        // exactly as 0.2.14 measured with a full chest behind the cable.
        Kind::Cable(_) => None,
        _ => comparator_of(kind, stored, capacity),
    }
}

/// What `/powah redstone` finds at a position. Upstream's `NextRedstoneModePacket` acts only on an
/// `AbstractTileEntity`, so a foreign block is "no block at all"; this mod's orb and rods are the
/// only case of a machine that has no mode to set (`AbstractTileScreen` has no button).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RedstoneTarget {
    /// A machine whose gate can be set.
    Machine,
    /// One of this mod's blocks with no redstone mode (the energizing orb, a rod).
    NoMode,
    /// No Powah block stands there.
    NoBlock,
}

/// The target `/powah redstone <mode> <pos>` finds in the block `block` names.
pub(crate) fn redstone_target(block: &str) -> RedstoneTarget {
    match Kind::of(block) {
        None => RedstoneTarget::NoBlock,
        Some(_) if !has_redstone_mode(block) => RedstoneTarget::NoMode,
        Some(_) => RedstoneTarget::Machine,
    }
}

/// The mode a machine's saved compound holds (`redstone_mode`), `IGNORE` when it carries none —
/// upstream's default, which is what `Redstone.values()[nbt.getInt("redstone_mode")]` reads on a
/// fresh block entity.
pub(crate) fn saved_mode(nbt: &Compound) -> RedstoneMode {
    match nbt.get(&KEY_REDSTONE_MODE.into()) {
        Some(Tag::Int(value)) => RedstoneMode::from_ordinal(*value),
        _ => RedstoneMode::Ignore,
    }
}

/// Whether any of the six neighbours powers `pos` (`Level.getBestNeighborSignal(pos) > 0`, the
/// read upstream's `checkRedstone` makes), through the level's own signal getter.
fn powered(host: &dyn Host, level: LevelRef, pos: [i32; 3]) -> bool {
    level.best_neighbor_signal(host, pos).unwrap_or(0) > 0
}

/// Upstream's `checkRedstone()` for the machine at `pos`, whose saved compound is at hand: its mode
/// against the neighbours' power.
pub(crate) fn gate_open(host: &dyn Host, level: LevelRef, pos: [i32; 3], saved: &Compound) -> bool {
    let mode = saved_mode(saved);
    // Upstream returns before it reads the signal on the default ("avoid checking redstone if mode
    // is IGNORE: `getBestNeighborSignal` is relatively expensive"), so a machine left alone pays
    // nothing for a gate nobody set — which is every machine this port placed until now.
    mode == RedstoneMode::Ignore || check_redstone(mode, powered(host, level, pos))
}

/// [`gate_open`] for a position whose compound is not at hand (the level-tick phase runs after the
/// machine's own tick): reads it from the block entity. A position with no readable block entity
/// answers open, so a missing read never stops a machine.
pub(crate) fn gate_open_at(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3]) -> bool {
    let Some(saved) = host
        .block_entity(level, pos)
        .and_then(|entity| entity.data(&*host))
    else {
        return true;
    };
    gate_open(&*host, level, pos, &saved)
}

/// Whether the machine at `pos` still answers energy on its faces. Upstream overrides
/// `canExtractEnergy`/`canReceiveEnergy` with `checkRedstone() && super…` in exactly three places —
/// `EnergyCellTile`, and `AbstractEnderTile` (the cell and the gate) — and nowhere else: a furnator,
/// a discharger or a solar panel keeps giving while its own tick is gated, which is what their
/// `postTick` does by running `extractFromSides` after the gate. A foreign block always answers.
pub(crate) fn faces_open(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3]) -> bool {
    match block_at(&*host, level, pos).as_deref().and_then(Kind::of) {
        Some(Kind::Cell(_) | Kind::CreativeCell(_) | Kind::EnderCell(_) | Kind::EnderGate(_)) => {
            gate_open_at(host, level, pos)
        }
        _ => true,
    }
}

// ---------------------------------------------------------------------------
// The side config: `SideConfig` / `Transfer` on every face
// ---------------------------------------------------------------------------

/// Whether the machine `block` names has a `SideConfig` at all. Upstream's
/// `AbstractEnergyStorage` owns one, so every tile that extends it does: the cell,
/// the furnator (`AbstractEnergyProvider`), the solar panel (`AbstractGeneratorBlock`),
/// the discharger, the hopper, the transmitter, the rods, the ender cell and gate.
/// The energizing orb owns its own energy field (no `SideConfig`) and a cable stores
/// nothing, so neither has one.
pub(crate) fn has_side_config(block: &str) -> bool {
    matches!(
        Kind::of(block),
        Some(
            Kind::Furnator(_)
                | Kind::Cell(_)
                | Kind::CreativeCell(_)
                | Kind::Discharger(_)
                | Kind::Hopper(_)
                | Kind::Solar(_)
                | Kind::EnderCell(_)
                | Kind::EnderGate(_)
                | Kind::Transmitter(_)
                | Kind::Rod(_)
        )
    )
}

/// The transfer type upstream's `SideConfig.init` fills every face with: the tile's
/// `getTransferType()`. It is **not** the same for every machine — an energy storage
/// (cell, hopper, transmitter, rod, ender cell/gate) defaults to `ALL`, while a
/// furnator (`AbstractEnergyProvider`), a solar panel (`AbstractGeneratorBlock`) and
/// a discharger default to `EXTRACT` (give only).
pub(crate) fn default_transfer(kind: Kind) -> TransferType {
    match kind {
        // Upstream's `EnergyCellTile.getTransferType()` is
        // `isCreative() ? Transfer.EXTRACT : super`: a creative cell gives only.
        Kind::Furnator(_) | Kind::Solar(_) | Kind::Discharger(_) | Kind::CreativeCell(_) => {
            TransferType::Extract
        }
        _ => TransferType::All,
    }
}

/// The six face types held in `saved`, or the machine's default when no valid
/// `side_transfer_type` is saved yet (upstream's `init` before the first tick).
pub(crate) fn side_types(block: &str, saved: &Compound) -> [TransferType; 6] {
    let default = Kind::of(block).map_or(TransferType::None, default_transfer);
    match saved.get(&KEY_SIDE_TRANSFER.into()) {
        Some(Tag::IntArray(values)) if values.len() == 6 => {
            std::array::from_fn(|index| TransferType::from_ordinal(values[index]))
        }
        _ => default_sides(default),
    }
}

/// Whether FE may cross `face` on the machine at `pos`, from the side config in its
/// own compound: `canExtractEnergy`/`canReceiveEnergy`'s second half (the declared
/// buffer's face booleans and `isEnergyPresent` are the first). A block with no side
/// config — and a block entity that cannot be read — never blocks a move.
fn face_allows(
    host: &dyn Host,
    level: LevelRef,
    pos: [i32; 3],
    face: Direction,
    extract: bool,
) -> bool {
    let Some(block) = block_at(host, level, pos) else {
        return true;
    };
    if !has_side_config(&block) {
        return true;
    }
    let Some(saved) = host
        .block_entity(level, pos)
        .and_then(|entity| entity.data(host))
    else {
        return true;
    };
    let kind = side_types(&block, &saved)[face as usize];
    if extract {
        kind.can_extract()
    } else {
        kind.can_receive()
    }
}

/// [`face_allows`] for a giver.
pub(crate) fn can_extract_at(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
    face: Direction,
) -> bool {
    face_allows(&*host, level, pos, face, true)
}

/// [`face_allows`] for a taker.
pub(crate) fn can_receive_at(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
    face: Direction,
) -> bool {
    face_allows(&*host, level, pos, face, false)
}

/// A machine's readout field for its side config, as a new opt-in line (never added
/// to `/powah info`, whose every line the earlier legs assert byte-for-byte).
pub(crate) fn side_text(pos: [i32; 3], types: [TransferType; 6], time: i64) -> String {
    let [x, y, z] = pos;
    let faces = Direction::ALL
        .iter()
        .map(|face| format!("{}={}", facing_name(*face), types[*face as usize].name()))
        .collect::<Vec<_>>()
        .join(",");
    format!("Machine at {x},{y},{z} (side {faces}, t={time})")
}

/// The side-config readout for the machine at `pos`, or the reason there is none.
pub(crate) fn side_line(host: &mut dyn ServerHost, pos: [i32; 3]) -> String {
    let Some(level) = level_now(host) else {
        return "No Powah block there.".to_owned();
    };
    let Some(block) = block_at(&*host, level, pos) else {
        return "No Powah block there.".to_owned();
    };
    if !has_side_config(&block) {
        return "That block has no side config.".to_owned();
    }
    let default = Kind::of(&block).map_or(TransferType::None, default_transfer);
    let types = host
        .block_entity(level, pos)
        .and_then(|entity| entity.data(&*host))
        .map_or(default_sides(default), |saved| side_types(&block, &saved));
    side_text(pos, types, level.game_time(&*host).unwrap_or(0))
}

/// A machine's readout field for its redstone gate: `, redstone off (powered)` when the mode is not
/// the default, and nothing at all on `IGNORE` — the default a machine is placed with, which keeps
/// every untouched machine's line the one the earlier slices printed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RedstoneField {
    /// The mode the machine saves.
    pub mode: RedstoneMode,
    /// Whether a neighbour powers it now.
    pub powered: bool,
}

impl RedstoneField {
    /// The field as a readout spells it, with its leading `, ` (or nothing on the default).
    #[must_use]
    pub(crate) fn text(self) -> String {
        if self.mode == RedstoneMode::Ignore {
            return String::new();
        }
        let powered = if self.powered { " (powered)" } else { "" };
        format!(", redstone {}{powered}", self.mode.name())
    }
}

/// The gate's readout field for the machine at `pos`, or nothing for a kind with no mode.
pub(crate) fn redstone_field(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3]) -> String {
    if !block_at(&*host, level, pos).is_some_and(|block| has_redstone_mode(&block)) {
        return String::new();
    }
    let Some(saved) = host
        .block_entity(level, pos)
        .and_then(|entity| entity.data(&*host))
    else {
        return String::new();
    };
    let mode = saved_mode(&saved);
    if mode == RedstoneMode::Ignore {
        return String::new();
    }
    RedstoneField {
        mode,
        powered: powered(&*host, level, pos),
    }
    .text()
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

/// The creative cell's buffer: upstream's `EnergyCellTile` at `Tier.CREATIVE`
/// (`getTransferType()` = `Transfer.EXTRACT`) answers `Energy.MAX` for both rows,
/// clamped to the SDK's `i32`. It takes nothing on any face and gives without a
/// per-call limit — its `extractEnergy` is simulated, so the source never drains.
pub(crate) fn creative_cell_battery() -> ModEnergyBuffer {
    let tier = creative_cell_tier();
    ModEnergyBuffer {
        capacity: tier.capacity,
        max_insert: 0,
        max_extract: tier.transfer,
        insert_faces: [false; 6],
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
        Some(Kind::CreativeCell(_)) => Some(creative_cell_battery()),
        Some(Kind::Discharger(tier)) => Some(discharger_battery(tier)),
        Some(Kind::Hopper(tier)) => Some(hopper_battery(tier)),
        Some(Kind::Solar(tier)) => Some(solar_battery(tier)),
        Some(Kind::Orb) => Some(orb_battery()),
        Some(Kind::Rod(tier)) => Some(rod_battery(tier)),
        Some(Kind::Transmitter(tier)) => Some(transmitter_battery(tier)),
        _ => None,
    }
}

/// A solar panel's buffer (`AbstractEnergyProvider` over `GeneratorConfig`): the tier's capacity,
/// taking nothing through any face and giving its transfer on the one face upstream's
/// `SolarTile.isEnergyPresent` names — `Direction.DOWN`.
pub(crate) fn solar_battery(tier: SolarTier) -> ModEnergyBuffer {
    let mut extract_faces = [false; 6];
    extract_faces[Direction::Down as usize] = true;
    ModEnergyBuffer {
        capacity: tier.capacity,
        max_insert: 0,
        max_extract: tier.transfer,
        insert_faces: [false; 6],
        extract_faces,
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

/// An ender cell's or an ender gate's container: upstream's three slots (`AbstractEnderTile`'s
/// `inv.add(3)`), slot 0 the extender and 1-2 the charge slots, every face reaching all three, no
/// menu. Upstream's per-slot `canInsert` (an `IEnderExtender` in slot 0, chargeables after it)
/// cannot be expressed by a container declaration at this pin, so the tick decides what it takes.
pub(crate) fn ender_container() -> ModContainer {
    let slots: Vec<usize> = (0..ENDER_SLOTS).collect();
    ModContainer {
        slots: ENDER_SLOTS,
        faces: std::array::from_fn(|_| slots.clone()),
        inputs: slots.clone(),
        outputs: slots,
        menu: None,
    }
}

/// A player transmitter's buffer (`AbstractEnergyStorage` over `ChargingConfig` with the default
/// `Transfer.ALL`): the tier's capacity, giving and taking its transfer on every face.
pub(crate) fn transmitter_battery(tier: crate::runtime::TransmitterTier) -> ModEnergyBuffer {
    ModEnergyBuffer {
        capacity: tier.capacity,
        max_insert: tier.transfer,
        max_extract: tier.transfer,
        insert_faces: [true; 6],
        extract_faces: [true; 6],
    }
}

/// A player transmitter's container: upstream's one slot (`PlayerTransmitterTile`'s `inv.add(1)`,
/// slot limit 1, a card with a bound player only), every face reaching it, no menu. Upstream's
/// per-slot `canInsert` cannot be expressed by a container declaration at this pin, so a card
/// without a bound player may sit there and the tick simply charges nothing.
pub(crate) fn transmitter_container() -> ModContainer {
    let slots: Vec<usize> = (0..TRANSMITTER_SLOTS).collect();
    ModContainer {
        slots: TRANSMITTER_SLOTS,
        faces: std::array::from_fn(|_| slots.clone()),
        inputs: slots.clone(),
        outputs: slots,
        menu: None,
    }
}

// ---------------------------------------------------------------------------
// The binding card: the player a transmitter charges
// ---------------------------------------------------------------------------

/// The player a binding card is bound to (`BoundPlayer`): the profile's UUID and the name shown
/// for them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BoundPlayerValue {
    /// The player's UUID.
    pub uuid: u128,
    /// The player's display name, as it was when the card was bound.
    pub name: String,
}

/// `powah:bound_player`, as upstream's `BoundPlayer.CODEC` writes it: `gameProfileId` is the
/// UUID as four ints (`UUIDUtil.CODEC`) and `name` a string.
pub(crate) struct BoundPlayerComponent;

impl Component for BoundPlayerComponent {
    type Value = BoundPlayerValue;

    fn id(&self) -> &str {
        BOUND_PLAYER
    }

    fn decode(&self, value: &Tag) -> Option<BoundPlayerValue> {
        let Tag::Compound(data) = value else {
            return None;
        };
        let Tag::IntArray(id) = data.get(&"gameProfileId".into())? else {
            return None;
        };
        // `UUIDUtil.CODEC` is exactly four ints; a shorter or longer array is not a bound player.
        let id: [i32; 4] = id.as_slice().try_into().ok()?;
        let Tag::String(name) = data.get(&"name".into())? else {
            return None;
        };
        Some(BoundPlayerValue {
            uuid: uuid_of(id),
            name: name.to_string_lossy(),
        })
    }

    fn encode(&self, value: BoundPlayerValue) -> Tag {
        let mut data = Compound::new();
        data.insert(
            "gameProfileId".into(),
            Tag::IntArray(uuid_ints(value.uuid).to_vec()),
        );
        data.insert("name".into(), Tag::String(value.name.as_str().into()));
        Tag::Compound(data)
    }
}

/// A UUID as `UUIDUtil` writes it: most significant int first.
fn uuid_ints(uuid: u128) -> [i32; 4] {
    [
        (uuid >> 96) as i32,
        (uuid >> 64) as i32,
        (uuid >> 32) as i32,
        uuid as i32,
    ]
}

/// The UUID four ints hold, most significant first.
fn uuid_of(ints: [i32; 4]) -> u128 {
    let [a, b, c, d] = ints.map(|part| u128::from(part as u32));
    (a << 96) | (b << 64) | (c << 32) | d
}

/// The player the card `stack` is bound to, `None` for a blank card or one whose component is not
/// a bound player.
pub(crate) fn bound_player_of(stack: &mod_api::ItemStack) -> Option<BoundPlayerValue> {
    stack.get(&BoundPlayerComponent)
}

// ---------------------------------------------------------------------------
// The ender network: one shared channel per owner
// ---------------------------------------------------------------------------

/// Upstream's `EnderNetwork` key: one channel of one owner's ender network. The owner is the
/// player the block was placed by (`owner_id` in upstream's saved compound, a `UUID`), written as
/// NBT writes a UUID — four ints — so a block placed with no player (`/setblock`, a dispenser) is
/// all-zero and still has channels of its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct ChannelKey {
    /// The owner's UUID, most significant int first.
    pub owner: [i32; 4],
    /// The channel index (0 ..= 11, upstream's `RangedInt(12)`).
    pub channel: i32,
}

impl ChannelKey {
    /// The key of `owner`'s channel `channel`.
    #[must_use]
    pub(crate) fn new(owner: u128, channel: i32) -> ChannelKey {
        ChannelKey {
            owner: [
                (owner >> 96) as i32,
                (owner >> 64) as i32,
                (owner >> 32) as i32,
                owner as i32,
            ],
            channel,
        }
    }

    /// The owner's UUID as the four ints hold it.
    #[must_use]
    #[allow(
        dead_code,
        reason = "the key's UUID form is what the saved-key test reads back; the game build never needs it"
    )]
    pub(crate) fn owner_uuid(self) -> u128 {
        self.owner
            .iter()
            .fold(0u128, |uuid, word| (uuid << 32) | u128::from(*word as u32))
    }
}

/// Every channel of every owner in one level: upstream's `EnderNetwork` `SavedData`, whose map
/// holds one owner's twelve shared `Energy` objects. The pin has no `SavedData`, so this is a
/// level attachment (`Holder::Level`, the `transfer_labels` pattern) — **one store per level**,
/// which is this port's recorded deviation from upstream's single store on the overworld server:
/// a channel here does not reach another dimension.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct ChannelStore {
    channels: BTreeMap<ChannelKey, Channel>,
}

impl ChannelStore {
    /// What the key's channel holds: an absent entry is an empty channel, as upstream's
    /// `computeIfAbsent(owner, k -> empty())` starts one.
    #[must_use]
    pub(crate) fn get(&self, key: &ChannelKey) -> Channel {
        self.channels.get(key).copied().unwrap_or_default()
    }

    /// Store the key's channel, dropping an entry that holds and can hold nothing so the saved
    /// compound stays as small as the network really is (upstream keeps twelve per owner for ever;
    /// an empty channel there behaves exactly like an absent one).
    pub(crate) fn set(&mut self, key: ChannelKey, channel: Channel) {
        if channel.capacity <= 0 && channel.stored <= 0 {
            self.channels.remove(&key);
        } else {
            self.channels.insert(key, channel);
        }
    }

    /// The entries in key order, which a `BTreeMap` keeps stable, so the saved compound is too.
    pub(crate) fn entries(&self) -> impl Iterator<Item = (&ChannelKey, &Channel)> {
        self.channels.iter()
    }

    /// One entry, as the codec's decoder rebuilds it.
    pub(crate) fn insert(&mut self, key: ChannelKey, channel: Channel) {
        self.channels.insert(key, channel);
    }
}

/// The owner's ender network as the level holds it. Saved by its codec, so a channel's FE survives
/// a relaunch exactly as upstream's `powah_network` does.
static ENDER_CHANNELS: AttachmentType<ChannelStore> = AttachmentType {
    namespace: "powah",
    name: "ender_channels",
    default: ChannelStore::default,
    codec: Some(AttachmentCodec {
        encode: encode_channels,
        decode: decode_channels,
    }),
    copy_on_death: false,
    sync: false,
};

/// `EnderNetwork.save`: one record per channel, keyed by the owner and the index.
fn encode_channels(store: &ChannelStore) -> Result<Compound, AttachmentError> {
    let records: Vec<Tag> = store
        .entries()
        .map(|(key, channel)| {
            Tag::compound([
                ("owner", Tag::IntArray(key.owner.to_vec())),
                ("channel", Tag::Int(key.channel)),
                ("stored", Tag::Int(channel.stored)),
                ("capacity", Tag::Int(channel.capacity)),
            ])
        })
        .collect();
    match Tag::compound([("channels", Tag::List(records))]) {
        Tag::Compound(root) => Ok(root),
        _ => Err(AttachmentError::Encode),
    }
}

/// `EnderNetwork`'s loader: an absent list is an empty network, a malformed record is a decode
/// error rather than a silently empty channel.
fn decode_channels(root: &Compound) -> Result<ChannelStore, AttachmentError> {
    let mut store = ChannelStore::default();
    let Some(record) = root.get(&"channels".into()) else {
        return Ok(store);
    };
    let Tag::List(records) = record else {
        return Err(AttachmentError::Decode);
    };
    for record in records {
        let Tag::Compound(entry) = record else {
            return Err(AttachmentError::Decode);
        };
        let Some(Tag::IntArray(owner)) = entry.get(&"owner".into()) else {
            return Err(AttachmentError::Decode);
        };
        if owner.len() != 4 {
            return Err(AttachmentError::Decode);
        }
        let (Some(Tag::Int(channel)), Some(Tag::Int(stored)), Some(Tag::Int(capacity))) = (
            entry.get(&"channel".into()),
            entry.get(&"stored".into()),
            entry.get(&"capacity".into()),
        ) else {
            return Err(AttachmentError::Decode);
        };
        store.insert(
            ChannelKey {
                owner: [owner[0], owner[1], owner[2], owner[3]],
                channel: *channel,
            },
            Channel {
                stored: *stored,
                capacity: *capacity,
            },
        );
    }
    Ok(store)
}

/// The level's ender network, or an empty one where it has none yet.
fn read_channels(host: &mut dyn ServerHost) -> ChannelStore {
    ENDER_CHANNELS
        .get_existing(host.attachments(), Holder::Level)
        .ok()
        .flatten()
        .unwrap_or_default()
}

/// Writes the level's ender network back; false where the level cannot hold it.
fn write_channels(host: &mut dyn ServerHost, store: &ChannelStore) -> bool {
    ENDER_CHANNELS
        .set(host.attachments(), Holder::Level, store.clone())
        .is_ok()
}

/// The channel a block entity names: its saved `channel` (0 when absent, clamped to upstream's
/// twelve) and its saved `owner` (all-zero when absent).
#[must_use]
pub(crate) fn channel_key(saved: &Tag) -> ChannelKey {
    let owner = match saved.get(KEY_OWNER) {
        Some(Tag::IntArray(owner)) if owner.len() == 4 => [owner[0], owner[1], owner[2], owner[3]],
        _ => [0; 4],
    };
    ChannelKey {
        owner,
        channel: int(saved, KEY_CHANNEL).clamp(0, MAX_CHANNELS - 1),
    }
}

/// The channel key of the ender block entity at `pos`, read from its saved compound.
pub(crate) fn ender_key_at(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
) -> Option<ChannelKey> {
    let entity = host.block_entity(level, pos)?;
    Some(channel_key(&Tag::Compound(entity.data(&*host)?)))
}

/// The `ENERGY` capability of an ender cell or gate: the owner's channel itself, read and written
/// in place. Upstream's `AbstractEnderTile.getEnergy()` hands every block on a channel the same
/// `Energy` object, so a fill into one is a fill into all; this handler is that object, which is
/// why two cells on one channel read the same FE and a channel that outlives its last cell keeps it.
pub(crate) struct EnderEnergy {
    level: LevelRef,
    pos: [i32; 3],
    tier: EnderTier,
}

impl EnergyHandler for EnderEnergy {
    /// What the channel holds (`Energy.getStored`).
    fn amount(&self, host: &mut dyn ServerHost) -> u32 {
        let Some(key) = ender_key_at(host, self.level, self.pos) else {
            return 0;
        };
        u32::try_from(read_channels(host).get(&key).stored.max(0)).unwrap_or(0)
    }

    /// What the channel can hold (`Energy.getCapacity`): 0 until an extender is inserted.
    fn capacity(&self, host: &mut dyn ServerHost) -> u32 {
        let Some(key) = ender_key_at(host, self.level, self.pos) else {
            return 0;
        };
        u32::try_from(read_channels(host).get(&key).capacity.max(0)).unwrap_or(0)
    }

    /// `AbstractEnergyStorage.receiveEnergy`: at most the room, the tier's transfer and the offer.
    /// `AbstractEnderTile.canReceiveEnergy` is `checkRedstone() && super…`, so a shut gate refuses
    /// the fill — for a foreign mover too, which is why the gate is here and not only in the tick.
    fn insert(&self, host: &mut dyn ServerHost, _tx: &Transaction, amount: u32) -> u32 {
        if !gate_open_at(host, self.level, self.pos) {
            return 0;
        }
        let Some(key) = ender_key_at(host, self.level, self.pos) else {
            return 0;
        };
        let mut store = read_channels(host);
        let (after, moved) = store.get(&key).insert(
            i32::try_from(amount).unwrap_or(i32::MAX),
            self.tier.transfer,
        );
        if moved <= 0 {
            return 0;
        }
        store.set(key, after);
        if !write_channels(host, &store) {
            return 0;
        }
        u32::try_from(moved).unwrap_or(0)
    }

    /// `AbstractEnergyStorage.extractEnergy`: at most what the channel holds and the transfer.
    /// `AbstractEnderTile.canExtractEnergy` gates it the same way `canReceiveEnergy` gates a fill.
    fn extract(&self, host: &mut dyn ServerHost, _tx: &Transaction, amount: u32) -> u32 {
        if !gate_open_at(host, self.level, self.pos) {
            return 0;
        }
        let Some(key) = ender_key_at(host, self.level, self.pos) else {
            return 0;
        };
        let mut store = read_channels(host);
        let (after, moved) = store.get(&key).extract(
            i32::try_from(amount).unwrap_or(i32::MAX),
            self.tier.transfer,
        );
        if moved <= 0 {
            return 0;
        }
        store.set(key, after);
        if !write_channels(host, &store) {
            return 0;
        }
        u32::try_from(moved).unwrap_or(0)
    }
}

/// The handler an ender block answers on one side: a cell on all six faces, a gate only on the
/// face its `facing` state names (`EnderGateTile.isEnergyPresent`). A gate's other five faces
/// answer no handler at all, which is upstream's `getExternalStorage` answering null there — a
/// neighbour on those faces sees no energy capability, as `canSurvive` expects.
pub(crate) fn ender_capability(
    host: &mut dyn ServerHost,
    at: BlockSide,
    state: mod_api::BlockState,
    entity: Option<mod_api::BlockEntityRef>,
    kind: Kind,
) -> Option<Box<dyn EnergyHandler>> {
    entity?;
    let tier = match kind {
        Kind::EnderCell(tier) => tier,
        Kind::EnderGate(tier) => {
            let facing = state
                .property(&host.block_states(), "facing")
                .ok()
                .flatten()
                .and_then(facing_of);
            if let (Some(facing), Some(side)) = (facing, at.side)
                && side != facing
            {
                return None;
            }
            tier
        }
        _ => return None,
    };
    Some(Box::new(EnderEnergy {
        level: at.level,
        pos: at.pos,
        tier,
    }))
}

/// An ender cell or gate: a block entity whose buffer is the owner's channel, with the extender
/// slot, the two charge slots and the level-tick queue for the faces it pushes on.
pub(crate) struct EnderPart {
    queue: Arc<Mutex<Queue>>,
    kind: Kind,
}

impl EnderPart {
    pub(crate) fn new(queue: Arc<Mutex<Queue>>, kind: Kind) -> EnderPart {
        EnderPart { queue, kind }
    }

    /// The tier this block carries, `None` for a kind that is not an ender block.
    fn tier(&self) -> Option<EnderTier> {
        match self.kind {
            Kind::EnderCell(tier) | Kind::EnderGate(tier) => Some(tier),
            _ => None,
        }
    }
}

#[mod_api::horizon_part]
impl BlockPart for EnderPart {
    /// A comparator beside this block gets the machine's reading: the block declares it has one, and
    /// the answer is the strength the level tick's drain recorded (the hook's level is read-only and
    /// cannot reach the block entity).
    fn has_analog_output(&self, _level: &dyn LevelHost, event: &mut HasAnalogOutputEvent) {
        event.output = true;
    }
    fn analog_output(&self, _level: &dyn LevelHost, event: &mut AnalogOutputEvent) {
        event.output = cached_reading(event.dimension, [event.pos.x, event.pos.y, event.pos.z]);
    }
}

#[mod_api::horizon_part]
impl BlockEntityPart for EnderPart {
    fn block_entity_ticks(&self, _host: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    /// One server tick (`AbstractEnderTile.postTick`): queue the position for the level's move
    /// phase, absorb an extender item from slot 0 (a cell only), then charge slots 1-2 out of the
    /// channel.
    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        if let Some(level) = level_now(host) {
            self.queue
                .lock()
                .expect("the powah queue lock is held for one insert")
                .push(level, event.pos);
        }
        let Some(tier) = self.tier() else {
            *event.reply = BlockEntityTickReply::Unchanged;
            return;
        };
        let resized = event.entity.items.len() != ENDER_SLOTS;
        if resized {
            event.entity.items.resize(ENDER_SLOTS, None);
        }
        let saved = Tag::Compound(std::mem::take(&mut event.entity.nbt));
        let key = channel_key(&saved);
        let mut changed = resized;
        // `isExtender()`: only an ender cell takes an item into slot 0 (`onSlotChanged`).
        if matches!(self.kind, Kind::EnderCell(_)) {
            changed |= absorb_extender(&mut *host, &mut event.entity.items, key, tier);
        }
        changed |= charge_from_channel(&mut *host, &mut event.entity.items, key, tier);
        if let Tag::Compound(nbt) = saved {
            event.entity.nbt = nbt;
        }
        if !changed {
            *event.reply = BlockEntityTickReply::Unchanged;
        }
    }
}

/// `AbstractEnderTile.onSlotChanged` for slot 0: an extender item's capacity joins the channel and
/// the item's own charge is spent into it, then the item is consumed. Answers whether anything
/// changed.
///
/// The extenders this package carries are the seven energy cell items (`EnergyCellItem implements
/// IEnderExtender`; a battery is not part of any slice), so `getExtendedCapacity` is the cell
/// tier's capacity and `getExtendedEnergy` is the FE the item's own payload holds.
fn absorb_extender(
    host: &mut dyn ServerHost,
    items: &mut [Option<mod_api::ItemStack>],
    key: ChannelKey,
    _tier: EnderTier,
) -> bool {
    let Some(stack) = items.get(ENDER_EXTENDER_SLOT).and_then(Option::as_ref) else {
        return false;
    };
    let id = stack.item().id(&*host);
    let Some(capacity) = cell_of(&id).map(|cell| cell.capacity) else {
        return false;
    };
    let mut store = read_channels(host);
    let after = store.get(&key).extended(capacity, held_energy(stack));
    if after == store.get(&key) {
        return false;
    }
    store.set(key, after);
    if !write_channels(host, &store) {
        return false;
    }
    items[ENDER_EXTENDER_SLOT] = stack.clone().shrink(1);
    true
}

/// `AbstractEnderTile.postTick`'s `chargeItems(1, 3)`: the two slots after the extender, each
/// taking `min(the tier's transfer, the channel's remaining budget, its own room)` and the channel
/// debited by exactly what the items took.
fn charge_from_channel(
    host: &mut dyn ServerHost,
    items: &mut [Option<mod_api::ItemStack>],
    key: ChannelKey,
    tier: EnderTier,
) -> bool {
    let mut store = read_channels(host);
    let channel = store.get(&key);
    if channel.stored <= 0 {
        return false;
    }
    let first = ENDER_EXTENDER_SLOT + 1;
    let mut rooms = Vec::with_capacity(ENDER_CHARGE_SLOTS);
    for slot in first..first + ENDER_CHARGE_SLOTS {
        rooms.push(items.get(slot).and_then(Option::as_ref).map_or(0, |stack| {
            let id = stack.item().id(&*host);
            item_buffer(&id).map_or(0, |buffer| {
                // The item's own capability caps one call too (`Info.maxInsert`), and the pass's
                // per-slot limit and the room are both minima, so the cap folds into the room.
                let room = buffer.capacity.saturating_sub(held_energy(stack).max(0));
                room.min(buffer.max_insert)
            })
        }));
    }
    let mut charged = 0;
    for (offset, asked) in crate::runtime::charge_slots(channel.stored, tier.transfer, &rooms)
        .into_iter()
        .enumerate()
    {
        if asked <= 0 {
            continue;
        }
        let slot = first + offset;
        let Some(stack) = items.get(slot).and_then(Option::as_ref) else {
            continue;
        };
        let held = held_energy(stack);
        let Some(charged_stack) = with_payload_energy(&*host, stack, held.saturating_add(asked))
        else {
            continue;
        };
        items[slot] = Some(charged_stack);
        charged += asked;
    }
    if charged <= 0 {
        return false;
    }
    store.set(
        key,
        Channel {
            stored: channel.stored - charged,
            ..channel
        },
    );
    write_channels(host, &store)
}

/// The `facing` value of the block at `pos`, `None` when the block has no such state or no block
/// stands there: the face an energy hopper charges through and an ender gate reaches energy on.
pub(crate) fn facing_at(
    host: &dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
) -> Option<Direction> {
    level
        .block_state(host, pos)?
        .property(&host.block_states(), "facing")
        .ok()
        .flatten()
        .and_then(facing_of)
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
#[expect(
    clippy::too_many_arguments,
    reason = "a format builder with one parameter per field the line prints; grouping them into a \
              struct that mirrors the format would add a type without removing a decision"
)]
pub(crate) fn status_line(
    kind: Kind,
    pos: [i32; 3],
    energy: i32,
    carbon: i32,
    fuel: Option<(&str, u32)>,
    charge: &[ChargeReadout<'_>],
    redstone: &str,
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
                 carbon {carbon}, burning {}, fuel {fuel}, charge {}, {} FE/t a side{redstone}, \
                 t={time})",
                tier.capacity,
                comparator_strength(energy, tier.capacity),
                carbon > 0,
                charge_text(charge),
                tier.transfer,
            )
        }
        Kind::Cell(tier) => format!(
            "Energy Cell at {x},{y},{z}: {energy}/{} FE (comparator {}, charge {}, {} FE/t a \
             side{redstone}, t={time})",
            tier.capacity,
            comparator_strength(energy, tier.capacity),
            charge_text(charge),
            tier.transfer,
        ),
        // The creative cell keeps the cell's formatting byte-for-byte; the new
        // `creative` field sits behind it so every earlier assertion still matches.
        Kind::CreativeCell(_) => creative_cell_line(pos, energy, charge, redstone, time),
        // A discharger is the cell's mirror: the same buffer, and its seven slots are
        // the ones it drains, so the readout names them in the same `charge` field.
        Kind::Discharger(tier) => format!(
            "Energy Discharger at {x},{y},{z}: {energy}/{} FE (comparator {}, charge {}, {} FE/t \
             a side{redstone}, t={time})",
            tier.capacity,
            comparator_strength(energy, tier.capacity),
            charge_text(charge),
            tier.transfer,
        ),
        Kind::Cable(tier) => format!(
            "Energy Cable at {x},{y},{z}: no storage, moves up to {} FE/t{redstone} (t={time})",
            tier.transfer
        ),
        // The hopper's readout names the face it charges through, which only [`info_line`] can
        // read from the block state; this arm is the type's exhaustiveness.
        Kind::Hopper(tier) => hopper_line(pos, energy, tier, None, redstone, time),
        // A solar panel's readout names the sky it reads, which only [`info_line`] can ask the
        // level; this arm is the type's exhaustiveness.
        Kind::Solar(tier) => solar_line(pos, energy, tier, None, charge, redstone, time),
        // An ender block's readout names the channel and the face a gate reaches, which only
        // [`info_line`] can read; this arm is the type's exhaustiveness.
        Kind::EnderCell(_) | Kind::EnderGate(_) => ender_line(
            kind,
            pos,
            Channel::default(),
            0,
            None,
            charge,
            redstone,
            time,
        ),
        Kind::Orb => orb_line(pos, energy, None, 0, time),
        Kind::Rod(tier) => rod_line(pos, energy, tier, None, time),
        // A transmitter's readout names the card and the player it is bound to, which only
        // [`info_line`] can read from the slot; this arm is the type's exhaustiveness.
        Kind::Transmitter(tier) => {
            transmitter_line(pos, energy, tier, CardReadout::Unknown, redstone, time)
        }
    }
}

/// The creative cell's readout: the cell's exact formatting with its `FE/t a side`
/// naming the creative tier and a trailing `creative` field behind it. Upstream's
/// `Energy.MAX` is clamped into the SDK's `i32`, so both numbers are 2,147,483,647.
pub(crate) fn creative_cell_line(
    pos: [i32; 3],
    energy: i32,
    charge: &[ChargeReadout<'_>],
    redstone: &str,
    time: i64,
) -> String {
    let [x, y, z] = pos;
    let tier = creative_cell_tier();
    format!(
        "Energy Cell at {x},{y},{z}: {energy}/{} FE (comparator {}, charge {}, {} FE/t a \
         side{redstone}, creative, t={time})",
        tier.capacity,
        comparator_strength(energy, tier.capacity),
        charge_text(charge),
        tier.transfer,
    )
}

/// What a player transmitter's one slot holds, as its readout names it: the three states the tick
/// tells apart (`inv.getFirst()` empty, a card with no bound player, a card bound to someone).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CardReadout<'a> {
    /// No block entity was read (the exhaustiveness arm of [`status_line`]).
    Unknown,
    /// No card in the slot.
    Empty,
    /// A card with no bound player: the control, which charges nothing.
    Blank,
    /// A card bound to this player.
    Bound(&'a BoundPlayerValue),
}

/// A player transmitter's readout: its buffer, the player its card is bound to (`empty` for no
/// card, `none` for a card with no bound player, `unknown` when no block entity was read) and its
/// tier's charging rate and per-side transfer.
pub(crate) fn transmitter_line(
    pos: [i32; 3],
    energy: i32,
    tier: crate::runtime::TransmitterTier,
    card: CardReadout<'_>,
    redstone: &str,
    time: i64,
) -> String {
    let [x, y, z] = pos;
    let bound = match card {
        CardReadout::Bound(player) => player.name.as_str(),
        CardReadout::Blank => "none",
        CardReadout::Empty => "empty",
        CardReadout::Unknown => "unknown",
    };
    format!(
        "Player Transmitter at {x},{y},{z}: {energy}/{} FE (comparator {}, bound {bound}, \
         charging {} FE/t, {} FE/t a side{redstone}, t={time})",
        tier.capacity,
        comparator_strength(energy, tier.capacity),
        tier.charging_rate,
        tier.transfer,
    )
}

/// An energy hopper's readout: its buffer, the face it charges through, its tier's charging rate
/// and the transfer every face gives and takes. `facing` is the block state's own value — [`None`]
/// only in [`status_line`]'s exhaustiveness arm, which has no block state to read.
pub(crate) fn hopper_line(
    pos: [i32; 3],
    energy: i32,
    tier: HopperTier,
    facing: Option<Direction>,
    redstone: &str,
    time: i64,
) -> String {
    let [x, y, z] = pos;
    let facing = facing.map_or("unknown", facing_name);
    format!(
        "Energy Hopper at {x},{y},{z}: {energy}/{} FE (comparator {}, facing {facing}, charging \
         {} FE/t, {} FE/t a side{redstone}, t={time})",
        tier.capacity,
        comparator_strength(energy, tier.capacity),
        tier.charging_rate,
        tier.transfer,
    )
}

/// A solar panel's readout: its buffer, the sky it reads, its tier's generation and the transfer it
/// gives a side. `sky` is the level's answer — [`None`] only in [`status_line`]'s exhaustiveness
/// arm, which has no level to ask.
pub(crate) fn solar_line(
    pos: [i32; 3],
    energy: i32,
    tier: SolarTier,
    sky: Option<Sky>,
    charge: &[ChargeReadout<'_>],
    redstone: &str,
    time: i64,
) -> String {
    let [x, y, z] = pos;
    let (sky, day) = match sky {
        Some(sky) => (
            if sky.can_see_sky { "open" } else { "covered" },
            if sky.daylight() { "true" } else { "false" },
        ),
        None => ("unknown", "unknown"),
    };
    format!(
        "Solar Panel at {x},{y},{z}: {energy}/{} FE (comparator {}, sky {sky}, day {day}, \
         generating {} FE/t, {} FE/t a side, charge {}{redstone}, t={time})",
        tier.capacity,
        comparator_strength(energy, tier.capacity),
        tier.generation,
        tier.transfer,
        charge_text(charge),
    )
}

/// An ender cell's and an ender gate's readout: what the owner's channel holds and can hold, the
/// channel it is on (of the tier's count), the transfer it moves a call and — for a gate — the face
/// it reaches energy on. `channel` is the shared store's entry, which is what makes two blocks on
/// one channel read the same number; `facing` is [`None`] only in [`status_line`]'s exhaustiveness
/// arm, which has no block state to read.
#[expect(
    clippy::too_many_arguments,
    reason = "a format builder with one parameter per field the line prints (the cell and the gate               share it); grouping them would add a type without removing a decision"
)]
pub(crate) fn ender_line(
    kind: Kind,
    pos: [i32; 3],
    channel: Channel,
    index: i32,
    facing: Option<Direction>,
    charge: &[ChargeReadout<'_>],
    redstone: &str,
    time: i64,
) -> String {
    let [x, y, z] = pos;
    let (name, tier, gate) = match kind {
        Kind::EnderGate(tier) => ("Ender Gate", tier, true),
        Kind::EnderCell(tier) => ("Ender Cell", tier, false),
        // Only the exhaustiveness arm of [`status_line`] reaches this, and it has no ender block.
        _ => (
            "Ender Cell",
            EnderTier {
                transfer: 0,
                channels: 0,
            },
            false,
        ),
    };
    let face = match (gate, facing) {
        (true, Some(facing)) => format!(", facing {}", facing_name(facing)),
        (true, None) => ", facing unknown".to_owned(),
        (false, _) => String::new(),
    };
    format!(
        "{name} at {x},{y},{z}: {}/{} FE (comparator {}, channel {index}/{}{face}, {} FE/t a \
         side, charge {}{redstone}, t={time})",
        channel.stored,
        channel.capacity,
        comparator_strength(channel.stored, channel.capacity),
        tier.channels,
        tier.transfer,
        charge_text(charge),
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

/// The comparator reading each machine last had, per level and position: what the machines'
/// `analog_output` part answers.
///
/// The comparator hooks get only a read-only [`LevelHost`], which cannot reach a block entity, so
/// the reading cannot be read live there. Every machine's FE change runs through its own tick and
/// the level tick's move phase, whose drain records the strength here from the buffer the machine
/// keeps; a changed value tells the comparators beside it to read it again. The key is the level's
/// dimension key, so two dimensions' machines at one position never share a number.
#[derive(Default)]
pub(crate) struct Outputs {
    strengths: HashMap<(LevelKey, [i32; 3]), i32>,
}

impl Outputs {
    /// Record `strength` for `pos`; whether the stored value changed (a first value counts).
    pub(crate) fn record(&mut self, dimension: LevelKey, pos: [i32; 3], strength: i32) -> bool {
        self.strengths.insert((dimension, pos), strength) != Some(strength)
    }

    /// The strength last recorded for `pos`, `None` where no machine recorded one.
    pub(crate) fn get(&self, dimension: LevelKey, pos: [i32; 3]) -> Option<i32> {
        self.strengths.get(&(dimension, pos)).copied()
    }

    /// Forget every reading: a world opened or closed.
    pub(crate) fn clear(&mut self) {
        self.strengths.clear();
    }
}

/// The process's readings: one per open world, cleared when the world lifecycle says so.
static OUTPUTS: OnceLock<Mutex<Outputs>> = OnceLock::new();

/// The readings every machine Part answers from.
fn outputs() -> &'static Mutex<Outputs> {
    OUTPUTS.get_or_init(|| Mutex::new(Outputs::default()))
}

/// The reading a comparator beside the machine at `pos` gets: the strength the level tick's drain
/// recorded, 0 where no machine recorded one (a comparator beside anything else keeps vanilla's).
pub(crate) fn cached_reading(dimension: LevelDimension, pos: [i32; 3]) -> i32 {
    outputs()
        .lock()
        .expect("the powah output lock is held for one read")
        .get(dimension.key(), pos)
        .unwrap_or(0)
}

/// Forget every recorded reading: a world opened or closed, so no number outlives its world.
pub(crate) fn clear_outputs() {
    outputs()
        .lock()
        .expect("the powah output lock is held for one clear")
        .clear();
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

/// What a solar panel at `pos` reads of the sky, upstream's three reads: `canSeeSky` at the block
/// above it (`Misc.canBlockSeeSky(world, pos.above())`), the dimension's sky light and the level's
/// darkening.
pub(crate) fn sky_at(host: &dyn Host, level: LevelRef, pos: [i32; 3]) -> Sky {
    let above = [pos[0], pos[1] + 1, pos[2]];
    Sky {
        can_see_sky: level.can_see_sky(host, above).unwrap_or(false),
        has_skylight: level
            .dimension_type(host)
            .is_some_and(|kind| kind.has_skylight),
        darken: level.get_sky_darken(host).unwrap_or(0),
    }
}

/// The `/powah` usages, all operator-only (level 2). They are administrative: `set` writes a
/// machine's stored FE, `item` writes the charge of the item in its charge slot, and `info` reads
/// its slots, at coordinates the caller chooses, so they are operator diagnostics rather than
/// gameplay (review finding 2026-11-18). `channel` and `redstone` are the same kind of control:
/// upstream chooses them in the block's GUI, which this port does not have, so each packet's
/// effect (`SetChannelPacket`, `NextRedstoneModePacket`) is a command here.
pub(crate) fn declare_commands(commands: &mut Vec<Command>) {
    commands.push(Command::op("powah info <pos:block_pos>", 2));
    commands.push(Command::op("powah set <amount:integer> <pos:block_pos>", 2));
    commands.push(Command::op("powah item <pos:block_pos>", 2));
    commands.push(Command::op(
        "powah item <amount:integer> <pos:block_pos>",
        2,
    ));
    commands.push(Command::op(
        "powah channel <channel:integer> <pos:block_pos>",
        2,
    ));
    // The mode is a literal, one usage per mode, so the dispatcher refuses a word that names no
    // mode (as `/powah channel` has the dispatcher refuse a non-integer).
    for mode in crate::runtime::RedstoneMode::ALL {
        commands.push(Command::op(
            &format!("powah redstone {} <pos:block_pos>", mode.command_word()),
            2,
        ));
    }
    // The side config's readout and its setter (the wrench button's replacement). Face
    // and type are words, not literals, so two usages cover all six faces × four types.
    commands.push(Command::op("powah side <pos:block_pos>", 2));
    commands.push(Command::op(
        "powah side <face:word> <type:word> <pos:block_pos>",
        2,
    ));
}

/// What `/powah channel` did, so the command can answer honestly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ChannelOutcome {
    /// The block is on this channel now, and its tier offers that many.
    Set(i32, i32),
    /// No ender cell or ender gate stands there.
    NoBlock,
}

/// Set an ender block's channel (`SetChannelPacket`: `cell.getChannel().set(channel)`), clamped to
/// the tier's own count — the values upstream's GUI offers.
pub(crate) fn set_channel_index(
    host: &mut dyn ServerHost,
    pos: [i32; 3],
    channel: i32,
) -> ChannelOutcome {
    let Some(level) = level_now(host) else {
        return ChannelOutcome::NoBlock;
    };
    let Some(block) = block_at(&*host, level, pos) else {
        return ChannelOutcome::NoBlock;
    };
    let tier = match Kind::of(&block) {
        Some(Kind::EnderCell(tier) | Kind::EnderGate(tier)) => tier,
        _ => return ChannelOutcome::NoBlock,
    };
    let Some(entity) = host.block_entity(level, pos) else {
        return ChannelOutcome::NoBlock;
    };
    let set = crate::runtime::channel_clamped(channel, tier.channels);
    let mut patch = Compound::new();
    patch.insert(KEY_CHANNEL.into(), Tag::Int(set));
    match entity.merge_data(host, patch) {
        Ok(_) => ChannelOutcome::Set(set, tier.channels),
        Err(_) => ChannelOutcome::NoBlock,
    }
}

/// What `/powah redstone` did, so the command can answer honestly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RedstoneOutcome {
    /// The machine saves this mode now.
    Set(RedstoneMode),
    /// The block is this mod's, but has no redstone mode (the energizing orb, a rod).
    NoMode,
    /// No Powah machine stands there.
    NoBlock,
}

/// Set a machine's redstone mode — the effect of the packet upstream's GUI button sends
/// (`NextRedstoneModePacket`: `tile.nextRedstoneMode(); tile.sync()`). The GUI is not ported, so
/// this command is its replacement, as `/powah channel` is the ender GUI's.
pub(crate) fn set_redstone_mode(
    host: &mut dyn ServerHost,
    pos: [i32; 3],
    mode: RedstoneMode,
) -> RedstoneOutcome {
    let Some(level) = level_now(host) else {
        return RedstoneOutcome::NoBlock;
    };
    let Some(block) = block_at(&*host, level, pos) else {
        return RedstoneOutcome::NoBlock;
    };
    match redstone_target(&block) {
        RedstoneTarget::NoBlock => return RedstoneOutcome::NoBlock,
        RedstoneTarget::NoMode => return RedstoneOutcome::NoMode,
        RedstoneTarget::Machine => {}
    }
    let Some(entity) = host.block_entity(level, pos) else {
        return RedstoneOutcome::NoBlock;
    };
    let mut patch = Compound::new();
    patch.insert(KEY_REDSTONE_MODE.into(), Tag::Int(mode.ordinal()));
    match entity.merge_data(host, patch) {
        Ok(_) => RedstoneOutcome::Set(mode),
        Err(_) => RedstoneOutcome::NoBlock,
    }
}

/// What `/powah side` did, so the command can answer honestly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SideOutcome {
    /// The face holds this type now.
    Set(Direction, TransferType),
    /// The block is this mod's, but has no side config (the orb, a cable).
    NoMode,
    /// No Powah machine stands there.
    NoBlock,
}

/// Set one face's transfer type — the effect of the wrench clicking a machine's side
/// (`NextEnergyConfigPacket`: `tile.getSideConfig().nextType(side); tile.sync()`). The
/// GUI is not ported, so this command is its replacement, as `/powah redstone` is.
pub(crate) fn set_side_type(
    host: &mut dyn ServerHost,
    pos: [i32; 3],
    face: Direction,
    kind: TransferType,
) -> SideOutcome {
    let Some(level) = level_now(host) else {
        return SideOutcome::NoBlock;
    };
    let Some(block) = block_at(&*host, level, pos) else {
        return SideOutcome::NoBlock;
    };
    if !has_side_config(&block) {
        return SideOutcome::NoMode;
    }
    let Some(entity) = host.block_entity(level, pos) else {
        return SideOutcome::NoBlock;
    };
    let default = Kind::of(&block).map_or(TransferType::None, default_transfer);
    let mut types = host
        .block_entity(level, pos)
        .and_then(|entity| entity.data(&*host))
        .map_or(default_sides(default), |saved| side_types(&block, &saved));
    types[face as usize] = kind;
    let mut patch = Compound::new();
    patch.insert(
        KEY_SIDE_TRANSFER.into(),
        Tag::IntArray(types.iter().map(|kind| kind.ordinal()).collect()),
    );
    match entity.merge_data(host, patch) {
        Ok(_) => SideOutcome::Set(face, kind),
        Err(_) => SideOutcome::NoBlock,
    }
}

/// A machine's charge slots as its readout shows them: each slot's item (with the
/// buffer it carries) or an empty slot.
fn charge_readout(
    host: &mut dyn ServerHost,
    entity: mod_api::BlockEntityRef,
    slots: usize,
) -> Vec<Option<(String, i32, i32)>> {
    charge_readout_from(host, entity, 0, slots)
}

/// The `count` slots from `first` on, as a readout shows them: an ender block's charge slots start
/// after its extender slot.
fn charge_readout_from(
    host: &mut dyn ServerHost,
    entity: mod_api::BlockEntityRef,
    first: usize,
    count: usize,
) -> Vec<Option<(String, i32, i32)>> {
    (first..first + count)
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
    // The gate's field, read once from the machine's own compound and the level's signal.
    let redstone = redstone_field(host, level, pos);
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
        Kind::CreativeCell(_) => {
            let entity = host.block_entity(level, pos)?;
            let saved = Tag::Compound(entity.data(&*host)?);
            let charge = charge_readout(host, entity, slots);
            // Upstream's readout always shows the creative cell's full capacity:
            // its `extractEnergy` is simulated, so giving FE never debits it.
            (
                clamp_energy(int(&saved, KEY_ENERGY), i32::MAX),
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
                &redstone,
                time,
            ));
        }
        Kind::Solar(tier) => {
            let entity = host.block_entity(level, pos)?;
            let saved = Tag::Compound(entity.data(&*host)?);
            let charge = charge_readout(host, entity, slots);
            let charge: Vec<ChargeReadout<'_>> = charge
                .iter()
                .map(|slot| {
                    slot.as_ref()
                        .map(|(item, held, capacity)| (item.as_str(), *held, *capacity))
                })
                .collect();
            return Some(solar_line(
                pos,
                clamp_energy(int(&saved, KEY_ENERGY), tier.capacity),
                tier,
                Some(sky_at(&*host, level, pos)),
                &charge,
                &redstone,
                time,
            ));
        }
        Kind::EnderCell(_) | Kind::EnderGate(_) => {
            let entity = host.block_entity(level, pos)?;
            let saved = Tag::Compound(entity.data(&*host)?);
            let key = channel_key(&saved);
            let channel = read_channels(host).get(&key);
            // The charge slots are 1 and 2: slot 0 is the extender (`inv.add(3)`).
            let charge = charge_readout_from(host, entity, ENDER_EXTENDER_SLOT + 1, slots);
            let charge: Vec<ChargeReadout<'_>> = charge
                .iter()
                .map(|slot| {
                    slot.as_ref()
                        .map(|(item, held, capacity)| (item.as_str(), *held, *capacity))
                })
                .collect();
            let gate = matches!(kind, Kind::EnderGate(_));
            return Some(ender_line(
                kind,
                pos,
                channel,
                key.channel,
                gate.then(|| facing_at(&*host, level, pos)).flatten(),
                &charge,
                &redstone,
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
        Kind::Transmitter(tier) => {
            let entity = host.block_entity(level, pos)?;
            let saved = Tag::Compound(entity.data(&*host)?);
            // The card in the slot: no card, a card with no bound player (the control's blank
            // one), or a bound one — the three states the tick tells apart.
            let stack = entity
                .item_slot(&*host, TRANSMITTER_CARD_SLOT as u32)
                .ok()
                .flatten();
            let bound = stack.as_ref().and_then(bound_player_of);
            let card = match (&stack, &bound) {
                (None, _) => CardReadout::Empty,
                (Some(_), Some(player)) => CardReadout::Bound(player),
                (Some(_), None) => CardReadout::Blank,
            };
            return Some(transmitter_line(
                pos,
                clamp_energy(int(&saved, KEY_ENERGY), tier.capacity),
                tier,
                card,
                &redstone,
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
    Some(status_line(
        kind, pos, energy, carbon, fuel, &charge, &redstone, time,
    ))
}

/// Set a furnator's, a cell's, the orb's or a rod's stored FE (`/powah set`): the
/// harness control the in-game check uses. Writing an `energy` key into a foreign
/// block's compound would be a foreign key in someone else's data, so only this
/// mod's blocks take it.
///
/// An ender cell or gate stores nothing in its own compound: the FE is the owner's channel, so
/// this writes the channel (which is what upstream's GUI and its `setEnergy` path do), clamped to
/// the channel's capacity — 0 until an extender is inserted.
pub(crate) fn set_energy(host: &mut dyn ServerHost, pos: [i32; 3], amount: i32) -> SetOutcome {
    let Some(level) = level_now(host) else {
        return SetOutcome::NoBlock;
    };
    let Some(block) = block_at(&*host, level, pos) else {
        return SetOutcome::NoBlock;
    };
    if let Some(kind @ (Kind::EnderCell(_) | Kind::EnderGate(_))) = Kind::of(&block) {
        let Some(key) = ender_key_at(host, level, pos) else {
            return SetOutcome::NoBlock;
        };
        let mut store = read_channels(host);
        let channel = store.get(&key);
        let stored = clamp_energy(amount, channel.capacity);
        store.set(key, Channel { stored, ..channel });
        return if write_channels(host, &store) {
            // The channel is the block's buffer: a comparator reads the new fraction at once.
            if let Some(reading) = comparator_of(kind, stored, channel.capacity) {
                publish_strength(host, level, pos, reading);
            }
            SetOutcome::Set(stored)
        } else {
            SetOutcome::NoBlock
        };
    }
    let kind = Kind::of(&block);
    let capacity = match kind {
        Some(Kind::Furnator(tier)) => tier.capacity,
        Some(Kind::Cell(tier)) => tier.capacity,
        Some(Kind::CreativeCell(_)) => i32::MAX,
        Some(Kind::Discharger(tier)) => tier.capacity,
        Some(Kind::Hopper(tier)) => tier.capacity,
        Some(Kind::Solar(tier)) => tier.capacity,
        Some(Kind::Rod(tier)) => tier.capacity,
        Some(Kind::Transmitter(tier)) => tier.capacity,
        // The orb's buffer is the recipe's own energy: with no recipe it stores
        // nothing, exactly as upstream's `onSlotChanged` leaves it.
        Some(Kind::Orb) => read_orb(host, level, pos)
            .and_then(|orb| orb.recipe)
            .map_or(0, |recipe| recipe.energy),
        Some(Kind::Cable(_)) => return SetOutcome::Cable,
        // An ender block's buffer is its owner's channel, written above.
        Some(Kind::EnderCell(_) | Kind::EnderGate(_)) => return SetOutcome::NoBlock,
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
        Ok(_) => {
            // Slice 15: a rod's own fraction changes with `/powah set`, so announce it. An orb's
            // comparator counts stacks, not FE, so a set leaves its cached count untouched.
            let reading = match kind {
                Some(Kind::Rod(_)) => Some(crate::runtime::comparator_strength(energy, capacity)),
                Some(kind) => comparator_of(kind, energy, capacity),
                None => None,
            };
            if let Some(reading) = reading {
                publish_strength(host, level, pos, reading);
            }
            SetOutcome::Set(energy)
        }
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
impl BlockPart for FurnatorPart {
    /// A comparator beside this block gets the machine's reading: the block declares it has one, and
    /// the answer is the strength the level tick's drain recorded (the hook's level is read-only and
    /// cannot reach the block entity).
    fn has_analog_output(&self, _level: &dyn LevelHost, event: &mut HasAnalogOutputEvent) {
        event.output = true;
    }
    fn analog_output(&self, _level: &dyn LevelHost, event: &mut AnalogOutputEvent) {
        event.output = cached_reading(event.dimension, [event.pos.x, event.pos.y, event.pos.z]);
    }
}

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
    ///
    /// `FurnatorTile.postTick` runs the burn behind `checkRedstone()` and its
    /// `extractFromSides` (this port's move phase) outside it: a shut gate stops the fuel and the
    /// carbon, not the FE the buffer already holds.
    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        let open = level_now(host)
            .is_none_or(|level| gate_open(&*host, level, event.pos, &event.entity.nbt));
        let changed = open && burn(host, event, self.tier);
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
    /// Whether this block's readout prints a comparator: a cell and a discharger do. A cable
    /// stores nothing and answers none; a rod prints no field but answers its own buffer fraction
    /// since slice 15, declared in `lib.rs` rather than through this flag.
    comparator: bool,
}

impl IdlePart {
    pub(crate) fn new(
        queue: Arc<Mutex<Queue>>,
        rules: Option<IdleRules>,
        comparator: bool,
    ) -> IdlePart {
        IdlePart {
            queue,
            rules,
            comparator,
        }
    }
}

#[mod_api::horizon_part]
impl BlockPart for IdlePart {
    /// A cell and a discharger answer here; a cable answers none, so a comparator beside one keeps
    /// vanilla's reading. A rod's `BlockPart` is this same type with `comparator` set — slice 15's
    /// fraction — and is declared separately in `lib.rs`.
    fn has_analog_output(&self, _level: &dyn LevelHost, event: &mut HasAnalogOutputEvent) {
        if self.comparator {
            event.output = true;
        }
    }

    fn analog_output(&self, _level: &dyn LevelHost, event: &mut AnalogOutputEvent) {
        if self.comparator {
            event.output = cached_reading(event.dimension, [event.pos.x, event.pos.y, event.pos.z]);
        }
    }
}

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

/// The creative energy cell's block entity: upstream's `EnergyCellTile.onFirstTick`
/// fills it to capacity, and because `extractEnergy` is simulated its buffer
/// never drops. The port refills it on every tick (the observable result of both
/// facts) and queues it so a neighbour can pull through a cable.
pub(crate) struct CreativeCellPart {
    queue: Arc<Mutex<Queue>>,
}

impl CreativeCellPart {
    pub(crate) fn new(queue: Arc<Mutex<Queue>>) -> CreativeCellPart {
        CreativeCellPart { queue }
    }
}

#[mod_api::horizon_part]
impl BlockPart for CreativeCellPart {
    fn has_analog_output(&self, _level: &dyn LevelHost, event: &mut HasAnalogOutputEvent) {
        event.output = true;
    }

    fn analog_output(&self, _level: &dyn LevelHost, event: &mut AnalogOutputEvent) {
        event.output = cached_reading(event.dimension, [event.pos.x, event.pos.y, event.pos.z]);
    }
}

#[mod_api::horizon_part]
impl BlockEntityPart for CreativeCellPart {
    fn block_entity_ticks(&self, _host: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        if let Some(level) = level_now(host) {
            self.queue
                .lock()
                .expect("the powah queue lock is held for one insert")
                .push(level, event.pos);
        }
        let tier = creative_cell_tier();
        let mut saved = Tag::Compound(std::mem::take(&mut event.entity.nbt));
        let before = int(&saved, KEY_ENERGY);
        // Upstream charges its two slots out of the creative buffer first
        // (`postTick` charges before it pushes); a simulated extraction leaves
        // the buffer full, which `creative_after_give` records as the pure rule.
        let charged = charge_items(
            &*host,
            CELL_SLOTS,
            tier.capacity,
            tier.transfer,
            &mut event.entity.items,
        );
        let energy = creative_after_give(tier.capacity, charged);
        write_int(&mut saved, KEY_ENERGY, energy);
        if let Tag::Compound(nbt) = saved {
            event.entity.nbt = nbt;
        }
        if before == energy && charged == 0 {
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
impl BlockPart for HopperPart {
    /// A comparator beside this block gets the machine's reading: the block declares it has one, and
    /// the answer is the strength the level tick's drain recorded (the hook's level is read-only and
    /// cannot reach the block entity).
    fn has_analog_output(&self, _level: &dyn LevelHost, event: &mut HasAnalogOutputEvent) {
        event.output = true;
    }
    fn analog_output(&self, _level: &dyn LevelHost, event: &mut AnalogOutputEvent) {
        event.output = cached_reading(event.dimension, [event.pos.x, event.pos.y, event.pos.z]);
    }
}

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

/// A solar panel tier: a block entity with one charge slot, a buffer the sky fills, and the
/// level-tick queue for the one face it gives on.
pub(crate) struct SolarPart {
    queue: Arc<Mutex<Queue>>,
    tier: SolarTier,
}

impl SolarPart {
    pub(crate) fn new(queue: Arc<Mutex<Queue>>, tier: SolarTier) -> SolarPart {
        SolarPart { queue, tier }
    }
}

#[mod_api::horizon_part]
impl BlockPart for SolarPart {
    /// A comparator beside this block gets the machine's reading: the block declares it has one, and
    /// the answer is the strength the level tick's drain recorded (the hook's level is read-only and
    /// cannot reach the block entity).
    fn has_analog_output(&self, _level: &dyn LevelHost, event: &mut HasAnalogOutputEvent) {
        event.output = true;
    }
    fn analog_output(&self, _level: &dyn LevelHost, event: &mut AnalogOutputEvent) {
        event.output = cached_reading(event.dimension, [event.pos.x, event.pos.y, event.pos.z]);
    }
}

#[mod_api::horizon_part]
impl BlockEntityPart for SolarPart {
    fn block_entity_ticks(&self, _host: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    /// One server tick: queue this position for the level's move phase, then charge the slot and
    /// produce from the sky (`SolarTile.postTick`).
    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        if let Some(level) = level_now(host) {
            self.queue
                .lock()
                .expect("the powah queue lock is held for one insert")
                .push(level, event.pos);
        }
        // `SolarTile.postTick` runs its generation behind `checkRedstone()`, after the item charge
        // and `extractFromSides`: a shut gate stops the panel making FE, not the FE it holds.
        let open = level_now(host)
            .is_none_or(|level| gate_open(&*host, level, event.pos, &event.entity.nbt));
        if !solar_tick(host, event, self.tier, open) {
            *event.reply = BlockEntityTickReply::Unchanged;
        }
    }
}

/// The solar panel's tick (`SolarTile.postTick`): clamp the buffer into the tier, charge the item in
/// its slot out of the buffer, then produce `min(generation, room)` while the panel sees the sky and
/// it is daylight. Answers whether anything changed.
fn solar_tick(
    host: &mut dyn ServerHost,
    event: &mut BlockEntityTickEvent<'_>,
    tier: SolarTier,
    open: bool,
) -> bool {
    let resized = event.entity.items.len() != SOLAR_SLOTS;
    if resized {
        event.entity.items.resize(SOLAR_SLOTS, None);
    }
    let mut saved = Tag::Compound(std::mem::take(&mut event.entity.nbt));
    let before = clamp_energy(int(&saved, KEY_ENERGY), tier.capacity);
    let mut energy = before;
    let charged = charge_items(
        &*host,
        SOLAR_SLOTS,
        energy,
        tier.transfer,
        &mut event.entity.items,
    );
    energy -= charged;
    let produced = match (open, level_now(host)) {
        (true, Some(level)) => crate::runtime::solar_production(
            energy,
            tier,
            sky_at(&*host, level, event.pos),
            // Upstream's Lens of Ender stands in for the sky; the item and the panel's upgrade
            // slot are not ported (see `notes.md`), so no panel this port places has one.
            false,
        ),
        _ => 0,
    };
    energy += produced;
    let changed = energy != before;
    if changed {
        write_int(&mut saved, KEY_ENERGY, energy);
    }
    if let Tag::Compound(nbt) = saved {
        event.entity.nbt = nbt;
    }
    resized || charged > 0 || produced > 0
}

/// A player transmitter tier: a block entity with one slot (the binding card) that charges the
/// bound player's inventory out of its own buffer, at its tier's rate.
pub(crate) struct TransmitterPart {
    tier: crate::runtime::TransmitterTier,
}

impl TransmitterPart {
    pub(crate) fn new(tier: crate::runtime::TransmitterTier) -> TransmitterPart {
        TransmitterPart { tier }
    }
}

#[mod_api::horizon_part]
impl BlockPart for TransmitterPart {
    /// A comparator beside this block gets the machine's reading: the block declares it has one, and
    /// the answer is the strength the level tick's drain recorded (the hook's level is read-only and
    /// cannot reach the block entity).
    fn has_analog_output(&self, _level: &dyn LevelHost, event: &mut HasAnalogOutputEvent) {
        event.output = true;
    }
    fn analog_output(&self, _level: &dyn LevelHost, event: &mut AnalogOutputEvent) {
        event.output = cached_reading(event.dimension, [event.pos.x, event.pos.y, event.pos.z]);
    }
}

#[mod_api::horizon_part]
impl BlockEntityPart for TransmitterPart {
    fn block_entity_ticks(&self, _host: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    /// `PlayerTransmitterTile.postTick`: clamp the buffer into the tier, then — behind
    /// `checkRedstone()` — spend the tier's rate on the inventory of the player the card in slot 0
    /// names, out of that buffer. A shut gate leaves the buffer alone: upstream returns before it
    /// reads the card at all.
    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        let open = level_now(host)
            .is_none_or(|level| gate_open(&*host, level, event.pos, &event.entity.nbt));
        let mut saved = Tag::Compound(std::mem::take(&mut event.entity.nbt));
        let before = int(&saved, KEY_ENERGY);
        let mut energy = crate::runtime::clamp_energy(before, self.tier.capacity);
        let card = event
            .entity
            .items
            .get(TRANSMITTER_CARD_SLOT)
            .and_then(Option::as_ref);
        let charged = if open {
            charge_bound_player(host, self.tier, energy, card)
        } else {
            0
        };
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

/// The pass a transmitter's tick runs, in the block entity's own tick: `postTick`'s card test, the
/// bound player's lookup and `ChargeUtil.chargeItemsInPlayerInv(player, getChargingSpeed(variant),
/// energy.getStored())`. Answers the FE that left the buffer.
///
/// Upstream's `card.isMultiDim(stack) || player.level().dimensionType().equals(world.dimensionType())`
/// is the same-level test here: the dimensional card (`binding_card_dim`) is not carried, so a card
/// only charges a player who is in the level the transmitter is in.
fn charge_bound_player(
    host: &mut dyn ServerHost,
    tier: crate::runtime::TransmitterTier,
    stored: i32,
    card: Option<&mod_api::ItemStack>,
) -> i32 {
    let Some(card) = card else {
        return 0;
    };
    if card.item().id(&*host) != BINDING_CARD {
        return 0;
    }
    let Some(bound) = bound_player_of(card) else {
        return 0;
    };
    // `card.getPlayer(level, stack)`: the profile's player, online on this server.
    let Some(player) = host.player(bound.uuid) else {
        return 0;
    };
    let Some(level) = level_now(host) else {
        return 0;
    };
    if player.level(&*host) != Some(level) {
        return 0;
    }
    charge_player_inventory(host, player, tier, stored)
}

/// `ChargeUtil.chargeItemsInPlayerInv` over `Player.invStacks` (the main inventory, the armour and
/// the off hand, `PLAYER_SLOTS` slots in `Inventory`'s numbers): each item is asked for
/// `min(the tier's rate, what is left of the buffer, its own room)` and takes it through its own
/// `Capabilities.EnergyStorage.ITEM` handler in a transaction, so this mod's cell item is charged by
/// [`MachineEnergy::insert`] and a foreign battery answers for itself. Answers the FE that moved.
fn charge_player_inventory(
    host: &mut dyn ServerHost,
    player: PlayerRef,
    tier: crate::runtime::TransmitterTier,
    stored: i32,
) -> i32 {
    let uuid = player.uuid();
    let mut rooms = Vec::with_capacity(PLAYER_SLOTS);
    for inventory_slot in 0..PLAYER_SLOTS {
        // `Player.invStacks`' `Inventory` numbers, read through the item place's own numbers
        // (`ItemAccess.forPlayerSlot`): 0..36 main, 100..=103 the armour, -106 the off hand.
        let Some(place) = crate::runtime::player_slot(inventory_slot)
            .map(|slot| ItemPlace::PlayerSlot { player: uuid, slot })
        else {
            continue;
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
    for (inventory_slot, asked) in
        crate::runtime::player_charges(stored, tier.charging_rate, &rooms)
            .into_iter()
            .enumerate()
    {
        let asked = u32::try_from(asked).unwrap_or(0);
        if asked == 0 {
            continue;
        }
        let Some(place) = crate::runtime::player_slot(inventory_slot)
            .map(|slot| ItemPlace::PlayerSlot { player: uuid, slot })
        else {
            continue;
        };
        let Some(item) = place.capability(host, &mod_api::ITEM_ENERGY) else {
            continue;
        };
        let moved = host
            .transaction(|host, tx| -> Result<u32, ()> { Ok(item.insert(host, tx, asked)) })
            .unwrap_or(0);
        charged += i32::try_from(moved.min(asked)).unwrap_or(0);
    }
    charged
}

/// The binding card: `BindingCardItem.use` binds an unbound card to the player who clicked it with
/// it in hand.
pub(crate) struct CardPart;

#[mod_api::horizon_part]
impl ItemPart for CardPart {
    /// `use`: an unbound card takes the clicking player's UUID and name; a bound one is upstream's
    /// `PASS` (it does nothing, and one clicked by someone else answers `FAIL` there — this port
    /// has no chat line for it, see `notes.md`).
    fn use_item(&self, host: HostSide, event: &mut UseItemEvent<'_>) -> Option<InteractionResult> {
        let HostSide::Server(host) = host else {
            return None;
        };
        if bound_player_of(event.stack).is_some() {
            return Some(InteractionResult::Pass);
        }
        let player = host.player(event.player)?;
        let name = player.name(&*host)?;
        let slot = hand_slot(&*host, player, event.hand)?;
        let mut bound = event.stack.clone();
        bound
            .set(
                &BoundPlayerComponent,
                BoundPlayerValue {
                    uuid: event.player,
                    name,
                },
            )
            .ok()?;
        player.set_inventory_slot(host, slot, Some(bound)).ok()?;
        Some(InteractionResult::Success)
    }
}

/// The `Inventory` slot `hand` holds its stack in: the selected hotbar slot for the main hand and
/// the off hand's own number (`Player.invStacks` reaches both).
fn hand_slot(host: &dyn Host, player: PlayerRef, hand: Hand) -> Option<u32> {
    match hand {
        Hand::Main => u32::try_from(player.selected_slot(host)?).ok(),
        Hand::Off => u32::try_from(OFF_HAND_SLOT).ok(),
    }
}

/// The aerial pearl: `AerialPearlItem.interactLivingEntity` — a pearl used on a zombie, a husk or a
/// zombie villager gives the player a player aerial pearl and kills it, which is the starter
/// transmitter recipe's `p`.
pub(crate) struct PearlPart;

#[mod_api::horizon_part]
impl ItemPart for PearlPart {
    fn item_interact_mob(
        &self,
        host: HostSide,
        event: &mut ItemInteractMobEvent<'_>,
    ) -> Option<InteractionResult> {
        let HostSide::Server(host) = host else {
            return None;
        };
        // Upstream's test is class equality: a zombie, a husk or a zombie villager.
        if !matches!(
            event.kind,
            "minecraft:zombie" | "minecraft:husk" | "minecraft:zombie_villager"
        ) {
            return None;
        }
        let player = host.players().acting_player()?;
        let entity = u32::try_from(event.entity)
            .ok()
            .and_then(|id| host.entity(id))?;
        let pearl = mod_api::ItemStack::of(item_of(PLAYER_AERIAL_PEARL)?);
        player.give_stack(host, pearl).ok()?;
        // `target.remove(Entity.RemovalReason.KILLED)`: the death sound is not played (no sound
        // calls in this port, as every earlier slice records).
        let _ = entity.remove(host, mod_api::RemovalReason::Killed);
        // `if (!player.isCreative()) stack.shrink(1)`: the pearl is spent.
        if !player.is_creative(&*host).unwrap_or(false) {
            let slot = hand_slot(&*host, player, event.hand)?;
            let count = event.stack.count().saturating_sub(1);
            if count == 0 {
                player.set_inventory_slot(host, slot, None).ok()?;
            } else {
                let mut rest = event.stack.clone();
                rest.set_count(count).ok()?;
                player.set_inventory_slot(host, slot, Some(rest)).ok()?;
            }
        }
        Some(InteractionResult::Success)
    }
}

/// The orb: a block entity with the seven slots and the recipe-gated buffer, which
/// ticks to fill itself and to turn a full buffer into its result.
pub(crate) struct OrbPart {
    queue: Arc<Mutex<Queue>>,
}

impl OrbPart {
    pub(crate) fn new(queue: Arc<Mutex<Queue>>) -> OrbPart {
        OrbPart { queue }
    }
}

#[mod_api::horizon_part]
impl BlockPart for OrbPart {
    /// Slice 15: upstream's `EnergizingOrbBlock` answers a comparator with its non-empty stack
    /// count, so the orb declares an output (0.2.14 declared none and a comparator beside it
    /// silently kept vanilla's reading).
    fn has_analog_output(&self, _level: &dyn LevelHost, event: &mut HasAnalogOutputEvent) {
        event.output = true;
    }
    fn analog_output(&self, _level: &dyn LevelHost, event: &mut AnalogOutputEvent) {
        event.output = cached_reading(event.dimension, [event.pos.x, event.pos.y, event.pos.z]);
    }
}

#[mod_api::horizon_part]
impl BlockEntityPart for OrbPart {
    fn block_entity_ticks(&self, _host: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        event.ticks = true;
    }

    /// One server tick: the recipe the inputs match, the buffer's value and the
    /// result, as `EnergizingOrbTile` decides them. The position is queued so the level tick's
    /// drain can record the stack count a comparator reads (an item insert changes no block state).
    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        if let Some(level) = level_now(host) {
            self.queue
                .lock()
                .expect("the powah queue lock is held for one insert")
                .push(level, event.pos);
        }
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

/// The four `SolarBlock` frame booleans for a panel placed at `pos`: a side is `true` (framed) when
/// no panel of the same tier stands beside it, which is upstream's inverted `canAttach`.
fn set_panel_frames(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3], block: &str) {
    for (side, name) in [
        (Direction::North, "north"),
        (Direction::South, "south"),
        (Direction::West, "west"),
        (Direction::East, "east"),
    ] {
        let attached = block_at(&*host, level, offset(pos, side)).as_deref() == Some(block);
        host.set_block_property(pos, name, if attached { "false" } else { "true" });
    }
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
        // `SolarBlock.getStateForPlacement` → `createState`: each horizontal side is framed when no
        // panel of the same tier stands there (upstream's states are `!canAttach`, so `true` draws
        // the frame). The blockstate's multipart reads exactly these four.
        if let Some(block) = block_at(&*host, level, event.pos)
            && solar_of(&block).is_some()
        {
            set_panel_frames(host, level, event.pos, &block);
        }
        // `AbstractEnderTile.onPlaced`: the owner is the player who placed the block
        // (`setOwner(placer.getGameProfile())`), and an ender gate's `facing` is the face it was
        // placed against — the face it reaches energy on (`isEnergyPresent`).
        if let Some(block) = block_at(&*host, level, event.pos)
            && let Some(Kind::EnderCell(_) | Kind::EnderGate(_)) = Kind::of(&block)
        {
            if matches!(Kind::of(&block), Some(Kind::EnderGate(_)))
                && let Some(facing) = face_direction(event.face)
            {
                host.set_block_property(event.pos, "facing", facing_name(facing));
            }
            if let Some(placer) = event.placer
                && let Some(entity) = host.block_entity(level, event.pos)
            {
                let key = ChannelKey::new(placer.uuid(), 0);
                let mut patch = Compound::new();
                patch.insert(KEY_OWNER.into(), Tag::IntArray(key.owner.to_vec()));
                let _ = entity.merge_data(host, patch);
            }
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

/// `/powah item <pos>` — the charge of the energy item in a machine's charge
/// slot, with its tier's capacity and transfer and the level's own clock. An
/// opt-in readout: it is never added to `/powah info`, whose lines every earlier
/// leg asserts byte-for-byte. The battery ladder is proven through this reply.
pub(crate) fn item_charge_line(host: &mut dyn ServerHost, pos: [i32; 3]) -> String {
    let Some(level) = level_now(host) else {
        return "No Powah block there.".to_owned();
    };
    let Some(block) = block_at(&*host, level, pos) else {
        return "No Powah block there.".to_owned();
    };
    if item_slots(&block) == 0 {
        return "That machine has no item charge slot.".to_owned();
    }
    let Some(entity) = host.block_entity(level, pos) else {
        return "No Powah block there.".to_owned();
    };
    let Ok(Some(stack)) = entity.item_slot(&*host, CHARGE_SLOT as u32) else {
        return "The charge slot holds no item that carries FE.".to_owned();
    };
    let item = stack.item().id(&*host).to_owned();
    let Some(buffer) = item_buffer(&item) else {
        return "The charge slot holds no item that carries FE.".to_owned();
    };
    let held = clamp_energy(held_energy(&stack), buffer.capacity);
    let time = level.game_time(&*host).unwrap_or(0);
    format!(
        "Charge on {item}: {held}/{} FE ({} FE/t, t={time})",
        buffer.capacity, buffer.max_extract
    )
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
/// The reading the machine at `pos` holds now, `None` where the block answers no comparator (a
/// cable, or nothing at all). The orb reads its stack count and a rod its own buffer fraction
/// since slice 15; both are recorded here from the block entity, which the hook's level cannot reach.
fn strength_at(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3]) -> Option<i32> {
    let block = block_at(&*host, level, pos)?;
    let kind = Kind::of(&block)?;
    // `stored`/`capacity` feed a rod's and the eight machines' fraction; `stacks` feeds the orb's
    // different rule. A cable declares no signal, so its zeros are never read.
    let (stored, capacity, stacks) = match kind {
        Kind::Furnator(tier) => (saved_energy(host, level, pos)?, tier.capacity, 0),
        Kind::Cell(tier) | Kind::Discharger(tier) => {
            (saved_energy(host, level, pos)?, tier.capacity, 0)
        }
        Kind::CreativeCell(_) => (saved_energy(host, level, pos)?, i32::MAX, 0),
        Kind::Hopper(tier) => (saved_energy(host, level, pos)?, tier.capacity, 0),
        Kind::Solar(tier) => (saved_energy(host, level, pos)?, tier.capacity, 0),
        Kind::Transmitter(tier) => (saved_energy(host, level, pos)?, tier.capacity, 0),
        // Slice 15: a rod answers its own buffer fraction, not "no comparator".
        Kind::Rod(tier) => (saved_energy(host, level, pos)?, tier.capacity, 0),
        // The orb's rule is `getInventory().getNonEmptyStacks().size()`, not its buffer's FE.
        Kind::Orb => (0, 0, orb_stack_count(host, level, pos)?),
        // An ender block's buffer is the owner's channel, not its own compound.
        Kind::EnderCell(_) | Kind::EnderGate(_) => {
            let entity = host.block_entity(level, pos)?;
            let saved = Tag::Compound(entity.data(&*host)?);
            let key = channel_key(&saved);
            let channel = read_channels(host).get(&key);
            (channel.stored, channel.capacity, 0)
        }
        Kind::Cable(_) => (0, 0, 0),
    };
    analog_reading(kind, stored, capacity, stacks)
}

/// The orb's non-empty inventory stacks across all seven slots (0 is the result, 1..6 the
/// inputs) — upstream's `getInventory().getNonEmptyStacks().size()`. A slot the entity has not
/// grown yet reads as empty, the same way [`read_orb`] treats its input slots.
fn orb_stack_count(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3]) -> Option<usize> {
    let entity = host.block_entity(level, pos)?;
    let mut count = 0;
    for slot in 0..ORB_SLOTS as u32 {
        if let Ok(Some(_stack)) = entity.item_slot(&*host, slot) {
            count += 1;
        }
    }
    Some(count)
}

/// The FE a machine keeps in its own saved compound (the readout clamps it into the tier).
fn saved_energy(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3]) -> Option<i32> {
    let entity = host.block_entity(level, pos)?;
    let saved = Tag::Compound(entity.data(&*host)?);
    Some(int(&saved, KEY_ENERGY))
}

/// Record what every machine moved this tick answers a comparator, and tell the comparators beside
/// a machine whose reading changed to read it again — vanilla's `setChanged` →
/// `Level.updateNeighbourForOutputSignal`, which a buffer changing without its state changing does
/// not get on its own.
fn publish_outputs(host: &mut dyn ServerHost, level: LevelRef, queued: &BTreeSet<[i32; 3]>) {
    let Some(dimension) = level.dimension().id(&*host) else {
        return;
    };
    let Ok(dimension) = LevelKey::parse(&dimension) else {
        return;
    };
    let mut changed = Vec::new();
    for &pos in queued {
        let Some(strength) = strength_at(host, level, pos) else {
            continue;
        };
        let is_new = outputs()
            .lock()
            .expect("the powah output lock is held for one record")
            .record(dimension, pos, strength);
        if is_new {
            changed.push(pos);
        }
    }
    for pos in changed {
        notify_comparators(host, level, pos);
    }
}

/// Record one machine's reading (a harness control's write) and tell its comparators when it moved.
fn publish_strength(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3], strength: i32) {
    let Some(dimension) = level.dimension().id(&*host) else {
        return;
    };
    let Ok(dimension) = LevelKey::parse(&dimension) else {
        return;
    };
    let changed = outputs()
        .lock()
        .expect("the powah output lock is held for one record")
        .record(dimension, pos, strength);
    if changed {
        notify_comparators(host, level, pos);
    }
}

/// `Level.updateNeighbourForOutputSignal`: the comparators beside `pos` read its answer again.
fn notify_comparators(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3]) {
    let Some(state) = level.block_state(&*host, pos) else {
        return;
    };
    // The update's outcome is the game's business: a level that refuses it leaves the reading in
    // place for the next change.
    let _ = level.update_neighbors(&mut *host, pos, state, NeighbourUpdate::OutputSignal);
}

pub(crate) fn on_level_tick_post(
    host: &mut dyn ServerHost,
    level: LevelRef,
    queued: BTreeSet<[i32; 3]>,
) {
    // The cables already covered by a network this tick: every cable in a network
    // ticks, so without this a run of N cables would plan the same relay N times.
    let mut planned: BTreeSet<[i32; 3]> = BTreeSet::new();
    for &pos in &queued {
        let Some(kind) = block_at(&*host, level, pos).as_deref().and_then(Kind::of) else {
            continue;
        };
        match kind {
            Kind::Furnator(tier) => push_out(host, level, pos, tier.transfer),
            // `EnergyCellTile.canExtractEnergy` is `checkRedstone() && super…`: a cell whose gate is
            // shut gives nothing on any face (`faces_open` also keeps a cable from pulling at it).
            Kind::Cell(tier) => {
                if faces_open(host, level, pos) {
                    push_out(host, level, pos, tier.transfer);
                }
            }
            // The creative cell is a source: it pushes on every face like a cell,
            // but its `extractEnergy` is simulated, so its own buffer never drops.
            Kind::CreativeCell(_) => {
                if faces_open(host, level, pos) {
                    push_out_creative(host, level, pos);
                }
            }
            // A discharger fills from its own slots first, then pushes: upstream's
            // `postTick` discharges and then runs `extractFromSides` — the drain is behind
            // `checkRedstone()`, the push is not.
            Kind::Discharger(tier) => {
                if gate_open_at(host, level, pos) {
                    drain_items(host, level, pos, tier);
                }
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
                if gate_open_at(host, level, pos) {
                    charge_container(host, level, pos, tier);
                }
            }
            // A solar panel gives only on the face upstream's `isEnergyPresent` names, its own
            // bottom (`Direction.DOWN`).
            Kind::Solar(tier) => {
                push_out_on(host, level, pos, tier.transfer, &[Direction::Down], false);
            }
            Kind::Orb => {}
            // A transmitter never pushes on its own (`postTick` has no `extractFromSides`): its
            // buffer leaves through the faces a neighbour pulls on, and into the player its card
            // names inside its own tick.
            Kind::Transmitter(_) => {}
            // An ender cell gives on every face; an ender gate only on its `facing` one, exactly
            // the face its capability answers on (`EnderGateTile.isEnergyPresent`).
            Kind::EnderCell(tier) => push_out(host, level, pos, tier.transfer),
            Kind::EnderGate(tier) => {
                if let Some(facing) = facing_at(&*host, level, pos) {
                    push_out_on(host, level, pos, tier.transfer, &[facing], false);
                }
            }
            // `CableTile.receiveEnergy` is the one place a cable reads its gate: the door FE comes
            // in through. Gated here, that cable relays nothing; an open cable of the same network
            // still does, because a gated one does not claim the network below.
            Kind::Cable(tier) => {
                if planned.contains(&pos) || !gate_open_at(host, level, pos) {
                    continue;
                }
                planned.extend(relay_network(host, level, pos, tier));
            }
        }
    }
    // After every move: what each machine answers a comparator now. The comparators beside a
    // machine whose reading changed are told to read it again.
    publish_outputs(host, level, &queued);
}

/// Upstream's `extractFromSides`: at most the transfer and what is stored, into
/// every side whose neighbour can take it. One transaction per side, so a refused
/// insert creates no energy.
fn push_out(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3], transfer: i32) {
    push_out_on(host, level, pos, transfer, &Direction::ALL, false);
}

/// The creative cell's push: the same six faces at the creative tier's transfer,
/// except that a give credits the taker without debiting the source — upstream's
/// `EnergyCellTile.extractEnergy(maxExtract, simulate || isCreative(), side)`.
fn push_out_creative(host: &mut dyn ServerHost, level: LevelRef, pos: [i32; 3]) {
    push_out_on(
        host,
        level,
        pos,
        creative_cell_tier().transfer,
        &Direction::ALL,
        true,
    );
}

/// [`push_out`] on the faces a machine gives on: a solar panel's `isEnergyPresent` is its own
/// bottom (`Direction.DOWN`) and nothing else, so the other five are never tried. When
/// `infinite` is set (the creative cell), the taker is inserted into directly and the source's
/// buffer is left alone.
fn push_out_on(
    host: &mut dyn ServerHost,
    level: LevelRef,
    pos: [i32; 3],
    transfer: i32,
    faces: &[Direction],
    infinite: bool,
) {
    // The buffer is read through a face the machine gives on: a solar panel extracts only on its
    // bottom, so reading through `Up` would answer no handler at all.
    let Some(&own_face) = faces.first() else {
        return;
    };
    let Some(own) = level.capability(&mut *host, &ENERGY, pos, Some(own_face)) else {
        return;
    };
    let stored = i32::try_from(own.amount(host)).unwrap_or(i32::MAX);
    let amount = push_amount(stored, transfer);
    if amount == 0 {
        return;
    }
    for &side in faces {
        // Upstream's `canExtractEnergy(side)` is
        // `isEnergyPresent(side) && sideConfig.getType(side).canExtract`: the six saved
        // face types gate this machine's own push as well as a neighbour's fill.
        if !can_extract_at(host, level, pos, side) {
            continue;
        }
        let neighbour = offset(pos, side);
        // Upstream's `extractFromSides` skips a side whose neighbour refuses a fill; this
        // mod's own blocks say so in their declaration (`furnator_battery` has no insert
        // face), and a foreign block is tried, its handler refusing.
        // Upstream's `canReceiveEnergy` on a cell or an ender block is `checkRedstone() && super…`:
        // a machine whose gate is shut takes nothing through its faces.
        if !faces_open(host, level, neighbour) {
            continue;
        }
        // The neighbour's own `canReceiveEnergy(side.opposite())` includes its side
        // config: a face set to `none` or `extract` refuses the fill even though the
        // declared block buffer still advertises that face.
        if !can_receive_at(host, level, neighbour, side.opposite()) {
            continue;
        }
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
            let Some(target) =
                level.capability(&mut *host, &ENERGY, neighbour, Some(side.opposite()))
            else {
                return Ok(0);
            };
            if infinite {
                // The creative source is simulated: credit the taker, keep our buffer.
                return Ok(target.insert(host, tx, want));
            }
            let Some(own) = level.capability(&mut *host, &ENERGY, pos, Some(side)) else {
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
    let Some(facing) = facing_at(&*host, level, pos) else {
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
#[derive(Clone, Copy, Debug)]
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
        // A creative cell is a source: upstream's `extractEnergy` is simulated,
        // so the cable credits the sink without taking anything out of it.
        let creative = matches!(
            block_at(&*host, level, from.pos)
                .as_deref()
                .and_then(Kind::of),
            Some(Kind::CreativeCell(_))
        );
        let _ = host.transaction(|host, tx| -> Result<u32, ()> {
            let Some(target) =
                level.capability(&mut *host, &ENERGY, to.pos, Some(to.cable_side.opposite()))
            else {
                return Ok(0);
            };
            if creative {
                return Ok(target.insert(host, tx, want));
            }
            let Some(source) = level.capability(
                &mut *host,
                &ENERGY,
                from.pos,
                Some(from.cable_side.opposite()),
            ) else {
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
    // A cell or an ender block whose gate is shut is not a port at all: `canExtractEnergy` and
    // `canReceiveEnergy` answer false there, so the network neither reads nor writes it.
    if !faces_open(host, level, pos) {
        return None;
    }
    let block = block_at(&*host, level, pos)?;
    let handler = level.capability(&mut *host, &ENERGY, pos, Some(side))?;
    let energy = i32::try_from(handler.amount(host)).unwrap_or(i32::MAX);
    let capacity = i32::try_from(handler.capacity(host)).unwrap_or(i32::MAX);
    let buffer = battery_of(&block);
    let face = side as usize;
    // A face's saved transfer type narrows the declared buffer's face booleans:
    // `canReceiveEnergy`/`canExtractEnergy` are `isEnergyPresent && type.can…`.
    let types = host
        .block_entity(level, pos)
        .and_then(|entity| entity.data(&*host))
        .map(|saved| side_types(&block, &saved));
    let receive = types.is_none_or(|types| types[face].can_receive());
    let extract = types.is_none_or(|types| types[face].can_extract());
    Some(Neighbour {
        energy,
        capacity,
        max_insert: buffer.as_ref().map_or(i32::MAX, |b| b.max_insert),
        max_extract: buffer.as_ref().map_or(i32::MAX, |b| b.max_extract),
        can_insert: buffer.as_ref().is_none_or(|b| b.insert_faces[face]) && receive,
        can_extract: buffer.as_ref().is_none_or(|b| b.extract_faces[face]) && extract,
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

/// `/powah`'s word argument (a face or a transfer type).
pub(crate) fn word(argument: Option<&CommandArgumentValue>) -> Option<String> {
    match argument? {
        CommandArgumentValue::String(value) => Some(value.clone()),
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
            10,
            "every usage is declared: `info`, `set`, the item charge readout and control, the \
             channel control, the three redstone modes and the side-config readout and setter"
        );
        let usages: Vec<&str> = commands
            .iter()
            .map(|command| command.usage.as_str())
            .collect();
        assert!(
            usages.contains(&"powah item <pos:block_pos>"),
            "the battery readout is opt-in, never a field on `info`: {usages:?}"
        );
        assert!(
            usages.contains(&"powah item <amount:integer> <pos:block_pos>"),
            "the item charge control is a usage of its own: {usages:?}"
        );
        assert!(
            usages.contains(&"powah channel <channel:integer> <pos:block_pos>"),
            "the channel control is the packet's replacement: {usages:?}"
        );
        for mode in crate::runtime::RedstoneMode::ALL {
            let usage = format!("powah redstone {} <pos:block_pos>", mode.command_word());
            assert!(
                usages.contains(&usage.as_str()),
                "the {mode:?} gate control is the GUI button's replacement: {usages:?}"
            );
        }
        assert!(
            usages.contains(&"powah side <pos:block_pos>"),
            "the six-face readout is opt-in, never a field on `info`: {usages:?}"
        );
        assert!(
            usages.contains(&"powah side <face:word> <type:word> <pos:block_pos>"),
            "the wrench button's replacement sets one face: {usages:?}"
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
    fn every_machine_has_upstreams_own_default_transfer_type() {
        // `AbstractEnergyBlock`/`AbstractEnergyStorage` default to ALL; a furnator
        // (`AbstractEnergyProvider`), a solar panel (`AbstractGeneratorBlock`) and a
        // discharger default to EXTRACT (give only). The orb and the cable have none.
        for block in [
            CELL,
            "powah:energy_cell_basic",
            "powah:energy_hopper_starter",
            "powah:player_transmitter_starter",
            "powah:energizing_rod_starter",
            "powah:ender_cell_starter",
            "powah:ender_gate_starter",
        ] {
            assert!(has_side_config(block), "{block} has a SideConfig");
            assert_eq!(
                default_transfer(Kind::of(block).unwrap_or_else(|| panic!("{block} is ours"))),
                TransferType::All,
                "{block} defaults to give and take on every face"
            );
        }
        for block in [
            FURNATOR,
            "powah:furnator_basic",
            "powah:solar_panel_starter",
            "powah:energy_discharger_starter",
        ] {
            assert!(has_side_config(block), "{block} has a SideConfig");
            assert_eq!(
                default_transfer(Kind::of(block).unwrap_or_else(|| panic!("{block} is ours"))),
                TransferType::Extract,
                "{block} defaults to give only"
            );
        }
        assert!(
            !has_side_config(ORB),
            "the orb owns its energy field, no SideConfig"
        );
        assert!(
            !has_side_config("powah:energy_cable_starter"),
            "a cable stores nothing"
        );
        assert!(
            !has_side_config("minecraft:stone"),
            "a foreign block is not ours"
        );
    }

    #[test]
    fn the_side_config_reads_and_writes_upstreams_saved_int_array() {
        let fresh = Compound::new();
        assert_eq!(side_types(CELL, &fresh), [TransferType::All; 6]);
        assert_eq!(side_types(FURNATOR, &fresh), [TransferType::Extract; 6]);
        let mut saved = Compound::new();
        saved.insert(
            KEY_SIDE_TRANSFER.into(),
            Tag::IntArray(vec![0, 0, 3, 0, 1, 2]),
        );
        assert_eq!(
            side_types(CELL, &saved),
            [
                TransferType::All,
                TransferType::All,
                TransferType::None,
                TransferType::All,
                TransferType::Extract,
                TransferType::Receive,
            ],
            "the int array is indexed by Direction's 3D data value"
        );
        // A short or malformed array is ignored rather than aborting a machine's tick.
        let mut short = Compound::new();
        short.insert(KEY_SIDE_TRANSFER.into(), Tag::IntArray(vec![0, 1]));
        assert_eq!(side_types(CELL, &short), [TransferType::All; 6]);
    }

    #[test]
    fn the_side_readout_names_all_six_faces_in_direction_order() {
        assert_eq!(
            side_text(
                [-4, 100, 60],
                [
                    TransferType::All,
                    TransferType::All,
                    TransferType::None,
                    TransferType::All,
                    TransferType::Extract,
                    TransferType::Receive,
                ],
                732
            ),
            "Machine at -4,100,60 (side down=all,up=all,north=none,south=all,west=extract,east=receive, t=732)"
        );
    }

    #[test]
    fn every_ender_block_is_one_of_the_two_ladders() {
        let cells = [
            ("powah:ender_cell_starter", 1_000, 1),
            ("powah:ender_cell_basic", 4_000, 2),
            ("powah:ender_cell_hardened", 10_000, 3),
            ("powah:ender_cell_blazing", 40_000, 5),
            ("powah:ender_cell_niotic", 100_000, 7),
            ("powah:ender_cell_spirited", 400_000, 9),
            ("powah:ender_cell_nitro", 2_000_000, 12),
        ];
        for (index, (id, transfer, channels)) in cells.into_iter().enumerate() {
            let tier = ender_cell_of(id).unwrap_or_else(|| panic!("{id} is an ender cell"));
            assert_eq!(
                (tier.transfer, tier.channels),
                (transfer, channels),
                "{id} is the cell ladder's tier {index}"
            );
            assert_eq!(Kind::of(id), Some(Kind::EnderCell(tier)));
        }
        for (id, transfer) in [
            ("powah:ender_gate_starter", 500),
            ("powah:ender_gate_basic", 2_000),
            ("powah:ender_gate_hardened", 5_000),
            ("powah:ender_gate_blazing", 20_000),
            ("powah:ender_gate_niotic", 50_000),
            ("powah:ender_gate_spirited", 200_000),
            ("powah:ender_gate_nitro", 1_000_000),
        ] {
            let tier = ender_gate_of(id).unwrap_or_else(|| panic!("{id} is an ender gate"));
            assert_eq!(tier.transfer, transfer, "{id} moves at the cable's row");
            assert_eq!(Kind::of(id), Some(Kind::EnderGate(tier)));
        }
        assert_eq!(ender_cell_of("powah:ender_gate_starter"), None);
        assert_eq!(ender_gate_of(CELL), None);
    }

    #[test]
    fn the_ender_container_is_the_extender_slot_and_two_charge_slots() {
        let container = ender_container();
        assert_eq!(container.slots, 3, "`AbstractEnderTile`'s `inv.add(3)`");
        assert_eq!(
            item_slots("powah:ender_cell_starter"),
            2,
            "`chargeItems(1, 3)` charges the two slots after the extender"
        );
        assert_eq!(
            item_slots("powah:ender_gate_nitro"),
            2,
            "a gate charges the same two slots"
        );
        assert_eq!(
            keep_capacity("powah:ender_cell_starter"),
            None,
            "`keepEnergy()` is false: the FE is the owner's channel, and the item carries nothing"
        );
        assert_eq!(keep_capacity("powah:ender_gate_nitro"), None);
        assert_eq!(
            item_buffer("powah:ender_cell_starter"),
            None,
            "an ender item is no energy item of its own"
        );
        assert_eq!(
            battery_of("powah:ender_cell_starter"),
            None,
            "the channel answers the capability; the block declares no buffer of its own"
        );
    }

    #[test]
    fn the_ender_readouts_name_the_channel_the_tier_and_a_gates_face() {
        let tier = ender_cell_tier(0).expect("the starter cell");
        let gate = ender_gate_tier(0).expect("the starter gate");
        let channel = Channel {
            stored: 100_000,
            capacity: 4_000_000,
        };
        let cell = ender_line(
            Kind::EnderCell(tier),
            [1, 2, 3],
            channel,
            0,
            None,
            &[None, None],
            "",
            42,
        );
        assert_eq!(
            cell,
            "Ender Cell at 1,2,3: 100000/4000000 FE (comparator 0, channel 0/1, 1000 FE/t a side, \
             charge empty, empty, t=42)"
        );
        let gate_line = ender_line(
            Kind::EnderGate(gate),
            [1, 2, 3],
            channel,
            1,
            Some(Direction::North),
            &[None, None],
            "",
            42,
        );
        assert_eq!(
            gate_line,
            "Ender Gate at 1,2,3: 100000/4000000 FE (comparator 0, channel 1/1, facing north, 500 \
             FE/t a side, charge empty, empty, t=42)"
        );
    }

    #[test]
    fn a_channel_store_survives_its_codec() {
        let mut store = ChannelStore::default();
        let cell = ChannelKey::new(0x1234_5678_9abc_def0_1122_3344_5566_7788, 3);
        let gate = ChannelKey::new(0x1234_5678_9abc_def0_1122_3344_5566_7788, 11);
        store.set(
            cell,
            Channel {
                stored: 12_345,
                capacity: 4_000_000,
            },
        );
        store.set(gate, Channel::default());
        assert_eq!(
            store.get(&gate),
            Channel::default(),
            "a channel that holds nothing is an empty channel"
        );
        let encoded = encode_channels(&store).expect("the store encodes");
        let decoded = decode_channels(&encoded).expect("the store decodes");
        assert_eq!(
            decoded.get(&cell),
            Channel {
                stored: 12_345,
                capacity: 4_000_000
            },
            "a channel's FE survives the save, which is what a relaunch reads"
        );
        assert_eq!(
            decoded.get(&gate),
            Channel::default(),
            "the emptied entry is not in the save at all"
        );
    }

    #[test]
    fn an_ender_blocks_saved_channel_and_owner_are_its_key() {
        // A fresh block entity has neither key: channel 0 of the all-zero owner.
        assert_eq!(
            channel_key(&Tag::Compound(Compound::new())),
            ChannelKey {
                owner: [0; 4],
                channel: 0
            }
        );
        let uuid = 0xdead_beef_0000_1111_2222_3333_4444_5555;
        let mut saved = Compound::new();
        saved.insert(KEY_CHANNEL.into(), Tag::Int(5));
        saved.insert(
            KEY_OWNER.into(),
            Tag::IntArray(ChannelKey::new(uuid, 5).owner.to_vec()),
        );
        let key = channel_key(&Tag::Compound(saved));
        assert_eq!(key.channel, 5);
        assert_eq!(key.owner_uuid(), uuid, "the four ints are the owner's UUID");
        // A doctored index is clamped to upstream's twelve channels.
        let mut doctored = Compound::new();
        doctored.insert(KEY_CHANNEL.into(), Tag::Int(99));
        assert_eq!(channel_key(&Tag::Compound(doctored)).channel, 11);
    }

    #[test]
    fn every_battery_item_is_one_of_the_ladder_with_its_own_buffer() {
        // `BATTERIES` are seven plain items, one per config row; each is a
        // chargeable ITEM_ENERGY buffer with insert/extract = the tier's transfer.
        let expected = [
            ("powah:battery_starter", 1_000_000, 1_000),
            ("powah:battery_basic", 4_000_000, 4_000),
            ("powah:battery_hardened", 10_000_000, 10_000),
            ("powah:battery_blazing", 40_000_000, 40_000),
            ("powah:battery_niotic", 100_000_000, 100_000),
            ("powah:battery_spirited", 400_000_000, 400_000),
            ("powah:battery_nitro", 2_000_000_000, 2_000_000),
        ];
        for (index, (id, capacity, transfer)) in expected.into_iter().enumerate() {
            assert_eq!(BATTERIES[index], id, "the ladder's item id {index}");
            let tier = battery_item_of(id).unwrap_or_else(|| panic!("{id} is a battery"));
            assert_eq!(
                (tier.capacity, tier.transfer),
                (capacity, transfer),
                "{id}'s numbers"
            );
            assert_eq!(
                item_buffer(id),
                Some(ItemBuffer {
                    capacity,
                    max_insert: transfer,
                    max_extract: transfer,
                }),
                "a battery is chargeable both ways at its transfer"
            );
        }
        assert_eq!(
            battery_item_of("minecraft:stone"),
            None,
            "a stone is not a battery"
        );
        assert_eq!(
            battery_item_of(CELL),
            None,
            "an energy cell is not a battery item"
        );
        assert_eq!(
            battery_item_of(BINDING_CARD),
            None,
            "a card is not a battery"
        );
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
        // Slice 17: the creative cell is a source with its own extract-only
        // buffer, not a tier on the ladder. It used to be `None` here.
        assert_eq!(
            battery_of(CREATIVE_CELL),
            Some(creative_cell_battery()),
            "the creative cell declares an extract-only source buffer"
        );
    }

    #[test]
    fn the_creative_cell_is_an_extract_only_source() {
        let tier = creative_cell_tier();
        assert_eq!(Kind::of(CREATIVE_CELL), Some(Kind::CreativeCell(tier)));
        let buffer = battery_of(CREATIVE_CELL).expect("the creative source's buffer");
        assert_eq!(buffer.capacity, 2_147_483_647, "upstream Energy.MAX as i32");
        assert_eq!(buffer.max_insert, 0, "Transfer.EXTRACT takes nothing");
        assert_eq!(buffer.max_extract, 2_147_483_647);
        assert_eq!(buffer.insert_faces, [false; 6]);
        assert_eq!(buffer.extract_faces, [true; 6]);
        assert_eq!(
            default_transfer(Kind::CreativeCell(tier)),
            TransferType::Extract,
            "EnergyCellTile.isCreative() returns Transfer.EXTRACT"
        );
        assert!(has_side_config(CREATIVE_CELL));
        assert!(has_redstone_mode(CREATIVE_CELL));
        assert_eq!(kind_item_slots(Kind::CreativeCell(tier)), CELL_SLOTS);
        assert_eq!(
            keep_capacity(CREATIVE_CELL),
            None,
            "keepEnergy() is !isCreative"
        );
        assert_eq!(item_buffer(CREATIVE_CELL), None);
        let line = creative_cell_line([1, 2, 3], tier.capacity, &[None, None], "", 42);
        assert_eq!(
            line,
            "Energy Cell at 1,2,3: 2147483647/2147483647 FE (comparator 15, charge empty, empty, 2147483647 FE/t a side, creative, t=42)"
        );
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
            status_line(starter_furnator, [0, 100, 0], 0, 0, None, &[None], "", 150),
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
                "",
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
                "",
                500
            ),
            "Furnator at 0,100,8: 40000000/40000000 FE (comparator 15, carbon 8000, burning \
             true, fuel empty, charge empty, 160000 FE/t a side, t=500)"
        );
        let starter = Kind::Cell(cell_tier(0).expect("the starter tier"));
        assert_eq!(
            status_line(starter, [2, 100, 0], 1_360, 0, None, &[None, None], "", 290),
            "Energy Cell at 2,100,0: 1360/1000000 FE (comparator 0, charge empty, empty, 1000 FE/t a \
             side, t=290)"
        );
        assert_eq!(
            status_line(
                starter,
                [2, 100, 0],
                1_000_000,
                0,
                None,
                &[None, None],
                "",
                380
            ),
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
                "",
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
                "",
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
                "",
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
    fn a_player_transmitter_declares_one_card_slot_a_buffer_and_its_kept_energy() {
        // `PlayerTransmitterTile`: `inv.add(1)` (slot limit 1, `canInsert` a card with a bound
        // player), `keepEnergy()` true, and a `ChargingConfig` buffer — so the transmitter is the
        // hopper's storage row with the slot a card sits in instead of a container it faces.
        for (index, (block, _)) in TRANSMITTERS.into_iter().enumerate() {
            let tier = crate::runtime::transmitter_tier(index)
                .unwrap_or_else(|| panic!("tier {index} is on the ladder"));
            assert_eq!(
                Kind::of(block),
                Some(Kind::Transmitter(tier)),
                "{block} is the transmitter of its tier"
            );
            assert_eq!(
                item_slots(block),
                0,
                "{block} charges no slot of its own: its slot holds the card"
            );
            assert_eq!(
                keep_capacity(block),
                Some(tier.capacity),
                "{block} keeps its buffer on its own item"
            );
            assert_eq!(
                item_buffer(block),
                Some(ItemBuffer {
                    capacity: tier.capacity,
                    max_insert: tier.transfer,
                    max_extract: tier.transfer,
                }),
                "{block}'s item takes and gives at its tier's transfer"
            );
            assert_eq!(
                battery_of(block),
                Some(ModEnergyBuffer {
                    capacity: tier.capacity,
                    max_insert: tier.transfer,
                    max_extract: tier.transfer,
                    insert_faces: [true; 6],
                    extract_faces: [true; 6],
                }),
                "{block}'s buffer answers every face both ways"
            );
        }
        assert_eq!(
            kind_item_slots(Kind::Transmitter(
                crate::runtime::transmitter_tier(0).expect("the starter tier")
            )),
            0,
            "a transmitter's pass is the bound player's inventory, not its own slot"
        );
        let container = transmitter_container();
        assert_eq!(container.slots, 1, "one slot: the binding card");
        assert_eq!(container.inputs, vec![0]);
        assert_eq!(container.outputs, vec![0]);
        assert!(container.faces.iter().all(|face| face == &vec![0]));
        assert_eq!(
            transmitter_of("powah:player_transmitter_nitro").map(|tier| tier.capacity),
            Some(2_000_000_000),
            "the nitro transmitter holds its own 2,000,000,000 FE"
        );
        assert_eq!(transmitter_of("powah:player_transmitter_typox"), None);
    }

    #[test]
    fn a_player_transmitter_reads_its_buffer_and_the_player_its_card_binds() {
        let tier = crate::runtime::transmitter_tier(0).expect("the starter tier");
        let bound = BoundPlayerValue {
            uuid: 0x0123_4567_89ab_cdef_0123_4567_89ab_cdef,
            name: "AdLf0208".to_owned(),
        };
        assert_eq!(
            transmitter_line(
                [3, 100, 5],
                5_000,
                tier,
                CardReadout::Bound(&bound),
                "",
                1_234
            ),
            "Player Transmitter at 3,100,5: 5000/1000000 FE (comparator 0, bound AdLf0208, \
             charging 500 FE/t, 1000 FE/t a side, t=1234)",
            "a bound card names the player the tick charges"
        );
        assert_eq!(
            transmitter_line([3, 100, 5], 5_000, tier, CardReadout::Blank, "", 1_234),
            "Player Transmitter at 3,100,5: 5000/1000000 FE (comparator 0, bound none, \
             charging 500 FE/t, 1000 FE/t a side, t=1234)",
            "a blank card is the control: nothing is bound, so nothing is charged"
        );
        assert_eq!(
            transmitter_line([3, 100, 5], 0, tier, CardReadout::Empty, "", 0),
            "Player Transmitter at 3,100,5: 0/1000000 FE (comparator 0, bound empty, \
             charging 500 FE/t, 1000 FE/t a side, t=0)",
            "an empty slot is not a blank card, so the readout says so"
        );
        assert!(
            transmitter_line([3, 100, 5], 0, tier, CardReadout::Unknown, "", 0)
                .contains("bound unknown"),
            "no block entity read is the exhaustiveness arm, not a claim"
        );
    }

    #[test]
    fn a_binding_card_component_is_the_uuid_and_the_name_and_nothing_else_is_a_player() {
        // `BoundPlayer.CODEC`: `gameProfileId` as `UUIDUtil`'s four ints, and `name`.
        let player = BoundPlayerValue {
            uuid: 0xffff_ffff_0000_0000_7fff_ffff_0000_0001,
            name: "Technici4n".to_owned(),
        };
        let encoded = BoundPlayerComponent.encode(player.clone());
        assert_eq!(
            encoded,
            Tag::compound([
                (
                    "gameProfileId",
                    Tag::IntArray(vec![-1, 0, 2_147_483_647, 1])
                ),
                ("name", Tag::String("Technici4n".into())),
            ]),
            "the component is upstream's own compound"
        );
        assert_eq!(
            BoundPlayerComponent.decode(&encoded),
            Some(player),
            "the UUID and the name round trip"
        );
        assert_eq!(
            BoundPlayerComponent.decode(&Tag::compound([("name", Tag::String("nobody".into()))])),
            None,
            "a payload without a profile id is no bound player"
        );
        assert_eq!(
            BoundPlayerComponent.decode(&Tag::compound([(
                "gameProfileId",
                Tag::IntArray(vec![1, 2, 3])
            )])),
            None,
            "a profile id that is not four ints is no bound player"
        );
        assert_eq!(
            uuid_of(uuid_ints(0x0123_4567_89ab_cdef_fedc_ba98_7654_3210)),
            0x0123_4567_89ab_cdef_fedc_ba98_7654_3210,
            "the four ints are the UUID's own halves"
        );
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
        let empty = status_line(furnator, [0, 100, 0], 0, 0, None, &[None], "", 150);
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
            "",
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
            "",
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
            "",
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
            status_line(starter, [20, 100, 34], 0, 0, None, &empty, "", 2600),
            "Energy Discharger at 20,100,34: 0/1000000 FE (comparator 0, charge empty, empty, \
             empty, empty, empty, empty, empty, 1000 FE/t a side, t=2600)"
        );
        let mut charged = empty;
        charged[0] = Some(("powah:energy_cell_starter", 5_000, 1_000_000));
        assert_eq!(
            status_line(starter, [20, 100, 34], 5_000, 0, None, &charged, "", 2612),
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
            "",
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
                "",
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
                "",
                2612,
            ),
            "Energy Hopper at 20,100,40: 5000/1000000 FE (comparator 0, facing north, charging \
             500 FE/t, 1000 FE/t a side, t=2612)"
        );
        // `status_line` gets the same line for the type's exhaustiveness, with no state to read.
        assert_eq!(
            status_line(starter, [20, 100, 40], 0, 0, None, &[], "", 2600),
            hopper_line(
                [20, 100, 40],
                0,
                crate::runtime::hopper_tier(0).unwrap(),
                None,
                "",
                2600
            )
        );
        let nitro = crate::runtime::hopper_tier(6).expect("the nitro tier");
        assert!(
            hopper_line([1, 2, 3], 2_000_000_000, nitro, Some(Direction::Down), "",  7)
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

    #[test]
    fn every_solar_tier_is_one_of_the_ladder_with_a_down_only_buffer() {
        // `PowahConfig.Generators.solar_panels` is a `GeneratorConfig`: passiveGeneration for its
        // 20 FE/t, the furnator's capacity and transfer for the rest. `SolarTile.isEnergyPresent`
        // is `Direction.DOWN`.
        let expected = [
            ("powah:solar_panel_starter", 20, 20_000, 80),
            ("powah:solar_panel_basic", 60, 80_000, 320),
            ("powah:solar_panel_hardened", 100, 200_000, 800),
            ("powah:solar_panel_blazing", 200, 800_000, 3_200),
            ("powah:solar_panel_niotic", 400, 2_000_000, 8_000),
            ("powah:solar_panel_spirited", 800, 8_000_000, 32_000),
            ("powah:solar_panel_nitro", 2_000, 40_000_000, 160_000),
        ];
        for (index, (id, generation, capacity, transfer)) in expected.into_iter().enumerate() {
            let tier = solar_of(id).unwrap_or_else(|| panic!("{id} is a solar panel"));
            assert_eq!(
                (tier.generation, tier.capacity, tier.transfer),
                (generation, capacity, transfer),
                "{id} is the ladder's tier {index}"
            );
            assert_eq!(
                Kind::of(id),
                Some(Kind::Solar(
                    crate::runtime::solar_tier(index).expect("the same tier in the rule's table")
                )),
                "{id} maps to its tier"
            );
            let buffer = battery_of(id).unwrap_or_else(|| panic!("{id} declares a buffer"));
            assert_eq!(buffer.capacity, capacity, "{id}'s capacity");
            assert_eq!(
                buffer.max_insert, 0,
                "{id} makes its own FE: `AbstractEnergyProvider` never takes a fill"
            );
            assert_eq!(buffer.max_extract, transfer, "{id}'s extract cap");
            assert_eq!(buffer.insert_faces, [false; 6], "{id} takes no FE in");
            let mut down_only = [false; 6];
            down_only[Direction::Down as usize] = true;
            assert_eq!(
                buffer.extract_faces, down_only,
                "{id} gives FE on its bottom face only (`isEnergyPresent` is `Direction.DOWN`)"
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
                1,
                "{id} holds `SolarTile`'s one charge slot"
            );
        }
        assert_eq!(
            solar_of("powah:solar_panel_creative"),
            None,
            "there is no solar tier past the ladder"
        );
        assert_eq!(solar_of("powah:energy_cell_starter"), None);
        assert_eq!(solar_of("minecraft:stone"), None);
    }

    #[test]
    fn the_solar_container_is_its_one_charge_slot() {
        let container = solar_container();
        assert_eq!(
            container.slots, 1,
            "`SolarTile`'s `inv.add(1)`, slot limit 1"
        );
        assert_eq!(container.inputs, vec![0]);
        assert_eq!(container.outputs, vec![0]);
        assert_eq!(container.faces, std::array::from_fn(|_| vec![0]));
        assert_eq!(container.menu, None, "this port has no GUI");
    }

    #[test]
    fn the_solar_readout_names_its_sky_its_generation_and_its_charge_slot() {
        let starter = crate::runtime::solar_tier(0).expect("the starter tier");
        let day = crate::runtime::Sky {
            can_see_sky: true,
            has_skylight: true,
            darken: 0,
        };
        assert_eq!(
            solar_line([16, 100, 20], 20, starter, Some(day), &[None], "", 1234),
            "Solar Panel at 16,100,20: 20/20000 FE (comparator 0, sky open, day true, generating \
             20 FE/t, 80 FE/t a side, charge empty, t=1234)"
        );
        assert_eq!(
            solar_line(
                [16, 100, 20],
                10_000,
                starter,
                Some(crate::runtime::Sky {
                    can_see_sky: false,
                    ..day
                }),
                &[None],
                "",
                1300
            ),
            "Solar Panel at 16,100,20: 10000/20000 FE (comparator 7, sky covered, day true, \
             generating 20 FE/t, 80 FE/t a side, charge empty, t=1300)"
        );
        assert!(
            solar_line(
                [16, 100, 20],
                10_000,
                starter,
                Some(crate::runtime::Sky { darken: 11, ..day }),
                &[None],
                "",
                1400
            )
            .contains("sky open, day false, generating 20 FE/t"),
            "a midnight panel still sees the sky but is not in daylight"
        );
        let nitro = crate::runtime::solar_tier(6).expect("the nitro tier");
        assert!(
            solar_line(
                [18, 100, 22],
                40_000_000,
                nitro,
                Some(day),
                &[Some(("powah:energy_cell_nitro", 5_000, 2_000_000_000))],
                "",
                1500
            )
            .contains(
                "40000000/40000000 FE (comparator 15, sky open, day true, generating 2000 FE/t, \
                 160000 FE/t a side, charge powah:energy_cell_nitro 5000/2000000000"
            ),
            "the nitro tier names its own generation, its own transfer and its charge slot"
        );
        // `status_line`'s exhaustiveness arm has no level to ask, so it reports neither sky nor day.
        assert_eq!(
            status_line(
                Kind::Solar(starter),
                [16, 100, 20],
                0,
                0,
                None,
                &[None],
                "",
                7
            ),
            solar_line([16, 100, 20], 0, starter, None, &[None], "", 7)
        );
        assert!(
            solar_line([16, 100, 20], 0, starter, None, &[None], "", 7)
                .contains("sky unknown, day unknown"),
            "the arm without a level says so instead of guessing"
        );
    }

    #[test]
    fn every_machine_with_a_redstone_button_has_a_mode_and_the_orb_and_rods_do_not() {
        // `AbstractEnergyScreen.init` offers the button to every energy machine, and exactly those
        // tile classes call `checkRedstone()`: the furnator, the cell, the cable, the discharger, the
        // hopper, the solar panel, the ender cell and gate, and the player transmitter.
        let offered = [
            powah_blocks::<{ FURNATOR_TIER_COUNT }>(&FURNATORS),
            powah_blocks::<{ CELL_TIER_COUNT }>(&CELLS),
            powah_blocks::<{ CABLE_TIER_COUNT }>(&CABLES),
            powah_blocks::<{ DISCHARGER_TIER_COUNT }>(&DISCHARGERS),
            powah_blocks::<{ HOPPER_TIER_COUNT }>(&HOPPERS),
            powah_blocks::<{ SOLAR_TIER_COUNT }>(&SOLARS),
            powah_blocks::<{ ENDER_TIER_COUNT }>(&ENDER_CELLS),
            powah_blocks::<{ ENDER_TIER_COUNT }>(&ENDER_GATES),
            powah_blocks::<{ TRANSMITTER_TIER_COUNT }>(&TRANSMITTERS),
        ];
        for (family, blocks) in [
            "furnator",
            "cell",
            "cable",
            "discharger",
            "hopper",
            "solar",
            "ender cell",
            "ender gate",
            "transmitter",
        ]
        .into_iter()
        .zip(offered)
        {
            assert!(!blocks.is_empty(), "the {family} ladder is not empty");
            for block in blocks {
                assert!(
                    has_redstone_mode(&block),
                    "{block} is a {family}: upstream's screen offers the button and its tile reads it"
                );
            }
        }
        // The orb and the rods are `AbstractTileScreen`s with neither the button nor the call.
        assert!(
            !has_redstone_mode(ORB),
            "the energizing orb has no redstone mode"
        );
        for (block, _) in RODS {
            assert!(
                !has_redstone_mode(block),
                "{block} is a rod: neither its screen nor its tile has a redstone mode"
            );
        }
        assert!(
            !has_redstone_mode("minecraft:stone"),
            "a foreign block has none"
        );
        // `/powah redstone` at a foreign block is "no block" (upstream's packet only acts on an
        // `AbstractTileEntity`); the orb and the rods are the "no mode" case. The in-game leg
        // caught this ordering first: a stone answered "no redstone mode" before "no block".
        assert_eq!(
            redstone_target("minecraft:stone"),
            RedstoneTarget::NoBlock,
            "a foreign block is not a machine without a mode, it is not one at all"
        );
        assert_eq!(redstone_target(ORB), RedstoneTarget::NoMode);
        assert_eq!(redstone_target(RODS[0].0), RedstoneTarget::NoMode);
        assert_eq!(redstone_target(TRANSMITTERS[0].0), RedstoneTarget::Machine);
        assert_eq!(redstone_target(CELLS[0].0), RedstoneTarget::Machine);
        assert_eq!(
            RedstoneMode::ALL.len(),
            3,
            "the three modes are the assembled set above, none missing"
        );
    }

    /// One ladder's block ids, as a `Vec`.
    fn powah_blocks<const N: usize>(ladder: &[(&'static str, &'static str); N]) -> Vec<String> {
        ladder
            .iter()
            .map(|(block, _)| (*block).to_owned())
            .collect()
    }

    #[test]
    fn a_saved_mode_is_the_int_upstream_writes_and_a_missing_key_is_the_default() {
        // `writeSync` puts the ordinal under `redstone_mode`; `Redstone.values()[getInt(..)]` reads
        // it back, and 0 (the default) is what a compound with no key reads as.
        let mut saved = Compound::new();
        assert_eq!(
            saved_mode(&saved),
            RedstoneMode::Ignore,
            "a fresh machine has no gate: upstream's default IGNORE"
        );
        for mode in RedstoneMode::ALL {
            saved.insert(KEY_REDSTONE_MODE.into(), Tag::Int(mode.ordinal()));
            assert_eq!(
                saved_mode(&saved),
                mode,
                "a saved ordinal {0} reads back as {1:?}",
                mode.ordinal(),
                mode,
            );
        }
        saved.insert(KEY_REDSTONE_MODE.into(), Tag::Int(9));
        assert_eq!(
            saved_mode(&saved),
            RedstoneMode::Ignore,
            "a doctored ordinal is the default, not a panic"
        );
        saved.insert(KEY_REDSTONE_MODE.into(), Tag::String("off".into()));
        assert_eq!(
            saved_mode(&saved),
            RedstoneMode::Ignore,
            "a key of the wrong type is the default"
        );
    }

    #[test]
    fn a_machines_readout_names_its_gate_only_when_it_is_not_the_default() {
        let tier = crate::runtime::transmitter_tier(0).expect("the starter transmitter");
        let plain = transmitter_line([3, 100, 5], 5000, tier, CardReadout::Empty, "", 1234);
        assert!(
            plain.ends_with("1000 FE/t a side, t=1234)"),
            "the default adds nothing, so every earlier slice's line is unchanged: {plain}"
        );
        // `IGNORE` adds nothing at all, which is what keeps the older legs' assertions matching.
        assert_eq!(
            RedstoneField {
                mode: RedstoneMode::Ignore,
                powered: true,
            }
            .text(),
            "",
            "the default mode is silent"
        );
        for (mode, powered, field) in [
            (RedstoneMode::Off, true, ", redstone off (powered)"),
            (RedstoneMode::Off, false, ", redstone off"),
            (RedstoneMode::On, true, ", redstone on (powered)"),
            (RedstoneMode::On, false, ", redstone on"),
        ] {
            assert_eq!(
                RedstoneField { mode, powered }.text(),
                field,
                "{mode:?} powered={powered}"
            );
        }
        // The field lands before the shared ` t=<n>)` tail the harness parses the clock from.
        let gated = transmitter_line(
            [3, 100, 5],
            5000,
            tier,
            CardReadout::Empty,
            &RedstoneField {
                mode: RedstoneMode::Off,
                powered: true,
            }
            .text(),
            1234,
        );
        assert!(
            gated.ends_with("1000 FE/t a side, redstone off (powered), t=1234)"),
            "the gate's field sits before the clock: {gated}"
        );
        // A cell's line carries it the same way.
        let cell = cell_tier(0).expect("the starter cell");
        let line = status_line(
            Kind::Cell(cell),
            [2, 100, 0],
            0,
            0,
            None,
            &[None],
            &RedstoneField {
                mode: RedstoneMode::On,
                powered: false,
            }
            .text(),
            77,
        );
        assert!(
            line.ends_with("FE/t a side, redstone on, t=77)"),
            "a cell's readout names its gate: {line}"
        );
    }

    /// The `comparator <n>` a readout line prints, so a test can compare the hook's answer with
    /// the number players already see.
    fn printed_comparator(line: &str) -> Option<i32> {
        let rest = line.split("comparator ").nth(1)?;
        let number: String = rest.chars().take_while(char::is_ascii_digit).collect();
        number.parse().ok()
    }

    #[test]
    fn only_the_machines_with_a_printed_comparator_answer_one() {
        let answers = [
            (Kind::Furnator(furnator_tier(0).unwrap()), 320, 20_000),
            (Kind::Cell(cell_tier(0).unwrap()), 500_000, 1_000_000),
            (
                Kind::Discharger(discharger_tier(0).unwrap()),
                500_000,
                1_000_000,
            ),
            (
                Kind::Hopper(crate::runtime::hopper_tier(0).unwrap()),
                500_000,
                1_000_000,
            ),
            (
                Kind::Solar(crate::runtime::solar_tier(0).unwrap()),
                10_000,
                20_000,
            ),
            (
                Kind::Transmitter(crate::runtime::transmitter_tier(0).unwrap()),
                500_000,
                1_000_000,
            ),
            (
                Kind::EnderCell(ender_cell_tier(0).unwrap()),
                500_000,
                4_000_000,
            ),
            (
                Kind::EnderGate(ender_gate_tier(0).unwrap()),
                500_000,
                4_000_000,
            ),
        ];
        for (kind, stored, capacity) in answers {
            assert!(has_comparator(kind), "{kind:?} prints a comparator field");
            let reading = comparator_of(kind, stored, capacity)
                .unwrap_or_else(|| panic!("{kind:?} answers a comparator"));
            // The readout is the authority: the hook must print the number the player reads.
            let line = match kind {
                Kind::Furnator(_) => status_line(kind, [1, 2, 3], stored, 0, None, &[None], "", 1),
                Kind::Hopper(tier) => {
                    hopper_line([1, 2, 3], stored, tier, Some(Direction::North), "", 1)
                }
                Kind::Solar(tier) => solar_line(
                    [1, 2, 3],
                    stored,
                    tier,
                    Some(Sky {
                        can_see_sky: true,
                        has_skylight: true,
                        darken: 0,
                    }),
                    &[None],
                    "",
                    1,
                ),
                Kind::Transmitter(tier) => {
                    transmitter_line([1, 2, 3], stored, tier, CardReadout::Empty, "", 1)
                }
                Kind::EnderCell(_) | Kind::EnderGate(_) => ender_line(
                    kind,
                    [1, 2, 3],
                    Channel { stored, capacity },
                    0,
                    None,
                    &[None],
                    "",
                    1,
                ),
                _ => status_line(kind, [1, 2, 3], stored, 0, None, &[None], "", 1),
            };
            assert_eq!(
                printed_comparator(&line),
                Some(reading),
                "the comparator at {kind:?} reads what its line prints: {line}"
            );
            // The fraction is upstream's `toComparatorPower` and the readout's own rounding.
            assert_eq!(
                reading,
                crate::runtime::comparator_strength(stored, capacity)
            );
            assert_eq!(comparator_of(kind, capacity * 2, capacity), Some(15));
            assert_eq!(comparator_of(kind, 0, capacity), Some(0));
        }
    }

    #[test]
    fn the_orb_and_the_rods_answer_a_comparator_and_the_cable_does_not() {
        // Slice 15 closes 0.2.14's deviation 1: upstream's orb answers its non-empty stack count
        // and a rod answers its own buffer fraction; a cable still stores nothing.
        assert_eq!(analog_reading(Kind::Orb, 0, 0, 3), Some(3));
        assert_eq!(analog_reading(Kind::Orb, 0, 0, 20), Some(15));
        let rod = Kind::Rod(rod_of("powah:energizing_rod_starter").unwrap());
        assert_eq!(analog_reading(rod, 5_000, 10_000, 0), Some(7));
        assert_eq!(analog_reading(rod, 10_000, 10_000, 0), Some(15));
        assert_eq!(analog_reading(rod, 0, 10_000, 0), Some(0));
        assert_eq!(
            analog_reading(
                Kind::Cable(cable_of("powah:energy_cable_starter").unwrap()),
                0,
                0,
                0
            ),
            None,
            "a cable stores nothing and is not given a signal to make the family look complete"
        );
    }

    #[test]
    fn a_comparators_reading_is_per_position_and_per_dimension() {
        let mut outputs = Outputs::default();
        assert_eq!(outputs.get(LevelKey::OVERWORLD, [1, 2, 3]), None);
        assert!(
            outputs.record(LevelKey::OVERWORLD, [1, 2, 3], 7),
            "a first reading is a change"
        );
        assert_eq!(outputs.get(LevelKey::OVERWORLD, [1, 2, 3]), Some(7));
        assert_eq!(
            outputs.get(LevelKey::NETHER, [1, 2, 3]),
            None,
            "another dimension's machine at one position has its own reading"
        );
        assert_eq!(outputs.get(LevelKey::OVERWORLD, [1, 2, 4]), None);
        assert!(
            !outputs.record(LevelKey::OVERWORLD, [1, 2, 3], 7),
            "an unchanged reading tells no comparator"
        );
        assert!(outputs.record(LevelKey::OVERWORLD, [1, 2, 3], 8));
        assert_eq!(outputs.get(LevelKey::OVERWORLD, [1, 2, 3]), Some(8));
        outputs.clear();
        assert_eq!(outputs.get(LevelKey::OVERWORLD, [1, 2, 3]), None);
    }
}
