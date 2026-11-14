# Design

## Context

- LiLa es un crate binario Rust (edition 2024) con UI GTK4 imperativa y overlay `gtk4-layer-shell` en Wayland.
- `actions::launch` usa hoy `gio::DesktopAppInfo::from_filename` + `launch_uris(&[], None)`.
- El modelo `Item`/`Finder` deriva serde; su forma es el formato en disco de `config.yml`.
- La configuración y los estilos viven en `~/.config/lila/`.
- La BD solo se accede desde el hilo principal de GTK (single-thread); no se necesita pool de conexiones.

## Goals / Non-Goals

**Goals:**
- Ejecución correcta de aplicaciones según los metadatos de su `.desktop`.
- Ranking útil que priorice lo que el usuario realmente usa.
- Presentación rica (icono + subtítulo) y control total por teclado.
- Persistencia fiable del historial con esquema versionado.
- Base extensible para futuros milestones.

**Non-Goals:**
- Nuevas extensiones (calculadora, portapapeles, ficheros, web, sistema).
- Arquitectura de plugins de terceros.
- Soporte X11 u otras plataformas.
- Multi-monitor, animaciones y pantalla de ajustes gráfica.

## Decisions

- **SQLite con `rusqlite` (feature `bundled`)**: evita depender de una librería del sistema y encaja con el objetivo de un lanzador "de verdad"; además deja lista la base para FTS en extensiones futuras. Alternativa considerada: fichero JSON, descartado por escalabilidad y por decisión explícita del usuario.
- **BD en `~/.config/lila/history.db`, modo WAL** por durabilidad y lecturas concurrentes.
- **Esquema**: `launcher_history(item_id TEXT PRIMARY KEY, launches INTEGER NOT NULL DEFAULT 0, first_used INTEGER NOT NULL, last_used INTEGER NOT NULL)` con tiempos en epoch millis, más `schema_version(version INTEGER)` e índice por `last_used`. Migraciones idempotentes al abrir.
- **`item_id` = ruta canónica del fichero `.desktop`**: estable entre sesiones.
- **Trait de repositorio de historial** (`record`, `score`, `load_all`) para desacoplar el ranking de la persistencia.
- **Conexión única detrás de un `Mutex`**, reutilizada por la UI y `actions`.
- **Fórmula de ranking**: `final = w_fuzzy*fuzzy + w_freq*ln(1+launches) + w_recency*decay(last_used)`, con pesos en `Config`.
- **Filas con `ListBoxRow`** (`Image` + `Box` vertical con nombre y subtítulo) en lugar de `Label`.

## Risks / Trade-offs

- La feature `bundled` incrementa el tiempo de compilación la primera vez; se acepta por portabilidad.
- El decaimiento por recencia exige elegir una constante de tiempo; se parametriza.
- Usar la ruta del `.desktop` como `item_id` puede duplicar entradas si una app tiene varios `.desktop`; aceptable en este milestone.
