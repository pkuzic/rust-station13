//! Security cameras and the consoles that watch them: pick a camera and see from it.
//! Spec: `docs/reference/cameras.md`.

use bevy_ecs::prelude::*;
use rs13_i18n::LocMsg;
use rs13_world::{
    Appearance, CameraPanelView, CameraRow, CameraView, Dense, GridPos, LightSource, MobStat, NetId,
    ObjectSpec, TilePos,
};

use crate::actions::{ClickOutcome, Outbox, in_reach};
use crate::health::Vitals;
use crate::power::{Channel, PowerUser};
use crate::wires::PanelViewer;

pub const CAMERA: &str = "camera";
pub const SECURITY_CONSOLE: &str = "security-console";
/// The console's draw, W, and its screen's glow (as the power monitor's).
pub const CONSOLE_POWER: f64 = 250.0;
pub const SCREEN_LIGHT: [u8; 3] = [0, 0, 2];
/// The network every camera and console on NSV Luna is on (Luna `network = "Luna"`).
pub const NETWORK: &str = "Luna";

/// A security camera: its name tag, its network, and whether it works.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct Camera {
    pub tag: Option<String>,
    pub network: String,
    pub on: bool,
}

/// A security camera console: its network, and the camera it shows.
#[derive(Component, Debug, Clone, PartialEq, Eq)]
pub struct SecurityConsole {
    pub network: String,
    pub current: Option<Entity>,
}

/// Camera buttons waiting for the next tick: the console, and the camera's line in its
/// list, or `None` to stop watching.
#[derive(Component, Debug, Default)]
pub struct PendingCameraOps(pub Vec<(Entity, Option<usize>)>);

/// What the mob's open console shows, for the server to send when it changes.
#[derive(Component, Debug, Clone, Default, PartialEq)]
pub struct CameraPanelOut(pub Option<CameraPanelView>);

pub fn spawn_camera(world: &mut World, spec: &ObjectSpec, pos: TilePos, id: NetId) -> Entity {
    let tag = spec
        .vars
        .get("c_tag")
        .map(|t| t.trim().to_owned())
        // Luna's unnamed cameras say `c_tag = null`.
        .filter(|t| !t.is_empty() && t != "null");
    let network = spec.vars.get("network").map_or(NETWORK, |n| n.trim()).to_owned();
    world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new(CAMERA, ""),
            Camera {
                tag,
                network,
                on: true,
            },
        ))
        .id()
}

pub fn spawn_console(world: &mut World, pos: TilePos, id: NetId) -> Entity {
    world
        .spawn((
            GridPos(pos),
            id,
            Appearance::new(SECURITY_CONSOLE, "seccam"),
            SecurityConsole {
                network: NETWORK.to_owned(),
                current: None,
            },
            PowerUser::new(Channel::Equipment, CONSOLE_POWER),
            LightSource(SCREEN_LIGHT),
            Dense,
        ))
        .id()
}

/// A console's cameras, as listed: on its network, named, sorted by name (Luna
/// `camera_sort`; unnamed ones are left out).
fn listed<'a>(
    network: &str,
    cameras: impl Iterator<Item = (Entity, &'a Camera)>,
) -> Vec<(Entity, &'a Camera)> {
    let mut list: Vec<_> = cameras
        .filter(|(_, c)| c.network == network && c.tag.is_some())
        .collect();
    list.sort_by(|a, b| a.1.tag.cmp(&b.1.tag));
    list
}

/// Clicking a console in reach, while it has power, opens its window.
pub fn open(
    consoles: &Query<(&GridPos, &PowerUser), With<SecurityConsole>>,
    mob: Entity,
    mob_pos: TilePos,
    target: Entity,
    commands: &mut Commands,
) -> Option<()> {
    let (pos, power) = consoles.get(target).ok()?;
    if power.powered && in_reach(mob_pos, pos.0) {
        commands.entity(mob).insert(PanelViewer(target));
    }
    Some(())
}

/// Luna `camera/attackby` with wirecutters: switch it off or on, which everyone sees.
pub fn cut(camera: &mut Camera) -> ClickOutcome {
    camera.on = !camera.on;
    let id = if camera.on {
        "camera-reactivated"
    } else {
        "camera-deactivated"
    };
    ClickOutcome {
        to_mob: None,
        seen: Some(LocMsg::new(id).arg("item", CAMERA)),
    }
}

/// Camera buttons (Luna's camera list): pick one to watch, or stop. In reach and awake.
pub fn process_camera_ops(
    mut mobs: Query<(&GridPos, &mut PendingCameraOps, Option<&Vitals>)>,
    mut consoles: Query<(&GridPos, &mut SecurityConsole)>,
    cameras: Query<(Entity, &Camera)>,
) {
    for (pos, mut pending, vitals) in &mut mobs {
        let awake = vitals.is_none_or(|v| v.stat == MobStat::Conscious);
        for (target, line) in pending.0.drain(..) {
            let Ok((at, mut console)) = consoles.get_mut(target) else {
                continue;
            };
            if !awake || !in_reach(pos.0, at.0) {
                continue;
            }
            console.current = match line {
                None => None,
                Some(i) => listed(&console.network, cameras.iter())
                    .get(i)
                    .filter(|(_, c)| c.on)
                    .map(|(e, _)| *e),
            };
        }
    }
}

type Viewer<'a> = (
    Entity,
    &'a GridPos,
    Option<&'a PanelViewer>,
    &'a mut CameraPanelOut,
    Option<&'a Vitals>,
    Option<&'a CameraView>,
    &'a mut Outbox,
);

/// Keep each viewer's console window in step, and their view on its camera (Luna
/// `check_eye`): it ends when they step away, the console loses power, or the camera goes
/// off ("The screen bursts into static.").
pub fn update_camera_views(
    mut viewers: Query<Viewer>,
    mut consoles: Query<(&GridPos, &mut SecurityConsole, &PowerUser, &NetId)>,
    cameras: Query<(Entity, &Camera, &GridPos)>,
    mut commands: Commands,
) {
    for (mob, pos, viewer, mut out, vitals, view, mut outbox) in &mut viewers {
        let mut eye = None;
        let target = viewer.map(|v| v.0).filter(|t| consoles.contains(*t));
        let shown = target.and_then(|t| {
            let (at, mut console, power, id) = consoles.get_mut(t).ok()?;
            let usable =
                in_reach(pos.0, at.0) && power.powered && vitals.is_none_or(|v| v.stat == MobStat::Conscious);
            if !usable {
                return None;
            }
            if let Some(current) = console.current {
                match cameras.get(current) {
                    Ok((_, camera, place)) if camera.on => eye = Some(place.0),
                    _ => {
                        console.current = None;
                        outbox.0.push(LocMsg::new("camera-static"));
                    }
                }
            }
            let list = listed(&console.network, cameras.iter().map(|(e, c, _)| (e, c)));
            Some(CameraPanelView {
                target: *id,
                cameras: list
                    .iter()
                    .map(|(_, c)| CameraRow {
                        tag: c.tag.clone().unwrap_or_default(),
                        on: c.on,
                    })
                    .collect(),
                current: console
                    .current
                    .and_then(|cur| list.iter().position(|(e, _)| *e == cur))
                    .map(|i| i as u16),
            })
        });
        if shown.is_none() && target.is_some() {
            commands.entity(mob).remove::<PanelViewer>();
        }
        match (eye, view) {
            (Some(at), Some(v)) if v.0 == at => {}
            (Some(at), _) => {
                commands.entity(mob).insert(CameraView(at));
            }
            (None, Some(_)) => {
                commands.entity(mob).remove::<CameraView>();
            }
            (None, None) => {}
        }
        if out.0 != shown {
            out.0 = shown;
        }
    }
}

/// A camera shows `camera1` when off (Luna); a console `seccam0` without power.
pub fn update_camera_looks(
    mut cameras: Query<(&Camera, &mut Appearance), Without<SecurityConsole>>,
    mut consoles: Query<(&PowerUser, &mut Appearance), With<SecurityConsole>>,
) {
    for (camera, mut look) in &mut cameras {
        let state = if camera.on { "" } else { "off" };
        if look.state != state {
            look.state = state.to_owned();
        }
    }
    for (power, mut look) in &mut consoles {
        let state = if power.powered { "seccam" } else { "seccam0" };
        if look.state != state {
            look.state = state.to_owned();
        }
    }
}
