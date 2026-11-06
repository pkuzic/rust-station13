//! An open spellbook (Luna's "The Book of Spells" window): spells left to learn, each spell
//! with its recharge and what it does, the artefacts, Re-memorize, and Close.

use bevy::picking::events::PointerClick;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use rs13_i18n::LocMsg;
use rs13_network::lightyear::prelude::client::*;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{ActionInput, IntentChannel, SpellbookPanel};
use rs13_world::SpellbookView;

use crate::{Loc, ui_text_font};

/// The book the server says is open.
#[derive(Resource, Default)]
pub struct OpenSpellbook(pub Option<SpellbookView>);

#[derive(Component)]
pub struct SpellbookWindow;

#[derive(Component, Clone, Copy)]
pub enum BookButton {
    Line(u16),
    Close,
}

const FACE: Color = Color::srgb(0.83, 0.82, 0.78);
const EDGE: Color = Color::srgb(0.45, 0.45, 0.45);
const TEXT: Color = Color::srgb(0.0, 0.0, 0.0);
const NOTE: Color = Color::srgb(0.25, 0.25, 0.25);
const LINK: Color = Color::srgb(0.0, 0.0, 0.6);

pub fn receive_spellbook(
    mut open: ResMut<OpenSpellbook>,
    mut receivers: Query<&mut MessageReceiver<SpellbookPanel>, With<Client>>,
) {
    for mut receiver in &mut receivers {
        if let Some(SpellbookPanel(view)) = receiver.receive().last() {
            open.0 = view;
        }
    }
}

/// Rebuild the window whenever what it shows changes.
pub fn show_spellbook(
    mut commands: Commands,
    loc: Res<Loc>,
    open: Res<OpenSpellbook>,
    windows: Query<Entity, With<SpellbookWindow>>,
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
            SpellbookWindow,
            Node {
                position_type: PositionType::Absolute,
                left: Val::VMin(15.0),
                top: Val::VMin(4.0),
                width: Val::Px(440.0),
                max_height: Val::Vh(88.0),
                overflow: Overflow::scroll_y(),
                flex_direction: FlexDirection::Column,
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
    let line = |commands: &mut Commands, words: String, size: f32, colour: Color| {
        commands.spawn((
            Text::new(words),
            ui_text_font(size),
            TextColor(colour),
            ChildOf(window),
        ));
    };
    line(&mut commands, text(LocMsg::new("book-title")), 15.0, TEXT);
    line(
        &mut commands,
        text(LocMsg::new("book-uses").num("value", i64::from(view.uses))),
        13.0,
        TEXT,
    );
    line(&mut commands, text(LocMsg::new("book-choose")), 13.0, TEXT);
    let mut n: u16 = 0;
    for (id, seconds) in &view.spells {
        let label = if *seconds == 0 {
            text(LocMsg::new("book-line-once").msg("name", format!("book-{id}")))
        } else {
            text(
                LocMsg::new("book-line")
                    .msg("name", format!("book-{id}"))
                    .num("value", i64::from(*seconds)),
            )
        };
        button(&mut commands, window, label, BookButton::Line(n), LINK);
        line(
            &mut commands,
            text(LocMsg::new(format!("book-desc-{id}"))),
            12.0,
            NOTE,
        );
        n += 1;
    }
    line(&mut commands, text(LocMsg::new("book-artefacts")), 14.0, TEXT);
    line(
        &mut commands,
        text(LocMsg::new("book-artefacts-note")),
        12.0,
        NOTE,
    );
    for id in &view.artefacts {
        button(
            &mut commands,
            window,
            text(LocMsg::new(format!("book-{id}"))),
            BookButton::Line(n),
            LINK,
        );
        line(
            &mut commands,
            text(LocMsg::new(format!("book-desc-{id}"))),
            12.0,
            NOTE,
        );
        n += 1;
    }
    button(
        &mut commands,
        window,
        text(LocMsg::new("book-rememorize")),
        BookButton::Line(n),
        LINK,
    );
    button(
        &mut commands,
        window,
        text(LocMsg::new("panel-close")),
        BookButton::Close,
        TEXT,
    );
}

fn button(commands: &mut Commands, parent: Entity, label: String, action: BookButton, colour: Color) {
    commands
        .spawn((
            action,
            Node {
                padding: UiRect::axes(Val::Px(3.0), Val::Px(1.0)),
                margin: UiRect::top(Val::Px(4.0)),
                ..default()
            },
            Hovered::default(),
            ChildOf(parent),
        ))
        .with_child((
            Text::new(label),
            ui_text_font(14.0),
            TextColor(colour),
            Pickable::IGNORE,
        ))
        .observe(click_button);
}

fn click_button(
    click: On<PointerClick>,
    buttons: Query<&BookButton>,
    mut open: ResMut<OpenSpellbook>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
) {
    let Ok(action) = buttons.get(click.entity) else {
        return;
    };
    let Ok(mut sender) = sender.single_mut() else {
        return;
    };
    match *action {
        BookButton::Line(line) => {
            if let Some(view) = &open.0 {
                sender.send::<IntentChannel>(ActionInput::Pick {
                    target: view.target,
                    line,
                });
            }
        }
        BookButton::Close => {
            sender.send::<IntentChannel>(ActionInput::ClosePanel);
            open.0 = None;
        }
    }
}
