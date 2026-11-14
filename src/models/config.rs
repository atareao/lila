use super::super::finders::{ApplicationFinder, Finder};
use super::super::utils::*;
use super::Extension;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use tracing::{debug, error};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Edge {
    #[serde(default)]
    pub margin: i32,
    #[serde(default)]
    pub anchor: bool,
}
impl Default for Edge {
    fn default() -> Self {
        Edge {
            margin: 100,
            anchor: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub width: i32,
    #[serde(default)]
    pub height: i32,
    #[serde(default)]
    pub left: Edge,
    #[serde(default)]
    pub right: Edge,
    #[serde(default)]
    pub top: Edge,
    #[serde(default)]
    pub bottom: Edge,
    #[serde(default)]
    pub extensions: Vec<Extension>,
    /// Peso de la coincidencia difusa en el ranking.
    #[serde(default = "default_fuzzy_weight")]
    pub fuzzy_weight: f64,
    /// Peso de la frecuencia de lanzamientos en el ranking.
    #[serde(default = "default_frequency_weight")]
    pub frequency_weight: f64,
    /// Peso de la recencia del último uso en el ranking.
    #[serde(default = "default_recency_weight")]
    pub recency_weight: f64,
    /// Número máximo de resultados mostrados.
    #[serde(default = "default_max_results")]
    pub max_results: usize,
    /// Si es `false` no se registran lanzamientos y el ranking usa solo el difuso.
    #[serde(default = "default_history_enabled")]
    pub history_enabled: bool,
}

/// Peso por defecto de la coincidencia difusa.
fn default_fuzzy_weight() -> f64 {
    1.0
}

/// Peso por defecto de la frecuencia de lanzamientos.
fn default_frequency_weight() -> f64 {
    0.5
}

/// Peso por defecto de la recencia del último uso.
fn default_recency_weight() -> f64 {
    0.5
}

/// Límite por defecto de resultados mostrados.
fn default_max_results() -> usize {
    50
}

/// El historial está activado por defecto.
fn default_history_enabled() -> bool {
    true
}

impl Config {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        if let Some(proj_dirs) = ProjectDirs::from(APP_QUALIFIER, APP_ORGANIZATION, APP_APPLICATION)
        {
            let mut config_dir = proj_dirs.config_dir().to_path_buf();
            debug!("config dir: {:?}", config_dir);
            if !config_dir.exists() {
                std::fs::create_dir_all(&config_dir)?;
            }
            config_dir.push("config.yml");
            if config_dir.exists() {
                let file = std::fs::File::open(config_dir)?;
                let config: Config = serde_yaml::from_reader(file)?;
                Ok(config)
            } else {
                match serde_yaml::to_string(&Config::default()) {
                    Ok(yaml_string) => match fs::write(config_dir, yaml_string) {
                        Ok(_) => debug!("Config file created"),
                        Err(e) => error!("Error creating config file: {}", e),
                    },
                    Err(e) => error!("Error serializing default config: {}", e),
                }
                Err("Failed to create config file".into())
            }
        } else {
            Ok(Config::default())
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        let extensions = vec![Extension::new(
            "application-finder".to_string(),
            None,
            Some("An extension to launch applications".to_string()),
            Finder::ApplicationFinder(ApplicationFinder::new()),
            Some(1800),
        )];
        Config {
            width: 800,
            height: 600,
            left: Edge::default(),
            right: Edge::default(),
            top: Edge::default(),
            bottom: Edge::default(),
            extensions,
            fuzzy_weight: default_fuzzy_weight(),
            frequency_weight: default_frequency_weight(),
            recency_weight: default_recency_weight(),
            max_results: default_max_results(),
            history_enabled: default_history_enabled(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// [TDD-RED-B2-01] Un `config.yml` antiguo, sin los campos nuevos, aplica
    /// los valores por defecto sin fallar.
    #[test]
    fn legacy_config_without_new_fields_uses_defaults() {
        let yaml = "width: 640\nheight: 480\n";
        let config: Config = serde_yaml::from_str(yaml).expect("debería deserializar");

        assert_eq!(config.fuzzy_weight, 1.0);
        assert_eq!(config.frequency_weight, 0.5);
        assert_eq!(config.recency_weight, 0.5);
        assert_eq!(config.max_results, 50);
        assert!(config.history_enabled);
    }

    /// [TDD-RED-B2-02] Los valores explícitos del YAML se respetan.
    #[test]
    fn config_with_new_fields_respects_explicit_values() {
        let yaml = "\
fuzzy_weight: 2.5
frequency_weight: 0.1
recency_weight: 0.9
max_results: 7
history_enabled: false
";
        let config: Config = serde_yaml::from_str(yaml).expect("debería deserializar");

        assert_eq!(config.fuzzy_weight, 2.5);
        assert_eq!(config.frequency_weight, 0.1);
        assert_eq!(config.recency_weight, 0.9);
        assert_eq!(config.max_results, 7);
        assert!(!config.history_enabled);
    }
}
