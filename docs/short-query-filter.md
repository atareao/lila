# Filtrado de consultas de un carácter

## Qué es

Documenta el change `short-query-filter` (issue [#39](https://github.com/atareao/lila/issues/39)):
una consulta de **un solo carácter** deja de coincidir por `Categories`. El filtrado por
`Name` y `Keywords` sigue funcionando igual, y las consultas de 2+ caracteres no cambian.

## Problema

Tras el check del issue #16 se comprobó en Wayland que teclear `u` hacía coincidir casi
todas las aplicaciones del índice: los tokens de `Categories` de muchos `.desktop`
(`Utility;`, `System;`…) contienen esa letra y el motor fuzzy no aplica umbral. El
listado resultante era inútil. La causa no era la UI ni el historial, sino que
`best_fuzzy_match` trataba por igual `Name`, `Keywords` y `Categories`.

## Ficheros implicados

- `src/finders/application.rs` — `best_fuzzy_match` (punto de cambio) y `filter_items`.
- `src/finders/ranking.rs` — `rank_items`, ruta con historial que también pasa por
  `best_fuzzy_match`.
- `openspec/changes/short-query-filter/specs/launcher/result-ranking/spec.md` — delta de la
  spec de ranking de resultados.
- `plans/PLAN-005.md` — plan de la feature.

## Cómo funciona

`best_fuzzy_match` aplica un guard antes de construir el iterador de coincidencias:

- Si `query.trim().chars().count() == 1`, se omiten los tokens de `categories`; solo se
  consideran `name()` y `keywords`.
- Se usa `chars().count()` y no `len()` para que un carácter Unicode multibyte (p. ej. `ñ`)
  cuente como 1 y no como 2 bytes.
- Con 2+ caracteres el comportamiento es intacto: `Categories` vuelve a participar, de modo
  que `spreadsheet` sigue encontrando una aplicación cuyo nombre no contiene esa palabra.
- El guard es compartido: `filter_items` (sin historial) y `rank_items` (con historial,
  ponderación por frecuencia/recencia) ven la misma regla. No hay ramas especiales en la UI.
- El recuento se hace sobre `trim()`, así que un espacio alrededor del carácter no lo
  convierte en una consulta de 2 caracteres.

## Cómo verificar

Puerta de calidad ya ejecutada por CLI en la rama `feature/short-query-filter`:

```bash
cargo test                                  # 131/131 en verde
cargo check                                 # sin errores
cargo fmt --check                           # limpio
cargo clippy --all-targets -- -D warnings  # limpio
openspec validate short-query-filter --strict
```

Comprobación manual sugerida (pendiente; no automatizada) en un compositor Wayland con
layer-shell:

1. Arrancar lila en una sesión Wayland y teclear `u`: el listado ya no debe inundarse de
   aplicaciones por `Categories`; deben quedar solo coincidencias por `Name`/`Keywords`.
2. Teclear `spreadsheet`: debe seguir apareciendo la aplicación por su categoría.
3. Comprobar un carácter multibyte (`ñ`) y una consulta de 2+ caracteres como `process` para
   confirmar que no hay regresión.
4. Para no tocar la config del usuario, se puede lanzar con
   `XDG_CONFIG_HOME=/tmp/opencode/e2e-xdg` (copia previa de `config.yml` y `style.css`),
   teclear con `ydotool type` y capturar con `grim`. lila se oculta al perder el foco: hay
   que teclear solo mientras tiene la activación y no enviar Enter accidentalmente.

## Decisiones

- **Opción B (elegida):** excluir `categories` en consultas de 1 carácter dentro de
  `best_fuzzy_match`. Es un único punto de cambio que cubre `filter_items` y `rank_items`.
- **Descartada (opción A):** exigir longitud mínima en el entry de `main.rs`. Bloquearía
  búsquedas legítimas por nombre y no cubriría llamadas directas a `rank_items` o
  `filter_items`.
- No se modifica la UI (`src/main.rs`) ni `Config` (`src/models/config.rs`): la regla es
  pura y no añade opciones de configuración ni formatos nuevos en disco.

## Follow-ups

- **Verificación manual en Wayland pendiente**: se decidió no automatizar teclado sobre el
  escritorio del usuario; queda registrada en `tasks.md` (2.1) y en `PLAN-005.md`.
- Los `Keywords` de una sola letra siguen siendo amplios por diseño: el guard no los
  excluye. Si resulta molesto, merece un change separado.
- Issue [#40](https://github.com/atareao/lila/issues/40) (ítems con `Name` vacío) se
  aborda después, en sesión separada.
- AUR (#38) queda aparcado por decisión del usuario.
