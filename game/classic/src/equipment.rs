//! Worn equipment, gas tanks and internals. Spec: `docs/reference/equipment.md`.

use std::collections::BTreeMap;

use bevy_ecs::prelude::*;
use rs13_simulation::atmos::gas::R;
use rs13_simulation::atmos::{Gas, GasMix};
use rs13_world::{NetId, Slot, WornView};
use serde::Deserialize;

use crate::environment::BREATH_VOLUME;

/// How a tank is filled when spawned.
#[derive(Debug, Clone, Deserialize)]
pub struct TankSpec {
    pub volume: f64,
    pub pressure: f64,
    pub gas: String,
}

/// How much of an attack of each kind a piece of clothing turns aside, % (Luna's `armor`).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
pub struct Armor {
    pub melee: u8,
    pub bullet: u8,
    pub laser: u8,
    pub energy: u8,
    #[serde(default)]
    pub bomb: u8,
}

/// An item prototype from `content/items.ron`.
#[derive(Debug, Clone, Deserialize)]
pub struct ItemDef {
    pub id: String,
    pub force: f32,
    #[serde(default)]
    pub slot: Option<String>,
    #[serde(default)]
    pub slowdown: f32,
    #[serde(default)]
    pub space: bool,
    #[serde(default)]
    pub internals: bool,
    #[serde(default)]
    pub tank: Option<TankSpec>,
    /// Gloves: how much of an electric shock gets through (Luna `siemens_coefficient`).
    #[serde(default = "one")]
    pub siemens: f32,
    /// Clothing: the body parts it covers, as Luna's `body_parts_covered` flags.
    #[serde(default)]
    pub covers: u16,
    #[serde(default)]
    pub armor: Armor,
    /// Clothing: the share of a fire's heat that gets through (Luna `heat_transfer_coefficient`).
    #[serde(default = "one")]
    pub heat: f32,
    /// Clothing: the hottest fire it's any use against, K (Luna `protective_temperature`).
    #[serde(default)]
    pub protect_k: f32,
    /// Shoes that don't slip (Luna `NOSLIP`).
    #[serde(default)]
    pub no_slip: bool,
    /// Luna `w_class`: 1 tiny … 5 huge. Luna's default is 3.
    #[serde(default = "normal_size")]
    pub size: u8,
    /// A container, and what it takes.
    #[serde(default)]
    pub storage: Option<crate::storage::StorageSpec>,
    /// Prototypes it starts with inside (Luna's container `New()`).
    #[serde(default)]
    pub contents: Vec<String>,
    /// Looks like, and is called, this other prototype (a loaded belt is a belt).
    #[serde(default)]
    pub base: Option<String>,
    /// Piles up to this many in one item (a coil of 30 lengths of cable). New ones are
    /// full.
    #[serde(default)]
    pub stack: Option<u32>,
    /// Electricity goes through it (Luna `CONDUCT`).
    #[serde(default)]
    pub conducts: bool,
    /// Shoes that hold to the floor: space wind doesn't move the wearer (Luna
    /// `shoes/magnetic`).
    #[serde(default)]
    pub magnetic: bool,
}

fn normal_size() -> u8 {
    3
}

fn one() -> f32 {
    1.0
}

/// Something that can be worn.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Wearable {
    pub slot: Slot,
    /// Added to movement delay while worn, deciseconds.
    pub slowdown: f32,
    /// Space suit or helmet.
    pub space: bool,
    /// A mask that can feed from a tank.
    pub internals: bool,
    /// Share of an electric shock that gets through (gloves).
    pub siemens: f32,
    /// The body parts it covers, as Luna's `body_parts_covered` flags.
    pub covers: u16,
    pub armor: Armor,
    /// The share of a fire's heat that gets through.
    pub heat: f32,
    /// The hottest fire it's any use against, K.
    pub protect_k: f32,
    pub no_slip: bool,
    /// Magnetic boots: the wearer isn't moved by space wind.
    pub magnetic: bool,
}

/// A gas tank: its contents and the pressure it releases breaths at.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct TankGas {
    pub mix: GasMix,
    /// How long it can stand too much pressure (Luna `integrity`).
    pub integrity: u8,
    /// Litres.
    pub volume: f64,
    /// kPa (Luna `distribute_pressure`, default one atmosphere).
    pub release: f64,
}

impl TankGas {
    pub fn filled(spec: &TankSpec) -> Self {
        let gas = match spec.gas.as_str() {
            "nitrogen" => Gas::Nitrogen,
            "plasma" => Gas::Plasma,
            "carbon-dioxide" => Gas::CarbonDioxide,
            "nitrous-oxide" => Gas::NitrousOxide,
            _ => Gas::Oxygen,
        };
        let mut mix = GasMix::standard_air(1.0).scaled(0.0);
        mix.temperature = rs13_simulation::atmos::gas::T20C;
        mix.set(gas, spec.pressure * spec.volume / (R * mix.temperature));
        Self {
            mix,
            volume: spec.volume,
            release: rs13_simulation::atmos::gas::ONE_ATMOSPHERE,
            integrity: 3,
        }
    }

    pub fn pressure(&self) -> f64 {
        self.mix.pressure(self.volume)
    }

    /// One breath: `BREATH_VOLUME` litres at the release pressure, or at whatever pressure
    /// is left (Luna `remove_air_volume`).
    pub fn take_breath(&mut self) -> GasMix {
        let pressure = self.release.min(self.pressure());
        let t = self.mix.temperature.max(1.0);
        self.mix.remove_moles(pressure * BREATH_VOLUME / (R * t))
    }
}

/// On a worn item: who wears it. A worn item has no `GridPos`.
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct WornBy(pub Entity);

/// What a person wears.
#[derive(Component, Debug, Clone, Default, PartialEq, Eq)]
pub struct Equipment(pub BTreeMap<Slot, Entity>);

impl Equipment {
    pub fn get(&self, slot: Slot) -> Option<Entity> {
        self.0.get(&slot).copied()
    }
}

/// Why an item cannot go into a slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EquipError {
    WrongSlot,
    Occupied,
    /// The ID and belt slots hang off a jumpsuit.
    NeedsUniform,
}

/// Luna's rules: the item must be made for the slot, the slot must be free, and the ID
/// and belt slots need a jumpsuit.
pub fn can_equip(equipment: &Equipment, wearable: Option<&Wearable>, slot: Slot) -> Result<(), EquipError> {
    if wearable.is_none_or(|w| w.slot != slot) {
        return Err(EquipError::WrongSlot);
    }
    if equipment.get(slot).is_some() {
        return Err(EquipError::Occupied);
    }
    if matches!(slot, Slot::Id | Slot::Belt) && equipment.get(Slot::Uniform).is_none() {
        return Err(EquipError::NeedsUniform);
    }
    Ok(())
}

/// Whether the wearer is sealed against vacuum: space suit and space helmet both on.
pub fn sealed(equipment: &Equipment, wearables: &Query<&Wearable>) -> bool {
    let space = |slot| {
        equipment
            .get(slot)
            .and_then(|e| wearables.get(e).ok())
            .is_some_and(|w| w.space)
    };
    space(Slot::Suit) && space(Slot::Head)
}

/// Movement delay from worn things (suit and shoes, as in Luna).
pub fn slowdown(equipment: &Equipment, wearables: &Query<&Wearable>) -> f32 {
    [Slot::Suit, Slot::Shoes]
        .into_iter()
        .filter_map(|slot| equipment.get(slot))
        .filter_map(|e| wearables.get(e).ok())
        .map(|w| w.slowdown)
        .sum()
}

/// Mirror `Equipment` into the replicated `WornView`.
pub fn sync_worn_view(mut people: Query<(&Equipment, &mut WornView)>, ids: Query<&NetId>) {
    for (equipment, mut view) in &mut people {
        let worn = WornView(
            equipment
                .0
                .iter()
                .filter_map(|(slot, e)| ids.get(*e).ok().map(|id| (*slot, *id)))
                .collect(),
        );
        view.set_if_neq(worn);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wearable(slot: Slot) -> Wearable {
        Wearable {
            slot,
            slowdown: 0.0,
            space: false,
            internals: false,
            siemens: 1.0,
            covers: 0,
            armor: Armor::default(),
            heat: 1.0,
            protect_k: 0.0,
            no_slip: false,
            magnetic: false,
        }
    }

    #[test]
    fn id_and_belt_need_a_jumpsuit() {
        let mut eq = Equipment::default();
        assert_eq!(
            can_equip(&eq, Some(&wearable(Slot::Id)), Slot::Id),
            Err(EquipError::NeedsUniform)
        );
        eq.0.insert(Slot::Uniform, Entity::PLACEHOLDER);
        assert_eq!(can_equip(&eq, Some(&wearable(Slot::Id)), Slot::Id), Ok(()));
        assert_eq!(
            can_equip(&eq, Some(&wearable(Slot::Head)), Slot::Id),
            Err(EquipError::WrongSlot)
        );
        assert_eq!(can_equip(&eq, None, Slot::Head), Err(EquipError::WrongSlot));
        assert_eq!(
            can_equip(&eq, Some(&wearable(Slot::Uniform)), Slot::Uniform),
            Err(EquipError::Occupied)
        );
    }

    #[test]
    fn a_luna_oxygen_tank_gives_about_700_full_breaths() {
        let mut tank = TankGas::filled(&TankSpec {
            volume: 70.0,
            pressure: 607.95,
            gas: "oxygen".into(),
        });
        assert!((tank.pressure() - 607.95).abs() < 1e-6);
        let breath = tank.take_breath();
        // A breath at one atmosphere: 0.5 L at 101.3 kPa.
        assert!((breath.pressure(BREATH_VOLUME) - 101.325).abs() < 1e-6);
        // Each full breath lowers 70 L by about 0.72 kPa: from 6 atm down to 1 atm is
        // about 700 breaths, some 47 minutes at one breath per 4 s.
        let mut breaths = 1;
        while tank.pressure() > 101.325 {
            tank.take_breath();
            breaths += 1;
        }
        assert!((680..720).contains(&breaths), "{breaths}");
    }
}

/// Equipment requests, waiting for the next tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EquipAction {
    /// Put the active-hand item on in this slot.
    Equip(Slot),
    /// Take the item off this slot into the empty active hand.
    Unequip(Slot),
    /// Connect the mask to a tank, or disconnect it.
    ToggleInternals,
}

#[derive(Component, Debug, Default)]
pub struct PendingEquipment(pub Vec<EquipAction>);

pub fn process_equipment(
    mut mobs: Query<(
        Entity,
        &rs13_world::GridPos,
        &mut crate::hands::Hands,
        &mut Equipment,
        &mut PendingEquipment,
        &mut crate::actions::Outbox,
        &mut crate::environment::Lungs,
        Option<&crate::health::Vitals>,
    )>,
    wearables: Query<&Wearable>,
    tanks: Query<(), With<TankGas>>,
    appearances: Query<&rs13_world::Appearance>,
    mut commands: Commands,
) {
    use crate::hands::HeldBy;
    use rs13_i18n::LocMsg;
    use rs13_world::{GridPos, MobStat};

    for (mob, pos, mut hands, mut equipment, mut pending, mut outbox, mut lungs, vitals) in &mut mobs {
        if vitals.is_some_and(|v| v.stat != MobStat::Conscious) {
            pending.0.clear();
            continue;
        }
        for action in pending.0.drain(..) {
            match action {
                EquipAction::Equip(slot) => {
                    let Some(item) = hands.active_item() else { continue };
                    match can_equip(&equipment, wearables.get(item).ok(), slot) {
                        Ok(()) => {
                            let active = hands.active;
                            hands.slot_mut(active).take();
                            equipment.0.insert(slot, item);
                            commands.entity(item).remove::<HeldBy>().insert(WornBy(mob));
                        }
                        Err(error) => {
                            let id = match error {
                                EquipError::WrongSlot => "equip-wrong-slot",
                                EquipError::Occupied => "equip-occupied",
                                EquipError::NeedsUniform => "equip-needs-uniform",
                            };
                            outbox.0.push(LocMsg::new(id));
                        }
                    }
                }
                EquipAction::Unequip(slot) => {
                    if hands.active_item().is_some() {
                        continue;
                    }
                    let Some(item) = equipment.0.remove(&slot) else {
                        continue;
                    };
                    let active = hands.active;
                    *hands.slot_mut(active) = Some(item);
                    commands.entity(item).remove::<WornBy>().insert(HeldBy(mob));
                    // Luna: taking off the jumpsuit drops what hangs off it.
                    if slot == Slot::Uniform {
                        for hanging in [Slot::Id, Slot::Belt] {
                            if let Some(dropped) = equipment.0.remove(&hanging) {
                                commands.entity(dropped).remove::<WornBy>().insert(GridPos(pos.0));
                                if lungs.internal == Some(dropped) {
                                    lungs.internal = None;
                                }
                            }
                        }
                    }
                    if slot == Slot::Mask || lungs.internal == Some(item) {
                        lungs.internal = None;
                    }
                }
                EquipAction::ToggleInternals => {
                    if lungs.internal.take().is_some() {
                        outbox.0.push(LocMsg::new("internals-off"));
                        continue;
                    }
                    let mask_ok = equipment
                        .get(Slot::Mask)
                        .and_then(|m| wearables.get(m).ok())
                        .is_some_and(|w| w.internals);
                    if !mask_ok {
                        outbox.0.push(LocMsg::new("internals-no-mask"));
                        continue;
                    }
                    let candidates = [
                        hands.get(hands.active),
                        hands.get(hands.active.other()),
                        equipment.get(Slot::Back),
                        equipment.get(Slot::Belt),
                    ];
                    match candidates.into_iter().flatten().find(|e| tanks.contains(*e)) {
                        Some(tank) => {
                            lungs.internal = Some(tank);
                            let sprite = appearances
                                .get(tank)
                                .map(|a| a.sprite.clone())
                                .unwrap_or_default();
                            outbox.0.push(LocMsg::new("internals-on").arg("item", sprite));
                        }
                        None => outbox.0.push(LocMsg::new("internals-no-tank")),
                    }
                }
            }
        }
    }
}
