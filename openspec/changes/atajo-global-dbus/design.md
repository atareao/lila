# Design

## Context

Ver `proposal.md` — Why. Restricciones que condicionan el diseño:

- LiLa es `gtk::Application::new(Some(APP_ID), ...)` (`src/main.rs`) y ya delega la instancia única en el bus de sesión vía GApplication (`connect_startup` solo corre en la primaria, `connect_activate` re-presenta).
- La ruta de re-presentación existe: `LaunchUi::present_existing()` hace `present()` + `grab_focus()` + limpia banner/`Entry` si la ventana estaba oculta, y arma el margen anti-pérdida de foco `PRESENT_FOCUS_GRACE = 250 ms`.
- `LaunchUi` guarda la ventana mientras vive el proceso (`connect_close_request` solo oculta).
- `Cargo.toml` no incluye crate D-Bus externa; se usa la integración de acciones de `gio`/GApplication.

## Goals / Non-Goals

**Goals:**
- Exponer `toggle` y `present` como acciones de la `GApplication` sin dependencias nuevas.
- Reutilizar `present_existing()` para que foco, margen y limpieza de estado sean idénticos a la invocación por binario.
- Dejar documentado un binding de compositor reproducible.

**Non-Goals:**
- Capturar el atajo global dentro de LiLa (Wayland no lo permite sin el portal/compositor).
- Auto-arrancar LiLa cuando la acción no tiene destinatario.
- UI de configuración del atajo, soporte X11 o multi-monitor.

## Decisions

- **Dos acciones: `toggle` primaria y `present` alias.** `toggle` permite un único binding para mostrar y ocultar, que es el caso de uso principal. Alternativa "solo `present`": obliga a un segundo binding para ocultar o a confiar en el auto-ocultado por pérdida de foco, que puede no ocurrir. Alternativa "solo `toggle`": impide re-presentar sin cambiar el estado visible (p. ej. re-centrar/limpiar de forma inequívoca). Ambas reutilizan la misma ruta; el coste es un `GSimpleAction` extra.
- **Registrar en `connect_startup`** (una vez, instancia primaria) con el action map de la `GApplication`, en vez de en `connect_activate` (que puede ejecutarse varias veces). Alternativa descartada: `set_accels_for_action()`; los accelerators GTK solo funcionan con la ventana enfocada, no sirven para un atajo global del compositor.
- **Decisión pura y testeable.** Un helper (`ActionTarget::from(visible)` o equivalente) traduce `(acción, visibilidad)` a `Present | Hide`; la acción solo lo aplica llamando a `present_existing()` o a un nuevo `hide_existing()`. Esto permite tests unitarios sin display, que es donde hay CI.
- **Trazado de acciones:** `toggle` decide con `window.is_visible()`; `present` no consulta el estado y delega en `present_existing()`.
- **Sin crate D-Bus:** GApplication exporta el action map automáticamente por `org.gtk.Actions` (`Activate`) y `org.freedesktop.Application` (`ActivateAction`). Alternativa descartada: `zbus`/`dbus` con un servicio propio, que añadiría dependencia y un registro manual innecesario.

### Contrato de interfaz

- Acciones: `toggle`, `present`.
- Object path: `/es/atareao/lila` (derivado de `APP_ID`).
- Métodos: `org.gtk.Actions.Activate(s action_name, av parameters, a{sv} platform_data)` y `org.freedesktop.Application.ActivateAction(s action_name, av parameter, a{sv} platform_data)`.
- Invocación de referencia:
  `gdbus call --session --dest es.atareao.lila --object-path /es/atareao/lila --method org.gtk.Actions.Activate toggle [] {}`
- Binding de ejemplo (Hyprland): `bind = SUPER, space, exec, gdbus call --session --dest es.atareao.lila --object-path /es/atareao/lila --method org.gtk.Actions.Activate toggle [] {}`

## Risks / Trade-offs

- [El binding del compositor ejecuta `gdbus`, un proceso efímero por pulsación] → aceptable: no arranca LiLa y el coste es mínimo; se documenta como alternativa al auto-ocultado. No existe API Wayland estándar para atajos sin proceso.
- [Si LiLa no está corriendo, la llamada falla con servicio desconocido] → mitigación documental: el compositor necesita un binding de arranque (`lila`) y el fallo no deja procesos colgados.
- [Ocultar por acción podría disparar el guard de pérdida de foco] → `hide_existing()` no debe re-verificar foco; el margen solo aplica a la presentación. Cubierto por el escenario "Atajo con el dashboard visible".
- [Colisión de nombres de acción con acciones internas de GTK] → se usan nombres propios (`toggle`, `present`) registrados explícitamente en el action map de la aplicación.

## Migration Plan

- Sin migración de datos ni de configuración: las acciones son aditivas.
- Despliegue: registrar las acciones al arrancar; el usuario añade el binding en su compositor cuando quiera.
- Rollback: eliminar el registro de acciones; el resto del comportamiento (invocación por binario, instancia única) queda intacto.

## Open Questions

- ¿Se documenta también un binding de arranque recomendado (p. ej. SUPER+space arranca LiLa si no está vivo) o se deja fuera del alcance? Diferible: no cambia specs, enfoque ni tareas.
