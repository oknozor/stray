

use std::str;
use std::str::FromStr;
use serde::Serialize;


use zbus::zvariant::{OwnedValue, Structure, Value};

use crate::dbus::dbusmenu::{MenuLayout};

#[derive(Debug, Serialize)]
pub struct TrayMenu {
    pub id: u32,
    pub submenus: Vec<SubMenu>
}

#[derive(Debug, Serialize)]
pub struct SubMenu {
    pub id: i32,
    pub children_display: Option<String>,
    pub label: String,
    pub enabled: bool,
    pub visible: bool,
    pub icon_name: Option<String>,
    pub toggle_state: Option<bool>,
    pub toggle_type: Option<ToggleType>,
    pub menu_type: MenuType,
    pub disposition: Disposition,
    pub submenu: Vec<SubMenu>,
}

impl Default for SubMenu {
    fn default() -> Self {
        Self {
            id: 0,
            children_display: None,
            label: "".to_string(),
            enabled: true,
            visible: true,
            icon_name: None,
            toggle_state: None,
            toggle_type: None,
            menu_type: MenuType::Standard,
            disposition: Disposition::Normal,
            submenu: vec![]
        }
    }
}

#[derive(Debug, Serialize, Copy, Clone)]
pub enum ToggleType {
    Checkmark,
    Radio,
}

#[derive(Debug, Serialize, Copy, Clone)]
pub enum MenuType {
    Separator,
    Standard,
}

#[derive(Debug, Serialize, Copy, Clone)]
pub enum Disposition {
    Normal,
    Informative,
    Warning,
    Alert,
}

impl FromStr for MenuType {
    type Err = zbus::zvariant::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "standard" => Ok(MenuType::Standard),
            "separator" => Ok(MenuType::Separator),
            _ => Err(zbus::zvariant::Error::IncorrectType)
        }
    }
}

impl FromStr for ToggleType {
    type Err = zbus::zvariant::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "checkmark" => Ok(ToggleType::Checkmark),
            "radio" => Ok(ToggleType::Radio),
            _ => Err(zbus::zvariant::Error::IncorrectType)
        }
    }
}

impl FromStr for Disposition {
    type Err = zbus::zvariant::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "normal" => Ok(Disposition::Normal),
            "informative" => Ok(Disposition::Informative),
            "warning" => Ok(Disposition::Warning),
            "alert" => Ok(Disposition::Alert),
            _ => Err(zbus::zvariant::Error::IncorrectType)
        }
    }
}

impl TryFrom<MenuLayout> for TrayMenu {
    type Error = zbus::zvariant::Error;

    fn try_from(value: MenuLayout) -> Result<Self, Self::Error> {
        let mut submenus = vec![];
        for menu in &value.fields.submenus {
            let menu = SubMenu::try_from(menu)?;
            submenus.push(menu);
        }

        Ok(TrayMenu {
            id: value.id,
            submenus,
        })
    }
}

impl TryFrom<&OwnedValue> for SubMenu {
    type Error = zbus::zvariant::Error;

    fn try_from(value: &OwnedValue) -> Result<Self, Self::Error> {
        let structure = value.downcast_ref::<Structure>().expect("Expected a layout");
        let mut fields = structure.fields().iter();
        let mut menu = SubMenu::default();

        if let Some(Value::I32(id)) = fields.next() {
            menu.id = *id;
        }

        if let Some(Value::Dict(dict)) = fields.next() {
            menu.children_display = dict.get::<str, str>("children_display")?.map(str::to_string);
            // see: https://github.com/AyatanaIndicators/libdbusmenu/blob/4d03141aea4e2ad0f04ab73cf1d4f4bcc4a19f6c/libdbusmenu-glib/dbus-menu.xml#L75
            menu.label = dict.get::<str, str>("label")?
                .map(|label| label.replace("_", ""))
                .unwrap_or("".to_string());

            if let Some(enabled) = dict.get::<str, bool>("enabled")? {
                menu.enabled = *enabled
            }

            if let Some(visible) = dict.get::<str, bool>("visible")? {
                menu.visible = *visible
            }

            menu.icon_name = dict.get::<str, str>("icon-name")?.map(str::to_string);
            if let Some(disposition) = dict
                .get::<str, str>("shortcut")?
                .map(Disposition::from_str)
                .map(Result::ok)
                .flatten(){
                menu.disposition = disposition;
            }
            menu.toggle_state = dict.get::<str, bool>("toggle-state")?.map(ToOwned::to_owned);
            menu.toggle_type = dict
                .get::<str, str>("toggle-type")?
                .map(ToggleType::from_str)
                .map(Result::ok)
                .flatten();
            menu.menu_type = dict.get::<str, str>("type")?
                .map(MenuType::from_str)
                .map(Result::ok)
                .flatten()
                .unwrap_or(MenuType::Standard);
        };

        if let Some(Value::Array(array)) = fields.next() {
            let mut submenu = vec![];
            for value in array.iter() {
                let value = OwnedValue::from(value);
                let menu = SubMenu::try_from(&value)?;
                submenu.push(menu);
            }

            menu.submenu = submenu;
        }

        Ok(menu)
    }
}