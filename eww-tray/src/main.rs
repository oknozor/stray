use std::collections::HashMap;
use stray::message::Message;
use stray::SystemTray;
use stray::tokio_stream::StreamExt;

use crate::icon::{EwwTrayItem, EwwTrayMenu, EwwTrayOutput, EwwTraySubMenu};

mod icon;

struct EwwTray {
    tray: SystemTray,
    icons: HashMap<String, EwwTrayItem>,
    menus: HashMap<String, Vec<EwwTraySubMenu>>,
}

impl EwwTray {
    async fn run(&mut self) -> anyhow::Result<()> {
        while let Some(message) = self.tray.next().await {
            self.handle_message(message)?;
            let json_tray = serde_json::to_string(&self.get_output())?;
            println!("{}", json_tray);
        }

        Ok(())
    }

    fn handle_message(&mut self, message: Message) -> anyhow::Result<()> {
        match message {
            Message::Update { id, item, menu } => {
                let icon = EwwTrayItem::try_from(&item)?;
                let icon_id = icon.id.clone();
                self.icons.insert(id, icon);
                menu.and_then(|menu| {
                    self.menus.insert(icon_id, EwwTrayMenu::from(&menu).submenu)
                });
            }
            Message::Remove { address: id } => {
                if let Some(icon_removed) = self.icons.remove(&id) {
                    self.menus.remove(&icon_removed.id);
                }
            }
        }

        Ok(())
    }

    fn get_output(&self) -> EwwTrayOutput {
        let icons = self.icons.iter()
            .map(|(_, icon)| icon)
            .collect();

        EwwTrayOutput {
            icons,
            menus: &self.menus
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    EwwTray {
        tray: SystemTray::new().await,
        icons: Default::default(),
        menus: Default::default()
    }
        .run()
        .await?;

    Ok(())
}
