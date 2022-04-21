use anyhow::anyhow;
use systray_rs::tray::StatusNotifierItem;
use serde::{Serialize};
use systray_rs::menu::{MenuType, SubMenu, TrayMenu};

#[derive(Serialize)]
pub struct EwwTrayItem {
    icon_path: String,
    menu: Option<EwwTrayMenu>,
}

#[derive(Serialize)]
pub struct EwwTrayMenu {
    submenu: Vec<EwwTraySubMenu>
}

impl From<&TrayMenu> for EwwTrayMenu {
    fn from(menu: &TrayMenu) -> Self {
        Self {
            submenu: menu.submenus
                .iter()
                .map(EwwTraySubMenu::from)
                .collect()
        }
    }
}

#[derive(Serialize)]
pub struct EwwTraySubMenu {
    label: String,
    r#type: MenuType,
    submenu: Vec<EwwTraySubMenu>
}

impl From<&SubMenu> for EwwTraySubMenu {
    fn from(menu: &SubMenu) -> Self {
        Self {
            label: menu.label.clone(),
            r#type: menu.menu_type,
            submenu: menu.submenu.iter()
                .map(EwwTraySubMenu::from)
                .collect()
        }
    }
}

impl TryFrom<&(StatusNotifierItem, Option<TrayMenu>)> for EwwTrayItem {
    type Error = anyhow::Error;

    fn try_from((item, menu): &(StatusNotifierItem, Option<TrayMenu>)) -> Result<Self, Self::Error> {
        if let Some(icon_name) = &item.icon_name {
            let icon_path = match &item.icon_theme_path {
                None => None,
                Some(path) if path.is_empty() => Some(path.as_str()),
                Some(path) => Some(path.as_str()),
            };

            let icon_path = try_fetch_icon(icon_name, icon_path)?;
            let menu = menu.as_ref().map(EwwTrayMenu::from);

            Ok(Self {
                icon_path,
                menu
            })
        } else {
            Err(anyhow!("No icon found"))
        }
    }
}

const FALL_BACK_THEME: &str = "hicolor";

fn try_fetch_icon(name: &str, additional_search_path: Option<&str>) -> anyhow::Result<String> {
    if let Some(path) = additional_search_path {
        return Ok(format!("{path}/{name}.png"));
    };

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

