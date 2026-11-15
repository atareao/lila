# Carga de la configuración en el primer arranque

## Qué es

Documenta cómo LiLa carga su fichero de configuración `config.yml` al arrancar y, sobre
todo, la política ante ausencia o errores del fichero. Corrige el bug por el que el
**primer arranque de una instalación nueva hacía `panic`** (issue #32).

## Ficheros implicados

- `src/models/config.rs` — `load_config_from(config_dir: Option<&Path>) -> Result<Config, ConfigError>`
  (función pura, testeable sin display), `Config::load()` (envoltorio XDG), `ConfigError`
  (`Unreadable`, `Invalid`), y los helpers `read_user_config`, `create_config_file`,
  `write_default_file`.
- `src/models/mod.rs` — re-export de `Config` y `ConfigError`.
- `src/main.rs` — `exit_config_error(&ConfigError) -> !`, `const EXIT_CONFIG_ERROR: i32 = 2;`.

## Cómo funciona

Al arrancar, `Config::load()` localiza el directorio de configuración XDG
(`~/.config/lila`) y delega en `load_config_from`, cuya política es:

| Caso | Resultado | Arranque |
|---|---|---|
| `config.yml` presente y parseable | se usa tal cual | continúa |
| `config.yml` ausente | se escribe el default y se usa | continúa |
| ausente + directorio no escribible | default en memoria + `warn!` | continúa |
| sin directorio XDG | default | continúa |
| presente pero ilegible / YAML inválido | `Err(ConfigError)` | error claro + salida con código 2 |

Detalles de robustez:

- La escritura del default usa `File::create_new` (`O_CREAT|O_EXCL`): **nunca** trunca ni
  sobrescribe un `config.yml` que ya exista (incluido un enlace simbólico, roto o no).
- Si la escritura del default falla a mitad (p. ej. disco lleno), el fichero parcial se
  **elimina** antes de propagar el error, para no dejar un YAML corrupto que rompería los
  arranques siguientes.
- Un enlace simbólico **roto** se reporta como `Unreadable` (no se sigue al escribir), con
  una pista en el mensaje.
- Ningún caso produce `panic`. Ante error de configuración, `main.rs` imprime el mensaje en
  `stderr`, lo registra con `error!` y sale con código 2 (`EXIT_CONFIG_ERROR`).

## Variables y configuración

- `XDG_CONFIG_HOME` — determina el directorio de configuración (por defecto `~/.config`);
  LiLa usa el subdirectorio `lila`.
- `config.yml` — formato YAML; campos: `width`, `height`, `left`, `right`, `top`, `bottom`,
  `extensions`, `fuzzy_weight`, `frequency_weight`, `recency_weight`, `max_results`,
  `history_enabled`. Los campos ausentes toman los valores por defecto.

## Cómo verificar

```bash
# Primer arranque limpio: no debe abortar y debe crear config.yml y style.css
XDG_CONFIG_HOME=$(mktemp -d) cargo run

# YAML inválido: debe salir con código 2 y no tocar el fichero
D=$(mktemp -d); mkdir -p "$D/lila"; printf 'width: [x\n' > "$D/lila/config.yml"
XDG_CONFIG_HOME="$D" cargo run; echo "exit=$?"
```

## Decisiones

- Política **B/C**: los fallos benignos (ausencia, directorio no escribible, sin XDG)
  degradan a los valores por defecto y continúan; los errores reales (fichero ilegible o
  YAML inválido) se reportan con un mensaje claro en lugar de silenciarse.
- Cambio de comportamiento intencionado: un enlace simbólico roto pasa de "escribir el
  default a través del enlace" a abortar con código 2.
- Se mantiene `Result` en `Config::load()` (no se ocultan los errores reales), a diferencia
  del camino del CSS.

## Follow-ups

- Un `config.yml` mínimo (p. ej. solo `width: 640`) parsea con los defaults *de tipo* de
  serde (`extensions: []`, `width/height = 0`), dejando el lanzador sin resultados y sin
  aviso. Candidato a `#[serde(default = "...")]` o a un `warn!` si `extensions` queda vacío.
- El envoltorio XDG (`Config::load()`) no tiene test unitario; se verifica a mano y con la
  función pura `load_config_from`.
