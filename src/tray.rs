use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::anyhow;
use serde::Serialize;
use tera::Tera;
use tokio::sync::mpsc::Receiver;
use zbus::zvariant::OwnedValue;

type DBusProperties = HashMap<std::string::String, OwnedValue>;

#[derive(Serialize, Clone, Debug)]
pub struct TrayIcon {
    pub(crate) icon_path: String,
    pub(crate) tooltip: String,
}

impl TryFrom<DBusProperties> for TrayIcon {
    type Error = anyhow::Error;

    fn try_from(props: HashMap<String, OwnedValue>) -> Result<Self, Self::Error> {
        let icon_theme_path = props
            .get("IconThemePath")
            .ok_or_else(|| anyhow!("Could not get property 'IconThemePath"))
            .map(|theme| theme.downcast_ref::<str>().unwrap_or("").to_string())?;

        let icon_name = props
            .get("IconName")
            .ok_or_else(|| anyhow!("Could not get property 'IconName'"))
            .map(|theme| theme.downcast_ref::<str>().unwrap_or("").to_string())?;

        Ok(TrayIcon {
            icon_path: format!("{icon_theme_path}/{icon_name}.png"),
            tooltip: "The tooltip".to_string(),
        })
    }
}

pub struct TrayUpdater {
    pub(crate) icons: HashMap<String, TrayIcon>,
    pub(crate) rx: Receiver<Message>,
    tera: Tera,
}

#[derive(Debug)]
pub enum Message {
    Update { address: String, icon: TrayIcon },
    Remove { address: String },
}

impl TrayUpdater {
    pub fn new(rx: Receiver<Message>) -> Self {
        let config = dirs::config_dir().expect("Could not find XDG_CONFIG_DIR");
        let config = config.join("eww-tray.yuck");

        let mut tera = Tera::default();
        tera.add_template_file(config, Some("default"))
            .expect("Failed to open template file");
        Self {
            icons: Default::default(),
            rx,
            tera,
        }
    }

    pub async fn run(&mut self) {
        while let Some(message) = self.rx.recv().await {
            match message {
                Message::Update { address, icon } => {
                    if PathBuf::from(&icon.icon_path).exists() {
                        let _ = self.icons.insert(address, icon);
                    }
                }
                Message::Remove { address } => {
                    let _ = self.icons.remove(&address);
                }
            }

            self.render();
        }
    }

    pub fn render(&self) {
        let mut context = tera::Context::new();
        let tray_icons: Vec<TrayIcon> = self.icons.values().cloned().collect();
        context.insert("tray_icons", &tray_icons);
        let eww_tray = self.tera.render("default", &context).unwrap();
        println!("{eww_tray}");
    }
}
