#![doc = include_str ! ("../README.md")]

pub use tokio;
use zbus::names::InterfaceName;

use crate::dbus::dbusmenu_proxy::MenuLayout;
use crate::message::{tray::StatusNotifierItem};
use dbus::notifier_watcher_service::DbusNotifierWatcher;

mod dbus;
mod notifier_watcher;
mod notifier_host;

/// Messages sent and received by the [`SystemTray`]
pub mod message;
pub mod error;

pub use notifier_watcher::StatusNotifierWatcher;
pub use message::NotifierItemMessage;


