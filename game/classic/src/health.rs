//! Bodies, damage, and whether a mob is conscious. Spec: `docs/reference/health.md`.

use bevy_ecs::prelude::*;
use rs13_world::{Appearance, MobStat};

use crate::rng::GameRng;

/// Health of an undamaged human.
pub const HEALTH_FULL: f32 = 100.0;
/// Below this, a human dies.
pub const DEATH_HEALTH: f32 = -100.0;

/// An external body part.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Zone {
    Head,
    Chest,
    Groin,
    LArm,
    RArm,
    LHand,
    RHand,
    LLeg,
    RLeg,
    LFoot,
    RFoot,
}

impl Zone {
    pub const ALL: [Zone; 11] = [
        Zone::Head,
        Zone::Chest,
        Zone::Groin,
        Zone::LArm,
        Zone::RArm,
        Zone::LHand,
        Zone::RHand,
        Zone::LLeg,
        Zone::RLeg,
        Zone::LFoot,
        Zone::RFoot,
    ];

    /// Luna's `max_damage`: brute plus burn on this part stops here.
    pub fn max_damage(self) -> f32 {
        match self {
            Zone::Chest => 150.0,
            Zone::Groin => 115.0,
            Zone::Head => 125.0,
            Zone::LArm | Zone::RArm | Zone::LLeg | Zone::RLeg => 85.0,
            Zone::LFoot | Zone::RFoot => 51.0,
            Zone::LHand | Zone::RHand => 45.0,
        }
    }

    /// Noun id in the locales, e.g. `zone-l_arm`.
    pub fn noun_id(self) -> &'static str {
        match self {
            Zone::Head => "zone-head",
            Zone::Chest => "zone-chest",
            Zone::Groin => "zone-groin",
            Zone::LArm => "zone-l_arm",
            Zone::RArm => "zone-r_arm",
            Zone::LHand => "zone-l_hand",
            Zone::RHand => "zone-r_hand",
            Zone::LLeg => "zone-l_leg",
            Zone::RLeg => "zone-r_leg",
            Zone::LFoot => "zone-l_foot",
            Zone::RFoot => "zone-r_foot",
        }
    }

    fn index(self) -> usize {
        Zone::ALL.iter().position(|z| *z == self).unwrap()
    }
}

/// Luna's `ran_zone(zone, 90)`: where a blow aimed at `zone` actually lands.
pub fn ran_zone(zone: Zone, rng: &mut GameRng) -> Zone {
    if zone == Zone::Chest {
        if rng.prob(90.0) {
            return Zone::Chest;
        }
        return match rng.range(1, 9) {
            1..=3 => Zone::Head,
            4..=6 => Zone::LArm,
            _ => Zone::RArm,
        };
    }
    if rng.prob(90.0 * 0.75) { zone } else { Zone::Chest }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Organ {
    pub brute: f32,
    pub burn: f32,
}

/// A human body: damage per external organ, plus whole-body damage types.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct Body {
    organs: [Organ; 11],
    pub oxy: f32,
    pub tox: f32,
}

impl Body {
    pub fn organ(&self, zone: Zone) -> Organ {
        self.organs[zone.index()]
    }

    /// Luna's `take_damage`: add damage to one organ, never past its `max_damage`.
    pub fn take_damage(&mut self, zone: Zone, brute: f32, burn: f32) {
        let (brute, burn) = (brute.max(0.0), burn.max(0.0));
        if brute == 0.0 && burn == 0.0 {
            return;
        }
        let organ = &mut self.organs[zone.index()];
        let room = (zone.max_damage() - organ.brute - organ.burn).max(0.0);
        let total = brute + burn;
        let scale = if total <= room { 1.0 } else { room / total };
        organ.brute += brute * scale;
        organ.burn += burn * scale;
    }

    /// Luna's `heal_damage`: take brute and burn off one organ, never below zero.
    pub fn heal(&mut self, zone: Zone, brute: f32, burn: f32) {
        let organ = &mut self.organs[zone.index()];
        organ.brute = (organ.brute - brute.max(0.0)).max(0.0);
        organ.burn = (organ.burn - burn.max(0.0)).max(0.0);
    }

    /// Spread damage evenly over every body part, as whole-body effects (pressure, cold)
    /// do in /tg/station.
    pub fn take_damage_spread(&mut self, brute: f32, burn: f32) {
        let n = Zone::ALL.len() as f32;
        for zone in Zone::ALL {
            self.take_damage(zone, brute / n, burn / n);
        }
    }

    pub fn health(&self) -> f32 {
        let organs: f32 = self.organs.iter().map(|o| o.brute + o.burn).sum();
        HEALTH_FULL - (self.oxy + self.tox + organs)
    }
}

/// A mob's state and current health, kept up to date every tick.
#[derive(Component, Debug, Clone, Copy, PartialEq, Default)]
pub struct Vitals {
    pub health: f32,
    pub stat: MobStat,
}

/// Recompute health and consciousness. Death is permanent.
pub fn update_vitals(mut mobs: Query<(&Body, &mut Vitals, &mut Appearance)>) {
    for (body, mut vitals, mut appearance) in &mut mobs {
        let health = body.health();
        let stat = if vitals.stat == MobStat::Dead || health < DEATH_HEALTH {
            MobStat::Dead
        } else if health < 0.0 {
            MobStat::Unconscious
        } else {
            MobStat::Conscious
        };
        vitals.set_if_neq(Vitals { health, stat });
        let state = match stat {
            MobStat::Conscious => "",
            MobStat::Unconscious => "unconscious",
            MobStat::Dead => "dead",
        };
        if appearance.state != state {
            appearance.state = state.to_owned();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn damage_stops_at_the_organ_limit() {
        let mut body = Body::default();
        body.take_damage(Zone::LHand, 40.0, 0.0);
        body.take_damage(Zone::LHand, 40.0, 0.0);
        assert_eq!(body.organ(Zone::LHand).brute, 45.0);
        assert_eq!(body.health(), 55.0);
    }

    #[test]
    fn mixed_damage_past_the_limit_is_split_in_proportion() {
        let mut body = Body::default();
        body.take_damage(Zone::LFoot, 50.0, 0.0);
        body.take_damage(Zone::LFoot, 3.0, 1.0);
        let foot = body.organ(Zone::LFoot);
        assert!((foot.brute + foot.burn - 51.0).abs() < 1e-4);
    }

    #[test]
    fn ran_zone_matches_luna_odds() {
        let mut rng = GameRng::seeded(7);
        let n = 20_000;
        let chest = (0..n)
            .filter(|_| ran_zone(Zone::Chest, &mut rng) == Zone::Chest)
            .count();
        let ratio = chest as f32 / n as f32;
        assert!((ratio - 0.9).abs() < 0.02, "chest ratio {ratio}");
        let head = (0..n)
            .filter(|_| ran_zone(Zone::Head, &mut rng) == Zone::Head)
            .count();
        let ratio = head as f32 / n as f32;
        assert!((ratio - 0.675).abs() < 0.02, "head ratio {ratio}");
    }
}
