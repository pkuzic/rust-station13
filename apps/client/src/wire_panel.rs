//! The airlock wire panel window (Luna's "Access Panel" browser window): one row per
//! wire with Cut / Mend and Pulse, the indicator lights, and Close.

use bevy::picking::events::PointerClick;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use rs13_network::lightyear::prelude::client::*;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{ActionInput, IntentChannel, WirePanel};
use rs13_world::WirePanelView;

use crate::{Loc, ui_text_font};

/// The panel the server says is open.
#[derive(Resource, Default)]
pub struct OpenPanel(pub Option<WirePanelView>);

#[derive(Component)]
pub struct PanelWindow;

#[derive(Component, Clone, Copy)]
pub enum PanelButton {
    /// Cut or mend (wirecutters), or pulse (multitool), the wire at this index.
    Wire {
        colour: u8,
        pulse: bool,
    },
    Close,
}

const FACE: Color = Color::srgb(0.83, 0.82, 0.78);
const EDGE: Color = Color::srgb(0.45, 0.45, 0.45);

pub fn receive_panel(
    mut open: ResMut<OpenPanel>,
    mut receivers: Query<&mut MessageReceiver<WirePanel>, With<Client>>,
) {
    for mut receiver in &mut receivers {
        if let Some(WirePanel(view)) = receiver.receive().last() {
            open.0 = view;
        }
    }
}

/// Rebuild the window whenever the panel changes.
pub fn show_panel(
    mut commands: Commands,
    loc: Res<Loc>,
    open: Res<OpenPanel>,
    windows: Query<Entity, With<PanelWindow>>,
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
            PanelWindow,
            Node {
                position_type: PositionType::Absolute,
                left: Val::VMin(20.0),
                top: Val::VMin(8.0),
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
        commands.spawn((
            Text::new(s),
            ui_text_font(14.0),
            TextColor(Color::BLACK),
            ChildOf(window),
        ));
    };
    line(&mut commands, text("panel-title"));

    for (index, (colour, cut)) in view.wires.iter().enumerate() {
        let colour_index = index as u8;
        let mut args = rs13_i18n::Args::new();
        args.set("colour", colour.as_str());
        let label = loc.0.format("panel-wire", Some(&args)).unwrap_or_default();
        let row = commands
            .spawn((
                Node {
                    column_gap: Val::Px(6.0),
                    align_items: AlignItems::Center,
                    ..default()
                },
                ChildOf(window),
            ))
            .id();
        commands.spawn((
            Node {
                width: Val::Px(190.0),
                ..default()
            },
            Text::new(label),
            ui_text_font(14.0),
            TextColor(Color::BLACK),
            ChildOf(row),
        ));
        if *cut {
            button(
                &mut commands,
                row,
                text("panel-mend"),
                PanelButton::Wire {
                    colour: colour_index,
                    pulse: false,
                },
            );
        } else {
            button(
                &mut commands,
                row,
                text("panel-cut"),
                PanelButton::Wire {
                    colour: colour_index,
                    pulse: false,
                },
            );
            button(
                &mut commands,
                row,
                text("panel-pulse"),
                PanelButton::Wire {
                    colour: colour_index,
                    pulse: true,
                },
            );
        }
    }
    for light in &view.lights {
        line(&mut commands, text(light));
    }
    let footer = commands
        .spawn((
            Node {
                margin: UiRect::top(Val::Px(6.0)),
                ..default()
            },
            ChildOf(window),
        ))
        .id();
    button(&mut commands, footer, text("panel-close"), PanelButton::Close);
}

fn button(commands: &mut Commands, parent: Entity, label: String, action: PanelButton) {
    commands
        .spawn((
            action,
            Node {
                border: UiRect::all(Val::Px(1.0)),
                padding: UiRect::axes(Val::Px(6.0), Val::Px(1.0)),
                ..default()
            },
            BackgroundColor(Color::srgb(0.9, 0.9, 0.88)),
            BorderColor::all(EDGE),
            Hovered::default(),
            ChildOf(parent),
        ))
        .with_child((
            Text::new(label),
            ui_text_font(13.0),
            TextColor(Color::srgb(0.0, 0.0, 0.6)),
            Pickable::IGNORE,
        ))
        .observe(click_button);
}

fn click_button(
    click: On<PointerClick>,
    buttons: Query<&PanelButton>,
    mut open: ResMut<OpenPanel>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
) {
    let Ok(action) = buttons.get(click.entity) else {
        return;
    };
    let Ok(mut sender) = sender.single_mut() else {
        return;
    };
    match *action {
        PanelButton::Wire { colour, pulse } => {
            if let Some(view) = &open.0 {
                sender.send::<IntentChannel>(ActionInput::Wire {
                    target: view.target,
                    colour,
                    pulse,
                });
            }
        }
        PanelButton::Close => {
            sender.send::<IntentChannel>(ActionInput::ClosePanel);
            open.0 = None;
        }
    }
}
