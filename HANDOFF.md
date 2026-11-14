# Handoff — lila — 2026-11-14 09:26

## Objetivo
Convertir LiLa en un lanzador de apps espectacular en Wayland. Milestone 1 (`core-lanzamiento`): core de lanzamiento sólido (ejecución robusta, ranking inteligente, UI con iconos, instancia única) antes de añadir extensiones. Issue #5 · Spec `openspec/changes/core-lanzamiento/` · Plan `plans/PLAN-001.md`.

## Estado
- **Hecho (32 tests verde, clippy `-D warnings` y fmt limpios):**
  - `d02958f` Item con accesores (`id/icon_path/exec/subtitle/keywords/categories`).
  - `1c2f893` historial en SQLite (`rusqlite` bundled, WAL, esquema versionado, trait `HistoryRepository`).
  - `aac89b0` ranking combinado difuso+frecuencia+recencia (`src/finders/ranking.rs`); `Config` ampliada; cableado en `main.rs`.
  - `7790206` correcciones de la auditoría (`max_results` en fallback, `now_millis()`, `saturating_sub`, warn de esquema).
  - `cbcb151` andamiaje SDD (issue, plan, OpenSpec, `.opencode/`).
- **En curso:** nada a medias; working tree limpio.
- **Rama actual:** `feature/core-lanzamiento` (base: `development`, nueva, sale de `main`).
- **Working tree:** limpio. Sin push al remoto todavía.

## Pendiente
Siguiente tarea: **Fase C (4.1) — filas de resultado con icono + nombre + subtítulo** (`ListBoxRow` con `Image` + `Box` vertical), que además consume los `#[allow(dead_code)]` pendientes de `Item`.

## Punto de partida
```bash
cd /data/rust/lila
git checkout feature/core-lanzamiento
cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check   # deben estar verdes
```
Resto del milestone, en orden: **A2** (1.3/1.4/1.6: robustez de `actions::launch`, error visible al usuario, tests de `Exec`/`Terminal`) → **C** (4.1–4.5: iconos/subtítulo, visibilidad con consulta vacía, teclado PageUp/Down/Home/End + wrap-around + scroll-to-selected) → **D** (5.1–5.3: instancia única + re-present) → **E** (6.2–6.4: `docs/core-lanzamiento.md`, README, verificación final).
Al cerrar: `openspec archive core-lanzamiento`, renombrar a `plans/PLAN-001-ARCHIVED.md`, PR `feature/core-lanzamiento` → `development`, cerrar #5.

## Metodología / reglas
- SDD + TDD: cada tarea RED → GREEN → REFACTOR. Marca casillas en `openspec/changes/core-lanzamiento/tasks.md` y `plans/PLAN-001.md`.
- Commits gitmoji + conventional; rama `feature/*` desde `development`; PR a `development`; nunca commit directo a `main`/`development`.
- Delegar la escritura de código a subagentes; auditar con `@rust-reviewer`. Requiere `gtk4-layer-shell`/GTK4 del sistema (ver `AGENTS.md`).

## Riesgos y notas
- **Rendimiento (auditoría #2/#3, follow-up):** `rank_items` ejecuta `load_all()` y `Item::id()` (→ `canonicalize()`, syscall FS) **por ítem y por pulsación**; cachear `id` y el `HashMap<id, Usage>` fuera del hot path antes de cerrar el milestone.
- **Deuda aceptada:** `#[allow(dead_code)]` en accesores de `Item` (los consume la Fase C) y en `usage`/`score`/`usage_score` de `history` (API del trait aún sin uso en producción).
- **GLib descarta** un `.desktop` cuyo binario de `Exec` no exista (`from_filename` → `None`): cuidado en fixtures de test.
- **Subagentes:** en esta sesión `@backend-dev` ejecutó con fiabilidad; `@rust-writer` y `@sqlite-expert` devolvieron vacío/no ejecutaron. Si un subagente no aplica cambios, verificar `git status` y reintentar o reasignar.
- **Sin push:** la rama no está en remoto; considerar `git push -u origin feature/core-lanzamiento`.
