//! A brig door timer's window (Luna's `door_timer` window): the time left, buttons to change it,
//! start or stop, and the cell's flash.

use bevy::picking::events::PointerClick;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use rs13_i18n::LocMsg;
use rs13_network::lightyear::prelude::client::*;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{ActionInput, BrigCommand, BrigPanel, IntentChannel};
use rs13_world::BrigPanelView;

use crate::{Loc, ui_text_font};

/// The timer the server says is open.
#[derive(Resource, Default)]
pub struct OpenBrig(pub Option<BrigPanelView>);

#[derive(Component)]
pub struct BrigWindow;

#[derive(Component, Clone, Copy)]
pub enum BrigButton {
    Command(BrigCommand),
    Close,
}

const FACE: Color = Color::srgb(0.83, 0.82, 0.78);
const EDGE: Color = Color::srgb(0.45, 0.45, 0.45);
const TEXT: Color = Color::srgb(0.0, 0.0, 0.0);
const LINK: Color = Color::srgb(0.0, 0.0, 0.6);

pub fn receive_brig(
    mut open: ResMut<OpenBrig>,
    mut receivers: Query<&mut MessageReceiver<BrigPanel>, With<Client>>,
) {
    for mut receiver in &mut receivers {
        if let Some(BrigPanel(view)) = receiver.receive().last() {
            open.0 = view;
        }
    }
}

/// Rebuild the window whenever what it shows changes.
pub fn show_brig(
    mut commands: Commands,
    loc: Res<Loc>,
    open: Res<OpenBrig>,
    windows: Query<Entity, With<BrigWindow>>,
) {
    if !open.is_changed() {
        return;
    }
    for window in &windows {
        commands.entity(window).despawn();
    }
    let Some(view) = &open.0 else { return };
    let text = |msg: LocMsg| loc.0.format_msg(&msg);
    let window = commands
        .spawn((
            BrigWindow,
            Node {
                position_type: PositionType::Absolute,
                left: Val::VMin(20.0),
                top: Val::VMin(8.0),
                min_width: Val::Px(320.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(2.0)),
                padding: UiRect::all(Val::Px(8.0)),
                row_gap: Val::Px(4.0),
                ..default()
            },
            BackgroundColor(FACE),
            BorderColor::all(EDGE),
            // Clicks on the window never reach the map under it.
            Hovered::default(),
            GlobalZIndex(10),
        ))
        .id();
    let label = |commands: &mut Commands, s: String| {
        commands.spawn((Text::new(s), ui_text_font(14.0), TextColor(TEXT), ChildOf(window)));
    };
    label(&mut commands, text(LocMsg::new("brig-title")));
    label(
        &mut commands,
        text(
            LocMsg::new("brig-time")
                .num("minutes", i64::from(view.seconds / 60))
                .num("seconds", i64::from(view.seconds % 60)),
        ),
    );
    let row = commands
        .spawn((
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(10.0),
                ..default()
            },
            ChildOf(window),
        ))
        .id();
    for (label, by) in [("-1:00", -60), ("-0:01", -1), ("+0:01", 1), ("+1:00", 60)] {
        button(
            &mut commands,
            row,
            label.to_owned(),
            BrigButton::Command(BrigCommand::Adjust(by)),
        );
    }
    let toggle = if view.timing { "brig-stop" } else { "brig-start" };
    button(
        &mut commands,
        window,
        text(LocMsg::new(toggle)),
        BrigButton::Command(BrigCommand::Toggle),
    );
    if view.has_flash {
        let flash = if view.flash_charging {
            "brig-flash-charging"
        } else {
            "brig-flash"
        };
        button(
            &mut commands,
            window,
            text(LocMsg::new(flash)),
            BrigButton::Command(BrigCommand::Flash),
        );
    }
    button(
        &mut commands,
        window,
        text(LocMsg::new("panel-close")),
        BrigButton::Close,
    );
}

fn button(commands: &mut Commands, parent: Entity, label: String, action: BrigButton) {
    commands
        .spawn((
            action,
            Node {
                padding: UiRect::axes(Val::Px(3.0), Val::Px(1.0)),
                ..default()
            },
            Hovered::default(),
            ChildOf(parent),
        ))
        .with_child((
            Text::new(label),
            ui_text_font(14.0),
            TextColor(LINK),
            Pickable::IGNORE,
        ))
        .observe(click_button);
}

fn click_button(
    click: On<PointerClick>,
    buttons: Query<&BrigButton>,
    mut open: ResMut<OpenBrig>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
) {
    let Ok(action) = buttons.get(click.entity) else {
        return;
    };
    let Ok(mut sender) = sender.single_mut() else {
        return;
    };
    match *action {
        BrigButton::Command(command) => {
            if let Some(view) = &open.0 {
                sender.send::<IntentChannel>(ActionInput::Brig {
                    target: view.target,
                    command,
                });
            }
        }
        BrigButton::Close => {
            sender.send::<IntentChannel>(ActionInput::ClosePanel);
            open.0 = None;
        }
    }
}
