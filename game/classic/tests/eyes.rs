//! How well someone sees: blindness for a while, blurred and nearsighted eyes, the blindfold,
//! glasses, the Blind and Mutate spells, the scrying orb's X-ray and the changeling's sting.
//! Spec: docs/reference/eyes.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::combat::VisibleEvents;
use rs13_classic::eyes;
use rs13_classic::filing::PendingPicks;
use rs13_classic::genetics::powers::{self, Gene};
use rs13_classic::health::Body;
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::spells::{self, SpellOp, SpellPanelOut};
use rs13_classic::wizard_items::{Choice, choices};
use rs13_i18n::LocMsg;
use rs13_world::{EyeTrouble, GridPos, HumanLook, NetId, ObjectSpec, SpellAim, TileMap, TilePos, Vision};

const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
    },
    rows: [
        "WWWWWWWWWWWW",
        "W.....W....W",
        "W.....W....W",
        "W.....W....W",
        "WWWWWWWWWWWW",
    ],
)"#;

fn setup() -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    tick(&mut app, 1);
    app
}

fn tick(app: &mut App, n: u32) {
    for _ in 0..n {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

/// Ticks for `ds` deciseconds to pass, and a couple more.
fn wait_ds(app: &mut App, ds: f32) {
    tick(app, (ds * rs13_core::TICK_RATE_HZ as f32 / 10.0) as u32 + 2);
}

fn person(app: &mut App, x: i32, y: i32) -> Entity {
    let mob = spawn_player_mob(app.world_mut(), TilePos::new(x, y, 0));
    app.world_mut().entity_mut(mob).insert(HumanLook::default());
    mob
}

fn wear(app: &mut App, mob: Entity, proto: &str) -> Entity {
    let at = app.world().get::<GridPos>(mob).unwrap().0;
    let item = spawn_object(app.world_mut(), &ObjectSpec::new(proto), at).unwrap();
    rs13_classic::jobs::wear(app.world_mut(), mob, item);
    item
}

fn vision(app: &App, mob: Entity) -> Option<Vision> {
    app.world().get::<Vision>(mob).copied()
}

fn trouble(app: &App, mob: Entity) -> EyeTrouble {
    app.world().get::<EyeTrouble>(mob).copied().unwrap_or_default()
}

fn told(app: &App, mob: Entity, id: &str) -> bool {
    app.world()
        .get::<Outbox>(mob)
        .unwrap()
        .0
        .iter()
        .any(|m| m.id == id)
}

fn cast(app: &mut App, mob: Entity, line: u8, target: Option<Entity>) {
    spells::command(
        app.world_mut(),
        mob,
        SpellOp::Cast {
            spell: line,
            target,
            area: None,
        },
    );
    tick(app, 1);
}

#[test]
fn blindness_blur_and_nearsightedness_wear_off() {
    let mut app = setup();
    let mob = person(&mut app, 2, 2);
    assert_eq!(vision(&app, mob), None);
    eyes::blind(app.world_mut(), mob, 1.0);
    eyes::blur(app.world_mut(), mob, 2.0);
    eyes::nearsight(app.world_mut(), mob, 60.0);
    tick(&mut app, 1);
    assert_eq!(vision(&app, mob), Some(Vision::Blind));
    assert_eq!(
        trouble(&app, mob),
        EyeTrouble {
            blurry: true,
            nearsighted: true
        }
    );
    // One life tick of blindness.
    wait_ds(&mut app, eyes::COUNT_DS);
    assert_eq!(vision(&app, mob), None, "sees again");
    assert!(trouble(&app, mob).blurry, "still blurred");
    wait_ds(&mut app, 2.0 * eyes::COUNT_DS);
    assert_eq!(trouble(&app, mob), EyeTrouble::default());
    assert!(app.world().get::<EyeTrouble>(mob).is_none());
}

#[test]
fn a_blindfold_blinds_and_glasses_put_nearsightedness_right() {
    let mut app = setup();
    let mob = person(&mut app, 2, 2);
    wear(&mut app, mob, "blindfold");
    tick(&mut app, 1);
    assert_eq!(vision(&app, mob), Some(Vision::Blind));

    let other = person(&mut app, 3, 2);
    eyes::nearsight(app.world_mut(), other, 600.0);
    tick(&mut app, 1);
    assert!(trouble(&app, other).nearsighted);
    wear(&mut app, other, "glasses");
    tick(&mut app, 1);
    assert!(!trouble(&app, other).nearsighted, "glasses help");
    assert_eq!(vision(&app, other), None);
}

#[test]
fn the_blind_see_nothing_happen_and_x_ray_sees_through_walls() {
    let mut app = setup();
    let blind = person(&mut app, 2, 2);
    let plain = person(&mut app, 3, 2);
    let xray = person(&mut app, 8, 2);
    let behind = person(&mut app, 9, 2);
    eyes::blind(app.world_mut(), blind, 10.0);
    powers::grant(app.world_mut(), xray, &[Gene::Xray], f32::INFINITY);
    app.world_mut().resource_mut::<VisibleEvents>().0.push((
        TilePos::new(4, 2, 0),
        LocMsg::new("seen-test"),
        None,
    ));
    tick(&mut app, 1);
    assert!(told(&app, plain, "seen-test"));
    assert!(!told(&app, blind, "seen-test"), "the blind see nothing");
    assert!(told(&app, xray, "seen-test"), "X-ray, through the wall");
    assert!(!told(&app, behind, "seen-test"), "a wall in the way");
}

#[test]
fn the_blind_spell_blinds_someone_in_view_and_no_one_else() {
    let mut app = setup();
    let wizard = person(&mut app, 2, 2);
    let target = person(&mut app, 4, 2);
    let away = person(&mut app, 8, 2);
    spells::give(app.world_mut(), wizard, "blind");
    spells::command(app.world_mut(), wizard, SpellOp::Open);
    tick(&mut app, 1);
    let view = app
        .world()
        .get::<SpellPanelOut>(wizard)
        .unwrap()
        .0
        .clone()
        .unwrap();
    assert_eq!(view.spells[0].aim, SpellAim::Someone);
    let target_id = *app.world().get::<NetId>(target).unwrap();
    assert_eq!(
        view.nearby.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
        [target_id],
        "only who's in view"
    );

    // Behind the wall: nothing, and the charge comes back.
    cast(&mut app, wizard, 0, Some(away));
    assert_eq!(vision(&app, away), None);
    cast(&mut app, wizard, 0, Some(target));
    assert_eq!(vision(&app, target), Some(Vision::Blind));
    assert_eq!(
        trouble(&app, target),
        EyeTrouble {
            blurry: true,
            nearsighted: true
        }
    );
    assert!(told(&app, target, "spell-blind-eyes"));
    // Ten life ticks, no robes needed.
    wait_ds(&mut app, spells::BLIND_COUNTS * eyes::COUNT_DS);
    assert_eq!(vision(&app, target), None);
}

#[test]
fn mutate_gives_its_powers_for_thirty_seconds() {
    let mut app = setup();
    let wizard = person(&mut app, 2, 2);
    for proto in ["wizard-robe", "sandals", "wizard-hat"] {
        wear(&mut app, wizard, proto);
    }
    spells::give(app.world_mut(), wizard, "mutate");
    cast(&mut app, wizard, 0, None);
    for gene in spells::MUTATE_GENES {
        assert!(powers::has(app.world(), wizard, gene), "{gene:?}");
    }
    assert_eq!(vision(&app, wizard), Some(Vision::Xray));
    assert!(app.world().get::<Body>(wizard).unwrap().burn_proof);
    assert!(told(&app, wizard, "spell-mutate-strong"));
    // A reading of the enzymes keeps them.
    powers::check(app.world_mut(), wizard, false);
    assert!(powers::has(app.world(), wizard, Gene::Hulk));
    wait_ds(&mut app, spells::MUTATE_DS);
    for gene in spells::MUTATE_GENES {
        assert!(!powers::has(app.world(), wizard, gene), "{gene:?} wore off");
    }
    assert_eq!(vision(&app, wizard), None);
    assert!(!app.world().get::<Body>(wizard).unwrap().burn_proof);
}

#[test]
fn buying_the_scrying_orb_gives_x_ray_for_good() {
    let mut app = setup();
    let wizard = person(&mut app, 2, 2);
    let at = TilePos::new(2, 2, 0);
    let book = spawn_object(app.world_mut(), &ObjectSpec::new("spellbook"), at).unwrap();
    app.world_mut()
        .get_mut::<PendingActions>(wizard)
        .unwrap()
        .0
        .push(Action::UseOn(book));
    tick(&mut app, 2);
    app.world_mut()
        .get_mut::<PendingActions>(wizard)
        .unwrap()
        .0
        .push(Action::UseOn(book));
    tick(&mut app, 2);
    let line = choices().iter().position(|c| *c == Choice::Scrying).unwrap();
    app.world_mut()
        .get_mut::<PendingPicks>(wizard)
        .unwrap()
        .0
        .push((book, line));
    tick(&mut app, 1);
    assert!(told(&app, wizard, "book-bought-orb"));
    assert!(told(&app, wizard, "gene-xray"));
    assert_eq!(vision(&app, wizard), Some(Vision::Xray));
    wait_ds(&mut app, 3000.0);
    assert_eq!(vision(&app, wizard), Some(Vision::Xray), "for good");
}
