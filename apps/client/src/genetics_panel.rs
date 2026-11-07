//! An open DNA modifier console (Luna's `scan_consolenew` window): the occupant's genes, the
//! identifier and enzyme pages with their digits to click, the emitter, and the three buffers.

use bevy::picking::events::PointerClick;
use bevy::picking::hover::Hovered;
use bevy::prelude::*;
use rs13_i18n::LocMsg;
use rs13_network::lightyear::prelude::client::*;
use rs13_network::lightyear::prelude::*;
use rs13_network::protocol::{ActionInput, GeneticsPanel, IntentChannel};
use rs13_world::{GeneOp, GenePage, GeneticsPanelView, MobStat};

use crate::{Loc, ui_text_font};

/// The console the server says is open, and which buffer is choosing a block for an iso
/// injector (Luna asks in a list).
#[derive(Resource, Default)]
pub struct OpenGenetics {
    pub view: Option<GeneticsPanelView>,
    pub iso: Option<u8>,
}

#[derive(Component)]
pub struct GeneticsWindow;

#[derive(Component, Clone, Copy)]
pub enum GeneButton {
    Op(GeneOp),
    /// Choose a block for buffer N's iso injector.
    Iso(u8),
    Close,
}

const FACE: Color = Color::srgb(0.83, 0.82, 0.78);
const EDGE: Color = Color::srgb(0.45, 0.45, 0.45);
const TEXT: Color = Color::srgb(0.0, 0.0, 0.0);
const BLUE: Color = Color::srgb(0.0, 0.0, 0.8);
const GREEN: Color = Color::srgb(0.0, 0.5, 0.0);
const RED: Color = Color::srgb(0.75, 0.0, 0.0);
const LINK: Color = Color::srgb(0.0, 0.0, 0.6);
const PICKED: Color = Color::srgb(0.6, 0.0, 0.0);

pub fn receive_genetics(
    mut open: ResMut<OpenGenetics>,
    mut receivers: Query<&mut MessageReceiver<GeneticsPanel>, With<Client>>,
) {
    for mut receiver in &mut receivers {
        if let Some(GeneticsPanel(view)) = receiver.receive().last() {
            if view.is_none() {
                open.iso = None;
            }
            open.view = view;
        }
    }
}

struct Ui<'a> {
    commands: Commands<'a, 'a>,
    loc: &'a Loc,
}

impl Ui<'_> {
    fn text(&self, msg: LocMsg) -> String {
        self.loc.0.format_msg(&msg)
    }

    fn line(&mut self, parent: Entity, words: String, colour: Color) {
        self.commands.spawn((
            Text::new(words),
            ui_text_font(13.0),
            TextColor(colour),
            ChildOf(parent),
        ));
    }

    fn row(&mut self, parent: Entity) -> Entity {
        self.commands
            .spawn((
                Node {
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: Val::Px(6.0),
                    align_items: AlignItems::Center,
                    ..default()
                },
                ChildOf(parent),
            ))
            .id()
    }

    fn button(&mut self, parent: Entity, label: String, action: GeneButton, colour: Color) {
        self.commands
            .spawn((
                action,
                Node {
                    padding: UiRect::axes(Val::Px(2.0), Val::Px(1.0)),
                    ..default()
                },
                Hovered::default(),
                ChildOf(parent),
            ))
            .with_child((
                Text::new(label),
                ui_text_font(13.0),
                TextColor(colour),
                Pickable::IGNORE,
            ))
            .observe(click_button);
    }

    fn op(&mut self, parent: Entity, id: &str, op: GeneOp) {
        let label = self.text(LocMsg::new(id));
        self.button(parent, label, GeneButton::Op(op), LINK);
    }

    /// `<- label ->`, stepping with the two ops.
    fn stepper(&mut self, parent: Entity, id: &str, back: GeneOp, forth: GeneOp) {
        let row = self.row(parent);
        self.button(row, "<-".into(), GeneButton::Op(back), LINK);
        let label = self.text(LocMsg::new(id));
        self.line(row, label, TEXT);
        self.button(row, "->".into(), GeneButton::Op(forth), LINK);
    }

    /// A gene string, every digit a button; the selected one marked.
    fn digits(&mut self, parent: Entity, genes: &str, block: u8, sub: u8) {
        let row = self.row(parent);
        self.commands.entity(row).insert(Node {
            flex_direction: FlexDirection::Row,
            flex_wrap: FlexWrap::Wrap,
            column_gap: Val::Px(0.0),
            ..default()
        });
        for (i, c) in genes.chars().enumerate() {
            let (b, s) = ((i / 3 + 1) as u8, (i % 3 + 1) as u8);
            let colour = if b == block && s == sub { PICKED } else { BLUE };
            self.button(row, c.to_string(), GeneButton::Op(GeneOp::Select(b, s)), colour);
            if s == 3 {
                self.line(row, "|".into(), GREEN);
            }
        }
    }
}

/// Rebuild the window whenever what it shows changes.
pub fn show_genetics(
    commands: Commands,
    loc: Res<Loc>,
    open: Res<OpenGenetics>,
    windows: Query<Entity, With<GeneticsWindow>>,
) {
    if !open.is_changed() {
        return;
    }
    let mut ui = Ui { commands, loc: &loc };
    for window in &windows {
        ui.commands.entity(window).despawn();
    }
    let Some(view) = &open.view else { return };
    let window = ui
        .commands
        .spawn((
            GeneticsWindow,
            Node {
                position_type: PositionType::Absolute,
                left: Val::VMin(12.0),
                top: Val::VMin(4.0),
                width: Val::Px(560.0),
                max_height: Val::Vh(88.0),
                overflow: Overflow::scroll_y(),
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
    let num = |id: &str, v: i64| LocMsg::new(id).num("value", v);
    let menus = |ui: &mut Ui| {
        ui.op(window, "genetics-main-menu", GeneOp::Main);
        ui.op(window, "genetics-buffers-link", GeneOp::Buffers);
    };
    if view.no_scanner {
        let t = ui.text(LocMsg::new("genetics-no-scanner"));
        ui.line(window, t, RED);
    } else {
        match &view.page {
            GenePage::Working(seconds) => {
                let t = ui.text(num("genetics-working", i64::from(*seconds)));
                ui.line(window, t, TEXT);
            }
            GenePage::Note(id) => {
                let t = ui.text(LocMsg::new(id.clone()));
                ui.line(window, t, TEXT);
                menus(&mut ui);
            }
            GenePage::Main => main_page(&mut ui, window, view),
            GenePage::Emitter => {
                let t = ui.text(num("genetics-duration", i64::from(view.duration)));
                ui.line(window, t, GREEN);
                let t = ui.text(num("genetics-strength", i64::from(view.strength)));
                ui.line(window, t, GREEN);
                ui.stepper(
                    window,
                    "genetics-duration-label",
                    GeneOp::DurationDown,
                    GeneOp::DurationUp,
                );
                ui.stepper(
                    window,
                    "genetics-strength-label",
                    GeneOp::StrengthDown,
                    GeneOp::StrengthUp,
                );
                menus(&mut ui);
            }
            GenePage::Identity | GenePage::Enzymes => {
                let identity = view.page == GenePage::Identity;
                if let Some(o) = &view.occupant {
                    let (genes, block, title) = if identity {
                        (&o.ui, view.ui_block, "genetics-ui")
                    } else {
                        (&o.se, view.se_block, "genetics-se")
                    };
                    let t = ui.text(LocMsg::new(title).arg("value", ""));
                    ui.line(window, t, TEXT);
                    ui.digits(window, genes, block, view.sub);
                    let t = ui.text(num("genetics-block", i64::from(block)));
                    ui.line(window, t, TEXT);
                    ui.stepper(
                        window,
                        "genetics-block-label",
                        GeneOp::Block(false),
                        GeneOp::Block(true),
                    );
                    let t = ui.text(num("genetics-sub", i64::from(view.sub)));
                    ui.line(window, t, TEXT);
                    ui.stepper(
                        window,
                        "genetics-sub-label",
                        GeneOp::Sub(false),
                        GeneOp::Sub(true),
                    );
                    if identity {
                        let t = ui.text(
                            LocMsg::new("genetics-target").arg("value", format!("{:X}", view.target_digit)),
                        );
                        ui.line(window, t, TEXT);
                        ui.stepper(
                            window,
                            "genetics-target-label",
                            GeneOp::Target(false),
                            GeneOp::Target(true),
                        );
                    }
                    let t = ui.text(LocMsg::new("genetics-modify-block"));
                    ui.line(window, t, TEXT);
                    ui.op(window, "genetics-irradiate", GeneOp::Irradiate);
                }
                menus(&mut ui);
            }
            GenePage::Buffers => {
                buffers_page(&mut ui, window, view, open.iso);
                menus(&mut ui);
            }
        }
    }
    let close = ui.text(LocMsg::new("panel-close"));
    ui.button(window, close, GeneButton::Close, TEXT);
}

fn main_page(ui: &mut Ui, window: Entity, view: &GeneticsPanelView) {
    let t = ui.text(LocMsg::new("genetics-occupant"));
    ui.line(window, t, BLUE);
    match &view.occupant {
        Some(o) if o.ruined => {
            let t = ui.text(LocMsg::new("genetics-ruined"));
            ui.line(window, t, TEXT);
            ui.op(window, "genetics-buffers-menu", GeneOp::Buffers);
            ui.op(window, "genetics-emitter-menu", GeneOp::Emitter);
        }
        Some(o) => {
            let stat = match o.stat {
                MobStat::Conscious => "genetics-stat-conscious",
                MobStat::Unconscious => "genetics-stat-unconscious",
                MobStat::Dead | MobStat::Ghost => "genetics-stat-dead",
            };
            let stat = ui.text(LocMsg::new(stat));
            let t = ui.text(
                LocMsg::new("genetics-health")
                    .num("value", i64::from(o.health))
                    .arg("stat", stat),
            );
            ui.line(window, t, if o.health > 50 { BLUE } else { RED });
            let t = ui.text(LocMsg::new("genetics-radiation").num("value", i64::from(o.radiation)));
            ui.line(window, t, GREEN);
            for (id, value) in [
                ("genetics-ue", &o.ue),
                ("genetics-ui", &o.ui),
                ("genetics-se", &o.se),
            ] {
                let t = ui.text(LocMsg::new(id).arg("value", value.clone()));
                ui.line(window, t, BLUE);
            }
            ui.op(window, "genetics-modify-ui", GeneOp::Identity);
            ui.op(window, "genetics-modify-se", GeneOp::Enzymes);
            ui.op(window, "genetics-buffers-menu", GeneOp::Buffers);
            ui.op(window, "genetics-pulse", GeneOp::Pulse);
            ui.op(window, "genetics-emitter-menu", GeneOp::Emitter);
            ui.op(window, "genetics-inject", GeneOp::Inject);
        }
        None => {
            let t = ui.text(LocMsg::new("genetics-empty"));
            ui.line(window, t, TEXT);
            ui.op(window, "genetics-buffers-menu", GeneOp::Buffers);
            ui.op(window, "genetics-emitter-menu", GeneOp::Emitter);
        }
    }
    if view.occupant.is_some() {
        let id = if view.locked {
            "genetics-unlock"
        } else {
            "genetics-lock"
        };
        ui.op(window, id, GeneOp::Lock);
    }
}

fn buffers_page(ui: &mut Ui, window: Entity, view: &GeneticsPanelView, iso: Option<u8>) {
    for (i, buffer) in view.buffers.iter().enumerate() {
        let b = i as u8;
        let t = ui.text(LocMsg::new("genetics-buffer").num("value", i64::from(b) + 1));
        ui.line(window, t, TEXT);
        match buffer {
            None => {
                let t = ui.text(LocMsg::new("genetics-buffer-empty"));
                ui.line(window, t, TEXT);
            }
            Some(buffer) => {
                let t = ui.text(LocMsg::new("genetics-buffer-data").arg("value", buffer.data.clone()));
                ui.line(window, t, BLUE);
                let t = ui.text(LocMsg::new("genetics-buffer-by").arg("value", buffer.owner.clone()));
                ui.line(window, t, BLUE);
                let label = if buffer.se {
                    "genetics-label-se"
                } else if buffer.with_ue {
                    "genetics-label-uiue"
                } else {
                    "genetics-label-ui"
                };
                let t = ui.text(LocMsg::new(label));
                ui.line(window, t, BLUE);
            }
        }
        if view.occupant.is_some() {
            let row = ui.row(window);
            let t = ui.text(LocMsg::new("genetics-save"));
            ui.line(row, t, TEXT);
            ui.op(row, "genetics-save-ui", GeneOp::SaveUi(b));
            ui.op(row, "genetics-save-uiue", GeneOp::SaveUiUe(b));
            ui.op(row, "genetics-save-se", GeneOp::SaveSe(b));
        }
        if let Some(buffer) = buffer {
            let row = ui.row(window);
            let t = ui.text(LocMsg::new("genetics-transfer-to"));
            ui.line(row, t, TEXT);
            ui.op(row, "genetics-to-occupant", GeneOp::Transfer(b));
            ui.op(row, "genetics-full-injector", GeneOp::Injector(b));
            let label = ui.text(LocMsg::new("genetics-iso-injector"));
            ui.button(row, label, GeneButton::Iso(b), LINK);
            if iso == Some(b) {
                let t = ui.text(LocMsg::new("genetics-iso-pick"));
                ui.line(window, t, TEXT);
                let row = ui.row(window);
                for n in 1..=(buffer.data.len() / 3) {
                    let digits = buffer.data.get((n - 1) * 3..n * 3).unwrap_or("");
                    ui.button(
                        row,
                        format!("{n}:{digits}"),
                        GeneButton::Op(GeneOp::IsoInjector(b, n as u8)),
                        LINK,
                    );
                }
            }
            ui.op(window, "genetics-clear", GeneOp::Clear(b));
        }
    }
}

fn click_button(
    click: On<PointerClick>,
    buttons: Query<&GeneButton>,
    mut open: ResMut<OpenGenetics>,
    mut sender: Query<&mut MessageSender<ActionInput>, (With<Client>, With<Connected>)>,
) {
    let Ok(action) = buttons.get(click.entity) else {
        return;
    };
    let Ok(mut sender) = sender.single_mut() else {
        return;
    };
    match *action {
        GeneButton::Op(op) => {
            if let Some(view) = &open.view {
                let target = view.target;
                if matches!(op, GeneOp::IsoInjector(..)) {
                    open.iso = None;
                }
                sender.send::<IntentChannel>(ActionInput::Pick {
                    target,
                    line: op.line(),
                });
            }
        }
        GeneButton::Iso(b) => {
            open.iso = if open.iso == Some(b) { None } else { Some(b) };
        }
        GeneButton::Close => {
            sender.send::<IntentChannel>(ActionInput::ClosePanel);
            open.view = None;
            open.iso = None;
        }
    }
}
