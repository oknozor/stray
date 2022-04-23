use anyhow::anyhow;
use serde::Serialize;
use systray_rs::message::menu::{MenuItem, MenuType, TrayMenu};
use systray_rs::message::tray::StatusNotifierItem;

#[derive(Serialize)]
pub struct EwwTrayItem {
    pub icon_path: String,
    pub menu: Option<String>,
}

#[derive(Serialize)]
pub struct EwwTrayMenu {
    pub submenu: Vec<EwwTraySubMenu>,
}

impl From<&TrayMenu> for EwwTrayMenu {
    fn from(menu: &TrayMenu) -> Self {
        Self {
            submenu: menu.submenus.iter().map(EwwTraySubMenu::from).collect(),
        }
    }
}

#[derive(Serialize)]
pub struct EwwTraySubMenu {
    pub label: String,
    pub r#type: MenuType,
    pub submenu: Vec<EwwTraySubMenu>,
}

impl From<&MenuItem> for EwwTraySubMenu {
    fn from(menu: &MenuItem) -> Self {
        Self {
            label: menu.label.clone(),
            r#type: menu.menu_type,
            submenu: menu.submenu.iter().map(EwwTraySubMenu::from).collect(),
        }
    }
}

impl TryFrom<&(StatusNotifierItem, Option<TrayMenu>)> for EwwTrayItem {
    type Error = anyhow::Error;

    fn try_from(
        (item, menu): &(StatusNotifierItem, Option<TrayMenu>),
    ) -> Result<Self, Self::Error> {
        if let Some(icon_name) = &item.icon_name {
            let icon_path = match &item.icon_theme_path {
                None => None,
                Some(path) if path.is_empty() => Some(path.as_str()),
                Some(path) => Some(path.as_str()),
            };

            let icon_path = try_fetch_icon(icon_name, icon_path)?;
            let menu = menu.as_ref().map(EwwTrayMenu::from);
            let menu = menu.map(|menu| {
                menu.submenu
                    .iter()
                    .filter(|sub| sub.r#type == MenuType::Standard)
                    .map(|sub| format!("(button :class 'menu active'  '{}')", sub.label))
                    .collect::<Vec<String>>()
                    .join(" ")
            });
            Ok(Self { icon_path, menu })
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
