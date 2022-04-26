use crate::message::menu::TrayMenu;
use crate::message::tray::StatusNotifierItem;
use serde::Serialize;
use std::collections::HashMap;
pub mod menu;
pub mod tray;

#[derive(Debug, Serialize)]
pub struct NotifierItems {
    items: HashMap<String, StatusNotifierItem>,
}

#[derive(Debug, Serialize)]
pub enum Message {
    Update {
        id: String,
        item: StatusNotifierItem,
        menu: Option<TrayMenu>,
    },
    Remove {
        address: String,
    },
}

#[derive(Debug)]
pub enum Command {
    MenuItemClicked {
        id: i32,
        menu_path: String,
        notifier_address: String,
    }
}
