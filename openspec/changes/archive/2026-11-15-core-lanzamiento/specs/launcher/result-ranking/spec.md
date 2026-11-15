# Spec Delta

## Purpose

Ordena los resultados de búsqueda para que las aplicaciones más relevantes y usadas aparezcan primero.

## ADDED Requirements

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
