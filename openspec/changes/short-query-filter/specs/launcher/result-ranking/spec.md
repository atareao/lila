# launcher/result-ranking Delta

## MODIFIED Requirements

### Requirement: Coincidencia más allá del nombre
The system SHALL match the query against an item's words and categories from its `.desktop` file in addition to its name, scoring the item by its best match among those sources, so an application can be found by a keyword or a category it declares. Excepto en consultas de un solo carácter, donde las categorías SHALL NOT participar en la coincidencia.

Contratos observables:
- Los ámbitos de coincidencia son el nombre del ítem y los valores de las claves `Keywords` y `Categories` de su `.desktop` (cada valor de la lista `;`-separada, sin elementos vacíos). `Comment` y `GenericName` no participan en la búsqueda.
- La puntuación difusa de un ítem es el **máximo** de las puntuaciones obtenidas contra esos ámbitos; un ítem cuya puntuación máxima no exista se descarta cuando la consulta no está vacía.
- **Regla de consulta corta:** si `query.trim().chars().count() == 1`, `best_fuzzy_match` excluye los tokens de `categories` del iterator de coincidencias; se mantienen `name()` y `keywords`. El conteo es de caracteres Unicode, no de bytes (`len()`), de modo que un carácter multibyte como `ñ` cuenta como uno. La regla se aplica por igual en `filter_items` y en la ruta con historial de `rank_items`.
- Para consultas de 2 o más caracteres, las categorías participan exactamente como antes, sin umbral de puntuación y sin penalización por haberse encontrado la coincidencia fuera del nombre.
- La coincidencia por nombre no se penaliza ni se premia frente a la coincidencia por palabra clave o categoría.
- Los metadatos de cada `.desktop` se resuelven **como máximo una vez por sesión** y por ruta (memorizados), sin releer el fichero en cada pulsación; un fallo de lectura no se memoriza y se reintenta.
- Con consulta vacía, el conjunto de resultados no cambia: el conjunto de coincidencias solo se amplía, nunca se reduce.
- El motor es `fuzzy-matcher 0.3.7` con `SkimMatcherV2::default()`, sin umbral; los empates conservan el orden; `rank_items` pondera fuzzy*1.0 + frecuencia*0.5 + recencia*0.5 y recorta a `max_results=50`.

#### Scenario: Coincidencia por palabra clave
- **WHEN** el usuario escribe una consulta que aparece en el campo `Keywords` de un `.desktop` y no en el nombre de la aplicación
- **THEN** esa aplicación aparece en los resultados

#### Scenario: Coincidencia por categoría
- **WHEN** el usuario escribe una consulta de 2 o más caracteres que aparece en el campo `Categories` de un `.desktop` y no en el nombre de la aplicación
- **THEN** esa aplicación aparece en los resultados (p. ej. la consulta `spreadsheet` sigue encontrando la aplicación)

#### Scenario: Consulta de un carácter no casa con categorías
- **GIVEN** una aplicación cuyo único match posible para la consulta es un token de `Categories`
- **WHEN** el usuario escribe una consulta de un carácter (p. ej. `u`, presente en `Utility`)
- **THEN** esa aplicación NO aparece en los resultados

#### Scenario: Consulta de un carácter casa con el nombre
- **GIVEN** una aplicación cuyo `Name` contiene la consulta de un carácter
- **WHEN** el usuario escribe esa consulta
- **THEN** la aplicación aparece en los resultados

#### Scenario: Consulta de un carácter casa con palabras clave
- **GIVEN** una aplicación cuyo `Keywords` contiene la consulta de un carácter y cuyo nombre no la contiene
- **WHEN** el usuario escribe esa consulta
- **THEN** la aplicación aparece en los resultados

#### Scenario: Carácter Unicode multibyte cuenta como uno
- **GIVEN** una aplicación cuyo único match posible es un token de `Categories` que contiene `ñ`
- **WHEN** el usuario escribe `ñ` (un carácter de 2 bytes en UTF-8)
- **THEN** se aplica la regla de consulta de un carácter y la aplicación NO aparece en los resultados

#### Scenario: La regla se aplica con historial
- **GIVEN** aplicaciones con historial de lanzamiento y una consulta de un carácter que solo casa con `Categories`
- **WHEN** se puntúa el ranking con `rank_items`
- **THEN** esas aplicaciones NO aparecen en los resultados, igual que en el filtrado directo

#### Scenario: El nombre conserva su peso
- **WHEN** dos aplicaciones coinciden con la consulta, una por su nombre y otra solo por una palabra clave o categoría
- **THEN** ambas aparecen en los resultados y el orden refleja sus puntuaciones, sin penalización por haberse encontrado la coincidencia fuera del nombre

#### Scenario: Metadatos leídos una sola vez por sesión
- **WHEN** el usuario teclea varias consultas sobre la misma lista de aplicaciones
- **THEN** el `.desktop` de cada aplicación se lee como máximo una vez por sesión y ruta, de modo que teclear no añade lecturas de disco

#### Scenario: Aplicación sin metadatos
- **WHEN** una aplicación cuyo `.desktop` no declara `Keywords` ni `Categories` coincide con la consulta
- **THEN** se puntúa exactamente igual que antes de esta capacidad (por su nombre) y sigue apareciendo en los resultados

#### Scenario: Metadatos aparecidos después
- **WHEN** el `.desktop` de una aplicación no se pudo leer en un momento dado y pasa a ser legible más tarde en la misma sesión
- **THEN** la siguiente búsqueda ya resuelve sus metadatos, sin reiniciar la aplicación
