use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use tracing::{debug, error};

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Config {
    pub width: u32,
    pub height: u32,
}

impl Config {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        if let Some(proj_dirs) = ProjectDirs::from("es", "atareao", "lila") {
            let mut config_dir = proj_dirs.config_dir().to_path_buf();
            debug!("config dir: {:?}", config_dir);
            if !config_dir.exists() {
                std::fs::create_dir_all(&config_dir)?;
            }
            config_dir.push("config.yaml");
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
        Config {
            width: 800,
            height: 600,
        }
    }
}
