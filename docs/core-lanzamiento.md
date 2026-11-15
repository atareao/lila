# core-lanzamiento

- **Issue milestone:** [#5](https://github.com/atareao/lila/issues/5)
- **Issue de documentación/verificación (Fase E):** [#10](https://github.com/atareao/lila/issues/10)
- **Change OpenSpec:** `openspec/changes/core-lanzamiento/` · **Plan:** `plans/PLAN-001.md`

## 1. Qué es

Milestone 1 de LiLa: convertir el prototipo en un **core de lanzamiento sólido** antes de añadir nuevas extensiones. Entrega cuatro piezas:

1. **Motor de lanzamiento** de ficheros `.desktop` conforme a sus metadatos (field codes, `Terminal`, `Path`, `TryExec`, D-Bus) con errores visibles al usuario.
2. **Ranking combinado** de resultados: coincidencia difusa + frecuencia de uso + recencia, con pesos configurables.
3. **Historial de uso** persistente en SQLite (WAL, esquema versionado) que alimenta al ranking.
4. **Instancia única** (`GApplication`): una segunda invocación re-presenta el dashboard existente en lugar de abrir otra ventana.

## 2. Ficheros implicados

| Fichero | Responsabilidad |
|---|---|
| `src/main.rs` | UI GTK imperativa. `startup` (solo instancia primaria) carga config, inicializa finders y abre el historial; `activate` construye o re-presenta la ventana; filtrar al teclear, navegar y lanzar con Enter. Mantiene la unicidad y el guard de foco. |
| `src/actions/mod.rs` | `launch(&Item) -> Result<(), LaunchError>`: motor de ejecución vía `gio::DesktopAppInfo` y diagnóstico tipado (`MissingDesktopFile` / `NotLaunchable` / `Spawn`). |
| `src/finders/finder.rs` | `enum Finder` (variantes `EmptyFinder` / `ApplicationFinder`) y trait `Find` (`init` / `update` / `search`) con el despacho por variante. |
| `src/finders/application.rs` | `ApplicationFinder` (cachea los `.desktop` del sistema vía `applications::AppInfoContext`) y `filter_items` (fuzzy puro con `SkimMatcherV2`). |
| `src/finders/ranking.rs` | `rank_items`: puntuación combinada fuzzy + frecuencia + recencia; aplica `max_results` en todas las rutas. |
| `src/finders/empty.rs` | `EmptyFinder`: no devuelve nada; variante por defecto del enum. |
| `src/history/mod.rs` | `SqliteHistory` + trait `HistoryRepository`; esquema, WAL, migraciones y puntuaciones `frequency_score` / `recency_score`. |
| `src/models/config.rs` | `Config` (YAML) con los campos del core y tolerancia a configs antiguas. |
| `src/models/css.rs` | `Css::load()`: proveedor de estilos desde `~/.config/lila/style.css` (o defaults). |
| `src/models/extension.rs` | `Extension` (id, name, key, description, `finder`, `max_time`). |
| `src/models/item.rs` | `Item` (enum de un solo brazo `App`): `id()` canónico, `name`, `icon_path`, `subtitle`, `exec`, `keywords`, `categories`. |
| `src/ui/row.rs` | `ResultRow`: view-model puro de fila (nombre, subtítulo, icono), sin GTK. |
| `src/ui/navigation.rs` | `Move` / `target_index` (wrap-around, PageUp/PageDown, Home/End) y `scroll_value` (scroll-to-selected). |
| `src/ui/error.rs` | `ErrorBanner`: estado puro del banner de error + `apply` al `Label`. |
| `src/utils/constants.rs` | `APP_ID = es.atareao.lila`, nombres GTK (`lila-listbox`, `lila-scroll`), tamaños de icono. |
| `src/utils/functions.rs` | `now()` (u128) y `now_millis()` (i64 saturado). |

## 3. Cómo funciona

### Búsqueda → ranking → selección → lanzamiento

1. **Carga de apps.** En `startup`, `ApplicationFinder::init()` llama a `update()`, que vuelca `AppInfoContext::refresh_apps()` a un `Vec<Item>` cacheado. `main` toma la extensión `application-finder` (o la primera) y llama `search("")` una vez para poblarla.
2. **Despacho por variante.** `Finder::search(query)` hace `match` sobre el enum y delega en `EmptyFinder` o `ApplicationFinder`; cualquier finder nuevo se añade como variante con sus brazos en `init`/`update`/`search` (`finders/finder.rs`).
3. **Filtrado y ranking.** Cada pulsación llama a `rank_items(&apps, text, history, &config, now_millis)`. Con la consulta vacía todas las apps entran (término difuso `0`), ordenadas por uso. El resultado se recorta a `config.max_results` en **todas** las rutas (ranking combinado y respaldos difusos).
4. **Selección.** Se reconstruyen las filas `ListBoxRow` (`ResultRow` + `build_result_row`) y se selecciona la primera cuando hay resultados. La navegación con teclado calcula el índice con `target_index` (wrap-around en flechas, PageUp/PageDown, Home/End) y `scroll_value` mantiene la fila a la vista.
5. **Lanzamiento.** Enter toma el `Item` seleccionado, llama a `actions::launch(&item)`; si va bien oculta la ventana y registra en el historial; si falla **no** la oculta y muestra el motivo en el banner.

### Fórmula de score

```
final_score = w_fuzzy * fuzzy + w_freq * ln(1 + launches) + w_recency * decay(last_used)
```

- `fuzzy`: `SkimMatcherV2::fuzzy_match(name, query)`; `0` con consulta vacía.
- `frequency_score = ln(1 + launches)` — crecimiento logarítmico.
- `recency_score = 0.5^(edad_días / HALF_LIFE_DAYS)`, con `HALF_LIFE_DAYS = 14`. Un `now` anterior al último uso se trata como edad `0` (nunca crece por reloj adelantado).
- Item sin historial: `Usage { launches: 0, .. }` → frecuencia `0`, recencia según `last_used = 0` (prácticamente `0`).
- Si `history_enabled == false`, no hay repositorio o falla `load_all`, se cae a `filter_items` (solo difuso), igualmente recortado a `max_results`.

El ranking lee el historial **una vez** (`load_all` → `HashMap<String, Usage>`) por refresco para no consultar SQLite por ítem.

### Instancia única

`gtk::Application::new(Some(APP_ID), ..)` arbitra la unicidad. `connect_startup` solo se emite en la instancia primaria: ahí se carga la `Config`, se inicializan los finders y se abre `history.db`; la secundaria se registra como remota, no repite ese trabajo pesado y termina. En `activate`, si ya hay ventana viva, se **re-presenta** (`present_existing`): `present()` + `grab_focus`, y solo si la ventana estaba oculta se limpian banner y `Entry`. La ventana nunca se destruye: `connect_close_request` solo la oculta. Un margen de `PRESENT_FOCUS_GRACE = 250 ms` (con re-verificación de foco por generación) evita que una pérdida de foco transitoria del compositor al mapear oculte la ventana.

### Motor de ejecución

`actions::launch` delega en GLib y solo añade el diagnóstico:

- Si el `.desktop` no existe (`!path.is_file()`) → `MissingDesktopFile` (antes de `from_filename`, que también daría `None`).
- `DesktopAppInfo::from_filename` valida `Type=Application`, resuelve `TryExec` y el binario de `Exec`, aplica `Path` como directorio de trabajo y descarta entradas no lanzables → si falla, `NotLaunchable`.
- `launch_uris(&[], None)` expande los **field codes** (`%f %F %u %U %i %c %k`), arranca **dentro de un terminal** si `Terminal=true` (el terminal lo resuelve GLib vía GSettings `org.gnome.desktop.default-applications.terminal` o `$TERMINAL`) y usa **activación D-Bus** si `DBusActivatable=true`; un fallo del sistema → `Spawn`.
- Los mensajes son presentables al usuario y evitan la jerga de claves del `.desktop`.

## 4. Variables y configuración

Ficheros de runtime en el directorio XDG del proyecto (`ProjectDirs`, qualifier `es`, org `atareao`, app `lila`):

- `~/.config/lila/config.yml` — `Config` (YAML). Si no existe, se escribe desde `Config::default()`.
- `~/.config/lila/style.css` — estilos GTK. `Css::load()` **no** reescribe un `style.css` existente (los estilos nuevos solo llegan a instalaciones previas borrando el fichero).
- `~/.config/lila/history.db` — base de datos del historial (ver §5).

Campos de `Config` (todos con `#[serde(default)]`, así que un `config.yml` antiguo arranca sin error):

| Campo | Tipo | Default | Uso |
|---|---|---|---|
| `width`, `height` | `i32` | `800`, `600` | Tamaño de ventana/scroll. |
| `left`,`right`,`top`,`bottom` | `Edge { margin:i32, anchor:bool }` | `margin=100, anchor=false` | Anclajes de la capa. |
| `extensions` | `Vec<Extension>` | `[application-finder]` | Finders disponibles. |
| `fuzzy_weight` | `f64` | `1.0` | Peso del término difuso. |
| `frequency_weight` | `f64` | `0.5` | Peso de `ln(1+lanzamientos)`. |
| `recency_weight` | `f64` | `0.5` | Peso del decaimiento por recencia. |
| `max_results` | `usize` | `50` | Límite de resultados mostrados. |
| `history_enabled` | `bool` | `true` | Si `false`: no se registra nada y el ranking es solo difuso. |

## 5. Esquema SQLite del historial

- **Fichero:** `~/.config/lila/history.db` (mismo `ProjectDirs` que `config.yml`; se crea el directorio si falta). `journal_mode = WAL`.
- **`launcher_history`** — una fila por aplicación lanzada:

  | Columna | Tipo | Uso |
  |---|---|---|
  | `item_id` | `TEXT PRIMARY KEY` | Identificador estable = ruta canónica del `.desktop` (o el path literal si la canonicalización falla). |
  | `launches` | `INTEGER NOT NULL DEFAULT 0` | Nº de lanzamientos; alimenta `ln(1+launches)`. |
  | `first_used` | `INTEGER NOT NULL` | Epoch millis del primer lanzamiento. |
  | `last_used` | `INTEGER NOT NULL` | Epoch millis del último lanzamiento; alimenta la recencia. |

- **`idx_launcher_history_last_used`** — índice sobre `launcher_history(last_used)`.
- **`schema_version(version INTEGER)`** — versión de esquema (actual `SCHEMA_VERSION = 1`); migraciones idempotentes al abrir. Si la BD declara una versión futura se avisa por log.
- `record` hace *upsert*: inserta con `launches = 1` o incrementa y actualiza `last_used` (`ON CONFLICT(item_id) DO UPDATE`). El trait `HistoryRepository` expone `record`, `usage`, `score` (frecuencia × recencia) y `load_all`; la conexión vive detrás de un `Mutex` (rusqlite no es `Sync`).

## 6. Cómo habilitar / desplegar

- **Dependencia de sistema:** `gtk4-layer-shell` (no basta la crate; sin la librería el enlace falla). Arch: `sudo pacman -S gtk4-layer-shell`.
- **Instalar el `.desktop`** de la aplicación en `~/.local/share/applications/` (o `/usr/share/applications/`) para que el lanzador sea invocable desde el menú/atajo.
- **Atajo global del compositor:** enlazar una tecla (p. ej. en el config de Sway/Hyprland) a la ejecución del binario `lila`. La **acción D-Bus** dedicada para el atajo global (tarea 5.3) queda como **follow-up no implementado**; hoy basta con invocar el binario, que por instancia única re-presenta la ventana.
- **Requisito de plataforma:** compositor **Wayland** con soporte layer-shell. La ventana usa `init_layer_shell()`, `Layer::Overlay`, `KeyboardMode::Exclusive` y anclajes; no arranca en X11/headless. Decisión del milestone: Wayland es la única plataforma, sin fallback X11.

## 7. Decisiones de diseño

- **Extensiones compiladas/estáticas** vía `enum Finder` + trait `Find`. Nada de `.so` ni subprocesos en este milestone.
- **Wayland única plataforma** (GTK4 + `gtk4-layer-shell`); sin X11.
- **Evolución incremental** del código existente (no reescritura), respetando OpenSpec + TDD.
- **SQLite desde el principio** (decisión del usuario), con `rusqlite` feature `bundled` (SQLite embebido, sin librería del sistema) y acceso detrás del trait `HistoryRepository` para desacoplar el ranking de la persistencia.
- **`item_id` = ruta canónica del `.desktop`** (estable entre sesiones); puede duplicar entradas si una app tiene varios `.desktop` (trade-off aceptado).
- **Caché de ids (`IdCache`, `src/models/item.rs`)**: canonicalizar la ruta es una syscall de E/S y el ranking resolvía el id de *todos* los ítems en *cada* pulsación. `IdCache` memoiza la canonicalización por `PathBuf` durante toda la vida de la ventana (una `IdCache` por sesión en `main.rs`), de modo que `rank_items` ya no canonicaliza por ítem y pulsación; solo se memorizan las resoluciones **correctas**, un fallo de E/S no se cachea y se reintenta cayendo al path literal (no es equivalencia 1:1 si un `.desktop` o su symlink desaparece a media sesión, pero es intencional). `load_all()` sigue siendo una única llamada por búsqueda, nunca por ítem.
- **`Css::load()` no reescribe `style.css` existente**: preserva la personalización del usuario; los estilos nuevos no llegan a instalaciones previas salvo borrar el fichero.
- **`Terminal=true` delegado a GLib**: no es testeable de forma hermética (requiere emulador en `PATH` y lanzaría un proceso interactivo), así que se verifica el dato declarado y la delegación documentada.

## 8. Verificación

Automática (no requiere display):

```bash
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

Cubre las piezas puras: ranking y pesos, historial/esquema/migraciones, motor de ejecución (fixtures en `TempDir` con `/usr/bin/env`), fila de resultado, navegación, banner de error y despacho del finder.

Manual (Wayland; sin display en CI):

- **`Terminal=true`:** crear un `.desktop` de prueba con `Terminal=true` y lanzarlo desde LiLa → debe abrirse dentro de un emulador de terminal (el emulador lo resuelve GLib vía GSettings/`$TERMINAL`).
- **Instancia única y foco:** `lila & lila; pgrep -c lila` debe dar `1`; la segunda invocación re-presenta la **misma** ventana con foco, campo vacío y sin banner obsoleto tras un error previo. Comprobar que una pérdida de foco real dentro del margen de 250 ms oculta la ventana, y que Home/End, wrap-around, PageUp/PageDown y scroll-to-selected funcionan.

## 9. Follow-ups y deuda aceptada

- **Rendimiento:** la canonicalización por ítem ya está cacheada (`IdCache`, ver §7). Queda como follow-up acotar la llamada a `load_all()` del historial (hoy una por búsqueda) si el historial crece mucho.
- **Eliminar `connect_window_removed` vacío** en `main.rs` (hoy es un `if app.windows().is_empty()` sin cuerpo).
- **`allow(dead_code)`** en accesores de `Item` (`exec`, `keywords`, `categories`) y en `HistoryRepository::usage`/`score` y `usage_score`; retirar al cablearlos o descartarlos.
- **Acción D-Bus** para el atajo global del compositor (tarea 5.3, diferida).
- **FTS5** sobre SQLite para extensiones futuras; índice de extensiones/plugins; multi-monitor y pantalla de ajustes (fuera del milestone).
