use stray::message::NotifierItemMessage;
use stray::SystemTray;
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() {
    let (_ui_tx, ui_rx) = tokio::sync::mpsc::channel(32);
    let mut tray = SystemTray::new(ui_rx).await;

    while let Some(message) = tray.next().await {
        match message {
            NotifierItemMessage::Update {
                address: id,
                item,
                menu,
            } => {
                println!(
                    "NotifierItem updated :
                    id   = {id},
                    item = {item:?},
                    menu = {menu:?}"
                );
            }
            NotifierItemMessage::Remove { address: id } => {
                println!("NotifierItem removed : id = {id}");
            }
        }
    }
}
