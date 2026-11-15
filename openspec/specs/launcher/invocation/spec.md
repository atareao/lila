# launcher/invocation Specification

## Purpose
Garantiza que solo exista una instancia del lanzador y que pueda mostrarse bajo demanda.

## Requirements

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

### Requirement: Invocación por acción D-Bus
The system SHALL expose its already-running dashboard as named GApplication actions reachable over the session D-Bus, so a compositor keybinding can present or hide LiLa without spawning a new process.

Contratos observables:

- Acciones nombradas `toggle` y `present`, registradas en el action map de la `GApplication` (`APP_ID = es.atareao.lila`).
- Exportadas por la interfaz `org.gtk.Actions` en el object path `/es/atareao/lila` (`Activate` con `a{sv}` vacío) y por `org.freedesktop.Application` (`ActivateAction`).
- `toggle`: dashboard oculto → presentar con foco y `Entry` vacío; dashboard visible → ocultar sin terminar el proceso.
- `present`: presentar siempre; si ya estaba visible, conservar el texto escrito (idempotente).

#### Scenario: Atajo con el dashboard oculto
- **WHEN** el dashboard está oculto y el compositor invoca la acción `toggle` del bus de sesión
- **THEN** la ventana existente se presenta con foco y el campo de búsqueda vacío, sin lanzar un proceso nuevo

#### Scenario: Atajo con el dashboard visible
- **WHEN** el dashboard ya está visible y el compositor invoca la acción `toggle`
- **THEN** la ventana se oculta sin terminar el proceso, que queda listo para una nueva invocación

#### Scenario: Presentar con el dashboard ya visible
- **WHEN** el dashboard ya está visible y el compositor invoca la acción `present`
- **THEN** la ventana se re-presenta con foco y conserva el texto que el usuario tenía escrito, sin lanzar un proceso nuevo

#### Scenario: Foco tras la invocación por acción
- **WHEN** la acción presenta el dashboard oculto
- **THEN** se aplica el mismo margen anti-pérdida de foco que la invocación por binario, de modo que una pérdida transitoria del compositor no lo oculta

#### Scenario: Proceso ausente
- **WHEN** no hay ninguna instancia de LiLa en ejecución y el compositor invoca la acción sobre el bus
- **THEN** la llamada falla con servicio desconocido y no deja procesos colgados; lanzar LiLa para que la acción pase a estar disponible es responsabilidad del compositor
