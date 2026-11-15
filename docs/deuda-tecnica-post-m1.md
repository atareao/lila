# deuda-tecnica-post-m1

- **Issue:** [#16](https://github.com/atareao/lila/issues/16)
- **Plan:** `plans/PLAN-003.md`
- **Change OpenSpec:** `openspec/changes/deuda-tecnica-post-m1/`

Cierra la deuda técnica del core del launcher en cuatro puntos —limpieza de `allow(dead_code)`, metadatos de `.desktop` en la búsqueda, política explícita de carga del CSS y caché del historial entre pulsaciones— sin cambiar el ranking más allá de la coincidencia ampliada por palabras clave y categorías.

## 1. Qué es

### A — Limpieza de `#[allow(dead_code)]`
Se retiran las supresiones de lint sin justificación: se **eliminan** `Item::exec()` y el envoltorio `Finder::update()` (no tienen consumidor en producción); se deja de ocultar `split_desktop_list()` (que pasa a usarse en A2) y `Item::keywords()`/`Item::categories()`, que quedan `#[cfg(test)]` como oráculo sin caché solo para los tests en lugar de reintroducir un `allow` (ver §6).
**Gana el proyecto:** el código deja de arrastrar símbolos muertos y supresiones que perpetuaban la duda; clippy se mantiene en `-D warnings` sin `allow` residuales entre los cuatro retirados.

### A2 — Metadatos en la búsqueda
La consulta pasa a coincidir también con las palabras clave (`Keywords`) y las categorías (`Categories`) del `.desktop`, no solo con el nombre. Los metadatos se resuelven con una caché por ruta (`MetadataCache`) para no reabrir el `.desktop` en cada pulsación.
**Gana el usuario:** puede encontrar una aplicación por una palabra que no aparece en su nombre (p. ej. `photo` → una app con `Keywords=photos;image;editor;`).

### B — Política del CSS
`Css::load()` deja de devolver `Result` y **nunca aborta**: define la precedencia (fichero del usuario → CSS embebido por defecto), no sobrescribe nunca un fichero existente y degrada con un aviso ante cualquier problema.
**Gana el usuario:** un `~/.config/lila` no escribible o un `style.css` ilegible ya no matan el arranque; el dashboard se muestra con el estilo por defecto.

### C — Caché del historial
Un decorador `CachedHistory` memoriza `load_all()` y lo invalida en `record()`, de modo que teclear no consulta SQLite y un lanzamiento se refleja en la siguiente búsqueda.
**Gana el proyecto:** desaparece una consulta completa a la base de datos por cada pulsación, sin alterar el orden de resultados.

## 2. Ficheros implicados

| Fichero | Cambio |
|---|---|
| `src/models/item.rs` | **A:** eliminado `Item::exec()` y reescritos sus tests como tests de `icon_path()`; retirados los `allow(dead_code)` de `keywords()`/`categories()` (que quedan `#[cfg(test)]`) y el `TODO` de `split_desktop_list()`. **A2:** añadidos `MetadataCache` y los helpers `keywords_of`/`categories_of`, compartidos con los accesores directos. |
| `src/models/mod.rs` | Exporta `MetadataCache`. |
| `src/models/css.rs` | **B:** función pura `resolve_stylesheet()` (con `ResolvedStylesheet`); `Css::load()` devuelve siempre `CssProvider` y registra los avisos. |
| `src/history/cache.rs` (nuevo) | **C:** decorador `CachedHistory` con su batería de tests. |
| `src/history/mod.rs` | Declara `mod cache` y reexporta `CachedHistory`; se conservan los tres `allow(dead_code)` del trait con su comentario. |
| `src/finders/finder.rs` | **A:** eliminado el envoltorio `Finder::update()` (se conserva `Find::update()`); `Finder::search` recibe `&MetadataCache`. |
| `src/finders/application.rs` | **A2:** `filter_items` y el nuevo `best_fuzzy_match` puntúan contra nombre, palabras clave y categorías. |
| `src/finders/ranking.rs` | **A2:** `rank_items` recibe la caché de metadatos y usa `best_fuzzy_match`. **C:** tests de regresión con y sin `CachedHistory`. |
| `src/finders/empty.rs` | Ajusta la firma de `search` a `&MetadataCache`. |
| `src/main.rs` | **A2:** construye `MetadataCache` en `activate` y lo cablea a la búsqueda. **B:** `Css::load()` sin `.expect`. **C:** envuelve `SqliteHistory` en `CachedHistory` y lo comparte. |
| `Cargo.lock` | Bump de la versión del crate `0.1.0 → 0.2.0`. |
| `openspec/changes/deuda-tecnica-post-m1/`, `plans/PLAN-003.md` | Spec y plan del cambio (A y C sin delta de spec; A2 y B con delta). |

## 3. Cómo funciona

### 3.1 A2 — Metadatos en la búsqueda

**`MetadataCache`** (`src/models/item.rs`) es una caché por ruta del `.desktop`.
- **Clave:** la ruta del `.desktop` (`item.desktop_path()`), un `PathBuf`; el valor es `Metadata { keywords, categories }`.
- **Qué memoriza:** toda resolución **correcta**, incluido el vector vacío de una app que legítimamente no declara metadatos. Una sola lectura (`DesktopAppInfo::from_filename`) cubre las dos claves y `keywords()`/`categories()` sirven cada campo desde el mismo registro.
- **Qué no memoriza:** un fallo de resolución (`.desktop` inexistente o ilegible). `ensure_resolved` sale **sin memorizar** el fallo, de modo que la siguiente consulta reintenta una sola lectura y una app instalada más tarde se resuelve sin reiniciar. Entre tanto, `keywords()`/`categories()` devuelven el vector vacío **sin reabrir el fichero que acaba de fallar** (no caen al respaldo de `Item`, que volvería a golpear el disco).
- **Por qué:** `App` viene de la crate externa `applications`, así que no se pueden añadir campos a `Item::App`; los metadatos tienen que vivir en una caché externa indexada por ruta. Reutiliza la misma política que `IdCache` (memorizar solo aciertos, reintentar fallos) para no divergir.

**`best_fuzzy_match`** (`src/finders/application.rs`, `pub(super)`) calcula el término difuso de un ítem: concatena el nombre con **cada** palabra clave y **cada** categoría y los puntúa **por separado** (nunca concatenados), devolviendo el **máximo** de `SkimMatcherV2::fuzzy_match`. Un `None` en todos los ámbitos descarta el ítem cuando la consulta no está vacía. Lo usan tanto `filter_items` como `rank_items`, de modo que el término difuso no puede divergir entre la ruta directa y la rankeada.

**Tres rutas de respaldo en `rank_items`** (`src/finders/ranking.rs`): las tres caen al ranking difuso puro (`filter_items`), siempre recortado a `cfg.max_results`:
1. `cfg.history_enabled == false` → difuso puro.
2. no hay repositorio de historial (`history == None`) → difuso puro.
3. `history.load_all()` devuelve `Err` → `tracing::warn!` y difuso puro.

Solo cuando hay historial activo y una carga correcta se combinan los pesos: `fuzzy_weight * best_fuzzy_match + frequency_weight * frecuencia + recency_weight * recencia`. Con consulta vacía el término difuso es `0.0` y ordenan frecuencia y recencia. El historial completo se carga una sola vez por búsqueda, nunca por ítem.

### 3.2 B — Política del CSS

**`resolve_stylesheet(config_dir: Option<&Path>) -> ResolvedStylesheet`** (`src/models/css.rs`) es una función **pura respecto a la decisión**: recibe el directorio de configuración y devuelve el CSS a aplicar más los avisos a registrar. **Nunca falla.**

| Caso | Decisión | Registro |
|---|---|---|
| Sin directorios XDG (`config_dir == None`) | CSS por defecto | nada |
| `style.css` existe y es legible | contenido del fichero del usuario, sin sobrescribir | `debug!` con el directorio |
| `style.css` existe pero no es legible | CSS por defecto en memoria, fichero intacto | `warn!` |
| `style.css` es un enlace simbólico roto | «existe pero no es legible»: CSS por defecto en memoria, **no** se crea el destino del enlace | `warn!` |
| `style.css` ausente y el directorio es escribible | `create_dir_all` + escribe el CSS por defecto y lo aplica | `debug!("Saved")` |
| `style.css` ausente y el directorio no es escribible | CSS por defecto en memoria | `warn!` |

La detección de existencia usa `fs::symlink_metadata(&path)`, **no** `path.exists()`: `exists()` sigue el enlace y devolvería `false` para un symlink roto, con lo que `fs::write` habría seguido el enlace y creado su destino al materializar el CSS por defecto. Con `symlink_metadata` un enlace roto cuenta como «existe» y no se toca.

El caso «ausente» funde creación del directorio y escritura en un único `fs::create_dir_all(...).and_then(|()| fs::write(...))`; si cualquiera de los dos pasos falla se acumula un aviso y se aplica igualmente el CSS en memoria. Cada ruta construye el CSS por defecto **como mucho una vez** (se elimina la doble llamada a `Css::default()` del código anterior).

**`Css::load()` ya no devuelve `Result`:** obtiene el `config_dir` de `ProjectDirs`, llama a `resolve_stylesheet`, emite cada aviso con `warn!` y devuelve siempre un `CssProvider` con `provider.load_from_string(&resolved.css)`. Un problema de estilos se degrada al CSS embebido y deja solo un aviso en el log; el `.expect("Can not load CSS file")` que hoy puede matar la aplicación desaparece del punto de llamada (`src/main.rs`).

**`load_from_string` en lugar de `load_from_path`:** antes el provider se cargaba desde la ruta del fichero (`load_from_path`), ahora se le pasa el contenido ya leído. El CSS se aplica igual, pero **las referencias relativas del fichero del usuario dejan de resolverse respecto a `~/.config/lila/style.css`**: un `url()` o un `@import` relativo no tienen ya el directorio del `style.css` como base. Quien personalice el estilo y referencie recursos (fuentes, imágenes, otras hojas) debe usar rutas absolutas o `url("file:///...")`.

### 3.3 C — Caché del historial

**`CachedHistory`** (`src/history/cache.rs`) envuelve a cualquier `HistoryRepository`.
- **Qué memoriza:** el resultado de `load_all()` (`Vec<(String, Usage)>`) en un `RefCell<Option<...>>`. Mientras no se invalide, las siguientes llamadas devuelven una **copia** de lo memorizado sin tocar el repositorio interno (ni, por tanto, SQLite). El préstamo del `RefCell` se suelta antes de delegar en el interno para no encontrarlo prestado.
- **Cuándo invalida:** en cada `record()` **correcto** (tras `inner.record(...)?` se pone la caché a `None`), de modo que un lanzamiento reordena el ranking en la siguiente pulsación. Un `record()` fallido deja la caché intacta: el historial no cambió.
- **Por qué un fallo no se memoriza:** solo se guarda el `Ok`. Un `Err` (p. ej. base de datos bloqueada) no se queda pegado toda la sesión; la siguiente llamada vuelve a intentarlo.
- **`usage()` delega sin caché:** consultar un ítem suelto no invalida ni rellena la carga completa.
- **Por qué la caché es por proceso y no rompe nada:** vive lo que viva la instancia de `CachedHistory`, creada en `startup`. LiLa es una aplicación de **instancia única** (`GApplication`) y el **único escritor** de `history.db`, así que ningún otro proceso puede dejar obsoleto el valor guardado. El riesgo de que una escritura externa no se refleje hasta el siguiente `record()` local es nulo en la práctica.

Cableado en `src/main.rs`: `LaunchState.history: Option<Rc<CachedHistory>>`, construido como `SqliteHistory::open() → Some(Rc::new(CachedHistory::new(Box::new(store))))` y compartido con `record_launch()` y con el closure de ranking (convertido a `&dyn HistoryRepository`).

## 4. Configuración / variables

No hay **ninguna** opción nueva ni cambio de esquema. La configuración existente sigue en el directorio XDG de la aplicación:

- `~/.config/lila/config.yml` — `Config` (pesos, `max_results`, `history_enabled`, extensiones).
- `~/.config/lila/style.css` — hoja de estilos del usuario.
- `~/.config/lila/history.db` — historial de uso (SQLite).

Lo que cambia en la práctica para el usuario:
- Un `~/.config/lila` no escribible (o un `style.css` ilegible) **ya no impide arrancar**: se aplica el estilo por defecto en memoria y queda un `warn!` en el log.
- Buscar por palabra clave o categoría funciona sin que el término esté en el nombre de la aplicación.
- El fichero `style.css` existente **nunca** se sobrescribe.

## 5. Cómo habilitar / desplegar

Nada especial: `cargo build` (requiere `gtk4-layer-shell` instalado como librería del sistema) y ejecutar el binario bajo un compositor Wayland con layer-shell. No hay migraciones ni pasos de configuración.

La puerta de calidad y los tests **corren sin display**:

```bash
cargo test                                  # 115 tests, sin necesidad de Wayland
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

## 6. Decisiones

- **A: eliminar antes que silenciar.** `Item::exec()` y `Finder::update()` no tienen consumidor (el lanzamiento usa `gio::DesktopAppInfo`; `main.rs` solo llama a `Finder::init()`), así que se borran en lugar de conservar el `allow`. Alternativa descartada: cablearlos a la UI, que sería inventar comportamiento fuera del alcance de #16. Los tres `allow` de `src/history/mod.rs` sí se conservan: `usage()`, `score()` y `usage_score()` son el contrato del trait `HistoryRepository` y se ejercitan desde los tests del módulo; borrarlos recortaría el trait.
- **Desviación del `PLAN-003.md` (hallazgo H2 de la auditoría): `keywords()`/`categories()` como oráculo de test.** El plan preveía retirar su `allow(dead_code)` porque «pasarían a usarse en A2». Al cerrar H2 —eliminar la doble lectura del camino de fallo de `MetadataCache`— la ruta de producción lee los metadatos siempre a través de la caché, así que `Item::keywords()` y `Item::categories()` se quedaron sin consumidor de producción. En lugar de reintroducir un `#[allow(dead_code)]` se marcaron `#[cfg(test)]` y quedan documentadas como **oráculo sin caché** con el que los tests contrastan `MetadataCache`; la decisión es no silenciar código muerto.
- **A2: caché análoga a `IdCache`, no campos nuevos.** `App` es externo, de modo que los metadatos se memorizan por ruta en `MetadataCache`. Alternativa descartada: leerlos dentro del bucle de puntuación, que multiplicaría las aperturas de `.desktop` por pulsación.
- **A2: el término difuso es el máximo sobre nombre, palabras clave y categorías.** Se descarta penalizar la coincidencia por palabra clave (p. ej. un factor `0.9`): introduciría un peso nuevo, sin configuración, que complica la explicación del orden. Con el máximo el ranking sigue siendo «cuanto mejor coincide, más arriba»; el comportamiento previo se conserva cuando el `.desktop` no declara metadatos (el máximo degenera en la puntuación por nombre).
- **A2: descartar el ítem solo si no coincide en ningún ámbito.** Con consulta no vacía se amplía el conjunto de coincidencias, nunca se reduce.
- **B: `Css::load()` sin `Result` y con decisión pura.** Devolver siempre un `CssProvider` y registrar los fallos con `tracing` traslada la política al código de decisión y quita del punto de llamada el `.expect(...)` que puede matar la aplicación. La decisión se aísla en `resolve_stylesheet()` para cubrirla en CI sin display.
- **B: precedencia fichero del usuario > CSS embebido.** Si el fichero existe se usa tal cual, aunque su contenido resulte inválido para GTK (GTK ya reporta el error de parseo; LiLa no lo valida). Alternativa descartada: validar el CSS propio en LiLa, que exigiría un parser y duplicaría lo que GTK ya hace.
- **B: no sobrescribir nunca un fichero existente.** Evita destruir personalizaciones; un `style.css` vacío creado por el usuario se respeta y queda a su cargo. Es preferible a perder su trabajo.
- **C: decorador sobre el trait, no caché dentro de `SqliteHistory`.** Un `CachedHistory` que envuelve cualquier `HistoryRepository` es testeable con un repositorio en memoria y no acopla la caché al motor de persistencia. Alternativa descartada: meter la caché en `SqliteHistory`, que haría intestable la política sin SQLite y mezclaría responsabilidades.
- **C: invalidar en `record()`, memorizar solo los `Ok`.** La invalidación en la escritura es lo que hace que un lanzamiento cambie el ranking en la siguiente pulsación; un `Err` no se memoriza para que un fallo transitorio no se quede pegado toda la sesión.
- **`impl Debug` manual en `CachedHistory`.** `#[derive(Debug)]` no es posible porque el campo `inner: Box<dyn HistoryRepository>` no implementa `Debug`; la implementación manual (`impl fmt::Debug`) expone un único campo, `memoized: bool` (si hay valor memorizado), y cierra con `finish_non_exhaustive` para no revelar el repositorio interno.

## 7. Verificación

**Tests (sin display):** la rama tiene **115 tests** frente a los **92** de `development` (+23). La feature pasó auditoría de código y los hallazgos importantes quedaron cerrados. Cubren, como tests unitarios:

- `MetadataCache`: devuelve keywords/categorías del `.desktop`, memoriza cada ruta (no relee el fichero), memoriza el vector vacío y reintenta un fallo de resolución.
- Coincidencia ampliada en `filter_items` y `rank_items`: consulta que solo coincide por palabra clave, que solo coincide por categoría, app sin metadatos puntuada como antes (solo por nombre) y, con dos ítems que coinciden con la misma consulta (uno por nombre, otro solo por palabra clave), ambos presentes sin penalizar la coincidencia por nombre.
- Función pura del CSS: fichero legible → se usa y no se sobrescribe; fichero ilegible → CSS por defecto con aviso; enlace simbólico roto → CSS por defecto con aviso y sin crear el destino del enlace; ausente y escribible → se escribe y se aplica; directorio no escribible → CSS por defecto en memoria con aviso; sin XDG → CSS por defecto sin aviso.
- `CachedHistory`: N `load_all()` → una consulta al interno; `record()` invalida (+1 recarga); `load_all()` fallido no se memoriza; `record()` fallido conserva la caché; `usage()` delega sin interferir.
- Regresión de ranking: envolver el historial en `CachedHistory` devuelve el **mismo orden** que sin decorar y consulta el repositorio interno una sola vez.

**Verificación manual en Wayland — pendiente.** Queda fuera de los tests, que corren sin display, y **no se ha ejecutado**: en este entorno headless no se puede arrancar la aplicación. Hay que hacerla en una sesión Wayland con layer-shell:

```bash
cargo build
./target/debug/lila          # o: cargo run
```

- Teclear en el dashboard sigue filtrando igual que antes.
- Una app cuyo `.desktop` declara `Keywords` coincide al buscar por esa palabra aunque no esté en su nombre.
- En el primer arranque se escribe y aplica el estilo por defecto.
- La instancia sigue siendo única: `pgrep -c lila` devuelve `1`.
- Un `~/.config/lila` no escribible no aborta el arranque (solo un `warn!` en el log).

## 8. Follow-ups

- **Los tres `#[allow(dead_code)]` de `src/history/mod.rs`** (`usage()`, `score()`, `usage_score()`): se mantienen porque son el contrato del trait `HistoryRepository` (los implementan todos los repos, incluidos los de test) y solo los ejercitan los tests del módulo; la ruta de ranking lee el historial de una vez con `load_all()` y no los invoca. Retirarlos exigiría recortar el trait.
- **Divergencia de clave entre escritura y lectura del historial:** `record_launch()` (`src/main.rs`) usa `item.id()` mientras el ranking consulta con `IdCache::id`. Hoy no divergen en la práctica, pero conviene unificarlos para que la clave con la que se escribe y la clave con la que se lee no puedan separarse.
- **Consultas cortas o genéricas con `Categories`:** incluir las categorías hace que consultas de un solo carácter (p. ej. `u` o `a`) casen con casi cualquier aplicación. Es el comportamiento sancionado por la spec («el conjunto de coincidencias solo se amplía, nunca se reduce»), pero hay que verificarlo en la comprobación manual; candidato a follow-up: exigir una longitud mínima de consulta o no usar categorías en consultas de un carácter.
- **Fixture `EXEC_BINARY = "/usr/bin/env"`:** los tests que leen metadatos necesitan un `Exec` que resuelva, porque GLib descarta un `.desktop` cuyo binario no existe; conviene una guarda que lo detecte si algún día falta en el entorno de CI.
- **Posible unificación de `IdCache` y `MetadataCache`** en un único tipo genérico de caché por ruta: se propuso mantener dos tipos paralelos por simplicidad de lectura; unificarlos queda como refactor posterior.
- **Recarga en caliente del CSS:** solo se define su política de carga al arrancar; editar `style.css` en caliente no se refleja hasta reiniciar.
