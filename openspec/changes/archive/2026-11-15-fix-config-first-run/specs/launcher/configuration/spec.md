# Spec Delta

## ADDED Requirements

### Requirement: Carga de la configuración al arranque
The system SHALL load user configuration from `config.yml` when it is present and parseable, SHALL continue startup with the built-in defaults when the file is missing or the configuration directory is not writable, and MUST NOT abort with a panic on any configuration problem; when an existing file is unreadable or contains invalid YAML, the system SHALL report a clear error and MUST NOT silently discard nor overwrite the user's file.

Contratos observables:
- `config.yml` presente y parseable → se usa tal cual y el arranque continúa.
- `config.yml` ausente → se escribe en el directorio de configuración el `Config::default()` serializado y se usa ese default; el arranque continúa y no se devuelve error por ausencia.
- `config.yml` ausente y directorio de configuración no escribible → se usa el default en memoria, se registra un aviso y el arranque continúa.
- Sin directorio de configuración XDG disponible → se usa el default y el arranque continúa.
- `config.yml` presente pero ilegible o con YAML inválido → se devuelve un error de configuración; el usuario recibe un **mensaje de error claro**, la aplicación **sale de forma controlada con código de salida distinto de cero** y el fichero del usuario permanece **intacto**.
- Ningún caso (ausencia, permisos, directorio inexistente, fichero ilegible, YAML inválido) provoca un `panic`.

#### Scenario: Primer arranque sin config.yml
- **WHEN** no existe todavía `config.yml` en el directorio de configuración
- **THEN** el sistema escribe el default, arranca con ese default y no falla

#### Scenario: Directorio de configuración no escribible
- **WHEN** `config.yml` no existe y el directorio de configuración no se puede crear o escribir
- **THEN** el sistema arranca con el default en memoria y continúa funcionando

#### Scenario: Sin directorios XDG
- **WHEN** no hay directorio de configuración XDG disponible
- **THEN** el sistema arranca con el default y no falla

#### Scenario: config.yml ilegible o inválido
- **WHEN** existe `config.yml` pero no se puede leer o no es YAML válido
- **THEN** el sistema muestra un error claro, sale de forma controlada con código distinto de cero y no modifica el fichero
