//! Cargo. Spec: docs/reference/cargo.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::access::IdCard;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::cargo::{self, Cargo, CargoConsole, CargoPanelOut, SupplyPacks};
use rs13_classic::closets::Closet;
use rs13_classic::filing::PendingPicks;
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_world::{
    CARGO_CATEGORIES, CargoOp, CargoPage, DisplayName, GridPos, HumanLook, MapLayer, ObjectSpec, ShuttleAt,
    TileMap, TilePos,
};

/// A berth of open space (13 × 7, as Luna's) walled off from the room with the two consoles.
const MAP: &str = r#"(
    legend: {
        ' ': (tile: (id: "space")),
        'W': (tile: (id: "wall", solid: true)),
        '.': (tile: (id: "floor")),
        'S': (tile: (id: "floor"), objects: ["supply-console"]),
        'O': (tile: (id: "floor"), objects: ["order-console"]),
    },
    area_legend: { 'b': "supply-station", 'h': "hangar" },
    area_rows: [
        "hbbbbbbbbbbbbbh",
        "hbbbbbbbbbbbbbh",
        "hbbbbbbbbbbbbbh",
        "hbbbbbbbbbbbbbh",
        "hbbbbbbbbbbbbbh",
        "hbbbbbbbbbbbbbh",
        "hbbbbbbbbbbbbbh",
        "hhhhhhhhhhhhhhh",
        "hhhhhhhhhhhhhhh",
        "hhhhhhhhhhhhhhh",
    ],
    rows: [
        "W             W",
        "W             W",
        "W             W",
        "W             W",
        "W             W",
        "W             W",
        "W             W",
        "WWWWWWWWWWWWWWW",
        "...S.O.........",
        "...............",
    ],
)"#;

/// Beside both consoles.
const DESK: TilePos = TilePos::new(4, 0, 0);

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

fn person(app: &mut App, at: TilePos, name: &str) -> Entity {
    let mob = spawn_player_mob(app.world_mut(), at);
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

fn press(app: &mut App, mob: Entity, console: Entity, op: CargoOp) {
    app.world_mut()
        .get_mut::<PendingPicks>(mob)
        .unwrap()
        .0
        .push((console, usize::from(op.line())));
    tick(app, 1);
}

/// Someone holding a card with this access.
fn with_card(app: &mut App, mob: Entity, access: &str) {
    let at = app.world().get::<GridPos>(mob).unwrap().0;
    let card = spawn_object(app.world_mut(), &ObjectSpec::new("id-card"), at).unwrap();
    app.world_mut().entity_mut(card).insert(IdCard {
        access: vec![access.to_owned()],
    });
    act(app, mob, Action::UseOn(card));
}

fn console(app: &mut App, supply: bool) -> Entity {
    let world = app.world_mut();
    let mut q = world.query::<(Entity, &CargoConsole)>();
    q.iter(world).find(|(_, c)| c.supply == supply).unwrap().0
}

fn told(app: &App, mob: Entity, id: &str) -> bool {
    app.world()
        .get::<Outbox>(mob)
        .unwrap()
        .0
        .iter()
        .any(|m| m.id == id)
}

fn view(app: &App, mob: Entity) -> Option<rs13_world::CargoPanelView> {
    app.world().get::<CargoPanelOut>(mob).and_then(|o| o.0.clone())
}

fn cargo(app: &App) -> &Cargo {
    app.world().resource::<Cargo>()
}

/// The place of a pack in a category's list.
fn place_of(app: &App, category: &str, pack: &str) -> (u8, u16) {
    let c = CARGO_CATEGORIES.iter().position(|c| *c == category).unwrap() as u8;
    let i = app
        .world()
        .resource::<SupplyPacks>()
        .in_category(c)
        .iter()
        .position(|p| p.id == pack)
        .unwrap() as u16;
    (c, i)
}

fn berth_tile(app: &mut App) -> String {
    let world = app.world_mut();
    let mut maps = world.query::<&MapLayer>();
    maps.iter(world)
        .next()
        .unwrap()
        .0
        .tile(TilePos::new(5, 5, 0))
        .unwrap()
        .id
        .clone()
}

fn crates(app: &mut App) -> Vec<Entity> {
    let world = app.world_mut();
    let mut q = world.query::<(Entity, &Closet, &GridPos)>();
    q.iter(world)
        .filter(|(_, c, _)| c.is_crate)
        .map(|(e, ..)| e)
        .collect()
}

/// Skip the shuttle's two minutes on the way.
fn arrive_now(app: &mut App) {
    let now = app
        .world()
        .resource::<rs13_classic::time::WorldTime>()
        .deciseconds();
    app.world_mut().resource_mut::<Cargo>().arrives_at = Some(now);
    tick(app, 2);
}

#[test]
fn points_start_at_fifty_and_grow_by_one_every_thirty_seconds() {
    let mut app = setup();
    assert_eq!(cargo(&app).points, cargo::START_POINTS);
    let now = app
        .world()
        .resource::<rs13_classic::time::WorldTime>()
        .deciseconds();
    app.world_mut().resource_mut::<Cargo>().next_point_at = now;
    tick(&mut app, 1);
    assert_eq!(cargo(&app).points, cargo::START_POINTS + 1);
    assert!(cargo(&app).next_point_at > now + cargo::POINT_EVERY_DS - 1.0);
}

#[test]
fn the_shuttle_console_wants_cargo_access() {
    let mut app = setup();
    let supply = console(&mut app, true);
    let clerk = person(&mut app, DESK, "Clerk");
    act(&mut app, clerk, Action::UseOn(supply));
    assert!(told(&app, clerk, "cargo-access-denied"));
    assert!(view(&app, clerk).is_none());
    // The ordering console is anyone's.
    let order = console(&mut app, false);
    act(&mut app, clerk, Action::UseOn(order));
    tick(&mut app, 1);
    let shown = view(&app, clerk).unwrap();
    assert!(!shown.supply);
    assert_eq!(shown.shuttle, ShuttleAt::Dock);
    assert_eq!(shown.points, cargo::START_POINTS);
}

#[test]
fn a_request_approved_comes_on_the_shuttle_in_its_crate_and_crates_sent_back_earn_points() {
    let mut app = setup();
    let (supply, order) = (console(&mut app, true), console(&mut app, false));
    // Anyone asks for metal; a cargo technician approves it, paying in points.
    let clerk = person(&mut app, DESK, "Clerk");
    act(&mut app, clerk, Action::UseOn(order));
    let (cat, i) = place_of(&app, "security", "armor");
    press(&mut app, clerk, order, CargoOp::Order);
    press(&mut app, clerk, order, CargoOp::Category(cat));
    tick(&mut app, 1);
    assert!(matches!(view(&app, clerk).unwrap().page, CargoPage::Packs(..)));
    press(&mut app, clerk, order, CargoOp::Pack(i));
    assert_eq!(cargo(&app).requests.len(), 1);
    assert_eq!(cargo(&app).requests[0].by, "Clerk");

    let tech = person(&mut app, TilePos::new(3, 0, 0), "Tech");
    with_card(&mut app, tech, "cargo");
    act(&mut app, tech, Action::UseOn(supply));
    press(&mut app, tech, supply, CargoOp::Approve(0));
    let armor_cost = app.world().resource::<SupplyPacks>().get("armor").unwrap().cost;
    assert_eq!(cargo(&app).points, cargo::START_POINTS - armor_cost);
    assert!(cargo(&app).requests.is_empty());
    assert_eq!(cargo(&app).orders.len(), 1);

    // Called: it is on its way, two minutes out, and the berth is still open space.
    press(&mut app, tech, supply, CargoOp::Send);
    assert!(told(&app, tech, "cargo-called"));
    assert!(matches!(cargo(&app).shuttle_at(0.0), ShuttleAt::Moving(_)));
    assert_eq!(berth_tile(&mut app), "space");
    // Ordering is shut while it's away.
    let (mcat, mi) = place_of(&app, "materials", "metal50");
    press(&mut app, tech, supply, CargoOp::Category(mcat));
    press(&mut app, tech, supply, CargoOp::Pack(mi));
    assert_eq!(cargo(&app).orders.len(), 1);

    arrive_now(&mut app);
    assert!(cargo(&app).at_station);
    assert_eq!(berth_tile(&mut app), "floor", "the shuttle's floor is laid");
    let delivered = crates(&mut app);
    assert_eq!(delivered.len(), 1);
    let closet = app.world().get::<Closet>(delivered[0]).unwrap().clone();
    // Three vests, three helmets and the manifest, locked to security.
    assert_eq!(closet.contents.len(), 7);
    let lock = app
        .world()
        .get::<rs13_classic::closets::ClosetLock>(delivered[0])
        .unwrap();
    assert!(lock.locked);
    assert_eq!(lock.access, ["security"]);
    let manifest = closet
        .contents
        .iter()
        .find_map(|e| app.world().get::<rs13_classic::paperwork::Paper>(*e))
        .unwrap();
    assert!(manifest.text.contains("[noun:armor-vest]"));
    // Its air is breathable after a moment.
    tick(&mut app, 3);
    let pressure = app
        .world()
        .resource::<rs13_classic::atmos::StationAtmos>()
        .0
        .pressure_at(TilePos::new(6, 5, 0));
    assert!(pressure > 50.0, "{pressure}");

    // Sent back: the crate is sold for 4 points and the berth is open space again.
    let before = cargo(&app).points;
    press(&mut app, tech, supply, CargoOp::Send);
    assert!(!cargo(&app).at_station);
    assert_eq!(cargo(&app).points, before + cargo::PER_CRATE);
    assert!(crates(&mut app).is_empty());
    assert!(cargo(&app).orders.is_empty());
    assert_eq!(berth_tile(&mut app), "space");
}

#[test]
fn it_wont_leave_with_anyone_aboard() {
    let mut app = setup();
    let supply = console(&mut app, true);
    let tech = person(&mut app, DESK, "Tech");
    with_card(&mut app, tech, "cargo");
    act(&mut app, tech, Action::UseOn(supply));
    press(&mut app, tech, supply, CargoOp::Send);
    arrive_now(&mut app);
    assert!(cargo(&app).at_station);
    let stowaway = person(&mut app, TilePos::new(6, 4, 0), "Stowaway");
    press(&mut app, tech, supply, CargoOp::Send);
    assert!(told(&app, tech, "cargo-no-people"));
    assert!(cargo(&app).at_station);
    assert!(app.world().get_entity(stowaway).is_ok());
}

#[test]
fn not_enough_points_orders_nothing() {
    let mut app = setup();
    let supply = console(&mut app, true);
    let tech = person(&mut app, DESK, "Tech");
    with_card(&mut app, tech, "cargo");
    act(&mut app, tech, Action::UseOn(supply));
    app.world_mut().resource_mut::<Cargo>().points = 5;
    let (cat, i) = place_of(&app, "materials", "metal50");
    press(&mut app, tech, supply, CargoOp::Order);
    press(&mut app, tech, supply, CargoOp::Category(cat));
    press(&mut app, tech, supply, CargoOp::Pack(i));
    tick(&mut app, 1);
    assert!(cargo(&app).orders.is_empty());
    assert_eq!(cargo(&app).points, 5);
    assert_eq!(
        view(&app, tech).unwrap().page,
        CargoPage::Note("cargo-not-enough".to_owned())
    );
}

#[test]
fn every_pack_can_be_delivered() {
    let mut app = setup();
    let packs: Vec<String> = app
        .world()
        .resource::<SupplyPacks>()
        .0
        .iter()
        .map(|p| p.id.clone())
        .collect();
    {
        let mut c = app.world_mut().resource_mut::<Cargo>();
        for pack in &packs {
            c.orders.push(cargo::Order {
                pack: pack.clone(),
                by: "Test".to_owned(),
            });
        }
    }
    arrive_now(&mut app);
    // Each one is on the shuttle: in its crate, or loose (canisters, tanks, sheets).
    let world = app.world_mut();
    let mut papers = world.query::<&rs13_classic::paperwork::Paper>();
    let manifests = papers
        .iter(world)
        .filter(|p| p.text.contains("cargo-manifest-title"))
        .count();
    assert_eq!(manifests, packs.len());
}

#[test]
fn on_lunas_deck_the_berth_is_the_shuttles_size_and_the_consoles_are_there() {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let map = TileMap::from_ron(include_str!("../maps/luna_deck4.ron"), 4).unwrap();
    let berth = (0..map.height as i32)
        .flat_map(|y| (0..map.width as i32).map(move |x| TilePos::new(x, y, 4)))
        .filter(|p| map.area(*p) == Some(cargo::BERTH_AREA))
        .count();
    assert_eq!(berth, 13 * 7);
    let _ = spawn_map(app.world_mut(), map);
    let world = app.world_mut();
    let mut q = world.query::<&CargoConsole>();
    let consoles: Vec<bool> = q.iter(world).map(|c| c.supply).collect();
    assert!(
        consoles.contains(&true) && consoles.contains(&false),
        "{consoles:?}"
    );
}
