//! Filing cabinets, bedsheets and linen bins. Spec: docs/reference/filing-cabinets.md,
//! docs/reference/bedsheets.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::bedsheets::{BIN_SHEETS, Bedsheet, LinenBin};
use rs13_classic::filing::{FilingCabinet, PendingPicks, PickListOut};
use rs13_classic::hands::Hands;
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::paperwork::Paper;
use rs13_world::{Appearance, GridPos, ObjectSpec, TileMap, TilePos};

const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'f': (tile: (id: "floor"), objects: ["filing-cabinet", (id: "paper", vars: {"name": "Old orders"})]),
        'l': (tile: (id: "floor"), objects: ["linen-bin"]),
    },
    rows: [
        "WWWWW",
        "Wf.lW",
        "WWWWW",
    ],
)"#;

const HERE: TilePos = TilePos::new(2, 1, 0);

fn setup() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    app.world_mut().run_schedule(FixedUpdate);
    let mob = spawn_player_mob(app.world_mut(), HERE);
    (app, mob)
}

fn tick(app: &mut App) {
    app.world_mut().run_schedule(FixedUpdate);
    app.world_mut().run_schedule(FixedUpdate);
}

fn act(app: &mut App, mob: Entity, action: Action) -> Vec<String> {
    let before = app.world().get::<Outbox>(mob).unwrap().0.len();
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(action);
    tick(app);
    app.world().get::<Outbox>(mob).unwrap().0[before..]
        .iter()
        .map(|m| m.id.clone())
        .collect()
}

fn one<C: Component>(app: &mut App) -> Entity {
    app.world_mut()
        .query_filtered::<Entity, With<C>>()
        .single(app.world())
        .unwrap()
}

fn active(app: &App, mob: Entity) -> Option<Entity> {
    app.world().get::<Hands>(mob).unwrap().active_item()
}

fn rows(app: &App, mob: Entity) -> Option<Vec<String>> {
    app.world()
        .get::<PickListOut>(mob)
        .unwrap()
        .0
        .as_ref()
        .map(|v| v.rows.clone())
}

#[test]
fn a_cabinet_files_sheets_and_hands_them_back_by_name() {
    let (mut app, mob) = setup();
    let cabinet = one::<FilingCabinet>(&mut app);
    // The sheet on its tile went in at round start.
    assert_eq!(
        app.world().get::<FilingCabinet>(cabinet).unwrap().contents.len(),
        1
    );

    // A new sheet goes in, and the drawer shows open for a moment.
    let sheet = spawn_object(app.world_mut(), &ObjectSpec::new("paper"), HERE).unwrap();
    app.world_mut().get_mut::<Paper>(sheet).unwrap().name = "New orders".to_owned();
    act(&mut app, mob, Action::UseOn(sheet));
    assert_eq!(act(&mut app, mob, Action::UseOn(cabinet)), ["cabinet-put"]);
    assert_eq!(
        app.world().get::<Appearance>(cabinet).unwrap().state,
        "tallcabinet-open"
    );

    // Anything else won't go in.
    let wrench = spawn_object(app.world_mut(), &ObjectSpec::new("wrench"), HERE).unwrap();
    let crowbar = spawn_object(app.world_mut(), &ObjectSpec::new("crowbar"), HERE).unwrap();
    act(&mut app, mob, Action::UseOn(crowbar));
    assert_eq!(act(&mut app, mob, Action::UseOn(cabinet)), ["cabinet-wont-fit"]);
    act(&mut app, mob, Action::Drop);

    // An empty hand opens its window, the last one filed first.
    act(&mut app, mob, Action::UseOn(cabinet));
    assert_eq!(
        rows(&app, mob).unwrap(),
        ["New orders".to_owned(), "Old orders".to_owned()]
    );
    // Picking the second line hands that sheet over and closes the window.
    app.world_mut()
        .get_mut::<PendingPicks>(mob)
        .unwrap()
        .0
        .push((cabinet, 1));
    tick(&mut app);
    let got = active(&app, mob).expect("a sheet");
    assert_eq!(app.world().get::<Paper>(got).unwrap().name, "Old orders");
    assert_eq!(rows(&app, mob), None);

    // A wrench frees it to be pulled, and bolts it down again.
    act(&mut app, mob, Action::Drop);
    act(&mut app, mob, Action::UseOn(wrench));
    assert_eq!(act(&mut app, mob, Action::UseOn(cabinet)), ["cabinet-unwrenched"]);
    assert!(!app.world().get::<FilingCabinet>(cabinet).unwrap().anchored);
    assert_eq!(act(&mut app, mob, Action::UseOn(cabinet)), ["cabinet-wrenched"]);
}

#[test]
fn an_empty_cabinet_says_so() {
    let (mut app, mob) = setup();
    let cabinet = one::<FilingCabinet>(&mut app);
    act(&mut app, mob, Action::UseOn(cabinet));
    app.world_mut()
        .get_mut::<PendingPicks>(mob)
        .unwrap()
        .0
        .push((cabinet, 0));
    tick(&mut app);
    act(&mut app, mob, Action::SwapHands);
    assert_eq!(act(&mut app, mob, Action::UseOn(cabinet)), ["cabinet-empty"]);
}

#[test]
fn a_linen_bin_hands_out_sheets_and_hides_something() {
    let (mut app, mob) = setup();
    let bin = one::<LinenBin>(&mut app);
    // Something small hides among the sheets.
    let pen = spawn_object(app.world_mut(), &ObjectSpec::new("pen"), HERE).unwrap();
    act(&mut app, mob, Action::UseOn(pen));
    assert_eq!(act(&mut app, mob, Action::UseOn(bin)), ["linen-bin-hide"]);
    assert_eq!(app.world().get::<LinenBin>(bin).unwrap().hidden, Some(pen));

    // Taking a sheet out, the pen falls out with it.
    assert_eq!(
        act(&mut app, mob, Action::UseOn(bin)),
        ["linen-bin-take", "linen-bin-falls-out"]
    );
    let sheet = active(&app, mob).expect("a sheet");
    assert!(app.world().get::<Bedsheet>(sheet).is_some());
    assert_eq!(app.world().get::<GridPos>(pen).unwrap().0, HERE);
    assert_eq!(app.world().get::<LinenBin>(bin).unwrap().amount, BIN_SHEETS - 1);

    // Back in it goes; then out again, the same one.
    assert_eq!(act(&mut app, mob, Action::UseOn(bin)), ["linen-bin-put"]);
    act(&mut app, mob, Action::UseOn(bin));
    assert_eq!(active(&app, mob), Some(sheet));

    // Used in the hand, it is spread where you stand; again, folded.
    act(&mut app, mob, Action::UseOn(sheet));
    assert_eq!(active(&app, mob), None);
    assert!(app.world().get::<Bedsheet>(sheet).unwrap().spread);
    assert_eq!(app.world().get::<Appearance>(sheet).unwrap().state, "spread");

    // Emptied, the bin looks empty.
    app.world_mut().get_mut::<LinenBin>(bin).unwrap().amount = 0;
    tick(&mut app);
    assert_eq!(
        app.world().get::<Appearance>(bin).unwrap().state,
        "linenbin-empty"
    );
}
