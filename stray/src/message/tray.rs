use std::collections::HashMap;
use std::str::FromStr;

use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use zbus::zvariant::{Array, ObjectPath, OwnedValue, Structure};

type DBusProperties = HashMap<String, OwnedValue>;

struct PropsWrapper(DBusProperties);

/// An Icon used for reporting the status of an application to the user or provide a quick access
/// to common actions performed by that application. You can read the full specification at
/// [freedesktop.org/wiki/Specifications/StatusNotifierItem](https://freedesktop.org/wiki/Specifications/StatusNotifierItem)
/// or take a look at [the reference implementation](https://github.com/AyatanaIndicators/libayatana-appindicator/blob/c43a76e643ab930725d20d306bc3ca5e7874eebe/src/notification-item.xml)
///
/// Note that this implementation is not feature complete. It only contains the minimal data
/// needed to build a system tray and display tray menus. If you feel something important is
/// should be added please reach out.
#[derive(Serialize, Debug, Clone)]
pub struct StatusNotifierItem {
    /// It's a name that should be unique for this application and consistent between sessions,
    /// such as the application name itself.
    pub id: String,
    /// Describes the category of this item.
    pub category: Category,
    /// Describes the status of this item or of the associated application.
    pub status: Status,
    /// The StatusNotifierItem can carry an icon that can be used by the visualization to identify the item.
    /// An icon can either be identified by its Freedesktop-compliant icon name, carried by
    /// this property of by the icon data itself, carried by the property IconPixmap.
    /// Visualizations are encouraged to prefer icon names over icon pixmaps if both are available
    pub icon_name: Option<String>,
    /// Carries an ARGB32 binary representation of the icon, the format of icon data used in this specification
    /// is described in Section Icons
    pub icon_accessible_desc: Option<String>,
    /// The Freedesktop-compliant name of an icon. this can be used by the visualization to indicate
    /// that the item is in RequestingAttention state.
    pub attention_icon_name: Option<String>,
    /// It's a name that describes the application, it can be more descriptive than Id.
    pub title: Option<String>,
    pub icon_theme_path: Option<String>,
    pub icon_pixmap: Option<Vec<IconPixmap>>,
    /// DBus path to an object which should implement the com.canonical.dbusmenu interface
    /// This can be used to retrieve the wigdet menu via gtk/qt libdbusmenu implementation
    /// Instead of building it from the raw data
    pub menu: Option<String>,
}

#[derive(Serialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub enum Status {
    /// The item doesn't convey important information to the user, it can be considered an
    /// "idle" status and is likely that visualizations will chose to hide it.
    Passive,
    /// The item is active, is more important that the item will be shown in some way to the user.
    Active,
}

impl FromStr for Status {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Passive" => Ok(Status::Active),
            "Active" => Ok(Status::Passive),
            other => Err(anyhow!(
                "Unknown 'Status' for status notifier item {}",
                other
            )),
        }
    }
}

/// Describes the category of this item.
#[derive(Serialize, Debug, Clone)]
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
            other => Err(anyhow!(
                "Unknown 'Status' for status notifier item {}",
                other
            )),
        }
    }
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct IconPixmap {
    pub width: i32,
    pub height: i32,
    pub pixels: Vec<u8>,
}

impl IconPixmap {
    fn from_array(a: &Array<'_>) -> Option<Vec<Self>> {
        let mut pixmaps = vec![];

        a.iter().for_each(|b| {
            let s = b.downcast_ref::<Structure>();
            let fields = s.unwrap().fields();
            let width = fields[0].downcast_ref::<i32>().unwrap();
            let height = fields[1].downcast_ref::<i32>().unwrap();
            let pixel_values = fields[2].downcast_ref::<Array>().unwrap().get();
            let mut pixels = vec![];
            pixel_values.iter().for_each(|p| {
                pixels.push(*p.downcast_ref::<u8>().unwrap());
            });
            pixmaps.push(IconPixmap {
                width: *width,
                height: *height,
                pixels,
            })
        });

        Some(pixmaps)
    }
}

impl TryFrom<DBusProperties> for StatusNotifierItem {
    type Error = anyhow::Error;
    fn try_from(props: HashMap<String, OwnedValue>) -> anyhow::Result<Self> {
        let props = PropsWrapper(props);
        match props.get_string("Id") {
            None => Err(anyhow!("StatusNotifier item should have an id")),
            Some(id) => Ok(StatusNotifierItem {
                id,
                title: props.get_string("Title"),
                category: props.get_category()?,
                icon_name: props.get_string("IconName"),
                status: props.get_status()?,
                icon_accessible_desc: props.get_string("IconAccessibleDesc"),
                attention_icon_name: props.get_string("AttentionIconName"),
                icon_theme_path: props.get_string("IconThemePath"),
                icon_pixmap: props.get_icon_pixmap(),
                menu: props.get_object_path("Menu"),
            }),
        }
    }
}

impl PropsWrapper {
    fn get_string(&self, key: &str) -> Option<String> {
        self.0
            .get(key)
            .and_then(|value| value.downcast_ref::<str>().map(|value| value.to_string()))
    }

    fn get_object_path(&self, key: &str) -> Option<String> {
        self.0.get(key).and_then(|value| {
            value
                .downcast_ref::<ObjectPath>()
                .map(|value| value.to_string())
        })
    }

    fn get_category(&self) -> anyhow::Result<Category> {
        self.0
            .get("Category")
            .and_then(|value| value.downcast_ref::<str>().map(Category::from_str))
            .unwrap_or_else(|| Err(anyhow!("'Category' not found for item")))
    }

    fn get_status(&self) -> anyhow::Result<Status> {
        self.0
            .get("Status")
            .and_then(|value| value.downcast_ref::<str>().map(Status::from_str))
            .unwrap_or_else(|| Err(anyhow!("'Status' not found for item")))
    }

    fn get_icon_pixmap(&self) -> Option<Vec<IconPixmap>> {
        self.0
            .get("IconPixmap")
            .and_then(|value| value.downcast_ref::<Array>().map(IconPixmap::from_array))
            .unwrap_or(None)
    }
}
