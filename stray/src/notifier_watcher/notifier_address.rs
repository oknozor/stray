use crate::error;
use crate::error::StatusNotifierWatcherError;

// A helper to convert RegisterStatusNotifier calls to
// StatusNotifier address parts
#[derive(Debug)]
pub(crate) struct NotifierAddress {
    // Notifier destination on the bus, ex: ":1.522"
    pub(crate) destination: String,
    // The notifier object path, ex: "/org/ayatana/NotificationItem/Element1"
    pub(crate) path: String,
}

impl NotifierAddress {
    pub(crate) fn from_notifier_service(service: &str) -> error::Result<Self> {
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
            Err(StatusNotifierWatcherError::DbusAddressError(
                service.to_string(),
            ))
        }
    }
}
