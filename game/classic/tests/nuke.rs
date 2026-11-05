//! The nuclear emergency. Spec: docs/reference/nuke.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::hands::{Hands, HeldBy};
use rs13_classic::jobs::HasJob;
use rs13_classic::nuke::{
    Landmark, NuclearBomb, NuclearDisk, NukeAgent, NukeOp, NukeState, Pinpointer, WantsNuke, apply, click,
    pin_state, report, setup_round, spawn_bomb, team_size, use_pinpointer,
};
use rs13_classic::objects::{NetIds, spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::round::{Phase, Round};
use rs13_world::{Appearance, DisplayName, GridPos, ObjectSpec, TileMap, TilePos};

const STATION: TilePos = TilePos::new(2, 2, 0);

const MAP: &str = r#"(
    legend: {
        '.': (tile: (id: "floor")),
    },
    rows: [
        "....................",
        "....................",
        "....................",
        "....................",
        "....................",
        "....................",
        "....................",
        "....................",
        "....................",
        "....................",
        "....................",
        "....................",
        "....................",
        "....................",
        "....................",
        "....................",
        "....................",
        "....................",
        "....................",
        "....................",
    ],
)"#;

fn setup() -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    app.world_mut().run_schedule(FixedUpdate);
    app
}

fn ticks(app: &mut App, n: u32) {
    for _ in 0..n {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn crew(app: &mut App, name: &str) -> Entity {
    let mob = spawn_player_mob(app.world_mut(), STATION);
    app.world_mut()
        .entity_mut(mob)
        .insert((HasJob("clown".to_owned()), DisplayName(name.to_owned())));
    mob
}

/// A bomb beside someone with the disk in hand, with the code 12345.
fn bomb_and_disk(app: &mut App) -> (Entity, Entity, Entity) {
    let mob = crew(app, "Agent");
    let id = app.world_mut().resource_mut::<NetIds>().alloc();
    let bomb = spawn_bomb(app.world_mut(), TilePos::new(3, 2, 0), id, "12345");
    let disk = spawn_object(app.world_mut(), &ObjectSpec::new("nuclear-disk"), STATION).unwrap();
    app.world_mut()
        .entity_mut(disk)
        .remove::<GridPos>()
        .insert(HeldBy(mob));
    let mut hands = app.world_mut().get_mut::<Hands>(mob).unwrap();
    let hand = hands.active;
    *hands.slot_mut(hand) = Some(disk);
    (mob, bomb, disk)
}

fn type_code(app: &mut App, mob: Entity, bomb: Entity, code: &str) {
    for digit in code.bytes() {
        apply(app.world_mut(), mob, bomb, NukeOp::Key(digit - b'0'));
    }
    apply(app.world_mut(), mob, bomb, NukeOp::Enter);
}

fn state(app: &App, bomb: Entity) -> NuclearBomb {
    app.world().get::<NuclearBomb>(bomb).unwrap().clone()
}

#[test]
fn the_team_is_a_third_of_the_players_within_limits() {
    assert_eq!(team_size(1), 1);
    assert_eq!(team_size(6), 2);
    assert_eq!(team_size(40), 5);
}

#[test]
fn the_first_click_bolts_it_down_and_the_next_opens_it() {
    let mut app = setup();
    let (mob, bomb, _) = bomb_and_disk(&mut app);
    click(app.world_mut(), mob, bomb);
    assert!(state(&app, bomb).extended);
    assert!(state(&app, bomb).anchored);
    assert_eq!(app.world().get::<Appearance>(bomb).unwrap().state, "1");
    assert!(app.world().get::<rs13_classic::wires::PanelViewer>(mob).is_none());
    click(app.world_mut(), mob, bomb);
    assert!(app.world().get::<rs13_classic::wires::PanelViewer>(mob).is_some());
}

#[test]
fn nothing_works_without_the_disk_and_the_right_code() {
    let mut app = setup();
    let (mob, bomb, disk) = bomb_and_disk(&mut app);
    type_code(&mut app, mob, bomb, "12345");
    assert!(!state(&app, bomb).code_ok, "no disk in it");

    apply(app.world_mut(), mob, bomb, NukeOp::Disk);
    assert_eq!(state(&app, bomb).disk, Some(disk));
    assert!(app.world().get::<Hands>(mob).unwrap().active_item().is_none());

    type_code(&mut app, mob, bomb, "99999");
    assert_eq!(state(&app, bomb).code, "ERROR");
    apply(app.world_mut(), mob, bomb, NukeOp::Timer);
    assert!(!state(&app, bomb).timing, "the wrong code");

    apply(app.world_mut(), mob, bomb, NukeOp::Reset);
    type_code(&mut app, mob, bomb, "12345");
    assert!(state(&app, bomb).code_ok);
    apply(app.world_mut(), mob, bomb, NukeOp::Time(-100));
    assert_eq!(state(&app, bomb).time_left, 5);
    apply(app.world_mut(), mob, bomb, NukeOp::Time(10_000));
    assert_eq!(state(&app, bomb).time_left, 600);

    // Taking it out forgets the code, and leaves it on the floor.
    apply(app.world_mut(), mob, bomb, NukeOp::Disk);
    assert!(!state(&app, bomb).code_ok);
    assert_eq!(app.world().get::<GridPos>(disk).unwrap().0, TilePos::new(3, 2, 0));
}

#[test]
fn six_digits_is_an_error() {
    let mut app = setup();
    let (mob, bomb, _) = bomb_and_disk(&mut app);
    apply(app.world_mut(), mob, bomb, NukeOp::Disk);
    for _ in 0..6 {
        apply(app.world_mut(), mob, bomb, NukeOp::Key(1));
    }
    assert_eq!(state(&app, bomb).code, "ERROR");
}

#[test]
fn someone_out_of_reach_cant_press_anything() {
    let mut app = setup();
    let (mob, bomb, _) = bomb_and_disk(&mut app);
    app.world_mut()
        .entity_mut(mob)
        .insert(GridPos(TilePos::new(15, 15, 0)));
    apply(app.world_mut(), mob, bomb, NukeOp::Disk);
    assert_eq!(state(&app, bomb).disk, None);
}

fn arm(app: &mut App, safety: bool) -> Entity {
    let (mob, bomb, _) = bomb_and_disk(app);
    apply(app.world_mut(), mob, bomb, NukeOp::Disk);
    type_code(app, mob, bomb, "12345");
    if !safety {
        apply(app.world_mut(), mob, bomb, NukeOp::Safety);
    }
    apply(app.world_mut(), mob, bomb, NukeOp::Timer);
    app.world_mut().get_mut::<NuclearBomb>(bomb).unwrap().time_left = 1;
    bomb
}

fn advance(app: &mut App, deciseconds: f32) {
    ticks(app, (deciseconds / rs13_classic::time::TICK_DS).ceil() as u32);
}

#[test]
fn with_the_safety_on_the_timer_just_stops() {
    let mut app = setup();
    let bomb = arm(&mut app, true);
    for _ in 0..3 {
        advance(&mut app, 15.0);
    }
    assert!(!state(&app, bomb).timing);
    assert!(!app.world().resource::<NukeState>().detonated);
}

#[test]
fn with_the_safety_off_it_goes_off_and_cant_be_stopped() {
    let mut app = setup();
    let bomb = arm(&mut app, false);
    advance(&mut app, 15.0);
    assert!(state(&app, bomb).blast_at.is_some());
    assert_eq!(app.world().get::<Appearance>(bomb).unwrap().state, "3");
    assert!(
        !app.world().resource::<NukeState>().detonated,
        "not for two seconds"
    );
    advance(&mut app, 25.0);
    assert!(app.world().resource::<NukeState>().detonated);
}

#[test]
fn a_pinpointer_tells_how_far_the_disk_is() {
    let at = TilePos::new(10, 10, 0);
    assert_eq!(pin_state(at, at), "direct");
    assert_eq!(pin_state(at, TilePos::new(18, 10, 0)), "close");
    assert_eq!(pin_state(at, TilePos::new(10, 26, 0)), "medium");
    assert_eq!(pin_state(at, TilePos::new(40, 10, 0)), "far");

    let mut app = setup();
    let mob = crew(&mut app, "Agent");
    let pin = spawn_object(app.world_mut(), &ObjectSpec::new("pinpointer"), STATION).unwrap();
    spawn_object(
        app.world_mut(),
        &ObjectSpec::new("nuclear-disk"),
        TilePos::new(6, 2, 0),
    )
    .unwrap();
    use_pinpointer(app.world_mut(), mob, pin);
    assert!(app.world().get::<Pinpointer>(pin).unwrap().active);
    ticks(&mut app, 1);
    assert_eq!(app.world().get::<Appearance>(pin).unwrap().state, "close");
    use_pinpointer(app.world_mut(), mob, pin);
    assert_eq!(app.world().get::<Appearance>(pin).unwrap().state, "");
}

fn landmarks(app: &mut App) {
    let marks = [
        ("landmark-synd-spawn", 10, 10),
        ("landmark-synd-spawn", 11, 10),
        ("landmark-disk-spawn", 3, 3),
        ("landmark-closet-spawn", 12, 12),
        ("landmark-gear-closet", 14, 12),
        ("landmark-synbomb", 15, 12),
    ];
    for (id, x, y) in marks {
        spawn_object(app.world_mut(), &ObjectSpec::new(id), TilePos::new(x, y, 0)).unwrap();
    }
}

#[test]
fn a_round_sets_up_the_team_the_disk_the_bomb_and_the_gear() {
    let mut app = setup();
    landmarks(&mut app);
    let mobs: Vec<Entity> = (0..6).map(|i| crew(&mut app, &format!("Crew {i}"))).collect();
    app.world_mut().entity_mut(mobs[4]).insert(WantsNuke);
    setup_round(app.world_mut());

    // The one who wanted to is the only one who did.
    for (i, mob) in mobs.iter().enumerate() {
        assert_eq!(app.world().get::<NukeAgent>(*mob).is_some(), i == 4, "{i}");
    }
    let leader = mobs[4];
    assert!(app.world().get::<HasJob>(leader).is_none());
    assert_eq!(app.world().get::<GridPos>(leader).unwrap().0.y, 10);
    assert!(
        app.world()
            .get::<DisplayName>(leader)
            .unwrap()
            .0
            .starts_with("Syndicate ")
    );

    let world = app.world_mut();
    assert_eq!(world.query::<&NuclearDisk>().iter(world).count(), 1);
    let bombs: Vec<NuclearBomb> = world.query::<&NuclearBomb>().iter(world).cloned().collect();
    assert_eq!(bombs.len(), 1);
    assert_eq!(bombs[0].real_code.len(), 5);
    let sprites: Vec<String> = world
        .query::<&Appearance>()
        .iter(world)
        .map(|a| a.sprite.clone())
        .collect();
    for wanted in [
        "closet-syndicate-nuclear",
        "closet-syndicate-personal",
        "c4-small",
    ] {
        assert!(sprites.iter().any(|s| s == wanted), "{wanted}");
    }
    assert_eq!(world.query::<&Landmark>().iter(world).count(), 6);
}

#[test]
fn a_map_without_the_landmarks_gets_no_team() {
    let mut app = setup();
    let mob = crew(&mut app, "Crew");
    setup_round(app.world_mut());
    assert!(app.world().get::<NukeAgent>(mob).is_none());
}

#[test]
fn the_report_is_neutral_if_the_disk_stayed_behind() {
    let mut app = setup();
    app.world_mut().insert_resource(Round {
        phase: Phase::Playing { mode: "nuclear" },
        forced_mode: None,
    });
    spawn_object(app.world_mut(), &ObjectSpec::new("nuclear-disk"), STATION).unwrap();
    let lines = report(app.world_mut());
    assert_eq!(lines[0].id, "nuke-neutral");
    app.world_mut().resource_mut::<Round>().phase = Phase::Playing { mode: "extended" };
    assert!(report(app.world_mut()).is_empty());
}
