# Proposal

## Why

El core del launcher arrastra deuda técnica de la primera iteración: cuatro `#[allow(dead_code)]` que silencian símbolos sin consumidor, metadatos de `.desktop` (`Keywords`, `Categories`) que se leen pero no influyen en la búsqueda, una política de carga del CSS implícita que puede abortar el arranque (`Css::load().expect(...)` sobre un `Result` que propaga errores de sistema de ficheros) y una consulta SQLite completa por cada pulsación (`history.load_all()` dentro de `rank_items`). Todo ello es invisible al usuario hasta que falla: un `~/.config/lila` no escribible mata la aplicación y cada tecla cuesta una lectura de disco y una consulta.

## What Changes

- **A — Limpieza de `#[allow(dead_code)]`:** retirar los cuatro `allow` sin justificación real (`Item::exec()`, `Finder::update()`, `split_desktop_list()`, `Item::keywords()/categories()`), eliminando los dos símbolos sin consumidor y dejando de ocultar los que pasan a usarse en A2. Los tres `allow` de `src/history/mod.rs` se mantienen: son el contrato del trait `HistoryRepository` y se consumen desde los tests del módulo.
- **A2 — Metadatos en la búsqueda:** la consulta pasa a coincidir también con las palabras clave y las categorías del `.desktop`, no solo con el nombre, con una caché de metadatos por ruta análoga a `IdCache` para no reabrir el `.desktop` en cada pulsación.
- **B — Política de carga del CSS:** `Css::load()` deja de devolver `Result` y nunca aborta; se define la precedencia (fichero del usuario → CSS embebido por defecto), la no sobrescritura del fichero existente y el comportamiento cuando el directorio de configuración no existe, no es escribible o no hay directorios XDG. La decisión se extrae a una función pura testeable sin display.
- **C — Caché del historial:** un decorador de `HistoryRepository` memoriza `load_all()` y lo invalida en `record()`, de modo que teclear no vuelve a consultar SQLite y un lanzamiento se refleja en la siguiente búsqueda. El orden de resultados no cambia.
- **Sin cambios BREAKING:** es un crate binario sin consumidores externos; A2 solo amplía el conjunto de consultas que producen coincidencia y C es una optimización con el mismo ranking.

## Capabilities

### New Capabilities
<!-- Ninguna: A2 modifica el ranking existente y B la política de presentación ya especificada. -->

### Modified Capabilities

- `launcher/result-ranking`: se añade una requirement de coincidencia más allá del nombre (palabras clave y categorías del `.desktop`), con su caché de metadatos por ruta.
- `launcher/results-ui`: se añade una requirement de política de resolución de la hoja de estilos (fichero del usuario, CSS embebido por defecto, no sobrescritura, ausencia de aborto).

A se resuelve sin delta de spec porque no cambia comportamiento observable (ni entradas, ni salidas, ni estado en disco): solo retira supresiones de lint y dos símbolos sin consumidor. C tampoco cambia el ranking observable (mismas puntuaciones y mismo orden; solo menos consultas), por lo que se cubre con tests de regresión en lugar de un delta. A2 y B sí cambian comportamiento observable y por eso llevan spec.

## Impact

- Código: `src/models/item.rs`, `src/models/mod.rs`, `src/models/css.rs`, `src/finders/finder.rs`, `src/finders/application.rs`, `src/finders/ranking.rs`, `src/history/` (nuevo decorador), `src/main.rs`.
- APIs: interfaz interna del crate; cambia la firma de `Css::load()` (sin `Result`) y desaparecen `Item::exec()` y `Finder::update()`.
- Dependencias: ninguna nueva (la caché de metadatos no necesita crate; el decorador reutiliza la trait existente).
- Datos: sin migraciones ni cambios de esquema; la caché de history es en memoria y se invalida en cada `record()`.
- Documentación: nuevo `docs/deuda-tecnica-post-m1.md`.