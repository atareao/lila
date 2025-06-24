// SPDX-License-Identifier: MIT
//
// Copyright (c) 2025 Lorenzo Carbonell
//
// This file is part of the LiLa project,
// and is licensed under the MIT License. See the LICENSE file for details.
//

mod actions;
mod finders;
mod models;
mod utils;

use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use gdk::Display;
use gtk::prelude::*;
use gtk::{Label, ListBox, ScrolledWindow};
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use models::{Config, Css};
use std::io;
use std::sync::{Arc, Mutex};
use tracing::debug;
use tracing_subscriber;
use utils::*;

// https://github.com/wmww/gtk-layer-shell/blob/master/examples/simple-example.c
fn activate(application: &gtk::Application, mutex_config: &Arc<Mutex<Config>>) {
    if let Some(existing_window) = application.windows().into_iter().next() {
        println!("La aplicación ya está ejecutándose, trayendo la ventana existente al frente.");
        existing_window.present(); // Muestra y enfoca la ventana existente
        return; // Salimos, no creamos una nueva ventana
    }
    let config;
    {
        config = mutex_config.lock().unwrap();
    }
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
        println!("Señal 'close-request' de la ventana recibida. Ocultando la ventana en lugar de cerrarla.");
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
        println!("{:?}", win);
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
    let triggers = vec!["g", "s"];

    // Lista de opciones para autocompletar
    let options = vec![
        "Rust",
        "Ruby",
        "Python",
        "Perl",
        "PHP",
        "Java",
        "JavaScript",
    ];
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

    let key_controller = gtk::EventControllerKey::new();
    key_controller.connect_key_pressed(move |_, keyval, _keycode, _state| {
        debug!("Tecla pulsada: {:?}", keyval.name());
        // Por ejemplo, detectar Enter
        if keyval == gdk::Key::Escape {
            debug!("Se pulsó Escape");
            window_clone.set_visible(false);
            debug!("Window hidden");
            true.into()
        } else if keyval == gdk::Key::Return {
            debug!("Se pulsó Enter");
            true.into()
        } else {
            false.into()
        }
    });
    entry.add_controller(key_controller);
    let entry_clone_for_activate = entry.clone();
    let listbox_clone = listbox.clone();
    entry.connect_activate(move |_| {
        let entered_text = entry_clone_for_activate.text();
        println!("Texto introducido (señal activate): \"{}\"", entered_text);
        if let Some(first_row) = listbox_clone.row_at_index(0) {
            if let Some(widget) = first_row.child() {
                if let Ok(label) = widget.downcast::<Label>() {
                    debug!("Selected text: {}", label.text());
                    entry_clone_for_activate.set_text(&label.text());
                    entry_clone_for_activate.set_position(-1);
                }
            }
        }
        // Opcional: borrar el Entry después de procesar
        // entry_clone.set_text("");
    });

    let entry_clone_for_listbox = entry.clone();
    listbox.connect_selected_rows_changed(move |d| {
        if let Some(selected_row) = d.selected_row() {
            if let Some(widget) = selected_row.child() {
                if let Ok(label) = widget.downcast::<Label>() {
                    debug!("Selected text: {}", label.text());
                    entry_clone_for_listbox.set_text(&label.text());
                    entry_clone_for_listbox.set_position(-1);
                }
            }
        }
    });
    // Filtrado manual al escribir
    entry.connect_changed(move |e| {
        let text = e.text().to_string().to_lowercase();
        if text.contains(" ") {
            let trigger = text.split_whitespace().collect::<Vec<&str>>()[0];
            if triggers.contains(&trigger) {
                debug!("trigger: {:?}", trigger);
                return;
            }
        }
        debug!("text: {}", text);

        // Limpiar el modelo
        listbox.remove_all();

        // Agregar opciones filtradas
        let matcher = SkimMatcherV2::default();

        let mut filtered_and_scored_options: Vec<(&str, i64)> = options
            .iter()
            .filter_map(|&option| {
                let result = matcher.fuzzy_match(option, &text);
                debug!("option: {:?}, result: {:?}", option, result);

                result.map(|score| (option, score)) // If result is Some(score), transform to (option, score)
            })
            .collect(); // Collect into a Vec of (option, score) tuples
        filtered_and_scored_options.sort_by(|a, b| b.1.cmp(&a.1));
        let result = filtered_and_scored_options
            .into_iter() // Use into_iter to consume the vector and avoid an extra copy
            .map(|(option, _score)| option)
            .collect::<Vec<&str>>();

        for option in result.as_slice() {
            let label = Label::new(Some(option));
            listbox.append(&label);
        }

        debug!("Number of items: {}", result.len());

        // Mostrar/ocultar según haya coincidencias
        listbox.set_visible(!result.is_empty() && !e.text().is_empty());
    });

    vbox.append(&entry);
    vbox.append(&scroll);
    window.set_child(Some(&vbox));

    window.present();
}

fn main() -> io::Result<()> {
    tracing_subscriber::fmt().with_env_filter("debug").init();
    match gtk::init() {
        Ok(result) => debug!("GTK initialized successfully: {:?}", result),
        Err(err) => debug!("Failed to initialize GTK: {}", err),
    }
    let application = gtk::Application::new(Some(APP_ID), Default::default());
    if let Some(existing_window) = application.windows().into_iter().next() {
        println!("La aplicación ya está ejecutándose, trayendo la ventana existente al frente.");
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
    application.connect_activate(move |app| {
        debug!("Application activated");
        activate(app, &config_clone);
    });
    application.connect_startup(|app| {
        let provider = Css::load().expect("Can not load CSS file");
        gtk::style_context_add_provider_for_display(
            &Display::default().expect("Could not connect to a display."),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    });
    application.connect_window_removed(|app, _window| {
            if app.windows().is_empty() {
                 println!("Todas las ventanas han sido cerradas/ocultadas. La aplicación permanece en segundo plano debido a `app.hold()`.");
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
