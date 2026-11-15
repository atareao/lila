# launcher/app-execution Specification

## Purpose
Ejecuta aplicaciones del sistema a partir de ficheros `.desktop`, respetando sus metadatos y notificando los fallos al usuario.

## Requirements

### Requirement: Ejecución conforme a los metadatos del .desktop
The system SHALL launch applications honoring the `Exec` key field codes, the `Terminal` key, the `Path` working directory and the `TryExec` availability check, and MUST surface a user-visible error when a launch fails.

#### Scenario: Aplicación con Terminal=true
- **WHEN** el usuario lanza una aplicación cuyo `.desktop` define `Terminal=true`
- **THEN** la aplicación se inicia dentro de un emulador de terminal

#### Scenario: Field codes en Exec
- **WHEN** el `.desktop` usa marcadores como `%U` o `%f` en `Exec`
- **THEN** el sistema los procesa según la especificación y lanza la aplicación sin argumentos residuales literales

#### Scenario: TryExec ausente
- **WHEN** el `.desktop` declara `TryExec` y el ejecutable no está disponible
- **THEN** la aplicación se marca como no lanzable y no se intenta ejecutar

#### Scenario: Fallo de lanzamiento visible
- **WHEN** el lanzamiento falla (por ejemplo, un `.desktop` inexistente)
- **THEN** el sistema muestra un error comprensible al usuario en lugar de fallar en silencio

### Requirement: Identificador estable por aplicación
The system SHALL expose a stable identifier for each item, derived from the canonical path of its `.desktop` file.

#### Scenario: Identificador de una app instalada
- **WHEN** se enumera una aplicación desde el sistema
- **THEN** su identificador es la ruta canónica de su fichero `.desktop`

### Requirement: Resultado del lanzamiento en la interfaz
The system SHALL hide the dashboard after a successful launch and keep it visible when the launch fails.

#### Scenario: Lanzamiento correcto
- **WHEN** el usuario confirma un resultado válido y la aplicación se lanza
- **THEN** el dashboard se oculta

#### Scenario: Lanzamiento fallido
- **WHEN** el lanzamiento falla
- **THEN** el dashboard permanece visible y se muestra el error
