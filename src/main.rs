// SPDX-License-Identifier: MIT
//
// Copyright (c) 2025 Lorenzo Carbonell
//
// This file is part of the LiLa project,
// and is licensed under the MIT License. See the LICENSE file for details.
//

mod actions;
mod finders;
mod history;
mod models;
mod utils;

use actions::launch;
use finders::rank_items;
use gdk::Display;
use gtk::prelude::*;
use gtk::{Label, ListBox, ScrolledWindow};
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use history::{HistoryRepository, SqliteHistory};
use models::{Config, Css, Item};
use std::cell::RefCell;
use std::io;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use tracing::debug;
use utils::*;

// https://github.com/wmww/gtk-layer-shell/blob/master/examples/simple-example.c
/// Registra en el historial el lanzamiento de `item` si está disponible y activo.
///
/// Un fallo al registrar no interrumpe el lanzamiento: solo se avisa por log.
fn record_launch(history: Option<&SqliteHistory>, enabled: bool, item: &Item) {
    if !enabled {
        return;
    }
    let Some(store) = history else {
        return;
    };
    if let Err(err) = store.record(&item.id(), now_millis()) {
        tracing::warn!("No se pudo registrar el lanzamiento: {err}");
    }
}

fn activate(
    application: &gtk::Application,
    mutex_config: &Arc<Mutex<Config>>,
    history: Option<Rc<SqliteHistory>>,
) {
    if let Some(existing_window) = application.windows().into_iter().next() {
        existing_window.present(); // Muestra y enfoca la ventana existente
        return; // Salimos, no creamos una nueva ventana
    }
    let config;
    {
        config = mutex_config.lock().unwrap();
    }
    let apps: Vec<Item> = config
        .extensions
        .iter()
        .find(|e| e.name == "application-finder")
        .or_else(|| config.extensions.first())
        .map(|e| e.finder.search(""))
        .unwrap_or_default();
    let apps = Rc::new(apps);
    let current: Rc<RefCell<Vec<Item>>> = Rc::new(RefCell::new(Vec::new()));
    // Las señales de GTK exigen closures `'static`: se clona la config y el
    // historial una sola vez y se reparten entre las señales.
    let ranking_config = Rc::new(config.clone());
    let history_enabled = config.history_enabled;
    // Create a normal GTK window however you like
    let window = gtk::ApplicationWindow::new(application);
    window.set_size_request(config.width, config.height);
    window.add_css_class("transparente");

    // Before the window is first realized, set it up to be a layer surface
    window.init_layer_shell();
    window.set_keyboard_mode(gtk4_layer_shell::KeyboardMode::Exclusive);
    window.set_namespace(Some(APP_NAME));

    // Display above normal windows
    window.set_layer(Layer::Overlay);
    window.set_decorated(false);
    window.set_focusable(true);

    // Push other windows out of the way
    window.auto_exclusive_zone_enable();

    window.connect_close_request(move |win| {
        win.set_visible(false); // Oculta la ventana
        true.into()
    });

    // Anchors are if the window is pinned to each edge of the output
    let anchors = [
        (Edge::Left, config.left),
        (Edge::Right, config.right),
        (Edge::Top, config.top),
        (Edge::Bottom, config.bottom),
    ];

    for (anchor, edge_config) in anchors {
        window.set_margin(anchor, edge_config.margin);
        window.set_anchor(anchor, edge_config.anchor);
    }

    let vbox = gtk::Box::new(gtk::Orientation::Vertical, 10);
    vbox.set_margin_start(10);
    vbox.set_margin_end(10);
    vbox.set_margin_top(10);
    vbox.set_margin_bottom(10);

    window.connect_is_active_notify(|win| {
        debug!("{:?}", win.is_active());
    });
    window.connect_notify_local(Some("is-active"), move |win, _| {
        debug!("{:?}", win.is_active());
        match win.is_active() {
            true => (),
            false => {
                win.close();
            }
        }
    });
    // Crear el controller para detectar teclas
    let window_clone = window.clone();

    // Entry donde el usuario escribe
    let entry = gtk::Entry::new();

    // ListBox para mostrar sugerencias
    let scroll = ScrolledWindow::builder()
        .name(SCROLL_NAME)
        .hscrollbar_policy(gtk::PolicyType::Never)
        .build();
    scroll.set_size_request(config.width, config.height);

    let listbox = ListBox::builder().name(LISTBOX_NAME).build();
    scroll.set_child(Some(&listbox));
    scroll.set_can_focus(false);
    listbox.set_visible(false);
    listbox.set_can_focus(false);

    let listbox_sel = listbox.clone();
    let key_controller = gtk::EventControllerKey::new();
    key_controller.connect_key_pressed(move |_, keyval, _keycode, _state| {
        debug!("Tecla pulsada: {:?}", keyval.name());
        // Por ejemplo, detectar Enter
        if keyval == gdk::Key::Escape {
            debug!("Se pulsó Escape");
            window_clone.set_visible(false);
            debug!("Window hidden");
            true.into()
        } else if keyval == gdk::Key::Down {
            let next = listbox_sel
                .selected_row()
                .map(|r| r.index() + 1)
                .unwrap_or(0);
            if let Some(row) = listbox_sel.row_at_index(next) {
                listbox_sel.select_row(Some(&row));
            }
            true.into()
        } else if keyval == gdk::Key::Up {
            let prev = listbox_sel
                .selected_row()
                .map(|r| r.index() - 1)
                .unwrap_or(0)
                .max(0);
            if let Some(row) = listbox_sel.row_at_index(prev) {
                listbox_sel.select_row(Some(&row));
            }
            true.into()
        } else {
            false.into()
        }
    });
    entry.add_controller(key_controller);
    let current_launch = Rc::clone(&current);
    let listbox_launch = listbox.clone();
    let window_launch = window.clone();
    let history_launch = history.clone();
    entry.connect_activate(move |_| {
        let idx = listbox_launch
            .selected_row()
            .map(|r| r.index())
            .unwrap_or(0);
        if let Some(item) = current_launch.borrow().get(idx as usize).cloned() {
            match launch(&item) {
                Ok(()) => {
                    window_launch.set_visible(false);
                    record_launch(history_launch.as_deref(), history_enabled, &item);
                }
                Err(err) => tracing::error!("No se pudo lanzar la aplicación: {err}"),
            }
        }
    });

    // Filtrado en vivo al escribir
    let apps_changed = Rc::clone(&apps);
    let current_changed = Rc::clone(&current);
    let listbox_changed = listbox.clone();
    entry.connect_changed(move |e| {
        let text = e.text().to_string();
        let history: Option<&dyn HistoryRepository> = history
            .as_ref()
            .map(|store| store.as_ref() as &dyn HistoryRepository);
        let now_millis = now_millis();
        let results = rank_items(&apps_changed, &text, history, &ranking_config, now_millis);
        listbox_changed.remove_all();
        for item in &results {
            listbox_changed.append(&Label::new(Some(item.name())));
        }
        *current_changed.borrow_mut() = results;
        let hay = !current_changed.borrow().is_empty() && !e.text().is_empty();
        listbox_changed.set_visible(hay);
        if hay {
            listbox_changed.select_row(listbox_changed.row_at_index(0).as_ref());
        }
    });

    vbox.append(&entry);
    vbox.append(&scroll);
    window.set_child(Some(&vbox));

    window.present();
}

fn main() -> io::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .init();
    match gtk::init() {
        Ok(result) => debug!("GTK initialized successfully: {:?}", result),
        Err(err) => debug!("Failed to initialize GTK: {}", err),
    }
    let application = gtk::Application::new(Some(APP_ID), Default::default());
    if let Some(existing_window) = application.windows().into_iter().next() {
        existing_window.present(); // Muestra y enfoca la ventana existente
        return Ok(()); // Salimos, no creamos una nueva ventana
    }
    let config = Arc::new(Mutex::new(
        Config::load().expect("Can not load config file"),
    ));
    {
        let mut config_guard = config.lock().unwrap(); // 🔓
        for extension in config_guard.extensions.iter_mut() {
            extension.finder.init();
        }
    } //🔓
    let config_clone = Arc::clone(&config);
    debug!("Config loaded");
    let history = match SqliteHistory::open() {
        Ok(store) => Some(Rc::new(store)),
        Err(err) => {
            tracing::warn!("No se pudo abrir el historial de uso, se continúa sin él: {err}");
            None
        }
    };
    let history_activate = history.clone();
    application.connect_activate(move |app| {
        debug!("Application activated");
        activate(app, &config_clone, history_activate.clone());
    });
    application.connect_startup(|_app| {
        let provider = Css::load().expect("Can not load CSS file");
        gtk::style_context_add_provider_for_display(
            &Display::default().expect("Could not connect to a display."),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    });
    application.connect_window_removed(|app, _window| {
        if app.windows().is_empty() {
            // Si quieres que la aplicación se cierre al cerrar la última ventana visible,
            // y has usado `app.hold()`, aquí necesitarías llamar a `app.release()` para contrarrestar el `hold`.
            // Si NO usas `app.hold()`, el comportamiento por defecto de GTK es que la aplicación termine cuando no haya más ventanas.
        }
    });
    debug!("Startup completed");
    application.run();
    debug!("Application exited");
    Ok(())
}
