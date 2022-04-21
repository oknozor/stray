use std::collections::HashMap;

use tera::Tera;
use systray_rs::menu::TrayMenu;

use systray_rs::SystemTray;
use systray_rs::tray::{Message, StatusNotifierItem};
use systray_rs::tokio_stream::StreamExt;
use crate::icon::EwwTrayItem;

mod icon;

struct EwwTray {
    tray: SystemTray,
    tera: Tera,
    items: HashMap<String, (StatusNotifierItem, Option<TrayMenu>)>,
}

impl EwwTray {
    fn render(&self) {
        let mut context = tera::Context::new();
        let tray_icons: Vec<EwwTrayItem> = self.items.values()
            .filter_map(|item| EwwTrayItem::try_from(item).ok())
            .collect();
        let result = serde_json::to_string(&tray_icons).unwrap();
        println!("{}", result);
        context.insert("tray_icons", &tray_icons);
        let eww_tray = self.tera.render("default", &context).unwrap();
        let eww_tray = eww_tray.replace('\n', "");
        println!("{eww_tray}");
    }

    async fn run(&mut self) {
        while let Some(message) = self.tray.next().await {
            match message {
                Message::Update { id, item, menu } => {
                    self.items.insert(id, (item, menu));
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



