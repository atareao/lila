# AGENTS.md — LiLa

Lanzador de aplicaciones para Wayland: overlay `gtk4-layer-shell` (GTK4) escrito en Rust (edition 2024). Crate binario único, sin workspace ni submódulos.

## Requisitos de build

- `gtk4-layer-shell` debe estar instalado como librería del sistema (no es solo una crate).
  - Arch: `sudo pacman -S gtk4-layer-shell`
- Sin esta dependencia el build falla en enlace, aunque `Cargo.toml` parezca completo.
- Ejecutar requiere un compositor Wayland que soporte layer-shell; no arranca en X11/headless.

## Comandos

```bash
cargo build
cargo test                      # tests inline, no necesitan display
cargo test filter_              # un test concreto (filtra por nombre)
cargo clippy -- -D warnings     # CI-asumido: cero warnings
cargo fmt
```

No hay CI, task runner ni `opencode.json`. `Cargo.lock` está en Git (app binaria, se versiona).

## Arquitectura (lo no obvio)

- `src/main.rs` — UI GTK **imperativa**. `activate()` construye ventana/entry/listbox, filtra al teclear (`connect_changed`) y lanza con Enter. La app es **singleton**: re-invocarla re-presenta la ventana existente.
- `src/models/` — `Config` (YAML), `Css`, `Extension`, `Item`.
- `src/finders/` — enum `Finder` + trait `Find` (`init`/`update`/`search`). El despacho por variante está en `finders/finder.rs`.
- `src/actions/mod.rs` — lanzar apps vía `gio::DesktopAppInfo::from_filename`.
- `src/utils/constants.rs` — IDs de app/nombres GTK (`APP_ID = es.atareao.lila`, etc.).

## Ficheros de config en runtime

- Se leen/crean en el dir XDG del proyecto: `~/.config/lila/config.yml` y `~/.config/lila/style.css`.
- Si no existen, se escriben desde `Config::default()` / `Css::default()`. Para cambiar defaults, edita esos `default()`, **no** los ficheros generados.
- `Finder` e `Item` derivan `Serialize`/`Deserialize`: **la forma del enum es el formato en disco**. Renombrar/añadir variantes rompe configs existentes.

## Añadir un finder (secuencia obligatoria)

1. Nuevo módulo en `src/finders/` implementando `Find`.
2. Exportarlo en `src/finders/mod.rs`.
3. Añadir variante al enum `Finder` y sus brazos en `init`/`update`/`search` de `finders/finder.rs`.

El filtrado fuzzy (`filter_items`, crate `fuzzy-matcher`/skim) es una función libre exportada por `finders`.

## Convenciones

- Aliases de import: `gtk4 → gtk`, `gdk4 → gdk`; crate `applications` para `App`.
- Tests inline bajo `#[cfg(test)]`, nombres `snake_case` descriptivos.
- Commits: gitmoji + conventional commits. Git Flow: nunca commit directo a `main`/`development`; todo por PR desde `feature/*` (ver skill `gitflow`).
