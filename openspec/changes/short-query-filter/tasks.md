# Tasks

## 1. Consultas de un carácter no coinciden por categoría (TDD)

- [x] 1.1 (RED) Tests en `src/finders/application.rs` para `filter_items`: (a) consulta de 1 carácter cuyo único match es un token de `Categories` → el ítem NO aparece; (b) 1 carácter con match solo en `Name` → aparece; (c) 1 carácter con match solo en `Keywords` → aparece; (d) carácter Unicode multibyte (`ñ`) cuenta como 1 carácter con `chars().count()`, no con `len()`; (e) 2+ caracteres con match solo en `Categories` → aparece (regresión; `filter_matches_category_absent_from_name` con `spreadsheet` sigue verde). `cargo test` en rojo por las razones correctas, con los tests previos verdes.
- [x] 1.2 (RED) Tests en `src/finders/ranking.rs` para la ruta con historial: una consulta de 1 carácter que solo casa con `Categories` no rankea el ítem, mientras que la misma consulta con 2+ caracteres sí lo rankea; verificar que un match por `Name` sigue rankeando. `cargo test` en rojo.
- [x] 1.3 (GREEN) Guard en `best_fuzzy_match`: si `query.trim().chars().count() == 1`, excluir los tokens de `categories` del iterator de `best_fuzzy_match`, manteniendo `name()` y `keywords`. Sin cambios en UI ni Config. `cargo test` en verde.
- [x] 1.4 (GREEN) `cargo check` sin errores.
- [x] 1.5 (REFACTOR) `cargo fmt` y `cargo clippy --all-targets -- -D warnings` en verde; re-ejecutar `cargo test` sin regresiones.

## 2. Verificación y cierre

- [ ] 2.1 Comprobación manual en Wayland: teclear `u` ya no inunda el listado; teclear `spreadsheet` sigue encontrando la aplicación por categoría.
  - Pendiente: no se automatizó teclado sobre el escritorio del usuario; ver follow-up en docs/short-query-filter.md
- [x] 2.2 Escribir `docs/short-query-filter.md` (qué es, ficheros implicados, regla de la consulta de 1 carácter, cómo verificar, decisiones y follow-ups).
- [ ] 2.3 Archivar el change (`openspec archive short-query-filter`).
