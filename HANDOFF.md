# Handoff — lila — 2026-11-14 — sesión cerrada, retomar por Fase E

## Objetivo
LiLa = lanzador de apps Wayland (GTK4 + gtk4-layer-shell). Milestone 1 `core-lanzamiento`: core de lanzamiento sólido antes de añadir extensiones. Issue #5 · Spec `openspec/changes/core-lanzamiento/` · Plan `plans/PLAN-001.md`.

## Estado
- **Hecho (79 tests verde, clippy `-D warnings`, fmt OK):**
  - `0f01fd5` **Fase D / 5.1–5.2** (issue #9, cerrado): early-return de `main()` antes de `application.run()` eliminado; unicidad por GApplication; `Config::load`/`finder.init()`/`SqliteHistory::open` movidos a `connect_startup` (la instancia secundaria no toca BD ni escanea `.desktop`); re-presentación con `grab_focus`, guard de foco por generación (`Cell<u64>`) con re-verificación a 250 ms y limpieza de banner+entry solo si la ventana estaba oculta; `ui::error::ErrorBanner` puro (`Option<String>`, 6 tests) y `sync_label` sin préstamos del `RefCell` a través de GTK.
  - `57dcbc9` **docs**: escenarios nuevos en `specs/launcher/invocation` (terminación de la secundaria, dependencia del bus de sesión, foco con margen re-verificado, banner conservado con ventana visible) + `tasks.md` (1.4, 5.1, 5.2 marcadas) + `plans/PLAN-001.md` D1 marcada.
  - `0e2cab3` **docs(handoff)**: cierre de Fase D. Working tree limpio en `feature/core-lanzamiento`.
- **Rama actual:** `feature/core-lanzamiento` (base `development`). **Sin push al remoto.**

## Pendiente (misma rama)
### Siguiente acción (retomar aquí)
- Abrir el issue de la **Fase E** (docs + verificación) asignado a `atareao` y referenciado desde #5.
- Orden de trabajo: 6.2 `docs/core-lanzamiento.md` → 6.3 `README.md` → 6.4 verificación final → cachear `rank_items` (riesgo de rendimiento) → `openspec archive core-lanzamiento` + renombrar `plans/PLAN-001-ARCHIVED.md` + PR a `development` + cerrar #5.
- Decisión pendiente del usuario: `git push -u origin feature/core-lanzamiento` (hoy sin push).

- **5.3** acción DBus para atajo global del compositor: opcional, **diferida como follow-up**, no hecha.
- **Fase E (6.2–6.4):** `docs/core-lanzamiento.md` (con `Terminal=true` como verificación **manual**), `README.md` y verificación final.
- **Cierre de milestone:** `openspec archive core-lanzamiento`, renombrar a `plans/PLAN-001-ARCHIVED.md`, PR a `development`, cerrar #5.

## Punto de partida
```bash
cd /data/rust/lila && cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check
```

## Metodología / reglas
- SDD + TDD por tarea (RED→GREEN→REFACTOR); issue por tarea asignado a `atareao` y referenciado desde #5; marcarlo cerrado al commitear.
- Auditar con `@rust-reviewer` antes de commit; commits gitmoji + conventional en `feature/*`; PR a `development`; nunca commit directo a `main`/`development`.

## Riesgos y notas
- **Rendimiento:** `rank_items` llama `load_all()` y `Item::id()` (`canonicalize`) por ítem y pulsación → cachear antes de cerrar el milestone.
- `Css::load()` no reescribe `style.css` existente (los estilos nuevos no llegan a instalaciones previas salvo borrando el fichero).
- **Deuda aceptada:** `allow(dead_code)` en accesores de `Item` y en `usage`/`score`/`usage_score`.
- GLib descarta `.desktop` con binario inexistente → usar `/usr/bin/env` en fixtures.
- `connect_window_removed` vacío en `main.rs`: eliminar en Fase E.
- **Verificación manual pendiente (sin Wayland en CI):** `lila & lila` → `pgrep -c lila` = 1; ventana re-presentada con foco y campo vacío; banner limpio tras error previo; pérdida de foco real dentro del margen de 250 ms oculta la ventana. Además Home/End, wrap-around, PageUp/PageDown y scroll-to-selected (heredado de 4.4/4.5).