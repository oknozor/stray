use gtk::glib;
use gtk::prelude::*;
use gtk::{IconLookupFlags, IconTheme, Image, Menu, MenuBar, MenuItem, SeparatorMenuItem};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;
use std::thread;
use stray::message::menu::{MenuType, TrayMenu};
use stray::message::tray::StatusNotifierItem;
use stray::message::{NotifierItemCommand, NotifierItemMessage};
use stray::StatusNotifierWatcher;
use tokio::runtime::Runtime;
use tokio::sync::mpsc;
use glib::GBoxed;
use crate::menu_bar::DbusMenuBar;
use crate::menu_item::DbusMenuItem;

pub mod menu_item;
pub mod menu_bar;

struct NotifierItem {
    item: StatusNotifierItem,
    menu: Option<TrayMenu>,
}

static STATE: Lazy<Mutex<HashMap<String, NotifierItem>>> = Lazy::new(|| Mutex::new(HashMap::new()));

impl NotifierItem {
    fn get_icon(&self) -> Option<Image> {
        self.item.icon_theme_path.as_ref().map(|path| {
            let theme = IconTheme::new();
            theme.append_search_path(&path);
            let icon_name = self.item.icon_name.as_ref().unwrap();
            let icon_info = theme
                .lookup_icon(icon_name, 24, IconLookupFlags::empty())
                .expect("Failed to lookup icon info");

            Image::from_pixbuf(icon_info.load_icon().ok().as_ref())
        })
    }
}

fn main() {
    let application = gtk::Application::new(
        Some("com.github.gtk-rs.examples.menu_bar_system"),
        Default::default(),
    );

    application.connect_activate(build_ui);

    application.run();
}

fn build_ui(application: &gtk::Application) {
    let window = gtk::ApplicationWindow::new(application);
    window.set_title("System menu bar");
    window.set_border_width(10);
    window.set_position(gtk::WindowPosition::Center);
    window.set_default_size(350, 70);

    let menu_bar = DbusMenuBar::new();
    window.add(&menu_bar);
    let (sender, receiver) = mpsc::channel(32);
    let (cmd_tx, cmd_rx) = mpsc::channel(32);

    spawn_local_handler(menu_bar, receiver, cmd_tx);
    start_communication_thread(sender, cmd_rx);
    window.show_all();
}

fn spawn_local_handler(
    menu_bar: DbusMenuBar,
    mut receiver: mpsc::Receiver<NotifierItemMessage>,
    cmd_tx: mpsc::Sender<NotifierItemCommand>,
) {
    let main_context = glib::MainContext::default();
    let future = async move {
        while let Some(item) = receiver.recv().await {
            let mut state = STATE.lock().unwrap();

            match item {
                NotifierItemMessage::Update {
                    address: id,
                    item,
                    menu,
                } => {
                    state.insert(id, NotifierItem { item: *item, menu });
                }
                NotifierItemMessage::Remove { address } => {
                    state.remove(&address);
                }
            }



            for (address, notifier_item) in state.iter() {
                if let Some(icon) = notifier_item.get_icon() {
                    let icon_name = notifier_item.item.icon_name.clone();
                    let icon_theme = notifier_item.item.icon_theme_path.clone();

                    menu_bar.set_property("icon-theme-path", icon_theme.unwrap()).unwrap();
                    menu_bar.set_property("icon-name", icon_name.unwrap()).unwrap();
                };

                menu_bar.show_all();
            }
        }
    };

    main_context.spawn_local(future);
}

fn start_communication_thread(
    sender: mpsc::Sender<NotifierItemMessage>,
    cmd_rx: mpsc::Receiver<NotifierItemCommand>,
) {
    thread::spawn(move || {
        let runtime = Runtime::new().expect("Failed to create tokio RT");

        runtime.block_on(async {
            let tray = StatusNotifierWatcher::new(cmd_rx).await.unwrap();
            let mut host = tray.create_notifier_host("MyHost").await.unwrap();

            while let Ok(message) = host.recv().await {
                sender
                    .send(message)
                    .await
                    .expect("failed to send message to UI");
            }

            host.destroy().await.unwrap();
        })
    });
}
