use anyhow::anyhow;
use systray_rs::tray::StatusNotifierItem;
use serde::{Serialize};

#[derive(Serialize)]
pub struct TrayIcon {
    icon_path: String
}

impl TryFrom<&StatusNotifierItem> for TrayIcon {
    type Error = anyhow::Error;

    fn try_from(item: &StatusNotifierItem) -> Result<Self, Self::Error> {
        let icon_name = &item.icon_name;

        let icon_path = if item.icon_theme_path.is_empty() {
            None
        } else {
            Some(item.icon_theme_path.as_str())
        };

        let icon_path = try_fetch_icon(icon_name, icon_path)?;

        Ok(Self {
            icon_path
        })
    }
}

const FALL_BACK_THEME: &str = "hicolor";

fn try_fetch_icon(name: &str, additional_search_path: Option<&str>) -> anyhow::Result<String> {
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

