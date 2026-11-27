# Design — short-query-filter

## Resumen de decisión

Implementar la regla en `best_fuzzy_match` (`src/finders/application.rs:72-85`) mediante un guard: si `query.trim().chars().count() == 1`, se omiten los tokens de `categories` del iterator de coincidencias y se mantienen `name()` y `keywords`. Las consultas de 2 o más caracteres conservan el comportamiento actual.

## Por qué ahí

`best_fuzzy_match` es el punto compartido por `filter_items` (`application.rs:50-64`, filtrado en vivo) y por la ruta con historial de `rank_items` (`src/finders/ranking.rs:53-108`). Un único guard en esa función cubre ambos caminos con un solo cambio, sin tocar `src/main.rs` ni `Config` y sin duplicar la condición en cada llamador.

## Alternativa descartada

Exigir una longitud mínima en el `connect_changed` de `src/main.rs:604/634`. Bloquearía búsquedas legítimas de un carácter por nombre o palabra clave y no cubriría las llamadas directas a `rank_items`/`filter_items`, dejando la ruta con historial inconsistente.

## Detalles de implementación

- El conteo usa `chars().count()`, no `len()`: en UTF-8 un carácter multibyte como `ñ` occupies varios bytes pero cuenta como una consulta de un carácter.
- El `trim()` evita que espacios circundantes alteren el conteo; no se normaliza Unicode.
- Sin cambios de firma pública: `best_fuzzy_match`, `filter_items` y `rank_items` mantienen sus interfaces.
- Sin umbral de score: se mantiene `SkimMatcherV2::default()` y el empate conserva el orden; `rank_items` sigue ponderando fuzzy*1.0 + frecuencia*0.5 + recencia*0.5 con recorte a `max_results=50`.
- `Comment` y `GenericName` siguen sin participar en la búsqueda.
- Sin cambios en UI, config ni metadatos en disco (`config.yml`, `style.css` y la forma serde de `Finder`/`Item`).

## Estrategia de tests

- **RED** en `src/finders/application.rs`: 1 carácter con match solo en `Categories` → sin resultados; 1 carácter por `Name` → presente; 1 carácter por `Keywords` → presente; regresión con 2+ caracteres por `Categories` → presente (el test existente `filter_matches_category_absent_from_name` con `spreadsheet` debe seguir verde).
- **RED** en `src/finders/ranking.rs`: con historial, una consulta de 1 carácter con match solo en `Categories` → ítem ausente del ranking.
- **GREEN**: añadir el guard en `best_fuzzy_match` y verificar con `cargo test` y `cargo check`.
- **REFACTOR**: `cargo fmt` y `cargo clippy --all-targets -- -D warnings`, re-ejecutando la suite.

## Riesgos

- Pueden seguir apareciendo coincidencias amplias por `Keywords` de una letra; la decisión es conservarlas porque son búsquedas legítimas. Se mitiga con tests de regresión que fijan el alcance exacto del guard.
- La spec existente `launcher/result-ranking` se actualiza mediante el delta del change; el resto de requisitos (ponderación, límite de resultados, caché de metadatos) se conservan textualmente.
