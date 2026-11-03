//! The traitor mode. Spec: docs/reference/traitor.md.

use bevy_app::{App, FixedUpdate};
use bevy_ecs::prelude::*;
use rs13_classic::ClassicPlugin;
use rs13_classic::actions::{Action, Outbox, PendingActions};
use rs13_classic::escape::{EscapePods, PendingCommsOps, PodState};
use rs13_classic::hands::Hands;
use rs13_classic::health::Vitals;
use rs13_classic::jobs::HasJob;
use rs13_classic::objects::{spawn_map, spawn_object, spawn_player_mob};
use rs13_classic::traitor::{
    CRYSTALS, GOODS, Objective, PendingUplinkOps, Traitor, UPLINK, Uplink, UplinkPanelOut, choose_traitors,
    how_many, report,
};
use rs13_world::{Appearance, DisplayName, GridPos, MobStat, ObjectSpec, TileMap, TilePos};

const MAP: &str = r#"(
    legend: { 'W': (tile: (id: "wall", solid: true)), '.': (tile: (id: "floor")) },
    rows: ["WWWWWWW", "W.....W", "W.....W", "WWWWWWW"],
    area_legend: { 'p': "shuttle-escape-station-pod1", 's': "bridge" },
    area_rows: ["pppssss", "pppssss", "pppssss", "pppssss"],
)"#;
const STATION: TilePos = TilePos::new(5, 1, 0);
const POD: TilePos = TilePos::new(1, 1, 0);

fn setup() -> App {
    let mut app = App::new();
    app.add_plugins(ClassicPlugin);
    let problems = spawn_map(app.world_mut(), TileMap::from_ron(MAP, 0).unwrap());
    assert!(problems.is_empty(), "{problems:?}");
    app.world_mut().run_schedule(FixedUpdate);
    app
}

fn crew(app: &mut App, at: TilePos, name: &str) -> Entity {
    let mob = spawn_player_mob(app.world_mut(), at);
    app.world_mut()
        .entity_mut(mob)
        .insert((HasJob("clown".to_owned()), DisplayName(name.to_owned())));
    mob
}

fn traitor_with(app: &mut App, at: TilePos, objectives: Vec<Objective>) -> Entity {
    let mob = crew(app, at, "The Traitor");
    app.world_mut().entity_mut(mob).insert(Traitor {
        objectives,
        remind_at: None,
    });
    mob
}

fn said(lines: &[rs13_i18n::LocMsg]) -> Vec<String> {
    lines
        .iter()
        .map(|l| {
            let done = l.args.iter().find(|(n, _)| n == "done");
            format!("{} {:?}", l.id, done.map(|(_, v)| v))
        })
        .collect()
}

#[test]
fn there_are_about_one_in_ten_from_one_to_four() {
    assert_eq!(how_many(1, 0), 1);
    assert_eq!(how_many(14, 1), 2);
    assert_eq!(how_many(30, 0), 3);
    assert_eq!(how_many(100, 5), 4);
}

#[test]
fn a_traitor_is_told_given_work_and_an_uplink() {
    let mut app = setup();
    let alone = crew(&mut app, STATION, "Alone");
    choose_traitors(app.world_mut());
    let traitor = app.world().get::<Traitor>(alone).unwrap().clone();
    let weight = traitor
        .objectives
        .iter()
        .filter(|o| matches!(o, Objective::Steal(_)))
        .count();
    assert_eq!(weight, 5, "with nobody else it's theft, until the work adds up");
    assert!(matches!(
        traitor.objectives.last(),
        Some(Objective::Escape | Objective::Hijack)
    ));
    let told: Vec<String> = app
        .world()
        .get::<Outbox>(alone)
        .unwrap()
        .0
        .iter()
        .map(|m| m.id.clone())
        .collect();
    assert!(told.contains(&"traitor-you".to_owned()), "{told:?}");
    assert!(told.iter().any(|m| m == "objective-steal"), "{told:?}");
    assert!(told.iter().any(|m| m.starts_with("traitor-uplink")), "{told:?}");
    let mut uplinks = app.world_mut().query::<&Uplink>();
    assert_eq!(uplinks.iter(app.world()).count(), 1);
}

#[test]
fn others_can_be_targets_and_never_both_ways() {
    let mut app = setup();
    let mobs: Vec<Entity> = (0..12)
        .map(|i| crew(&mut app, STATION, &format!("Crew {i}")))
        .collect();
    for _ in 0..30 {
        for mob in &mobs {
            app.world_mut().entity_mut(*mob).remove::<Traitor>();
        }
        choose_traitors(app.world_mut());
        let mut traitors = app.world_mut().query::<(Entity, &Traitor)>();
        let all: Vec<(Entity, Traitor)> = traitors.iter(app.world()).map(|(e, t)| (e, t.clone())).collect();
        assert!((1..=2).contains(&all.len()), "{}", all.len());
        for (mob, traitor) in all {
            let mut targets = Vec::new();
            for objective in &traitor.objectives {
                if let Objective::Assassinate { target, .. } | Objective::Protect { target, .. } = objective {
                    assert_ne!(*target, mob, "not themselves");
                    assert!(!targets.contains(target), "one job per person");
                    targets.push(*target);
                }
            }
        }
    }
}

#[test]
fn an_uplink_sells_what_it_can_afford_into_the_hand() {
    let mut app = setup();
    let mob = crew(&mut app, STATION, "Buyer");
    let uplink = spawn_object(app.world_mut(), &ObjectSpec::new(UPLINK), STATION).unwrap();
    click(&mut app, mob, uplink);
    assert!(
        app.world().get::<UplinkPanelOut>(mob).unwrap().0.is_none(),
        "picking it up doesn't open it"
    );
    click(&mut app, mob, uplink);
    app.world_mut().run_schedule(FixedUpdate);
    let view = app.world().get::<UplinkPanelOut>(mob).unwrap().0.clone().unwrap();
    assert_eq!(view.crystals, CRYSTALS);

    let revolver = GOODS
        .iter()
        .position(|(p, _)| *p == "syndicate-revolver")
        .unwrap();
    let press = |app: &mut App, line: usize| {
        app.world_mut()
            .get_mut::<PendingUplinkOps>(mob)
            .unwrap()
            .0
            .push((uplink, line));
        app.world_mut().run_schedule(FixedUpdate);
    };
    press(&mut app, revolver);
    assert_eq!(app.world().get::<Uplink>(uplink).unwrap().crystals, CRYSTALS - 7);
    let hands = *app.world().get::<Hands>(mob).unwrap();
    let other = hands
        .left
        .filter(|e| *e != uplink)
        .or(hands.right.filter(|e| *e != uplink));
    let bought = other.expect("in the free hand");
    assert_eq!(
        app.world().get::<Appearance>(bought).unwrap().sprite,
        "syndicate-revolver"
    );

    // Too dear now.
    press(&mut app, revolver);
    assert_eq!(app.world().get::<Uplink>(uplink).unwrap().crystals, CRYSTALS - 7);
}

fn click(app: &mut App, mob: Entity, target: Entity) {
    app.world_mut()
        .get_mut::<PendingActions>(mob)
        .unwrap()
        .0
        .push(Action::UseOn(target));
    app.world_mut().run_schedule(FixedUpdate);
}

#[test]
fn the_report_checks_each_kind_of_objective() {
    let mut app = setup();
    let victim = crew(&mut app, STATION, "Victim");
    let friend = crew(&mut app, POD, "Friend");
    app.world_mut().get_mut::<Vitals>(victim).unwrap().stat = MobStat::Dead;
    let job = || "clown".to_owned();
    let mob = traitor_with(
        &mut app,
        POD,
        vec![
            Objective::Steal("magboots"),
            Objective::Steal("hypospray"),
            Objective::Assassinate {
                target: victim,
                name: "Victim".to_owned(),
                job: job(),
            },
            Objective::Assassinate {
                target: friend,
                name: "Friend".to_owned(),
                job: job(),
            },
            Objective::Protect {
                target: friend,
                name: "Friend".to_owned(),
                job: job(),
            },
            Objective::Protect {
                target: victim,
                name: "Victim".to_owned(),
                job: job(),
            },
            Objective::Escape,
            Objective::Hijack,
        ],
    );
    let boots = spawn_object(app.world_mut(), &ObjectSpec::new("magboots"), POD).unwrap();
    click(&mut app, mob, boots);

    let told = said(&report(app.world_mut()));
    let won = |s: &str| s.contains("won");
    let results: Vec<bool> = told[1..9].iter().map(|l| won(l)).collect();
    assert_eq!(
        results,
        [true, false, true, false, true, false, true, false],
        "{told:?}"
    );
    assert_eq!(told[9].split(' ').next(), Some("round-traitor-lost"));
}

#[test]
fn a_traitor_with_everything_done_wins_when_the_pods_arrive() {
    let mut app = setup();
    let mob = traitor_with(&mut app, POD, vec![Objective::Hijack]);
    let bystander = crew(&mut app, STATION, "Bystander");
    let mut pods = EscapePods::default();
    pods.state = PodState::Transit { at: 0.0 };
    app.insert_resource(pods);
    app.world_mut().run_schedule(FixedUpdate);
    assert_eq!(app.world().resource::<EscapePods>().state, PodState::Arrived);
    let told: Vec<String> = app
        .world()
        .get::<Outbox>(bystander)
        .unwrap()
        .0
        .iter()
        .map(|m| m.id.clone())
        .collect();
    assert!(told.contains(&"round-traitor".to_owned()), "{told:?}");
    assert!(told.contains(&"round-traitor-won".to_owned()), "{told:?}");
    let _ = (mob, PendingCommsOps::default(), GridPos(POD));
}
