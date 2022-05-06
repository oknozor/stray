use std::cell::{RefCell, RefMut};
use std::collections::HashMap;
use std::ops::Deref;
use std::ptr::NonNull;
use gtk::gio::MenuModel;
use gtk::subclass::prelude::ObjectImpl;
use glib::subclass::prelude::{ObjectSubclass, ObjectSubclassType};
use glib::subclass::{Signal, TypeData};
use glib::{BindingFlags, object_subclass, SignalHandlerId};
use glib::{Type, wrapper};
use glib::clone::Downgrade;
use glib::ffi::GList;
use gtk::{cairo, Container, DirectionType, IconLookupFlags, IconTheme, Image, MenuDirectionType, MenuItem, prelude::*, subclass::prelude::*, Widget};
use once_cell::sync::Lazy;
use crate::{DbusMenuItem, NotifierItem};
use glib::object::ObjectExt;

wrapper! {
    pub struct DbusMenuBar(ObjectSubclass<DbusMenuBarPriv>)
    @extends gtk::Bin, gtk::Container, gtk::Widget, gtk::MenuBar;
}

impl DbusMenuBar {
    pub fn new() -> Self {
        glib::Object::new::<Self>(&[]).expect("Failed to create DubusMenuItem")
    }
}

pub struct DbusMenuBarPriv {
    pub icon_name: RefCell<Option<String>>,
    pub icon_theme_path: RefCell<Option<String>>,
    pub menu_item: RefCell<MenuItem>,
}

impl DbusMenuBarPriv {
    fn get_icon(icon: &str, theme: &Option<String>) -> Option<Image> {
        theme.as_ref().map(|path| {
            let theme = IconTheme::new();
            theme.append_search_path(&path);
            let icon_info = theme
                .lookup_icon(icon, 24, IconLookupFlags::empty())
                .expect("Failed to lookup icon info");

            Image::from_pixbuf(icon_info.load_icon().ok().as_ref())
        })
    }
}

#[object_subclass]
impl ObjectSubclass for DbusMenuBarPriv {
    type ParentType = gtk::Bin;
    type Type = DbusMenuBar;

    const NAME: &'static str = "DbusMenuBar";

    fn class_init(klass: &mut Self::Class) {
        klass.set_css_name("system-tray-menu-bar");
    }

    fn new() -> Self {
        Self {
            icon_name: RefCell::new(None),
            icon_theme_path: RefCell::new(None),
            menu_item: RefCell::new(MenuItem::new()),
        }
    }
}

impl ObjectImpl for DbusMenuBarPriv {
    fn properties() -> &'static [glib::ParamSpec] {
        use once_cell::sync::Lazy;
        static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
            vec![
                glib::ParamSpec::new_string(
                    "icon-name",
                    "icon-name",
                    "icon-name",
                    None,
                    glib::ParamFlags::READWRITE,
                ),
                glib::ParamSpec::new_string(
                    "icon-theme-path",
                    "icon-theme-path",
                    "icon-theme-path",
                    None,
                    glib::ParamFlags::READWRITE,
                ),
            ]
        });

        PROPERTIES.as_ref()
    }

    fn set_property(&self, obj: &Self::Type, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
        match pspec.name() {
            "icon-name" => {
                let icon = value.get::<String>()
                    .expect("type conformity checked by `Object::set_property`");

                self.icon_name.replace_with(|old| {
                    if let Some(value) = old {
                        if value != &icon {
                            Some(icon)
                        } else {
                            old.to_owned()
                        }
                    } else {
                        Some(icon)
                    }
                });

                let new_icon = self.icon_name.borrow();
                let new_icon = new_icon.as_ref()
                    .expect("icon_name should be known at this point");

                let image = DbusMenuBarPriv::get_icon(&new_icon, &*self.icon_theme_path.borrow());
                if let Some(icon) = image {
                    let menu_item = &*self.menu_item.borrow();
                    let menu_item_box = gtk::Box::builder()
                        .child(&icon)
                        .build();
                    for widget in menu_item.children() {
                        menu_item.remove(&widget);
                    }
                    menu_item.add(&menu_item_box);
                    obj.queue_draw();
                }
            }

            "icon-theme-path" => {
                let theme = value.get::<String>()
                    .expect("type conformity checked by `Object::set_property`");

                self.icon_theme_path.replace_with(|old| {
                    if let Some(value) = old {
                        if value != &theme {
                            Some(theme)
                        } else {
                            old.to_owned()
                        }
                    } else {
                        Some(theme)
                    }
                });
            }
            x => panic!("Tried to set inexistant property of DbusMenuItem: {}", x, ),
        }
    }

    fn property(&self, _obj: &Self::Type, _id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        match pspec.name() {
            "icon-name" => self.icon_name.borrow().to_value(),
            "icon-theme" => self.icon_theme_path.borrow().to_value(),
            x => panic!("Tried to access inexistant property of CircProg: {}", x, ),
        }
    }

    fn constructed(&self, obj: &DbusMenuBar) {
        self.parent_constructed(obj);
        let self_ = obj.downcast_ref::<DbusMenuBar>().unwrap();
        let menu_bar_priv = DbusMenuBarPriv::from_instance(obj);
        let menu_item = &*menu_bar_priv.menu_item.borrow();
        self_.add(menu_item);
    }
}


impl ContainerImpl for DbusMenuBarPriv {

}

impl Default for DbusMenuBarPriv {
    fn default() -> Self {
        Self {
            icon_name: RefCell::new(None),
            icon_theme_path: RefCell::new(None),
            menu_item: RefCell::new(MenuItem::new()),
        }
    }
}

impl BinImpl for DbusMenuBarPriv {
}

impl WidgetImpl for DbusMenuBarPriv {}