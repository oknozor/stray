use crate::message::menu::TrayMenu;
use crate::message::tray::StatusNotifierItem;
use serde::Serialize;

/// Implementation of [com.canonical.dbusmenu](https://github.com/AyatanaIndicators/libdbusmenu/blob/4d03141aea4e2ad0f04ab73cf1d4f4bcc4a19f6c/libdbusmenu-glib/dbus-menu.xml#L75)
pub mod menu;
/// Implementation of [StatusNotifierItem](https://freedesktop.org/wiki/Specifications/StatusNotifierItem)
pub mod tray;

/// Messages send via by [`crate::SystemTray`]
#[derive(Debug, Serialize, Clone)]
pub enum NotifierItemMessage {
    /// Notify the state of an item along with its menu
    Update {
        /// The address of the NotifierItem on dbus, this will be required
        /// to request the activation of a manu entry via [`NotifierItemCommand::MenuItemClicked`]
        /// and remove the item when it is closed by the user.
        address: String,
        /// the status [`StatusNotifierItem`] and its metadata, to build a system tray ui
        /// the minimal would be to display it's icon and use it's menu address to send menu activation
        /// requests.
        item: Box<StatusNotifierItem>,
        /// The menu layout of the item.
        menu: Option<TrayMenu>,
    },
    /// A [`StatusNotifierItem`] has been removed from the tray
    Remove {
        /// The dbus address of the item, it serves as an unique identifier.
        address: String,
    },
}

/// Command to send to a [`StatusNotifierItem`]
#[derive(Debug)]
pub enum NotifierItemCommand {
    /// Request activation of a menu item
    MenuItemClicked {
        /// Unique identifier of the item, see: [`crate::message::menu::MenuItem`]
        submenu_id: i32,
        /// DBus path of the menu item, (see: [`StatusNotifierItem`])
        menu_path: String,
        /// Dbus address of the [`StatusNotifierItem`]
        notifier_address: String,
    },
}
