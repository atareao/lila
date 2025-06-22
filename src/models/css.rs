use directories::ProjectDirs;
use gtk::CssProvider;
use std::fs;
use tracing::{debug, error};

pub struct Css {}

impl Css {
    pub fn load() -> Result<CssProvider, Box<dyn std::error::Error>> {
        let provider = CssProvider::new();
        if let Some(proj_dirs) = ProjectDirs::from("es", "atareao", "lila") {
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
                match fs::write(config_dir, Css::default().to_string()) {
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
