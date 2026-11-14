# Spec Delta

## Purpose

Garantiza que solo exista una instancia del lanzador y que pueda mostrarse bajo demanda.

## ADDED Requirements

### Requirement: Instancia única
The system SHALL run as a single instance so that invoking the binary again does not open a second window.

#### Scenario: Segunda invocación del binario
- **WHEN** LiLa ya está en ejecución y se invoca el binario de nuevo
- **THEN** no se abre una segunda ventana

### Requirement: Re-presentación del dashboard
The system SHALL present and focus the existing dashboard when a new invocation occurs.

#### Scenario: Mostrar la instancia existente
- **WHEN** LiLa ya está en ejecución y se invoca el binario de nuevo
- **THEN** la ventana existente se muestra y recibe el foco

### Requirement: Ocultar al perder el foco
The system SHALL hide, without quitting, the dashboard when it loses focus, keeping the process alive for later invocations.

#### Scenario: Pérdida de foco
- **WHEN** la ventana del dashboard pierde el foco
- **THEN** se oculta y el proceso continúa en ejecución
