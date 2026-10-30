//! The equipment panel: one grey box per worn slot in the bottom-right corner, and the
//! Internals button under them.

use bevy::picking::events::PointerClick;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use rs13_i18n::Case;
use rs13_network::lightyear::prelude::client::*;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{ActionInput, IntentChannel};
use rs13_world::{Appearance, HandsView, NetId, Slot, WornView};

use crate::{Loc, OwnPlayer, OwnStatus, ui_text_font};

/// Top to bottom, head to toe as on Luna's inventory screen.
const PANEL_ORDER: [Slot; 9] = [
    Slot::Head,
    Slot::Mask,
    Slot::Suit,
    Slot::Uniform,
    Slot::Id,
    Slot::Gloves,
    Slot::Belt,
    Slot::Back,
    Slot::Shoes,
];

const PANEL_GREY: Color = Color::srgb(0.75, 0.75, 0.72);
const BORDER_DARK: Color = Color::srgb(0.4, 0.4, 0.4);
const BORDER_LIT: Color = Color::srgb(0.95, 0.95, 0.95);
const INTERNALS_ON: Color = Color::srgb(0.55, 0.8, 0.55);

#[derive(Component)]
pub struct SlotBox(Slot);

#[derive(Component)]
pub struct SlotLabel(Slot);

#[derive(Component)]
pub struct InternalsButton;

#[derive(Component)]
pub struct InternalsLabel;

fn panel_box(width: f32) -> (Hovered, Node, BackgroundColor, BorderColor) {
    (
        Hovered::default(),
        Node {
            width: Val::Px(width),
            height: Val::Px(24.0),
            border: UiRect::all(Val::Px(2.0)),
            padding: UiRect::horizontal(Val::Px(6.0)),
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(PANEL_GREY),
        BorderColor::all(BORDER_DARK),
    )
}

pub fn spawn_panel(mut commands: Commands) {
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            right: Val::Px(8.0),
            bottom: Val::Px(8.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(3.0),
            ..default()
        })
        .with_children(|panel| {
            for slot in PANEL_ORDER {
                panel
                    .spawn((SlotBox(slot), panel_box(200.0)))
                    .with_child((
                        SlotLabel(slot),
                        Text::new(""),
                        ui_text_font(13.0),
                        TextColor(Color::BLACK),
                        Pickable::IGNORE,
                    ))
                    .observe(click_panel);
            }
            panel
                .spawn((InternalsButton, panel_box(200.0)))
                .with_child((
                    InternalsLabel,
                    Text::new(""),
                    ui_text_font(13.0),
                    TextColor(Color::BLACK),
                    Pickable::IGNORE,
                ))
                .observe(click_panel);
        });
}

/// Slot labels show what is worn there; the Internals button lights up while in use.
pub fn update_panel(
    loc: Res<Loc>,
    status: Res<OwnStatus>,
    own: Query<&WornView, With<OwnPlayer>>,
    items: Query<(&NetId, &Appearance)>,
    mut labels: Query<(&SlotLabel, &mut Text), Without<InternalsLabel>>,
    mut internals_label: Single<&mut Text, With<InternalsLabel>>,
    mut internals_box: Single<&mut BackgroundColor, With<InternalsButton>>,
    mut boxes: Query<(&Hovered, &mut BorderColor)>,
) {
    let worn = own.single().ok();
    for (SlotLabel(slot), mut text) in &mut labels {
        let mut args = rs13_i18n::Args::new();
        args.set("slot", slot.id());
        let name = loc.0.format("hud-slot", Some(&args)).unwrap_or_default();
        let item = worn
            .and_then(|w| w.get(*slot))
            .and_then(|id| items.iter().find(|(n, _)| **n == id))
            .map(|(_, a)| loc.0.noun(&a.sprite, Case::Nom))
            .unwrap_or_else(|| "—".to_owned());
        let new = format!("{name}: {item}");
        if text.0 != new {
            text.0 = new;
        }
    }
    let on = status.0.as_ref().is_some_and(|s| s.internals);
    let label = loc.0.format("hud-internals", None).unwrap_or_default();
    let new = format!("{label}: {}", if on { "✓" } else { "—" });
    if internals_label.0 != new {
        internals_label.0 = new;
    }
    internals_box.set_if_neq(BackgroundColor(if on { INTERNALS_ON } else { PANEL_GREY }));
    for (hovered, mut border) in &mut boxes {
        let lit = hovered.0;
        border.set_if_neq(BorderColor::all(if lit { BORDER_LIT } else { BORDER_DARK }));
    }
}

/// A slot click puts on the active-hand item there, or with an empty hand takes off what
/// is there. The server checks the rules.
fn click_panel(
    click: On<PointerClick>,
    slots: Query<&SlotBox>,
    internals: Query<(), With<InternalsButton>>,
    own: Query<&HandsView, With<OwnPlayer>>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
) {
    let Ok(mut sender) = sender.single_mut() else {
        return;
    };
    if let Ok(SlotBox(slot)) = slots.get(click.entity) {
        let holding = own.single().is_ok_and(|hands| match hands.active {
            rs13_world::Hand::Left => hands.left.is_some(),
            rs13_world::Hand::Right => hands.right.is_some(),
        });
        sender.send::<IntentChannel>(if holding {
            ActionInput::Equip(*slot)
        } else {
            ActionInput::Unequip(*slot)
        });
    } else if internals.contains(click.entity) {
        sender.send::<IntentChannel>(ActionInput::ToggleInternals);
    }
}
