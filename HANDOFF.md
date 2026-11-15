# Handoff — lila — 2026-11-15 — Deuda técnica post-M1 (#16) implementada, auditada y con PR #29 abierto; falta merge + comprobación manual

## Estado
- **Rama de trabajo**: `feature/deuda-tecnica-post-m1`, 9 commits sobre `development`.
- **PR #29 abierto** contra `development` (`Closes #16`): `♻️♻️ refactor: cierra la deuda técnica post-M1 (#16)`.
- **Los cuatro puntos cerrados**:
  - **A** — limpieza de `allow(dead_code)`: eliminados `Item::exec()` y el envoltorio sin consumidor `Finder::update()`.
  - **A2** — `MetadataCache` + coincidencia por `Keywords`/`Categories` del `.desktop` (spec `launcher/result-ranking`, *Coincidencia más allá del nombre*).
  - **B** — `resolve_stylesheet` / `Css::load()` sin `Result`: nunca aborta el arranque ni sobrescribe el CSS del usuario (spec `launcher/results-ui`).
  - **C** — `CachedHistory` memoriza `load_all()` entre pulsaciones en vez de consultar SQLite en cada tecla.
- **Change OpenSpec archivado** en `openspec/changes/archive/2026-11-15-deuda-tecnica-post-m1/`; deltas mergeados en `openspec/specs/launcher/{result-ranking,results-ui}/spec.md`.
- **Puerta de calidad**: **115 tests** verde (baseline `development`: 92), `cargo clippy --all-targets -- -D warnings` limpio, `cargo fmt --check` limpio. Auditoría `@rust-reviewer` cerrada **sin bloqueantes** (2 hallazgos importantes y 2 menores corregidos en `0bb08ea`).
- **Documentación**: `docs/deuda-tecnica-post-m1.md`. **Plan**: `plans/PLAN-003.md`.

## Pendiente inmediato
1. **Comprobación manual en Wayland** (tarea 5.2 del change archivado): filtrado en vivo, coincidencia por palabra clave, coincidencia por categoría, estilo por defecto en el primer arranque y `pgrep -c lila` == 1.
2. **Fusionar PR #29** a `development`.
3. **Renombrar** `plans/PLAN-003.md` → `plans/PLAN-003-ARCHIVED.md`.
4. Comprobar que el **issue #16 se cierra solo** (por `Closes #16`).

## Siguiente acción
`gh pr checks 29`; si CI está verde y la comprobación manual en Wayland pasa, fusionar a `development`. Después, el renombrado del plan.

## Punto de partida
```bash
cd /data/rust/lila && git checkout feature/deuda-tecnica-post-m1 && cargo test && cargo clippy --all-targets -- -D warnings && cargo fmt --check
```
Esperado: 115 tests verde, clippy y fmt limpios.

## Notas
- **Releases publicadas**: `v0.1.0` (M1) y **`v0.2.0`**. `main` == `development` en versión `0.2.0`.
- **Cómo lanzar una release**: `gh workflow run "Prepare Release"` (o Actions → Prepare Release → Run); abre un PR `release/vX.Y.Z` → `main`. Al fusionarlo, `release.yml` crea el tag, publica la GitHub Release con el binario y resincroniza `development`.
- `CARGO_REGISTRY_TOKEN` no configurado: no se publica en crates.io (app GUI).
- El build en CI/Release usa **contenedor Arch** por la dependencia `gtk4-layer-shell` + GTK >= 4.17 (`ubuntu-latest` trae GTK 4.14).
- **Follow-ups abiertos** de la auditoría (detalle en `docs/deuda-tecnica-post-m1.md`):
  - Unificar `IdCache` con `record_launch`: `record_launch` usa `item.id()` sin `IdCache`, a diferencia del ranking; la clave de escritura y la de lectura no deben divergir.
  - Consultas de 1 carácter con `Categories`: casan con casi todo (sancionado por la spec); candidato a longitud mínima de consulta.
  - `url()`/`@import` relativos del CSS del usuario ya no se resuelven respecto al fichero (`load_from_string` en vez de `load_from_path`).
  - Los 3 `#[allow(dead_code)]` de `usage()`/`score()`/`usage_score()` se mantienen: son contrato del trait `HistoryRepository` y solo los ejercitan los tests.
