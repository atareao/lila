// SPDX-License-Identifier: MIT
//
// Copyright (c) 2025 Lorenzo Carbonell
//
// This file is part of the LiLa project,
// and is licensed under the MIT License. See the LICENSE file for details.
//

//! View-model puro de una fila de resultado.
//!
//! Se mantiene al margen de GTK para poder probarse sin display: la capa de UI
//! (`main.rs`) consume este modelo y construye los widgets a partir de él.

use crate::models::Item;
use std::path::{Path, PathBuf};

/// Datos ya resueltos de una fila de resultados.
///
/// Es un tipo fuerte y sin dependencias de GTK: la UI decide cómo pintarlo
/// (icono, nombre y subtítulo atenuado) pero la lógica de extracción es testeable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultRow {
    name: String,
    subtitle: Option<String>,
    icon_path: Option<PathBuf>,
}

impl ResultRow {
    /// Deriva la fila a partir de un `Item` del finder.
    pub fn from_item(item: &Item) -> Self {
        Self {
            name: item.name().to_string(),
            subtitle: item.subtitle(),
            icon_path: item.icon_path().map(Path::to_path_buf),
        }
    }

    /// Nombre visible de la aplicación.
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Subtítulo atenuado, si la aplicación aporta nombre genérico o descripción.
    pub fn subtitle(&self) -> Option<&str> {
        self.subtitle.as_deref()
    }

    /// Ruta del icono, si está disponible.
    pub fn icon_path(&self) -> Option<&Path> {
        self.icon_path.as_deref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Item;
    use applications::App;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    /// GLib descarta un `.desktop` cuyo binario de `Exec` no exista, así que las
    /// fixtures apuntan a un ejecutable real del sistema.
    const EXEC_BINARY: &str = "/usr/bin/env";

    /// Construye un `Item` con un `.desktop` real en `dir` a partir del cuerpo dado.
    fn item_with_desktop(dir: &TempDir, body: &str, icon_path: Option<PathBuf>) -> Item {
        let desktop_path = dir.path().join("app.desktop");
        fs::write(
            &desktop_path,
            format!(
                "\
[Desktop Entry]
Type=Application
{body}
Exec={EXEC_BINARY}
"
            ),
        )
        .expect("write");
        Item::App(App {
            name: String::from("Icon App"),
            icon_path,
            app_path_exe: Some(PathBuf::from(EXEC_BINARY)),
            app_desktop_path: desktop_path,
        })
    }

    /// [TDD-RED-4.1a] Con icono disponible la fila expone su ruta y, sin campos
    /// de contexto en el `.desktop`, no tiene subtítulo.
    #[test]
    fn from_item_keeps_icon_path_and_no_subtitle_without_context_fields() {
        let tmp = TempDir::new().expect("TempDir::new()");
        let icon_path = tmp.path().join("icon.png");
        let item = item_with_desktop(&tmp, "Name=Icon App", Some(icon_path.clone()));

        let row = ResultRow::from_item(&item);

        assert_eq!(row.icon_path(), Some(icon_path.as_path()));
        assert_eq!(row.name(), "Icon App");
        assert_eq!(row.subtitle(), None);
    }

    /// [TDD-RED-4.1b] Sin icono pero con `GenericName`, la fila conserva el
    /// nombre y expone el subtítulo derivado.
    #[test]
    fn from_item_without_icon_uses_generic_name_as_subtitle() {
        let tmp = TempDir::new().expect("TempDir::new()");
        let item = item_with_desktop(&tmp, "Name=Icon App\nGenericName=Generic Subtitle", None);

        let row = ResultRow::from_item(&item);

        assert_eq!(row.icon_path(), None);
        assert_eq!(row.name(), "Icon App");
        assert_eq!(row.subtitle(), Some("Generic Subtitle"));
    }

    /// [TDD-RED-4.1c] Sin `GenericName` ni `Comment` el subtítulo es `None` y
    /// no se produce ningún pánico.
    #[test]
    fn from_item_without_subtitle_fields_has_no_subtitle() {
        let tmp = TempDir::new().expect("TempDir::new()");
        let item = item_with_desktop(&tmp, "Name=Icon App", None);

        let row = ResultRow::from_item(&item);

        assert_eq!(row.subtitle(), None);
    }

    /// [TDD-RED-4.1d] El nombre de la fila coincide con el del `Item`.
    #[test]
    fn from_item_name_matches_item_name() {
        let tmp = TempDir::new().expect("TempDir::new()");
        let item = item_with_desktop(&tmp, "Name=Icon App", None);

        let row = ResultRow::from_item(&item);

        assert_eq!(row.name(), item.name());
    }
}
