use std::cell::RefCell;
use std::ptr::NonNull;
use gtk::gio::MenuModel;
use gtk::subclass::prelude::ObjectImpl;
use glib::subclass::prelude::{ObjectSubclass, ObjectSubclassType};
use glib::subclass::{Signal, TypeData};
use glib::object_subclass;
use glib::{Type, wrapper};
use glib::clone::Downgrade;
use gtk::{prelude::*, subclass::prelude::*};
use once_cell::sync::Lazy;

wrapper! {
    pub struct DbusMenuItem(ObjectSubclass<DbusMenuItemPriv>)
    @extends gtk::Bin, gtk::Container, gtk::Widget, gtk::MenuItem;
}

impl DbusMenuItem {
    pub fn new() -> Self {
        glib::Object::new::<Self>(&[]).expect("Failed to create DubusMenuItem")
    }
}

pub struct DbusMenuItemPriv {
    pub label: RefCell<String>,
    pub content: RefCell<Option<gtk::Widget>>,
}

#[object_subclass]
impl ObjectSubclass for DbusMenuItemPriv {
    type ParentType = gtk::Bin;
    type Type = DbusMenuItem;

    const NAME: &'static str = "DbusMenuItem";

    fn class_init(klass: &mut Self::Class) {
        klass.set_css_name("system-tray-menu-item");
    }

    fn new() -> Self {
        Self {
            label: RefCell::new("".to_string()),
            content: RefCell::new(None)
        }
    }
}

impl ObjectImpl for DbusMenuItemPriv {
    fn properties() -> &'static [glib::ParamSpec] {
        use once_cell::sync::Lazy;
        static PROPERTIES: Lazy<Vec<glib::ParamSpec>> = Lazy::new(|| {
            vec![
                glib::ParamSpec::new_string("label", "Label", "The label", None, glib::ParamFlags::READWRITE),
            ]
        });

        PROPERTIES.as_ref()
    }

    fn signals() -> &'static [Signal] {
        static SIGNALS: Lazy<Vec<Signal>> = Lazy::new(|| {
            vec![Signal::builder(
                // Signal name
                "label-changed",
                // Types of the values which will be sent to the signal handler
                &[str::static_type().into()],
                // Type of the value the signal handler sends back
                <()>::static_type().into(),
            )
                .build()]
        });
        SIGNALS.as_ref()
    }
}


impl ContainerImpl for DbusMenuItemPriv {
    fn add(&self, container: &Self::Type, widget: &gtk::Widget) {
        if let Some(widget) = &*self.content.borrow() {
            self.parent_remove(container, widget);
        }
        self.parent_add(container, widget);
        self.content.replace(Some(widget.clone()));
    }
}

impl DbusMenuItemPriv {
    fn update(&self, new_label: String) {
        self.label.replace_with(|label| {
            if label != new_label.as_str() {
                new_label
            } else {
                label.to_string()
            }
        });
    }
}

impl Default for DbusMenuItemPriv {
    fn default() -> Self {
        Self {
            label: RefCell::new("".to_string()),
            content: RefCell::new(None)
        }
    }
}

impl BinImpl for DbusMenuItemPriv {}
impl WidgetImpl for DbusMenuItemPriv {}