//! Powah for Horizon — slice 1, the starter Furnator, Energy Cell and Energy Cable.
//!
//! One flat SDK package (mod API 0.2): the carried blocks, their items, models,
//! textures, recipes, loot, tags and lang are the `assets/` and `data/` beside
//! `src/`, and the behaviour is the one `impl Hooks` below. The rules themselves
//! live in [`runtime`], byte-identical to the retired `.hnmod` bundle's
//! `runtime.rs`; [`powah`] is the glue.
//!
//! What the retired bundle expressed as nine injected helpers is here four shapes:
//!
//! - three **block entity types** declared in [`Hooks::register`]
//!   (`block_entity_type(..).blocks(..)`), with the furnator and the
//!   cell/cable each carrying a **Part** — a ticked machine, so the nine hooks it
//!   used (`block_entity`, `block_entity_ticks`, `block_entity_tick`) are one
//!   declaration and two Part methods;
//! - two **declarations**: the furnator's container (one fuel slot) and the energy
//!   buffers of the furnator and the cell (`Registrar::block_container`,
//!   `Registrar::block_energy`) — the old `block_entity_container` and
//!   `block_entity_energy` hooks, keyed by the block instead of a shared output;
//! - one **level hook**, [`Hooks::level_tick_post`]: the queued energy moves, since
//!   a block entity is out of the world during its own tick;
//! - the **typed command pair** [`Hooks::register_commands`] /
//!   [`Hooks::run_mod_command`], which replaces `register_commands` +
//!   `perform_command`, plus [`Hooks::world_lifecycle`], which drops the queue when
//!   a world opens or closes.
//!
//! Its `content_root` embedded-content helper has no successor: one `.hmod`
//! carries the content and the code, which is what that helper existed to work
//! around.
pub mod runtime;

mod powah;

use std::sync::{Arc, Mutex};

use mod_api::{
    self, BlockDropsEvent, ComponentTypesEvent, Flow, Hooks, HostSide, LevelHost,
    LevelTickPostEvent, RegisterCommandsEvent, Registrar, RegistrationContext, RunModCommandEvent,
    ServerHost, Text, WorldLifecycleEvent,
};

use crate::powah::{Queue, SetOutcome};

/// Declare one idle block entity type (a cell, a cable or a rod) and attach the
/// Part that queues its position for the level tick's move phase and clamps a
/// buffer that is out of its tier.
fn declare_idle(
    registrar: &mut dyn Registrar,
    context: &RegistrationContext,
    queue: &Arc<Mutex<Queue>>,
    name: &str,
    rules: Option<powah::IdleRules>,
) {
    let kind = powah::declare(registrar, name);
    let part = powah::IdlePart::new(Arc::clone(queue), rules);
    if let Err(error) = registrar.block_entity_part(&kind, part) {
        context.refuse(&error.to_string());
    }
}

/// Declare a machine's own item: the Part that restores the buffer a placed item
/// carried, and — when the item holds one — the `Capabilities.Energy.ITEM`
/// handler a charger reads.
fn declare_item(registrar: &mut dyn Registrar, block: &'static str) {
    let item = registrar.item_key(block);
    powah::remember_item(block, item);
    registrar.item_part(item, powah::MachineItem);
    if let Some(buffer) = powah::item_buffer(block) {
        registrar.item_capability(&mod_api::ITEM_ENERGY, block, move |_, place| {
            Some(Box::new(powah::MachineEnergy::new(
                place.clone(),
                block,
                buffer,
            )))
        });
    }
}

/// The mod's session state: the machines whose block entity ticked this tick,
/// per level, drained at the end of that level's tick.
#[derive(Default)]
struct Mod {
    queue: Arc<Mutex<Queue>>,
}

#[mod_api::horizon_mod]
impl Hooks for Mod {
    /// Declare the machines: a block entity type each, every furnator tier's one
    /// fuel slot and FE buffer, every cell tier's buffer, and the Parts that make
    /// them tick.
    ///
    /// Twenty-nine blocks in all: the three ladders of seven (furnators, cells,
    /// cables), the orb and its seven rods. A cable declares no buffer (it stores
    /// nothing; upstream's capacity is 0) and still needs a type and a tick,
    /// because a cable is what queues its own position for the level's move phase.
    fn register(&self, registrar: &mut dyn Registrar, context: &RegistrationContext) {
        // Every furnator tier: a charge slot then a fuel slot, one buffer with the
        // tier's own generation capacity and per-side transfer (upstream's
        // `AbstractGeneratorBlock` over `GeneratorConfig`).
        for (index, (block, name)) in powah::FURNATORS.into_iter().enumerate() {
            let tier = runtime::furnator_tier(index).expect("every furnator id is on the ladder");
            let furnator = registrar.block_key(block);
            registrar.block_container(&furnator, powah::container());
            registrar.block_energy(&furnator, powah::furnator_battery(tier));
            let kind = powah::declare(registrar, name);
            let part = powah::FurnatorPart::new(Arc::clone(&self.queue), tier);
            if let Err(error) = registrar.block_entity_part(&kind, part) {
                context.refuse(&error.to_string());
            }
            declare_item(registrar, block);
        }
        // Every energy cell tier: two charge slots and one buffer with the tier's
        // own capacity and per-side transfer (upstream's `AbstractEnergyStorage`
        // over `EnergyConfig`).
        for (index, (block, name)) in powah::CELLS.into_iter().enumerate() {
            let tier = runtime::cell_tier(index).expect("every cell id is on the tier ladder");
            let cell = registrar.block_key(block);
            registrar.block_container(&cell, powah::cell_container());
            registrar.block_energy(&cell, powah::cell_battery(tier));
            declare_idle(
                registrar,
                context,
                &self.queue,
                name,
                Some(powah::IdleRules {
                    capacity: tier.capacity,
                    transfer: tier.transfer,
                    slots: powah::kind_item_slots(powah::Kind::Cell(tier)),
                }),
            );
            declare_item(registrar, block);
        }
        // Every energy discharger tier: seven slots whose items it drains, and one
        // extract-only buffer (upstream's `devices.dischargers` — the battery config —
        // over `AbstractEnergyBlock` with `getTransferType()` = `Transfer.EXTRACT`).
        for (index, (block, name)) in powah::DISCHARGERS.into_iter().enumerate() {
            let tier =
                runtime::discharger_tier(index).expect("every discharger id is on the ladder");
            let discharger = registrar.block_key(block);
            registrar.block_container(&discharger, powah::discharger_container());
            registrar.block_energy(&discharger, powah::discharger_battery(tier));
            declare_idle(
                registrar,
                context,
                &self.queue,
                name,
                Some(powah::IdleRules {
                    capacity: tier.capacity,
                    transfer: tier.transfer,
                    // The drain runs at the level tick's end (`drain_items`): a block
                    // entity is out of the world during its own tick, so an item
                    // capability there answers nothing.
                    slots: 0,
                }),
            );
            declare_item(registrar, block);
        }
        // Every energy hopper tier: a buffer with the tier's own capacity, transfer and charging
        // rate and **no slots of its own** — it charges the container on the face its `facing`
        // names (upstream's `devices.hoppers`, a `ChargingConfig`, over `AbstractEnergyBlock`'s
        // default `Transfer.ALL`).
        for (index, (block, name)) in powah::HOPPERS.into_iter().enumerate() {
            let tier = runtime::hopper_tier(index).expect("every hopper id is on the ladder");
            let hopper = registrar.block_key(block);
            registrar.block_energy(&hopper, powah::hopper_battery(tier));
            let kind = powah::declare(registrar, name);
            let part = powah::HopperPart::new(Arc::clone(&self.queue), tier);
            if let Err(error) = registrar.block_entity_part(&kind, part) {
                context.refuse(&error.to_string());
            }
            declare_item(registrar, block);
        }
        // Every solar panel tier: one charge slot, a buffer the sky fills, and the bottom face it
        // gives on (upstream's `devices.solar_panels` over `AbstractEnergyProvider`, whose
        // `isEnergyPresent` is `Direction.DOWN`).
        for (index, (block, name)) in powah::SOLARS.into_iter().enumerate() {
            let tier = runtime::solar_tier(index).expect("every solar id is on the ladder");
            let solar = registrar.block_key(block);
            registrar.block_container(&solar, powah::solar_container());
            registrar.block_energy(&solar, powah::solar_battery(tier));
            let kind = powah::declare(registrar, name);
            let part = powah::SolarPart::new(Arc::clone(&self.queue), tier);
            if let Err(error) = registrar.block_entity_part(&kind, part) {
                context.refuse(&error.to_string());
            }
            declare_item(registrar, block);
        }
        // Every ender cell tier: three slots (an extender and two charge slots), and the `ENERGY`
        // capability that *is* the owner's channel — upstream's `AbstractEnderTile.getEnergy()`
        // handing every block on a channel the one shared `Energy`. A cell answers all six faces
        // and takes an extender into slot 0 (upstream's `devices.ender_cells`, `batteryTransfer`).
        for (index, (block, name)) in powah::ENDER_CELLS.into_iter().enumerate() {
            let tier =
                runtime::ender_cell_tier(index).expect("every ender cell id is on the ladder");
            let cell = registrar.block_key(block);
            registrar.block_container(&cell, powah::ender_container());
            registrar.block_capability_before_game(
                &mod_api::ENERGY,
                &cell,
                move |host, at, state, entity| {
                    powah::ender_capability(host, at, state, entity, powah::Kind::EnderCell(tier))
                },
            );
            let kind = powah::declare(registrar, name);
            let part = powah::EnderPart::new(Arc::clone(&self.queue), powah::Kind::EnderCell(tier));
            if let Err(error) = registrar.block_entity_part(&kind, part) {
                context.refuse(&error.to_string());
            }
            declare_item(registrar, block);
        }
        // Every ender gate tier: the same three slots and the same channel, but its capability and
        // its pushes answer only the face its `facing` names (`EnderGateTile.isEnergyPresent`), and
        // it never extends the network (`getExtendedCapacity` on its item is nothing: it is not an
        // `IEnderExtender`). Upstream's `devices.ender_gates` transfer at `cableTransfer`.
        for (index, (block, name)) in powah::ENDER_GATES.into_iter().enumerate() {
            let tier =
                runtime::ender_gate_tier(index).expect("every ender gate id is on the ladder");
            let gate = registrar.block_key(block);
            registrar.block_container(&gate, powah::ender_container());
            registrar.block_capability_before_game(
                &mod_api::ENERGY,
                &gate,
                move |host, at, state, entity| {
                    powah::ender_capability(host, at, state, entity, powah::Kind::EnderGate(tier))
                },
            );
            let kind = powah::declare(registrar, name);
            let part = powah::EnderPart::new(Arc::clone(&self.queue), powah::Kind::EnderGate(tier));
            if let Err(error) = registrar.block_entity_part(&kind, part) {
                context.refuse(&error.to_string());
            }
            declare_item(registrar, block);
        }
        // Every player transmitter tier: one slot (the binding card) and a buffer with the tier's
        // capacity and transfer, charged into the bound player's inventory at the tier's rate
        // (upstream's `devices.player_transmitters`, a `ChargingConfig`, over
        // `PlayerTransmitterTile.postTick`).
        for (index, (block, name)) in powah::TRANSMITTERS.into_iter().enumerate() {
            let tier = runtime::transmitter_tier(index)
                .expect("every player transmitter id is on the ladder");
            let transmitter = registrar.block_key(block);
            registrar.block_container(&transmitter, powah::transmitter_container());
            registrar.block_energy(&transmitter, powah::transmitter_battery(tier));
            let kind = powah::declare(registrar, name);
            let part = powah::TransmitterPart::new(tier);
            if let Err(error) = registrar.block_entity_part(&kind, part) {
                context.refuse(&error.to_string());
            }
            declare_item(registrar, block);
        }
        // A player transmitter's block entity and its one slot are registered above; the card's
        // own item is the one Part that is not a machine item.
        let card = registrar.item_key(powah::BINDING_CARD);
        registrar.item_part(card, powah::CardPart);
        // The aerial pearl: used on a zombie it makes the player aerial pearl the starter
        // transmitter's recipe is built from.
        let pearl = registrar.item_key(powah::AERIAL_PEARL);
        powah::remember_item(
            powah::PLAYER_AERIAL_PEARL,
            registrar.item_key(powah::PLAYER_AERIAL_PEARL),
        );
        registrar.item_part(pearl, powah::PearlPart);
        // The energizing orb: the seven slots and the insert-only buffer a rod fills.
        let orb = registrar.block_key(powah::ORB);
        registrar.block_container(&orb, powah::orb_container());
        registrar.block_energy(&orb, powah::orb_battery());
        // Every rod tier: its own capacity and per-side transfer.
        for (index, (block, name)) in powah::RODS.into_iter().enumerate() {
            let tier = runtime::rod_tier(index).expect("every rod id is on the tier ladder");
            let rod = registrar.block_key(block);
            registrar.block_energy(&rod, powah::rod_battery(tier));
            declare_idle(
                registrar,
                context,
                &self.queue,
                name,
                Some(powah::IdleRules {
                    capacity: tier.capacity,
                    transfer: tier.transfer,
                    slots: powah::kind_item_slots(powah::Kind::Rod(tier)),
                }),
            );
            declare_item(registrar, block);
        }

        // The orb fills and completes; a cable only queues its position for the
        // move phase. Every furnator tier, cell tier and rod was declared above,
        // where its buffer is.
        let orb_kind = powah::declare(registrar, powah::ORB_TYPE);
        if let Err(error) = registrar.block_entity_part(&orb_kind, powah::OrbPart) {
            context.refuse(&error.to_string());
        }
        for (block, name) in powah::CABLES {
            declare_idle(registrar, context, &self.queue, name, None);
            // A cable's item stores nothing, but it is still one of this mod's
            // blocks: it has no payload to restore and the Part simply finds none.
            declare_item(registrar, block);
        }
    }

    /// The components this mod registers: a binding card's bound player (`powah:bound_player`,
    /// upstream's `PowahComponents.BOUND_PLAYER`), which the card's own `use` writes and the
    /// transmitter's tick reads.
    fn component_types(&self, _context: &RegistrationContext, event: &mut ComponentTypesEvent<'_>) {
        event.types.push(powah::BOUND_PLAYER.to_owned());
    }

    /// A block broke: its own item carries its buffer when it is one of the
    /// machines upstream's `keepEnergy` names (every furnator, cell and rod), so
    /// placing the item puts the FE back.
    fn block_drops(&self, level: &dyn LevelHost, event: &mut BlockDropsEvent<'_>) -> Flow {
        powah::on_block_drops(level, event)
    }

    /// After the level's tick: run the queued moves, where every block entity is
    /// back in the world. A client's tick runs nothing.
    fn level_tick_post(&self, host: HostSide, event: &mut LevelTickPostEvent) {
        let HostSide::Server(host) = host else {
            return;
        };
        // The moves reach the machines through this level's `ENERGY` capability, and
        // `LevelRef::capability` answers only for the level the server ticks now.
        // Outside that window the queue is left alone and the next tick retries, so
        // no machine's move is dropped; the queue is bounded by the machine count.
        if !event.level.in_place(host) {
            return;
        }
        let queued = self
            .queue
            .lock()
            .expect("the powah queue lock is held for one drain")
            .take(event.level);
        powah::on_level_tick_post(host, event.level, queued);
    }

    /// A world opened or closed: queued positions belong to the world they were
    /// queued in, so none outlives it.
    fn world_lifecycle(&self, _host: &mut dyn ServerHost, _event: &mut WorldLifecycleEvent<'_>) {
        self.queue
            .lock()
            .expect("the powah queue lock is held for one clear")
            .clear();
    }

    /// `/powah info <pos>` and `/powah set <amount> <pos>`: operator-only
    /// diagnostics (level 2). Slice 1 has no GUI and no in-game FE readout, so
    /// these are the harness controls its in-game check uses; because they act
    /// on any loaded machine at chosen coordinates, the server refuses them to
    /// non-operators. Upstream's own commands are its config and GUI actions;
    /// none are ported.
    fn register_commands(&self, _host: &mut dyn ServerHost, event: &mut RegisterCommandsEvent<'_>) {
        powah::declare_commands(event.commands);
    }

    /// The command bodies: the arguments are parsed and resolved by the server's
    /// dispatcher, so this only reads a machine and answers, in 0.1.0's words.
    fn run_mod_command(
        &self,
        host: &mut dyn ServerHost,
        event: &mut RunModCommandEvent<'_>,
    ) -> Flow {
        let literals: Vec<&str> = event.command.literals.iter().map(String::as_str).collect();
        let pos = powah::block_pos(event.command.argument("pos"));
        let amount = powah::integer(event.command.argument("amount"));
        let channel = powah::integer(event.command.argument("channel"));
        let line = match (literals.as_slice(), pos) {
            (["powah", "info"], Some(pos)) => {
                powah::info_line(host, pos).unwrap_or_else(|| "No Powah block there.".to_owned())
            }
            (["powah", "set"], Some(pos)) => {
                let Some(amount) = amount else {
                    return Flow::Continue;
                };
                match powah::set_energy(host, pos, amount) {
                    SetOutcome::Set(after) => format!("Energy set to {after} FE."),
                    SetOutcome::Cable => "The energy cable stores no energy.".to_owned(),
                    SetOutcome::NoBlock => "No Powah block there.".to_owned(),
                }
            }
            (["powah", "item"], Some(pos)) => {
                let Some(amount) = amount else {
                    return Flow::Continue;
                };
                match powah::set_item_energy(host, pos, amount) {
                    powah::SetItemOutcome::Set(item, after) => {
                        format!("Charge set to {after} FE on {item}.")
                    }
                    powah::SetItemOutcome::NoSlot => {
                        "That machine has no item charge slot.".to_owned()
                    }
                    powah::SetItemOutcome::NotChargeable => {
                        "The charge slot holds no item that carries FE.".to_owned()
                    }
                    powah::SetItemOutcome::NoBlock => "No Powah block there.".to_owned(),
                }
            }
            (["powah", "channel"], Some(pos)) => {
                let Some(channel) = channel else {
                    return Flow::Continue;
                };
                match powah::set_channel_index(host, pos, channel) {
                    powah::ChannelOutcome::Set(set, channels) => {
                        format!("Channel set to {set} of 0..={}.", channels - 1)
                    }
                    powah::ChannelOutcome::NoBlock => "No ender cell or gate there.".to_owned(),
                }
            }
            // `powah redstone <mode> <pos>`: the mode is a literal of the usage, so the dispatcher
            // has already refused a word that names none (as `/powah channel` refuses a non-integer).
            (["powah", "redstone", mode], Some(pos)) => {
                let Some(mode) = runtime::RedstoneMode::named(mode) else {
                    return Flow::Continue;
                };
                match powah::set_redstone_mode(host, pos, mode) {
                    powah::RedstoneOutcome::Set(mode) => {
                        format!("Redstone mode set to {}.", mode.command_word())
                    }
                    powah::RedstoneOutcome::NoMode => "That block has no redstone mode.".to_owned(),
                    powah::RedstoneOutcome::NoBlock => "No Powah block there.".to_owned(),
                }
            }
            _ => return Flow::Continue,
        };
        let _ = host.commands().feedback(Text::literal(line));
        Flow::Cancel
    }
}
