//! Forensics. Spec: docs/reference/forensics.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::access::IdCard;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::filing::{PendingPicks, PickListOut};
use rs13_classic::forensics::{self, FibreKind, FingerprintCard, ForensicComputer, Scanner, Traces};
use rs13_classic::hands::Hands;
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::paperwork::Paper;
use rs13_world::{DisplayName, GridPos, HumanLook, ObjectSpec, TileMap, TilePos};

const MAP: &str = r#"(
    legend: {
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'F': (tile: (id: "floor"), objects: ["forensic-computer"]),
    },
    rows: [
        "WWWWWWW",
        "W.F...W",
        "W.....W",
        "WWWWWWW",
    ],
)"#;

/// Beside the computer.
const DESK: TilePos = TilePos::new(2, 1, 0);

fn setup() -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    tick(&mut app, 2);
    app
}

fn tick(app: &mut App, n: u32) {
    for _ in 0..n {
        app.world_mut().run_schedule(FixedUpdate);
    }
}

fn person(app: &mut App, name: &str) -> Entity {
    let mob = spawn_player_mob(app.world_mut(), DESK);
    app.world_mut()
        .entity_mut(mob)
        .insert((HumanLook::default(), DisplayName(name.to_owned())));
    mob
}

fn act(app: &mut App, mob: Entity, action: Action) {
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(action);
    tick(app, 1);
}

fn thing(app: &mut App, id: &str) -> Entity {
    spawn_object(app.world_mut(), &ObjectSpec::new(id), DESK).unwrap()
}

/// Pick something up into the active hand.
fn take(app: &mut App, mob: Entity, id: &str) -> Entity {
    let item = thing(app, id);
    act(app, mob, Action::UseOn(item));
    assert_eq!(held(app, mob), Some(item));
    item
}

fn held(app: &App, mob: Entity) -> Option<Entity> {
    app.world().get::<Hands>(mob).and_then(Hands::active_item)
}

fn swap(app: &mut App, mob: Entity) {
    act(app, mob, Action::SwapHands);
}

fn wear(app: &mut App, mob: Entity, id: &str) {
    let item = thing(app, id);
    rs13_classic::jobs::wear(app.world_mut(), mob, item);
}

fn print_of(app: &mut App, mob: Entity) -> String {
    forensics::fingerprint(app.world_mut(), mob).unwrap()
}

fn prints(app: &App, e: Entity) -> Vec<String> {
    app.world()
        .get::<Traces>(e)
        .map(|t| t.prints.clone())
        .unwrap_or_default()
}

fn told(app: &App, mob: Entity, id: &str) -> bool {
    app.world()
        .get::<Outbox>(mob)
        .unwrap()
        .0
        .iter()
        .any(|m| m.id == id)
}

fn computer(app: &mut App) -> Entity {
    let world = app.world_mut();
    let mut q = world.query_filtered::<Entity, With<ForensicComputer>>();
    q.iter(world).next().unwrap()
}

fn press(app: &mut App, mob: Entity, target: Entity, line: usize) {
    app.world_mut()
        .get_mut::<PendingPicks>(mob)
        .unwrap()
        .0
        .push((target, line));
    tick(app, 1);
}

fn rows(app: &App, mob: Entity) -> Vec<String> {
    app.world()
        .get::<PickListOut>(mob)
        .and_then(|o| o.0.clone())
        .map(|v| v.rows)
        .unwrap_or_default()
}

#[test]
fn bare_hands_leave_prints_and_gloves_do_not() {
    let mut app = setup();
    let ann = person(&mut app, "Ann");
    let crowbar = take(&mut app, ann, "crowbar");
    let ann_print = print_of(&mut app, ann);
    assert_eq!(prints(&app, crowbar), [ann_print.as_str()]);

    let bob = person(&mut app, "Bob");
    wear(&mut app, bob, "black-gloves");
    let wrench = take(&mut app, bob, "wrench");
    assert!(prints(&app, wrench).is_empty());
    assert_ne!(print_of(&mut app, bob), ann_print);
}

#[test]
fn worn_clothes_shed_fibres_on_what_is_handled() {
    let mut app = setup();
    let ann = person(&mut app, "Ann");
    wear(&mut app, ann, "grey-jumpsuit");
    let crowbar = take(&mut app, ann, "crowbar");
    // A quarter of touches leave a uniform's fibres.
    for _ in 0..60 {
        forensics::touch(app.world_mut(), ann, crowbar);
    }
    let traces = app.world().get::<Traces>(crowbar).unwrap();
    assert_eq!(traces.fibres, [(FibreKind::Uniform, "grey-jumpsuit".to_owned())]);
}

#[test]
fn the_scanner_reads_prints_and_prints_cards() {
    let mut app = setup();
    let ann = person(&mut app, "Ann");
    let crowbar = take(&mut app, ann, "crowbar");
    let ann_print = print_of(&mut app, ann);
    act(&mut app, ann, Action::Drop);

    let det = person(&mut app, "Det");
    let scanner = take(&mut app, det, "forensic-scanner");
    // Printing on.
    act(&mut app, det, Action::UseOn(scanner));
    assert!(app.world().get::<Scanner>(scanner).unwrap().printing);
    act(&mut app, det, Action::UseOn(crowbar));
    assert!(told(&app, det, "forensic-isolated"));
    let said: Vec<String> = app
        .world()
        .get::<Outbox>(det)
        .unwrap()
        .0
        .iter()
        .filter(|m| m.id == "forensic-print-line")
        .map(|m| format!("{:?}", m.args))
        .collect();
    assert!(said.iter().any(|a| a.contains(&ann_print)), "{said:?}");
    // The scanner leaves no prints of its own on what it reads.
    assert_eq!(prints(&app, crowbar), [ann_print.as_str()]);
    assert_eq!(
        app.world().get::<Scanner>(scanner).unwrap().cards,
        forensics::SCANNER_CARDS - 1
    );
    let world = app.world_mut();
    let mut q = world.query::<&FingerprintCard>();
    assert!(q.iter(world).any(|c| c.prints == [ann_print.as_str()]));

    // Scanning a person reads theirs.
    act(&mut app, det, Action::UseOn(ann));
    assert!(told(&app, det, "forensic-prints-of"));
}

#[test]
fn the_computer_wants_forensics_access_then_scans_and_prints() {
    let mut app = setup();
    let ann = person(&mut app, "Ann");
    let crowbar = take(&mut app, ann, "crowbar");
    let ann_print = print_of(&mut app, ann);
    let pc = computer(&mut app);

    // Ann holds the crowbar in one hand and a card in the other.
    swap(&mut app, ann);
    let card = take(&mut app, ann, "id-card");
    app.world_mut().entity_mut(card).insert(IdCard {
        access: vec!["security".to_owned()],
    });
    act(&mut app, ann, Action::UseOn(pc));
    assert_eq!(rows(&app, ann), ["msg:forensic-log-in"]);
    press(&mut app, ann, pc, 0);
    assert!(!app.world().get::<ForensicComputer>(pc).unwrap().authenticated);
    app.world_mut().entity_mut(card).insert(IdCard {
        access: vec!["forensics_lockers".to_owned()],
    });
    press(&mut app, ann, pc, 0);
    assert!(app.world().get::<ForensicComputer>(pc).unwrap().authenticated);

    // Insert the crowbar, scan it.
    swap(&mut app, ann);
    assert_eq!(held(&app, ann), Some(crowbar));
    press(&mut app, ann, pc, 1);
    assert_eq!(
        app.world().get::<ForensicComputer>(pc).unwrap().item,
        Some(crowbar)
    );
    assert!(held(&app, ann).is_none());
    press(&mut app, ann, pc, 2);
    assert!(rows(&app, ann).contains(&"msg:forensic-scanning:25".to_owned()));
    // Three stages of five seconds (20 ticks a second).
    tick(&mut app, 310);
    let shown = rows(&app, ann);
    assert!(
        shown.contains(&format!("msg:forensic-print-row:#1 - {ann_print}")),
        "{shown:?}"
    );

    // Print it.
    press(&mut app, ann, pc, 3);
    let world = app.world_mut();
    let mut q = world.query::<(&Paper, &GridPos)>();
    let paper = q.iter(world).find(|(p, _)| p.text.contains(&ann_print));
    assert!(paper.is_some());
    assert_eq!(
        app.world().get::<ForensicComputer>(pc).unwrap().note,
        Some("forensic-note-printed")
    );
}
