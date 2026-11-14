# Tasks

## 1. Modelo y ejecución

- [x] 1.1 Añadir accesores a `Item`: `id()`, `icon_path()`, `subtitle()`, `exec()`, `keywords()`/`categories()`.
- [x] 1.2 Implementar `id()` como ruta canónica del fichero `.desktop`.
- [ ] 1.3 Robustecer `actions::launch`: field codes, `Terminal=true`, `Path`, `TryExec` y activación DBus.
- [ ] 1.4 Hacer visible el error de lanzamiento al usuario y mantener el dashboard abierto si falla.
- [x] 1.5 Registrar el lanzamiento en el historial tras un éxito.
- [ ] 1.6 Tests: parsing de `Exec`, caso `Terminal`, `.desktop` inexistente.

## 2. Historial (SQLite)

- [x] 2.1 Añadir la crate `rusqlite` con feature `bundled`.
- [x] 2.2 Crear el módulo `src/history/` con apertura de BD, modo WAL y `schema_version`.
- [x] 2.3 Definir la tabla `launcher_history` y el índice por `last_used`.
- [x] 2.4 Implementar migraciones idempotentes.
- [x] 2.5 Implementar el trait de repositorio (`record`, `score`, `load_all`).
- [x] 2.6 Tests: agregación de frecuencia, decaimiento por recencia y aplicación nunca usada.

## 3. Ranking

- [x] 3.1 Implementar la puntuación combinada fuzzy + frecuencia + recencia.
- [x] 3.2 Leer los pesos desde `Config`.
- [x] 3.3 Aplicar el límite de resultados configurable.
- [x] 3.4 Tests: la frecuencia desempata, recencia con consulta vacía y fuzzy que adelanta.

## 4. Presentación y teclado

- [x] 4.1 Filas con `ListBoxRow` (icono + nombre + subtítulo).
- [x] 4.2 Estilos por defecto para filas y selección.
- [x] 4.3 Visibilidad de la lista con consulta vacía y sin resultados.
- [x] 4.4 Navegación con PageUp/PageDown/Home/End y wrap-around en las flechas.
- [x] 4.5 Desplazar la lista para mantener visible la fila seleccionada.

## 5. Invocación

- [ ] 5.1 Corregir el arranque: eliminar el early-return previo a `application.run()`.
- [ ] 5.2 Garantizar instancia única con re-presentación al invocar de nuevo.
- [ ] 5.3 (Opcional) Acción DBus para atajo global del compositor.

## 6. Configuración, documentación y verificación

- [x] 6.1 Ampliar `Config` con los campos nuevos (`#[serde(default)]`).
- [ ] 6.2 Escribir `docs/core-lanzamiento.md`.
- [ ] 6.3 Actualizar `README.md`.
- [ ] 6.4 `cargo fmt --check`, `cargo clippy -- -D warnings` y `cargo test` en verde.
