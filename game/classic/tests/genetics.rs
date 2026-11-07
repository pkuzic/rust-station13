//! Genetics: the DNA console and its scanner, injectors, the genes' powers and radiation.
//! Spec: docs/reference/genetics.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::cloning::DnaScanner;
use rs13_classic::closets::InCloset;
use rs13_classic::filing::PendingPicks;
use rs13_classic::flashes::Weakened;
use rs13_classic::genetics::dna::{self, MONKEY_BLOCK};
use rs13_classic::genetics::powers::{self, Gene};
use rs13_classic::genetics::{
    GeneticsConsole, GeneticsPanelOut, Genome, INJECTOR_WARMUP_DS, Injector, Radiation, genome, layout,
};
use rs13_classic::hands::Hands;
use rs13_classic::health::{Body, Zone};
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::power::PowerUser;
use rs13_world::{
    Appearance, DisplayName, GeneOp, GenePage, GridPos, Hidden, HumanLook, ObjectSpec, TileMap, TilePos,
    Vision,
};

const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
    },
    rows: [
        "WWWWWWWWWW",
        "W........W",
        "W........W",
        "W........W",
        "W........W",
        "WWWWWWWWWW",
    ],
)"#;

const SCANNER_AT: TilePos = TilePos::new(2, 2, 0);
const CONSOLE_AT: TilePos = TilePos::new(3, 2, 0);
const GENETICIST_AT: TilePos = TilePos::new(4, 2, 0);

fn tick(app: &mut App, n: u32) {
    for _ in 0..n {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn setup() -> (App, Entity, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    let scanner = spawn_object(app.world_mut(), &ObjectSpec::new("dna-scanner"), SCANNER_AT).unwrap();
    let console = spawn_object(app.world_mut(), &ObjectSpec::new("genetics-console"), CONSOLE_AT).unwrap();
    app.world_mut().get_mut::<PowerUser>(console).unwrap().powered = true;
    tick(&mut app, 1);
    (app, scanner, console)
}

fn person(app: &mut App, at: TilePos) -> Entity {
    let mob = spawn_player_mob(app.world_mut(), at);
    app.world_mut().entity_mut(mob).insert(HumanLook::default());
    mob
}

fn lay_in(app: &mut App, scanner: Entity, who: Entity) {
    app.world_mut()
        .entity_mut(who)
        .insert((InCloset(scanner), Hidden, GridPos(SCANNER_AT)));
    app.world_mut().get_mut::<DnaScanner>(scanner).unwrap().occupant = Some(who);
}

fn press(app: &mut App, mob: Entity, console: Entity, op: GeneOp) {
    app.world_mut().get_mut::<PowerUser>(console).unwrap().powered = true;
    app.world_mut()
        .get_mut::<PendingPicks>(mob)
        .unwrap()
        .0
        .push((console, usize::from(op.line())));
    tick(app, 1);
}

fn act(app: &mut App, mob: Entity, action: Action) {
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(action);
    tick(app, 1);
}

fn told(app: &App, mob: Entity, id: &str) -> bool {
    app.world()
        .get::<Outbox>(mob)
        .unwrap()
        .0
        .iter()
        .any(|m| m.id == id)
}

fn console_state(app: &App, console: Entity) -> GeneticsConsole {
    app.world().get::<GeneticsConsole>(console).unwrap().clone()
}

/// Put `proto` in the mob's active hand.
fn hold(app: &mut App, mob: Entity, proto: &str) -> Entity {
    let at = app.world().get::<GridPos>(mob).unwrap().0;
    let item = spawn_object(app.world_mut(), &ObjectSpec::new(proto), at).unwrap();
    act(app, mob, Action::UseOn(item));
    assert_eq!(app.world().get::<Hands>(mob).unwrap().active_item(), Some(item));
    item
}

/// Set a gene's block on (`FED`) or off (`708`) in someone's enzymes.
fn set_gene(app: &mut App, mob: Entity, gene: Gene, on: bool) {
    let block = layout(app.world_mut()).block(gene);
    let mut g = genome(app.world_mut(), mob).unwrap();
    dna::set_block(&mut g.se, block, if on { "FED" } else { "708" });
    app.world_mut().entity_mut(mob).insert(g);
}

#[test]
fn the_console_shows_the_occupant_and_saves_their_genes_to_a_buffer() {
    let (mut app, scanner, console) = setup();
    let geneticist = person(&mut app, GENETICIST_AT);
    let subject = person(&mut app, SCANNER_AT);
    app.world_mut()
        .entity_mut(subject)
        .insert(DisplayName("Jane Doe".into()));
    lay_in(&mut app, scanner, subject);
    act(&mut app, geneticist, Action::UseOn(console));
    tick(&mut app, 1);
    let view = app
        .world()
        .get::<GeneticsPanelOut>(geneticist)
        .and_then(|o| o.0.clone())
        .expect("the window is open");
    let g = app.world().get::<Genome>(subject).unwrap().clone();
    let occupant = view.occupant.expect("someone in the scanner");
    assert_eq!(occupant.ui, g.ui);
    assert_eq!(occupant.se.len(), 75);
    assert_eq!(occupant.ue, g.ue.to_uppercase());
    assert_eq!(view.page, GenePage::Main);

    press(&mut app, geneticist, console, GeneOp::SaveUiUe(1));
    let buffer = console_state(&app, console).buffers[1].clone().unwrap();
    assert_eq!(buffer.data, g.ui);
    assert!(buffer.with_ue && !buffer.se);
    assert_eq!(buffer.owner, "Jane Doe");

    // Locked, they stay in.
    press(&mut app, geneticist, console, GeneOp::Lock);
    assert!(app.world().get::<DnaScanner>(scanner).unwrap().locked);
    rs13_classic::cloning::leave(app.world_mut(), subject, scanner);
    assert_eq!(
        app.world().get::<DnaScanner>(scanner).unwrap().occupant,
        Some(subject)
    );
}

#[test]
fn irradiating_a_block_changes_one_digit_after_the_beam_and_holds_the_scanner_shut() {
    let (mut app, scanner, console) = setup();
    let geneticist = person(&mut app, GENETICIST_AT);
    let subject = person(&mut app, SCANNER_AT);
    lay_in(&mut app, scanner, subject);
    act(&mut app, geneticist, Action::UseOn(console));
    press(&mut app, geneticist, console, GeneOp::Enzymes);
    press(&mut app, geneticist, console, GeneOp::Select(5, 2));
    let before = genome(app.world_mut(), subject).unwrap().se;
    press(&mut app, geneticist, console, GeneOp::Irradiate);
    assert_eq!(console_state(&app, console).page, GenePage::Working(2));
    assert!(app.world().get::<DnaScanner>(scanner).unwrap().beam);
    // Two seconds of beam.
    tick(&mut app, 45);
    let state = console_state(&app, console);
    assert_eq!(state.page, GenePage::Enzymes);
    assert!(!app.world().get::<DnaScanner>(scanner).unwrap().beam);
    let after = genome(app.world_mut(), subject).unwrap().se;
    let changed: Vec<usize> = before
        .chars()
        .zip(after.chars())
        .enumerate()
        .filter(|(_, (a, b))| a != b)
        .map(|(i, _)| i)
        .collect();
    assert!(changed.len() <= 3, "a beam touches little: {changed:?}");
    assert!(app.world().get::<Radiation>(subject).is_some_and(|r| r.0 > 0.0));
}

#[test]
fn the_replicator_makes_an_injector_that_writes_a_block_into_whoever_it_is_used_on() {
    let (mut app, scanner, console) = setup();
    let geneticist = person(&mut app, GENETICIST_AT);
    let donor = person(&mut app, SCANNER_AT);
    set_gene(&mut app, donor, Gene::Hulk, true);
    lay_in(&mut app, scanner, donor);
    act(&mut app, geneticist, Action::UseOn(console));
    press(&mut app, geneticist, console, GeneOp::SaveSe(0));
    let hulk_block = layout(app.world_mut()).block(Gene::Hulk) as u8;
    press(&mut app, geneticist, console, GeneOp::IsoInjector(0, hulk_block));
    assert_eq!(
        console_state(&app, console).page,
        GenePage::Note("genetics-replicator-not-ready".into())
    );
    tick(&mut app, (INJECTOR_WARMUP_DS * 2.0) as u32 + 2);
    press(&mut app, geneticist, console, GeneOp::IsoInjector(0, hulk_block));
    let world = app.world_mut();
    let mut q = world.query::<(Entity, &Injector)>();
    let (injector, contents) = q.single(world).map(|(e, i)| (e, i.clone())).unwrap();
    assert_eq!(contents.block, Some(usize::from(hulk_block)));
    assert_eq!(contents.data, "FED");

    // Into someone else, after a struggle.
    let other = person(&mut app, TilePos::new(5, 2, 0));
    act(&mut app, geneticist, Action::UseOn(injector));
    act(&mut app, geneticist, Action::UseOn(other));
    assert!(app.world().get_entity(injector).is_ok(), "four seconds to go");
    tick(&mut app, 90);
    assert!(app.world().get_entity(injector).is_err(), "used up");
    assert!(powers::block_on(app.world_mut(), other, Gene::Hulk));
    assert!(app.world().get::<Radiation>(other).is_some_and(|r| r.0 >= 19.0));
}

#[test]
fn a_hulk_shrugs_off_being_knocked_down_until_badly_hurt() {
    let (mut app, ..) = setup();
    let hulk = person(&mut app, GENETICIST_AT);
    set_gene(&mut app, hulk, Gene::Hulk, true);
    powers::check(app.world_mut(), hulk, true);
    assert!(powers::has(app.world(), hulk, Gene::Hulk));
    assert!(told(&app, hulk, "gene-hulk"));
    app.world_mut().entity_mut(hulk).insert(Weakened(1.0e6));
    tick(&mut app, 1);
    assert!(app.world().get::<Weakened>(hulk).is_none());

    app.world_mut()
        .get_mut::<Body>(hulk)
        .unwrap()
        .take_damage(Zone::Chest, 80.0, 0.0);
    tick(&mut app, 45);
    assert!(!powers::has(app.world(), hulk, Gene::Hulk));
    assert!(told(&app, hulk, "gene-hulk-lost"));
}

#[test]
fn the_monkey_block_makes_a_monkey_and_back_with_the_look_kept() {
    let (mut app, ..) = setup();
    let subject = person(&mut app, GENETICIST_AT);
    let look = HumanLook {
        hair: 4,
        hair_rgb: [200, 10, 10],
        ..HumanLook::default()
    };
    app.world_mut().entity_mut(subject).insert(look);
    let mut g = genome(app.world_mut(), subject).unwrap();
    dna::set_block(&mut g.se, MONKEY_BLOCK, "FA0");
    app.world_mut().entity_mut(subject).insert(g.clone());
    powers::check(app.world_mut(), subject, false);
    assert_eq!(app.world().get::<Appearance>(subject).unwrap().sprite, "monkey");
    assert!(app.world().get::<HumanLook>(subject).is_none());

    dna::set_block(&mut g.se, MONKEY_BLOCK, "508");
    app.world_mut().entity_mut(subject).insert(g);
    powers::check(app.world_mut(), subject, false);
    assert_eq!(app.world().get::<Appearance>(subject).unwrap().sprite, "human");
    let back = *app.world().get::<HumanLook>(subject).unwrap();
    assert_eq!((back.hair, back.hair_rgb), (look.hair, look.hair_rgb));
}

#[test]
fn a_ready_made_injector_works_out_its_block_for_this_round() {
    let (mut app, ..) = setup();
    let user = person(&mut app, GENETICIST_AT);
    let target = person(&mut app, TilePos::new(5, 2, 0));
    let injector = hold(&mut app, user, "dna-injector-xray");
    act(&mut app, user, Action::UseOn(target));
    tick(&mut app, 90);
    assert!(app.world().get_entity(injector).is_err());
    assert!(powers::block_on(app.world_mut(), target, Gene::Xray));
}

#[test]
fn radiation_wears_off_and_poisons() {
    let (mut app, ..) = setup();
    let mob = person(&mut app, GENETICIST_AT);
    app.world_mut().entity_mut(mob).insert(Radiation(60.0));
    tick(&mut app, 45);
    let left = app.world().get::<Radiation>(mob).map_or(0.0, |r| r.0);
    assert!(left < 60.0, "{left}");
    assert!(app.world().get::<Body>(mob).unwrap().tox >= 1.0);
}

#[test]
fn transferring_an_identifier_with_its_enzymes_gives_the_name_and_the_look() {
    let (mut app, scanner, console) = setup();
    let geneticist = person(&mut app, GENETICIST_AT);
    let donor = person(&mut app, SCANNER_AT);
    app.world_mut().entity_mut(donor).insert((
        DisplayName("Ann Donor".into()),
        HumanLook {
            hair: 7,
            female: true,
            ..HumanLook::default()
        },
    ));
    lay_in(&mut app, scanner, donor);
    act(&mut app, geneticist, Action::UseOn(console));
    press(&mut app, geneticist, console, GeneOp::SaveUiUe(0));
    // Swap who lies there.
    app.world_mut()
        .entity_mut(donor)
        .remove::<(InCloset, Hidden)>()
        .insert(GridPos(TilePos::new(6, 3, 0)));
    let subject = person(&mut app, SCANNER_AT);
    lay_in(&mut app, scanner, subject);
    press(&mut app, geneticist, console, GeneOp::Transfer(0));
    assert_eq!(app.world().get::<DisplayName>(subject).unwrap().0, "Ann Donor");
    let look = app.world().get::<HumanLook>(subject).unwrap();
    assert_eq!((look.hair, look.female), (7, true));
}

#[test]
fn a_lab_monkey_in_the_scanner_shows_on_the_console_with_the_monkey_block_on() {
    let (mut app, scanner, console) = setup();
    let geneticist = person(&mut app, GENETICIST_AT);
    let monkey = spawn_object(app.world_mut(), &ObjectSpec::new("monkey"), SCANNER_AT).unwrap();
    lay_in(&mut app, scanner, monkey);
    act(&mut app, geneticist, Action::UseOn(console));
    tick(&mut app, 1);
    let view = app
        .world()
        .get::<GeneticsPanelOut>(geneticist)
        .and_then(|o| o.0.clone())
        .unwrap();
    let se = view.occupant.expect("the monkey is read").se;
    let block_add = layout(app.world_mut()).block_add;
    assert!(dna::is_on(
        dna::block(&se, MONKEY_BLOCK),
        dna::Threshold::Normal,
        block_add
    ));
}

#[test]
fn someone_who_never_chose_a_look_still_has_genes() {
    let (mut app, scanner, console) = setup();
    let geneticist = person(&mut app, GENETICIST_AT);
    // As a client that sends no character: no look of their own.
    let plain = spawn_player_mob(app.world_mut(), SCANNER_AT);
    assert!(app.world().get::<HumanLook>(plain).is_none());
    lay_in(&mut app, scanner, plain);
    act(&mut app, geneticist, Action::UseOn(console));
    tick(&mut app, 1);
    let view = app
        .world()
        .get::<GeneticsPanelOut>(geneticist)
        .and_then(|o| o.0.clone())
        .unwrap();
    assert!(view.occupant.is_some(), "read like anyone else");
}

#[test]
fn pulled_into_the_scanner_by_hand_the_console_reads_them() {
    let (mut app, scanner, console) = setup();
    // Beside the scanner and below the console, as a player stands.
    let geneticist = spawn_player_mob(app.world_mut(), TilePos::new(3, 1, 0));
    let subject = spawn_player_mob(app.world_mut(), TilePos::new(4, 1, 0));
    app.world_mut()
        .entity_mut(geneticist)
        .insert(rs13_classic::pulling::Pulling(subject));
    act(&mut app, geneticist, Action::UseOn(scanner));
    assert_eq!(
        app.world().get::<DnaScanner>(scanner).unwrap().occupant,
        Some(subject)
    );
    act(&mut app, geneticist, Action::UseOn(console));
    tick(&mut app, 2);
    let view = app
        .world()
        .get::<GeneticsPanelOut>(geneticist)
        .and_then(|o| o.0.clone())
        .expect("open");
    assert!(view.occupant.is_some(), "{view:?}");
}

#[test]
fn x_ray_and_blindness_change_how_they_see_and_blindness_wins() {
    let (mut app, ..) = setup();
    let mob = person(&mut app, GENETICIST_AT);
    let vision = |app: &App| app.world().get::<Vision>(mob).copied();
    set_gene(&mut app, mob, Gene::Xray, true);
    powers::check(app.world_mut(), mob, true);
    assert_eq!(vision(&app), Some(Vision::Xray));
    set_gene(&mut app, mob, Gene::Blind, true);
    powers::check(app.world_mut(), mob, true);
    assert_eq!(vision(&app), Some(Vision::Blind));
    set_gene(&mut app, mob, Gene::Xray, false);
    set_gene(&mut app, mob, Gene::Blind, false);
    powers::check(app.world_mut(), mob, true);
    assert_eq!(vision(&app), None);
}
