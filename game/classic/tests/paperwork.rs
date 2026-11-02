//! Paper bins, paper, pens, clipboards, and status displays. Spec:
//! docs/reference/paperwork.md, docs/reference/status-displays.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::hands::Hands;
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::paperwork::{
    BIN_SHEETS, Clipboard, MAX_TEXT, Paper, PaperBin, PaperOp, PaperOut, PendingPaperOps,
};
use rs13_classic::status_displays::Glyph;
use rs13_world::{Appearance, ObjectSpec, PaperMode, PixelOffset, TileMap, TilePos};

const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'b': (tile: (id: "floor"), objects: ["paper-bin"]),
        's': (tile: (id: "floor"), objects: [(id: "status-display", vars: {"mode": "4"})]),
        'd': (tile: (id: "floor"), objects: ["status-display"]),
    },
    rows: [
        "WWWWWW",
        "W.b.sW",
        "W...dW",
        "WWWWWW",
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

fn act(app: &mut App, mob: Entity, action: Action) -> Vec<String> {
    let before = app.world().get::<Outbox>(mob).unwrap().0.len();
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(action);
    app.world_mut().run_schedule(FixedUpdate);
    app.world_mut().run_schedule(FixedUpdate);
    app.world().get::<Outbox>(mob).unwrap().0[before..]
        .iter()
        .map(|m| m.id.clone())
        .collect()
}

fn op(app: &mut App, mob: Entity, op: PaperOp) {
    app.world_mut()
        .get_mut::<PendingPaperOps>(mob)
        .unwrap()
        .0
        .push(op);
    app.world_mut().run_schedule(FixedUpdate);
    app.world_mut().run_schedule(FixedUpdate);
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

#[test]
fn a_bin_hands_out_sheets_and_takes_them_back() {
    let (mut app, mob) = setup();
    let bin = one::<PaperBin>(&mut app);
    assert_eq!(act(&mut app, mob, Action::UseOn(bin)), ["paper-bin-take"]);
    let sheet = active(&app, mob).expect("a sheet");
    assert_eq!(app.world().get::<Paper>(sheet).unwrap().name, "Paper");
    assert_eq!(app.world().get::<PaperBin>(bin).unwrap().amount, BIN_SHEETS - 1);
    assert_eq!(act(&mut app, mob, Action::UseOn(bin)), ["paper-bin-put"]);
    assert_eq!(active(&app, mob), None);
    assert_eq!(app.world().get::<PaperBin>(bin).unwrap().amount, BIN_SHEETS);
    // The same sheet comes out again first.
    act(&mut app, mob, Action::UseOn(bin));
    assert_eq!(active(&app, mob), Some(sheet));

    app.world_mut().get_mut::<PaperBin>(bin).unwrap().amount = 0;
    act(&mut app, mob, Action::SwapHands);
    assert_eq!(act(&mut app, mob, Action::UseOn(bin)), ["paper-bin-empty"]);
    assert_eq!(app.world().get::<Appearance>(bin).unwrap().state, "paper_bin0");
}

#[test]
fn a_pen_writes_and_a_sheet_in_hand_is_labelled() {
    let (mut app, mob) = setup();
    let sheet = spawn_object(app.world_mut(), &ObjectSpec::new("paper"), HERE).unwrap();
    let pen = spawn_object(app.world_mut(), &ObjectSpec::new("pen"), HERE).unwrap();
    act(&mut app, mob, Action::UseOn(pen));
    // The pen on the sheet opens its text box.
    act(&mut app, mob, Action::UseOn(sheet));
    let windows = std::mem::take(&mut app.world_mut().get_mut::<PaperOut>(mob).unwrap().0);
    assert_eq!(windows.len(), 1);
    assert_eq!(windows[0].mode, PaperMode::Write);
    op(
        &mut app,
        mob,
        PaperOp::Write(sheet, format!("[b]Orders[/b]\n{}", "x".repeat(4000))),
    );
    let text = app.world().get::<Paper>(sheet).unwrap().text.clone();
    assert!(text.starts_with("[b]Orders[/b]\n"));
    assert_eq!(text.chars().count(), MAX_TEXT, "cut to Luna's length");

    // Labelled once in hand.
    act(&mut app, mob, Action::SwapHands);
    act(&mut app, mob, Action::UseOn(sheet));
    op(
        &mut app,
        mob,
        PaperOp::Label(sheet, "Captain's orders".to_owned()),
    );
    assert_eq!(
        app.world().get::<Paper>(sheet).unwrap().name,
        "paper- 'Captain's orders'"
    );
    // Without a pen nothing is written.
    op(&mut app, mob, PaperOp::Write(sheet, "scribble".to_owned()));
    assert!(
        app.world()
            .get::<Paper>(sheet)
            .unwrap()
            .text
            .starts_with("[b]Orders")
    );
}

#[test]
fn a_clipboard_holds_sheets_and_a_pen_writes_on_the_top_one() {
    let (mut app, mob) = setup();
    let board = spawn_object(app.world_mut(), &ObjectSpec::new("clipboard"), HERE).unwrap();
    let sheet = spawn_object(app.world_mut(), &ObjectSpec::new("paper"), HERE).unwrap();
    act(&mut app, mob, Action::UseOn(board));
    act(&mut app, mob, Action::SwapHands);
    act(&mut app, mob, Action::UseOn(sheet));
    assert_eq!(act(&mut app, mob, Action::UseOn(board)), ["clipboard-clipped"]);
    assert_eq!(app.world().get::<Clipboard>(board).unwrap().papers, [sheet]);
    assert!(
        app.world()
            .get::<Appearance>(board)
            .unwrap()
            .state
            .contains("paper")
    );

    // A pen on the clipboard writes on its top sheet.
    let pen = spawn_object(app.world_mut(), &ObjectSpec::new("pen"), HERE).unwrap();
    act(&mut app, mob, Action::UseOn(pen));
    act(&mut app, mob, Action::UseOn(board));
    let windows = std::mem::take(&mut app.world_mut().get_mut::<PaperOut>(mob).unwrap().0);
    assert_eq!(windows[0].sheet, sheet);

    // Its window's pen slot, then taking the sheet back out.
    op(&mut app, mob, PaperOp::Pen(board));
    assert_eq!(app.world().get::<Clipboard>(board).unwrap().pen, Some(pen));
    assert!(
        app.world()
            .get::<Appearance>(board)
            .unwrap()
            .state
            .contains("clipboard_pen")
    );
    op(&mut app, mob, PaperOp::Remove(board, 0));
    assert_eq!(active(&app, mob), Some(sheet));
    assert!(app.world().get::<Clipboard>(board).unwrap().papers.is_empty());
}

#[test]
fn a_supply_display_shows_its_two_lines_and_the_others_are_blank() {
    let (mut app, _) = setup();
    for _ in 0..3 {
        app.world_mut().run_schedule(FixedUpdate);
    }
    let mut q = app.world_mut().query::<(&Appearance, &PixelOffset, &Glyph)>();
    let mut glyphs: Vec<(String, [i8; 2])> = q
        .iter(app.world())
        .map(|(a, p, _)| (a.state.clone(), p.0))
        .collect();
    glyphs.sort_by_key(|(_, p)| (-p[1], p[0]));
    let top: String = glyphs
        .iter()
        .filter(|(_, p)| p[1] == -9)
        .map(|(c, _)| c.as_str())
        .collect();
    let bottom: String = glyphs
        .iter()
        .filter(|(_, p)| p[1] == -17)
        .map(|(c, _)| c.as_str())
        .collect();
    assert_eq!(top, "SPPLY");
    assert_eq!(bottom, "AWAY");
    assert_eq!(glyphs.len(), 9, "the plain display draws nothing");
}
