//! A Syndicate uplink's window (Luna's `uplink/generate_menu`): spend its crystals.

use bevy::picking::events::PointerClick;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use rs13_i18n::LocMsg;
use rs13_network::lightyear::prelude::client::*;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{ActionInput, IntentChannel, UplinkPanel};
use rs13_world::UplinkPanelView;

use crate::{Loc, ui_text_font};

/// The uplink the server says is open.
#[derive(Resource, Default)]
pub struct OpenUplink(pub Option<UplinkPanelView>);

#[derive(Component)]
pub struct UplinkWindow;

#[derive(Component, Clone, Copy)]
pub enum UplinkButton {
    Buy(u8),
    Close,
}

const FACE: Color = Color::srgb(0.83, 0.82, 0.78);
const EDGE: Color = Color::srgb(0.45, 0.45, 0.45);
const TEXT: Color = Color::srgb(0.0, 0.0, 0.0);
const LINK: Color = Color::srgb(0.0, 0.0, 0.6);

pub fn receive_uplink(
    mut open: ResMut<OpenUplink>,
    mut receivers: Query<&mut MessageReceiver<UplinkPanel>, With<Client>>,
) {
    for mut receiver in &mut receivers {
        if let Some(UplinkPanel(view)) = receiver.receive().last() {
            open.0 = view;
        }
    }
}

/// Rebuild the window whenever what it shows changes.
pub fn show_uplink(
    mut commands: Commands,
    loc: Res<Loc>,
    open: Res<OpenUplink>,
    windows: Query<Entity, With<UplinkWindow>>,
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
            UplinkWindow,
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
    label(&mut commands, text(LocMsg::new("uplink-title")));
    label(
        &mut commands,
        text(LocMsg::new("uplink-crystals").num("value", i64::from(view.crystals))),
    );
    // What's too dear isn't offered (Luna `generate_menu`).
    for (line, (proto, cost)) in view.goods.iter().enumerate() {
        if *cost <= view.crystals {
            let good = LocMsg::new("uplink-good")
                .arg("item", proto.clone())
                .num("value", i64::from(*cost));
            button(&mut commands, window, text(good), UplinkButton::Buy(line as u8));
        }
    }
    button(
        &mut commands,
        window,
        text(LocMsg::new("panel-close")),
        UplinkButton::Close,
    );
}

fn button(commands: &mut Commands, parent: Entity, label: String, action: UplinkButton) {
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
    buttons: Query<&UplinkButton>,
    mut open: ResMut<OpenUplink>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
) {
    let Ok(action) = buttons.get(click.entity) else {
        return;
    };
    let Ok(mut sender) = sender.single_mut() else {
        return;
    };
    match *action {
        UplinkButton::Buy(line) => {
            if let Some(view) = &open.0 {
                sender.send::<IntentChannel>(ActionInput::Uplink {
                    target: view.target,
                    line,
                });
            }
        }
        UplinkButton::Close => {
            sender.send::<IntentChannel>(ActionInput::ClosePanel);
            open.0 = None;
        }
    }
}
