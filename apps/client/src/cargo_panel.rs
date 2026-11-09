//! A cargo console's window (Luna's `supplycomp` and `ordercomp`): request, order and approve
//! supply packs, and send the supply shuttle (`cargo.md`).

use bevy::picking::events::PointerClick;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use rs13_i18n::LocMsg;
use rs13_network::lightyear::prelude::client::*;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{ActionInput, CargoPanel, IntentChannel};
use rs13_world::{CARGO_CATEGORIES, CargoOp, CargoPage, CargoPanelView, ShuttleAt};

use crate::{Loc, ui_text_font};

/// The console the server says is open.
#[derive(Resource, Default)]
pub struct OpenCargo(pub Option<CargoPanelView>);

#[derive(Component)]
pub struct CargoWindow;

#[derive(Component, Clone, Copy)]
pub enum CargoButton {
    Op(CargoOp),
    Close,
}

const FACE: Color = Color::srgb(0.83, 0.82, 0.78);
const EDGE: Color = Color::srgb(0.45, 0.45, 0.45);
const TEXT: Color = Color::srgb(0.0, 0.0, 0.0);
const LINK: Color = Color::srgb(0.0, 0.0, 0.6);

pub fn receive_cargo(
    mut open: ResMut<OpenCargo>,
    mut receivers: Query<&mut MessageReceiver<CargoPanel>, With<Client>>,
) {
    for mut receiver in &mut receivers {
        if let Some(CargoPanel(view)) = receiver.receive().last() {
            open.0 = view;
        }
    }
}

/// The lines of a window, top to bottom: a line of text, or a button.
pub enum Line {
    Text(LocMsg),
    Button(LocMsg, CargoOp),
}

fn pack_name(id: &str) -> LocMsg {
    LocMsg::new(format!("cargo-pack-{id}"))
}

/// What the window shows for a view, as lines (kept apart from drawing so it can be tested).
pub fn lines(view: &CargoPanelView, name: impl Fn(&LocMsg) -> String) -> Vec<Line> {
    let mut out = vec![
        Line::Text(LocMsg::new(if view.supply {
            "cargo-title-supply"
        } else {
            "cargo-title-order"
        })),
        Line::Text(match view.shuttle {
            ShuttleAt::Dock => LocMsg::new("cargo-at-dock"),
            ShuttleAt::Station => LocMsg::new("cargo-at-station"),
            ShuttleAt::Moving(s) => LocMsg::new("cargo-moving").num("value", i64::from(s)),
        }),
        Line::Text(LocMsg::new("cargo-points").num("value", i64::from(view.points))),
    ];
    let main = Line::Button(LocMsg::new("cargo-main-menu"), CargoOp::Main);
    match &view.page {
        CargoPage::Main => {
            let docked = view.shuttle == ShuttleAt::Dock;
            if !view.supply {
                out.push(Line::Button(LocMsg::new("cargo-request"), CargoOp::Order));
            } else if docked {
                out.push(Line::Button(LocMsg::new("cargo-order"), CargoOp::Order));
            } else {
                out.push(Line::Text(LocMsg::new("cargo-must-dock-line")));
            }
            if view.supply {
                out.push(match view.shuttle {
                    ShuttleAt::Dock => Line::Button(LocMsg::new("cargo-send-to-station"), CargoOp::Send),
                    ShuttleAt::Station => Line::Button(LocMsg::new("cargo-send-to-dock"), CargoOp::Send),
                    ShuttleAt::Moving(_) => Line::Text(LocMsg::new("cargo-already-called")),
                });
            }
            out.push(Line::Button(
                LocMsg::new("cargo-view-requests"),
                CargoOp::Requests,
            ));
            out.push(Line::Button(LocMsg::new("cargo-view-orders"), CargoOp::Orders));
        }
        CargoPage::Categories => {
            for (i, id) in CARGO_CATEGORIES.iter().enumerate() {
                out.push(Line::Button(
                    LocMsg::new(format!("cargo-category-{id}")),
                    CargoOp::Category(i as u8),
                ));
            }
            out.push(main);
        }
        CargoPage::Packs(category, packs) => {
            if let Some(id) = CARGO_CATEGORIES.get(usize::from(*category)) {
                out.push(Line::Text(LocMsg::new(format!("cargo-category-{id}"))));
            }
            out.push(Line::Text(LocMsg::new("cargo-request-what")));
            for (i, (pack, cost)) in packs.iter().enumerate() {
                out.push(Line::Button(
                    LocMsg::new("cargo-pack-row")
                        .arg("name", name(&pack_name(pack)))
                        .num("value", i64::from(*cost)),
                    CargoOp::Pack(i as u16),
                ));
            }
            out.push(Line::Button(LocMsg::new("cargo-back"), CargoOp::Order));
            out.push(main);
        }
        CargoPage::Requests(requests) => {
            if requests.is_empty() {
                out.push(Line::Text(LocMsg::new("cargo-none")));
            }
            let docked = view.shuttle == ShuttleAt::Dock;
            for (i, (pack, by)) in requests.iter().enumerate() {
                out.push(Line::Text(
                    LocMsg::new("cargo-request-row")
                        .arg("name", name(&pack_name(pack)))
                        .arg("by", by.clone()),
                ));
                if view.supply && docked {
                    out.push(Line::Button(
                        LocMsg::new("cargo-approve"),
                        CargoOp::Approve(i as u16),
                    ));
                    out.push(Line::Button(
                        LocMsg::new("cargo-remove"),
                        CargoOp::Remove(i as u16),
                    ));
                }
            }
            if view.supply {
                out.push(Line::Button(LocMsg::new("cargo-clear"), CargoOp::ClearRequests));
            }
            out.push(main);
        }
        CargoPage::Orders(orders) => {
            if orders.is_empty() {
                out.push(Line::Text(LocMsg::new("cargo-none")));
            }
            for (pack, by) in orders {
                out.push(Line::Text(
                    LocMsg::new("cargo-order-row")
                        .arg("name", name(&pack_name(pack)))
                        .arg("by", by.clone()),
                ));
            }
            out.push(main);
        }
        CargoPage::Note(id) => {
            out.push(Line::Text(LocMsg::new(id.clone())));
            out.push(main);
        }
    }
    out
}

/// Rebuild the window whenever what it shows changes.
pub fn show_cargo(
    mut commands: Commands,
    loc: Res<Loc>,
    open: Res<OpenCargo>,
    windows: Query<Entity, With<CargoWindow>>,
) {
    if !open.is_changed() {
        return;
    }
    for window in &windows {
        commands.entity(window).despawn();
    }
    let Some(view) = &open.0 else { return };
    let text = |msg: &LocMsg| loc.0.format_msg(msg);
    let window = commands
        .spawn((
            CargoWindow,
            Node {
                position_type: PositionType::Absolute,
                left: Val::VMin(14.0),
                top: Val::VMin(6.0),
                min_width: Val::Px(360.0),
                max_height: Val::VMin(86.0),
                flex_direction: FlexDirection::Column,
                align_items: AlignItems::Start,
                border: UiRect::all(Val::Px(2.0)),
                padding: UiRect::all(Val::Px(8.0)),
                row_gap: Val::Px(3.0),
                overflow: Overflow::scroll_y(),
                ..default()
            },
            BackgroundColor(FACE),
            BorderColor::all(EDGE),
            // Clicks on the window never reach the map under it.
            Hovered::default(),
            GlobalZIndex(10),
        ))
        .id();
    for line in lines(view, text) {
        match line {
            Line::Text(msg) => {
                commands.spawn((
                    Text::new(text(&msg)),
                    ui_text_font(14.0),
                    TextColor(TEXT),
                    ChildOf(window),
                ));
            }
            Line::Button(msg, op) => button(&mut commands, window, text(&msg), CargoButton::Op(op)),
        }
    }
    button(
        &mut commands,
        window,
        text(&LocMsg::new("panel-close")),
        CargoButton::Close,
    );
}

fn button(commands: &mut Commands, parent: Entity, label: String, action: CargoButton) {
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
    buttons: Query<&CargoButton>,
    mut open: ResMut<OpenCargo>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
) {
    let Ok(action) = buttons.get(click.entity) else {
        return;
    };
    let Ok(mut sender) = sender.single_mut() else {
        return;
    };
    match *action {
        CargoButton::Op(op) => {
            if let Some(view) = &open.0 {
                sender.send::<IntentChannel>(ActionInput::Pick {
                    target: view.target,
                    line: op.line(),
                });
            }
        }
        CargoButton::Close => {
            sender.send::<IntentChannel>(ActionInput::ClosePanel);
            open.0 = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn view(supply: bool, shuttle: ShuttleAt, page: CargoPage) -> CargoPanelView {
        CargoPanelView {
            target: rs13_world::NetId(1),
            supply,
            points: 50,
            shuttle,
            page,
        }
    }

    fn buttons(lines: &[Line]) -> Vec<CargoOp> {
        lines
            .iter()
            .filter_map(|l| match l {
                Line::Button(_, op) => Some(*op),
                Line::Text(_) => None,
            })
            .collect()
    }

    #[test]
    fn only_the_supply_console_sends_the_shuttle_and_orders_only_at_the_dock() {
        let name = |_: &LocMsg| String::new();
        let order = buttons(&lines(&view(false, ShuttleAt::Dock, CargoPage::Main), name));
        assert!(order.contains(&CargoOp::Order) && !order.contains(&CargoOp::Send));
        let supply = buttons(&lines(&view(true, ShuttleAt::Dock, CargoPage::Main), name));
        assert!(supply.contains(&CargoOp::Order) && supply.contains(&CargoOp::Send));
        let away = buttons(&lines(&view(true, ShuttleAt::Moving(30), CargoPage::Main), name));
        assert!(!away.contains(&CargoOp::Order) && !away.contains(&CargoOp::Send));
    }

    #[test]
    fn requests_can_be_approved_on_the_supply_console() {
        let page = CargoPage::Requests(vec![("metal50".to_owned(), "Ann".to_owned())]);
        let name = |_: &LocMsg| String::new();
        let supply = buttons(&lines(&view(true, ShuttleAt::Dock, page.clone()), name));
        assert!(supply.contains(&CargoOp::Approve(0)) && supply.contains(&CargoOp::Remove(0)));
        let order = buttons(&lines(&view(false, ShuttleAt::Dock, page), name));
        assert!(!order.contains(&CargoOp::Approve(0)));
    }
}
