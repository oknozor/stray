use crate::NotifierItemMessage;
use thiserror::Error;
use tokio::sync::broadcast;

pub type Result<T> = std::result::Result<T, StatusNotifierWatcherError>;

#[derive(Error, Debug)]
pub enum StatusNotifierWatcherError {
    #[error("Dbus connection error")]
    DbusError(#[from] zbus::Error),
    #[error("Invalid DBus interface name")]
    InterfaceNameError(#[from] zbus::names::Error),
    #[error("Failed to call DBus standard interface method")]
    DBusStandardInterfaceError(#[from] zbus::fdo::Error),
    #[error("Serialization error")]
    ZvariantError(#[from] zbus::zvariant::Error),
    #[error("Service path {0} was not understood")]
    DbusAddressError(String),
    #[error("Failed to broadcast message to notifier hosts")]
    BroadCastSendError(#[from] broadcast::error::SendError<NotifierItemMessage>),
    #[error("Error receiving broadcast message")]
    BroadCastRecvError(#[from] broadcast::error::RecvError),
}
