use crate::{
    InterfaceName, MenuLayout, NotifierAddress, NotifierItemMessage, StatusNotifierItem, Watcher,
};
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::task::JoinHandle;
use zbus::{Connection, ConnectionBuilder};

pub(super) mod dbusmenu_proxy;
pub(super) mod notifier_item_proxy;
pub(super) mod notifier_watcher_proxy;
pub(super) mod notifier_watcher_service;

use crate::message::menu::TrayMenu;
use crate::message::NotifierItemCommand;
use dbusmenu_proxy::DBusMenuProxy;
use notifier_item_proxy::StatusNotifierItemProxy;
use notifier_watcher_proxy::StatusNotifierWatcherProxy;
use tokio_stream::StreamExt;
use zbus::fdo::PropertiesProxy;

pub async fn start_notifier_watcher(
    sender: Sender<NotifierItemMessage>,
    mut ui_rx: Receiver<NotifierItemCommand>,
) -> anyhow::Result<()> {
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

    let handle_ui_event = tokio::spawn(async move {
        while let Some(event) = ui_rx.recv().await {
            match event {
                NotifierItemCommand::MenuItemClicked {
                    submenu_id: id,
                    menu_path,
                    notifier_address,
                } => {
                    let dbus_menu_proxy = DBusMenuProxy::builder(&conn)
                        .destination(notifier_address)
                        .unwrap()
                        .path(menu_path)
                        .unwrap()
                        .build()
                        .await
                        .unwrap();

                    dbus_menu_proxy
                        .event(
                            id,
                            "clicked",
                            &zbus::zvariant::Value::I32(32),
                            chrono::offset::Local::now().timestamp_subsec_micros(),
                        )
                        .await
                        .unwrap();
                }
            }
        }
    });

    let _ = tokio::join!(
        status_notifier_removed_handle,
        status_notifier_watcher_listener,
        status_notifier_host_handle,
        handle_ui_event
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
// 2. Query already registered StatusNotifier, call GetAll to update the UI  and  listen for property changes via Dbus.PropertiesChanged
// 3. subscribe to StatusNotifierWatcher.RegisteredStatusNotifierItems
// 4. Whenever a new notifier is registered repeat steps 2
async fn status_notifier_host_handle(sender: Sender<NotifierItemMessage>) -> anyhow::Result<()> {
    let connection = Connection::session().await?;
    let pid = std::process::id();
    let host = format!("org.freedesktop.StatusNotifierHost-{pid}-MyNotifierHost");
    connection.request_name(host.as_str()).await?;
    let status_notifier_proxy = StatusNotifierWatcherProxy::new(&connection).await?;
    status_notifier_proxy
        .register_status_notifier_host(&host)
        .await?;

    let notifier_items: Vec<String> = status_notifier_proxy
        .registered_status_notifier_items()
        .await?;

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
    let mut new_notifier = status_notifier_proxy
        .receive_status_notifier_item_registered()
        .await?;

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
    sender: Sender<NotifierItemMessage>,
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
        )
        .await?;

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
    sender: Sender<NotifierItemMessage>,
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
            .send(NotifierItemMessage::Update {
                address: item_address.to_string(),
                item,
                menu,
            })
            .await?;
    }

    Ok(())
}
