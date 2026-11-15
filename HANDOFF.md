# Handoff — lila — 2026-11-15 — Milestone 1 cerrado, PR #11 en revisión

## Objetivo
LiLa = lanzador de apps Wayland (GTK4 + gtk4-layer-shell). Milestone 1 `core-lanzamiento` COMPLETADO.

## Estado
- Fase E finalizada en 3 commits sobre `feature/core-lanzamiento`:
  - `50aa5a1` ♻️ refactor(ranking): memoización de canonicalización con `IdCache` (nueva firma `rank_items(items, ids: &IdCache, ...)`) + eliminado callback vacío `connect_window_removed` de `main.rs`.
  - `4f07a98` 📝 docs: `docs/core-lanzamiento.md` nuevo + `README.md` ampliado + nota de diseño de la caché.
  - `a76aa06` 📝 docs(openspec): `openspec archive core-lanzamiento` (→ `openspec/changes/archive/2026-11-15-core-lanzamiento/` + specs en `openspec/specs/launcher/`) y `plans/PLAN-001.md` → `plans/PLAN-001-ARCHIVED.md` (E1–E3 marcadas).
- Verificación: `cargo test` 87 verde · `cargo clippy --all-targets -- -D warnings` 0 warnings · `cargo fmt --check` OK. Auditoría `@rust-reviewer`: APROBADO.
- Publicado: `development` creado en origin (no existía); rama `feature/core-lanzamiento` pusheada; **PR #11** a `development`. Issues **#10** y **#5** CERRADOS.

## Pendiente / siguiente
- **Fusionar PR #11** `feature/core-lanzamiento` → `development` (revisión). Tras el merge, borrar la rama feature.
- **Verificación manual pendiente (sin Wayland en CI):** `lila & lila` → `pgrep -c lila` = 1; re-presentación con foco y campo vacío; banner limpio tras error; pérdida de foco real dentro del margen de 250 ms oculta la ventana. Además Home/End, wrap-around, PageUp/PageDown y scroll-to-selected.
- **Follow-up diferido:** 5.3 acción DBus para atajo global del compositor (opcional).

## Riesgos / deuda aceptada
- `IdCache` memoiza solo canonicalizaciones correctas; un fallo de E/S se reintenta y cae al path literal (documentado en `docs/core-lanzamiento.md` §7).
- `Css::load()` no reescribe `style.css` existente (estilos nuevos no llegan a instalaciones previas salvo borrar el fichero).
- `allow(dead_code)` en accesores de `Item` y en `usage`/`score`/`usage_score`.
- GLib descarta `.desktop` con binario inexistente → usar `/usr/bin/env` en fixtures.

## Punto de partida
```bash
cd /data/rust/lila && cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check
```
