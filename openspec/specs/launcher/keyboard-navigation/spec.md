# launcher/keyboard-navigation Specification

## Purpose
Permite operar el lanzador por completo mediante el teclado.

## Requirements

### Requirement: Navegación vertical con wrap-around
The system SHALL move the selection down and up with the arrow keys, wrapping around at the ends, and SHALL keep the selected row visible.

#### Scenario: Bajar desde el último resultado
- **WHEN** la selección está en el último resultado y el usuario pulsa la flecha abajo
- **THEN** la selección pasa al primer resultado

#### Scenario: Subir desde el primer resultado
- **WHEN** la selección está en el primer resultado y el usuario pulsa la flecha arriba
- **THEN** la selección pasa al último resultado

#### Scenario: Fila seleccionada visible
- **WHEN** la selección cambia a una fila fuera del área visible
- **THEN** la lista se desplaza para dejarla a la vista

### Requirement: Navegación por páginas e inicio/fin
The system SHALL support PageUp, PageDown, Home and End to move the selection.

#### Scenario: Inicio y fin
- **WHEN** el usuario pulsa Home o End
- **THEN** la selección pasa al primer o al último elemento, respectivamente

### Requirement: Confirmar y ocultar
The system SHALL launch the selected result with Enter and hide the window with Escape.

#### Scenario: Enter lanza
- **WHEN** el usuario pulsa Enter con una fila seleccionada
- **THEN** se lanza la aplicación y el dashboard se oculta

#### Scenario: Escape oculta
- **WHEN** el usuario pulsa Escape
- **THEN** el dashboard se oculta sin lanzar nada
