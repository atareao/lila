# Handoff — lila — 2026-11-14 (actualizado)

## Objetivo
LiLa = lanzador de apps Wayland (GTK4 + gtk4-layer-shell). Milestone 1 `core-lanzamiento`: core de lanzamiento sólido antes de añadir extensiones. Issue #5 · Spec `openspec/changes/core-lanzamiento/` · Plan `plans/PLAN-001.md`.

## Estado
- **Hecho (67 tests verde, clippy `-D warnings` y fmt limpios; Fase C/UI completa + 1.3):**
  - `d02958f` accesores de `Item`; `1c2f893` historial SQLite; `aac89b0` ranking; `7790206` correcciones de auditoría; `cbcb151` andamiaje SDD.
  - `e3fe01f` **4.1** filas `ListBoxRow` (icono + nombre + subtítulo) + view-model puro `ResultRow` (`src/ui/row.rs`). Retirado `allow(dead_code)` de `icon_path`/`subtitle`.
  - `aa75b92` **4.2** estilos por defecto de filas/selección en `Css::default()` (`result-name`, `result-subtitle`, `row:selected` + `background-image: none`).
  - `98f7c0b` **4.3** predicado puro `ui::results_visible(count)` + `refresh()` reutilizable y render inicial en `activate()`, de modo que la lista se rige solo por existencia de resultados y muestra el historial al abrir el dashboard.
  - `bd8e704` **4.4** módulo puro `ui::navigation` (`Move`/`target_index`/`PAGE_STEP`); wrap-around en Down/Up, PageUp/PageDown (±10) y Home/End. `EventControllerKey` en `PropagationPhase::Capture` (intercepta Home/End antes del `GtkText` interno del `Entry`); con modificadores se deja pasar la edición de texto.
  - `0605f19` **4.5** `ui::navigation::scroll_value` puro + handler `row-selected` que ajusta la `vadjustment` para mantener visible la fila seleccionada. **Cierra la Fase C (UI)** → `C2` del plan marcada.
  - `cf31b35` **1.3** (issue #6, cerrado) `actions::launch` devuelve `LaunchError` tipado (`MissingDesktopFile`/`NotLaunchable`/`Spawn`); field codes, `Terminal`, `Path`, `TryExec` y DBus delegados en `GDesktopAppInfo::launch_uris` (verificado en fuente GLib). Tests con fixtures en `TempDir`.
- **En curso:** nada a medias; working tree limpio tras el commit de 1.3.
- **Rama actual:** `feature/core-lanzamiento` (base `development`). **Sin push al remoto.**

## Pendiente (misma rama)
Fase C (UI) completa y 1.3 hecha. Pendiente en orden:
- **1.4** (issue nuevo al iniciar) mostrar el error de lanzamiento en la UI y mantener el dashboard abierto.
- **1.6** (issue nuevo al iniciar) tests puros de parsing de `Exec`; `Terminal=true` queda delegado a GLib (no testable hermético).
- **D** instancia única / re-present (5.1–5.3).
- **E** docs/README/verificación final (6.2–6.4).

## Punto de partida
```bash
cd /data/rust/lila && git checkout feature/core-lanzamiento
cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check   # verde (67 tests)
```

## Metodología / reglas
- SDD + TDD por tarea (RED→GREEN→REFACTOR); marcar casillas en `openspec/changes/core-lanzamiento/tasks.md` y `plans/PLAN-001.md` (C1 y C2 ya marcadas).
- Delegar código a subagentes; auditar con `@rust-reviewer` antes de commit. En esta sesión `@backend-dev` y `@rust-reviewer` funcionaron con fiabilidad.
- Commits gitmoji + conventional en `feature/*`; PR a `development`; nunca commit directo a `main`/`development`.
- **Política de issues (desde esta sesión):** abrir un issue por cada tarea que se inicie, asignado a `atareao` y referenciado desde #5; cerrarlo al commitear la tarea.
- Cierre de milestone: `openspec archive core-lanzamiento`, renombrar `plans/PLAN-001-ARCHIVED.md`, PR → `development`, cerrar #5.

## Riesgos y notas
- **Rendimiento (auditoría #2/#3, follow-up):** `rank_items` ejecuta `load_all()` y `Item::id()` (→ `canonicalize()`, syscall FS) por ítem y por pulsación; cachear antes de cerrar el milestone.
- **Estilos y config previa:** `Css::load()` solo escribe el default si `~/.config/lila/style.css` no existe → instalaciones previas no reciben los estilos nuevos sin borrar el fichero (comportamiento preexistente).
- **Deuda aceptada:** `#[allow(dead_code)]` en `exec()`/`keywords()`/`categories()` de `Item` y en `usage`/`score`/`usage_score` de `history`.
- **GLib descarta** un `.desktop` cuyo binario de `Exec` no exista: usar `/usr/bin/env` en fixtures de test.
- **Verificación manual pendiente (4.4/4.5, sin Wayland en CI):** confirmar en runtime que Home/End navegan (con `Capture`), Up/Down envuelven, PageUp/PageDown ±10 acotan, que el scroll mantiene visible la fila seleccionada al navegar/filtrar, y que la edición de texto (`Ctrl+A/Z`, `Shift+Home/End`) sigue intacta.
- **Sin push:** considerar `git push -u origin feature/core-lanzamiento`.
