//! Vending machines: a window of products and how many are left, and a short wait while
//! the one picked drops out onto the machine's tile. Spec: `docs/reference/vending.md`.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, GridPos, MobStat, NetId, ObjectSpec, TilePos, VendingPanelView, VendingRow};

use crate::access::{IdCard, Shown};
use crate::actions::{Outbox, in_reach};
use crate::equipment::Equipment;
use crate::hands::Hands;
use crate::health::Vitals;
use crate::power::{Channel, PowerUser};
use crate::time::WorldTime;
use crate::wires::PanelViewer;

/// A kind of vending machine (Luna's `vending_tg` subtypes): its prototype, sprite state,
/// how long it takes to hand something out, ds, and what it starts with.
#[derive(Debug, PartialEq)]
pub struct VendingKind {
    pub id: &'static str,
    pub look: &'static str,
    pub delay_ds: f32,
    pub products: &'static [(&'static str, u16)],
    /// The access it wants, if any (Luna `req_access_txt`).
    pub access: Option<&'static str>,
    /// Hung on a wall, so nothing bumps into it.
    pub wall: bool,
}

/// The kinds on NSV Luna.
pub const KINDS: [VendingKind; 10] = [
    VendingKind {
        id: "vending-seeds",
        look: "seeds",
        delay_ds: 10.0,
        products: &[
            ("seed-tomato", 10),
            ("seed-apple", 10),
            ("seed-banana", 10),
            ("seed-lemon", 10),
            ("seed-lime", 10),
            ("seed-orange", 10),
            ("seed-carrot", 10),
            ("seed-potato", 10),
        ],
        access: None,
        wall: false,
    },
    VendingKind {
        id: "vending-medical",
        look: "med",
        delay_ds: 10.0,
        products: &[
            ("white-shoes", 4),
            ("health-analyzer", 4),
            ("bruise-pack", 4),
            ("ointment", 4),
            ("bottle-antitoxin", 4),
            ("bottle-inaprovaline", 4),
            ("syringe", 4),
            ("latex-gloves", 4),
            ("first-aid-kit", 4),
            ("first-aid-kit-toxin", 4),
            ("first-aid-kit-fire", 4),
            ("first-aid-kit-o2", 4),
        ],
        access: Some("medical"),
        wall: false,
    },
    VendingKind {
        id: "vending-wallmed",
        look: "wallmed",
        delay_ds: 10.0,
        products: &[
            ("bruise-pack", 3),
            ("ointment", 3),
            ("syringe-inaprovaline", 4),
            ("syringe-antitoxin", 4),
            ("health-analyzer", 2),
        ],
        access: None,
        wall: true,
    },
    VendingKind {
        id: "vending-toxins",
        look: "generic",
        delay_ds: 10.0,
        products: &[
            ("screwdriver", 4),
            ("wrench", 4),
            ("wirecutters", 4),
            ("transfer-valve", 4),
            ("igniter-device", 4),
            ("timer", 4),
            ("prox-sensor", 4),
        ],
        access: Some("tox"),
        wall: false,
    },
    VendingKind {
        id: "vending-engineering",
        look: "engi",
        delay_ds: 10.0,
        products: &[
            ("hardhat", 4),
            ("utility-belt", 4),
            ("meson-glasses", 4),
            ("insulated-gloves", 4),
            ("screwdriver", 4),
            ("crowbar", 4),
            ("wirecutters", 4),
            ("multitool", 4),
            ("wrench", 4),
            ("t-ray-scanner", 4),
            ("cable-coil", 4),
            ("power-cell", 4),
            ("welder", 4),
            ("welding-helmet", 4),
            ("light-tube", 4),
            ("firesuit", 4),
        ],
        access: Some("engine"),
        wall: false,
    },
    VendingKind {
        id: "vending-coffee",
        look: "coffee",
        delay_ds: 34.0,
        products: &[("coffee", 25), ("tea", 25), ("hot-chocolate", 25)],
        access: None,
        wall: false,
    },
    VendingKind {
        id: "vending-snack",
        look: "snack",
        delay_ds: 10.0,
        products: &[
            ("candy", 6),
            ("dry-ramen", 6),
            ("chips", 6),
            ("beef-jerky", 6),
            ("no-raisin", 6),
            ("space-twinkie", 6),
            ("cheesie-honkers", 6),
        ],
        access: None,
        wall: false,
    },
    VendingKind {
        id: "vending-cola",
        look: "Cola_Machine",
        delay_ds: 10.0,
        products: COLA,
        access: None,
        wall: false,
    },
    VendingKind {
        id: "vending-soda",
        look: "soda",
        delay_ds: 10.0,
        products: COLA,
        access: None,
        wall: false,
    },
    VendingKind {
        id: "vending-cigarette",
        look: "cigs",
        delay_ds: 34.0,
        products: &[("cigarette-packet", 10), ("matchbox", 10), ("lighter", 4)],
        access: None,
        wall: false,
    },
];

const COLA: &[(&str, u16)] = &[
    ("cola", 10),
    ("space-mountain-wind", 10),
    ("dr-gibb", 10),
    ("starkist", 10),
    ("space-up", 10),
];

pub fn kind(proto: &str) -> Option<&'static VendingKind> {
    KINDS.iter().find(|k| k.id == proto)
}

/// A vending machine: what is left of each product, and what it is handing out.
#[derive(Component, Debug, Clone, PartialEq)]
pub struct Vending {
    pub kind: &'static VendingKind,
    pub stock: Vec<u16>,
    /// The line being handed out, and when it drops out, ds.
    pub vending: Option<(usize, f32)>,
}

/// Vend buttons waiting for the next tick: the machine and the product line.
#[derive(Component, Debug, Default)]
pub struct PendingVendOps(pub Vec<(Entity, usize)>);

/// What the mob's open vending window shows, for the server to send when it changes.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct VendingPanelOut(pub Option<VendingPanelView>);

/// Spawn one, full.
pub fn spawn_vending(world: &mut World, kind: &'static VendingKind, pos: TilePos, id: NetId) -> Entity {
    let mut machine = world.spawn((
        GridPos(pos),
        id,
        Appearance::new(kind.id, kind.look),
        Vending {
            kind,
            stock: kind.products.iter().map(|(_, n)| *n).collect(),
            vending: None,
        },
        PowerUser::new(Channel::Equipment, 0.0),
    ));
    if !kind.wall {
        machine.insert(rs13_world::Dense);
    }
    machine.id()
}

/// Clicking one in reach opens its window, if it has power (Luna `attack_hand`).
pub fn open(
    machines: &Query<(&GridPos, &PowerUser), With<Vending>>,
    mob: Entity,
    mob_pos: TilePos,
    target: Entity,
    commands: &mut Commands,
) -> Option<()> {
    let (pos, power) = machines.get(target).ok()?;
    if in_reach(mob_pos, pos.0) && power.powered {
        commands.entity(mob).insert(PanelViewer(target));
    }
    Some(())
}

/// Vend buttons (Luna `Topic`): one thing at a time, while there is some, in reach and
/// awake; it drops out after the machine's delay.
pub fn process_vend_ops(
    time: Res<WorldTime>,
    mut mobs: Query<(
        &GridPos,
        &mut PendingVendOps,
        Option<&Vitals>,
        Option<&Hands>,
        Option<&Equipment>,
        Option<&mut Outbox>,
    )>,
    mut machines: Query<(&GridPos, &mut Vending, &PowerUser)>,
    cards: Query<&IdCard>,
) {
    let now = time.deciseconds();
    for (pos, mut pending, vitals, hands, equipment, mut outbox) in &mut mobs {
        let awake = vitals.is_none_or(|v| v.stat == MobStat::Conscious);
        for (target, line) in pending.0.drain(..) {
            let Ok((at, mut machine, power)) = machines.get_mut(target) else {
                continue;
            };
            if !awake || !in_reach(pos.0, at.0) || !power.powered || machine.vending.is_some() {
                continue;
            }
            if let Some(access) = machine.kind.access
                && !Shown::of(hands, equipment, &cards).opens(&[access.to_owned()])
            {
                if let Some(outbox) = outbox.as_mut() {
                    outbox.0.push(LocMsg::new("vending-denied"));
                }
                continue;
            }
            let Some(left) = machine.stock.get_mut(line).filter(|n| **n > 0) else {
                continue;
            };
            *left -= 1;
            machine.vending = Some((line, now + machine.kind.delay_ds));
        }
    }
}

/// Drop out what is being handed out once its time comes (Luna `spawn(vend_delay)`).
pub fn run_vending(
    time: Res<WorldTime>,
    mut machines: Query<(&GridPos, &mut Vending)>,
    mut commands: Commands,
) {
    let now = time.deciseconds();
    for (pos, mut machine) in &mut machines {
        let Some((line, at)) = machine.vending else {
            continue;
        };
        if now < at {
            continue;
        }
        machine.vending = None;
        let item = machine.kind.products[line].0;
        let tile = pos.0;
        commands.queue(move |world: &mut World| {
            // Every product is a prototype (`tests/vending.rs` checks).
            let _ = crate::objects::spawn_object(world, &ObjectSpec::new(item), tile);
        });
    }
}

/// Keep each viewer's vending window in step; close it out of reach or without power.
pub fn update_vending_views(
    mut viewers: Query<(
        Entity,
        &GridPos,
        Option<&PanelViewer>,
        &mut VendingPanelOut,
        Option<&Vitals>,
    )>,
    machines: Query<(&GridPos, &Vending, &PowerUser, &NetId)>,
    mut commands: Commands,
) {
    for (mob, pos, viewer, mut out, vitals) in &mut viewers {
        let target = viewer.and_then(|v| machines.get(v.0).ok());
        let view = target.and_then(|(at, machine, power, id)| {
            let usable =
                in_reach(pos.0, at.0) && power.powered && vitals.is_none_or(|v| v.stat == MobStat::Conscious);
            usable.then(|| VendingPanelView {
                target: *id,
                machine: machine.kind.id.to_owned(),
                products: machine
                    .kind
                    .products
                    .iter()
                    .zip(&machine.stock)
                    .map(|((item, _), amount)| VendingRow {
                        item: (*item).to_owned(),
                        amount: *amount,
                    })
                    .collect(),
                ready: machine.vending.is_none(),
            })
        });
        if view.is_none() && target.is_some() {
            commands.entity(mob).remove::<PanelViewer>();
        }
        if out.0 != view {
            out.0 = view;
        }
    }
}

/// Luna `power_change`: `-off` without power. While handing something out, the hot drinks
/// machine plays its `coffee-vend` animation (Luna `icon_vend`).
pub fn update_vending_looks(mut machines: Query<(&Vending, &PowerUser, &mut Appearance)>) {
    for (machine, power, mut look) in &mut machines {
        let state = match (power.powered, machine.vending.is_some()) {
            (false, _) => format!("{}-off", machine.kind.look),
            (true, true) if machine.kind.id == "vending-coffee" => "coffee-vend".to_owned(),
            (true, _) => machine.kind.look.to_owned(),
        };
        if look.state != state {
            look.state = state;
        }
    }
}
