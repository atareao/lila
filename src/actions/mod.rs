use crate::models::Item;
use gio::prelude::AppInfoExt;
use std::error::Error;
use std::fmt;
use std::path::PathBuf;

/// Motivo por el que un lanzamiento no se ha completado.
///
/// Los mensajes son directamente presentables al usuario: la interfaz los
/// muestra tal cual en lugar de fallar en silencio.
#[derive(Debug)]
pub enum LaunchError {
    /// El fichero `.desktop` no existe o no es accesible.
    MissingDesktopFile(PathBuf),
    /// El `.desktop` existe pero no es lanzable: `Type` distinto de
    /// `Application`, `TryExec` declarado y no disponible, `Exec` sin
    /// ejecutable resoluble o fichero mal formado.
    NotLaunchable(PathBuf),
    /// El sistema operativo ha rechazado el lanzamiento (spawn, activación
    /// D-Bus, ausencia de emulador de terminal…).
    Spawn(glib::Error),
}

impl fmt::Display for LaunchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LaunchError::MissingDesktopFile(path) => {
                write!(f, "No se encontró el fichero .desktop: {}", path.display())
            }
            // Sin mencionar las claves del `.desktop` (`Exec`, `TryExec`,
            // `Type`): el mensaje lo lee quien usa el lanzador, no quien
            // depura la entrada.
            LaunchError::NotLaunchable(path) => write!(
                f,
                "La aplicación no se puede lanzar: {} no es una entrada de \
                 aplicación válida",
                path.display()
            ),
            LaunchError::Spawn(err) => {
                write!(f, "El sistema no pudo iniciar la aplicación: {err}")
            }
        }
    }
}

impl Error for LaunchError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            LaunchError::Spawn(err) => Some(err),
            LaunchError::MissingDesktopFile(_) | LaunchError::NotLaunchable(_) => None,
        }
    }
}

/// Lanza la aplicación descrita por `item`.
///
/// Delega en GLib la semántica de ejecución: `DesktopAppInfo::from_filename`
/// valida el `Type`, resuelve `TryExec` y el binario de `Exec` (usando `Path`
/// como directorio de trabajo) y descarta las entradas no lanzables, mientras
/// que `launch_uris` expande los *field codes* de `Exec`, arranca dentro de un
/// terminal si `Terminal=true` y usa la activación D-Bus si `DBusActivatable`.
/// Aquí solo se aporta el diagnóstico tipado del fallo.
pub fn launch(item: &Item) -> Result<(), LaunchError> {
    let Item::App(app) = item;
    let path = &app.app_desktop_path;

    // `from_filename` también devuelve `None` cuando el fichero no existe; se
    // comprueba antes para no etiquetar como "no lanzable" algo que falta.
    if !path.is_file() {
        return Err(LaunchError::MissingDesktopFile(path.clone()));
    }

    let info = gio::DesktopAppInfo::from_filename(path)
        .ok_or_else(|| LaunchError::NotLaunchable(path.clone()))?;

    info.launch_uris(&[], None::<&gio::AppLaunchContext>)
        .map_err(LaunchError::Spawn)
}

#[cfg(test)]
mod tests {
    use crate::actions::{LaunchError, launch};
    use crate::models::Item;
    use applications::App;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::{Path, PathBuf};
    use std::time::{Duration, Instant};
    use tempfile::TempDir;

    /// Ejecutable real del sistema para las fixtures que solo necesitan cargar
    /// el `.desktop`: GLib descarta las entradas cuyo binario de `Exec` no
    /// exista.
    const EXEC_BINARY: &str = "/usr/bin/env";

    /// `Item` de una app apuntando al `.desktop` indicado.
    fn item_at(desktop_path: &Path) -> Item {
        Item::App(App {
            name: String::from("Fixture"),
            icon_path: None,
            app_path_exe: Some(PathBuf::from(EXEC_BINARY)),
            app_desktop_path: desktop_path.to_path_buf(),
        })
    }

    /// Escribe en `dir` un script ejecutable que registra con qué argumentos se
    /// le invoca. Permite observar de forma empírica cómo expande GLib los
    /// *field codes* de `Exec`, sin depender de `.desktop` del sistema.
    ///
    /// Devuelve `(ruta del script, ruta del fichero de argumentos)`.
    fn write_probe(dir: &Path) -> (PathBuf, PathBuf) {
        let script = dir.join("probe.sh");
        let args = dir.join("args.txt");
        let body = format!(
            "#!/bin/sh\nprintf 'count=%s\\n' \"$#\" > \"{out}\"\nfor a in \"$@\"; do printf 'arg=%s\\n' \"$a\" >> \"{out}\"; done\n",
            out = args.display()
        );
        fs::write(&script, body).expect("escribir probe");
        let mut perms = fs::metadata(&script).expect("metadata probe").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&script, perms).expect("chmod +x probe");
        (script, args)
    }

    /// Escribe un `.desktop` con el contenido indicado y devuelve su ruta.
    fn write_desktop(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        fs::write(&path, content).expect("escribir .desktop");
        path
    }

    /// Espera (con tope) a que exista `path` y devuelve su contenido.
    fn wait_for_content(path: &Path, timeout: Duration) -> Option<String> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Ok(content) = fs::read_to_string(path) {
                return Some(content);
            }
            if Instant::now() >= deadline {
                return None;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// [TDD-RED-01] `.desktop` inexistente: error de fichero ausente, con la
    /// ruta en el mensaje (escenario "Fallo de lanzamiento visible").
    ///
    /// Consolidado en 1.6: la ruta ausente se construye dentro de un `TempDir`
    /// (antes un `/tmp/...` fijo) para que el test sea hermético y no dependa de
    /// que ese directorio no exista en la máquina.
    #[test]
    fn launch_missing_desktop_returns_err() {
        let tmp = TempDir::new().expect("TempDir::new()");
        let missing = tmp.path().join("ghost.desktop");
        let item = item_at(&missing);

        let err = launch(&item).expect_err("un .desktop inexistente no es lanzable");

        assert!(matches!(&err, LaunchError::MissingDesktopFile(p) if p == &missing));
        assert!(err.to_string().contains("ghost.desktop"), "mensaje: {err}");
    }

    /// [TDD-RED-02] `TryExec` declarado y no disponible: la app se marca como no
    /// lanzable, el mensaje lo dice y **no se intenta ejecutar** (`Exec` apunta a
    /// un script sonda que nunca debe correr).
    #[test]
    fn launch_unavailable_try_exec_is_not_launchable_and_does_not_run() {
        let tmp = TempDir::new().expect("TempDir::new()");
        let (probe, args) = write_probe(tmp.path());
        let desktop = write_desktop(
            tmp.path(),
            "tryexec.desktop",
            &format!(
                "\
[Desktop Entry]
Type=Application
Name=TryExecMissing
Exec={}
TryExec=/nonexistent/binary-xyz
",
                probe.display()
            ),
        );
        let item = item_at(&desktop);

        let err = launch(&item).expect_err("TryExec no disponible no es lanzable");

        assert!(matches!(&err, LaunchError::NotLaunchable(p) if p == &desktop));
        assert!(
            err.to_string().contains("no se puede lanzar"),
            "mensaje: {err}"
        );

        // Margen para detectar un spawn asíncrono que no debería haberse iniciado.
        std::thread::sleep(Duration::from_millis(300));
        assert!(!args.exists(), "no debe intentarse ejecutar Exec: {err}");
    }

    /// [TDD-RED-04] El mensaje de un `.desktop` ausente es presentable al
    /// usuario: no vacío, distinto del `Debug`, nombra el fichero y explica el
    /// fallo (escenario "Fallo de lanzamiento visible").
    #[test]
    fn missing_desktop_message_is_user_facing() {
        let path = PathBuf::from("/tmp/launcher/ghost.desktop");
        let err = LaunchError::MissingDesktopFile(path.clone());
        let message = err.to_string();

        assert!(!message.trim().is_empty(), "el mensaje no debe estar vacío");
        assert_ne!(message, format!("{err:?}"), "no debe volcar el Debug");
        assert!(
            message.contains("ghost.desktop"),
            "debe nombrar el fichero: {message}"
        );
        assert!(
            message.to_lowercase().contains("no se"),
            "debe explicar el fallo: {message}"
        );
    }

    /// [TDD-RED-05] El mensaje de una entrada no lanzable evita la jerga de las
    /// claves del `.desktop` (`Exec`/`TryExec`/`Type`): el usuario final no debe
    /// interpretar el formato del fichero, solo entender que la app no arranca.
    #[test]
    fn not_launchable_message_is_user_facing_without_entry_jargon() {
        let path = PathBuf::from("/tmp/launcher/rota.desktop");
        let err = LaunchError::NotLaunchable(path.clone());
        let message = err.to_string();

        assert!(!message.trim().is_empty(), "el mensaje no debe estar vacío");
        assert!(
            message.contains("rota.desktop"),
            "debe nombrar la entrada: {message}"
        );
        assert!(
            message.to_lowercase().contains("no se"),
            "debe explicar el fallo: {message}"
        );
        for jargon in ["Exec", "TryExec", "Type="] {
            assert!(
                !message.contains(jargon),
                "sin jerga de claves del .desktop ('{jargon}'): {message}"
            );
        }
    }

    /// [TDD-RED-06] Capa de datos: las claves de `Exec` se leen **verbatim** del
    /// `.desktop`, con los *field codes* sin expandir. La expansión (`%U`/`%f` a
    /// nada cuando no hay URIs) es cosa de `launch_uris`, no del parseo, y se
    /// verifica en `launch_exec_with_field_codes_passes_no_literal_arguments`.
    ///
    /// No spawnea nada: solo carga la entrada con `from_filename` y consulta sus
    /// claves.
    #[test]
    fn desktop_exec_key_is_exposed_verbatim_with_field_codes() {
        let tmp = TempDir::new().expect("TempDir::new()");
        let desktop = write_desktop(
            tmp.path(),
            "exec-fieldcodes.desktop",
            &format!(
                "\
[Desktop Entry]
Type=Application
Name=ExecFieldCodes
Exec={EXEC_BINARY} %U %f
"
            ),
        );

        let info = gio::DesktopAppInfo::from_filename(&desktop).expect("la entrada debe cargarse");
        let expected_exec = format!("{EXEC_BINARY} %U %f");

        assert!(
            info.has_key("Exec"),
            "la clave Exec debe estar presente en la entrada cargada"
        );
        assert_eq!(
            info.string("Exec").as_deref(),
            Some(expected_exec.as_str()),
            "Exec debe conservar los field codes tal cual (sin expandir)"
        );
        assert_eq!(
            info.filename().as_deref(),
            Some(desktop.as_path()),
            "la entrada debe quedar ligada a la fixture, no a un .desktop del sistema"
        );
    }

    /// [TDD-RED-07] Capa de datos: `Terminal=true` se declara y se lee del
    /// `.desktop`.
    ///
    /// El **arranque dentro de un emulador** no se comprueba aquí, y no es
    /// testeable de forma hermética: lo realiza GLib dentro de
    /// `g_desktop_app_info_launch_uris_with_spawn()`, que antepone el terminal
    /// resuelto por `GDesktopAppInfo` (GSettings
    /// `org.gnome.desktop.default-applications.terminal` o `$TERMINAL`). Un test
    /// real requeriría un emulador instalado en `PATH` y lanzaría un proceso
    /// interactivo, así que la verificación se queda en el dato declarado y en la
    /// delegación documentada en `actions::launch`.
    #[test]
    fn desktop_terminal_key_is_exposed_and_startup_is_delegated_to_glib() {
        let tmp = TempDir::new().expect("TempDir::new()");
        let desktop = write_desktop(
            tmp.path(),
            "terminal.desktop",
            &format!(
                "\
[Desktop Entry]
Type=Application
Name=TerminalApp
Exec={EXEC_BINARY}
Terminal=true
"
            ),
        );

        let info = gio::DesktopAppInfo::from_filename(&desktop).expect("la entrada debe cargarse");

        assert!(
            info.has_key("Terminal"),
            "la clave Terminal debe estar presente"
        );
        assert!(
            info.boolean("Terminal"),
            "Terminal=true debe leerse como booleano verdadero"
        );
        assert_eq!(
            info.string("Terminal").as_deref(),
            Some("true"),
            "Terminal debe exponerse tal cual en la capa de datos"
        );
    }

    /// [TDD-RED-08] Capa de datos: un fichero ausente no produce ninguna entrada,
    /// que es justo por lo que `actions::launch` comprueba `is_file()` antes de
    /// llamar a `from_filename` (si no, un fichero que falta se reportaría como
    /// "no lanzable" en lugar de "no se encontró").
    #[test]
    fn desktop_app_info_from_missing_path_is_none() {
        let tmp = TempDir::new().expect("TempDir::new()");
        let missing = tmp.path().join("ghost.desktop");

        assert!(
            gio::DesktopAppInfo::from_filename(&missing).is_none(),
            "sin fichero no hay DesktopAppInfo"
        );
    }

    /// [TDD-RED-03] `Exec` con *field codes* (`%U`, `%f`) y sin URIs: GLib los
    /// expande a nada, de modo que la app arranca sin argumentos residuales
    /// literales. Se verifica de verdad observando los argumentos del proceso.
    #[test]
    fn launch_exec_with_field_codes_passes_no_literal_arguments() {
        let tmp = TempDir::new().expect("TempDir::new()");
        let (probe, args) = write_probe(tmp.path());
        let desktop = write_desktop(
            tmp.path(),
            "fieldcodes.desktop",
            &format!(
                "\
[Desktop Entry]
Type=Application
Name=FieldCodes
Exec={} %U %f
",
                probe.display()
            ),
        );
        let item = item_at(&desktop);

        launch(&item).expect("debe lanzarse");

        let content = wait_for_content(&args, Duration::from_secs(5))
            .expect("el ejecutable de Exec debe haber corrido");
        assert_eq!(content, "count=0\n", "GLib expande %U/%f a nada sin URIs");
        assert!(
            !content.contains('%'),
            "sin field codes residuales: {content}"
        );
    }
}
