//! The vending machine window (Luna's `vending` browser window): each product, how many
//! are left, and Vend, or Sold Out.

use bevy::picking::events::PointerClick;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use rs13_network::lightyear::prelude::client::*;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{ActionInput, IntentChannel, VendingPanel};
use rs13_world::VendingPanelView;

use crate::{Loc, ui_text_font};

/// The machine the server says is open.
#[derive(Resource, Default)]
pub struct OpenVending(pub Option<VendingPanelView>);

#[derive(Component)]
pub struct VendingWindow;

#[derive(Component, Clone, Copy)]
pub enum VendingButton {
    Vend(u8),
    Close,
}

const FACE: Color = Color::srgb(0.83, 0.82, 0.78);
const EDGE: Color = Color::srgb(0.45, 0.45, 0.45);
const TEXT: Color = Color::srgb(0.0, 0.0, 0.0);
const SOLD_OUT: Color = Color::srgb(0.5, 0.5, 0.5);

pub fn receive_vending(
    mut open: ResMut<OpenVending>,
    mut receivers: Query<&mut MessageReceiver<VendingPanel>, With<Client>>,
) {
    for mut receiver in &mut receivers {
        if let Some(VendingPanel(view)) = receiver.receive().last() {
            open.0 = view;
        }
    }
}

/// Rebuild the window whenever what it shows changes.
pub fn show_vending(
    mut commands: Commands,
    loc: Res<Loc>,
    open: Res<OpenVending>,
    windows: Query<Entity, With<VendingWindow>>,
) {
    if !open.is_changed() {
        return;
    }
    for window in &windows {
        commands.entity(window).despawn();
    }
    let Some(view) = &open.0 else { return };
    let text = |id: &str| loc.0.format(id, None).unwrap_or_default();
    let noun = |id: &str| {
        let mut a = rs13_i18n::Args::new();
        a.set("item", id.to_owned());
        loc.0
            .format("menu-thing", Some(&a))
            .unwrap_or_else(|_| id.to_owned())
    };

    let window = commands
        .spawn((
            VendingWindow,
            Node {
                position_type: PositionType::Absolute,
                left: Val::VMin(20.0),
                top: Val::VMin(8.0),
                min_width: Val::Px(320.0),
                flex_direction: FlexDirection::Column,
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
    let line = |commands: &mut Commands, s: String, colour: Color, parent: Entity, width: f32| {
        commands.spawn((
            Text::new(s),
            ui_text_font(14.0),
            TextColor(colour),
            Node {
                min_width: Val::Px(width),
                ..default()
            },
            ChildOf(parent),
        ));
    };
    line(&mut commands, noun(&view.machine), TEXT, window, 0.0);
    line(&mut commands, text("vending-select"), TEXT, window, 0.0);
    for (i, row) in view.products.iter().enumerate() {
        let entry = commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(6.0),
                    ..default()
                },
                ChildOf(window),
            ))
            .id();
        let colour = if row.amount > 0 { TEXT } else { SOLD_OUT };
        line(&mut commands, noun(&row.item), colour, entry, 200.0);
        line(&mut commands, row.amount.to_string(), colour, entry, 30.0);
        if row.amount > 0 {
            button(
                &mut commands,
                entry,
                text("vending-vend"),
                VendingButton::Vend(i as u8),
            );
        } else {
            line(&mut commands, text("vending-sold-out"), SOLD_OUT, entry, 0.0);
        }
    }
    let close = commands
        .spawn((
            Node {
                margin: UiRect::top(Val::Px(6.0)),
                ..default()
            },
            ChildOf(window),
        ))
        .id();
    button(&mut commands, close, text("panel-close"), VendingButton::Close);
}

fn button(commands: &mut Commands, parent: Entity, label: String, action: VendingButton) {
    commands
        .spawn((
            action,
            Node {
                border: UiRect::all(Val::Px(1.0)),
                padding: UiRect::axes(Val::Px(5.0), Val::Px(1.0)),
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
    buttons: Query<&VendingButton>,
    mut open: ResMut<OpenVending>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
) {
    let Ok(action) = buttons.get(click.entity) else {
        return;
    };
    let Ok(mut sender) = sender.single_mut() else {
        return;
    };
    match *action {
        VendingButton::Vend(line) => {
            if let Some(view) = &open.0 {
                sender.send::<IntentChannel>(ActionInput::Vend {
                    target: view.target,
                    line,
                });
            }
        }
        VendingButton::Close => {
            sender.send::<IntentChannel>(ActionInput::ClosePanel);
            open.0 = None;
        }
    }
}
