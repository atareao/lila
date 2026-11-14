# Handoff — lila — 2026-11-14 (actualizado)

## Objetivo
LiLa = lanzador de apps Wayland (GTK4 + gtk4-layer-shell). Milestone 1 `core-lanzamiento`: core de lanzamiento sólido antes de añadir extensiones. Issue #5 · Spec `openspec/changes/core-lanzamiento/` · Plan `plans/PLAN-001.md`.

## Estado
- **Hecho (44 tests verde, clippy `-D warnings` y fmt limpios):**
  - `d02958f` accesores de `Item`; `1c2f893` historial SQLite; `aac89b0` ranking; `7790206` correcciones de auditoría; `cbcb151` andamiaje SDD.
  - `e3fe01f` **4.1** filas `ListBoxRow` (icono + nombre + subtítulo) + view-model puro `ResultRow` (`src/ui/row.rs`). Retirado `allow(dead_code)` de `icon_path`/`subtitle`.
  - `aa75b92` **4.2** estilos por defecto de filas/selección en `Css::default()` (`result-name`, `result-subtitle`, `row:selected` + `background-image: none`).
  - `98f7c0b` **4.3** predicado puro `ui::results_visible(count)` + `refresh()` reutilizable y render inicial en `activate()`, de modo que la lista se rige solo por existencia de resultados y muestra el historial al abrir el dashboard.
- **En curso:** nada a medias; working tree limpio tras el commit de 4.3.
- **Rama actual:** `feature/core-lanzamiento` (base `development`). **Sin push al remoto.**

## Pendiente (misma rama)
Fase C restante, en orden:
- **4.4** Navegación PageUp/PageDown/Home/End + wrap-around en flechas (hoy Down/Up no envuelven ni hay Page/Home/End).
- **4.5** Scroll para mantener visible la fila seleccionada.
Luego **A2** (1.3/1.4/1.6 robustez de `actions::launch` + error visible), **D** (5.1–5.3 instancia única/re-present), **E** (6.2–6.4 docs/README/verificación final).

## Punto de partida
```bash
cd /data/rust/lila && git checkout feature/core-lanzamiento
cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check   # verde (41 tests)
```

## Metodología / reglas
- SDD + TDD por tarea (RED→GREEN→REFACTOR); marcar casillas en `openspec/changes/core-lanzamiento/tasks.md` y `plans/PLAN-001.md` (C1 ya marcada).
- Delegar código a subagentes; auditar con `@rust-reviewer` antes de commit. En esta sesión `@backend-dev` y `@rust-reviewer` funcionaron con fiabilidad.
- Commits gitmoji + conventional en `feature/*`; PR a `development`; nunca commit directo a `main`/`development`.
- Cierre de milestone: `openspec archive core-lanzamiento`, renombrar `plans/PLAN-001-ARCHIVED.md`, PR → `development`, cerrar #5.

## Riesgos y notas
- **Rendimiento (auditoría #2/#3, follow-up):** `rank_items` ejecuta `load_all()` y `Item::id()` (→ `canonicalize()`, syscall FS) por ítem y por pulsación; cachear antes de cerrar el milestone.
- **Estilos y config previa:** `Css::load()` solo escribe el default si `~/.config/lila/style.css` no existe → instalaciones previas no reciben los estilos nuevos sin borrar el fichero (comportamiento preexistente).
- **Deuda aceptada:** `#[allow(dead_code)]` en `exec()`/`keywords()`/`categories()` de `Item` y en `usage`/`score`/`usage_score` de `history`.
- **GLib descarta** un `.desktop` cuyo binario de `Exec` no exista: usar `/usr/bin/env` en fixtures de test.
- **Sin push:** considerar `git push -u origin feature/core-lanzamiento`.
