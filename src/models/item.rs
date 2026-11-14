use applications::App;
use gio::DesktopAppInfo;
use gio::prelude::AppInfoExt;
use std::path::Path;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Item {
    App(App),
}

impl Item {
    pub fn name(&self) -> &str {
        match self {
            Item::App(app) => &app.name,
        }
    }

    /// ID estable de la aplicación: ruta canónica del fichero `.desktop`.
    /// Si la canonicalización falla, cae al path tal cual como String.
    pub fn id(&self) -> String {
        match self {
            Item::App(app) => app
                .app_desktop_path
                .canonicalize()
                .map(|canonical| canonical.to_string_lossy().into_owned())
                .unwrap_or_else(|_| app.app_desktop_path.to_string_lossy().into_owned()),
        }
    }

    /// Ruta del icono de la aplicación (si existe)
    // TODO(core-lanzamiento): se consumirá al cablear la UI/historial (tareas 3.x/4.x). Retirar este allow entonces.
    #[allow(dead_code)]
    pub fn icon_path(&self) -> Option<&Path> {
        match self {
            Item::App(app) => app.icon_path.as_deref(),
        }
    }

    /// Ruta del ejecutable de la aplicación (si existe)
    // TODO(core-lanzamiento): se consumirá al cablear la UI/historial (tareas 3.x/4.x). Retirar este allow entonces.
    #[allow(dead_code)]
    pub fn exec(&self) -> Option<&Path> {
        match self {
            Item::App(app) => app.app_path_exe.as_deref(),
        }
    }

    /// Subtítulo derivado de `GenericName` o, en su defecto, `Comment`.
    /// Devuelve `None` sin entrar en pánico si el `.desktop` no se puede cargar.
    // TODO(core-lanzamiento): se consumirá al cablear la UI/historial (tareas 3.x/4.x). Retirar este allow entonces.
    #[allow(dead_code)]
    pub fn subtitle(&self) -> Option<String> {
        match self {
            Item::App(app) => {
                let info = DesktopAppInfo::from_filename(&app.app_desktop_path)?;
                info.generic_name()
                    .or_else(|| info.description())
                    .map(String::from)
            }
        }
    }

    /// Palabras clave derivadas del campo `Keywords` del `.desktop`.
    /// Devuelve un vector vacío si no se puede cargar.
    // TODO(core-lanzamiento): se consumirá al cablear la UI/historial (tareas 3.x/4.x). Retirar este allow entonces.
    #[allow(dead_code)]
    pub fn keywords(&self) -> Vec<String> {
        match self {
            Item::App(app) => DesktopAppInfo::from_filename(&app.app_desktop_path)
                .map(|info| {
                    info.keywords()
                        .into_iter()
                        .map(String::from)
                        .collect::<Vec<String>>()
                })
                .unwrap_or_default(),
        }
    }

    /// Categorías derivadas del campo `Categories` del `.desktop`.
    /// Devuelve un vector vacío si no se puede cargar.
    // TODO(core-lanzamiento): se consumirá al cablear la UI/historial (tareas 3.x/4.x). Retirar este allow entonces.
    #[allow(dead_code)]
    pub fn categories(&self) -> Vec<String> {
        match self {
            Item::App(app) => DesktopAppInfo::from_filename(&app.app_desktop_path)
                .and_then(|info| info.categories())
                .map(|categories| split_desktop_list(&categories))
                .unwrap_or_default(),
        }
    }
}

/// Divide una lista estilo `.desktop` (`a;b;c;`) en elementos no vacíos
// TODO(core-lanzamiento): se consumirá al cablear la UI/historial (tareas 3.x/4.x). Retirar este allow entonces.
#[allow(dead_code)]
fn split_desktop_list(value: &str) -> Vec<String> {
    value
        .split(';')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(String::from)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    /// GLib descarta un `.desktop` cuyo binario de `Exec` no exista:
    /// `g_desktop_app_info_load_from_keyfile()` hace `goto error` cuando
    /// `g_find_program_for_path()` no resuelve el ejecutable, y entonces
    /// `DesktopAppInfo::from_filename()` devuelve `None`. Por eso las
    /// fixtures apuntan a un ejecutable real del sistema.
    const EXEC_BINARY: &str = "/usr/bin/env";

    /// [TDD-RED-01] Un `App` con un `.desktop` real devuelve un `id()` canónico
    #[test]
    fn id_returns_canonical_path() {
        let tmp = TempDir::new().expect("TempDir::new()");
        let desktop_path = tmp.path().join("test.desktop");
        fs::write(&desktop_path, "[Desktop Entry]\nName=Test\n").expect("write");

        let app = App {
            name: String::from("TestApp"),
            icon_path: Some(tmp.path().to_path_buf()),
            app_path_exe: Some(PathBuf::from("/usr/bin/test_binary")),
            app_desktop_path: desktop_path.clone(),
        };
        let item = Item::App(app);

        let canonical = desktop_path.canonicalize().expect("canonicalize");
        assert_eq!(item.id(), canonical.to_string_lossy());
    }

    /// [TDD-RED-01b] `id()` cae al path literal cuando no se puede canonicalizar
    #[test]
    fn id_falls_back_to_literal_path_when_missing() {
        let tmp = TempDir::new().expect("TempDir::new()");
        let missing = tmp.path().join("missing.desktop");

        let app = App {
            name: String::from("MissingApp"),
            icon_path: None,
            app_path_exe: None,
            app_desktop_path: missing.clone(),
        };
        let item = Item::App(app);

        assert_eq!(item.id(), missing.to_string_lossy());
    }

    /// [TDD-RED-02] `icon_path()` y `exec()` devuelven lo esperado con datos conocidos
    #[test]
    fn icon_path_and_exec_return_expected_values() {
        let tmp = TempDir::new().expect("TempDir::new()");
        let icon = tmp.path().join("icon.png");
        let binary = tmp.path().join("binary");

        let app = App {
            name: String::from("IconExecApp"),
            icon_path: Some(icon.clone()),
            app_path_exe: Some(binary.clone()),
            app_desktop_path: tmp.path().join("app.desktop"),
        };
        let item = Item::App(app);

        assert_eq!(item.icon_path(), Some(icon.as_path()));
        assert_eq!(item.exec(), Some(binary.as_path()));
    }

    /// [TDD-RED-02b] `icon_path()`/`exec()` devuelven `None` cuando no hay datos
    #[test]
    fn icon_path_and_exec_are_none_when_absent() {
        let tmp = TempDir::new().expect("TempDir::new()");
        let app = App {
            name: String::from("NoPathsApp"),
            icon_path: None,
            app_path_exe: None,
            app_desktop_path: tmp.path().join("app.desktop"),
        };
        let item = Item::App(app);

        assert_eq!(item.icon_path(), None);
        assert_eq!(item.exec(), None);
    }

    /// [TDD-RED-03] `subtitle()` devuelve `None` sin pánico cuando el `.desktop` no existe
    #[test]
    fn subtitle_returns_none_for_missing_desktop_file() {
        let app = App {
            name: String::from("MissingDesktopApp"),
            icon_path: None,
            app_path_exe: None,
            app_desktop_path: PathBuf::from("/nonexistent/path/to/app.desktop"),
        };
        let item = Item::App(app);

        assert_eq!(item.subtitle(), None);
        assert!(item.keywords().is_empty());
        assert!(item.categories().is_empty());
    }

    /// [TDD-RED-04] `subtitle()` prefiere `GenericName` sobre `Comment`
    #[test]
    fn subtitle_prefers_generic_name_from_desktop_entry() {
        let tmp = TempDir::new().expect("TempDir::new()");
        let desktop_path = tmp.path().join("app.desktop");
        fs::write(
            &desktop_path,
            format!(
                "\
[Desktop Entry]
Type=Application
Name=App With Subtitle
GenericName=Subtitle from GenericName
Comment=Comment instead
Exec={EXEC_BINARY}
"
            ),
        )
        .expect("write");

        let app = App {
            name: String::from("App"),
            icon_path: None,
            app_path_exe: Some(PathBuf::from(EXEC_BINARY)),
            app_desktop_path: desktop_path,
        };
        let item = Item::App(app);

        assert_eq!(
            item.subtitle(),
            Some(String::from("Subtitle from GenericName"))
        );
    }

    /// [TDD-RED-05] `keywords()` y `categories()` devuelven los vectores esperados
    #[test]
    fn keywords_and_categories_return_expected_vectors() {
        let tmp = TempDir::new().expect("TempDir::new()");
        let desktop_path = tmp.path().join("app.desktop");
        fs::write(
            &desktop_path,
            format!(
                "\
[Desktop Entry]
Type=Application
Name=App With Keywords
Comment=Description
Exec={EXEC_BINARY}
Keywords=photos;image;editor;
Categories=Utility;Editor;
"
            ),
        )
        .expect("write");

        let app = App {
            name: String::from("App"),
            icon_path: None,
            app_path_exe: Some(PathBuf::from(EXEC_BINARY)),
            app_desktop_path: desktop_path,
        };
        let item = Item::App(app);

        assert_eq!(
            item.keywords(),
            vec![
                String::from("photos"),
                String::from("image"),
                String::from("editor")
            ]
        );
        assert_eq!(
            item.categories(),
            vec![String::from("Utility"), String::from("Editor")]
        );
    }
}
