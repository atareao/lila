// SPDX-License-Identifier: MIT
//
// Copyright (c) 2025 Lorenzo Carbonell
//
// This file is part of the LiLa project,
// and is licensed under the MIT License. See the LICENSE file for details.
//

//! Estado del banner de error de lanzamiento.
//!
//! El estado (`ErrorBanner`) es independiente de GTK y se prueba sin display.
//! Solo la función [`apply`] toca el widget: copia la visibilidad y el texto al
//! `Label` que pinta el banner.

use gtk::prelude::*;

/// Banner que informa del fallo del último intento de lanzamiento.
///
/// Nace oculto y sin texto. `show` lo hace visible con el motivo del fallo y
/// `clear` lo devuelve al estado inicial, de modo que re-presentar la ventana
/// tras un error no arrastre mensajes obsoletos. Un mensaje vacío nunca deja el
/// banner visible: no habría nada que contar.
pub struct ErrorBanner {
    message: Option<String>,
}

impl ErrorBanner {
    /// Crea un banner oculto y sin texto.
    #[must_use]
    pub fn new() -> Self {
        Self { message: None }
    }

    /// Muestra `message` como error visible; un mensaje vacío lo oculta.
    pub fn show(&mut self, message: impl Into<String>) {
        self.message = Some(message.into()).filter(|message| !message.is_empty());
    }

    /// Devuelve el banner a su estado inicial: oculto y sin texto.
    pub fn clear(&mut self) {
        self.message = None;
    }

    /// Texto del error actualmente mostrado (o vacío si no hay error).
    #[must_use]
    pub fn text(&self) -> &str {
        self.message.as_deref().unwrap_or_default()
    }

    /// Indica si el banner debe mostrarse.
    #[must_use]
    pub fn is_visible(&self) -> bool {
        self.message.is_some()
    }
}

impl Default for ErrorBanner {
    fn default() -> Self {
        Self::new()
    }
}

/// Refleja el estado del banner en el `Label` de la UI.
///
/// Capa imperativa sin test: GTK exige un display. La lógica testeable vive en
/// `ErrorBanner`; esta función solo copia `visible` y `text` al widget. Recibe
/// el estado ya desacoplado para que quien la llama no mantenga préstamos del
/// `RefCell` del dashboard mientras muta widgets GTK.
pub fn apply(label: &gtk::Label, visible: bool, text: &str) {
    label.set_visible(visible);
    label.set_text(text);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_should_start_hidden_and_empty() {
        let banner = ErrorBanner::new();
        assert!(!banner.is_visible());
        assert_eq!(banner.text(), "");
    }

    #[test]
    fn show_should_make_banner_visible_with_message() {
        let mut banner = ErrorBanner::new();
        banner.show("no se pudo lanzar");
        assert!(banner.is_visible());
        assert_eq!(banner.text(), "no se pudo lanzar");
    }

    #[test]
    fn clear_after_show_should_hide_banner_and_empty_text() {
        let mut banner = ErrorBanner::new();
        banner.show("no se pudo lanzar");
        banner.clear();
        assert!(!banner.is_visible());
        assert_eq!(banner.text(), "");
    }

    #[test]
    fn show_should_replace_previous_message() {
        let mut banner = ErrorBanner::new();
        banner.show("primer fallo");
        banner.show("segundo fallo");
        assert!(banner.is_visible());
        assert_eq!(banner.text(), "segundo fallo");
    }

    #[test]
    fn repeated_clear_should_keep_banner_hidden_and_empty() {
        let mut banner = ErrorBanner::new();
        banner.clear();
        banner.clear();
        assert!(!banner.is_visible());
        assert_eq!(banner.text(), "");
    }

    #[test]
    fn show_empty_message_should_keep_banner_hidden() {
        let mut banner = ErrorBanner::new();
        banner.show("");
        assert!(!banner.is_visible());
        assert_eq!(banner.text(), "");
    }
}
