//! Snacks, drinks and hunger. Spec: `docs/reference/food.md`.
//!
//! Luna's food is reagents: each bite or gulp moves some into the eater, and they feed it
//! as they metabolise. There are no reagents here yet, so each bite or gulp gives its share
//! of the nutrition those reagents would have given, at once.

use bevy_ecs::prelude::*;
use bevy_ecs::system::SystemParam;
use rs13_i18n::LocMsg;
use rs13_world::{Appearance, GridPos, MobStat, ObjectSpec, TilePos};

use crate::actions::{ClickOutcome, in_reach};
use crate::environment::LIFE_TICKS;
use crate::hands::{Hands, HeldBy};
use crate::health::Vitals;
use crate::rng::GameRng;
use crate::sound::SoundEvents;
use crate::time::WorldTime;

/// How fed a mob is (Luna `nutrition`): it starts at 1 600 and drops by 2 every life cycle.
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Nutrition {
    pub level: f32,
    /// When the last hunger message was, ds.
    pub last_pang: Option<f32>,
}

impl Default for Nutrition {
    fn default() -> Self {
        Self {
            level: STARTING_NUTRITION,
            last_pang: None,
        }
    }
}

pub const STARTING_NUTRITION: f32 = 1600.0;
/// Nutrition lost every life cycle (Luna: 1, twice, in `human/Life`).
pub const HUNGER_PER_CYCLE: f32 = 2.0;
/// Time between hunger messages at nothing left, ds (Luna 6 000).
pub const PANG_DS: f32 = 6000.0;
/// Nutrition per unit of each reagent, as Luna's `on_mob_life` gives it over time.
const NUTRIMENT: f32 = 15.0;
const SUGAR: f32 = 2.5;
const SODA: f32 = 2.5;
const JUICE: f32 = 3.5;

/// A snack: what is left of it, what each bite gives, what it leaves (Luna's `snacks`).
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Snack {
    pub bites_left: u8,
    pub bites_taken: u8,
    pub per_bite: f32,
    pub trash: Option<&'static str>,
    /// The verb it is eaten with, picked at the first bite (Luna `eatverb`).
    pub verb: Option<&'static str>,
}

/// A drink: units left, and what each unit gives (Luna's `drinks`).
#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub struct Drink {
    pub units: f32,
    pub volume: f32,
    pub per_unit: f32,
}

/// The snack or drink a new item of prototype `proto` is.
pub fn spawn_food(entity: &mut EntityWorldMut, proto: &str) {
    // (units, bite size, nutrition of the whole, trash): Luna's reagents, with every
    // snack's 8 nutriment from `snacks/New` included.
    let snack = |units: f32, bite: f32, nutrition: f32, trash: Option<&'static str>| {
        let bites = (units / bite).ceil() as u8;
        Snack {
            bites_left: bites,
            bites_taken: 0,
            per_bite: nutrition / f32::from(bites),
            trash,
            verb: None,
        }
    };
    let drink = |volume: f32, units: f32, nutrition: f32| Drink {
        units,
        volume,
        per_unit: nutrition / units,
    };
    match proto {
        "candy" => {
            entity.insert(snack(
                12.0,
                2.0,
                9.0 * NUTRIMENT + 3.0 * SUGAR,
                Some("trash-candy"),
            ));
        }
        crate::microwaves::DONK_POCKET => {
            entity.insert(snack(12.0, 2.0, 12.0 * NUTRIMENT, None));
        }
        crate::microwaves::BURNT_MESS => {
            entity.insert(snack(12.0, 2.0, 8.0 * NUTRIMENT, None));
        }
        "chips" => {
            entity.insert(snack(11.0, 1.0, 11.0 * NUTRIMENT, Some("trash-chips")));
        }
        "beef-jerky" => {
            entity.insert(snack(12.0, 2.0, 12.0 * NUTRIMENT, Some("trash-sosjerky")));
        }
        "no-raisin" => {
            entity.insert(snack(14.0, 1.0, 14.0 * NUTRIMENT, Some("trash-raisins")));
        }
        "space-twinkie" => {
            entity.insert(snack(14.0, 2.0, 10.0 * NUTRIMENT + 4.0 * SUGAR, None));
        }
        "cheesie-honkers" => {
            entity.insert(snack(12.0, 2.0, 12.0 * NUTRIMENT, Some("trash-cheesie")));
        }
        "coffee" => {
            entity.insert(drink(50.0, 30.0, 0.0));
        }
        "tea" => {
            entity.insert(drink(30.0, 30.0, 0.0));
        }
        "hot-chocolate" => {
            entity.insert(drink(30.0, 30.0, 30.0 * 2.0));
        }
        "dry-ramen" => {
            entity.insert(drink(50.0, 30.0, 30.0));
        }
        "cola" | "space-mountain-wind" | "space-up" => {
            entity.insert(drink(50.0, 30.0, 30.0 * SODA));
        }
        "dr-gibb" | "starkist" => {
            entity.insert(drink(50.0, 30.0, 20.0 * SODA + 10.0 * JUICE));
        }
        _ => {}
    }
}

/// Luna's eating verbs.
const VERBS: [&str; 6] = ["bite", "chew", "nibble", "gnaw", "gobble", "chomp"];

/// Everything eating and drinking needs.
#[derive(SystemParam)]
pub struct FoodWork<'w, 's> {
    snacks: Query<'w, 's, &'static mut Snack>,
    drinks: Query<'w, 's, (&'static mut Drink, &'static Appearance)>,
    looks: Query<'w, 's, &'static Appearance>,
    nutrition: Query<'w, 's, &'static mut Nutrition>,
}

impl FoodWork<'_, '_> {
    pub fn is_food(&self, item: Option<Entity>) -> bool {
        item.is_some_and(|i| self.snacks.contains(i) || self.drinks.contains(i))
    }

    /// Luna `snacks/attack` and `drinks/attack`: the food in the active hand used on a
    /// person: yourself to eat or drink, someone in reach to feed them. Returns the message
    /// for the mob and the one everyone sees (wanting `$name`), if any.
    #[allow(clippy::too_many_arguments)]
    pub fn use_on(
        &mut self,
        mob: Entity,
        mob_pos: TilePos,
        target: Entity,
        target_pos: TilePos,
        target_name: &str,
        hands: &mut Hands,
        (rng, sounds): (&mut GameRng, &mut SoundEvents),
        commands: &mut Commands,
    ) -> Option<ClickOutcome> {
        let item = hands.active_item()?;
        if target != mob && !in_reach(mob_pos, target_pos) {
            return Some(ClickOutcome::default());
        }
        let noun = self.looks.get(item).map(|a| a.sprite.clone()).unwrap_or_default();
        if let Ok(mut snack) = self.snacks.get_mut(item) {
            let verb = *snack.verb.get_or_insert_with(|| VERBS[rng.range(0, 5) as usize]);
            let mut to_mob = None;
            if target == mob {
                let fullness = self.nutrition.get(mob).map_or(0.0, |n| n.level / 2.0);
                let id = match fullness {
                    f if f <= 100.0 => Some("food-eat-gobble"),
                    f if f < 250.0 => Some("food-eat-begin"),
                    f if f > 250.0 && f < 550.0 => Some("food-eat"),
                    f if f > 550.0 && f < 850.0 => Some("food-eat-unwilling"),
                    f if f > 850.0 => {
                        return Some(ClickOutcome {
                            to_mob: Some(LocMsg::new("food-eat-full").arg("item", noun)),
                            seen: None,
                        });
                    }
                    _ => None,
                };
                to_mob = id.map(|id| LocMsg::new(id).arg("item", noun.clone()).arg("verb", verb));
            }
            sounds.play(crate::sound::EAT_FOOD, target_pos, 30.0);
            if let Ok(mut fed) = self.nutrition.get_mut(target) {
                fed.level += snack.per_bite;
                // Warm Donk-pockets do you good (`microwaves.md`).
                commands.queue(move |world: &mut World| {
                    if world.get::<crate::microwaves::Warm>(item).is_some()
                        && let Some(mut body) = world.get_mut::<crate::health::Body>(target)
                    {
                        crate::microwaves::warm_bite(&mut body);
                    }
                });
            }
            snack.bites_left = snack.bites_left.saturating_sub(1);
            snack.bites_taken += 1;
            if snack.bites_left == 0 {
                // Eaten up: its wrapper ends up in the feeder's hand (Luna `On_Consume`).
                let hand = hands.active;
                *hands.slot_mut(hand) = None;
                commands.entity(item).despawn();
                if let Some(trash) = snack.trash {
                    commands.queue(move |world: &mut World| {
                        let Ok(wrapper) =
                            crate::objects::spawn_object(world, &ObjectSpec::new(trash), mob_pos)
                        else {
                            return;
                        };
                        world.entity_mut(wrapper).remove::<GridPos>().insert(HeldBy(mob));
                        if let Some(mut hands) = world.get_mut::<Hands>(mob) {
                            *hands.slot_mut(hand) = Some(wrapper);
                        }
                    });
                }
            }
            return Some(ClickOutcome { to_mob, seen: None });
        }
        let (mut drink, _) = self.drinks.get_mut(item).ok()?;
        if drink.units <= 0.0 {
            return Some(ClickOutcome::told("drink-empty").with_item(&noun));
        }
        // Luna's gulp: a fifth of what is left, at least 5 units.
        let gulp = (drink.units / 5.0).round().max(5.0).min(drink.units);
        drink.units -= gulp;
        let gained = gulp * drink.per_unit;
        if let Ok(mut fed) = self.nutrition.get_mut(target) {
            fed.level += gained;
        }
        sounds.play(crate::sound::DRINK, target_pos, 30.0);
        Some(if target == mob {
            ClickOutcome::told("drink-gulp").with_item(&noun)
        } else {
            ClickOutcome {
                to_mob: None,
                seen: Some(
                    LocMsg::new("drink-feeds")
                        .arg("item", noun)
                        .arg("target", target_name.to_owned()),
                ),
            }
        })
    }

    /// What examining a snack or a drink adds.
    pub fn examine(&self, target: Entity) -> Option<LocMsg> {
        let noun = self
            .looks
            .get(target)
            .map(|a| a.sprite.clone())
            .unwrap_or_default();
        if let Ok(snack) = self.snacks.get(target) {
            let id = match snack.bites_taken {
                0 => return None,
                1 => "examine-bitten-once",
                2 | 3 => "examine-bitten",
                _ => "examine-bitten-many",
            };
            return Some(
                LocMsg::new(id)
                    .arg("item", noun)
                    .num("count", i64::from(snack.bites_taken)),
            );
        }
        let (drink, _) = self.drinks.get(target).ok()?;
        let share = drink.units / drink.volume;
        let level = match share {
            s if s <= 0.0 => "empty",
            s if s <= 0.25 => "almost-empty",
            s if s <= 0.66 => "half",
            s if s <= 0.9 => "almost-full",
            _ => "full",
        };
        Some(LocMsg::new("examine-drink").arg("item", noun).arg("level", level))
    }
}

impl ClickOutcome {
    fn with_item(mut self, noun: &str) -> Self {
        self.to_mob = self.to_mob.map(|m| m.arg("item", noun.to_owned()));
        self
    }
}

/// Every life cycle the living grow hungrier; with nothing left, a pang now and then
/// (Luna `human/Life`, `handle_chemicals_in_body`).
pub fn get_hungry(
    time: Res<WorldTime>,
    mut rng: ResMut<GameRng>,
    mut mobs: Query<(&mut Nutrition, &mut crate::actions::Outbox, Option<&Vitals>)>,
) {
    if !time.tick.0.is_multiple_of(LIFE_TICKS) {
        return;
    }
    let now = time.deciseconds();
    for (mut fed, mut outbox, vitals) in &mut mobs {
        if vitals.is_some_and(|v| v.stat == MobStat::Dead) {
            continue;
        }
        fed.level = (fed.level - HUNGER_PER_CYCLE).max(0.0);
        if fed.level <= 0.0 && fed.last_pang.is_none_or(|t| now >= t + PANG_DS) {
            fed.last_pang = Some(now);
            let n = rng.range(1, 9);
            outbox.0.push(LocMsg::new(format!("hunger-{n}")));
        }
    }
}
