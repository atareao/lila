# PLAN-004 — Primer arranque sin config.yml no debe fallar

- **Issue**: #32 (https://github.com/atareao/lila/issues/32)
- **Change OpenSpec**: `openspec/changes/fix-config-first-run`
- **Rama**: `feature/fix-config-first-run`
- **Estado**: borrador (pendiente de aprobación del spec)

## Objetivo

Que el primer arranque de LiLa en una máquina sin `~/.config/lila/config.yml` no falla: `Config::load()` debe crear el default y continuar. Los errores reales (YAML inválido o fichero ilegible) se reportan con un mensaje claro y una salida controlada, nunca con un `panic`.

## Política de carga aprobada

- Ausente → escribe default y continúa.
- Directorio no escribible / sin XDG → default en memoria + aviso, continúa.
- Presente pero ilegible o YAML inválido → error claro, salida controlada (código != 0), fichero intacto.

## Contexto

- Bug pre-existente (commit `9c9678b`, 2025-06-22), no introducido por la deuda técnica post-M1.
- Reproducido en sesión Wayland: con `XDG_CONFIG_HOME` vacío, `target/debug/lila` hace `panic` en `src/main.rs:685` tras crear `config.yml`. El 2º arranque funciona.
- `Config::load()` no tenía cobertura de test.

## Tareas

1. **TDD** — función pura `load_config_from(Option<&Path>) -> Result<Config, ConfigError>` + reescritura de `Config::load()` (detalle en `openspec/changes/fix-config-first-run/tasks.md`, sección 1).
2. **Punto de llamada** — sustituir el `.expect` de `src/main.rs:685` por un `match` con error claro y `exit(2)`.
3. **Verificación** — arranque limpio en Wayland + singleton + caso inválido; comprobación manual pendiente del issue #16.
4. **Cierre** — `docs/config-first-run.md`, archivar change y planes.

## Criterio de aceptación

- Primer arranque sin `config.yml`: sin fallo, fichero creado con defaults, ventana visible, un solo proceso.
- `config.yml` inválido: error claro, salida con código != 0, sin panic, fichero intacto.
- `cargo test`, `cargo clippy --all-targets -- -D warnings` y `cargo fmt --check` en verde.
- Change validado con `openspec validate fix-config-first-run --strict`.
