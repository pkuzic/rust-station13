//! The power monitoring computer's window (Luna's `powcomp` browser window): the grid's
//! supply and load, then one line per APC.

use bevy::picking::events::PointerClick;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use rs13_network::lightyear::prelude::client::*;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{ActionInput, IntentChannel, PowerMonitorPanel};
use rs13_world::PowerMonitorView;

use crate::{Loc, ui_text_font};

/// The power monitor the server says is open.
#[derive(Resource, Default)]
pub struct OpenMonitor(pub Option<PowerMonitorView>);

#[derive(Component)]
pub struct MonitorWindow;

#[derive(Component)]
pub struct MonitorClose;

const FACE: Color = Color::srgb(0.83, 0.82, 0.78);
const EDGE: Color = Color::srgb(0.45, 0.45, 0.45);
const TEXT: Color = Color::srgb(0.0, 0.0, 0.0);

pub fn receive_monitor(
    mut open: ResMut<OpenMonitor>,
    mut receivers: Query<&mut MessageReceiver<PowerMonitorPanel>, With<Client>>,
) {
    for mut receiver in &mut receivers {
        if let Some(PowerMonitorPanel(view)) = receiver.receive().last() {
            open.0 = view;
        }
    }
}

/// Rebuild the window whenever what it shows changes.
pub fn show_monitor(
    mut commands: Commands,
    loc: Res<Loc>,
    open: Res<OpenMonitor>,
    windows: Query<Entity, With<MonitorWindow>>,
) {
    if !open.is_changed() {
        return;
    }
    for window in &windows {
        commands.entity(window).despawn();
    }
    let Some(view) = &open.0 else { return };
    let text = |id: &str| loc.0.format(id, None).unwrap_or_default();

    let window = commands
        .spawn((
            MonitorWindow,
            Node {
                position_type: PositionType::Absolute,
                left: Val::VMin(15.0),
                top: Val::VMin(6.0),
                min_width: Val::Px(360.0),
                flex_direction: FlexDirection::Column,
                border: UiRect::all(Val::Px(2.0)),
                padding: UiRect::all(Val::Px(8.0)),
                row_gap: Val::Px(3.0),
                ..default()
            },
            BackgroundColor(FACE),
            BorderColor::all(EDGE),
            // Clicks on the window never reach the map under it.
            Hovered::default(),
            GlobalZIndex(10),
        ))
        .id();
    let line = |commands: &mut Commands, s: String| {
        commands.spawn((Text::new(s), ui_text_font(14.0), TextColor(TEXT), ChildOf(window)));
    };

    line(&mut commands, text("monitor-title"));
    let mut args = rs13_i18n::Args::new();
    args.set("supply", i64::from(view.supply));
    args.set("load", i64::from(view.load));
    line(
        &mut commands,
        loc.0.format("monitor-totals", Some(&args)).unwrap_or_default(),
    );
    for row in &view.apcs {
        let mut args = rs13_i18n::Args::new();
        args.set("area", text(&format!("area-{}", row.area)));
        args.set("equipment", i64::from(row.channels[0]));
        args.set("lighting", i64::from(row.channels[1]));
        args.set("environment", i64::from(row.channels[2]));
        args.set("load", i64::from(row.load));
        let id = match row.cell_percent {
            Some(percent) => {
                args.set("value", i64::from(percent));
                args.set("state", i64::from(row.charging));
                "monitor-apc"
            }
            None => "monitor-apc-no-cell",
        };
        line(&mut commands, loc.0.format(id, Some(&args)).unwrap_or_default());
    }

    commands
        .spawn((
            MonitorClose,
            Node {
                margin: UiRect::top(Val::Px(8.0)),
                border: UiRect::all(Val::Px(1.0)),
                padding: UiRect::axes(Val::Px(6.0), Val::Px(1.0)),
                align_self: AlignSelf::Start,
                ..default()
            },
            BackgroundColor(Color::srgb(0.9, 0.9, 0.88)),
            BorderColor::all(EDGE),
            Hovered::default(),
            ChildOf(window),
        ))
        .with_child((
            Text::new(text("panel-close")),
            ui_text_font(13.0),
            TextColor(Color::srgb(0.0, 0.0, 0.6)),
            Pickable::IGNORE,
        ))
        .observe(click_close);
}

fn click_close(
    _click: On<PointerClick>,
    mut open: ResMut<OpenMonitor>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
) {
    if let Ok(mut sender) = sender.single_mut() {
        sender.send::<IntentChannel>(ActionInput::ClosePanel);
    }
    open.0 = None;
}
