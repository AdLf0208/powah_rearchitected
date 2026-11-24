// Pure machine logic for Powah slice 1 (Furnator, Energy Cell, Energy Cable).
//
// This file is both the standalone Cargo library and the injected
// `crate::game::powah` module. It has no game types: the hook glue in
// `support.rs` turns block entities, energy buffers and item stacks into the
// small values used here.
//
// Numbers are upstream's STARTER-tier defaults (`config/v2/DefaultEnergies`):
// see `FINDINGS.md` for the derivation.

/// Furnator STARTER buffer (`generatorCapacity`).
pub const FURNATOR_CAPACITY: i32 = 20_000;
/// Furnator STARTER transfer, per side and per tick (`generatorTransfer`).
pub const FURNATOR_TRANSFER: i32 = 80;
/// Furnator generation while burning (`activeProduction`).
pub const FURNATOR_GENERATION: i32 = 20;
/// `general.energy_per_fuel_tick`: one burn tick is worth this much carbon.
pub const ENERGY_PER_FUEL_TICK: i32 = 30;
/// Energy Cell STARTER buffer (`batteryCapacity`).
pub const CELL_CAPACITY: i32 = 1_000_000;
/// Energy Cell STARTER transfer, per side and per tick (`batteryTransfer`).
pub const CELL_TRANSFER: i32 = 1_000;
/// Energy Cable STARTER transfer (`cableTransfer`); the cable stores nothing.
pub const CABLE_TRANSFER: i32 = 500;

/// The tiers a non-creative Powah block has (`Tier.getNormalVariants`), in order:
/// starter, basic, hardened, blazing, niotic, spirited, nitro.
pub const CELL_TIER_COUNT: usize = 7;

/// The tiers a furnator has, in `Tier.getNormalVariants` order: starter, basic,
/// hardened, blazing, niotic, spirited, nitro.
pub const FURNATOR_TIER_COUNT: usize = 7;

/// The tiers an energy discharger has, in `Tier.getNormalVariants` order: starter, basic,
/// hardened, blazing, niotic, spirited, nitro (upstream's `Tier.getNormalVariants`, which
/// `Blcks.ENERGY_DISCHARGER` uses).
pub const DISCHARGER_TIER_COUNT: usize = 7;

/// One furnator tier's numbers: `activeProduction` (its generation while
/// burning), `generatorCapacity` and `generatorTransfer` at that tier
/// (`config/v2/DefaultEnergies`, derived from `baseScaling`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FurnatorTier {
    pub generation: i32,
    pub capacity: i32,
    pub transfer: i32,
}

/// The furnator tier at `index` (0 = starter … 6 = nitro), `None` past the ladder.
#[must_use]
pub fn furnator_tier(index: usize) -> Option<FurnatorTier> {
    /// `DefaultEnergies.baseScaling`, in `Tier.getNormalVariants` order.
    const BASE_SCALING: [i32; FURNATOR_TIER_COUNT] = [1, 4, 10, 40, 100, 400, 2_000];
    /// `activeProduction` = `baseScaling * 20`.
    const GENERATION_PER_SCALING: i32 = 20;
    /// `generatorCapacity` = `activeProduction * 1000`.
    const CAPACITY_PER_SCALING: i32 = 20_000;
    /// `generatorTransfer` = `activeProduction * 4`.
    const TRANSFER_PER_SCALING: i32 = 80;
    let scaling = *BASE_SCALING.get(index)?;
    Some(FurnatorTier {
        generation: scaling * GENERATION_PER_SCALING,
        capacity: scaling * CAPACITY_PER_SCALING,
        transfer: scaling * TRANSFER_PER_SCALING,
    })
}

/// The tiers an energy cable has, in `Tier.getNormalVariants` order: starter,
/// basic, hardened, blazing, niotic, spirited, nitro.
pub const CABLE_TIER_COUNT: usize = 7;

/// One energy cable tier's rate: `cableTransfer` at that tier
/// (`config/v2/DefaultEnergies`, `baseScaling * 500`). A cable stores nothing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CableTier {
    pub transfer: i32,
}

/// The energy cable tier at `index` (0 = starter … 6 = nitro), `None` past the
/// ladder.
#[must_use]
pub fn cable_tier(index: usize) -> Option<CableTier> {
    /// `DefaultEnergies.baseScaling`, in `Tier.getNormalVariants` order.
    const BASE_SCALING: [i32; CABLE_TIER_COUNT] = [1, 4, 10, 40, 100, 400, 2_000];
    /// `cableTransfer` = `reactorProduction * 2` = `baseScaling * 250 * 2`.
    const TRANSFER_PER_SCALING: i32 = 500;
    let scaling = *BASE_SCALING.get(index)?;
    Some(CableTier {
        transfer: scaling * TRANSFER_PER_SCALING,
    })
}

/// How far from an orb a rod may stand (`PowahConfig.general.energizing_range`),
/// in blocks, Chebyshev distance like upstream's `BlockPos.betweenClosed`.
pub const ENERGIZING_RANGE: i32 = 4;
/// The orb's inventory (`this.inv.set(7)`): slot 0 is the result, 1..6 the inputs.
pub const ORB_SLOTS: usize = 7;
/// The orb's result slot, the only one a hopper may take from.
pub const ORB_OUTPUT_SLOT: usize = 0;
/// The orb's six input slots, the only ones a hopper may fill.
pub const ORB_INPUT_SLOTS: usize = ORB_SLOTS - 1;
/// The orb's declared buffer: the largest energizing recipe
/// (`crystal_nitro`'s 20,000,000). A matched recipe's own energy is the cap its
/// tick enforces and its readout shows.
pub const ORB_MAX_ENERGY: i32 = 20_000_000;
/// How many ticks a rod charges before its first fill
/// (`EnergizingRodTile`'s `Ticker(20)`); it fills every tick after that.
pub const ROD_WARMUP: i32 = 20;

/// One energy cell tier's storage numbers: `batteryCapacity` and
/// `batteryTransfer` at that tier (`config/v2/DefaultEnergies`, both derived from
/// `baseScaling` — `× 1_000_000` and `× 1_000`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellTier {
    pub capacity: i32,
    pub transfer: i32,
}

/// The energy cell tier at `index` (0 = starter … 6 = nitro), `None` past the
/// ladder — the creative cell is a source, not a storage tier, and is not carried.
#[must_use]
pub fn cell_tier(index: usize) -> Option<CellTier> {
    /// `DefaultEnergies.baseScaling`, in `Tier.getNormalVariants` order.
    const BASE_SCALING: [i32; CELL_TIER_COUNT] = [1, 4, 10, 40, 100, 400, 2_000];
    /// `batteryCapacity` = `reactorProduction * 4000` = `baseScaling * 250 * 4000`.
    const CAPACITY_PER_SCALING: i32 = 1_000_000;
    /// `batteryTransfer` = `cableTransfer * 2` = `baseScaling * 250 * 4`.
    const TRANSFER_PER_SCALING: i32 = 1_000;
    let scaling = *BASE_SCALING.get(index)?;
    Some(CellTier {
        capacity: scaling * CAPACITY_PER_SCALING,
        transfer: scaling * TRANSFER_PER_SCALING,
    })
}

/// One energy discharger tier's storage numbers: upstream's `devices.dischargers` config is the
/// battery config (`PowahConfig.EnergyDevices`), so a tier's numbers are its energy cell's —
/// `batteryCapacity` and `batteryTransfer` at that tier.
pub type DischargerTier = CellTier;

/// The energy discharger tier at `index` (0 = starter … 6 = nitro), `None` past the ladder.
#[must_use]
pub fn discharger_tier(index: usize) -> Option<DischargerTier> {
    if index >= DISCHARGER_TIER_COUNT {
        return None;
    }
    cell_tier(index)
}

/// The tiers an energy hopper has, in `Tier.getNormalVariants` order: starter, basic, hardened,
/// blazing, niotic, spirited, nitro (upstream's `Tier.getNormalVariants`, which
/// `Blcks.ENERGY_HOPPER` uses).
pub const HOPPER_TIER_COUNT: usize = 7;

/// One energy hopper tier's numbers (`PowahConfig.EnergyDevices.hoppers`, a `ChargingConfig`):
/// `batteryCapacity` and `batteryTransfer` for its buffer and its faces, and `chargingTransfer`
/// for the most it spends on one item in the container it faces.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HopperTier {
    /// The tier's buffer (`batteryCapacity`), the same row as the energy cell's.
    pub capacity: i32,
    /// The tier's per-side transfer (`batteryTransfer`), both ways (`Transfer.ALL`).
    pub transfer: i32,
    /// The tier's per-item charging rate (`chargingTransfer`).
    pub charging_rate: i32,
}

/// The energy hopper tier at `index` (0 = starter … 6 = nitro), `None` past the ladder.
#[must_use]
pub fn hopper_tier(index: usize) -> Option<HopperTier> {
    /// `DefaultEnergies.baseScaling`, in `Tier.getNormalVariants` order.
    const BASE_SCALING: [i32; HOPPER_TIER_COUNT] = [1, 4, 10, 40, 100, 400, 2_000];
    /// `batteryCapacity` = `reactorProduction * 4000` = `baseScaling * 1_000_000`.
    const CAPACITY_PER_SCALING: i32 = 1_000_000;
    /// `batteryTransfer` = `cableTransfer * 2` = `baseScaling * 1_000`.
    const TRANSFER_PER_SCALING: i32 = 1_000;
    /// `chargingTransfer` = `cableTransfer` = `baseScaling * 500`, the per-item charge rate.
    const CHARGING_PER_SCALING: i32 = 500;
    let scaling = *BASE_SCALING.get(index)?;
    Some(HopperTier {
        capacity: scaling * CAPACITY_PER_SCALING,
        transfer: scaling * TRANSFER_PER_SCALING,
        charging_rate: scaling * CHARGING_PER_SCALING,
    })
}

/// The tiers a player transmitter has, in `Tier.getNormalVariants` order: starter, basic, hardened,
/// blazing, niotic, spirited, nitro (upstream's `Tier.getNormalVariants`, which
/// `Blcks.PLAYER_TRANSMITTER` uses).
pub const TRANSMITTER_TIER_COUNT: usize = 7;

/// One player transmitter tier's numbers (`PowahConfig.EnergyDevices.player_transmitters`, a
/// `ChargingConfig`): `batteryCapacity`/`batteryTransfer` for its buffer and its faces and
/// `chargingTransfer` for the most it spends on one item of the bound player's inventory — the same
/// three rows `hoppers` reads, so a tier's transmitter and hopper are the same numbers.
pub type TransmitterTier = HopperTier;

/// The player transmitter tier at `index` (0 = starter … 6 = nitro), `None` past the ladder.
#[must_use]
pub fn transmitter_tier(index: usize) -> Option<TransmitterTier> {
    if index >= TRANSMITTER_TIER_COUNT {
        return None;
    }
    hopper_tier(index)
}

/// How many of a player's inventory slots `Player.invStacks` walks: 36 main (0..36), the armour
/// (36..40) and the off hand (40), in `Inventory`'s own numbers.
pub const PLAYER_INVENTORY_SLOTS: usize = 41;

/// The `ItemPlace::PlayerSlot` number of a slot `Inventory` numbers `slot` (`Player.invStacks`):
/// `Inventory`'s 0..36 are the same, the armour's 36..40 (feet to head) are the item place's
/// 100..=103, and the off hand's 40 is -106 — the numbering NeoForge's `ItemAccess.forPlayerSlot`
/// takes, not `Inventory`'s. `None` past the inventory.
#[must_use]
pub fn player_slot(slot: usize) -> Option<i32> {
    match slot {
        0..=35 => i32::try_from(slot).ok(),
        36..=39 => i32::try_from(100 + (slot - 36)).ok(),
        40 => Some(-106),
        _ => None,
    }
}

/// One pass over a bound player's inventory (`ChargeUtil.chargeItemsInPlayerInv(player,
/// charging_rates.get(variant), energy.getStored())`): the hopper's pass over 41 slots instead of a
/// container's — `Player.invStacks` is the main inventory, the armour and the off hand, so
/// `rooms[slot]` is one of those slots' rooms and `0` for an empty slot or an item with no energy
/// buffer.
#[must_use]
pub fn player_charges(stored: i32, charging_rate: i32, rooms: &[i32]) -> Vec<i32> {
    charge_slots(stored, charging_rate, rooms)
}

/// One pass over the container an energy hopper faces
/// (`ChargeUtil.chargeItemsInContainer(container, charging_rates.get(variant), energy.getStored())`):
/// each slot is asked for `min(the tier's charging rate, what is left of the buffer, the slot's own
/// room)`, the buffer's budget shrinking by what the pass has asked for so far. `rooms[slot]` is
/// that slot's room — `0` for an empty slot or an item with no energy buffer, which is left alone.
#[must_use]
pub fn hopper_charges(stored: i32, charging_rate: i32, rooms: &[i32]) -> Vec<i32> {
    charge_slots(stored, charging_rate, rooms)
}

/// The tiers a solar panel has, in `Tier.getNormalVariants` order: starter, basic, hardened,
/// blazing, niotic, spirited, nitro (upstream's `Tier.getNormalVariants`, which `Blcks.SOLAR_PANEL`
/// uses).
pub const SOLAR_TIER_COUNT: usize = 7;

/// One solar panel tier's numbers (`PowahConfig.Generators.solar_panels`, a `GeneratorConfig`):
/// `passiveProduction` while the panel sees the sky, the tier's `generatorCapacity` for its buffer
/// and `generatorTransfer` for the FE it gives a side.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SolarTier {
    /// `passiveProduction` — the one generator row that is not `activeProduction`.
    pub generation: i32,
    /// `generatorCapacity`, the same row a furnator of this tier stores.
    pub capacity: i32,
    /// `generatorTransfer`, the same row a furnator of this tier gives a side.
    pub transfer: i32,
}

/// The solar panel tier at `index` (0 = starter … 6 = nitro), `None` past the ladder.
#[must_use]
pub fn solar_tier(index: usize) -> Option<SolarTier> {
    /// `DefaultEnergies.baseScaling`, in `Tier.getNormalVariants` order.
    const BASE_SCALING: [i32; SOLAR_TIER_COUNT] = [1, 4, 10, 40, 100, 400, 2_000];
    /// `DefaultEnergies.baseSlowScaling`, the row `passiveProduction` is derived from.
    const BASE_SLOW_SCALING: [i32; SOLAR_TIER_COUNT] = [1, 3, 5, 10, 20, 40, 100];
    /// `passiveProduction` = `baseSlowScaling * 20`.
    const GENERATION_PER_SCALING: i32 = 20;
    /// `generatorCapacity` = `activeProduction * 1000` = `baseScaling * 20_000`.
    const CAPACITY_PER_SCALING: i32 = 20_000;
    /// `generatorTransfer` = `activeProduction * 4` = `baseScaling * 80`.
    const TRANSFER_PER_SCALING: i32 = 80;
    let scaling = *BASE_SCALING.get(index)?;
    let passive = *BASE_SLOW_SCALING.get(index)?;
    Some(SolarTier {
        generation: passive * GENERATION_PER_SCALING,
        capacity: scaling * CAPACITY_PER_SCALING,
        transfer: scaling * TRANSFER_PER_SCALING,
    })
}

/// What a solar panel reads of the sky above it, as `SolarTile.postTick` does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sky {
    /// `Misc.canBlockSeeSky(world, pos.above())`: full sky light at the block above the panel.
    pub can_see_sky: bool,
    /// `world.dimensionType().hasSkyLight()`: the Nether and the End fail this.
    pub has_skylight: bool,
    /// `world.getSkyDarken()`: 0 at noon, 11 at midnight; upstream's daylight test is `< 4`.
    pub darken: i32,
}

impl Sky {
    /// Upstream's `dimensionType().hasSkyLight() && getSkyDarken() < 4`.
    #[must_use]
    pub fn daylight(&self) -> bool {
        self.has_skylight && self.darken < SOLAR_DAY_MAX_DARKEN
    }
}

/// Upstream's `getSkyDarken()` bound: below this it is day, at or above it the panel sleeps.
pub const SOLAR_DAY_MAX_DARKEN: i32 = 4;

/// One solar panel tick's production (`SolarTile.postTick`): a buffer with room, an open sky (a
/// Lens of Ender stands in for it) and daylight in a dimension the sky lights make
/// `min(generation, room)`; anything else makes nothing.
#[must_use]
pub fn solar_production(energy: i32, tier: SolarTier, sky: Sky, lens_of_ender: bool) -> i32 {
    if !(sky.can_see_sky || lens_of_ender) || !sky.daylight() {
        return 0;
    }
    (tier.capacity - energy).max(0).min(tier.generation)
}

/// The channels one owner's ender network has at most (`EnderNetwork.MAX_CHANNELS`): a channel
/// index is 0 ..= 11, and a tier's own count (below) is what its GUI offers.
pub const MAX_CHANNELS: i32 = 12;

/// The tiers an ender cell and an ender gate have, in `Tier.getNormalVariants` order: starter,
/// basic, hardened, blazing, niotic, spirited, nitro (`Blcks.ENDER_CELL` / `Blcks.ENDER_GATE`).
pub const ENDER_TIER_COUNT: usize = 7;

/// One ender cell or gate tier's numbers (`PowahConfig.EnergyDevices.ender_cells` /
/// `ender_gates`, an `EnderConfig`): the FE one call moves through the channel
/// (`EnderConfig.getTransfer`) and the channels the tier offers
/// (`EnderConfig.channels`, a `TieredChannelValues`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnderTier {
    /// The most one insert or extract moves for this tier (`batteryTransfer` for a cell,
    /// `cableTransfer` for a gate).
    pub transfer: i32,
    /// How many channels the tier offers: the saved index is 0 ..= `channels - 1`
    /// (`TieredChannelValues.getDefault()` = 1, 2, 3, 5, 7, 9, 12).
    pub channels: i32,
}

/// `TieredChannelValues.getDefault()`, in `Tier.getNormalVariants` order.
const ENDER_CHANNELS: [i32; ENDER_TIER_COUNT] = [1, 2, 3, 5, 7, 9, 12];

/// The ender cell tier at `index` (0 = starter … 6 = nitro), `None` past the ladder.
///
/// Its transfer is `batteryTransfer()` = `reactorProduction * 2 * 2` = `baseScaling * 1_000`:
/// the energy cell's per-side row, which `PowahConfig` hands the cell's `EnderConfig`.
#[must_use]
pub fn ender_cell_tier(index: usize) -> Option<EnderTier> {
    /// `DefaultEnergies.baseScaling`, in `Tier.getNormalVariants` order.
    const BASE_SCALING: [i32; ENDER_TIER_COUNT] = [1, 4, 10, 40, 100, 400, 2_000];
    /// `batteryTransfer` = `reactorProduction * 4` = `baseScaling * 250 * 4`.
    const TRANSFER_PER_SCALING: i32 = 1_000;
    let scaling = *BASE_SCALING.get(index)?;
    Some(EnderTier {
        transfer: scaling * TRANSFER_PER_SCALING,
        channels: *ENDER_CHANNELS.get(index)?,
    })
}

/// The ender gate tier at `index` (0 = starter … 6 = nitro), `None` past the ladder.
///
/// Its transfer is `cableTransfer()` = `reactorProduction * 2` = `baseScaling * 500`: the cable's
/// rate, which `PowahConfig` hands the gate's `EnderConfig` — a gate is a tap, so it moves at the
/// cable's rate, and it cannot extend the network.
#[must_use]
pub fn ender_gate_tier(index: usize) -> Option<EnderTier> {
    /// `DefaultEnergies.baseScaling`, in `Tier.getNormalVariants` order.
    const BASE_SCALING: [i32; ENDER_TIER_COUNT] = [1, 4, 10, 40, 100, 400, 2_000];
    /// `cableTransfer` = `reactorProduction * 2` = `baseScaling * 250 * 2`.
    const TRANSFER_PER_SCALING: i32 = 500;
    let scaling = *BASE_SCALING.get(index)?;
    Some(EnderTier {
        transfer: scaling * TRANSFER_PER_SCALING,
        channels: *ENDER_CHANNELS.get(index)?,
    })
}

/// The channel a tier offers at `channel`.
///
/// Upstream saves the index on the tile as a `RangedInt(12)` (0..11) and gates the *buttons* it
/// shows with `getMaxChannels()`; the port has no GUI, so its `/powah channel` is that gate and a
/// value past the tier's count never reaches the tile.
#[must_use]
pub fn channel_clamped(channel: i32, channels: i32) -> i32 {
    channel.clamp(0, (channels - 1).max(0))
}

/// One channel of an owner's ender network: what it holds and what it can hold
/// (`EnderNetwork`'s shared `Energy`). Its capacity is 0 until an extender item is inserted, so a
/// fresh channel stores nothing — upstream's own empty network.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Channel {
    /// The FE the channel holds.
    pub stored: i32,
    /// The FE the channel can hold.
    pub capacity: i32,
}

impl Channel {
    /// `Energy.receiveEnergy`: at most the room, the tier's transfer and what was offered.
    #[must_use]
    pub fn insert(self, amount: i32, transfer: i32) -> (Channel, i32) {
        let moved = (self.capacity - self.stored)
            .max(0)
            .min(transfer.max(0))
            .min(amount.max(0));
        (
            Channel {
                stored: self.stored + moved,
                ..self
            },
            moved,
        )
    }

    /// `Energy.extractEnergy`: at most what the channel holds and the tier's transfer.
    #[must_use]
    pub fn extract(self, amount: i32, transfer: i32) -> (Channel, i32) {
        let moved = self.stored.max(0).min(transfer.max(0)).min(amount.max(0));
        (
            Channel {
                stored: self.stored - moved,
                ..self
            },
            moved,
        )
    }

    /// `AbstractEnderTile.onSlotChanged`: an extender item's capacity joins the channel and the
    /// item's own charge is spent into it, clamped to the new capacity (upstream runs
    /// `energy.setCapacity(newCap)` then `energy.setStored(stored + extended)`, whose setters clamp).
    #[must_use]
    pub fn extended(self, capacity: i32, stored: i32) -> Channel {
        let capacity = self.capacity.saturating_add(capacity.max(0));
        Channel {
            stored: clamp_energy(self.stored.saturating_add(stored.max(0)), capacity),
            capacity,
        }
    }
}

/// One pass's worth of charge steps over a machine's slots
/// (`ChargeUtil.transferSlotList` with `min(maxPerStack, maxTotal - charged)`): each slot takes
/// `min(per_slot, what is left of the budget, the slot's own room)`, the budget shrinking by what
/// the pass has taken so far. `rooms[slot]` is that slot's room — `0` for an empty slot or an item
/// with no energy buffer, which is left alone.
#[must_use]
pub fn charge_slots(stored: i32, per_slot: i32, rooms: &[i32]) -> Vec<i32> {
    let mut budget = stored.max(0);
    let mut planned = Vec::with_capacity(rooms.len());
    for room in rooms {
        let asked = charge_step(budget, per_slot, *room);
        budget -= asked;
        planned.push(asked);
    }
    planned
}

/// Energy clamped into a buffer.
#[must_use]
pub fn clamp_energy(energy: i32, capacity: i32) -> i32 {
    energy.clamp(0, capacity.max(0))
}

/// `floor(energy / capacity * 15)`, as upstream's `getComparatorStrength`.
#[must_use]
pub fn comparator_strength(energy: i32, capacity: i32) -> i32 {
    if capacity <= 0 {
        return 0;
    }
    let strength = (i64::from(energy.max(0)) * 15 / i64::from(capacity)) as i32;
    strength.clamp(0, 15)
}

/// Upstream `EnergizingOrbBlock.getAnalogOutputSignal`: the count of non-empty stacks in the orb's
/// inventory, capped at 15. The orb has seven slots upstream, so the cap is a guard rather than a
/// reachable value; the rule is a count, not a fraction of the buffer.
#[must_use]
pub fn stack_comparator(non_empty_stacks: usize) -> i32 {
    non_empty_stacks.min(15) as i32
}

/// Upstream's `extractFromSides`: at most the transfer, and never more than is
/// stored.
#[must_use]
pub fn push_amount(stored: i32, transfer: i32) -> i32 {
    stored.max(0).min(transfer.max(0))
}

/// How much one charge step moves into an item (`ChargeUtil.transferSlotList`
/// with `min(maxPerStack, maxTotal - charged)` and the item's own room):
/// `min(transfer, stored, room)`, never negative.
#[must_use]
pub fn charge_step(stored: i32, transfer: i32, room: i32) -> i32 {
    stored.max(0).min(transfer.max(0)).min(room.max(0))
}

/// How much one discharge step takes out of an item (`ChargeUtil.transferSlotList` with
/// `min(maxPerStack, maxTotal - charged)`, calling the item's `extractEnergy`):
/// `min(the item's own charge, the tier's transfer, the buffer's room)`, never negative. It is
/// [`charge_step`]'s mirror — the item is the source and the machine's buffer takes.
#[must_use]
pub fn discharge_step(held: i32, transfer: i32, room: i32) -> i32 {
    held.max(0).min(transfer.max(0)).min(room.max(0))
}

/// What a broken machine's own item carries (upstream's `keepEnergy`):
/// `Some(energy)` when the buffer holds something, clamped into the tier,
/// `None` when it is empty — upstream writes the storable compound only when it
/// is not empty.
#[must_use]
pub fn carried_energy(energy: i32, capacity: i32) -> Option<i32> {
    let carried = clamp_energy(energy, capacity);
    (carried > 0).then_some(carried)
}

/// The FE a placed machine takes from its item's payload, clamped into the
/// tier's capacity so a doctored item cannot create FE.
#[must_use]
pub fn restored_energy(asked: i32, capacity: i32) -> i32 {
    clamp_energy(asked, capacity)
}

/// The Furnator's saved state: the buffer and the carbon left to burn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Furnator {
    pub energy: i32,
    pub carbon: i32,
}

impl Furnator {
    /// The empty furnator.
    #[must_use]
    #[allow(
        dead_code,
        reason = "pure API: the glue builds the state from NBT, and the game build does not compile the tests"
    )]
    pub fn new() -> Self {
        Self::default()
    }

    /// Upstream's refuel gate: no carbon left (the buffer's room does not
    /// matter — fuel becomes carbon first and burns when there is room).
    #[must_use]
    pub fn wants_fuel(&self) -> bool {
        self.carbon <= 0
    }

    /// True while carbon is left (upstream's `!carbon.isEmpty()`).
    #[must_use]
    pub fn burning(&self) -> bool {
        self.carbon > 0
    }

    /// Take one fuel item's burn time. False when nothing was taken.
    pub fn load_fuel(&mut self, burn_time: u32) -> bool {
        if self.burning() || burn_time == 0 {
            return false;
        }
        self.carbon = i32::try_from(burn_time)
            .unwrap_or(i32::MAX)
            .saturating_mul(ENERGY_PER_FUEL_TICK);
        true
    }

    /// One server tick: produce `min(space, generation, carbon)` at this tier.
    /// Returns the FE stored (0 when the buffer is full or nothing is burning). A
    /// full buffer does not drain carbon.
    pub fn tick(&mut self, tier: FurnatorTier) -> i32 {
        let space = (tier.capacity - self.energy).max(0);
        let produced = space.min(tier.generation).min(self.carbon.max(0));
        if produced > 0 {
            self.energy += produced;
            self.carbon -= produced;
        }
        produced
    }
}

/// A neighbour's energy buffer as one side reaches it (`native_energy`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Neighbour {
    pub energy: i32,
    pub capacity: i32,
    pub max_insert: i32,
    pub max_extract: i32,
    pub can_insert: bool,
    pub can_extract: bool,
}

impl Neighbour {
    /// The room left for an insert.
    #[must_use]
    pub fn space(&self) -> i32 {
        if self.can_insert {
            (self.capacity - self.energy).max(0)
        } else {
            0
        }
    }

    /// What one move may take out of it.
    #[must_use]
    pub fn available(&self) -> i32 {
        if self.can_extract {
            self.energy.max(0).min(self.max_extract.max(0))
        } else {
            0
        }
    }
}

/// One `native_move_energy` call: `from` and `to` are neighbour indexes
/// (0 = down, as `Direction`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Transfer {
    pub from: usize,
    pub to: usize,
    pub amount: i32,
}

/// The moves one cable makes in a tick, from its six sides as `native_energy`
/// presents them: from each neighbour that can give into each that can take, up
/// to the cable's transfer rate in total. `rotation` rotates the starting side so
/// two equal sources share the cable as upstream's round-robin does. A cable
/// stores nothing, so no move starts and ends at the same neighbour, and the
/// total never exceeds `transfer`.
///
/// This is [`network_moves`] on one cable's six sides; both delegate to the one
/// planner, so the single-cable plan slice 1 proved in game and the network plan
/// cannot drift apart.
#[must_use]
pub fn cable_moves(
    neighbours: &[Option<Neighbour>; 6],
    transfer: i32,
    rotation: usize,
) -> Vec<Transfer> {
    plan(neighbours, transfer, rotation)
}

/// The one planner: `sides` is every place the relay can reach, in the order the
/// caller gathered them (`None` for an unreachable place). Upstream's
/// `CableNet` walks the connected cables round-robin and pushes into each energy
/// side at most a cable's transfer; here one plan covers the whole relay, so the
/// budget is spent once across every source and sink in it.
fn plan(sides: &[Option<Neighbour>], transfer: i32, rotation: usize) -> Vec<Transfer> {
    let mut moves = Vec::new();
    let count = sides.len();
    if count == 0 {
        return moves;
    }
    let start = rotation % count;
    let mut budget = transfer.max(0);
    let mut sink = sides.to_vec();
    for step in 0..count {
        if budget == 0 {
            break;
        }
        let from = (start + step) % count;
        let Some(source) = sides[from] else {
            continue;
        };
        let mut available = source.available();
        for other in 0..count {
            if available == 0 || budget == 0 {
                break;
            }
            let to = (start + other) % count;
            if to == from {
                continue;
            }
            let Some(target) = sink[to] else {
                continue;
            };
            // A network only runs from the fuller store to the emptier one: a storage would
            // otherwise give back what it received on the next tick, because the rotation picks
            // the receiver as a source as soon as it holds a step (measured in game 2026-11-23:
            // two cells on one cable ping-ponged 500/0 for a hundred ticks). A provider — a
            // furnator, a solar panel, a discharger, whose `can_insert` is false — always gives:
            // its buffer is filled by its own tick, not by the network.
            if source.can_insert && target.can_insert && source.energy <= target.energy {
                continue;
            }
            let amount = available
                .min(target.space())
                .min(target.max_insert.max(0))
                .min(budget);
            if amount <= 0 {
                continue;
            }
            moves.push(Transfer { from, to, amount });
            available -= amount;
            budget -= amount;
            let mut after = target;
            after.energy += amount;
            sink[to] = Some(after);
        }
    }
    moves
}

/// The moves one **cable network** makes in a tick. `ports` are the network's
/// external energy sides — every neighbour that is not a cable, of every cable in
/// the network — in the order the caller gathered them; `from`/`to` index that
/// slice.
///
/// The network is one relay: it spends one cable's transfer per tick, not one per
/// cable, so a longer run carries the same rate and the readout's "moves up to
/// 500 FE/t" stays true for the whole network.
#[must_use]
pub fn network_moves(ports: &[Neighbour], transfer: i32, rotation: usize) -> Vec<Transfer> {
    let sides: Vec<Option<Neighbour>> = ports.iter().copied().map(Some).collect();
    plan(&sides, transfer, rotation)
}

/// The most ingredients one energizing recipe has (the nitro crystal's four).
pub const MAX_INGREDIENTS: usize = 6;

/// The tiers an energizing rod has, in `Tier.getNormalVariants` order: starter,
/// basic, hardened, blazing, niotic, spirited, nitro.
pub const ROD_TIER_COUNT: usize = 7;

/// One energizing rod tier's numbers: `energizingCapacity` and
/// `energizingTransfer` at that tier (`config/v2/DefaultEnergies`, both derived
/// from `baseScaling` — `× 10_000` and `× 100`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RodTier {
    pub capacity: i32,
    pub transfer: i32,
}

/// The energizing rod tier at `index` (0 = starter … 6 = nitro), `None` past the
/// ladder.
#[must_use]
pub fn rod_tier(index: usize) -> Option<RodTier> {
    /// `DefaultEnergies.baseScaling`, in `Tier.getNormalVariants` order.
    const BASE_SCALING: [i32; ROD_TIER_COUNT] = [1, 4, 10, 40, 100, 400, 2_000];
    /// `energizingTransfer` = `baseScaling * 100`.
    const TRANSFER_PER_SCALING: i32 = 100;
    /// `energizingCapacity` = `energizingTransfer * 100`.
    const CAPACITY_PER_SCALING: i32 = 10_000;
    let scaling = *BASE_SCALING.get(index)?;
    Some(RodTier {
        capacity: scaling * CAPACITY_PER_SCALING,
        transfer: scaling * TRANSFER_PER_SCALING,
    })
}

/// One energizing recipe, as the jar's `data/powah/recipe/energizing/*.json`
/// declare it: the result, the energy the orb's buffer must reach, and the
/// ingredients the six input slots must hold.
///
/// An ingredient is the list of item ids one jar ingredient accepts. The jar
/// writes most of them as NeoForge `c:` tags, which 26.3 has no equivalent of;
/// each is carried as the vanilla member(s) it resolves to (see `notes.md`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EnergizingRecipe {
    pub output: &'static str,
    pub count: i32,
    pub energy: i32,
    pub ingredients: &'static [&'static [&'static str]],
}

/// The energizing recipes this port carries, in the jar's own order: steel, the
/// one blaze-powder and the one blaze-rod blazing crystal, niotic, spirited and
/// the nitro crystal, then the ender core slice 11's recipes are built on.
#[must_use]
pub fn energizing() -> &'static [EnergizingRecipe] {
    /// The jar's `data/powah/recipe/energizing/*.json`. Each jar ingredient is a
    /// NeoForge `c:` tag (or an item); 26.3 has no `c:` tags, so the list is the
    /// vanilla member(s) the tag resolves to (see `notes.md`, slice 5).
    static RECIPES: [EnergizingRecipe; 7] = [
        EnergizingRecipe {
            output: "powah:steel_energized",
            count: 2,
            energy: 10_000,
            // `c:ingots/iron`, `c:ingots/gold`
            ingredients: &[&["minecraft:iron_ingot"], &["minecraft:gold_ingot"]],
        },
        EnergizingRecipe {
            output: "powah:crystal_blazing",
            count: 1,
            energy: 120_000,
            // four `minecraft:blaze_powder`
            ingredients: &[
                &["minecraft:blaze_powder"],
                &["minecraft:blaze_powder"],
                &["minecraft:blaze_powder"],
                &["minecraft:blaze_powder"],
            ],
        },
        EnergizingRecipe {
            output: "powah:crystal_blazing",
            count: 1,
            energy: 120_000,
            // `c:rods/blaze`
            ingredients: &[&["minecraft:blaze_rod"]],
        },
        EnergizingRecipe {
            output: "powah:crystal_niotic",
            count: 1,
            energy: 300_000,
            // `c:gems/diamond`
            ingredients: &[&["minecraft:diamond"]],
        },
        EnergizingRecipe {
            output: "powah:crystal_spirited",
            count: 1,
            energy: 1_000_000,
            // `c:gems/emerald`
            ingredients: &[&["minecraft:emerald"]],
        },
        EnergizingRecipe {
            output: "powah:crystal_nitro",
            count: 16,
            energy: 20_000_000,
            // `c:nether_stars`, two `c:storage_blocks/redstone`, a blazing crystal block
            ingredients: &[
                &["minecraft:nether_star"],
                &["minecraft:redstone_block"],
                &["minecraft:redstone_block"],
                &["powah:blazing_crystal_block"],
            ],
        },
        EnergizingRecipe {
            // `data/powah/recipe/energizing/ender_core.json`: the cell and gate ladders' core.
            output: "powah:ender_core",
            count: 1,
            energy: 50_000,
            ingredients: &[
                &["minecraft:ender_eye"],
                &["powah:dielectric_casing"],
                &["powah:capacitor_basic_tiny"],
            ],
        },
    ];
    &RECIPES
}

impl EnergizingRecipe {
    /// Upstream's `EnergizingRecipe.matches`: each filled input slot takes one
    /// ingredient, in any order, and every ingredient must be used. A stack of
    /// four blaze powder in one slot is *one* filled slot, as the jar's
    /// slot-limit-1 inputs make it.
    #[must_use]
    pub fn matches(&self, inputs: &[&str]) -> bool {
        let mut used = [false; MAX_INGREDIENTS];
        if inputs.len() != self.ingredients.len() || inputs.len() > MAX_INGREDIENTS {
            return false;
        }
        for input in inputs {
            let mut taken = false;
            for (index, ingredient) in self.ingredients.iter().enumerate() {
                if !used[index] && ingredient.contains(input) {
                    used[index] = true;
                    taken = true;
                    break;
                }
            }
            if !taken {
                return false;
            }
        }
        true
    }
}

/// The recipe the orb's filled input slots match, `None` when they match none.
#[must_use]
pub fn energize(inputs: &[&str]) -> Option<&'static EnergizingRecipe> {
    energizing().iter().find(|recipe| recipe.matches(inputs))
}

/// What one orb tick decides: the recipe its inputs match, the buffer it holds
/// afterwards (0 when it completes or clears) and whether the recipe is finished.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OrbStep {
    pub recipe: Option<&'static EnergizingRecipe>,
    pub energy: i32,
    pub done: bool,
}

/// The orb's rule (`EnergizingOrbTile.onSlotChanged` + `fillEnergy`): a matching
/// recipe sets the buffer's capacity to *its* energy, a different recipe clears
/// it (`onSlotChanged` zeroes the buffer), and a buffer that reached the capacity
/// produces the result. `recorded` is the recipe output the orb last matched;
/// `None` adopts the energy a rod already moved in (upstream has no such window,
/// but a rod may fill on the tick the inputs arrive, and that energy is the
/// recipe's, not another's).
///
/// A buffer above the recipe's energy is `done`, never clamped down to it: the
/// card only ever takes room up to the capacity, so a higher value can only come
/// from a harness `set`.
#[must_use]
pub fn orb_step(inputs: &[&str], energy: i32, recorded: Option<&str>) -> OrbStep {
    let Some(recipe) = energize(inputs) else {
        return OrbStep {
            recipe: None,
            energy: 0,
            done: false,
        };
    };
    let changed_recipe = recorded.is_some_and(|output| output != recipe.output);
    let held = if changed_recipe {
        0
    } else {
        clamp_energy(energy, recipe.energy)
    };
    let done = held >= recipe.energy;
    OrbStep {
        recipe: Some(recipe),
        energy: if done { 0 } else { held },
        done,
    }
}

/// What one rod tick moves into its orb, from the orb's own formula
/// (`EnergizingOrbTile.fillEnergy`): the room left in the orb, capped by what the
/// rod holds and by the rod tier's transfer.
#[must_use]
pub fn orb_fill(rod_stored: i32, rod_transfer: i32, orb_stored: i32, capacity: i32) -> i32 {
    let room = (capacity.max(0) - orb_stored.max(0)).max(0);
    room.min(rod_stored.max(0).min(rod_transfer.max(0)))
}

/// One rod tick's warm-up (`EnergizingRodTile`'s `Ticker(20)`): `charging` is
/// true while the orb has a recipe *and* the rod holds energy. The counter walks
/// up to [`ROD_WARMUP`] and then stays there, which is upstream's behaviour: the
/// warm-up delays the first fill by 20 ticks, it does not throttle later ones.
#[must_use]
pub fn rod_cooldown(charging: bool, cooldown: i32) -> i32 {
    if charging {
        (cooldown.max(0) + 1).min(ROD_WARMUP)
    } else {
        (cooldown - 1).max(0)
    }
}

/// Upstream's `Redstone` (`lib/logistics/Redstone.java`): the three modes a machine's GUI button
/// cycles through, in the enum's own ordinal order, which is what the block entity saves under
/// `redstone_mode`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RedstoneMode {
    /// No gate: the machine always runs (upstream's default, `Redstone.IGNORE`).
    #[default]
    Ignore,
    /// The machine runs only while a neighbour powers it (`Redstone.ON`).
    On,
    /// The machine runs only while no neighbour powers it (`Redstone.OFF`).
    Off,
}

impl RedstoneMode {
    /// The three modes in `Redstone.values()` order, which is the ordinal order.
    pub const ALL: [RedstoneMode; 3] = [Self::Ignore, Self::On, Self::Off];

    /// The ordinal upstream saves (`writeSync`'s `redstone_mode`).
    #[must_use]
    pub fn ordinal(self) -> i32 {
        match self {
            Self::Ignore => 0,
            Self::On => 1,
            Self::Off => 2,
        }
    }

    /// The mode an ordinal names. Upstream indexes `Redstone.values()` directly and would throw
    /// on a doctored value; a value outside the three reads as the default here.
    #[must_use]
    pub fn from_ordinal(value: i32) -> RedstoneMode {
        Self::ALL
            .into_iter()
            .find(|mode| mode.ordinal() == value)
            .unwrap_or_default()
    }

    /// The mode's name as this port's readout spells it: upstream's lang keys are
    /// `info.lollipop.ignore`/`on`/`off`, named after `Redstone.IGNORE` and friends.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Ignore => "ignore",
            Self::On => "on",
            Self::Off => "off",
        }
    }

    /// The word the `/powah redstone` usage uses for this mode: `ignored` for the default (upstream's
    /// enum is `IGNORE`, and a command reads as English) and the mode's own name otherwise. `ignore`
    /// is accepted beside it, because that is the spelling `name` gives the readout.
    #[must_use]
    pub fn command_word(self) -> &'static str {
        match self {
            Self::Ignore => "ignored",
            other => other.name(),
        }
    }

    /// The mode a command word names, `None` for anything else. `ignored` is accepted beside
    /// `ignore`, because the command reads better that way and both name the same mode.
    #[must_use]
    pub fn named(name: &str) -> Option<RedstoneMode> {
        match name {
            "ignore" | "ignored" => Some(Self::Ignore),
            "on" => Some(Self::On),
            "off" => Some(Self::Off),
            _ => None,
        }
    }

    /// The next mode, as the GUI button's `NextRedstoneModePacket` cycles: `IGNORE` → `ON` →
    /// `OFF` → `IGNORE`.
    #[must_use]
    pub fn next(self) -> RedstoneMode {
        match self {
            Self::Ignore => Self::On,
            Self::On => Self::Off,
            Self::Off => Self::Ignore,
        }
    }
}

/// `AbstractTileEntity.checkRedstone()`: whether the machine does its work this tick, from its
/// mode and whether any neighbour powers it (`level.getBestNeighborSignal(pos) > 0`).
#[must_use]
pub fn check_redstone(mode: RedstoneMode, powered: bool) -> bool {
    match mode {
        // Upstream returns true before it ever reads the signal: IGNORE is the cheap default.
        RedstoneMode::Ignore => true,
        RedstoneMode::On => powered,
        RedstoneMode::Off => !powered,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_three_redstone_modes_are_upstreams_ordinals_and_cycle_in_its_order() {
        // `Redstone.java`: IGNORE, ON, OFF — the ordinal is the saved `redstone_mode`.
        let expected = [
            (RedstoneMode::Ignore, 0, "ignore"),
            (RedstoneMode::On, 1, "on"),
            (RedstoneMode::Off, 2, "off"),
        ];
        for (mode, ordinal, name) in expected {
            assert_eq!(mode.ordinal(), ordinal, "{mode:?}'s saved ordinal");
            assert_eq!(
                RedstoneMode::from_ordinal(ordinal),
                mode,
                "ordinal {ordinal} names {mode:?}"
            );
            assert_eq!(mode.name(), name, "{mode:?}'s name");
        }
        assert_eq!(
            RedstoneMode::from_ordinal(3),
            RedstoneMode::Ignore,
            "a doctored ordinal above the three is the default, not a panic"
        );
        assert_eq!(
            RedstoneMode::from_ordinal(-1),
            RedstoneMode::Ignore,
            "a negative ordinal is the default"
        );
        // `next()`: ordinal + 1, wrapping past OFF back to IGNORE.
        assert_eq!(RedstoneMode::Ignore.next(), RedstoneMode::On);
        assert_eq!(RedstoneMode::On.next(), RedstoneMode::Off);
        assert_eq!(RedstoneMode::Off.next(), RedstoneMode::Ignore);
        // The command's words are the names, `ignored` included.
        assert_eq!(RedstoneMode::named("ignored"), Some(RedstoneMode::Ignore));
        assert_eq!(RedstoneMode::named("on"), Some(RedstoneMode::On));
        assert_eq!(RedstoneMode::named("off"), Some(RedstoneMode::Off));
        assert_eq!(RedstoneMode::named("ignore"), Some(RedstoneMode::Ignore));
        assert_eq!(RedstoneMode::named("ON"), None, "the words are lower case");
        assert_eq!(RedstoneMode::named("else"), None);
        // The usage's word is `ignored` (the command reads as English); `name` stays upstream's
        // `ignore` for the readout, and both parse.
        assert_eq!(RedstoneMode::Ignore.command_word(), "ignored");
        assert_eq!(RedstoneMode::On.command_word(), "on");
        assert_eq!(RedstoneMode::Off.command_word(), "off");
        assert_eq!(RedstoneMode::default(), RedstoneMode::Ignore);
        assert_eq!(RedstoneMode::ALL.len(), 3, "upstream has exactly three");
    }

    #[test]
    fn a_machine_runs_unless_its_mode_names_the_other_power_state() {
        // `checkRedstone`: IGNORE is always open; ON needs power; OFF needs no power.
        assert!(check_redstone(RedstoneMode::Ignore, false));
        assert!(check_redstone(RedstoneMode::Ignore, true));
        assert!(
            !check_redstone(RedstoneMode::On, false),
            "ON with no power is shut"
        );
        assert!(
            check_redstone(RedstoneMode::On, true),
            "ON with power is open"
        );
        assert!(
            !check_redstone(RedstoneMode::Off, true),
            "OFF with power is shut"
        );
        assert!(
            check_redstone(RedstoneMode::Off, false),
            "OFF with no power is open"
        );
    }

    fn buffer(energy: i32, capacity: i32, max_insert: i32, max_extract: i32) -> Neighbour {
        Neighbour {
            energy,
            capacity,
            max_insert,
            max_extract,
            can_insert: max_insert > 0,
            can_extract: max_extract > 0,
        }
    }

    fn empty() -> [Option<Neighbour>; 6] {
        [None; 6]
    }

    /// The starter tier, the one slice 1's numbers are (`FURNATOR_*`).
    fn starter_furnator() -> FurnatorTier {
        furnator_tier(0).expect("the starter tier is the first of the ladder")
    }

    #[test]
    fn every_furnator_tier_scales_upstreams_numbers() {
        // `DefaultEnergies`: activeProduction = baseScaling * 20,
        // generatorCapacity = activeProduction * 1000,
        // generatorTransfer = activeProduction * 4.
        let expected = [
            (20, 20_000, 80),
            (80, 80_000, 320),
            (200, 200_000, 800),
            (800, 800_000, 3_200),
            (2_000, 2_000_000, 8_000),
            (8_000, 8_000_000, 32_000),
            (40_000, 40_000_000, 160_000),
        ];
        for (index, (generation, capacity, transfer)) in expected.into_iter().enumerate() {
            let tier =
                furnator_tier(index).unwrap_or_else(|| panic!("tier {index} is on the ladder"));
            assert_eq!(
                (tier.generation, tier.capacity, tier.transfer),
                (generation, capacity, transfer),
                "furnator tier {index}'s numbers"
            );
        }
        assert_eq!(
            furnator_tier(FURNATOR_TIER_COUNT),
            None,
            "there is no eighth furnator tier"
        );
    }

    #[test]
    fn the_starter_furnator_keeps_slice_1_s_numbers() {
        assert_eq!(
            (FURNATOR_GENERATION, FURNATOR_CAPACITY, FURNATOR_TRANSFER),
            (20, 20_000, 80),
            "slice 1's constants are the starter tier's row"
        );
        let starter = starter_furnator();
        assert_eq!(
            (starter.generation, starter.capacity, starter.transfer),
            (FURNATOR_GENERATION, FURNATOR_CAPACITY, FURNATOR_TRANSFER)
        );
    }

    #[test]
    fn every_cable_tier_scales_upstreams_numbers() {
        // `DefaultEnergies`: cableTransfer = reactorProduction * 2 = baseScaling * 500.
        let expected = [500, 2_000, 5_000, 20_000, 50_000, 200_000, 1_000_000];
        for (index, transfer) in expected.into_iter().enumerate() {
            let tier = cable_tier(index).unwrap_or_else(|| panic!("tier {index} is on the ladder"));
            assert_eq!(tier.transfer, transfer, "cable tier {index}'s rate");
        }
        assert_eq!(
            cable_tier(CABLE_TIER_COUNT),
            None,
            "there is no eighth cable tier"
        );
        assert_eq!(
            cable_tier(0).expect("the starter cable").transfer,
            CABLE_TRANSFER,
            "slice 1's cable rate is the starter tier's row"
        );
    }

    #[test]
    fn a_nitro_furnator_burns_one_coal_in_two_ticks() {
        let nitro = furnator_tier(6).expect("the nitro tier");
        let mut furnator = Furnator::new();
        assert!(furnator.load_fuel(1_600), "one coal is 48,000 carbon");
        assert_eq!(furnator.carbon, 48_000);
        assert_eq!(
            furnator.tick(nitro),
            40_000,
            "the nitro tier generates its own 40,000 in one tick"
        );
        assert_eq!(
            furnator.carbon, 8_000,
            "and takes exactly that off the carbon"
        );
        assert_eq!(furnator.tick(nitro), 8_000, "the rest of the coal");
        assert_eq!(furnator.carbon, 0);
        assert_eq!(furnator.tick(nitro), 0, "one coal is spent in two ticks");
    }

    #[test]
    fn every_energy_cell_tier_scales_upstreams_battery_numbers() {
        // `DefaultEnergies`: baseScaling = 1, 4, 10, 40, 100, 400, 2000;
        // batteryCapacity = reactorProduction * 4000 = baseScaling * 1_000_000 and
        // batteryTransfer = cableTransfer * 2 = reactorProduction * 4 = baseScaling * 1_000.
        let expected = [
            (1_000_000, 1_000),
            (4_000_000, 4_000),
            (10_000_000, 10_000),
            (40_000_000, 40_000),
            (100_000_000, 100_000),
            (400_000_000, 400_000),
            (2_000_000_000, 2_000_000),
        ];
        for (index, (capacity, transfer)) in expected.into_iter().enumerate() {
            let tier = cell_tier(index).unwrap_or_else(|| panic!("tier {index} is on the ladder"));
            assert_eq!(
                (tier.capacity, tier.transfer),
                (capacity, transfer),
                "tier {index}'s numbers"
            );
        }
        assert_eq!(
            cell_tier(CELL_TIER_COUNT),
            None,
            "the creative tier is not a storage tier"
        );
    }

    #[test]
    fn the_starter_cell_keeps_slice_1_s_numbers() {
        let starter = cell_tier(0).expect("the starter tier is the first of the ladder");
        assert_eq!(
            (starter.capacity, starter.transfer),
            (CELL_CAPACITY, CELL_TRANSFER)
        );
    }

    #[test]
    fn energy_is_clamped_into_the_buffer() {
        assert_eq!(clamp_energy(-5, 100), 0);
        assert_eq!(clamp_energy(50, 100), 50);
        assert_eq!(clamp_energy(500, 100), 100);
        assert_eq!(clamp_energy(5, 0), 0);
    }

    #[test]
    fn the_comparator_reads_the_fraction_of_the_buffer() {
        assert_eq!(comparator_strength(0, CELL_CAPACITY), 0);
        assert_eq!(comparator_strength(CELL_CAPACITY, CELL_CAPACITY), 15);
        assert_eq!(comparator_strength(CELL_CAPACITY / 2, CELL_CAPACITY), 7);
        assert_eq!(comparator_strength(5, 0), 0);
    }

    #[test]
    fn the_orb_comparator_counts_non_empty_stacks_capped_at_15() {
        assert_eq!(stack_comparator(0), 0);
        assert_eq!(stack_comparator(3), 3);
        assert_eq!(stack_comparator(7), 7);
        assert_eq!(stack_comparator(20), 15);
    }

    #[test]
    fn a_rods_comparator_is_its_own_buffer_fraction_truncated() {
        // Upstream's rod is an `AbstractEnergyStorage`: (int)(stored / capacity * 15).
        assert_eq!(comparator_strength(5_000, 10_000), 7);
        assert_eq!(comparator_strength(10_000, 10_000), 15);
        assert_eq!(comparator_strength(0, 10_000), 0);
    }

    #[test]
    fn a_push_moves_at_most_the_transfer_and_never_more_than_is_stored() {
        assert_eq!(push_amount(10, 80), 10);
        assert_eq!(push_amount(1_000, 80), 80);
        assert_eq!(push_amount(0, 80), 0);
        assert_eq!(push_amount(-1, 80), 0);
    }

    #[test]
    fn a_furnator_with_no_fuel_makes_nothing() {
        let mut furnator = Furnator::new();
        assert_eq!(furnator.tick(starter_furnator()), 0);
        assert_eq!(furnator.energy, 0);
        assert!(!furnator.burning());
    }

    #[test]
    fn one_coal_is_thirty_per_burn_tick_of_carbon() {
        let mut furnator = Furnator::new();
        assert!(furnator.load_fuel(1_600));
        assert_eq!(furnator.carbon, 48_000);
        assert_eq!(furnator.tick(starter_furnator()), FURNATOR_GENERATION);
        assert_eq!(furnator.energy, 20);
        assert_eq!(furnator.carbon, 47_980);
    }

    #[test]
    fn one_coal_yields_forty_eight_thousand_fe_over_two_thousand_four_hundred_ticks() {
        let mut furnator = Furnator::new();
        assert!(furnator.load_fuel(1_600));
        let mut ticks = 0;
        while furnator.burning() {
            // A cell next to it keeps taking the energy, so the buffer never fills.
            furnator.energy = 0;
            assert_eq!(furnator.tick(starter_furnator()), FURNATOR_GENERATION);
            ticks += 1;
        }
        assert_eq!(ticks, 2_400, "48,000 carbon at 20 FE/t");
        assert_eq!(furnator.carbon, 0);
    }

    #[test]
    fn a_full_buffer_pauses_production_without_burning_carbon() {
        let mut furnator = Furnator {
            energy: FURNATOR_CAPACITY,
            carbon: 100,
        };
        assert_eq!(furnator.tick(starter_furnator()), 0);
        assert_eq!(furnator.carbon, 100);
        assert_eq!(furnator.energy, FURNATOR_CAPACITY);

        let mut nearly = Furnator {
            energy: FURNATOR_CAPACITY - 5,
            carbon: 100,
        };
        assert_eq!(nearly.tick(starter_furnator()), 5);
        assert_eq!(nearly.energy, FURNATOR_CAPACITY);
        assert_eq!(
            nearly.carbon, 95,
            "only the stored energy comes off the carbon"
        );
    }

    #[test]
    fn fuel_is_only_taken_when_no_carbon_is_left() {
        let mut furnator = Furnator::new();
        assert!(furnator.wants_fuel());
        assert!(!furnator.load_fuel(0), "not a fuel");
        assert!(furnator.load_fuel(200));
        assert!(!furnator.wants_fuel(), "still burning");
        assert!(!furnator.load_fuel(1_600), "one fuel item at a time");

        // Upstream's refuel block does not look at the buffer: fuel becomes
        // carbon and burns once there is room for the energy.
        let mut full = Furnator {
            energy: FURNATOR_CAPACITY,
            carbon: 0,
        };
        assert!(full.wants_fuel(), "a full buffer still takes the next fuel");
        assert!(full.load_fuel(1_600));
        assert_eq!(full.carbon, 48_000);
        assert_eq!(
            full.tick(starter_furnator()),
            0,
            "no room, so nothing is produced"
        );
        assert_eq!(full.carbon, 48_000, "and no carbon is burnt");
    }

    #[test]
    fn a_cable_moves_energy_from_a_source_to_a_sink_up_to_its_rate() {
        let mut sides = empty();
        sides[0] = Some(buffer(10_000, 20_000, 0, 80)); // furnator below: 80 a call
        sides[1] = Some(buffer(0, 1_000_000, 1_000, 1_000)); // cell above
        assert_eq!(
            cable_moves(&sides, CABLE_TRANSFER, 0),
            vec![Transfer {
                from: 0,
                to: 1,
                amount: 80
            }],
            "the furnator gives at most its own 80 a call"
        );

        let mut cell = empty();
        cell[0] = Some(buffer(10_000, 1_000_000, 1_000, 1_000)); // a cell gives 1,000
        cell[1] = Some(buffer(0, 1_000_000, 1_000, 1_000));
        assert_eq!(
            cable_moves(&cell, CABLE_TRANSFER, 0),
            vec![Transfer {
                from: 0,
                to: 1,
                amount: 500
            }],
            "a cell can give more, so the cable's own 500 caps it"
        );
    }

    #[test]
    fn a_cable_stops_at_the_sink_room_the_source_and_the_per_call_limit() {
        let mut sides = empty();
        sides[0] = Some(buffer(10, 20_000, 0, 80)); // 10 FE only
        sides[1] = Some(buffer(999_990, 1_000_000, 1_000, 1_000)); // 10 room
        assert_eq!(
            cable_moves(&sides, CABLE_TRANSFER, 0),
            vec![Transfer {
                from: 0,
                to: 1,
                amount: 10
            }],
            "min(source 10, room 10, transfer 500)"
        );

        let mut limited = empty();
        limited[0] = Some(buffer(10_000, 20_000, 0, 40)); // 40 per call
        limited[1] = Some(buffer(0, 1_000_000, 30, 1_000)); // 30 per call
        assert_eq!(
            cable_moves(&limited, CABLE_TRANSFER, 0),
            vec![Transfer {
                from: 0,
                to: 1,
                amount: 30
            }]
        );
    }

    #[test]
    fn a_cable_with_no_source_or_no_sink_moves_nothing() {
        let mut sink_only = empty();
        sink_only[2] = Some(buffer(0, 100, 100, 0));
        assert!(cable_moves(&sink_only, CABLE_TRANSFER, 0).is_empty());

        let mut source_only = empty();
        source_only[2] = Some(buffer(100, 100, 0, 100));
        assert!(cable_moves(&source_only, CABLE_TRANSFER, 0).is_empty());

        assert!(cable_moves(&empty(), CABLE_TRANSFER, 0).is_empty());
    }

    #[test]
    fn a_move_never_starts_and_ends_at_the_same_neighbour() {
        let mut sides = empty();
        sides[0] = Some(buffer(5_000, 10_000, 500, 500)); // a full battery on one side
        let moves = cable_moves(&sides, CABLE_TRANSFER, 0);
        assert!(
            moves.is_empty(),
            "one battery cannot charge itself: {moves:?}"
        );
    }

    #[test]
    fn two_sources_share_the_cable_by_the_rotation_and_the_budget() {
        let mut sides = empty();
        sides[0] = Some(buffer(10_000, 20_000, 0, 500));
        sides[1] = Some(buffer(10_000, 20_000, 0, 500));
        sides[2] = Some(buffer(0, 1_000_000, 1_000, 0));
        let moves = cable_moves(&sides, 600, 0);
        let total: i32 = moves.iter().map(|m| m.amount).sum();
        assert_eq!(total, 600, "the cable rate caps both sources together");
        assert_eq!(moves[0].from, 0, "the first move starts at side 0");
        assert_eq!(moves[0].amount, 500);

        let rotated = cable_moves(&sides, 600, 1);
        assert_eq!(
            rotated[0].from, 1,
            "side 1 goes first when the rotation starts there"
        );
    }

    #[test]
    fn a_network_relays_across_the_cables_between_the_source_and_the_sink() {
        let ports = [
            buffer(1_000, 1_000_000, 1_000, 1_000),
            buffer(0, 1_000_000, 1_000, 1_000),
        ];
        assert_eq!(
            network_moves(&ports, CABLE_TRANSFER, 0),
            vec![Transfer {
                from: 0,
                to: 1,
                amount: CABLE_TRANSFER
            }],
            "a source and a sink on two different cables of one network"
        );
    }

    #[test]
    fn a_network_spends_one_cable_s_rate_not_a_rate_per_cable() {
        let ports = [
            buffer(10_000, 20_000, 0, 500),
            buffer(10_000, 20_000, 0, 500),
            buffer(0, 1_000_000, 1_000, 0),
        ];
        let moves = network_moves(&ports, CABLE_TRANSFER, 0);
        let total: i32 = moves.iter().map(|m| m.amount).sum();
        assert_eq!(total, CABLE_TRANSFER, "the network is one relay");
        assert_eq!(
            moves[0],
            Transfer {
                from: 0,
                to: 2,
                amount: CABLE_TRANSFER
            },
            "the first source spends the whole budget"
        );
    }

    #[test]
    fn a_player_slot_number_is_the_item_places_not_the_inventorys() {
        // `Player.invStacks` walks `Inventory`'s numbers; `ItemPlace::PlayerSlot` takes
        // `ItemAccess.forPlayerSlot`'s: the main inventory as itself, the armour 100..=103 and the
        // off hand -106.
        assert_eq!(player_slot(0), Some(0), "hotbar slot 0 is the same number");
        assert_eq!(player_slot(8), Some(8), "hotbar slot 8 is the same number");
        assert_eq!(player_slot(9), Some(9), "the main inventory continues at 9");
        assert_eq!(player_slot(35), Some(35), "the last main slot");
        assert_eq!(
            player_slot(36),
            Some(100),
            "Inventory's 36 (feet) is the item place's 100"
        );
        assert_eq!(
            player_slot(39),
            Some(103),
            "Inventory's 39 (head) is the item place's 103"
        );
        assert_eq!(
            player_slot(40),
            Some(-106),
            "Inventory's 40 (off hand) is -106"
        );
        assert_eq!(player_slot(41), None, "there is no 42nd slot");
        assert_eq!(
            (0..PLAYER_INVENTORY_SLOTS).filter_map(player_slot).count(),
            PLAYER_INVENTORY_SLOTS,
            "every slot `Player.invStacks` walks has an item place number"
        );
    }

    #[test]
    fn a_network_equalizes_two_storages_and_never_takes_back_what_it_gave() {
        // Two energy cells on one cable are both storages: each answers an extract (1,000 a call)
        // and an insert (1,000 a call), so a plan that rotates its start lets the receiver give
        // the same 500 back on the next tick and the pair ping-pongs instead of equalizing
        // (measured 2026-11-23: the slice-3 leg read 0 or 500 depending on which tick the read
        // landed on). Upstream's tiles give through `extractFromSides` into a neighbour's
        // `receiveEnergy`, so a network only ever runs from the fuller store to the emptier one.
        let full = buffer(100_000, 1_000_000, 1_000, 1_000);
        let empty_cell = buffer(0, 1_000_000, 1_000, 1_000);
        assert_eq!(
            network_moves(&[full, empty_cell], 500, 0),
            vec![Transfer {
                from: 0,
                to: 1,
                amount: 500
            }],
            "the fuller storage gives the emptier its step"
        );
        let after = [
            buffer(99_500, 1_000_000, 1_000, 1_000),
            buffer(500, 1_000_000, 1_000, 1_000),
        ];
        assert_eq!(
            network_moves(&after, 500, 1),
            vec![Transfer {
                from: 0,
                to: 1,
                amount: 500
            }],
            "the rotation starts at the receiver, and the receiver still does not give back: \
             the fuller store is the one that moves"
        );
        assert_eq!(
            network_moves(&after, 500, 0),
            vec![Transfer {
                from: 0,
                to: 1,
                amount: 500
            }],
            "the fuller storage keeps giving until the two are level"
        );
        let level = [
            buffer(500, 1_000_000, 1_000, 1_000),
            buffer(500, 1_000_000, 1_000, 1_000),
        ];
        assert_eq!(
            network_moves(&level, 500, 0),
            Vec::new(),
            "two level storages have nothing to move"
        );
        // A generator is not a storage: its buffer is filled by its own tick, so it gives whatever
        // it holds, however little (a furnator holding 20 FE into a cell holding 500).
        let generator = buffer(20, 20_000, 0, 80);
        assert_eq!(
            network_moves(&[generator, buffer(500, 1_000_000, 1_000, 1_000)], 500, 1),
            vec![Transfer {
                from: 0,
                to: 1,
                amount: 20
            }],
            "a provider gives its whole buffer, fuller or not"
        );
    }

    #[test]
    fn one_cable_is_the_same_plan_as_its_six_sides() {
        let mut sides = empty();
        sides[0] = Some(buffer(1_000, 20_000, 0, 80));
        sides[1] = Some(buffer(0, 1_000_000, 1_000, 1_000));
        let ports = [sides[0].unwrap(), sides[1].unwrap()];
        let six = cable_moves(&sides, CABLE_TRANSFER, 0);
        let net = network_moves(&ports, CABLE_TRANSFER, 0);
        assert_eq!(six.len(), 1, "one move, as slice 1's cable made");
        assert_eq!(net.len(), 1);
        assert_eq!(six[0].amount, net[0].amount);
        assert_eq!(six[0].amount, 80, "the source's own per-call limit");

        let mut shared = empty();
        shared[0] = Some(buffer(10_000, 20_000, 0, 500));
        shared[1] = Some(buffer(10_000, 20_000, 0, 500));
        shared[2] = Some(buffer(0, 1_000_000, 1_000, 0));
        let ports = [shared[0].unwrap(), shared[1].unwrap(), shared[2].unwrap()];
        let six = cable_moves(&shared, CABLE_TRANSFER, 1);
        let net = network_moves(&ports, CABLE_TRANSFER, 1);
        assert_eq!(six[0].from, 1);
        assert_eq!(net[0].from, 1, "the rotation moves the same way");
        assert_eq!(six[0].amount, net[0].amount);
    }

    #[test]
    fn a_network_without_a_giver_or_a_taker_moves_nothing() {
        assert!(
            network_moves(&[], CABLE_TRANSFER, 0).is_empty(),
            "no port, no move"
        );
        let sink_only = [buffer(0, 100, 100, 0)];
        assert!(
            network_moves(&sink_only, CABLE_TRANSFER, 0).is_empty(),
            "nothing in the network can give"
        );
        let source_only = [buffer(100, 100, 0, 100)];
        assert!(
            network_moves(&source_only, CABLE_TRANSFER, 0).is_empty(),
            "one port cannot give to itself"
        );
        let battery = [buffer(100, 1_000, 100, 100)];
        assert!(
            network_moves(&battery, CABLE_TRANSFER, 0).is_empty(),
            "a battery cannot charge itself"
        );
    }

    #[test]
    fn every_energizing_rod_tier_scales_upstreams_numbers() {
        // `DefaultEnergies`: energizingTransfer = baseScaling * 100 and
        // energizingCapacity = energizingTransfer * 100.
        let expected = [
            (10_000, 100),
            (40_000, 400),
            (100_000, 1_000),
            (400_000, 4_000),
            (1_000_000, 10_000),
            (4_000_000, 40_000),
            (20_000_000, 200_000),
        ];
        for (index, (capacity, transfer)) in expected.into_iter().enumerate() {
            let tier = rod_tier(index).unwrap_or_else(|| panic!("tier {index} is on the ladder"));
            assert_eq!(
                (tier.capacity, tier.transfer),
                (capacity, transfer),
                "rod tier {index}'s numbers"
            );
        }
        assert_eq!(
            rod_tier(ROD_TIER_COUNT),
            None,
            "there is no eighth rod tier"
        );
    }

    #[test]
    fn every_energizing_recipe_is_the_jars_own_row() {
        let by_output: Vec<(&str, i32, i32)> = energizing()
            .iter()
            .map(|recipe| (recipe.output, recipe.count, recipe.energy))
            .collect();
        assert_eq!(
            by_output,
            vec![
                ("powah:steel_energized", 2, 10_000),
                ("powah:crystal_blazing", 1, 120_000),
                ("powah:crystal_blazing", 1, 120_000),
                ("powah:crystal_niotic", 1, 300_000),
                ("powah:crystal_spirited", 1, 1_000_000),
                ("powah:crystal_nitro", 16, 20_000_000),
                ("powah:ender_core", 1, 50_000),
            ],
            "the jar's five materials, its second blazing variant (one blaze rod) and the core \
             slice 11's ender recipes are built on"
        );
        let nitro = energizing()
            .iter()
            .find(|recipe| recipe.output == "powah:crystal_nitro")
            .expect("the nitro recipe");
        assert_eq!(
            nitro.ingredients.len(),
            4,
            "a nether star, two redstone blocks and a blazing crystal block"
        );
    }

    #[test]
    fn an_orb_matches_its_recipe_in_any_slot_order_and_nothing_else() {
        let steel = energize(&["minecraft:iron_ingot", "minecraft:gold_ingot"])
            .expect("iron and gold are the energized steel recipe");
        assert_eq!(steel.output, "powah:steel_energized");
        assert_eq!(
            energize(&["minecraft:gold_ingot", "minecraft:iron_ingot"]),
            Some(steel),
            "the slots are a bag of ingredients, not a pattern"
        );
        assert_eq!(energize(&["minecraft:iron_ingot"]), None, "half a recipe");
        assert_eq!(
            energize(&["minecraft:iron_ingot", "minecraft:iron_ingot"]),
            None,
            "two of one, none of the other"
        );
        assert_eq!(
            energize(&["minecraft:iron_ingot", "minecraft:diamond"]),
            None,
            "one wrong ingredient"
        );
        assert_eq!(energize(&[]), None, "an empty orb has no recipe");
    }

    #[test]
    fn a_stack_of_blaze_powder_in_one_slot_is_one_filled_slot() {
        // Upstream's input slots hold one item each, so four blaze powder are four
        // slots; a single stack is one filled slot and matches one ingredient only.
        assert_eq!(
            energize(&["minecraft:blaze_powder"]),
            None,
            "one powder is a quarter of the blazing recipe"
        );
        let four = [
            "minecraft:blaze_powder",
            "minecraft:blaze_powder",
            "minecraft:blaze_powder",
            "minecraft:blaze_powder",
        ];
        assert_eq!(
            energize(&four).map(|recipe| recipe.output),
            Some("powah:crystal_blazing")
        );
        assert_eq!(
            energize(&["minecraft:blaze_rod"]).map(|recipe| recipe.output),
            Some("powah:crystal_blazing"),
            "the jar's second blazing recipe takes one blaze rod"
        );
    }

    #[test]
    fn the_orb_holds_the_recipes_energy_clears_a_changed_recipe_and_produces_when_full() {
        let steel = energizing()
            .iter()
            .find(|recipe| recipe.output == "powah:steel_energized")
            .expect("the steel recipe");
        let iron_gold = ["minecraft:iron_ingot", "minecraft:gold_ingot"];

        assert_eq!(
            orb_step(&iron_gold, 0, None),
            OrbStep {
                recipe: Some(steel),
                energy: 0,
                done: false
            },
            "a fresh orb holds the recipe and no energy"
        );
        assert_eq!(
            orb_step(&iron_gold, 5_000, Some("powah:steel_energized")).energy,
            5_000,
            "a rod's fill stays in a buffer under the recipe's 10,000"
        );
        assert_eq!(
            orb_step(&iron_gold, 5_000, None).energy,
            5_000,
            "energy a rod moved in before the orb recorded a recipe is adopted, not thrown away"
        );
        assert_eq!(
            orb_step(&iron_gold, 10_000, Some("powah:steel_energized")),
            OrbStep {
                recipe: Some(steel),
                energy: 0,
                done: true
            },
            "at the recipe's energy the orb produces and empties"
        );
        assert_eq!(
            orb_step(&iron_gold, 5_000, Some("powah:crystal_niotic")).energy,
            0,
            "a different recipe clears what the last one stored (onSlotChanged)"
        );
        assert!(
            orb_step(
                &["minecraft:diamond"],
                400_000,
                Some("powah:crystal_niotic")
            )
            .done,
            "a buffer above the recipe's energy is done, not clamped back"
        );
        assert_eq!(
            orb_step(&[], 5_000, None),
            OrbStep {
                recipe: None,
                energy: 0,
                done: false
            },
            "an orb with no recipe stores nothing"
        );
        assert_eq!(
            orb_step(&["minecraft:diamond"], 0, None)
                .recipe
                .map(|recipe| recipe.output),
            Some("powah:crystal_niotic"),
            "one diamond is the niotic recipe"
        );
    }

    #[test]
    fn an_orb_takes_the_room_it_has_and_the_rod_gives_no_more_than_it_holds() {
        assert_eq!(
            orb_fill(200_000, 200_000, 0, 120_000),
            120_000,
            "a nitro rod offers 200,000 into 120,000 of room: the orb takes 120,000"
        );
        assert_eq!(
            orb_fill(10_000, 100, 0, 10_000),
            100,
            "a starter rod gives its own 100 a tick"
        );
        assert_eq!(
            orb_fill(50_000, 200_000, 119_950, 120_000),
            50,
            "the last 50 FE of the recipe"
        );
        assert_eq!(
            orb_fill(0, 200_000, 0, 120_000),
            0,
            "an empty rod gives nothing"
        );
        assert_eq!(
            orb_fill(10_000, 100, 10_000, 10_000),
            0,
            "a full orb has no room"
        );
        assert_eq!(orb_fill(-5, 100, 0, 10_000), 0);
    }

    #[test]
    fn a_rod_warms_up_for_twenty_ticks_and_then_fills_every_tick() {
        let mut cooldown = 0;
        for tick in 1..=ROD_WARMUP {
            cooldown = rod_cooldown(true, cooldown);
            assert_eq!(cooldown, tick, "tick {tick} of the warm-up");
        }
        assert_eq!(
            rod_cooldown(true, cooldown),
            ROD_WARMUP,
            "the warm-up holds at 20: upstream's Ticker.ended() stays true, so a warm rod fills every tick"
        );
        assert_eq!(
            rod_cooldown(false, ROD_WARMUP),
            ROD_WARMUP - 1,
            "an idle rod cools"
        );
        assert_eq!(rod_cooldown(false, 0), 0, "and stops at zero");
    }

    #[test]
    fn the_network_rotation_starts_at_another_port() {
        let ports = [
            buffer(10_000, 20_000, 0, 500),
            buffer(10_000, 20_000, 0, 500),
            buffer(0, 1_000_000, 1_000, 0),
        ];
        assert_eq!(network_moves(&ports, CABLE_TRANSFER, 0)[0].from, 0);
        assert_eq!(network_moves(&ports, CABLE_TRANSFER, 1)[0].from, 1);
        assert_eq!(
            network_moves(&ports, CABLE_TRANSFER, 11)[0].from,
            0,
            "a rotation past the port count wraps: 11 % 3 leaves the sink first"
        );
    }

    #[test]
    fn a_cable_spreads_one_source_over_several_sinks_without_overdrawing() {
        let mut sides = empty();
        sides[0] = Some(buffer(1_000, 20_000, 0, 500));
        sides[1] = Some(buffer(0, 100, 100, 0)); // 100 room
        sides[2] = Some(buffer(0, 100, 100, 0)); // 100 room
        sides[3] = Some(buffer(0, 1_000_000, 1_000, 0));
        let moves = cable_moves(&sides, CABLE_TRANSFER, 0);
        let moved: i32 = moves.iter().map(|m| m.amount).sum();
        assert_eq!(moved, 500, "the cable carries 500 in total");
        assert_eq!(
            moves[0],
            Transfer {
                from: 0,
                to: 1,
                amount: 100
            }
        );
        assert_eq!(
            moves[1],
            Transfer {
                from: 0,
                to: 2,
                amount: 100
            }
        );
        assert_eq!(
            moves[2],
            Transfer {
                from: 0,
                to: 3,
                amount: 300
            }
        );
    }

    #[test]
    fn a_charge_step_moves_the_transfer_the_balance_or_the_room() {
        assert_eq!(
            charge_step(40, 80, 1_000_000),
            40,
            "the balance limits the step when it is below the transfer"
        );
        assert_eq!(
            charge_step(1_000, 80, 1_000_000),
            80,
            "the machine's transfer limits the step"
        );
        assert_eq!(
            charge_step(1_000, 80, 20),
            20,
            "the item's own room limits the step"
        );
        assert_eq!(
            charge_step(0, 80, 1_000),
            0,
            "an empty machine moves nothing"
        );
        assert_eq!(
            charge_step(-5, 80, 1_000),
            0,
            "a negative balance is an empty one"
        );
        assert_eq!(
            charge_step(1_000, 0, 1_000),
            0,
            "a machine with no transfer moves nothing"
        );
        assert_eq!(charge_step(1_000, 80, -1), 0, "a full item has no room");
    }

    #[test]
    fn a_discharge_step_takes_the_item_s_charge_the_transfer_or_the_room() {
        // `EnergyDischargerTile.postTick`: `dischargeItemsInInventory(inv, transfer,
        // capacity - stored)` → per slot `min(transfer, room left)`, and the item's own
        // `extractEnergy` caps it at what the item holds. The mirror of `charge_step`.
        assert_eq!(
            discharge_step(5_000, 1_000, 1_000_000),
            1_000,
            "a charged item gives one transfer a tick"
        );
        assert_eq!(
            discharge_step(300, 1_000, 1_000_000),
            300,
            "the item's own charge limits the step when it holds less than the transfer"
        );
        assert_eq!(
            discharge_step(5_000, 1_000, 20),
            20,
            "the buffer's room limits the step: 20 of room takes 20, not 1,000"
        );
        assert_eq!(
            discharge_step(0, 1_000, 1_000_000),
            0,
            "an empty item gives nothing"
        );
        assert_eq!(
            discharge_step(-5, 1_000, 1_000_000),
            0,
            "a negative charge is an empty one"
        );
        assert_eq!(
            discharge_step(5_000, 0, 1_000_000),
            0,
            "a discharger with no transfer takes nothing"
        );
        assert_eq!(
            discharge_step(5_000, 1_000, 0),
            0,
            "a full buffer has no room"
        );
    }

    #[test]
    fn the_discharger_ladder_is_the_battery_config() {
        // `PowahConfig.EnergyDevices`: `dischargers = new EnergyConfig(batteryCapacity(),
        // batteryTransfer())` — the battery numbers, so the discharger ladder is the cell
        // ladder: 1M/1k at starter, 2G/2M at nitro.
        let expected = [
            (1_000_000, 1_000),
            (4_000_000, 4_000),
            (10_000_000, 10_000),
            (40_000_000, 40_000),
            (100_000_000, 100_000),
            (400_000_000, 400_000),
            (2_000_000_000, 2_000_000),
        ];
        for (index, (capacity, transfer)) in expected.into_iter().enumerate() {
            let tier =
                discharger_tier(index).unwrap_or_else(|| panic!("tier {index} is on the ladder"));
            assert_eq!(
                (tier.capacity, tier.transfer),
                (capacity, transfer),
                "discharger tier {index}'s numbers"
            );
            assert_eq!(
                discharger_tier(index),
                cell_tier(index),
                "tier {index} is the battery config, which is the cell's row"
            );
        }
        assert_eq!(
            discharger_tier(CELL_TIER_COUNT),
            None,
            "there is no eighth discharger tier"
        );
    }

    #[test]
    fn a_broken_machine_carries_its_buffer_only_when_it_holds_one() {
        assert_eq!(carried_energy(5_000, 20_000), Some(5_000));
        assert_eq!(carried_energy(20_000, 20_000), Some(20_000));
        assert_eq!(
            carried_energy(0, 20_000),
            None,
            "an empty machine carries nothing: upstream writes the storable compound only when it is not empty"
        );
        assert_eq!(carried_energy(-1, 20_000), None, "an empty buffer is empty");
        assert_eq!(
            carried_energy(2_000_000_000, 20_000),
            Some(20_000),
            "the carried value is clamped into the tier"
        );
    }

    #[test]
    fn a_placed_machine_takes_its_item_s_buffer_clamped_to_its_tier() {
        assert_eq!(restored_energy(5_000, 20_000), 5_000);
        assert_eq!(
            restored_energy(2_000_000_000, 20_000),
            20_000,
            "the counterfactual: an item doctored to 2,000,000,000 FE places a full 20,000 FE machine, not 2,000,000,000"
        );
        assert_eq!(
            restored_energy(5_000, 1_000_000),
            5_000,
            "a bigger tier's buffer keeps what the item carried"
        );
        assert_eq!(restored_energy(-5, 20_000), 0, "nothing negative survives");
        assert_eq!(
            restored_energy(5_000, 0),
            0,
            "a buffer of capacity 0 holds nothing"
        );
    }

    #[test]
    fn the_hopper_ladder_is_the_battery_config_with_its_own_charging_rate() {
        // `PowahConfig.EnergyDevices`: `hoppers = new ChargingConfig(batteryCapacity(),
        // batteryTransfer(), chargingTransfer())` — the cell's storage row, and a charge rate of
        // `cableTransfer` (`baseScaling * 500`): 500 FE a tick at starter, 1,000,000 at nitro.
        let expected = [
            (1_000_000, 1_000, 500),
            (4_000_000, 4_000, 2_000),
            (10_000_000, 10_000, 5_000),
            (40_000_000, 40_000, 20_000),
            (100_000_000, 100_000, 50_000),
            (400_000_000, 400_000, 200_000),
            (2_000_000_000, 2_000_000, 1_000_000),
        ];
        for (index, (capacity, transfer, charging_rate)) in expected.into_iter().enumerate() {
            let tier =
                hopper_tier(index).unwrap_or_else(|| panic!("tier {index} is on the ladder"));
            assert_eq!(
                (tier.capacity, tier.transfer, tier.charging_rate),
                (capacity, transfer, charging_rate),
                "hopper tier {index}'s numbers"
            );
            assert_eq!(
                cell_tier(index).map(|cell| (cell.capacity, cell.transfer)),
                Some((capacity, transfer)),
                "tier {index} stores what the cell of that tier stores: the battery config"
            );
            assert_eq!(
                cable_tier(index).map(|cable| cable.transfer),
                Some(charging_rate),
                "tier {index} charges at its cable's transfer: `chargingTransfer`"
            );
        }
        assert_eq!(
            hopper_tier(HOPPER_TIER_COUNT),
            None,
            "there is no eighth hopper tier"
        );
    }

    #[test]
    fn a_hopper_pass_charges_each_item_at_its_rate_within_one_budget() {
        // `ChargeUtil.transferSlotList`: `min(maxPerStack, maxTotal - charged)` per slot, in slot
        // order. 500 a slot at starter, out of one 1,200 budget: two full steps and a remainder.
        assert_eq!(
            hopper_charges(1_200, 500, &[10_000, 10_000, 10_000]),
            vec![500, 500, 200],
            "the pass spends the tier's rate a slot, then only what the buffer has left"
        );
        assert_eq!(
            hopper_charges(5_000, 500, &[10_000, 10_000]),
            vec![500, 500],
            "a buffer with plenty charges every item its full rate"
        );
        assert_eq!(
            hopper_charges(1_200, 500, &[100, 10_000]),
            vec![100, 500],
            "an item with 100 of room takes 100, and the next still gets its rate"
        );
        assert_eq!(
            hopper_charges(1_200, 500, &[0, 10_000]),
            vec![0, 500],
            "an empty slot, or an item with no energy buffer, is left alone and costs no budget"
        );
        assert_eq!(
            hopper_charges(0, 500, &[10_000]),
            vec![0],
            "an empty buffer charges nothing"
        );
        assert_eq!(
            hopper_charges(5_000, 0, &[10_000]),
            vec![0],
            "a tier with no charging rate charges nothing"
        );
        assert_eq!(
            hopper_charges(-5, 500, &[10_000]),
            vec![0],
            "nothing negative is charged"
        );
        assert_eq!(
            hopper_charges(5_000, 500, &[]),
            Vec::<i32>::new(),
            "a container with no slots is a pass that does nothing"
        );
    }

    #[test]
    fn every_player_transmitter_tier_is_the_battery_config_with_its_own_charging_rate() {
        // `PowahConfig.EnergyDevices`: `player_transmitters = new ChargingConfig(batteryCapacity(),
        // batteryTransfer(), chargingTransfer())` — the cell's storage row and `cableTransfer`
        // (`baseScaling * 500`) as the per-item charge rate: 500 FE a tick at starter, 1,000,000 at
        // nitro.
        let expected = [
            (1_000_000, 1_000, 500),
            (4_000_000, 4_000, 2_000),
            (10_000_000, 10_000, 5_000),
            (40_000_000, 40_000, 20_000),
            (100_000_000, 100_000, 50_000),
            (400_000_000, 400_000, 200_000),
            (2_000_000_000, 2_000_000, 1_000_000),
        ];
        for (index, (capacity, transfer, charging_rate)) in expected.into_iter().enumerate() {
            let tier = transmitter_tier(index)
                .unwrap_or_else(|| panic!("tier {index} is on the transmitter ladder"));
            assert_eq!(
                (tier.capacity, tier.transfer, tier.charging_rate),
                (capacity, transfer, charging_rate),
                "player transmitter tier {index}'s numbers"
            );
            assert_eq!(
                cell_tier(index).map(|cell| (cell.capacity, cell.transfer)),
                Some((capacity, transfer)),
                "tier {index} stores what the cell of that tier stores: the battery config"
            );
            assert_eq!(
                hopper_tier(index).map(|hopper| hopper.charging_rate),
                Some(charging_rate),
                "tier {index} charges at the hopper's rate: upstream gives both `chargingTransfer`"
            );
        }
        assert_eq!(
            transmitter_tier(TRANSMITTER_TIER_COUNT),
            None,
            "there is no eighth player transmitter tier"
        );
    }

    #[test]
    fn a_player_pass_charges_each_inventory_item_at_its_rate_within_one_budget() {
        // `ChargeUtil.chargeItemsInPlayerInv` is the container pass over `Player.invStacks`: the
        // main inventory, the armour and the off hand, one `min(rate, budget, room)` step each.
        assert_eq!(
            player_charges(1_200, 500, &[10_000, 10_000, 10_000]),
            vec![500, 500, 200],
            "the pass spends the tier's rate a slot, then only what the buffer has left"
        );
        assert_eq!(
            player_charges(100_000, 500, &[10_000; 41]),
            vec![500; 41],
            "a full inventory of roomy items takes the rate in every one of the 41 slots"
        );
        assert_eq!(
            player_charges(1_200, 500, &[0, 10_000, 0, 100]),
            vec![0, 500, 0, 100],
            "empty slots and items with no energy buffer are left alone and cost no budget"
        );
        assert_eq!(
            player_charges(0, 500, &[10_000]),
            vec![0],
            "a transmitter with an empty buffer charges nothing"
        );
        assert_eq!(
            player_charges(-5, 500, &[10_000]),
            vec![0],
            "nothing negative is charged"
        );
    }

    #[test]
    fn every_ender_tier_is_one_of_the_ladder_with_its_own_transfer_and_channels() {
        // `PowahConfig.EnergyDevices`: cells take `batteryTransfer()` (`baseScaling * 1_000`),
        // gates `cableTransfer()` (`baseScaling * 500`) — so a cell's row is exactly twice a gate's
        // at every tier — and both take `TieredChannelValues.getDefault()` for their channels.
        let expected = [
            (1_000, 500, 1),
            (4_000, 2_000, 2),
            (10_000, 5_000, 3),
            (40_000, 20_000, 5),
            (100_000, 50_000, 7),
            (400_000, 200_000, 9),
            (2_000_000, 1_000_000, 12),
        ];
        for (index, (cell, gate, channels)) in expected.into_iter().enumerate() {
            let cell_tier =
                ender_cell_tier(index).unwrap_or_else(|| panic!("tier {index} is a cell"));
            let gate_tier =
                ender_gate_tier(index).unwrap_or_else(|| panic!("tier {index} is a gate"));
            assert_eq!(
                (cell_tier.transfer, cell_tier.channels),
                (cell, channels),
                "ender cell tier {index}'s numbers"
            );
            assert_eq!(
                (gate_tier.transfer, gate_tier.channels),
                (gate, channels),
                "ender gate tier {index}'s numbers"
            );
            assert_eq!(
                cell_tier.transfer,
                gate_tier.transfer * 2,
                "the cell row is `batteryTransfer`, twice the gate's `cableTransfer`"
            );
        }
        assert_eq!(ender_cell_tier(ENDER_TIER_COUNT), None);
        assert_eq!(ender_gate_tier(ENDER_TIER_COUNT), None);
        assert_eq!(MAX_CHANNELS, 12, "`EnderNetwork.MAX_CHANNELS`");
        assert_eq!(
            ender_cell_tier(ENDER_TIER_COUNT - 1)
                .expect("the nitro cell")
                .channels,
            MAX_CHANNELS,
            "the nitro tier is the one that offers every channel"
        );
    }

    #[test]
    fn a_channel_takes_and_gives_at_most_its_transfer() {
        // A fresh channel has capacity 0 (`EnderConfig.getCapacity` is 0 and `Energy.create(0)`
        // starts empty), so nothing can even be put into an owner's network before an extender.
        assert_eq!(
            Channel::default().insert(5_000, 1_000),
            (Channel::default(), 0),
            "an empty channel has no room, whatever is offered"
        );
        let open = Channel {
            stored: 0,
            capacity: 4_000_000,
        };
        assert_eq!(
            open.insert(5_000, 1_000),
            (
                Channel {
                    stored: 1_000,
                    capacity: 4_000_000
                },
                1_000
            ),
            "one call moves the tier's transfer, not the offer"
        );
        let full = Channel {
            stored: 4_000_000,
            capacity: 4_000_000,
        };
        assert_eq!(
            full.insert(1_000, 1_000).1,
            0,
            "a full channel takes nothing"
        );
        let near = Channel {
            stored: 3_999_500,
            capacity: 4_000_000,
        };
        assert_eq!(
            near.insert(1_000, 1_000).1,
            500,
            "only the room left goes in"
        );
        let held = Channel {
            stored: 100_000,
            capacity: 4_000_000,
        };
        assert_eq!(
            held.extract(5_000, 500),
            (
                Channel {
                    stored: 99_500,
                    capacity: 4_000_000
                },
                500
            ),
            "a gate's row moves 500 a call out of the same channel"
        );
        assert_eq!(
            Channel {
                stored: 200,
                capacity: 4_000_000
            }
            .extract(5_000, 500)
            .1,
            200,
            "a channel gives what it holds, never more"
        );
        assert_eq!(
            Channel {
                stored: 100,
                capacity: 4_000_000
            }
            .insert(-5, 1_000)
            .1,
            0,
            "nothing negative is inserted"
        );
    }

    #[test]
    fn an_extender_adds_its_capacity_and_its_own_charge() {
        // `AbstractEnderTile.onSlotChanged`: `newCap = capacity + extender capacity`, then
        // `setStored(extender stored + channel stored)` — a 4,000,000-FE basic energy cell holding
        // 500 gives the channel both.
        assert_eq!(
            Channel {
                stored: 100,
                capacity: 1_000
            }
            .extended(4_000_000, 500),
            Channel {
                stored: 600,
                capacity: 4_001_000
            },
            "the item's capacity joins the channel and its charge is spent into it"
        );
        assert_eq!(
            Channel {
                stored: 900,
                capacity: 1_000
            }
            .extended(0, 5_000),
            Channel {
                stored: 1_000,
                capacity: 1_000
            },
            "an addend beyond the capacity is clamped, as `setStored` does"
        );
        assert_eq!(
            Channel {
                stored: 0,
                capacity: 0
            }
            .extended(-5, -5),
            Channel {
                stored: 0,
                capacity: 0
            },
            "nothing negative is added"
        );
    }

    #[test]
    fn a_channel_index_is_clamped_to_the_tiers_own_offering() {
        // The saved index is a `RangedInt(12)` and `getMaxChannels()` is what the GUI offers, so a
        // starter cell has exactly one channel (index 0) and a basic one two.
        assert_eq!(channel_clamped(0, 1), 0);
        assert_eq!(
            channel_clamped(3, 1),
            0,
            "a starter cell offers channel 0 only"
        );
        assert_eq!(channel_clamped(-1, 1), 0);
        assert_eq!(channel_clamped(1, 2), 1, "a basic cell offers 0 and 1");
        assert_eq!(channel_clamped(5, 5), 4, "a blazing cell offers 0..=4");
        assert_eq!(
            channel_clamped(11, 12),
            11,
            "the nitro cell offers every channel"
        );
        assert_eq!(channel_clamped(20, 12), 11);
    }

    #[test]
    fn an_ender_pass_charges_its_two_slots_out_of_one_budget() {
        // `AbstractEnergyStorage.chargeItems(1, 3)` is slots 1 and 2 with `maxPerSlot` the tier's
        // transfer and one budget for the pass.
        assert_eq!(
            charge_slots(1_500, 1_000, &[10_000, 10_000]),
            vec![1_000, 500],
            "the pass spends the transfer a slot, then only what the channel has left"
        );
        assert_eq!(
            charge_slots(200, 1_000, &[10_000, 10_000]),
            vec![200, 0],
            "an almost empty channel charges one slot and no more"
        );
    }

    #[test]
    fn every_solar_tier_scales_upstreams_passive_numbers() {
        // `DefaultEnergies`: `passiveProduction = baseSlowScaling * 20` (1, 3, 5, 10, 20, 40, 100),
        // while `generatorCapacity` and `generatorTransfer` are the active row every generator
        // shares: `baseScaling * 20_000` and `baseScaling * 80`.
        let expected = [
            (20, 20_000, 80),
            (60, 80_000, 320),
            (100, 200_000, 800),
            (200, 800_000, 3_200),
            (400, 2_000_000, 8_000),
            (800, 8_000_000, 32_000),
            (2_000, 40_000_000, 160_000),
        ];
        for (index, (generation, capacity, transfer)) in expected.into_iter().enumerate() {
            let tier = solar_tier(index).unwrap_or_else(|| panic!("tier {index} is on the ladder"));
            assert_eq!(
                (tier.generation, tier.capacity, tier.transfer),
                (generation, capacity, transfer),
                "solar tier {index}'s numbers"
            );
            let furnator = furnator_tier(index).expect("the furnator of the same tier");
            assert_eq!(
                (tier.capacity, tier.transfer),
                (furnator.capacity, furnator.transfer),
                "tier {index} stores and gives what a furnator of its tier does"
            );
        }
        assert_eq!(
            solar_tier(SOLAR_TIER_COUNT),
            None,
            "there is no eighth solar tier"
        );
        // The one number a solar panel does not share with the furnator: upstream derives it from
        // `baseSlowScaling`, so a basic panel makes 60 FE/t where a basic furnator burns 80.
        assert_eq!(
            solar_tier(1).expect("the basic tier").generation,
            60,
            "`passiveProduction` is the slow scaling"
        );
        assert_eq!(
            furnator_tier(1).expect("the basic furnator").generation,
            80,
            "while the furnator's `activeProduction` is the fast one"
        );
    }

    #[test]
    fn the_solar_daylight_test_is_upstreams_two_reads() {
        // `dimensionType().hasSkyLight() && getSkyDarken() < 4`.
        let open = |darken| Sky {
            can_see_sky: true,
            has_skylight: true,
            darken,
        };
        assert!(open(0).daylight(), "noon");
        assert!(open(SOLAR_DAY_MAX_DARKEN - 1).daylight(), "dusk");
        assert!(!open(SOLAR_DAY_MAX_DARKEN).daylight(), "the boundary");
        assert!(!open(11).daylight(), "midnight");
        assert!(
            !Sky {
                has_skylight: false,
                ..open(0)
            }
            .daylight(),
            "the Nether and the End have no sky to light a panel"
        );
    }

    #[test]
    fn a_solar_panel_generates_only_with_sky_daylight_and_room() {
        let starter = solar_tier(0).expect("the starter tier");
        let day = Sky {
            can_see_sky: true,
            has_skylight: true,
            darken: 0,
        };
        assert_eq!(
            solar_production(0, starter, day, false),
            starter.generation,
            "a panel under an open noon sky makes its 20 FE a tick"
        );
        assert_eq!(
            solar_production(19_980, starter, day, false),
            20,
            "with exactly 20 of room it makes the whole generation"
        );
        assert_eq!(
            solar_production(19_990, starter, day, false),
            10,
            "with 10 of room it makes 10, not 20"
        );
        assert_eq!(
            solar_production(starter.capacity, starter, day, false),
            0,
            "a full buffer pauses production"
        );
        assert_eq!(
            solar_production(starter.capacity + 5, starter, day, false),
            0,
            "and a doctored buffer above the tier makes nothing"
        );
        assert_eq!(
            solar_production(
                0,
                starter,
                Sky {
                    can_see_sky: false,
                    ..day
                },
                false
            ),
            0,
            "a stone above the panel blocks it"
        );
        assert_eq!(
            solar_production(0, starter, Sky { darken: 11, ..day }, false),
            0,
            "midnight makes nothing, however open the sky"
        );
        assert_eq!(
            solar_production(
                0,
                starter,
                Sky {
                    has_skylight: false,
                    ..day
                },
                false
            ),
            0,
            "a dimension the sky does not light makes nothing"
        );
        assert_eq!(
            solar_production(
                0,
                starter,
                Sky {
                    can_see_sky: false,
                    ..day
                },
                true
            ),
            20,
            "a Lens of Ender stands in for the sky (the item itself is not ported)"
        );
        assert_eq!(
            solar_production(
                0,
                starter,
                Sky {
                    can_see_sky: false,
                    darken: 11,
                    ..day
                },
                true
            ),
            0,
            "but the lens does not make it night"
        );
        let nitro = solar_tier(6).expect("the nitro tier");
        assert_eq!(
            solar_production(0, nitro, day, false),
            2_000,
            "the nitro panel makes its 2,000 FE a tick"
        );
    }
}
