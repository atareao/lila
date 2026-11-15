# Proposal

## Why

`Config::load()` (`src/models/config.rs`) devuelve `Err("Failed to create config file")` cuando `config.yml` todavía no existe, **después** de haberlo escrito correctamente con el default. El llamador en `src/main.rs:685` hace `Config::load().expect("Can not load config file")`, de modo que ese `Err` convierte el **primer arranque de un usuario nuevo en un `panic`**. El fichero queda creado, por lo que el segundo arranque ya funciona: el fallo solo castiga a quien nunca ha ejecutado LiLa. Es el mismo tipo de defecto que el ya corregido en el camino del CSS (`Css::load().expect(...)`, punto B del issue #16), pero en la configuración no se aplicó la misma política.

## What Changes

- `Config::load()` **sigue devolviendo `Result`**, pero corrige la política de errores:
  - **Fichero ausente** → se escribe `Config::default()` y se devuelve `Ok(default)`; el arranque continúa (es el bug que se arregla).
  - **Directorio de configuración no escribible / sin XDG** → se devuelve `Ok(default)` en memoria registrando un `warn!`; el arranque continúa (caso benigno).
  - **Fichero presente pero ilegible o con YAML inválido** → se devuelve `Err` de configuración; el llamador muestra un **error claro y sale de forma controlada con código distinto de cero**, sin `panic` y **sin sobrescribir** el fichero del usuario. No se colapsa todo en "ignorar en silencio": un `config.yml` corrupto debe ser visible.
- La lógica de decisión se extrae a una **función pura testeable sin display** que recibe el directorio de configuración como parámetro y devuelve `Result`, análoga a `resolve_stylesheet` en `src/models/css.rs`.
- Se elimina el `.expect("Can not load config file")` de `src/main.rs:685`; se sustituye por un `match` que registra el error, lo imprime en `stderr` y llama a `std::process::exit` con código distinto de cero.
- **Sin cambios BREAKING:** es un crate binario sin consumidores externos; la forma de `Config` en disco no cambia, así que los `config.yml` existentes siguen siendo válidos.

## Capabilities

### New Capabilities
<!-- Ninguna. -->

### Modified Capabilities

- `launcher/configuration`: se añade una requirement de carga al arranque (fichero del usuario; default y continuación ante ausencia o directorio no escribible; error explícito y salida controlada ante fichero ilegible o inválido, sin sobrescritura).

## Impact

- Código: `src/models/config.rs` (función pura + reescritura de `Config::load()`), `src/main.rs` (punto de llamada, sin `panic`).
- APIs: interfaz interna del crate; `Config::load()` mantiene `Result` pero cambia su política.
- Dependencias: ninguna nueva.
- Datos: sin migraciones; el formato en disco de `config.yml` no cambia.
- Documentación: nuevo `docs/config-first-run.md`.
