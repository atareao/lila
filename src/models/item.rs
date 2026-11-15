use std::cell::RefCell;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use applications::App;
use gio::DesktopAppInfo;
use gio::prelude::AppInfoExt;

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

    /// Ruta del fichero `.desktop` asociado al ítem.
    ///
    /// Es la base de [`Item::id`] y la clave con la que [`IdCache`] memoriza la
    /// canonicalización.
    pub(crate) fn desktop_path(&self) -> &Path {
        match self {
            Item::App(app) => &app.app_desktop_path,
        }
    }

    /// ID estable de la aplicación: ruta canónica del fichero `.desktop`.
    /// Si la canonicalización falla, cae al path tal cual como String.
    pub fn id(&self) -> String {
        let path = self.desktop_path();
        canonical_path(path).unwrap_or_else(|| path.to_string_lossy().into_owned())
    }

    /// Ruta del icono de la aplicación (si existe)
    pub fn icon_path(&self) -> Option<&Path> {
        match self {
            Item::App(app) => app.icon_path.as_deref(),
        }
    }

    /// Subtítulo derivado de `GenericName` o, en su defecto, `Comment`.
    /// Devuelve `None` sin entrar en pánico si el `.desktop` no se puede cargar.
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
    ///
    /// Solo la usan los tests, como oráculo **sin caché** con el que contrastar
    /// [`MetadataCache`]: la ruta de producción lee los metadatos a través de la
    /// caché.
    #[cfg(test)]
    pub fn keywords(&self) -> Vec<String> {
        match self {
            Item::App(app) => DesktopAppInfo::from_filename(&app.app_desktop_path)
                .map(|info| keywords_of(&info))
                .unwrap_or_default(),
        }
    }

    /// Categorías derivadas del campo `Categories` del `.desktop`.
    /// Devuelve un vector vacío si no se puede cargar.
    ///
    /// Solo la usan los tests, como oráculo **sin caché** con el que contrastar
    /// [`MetadataCache`].
    #[cfg(test)]
    pub fn categories(&self) -> Vec<String> {
        match self {
            Item::App(app) => DesktopAppInfo::from_filename(&app.app_desktop_path)
                .map(|info| categories_of(&info))
                .unwrap_or_default(),
        }
    }
}

/// Palabras clave declaradas por un `.desktop` ya cargado (`Keywords`).
///
/// Es la extracción que comparten [`Item::keywords`] y [`MetadataCache`], para
/// que la versión memorizada no pueda divergir de la directa.
fn keywords_of(info: &DesktopAppInfo) -> Vec<String> {
    info.keywords().into_iter().map(String::from).collect()
}

/// Categorías declaradas por un `.desktop` ya cargado (`Categories`).
///
/// Es la extracción que comparten [`Item::categories`] y [`MetadataCache`].
fn categories_of(info: &DesktopAppInfo) -> Vec<String> {
    info.categories()
        .map(|categories| split_desktop_list(&categories))
        .unwrap_or_default()
}

/// Divide una lista estilo `.desktop` (`a;b;c;`) en elementos no vacíos
fn split_desktop_list(value: &str) -> Vec<String> {
    value
        .split(';')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(String::from)
        .collect()
}

/// Canonicaliza `path` como id estable, o `None` si no se puede resolver.
///
/// Es la única canonicalización del modelo: [`Item::id`] y [`IdCache`] la
/// comparten para no divergir en el formato del id.
fn canonical_path(path: &Path) -> Option<String> {
    path.canonicalize()
        .ok()
        .map(|canonical| canonical.to_string_lossy().into_owned())
}

/// Caché de ids canónicos de `.desktop` por ruta.
///
/// [`Item::id`] canonicaliza su ruta en cada llamada, y eso es una syscall de
/// E/S: el ranking consulta el id de *todos* los ítems en *cada* pulsación, así
/// que sin caché la misma ruta se canonicalizaría decenas de veces por
/// búsqueda. Esta caché resuelve cada ruta una sola vez por sesión y reutiliza
/// el resultado entre búsquedas.
///
/// Solo memoriza resoluciones correctas: si la canonicalización falla, cada
/// consulta lo reintenta y aplica el respaldo de [`Item::id`] (el path
/// literal), de modo que un fichero que aparezca más tarde se resuelve igual
/// que sin caché.
#[derive(Debug, Default)]
pub struct IdCache {
    canonical: RefCell<HashMap<PathBuf, String>>,
}

impl IdCache {
    /// Id estable de `item`, reutilizando la canonicalización ya resuelta.
    ///
    /// Equivale a [`Item::id`] pero sin repetir la E/S cuando la ruta ya se
    /// resolvió antes.
    pub fn id(&self, item: &Item) -> String {
        let path = item.desktop_path();
        if let Some(cached) = self.canonical.borrow().get(path) {
            return cached.clone();
        }
        let Some(canonical) = canonical_path(path) else {
            return path.to_string_lossy().into_owned();
        };
        self.canonical
            .borrow_mut()
            .insert(path.to_path_buf(), canonical.clone());
        canonical
    }
}

/// Metadatos de un `.desktop` que participan en la búsqueda.
#[derive(Debug, Clone)]
struct Metadata {
    keywords: Vec<String>,
    categories: Vec<String>,
}

/// Caché de palabras clave y categorías de `.desktop` por ruta.
///
/// [`Item::keywords`] y [`Item::categories`] abren el `.desktop` con
/// `DesktopAppInfo` en cada llamada, y eso es E/S: el ranking consulta ambos
/// metadatos de *todos* los ítems en *cada* pulsación, así que sin caché la
/// misma ruta se leería decenas de veces por búsqueda. Esta caché resuelve cada
/// ruta una vez por sesión —con una sola lectura, que cubre las dos claves— y
/// reutiliza el resultado entre búsquedas.
///
/// Comparte la política de [`IdCache`]: memoriza toda resolución correcta,
/// incluido el vector vacío de una app que legítimamente no declara metadatos.
/// Lo único que no memoriza es un fallo de resolución (`.desktop` inexistente o
/// ilegible): en ese caso devuelve el vector vacío **sin reabrir el fichero que
/// acaba de fallar**, de modo que la siguiente consulta reintenta la lectura
/// una sola vez y una app instalada más tarde se resuelve sin reiniciar.
#[derive(Debug, Default)]
pub struct MetadataCache {
    resolved: RefCell<HashMap<PathBuf, Metadata>>,
}

impl MetadataCache {
    /// Palabras clave de `item`, reutilizando las ya resueltas para su ruta.
    ///
    /// Equivale a [`Item::keywords`] pero sin repetir la E/S cuando la ruta ya
    /// se resolvió antes. Si la ruta no se pudo resolver devuelve vacío, sin
    /// reabrir el `.desktop` que acaba de fallar.
    pub fn keywords(&self, item: &Item) -> Vec<String> {
        let path = item.desktop_path();
        self.ensure_resolved(path);
        self.resolved
            .borrow()
            .get(path)
            .map(|metadata| metadata.keywords.clone())
            .unwrap_or_default()
    }

    /// Categorías de `item`, reutilizando las ya resueltas para su ruta.
    ///
    /// Equivale a [`Item::categories`] pero sin repetir la E/S cuando la ruta
    /// ya se resolvió antes. Si la ruta no se pudo resolver devuelve vacío, sin
    /// reabrir el `.desktop` que acaba de fallar.
    pub fn categories(&self, item: &Item) -> Vec<String> {
        let path = item.desktop_path();
        self.ensure_resolved(path);
        self.resolved
            .borrow()
            .get(path)
            .map(|metadata| metadata.categories.clone())
            .unwrap_or_default()
    }

    /// Resuelve y memoriza los metadatos de `path` si aún no lo están.
    ///
    /// Una sola lectura cubre las dos claves. Si el `.desktop` no se puede
    /// cargar no se memoriza nada: la siguiente consulta lo reintenta y, entre
    /// tanto, [`MetadataCache::keywords`] y [`MetadataCache::categories`]
    /// devuelven el vector vacío sin volver a tocar el disco.
    fn ensure_resolved(&self, path: &Path) {
        if self.resolved.borrow().contains_key(path) {
            return;
        }
        let Some(info) = DesktopAppInfo::from_filename(path) else {
            return;
        };
        let metadata = Metadata {
            keywords: keywords_of(&info),
            categories: categories_of(&info),
        };
        self.resolved
            .borrow_mut()
            .insert(path.to_path_buf(), metadata);
    }
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

    /// `Item::App` con el nombre y la ruta de `.desktop` indicados.
    fn item_at(desktop_path: &Path, name: &str) -> Item {
        Item::App(App {
            name: name.to_string(),
            icon_path: None,
            app_path_exe: None,
            app_desktop_path: desktop_path.to_path_buf(),
        })
    }

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

    /// [TDD-RED-02] `icon_path()` devuelve lo esperado con datos conocidos
    #[test]
    fn icon_path_returns_expected_values() {
        let tmp = TempDir::new().expect("TempDir::new()");
        let icon = tmp.path().join("icon.png");

        let app = App {
            name: String::from("IconExecApp"),
            icon_path: Some(icon.clone()),
            app_path_exe: None,
            app_desktop_path: tmp.path().join("app.desktop"),
        };
        let item = Item::App(app);

        assert_eq!(item.icon_path(), Some(icon.as_path()));
    }

    /// [TDD-RED-02b] `icon_path()` devuelve `None` cuando no hay datos
    #[test]
    fn icon_path_is_none_when_absent() {
        let tmp = TempDir::new().expect("TempDir::new()");
        let app = App {
            name: String::from("NoPathsApp"),
            icon_path: None,
            app_path_exe: None,
            app_desktop_path: tmp.path().join("app.desktop"),
        };
        let item = Item::App(app);

        assert_eq!(item.icon_path(), None);
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

    /// [TDD-RED-CACHE-05] `IdCache` devuelve el mismo id que `Item::id()`
    #[test]
    fn id_cache_matches_item_id() {
        let tmp = TempDir::new().expect("TempDir::new()");
        let desktop_path = tmp.path().join("cached.desktop");
        fs::write(&desktop_path, "[Desktop Entry]\nName=Cached\n").expect("write");
        let item = item_at(&desktop_path, "Cached");

        let cache = IdCache::default();

        assert_eq!(cache.id(&item), item.id());
        assert_eq!(cache.id(&item), item.id());
    }

    /// [TDD-RED-CACHE-06] `IdCache` memoriza cada ruta: una vez resuelta, el id
    /// no vuelve a canonicalizarse aunque el fichero desaparezca
    #[test]
    fn id_cache_memoizes_the_canonical_path() {
        let tmp = TempDir::new().expect("TempDir::new()");
        let target = tmp.path().join("memoized.desktop");
        fs::write(&target, "[Desktop Entry]\nName=Memoized\n").expect("write");
        // El enlace hace que el path literal y el canónico difieran.
        let link = tmp.path().join("linked.desktop");
        std::os::unix::fs::symlink(&target, &link).expect("symlink");
        let item = item_at(&link, "Memoized");
        let canonical = target
            .canonicalize()
            .expect("canonicalize")
            .to_string_lossy()
            .into_owned();

        let cache = IdCache::default();
        assert_eq!(cache.id(&item), canonical);

        fs::remove_file(&target).expect("remove");
        assert_ne!(item.id(), canonical, "fixture: el enlace ya está colgado");

        assert_eq!(cache.id(&item), canonical);
    }

    /// [TDD-RED-CACHE-07] Un fallo de canonicalización no se memoriza: si el
    /// fichero aparece después, la siguiente consulta ya lo resuelve
    #[test]
    fn id_cache_retries_failed_canonicalization() {
        let tmp = TempDir::new().expect("TempDir::new()");
        let desktop_path = tmp.path().join("late.desktop");
        let item = item_at(&desktop_path, "Late");

        let cache = IdCache::default();
        assert_eq!(cache.id(&item), desktop_path.to_string_lossy());

        fs::write(&desktop_path, "[Desktop Entry]\nName=Late\n").expect("write");
        let canonical = desktop_path
            .canonicalize()
            .expect("canonicalize")
            .to_string_lossy()
            .into_owned();

        assert_eq!(cache.id(&item), canonical);
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

    /// Escribe un `.desktop` real con los metadatos indicados.
    ///
    /// `Exec` apunta a un binario existente: GLib descarta los `.desktop` cuyo
    /// ejecutable no resuelve (ver [`EXEC_BINARY`]).
    fn write_desktop(path: &Path, name: &str, keywords: Option<&str>, categories: Option<&str>) {
        let mut body =
            format!("[Desktop Entry]\nType=Application\nName={name}\nExec={EXEC_BINARY}\n");
        if let Some(keywords) = keywords {
            body.push_str(&format!("Keywords={keywords}\n"));
        }
        if let Some(categories) = categories {
            body.push_str(&format!("Categories={categories}\n"));
        }
        fs::write(path, body).expect("write");
    }

    /// [TDD-RED-A2-01] `MetadataCache` devuelve las palabras clave y categorías
    /// del `.desktop`
    #[test]
    fn metadata_cache_returns_keywords_and_categories() {
        let tmp = TempDir::new().expect("TempDir::new()");
        let desktop_path = tmp.path().join("meta.desktop");
        write_desktop(
            &desktop_path,
            "Meta",
            Some("photos;image;editor;"),
            Some("Utility;Editor;"),
        );
        let item = item_at(&desktop_path, "Meta");

        let cache = MetadataCache::default();

        assert_eq!(
            cache.keywords(&item),
            vec![
                String::from("photos"),
                String::from("image"),
                String::from("editor")
            ]
        );
        assert_eq!(
            cache.categories(&item),
            vec![String::from("Utility"), String::from("Editor")]
        );
    }

    /// [TDD-RED-A2-02] `MetadataCache` memoriza cada ruta: una vez resuelta, el
    /// `.desktop` no se vuelve a leer aunque cambie
    #[test]
    fn metadata_cache_memoizes_the_resolved_metadata() {
        let tmp = TempDir::new().expect("TempDir::new()");
        let desktop_path = tmp.path().join("memoized.desktop");
        write_desktop(&desktop_path, "Memoized", Some("photos;"), None);
        let item = item_at(&desktop_path, "Memoized");

        let cache = MetadataCache::default();
        assert_eq!(cache.keywords(&item), vec![String::from("photos")]);

        write_desktop(&desktop_path, "Memoized", Some("photos;image;"), None);
        assert_eq!(
            item.keywords(),
            vec![String::from("photos"), String::from("image")],
            "fixture: el fichero ya declara dos palabras clave"
        );
        assert_eq!(
            cache.keywords(&item),
            vec![String::from("photos")],
            "la caché sirve el valor memorizado, sin releer el fichero"
        );
    }

    /// [TDD-RED-A2-03] Un vector vacío también se memoriza: la app que no declara
    /// metadatos no se relee en cada pulsación
    #[test]
    fn metadata_cache_memoizes_empty_metadata() {
        let tmp = TempDir::new().expect("TempDir::new()");
        let desktop_path = tmp.path().join("empty.desktop");
        write_desktop(&desktop_path, "Empty", None, None);
        let item = item_at(&desktop_path, "Empty");

        let cache = MetadataCache::default();
        assert!(cache.keywords(&item).is_empty());
        assert!(cache.categories(&item).is_empty());

        write_desktop(&desktop_path, "Empty", Some("zzz;"), None);
        assert_eq!(
            item.keywords(),
            vec![String::from("zzz")],
            "fixture: el fichero ya declara palabras clave"
        );
        assert!(
            cache.keywords(&item).is_empty(),
            "el éxito con vector vacío se memoriza"
        );
    }

    /// [TDD-RED-A2-04] Un fallo de lectura no se memoriza: si el `.desktop`
    /// aparece después, la siguiente consulta ya resuelve los metadatos
    #[test]
    fn metadata_cache_retries_failed_resolution() {
        let tmp = TempDir::new().expect("TempDir::new()");
        let desktop_path = tmp.path().join("late.desktop");
        let item = item_at(&desktop_path, "Late");

        let cache = MetadataCache::default();
        assert!(cache.keywords(&item).is_empty());
        assert!(cache.categories(&item).is_empty());

        write_desktop(&desktop_path, "Late", Some("photos;"), Some("Graphics;"));

        assert_eq!(cache.keywords(&item), vec![String::from("photos")]);
        assert_eq!(cache.categories(&item), vec![String::from("Graphics")]);
    }
}
