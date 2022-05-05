use crate::dbus::notifier_watcher_proxy::StatusNotifierWatcherProxy;
use crate::error::{Result, StatusNotifierWatcherError};
use crate::{NotifierItemMessage, StatusNotifierWatcher};
use tokio::sync::broadcast;
use zbus::{Connection, ConnectionBuilder};

pub struct NotifierHost {
    wellknown_name: String,
    rx: broadcast::Receiver<NotifierItemMessage>,
    conn: Connection,
}

impl StatusNotifierWatcher {
    pub async fn create_notifier_host(&self, unique_id: &str) -> Result<NotifierHost> {
        let pid = std::process::id();
        let id = &unique_id;
        let wellknown_name = format!("org.freedesktop.StatusNotifierHost-{pid}-{id}");

        let conn = ConnectionBuilder::session()?
            .name(wellknown_name.as_str())?
            .build()
            .await?;

        let status_notifier_proxy = StatusNotifierWatcherProxy::new(&conn).await?;

        status_notifier_proxy
            .register_status_notifier_host(&wellknown_name)
            .await?;

        Ok(NotifierHost {
            wellknown_name,
            rx: self.tx.subscribe(),
            conn,
        })
    }
}

impl NotifierHost {
    pub async fn recv(&mut self) -> Result<NotifierItemMessage> {
        self.rx
            .recv()
            .await
            .map_err(StatusNotifierWatcherError::from)
    }

    /// This is used to drop the StatusNotifierHost and tell Dbus to release the name
    pub async fn destroy(self) -> Result<()> {
        let _ = self.conn.release_name(self.wellknown_name.as_str()).await?;
        Ok(())
    }
}
