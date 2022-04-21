use std::collections::HashMap;

use tera::Tera;

use systray_rs::SystemTray;
use systray_rs::tray::{Message, StatusNotifierItem};
use systray_rs::tokio_stream::StreamExt;
use crate::icon::TrayIcon;

mod icon;

struct EwwTray {
    tray: SystemTray,
    tera: Tera,
    items: HashMap<String, StatusNotifierItem>,
}

impl EwwTray {
    fn render(&self) {
        let mut context = tera::Context::new();
        let tray_icons: Vec<TrayIcon> = self.items.values()
            .filter_map(|item| TrayIcon::try_from(item).ok())
            .collect();
        context.insert("tray_icons", &tray_icons);
        let eww_tray = self.tera.render("default", &context).unwrap();
        let eww_tray = eww_tray.replace('\n', "");
        println!("{eww_tray}");
    }

    async fn run(&mut self) {
        while let Some(message) = self.tray.next().await {
            match message {
                Message::Update { id, item } => {
                    self.items.insert(id, item);
                }
                Message::Remove { address: id } => {
                    self.items.remove(&id);
                }
            }

            self.render();
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = dirs::config_dir().expect("Could not find XDG_CONFIG_DIR");
    let config = config.join("eww-tray.yuck");

    let mut tera = Tera::default();
    tera.add_template_file(config, Some("default"))?;

    EwwTray {
        tray: SystemTray::new().await,
        tera,
        items: HashMap::new(),
    }.run().await;

    Ok(())
}



