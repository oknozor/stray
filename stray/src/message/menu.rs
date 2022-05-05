use serde::Serialize;
use std::str;
use std::str::FromStr;

use zbus::zvariant::{OwnedValue, Structure, Value};

use crate::dbus::dbusmenu_proxy::MenuLayout;

/// A menu that should be displayed when clicking corresponding tray icon
#[derive(Debug, Serialize, Clone)]
pub struct TrayMenu {
    /// The unique identifier of the menu
    pub id: u32,
    /// A recursive list of submenus
    pub submenus: Vec<MenuItem>,
}

/// Represent an entry in a menu as described in [com.canonical.dbusmenu](https://github.com/AyatanaIndicators/libdbusmenu/blob/4d03141aea4e2ad0f04ab73cf1d4f4bcc4a19f6c/libdbusmenu-glib/dbus-menu.xml#L75)
/// This implementation currently support a sub section of the spec, if you feel something is missing don't hesitate to submit an issue.
#[derive(Debug, Serialize, Clone)]
pub struct MenuItem {
    /// Unique numeric id
    pub id: i32,
    /// If the menu item has children this property should be set to "submenu"
    pub children_display: Option<String>,
    /// Text of the item,
    pub label: String,
    /// Whether the item can be activated or not.
    pub enabled: bool,
    /// True if the item is visible in the menu.
    pub visible: bool,
    /// Icon name of the item, following the freedesktop.org icon spec.
    pub icon_name: Option<String>,
    /// Describe the current state of a "togglable" item. Can be one of:
    ///   - Some(true): on
    ///   - Some(false): off
    ///   - None: indeterminate
    pub toggle_state: ToggleState,
    /// How the menuitem feels the information it's displaying to the
    /// user should be presented.
    pub toggle_type: ToggleType,
    /// Either a standard menu item or a separator [`MenuType`]
    pub menu_type: MenuType,
    /// How the menuitem feels the information it's displaying to the user should be presented.
    pub disposition: Disposition,
    /// A submenu for this item, typically this would ve revealed to the user by hovering the current item
    pub submenu: Vec<MenuItem>,
}

impl Default for MenuItem {
    fn default() -> Self {
        Self {
            id: 0,
            children_display: None,
            label: "".to_string(),
            enabled: true,
            visible: true,
            icon_name: None,
            toggle_state: ToggleState::Indeterminate,
            toggle_type: ToggleType::CannotBeToggled,
            menu_type: MenuType::Standard,
            disposition: Disposition::Normal,
            submenu: vec![],
        }
    }
}

/// How the menuitem feels the information it's displaying to the
/// user should be presented.
#[derive(Debug, Serialize, Copy, Clone, Eq, PartialEq)]
pub enum ToggleType {
    /// Item is an independent togglable item
    Checkmark,
    /// Item is part of a group where only one item can be
    /// toggled at a time
    Radio,
    /// Item cannot be toggled
    CannotBeToggled,
}

/// Either a standard menu item or a separator
#[derive(Debug, Serialize, Copy, Clone, Eq, PartialEq)]
pub enum MenuType {
    ///  a separator
    Separator,
    /// an item which can be clicked to trigger an action or show another menu
    Standard,
}

/// How the menuitem feels the information it's displaying to the
/// user should be presented.
#[derive(Debug, Serialize, Copy, Clone, Eq, PartialEq)]
pub enum Disposition {
    /// a standard menu item
    Normal,
    /// providing additional information to the user
    Informative,
    ///  looking at potentially harmful results
    Warning,
    /// something bad could potentially happen
    Alert,
}

/// Describe the current state of a "togglable" item.
#[derive(Debug, Serialize, Copy, Clone, Eq, PartialEq)]
pub enum ToggleState {
    /// This item is toggled
    On,
    /// Item is not toggled
    Off,
    /// Item is not toggalble
    Indeterminate,
}

impl FromStr for MenuType {
    type Err = zbus::zvariant::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "standard" => Ok(MenuType::Standard),
            "separator" => Ok(MenuType::Separator),
            _ => Err(zbus::zvariant::Error::IncorrectType),
        }
    }
}

impl FromStr for ToggleType {
    type Err = zbus::zvariant::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "checkmark" => Ok(ToggleType::Checkmark),
            "radio" => Ok(ToggleType::Radio),
            _ => Err(zbus::zvariant::Error::IncorrectType),
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
            _ => Err(zbus::zvariant::Error::IncorrectType),
        }
    }
}

impl From<bool> for ToggleState {
    fn from(value: bool) -> Self {
        if value {
            ToggleState::On
        } else {
            ToggleState::Indeterminate
        }
    }
}

impl TryFrom<MenuLayout> for TrayMenu {
    type Error = zbus::zvariant::Error;

    fn try_from(value: MenuLayout) -> Result<Self, Self::Error> {
        let mut submenus = vec![];
        for menu in &value.fields.submenus {
            let menu = MenuItem::try_from(menu)?;
            submenus.push(menu);
        }

        Ok(TrayMenu {
            id: value.id,
            submenus,
        })
    }
}

impl TryFrom<&OwnedValue> for MenuItem {
    type Error = zbus::zvariant::Error;

    fn try_from(value: &OwnedValue) -> Result<Self, Self::Error> {
        let structure = value
            .downcast_ref::<Structure>()
            .expect("Expected a layout");

        let mut fields = structure.fields().iter();
        let mut menu = MenuItem::default();

        if let Some(Value::I32(id)) = fields.next() {
            menu.id = *id;
        }

        if let Some(Value::Dict(dict)) = fields.next() {
            menu.children_display = dict
                .get::<str, str>("children_display")?
                .map(str::to_string);

            // see: https://github.com/AyatanaIndicators/libdbusmenu/blob/4d03141aea4e2ad0f04ab73cf1d4f4bcc4a19f6c/libdbusmenu-glib/dbus-menu.xml#L75
            menu.label = dict
                .get::<str, str>("label")?
                .map(|label| label.replace('_', ""))
                .unwrap_or_default();

            if let Some(enabled) = dict.get::<str, bool>("enabled")? {
                menu.enabled = *enabled
            }

            if let Some(visible) = dict.get::<str, bool>("visible")? {
                menu.visible = *visible;
            }

            menu.icon_name = dict.get::<str, str>("icon-name")?.map(str::to_string);

            if let Some(disposition) = dict
                .get::<str, str>("disposition")
                .ok()
                .flatten()
                .map(Disposition::from_str)
                .and_then(Result::ok)
            {
                menu.disposition = disposition;
            }

            menu.toggle_state = dict
                .get::<str, bool>("toggle-state")
                .ok()
                .flatten()
                .map(|value| ToggleState::from(*value))
                .unwrap_or(ToggleState::Indeterminate);

            menu.toggle_type = dict
                .get::<str, str>("toggle-type")
                .ok()
                .flatten()
                .map(ToggleType::from_str)
                .and_then(Result::ok)
                .unwrap_or(ToggleType::CannotBeToggled);

            menu.menu_type = dict
                .get::<str, str>("type")
                .ok()
                .flatten()
                .map(MenuType::from_str)
                .and_then(Result::ok)
                .unwrap_or(MenuType::Standard);
        };

        if let Some(Value::Array(array)) = fields.next() {
            let mut submenu = vec![];
            for value in array.iter() {
                let value = OwnedValue::from(value);
                let menu = MenuItem::try_from(&value)?;
                submenu.push(menu);
            }

            menu.submenu = submenu;
        }

        Ok(menu)
    }
}
