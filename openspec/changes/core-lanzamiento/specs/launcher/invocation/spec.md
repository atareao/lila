# Spec Delta

## Purpose

Garantiza que solo exista una instancia del lanzador y que pueda mostrarse bajo demanda.

## ADDED Requirements

### Requirement: Instancia única
The system SHALL run as a single instance so that invoking the binary again does not open a second window.

#### Scenario: Segunda invocación del binario
- **WHEN** LiLa ya está en ejecución y se invoca el binario de nuevo
- **THEN** no se abre una segunda ventana

#### Scenario: La segunda invocación termina
- **WHEN** LiLa ya está en ejecución y se invoca el binario de nuevo
- **THEN** el proceso secundario termina sin cargar configuración ni abrir la base de datos y no deja procesos colgados

### Requirement: Unicidad dependiente del bus de sesión
The system SHALL delegate single-instance arbitration to the session D-Bus, falling back to an independent primary instance when no bus is available.

#### Scenario: Sin bus de sesión
- **WHEN** no hay bus de sesión disponible
- **THEN** la invocación arranca como instancia primaria local, sin reenviar la activación

### Requirement: Re-presentación del dashboard
The system SHALL present and focus the existing dashboard when a new invocation occurs.

#### Scenario: Mostrar la instancia existente
- **WHEN** LiLa ya está en ejecución y se invoca el binario de nuevo
- **THEN** la ventana existente se muestra y recibe el foco

#### Scenario: Foco tras la re-presentación
- **WHEN** la ventana existente se re-presenta
- **THEN** recibe el foco y una pérdida de foco transitoria del compositor no la oculta

### Requirement: Re-presentación sin mensajes obsoletos
The system SHALL reset transient UI state when re-presenting a hidden dashboard, so stale launch errors are not shown again, and SHALL keep the user's query when the dashboard was already visible.

#### Scenario: Re-presentación tras un fallo de lanzamiento
- **WHEN** la ventana existente, oculta, se re-presenta tras un fallo de lanzamiento
- **THEN** el mensaje de error deja de mostrarse y el campo de búsqueda queda vacío

#### Scenario: Re-presentación sin error previo
- **WHEN** la ventana existente está oculta y se re-presenta
- **THEN** el campo de búsqueda queda vacío

#### Scenario: Ventana ya visible
- **WHEN** la ventana existente ya está visible y se invoca el binario de nuevo
- **THEN** se conserva el texto que el usuario tenía escrito

#### Scenario: Ventana visible con banner de error
- **WHEN** la ventana existente ya está visible y muestra un error
- **THEN** el banner se conserva, porque la re-presentación no reinicia el estado de una ventana que no estaba oculta

### Requirement: Ocultar al perder el foco
The system SHALL hide, without quitting, the dashboard when it loses focus, keeping the process alive for later invocations. The hide MAY be deferred for a short, bounded grace period right after the dashboard is presented; any focus loss seen during that period SHALL be re-checked when it ends instead of being discarded.

#### Scenario: Pérdida de foco
- **WHEN** la ventana del dashboard pierde el foco
- **THEN** se oculta y el proceso continúa en ejecución

#### Scenario: Pérdida de foco real durante el margen
- **WHEN** la ventana pierde el foco de forma real dentro del margen posterior a su presentación
- **THEN** al terminar el margen se re-verifica el foco y la ventana se oculta si sigue sin él
