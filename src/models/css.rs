use super::super::utils::*;
use directories::ProjectDirs;
use gtk::CssProvider;
use std::fs;
use std::path::Path;
use tracing::{debug, warn};

/// Nombre del fichero de estilos dentro del directorio de configuración.
const STYLESHEET_FILE: &str = "style.css";

pub struct Css {}

/// Resultado de resolver la hoja de estilos: el CSS a aplicar y los avisos
/// (problemas degradados) que deben registrarse.
#[derive(Debug)]
pub(crate) struct ResolvedStylesheet {
    pub css: String,
    pub warnings: Vec<String>,
}

/// Resuelve el CSS a aplicar desde `config_dir`. **Nunca falla**: ante
/// cualquier problema devuelve el CSS por defecto y acumula un aviso.
///
/// Política de resolución, en orden de precedencia:
/// 1. Sin directorio de configuración, solo cabe el CSS por defecto.
/// 2. Si `<config_dir>/style.css` es legible, manda el fichero del usuario: se
///    usa su contenido y no se escribe nada (el fichero es del usuario).
/// 3. Si existe pero no es legible, se usa el CSS por defecto en memoria y se
///    avisa: nunca se sobrescribe ni se borra lo que el usuario tenga ahí.
/// 4. Si no existe, se intenta materializar el CSS por defecto en el
///    directorio (creándolo si hace falta). Si la escritura falla, se aplica
///    igualmente el CSS por defecto desde memoria y se avisa.
pub(crate) fn resolve_stylesheet(config_dir: Option<&Path>) -> ResolvedStylesheet {
    let Some(config_dir) = config_dir else {
        return ResolvedStylesheet {
            css: Css::default(),
            warnings: Vec::new(),
        };
    };

    let path = config_dir.join(STYLESHEET_FILE);
    match fs::read_to_string(&path) {
        Ok(css) => {
            debug!("config dir: {}", config_dir.display());
            return ResolvedStylesheet {
                css,
                warnings: Vec::new(),
            };
        }
        Err(e) if path.exists() => {
            return ResolvedStylesheet {
                css: Css::default(),
                warnings: vec![format!(
                    "no se pudo leer {}: {e}; se usa el CSS por defecto",
                    path.display()
                )],
            };
        }
        Err(_) => {}
    }

    // El fichero no existe: se intenta guardar el CSS por defecto para que el
    // usuario tenga un punto de partida editable.
    let default_css = Css::default();
    let mut warnings = Vec::new();
    if let Err(e) = fs::create_dir_all(config_dir).and_then(|()| fs::write(&path, &default_css)) {
        warnings.push(format!(
            "no se pudo escribir {}: {e}; se usa el CSS por defecto en memoria",
            path.display()
        ));
    } else {
        debug!("Saved");
    }
    ResolvedStylesheet {
        css: default_css,
        warnings,
    }
}

impl Css {
    /// Carga la hoja de estilos aplicable a la aplicación.
    ///
    /// No falla nunca: si el directorio de configuración no existe, no se puede
    /// crear o no es escribible, se aplica el CSS por defecto desde memoria y se
    /// registra un aviso. Un problema de estilos no debe abortar el arranque.
    pub fn load() -> CssProvider {
        let resolved = resolve_stylesheet(
            ProjectDirs::from(APP_QUALIFIER, APP_ORGANIZATION, APP_APPLICATION)
                .map(|proj_dirs| proj_dirs.config_dir().to_path_buf())
                .as_deref(),
        );
        for warning in &resolved.warnings {
            warn!("{warning}");
        }
        let provider = CssProvider::new();
        provider.load_from_string(&resolved.css);
        provider
    }
    fn default() -> String {
        "
            .transparente {
                background-color: rgba(0, 0, 0, 0); /* RGBA con alfa 0 (completamente transparente) */
                background-image: none; /* Asegura que no haya imagen de fondo */
            }
            #lila-listbox {
                border-radius: 10px;
            }
            #lila-listbox row {
                padding: 8px 12px;
                border-radius: 6px;
            }
            .result-name {
                font-weight: bold;
            }
            .result-subtitle {
                opacity: 0.6;
                font-size: smaller;
            }
            .launch-error {
                /* Rojo claro: legible sobre fondo oscuro y sin cambiar el
                   tamaño del layout (solo aparece cuando hay error). */
                color: #ff6b6b;
                font-size: smaller;
                padding: 0 4px;
            }
            #lila-listbox row:selected {
                background-color: rgba(128, 128, 128, 0.25);
                /* Un tema GTK puede pintar un gradiente en la fila seleccionada;
                   anular la imagen de fondo garantiza que el resaltado se vea. */
                background-image: none;
            }
            .overlay {
                -gtk-render-background: false; /* Importante para que GTK no dibuje el fondo */
                /* Otras propiedades que pueden ser útiles para un overlay: */
                /* Establece que las áreas transparentes no deben interceptar eventos de ratón */
                background-clip: padding-box; /* Asegura que el fondo solo se aplique al padding box */
            }
            .mi-texto-bonito {
                color: white; /* Para que el texto sea visible en un fondo transparente */
                font-size: 24px;
                background-color: rgba(0, 0, 0, 0.5); /* Un fondo semitransparente para el texto */
                padding: 10px;
                border-radius: 5px;
            }
        ".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::{Css, resolve_stylesheet};
    use crate::utils::LISTBOX_NAME;
    use std::fs;
    use tempfile::TempDir;

    /// Extrae el bloque de declaraciones que sigue al primer `selector` encontrado.
    fn rule_block(css: &str, selector: &str) -> String {
        let start = match css.find(selector) {
            Some(pos) => pos,
            None => panic!("el CSS por defecto no define la regla '{selector}'"),
        };
        let rest = &css[start..];
        let end = rest.find('}').unwrap_or(rest.len());
        rest[..end].to_string()
    }

    #[test]
    fn default_css_styles_result_name() {
        let css = Css::default();
        let block = rule_block(&css, ".result-name");
        assert!(
            block.contains("font-weight"),
            "la regla .result-name debería definir font-weight: {block}"
        );
    }

    #[test]
    fn default_css_attenuates_result_subtitle() {
        let css = Css::default();
        let block = rule_block(&css, ".result-subtitle");
        assert!(
            block.contains("opacity"),
            "la regla .result-subtitle debería atenuarse con opacity: {block}"
        );
    }

    #[test]
    fn default_css_styles_launch_error() {
        let css = Css::default();
        let block = rule_block(&css, ".launch-error");
        assert!(
            block.contains("color"),
            "la regla .launch-error debería definir un color legible: {block}"
        );
    }

    #[test]
    fn default_css_gives_rows_padding() {
        let css = Css::default();
        let block = rule_block(&css, &format!("#{LISTBOX_NAME} row"));
        assert!(
            block.contains("padding"),
            "las filas del listbox deberían tener padding: {block}"
        );
    }

    #[test]
    fn default_css_highlights_selected_row() {
        let css = Css::default();
        let block = rule_block(&css, &format!("#{LISTBOX_NAME} row:selected"));
        assert!(
            block.contains("background"),
            "la fila seleccionada debería resaltarse con un background: {block}"
        );
    }

    #[test]
    fn default_css_selected_row_ignores_theme_background_image() {
        let css = Css::default();
        let block = rule_block(&css, &format!("#{LISTBOX_NAME} row:selected"));
        assert!(
            block.contains("background-image: none"),
            "la fila seleccionada debería anular la imagen de fondo del tema \
             (un gradiente enmascararía el resaltado): {block}"
        );
    }

    #[test]
    fn resolve_uses_user_stylesheet_when_readable() {
        let dir = TempDir::new().expect("no se pudo crear el directorio temporal");
        let user_css = ".result-name { color: red; }";
        fs::write(dir.path().join("style.css"), user_css).expect("no se pudo escribir el CSS");

        let resolved = resolve_stylesheet(Some(dir.path()));

        assert_eq!(resolved.css, user_css);
        assert!(
            resolved.warnings.is_empty(),
            "no debería haber avisos con un CSS legible: {:?}",
            resolved.warnings
        );
    }

    #[test]
    fn resolve_does_not_overwrite_readable_user_stylesheet() {
        let dir = TempDir::new().expect("no se pudo crear el directorio temporal");
        let path = dir.path().join("style.css");
        let user_css = ".result-name { color: red; }";
        fs::write(&path, user_css).expect("no se pudo escribir el CSS");

        let resolved = resolve_stylesheet(Some(dir.path()));

        assert_eq!(resolved.css, user_css);
        let after = fs::read_to_string(&path).expect("el fichero debería seguir siendo legible");
        assert_eq!(
            after, user_css,
            "un CSS legible del usuario nunca debe sobrescribirse"
        );
    }

    #[test]
    fn resolve_falls_back_when_user_stylesheet_is_unreadable() {
        let dir = TempDir::new().expect("no se pudo crear el directorio temporal");
        let path = dir.path().join("style.css");
        // Un directorio con ese nombre hace fallar la lectura incluso como root.
        fs::create_dir(&path).expect("no se pudo crear el directorio style.css");

        let resolved = resolve_stylesheet(Some(dir.path()));

        assert_eq!(resolved.css, Css::default());
        assert!(
            !resolved.warnings.is_empty(),
            "un CSS ilegible debería acumular un aviso"
        );
        assert!(
            path.is_dir(),
            "el CSS ilegible del usuario no debe borrarse ni sobrescribirse"
        );
    }

    #[test]
    fn resolve_writes_default_stylesheet_when_missing() {
        let dir = TempDir::new().expect("no se pudo crear el directorio temporal");
        let path = dir.path().join("style.css");

        let resolved = resolve_stylesheet(Some(dir.path()));

        assert_eq!(resolved.css, Css::default());
        assert!(
            resolved.warnings.is_empty(),
            "crear el CSS por defecto no debería avisar: {:?}",
            resolved.warnings
        );
        let written = fs::read_to_string(&path).expect("debería haberse escrito style.css");
        assert_eq!(written, Css::default());
    }

    #[test]
    fn resolve_falls_back_in_memory_when_directory_is_not_writable() {
        let dir = TempDir::new().expect("no se pudo crear el directorio temporal");
        // El padre de `config_dir` es un fichero regular: `create_dir_all` y la
        // escritura fallan incluso ejecutando como root.
        let blocker = dir.path().join("afile");
        fs::write(&blocker, "no soy un directorio").expect("no se pudo escribir el fichero");

        let resolved = resolve_stylesheet(Some(&blocker));

        assert_eq!(resolved.css, Css::default());
        assert!(
            !resolved.warnings.is_empty(),
            "un directorio no escribible debería acumular un aviso"
        );
    }

    #[test]
    fn resolve_falls_back_without_config_dir() {
        let resolved = resolve_stylesheet(None);

        assert_eq!(resolved.css, Css::default());
        assert!(
            resolved.warnings.is_empty(),
            "sin directorio de configuración no hay nada que avisar: {:?}",
            resolved.warnings
        );
    }
}
