//! # Stray
//!
//! Stray is a minimal [SystemNotifierWatcher](https://www.freedesktop.org/wiki/Specifications/StatusNotifierItem/StatusNotifierWatcher/)
//! implementation which goal is to provide a minimalistic API to access tray icons and menu.
//!
//! ## Examples
//!
//! ### Start the system tray and listen for changes
//! ```rust, ignore
//! use stray::{SystemTray};
//! use tokio_stream::StreamExt;
//! use stray::message::NotifierItemMessage;
//! use stray::message::NotifierItemCommand;
//!
//! #[tokio::main]
//! async fn main() {
//!
//!     // A mpsc channel to send menu activation requests later
//!     let (ui_tx, ui_rx) = tokio::sync::mpsc::channel(32);
//!     let mut tray = SystemTray::new(ui_rx).await;
//!
//!     while let Some(message) = tray.next().await {
//!         match message {
//!             NotifierItemMessage::Update { address: id, item, menu } => {
//!                 println!("NotifierItem updated :
//!                     id   = {id},
//!                     item = {item:?},
//!                     menu = {menu:?}"
//!                 )
//!             }
//!             NotifierItemMessage::Remove { address: id } => {
//!                 println!("NotifierItem removed : id = {id}");
//!             }
//!         }
//!     }
//! }
//! ```
//!
//! ### Send menu activation request to the system tray
//!
//! ```rust,  ignore
//!  # fn main() {
//!  # let (ui_tx, ui_rx) = tokio::sync::mpsc::channel(32);
//!  // Assuming we stored our menu items in some UI state we can send menu item activation request:
//!  # use stray::message::NotifierItemCommand;
//!  ui_tx.clone().try_send(NotifierItemCommand::MenuItemClicked {
//!     // The submenu to activate
//!     submenu_id: 32,
//!     // dbus menu path, available in the `StatusNotifierItem`
//!     menu_path: "/org/ayatana/NotificationItem/Element1/Menu".to_string(),
//!     // the notifier address we previously got from `NotifierItemMessage::Update`
//!     notifier_address: ":1.2161".to_string(),
//!  }).unwrap();
//! # }
//! ```
//!
use std::pin::Pin;
use std::task::{Context, Poll};

use anyhow::anyhow;
pub use tokio;
use tokio::sync::mpsc::channel;
use tokio::sync::mpsc::Receiver;
pub use tokio_stream;
use tokio_stream::Stream;
use zbus::names::InterfaceName;

use crate::dbus::dbusmenu_proxy::MenuLayout;
use crate::message::{tray::StatusNotifierItem, NotifierItemCommand, NotifierItemMessage};
use dbus::notifier_watcher_service::Watcher;

mod dbus;

/// Messages sent and received by the [`SystemTray`]
pub mod message;

/// Wrap the implementation of [org.freedesktop.StatusNotifierWatcher](https://www.freedesktop.org/wiki/Specifications/StatusNotifierItem/StatusNotifierWatcher/)
/// and [org.freedesktop.StatusNotifierHost](https://www.freedesktop.org/wiki/Specifications/StatusNotifierItem/StatusNotifierHost/).
pub struct SystemTray(Receiver<NotifierItemMessage>);

impl SystemTray {
    /// Creates a new system stray and register a [StatusNotifierWatcher](https://www.freedesktop.org/wiki/Specifications/StatusNotifierItem/StatusNotifierWatcher/) and [StatusNotifierHost](https://www.freedesktop.org/wiki/Specifications/StatusNotifierItem/StatusNotifierHost/) on dbus.
    /// Once created you can receive [`StatusNotifierItem`]. Once created you can start to poll message
    /// using the [`Stream`] implementation.
    pub async fn new(ui_rx: Receiver<NotifierItemCommand>) -> SystemTray {
        let (tx, rx) = channel(5);
        let tx_clone = tx.clone();

        tokio::spawn(async {
            dbus::start_notifier_watcher(tx_clone, ui_rx)
                .await
                .expect("Error occurred in notifier watcher task")
        });

        SystemTray(rx)
    }
}

// Wrap the receiver into a stream so we dont need to expose tokio receiver directly
impl Stream for SystemTray {
    type Item = NotifierItemMessage;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.0.poll_recv(cx)
    }
}

// A helper to convert RegisterStatusNotifier calls to
// StatusNotifier address parts
#[derive(Debug)]
struct NotifierAddress {
    // Notifier destination on the bus, ex: ":1.522"
    destination: String,
    // The notifier object path, ex: "/org/ayatana/NotificationItem/Element1"
    path: String,
}

impl NotifierAddress {
    fn from_notifier_service(service: &str) -> anyhow::Result<Self> {
        if let Some((destination, path)) = service.split_once('/') {
            Ok(NotifierAddress {
                destination: destination.to_string(),
                path: format!("/{}", path),
            })
        } else if service.contains(':') {
            let split = service.split(':').collect::<Vec<&str>>();
            // Some StatusNotifierItems will not return an object path, in that case we fallback
            // to the default path.
            Ok(NotifierAddress {
                destination: format!(":{}", split[1]),
                path: "/StatusNotifierItem".to_string(),
            })
        } else {
            return Err(anyhow!("Service path {:?} was not understood", service));
        }
    }
}
