//! Bruise packs, ointment and health analyzers. Spec: `docs/reference/medical.md`.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_simulation::atmos::gas::T0C;
use rs13_world::{Appearance, DisplayName, GridPos};

use crate::actions::{Outbox, in_reach};
use crate::combat::VisibleEvents;
use crate::environment::Lungs;
use crate::hands::{Hands, Stack};
use crate::health::{Body, Vitals, Zone};

/// The health analyzer prototype.
pub const ANALYZER: &str = "health-analyzer";
/// Luna's analyzer calls damage of one kind over this "severe".
pub const SEVERE_DAMAGE: f32 = 50.0;
/// Without a zone selector, treatment goes on the chest (Luna's default `zone_sel`).
const TREATED_ZONE: Zone = Zone::Chest;

/// Brute and burn a medical pack heals (Luna `heal_brute`, `heal_burn`).
pub fn heals(proto: &str) -> Option<(f32, f32)> {
    match proto {
        "bruise-pack" => Some((30.0, 0.0)),
        "ointment" => Some((0.0, 20.0)),
        _ => None,
    }
}

/// Whether using `proto` on a person treats or scans them rather than hitting them.
pub fn treats(proto: &str) -> bool {
    proto == ANALYZER || heals(proto).is_some()
}

/// Medical items used on people this tick: who, on whom.
#[derive(Resource, Debug, Default)]
pub struct PendingTreatments(pub Vec<(Entity, Entity)>);

/// Apply packs and run scans (Luna `stack/medical/attack`, `healthanalyzer/attack`).
#[allow(clippy::type_complexity)]
pub fn resolve_treatments(
    mut pending: ResMut<PendingTreatments>,
    mut visible: ResMut<VisibleEvents>,
    mut medics: Query<(&GridPos, &mut Hands, &DisplayName, &mut Outbox)>,
    mut patients: Query<(&GridPos, &mut Body, &Vitals, &DisplayName, Option<&Lungs>)>,
    mut items: Query<(&Appearance, Option<&mut Stack>)>,
    mut commands: Commands,
) {
    for (medic, patient) in pending.0.drain(..) {
        let Ok((from, mut hands, medic_name, mut outbox)) = medics.get_mut(medic) else {
            continue;
        };
        let Some(item) = hands.active_item() else { continue };
        let Ok((look, stack)) = items.get_mut(item) else {
            continue;
        };
        let Ok((to, mut body, vitals, patient_name, lungs)) = patients.get_mut(patient) else {
            continue;
        };
        if !in_reach(from.0, to.0) {
            continue;
        }
        let (name, target, at) = (medic_name.0.clone(), patient_name.0.clone(), to.0);
        if look.sprite == ANALYZER {
            outbox.0.extend(scan(&body, vitals, lungs, &target));
            let seen = LocMsg::new("analyzer-used")
                .arg("name", name)
                .arg("target", target);
            visible.0.push((at, seen, Some(medic)));
            continue;
        }
        let Some((brute, burn)) = heals(&look.sprite) else {
            continue;
        };
        // Luna won't treat someone in critical condition with a plain pack.
        if vitals.health < 0.0 {
            continue;
        }
        body.heal(TREATED_ZONE, brute, burn);
        let msg = if medic == patient {
            LocMsg::new("medical-applied-self")
        } else {
            LocMsg::new("medical-applied").arg("target", target)
        };
        visible
            .0
            .push((at, msg.arg("name", name).arg("item", look.sprite.clone()), None));
        // Use one from the stack; the last one is gone.
        if let Some(mut stack) = stack {
            stack.amount = stack.amount.saturating_sub(1);
            if stack.amount == 0 {
                commands.entity(item).despawn();
                if hands.left == Some(item) {
                    hands.left = None;
                }
                if hands.right == Some(item) {
                    hands.right = None;
                }
            }
        }
    }
}

/// What the analyzer tells its user (Luna `healthanalyzer/attack`).
pub fn scan(body: &Body, vitals: &Vitals, lungs: Option<&Lungs>, target: &str) -> Vec<LocMsg> {
    let organs = Zone::ALL.map(|z| (z, body.organ(z)));
    let brute: f32 = organs.iter().map(|(_, o)| o.brute).sum();
    let burn: f32 = organs.iter().map(|(_, o)| o.burn).sum();
    let round = |v: f32| v.round() as i64;
    let dead = vitals.stat == rs13_world::MobStat::Dead;
    let mut out = vec![
        LocMsg::new("analyzer-used-self").arg("target", target),
        LocMsg::new("analyzer-status")
            .arg("target", target)
            .arg("dead", if dead { "yes" } else { "no" })
            .num("health", round(vitals.health)),
        LocMsg::new("analyzer-damage")
            .num("oxy", round(body.oxy))
            .num("tox", round(body.tox))
            .num("burn", round(burn))
            .num("brute", round(brute)),
    ];
    if let Some(lungs) = lungs {
        let kelvin = f64::from(lungs.body_temperature);
        out.push(
            LocMsg::new("analyzer-temperature")
                .num("celsius", (kelvin - T0C).round() as i64)
                .num("fahrenheit", (kelvin * 1.8 - 459.67).round() as i64),
        );
    }
    out.push(LocMsg::new("analyzer-localized"));
    let damaged: Vec<_> = organs
        .iter()
        .filter(|(_, o)| o.brute > 0.0 || o.burn > 0.0)
        .collect();
    if damaged.is_empty() {
        out.push(LocMsg::new("analyzer-limbs-ok"));
    }
    for (zone, organ) in damaged {
        out.push(
            LocMsg::new("analyzer-organ")
                .arg("zone", zone.noun_id())
                .num("brute", round(organ.brute))
                .num("burn", round(organ.burn)),
        );
    }
    let severe = |v: f32| if v > SEVERE_DAMAGE { "severe" } else { "ok" };
    out.push(
        LocMsg::new("analyzer-summary")
            .arg("oxy", severe(body.oxy))
            .arg("tox", severe(body.tox))
            .arg("burn", severe(burn))
            .arg("brute", severe(brute)),
    );
    out
}
