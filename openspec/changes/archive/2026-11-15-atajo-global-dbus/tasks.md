# Tasks

## 1. Decisión pura de la acción (TDD)

- [x] 1.1 (RED) Escribir tests unitarios de la traducción `(acción, visibilidad) → Present | Hide` para `toggle` y `present`, y ejecutarlos con `cargo test` esperando que fallen.
- [x] 1.2 (GREEN) Implementar el helper mínimo en `src/ui/` (sin GTK) que satisfaga los tests; `cargo test` en verde. — Desviación: el helper (`next_action`, `ShortcutAction`, `Resolution`) vive en `src/main.rs`, junto a las acciones que lo aplican, en vez de `src/ui/`; es lógica pura igualmente testeable.
- [x] 1.3 (REFACTOR) Limpiar nombres y documentar el helper; `cargo clippy --all-targets -- -D warnings` y `cargo fmt --check` en verde.

## 2. Registro de acciones en la GApplication

- [x] 2.1 (RED) Escribir un test que verifique que el helper y los nombres de acción exportados son los esperados (`toggle`, `present`); `cargo test` falla donde aún no existe el registro.
- [x] 2.2 (GREEN) Registrar los `GSimpleAction` en el action map durante `connect_startup` (`src/main.rs`), con `toggle` llamando a `present_existing()`/`hide_existing()` según visibilidad y `present` delegando siempre en `present_existing()`; `cargo test` y `cargo check` en verde.
- [x] 2.3 Añadir `hide_existing()` reutilizando el ocultado actual sin re-verificar el margen de foco; verificar con los tests de la decisión.
- [x] 2.4 (REFACTOR) Centralizar en `src/utils/constants.rs` los nombres de acción; `cargo clippy --all-targets -- -D warnings` en verde. — Desviación: el object path (`/es/atareao/lila`) NO se centraliza como constante; lo deriva GLib del `APP_ID` al exportar el action map, por lo que una const propia quedaría sin uso (`dead_code`). Queda documentado en el doc-comment de `TOGGLE_ACTION` (`src/utils/constants.rs`).
- [x] 2.5 Verificación manual (requiere sesión Wayland): con LiLa en marcha, `gdbus call --session --dest es.atareao.lila --object-path /es/atareao/lila --method org.gtk.Actions.Activate toggle [] {}` presenta con foco y `Entry` vacío, y una segunda llamada oculta; `pgrep -c lila` == 1.

## 3. Documentación del binding

- [x] 3.1 Escribir `docs/atajo-global-dbus.md` con: object path/interfaz, comando `gdbus`, semántica `toggle`/`present` y binding de ejemplo (Hyprland); verificar que el comando documentado funciona tal cual.
- [x] 3.2 Añadir una sección breve al `README.md` que enlace a `docs/atajo-global-dbus.md`.

## 4. Verificación de integración (#14)

- [x] 4.1 (Proceso ausente) Sin LiLa en ejecución, ejecutar el `gdbus call` y verificar que falla con servicio desconocido sin dejar procesos colgados.
- [x] 4.2 Confirmar que la invocación por binario sigue intacta: `lila & lila` → `pgrep -c lila` == 1 y la ventana se re-presenta con foco.
- [x] 4.3 Cerrar con la puerta de calidad completa: `cargo test`, `cargo clippy --all-targets -- -D warnings` y `cargo fmt --check` en verde.
