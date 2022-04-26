use stray::{message::Message, SystemTray};
use tokio_stream::StreamExt;

#[tokio::main]
async fn main() {
    let mut tray = SystemTray::new().await;

    while let Some(message) = tray.next().await {
        match message {
            Message::Update { id, item, menu } => {
                println!("Got Update command id={id}, item={item:?}, menu={menu:?}");
            }
            Message::Remove { address: id } => {
                println!("Got Remove command  id={id}");
            }
        }
    }
}
