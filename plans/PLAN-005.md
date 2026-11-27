# PLAN-005 — Filtrado de consultas de un carácter (issue #39)

- Issue: https://github.com/atareao/lila/issues/39
- Estado: implementado; pendiente de verificación manual en Wayland y archivado
- Rama: `feature/short-query-filter` (desde `development`)

## Objetivo
Eliminar los falsos positivos producidos por `Categories` cuando el usuario teclea una consulta de un solo carácter, sin romper las coincidencias por `Name`/`Keywords` ni el comportamiento de consultas de 2+ caracteres (incluida la ruta con historial).

## Mapa arquitectónico
- Entrada: `src/main.rs:604` (lectura de `entry.text()`) y `src/main.rs:634` (`connect_changed`) → closure `refresh` (`src/main.rs:591-630`) → `rank_items` (`src/main.rs:609-617`).
- `rank_items`: `src/finders/ranking.rs:53-108`. Sin historial delega en `filter_items` (`:71-82`); con historial usa `best_fuzzy_match` (`:86-94`) y pondera frecuencia/recencia (`:96-103`).
- Punto de cambio compartido: `best_fuzzy_match` en `src/finders/application.rs:72-85`, usado por `filter_items` (`application.rs:50-64`) y por `rank_items`.
- Metadatos: `Item::MetadataCache::categories()` (`src/models/item.rs:212-220`), tokens divididos por `split_desktop_list()` (`item.rs:107-115`). `Comment`/`GenericName` solo se muestran en `subtitle()` (`item.rs:49-58`), no participan en búsqueda.
- Motor: `fuzzy-matcher 0.3.7`, `SkimMatcherV2::default()`, sin umbral ni normalización Unicode; se ordena por score descendente.
- Spec vigente a modificar: `openspec/specs/launcher/result-ranking/spec.md:38-64`.

## Decisión técnica
Opción (B): en `best_fuzzy_match`, si `query.trim().chars().count() == 1`, omitir los tokens de `categories` del iterador de coincidencias. Se usa `chars().count()` (no `len()`) por UTF-8. No se modifica la UI (`main.rs`) ni `Config` (`src/models/config.rs`). Se descarta la opción (A) de longitud mínima en el entry porque bloquearía búsquedas legítimas por nombre y no cubriría llamadas directas a `rank_items`/`filter_items`.

## Tareas (TDD; solo después de aprobar el proposal OpenSpec)
- [x] RED — Añadir tests en `src/finders/application.rs`: consulta de 1 carácter con coincidencia solo en `Categories` devuelve vacío; 1 carácter sigue matcheando por `Name`; 1 carácter sigue matcheando por `Keywords`; regresión de 2+ caracteres por `Categories` (`filter_matches_category_absent_from_name` ya cubre "spreadsheet").
- [x] RED — Añadir test en `src/finders/ranking.rs`: con historial habilitado, una query de 1 carácter no incluye una app cuyo único match es `Categories`.
- [x] RED — Ejecutar `cargo test` y verificar que fallan los tests nuevos y los 126 existentes siguen en verde.
- [x] GREEN — Implementar el guard en `best_fuzzy_match`; ejecutar `cargo test` y `cargo check`.
- [x] REFACTOR — `cargo fmt` y `cargo clippy --all-targets -- -D warnings`; re-ejecutar tests.
- [x] Spec delta redactada en `openspec/changes/short-query-filter/specs/launcher/result-ranking/spec.md` y validada con `openspec validate short-query-filter --strict`.
- [ ] Verificación manual en Wayland (`tasks.md` 2.1): `u` ya no inunda por `Categories`, `spreadsheet` sigue encontrando la aplicación; no automatizada sobre el escritorio del usuario (ver `docs/short-query-filter.md`).
- [ ] Archivar el change (`openspec archive short-query-filter`) tras la verificación manual; queda reflejado en `tasks.md` (2.3).
- [x] Documentar en `docs/short-query-filter.md` el cambio, las decisiones y los follow-ups.
- [ ] Marcar este plan como completado y renombrarlo a `PLAN-005-ARCHIVED.md` (tras verificación manual y archivado del change).
- [ ] PR a `development` con conventional commit + gitmoji (`✨ feat(finders): ...`).

## Criterios de verificación
- `cargo test` en verde (131/131: 126 iniciales + los nuevos).
- `cargo clippy --all-targets -- -D warnings` limpio.
- `cargo fmt --check` limpio.
- Verificación manual en Wayland: teclear `u` ya no produce coincidencia masiva por `Categories`; `process`, `decibels` y `monitor` siguen funcionando como en el issue #16.

## Fuera de alcance
- Issue #40 (ítems con `Name` vacío): se aborda después, en sesión separada.
- AUR (#38) queda aparcado por decisión del usuario.
