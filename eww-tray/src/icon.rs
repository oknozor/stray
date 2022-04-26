use anyhow::anyhow;
use serde::Serialize;
use std::collections::HashMap;

use stray::message::menu::{MenuItem, MenuType, TrayMenu};
use stray::message::tray::StatusNotifierItem;

#[derive(Serialize, Debug)]
pub struct EwwTrayOutput<'a> {
    pub(crate) icons: Vec<&'a EwwTrayItem>,
    pub(crate) menus: &'a HashMap<String, Vec<EwwTraySubMenu>>,
}

#[derive(Serialize, Debug)]
pub struct EwwTrayItem {
    pub id: String,
    pub icon_path: String,
}

#[derive(Serialize, Debug)]
pub struct EwwTrayMenu {
    pub id: u32,
    pub submenu: Vec<EwwTraySubMenu>,
}

impl From<&TrayMenu> for EwwTrayMenu {
    fn from(menu: &TrayMenu) -> Self {
        Self {
            id: menu.id,
            submenu: menu.submenus.iter().map(EwwTraySubMenu::from).collect(),
        }
    }
}

#[derive(Serialize, Debug)]
pub struct EwwTraySubMenu {
    pub id: i32,
    pub label: String,
    pub r#type: MenuType,
    pub submenu: Vec<EwwTraySubMenu>,
}

impl From<&MenuItem> for EwwTraySubMenu {
    fn from(menu: &MenuItem) -> Self {
        Self {
            id: menu.id,
            label: menu.label.clone(),
            r#type: menu.menu_type,
            submenu: menu.submenu.iter().map(EwwTraySubMenu::from).collect(),
        }
    }
}

impl TryFrom<&StatusNotifierItem> for EwwTrayItem {
    type Error = anyhow::Error;

    fn try_from(item: &StatusNotifierItem) -> Result<Self, Self::Error> {
        if let Some(icon_name) = &item.icon_name {
            let icon_path = match &item.icon_theme_path {
                None => None,
                Some(path) if path.is_empty() => Some(path.as_str()),
                Some(path) => Some(path.as_str()),
            };

            let icon_path = try_fetch_icon(icon_name, icon_path)?;
            Ok(Self {
                id: item.id.clone(),
                icon_path,
            })
        } else {
            Err(anyhow!("No icon found"))
        }
    }
}

const FALL_BACK_THEME: &str = "hicolor";

fn try_fetch_icon(name: &str, additional_search_path: Option<&str>) -> anyhow::Result<String> {
    match additional_search_path {
        Some(path) if !path.is_empty() => {
            return Ok(format!("{path}/{name}.png"));
        }
        _ => {
            let theme = linicon::get_system_theme().unwrap();
            linicon::lookup_icon(name)
                .from_theme(theme)
                .use_fallback_themes(true)
                .next()
                .and_then(|icon| icon.ok())
                .or_else(|| {
                    linicon::lookup_icon(name)
                        .from_theme(FALL_BACK_THEME)
                        .next()
                        .and_then(|icon| icon.ok())
                })
                .map(|icon| icon.path.to_str().unwrap().to_string())
                .ok_or_else(|| anyhow!("Icon not found"))
        }
    }
}
