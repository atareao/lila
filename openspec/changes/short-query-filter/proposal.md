# Proposal

## Why

Durante la verificación del issue #16 se observó un fallo de usabilidad: al teclear una consulta de un solo carácter, p. ej. `u`, aparecía una coincidencia masiva de aplicaciones. La causa es que `best_fuzzy_match` (`src/finders/application.rs:72-85`) puntúa también los tokens de `Categories` (por ejemplo `Utility;`), compartidos por casi todos los `.desktop`. El motor `fuzzy-matcher` no aplica umbral, de modo que una letra suelta basta para que casi cualquier categoría puntúe y el listado se inunde. `Comment`/`GenericName` no participan en la búsqueda. No existe hoy longitud mínima ni normalización Unicode.

## What Changes

- Añadir un guard en `best_fuzzy_match`: cuando `query.trim().chars().count() == 1`, excluir los tokens de `categories` del iterator de coincidencias; se mantienen `name()` y `keywords`. El conteo es por caracteres Unicode (`chars().count()`), no por bytes (`len()`), de modo que `ñ` cuenta como un carácter.
- El guard es común a las dos rutas que usan la función: `filter_items` (`application.rs:50-64`, filtrado en vivo) y `rank_items` (`src/finders/ranking.rs:53-108`, ruta con historial de frecuencia/recencia), invocadas desde `src/main.rs:604/634`.
- Las consultas de 2 o más caracteres conservan exactamente su comportamiento actual, incluida la coincidencia por categoría.
- Añadir el spec delta `launcher/result-ranking` con el contrato de la regla y sus escenarios BDD.
- **Sin cambios BREAKING:** no cambian la UI, `Config` ni el formato en disco; se descarta exigir una longitud mínima en el `entry` porque impediría teclear consultas de una letra que sí son útiles.

## Capabilities

### New Capabilities
<!-- Ninguna. -->

### Modified Capabilities

- `launcher/result-ranking`: se refina la requirement «Coincidencia más allá del nombre» para que las categorías no participen en consultas de un solo carácter, tanto en el filtrado directo como en el ranking con historial.

## Impact

- Código: `src/finders/application.rs` (único punto de cambio, en `best_fuzzy_match`); sin cambios en `src/main.rs`, `src/models/` ni UI.
- Tests: nuevos casos en `src/finders/application.rs` y `src/finders/ranking.rs`; el test existente `filter_matches_category_absent_from_name` (consulta `spreadsheet`) debe seguir pasando.
- Dependencias: ninguna (`fuzzy-matcher 0.3.7` con `SkimMatcherV2::default()`, sin umbral; empates conservan el orden; `rank_items` pondera fuzzy*1.0 + frecuencia*0.5 + recencia*0.5 y recorta a `max_results=50`).
- Datos: sin migraciones; ni `config.yml` ni `style.css` cambian.
- Documentación: `docs/short-query-filter.md`, además del delta OpenSpec y del archivado del change.

## Scope y no-go

- **Incluido:** consultas de 1 carácter sobre `Categories`; consultas Unicode multibyte; coherencia entre `filter_items` y `rank_items`.
- **Fuera de alcance:** el issue #40; publicación en AUR; umbrales de puntuación, normalización Unicode y longitud mínima en el entry.
