# Stray

Stray is a minimal [SystemNotifierWatcher](https://www.freedesktop.org/wiki/Specifications/StatusNotifierItem/StatusNotifierWatcher/)
implementation which goal is to provide a minimalistic API to access tray icons and menu.

## Examples

### Start the system tray and listen for changes
```rust, ignore
use stray::{SystemTray};
use tokio_stream::StreamExt;
use stray::message::NotifierItemMessage;
use stray::message::NotifierItemCommand;

#[tokio::main]
async fn main() {

    // A mpsc channel to send menu activation requests later
    let (ui_tx, ui_rx) = tokio::sync::mpsc::channel(32);
    let mut tray = SystemTray::new(ui_rx).await;

    while let Some(message) = tray.next().await {
        match message {
            NotifierItemMessage::Update { address: id, item, menu } => {
                println!("NotifierItem updated :
                    id   = {id},
                    item = {item:?},
                    menu = {menu:?}"
                )
            }
            NotifierItemMessage::Remove { address: id } => {
                println!("NotifierItem removed : id = {id}");
            }
        }
    }
}
```

### Send menu activation request to the system tray

```rust,  ignore
 // Assuming we stored our menu items in some UI state we can send menu item activation request:
 use stray::message::NotifierItemCommand;

 ui_tx.clone().try_send(NotifierItemCommand::MenuItemClicked {
    // The submenu to activate
    submenu_id: 32,
    // dbus menu path, available in the `StatusNotifierItem`
    menu_path: "/org/ayatana/NotificationItem/Element1/Menu".to_string(),
    // the notifier address we previously got from `NotifierItemMessage::Update`
    notifier_address: ":1.2161".to_string(),
 }).unwrap();
```

### Gtk example

For a detailed, real life example, you can take a look at the [gtk-tray](https://github.com/oknozor/stray/tree/main/gtk-tray).

```shell
git clone git@github.com:oknozor/stray.git
cd stray/gtk-tray
cargo run
```
