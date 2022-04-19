use std::collections::HashMap;

use anyhow::anyhow;
use serde::Serialize;
use tera::Tera;
use tokio::sync::mpsc::Receiver;
use zbus::zvariant::OwnedValue;

type DBusProperties = HashMap<std::string::String, OwnedValue>;

#[derive(Serialize, Clone, Debug)]
pub struct TrayIcon {
    icon_path: String,
    tooltip: String,
}
#[derive(Debug)]
pub struct TrayIconMessage {
    pub(crate) theme_path: Option<String>,
    pub(crate) icon_name: String,
}

impl TryFrom<DBusProperties> for TrayIconMessage {
    type Error = anyhow::Error;

    fn try_from(props: HashMap<String, OwnedValue>) -> Result<Self, Self::Error> {
        let theme_path = props
            .get("IconThemePath")
            .ok_or_else(|| anyhow!("Could not get property 'IconThemePath"))
            .map(|theme| theme.downcast_ref::<str>().unwrap_or("").to_string())?;

        let theme_path = if theme_path.is_empty() {
            None
        } else {
            Some(theme_path)
        };

        let icon_name = props
            .get("IconName")
            .ok_or_else(|| anyhow!("Could not get property 'IconName'"))
            .map(|theme| theme.downcast_ref::<str>().unwrap_or("").to_string())?;

        Ok(TrayIconMessage {
            theme_path,
            icon_name,
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
    Update {
        address: String,
        icon: TrayIconMessage,
    },
    Remove {
        address: String,
    },
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
                    let icon_name = try_fetch_icon(&icon.icon_name, icon.theme_path);

                    if let Ok(icon) = icon_name {
                        let icon = TrayIcon {
                            icon_path: icon,
                            tooltip: "the tool tip".to_string(),
                        };
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
        let eww_tray = eww_tray.replace('\n', "");
        println!("{eww_tray}");
    }
}

const FALL_BACK_THEME: &str = "hicolor";

fn try_fetch_icon(name: &str, additional_search_path: Option<String>) -> anyhow::Result<String> {
    if let Some(path) = additional_search_path {
        return Ok(format!("{path}/{name}.png"));
    };

    let theme = linicon::get_system_theme().unwrap();
    linicon::lookup_icon(name)
        .from_theme(theme)
        .use_fallback_themes(true)
        .next()
        .and_then(|icon| icon.ok())
        .or_else(|| {
            linicon::lookup_icon(name)
                .from_theme(FALL_BACK_THEME)
                .next()
                .and_then(|icon| icon.ok())
        })
        .map(|icon| icon.path.to_str().unwrap().to_string())
        .ok_or_else(|| anyhow!("Icon not found"))
}
