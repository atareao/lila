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
mod ui;
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
use ui::navigation::{Move, target_index};
use utils::*;

// https://github.com/wmww/gtk-layer-shell/blob/master/examples/simple-example.c
/// Traduce una tecla de navegación de GTK a un movimiento de la selección.
///
/// Devuelve `None` para cualquier otra tecla, de modo que el manejador de
/// teclado pueda dejarla pasar sin consumirla. El controlador de teclado actúa
/// en fase *capture*, por lo que también devuelve `None` cuando hay
/// modificadores presentes: así `Ctrl+A`, `Ctrl+Z`, `Shift+End` y compañía
/// siguen llegando al editor de texto del `Entry`.
fn movement_of(keyval: gdk::Key, state: gdk::ModifierType) -> Option<Move> {
    let editing_modifiers = gdk::ModifierType::CONTROL_MASK
        | gdk::ModifierType::ALT_MASK
        | gdk::ModifierType::SHIFT_MASK;
    if state.intersects(editing_modifiers) {
        return None;
    }
    if keyval == gdk::Key::Down {
        Some(Move::Down)
    } else if keyval == gdk::Key::Up {
        Some(Move::Up)
    } else if keyval == gdk::Key::Page_Down {
        Some(Move::PageDown)
    } else if keyval == gdk::Key::Page_Up {
        Some(Move::PageUp)
    } else if keyval == gdk::Key::Home {
        Some(Move::Home)
    } else if keyval == gdk::Key::End {
        Some(Move::End)
    } else {
        None
    }
}

/// Cuenta las filas de `listbox` recorriendo sus hijos directos.
///
/// La `ListBox` no expone el número de filas en gtk4-rs 0.9, así que se
/// recorren los hijos y solo se cuentan los que son `ListBoxRow`.
fn row_count(listbox: &ListBox) -> usize {
    let mut count = 0;
    let mut child = listbox.first_child();
    while let Some(widget) = child {
        if widget.is::<gtk::ListBoxRow>() {
            count += 1;
        }
        child = widget.next_sibling();
    }
    count
}

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

/// Construye la fila de resultado: icono opcional, nombre y subtítulo atenuado.
///
/// Si la fila no tiene icono, se compone solo con los labels para no dejar un
/// hueco en la disposición.
fn build_result_row(row: &ui::row::ResultRow) -> gtk::ListBoxRow {
    let list_row = gtk::ListBoxRow::new();
    let hbox = gtk::Box::new(gtk::Orientation::Horizontal, ICON_TEXT_SPACING);
    hbox.set_valign(gtk::Align::Center);

    if let Some(icon_path) = row.icon_path() {
        let image = gtk::Image::from_file(icon_path);
        image.set_pixel_size(ICON_PIXEL_SIZE);
        hbox.append(&image);
    }

    let labels = gtk::Box::new(gtk::Orientation::Vertical, 0);
    labels.set_valign(gtk::Align::Center);
    let name = Label::new(Some(row.name()));
    name.set_xalign(0.0);
    name.add_css_class("result-name");
    labels.append(&name);
    if let Some(subtitle) = row.subtitle() {
        let subtitle = Label::new(Some(subtitle));
        subtitle.set_xalign(0.0);
        subtitle.add_css_class("result-subtitle");
        labels.append(&subtitle);
    }
    hbox.append(&labels);

    list_row.set_child(Some(&hbox));
    list_row
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
    key_controller.connect_key_pressed(move |_, keyval, _keycode, state| {
        debug!("Tecla pulsada: {:?}", keyval.name());
        // Por ejemplo, detectar Enter
        if keyval == gdk::Key::Escape {
            debug!("Se pulsó Escape");
            window_clone.set_visible(false);
            debug!("Window hidden");
            true.into()
        } else if let Some(mv) = movement_of(keyval, state) {
            // Los índices negativos no son posibles en una `ListBox`, pero se
            // acotan por si el widget los devolviera en algún estado transitorio.
            let current = listbox_sel
                .selected_row()
                .map(|r| r.index().max(0) as usize);
            let len = row_count(&listbox_sel);
            match target_index(current, len, mv) {
                Some(index) => {
                    if let Some(row) = listbox_sel.row_at_index(index as i32) {
                        listbox_sel.select_row(Some(&row));
                    }
                    true.into()
                }
                // Sin resultados la tecla sigue su curso normal (edición de texto).
                None => false.into(),
            }
        } else {
            false.into()
        }
    });
    // El `GtkText` interno del `Entry` registra Home/End como shortcuts de
    // clase en fase *bubble* y los consume antes de que este controlador los
    // vea; en fase *capture* los interceptamos primero.
    key_controller.set_propagation_phase(gtk::PropagationPhase::Capture);
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

    // Recalcula los resultados a partir del texto actual del entry. Se usa
    // tanto al escribir (filtrado en vivo) como en el render inicial, ya que
    // `gtk::Entry` no emite `changed` al construirse.
    let refresh: Rc<dyn Fn()> = {
        let apps = Rc::clone(&apps);
        let current = Rc::clone(&current);
        let listbox = listbox.clone();
        let entry = entry.clone();
        let history = history.clone();
        let ranking_config = Rc::clone(&ranking_config);
        Rc::new(move || {
            let text = entry.text().to_string();
            let history: Option<&dyn HistoryRepository> = history
                .as_ref()
                .map(|store| store.as_ref() as &dyn HistoryRepository);
            let now_millis = now_millis();
            let results = rank_items(&apps, &text, history, &ranking_config, now_millis);
            listbox.remove_all();
            for item in &results {
                let row = ui::row::ResultRow::from_item(item);
                listbox.append(&build_result_row(&row));
            }
            let hay = ui::results_visible(results.len());
            *current.borrow_mut() = results;
            listbox.set_visible(hay);
            if hay {
                listbox.select_row(listbox.row_at_index(0).as_ref());
            }
        })
    };

    // Filtrado en vivo al escribir
    let refresh_changed = Rc::clone(&refresh);
    entry.connect_changed(move |_| refresh_changed());

    vbox.append(&entry);
    vbox.append(&scroll);
    window.set_child(Some(&vbox));

    // Render inicial: consulta vacía con el historial disponible.
    refresh();

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
