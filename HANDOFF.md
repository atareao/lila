# Handoff — lila — 2026-11-15 — Release v0.2.0 publicado; retomar por deuda técnica (#16)

## Estado
- **Milestone 1 `core-lanzamiento`**: COMPLETADO y en `main` (PR #13). Plan `plans/PLAN-001-ARCHIVED.md`.
- **Follow-up 5.3 (atajo global D-Bus)**: COMPLETADO (PR #15). Acciones `toggle`/`present`, docs `docs/atajo-global-dbus.md`, spec `launcher/invocation`.
- **Tooling de release/CI montado y operativo**:
  - `cliff.toml` (git-cliff) y `.vampus.yml` (vampus 0.4.6, `current_version: 0.2.0`).
  - `.github/workflows/`: `ci.yml` (fmt + clippy -D warnings + test en **contenedor `archlinux:base-devel`** con `gtk4` + `gtk4-layer-shell`, porque `ubuntu-latest` trae GTK 4.14 < 4.17 que exige `gdk4-sys`); `release-prepare.yml` (**manual**, `workflow_dispatch`: bump vampus + changelog git-cliff y **abre PR de release** — `main` protegida con enforce_admins); `release.yml` (en push a `main` crea el tag si falta **y resincroniza `development`**; en tag `v*` compila en contenedor Arch y publica GitHub Release).
  - `GIT_FLOW.md` y `CHANGELOG.md`.
  - Secreto **`GH_PAT`** configurado.
- **Releases publicadas**: `v0.1.0` (M1) y **`v0.2.0`** (release del tooling + 5.3). La `v0.2.0` se lanzó end-to-end: `Prepare Release` → PR #25 → merge a `main` → tag + GitHub Release con binario. CI verde.
- `main` == `development` (mismo commit), ambos en versión `0.2.0`. Sin ramas feature abiertas.

## Cómo lanzar una release
1. `gh workflow run "Prepare Release"` (o Actions → Prepare Release → Run). Abre un PR `release/vX.Y.Z` → `main`.
2. Fusionar el PR: `release.yml` crea el tag `vX.Y.Z`, publica la GitHub Release con el binario y resincroniza `development`.

## PENDIENTE — Deuda técnica (issue #16, SESIÓN NUEVA)
Decisiones: A + A2 + C + B.
- A: retirar `allow(dead_code)` inservible (`Item::exec()` `src/models/item.rs:49`, `Finder::update()` `src/finders/finder.rs:27`, `split_desktop_list()` `item.rs:102` si sobra...).
- A2 (feature): cablear `Item::keywords()`/`categories()` (`item.rs:72,89`) a la búsqueda.
- B (requiere spec): política de `Css::load()` (`src/models/css.rs`).
- C: cachear `load_all()` en `rank_items` entre pulsaciones.

## Siguiente acción
Crear `plans/PLAN-003.md` + change OpenSpec para #16 (skip_specs donde no cambie comportamiento; B sí) y TDD por punto; auditoría `@rust-reviewer`; PR a `development`.

## Punto de partida
```bash
cd /data/rust/lila && git checkout development && git pull && cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check
```
Esperado: 92 tests verde.

## Notas
- `CARGO_REGISTRY_TOKEN` no configurado: no se publica en crates.io (app GUI).
- El build en CI/Release usa contenedor Arch por la dependencia `gtk4-layer-shell` + GTK>=4.17.
