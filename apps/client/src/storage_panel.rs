//! An open container on the HUD, as in Luna's `orient_objs(4, 10, 4, 3)`: a column of
//! cells at screen x 4 from y 10 down, one per slot, and the close button under them.
//! Clicking an item takes it out into the active hand.

use bevy::picking::events::PointerClick;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use rs13_network::lightyear::prelude::client::*;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{ActionInput, IntentChannel, StoragePanel};
use rs13_world::{Appearance, NetId, StorageView};

use crate::hud::place;
use crate::sprites::SpriteAtlas;

/// Screen column and top row of the container's cells.
const COLUMN: u8 = 4;
const TOP_ROW: u8 = 10;

/// The container the server says is open.
#[derive(Resource, Default)]
pub struct OpenStorage(pub Option<StorageView>);

/// Part of the drawn container; despawned when it changes.
#[derive(Component)]
pub struct StorageCell;

/// What clicking a cell does.
#[derive(Component, Clone, Copy)]
pub enum StorageButton {
    Item(NetId),
    Close,
}

pub fn receive_storage(
    mut open: ResMut<OpenStorage>,
    mut receivers: Query<&mut MessageReceiver<StoragePanel>, With<Client>>,
) {
    for mut receiver in &mut receivers {
        if let Some(StoragePanel(view)) = receiver.receive().last() {
            open.0 = view;
        }
    }
}

/// Redraw the column whenever the contents change.
pub fn show_storage(
    mut commands: Commands,
    atlas: Res<SpriteAtlas>,
    open: Res<OpenStorage>,
    drawn: Query<Entity, With<StorageCell>>,
    items: Query<(&NetId, &Appearance)>,
) {
    if !open.is_changed() {
        return;
    }
    for cell in &drawn {
        commands.entity(cell).despawn();
    }
    let Some(view) = &open.0 else { return };
    for slot in 0..view.slots {
        let y = TOP_ROW - slot;
        let Some(block) = atlas.ui_image("hud", "block", 0) else {
            continue;
        };
        let mut cell = commands.spawn((StorageCell, place(COLUMN, y), block, GlobalZIndex(5)));
        let Some(id) = view.items.get(usize::from(slot)) else {
            continue;
        };
        let icon = items
            .iter()
            .find(|(n, _)| *n == id)
            .and_then(|(_, a)| atlas.ui_image(&a.sprite, "", 0));
        cell.insert((StorageButton::Item(*id), Hovered::default()))
            .observe(click_cell);
        if let Some(icon) = icon {
            cell.with_child((
                icon,
                Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    ..default()
                },
                Pickable::IGNORE,
            ));
        }
    }
    if let Some(close) = atlas.ui_image("hud", "close", 0) {
        commands
            .spawn((
                StorageCell,
                StorageButton::Close,
                place(COLUMN, TOP_ROW - view.slots),
                close,
                Hovered::default(),
                GlobalZIndex(5),
            ))
            .observe(click_cell);
    }
}

fn click_cell(
    click: On<PointerClick>,
    buttons: Query<&StorageButton>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
) {
    let (Ok(button), Ok(mut sender)) = (buttons.get(click.entity), sender.single_mut()) else {
        return;
    };
    let action = match *button {
        StorageButton::Item(id) => ActionInput::UseOn(id),
        StorageButton::Close => ActionInput::CloseStorage,
    };
    sender.send::<IntentChannel>(action);
}
