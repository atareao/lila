# launcher/configuration Specification

## Purpose
Define las opciones configurables del core de lanzamiento y la compatibilidad con configuraciones existentes.

## Requirements

### Requirement: Opciones del core
The system SHALL expose configuration for window width and height, result limit, ranking weights and history enabled/disabled.

#### Scenario: Valor por defecto
- **WHEN** el usuario no define una opción
- **THEN** el sistema usa un valor por defecto razonable

### Requirement: Compatibilidad con configuraciones antiguas
The system SHALL tolerate existing `config.yml` files that lack the new options, applying defaults without error.

#### Scenario: Configuración antigua
- **WHEN** `config.yml` no contiene los campos nuevos
- **THEN** el sistema arranca aplicando los valores por defecto sin fallar

### Requirement: Historial desactivable
The system SHALL allow disabling history so that no launches are recorded.

#### Scenario: Historial desactivado
- **WHEN** el historial está desactivado
- **THEN** no se registra ningún lanzamiento y el ranking usa solo la coincidencia difusa
