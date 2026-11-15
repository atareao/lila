# atajo-global-dbus

- **Issue:** [#14](https://github.com/atareao/lila/issues/14)
- **Change OpenSpec:** `openspec/changes/atajo-global-dbus/`

## 1. Qué es y por qué

Hasta ahora la única vía para re-presentar LiLa era volver a invocar el binario `lila`. La instancia sigue siendo única (`GApplication`), pero cada pulsación **crea un proceso nuevo** que se registra como secundario en el bus y muere: añade latencia al atajo y trabajo inútil.

Esta feature expone el dashboard **ya en marcha** como **acciones nombradas de la `GApplication`**, alcanzables por el bus de sesión. El compositor enlaza una tecla a una llamada `gdbus` y esta dispara `toggle`/`present` **sobre el proceso vivo, sin arrancar nada**. Sin dependencias nuevas: se usa la integración de acciones de `gio`/`GApplication` que ya existía.

## 2. Ficheros implicados

| Fichero | Responsabilidad |
|---|---|
| `src/main.rs` | `enum ShortcutAction` (`Toggle`/`Present`) y `enum Resolution` (`Present { preserve_text }` / `Hide`); helper puro `next_action(visible, action)`; `LaunchUi::dispatch` ejecuta el efecto reutilizando `present_window`/`hide_existing`; `connect_startup` registra los `gio::SimpleAction` y conecta `connect_activate` → `ui.dispatch`. |
| `src/utils/constants.rs` | Nombres de acción `TOGGLE_ACTION = "toggle"` y `PRESENT_ACTION = "present"`; documenta que el object path `/es/atareao/lila` deriva de `APP_ID`. |

Sin crate D-Bus externa: `GApplication` exporta el **action map** automáticamente.

## 3. Cómo funciona

1. **Registro.** En `connect_startup` (solo corre en la instancia **primaria**), se crea un `gio::SimpleAction` por cada entrada de `SHORTCUT_ACTIONS` (`toggle`, `present`) y se añade al action map de la `GApplication` con `app.add_action()`. Se hace una sola vez, no en `connect_activate` (que puede ejecutarse varias veces).
2. **Exportación.** `GApplication` publica ese action map en el bus de sesión, con destino `es.atareao.lila` y object path `/es/atareao/lila` (derivado de `APP_ID`):
   - `org.gtk.Actions` → `Activate(s action_name, av parameters, a{sv} platform_data)`.
   - `org.freedesktop.Application` → `ActivateAction(s action_name, av parameter, a{sv} platform_data)`.
3. **Invocación.** El compositor llama al método con `toggle`/`present` y `parameters` vacío. Se emite `activate` del `SimpleAction`, que llama a `ui.dispatch(action)`.
4. **Decisión.** `dispatch` consulta la visibilidad real (`window.is_visible()`), traduce con `next_action` a un `Resolution` y aplica el efecto. La decisión es **pura y testeable sin display**; `dispatch` solo toca los widgets.
5. **Efecto.** `Present` → `present_window(preserve_text)` (misma ruta que la re-presentación por binario: `present()` + `grab_focus`, margen anti-pérdida de foco y limpieza condicional). `Hide` → `hide_existing()` (`window.close()`), que solo oculta.

## 4. Semántica exacta

`next_action(visible, action)`:

| Acción | Dashboard oculto | Dashboard visible |
|---|---|---|
| `toggle` | `Present { preserve_text: false }` → presenta con foco y **`Entry` vacío** | `Hide` → oculta **sin terminar el proceso** |
| `present` | `Present { preserve_text: false }` → presenta con foco y `Entry` vacío | `Present { preserve_text: true }` → re-presenta con foco y **conserva el texto** (idempotente) |

- **`toggle`**: un único binding sirve para mostrar y ocultar. Al mostrarse desde oculto se parte de estado limpio (banner y `Entry`); al ocultarse, el proceso queda vivo y listo para la siguiente invocación.
- **`present`**: presenta siempre; si ya estaba visible **no** borra lo que el usuario llevaba escrito. Es el alias para re-centrar/re-enfocar sin cambiar el estado.
- **Foco**: la presentación arma el mismo margen `PRESENT_FOCUS_GRACE = 250 ms` que la invocación por binario, de modo que una pérdida transitoria de foco del compositor al mapear no oculte la ventana. El ocultado por acción **no** re-verifica ese margen: el margen solo aplica a la presentación.

## 5. Cómo habilitar

Enlaza una tecla del compositor a la llamada D-Bus. Comando de referencia:

```bash
gdbus call --session --dest es.atareao.lila --object-path /es/atareao/lila \
  --method org.gtk.Actions.Activate toggle [] {}
```

Cambia `toggle` por `present` para el alias idempotente. Ejemplos de binding:

- **Hyprland** (`~/.config/hypr/hyprland.conf`):
  `bind = SUPER, space, exec, gdbus call --session --dest es.atareao.lila --object-path /es/atareao/lila --method org.gtk.Actions.Activate toggle [] {}`
- **niri** (`~/.config/niri/config.kdl`):
  `Mod+Space { spawn "gdbus" "call" "--session" "--dest" "es.atareao.lila" "--object-path" "/es/atareao/lila" "--method" "org.gtk.Actions.Activate" "toggle" "[]" "{}"; }`
- **Sway** (`~/.config/sway/config`):
  `bindsym $mod+space exec gdbus call --session --dest es.atareao.lila --object-path /es/atareao/lila --method org.gtk.Actions.Activate toggle [] {}`

Estos compositores permiten ejecutar un comando arbitrario en un binding, que es el mecanismo que usamos. **Alternativa de respaldo:** si prefieres no depender de `gdbus`, enlazar la ejecución del binario `lila`. Funciona siempre (la instancia única re-presenta la ventana), a costa de crear un proceso efímero por pulsación.

**Si LiLa no está corriendo**, la llamada D-Bus falla con *servicio desconocido* y **no deja procesos colgados**; el compositor necesita un binding de arranque (`lila`) y, a partir de ahí, el atajo D-Bus ya tiene destinatario. Arrancar LiLa automáticamente cuando la acción no tiene destinatario queda fuera de alcance.

## 6. Decisiones de diseño y follow-ups

- **Dos acciones: `toggle` primaria y `present` alias.** `toggle` da un único binding para mostrar y ocultar (caso de uso principal). `present` cubre re-presentar sin cambiar el estado visible. Ambas reutilizan la misma ruta; el coste es un `GSimpleAction` extra. Alternativas descartadas: solo `present` (obligaría a un segundo binding o a confiar en el auto-ocultado por pérdida de foco, que puede no ocurrir) y solo `toggle` (impide re-presentar sin alternar).
- **Registrar en `connect_startup`, no en `connect_activate`.** `startup` corre una única vez en la instancia primaria; `activate` puede repetirse y duplicaría acciones.
- **`set_accels_for_action` descartado.** Los accelerators GTK solo actúan con la ventana enfocada; no sirven para un atajo global del compositor.
- **Decisión pura y testeable.** `next_action` traduce `(acción, visibilidad)` a `Present`/`Hide` sin GTK, así que los escenarios se cubren con tests unitarios sin display (`cargo test`).
- **Sin crate D-Bus.** `GApplication` exporta el action map por `org.gtk.Actions`/`org.freedesktop.Application`; un servicio `zbus`/`dbus` propio añadiría dependencia y registro manual innecesarios.
- **Trade-off aceptado:** el binding ejecuta `gdbus`, un proceso efímero por pulsación. No arranca LiLa y el coste es mínimo; no existe API Wayland estándar para atajos sin proceso. Alternativa aceptada y documentada arriba: ejecutar el binario.
- **Follow-up (Open Question del change):** documentar un binding de arranque recomendado (p. ej. que `SUPER+space` arranque LiLa si no está vivo) — diferible, no cambia specs ni enfoque.
