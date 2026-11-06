//! The spell window (Luna's "Spells" panel): F7 asks for it, and only someone who knows a spell
//! gets one. Spec: `docs/reference/spells.md`.

use bevy::picking::events::PointerClick;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use rs13_i18n::LocMsg;
use rs13_network::lightyear::prelude::client::*;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{ActionInput, IntentChannel, SpellCommand, SpellPanel};
use rs13_world::{NetId, SpellAim, SpellPanelView};

use crate::{ChatInput, Loc, ui_text_font};

/// The window, if it's open, and the spell being aimed.
#[derive(Resource, Default)]
pub struct OpenSpells {
    pub view: Option<SpellPanelView>,
    picking: Option<u8>,
}

#[derive(Component)]
pub struct SpellWindow;

#[derive(Component, Clone, Copy)]
pub enum SpellButton {
    Cast(u8),
    Target(NetId),
    Area(u16),
    Cancel,
    Close,
}

const FACE: Color = Color::srgb(0.83, 0.82, 0.78);
const EDGE: Color = Color::srgb(0.45, 0.45, 0.45);
const TEXT: Color = Color::srgb(0.0, 0.0, 0.0);
const LINK: Color = Color::srgb(0.0, 0.0, 0.6);

/// F7: ask for the window, or close it.
pub fn toggle_spells(
    keys: Res<ButtonInput<KeyCode>>,
    chat: Res<ChatInput>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
) {
    if chat.open || !keys.just_pressed(KeyCode::F7) {
        return;
    }
    if let Ok(mut sender) = sender.single_mut() {
        sender.send::<IntentChannel>(ActionInput::Spell(SpellCommand::Open));
    }
}

pub fn receive_spells(
    mut open: ResMut<OpenSpells>,
    mut receivers: Query<&mut MessageReceiver<SpellPanel>, With<Client>>,
) {
    for mut receiver in &mut receivers {
        if let Some(SpellPanel(view)) = receiver.receive().last() {
            if view.is_none() {
                open.picking = None;
            }
            open.view = view;
        }
    }
}

/// Rebuild the window whenever what it shows changes.
pub fn show_spells(
    mut commands: Commands,
    loc: Res<Loc>,
    open: Res<OpenSpells>,
    windows: Query<Entity, With<SpellWindow>>,
) {
    if !open.is_changed() {
        return;
    }
    for window in &windows {
        commands.entity(window).despawn();
    }
    let Some(view) = &open.view else { return };
    let text = |msg: LocMsg| loc.0.format_msg(&msg);
    let window = commands
        .spawn((
            SpellWindow,
            Node {
                position_type: PositionType::Absolute,
                right: Val::VMin(40.0),
                top: Val::VMin(3.0),
                min_width: Val::Px(260.0),
                max_height: Val::Vh(80.0),
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Center,
                border: UiRect::all(Val::Px(2.0)),
                padding: UiRect::all(Val::Px(8.0)),
                row_gap: Val::Px(2.0),
                ..default()
            },
            BackgroundColor(FACE),
            BorderColor::all(EDGE),
            // Clicks on the window never reach the map under it.
            Hovered::default(),
            GlobalZIndex(10),
        ))
        .id();
    commands.spawn((
        Text::new(text(LocMsg::new("spell-title"))),
        ui_text_font(14.0),
        TextColor(TEXT),
        ChildOf(window),
    ));
    if let Some(line) = open.picking {
        let aim = view.spells.get(usize::from(line)).map(|s| s.aim);
        commands.spawn((
            Text::new(text(LocMsg::new("spell-choose-area"))),
            ui_text_font(14.0),
            TextColor(TEXT),
            ChildOf(window),
        ));
        match aim {
            Some(SpellAim::Area) => {
                for (i, name) in view.areas.iter().enumerate() {
                    button(&mut commands, window, name.clone(), SpellButton::Area(i as u16));
                }
            }
            Some(SpellAim::Someone) => {
                for (id, name) in &view.nearby {
                    button(&mut commands, window, name.clone(), SpellButton::Target(*id));
                }
            }
            _ => {}
        }
        button(
            &mut commands,
            window,
            text(LocMsg::new("spell-cancel")),
            SpellButton::Cancel,
        );
        return;
    }
    for (line, spell) in view.spells.iter().enumerate() {
        let shown = text(
            LocMsg::new("spell-line")
                .msg("name", format!("spell-{}", spell.id))
                .num("value", i64::from(spell.now))
                .num("max", i64::from(spell.max)),
        );
        button(&mut commands, window, shown, SpellButton::Cast(line as u8));
    }
    button(
        &mut commands,
        window,
        text(LocMsg::new("panel-close")),
        SpellButton::Close,
    );
}

fn button(commands: &mut Commands, parent: Entity, label: String, action: SpellButton) {
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
    buttons: Query<&SpellButton>,
    mut open: ResMut<OpenSpells>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
) {
    let Ok(action) = buttons.get(click.entity) else {
        return;
    };
    let Ok(mut sender) = sender.single_mut() else {
        return;
    };
    let mut send = |command: SpellCommand| {
        sender.send::<IntentChannel>(ActionInput::Spell(command));
    };
    match *action {
        SpellButton::Cast(line) => {
            let aim = open
                .view
                .as_ref()
                .and_then(|v| v.spells.get(usize::from(line)))
                .map(|s| s.aim);
            if matches!(aim, Some(SpellAim::Area | SpellAim::Someone)) {
                open.picking = Some(line);
            } else {
                send(SpellCommand::Cast {
                    spell: line,
                    target: None,
                    area: None,
                });
            }
        }
        SpellButton::Target(target) => {
            let Some(line) = open.picking.take() else { return };
            send(SpellCommand::Cast {
                spell: line,
                target: Some(target),
                area: None,
            });
        }
        SpellButton::Area(area) => {
            let Some(line) = open.picking.take() else { return };
            send(SpellCommand::Cast {
                spell: line,
                target: None,
                area: Some(area),
            });
        }
        SpellButton::Cancel => open.picking = None,
        SpellButton::Close => send(SpellCommand::Open),
    }
}
