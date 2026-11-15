# Tasks

## 1. Config::load() sin panic y con errores visibles (TDD)

- [x] 1.1 (RED) Tests de la función pura `load_config_from(config_dir: Option<&Path>) -> Result<Config, ConfigError>` en `src/models/config.rs`, sobre un `TempDir` (sin XDG ni display): (a) fichero válido presente → `Ok` con su contenido y **sin reescribirlo**; (b) ausente + directorio escribible → escribe el default y devuelve `Ok(default)`; (c) ausente + directorio no escribible → `Ok(default)` en memoria, sin escribir y sin panic; (d) presente pero YAML inválido → `Err(Invalid)` con el fichero intacto; (e) presente pero ilegible (directorio con ese nombre, estable incluso como `root`) → `Err(Unreadable)`; (f) `None` → `Ok(default)`; (g) directorio de configuración inexistente y anidado → se crea y se escribe el default; (h) cualquier fallo de lectura que no sea `NotFound`/`NotADirectory` → `Err(Unreadable)`; (i) `#[cfg(unix)]` enlace simbólico roto → `Err(Unreadable)`, sin seguir el enlace ni crear su destino; (j) fallo de escritura a mitad (escritor inyectado que no acepta bytes) → `Err` y el fichero a medio escribir **se elimina**; (k) la escritura del default usa `create_new` y nunca trunca un destino preexistente. `cargo test` en rojo por las razones correctas, con los 115 tests previos verdes.
- [x] 1.2 (GREEN) Implementar la función pura `load_config_from` con los helpers `read_user_config` (solo `NotFound`/`NotADirectory` cuentan como «ausente») y `create_config_file` (`File::create_new` + `write_default_file`, que borra el fichero si la escritura falla), y reescribir `Config::load()` para delegar en ella manteniendo `Result`; registrar los fallos benignos con `tracing::warn!`. `cargo test` en verde.
- [x] 1.3 Sustituir el `.expect("Can not load config file")` de `src/main.rs:685` por un `match`: `Ok` → usa la config; `Err(e)` → `exit_config_error(&e)` (`tracing::error!` + `eprintln!` del mensaje y `std::process::exit(EXIT_CONFIG_ERROR)`, con `const EXIT_CONFIG_ERROR: i32 = 2`). `cargo check` en verde.
- [x] 1.4 (REFACTOR) `cargo clippy --all-targets -- -D warnings` y `cargo fmt --check` en verde; recuento final **126 tests** (115 iniciales + 11 nuevos), sin ninguno ignorado.

## 2. Verificación y cierre

- [x] 2.1 Verificación manual en Wayland con `XDG_CONFIG_HOME` vacío: el primer arranque **no** falla, se crean `config.yml` y `style.css` con los defaults, la ventana se muestra y `pgrep -c lila` == 1.
- [x] 2.2 Verificación manual del caso inválido: un `config.yml` con YAML roto produce un error claro y salida con código != 0, sin panic y con el fichero intacto.
- [ ] 2.3 Repetir la comprobación manual pendiente del issue #16: filtrado en vivo, coincidencia por palabra clave y por categoría.
- [x] 2.4 Escribir `docs/config-first-run.md` (qué es, ficheros implicados, política de carga, cómo verificar).
- [ ] 2.5 Archivar el change (`openspec archive fix-config-first-run`) y renombrar `plans/PLAN-003.md` → `plans/PLAN-003-ARCHIVED.md` y `plans/PLAN-004.md` → `plans/PLAN-004-ARCHIVED.md`.

## Follow-ups (fuera de alcance de este change)

- Un `config.yml` con `extensions: []` deja el lanzador sin finders (hallazgo pre-existente del review): decidir si se avisa o se rellena con el default.
- Test del envoltorio `Config::load()` con XDG (hoy cubierto por la función pura).
