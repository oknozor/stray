use std::pin::Pin;
use std::task::{Context, Poll};

use anyhow::anyhow;
pub use tokio;
use tokio::sync::mpsc::{channel, Sender};
use tokio::sync::mpsc::Receiver;
use tokio::task::JoinHandle;
use tokio_stream::{Stream, StreamExt};
pub use tokio_stream;
use zbus::{Connection, ConnectionBuilder};
use zbus::fdo::PropertiesProxy;
use zbus::names::InterfaceName;

use dbus::dbusmenu::DBusMenuProxy;
use dbus::notifier_item_proxy::StatusNotifierItemProxy;
use dbus::notifier_watcher_proxy::StatusNotifierWatcherProxy;
use dbus::notifier_watcher_service::Watcher;
use crate::dbus::dbusmenu::{MenuLayout};

use crate::menu::TrayMenu;
use crate::tray::{Message, StatusNotifierItem};

pub mod dbus;
pub mod tray;
pub mod menu;

pub struct SystemTray(Receiver<Message>);

impl SystemTray {
    pub async fn new() -> SystemTray {
        let (tx, rx) = channel(5);

        tokio::spawn(async {
            start_notifier_watcher(tx)
                .await
                .expect("Error occurred in notifier watcher task")
        });

        SystemTray(rx)
    }
}

// Wrap the receiver into a stream so we dont need to expose tokio receiver directly
impl Stream for SystemTray {
    type Item = Message;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        self.0.poll_recv(cx)
    }
}

async fn start_notifier_watcher(sender: Sender<Message>) -> anyhow::Result<()> {
    let watcher = Watcher::new(sender.clone());
    let done_listener = watcher.event.listen();
    let conn = ConnectionBuilder::session()?
        .name("org.kde.StatusNotifierWatcher")?
        .serve_at("/StatusNotifierWatcher", watcher)?
        .build()
        .await?;

    let status_notifier_watcher_listener = tokio::spawn(async { done_listener.wait() });
    let status_notifier_removed_handle = status_notifier_removed_handle(conn.clone());
    let status_notifier_host_handle = {
        tokio::spawn(async move {
            status_notifier_host_handle(sender)
                .await
                .expect("Host failure");
        })
    };

    let _ = tokio::join!(
        status_notifier_removed_handle,
        status_notifier_watcher_listener,
        status_notifier_host_handle,
    );

    Ok(())
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
            Ok(NotifierAddress {
                destination: format!(":{}", split[1]),
                path: "/StatusNotifierItem".to_string(),
            })
        } else {
            return Err(anyhow!("Service path {:?} was not understood", service));
        }
    }
}

// Listen for 'NameOwnerChanged' on DBus whenever a service is removed
// send 'UnregisterStatusNotifierItem' request to 'StatusNotifierWatcher' via dbus
fn status_notifier_removed_handle(connection: Connection) -> JoinHandle<()> {
    tokio::spawn(async move {
        let dbus_proxy = zbus::fdo::DBusProxy::new(&connection).await.unwrap();

        let mut changed = dbus_proxy
            .receive_name_owner_changed()
            .await
            .expect("fail to receive Dbus NameOwnerChanged");

        while let Some(signal) = changed.next().await {
            let args = signal.args().expect("Failed to get signal args");
            let old = args.old_owner();
            let new = args.new_owner();

            if old.is_some() && new.is_none() {
                let old_owner: String = old.as_ref().unwrap().to_string();
                let watcher_proxy = StatusNotifierWatcherProxy::new(&connection)
                    .await
                    .expect("Failed to open StatusNotifierWatcherProxy");

                watcher_proxy
                    .unregister_status_notifier_item(&old_owner)
                    .await
                    .expect("failed to unregister status notifier");
            }
        }
    })
}

// 1. Start StatusNotifierHost on DBus
// 2. Query already registered StatusNotifier, call GetAll to update the UI  and  listen for property changes via Dbus.PropertiesChanged
// 3. subscribe to StatusNotifierWatcher.RegisteredStatusNotifierItems
// 4. Whenever a new notifier is registered repeat steps 2
async fn status_notifier_host_handle(sender: Sender<Message>) -> anyhow::Result<()> {
    let connection = Connection::session().await?;
    let pid = std::process::id();
    let host = format!("org.freedesktop.StatusNotifierHost-{pid}-MyNotifierHost");
    connection.request_name(host.as_str()).await?;
    let status_notifier_proxy = StatusNotifierWatcherProxy::new(&connection).await?;
    status_notifier_proxy.register_status_notifier_host(&host).await?;

    let notifier_items: Vec<String> = status_notifier_proxy.registered_status_notifier_items().await?;

    // Start watching for all registered notifier items
    for service in notifier_items.iter() {
        let service = NotifierAddress::from_notifier_service(service);
        if let Ok(notifier_address) = service {
            let connection = connection.clone();
            let sender = sender.clone();
            watch_notifier_props(notifier_address, connection, sender).await?;
        }
    }

    // Listen for new notifier items
    let mut new_notifier = status_notifier_proxy.receive_status_notifier_item_registered().await?;

    while let Some(notifier) = new_notifier.next().await {
        let args = notifier.args()?;
        let service: &str = args.service();

        let service = NotifierAddress::from_notifier_service(service);
        if let Ok(notifier_address) = service {
            let connection = connection.clone();
            let sender = sender.clone();
            tokio::spawn(async move {
                watch_notifier_props(notifier_address, connection, sender)
                    .await
                    .expect("Could not watch for notifier item props");
            });
        }
    }

    Ok(())
}

// Listen for PropertiesChanged on DBus and send an update request on change
async fn watch_notifier_props(
    address_parts: NotifierAddress,
    connection: Connection,
    sender: Sender<Message>,
) -> anyhow::Result<()> {
    tokio::spawn(async move {
        // Connect to DBus.Properties
        let dbus_properties_proxy = zbus::fdo::PropertiesProxy::builder(&connection)
            .destination(address_parts.destination.as_str())?
            .path(address_parts.path.as_str())?
            .build()
            .await?;

        // call Properties.GetAll once and send an update to the UI
        fetch_properties_and_update(
            sender.clone(),
            &dbus_properties_proxy,
            address_parts.destination.clone(),
            connection.clone(),
        ).await?;

        // Connect to the notifier proxy to watch for properties change
        let notifier_item_proxy = StatusNotifierItemProxy::builder(&connection)
            .destination(address_parts.destination.as_str())?
            .path(address_parts.path.as_str())?
            .build()
            .await?;

        let mut props_changed = notifier_item_proxy.receive_all_signals().await?;

        // Whenever a property change query all props and update the UI
        while props_changed.next().await.is_some() {
            fetch_properties_and_update(
                sender.clone(),
                &dbus_properties_proxy,
                address_parts.destination.clone(),
                connection.clone(),
            )
                .await?;
        }

        Result::<(), anyhow::Error>::Ok(())
    });

    Ok(())
}

// Fetch Properties from DBus proxy and send an update to the UI channel
async fn fetch_properties_and_update(
    sender: Sender<Message>,
    dbus_properties_proxy: &PropertiesProxy<'_>,
    item_address: String,
    connection: Connection,
) -> anyhow::Result<()> {
    let interface = InterfaceName::from_static_str("org.kde.StatusNotifierItem")?;
    let props = dbus_properties_proxy.get_all(interface).await?;
    let item = StatusNotifierItem::try_from(props);

    // Only send item that maps correctly to our internal StatusNotifierItem representation
    if let Ok(item) = item {
        let menu = match &item.menu {
            None => None,
            Some(menu_address) => {
                let item_address = item_address.as_str();
                let dbus_menu_proxy = DBusMenuProxy::builder(&connection)
                    .destination(item_address)?
                    .path(menu_address.as_str())?
                    .build()
                    .await?;

                let menu: MenuLayout = dbus_menu_proxy.get_layout(0, 10, &[]).await.unwrap();
                Some(TrayMenu::try_from(menu)?)
            }
        };

        sender
            .send(Message::Update {
                id: item_address.to_string(),
                item,
                menu
            })
            .await?;
    }

    Ok(())
}
