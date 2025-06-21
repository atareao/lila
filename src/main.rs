// SPDX-License-Identifier: MIT
//
// Copyright (c) 2025 Lorenzo Carbonell
//
// This file is part of the LiLa project,
// and is licensed under the MIT License. See the LICENSE file for details.
//

mod config;
mod constants;
mod css;

use gdk::Display;
use gtk::prelude::*;
use gtk::{CssProvider, Label, ListBox, ScrolledWindow};
use gtk4_layer_shell::{Edge, Layer, LayerShell};
use tracing::debug;
use tracing_subscriber;

use constants::*;

// https://github.com/wmww/gtk-layer-shell/blob/master/examples/simple-example.c
fn activate(application: &gtk::Application) {
    // Create a normal GTK window however you like
    let window = gtk::ApplicationWindow::new(application);
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

    // The margins are the gaps around the window's edges
    // Margins and anchors can be set like this...
    let margin = 100;
    window.set_margin(Edge::Left, margin);
    window.set_margin(Edge::Right, margin);
    window.set_margin(Edge::Top, margin);
    window.set_margin(Edge::Bottom, margin);

    // ... or like this
    // Anchors are if the window is pinned to each edge of the output
    let anchors = [
        (Edge::Left, false),
        (Edge::Right, false),
        (Edge::Top, false),
        (Edge::Bottom, false),
    ];

    for (anchor, state) in anchors {
        window.set_anchor(anchor, state);
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

    let listbox = ListBox::builder().name(LISTBOX_NAME).build();
    scroll.set_child(Some(&listbox));
    scroll.set_can_focus(false);
    listbox.set_visible(false);
    listbox.set_can_focus(false);

    let key_controller = gtk::EventControllerKey::new();
    key_controller.connect_key_pressed(move |_, keyval, _keycode, _state| {
        println!("Tecla pulsada: {:?}", keyval.name());
        // Por ejemplo, detectar Enter
        if keyval == gdk::Key::Escape {
            println!("Se pulsó Escape");
            window_clone.set_visible(false);
            true.into()
        } else if keyval == gdk::Key::Return {
            println!("Se pulsó Enter");
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
        let filtered_options: Vec<&str> = options
            .iter()
            .filter(|option| option.to_lowercase().contains(&text))
            .copied()
            .collect();

        for option in filtered_options.as_slice() {
            let label = Label::new(Some(option));
            listbox.append(&label);
        }

        debug!("Number of items: {}", filtered_options.len());

        // Mostrar/ocultar según haya coincidencias
        listbox.set_visible(!filtered_options.is_empty() && !e.text().is_empty());
    });

    vbox.append(&entry);
    vbox.append(&scroll);
    window.set_child(Some(&vbox));

    window.present();
}

fn main() {
    tracing_subscriber::fmt().with_env_filter("debug").init();
    match gtk::init() {
        Ok(result) => debug!("GTK initialized successfully: {:?}", result),
        Err(err) => debug!("Failed to initialize GTK: {}", err),
    }
    let config = config::Config::load().expect("Can not load config file");

    let application = gtk::Application::new(Some(APP_ID), Default::default());

    application.connect_activate(|app| {
        activate(app);
    });
    application.connect_startup(|_| {
        let provider = css::Css::load().expect("Can not load CSS file");
        gtk::style_context_add_provider_for_display(
            &Display::default().expect("Could not connect to a display."),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    });

    application.run();
}
