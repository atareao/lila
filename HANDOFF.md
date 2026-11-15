# Handoff — lila — 2026-11-15 — Milestone 1 + follow-up 5.3 cerrados; retomar por deuda técnica (#16)

## Estado
- **Milestone 1 `core-lanzamiento`**: COMPLETADO y fusionado en `main` (`aa955bd`, PR #13). Plan archivado en `plans/PLAN-001-ARCHIVED.md`.
- **Follow-up 5.3 (atajo global D-Bus)**: COMPLETADO (PR #15 fusionado en `development`, `c9ff840`; issue #14 cerrado). Acciones `toggle`/`present` en `src/main.rs`, docs en `docs/atajo-global-dbus.md`, spec en `openspec/specs/launcher/invocation/`. Plan `plans/PLAN-002.md` (E1–E5).
- Rama actual: `development` (sincronizada). Sin ramas feature abiertas. Sin issues abiertos salvo #16.

## PENDIENTE — Deuda técnica (issue #16), en SESIÓN NUEVA por higiene
Decisiones ya tomadas: hacer A + A2 + C + B.
- **A**: retirar `allow(dead_code)` de código inservible (`Item::exec()` `src/models/item.rs:49`, `Finder::update()` `src/finders/finder.rs:27`, `split_desktop_list()` `item.rs:102` si queda sin uso, etc.).
- **A2 (feature)**: cablear `Item::keywords()`/`Item::categories()` (`item.rs:72,89`) a la búsqueda fuzzy.
- **B (cambio de comportamiento → requiere spec)**: política de `Css::load()` (`src/models/css.rs`): hoy no reescribe `style.css` existente. Decidir respetar/sobrescribir/versionar.
- **C**: cachear `load_all()` del historial entre pulsaciones en `rank_items` (como `IdCache`), sin cambiar orden/puntuaciones.
- Solo-tests a decidir: `HistoryRepository::usage()/score()` y `usage_score()` (`src/history/mod.rs:113,122,157`).

## Siguiente acción (retomar aquí)
Abrir en la sesión nueva: `plans/PLAN-003.md` + change OpenSpec para #16 (usar `skip_specs` en los puntos sin cambio de comportamiento; B necesita delta de spec). Luego TDD por punto, auditoría `@rust-reviewer`, PR a `development`.

## Punto de partida
```bash
cd /data/rust/lila && git checkout development && git pull && cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check
```
Estado esperado: 92 tests verde.

## Metodología
SDD + TDD; issue por tarea asignado a `atareao`; commits gitmoji + conventional en `feature/*`; PR a `development`; nunca commit directo a `main`/`development`; auditoría antes de commit.
