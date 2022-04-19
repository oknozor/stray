use tokio::sync::mpsc::{channel, Sender};

use tokio::task::JoinHandle;
use tokio_stream::StreamExt;
use zbus::fdo::PropertiesProxy;
use zbus::names::InterfaceName;
use zbus::{Connection, ConnectionBuilder};

use dbus::notifier_item_proxy::StatusNotifierItemProxy;
use dbus::notifier_watcher_proxy::StatusNotifierWatcherProxy;
use dbus::notifier_watcher_service::Watcher;

use crate::tray::{Message, TrayIconMessage, TrayUpdater};

mod dbus;
pub mod tray;

#[tokio::main]
pub async fn main() -> anyhow::Result<()> {
    let (tx, rx) = channel(3);
    let mut tray_updater = TrayUpdater::new(rx);
    let watcher = Watcher::new(tx.clone());
    let done_listener = watcher.event.listen();
    let conn = ConnectionBuilder::session()?
        .name("org.kde.StatusNotifierWatcher")?
        .serve_at("/StatusNotifierWatcher", watcher)?
        .build()
        .await?;
    let status_notifier_watcher_listener = tokio::spawn(async { done_listener.wait() });
    let status_notifier_removed_handle = status_notifier_removed_handle(conn.clone());
    let status_notifier_host_handle = {
        tokio::spawn(async {
            status_notifier_host_handle(tx).await.expect("Host failure");
        })
    };
    let tray_icon_updater_handle = tokio::spawn(async move { tray_updater.run().await });
    let _ = tokio::join!(
        status_notifier_removed_handle,
        status_notifier_watcher_listener,
        status_notifier_host_handle,
        tray_icon_updater_handle
    );
    Ok(())
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
// 2. Query already registered StatusNotifier, call GetAll to update the UI  and  listen for property changes via listen for changes via Dbus.PropertiesChanged
// 3. subscribe to StatusNotifierWatcher.RegisteredStatusNotifierItems
// 4. Whenever a new notifier is registered repeat steps 2
async fn status_notifier_host_handle(sender: Sender<Message>) -> anyhow::Result<()> {
    let connection = Connection::session().await?;
    let pid = std::process::id();
    let host = format!("org.freedesktop.StatusNotifierHost-{pid}-MyNotifierHost");
    connection.request_name(host.as_str()).await?;
    let proxy = StatusNotifierWatcherProxy::new(&connection).await?;
    proxy.register_status_notifier_host(&host).await?;

    let notifier_items: Vec<String> = proxy.registered_status_notifier_items().await?;

    for service in notifier_items.iter() {
        let connection = connection.clone();
        let (destination, path) = service
            .split_once('/')
            .expect("service should be of the form : {{address}}/{{/path/to/service}}");
        let path = format!("/{}", path);
        let destination = destination.to_string();
        let path = path.to_string();
        let connection = connection.clone();

        {
            let sender = sender.clone();
            tokio::spawn(async move {
                watch_notifier_props(destination, path, connection, sender)
                    .await
                    .expect("Could not watch for notifier item props");
            });
        }
    }

    let mut new_notifier = proxy.receive_status_notifier_item_registered().await?;

    while let Some(notifier) = new_notifier.next().await {
        let args = notifier.args()?;
        let service: &str = args.service();
        let (destination, path) = service
            .split_once('/')
            .expect("service should be of the form : {{address}}/{{/path/to/service}}");
        let path = format!("/{}", path);
        let destination = destination.to_string();
        let path = path.to_string();
        let connection = connection.clone();
        let tray = sender.clone();

        tokio::spawn(async move {
            watch_notifier_props(destination, path, connection, tray)
                .await
                .expect("Could not watch for notifier item props");
        });
    }

    Ok(())
}

// Listen for PropertiesChanged on DBus and send an update request on change
async fn watch_notifier_props(
    destination: String,
    path: String,
    connection: Connection,
    sender: Sender<Message>,
) -> anyhow::Result<()> {
    let dbus_properties_proxy = zbus::fdo::PropertiesProxy::builder(&connection)
        .destination(destination.as_str())?
        .path(path.as_str())?
        .build()
        .await?;

    fetch_properties_and_update(&destination, sender.clone(), &dbus_properties_proxy).await?;

    let notifier_item_proxy = StatusNotifierItemProxy::builder(&connection)
        .destination(destination.as_str())?
        .path(path.as_str())?
        .build()
        .await?;

    let mut props_changed = notifier_item_proxy.receive_all_signals().await?;

    while props_changed.next().await.is_some() {
        fetch_properties_and_update(&destination, sender.clone(), &dbus_properties_proxy).await?;
    }

    Ok(())
}

async fn fetch_properties_and_update(
    destination: &str,
    sender: Sender<Message>,
    dbus_properties_proxy: &PropertiesProxy<'_>,
) -> anyhow::Result<()> {
    let interface = InterfaceName::from_static_str("org.kde.StatusNotifierItem")?;
    let props = dbus_properties_proxy.get_all(interface).await?;
    let icon = TrayIconMessage::try_from(props)?;
    sender
        .send(Message::Update {
            address: destination.to_string(),
            icon,
        })
        .await?;

    Ok(())
}
