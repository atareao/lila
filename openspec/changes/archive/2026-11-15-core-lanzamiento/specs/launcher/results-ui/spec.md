# Spec Delta

## Purpose

Presenta cada resultado de forma clara con su icono, su nombre y una línea de contexto.

## ADDED Requirements

### Requirement: Fila con icono y subtítulo
The system SHALL display each result as a row containing the application icon, its name and an optional dimmed subtitle.

#### Scenario: Aplicación con icono disponible
- **WHEN** un resultado tiene icono
- **THEN** la fila muestra el icono junto al nombre

#### Scenario: Aplicación sin icono
- **WHEN** un resultado no tiene icono disponible
- **THEN** la fila muestra el nombre y, si existe, el subtítulo, sin errores de disposición

#### Scenario: Subtítulo opcional
- **WHEN** un resultado tiene descripción o nombre genérico
- **THEN** se muestra como subtítulo atenuado bajo el nombre

### Requirement: Visibilidad de la lista de resultados
The system SHALL show the results list whenever at least one result is available, including with an empty query, and hide it when there are no results.

#### Scenario: Consulta vacía con historial
- **WHEN** el usuario abre el dashboard y existe historial de uso
- **THEN** se muestra la lista con las aplicaciones mejor posicionadas

#### Scenario: Sin resultados
- **WHEN** la búsqueda no produce coincidencias
- **THEN** la lista de resultados permanece oculta

### Requirement: Estilo de la fila seleccionada
The system SHALL style the selected row distinctly so that the current selection is always identifiable.

#### Scenario: Fila seleccionada
- **WHEN** una fila está seleccionada
- **THEN** se resalta visualmente mediante los estilos del tema
