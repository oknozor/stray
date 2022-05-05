#![doc = include_str ! ("../README.md")]

pub use tokio;
use zbus::names::InterfaceName;

use crate::dbus::dbusmenu_proxy::MenuLayout;
use crate::message::tray::StatusNotifierItem;
use dbus::notifier_watcher_service::DbusNotifierWatcher;

mod dbus;
mod notifier_host;
mod notifier_watcher;

pub mod error;
/// Messages sent and received by the [`SystemTray`]
pub mod message;

pub use message::NotifierItemMessage;
pub use notifier_watcher::StatusNotifierWatcher;
