---
# PLAN-002 — Atajo global por D-Bus (follow-up 5.3)

- **Issue:** #14 (referencia al Milestone 1 #5)
- **Change OpenSpec:** `atajo-global-dbus`
- **Estado:** propuesta — pendiente de aprobación del usuario antes de escribir código
- **Contexto:** LiLa (launcher Wayland GTK4 + gtk4-layer-shell, Rust edition 2024). Tras cerrar el Milestone 1 (`plans/PLAN-001-ARCHIVED.md`), el atajo global del compositor se resuelve hoy lanzando el binario (singleton re-presenta la ventana), creando un proceso efímero por pulsación.

## Objetivo
Exponer una **acción D-Bus** sobre la `GApplication` (`APP_ID = es.atareao.lila`) para que el compositor invoque/presente/oculte LiLa desde un atajo global **sin crear procesos nuevos**, reutilizando la ruta de re-presentación y el guard de foco (margen 250 ms) ya existentes.

## Alcance (IN)
- Registrar un `GSimpleAction` (propuesto: `toggle`) en el action map de la aplicación.
- Semántica: `toggle` → si la ventana está visible, ocultar; si está oculta, presentar con foco y entry vacío. (Alternativa a confirmar: `present` sin ocultar.)
- Documentar el binding para compositores (niri/Hyprland/sway) con ejemplo `gdbus call`.
- Tests de la lógica pura de decisión (visible/oculto → acción).

## Fuera de alcance (OUT)
- Autoconfiguración del compositor (el usuario define su keybind).
- Añadir crate de D-Bus externa (se usa la integración nativa de `GApplication`).
- Extensiones del Milestone 2.

## Fases
- [ ] **E1 — Especificación.** Artefactos del change `atajo-global-dbus` (proposal/specs/design/tasks) escritos y validados (`openspec validate`).
- [ ] **E2 — Implementación.** `GSimpleAction` `toggle` registrado en `activate()` de `src/main.rs`, cableado a presentar/ocultar reutilizando la ruta de re-presentación y el guard de foco.
- [ ] **E3 — Tests (TDD).** RED→GREEN→REFACTOR: lógica pura de decisión toggle y (si aplica) nombre/path de la acción. `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check` en verde.
- [ ] **E4 — Documentación.** `docs/atajo-global-dbus.md` + sección en `README.md` con el binding `gdbus call` por compositor.
- [ ] **E5 — Verificación y cierre.** Verificación manual en Wayland (presenta/oculta sin nuevos procesos: `pgrep -c lila` estable); `openspec archive atajo-global-dbus`; PR de `feature/*` a `development`; cerrar #14.

## Escenarios de aceptación (BDD)
- **Toggle mostrando.** Given LiLa oculto, When el compositor invoca la acción `toggle`, Then la ventana se presenta con foco y entry vacío y `pgrep -c lila` no aumenta.
- **Toggle ocultando.** Given LiLa visible, When el compositor invoca `toggle`, Then la ventana se oculta.
- **Proceso ausente.** Given LiLa no está corriendo, When el compositor invoca la acción por D-Bus, Then la llamada falla limpiamente (destino inexistente) y el keybind documentado usa el arranque del binario como respaldo.

## Riesgos y notas
- Semántica toggle vs present a decidir por el usuario.
- El compositor debe permitir el keybind a un `gdbus call` (algunos prefieren ejecutar el binario); documentar ambos.
- No romper la instancia única ni el singleton ya probados en el Milestone 1.

## Metodología
SDD + TDD por tarea; issue #14 asignado a `atareao`; auditoría con `@rust-reviewer` antes de commit; commits gitmoji + conventional en `feature/*`; PR a `development`; nunca commit directo a `main`/`development`.
---