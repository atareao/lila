# Proposal

## Why

Hoy la única vía para re-presentar LiLa es volver a invocar el binario, lo que crea un proceso nuevo (aunque la instancia sea única) y añade latencia al atajo. Un atajo global del compositor Wayland debería presentar u ocultar el dashboard ya en marcha sin arrancar nada: la `GApplication` ya vive en el bus de sesión y solo falta exponer una acción que dispare ese comportamiento.

## What Changes

- Registrar acciones nombradas (`toggle` y `present`) en el action map de la `GApplication`, visibles por el bus de sesión a través de `org.gtk.Actions` / `org.freedesktop.Application`.
- `toggle`: dashboard oculto → presentar con foco y `Entry` vacío; dashboard visible → ocultar sin terminar el proceso.
- `present`: presentar siempre; si ya está visible, conservar el texto escrito (idempotente).
- Reutilizar la ruta de re-presentación existente (`present_existing`: foco, margen anti-pérdida de foco de 250 ms, limpieza de estado transitorio); sin proceso nuevo.
- Documentar el binding del compositor (p. ej. `gdbus call --session --dest es.atareao.lila --object-path /es/atareao/lila --method org.gtk.Actions.Activate toggle [] {}`).
- Sin cambios **BREAKING**: las acciones son aditivas y no alteran la invocación por binario.

## Capabilities

### New Capabilities
<!-- Ninguna: el comportamiento es otra vía de disparo de la invocación ya especificada. -->

### Modified Capabilities

- `launcher/invocation`: se añade una requirement de invocación por acción D-Bus (`toggle`/`present`) sin proceso nuevo, con foco y `Entry` vacío.

Se modifica `launcher/invocation` y no se crea una capability nueva porque esta capability ya cubre la instancia única y la re-presentación del dashboard bajo demanda; la acción D-Bus es una segunda vía de disparo del mismo comportamiento, no un dominio distinto. Introducir una capability aparte duplicaría las requirements de re-presentación y ocultado.

## Impact

- Código: `src/main.rs` (registro de acciones junto a `connect_startup`/`connect_activate`), posible helper de decisión en `src/ui/`; `src/utils/constants.rs` (nombres de acción y object path si se centralizan).
- APIs: interfaz pública en el bus de sesión (`org.gtk.Actions.Activate`, `org.freedesktop.Application.ActivateAction` en `/es/atareao/lila`).
- Dependencias: ninguna nueva; se usa la integración de acciones de `gio`/`GApplication` ya presente (gio/glib/gtk 0.20/0.9), sin crate D-Bus externa.
- Documentación: nuevo `docs/atajo-global-dbus.md` y nota de binding en `README.md`.
