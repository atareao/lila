# Design

## Context

Ver `proposal.md` — Why. Restricciones que condicionan el diseño:

- Crate binario único (Rust edition 2024), GTK4 + gtk4-layer-shell; los tests deben correr sin display, así que toda decisión con valor debe vivir fuera del código GTK.
- `App` proviene de la crate externa `applications`, por lo que **no se pueden añadir campos** a `Item::App`: cualquier metadato adicional tiene que vivir en una caché externa indexada por la ruta del `.desktop`.
- `IdCache` (`src/models/item.rs`) ya establece el patrón: caché por ruta, memoriza solo resoluciones correctas, reintenta tras un fallo. A2 y C deben copiar esa política para no divergir.
- El ranking ya afirma cargar el historial una sola vez por búsqueda (`src/finders/ranking.rs:42`); C lleva esa propiedad al eje temporal (entre pulsaciones).
- La puerta de calidad exige clippy con `-D warnings`, de modo que un `allow` retirado no puede dejar un `dead_code` real.

## Goals / Non-Goals

**Goals:**
- Eliminar las supresiones de lint sin justificación y los símbolos sin consumidor.
- Que las palabras clave y las categorías influyan en la búsqueda sin coste de E/S por pulsación.
- Definir una política de CSS que nunca aborte el arranque y nunca destruya lo escrito por el usuario.
- Que teclear no consulte SQLite y que un lanzamiento se refleje inmediatamente en el ranking.

**Non-Goals:**
- Cambiar pesos, fórmula de puntuación o `max_results`.
- Recarga en caliente del CSS o edición del fichero del usuario desde la UI.
- Añadir capacidades nuevas al launcher (Milestone 2).

## Decisions

- **A: eliminar antes que silenciar.** Para `Item::exec()` y `Finder::update()` se elige **borrar** en lugar de mantener el `allow`: no tienen consumidor (el lanzamiento usa `DesktopAppInfo`; `main.rs` solo llama a `Finder::init()`), y mantener el `allow` perpetúa la duda. Alternativa descartada: conservarlos cableándolos a la UI, que sería inventar comportamiento fuera del alcance de #16. Los `allow` de `src/history/mod.rs` sí se conservan: `usage()`/`score()`/`usage_score()` son el contrato del trait `HistoryRepository` y se ejercitan desde los tests del módulo; borrarlos recortaría el trait.
- **A2: caché de metadatos análoga a `IdCache`, no campos nuevos.** Como `App` es externo, los `Keywords`/`Categories` se leen con `DesktopAppInfo` y se memorizan por ruta en `MetadataCache`. Alternativa descartada: leerlos dentro del bucle de puntuación, que multiplicaría las aperturas de `.desktop` por pulsación (justo el problema que #16 señala).
- **A2: el término difuso es el máximo sobre nombre, palabras clave y categorías.** Se descarta penalizar la coincidencia por palabra clave (por ejemplo, un factor 0.9): introduciría un peso nuevo, sin configuración, que el usuario no puede ajustar y que complica la explicación del orden. Con el máximo, el ranking sigue siendo "cuanto mejor coincide, más arriba", sea cual sea el campo que coincida, y el comportamiento previo se conserva cuando el `.desktop` no declara metadatos (el máximo degenera en la puntuación por nombre).
- **A2: descartar el ítem solo si no coincide en ningún ámbito.** Con consulta no vacía, un ítem que no coincide ni por nombre ni por metadatos se descarta igual que hoy; es decir, se amplía el conjunto de coincidencias, nunca se reduce.
- **B: `Css::load()` sin `Result` y con decisión pura.** Devolver siempre un `CssProvider` y registrar los fallos con `tracing` traslada la política al código de decisión y quita del punto de llamada el `.expect(...)` que hoy puede matar la aplicación. La decisión (de dónde se carga el CSS) se aísla en una función pura para poder cubrirla en CI sin display; el pegamento GTK queda reducido a aplicar la decisión.
- **B: precedencia fichero del usuario > CSS embebido.** Si el fichero existe se usa tal cual, aunque su contenido resulte inválido para GTK (GTK ya reporta el error de parseo de CSS; LiLa no lo interpreta ni lo valida). Solo se cae al embebido cuando el fichero no se puede leer. Alternativa descartada: validar el CSS propio en LiLa, que exigiría un parser CSS y duplicaría lo que GTK ya hace.
- **B: no sobrescribir nunca un fichero existente.** Evita destruir personalizaciones; el coste es que un `style.css` vacío creado por el usuario se respeta (queda a su cargo). Es preferible a perder su trabajo.
- **C: decorador sobre el trait, no caché dentro de `SqliteHistory`.** Un `CachedHistory` que envuelve cualquier `HistoryRepository` es testeable con un repositorio en memoria y no acopla la caché al motor de persistencia. Alternativa descartada: meter la caché en `Sql,iteHistory`, que haría intestable la política sin SQLite y mezclaría responsabilidades.
- **C: invalidar en `record()`, memorizar solo los `Ok`.** La invalidación en la escritura es lo que hace que un lanzamiento reorden cambie el ranking en la siguiente pulsación; un `Err` no se memoriza para que un fallo transitorio (por ejemplo, bloqueo de la base de datos) no se quede pegado toda la sesión.
- **Riesgo asumido en C:** la caché es por proceso. Si otro proceso escribiera el historial, el orden no lo reflejaría hasta el siguiente `record()` local. LiLa es instancia única y el único escritor, así que el riesgo es nulo en la práctica; se documenta en `docs/`.

## Open Questions

- Precedencia nombre vs palabra clave: la propuesta es "máximo sin penalización". Confirmar en la aprobación de la spec.
- ¿Debe la caché de metadatos compartir su política de reintento con `IdCache` en un único tipo genérico o mantenerse como dos tipos paralelos? Se propone **dos tipos paralelos** por simplicidad de lectura; unificarlos queda como posible refactor posterior.