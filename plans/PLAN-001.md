# PLAN-001 — Core de lanzamiento espectacular (Milestone 1)

- **Issue:** https://github.com/atareao/lila/issues/5 (#5)
- **Estado:** PROPUESTO — pendiente de aprobación del usuario
- **Fecha:** 2026-11-14

## 1. Objetivo

Convertir LiLa de prototipo a un lanzador de aplicaciones sólido, rápido y agradable en Wayland, **antes** de añadir nuevas extensiones. Este plan es la base sobre la que se construirán los siguientes milestones.

## 2. Decisiones arquitectónicas fijadas

1. Extensiones **compiladas/estáticas**: se mantiene el enum `Finder` + trait `Find`. Nada de `.so` ni subprocesos en este milestone.
2. Plataforma única: **Wayland (Linux)** con `gtk4-layer-shell`. Sin fallback X11.
3. **Evolución incremental** del código actual (no reescritura). Se respeta el flujo OpenSpec + TDD.
4. Persistencia del historial de uso en **SQLite desde el principio** (decisión del usuario). Se accede detrás de un trait de repositorio para que la lógica de ranking no dependa de la implementación concreta.

## 3. Alcance

### IN
- Motor de ejecución de `.desktop` robusto.
- Filas de resultado con icono + nombre + subtítulo.
- Ranking inteligente: fuzzy + frecuencia + recencia, con pesos configurables.
- Historial de uso persistente en SQLite.
- Navegación de teclado completa.
- Instancia única correcta + re-present del dashboard.
- Tests unitarios de las piezas puras.

### OUT (planes futuros)
- Nuevas extensiones (calculadora, portapapeles, emoji, ficheros, web, controles de sistema).
- Arquitectura de plugins de terceros.
- Multi-monitor, animaciones, pantalla de ajustes.
- X13 / otras plataformas.

## 4. Mapa de arquitectura actual

- `src/main.rs` — UI GTK imperativa; `activate()` construye ventana/entry/listbox y conecta señales. App singleton por `APP_ID`.
- `src/models/` — `Config` (YAML), `Css`, `Extension`, `Item`.
- `src/finders/` — enum `Finder` + trait `Find` (`init`/`update`/`search`); despacho en `finders/finder.rs`; `ApplicationFinder` + `EmptyFinder`; `filter_items` (fuzzy, skim).
- `src/actions/mod.rs` — lanzamiento vía `gio::DesktopAppInfo`.
- `src/utils/constants.rs` — IDs y nombres GTK.

## 5. Persistencia en SQLite (decisión)

- **Crate:** `rusqlite` con la feature `bundled` (compila SQLite embebido, sin dependencia de librería del sistema; versión exacta a fijar al implementar). Justificación: coherencia con el objetivo "lanzador de verdad" y base lista para FTS en extensiones futuras.
- **Ubicación:** `~/.config/lila/history.db` (mismo `ProjectDirs` que `config.yml`).
- **Modo:** WAL para durabilidad y lecturas concurrentes.
- **Esquema inicial:**
  - Tabla `launcher_history(item_id TEXT PRIMARY KEY, launches INTEGER NOT NULL DEFAULT 0, first_used INTEGER NOT NULL, last_used INTEGER NOT NULL)`; tiempos en epoch millis.
  - `item_id` = identificador estable de la app (ruta canónica del fichero `.desktop`).
  - Índice sobre `last_used`.
- **Migraciones:** tabla `schema_version(version INTEGER)` y migraciones idempotentes al abrir la BD.
- **API (trait de repositorio):** `record(item_id)`, `score(item_id) -> f64` (frecuencia + recencia con decaimiento), `load_all()`. La implementación abre la conexión una vez y la mantiene detrás de un `Mutex` (acceso desde el hilo GTK).

## 6. Tareas atómicas

### Fase A — Modelo y ejecución
- [ ] **A1 — Enriquecer `Item`.** Añadir accesores: `id() -> String` (ruta canónica del `.desktop`), `icon_path() -> Option<&Path>`, `subtitle() -> Option<&str>`, `exec() -> Option<&str>`, `keywords()`/`categories()`. No romper la serialización en disco (`Item` deriva serde).
- [ ] **A2 — Motor de lanzamiento robusto.** Auditar `actions::launch`: cubrir field codes (`%f %F %u %U %i %c %k`), `Terminal=true`, `Path` (working dir), `TryExec` y activación DBus. Lanzamiento desacoplado del bucle GTK y **errores visibles** al usuario. Tests de parsing/validación de `Exec` y caso `.desktop` inexistente (mantener el existente).

### Fase B — Historial y ranking
- [ ] **B1 — Módulo `history` con SQLite.** Añadir dependencia `rusqlite` (feature `bundled`). Implementar apertura de BD, creación de esquema + `schema_version`, y el trait de repositorio (`record`/`score`/`load_all`). Registrar cada lanzamiento desde `actions`/UI. Tests de agregación de frecuencia y decaimiento por recencia.
- [ ] **B2 — Ranking combinado.** `final_score = w_fuzzy*fuzzy + w_freq*ln(1+lanzamientos) + w_recency*decay(último_uso)`, con pesos en `Config`. Nuevo `rank_items` (o `filter_items` extendido) ordena por score final, consultando el historial. Tests: query vacía → dominan frecuencia/recencia; el fuzzy puede adelantar a un ítem no usado.

### Fase C — UI
- [ ] **C1 — Filas con icono y subtítulo.** Sustituir `Label` por `ListBoxRow` con `Box` horizontal: `Image` (icono de la app) + `Box` vertical (nombre + subtítulo atenuado). Estilos en `Css::default()`, respetando `#lila-listbox`.
- [ ] **C2 — Teclado completo.** PageUp/PageDown, Home/End, wrap-around en Up/Down y `scroll-to-selected`. Mantener Escape (ocultar) y Enter (lanzar). Reservar Ctrl+Enter para acciones secundarias (follow-up).

### Fase D — Invocación
- [ ] **D1 — Instancia única.** Corregir el arranque: eliminar el early-return defectuoso previo a `application.run()`; garantizar que una segunda invocación del binario re-presenta la ventana existente. Opcional: acción DBus para atajo global del compositor.

### Fase E — Config, docs y verificación
- [ ] **E1 — `Config` ampliada.** Nuevos campos con `#[serde(default)]` (límite de resultados, pesos del ranking, historial on/off). La carga debe tolerar configs antiguas.
- [ ] **E2 — Documentación.** Crear `docs/core-lanzamiento.md` (qué es, ficheros, cómo funciona, config, esquema SQLite, decisiones, follow-ups) y actualizar `README.md` (instalación del `.desktop`, atajo global, ubicación de config y BD).
- [ ] **E3 — Verificación final.** `cargo fmt --check`, `cargo clippy -- -D warnings` (cero warnings), `cargo test` verde. Repaso contra los criterios de éxito del issue #5.

## 7. Escenarios de aceptación (BDD)

- **Buscar y lanzar.** Given una app instalada, When escribo parte de su nombre y pulso Enter, Then se lanza correctamente y su fila muestra icono y nombre.
- **Terminal.** Given una app con `Terminal=true`, When la lanzo, Then se abre en un emulador de terminal.
- **Ranking por uso.** Given dos apps que coinciden con la query, When una se ha lanzado muchas veces, Then aparece antes que la otra.
- **Recencia.** Given una app recién lanzada, When abro el dashboard sin escribir, Then aparece arriba.
- **Persistencia (SQLite).** Given uso de apps, When reinicio LiLa, Then el historial se conserva leyéndolo de `history.db`.
- **Instancia única.** Given LiLa abierto, When invoco el binario de nuevo, Then se re-presenta la misma ventana.
- **Navegación.** Given una lista de resultados, When pulso Down hasta el final, Then hay wrap-around y la fila queda visible.

## 8. Dependencias y requisitos

- Librería del sistema `gtk4-layer-shell` (ver `AGENTS.md`).
- Nueva crate: `rusqlite` con feature `bundled` (SQLite embebido). El resto ya está (serde, serde_yaml, gio, gtk).
- Entorno: compositor Wayland con layer-shell. Los tests no requieren display.

## 9. Flujo de trabajo (OpenSpec + TDD)

1. Aprobar este plan → crear el change de OpenSpec en `openspec/changes/core-lanzamiento/` con deltas de spec (contratos + escenarios).
2. Por cada tarea: RED → GREEN → REFACTOR → commit gitmoji en rama `feature/*`.
3. Contar con el subagente `sqlite-expert` para el diseño fino del esquema y las migraciones de B1.
4. Al cerrar: renombrar a `plans/PLAN-001-ARCHIVED.md` y `openspec archive`.

## 10. Follow-ups (no en este plan)

- Ctrl+Enter → acciones secundarias por resultado.
- FTS5 sobre SQLite para extensiones futuras (ficheros, portapapeles).
- Índice de extensiones y arquitectura de plugins.
- Multi-monitor y pantalla de ajustes.

## 11. Criterio de cierre

Todas las tareas marcadas, `cargo test`/`clippy` verdes, `docs/core-lanzamiento.md` escrito y escenarios BDD del apartado 7 verificados.