# launcher/result-ranking Specification

## Purpose
Ordena los resultados de búsqueda para que las aplicaciones más relevantes y usadas aparezcan primero.

## Requirements

### Requirement: Ranking combinado
The system SHALL rank items by a combined score of fuzzy match, launch frequency and recency, using weights taken from configuration.

#### Scenario: La frecuencia desempata coincidencias
- **WHEN** dos aplicaciones coinciden con la consulta y una se ha lanzado más veces
- **THEN** la más lanzada aparece antes

#### Scenario: Recencia con consulta vacía
- **WHEN** el usuario abre el dashboard sin escribir nada
- **THEN** las aplicaciones usadas recientemente aparecen en las primeras posiciones

#### Scenario: La coincidencia difusa adelanta a una no usada
- **WHEN** una aplicación coincide fuertemente con la consulta aunque no se haya usado nunca
- **THEN** puede situarse por delante de otras con un uso leve

### Requirement: Pesos configurables
The system SHALL allow configuring the relative weights of fuzzy match, frequency and recency in the ranking.

#### Scenario: Cambiar los pesos
- **WHEN** el usuario modifica los pesos en la configuración
- **THEN** el orden de los resultados refleja los nuevos pesos

### Requirement: Límite de resultados
The system SHALL limit the number of displayed results to a configurable maximum.

#### Scenario: Más coincidencias que el límite
- **WHEN** el número de coincidencias supera el límite configurado
- **THEN** solo se muestran los resultados mejor puntuados hasta el límite

### Requirement: Coincidencia más allá del nombre
The system SHALL match the query against an item's words and categories from its `.desktop` file in addition to its name, scoring the item by its best match among those sources, so an application can be found by a keyword or a category it declares.

Contratos observables:
- Los ámbitos de coincidencia son el nombre del ítem y los valores de las claves `Keywords` y `Categories` de su `.desktop` (cada valor de la lista `;`-separada, sin elementos vacíos).
- La puntuación difusa de un ítem es el **máximo** de las puntuaciones obtenidas contra esos ámbitos; un ítem cuya puntuación máxima no exista se descarta cuando la consulta no está vacía.
- La coincidencia por nombre no se penaliza ni se premia frente a la coincidencia por palabra clave o categoría.
- Los metadatos de cada `.desktop` se resuelven **como máximo una vez por sesión** y por ruta (memorizados), sin releer el fichero en cada pulsación; un fallo de lectura no se memoriza y se reintenta.
- Con consulta vacía, el conjunto de resultados no cambia: el conjunto de coincidencias solo se amplía, nunca se reduce.

#### Scenario: Coincidencia por palabra clave
- **WHEN** el usuario escribe una consulta que aparece en el campo `Keywords` de un `.desktop` y no en el nombre de la aplicación
- **THEN** esa aplicación aparece en los resultados

#### Scenario: Coincidencia por categoría
- **WHEN** el usuario escribe una consulta que aparece en el campo `Categories` de un `.desktop` y no en el nombre de la aplicación
- **THEN** esa aplicación aparece en los resultados

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
