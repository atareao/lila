# Spec Delta

## ADDED Requirements

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
