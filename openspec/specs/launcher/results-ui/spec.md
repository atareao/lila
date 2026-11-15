# launcher/results-ui Specification

## Purpose
Presenta cada resultado de forma clara con su icono, su nombre y una línea de contexto.

## Requirements

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

### Requirement: Resolución de la hoja de estilos
The system SHALL resolve its stylesheet from the user's `style.css` when that file is present and readable, falling back to the built-in default stylesheet in memory otherwise, and MUST NOT abort startup or overwrite a user stylesheet because of a styling problem.

Contratos observables:
- Ubicación del fichero del usuario: `style.css` dentro del directorio de configuración XDG de la aplicación.
- Si el fichero existe y se puede leer → se aplica ese contenido y **nunca** se sobrescribe.
- Si el fichero existe pero no se puede leer → se aplica el CSS embebido por defecto en memoria; el fichero se deja intacto.
- Si el fichero no existe → se crea el directorio de configuración si hace falta, se escribe en él el CSS por defecto y se aplica; si la escritura falla, se aplica el CSS por defecto en memoria y se registra un aviso.
- Si no hay directorio de configuración XDG disponible → se aplica el CSS por defecto en memoria.
- Ningún problema de estilo (permisos, directorio inexistente, fichero ilegible) impide el arranque ni provoca un error visible al usuario; se registra como aviso en el log.
- El CSS por defecto se construye una sola vez por arranque.

#### Scenario: Fichero del usuario presente
- **WHEN** existe `style.css` en el directorio de configuración y es legible
- **THEN** el dashboard usa ese contenido y el fichero no se modifica

#### Scenario: Fichero del usuario ilegible
- **WHEN** existe `style.css` pero no se puede leer
- **THEN** el dashboard usa el estilo por defecto en memoria y el fichero permanece intacto

#### Scenario: Primer arranque sin fichero de estilo
- **WHEN** el fichero `style.css` no existe todavía
- **THEN** el sistema escribe el estilo por defecto en el directorio de configuración y lo aplica

#### Scenario: Directorio de configuración no escribible
- **WHEN** el directorio de configuración no se puede crear o escribir
- **THEN** el dashboard se muestra con el estilo por defecto en memoria, la aplicación sigue funcionando y solo queda un aviso en el log

#### Scenario: Sin directorios XDG
- **WHEN** no hay directorio de configuración XDG disponible
- **THEN** el dashboard se muestra con el estilo por defecto en memoria y el arranque no falla
