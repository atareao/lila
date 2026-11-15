# Proposal

## Why

LiLa hoy es un prototipo funcional: encuentra y lanza aplicaciones con emparejamiento difuso, pero no recuerda el uso, no muestra iconos y puede ejecutar incorrectamente algunos `.desktop` (por ejemplo `Terminal=true` o field codes). Para aspirar a ser un lanzador de referencia en Wayland hay que cimentar primero un core de lanzamiento sólido.

## What Changes

- Motor de ejecución de `.desktop` robusto: field codes, `Terminal=true`, `Path`, `TryExec`, activación DBus y errores visibles al usuario.
- Resultados con icono, nombre y subtítulo atenuado en lugar de una sola etiqueta de texto.
- Historial de uso persistente en SQLite (`rusqlite`, feature `bundled`) en `~/.config/lila/history.db`, con modo WAL y esquema versionado.
- Ranking combinado: coincidencia difusa + frecuencia de uso + recencia, con pesos configurables.
- Navegación de teclado completa: PageUp/PageDown, Home/End, wrap-around y scroll-to-selected.
- Instancia única correcta: una segunda invocación re-presenta el dashboard existente.
- `Config` ampliada (límite de resultados, pesos del ranking, historial activado) tolerante a configuraciones antiguas.

## Capabilities

### New Capabilities
- `launcher/app-execution`: ejecución correcta y segura de ficheros `.desktop`, con manejo de errores visible.
- `launcher/usage-history`: registro y consulta del historial de lanzamientos, persistido en SQLite.
- `launcher/result-ranking`: ordenación de resultados combinando coincidencia difusa, frecuencia y recencia.
- `launcher/results-ui`: presentación de cada resultado con icono, nombre y subtítulo.
- `launcher/keyboard-navigation`: control de la interfaz mediante teclado, incluida la navegación circular.
- `launcher/invocation`: comportamiento de instancia única y re-presentación del dashboard.
- `launcher/configuration`: opciones configurables del core de lanzamiento y compatibilidad con configuraciones previas.

### Modified Capabilities
<!-- Ninguna: no existe todavía ninguna capability en openspec/specs/. -->

## Impact

- Código: `src/actions/`, `src/finders/`, `src/models/`, `src/main.rs` y nuevo módulo `src/history/`.
- Dependencias: nueva crate `rusqlite` (feature `bundled`).
- Datos: nuevo fichero `~/.config/lila/history.db`.
- Sin cambios incompatibles en `config.yml`: los campos nuevos son opcionales.
