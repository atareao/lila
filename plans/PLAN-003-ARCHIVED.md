# PLAN-003 — Deuda técnica post-M1 (issue #16)

- **Issue:** #16 (asignado a `atareao`)
- **Change OpenSpec:** `openspec/changes/deuda-tecnica-post-m1/`
- **Estado:** completado — fusionado en `development` (PR #30/#31); comprobación manual en Wayland superada el 2026-11-15 (niri): vacío ≈13-16 filas, `process` → btop++, `decibels` → Reproductor de sonido, `monitor` → btop++, `u` casa con casi todo (follow-up ya documentado)
- **Contexto:** LiLa (launcher Wayland GTK4 + gtk4-layer-shell, Rust edition 2024). Tras el Milestone 1 (`plans/PLAN-001-ARCHIVED.md`) y el follow-up 5.3 (`plans/PLAN-002.md`, atajo D-Bus), la release `v0.2.0` está publicada y `main == development`. Queda deuda técnica acumulada del core: `#[allow(dead_code)]` sin justificación, metadatos de `.desktop` (`Keywords`/`Categories`) calculados pero no usados en la búsqueda, política de carga del CSS sin especificar y `history.load_all()` ejecutándose en cada pulsación.

## Objetivo
Cerrar la deuda técnica de #16 en cuatro puntos (A, A2, B, C) sin cambiar el comportamiento observable de la búsqueda salvo donde A2 lo requiere explícitamente, dejando la puerta de calidad en verde (`cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`).

## Alcance (IN)

### A — Retirar `#[allow(dead_code)]` inservibles (sin cambio de comportamiento)
1. `Item::exec()` — `src/models/item.rs:48-54`. Sin consumidor en producción (el lanzamiento va por `gio::DesktopAppInfo`; solo lo usaban los tests). **Decisión propuesta:** eliminar el método y reescribir sus dos tests como tests de `icon_path()` (`icon_path_returns_expected_values`, `icon_path_is_none_when_absent`, sustituyendo `icon_path_and_exec_return_expected_values` y `icon_path_and_exec_are_none_when_absent`).
2. `Finder::update()` — `src/finders/finder.rs:27-33`. El envoltorio del enum no lo llama nadie (`src/main.rs` usa `Finder::init()`; `ApplicationFinder::init()` ya llama a su propio `update()`). **Decisión propuesta:** eliminar el método envoltorio del enum `Finder`; se conserva `Find::update()` en el trait (`src/finders/finder.rs:51`) porque lo implementan y usan `ApplicationFinder::init` y `EmptyFinder`.
3. `split_desktop_list()` — `src/models/item.rs:100-110`. Pasa a usarse en A2 → retirar `#[allow(dead_code)]` y el comentario `TODO(core-lanzamiento)`.
4. `Item::keywords()` / `Item::categories()` — `src/models/item.rs:69-97`. Pasan a usarse en A2 → retirar `#[allow(dead_code)]` y el `TODO` asociado.
5. **Fuera de alcance de A (se mantienen, con su comentario justificativo):** los tres `#[allow(dead_code)]` de `src/history/mod.rs` (`usage()`:113, `score()`:122, `usage_score()`:157). Forman parte del contrato del trait `HistoryRepository` (los implementan todos los repos, incluidos los de test) y se consumen desde los tests del propio módulo. Retirarlos exigiría recortar el trait; se documenta la razón y no se toca.

### A2 — Cablear `Keywords`/`Categories` a la búsqueda (cambio de comportamiento: REQUIERE SPEC)
Hoy `filter_items()` (`src/finders/application.rs:38`) y `rank_items()` (`src/finders/ranking.rs:46`) solo puntúan contra `Item::name()`. Objetivo: que la consulta pueda coincidir también con las palabras clave y las categorías del `.desktop`, sin penalizar la coincidencia por nombre.

- `Item::keywords()`/`categories()` abren el `.desktop` con `DesktopAppInfo::from_filename()` en cada llamada → sería E/S por ítem y por pulsación. **Decisión propuesta:** añadir una caché de metadatos por ruta, análoga a `IdCache` (`src/models/item.rs:134`), con la misma política (memoriza solo resoluciones correctas; un fallo no se memoriza y se reintenta). Nombre propuesto: `MetadataCache` con `keywords(&Item) -> Vec<String>` y `categories(&Item) -> Vec<String>`, exportada desde `src/models/mod.rs` y construida una vez en `src/main.rs` (`src/main.rs:391`).
- **Puntuación propuesta:** el término difuso de un ítem es el **máximo** de las puntuaciones difusas contra `name`, `keywords` y `categories`. Un ítem que no coincide por ninguno se descarta (consulta no vacía). La coincidencia por nombre no se penaliza ni se premia respecto a la de una keyword: se toma el mejor resultado (decisión a confirmar en la aprobación de la spec).
- La afirmación "el historial completo se carga una sola vez por búsqueda, nunca por ítem" (`src/finders/ranking.rs:42`) se mantiene.

### B — Política de `Css::load()` (REQUIERE SPEC)
Hoy `Css::load()` (`src/models/css.rs:10-35`) no tiene política declarada: devuelve `Result<CssProvider, Box<dyn Error>>`, propaga el fallo de `create_dir_all`, no distingue el resultado de `load_from_path`, llama a `Css::default()` dos veces (dos asignaciones) y el punto de llamada hace `.expect("Can not load CSS file")` (`src/main.rs:660`), de modo que un problema de permisos en `~/.config/lila/` mata la aplicación.

Política a especificar y aplicar:
- **Nunca abortar por CSS:** `Css::load()` deja de devolver `Result` y devuelve siempre un `CssProvider` (el fallo se registra con `tracing::warn!`, no se propaga ni hace panic).
- **Precedencia:** si `~/.config/lila/style.css` existe → se carga ese fichero; si su lectura falla o no es utilizable → se cae al CSS embebido por defecto **en memoria**, sin tocar el fichero del usuario.
- **Sin sobrescrituras:** si el fichero existe, **nunca** se sobrescribe. Si no existe → se crea el directorio XDG si hace falta y se escribe el CSS por defecto; si la escritura falla, se registra un `warn` y se carga el CSS por defecto en memoria.
- **Sin directorios XDG** (`ProjectDirs::from` → `None`): se carga el CSS embebido por defecto, sin error.
- `Css::default()` se evalúa una sola vez por ejecución (eliminar la doble llamada).
- Extraer la decisión a una función pura testeable sin display (entrada: si el fichero existe / resultado de su lectura; salida: de dónde se carga el CSS) para poder cubrirla con tests en CI.

### C — Cachear `load_all()` entre pulsaciones (sin cambio de ranking; tests de regresión)
Hoy `rank_items()` llama a `history.load_all()` en cada pulsación (`src/finders/ranking.rs:69`), lo que implica una consulta SQLite completa por tecla (`src/history/mod.rs:289`).

- **Decisión propuesta:** un decorador `CachedHistory` (módulo nuevo `src/history/cache.rs`, o submódulo de `src/history/mod.rs`) que envuelva cualquier `HistoryRepository`, memorice el resultado de `load_all()` y **invalide la caché en `record()`**. `usage()` delega sin caché.
- `src/main.rs` construye el repositorio envuelto (`SqliteHistory` → `CachedHistory`) y lo comparte con `record_launch()` (`src/main.rs:131`) y con el ranking (`src/main.rs:592`), de modo que un lanzamiento invalida la caché del propio proceso.
- Tests unitarios con un repositorio contador: N búsquedas → 1 `load_all()`; tras un `record()` → 1 recarga más. Un `load_all()` fallido **no** se memoriza (siguiente intento vuelve a consultar).
- El orden y las puntuaciones resultantes deben ser idénticos a los actuales: es una optimización, no un cambio de ranking.

## Fuera de alcance (OUT)
- Los `#[allow(dead_code)]` de `src/history/mod.rs` (ver A.5).
- Cambios de pesos, de fórmula de puntuación o de `max_results`.
- Edición o recarga en caliente del CSS del usuario (solo se define su política de carga al arrancar).
- Milestone 2 y cualquier nueva vía de invocación.
- Publicación en crates.io.

## Ficheros afectados
- `src/models/item.rs` (A: eliminar `exec()` y ajustar tests; retirar 2 `allow`; A2: mantener `keywords`/`categories`, añadir `MetadataCache`).
- `src/models/mod.rs` (exportar `MetadataCache`).
- `src/finders/finder.rs` (A: eliminar `Finder::update()`).
- `src/finders/application.rs` (A2: `filter_items` puntúa contra name + keywords + categories).
- `src/finders/ranking.rs` (A2: término difuso = máximo sobre los tres ámbitos).
- `src/models/css.rs` (B: política + función pura + tests).
- `src/history/mod.rs` (C: `CachedHistory` o módulo hermano) y `src/history/` si se crea `cache.rs`.
- `src/main.rs` (A2: construir `MetadataCache`; B: quitar el `.expect` de CSS; C: envolver el repositorio).

## Fases
- [x] **E1 — Especificación.** Change `deuda-tecnica-post-m1` con proposal/design/tasks + deltas en `launcher/result-ranking` (A2) y `launcher/results-ui` (B). A y C sin delta de spec (no cambian comportamiento observable); `openspec validate` en verde. **STOP: aprobación del usuario.**
- [x] **E2 — A (limpieza).** TDD-test-first donde aplique; sin cambios de comportamiento: `cargo test` (base 92 tests, los renombrados de A al mismo recuento) en verde; clippy sin `allow` residuales entre los cuatro retirados.
- [x] **E3 — A2 (feature).** RED→GREEN→REFACTOR: tests de `MetadataCache` (memoiza cada ruta, reintenta tras fallo) y de la coincidencia por keywords/categories en `filter_items`/`rank_items`; implementación; clippy/fmt en verde.
- [x] **E4 — B (política CSS).** RED→GREEN→REFACTOR: tests de la función pura de decisión (existe/lee mal/no existe/escribible/no escribible/sin XDG) y de que `Css::load()` no hace panic ni devuelve `Result`; quitar el `.expect` del punto de llamada.
- [x] **E5 — C (caché de historial).** RED→GREEN→REFACTOR: tests del decorador (1 `load_all` para N búsquedas, invalidación en `record`, no se memoriza el fallo) y de que el ranking no cambia; cableado en `main.rs`.
- [x] **E6 — Verificación y cierre.** (en curso: docs escritas, auditoría cerrada, change archivado y PR abierto a `development`; comprobación manual en Wayland superada el 2026-11-15 y plan archivado) `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` en verde; `docs/deuda-tecnica-post-m1.md`; auditoría `@rust-reviewer`; `openspec archive deuda-tecnica-post-m1`; PR de `feature/deuda-tecnica-post-m1` a `development`; cerrar #16; marcar PLAN-003 como completado y renombrarlo a `plans/PLAN-003-ARCHIVED.md`.

## Escenarios de aceptación (BDD)
- **Coincidencia por palabra clave.** Given una aplicación cuyo `.desktop` declara `Keywords=photos;image;editor;` y cuyo nombre no contiene ninguna, When el usuario escribe `photo`, Then la aplicación aparece como resultado.
- **Coincidencia por categoría.** Given una aplicación con `Categories=Utility;Editor;`, When el usuario escribe `editor`, Then la aplicación aparece como resultado.
- **El nombre sigue mandando.** Given dos aplicaciones, una que coincide por nombre y otra solo por keyword, When se escribe la consulta de ambas, Then ninguna queda descartada y el orden respeta las puntuaciones (la coincidencia por nombre no se penaliza).
- **Coste de metadatos.** Given N pulsaciones sobre la misma lista de aplicaciones, When se busca, Then cada `.desktop` se lee como máximo una vez por sesión (comprobable con la caché de metadatos).
- **CSS ilegible.** Given `~/.config/lila/style.css` existe pero no se puede leer, When LiLa arranca, Then el dashboard usa el estilo por defecto y no aborta ni sobrescribe el fichero.
- **CSS ausente.** Given `~/.config/lila/style.css` no existe, When LiLa arranca por primera vez, Then se escribe el estilo por defecto y se aplica.
- **Directorio no escribible.** Given el directorio de configuración no se puede crear ni escribir, When LiLa arranca, Then se aplica el estilo por defecto en memoria, LiLa sigue funcionando y solo queda un `warn` en el log.
- **Una carga de historial por pulsación.** Given un repositorio de historial válido, When el usuario teclea 10 caracteres, Then `load_all()` se ejecuta una sola vez; When lanza una aplicación y vuelve a teclear, Then se recarga una sola vez más.

## Riesgos y notas
- **A vs A2 están acoplados:** los `allow` de `keywords`/`categories`/`split_desktop_list` solo se pueden retirar cuando A2 los consume. Si A2 se aplaza, A debe limitarse a `exec()`, `Finder::update()` y su razón documentada.
- **A2 es el único punto con cambio observable.** Requiere spec y decisión explícita sobre la precedencia nombre vs keywords/categorías (propuesta: máximo, sin penalización).
- **A2 y escrituras en disco:** la caché de metadatos debe compartir la política de `IdCache` (no memorizar fallos) para que una app instalada después se resuelva sin reiniciar.
- **B cambia una firma pública interna** (`Css::load() -> CssProvider` en lugar de `Result`). Es crate binario, sin consumidores externos.
- **C y el reloj:** la memorización de `load_all()` no debe alterar `now_millis`, que se calcula por búsqueda en `main.rs`; solo se cachean los datos de uso.
- Base de calidad al empezar: **92 tests** en verde, clippy y fmt limpios. Ningún punto puede bajar ese recuento (los tests renombrados de A cuentan igual).

## Metodología
SDD + TDD por punto; issue #16 asignado a `atareao`; un punto por commit en la rama `feature/deuda-tecnica-post-m1`; auditoría `@rust-reviewer` antes del PR; commits gitmoji + conventional; PR a `development`; nunca commit directo a `main`/`development`.