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
            LaunchError::NotLaunchable(path) => write!(
                f,
                "La aplicación no se puede lanzar: {} no es una entrada lanzable \
                 (revisa Exec/TryExec y el tipo de la entrada)",
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
    #[test]
    fn launch_missing_desktop_returns_err() {
        let missing = Path::new("/tmp/no-existe-jamas/ghost.desktop");
        let item = item_at(missing);

        let err = launch(&item).expect_err("un .desktop inexistente no es lanzable");

        assert!(matches!(&err, LaunchError::MissingDesktopFile(p) if p == missing));
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
