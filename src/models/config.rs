use super::super::finders::{ApplicationFinder, Finder};
use super::super::utils::*;
use super::Extension;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use tracing::{debug, warn};

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

/// Nombre del fichero de configuración dentro del directorio de configuración.
const CONFIG_FILE: &str = "config.yml";

/// Errores de carga de la configuración del usuario.
///
/// Solo representan problemas **reales** con un fichero existente: la ausencia
/// del fichero, un directorio no escribible o la falta de directorios XDG no son
/// errores (se arranca con los valores por defecto).
#[derive(Debug)]
pub enum ConfigError {
    /// El fichero de configuración existe pero no se pudo leer.
    Unreadable {
        path: PathBuf,
        source: std::io::Error,
    },
    /// El fichero de configuración existe pero su YAML no es válido.
    Invalid {
        path: PathBuf,
        source: serde_yaml::Error,
    },
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::Unreadable { path, source } => {
                write!(f, "no se pudo leer {}", path.display())?;
                // Un enlace simbólico roto da `NotFound` en la lectura aunque el
                // enlace exista: sin esta pista el mensaje resulta engañoso.
                if source.kind() == ErrorKind::NotFound {
                    write!(f, " (¿enlace simbólico roto?)")?;
                }
                write!(f, ": {source}")
            }
            ConfigError::Invalid { path, source } => {
                write!(f, "el YAML de {} no es válido: {source}", path.display())
            }
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConfigError::Unreadable { source, .. } => Some(source),
            ConfigError::Invalid { source, .. } => Some(source),
        }
    }
}

/// Lee la configuración del usuario en `path`.
///
/// - `Ok(Some(config))` si el fichero existe, es legible y su YAML es válido.
/// - `Ok(None)` **solo** si aquí no hay fichero: `NotFound` sin nada en el path
///   o `NotADirectory` (el «directorio» de configuración es un fichero regular,
///   el caso de un directorio no escribible).
/// - `Err(..)` en cualquier otro fallo de lectura (`PermissionDenied`,
///   `IsADirectory`, …) o si el YAML no es válido.
///
/// La existencia se comprueba con `symlink_metadata` para el caso `NotFound`: un
/// enlace simbólico roto cuenta como «existe» (la lectura sigue el enlace y
/// falla, pero el enlace es del usuario) y nunca se sigue al escribir.
fn read_user_config(path: &Path) -> Result<Option<Config>, ConfigError> {
    match fs::read_to_string(path) {
        Ok(contents) => match serde_yaml::from_str(&contents) {
            Ok(config) => Ok(Some(config)),
            Err(source) => Err(ConfigError::Invalid {
                path: path.to_path_buf(),
                source,
            }),
        },
        Err(e) if e.kind() == ErrorKind::NotADirectory => Ok(None),
        // Solo es «no hay fichero» si el propio path tampoco existe: un enlace
        // simbólico roto da `NotFound` al leer, pero el path sí está ahí y es
        // del usuario.
        Err(e)
            if e.kind() == ErrorKind::NotFound
                && fs::symlink_metadata(path).is_err_and(|m| m.kind() == ErrorKind::NotFound) =>
        {
            Ok(None)
        }
        Err(source) => Err(ConfigError::Unreadable {
            path: path.to_path_buf(),
            source,
        }),
    }
}

/// Escribe `yaml` en `writer` (el `config.yml` recién creado con `O_EXCL`).
///
/// Si la escritura falla a mitad (`ENOSPC`, `EDQUOT`, …), cierra el descriptor y
/// **elimina el fichero**: es nuestro (lo acabamos de crear, no existía), y
/// dejarlo a medias haría que el siguiente arranque lo leyera como YAML inválido
/// y abortara de forma permanente.
fn write_default_file<W: Write>(mut writer: W, path: &Path, yaml: &str) -> std::io::Result<()> {
    if let Err(e) = writer.write_all(yaml.as_bytes()) {
        drop(writer);
        let _ = fs::remove_file(path);
        return Err(e);
    }
    Ok(())
}

/// Escribe el default en `path` **solo si no existe**.
///
/// `File::create_new` abre con `O_CREAT | O_EXCL`: es atómico y no trunca, así
/// que un fichero que aparezca entre la comprobación de ausencia y la escritura
/// (otra instancia, el usuario, un enlace simbólico) nunca se sobrescribe.
/// Devuelve `Ok(true)` si lo ha creado, `Ok(false)` si ya existía y `Err` si no
/// se ha podido escribir (directorio no escribible, disco lleno, …).
fn create_config_file(config_dir: &Path, path: &Path, yaml: &str) -> std::io::Result<bool> {
    fs::create_dir_all(config_dir)?;
    match fs::File::create_new(path) {
        Ok(file) => {
            write_default_file(file, path, yaml)?;
            Ok(true)
        }
        Err(e) if e.kind() == ErrorKind::AlreadyExists => Ok(false),
        Err(e) => Err(e),
    }
}

/// Carga la configuración desde `config_dir`. Función pura: no depende de GTK ni
/// de los directorios XDG reales, de modo que es testeable con un directorio
/// temporal.
///
/// Política de carga:
/// 1. Sin directorio de configuración (`None`), solo cabe el default.
/// 2. Si `<config_dir>/config.yml` es legible y su YAML es válido, manda el
///    fichero del usuario y no se escribe nada.
/// 3. Si existe pero es ilegible, o su YAML es inválido, se devuelve un
///    [`ConfigError`]: nunca se sobrescribe ni se borra lo que el usuario tenga
///    ahí. La existencia se comprueba con `symlink_metadata`, así que un enlace
///    simbólico roto también cuenta como «existe» y no se sigue al escribir.
/// 4. Si no existe, se materializa el default serializado en el directorio
///    (creándolo si hace falta) y se devuelve ese mismo default. La escritura es
///    atómica (`create_new`), de modo que jamás pisa un fichero que aparezca en
///    la ventana entre la lectura y la escritura. Si la escritura falla, se
///    arranca igualmente con el default desde memoria y se avisa.
pub(crate) fn load_config_from(config_dir: Option<&Path>) -> Result<Config, ConfigError> {
    let Some(config_dir) = config_dir else {
        debug!("sin directorio de configuración: se usa la configuración por defecto");
        return Ok(Config::default());
    };

    let path = config_dir.join(CONFIG_FILE);
    let default = Config::default();

    // 1. El fichero del usuario manda si está y se puede leer.
    if let Some(config) = read_user_config(&path)? {
        debug!("config dir: {}", config_dir.display());
        return Ok(config);
    }

    // 2. No hay fichero: se guarda el default para que el usuario tenga un punto
    //    de partida editable. Un fallo al escribirlo no impide el arranque.
    let yaml = match serde_yaml::to_string(&default) {
        Ok(yaml) => yaml,
        Err(e) => {
            warn!("no se pudo serializar la configuración por defecto: {e}");
            return Ok(default);
        }
    };
    match create_config_file(config_dir, &path, &yaml) {
        Ok(true) => {
            debug!("config file created: {}", path.display());
            Ok(default)
        }
        // Carrera perdida: el fichero ha aparecido entre la lectura y la
        // escritura y `create_new` no lo ha truncado. Se respeta lo que haya.
        Ok(false) => match read_user_config(&path)? {
            Some(config) => {
                debug!(
                    "{} ha aparecido durante el arranque: se usa su contenido",
                    path.display()
                );
                Ok(config)
            }
            None => {
                warn!(
                    "{} ha aparecido y desaparecido durante el arranque; \
                     se usa la configuración por defecto en memoria",
                    path.display()
                );
                Ok(default)
            }
        },
        Err(e) => {
            warn!(
                "no se pudo escribir {}: {e}; se usa la configuración por defecto en memoria",
                path.display()
            );
            Ok(default)
        }
    }
}

impl Config {
    /// Carga la configuración del usuario.
    ///
    /// Mantiene la firma `Result` para que el llamador pueda distinguir un
    /// problema real (fichero existente ilegible o con YAML inválido) de un
    /// arranque normal. La ausencia del fichero, un directorio no escribible o
    /// la falta de directorios XDG **no** son errores: se arranca con los
    /// valores por defecto, que es lo que necesita el primer arranque.
    pub fn load() -> Result<Self, ConfigError> {
        let config_dir = ProjectDirs::from(APP_QUALIFIER, APP_ORGANIZATION, APP_APPLICATION)
            .map(|proj_dirs| proj_dirs.config_dir().to_path_buf());
        load_config_from(config_dir.as_deref())
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
    use super::{
        CONFIG_FILE, Config, ConfigError, create_config_file, load_config_from, write_default_file,
    };
    use std::fs;
    use tempfile::TempDir;

    /// [TDD-FIRST-RUN-01] (a) Un `config.yml` presente y válido se usa tal cual y
    /// no se reescribe.
    #[test]
    fn load_uses_present_config_file() {
        let dir = TempDir::new().expect("no se pudo crear el directorio temporal");
        let path = dir.path().join(CONFIG_FILE);
        let original = "width: 640\nmax_results: 7\n";
        fs::write(&path, original).expect("no se pudo escribir config.yml");

        let config = load_config_from(Some(dir.path())).expect("debería cargar el fichero");

        assert_eq!(config.width, 640);
        assert_eq!(config.max_results, 7);
        assert_eq!(
            fs::read_to_string(&path).expect("el fichero debe seguir siendo legible"),
            original,
            "un config.yml que existe no debe reescribirse nunca"
        );
    }

    /// [TDD-FIRST-RUN-02] (b) Sin `config.yml` se escribe el default y se arranca
    /// con él: el primer arranque de un usuario nuevo no puede fallar.
    #[test]
    fn load_writes_default_when_config_file_is_absent() {
        let dir = TempDir::new().expect("no se pudo crear el directorio temporal");
        let path = dir.path().join(CONFIG_FILE);
        let defaults = Config::default();

        let config = load_config_from(Some(dir.path())).expect("la ausencia no es un error");

        assert_eq!(config.width, defaults.width);
        assert_eq!(config.max_results, defaults.max_results);
        let written = fs::read_to_string(&path).expect("debería haberse escrito config.yml");
        let reparsed: Config =
            serde_yaml::from_str(&written).expect("el default escrito debería ser YAML válido");
        assert_eq!(reparsed.width, defaults.width);
        assert_eq!(reparsed.max_results, defaults.max_results);
    }

    /// [TDD-FIRST-RUN-03] (c) Si el directorio no se puede crear ni escribir se
    /// usa el default en memoria: ni error ni `panic`. El padre del «directorio»
    /// de configuración es un fichero regular, así que `create_dir_all` y la
    /// escritura fallan incluso ejecutando como root.
    #[test]
    fn load_falls_back_in_memory_when_directory_is_not_writable() {
        let tmp = TempDir::new().expect("no se pudo crear el directorio temporal");
        let blocker = tmp.path().join("afile");
        fs::write(&blocker, "no soy un directorio").expect("no se pudo escribir el fichero");

        let config =
            load_config_from(Some(&blocker)).expect("un directorio no escribible no es error");

        assert_eq!(config.width, Config::default().width);
        assert_eq!(config.max_results, Config::default().max_results);
        assert!(
            !blocker.join(CONFIG_FILE).exists(),
            "no debería haberse escrito nada dentro del bloqueador"
        );
    }

    /// [TDD-FIRST-RUN-04] (d) Un YAML inválido en el fichero del usuario es un
    /// error de configuración y el fichero queda intacto.
    #[test]
    fn load_reports_invalid_yaml() {
        let dir = TempDir::new().expect("no se pudo crear el directorio temporal");
        let path = dir.path().join(CONFIG_FILE);
        let broken = "width: [no es un entero\n";
        fs::write(&path, broken).expect("no se pudo escribir config.yml");

        let result = load_config_from(Some(dir.path()));

        assert!(
            matches!(result, Err(ConfigError::Invalid { .. })),
            "un YAML inválido debería ser ConfigError::Invalid: {result:?}"
        );
        assert_eq!(
            fs::read_to_string(&path).expect("el fichero del usuario debe seguir ahí"),
            broken,
            "un fichero inválido nunca debe sobrescribirse"
        );
    }

    /// [TDD-FIRST-RUN-05] (e) Un `config.yml` presente pero ilegible es un error
    /// de configuración. Un **directorio** con ese nombre hace fallar la lectura
    /// incluso ejecutando como root, así que el test es estable en cualquier
    /// entorno, sin depender de permisos.
    #[test]
    fn load_reports_unreadable_config_file() {
        let dir = TempDir::new().expect("no se pudo crear el directorio temporal");
        let path = dir.path().join(CONFIG_FILE);
        fs::create_dir(&path).expect("no se pudo crear el directorio config.yml");

        let result = load_config_from(Some(dir.path()));

        assert!(
            matches!(result, Err(ConfigError::Unreadable { .. })),
            "un fichero ilegible debería ser ConfigError::Unreadable: {result:?}"
        );
        assert!(
            path.is_dir(),
            "el fichero ilegible del usuario no debe borrarse ni sobrescribirse"
        );
    }

    /// [TDD-FIRST-RUN-06] (f) Sin directorio de configuración XDG se arranca con
    /// el default.
    #[test]
    fn load_uses_default_without_config_dir() {
        let config = load_config_from(None).expect("sin directorio no hay error posible");

        assert_eq!(config.width, Config::default().width);
        assert_eq!(config.max_results, Config::default().max_results);
    }

    /// [TDD-FIRST-RUN-07] El bug real ocurre cuando `~/.config/lila` todavía no
    /// existe: el directorio debe crearse (incluso anidado) y el default quedar
    /// escrito, sin fallar.
    #[test]
    fn load_creates_missing_config_directory_and_writes_default() {
        let tmp = TempDir::new().expect("no se pudo crear el directorio temporal");
        let config_dir = tmp.path().join("lila").join("a").join("b");
        let path = config_dir.join(CONFIG_FILE);
        let defaults = Config::default();

        let config = load_config_from(Some(&config_dir)).expect("debería crear el directorio");

        assert_eq!(config.width, defaults.width);
        assert_eq!(config.max_results, defaults.max_results);
        assert!(path.is_file(), "debería existir {}", path.display());
        let written = fs::read_to_string(&path).expect("debería poderse leer config.yml");
        let reparsed: Config =
            serde_yaml::from_str(&written).expect("el default escrito debería ser YAML válido");
        assert_eq!(reparsed.max_results, defaults.max_results);
    }

    /// [TDD-FIRST-RUN-08] La escritura del default usa `create_new`: si el
    /// fichero aparece entre la comprobación de ausencia y la escritura (otra
    /// instancia, el usuario…), **no** se trunca ni se sobrescribe.
    #[test]
    fn create_config_file_does_not_overwrite_an_existing_file() {
        let dir = TempDir::new().expect("no se pudo crear el directorio temporal");
        let path = dir.path().join(CONFIG_FILE);
        let user_content = "width: 320\n";
        fs::write(&path, user_content).expect("no se pudo escribir config.yml");

        let created = create_config_file(dir.path(), &path, "width: 1280\n")
            .expect("escribir sobre un fichero existente no es un error de E/S");

        assert!(
            !created,
            "`create_new` debería avisar de que el fichero ya existía"
        );
        assert_eq!(
            fs::read_to_string(&path).expect("el fichero del usuario debe seguir ahí"),
            user_content,
            "un fichero que aparece en la ventana entre lectura y escritura no debe truncarse"
        );
    }

    /// [TDD-FIRST-RUN-09] Cualquier fallo de lectura que no sea «no existe»
    /// (`NotFound`) o «aquí no hay fichero» (`NotADirectory`) es un fichero
    /// ilegible, no un fichero ausente: no debe caerse en silencio al default.
    ///
    /// Un nombre de componente imposible de resolver (`ENAMETOOLONG`) provoca un
    /// error de lectura estable en cualquier entorno, incluido `root`: con
    /// permisos no serviría, porque `root` los esquiva.
    #[test]
    fn load_reports_unreadable_for_errors_other_than_not_found() {
        let tmp = TempDir::new().expect("no se pudo crear el directorio temporal");
        let config_dir = tmp.path().join("x".repeat(300));

        let result = load_config_from(Some(&config_dir));

        assert!(
            matches!(result, Err(ConfigError::Unreadable { .. })),
            "un error de lectura distinto de «no existe» debería ser ConfigError::Unreadable: \
             {result:?}"
        );
    }

    /// [TDD-FIRST-RUN-10] Un enlace simbólico roto se considera «existe»: se
    /// reporta como ilegible y **no** se sigue al escribir, así que no se crea
    /// el destino del enlace ni se toca el enlace del usuario.
    #[cfg(unix)]
    #[test]
    fn load_reports_broken_symlink_and_does_not_follow_it() {
        let tmp = TempDir::new().expect("no se pudo crear el directorio temporal");
        let config_dir = tmp.path().join("config");
        fs::create_dir(&config_dir).expect("no se pudo crear el directorio de configuración");
        let target = tmp.path().join("outside.yml");
        let link = config_dir.join(CONFIG_FILE);
        std::os::unix::fs::symlink(&target, &link).expect("no se pudo crear el enlace");

        let result = load_config_from(Some(&config_dir));

        let err = result.expect_err("un enlace roto debería ser un error de configuración");
        assert!(
            matches!(err, ConfigError::Unreadable { .. }),
            "un enlace roto debería ser ConfigError::Unreadable: {err:?}"
        );
        assert!(
            err.to_string().contains(&link.display().to_string()),
            "el error debería identificar el fichero problemático: {err}"
        );
        assert!(
            !target.exists(),
            "materializar el default no debe seguir el enlace ni crear su destino"
        );
        assert!(
            fs::symlink_metadata(&link).is_ok(),
            "el enlace del usuario no debe borrarse ni reemplazarse"
        );
    }

    /// Escritor que no acepta nada, como un disco lleno (`ENOSPC`/`EDQUOT`) a
    /// mitad de la escritura. `write_all` lo convierte en `WriteZero`.
    struct FullDisk;

    impl std::io::Write for FullDisk {
        fn write(&mut self, _buf: &[u8]) -> std::io::Result<usize> {
            Ok(0)
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    /// [TDD-FIRST-RUN-11] Un fallo de escritura a mitad **no** debe dejar un
    /// `config.yml` a medio escribir: lo hemos creado nosotros con `O_EXCL`, así
    /// que se elimina. De quedarse, el siguiente arranque lo leería como YAML
    /// inválido y el usuario tendría un `exit(2)` permanente sin saber por qué.
    ///
    /// La escritura se inyecta con un `Write` que falla, de modo que el test es
    /// determinista y no depende de llenar un disco de verdad.
    #[test]
    fn failed_write_removes_the_partially_written_file() {
        let dir = TempDir::new().expect("no se pudo crear el directorio temporal");
        let path = dir.path().join(CONFIG_FILE);
        // Se simula el fichero que `create_new` acaba de crear y deja a medias.
        fs::write(&path, "width: 8").expect("no se pudo escribir config.yml");

        let result = write_default_file(FullDisk, &path, "width: 800\n");

        assert!(result.is_err(), "la escritura debería fallar y propagarse");
        assert!(
            !path.exists(),
            "un default a medio escribir debe eliminarse, no quedar en disco"
        );
        // Y el arranque siguiente se recupera solo: vuelve a escribirse el default.
        let config = load_config_from(Some(dir.path())).expect("debería regenerarse el default");
        assert_eq!(config.width, Config::default().width);
    }

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
