use std::collections::HashMap;
use std::str::FromStr;

use anyhow::anyhow;
use serde::Serialize;
use zbus::zvariant::{ObjectPath, OwnedValue};

type DBusProperties = HashMap<std::string::String, OwnedValue>;

struct Props(DBusProperties);

#[derive(Debug, Serialize)]
pub struct NotifierItems {
    items: HashMap<String, StatusNotifierItem>,
}

#[derive(Debug, Serialize)]
pub enum Message {
    Update {
        id: String,
        item: StatusNotifierItem,
    },
    Remove {
        address: String,
    },
}

/// Represent a Notifier item status, see https://github.com/AyatanaIndicators/libayatana-appindicator/blob/c43a76e643ab930725d20d306bc3ca5e7874eebe/src/notification-item.xml
/// TODO
#[derive(Serialize, Debug)]
pub struct StatusNotifierItem {
    pub id: String,
    /// Describes the category of this item.
    pub category: Category,
    pub status: Status,

    /// The StatusNotifierItem can carry an icon that can be used by the visualization to identify the item.
    /// An icon can either be identified by its Freedesktop-compliant icon name, carried by
    /// this property of by the icon data itself, carried by the property IconPixmap.
    /// Visualizations are encouraged to prefer icon names over icon pixmaps if both are available
    pub icon_name: String,
    /// Carries an ARGB32 binary representation of the icon, the format of icon data used in this specification
    /// is described in Section Icons
    pub icon_accessible_desc: String,
    pub attention_icon_name: String,
    pub attention_accessible_desc: String,
    /// It's a name that describes the application, it can be more descriptive than Id.
    pub title: String,
    pub icon_theme_path: String,
    pub menu: String,
    pub x_ayatana_label: String,
    pub x_ayatana_label_guide: String,
    pub x_ayatana_ordering_index: u32,
}

#[derive(Serialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub enum Status {
    Passive,
    Active,
}

impl FromStr for Status {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Passive" => Ok(Status::Active),
            "Active" => Ok(Status::Passive),
            other => Err(anyhow!("Unknown 'Status' for status notifier item {}", other))
        }
    }
}

/// Describes the category of this item.
#[derive(Serialize, Debug)]
#[serde(rename_all = "PascalCase")]
pub enum Category {
    /// The item describes the status of a generic application, for instance the current state
    /// of a media player. In the case where the category of the item can not be known, such as
    /// when the item is being proxied from another incompatible or emulated system,
    /// ApplicationStatus can be used a sensible default fallback.
    ApplicationStatus,
    /// The item describes the status of communication oriented applications, like an instant
    /// messenger or an email client.
    Communications,
    /// The item describes services of the system not seen as a stand alone application by the user,
    /// such as an indicator for the activity of a disk indexing service.
    SystemServices,
    /// The item describes the state and control of a particular hardware, such as an indicator
    /// of the battery charge or sound card volume control.
    Hardware,
}

impl FromStr for Category {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "ApplicationStatus" => Ok(Category::ApplicationStatus),
            "Communications" => Ok(Category::Communications),
            "SystemServices" => Ok(Category::SystemServices),
            "Hardware" => Ok(Category::Hardware),
            other => Err(anyhow!("Unknown 'Status' for status notifier item {}", other))
        }
    }
}

/// ***
/// It's a name that should be unique for this application and consistent between sessions,
/// such as the application name itself.
/// ***
///
impl TryFrom<DBusProperties> for StatusNotifierItem {
    type Error = anyhow::Error;
    fn try_from(props: HashMap<String, OwnedValue>) -> anyhow::Result<Self> {
        let props = Props(props);

        Ok(StatusNotifierItem {
            id: props.get_string("Id").ok_or_else(||anyhow!("Missing property 'Id'"))?,
            title: props.get_string("Title").ok_or_else(||anyhow!("Missing property 'Title'"))?,
            category: props.get_category()?,
            icon_name: props.get_string("IconName").ok_or_else(||anyhow!("Missing property 'IconName'"))?,
            status: props.get_status()?,
            icon_accessible_desc: props.get_string("IconAccessibleDesc").ok_or_else(||anyhow!("Missing property 'IconAccessibleDesc'"))?,
            attention_icon_name: props.get_string("AttentionIconName").ok_or_else(||anyhow!("Missing property 'AttentionIconName'"))?,
            attention_accessible_desc: props.get_string("AttentionAccessibleDesc").ok_or_else(||anyhow!("Missing property 'AttentionAccessibleDesc'"))?,
            icon_theme_path: props.get_string("IconThemePath").ok_or_else(||anyhow!("Missing property 'IconThemePath'"))?,
            menu: props.get_object_path("Menu").ok_or_else(||anyhow!("Missing property 'Menu'"))?,
            x_ayatana_label: props.get_string("XAyatanaLabel").ok_or_else(||anyhow!("Missing property 'XAyatanaLabel'"))?,
            x_ayatana_label_guide: props.get_string("XAyatanaLabelGuide").ok_or_else(||anyhow!("Missing property 'XAyatanaLabelGuide'"))?,
            x_ayatana_ordering_index: props.get_u32("XAyatanaOrderingIndex").ok_or_else(||anyhow!("Missing property 'XAyatanaOrderingIndex'"))?,
        })
    }
}

impl Props {
    fn get_string(&self, key: &str) -> Option<String> {
        self.0.get(key)
            .map(|value| value.downcast_ref::<str>()
                .map(|value| value.to_string()))
            .flatten()
    }

    fn get_object_path(&self, key: &str) -> Option<String> {
        self.0.get(key)
            .map(|value| value.downcast_ref::<ObjectPath>()
                .map(|value| value.to_string()))
            .flatten()
    }

    fn get_category(&self) -> anyhow::Result<Category> {
        self.0.get("Category")
            .map(|value| value.downcast_ref::<str>()
                .map(|value| Category::from_str(value)))
            .flatten()
            .unwrap_or(Err(anyhow!("'Category' not found for item")))
    }

    fn get_status(&self) -> anyhow::Result<Status> {
        self.0.get("Status")
            .map(|value| value.downcast_ref::<str>()
                .map(|value| Status::from_str(value)))
            .flatten()
            .unwrap_or(Err(anyhow!("'Status' not found for item")))
    }

    fn get_u32(&self, key: &str) -> Option<u32> {
        self.0.get(key)
            .map(|value| value.downcast_ref::<u32>()
                .map(|value| *value))
            .flatten()
    }
}
