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
use models::{Config, Css, IdCache, Item, MetadataCache};
use std::cell::{Cell, OnceCell, RefCell};
use std::io;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tracing::debug;
use ui::error::ErrorBanner;
use ui::navigation::{Move, scroll_value, target_index};
use utils::*;

/// Margen tras presentar en el que una pérdida de foco no oculta la ventana.
///
/// El compositor puede no conceder el foco al mapear la superficie; durante
/// este margen la pérdida se aplaza y se re-verifica al terminar.
const PRESENT_FOCUS_GRACE: Duration = Duration::from_millis(250);

/// Acción que el compositor dispara sobre el dashboard ya en ejecución.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShortcutAction {
    /// Alterna la visibilidad: presenta si está oculto, oculta si está visible.
    Toggle,
    /// Presenta siempre, conservando el texto si ya estaba visible.
    Present,
}

/// Efecto ya decidido de una acción sobre el dashboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Resolution {
    /// Re-presenta la ventana; con `preserve_text` conserva el texto del `Entry`.
    Present { preserve_text: bool },
    /// Oculta la ventana sin terminar el proceso.
    Hide,
}

/// Acciones exportadas por el bus de sesión, en el orden de registro.
const SHORTCUT_ACTIONS: [(&str, ShortcutAction); 2] = [
    (TOGGLE_ACTION, ShortcutAction::Toggle),
    (PRESENT_ACTION, ShortcutAction::Present),
];

/// Traduce `(acción, visibilidad)` al efecto sobre el dashboard.
///
/// Lógica pura, sin GTK ni display: la acción solo aplica el resultado sobre los
/// widgets, de modo que la decisión se puede cubrir con tests unitarios.
fn next_action(visible: bool, action: ShortcutAction) -> Resolution {
    match (action, visible) {
        (ShortcutAction::Toggle, true) => Resolution::Hide,
        (ShortcutAction::Toggle, false) | (ShortcutAction::Present, false) => Resolution::Present {
            preserve_text: false,
        },
        (ShortcutAction::Present, true) => Resolution::Present {
            preserve_text: true,
        },
    }
}

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

/// Widgets y estado transitorio del dashboard ya construido.
///
/// GTK construye la ventana una sola vez; las invocaciones posteriores solo la
/// re-presentan. La ventana, el banner y el `Entry` viven aquí para poder
/// re-presentarlos y limpiarlos sin reconstruir la UI.
struct LaunchWidgets {
    window: Option<gtk::ApplicationWindow>,
    entry: Option<gtk::Entry>,
    label: Option<Label>,
    banner: ErrorBanner,
    /// Generación del margen anti-pérdida-de-foco; `0` = margen desarmado.
    focus_loss_grace: Rc<Cell<u64>>,
}

impl LaunchWidgets {
    fn new() -> Self {
        Self {
            window: None,
            entry: None,
            label: None,
            banner: ErrorBanner::new(),
            focus_loss_grace: Rc::new(Cell::new(0)),
        }
    }
}

/// Handle clonable al estado transitorio del dashboard.
#[derive(Clone)]
struct LaunchUi(Rc<RefCell<LaunchWidgets>>);

impl LaunchUi {
    fn new() -> Self {
        Self(Rc::new(RefCell::new(LaunchWidgets::new())))
    }

    /// Guarda las referencias a la ventana y a sus widgets transitorios.
    fn bind(&self, window: &gtk::ApplicationWindow, entry: &gtk::Entry, label: &Label) {
        {
            let mut widgets = self.0.borrow_mut();
            widgets.window = Some(window.clone());
            widgets.entry = Some(entry.clone());
            widgets.label = Some(label.clone());
        }
        self.sync_label();
    }

    /// Ventana del dashboard, si ya se construyó.
    fn window(&self) -> Option<gtk::ApplicationWindow> {
        self.0.borrow().window.clone()
    }

    /// `Entry` de búsqueda, si la ventana ya se construyó.
    fn entry(&self) -> Option<gtk::Entry> {
        self.0.borrow().entry.clone()
    }

    /// Muestra el motivo de un fallo de lanzamiento.
    fn show_error(&self, message: impl Into<String>) {
        self.0.borrow_mut().banner.show(message);
        self.sync_label();
    }

    /// Descarta el error del intento anterior.
    ///
    /// Si el banner ya estaba oculto no se toca el `Label`: el refresco de cada
    /// pulsación no debe pagar dos `set` sobre GTK.
    fn clear_error(&self) {
        let was_visible = {
            let mut widgets = self.0.borrow_mut();
            let was_visible = widgets.banner.is_visible();
            widgets.banner.clear();
            was_visible
        };
        if was_visible {
            self.sync_label();
        }
    }

    /// Refleja el banner en su `Label` sin retener préstamos del estado.
    ///
    /// El préstamo se suelta antes de tocar el widget: mutar GTK mientras el
    /// `RefCell` sigue prestado puede reentrar en este estado desde una señal.
    fn sync_label(&self) {
        let (label, visible, text) = {
            let widgets = self.0.borrow();
            (
                widgets.label.clone(),
                widgets.banner.is_visible(),
                widgets.banner.text().to_string(),
            )
        };
        if let Some(label) = label {
            ui::error::apply(&label, visible, &text);
        }
    }

    /// Arma el margen temporal en el que una pérdida de foco no oculta la
    /// ventana.
    ///
    /// Cada armado estrena generación: el temporizador de una presentación
    /// anterior ve que ya no es el vigente y no desarma el margen nuevo, así
    /// que presentaciones seguidas no lo truncan ni acumulan limpiezas
    /// prematuras.
    fn arm_focus_loss_grace(&self) {
        let grace = self.focus_loss_grace();
        let armed = grace.get().wrapping_add(1);
        grace.set(armed);
        gtk::glib::timeout_add_local_once(PRESENT_FOCUS_GRACE, move || {
            // Solo desarma el margen el temporizador de la última generación.
            if grace.get() == armed {
                grace.set(0);
            }
        });
    }

    /// Generación del margen; `0` significa margen desarmado.
    fn focus_loss_grace(&self) -> Rc<Cell<u64>> {
        Rc::clone(&self.0.borrow().focus_loss_grace)
    }

    /// Re-presenta la ventana existente, con foco y estado transitorio limpio.
    ///
    /// Si la ventana ya estaba visible se conserva lo que el usuario tenía
    /// escrito; solo se parte de cero al re-presentar una ventana oculta.
    fn present_existing(&self) {
        self.present_window(self.is_window_visible());
    }

    /// Aplica una acción del compositor al dashboard vivo.
    ///
    /// La decisión vive en [`next_action`]; aquí solo se ejecuta sobre los
    /// widgets, reutilizando la misma ruta de re-presentación que la invocación
    /// por binario (foco, margen y limpieza idénticos).
    fn dispatch(&self, action: ShortcutAction) {
        match next_action(self.is_window_visible(), action) {
            Resolution::Present { preserve_text } => self.present_window(preserve_text),
            Resolution::Hide => self.hide_existing(),
        }
    }

    /// `true` si la ventana existe y su flag `visible` está activo.
    ///
    /// No implica que ya esté mapeada: el compositor puede tardar en mapear la
    /// superficie, y por eso la presentación arma el margen de foco. Devuelve
    /// `false` mientras la ventana no se haya construido.
    fn is_window_visible(&self) -> bool {
        self.window().is_some_and(|window| window.is_visible())
    }

    /// Re-presenta la ventana con foco; `preserve_text` conserva el `Entry`.
    ///
    /// `present` va antes que la limpieza: así el refresco que dispara vaciar
    /// el `Entry` corre con la ventana ya mapeada y el scroll se recoloca de
    /// forma determinista.
    fn present_window(&self, preserve_text: bool) {
        let Some(window) = self.window() else {
            return;
        };
        // El margen se arma antes de presentar para cubrir también el `map`.
        self.arm_focus_loss_grace();
        window.present(); // Muestra y enfoca la ventana existente
        let entry = self.entry();
        if let Some(entry) = &entry {
            entry.grab_focus();
        }
        if !preserve_text {
            self.clear_error();
            if let Some(entry) = &entry {
                entry.set_text("");
            }
        }
    }

    /// Oculta la ventana sin terminar el proceso.
    ///
    /// Usa la misma ruta que el auto-ocultado por pérdida de foco
    /// (`connect_close_request` solo la esconde), así que el proceso queda vivo y
    /// listo para la siguiente invocación. No re-verifica el margen de foco: el
    /// margen solo aplica a la presentación.
    fn hide_existing(&self) {
        if let Some(window) = self.window() {
            window.close();
        }
    }
}

fn activate(
    application: &gtk::Application,
    mutex_config: &Arc<Mutex<Config>>,
    history: Option<Rc<SqliteHistory>>,
    ui: &LaunchUi,
) {
    // Una segunda activación no construye otra ventana: re-presenta la viva.
    // Basta con `is_some()`: la ventana vive toda la sesión porque
    // `connect_close_request` solo la oculta y nunca la destruye, de modo que
    // un `Some` aquí no puede apuntar a un widget ya destruido.
    if ui.window().is_some() {
        ui.present_existing();
        return; // Salimos, no creamos una nueva ventana
    }
    let config;
    {
        config = mutex_config.lock().unwrap();
    }
    // Caché de metadatos (`Keywords`/`Categories`) por ruta: la búsqueda los
    // consulta para todos los ítems en cada pulsación y sin caché cada uno
    // reabriría su `.desktop`. Vive tanto como la ventana.
    let meta = Rc::new(MetadataCache::default());
    let apps: Vec<Item> = config
        .extensions
        .iter()
        .find(|e| e.name == "application-finder")
        .or_else(|| config.extensions.first())
        .map(|e| e.finder.search("", &meta))
        .unwrap_or_default();
    let apps = Rc::new(apps);
    // Caché de ids canónicos: `Item::id()` canonicaliza su `.desktop` (E/S) y el
    // ranking lo consulta para todos los ítems en cada pulsación. Vive tanto
    // como la ventana.
    let id_cache = Rc::new(IdCache::default());
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
    let focus_loss_grace = ui.focus_loss_grace();
    window.connect_notify_local(Some("is-active"), move |win, _| {
        debug!("{:?}", win.is_active());
        if win.is_active() {
            return;
        }
        if focus_loss_grace.get() != 0 {
            // El margen solo aplaza el auto-ocultado: una pérdida de foco real
            // dentro de él se re-verifica al terminar, no se descarta.
            debug!("Pérdida de foco dentro del margen de presentación: se re-verifica");
            let win = win.clone();
            gtk::glib::timeout_add_local_once(PRESENT_FOCUS_GRACE, move || {
                if !win.is_active() {
                    win.close();
                }
            });
            return;
        }
        win.close();
    });
    // Crear el controller para detectar teclas
    let window_clone = window.clone();

    // Entry donde el usuario escribe
    let entry = gtk::Entry::new();

    // Mensaje de error de lanzamiento. Nace oculto: solo se muestra cuando un
    // intento de lanzar falla, y se limpia al reintentar la búsqueda. Es lo que
    // mantiene informado al usuario mientras el dashboard sigue visible.
    let error_label = Label::new(None);
    error_label.add_css_class("launch-error");
    error_label.set_visible(false);
    error_label.set_wrap(true);
    error_label.set_xalign(0.0);
    // La ventana ya existe: el estado transitorio pasa a ser accesible desde
    // el camino de re-presentación.
    ui.bind(&window, &entry, &error_label);

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

    // `select_row` no desplaza el `ScrolledWindow`: la `GtkListBox` solo cambia
    // la selección. Mantenemos aquí la fila seleccionada a la vista, tanto al
    // navegar con el teclado como al filtrar (que reselecciona la primera fila).
    let scroll_sel = scroll.clone();
    listbox.connect_row_selected(move |listbox, row| {
        let Some(row) = row else {
            return;
        };
        let Some(point) = row.compute_point(listbox, &gtk::graphene::Point::new(0.0, 0.0)) else {
            return;
        };
        let top = f64::from(point.y());
        let bottom = top + f64::from(row.height());
        let adj = scroll_sel.vadjustment();
        let new = scroll_value(top, bottom, adj.value(), adj.page_size());
        if (new - adj.value()).abs() > f64::EPSILON {
            adj.set_value(new);
        }
    });

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
    let ui_launch = ui.clone();
    entry.connect_activate(move |_| {
        let idx = listbox_launch
            .selected_row()
            .map(|r| r.index())
            .unwrap_or(0);
        if let Some(item) = current_launch.borrow().get(idx as usize).cloned() {
            match launch(&item) {
                Ok(()) => {
                    window_launch.set_visible(false);
                    ui_launch.clear_error();
                    record_launch(history_launch.as_deref(), history_enabled, &item);
                }
                Err(err) => {
                    // La ventana NO se oculta: el dashboard permanece visible
                    // con el motivo del fallo para que el usuario lo vea.
                    tracing::error!("No se pudo lanzar la aplicación: {err}");
                    ui_launch.show_error(err.to_string());
                }
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
        let ids = Rc::clone(&id_cache);
        let meta = Rc::clone(&meta);
        let ranking_config = Rc::clone(&ranking_config);
        let ui_refresh = ui.clone();
        Rc::new(move || {
            // Reintentar la búsqueda descarta el error del intento anterior.
            ui_refresh.clear_error();
            let text = entry.text().to_string();
            let history: Option<&dyn HistoryRepository> = history
                .as_ref()
                .map(|store| store.as_ref() as &dyn HistoryRepository);
            let now_millis = now_millis();
            let results = rank_items(
                &apps,
                &ids,
                &meta,
                &text,
                history,
                &ranking_config,
                now_millis,
            );
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
    vbox.append(&error_label);
    vbox.append(&scroll);
    window.set_child(Some(&vbox));

    // Render inicial: consulta vacía con el historial disponible.
    refresh();

    // Mismo margen que en la re-presentación: si el compositor tarda en dar
    // foco al mapear, la ventana no debe auto-ocultarse.
    ui.arm_focus_loss_grace();
    window.present();
    entry.grab_focus();
}

/// Estado de la instancia primaria, creado en `startup`.
struct LaunchState {
    /// Configuración compartida con el closure de activación (finders ya
    /// inicializados por `startup`).
    config: Arc<Mutex<Config>>,
    /// Historial de uso; `None` si la base de datos no pudo abrirse.
    history: Option<Rc<SqliteHistory>>,
    /// Estado transitorio del dashboard (ventana, banner, `Entry`).
    ui: LaunchUi,
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
    // `startup` solo se emite en la instancia primaria de GApplication. Es el
    // punto seguro para cargar la config, inicializar los finders y abrir la
    // base de datos: una segunda invocación se registra como remota y no hace
    // nada de este trabajo pesado.
    let launcher: Rc<OnceCell<LaunchState>> = Rc::new(OnceCell::new());
    let startup_launcher = Rc::clone(&launcher);
    application.connect_startup(move |app| {
        let provider = Css::load();
        gtk::style_context_add_provider_for_display(
            &Display::default().expect("Could not connect to a display."),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        let config = Arc::new(Mutex::new(
            Config::load().expect("Can not load config file"),
        ));
        {
            let mut config_guard = config.lock().unwrap(); // 🔓
            for extension in config_guard.extensions.iter_mut() {
                extension.finder.init();
            }
        } //🔓
        debug!("Config loaded");
        let history = match SqliteHistory::open() {
            Ok(store) => Some(Rc::new(store)),
            Err(err) => {
                tracing::warn!("No se pudo abrir el historial de uso, se continúa sin él: {err}");
                None
            }
        };
        let ui = LaunchUi::new();
        let _ = startup_launcher.set(LaunchState {
            config,
            history,
            ui: ui.clone(),
        });
        // Las acciones se registran una sola vez, en la instancia primaria.
        // `GApplication` exporta el action map por `org.gtk.Actions` y
        // `org.freedesktop.Application` en el object path derivado de `APP_ID`
        // (`/es/atareao/lila`), de modo que el compositor puede invocarlas sin
        // arrancar un proceso nuevo.
        for (name, action) in SHORTCUT_ACTIONS {
            let ui_for_action = ui.clone();
            let simple_action = gio::SimpleAction::new(name, None);
            simple_action.connect_activate(move |_, _| ui_for_action.dispatch(action));
            app.add_action(&simple_action);
        }
    });
    application.connect_activate(move |app| {
        debug!("Application activated");
        // En la instancia primaria `startup` siempre precede a `activate`.
        let Some(state) = launcher.get() else {
            // Imposible salvo error de programación: sin `startup` no hay
            // dashboard que construir. Se cierra la aplicación en vez de dejar
            // un proceso vivo sin ventana.
            tracing::error!("Activación sin estado inicializado: se cierra la aplicación");
            app.quit();
            return;
        };
        activate(app, &state.config, state.history.clone(), &state.ui);
    });
    debug!("Startup completed");
    application.run();
    debug!("Application exited");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shortcut_actions_should_export_toggle_and_present_names() {
        let names: Vec<&str> = SHORTCUT_ACTIONS.iter().map(|(name, _)| *name).collect();
        assert_eq!(names, ["toggle", "present"]);
    }

    #[test]
    fn toggle_should_present_without_text_when_dashboard_hidden() {
        assert_eq!(
            next_action(false, ShortcutAction::Toggle),
            Resolution::Present {
                preserve_text: false
            }
        );
    }

    #[test]
    fn toggle_should_hide_when_dashboard_visible() {
        assert_eq!(next_action(true, ShortcutAction::Toggle), Resolution::Hide);
    }

    #[test]
    fn present_should_preserve_text_when_dashboard_visible() {
        assert_eq!(
            next_action(true, ShortcutAction::Present),
            Resolution::Present {
                preserve_text: true
            }
        );
    }

    #[test]
    fn present_should_present_without_text_when_dashboard_hidden() {
        assert_eq!(
            next_action(false, ShortcutAction::Present),
            Resolution::Present {
                preserve_text: false
            }
        );
    }
}
