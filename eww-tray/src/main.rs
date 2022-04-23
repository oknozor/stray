use serde::Serialize;
use std::collections::HashMap;

use tera::Tera;

use systray_rs::message::menu::TrayMenu;
use systray_rs::message::tray::StatusNotifierItem;
use systray_rs::message::Message;
use systray_rs::tokio_stream::StreamExt;
use systray_rs::SystemTray;

use crate::icon::EwwTrayItem;

mod icon;

struct EwwTray {
    tray: SystemTray,
    tera: Tera,
    items: HashMap<String, (StatusNotifierItem, Option<TrayMenu>)>,
}

impl EwwTray {
    fn render<T>(&self, value: T)
    where
        T: Serialize,
    {
        let mut context = tera::Context::new();
        context.insert("tray_icons", &value);
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
            let tray_icons: Vec<EwwTrayItem> = self
                .items
                .values()
                .filter_map(|item| EwwTrayItem::try_from(item).ok())
                .collect();
            let menus = tray_icons
                .iter()
                .filter_map(|item| item.menu.as_ref())
                .map(|s| s.to_string())
                .collect::<Vec<String>>()
                .join(" ");

            let update = format!("tray_menu_content={}", &menus);
            println!("{}", update);
            tokio::process::Command::new("eww")
                .args(&["update", &update])
                .output()
                .await
                .unwrap();

            self.render(tray_icons);
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
    }
    .run()
    .await;

    Ok(())
}
