//! An autolathe's window (Luna's `autolathe` window): the metal and glass it has, and what it can
//! make with them.

use bevy::picking::events::PointerClick;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use rs13_i18n::LocMsg;
use rs13_network::lightyear::prelude::client::*;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{ActionInput, AutolathePanel, IntentChannel};
use rs13_world::AutolathePanelView;

use crate::{Loc, ui_text_font};

/// The autolathe the server says is open.
#[derive(Resource, Default)]
pub struct OpenLathe(pub Option<AutolathePanelView>);

#[derive(Component)]
pub struct LatheWindow;

#[derive(Component, Clone, Copy)]
pub enum LatheButton {
    Line(u8),
    Close,
}

const FACE: Color = Color::srgb(0.83, 0.82, 0.78);
const EDGE: Color = Color::srgb(0.45, 0.45, 0.45);
const TEXT: Color = Color::srgb(0.0, 0.0, 0.0);
const LINK: Color = Color::srgb(0.0, 0.0, 0.6);
const GREY: Color = Color::srgb(0.45, 0.45, 0.45);

pub fn receive_lathe(
    mut open: ResMut<OpenLathe>,
    mut receivers: Query<&mut MessageReceiver<AutolathePanel>, With<Client>>,
) {
    for mut receiver in &mut receivers {
        if let Some(AutolathePanel(view)) = receiver.receive().last() {
            open.0 = view;
        }
    }
}

/// Rebuild the window whenever what it shows changes.
pub fn show_lathe(
    mut commands: Commands,
    loc: Res<Loc>,
    open: Res<OpenLathe>,
    windows: Query<Entity, With<LatheWindow>>,
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
            LatheWindow,
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
    label(
        &mut commands,
        text(
            LocMsg::new("lathe-title")
                .num("metal", i64::from(view.metal))
                .num("glass", i64::from(view.glass)),
        ),
    );
    if view.busy {
        label(&mut commands, text(LocMsg::new("lathe-busy")));
    }
    // In two columns: there are many, and the window has to fit the map beside it.
    let columns = commands
        .spawn((
            Node {
                display: Display::Grid,
                grid_template_columns: RepeatedGridTrack::flex(2, 1.0),
                column_gap: Val::Px(8.0),
                ..default()
            },
            ChildOf(window),
        ))
        .id();
    for (line, row) in view.rows.iter().enumerate() {
        let shown = text(
            LocMsg::new("lathe-row")
                .arg("item", row.item.clone())
                .num("metal", i64::from(row.metal))
                .num("glass", i64::from(row.glass)),
        );
        if row.can_make {
            button(&mut commands, columns, shown, LatheButton::Line(line as u8));
        } else {
            commands.spawn((
                Text::new(shown),
                ui_text_font(14.0),
                TextColor(GREY),
                ChildOf(columns),
            ));
        }
    }
    button(
        &mut commands,
        window,
        text(LocMsg::new("panel-close")),
        LatheButton::Close,
    );
}

fn button(commands: &mut Commands, parent: Entity, label: String, action: LatheButton) {
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
    buttons: Query<&LatheButton>,
    mut open: ResMut<OpenLathe>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
) {
    let Ok(action) = buttons.get(click.entity) else {
        return;
    };
    let Ok(mut sender) = sender.single_mut() else {
        return;
    };
    match *action {
        LatheButton::Line(line) => {
            if let Some(view) = &open.0 {
                sender.send::<IntentChannel>(ActionInput::Autolathe {
                    target: view.target,
                    line,
                });
            }
        }
        LatheButton::Close => {
            sender.send::<IntentChannel>(ActionInput::ClosePanel);
            open.0 = None;
        }
    }
}
