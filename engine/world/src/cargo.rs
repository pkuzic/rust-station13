//! The cargo consoles' window (Luna's `supplycomp` and `ordercomp`), as shown to the player
//! using one, and its buttons.

use serde::{Deserialize, Serialize};

use crate::NetId;

/// Where the supply shuttle is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShuttleAt {
    /// At Centcom's dock: orders can be placed.
    Dock,
    /// On its way to the station, this many seconds out.
    Moving(u32),
    /// Docked at the station.
    Station,
}

/// What a cargo console's window shows besides the shuttle and points.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CargoPage {
    Main,
    /// The categories to order or request from.
    Categories,
    /// The packs of one category (its index in Luna's list), by id, with their cost.
    Packs(u8, Vec<(String, u32)>),
    /// Requests waiting for approval: pack id and who asked.
    Requests(Vec<(String, String)>),
    /// Approved orders: pack id and who approved it.
    Orders(Vec<(String, String)>),
    /// A line to read, by message id ("Thanks for your order."), with a way back.
    Note(String),
}

/// An open cargo console.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CargoPanelView {
    /// The console.
    pub target: NetId,
    /// The supply shuttle console (cargo staff), rather than the ordering console (anyone).
    pub supply: bool,
    pub points: u32,
    pub shuttle: ShuttleAt,
    pub page: CargoPage,
}

/// Luna's order categories, in its order. Their names are `cargo-category-ID` messages.
pub const CARGO_CATEGORIES: [&str; 7] = [
    "emergency",
    "security",
    "engineering",
    "medical",
    "science",
    "materials",
    "miscellaneous",
];

/// A button of a cargo console, sent as a line of `ActionInput::Pick`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CargoOp {
    Main,
    /// Open the categories, to order (the supply console) or request (the ordering console).
    Order,
    /// Send the shuttle: to the station from the dock, or back to the dock.
    Send,
    Requests,
    Orders,
    ClearRequests,
    /// Open a category.
    Category(u8),
    /// Order or request the pack at this place in the open category.
    Pack(u16),
    /// Approve or remove the request at this place in the list.
    Approve(u16),
    Remove(u16),
}

impl CargoOp {
    /// As a line: the plain buttons first, then a category (from 10), a pack (from 100), a
    /// request to approve (from 1000) or remove (from 2000).
    pub fn line(self) -> u16 {
        match self {
            CargoOp::Main => 0,
            CargoOp::Order => 1,
            CargoOp::Send => 2,
            CargoOp::Requests => 3,
            CargoOp::Orders => 4,
            CargoOp::ClearRequests => 5,
            CargoOp::Category(c) => 10 + u16::from(c),
            CargoOp::Pack(i) => 100 + i.min(899),
            CargoOp::Approve(i) => 1000 + i.min(999),
            CargoOp::Remove(i) => 2000 + i.min(999),
        }
    }

    pub fn from_line(line: u16) -> Option<Self> {
        Some(match line {
            0 => CargoOp::Main,
            1 => CargoOp::Order,
            2 => CargoOp::Send,
            3 => CargoOp::Requests,
            4 => CargoOp::Orders,
            5 => CargoOp::ClearRequests,
            10..=99 => CargoOp::Category(u8::try_from(line - 10).ok()?),
            100..=999 => CargoOp::Pack(line - 100),
            1000..=1999 => CargoOp::Approve(line - 1000),
            2000..=2999 => CargoOp::Remove(line - 2000),
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cargo_buttons_survive_being_sent_as_lines() {
        let ops = [
            CargoOp::Main,
            CargoOp::Order,
            CargoOp::Send,
            CargoOp::Requests,
            CargoOp::Orders,
            CargoOp::ClearRequests,
            CargoOp::Category(6),
            CargoOp::Pack(12),
            CargoOp::Approve(3),
            CargoOp::Remove(0),
        ];
        for op in ops {
            assert_eq!(CargoOp::from_line(op.line()), Some(op), "{op:?}");
        }
        assert_eq!(CargoOp::from_line(6), None);
    }
}
