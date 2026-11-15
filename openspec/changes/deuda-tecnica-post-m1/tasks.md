# Tasks

## 1. A — Limpieza de `#[allow(dead_code)]` (sin cambio de comportamiento)

- [ ] 1.1 (RED) Reescribir los tests de `src/models/item.rs` que usan `exec()` como tests de solo `icon_path()` (`icon_path_returns_expected_values`, `icon_path_is_none_when_absent`); `cargo test` falla al no existir ya las aserciones sobre `exec()`.
- [ ] 1.2 (GREEN) Eliminar `Item::exec()` y ejecutar `cargo test` + `cargo clippy --all-targets -- -D warnings`: sin `dead_code` residual.
- [ ] 1.3 (GREEN) Eliminar el envoltorio `Finder::update()` (`src/finders/finder.rs`) conservando `Find::update()` en el trait; `cargo test` y clippy en verde.
- [ ] 1.4 Retirar los `#[allow(dead_code)]` y los `TODO(core-lanzamiento)` de `Item::keywords()`, `Item::categories()` y `split_desktop_list()` **después** de la tarea 2.2 (hasta entonces siguen sin consumidor).
- [ ] 1.5 Documentar en el propio `src/history/mod.rs` por qué se conservan los tres `allow` del trait (comentario ya existente; verificar que sigue siendo exacto).
- [ ] 1.6 (REFACTOR) `cargo fmt --check` y clippy en verde; recuento de tests igual o mayor al inicial (92).

## 2. A2 — Palabras clave y categorías en la búsqueda

- [ ] 2.1 (RED) Tests de `MetadataCache` en `src/models/item.rs`: devuelve las keywords/categorías del `.desktop` real, memoriza cada ruta (no se relee el fichero) y no memoriza un fallo (se reintenta cuando el `.desktop` aparece). `cargo test` en rojo.
- [ ] 2.2 (GREEN) Implementar `MetadataCache` junto a `IdCache`, exportarlo desde `src/models/mod.rs` y construir la instancia en `src/main.rs`; `cargo test` en verde.
- [ ] 2.3 (RED) Tests de coincidencia en `src/finders/application.rs` y `src/finders/ranking.rs`: consulta que solo coincide por keyword, que solo coincide por categoría, y dos ítems (nombre vs keyword) ambos presentes con el nombre sin penalización; `cargo test` en rojo.
- [ ] 2.4 (GREEN) Ampliar `filter_items()` y `rank_items()` para puntuar con el máximo sobre nombre, palabras clave y categorías, recibiendo la caché de metadatos; `cargo test` en verde.
- [ ] 2.5 Cablear la caché desde `src/main.rs:596` hasta `rank_items` sin cambiar la firma de `Finder::search`; `cargo check` en verde.
- [ ] 2.6 (REFACTOR) Limpiar nombres y doc-comments (incluida la nota de "una sola carga de historial por búsqueda" en `src/finders/ranking.rs`); clippy y fmt en verde.

## 3. B — Política de carga del CSS

- [ ] 3.1 (RED) Tests de la función pura de decisión en `src/models/css.rs`: fichero existente y legible → se carga del fichero; fichero existente pero ilegible → CSS embebido; fichero ausente y directorio escribible → se escribe el CSS por defecto y se aplica; directorio no escribible → CSS embebido en memoria; sin directorios XDG → CSS embebido. `cargo test` en rojo.
- [ ] 3.2 (GREEN) Implementar la función pura y reescribir `Css::load()` para devolver siempre `CssProvider`, registrando los fallos con `tracing::warn!` y evaluando `Css::default()` una sola vez; `cargo test` en verde.
- [ ] 3.3 Quitar el `.expect("Can not load CSS file")` de `src/main.rs:660` y ajustar el punto de llamada a la nueva firma.
- [[ ] 3.4 Verificar que un `style.css` existente nunca se sobrescribe (test de la función pura o de la ruta de decisión) y que no queda ninguna llamada duplicada a `Css::default()`.
- [ ] 3.5 (REFACTOR) clippy `--all-targets -- -D warnings` y `cargo fmt --check` en verde.

## 4. C — Caché de `load_all()` entre pulsaciones

- [ ] 4.1 (RED) Tests del decorador `CachedHistory` con un repositorio contador: N `load_all()` → 1 consulta al repositorio interno; `record()` invalida (siguiente `load_all()` vuelve a consultar, exactamente +1); un `load_all()` fallido no se memoriza; el resultado devuelto es idéntico al del repositorio interno. `cargo test` en rojo.
- [ ] 4.2 (GREEN) Implementar `CachedHistory` en `src/history/` (módulo `cache.rs` o submódulo) e implementar `HistoryRepository` delegando `record`/`usage` y memorizando `load_all`; `cargo test` en verde.
- [ ] 4.3 Cablear en `src/main.rs`: construir `SqliteHistory` envuelto en `CachedHistory`, compartirlo con `record_launch()` y con la ruta de ranking.
- [ ] 4.4 Test de regresión del ranking: con el repositorio decorado, el orden resultante es idéntico al del repositorio sin decorar para una misma fixture de usos.
- [ ] 4.5 (REFACTOR) clippy `--all-targets -- -D warnings` y `cargo fmt --check` en verde.

## 5. Verificación y cierre (#16)

- [ ] 5.1 (Proceso y arranque) Comprobar que bajo `main` (sin display, con el `.expect` retirado) un `~/.config/lila` no escribible no aborta: cubierto por los tests de 3.1 y verificado en manual.
- [ ] 5.2 Verificación manual en Wayland: teclear en el dashboard sigue filtrando igual; una app con `Keywords` coincide al buscar por su palabra clave; el estilo por defecto se aplica en el primer arranque; `pgrep -c lila` == 1.
- [ ] 5.3 Escribir `docs/deuda-tecnica-post-m1.md` (qué es, ficheros implicados, cómo funciona A2/B/C, decisiones y follow-ups).
- [ ] 5.4 Cerrar con la puerta de calidad completa: `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` en verde.
- [ ] 5.5 Auditoría `@rust-reviewer` y PR de `feature/deuda-tecnica-post-m1` a `development`.
- [ ] 5.6 `openspec archive deuda-tecnica-post-m1`, marcar PLAN-003 y renombrarlo a `plans/PLAN-003-ARCHIVED.md`, cerrar el issue #16.