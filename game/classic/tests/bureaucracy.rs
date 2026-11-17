//! Rubber stamps, folders and the photocopier. Spec: docs/reference/paperwork.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::bureaucracy::{Folder, Photocopier, TONER_FULL};
use rs13_classic::filing::{FilingCabinet, PendingPicks, PickListOut};
use rs13_classic::hands::{Hands, HeldBy};
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::paperwork::{Filed, Paper, PaperOut, Stamped};
use rs13_world::{Appearance, GridPos, ObjectSpec, PaperMode, TileMap, TilePos};

const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
    },
    rows: [
        "WWWWWWW",
        "W.....W",
        "W.....W",
        "WWWWWWW",
    ],
)"#;

const HERE: TilePos = TilePos::new(2, 1, 0);
const NEXT: TilePos = TilePos::new(3, 1, 0);

fn setup() -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    tick(&mut app, 1);
    let mob = spawn_player_mob(app.world_mut(), HERE);
    (app, mob)
}

fn tick(app: &mut App, n: u32) {
    for _ in 0..n {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

/// What the mob was told doing it.
fn act(app: &mut App, mob: Entity, action: Action) -> Vec<String> {
    let before = app.world().get::<Outbox>(mob).unwrap().0.len();
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(action);
    tick(app, 2);
    app.world().get::<Outbox>(mob).unwrap().0[before..]
        .iter()
        .map(|m| m.id.clone())
        .collect()
}

fn pick(app: &mut App, mob: Entity, target: Entity, line: usize) {
    app.world_mut()
        .get_mut::<PendingPicks>(mob)
        .unwrap()
        .0
        .push((target, line));
    tick(app, 2);
}

fn spawn(app: &mut App, id: &str, pos: TilePos) -> Entity {
    spawn_object(app.world_mut(), &ObjectSpec::new(id), pos).unwrap()
}

fn give(app: &mut App, mob: Entity, item: Entity) {
    let hand = app.world().get::<Hands>(mob).unwrap().active;
    app.world_mut()
        .entity_mut(item)
        .remove::<GridPos>()
        .insert(HeldBy(mob));
    *app.world_mut().get_mut::<Hands>(mob).unwrap().slot_mut(hand) = Some(item);
}

/// Empty the active hand, the item left on the floor.
fn put_down(app: &mut App, mob: Entity) {
    let mut hands = app.world_mut().get_mut::<Hands>(mob).unwrap();
    let hand = hands.active;
    if let Some(item) = hands.slot_mut(hand).take() {
        app.world_mut()
            .entity_mut(item)
            .remove::<HeldBy>()
            .insert(GridPos(HERE));
    }
}

fn active(app: &App, mob: Entity) -> Option<Entity> {
    app.world().get::<Hands>(mob).unwrap().active_item()
}

fn rows(app: &App, mob: Entity) -> Vec<String> {
    app.world()
        .get::<PickListOut>(mob)
        .unwrap()
        .0
        .as_ref()
        .map(|v| v.rows.clone())
        .unwrap_or_default()
}

fn state(app: &App, e: Entity) -> String {
    app.world().get::<Appearance>(e).unwrap().state.clone()
}

fn written(app: &mut App, text: &str) -> Entity {
    let sheet = spawn(app, "paper", NEXT);
    app.world_mut().get_mut::<Paper>(sheet).unwrap().text = text.to_owned();
    sheet
}

#[test]
fn a_stamp_marks_a_sheet_and_no_pen_changes_it_after() {
    let (mut app, mob) = setup();
    let sheet = written(&mut app, "Approved.");
    let stamp = spawn(&mut app, "stamp-captain", HERE);
    give(&mut app, mob, stamp);
    let told = act(&mut app, mob, Action::UseOn(sheet));
    assert_eq!(told, ["paper-stamp"]);
    assert!(app.world().get::<Stamped>(sheet).is_some());
    let text = &app.world().get::<Paper>(sheet).unwrap().text;
    assert!(text.starts_with("Approved."), "{text}");
    assert!(
        text.contains("[msg:paper-stamped-with] [noun:stamp-captain]."),
        "{text}"
    );
    assert_eq!(state(&app, sheet), "stamped");

    put_down(&mut app, mob);
    let pen = spawn(&mut app, "pen", HERE);
    give(&mut app, mob, pen);
    let told = act(&mut app, mob, Action::UseOn(sheet));
    assert_eq!(told, ["paper-stamped-no-edit"]);
    assert!(app.world().get::<PaperOut>(mob).unwrap().0.is_empty());
}

#[test]
fn a_stamp_on_a_clipboard_stamps_its_top_sheet() {
    let (mut app, mob) = setup();
    let board = spawn(&mut app, "clipboard", NEXT);
    let sheet = written(&mut app, "");
    give(&mut app, mob, sheet);
    act(&mut app, mob, Action::UseOn(board));
    assert_eq!(app.world().get::<Filed>(sheet).map(|f| f.0), Some(board));
    let stamp = spawn(&mut app, "stamp-denied", HERE);
    give(&mut app, mob, stamp);
    assert_eq!(act(&mut app, mob, Action::UseOn(board)), ["paper-stamp"]);
    assert!(app.world().get::<Stamped>(sheet).is_some());
}

#[test]
fn a_folder_holds_sheets_and_its_window_reads_and_takes_them_out() {
    let (mut app, mob) = setup();
    let folder = spawn(&mut app, "folder-red", NEXT);
    let sheet = written(&mut app, "Secret.");
    give(&mut app, mob, sheet);
    assert_eq!(act(&mut app, mob, Action::UseOn(folder)), ["folder-put"]);
    assert_eq!(app.world().get::<Folder>(folder).unwrap().0, [sheet]);
    assert_eq!(app.world().get::<Filed>(sheet).map(|f| f.0), Some(folder));
    assert_eq!(active(&app, mob), None);
    assert_eq!(state(&app, folder), "+folder_paper");

    // Used in the hand, its window lists the sheet: take it out, or read it.
    give(&mut app, mob, folder);
    act(&mut app, mob, Action::UseOn(folder));
    assert_eq!(
        rows(&app, mob),
        ["msg:folder-remove:Paper", "msg:folder-read:Paper"]
    );
    pick(&mut app, mob, folder, 1);
    let read = app.world().get::<PaperOut>(mob).unwrap().0.clone();
    assert!(
        read.iter()
            .any(|w| w.sheet == sheet && w.mode == PaperMode::Read && w.text == "Secret.")
    );
    pick(&mut app, mob, folder, 0);
    assert!(app.world().get::<Folder>(folder).unwrap().0.is_empty());
    let hands = app.world().get::<Hands>(mob).unwrap();
    assert!(hands.left == Some(sheet) || hands.right == Some(sheet));
    assert!(app.world().get::<Filed>(sheet).is_none());
    tick(&mut app, 1);
    assert_eq!(state(&app, folder), "");
}

#[test]
fn a_folder_goes_into_a_filing_cabinet_by_its_name() {
    let (mut app, mob) = setup();
    let cabinet = spawn(&mut app, "filing-cabinet", NEXT);
    let folder = spawn(&mut app, "folder-blue", HERE);
    give(&mut app, mob, folder);
    act(&mut app, mob, Action::UseOn(cabinet));
    assert_eq!(
        app.world().get::<FilingCabinet>(cabinet).unwrap().contents,
        [folder]
    );
    act(&mut app, mob, Action::UseOn(cabinet));
    assert_eq!(rows(&app, mob), ["msg:pick-noun:folder-blue"]);
}

#[test]
fn the_photocopier_prints_copies_of_a_sheet_a_unit_of_toner_each() {
    let (mut app, mob) = setup();
    let copier = spawn(&mut app, "photocopier", NEXT);
    let sheet = written(&mut app, "Memo");
    app.world_mut().entity_mut(sheet).insert(Stamped);
    give(&mut app, mob, sheet);
    assert_eq!(act(&mut app, mob, Action::UseOn(copier)), ["copier-inserted"]);
    assert_eq!(app.world().get::<Photocopier>(copier).unwrap().copy, Some(sheet));

    // A second sheet won't go in.
    let other = written(&mut app, "");
    give(&mut app, mob, other);
    assert_eq!(act(&mut app, mob, Action::UseOn(copier)), ["copier-full"]);
    put_down(&mut app, mob);

    act(&mut app, mob, Action::UseOn(copier));
    assert_eq!(
        rows(&app, mob),
        [
            "msg:copier-remove",
            "msg:copier-copy",
            "msg:copier-printing:1",
            "msg:copier-less",
            "msg:copier-more",
            "msg:copier-toner:30",
        ]
    );
    pick(&mut app, mob, copier, 4);
    pick(&mut app, mob, copier, 4);
    assert_eq!(rows(&app, mob)[2], "msg:copier-printing:3");
    pick(&mut app, mob, copier, 1);
    // One every second and a half.
    tick(&mut app, 2 * 35);
    let mut q = app
        .world_mut()
        .query::<(Entity, &Paper, &GridPos, Has<Stamped>)>();
    let copies: Vec<(Entity, bool)> = q
        .iter(app.world())
        .filter(|(_, p, at, _)| at.0 == NEXT && p.text == "Memo")
        .map(|(e, _, _, stamped)| (e, stamped))
        .collect();
    assert_eq!(copies.len(), 3);
    assert!(copies.iter().all(|(e, stamped)| *e != sheet && !stamped));
    assert_eq!(
        app.world().get::<Photocopier>(copier).unwrap().toner,
        TONER_FULL - 3
    );

    // Taken back out.
    pick(&mut app, mob, copier, 0);
    assert_eq!(active(&app, mob), Some(sheet));
    assert!(app.world().get::<Photocopier>(copier).unwrap().copy.is_none());
}

#[test]
fn only_an_empty_photocopier_takes_a_new_toner_cartridge() {
    let (mut app, mob) = setup();
    let copier = spawn(&mut app, "photocopier", NEXT);
    let toner = spawn(&mut app, "toner", HERE);
    give(&mut app, mob, toner);
    assert_eq!(
        act(&mut app, mob, Action::UseOn(copier)),
        ["copier-toner-not-empty"]
    );
    app.world_mut().get_mut::<Photocopier>(copier).unwrap().toner = 0;
    act(&mut app, mob, Action::UseOn(copier));
    assert_eq!(app.world().get::<Photocopier>(copier).unwrap().toner, TONER_FULL);
    assert!(app.world().get_entity(toner).is_err());
}

#[test]
fn a_wrench_frees_the_photocopier_and_bolts_it_down_again() {
    let (mut app, mob) = setup();
    let copier = spawn(&mut app, "photocopier", NEXT);
    let wrench = spawn(&mut app, "wrench", HERE);
    give(&mut app, mob, wrench);
    assert_eq!(act(&mut app, mob, Action::UseOn(copier)), ["copier-unwrenched"]);
    assert!(!app.world().get::<Photocopier>(copier).unwrap().anchored);
    assert_eq!(act(&mut app, mob, Action::UseOn(copier)), ["copier-wrenched"]);
    assert!(app.world().get::<Photocopier>(copier).unwrap().anchored);
}

#[test]
fn a_noticeboard_holds_five_sheets_to_read_write_on_and_take_down() {
    use rs13_classic::bureaucracy::{MAX_NOTICES, Noticeboard};
    let (mut app, mob) = setup();
    let board = spawn(&mut app, "noticeboard", NEXT);
    let first = written(&mut app, "Lost cat.");
    give(&mut app, mob, first);
    assert_eq!(act(&mut app, mob, Action::UseOn(board)), ["notice-pinned"]);
    assert_eq!(app.world().get::<Filed>(first).map(|f| f.0), Some(board));
    for _ in 1..MAX_NOTICES {
        let sheet = written(&mut app, "");
        give(&mut app, mob, sheet);
        act(&mut app, mob, Action::UseOn(board));
    }
    assert_eq!(state(&app, board), "nboard05");
    let sixth = written(&mut app, "");
    give(&mut app, mob, sixth);
    assert_eq!(act(&mut app, mob, Action::UseOn(board)), ["notice-too-many"]);
    assert_eq!(active(&app, mob), Some(sixth));
    put_down(&mut app, mob);

    // An empty hand opens its window: each sheet to read, write on or take down.
    act(&mut app, mob, Action::UseOn(board));
    assert_eq!(
        rows(&app, mob)[..3],
        ["msg:notice-read:Paper", "msg:notice-write", "msg:notice-remove"]
    );
    pick(&mut app, mob, board, 0);
    assert!(
        app.world()
            .get::<PaperOut>(mob)
            .unwrap()
            .0
            .iter()
            .any(|w| w.sheet == first && w.mode == PaperMode::Read)
    );
    // Writing wants a pen in hand.
    let before = app.world().get::<Outbox>(mob).unwrap().0.len();
    pick(&mut app, mob, board, 1);
    assert_eq!(
        app.world().get::<Outbox>(mob).unwrap().0[before..]
            .iter()
            .map(|m| m.id.as_str())
            .collect::<Vec<_>>(),
        ["notice-no-pen"]
    );
    let pen = spawn(&mut app, "pen", HERE);
    give(&mut app, mob, pen);
    pick(&mut app, mob, board, 1);
    assert!(
        app.world()
            .get::<PaperOut>(mob)
            .unwrap()
            .0
            .iter()
            .any(|w| w.sheet == first && w.mode == PaperMode::Write)
    );
    // Taken down, it falls to the floor under the board.
    pick(&mut app, mob, board, 2);
    assert_eq!(
        app.world().get::<Noticeboard>(board).unwrap().0.len(),
        MAX_NOTICES - 1
    );
    assert_eq!(app.world().get::<GridPos>(first).map(|p| p.0), Some(NEXT));
    assert!(app.world().get::<Filed>(first).is_none());
    tick(&mut app, 1);
    assert_eq!(state(&app, board), "nboard04");
}
