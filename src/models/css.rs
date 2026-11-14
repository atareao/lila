use super::super::utils::*;
use directories::ProjectDirs;
use gtk::CssProvider;
use std::fs;
use tracing::{debug, error};

pub struct Css {}

impl Css {
    pub fn load() -> Result<CssProvider, Box<dyn std::error::Error>> {
        let provider = CssProvider::new();
        if let Some(proj_dirs) = ProjectDirs::from(APP_QUALIFIER, APP_ORGANIZATION, APP_APPLICATION)
        {
            let mut config_dir = proj_dirs.config_dir().to_path_buf();
            debug!("config dir: {:?}", config_dir);
            if !config_dir.exists() {
                std::fs::create_dir_all(&config_dir)?;
            }
            config_dir.push("style.css");
            if config_dir.exists() {
                provider.load_from_path(config_dir);
                Ok(provider)
            } else {
                match fs::write(config_dir, Css::default()) {
                    Ok(_) => debug!("Saved"),
                    Err(e) => error!("Error al escribir el archivo: {}", e),
                }
                provider.load_from_string(&Css::default());
                Ok(provider)
            }
        } else {
            provider.load_from_string(&Css::default());
            Ok(provider)
        }
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
    use super::Css;
    use crate::utils::LISTBOX_NAME;

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
}
