use crate::dbus::dbusmenu_proxy::DBusMenuProxy;
use crate::dbus::notifier_item_proxy::StatusNotifierItemProxy;
use crate::dbus::notifier_watcher_proxy::StatusNotifierWatcherProxy;
use crate::error::Result;
use crate::message::menu::TrayMenu;
use crate::message::NotifierItemCommand;
use crate::notifier_watcher::notifier_address::NotifierAddress;
use crate::{
    DbusNotifierWatcher, InterfaceName, MenuLayout, NotifierItemMessage, StatusNotifierItem,
};
use tokio::sync::{broadcast, mpsc};
use tokio_stream::StreamExt;
use zbus::fdo::PropertiesProxy;
use zbus::{Connection, ConnectionBuilder};

pub(crate) mod notifier_address;

/// Wrap the implementation of [org.freedesktop.StatusNotifierWatcher](https://www.freedesktop.org/wiki/Specifications/StatusNotifierItem/StatusNotifierWatcher/)
/// and [org.freedesktop.StatusNotifierHost](https://www.freedesktop.org/wiki/Specifications/StatusNotifierItem/StatusNotifierHost/).
#[derive(Debug)]
pub struct StatusNotifierWatcher {
    pub(crate) tx: broadcast::Sender<NotifierItemMessage>,
    _rx: broadcast::Receiver<NotifierItemMessage>,
}

impl StatusNotifierWatcher {
    /// Creates a new system stray and register a [StatusNotifierWatcher](https://www.freedesktop.org/wiki/Specifications/StatusNotifierItem/StatusNotifierWatcher/) and [StatusNotifierHost](https://www.freedesktop.org/wiki/Specifications/StatusNotifierItem/StatusNotifierHost/) on dbus.
    /// Once created you can receive [`StatusNotifierItem`]. Once created you can start to poll message
    /// using the [`Stream`] implementation.
    pub async fn new(cmd_rx: mpsc::Receiver<NotifierItemCommand>) -> Result<StatusNotifierWatcher> {
        let (tx, rx) = broadcast::channel(5);

        {
            tracing::info!("Starting notifier watcher");
            let tx = tx.clone();

            tokio::spawn(async move {
                start_notifier_watcher(tx)
                    .await
                    .expect("Unexpected StatusNotifierError");
            });
        }

        tokio::spawn(async move {
            dispatch_ui_command(cmd_rx)
                .await
                .expect("Unexpected error while dispatching UI command");
        });

        Ok(StatusNotifierWatcher { tx, _rx: rx })
    }
}

// Forward UI command to the Dbus menu proxy
async fn dispatch_ui_command(mut cmd_rx: mpsc::Receiver<NotifierItemCommand>) -> Result<()> {
    let connection = Connection::session().await?;

    while let Some(command) = cmd_rx.recv().await {
        match command {
            NotifierItemCommand::MenuItemClicked {
                submenu_id: id,
                menu_path,
                notifier_address,
            } => {
                let dbus_menu_proxy = DBusMenuProxy::builder(&connection)
                    .destination(notifier_address)
                    .unwrap()
                    .path(menu_path)
                    .unwrap()
                    .build()
                    .await?;

                dbus_menu_proxy
                    .event(
                        id,
                        "clicked",
                        &zbus::zvariant::Value::I32(32),
                        chrono::offset::Local::now().timestamp_subsec_micros(),
                    )
                    .await?;
            }
        }
    }

    Ok(())
}

async fn start_notifier_watcher(sender: broadcast::Sender<NotifierItemMessage>) -> Result<()> {
    let watcher = DbusNotifierWatcher::new(sender.clone());

    let connection = ConnectionBuilder::session()?
        .name("org.kde.StatusNotifierWatcher")?
        .serve_at("/StatusNotifierWatcher", watcher)?
        .build()
        .await?;

    let status_notifier_removed = {
        let connection = connection.clone();
        tokio::spawn(async move {
            status_notifier_removed_handle(connection).await?;
            Result::<()>::Ok(())
        })
    };

    let status_notifier =
        tokio::spawn(async move { status_notifier_handle(connection, sender).await.unwrap() });

    tokio::spawn(async move {
        let (r1, r2) = tokio::join!(status_notifier, status_notifier_removed,);
        if let Err(err) = r1 {
            tracing::error!("Status notifier error: {err:?}")
        }

        if let Err(err) = r2 {
            tracing::error!("Status notifier removed error: {err:?}")
        }
    });

    Ok(())
}

// Listen for 'NameOwnerChanged' on DBus whenever a service is removed
// send 'UnregisterStatusNotifierItem' request to 'StatusNotifierWatcher' via dbus
async fn status_notifier_removed_handle(connection: Connection) -> Result<()> {
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

            if let Err(err) = watcher_proxy
                .unregister_status_notifier_item(&old_owner)
                .await
            {
                tracing::error!("Failed to unregister status notifier: {err:?}")
            }
        }
    }

    Ok(())
}

// 1. Start StatusNotifierHost on DBus
// 2. Query already registered StatusNotifier, call GetAll to update the UI  and  listen for property changes via Dbus.PropertiesChanged
// 3. subscribe to StatusNotifierWatcher.RegisteredStatusNotifierItems
// 4. Whenever a new notifier is registered repeat steps 2
// FIXME : Move this to HOST
async fn status_notifier_handle(
    connection: Connection,
    sender: broadcast::Sender<NotifierItemMessage>,
) -> Result<()> {
    let status_notifier_proxy = StatusNotifierWatcherProxy::new(&connection).await?;

    let notifier_items: Vec<String> = status_notifier_proxy
        .registered_status_notifier_items()
        .await?;

    tracing::info!("Got {} notifier items", notifier_items.len());

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
        tracing::info!(
            "StatusNotifierItemRegistered signal received service={}",
            service
        );

        let service = NotifierAddress::from_notifier_service(service);
        if let Ok(notifier_address) = service {
            let connection = connection.clone();
            let sender = sender.clone();
            tokio::spawn(async move {
                watch_notifier_props(notifier_address, connection, sender).await?;
                Result::<()>::Ok(())
            });
        }
    }

    Ok(())
}

// Listen for PropertiesChanged on DBus and send an update request on change
async fn watch_notifier_props(
    address_parts: NotifierAddress,
    connection: Connection,
    sender: broadcast::Sender<NotifierItemMessage>,
) -> Result<()> {
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

        Result::<()>::Ok(())
    });

    Ok(())
}

// Fetch Properties from DBus proxy and send an update to the UI channel
async fn fetch_properties_and_update(
    sender: broadcast::Sender<NotifierItemMessage>,
    dbus_properties_proxy: &PropertiesProxy<'_>,
    item_address: String,
    connection: Connection,
) -> Result<()> {
    let interface = InterfaceName::from_static_str("org.kde.StatusNotifierItem")?;
    let props = dbus_properties_proxy.get_all(interface).await?;
    let item = StatusNotifierItem::try_from(props);

    // Only send item that maps correctly to our internal StatusNotifierItem representation
    if let Ok(item) = item {
        let menu = match &item.menu {
            None => None,
            Some(menu_address) => watch_menu(
                item_address.clone(),
                item.clone(),
                connection.clone(),
                menu_address.clone(),
                sender.clone(),
            )
            .await
            .ok(),
        };

        tracing::info!("StatusNotifierItem updated, dbus-address={item_address}");

        sender
            .send(NotifierItemMessage::Update {
                address: item_address.to_string(),
                item: Box::new(item),
                menu,
            })
            .expect("Failed to dispatch NotifierItemMessage");
    }

    Ok(())
}

async fn watch_menu(
    item_address: String,
    item: StatusNotifierItem,
    connection: Connection,
    menu_address: String,
    sender: broadcast::Sender<NotifierItemMessage>,
) -> Result<TrayMenu> {
    let dbus_menu_proxy = DBusMenuProxy::builder(&connection)
        .destination(item_address.as_str())?
        .path(menu_address.as_str())?
        .build()
        .await?;

    let menu: MenuLayout = dbus_menu_proxy.get_layout(0, 10, &[]).await.unwrap();

    tokio::spawn(async move {
        let dbus_menu_proxy = DBusMenuProxy::builder(&connection)
            .destination(item_address.as_str())?
            .path(menu_address.as_str())?
            .build()
            .await?;

        let mut props_changed = dbus_menu_proxy.receive_all_signals().await?;

        while props_changed.next().await.is_some() {
            let menu: MenuLayout = dbus_menu_proxy.get_layout(0, 10, &[]).await.unwrap();
            let menu = TrayMenu::try_from(menu).ok();
            sender.send(NotifierItemMessage::Update {
                address: item_address.to_string(),
                item: Box::new(item.clone()),
                menu,
            })?;
        }
        anyhow::Result::<(), anyhow::Error>::Ok(())
    });

    TrayMenu::try_from(menu).map_err(Into::into)
}
