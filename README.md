# LiLa

LiLa es un lanzador de aplicaciones para Wayland: un overlay GTK4 (`gtk4-layer-shell`) que busca y lanza ficheros `.desktop` desde el teclado.

## Requisitos

- **`gtk4-layer-shell`** instalado como librería del sistema (no basta con la crate; sin la librería el enlace falla). En ArchLinux:

  ```bash
  sudo pacman -S gtk4-layer-shell
  ```

- **Compositor Wayland** con soporte layer-shell. Wayland es la única plataforma soportada: LiLa usa `init_layer_shell()` con `Layer::Overlay` y **no arranca en X11 ni en headless**.

## Compilación, ejecución y tests

```bash
cargo build                       # compilar
cargo run                         # ejecutar (requiere compositor Wayland)
cargo test                        # tests (no necesitan display)
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

## Uso

- **Atajo global:** enlaza una tecla del compositor (p. ej. en Sway/Hyprland) a la ejecución del binario `lila`. La **acción D-Bus** dedicada para el atajo global quedó como **follow-up no implementado**; hoy basta con invocar el binario.
- **Instancia única:** LiLa es singleton; una segunda invocación **re-presenta** la ventana existente (misma ventana, con foco) en lugar de abrir otra.
- **Búsqueda:** filtrado fuzzy al teclear, con **ranking combinado** por coincidencia difusa + frecuencia de uso + recencia. Al lanzar con Enter se registra en el historial.

## Configuración

Ficheros de runtime en el directorio XDG del proyecto (`~/.config/lila/`):

- **`~/.config/lila/config.yml`** — configuración (YAML): pesos del ranking (`fuzzy_weight`, `frequency_weight`, `recency_weight`), `max_results` y `history_enabled` (historial on/off), entre otros. Si no existe, se escribe desde `Config::default()`.
- **`~/.config/lila/style.css`** — estilos GTK. No se reescribe si ya existe.
- **`~/.config/lila/history.db`** — base de datos SQLite del historial de uso (WAL), que alimenta el ranking.

## Instalación del lanzador

Instala el `.desktop` de LiLa en `~/.local/share/applications/` (o `/usr/share/applications/`) para poder invocarlo desde el menú/atajo del escritorio.

## Documentación

Detalle técnico del milestone (arquitectura, motor de ejecución, esquema SQLite, decisiones y follow-ups): [`docs/core-lanzamiento.md`](docs/core-lanzamiento.md).
