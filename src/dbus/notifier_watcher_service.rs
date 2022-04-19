use std::collections::HashSet;

use event_listener::Event;
use tokio::sync::mpsc::Sender;
use zbus::dbus_interface;
use zbus::Result;
use zbus::{MessageHeader, SignalContext};

use crate::Message;

pub struct Watcher {
    pub status_notifier_hosts: HashSet<String>,
    pub registered_status_notifier_items: HashSet<String>,
    pub protocol_version: i32,
    pub is_status_notifier_host_registered: bool,
    pub event: Event,
    pub sender: Sender<Message>,
}

impl Watcher {
    pub(crate) fn new(sender: Sender<Message>) -> Self {
        Watcher {
            registered_status_notifier_items: HashSet::new(),
            protocol_version: 0,
            event: event_listener::Event::new(),
            is_status_notifier_host_registered: false,
            status_notifier_hosts: HashSet::new(),
            sender,
        }
    }
}

impl Watcher {
    pub async fn remove_notifier(&mut self, notifier_address: &str) {
        let to_remove = self
            .registered_status_notifier_items
            .iter()
            .find(|item| item.contains(notifier_address))
            .cloned();

        if let Some(notifier) = to_remove {
            let removed = self.registered_status_notifier_items.remove(&notifier);
            if removed {
                self.sender
                    .send(Message::Remove { address: notifier_address.to_string() })
                    .await
                    .unwrap();
            }
        }
    }
}

#[dbus_interface(name = "org.kde.StatusNotifierWatcher")]
impl Watcher {
    async fn register_status_notifier_host(
        &mut self,
        service: &str,
        #[zbus(signal_context)] ctxt: SignalContext<'_>,
    ) {
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

        Self::status_notifier_item_registered(&ctxt, &notifier_item)
            .await
            .unwrap();
    }

    async fn unregister_status_notifier_item(&mut self, service: &str) {
        self.remove_notifier(service).await;
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
    ) -> zbus::Result<()>;

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
