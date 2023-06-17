use std::collections::HashSet;
use tokio::sync::broadcast;

use zbus::dbus_interface;
use zbus::Result;
use zbus::{MessageHeader, SignalContext};

use crate::NotifierItemMessage;

pub struct DbusNotifierWatcher {
    pub status_notifier_hosts: HashSet<String>,
    pub registered_status_notifier_items: HashSet<String>,
    pub protocol_version: i32,
    pub is_status_notifier_host_registered: bool,
    pub sender: broadcast::Sender<NotifierItemMessage>,
}

impl DbusNotifierWatcher {
    pub(crate) fn new(sender: broadcast::Sender<NotifierItemMessage>) -> Self {
        DbusNotifierWatcher {
            registered_status_notifier_items: HashSet::new(),
            protocol_version: 0,
            is_status_notifier_host_registered: false,
            status_notifier_hosts: HashSet::new(),
            sender,
        }
    }
}

impl DbusNotifierWatcher {
    pub async fn remove_notifier(&mut self, notifier_address: &str) -> Result<()> {
        let to_remove = self
            .registered_status_notifier_items
            .iter()
            .find(|item| item.contains(notifier_address))
            .cloned();

        if let Some(notifier) = to_remove {
            let removed = self.registered_status_notifier_items.remove(&notifier);
            if removed {
                self.sender
                    .send(NotifierItemMessage::Remove {
                        address: notifier_address.to_string(),
                    })
                    .expect("Failed to dispatch notifier item removed message");
            }
        }

        Ok(())
    }
}

#[allow(dead_code)]
#[dbus_interface(name = "org.kde.StatusNotifierWatcher")]
impl DbusNotifierWatcher {
    async fn register_status_notifier_host(
        &mut self,
        service: &str,
        #[zbus(signal_context)] ctxt: SignalContext<'_>,
    ) {
        tracing::info!("StatusNotifierHost registered: '{}'", service);
        self.status_notifier_hosts.insert(service.to_string());
        self.is_status_notifier_host_registered = true;
        self.is_status_notifier_host_registered_changed(&ctxt)
            .await
            .unwrap();
    }

    async fn register_status_notifier_item(
        &mut self,
        service: &str,
        #[zbus(header)] header: MessageHeader<'_>,
        #[zbus(signal_context)] ctxt: SignalContext<'_>,
    ) {
        let address = header
            .sender()
            .expect("Failed to get message sender in header")
            .map(|name| name.to_string())
            .expect("Failed to get unique name for notifier");

        let notifier_item = format!("{}{}", address, service);

        self.registered_status_notifier_items
            .insert(notifier_item.clone());

        tracing::info!("StatusNotifierItem registered: '{}'", notifier_item);

        Self::status_notifier_item_registered(&ctxt, &notifier_item)
            .await
            .unwrap();
    }

    async fn unregister_status_notifier_item(&mut self, service: &str) {
        self.remove_notifier(service)
            .await
            .expect("Failed to unregister StatusNotifierItem")
    }

    #[dbus_interface(signal)]
    async fn status_notifier_host_registered(ctxt: &SignalContext<'_>) -> Result<()>;

    #[dbus_interface(signal)]
    async fn status_notifier_host_unregistered(ctxt: &SignalContext<'_>) -> Result<()>;

    #[dbus_interface(signal)]
    async fn status_notifier_item_registered(ctxt: &SignalContext<'_>, service: &str)
        -> Result<()>;

    #[dbus_interface(signal)]
    async fn status_notifier_item_unregistered(
        ctxt: &SignalContext<'_>,
        service: &str,
    ) -> Result<()>;

    #[dbus_interface(property)]
    async fn is_status_notifier_host_registered(&self) -> bool {
        self.is_status_notifier_host_registered
    }

    #[dbus_interface(property)]
    async fn protocol_version(&self) -> i32 {
        self.protocol_version
    }

    #[dbus_interface(property)]
    fn registered_status_notifier_items(&self) -> Vec<String> {
        self.registered_status_notifier_items
            .iter()
            .cloned()
            .collect()
    }
}
