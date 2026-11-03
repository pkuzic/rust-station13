//! Crew records. Spec: docs/reference/records.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::access::IdCard;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::health::Vitals;
use rs13_classic::jobs::HasJob;
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::records::{Criminal, PendingRecordOps, RecordsPanelOut};
use rs13_world::{MobStat, ObjectSpec, TileMap, TilePos};

const MAP: &str = r#"(
    legend: { 'W': (tile: (id: "wall", solid: true)), '.': (tile: (id: "floor")) },
    rows: ["WWWWWW", "W....W", "WWWWWW"],
)"#;
const NEAR: TilePos = TilePos::new(1, 1, 0);
const AT: TilePos = TilePos::new(2, 1, 0);

/// Someone next to a computer, with a card of this access in hand, and one more person aboard.
fn setup(proto: &str, access: &str) -> (App, Entity, Entity, Entity) {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    let mob = spawn_player_mob(app.world_mut(), NEAR);
    let other = spawn_player_mob(app.world_mut(), TilePos::new(4, 1, 0));
    app.world_mut()
        .entity_mut(mob)
        .insert(HasJob("security-officer".to_owned()));
    app.world_mut()
        .entity_mut(other)
        .insert(HasJob("clown".to_owned()));
    let machine = spawn_object(app.world_mut(), &ObjectSpec::new(proto), AT).unwrap();
    let card = spawn_object(app.world_mut(), &ObjectSpec::new("id-card"), NEAR).unwrap();
    app.world_mut().entity_mut(card).insert(IdCard {
        access: vec![access.to_owned()],
    });
    app.world_mut().run_schedule(FixedUpdate);
    click(&mut app, mob, card);
    (app, mob, other, machine)
}

fn click(app: &mut App, mob: Entity, target: Entity) {
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(target));
    app.world_mut().run_schedule(FixedUpdate);
}

fn view(app: &mut App, mob: Entity) -> rs13_world::RecordsPanelView {
    app.world_mut().run_schedule(FixedUpdate);
    app.world()
        .get::<RecordsPanelOut>(mob)
        .unwrap()
        .0
        .clone()
        .expect("open")
}

#[test]
fn security_records_list_the_crew_and_change_status() {
    let (mut app, mob, other, machine) = setup("security-records", "security");
    click(&mut app, mob, machine);
    let shown = view(&mut app, mob);
    assert!(!shown.medical);
    assert_eq!(shown.rows.len(), 2);
    let line = shown.rows.iter().position(|r| r.job == "clown").unwrap();
    assert_eq!(shown.rows[line].status, "none");

    app.world_mut()
        .get_mut::<PendingRecordOps>(mob)
        .unwrap()
        .0
        .push((machine, line));
    let shown = view(&mut app, mob);
    assert_eq!(shown.rows[line].status, "arrest");
    assert_eq!(app.world().get::<Criminal>(other).unwrap().0, 1);
    for _ in 0..4 {
        app.world_mut()
            .get_mut::<PendingRecordOps>(mob)
            .unwrap()
            .0
            .push((machine, line));
        app.world_mut().run_schedule(FixedUpdate);
    }
    assert_eq!(view(&mut app, mob).rows[line].status, "none", "round again");
}

#[test]
fn the_wrong_card_is_refused() {
    let (mut app, mob, _, machine) = setup("security-records", "medical");
    click(&mut app, mob, machine);
    app.world_mut().run_schedule(FixedUpdate);
    assert!(app.world().get::<RecordsPanelOut>(mob).unwrap().0.is_none());
    assert_eq!(
        app.world().get::<Outbox>(mob).unwrap().0.last().unwrap().id,
        "records-denied"
    );
}

#[test]
fn medical_records_show_how_people_are_and_cant_be_changed() {
    let (mut app, mob, other, machine) = setup("medical-records", "medical");
    click(&mut app, mob, machine);
    let shown = view(&mut app, mob);
    assert!(shown.medical);
    let line = shown.rows.iter().position(|r| r.job == "clown").unwrap();
    assert_eq!(shown.rows[line].status, "active");

    app.world_mut().get_mut::<Vitals>(other).unwrap().stat = MobStat::Dead;
    app.world_mut()
        .get_mut::<PendingRecordOps>(mob)
        .unwrap()
        .0
        .push((machine, line));
    let shown = view(&mut app, mob);
    assert_eq!(shown.rows[line].status, "deceased");
    assert!(app.world().get::<Criminal>(other).is_none());
}
