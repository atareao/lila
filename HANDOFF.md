# HANDOFF — LiLa · feature `core-lanzamiento`

> Relevo de sesión. Estado en ficheros, no en la conversación.
> Última actualización: 2026-11-14

## Dónde estamos

- **Rama:** `feature/core-lanzamiento` (creada desde `development`, que a su vez sale de `main`). `development` es nueva y está vacía respecto a `main`.
- **Issue:** https://github.com/atareao/lila/issues/5
- **Plan:** `plans/PLAN-001.md`
- **Spec (OpenSpec):** `openspec/changes/core-lanzamiento/` (proposal, design, tasks, 7 spec deltas) — validada con `openspec validate core-lanzamiento --strict`.
- **Estado:** 32 tests en verde · `cargo clippy --all-targets -- -D warnings` limpio · `cargo fmt --check` limpio.

## Commits de esta feature (sobre `main`)

```
7790206 🐛 fix(ranking): respeta max_results en las rutas de fallback
aac89b0 ✨ feat(ranking): ranking combinado difuso + frecuencia + recencia
1c2f893 ✨ feat(history): historial de uso en SQLite con esquema versionado
d02958f ✨ feat(item): añade accesores de Item y subtítulo derivado del .desktop
cbcb151 🔧 chore(openspec): incorpora flujo SDD, plan del core y convenciones del proyecto
```

## Hecho

- **A1** — Accesores en `Item`: `id()` (ruta canónica del `.desktop`), `icon_path()`, `exec()`, `subtitle()`, `keywords()`, `categories()`.
- **B1** — Módulo `src/history/`: SQLite (`rusqlite` bundled, **WAL**, esquema `launcher_history` + `schema_version`, migraciones idempotentes), trait `HistoryRepository` (`record`/`usage`/`score`/`load_all`) con decaimiento de 14 días.
- **B2** — `src/finders/ranking.rs::rank_items` (fuzzy + frecuencia + recencia, pesos y `max_results` en `Config`); cableado en `main.rs` (abre historial con fallback, registra lanzamientos, ordena en vivo). `Config` ampliada tolerante a YAML antiguo.
- **Revisión** de `@rust-reviewer` aplicada: `max_results` respetado en todas las rutas, `now_millis()` con cast comprobado, `saturating_sub` en el decaimiento, `warn` ante esquema futuro, código muerto eliminado.

## Pendiente (continuar el milestone)

- **A2 (1.3/1.4/1.6)** — Robustez de `actions::launch`: field codes, `Terminal=true`, `Path`, `TryExec`, activación DBus; **error visible al usuario** (hoy solo log); tests de parsing de `Exec`.
- **C (4.1–4.5)** — UI: filas `ListBoxRow` con **icono + nombre + subtítulo** (consume los `allow(dead_code)` de `Item`); visibilidad con consulta vacía; teclado completo (PageUp/PageDown/Home/End, wrap-around, scroll-to-selected).
- **D (5.1–5.3)** — Instancia única: eliminar el early-return defectuoso previo a `application.run()` y garantizar re-present.
- **E (6.2–6.4)** — `docs/core-lanzamiento.md`, actualizar `README.md`, verificación final.
- **Follow-up de rendimiento** (auditoría #2/#3): `rank_items` hace `load_all()` + `Item::id()` (que llama `canonicalize()`, syscall FS) **por ítem y por pulsación de tecla**, y clona `Config` completo. Cachear `id` y el `HashMap<id, Usage>` fuera del hot path.
- **Deuda aceptada:** `#[allow(dead_code)]` en accesores de `Item` (Fase C) y en `usage`/`score`/`usage_score` de `history` (API del trait aún sin consumir en producción).

## Cómo continuar

1. `cargo test` / `cargo clippy --all-targets -- -D warnings` / `cargo fmt --check` deben seguir verdes.
2. Cada tarea: **RED → GREEN → REFACTOR**; commit gitmoji en `feature/*`; marcar en `openspec/changes/core-lanzamiento/tasks.md` y `plans/PLAN-001.md`.
3. Al terminar todo: `openspec archive core-lanzamiento`, renombrar el plan a `plans/PLAN-001-ARCHIVED.md`, cerrar el issue #5 y PR de `feature/core-lanzamiento` → `development`.

## Notas de entorno

- Requiere `gtk4-layer-shell` (y GTK4) instalados; ver `AGENTS.md`. Los tests no necesitan display.
- GLib **descarta** un `.desktop` cuyo binario de `Exec` no exista (`DesktopAppInfo::from_filename` → `None`): tenlo en cuenta en fixtures.
