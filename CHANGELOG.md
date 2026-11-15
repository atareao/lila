# Changelog
## [0.2.0] - 2026-11-15

### Bug Fixes

- *(ranking)* Respeta max_results en las rutas de fallback

### Documentation

- *(handoff)* Relevo del milestone core-lanzamiento
- *(handoff)* Relevo del milestone en formato canónico
- *(handoff)* Actualiza relevo tras 4.1 y 4.2
- *(handoff)* Registrar tarea 4.3 completada
- *(handoff)* Registrar tarea 4.4 completada
- *(handoff)* Registrar tarea 4.5 completada y cierre de Fase C
- *(handoff)* Precisar C1+C2 marcadas y verificación manual 4.4/4.5
- *(handoff)* Registrar tarea 1.3 (issue #6) y política de issues
- *(handoff)* Registrar tarea 1.4 (issue #7)
- *(handoff)* Registrar tarea 1.6 (issue #8) y cierre de Fase A
- *(openspec)* Escenarios de invocación y cierre de la Fase D en el plan
- *(handoff)* Registrar cierre de la Fase D (issue #9)
- *(handoff)* Cierre de sesión y punto de retomada en Fase E
- Documentar milestone core-lanzamiento
- *(openspec)* Archivar core-lanzamiento y cerrar el plan
- *(handoff)* Cierre del Milestone 1 (PR #11 en revisión)
- *(handoff)* Reflejar el merge del PR #11 en development
- *(openspec)* Propuesta del atajo global por D-Bus (follow-up 5.3)
- *(openspec)* Archivar atajo-global-dbus y cerrar PLAN-002
- *(handoff)* Relevo a sesión nueva para deuda técnica (#16)
- *(handoff)* Relevo con el tooling de release/CI montado

### Features

- Lanzar las aplicaciones del sistema con búsqueda difusa
- *(item)* Añade accesores de Item y subtítulo derivado del .desktop
- *(history)* Historial de uso en SQLite con esquema versionado
- *(ranking)* Ranking combinado difuso + frecuencia + recencia
- *(ui)* Filas de resultado con icono y subtítulo
- *(ui)* Mostrar la lista de resultados con consulta vacía
- *(ui)* Navegación de teclado completa con Page/Home/End y wrap-around
- *(ui)* Desplazar la lista para mantener visible la fila seleccionada
- *(actions)* Errores tipados en el lanzamiento (cierra 1.3)
- *(ui)* Mostrar el error de lanzamiento y mantener abierto el dashboard
- *(ui)* Instancia única y re-presentación sin error obsoleto (cierra 5.1–5.2)
- *(shortcut)* Atajo global por acciones D-Bus toggle/present

### Miscellaneous Tasks

- *(openspec)* Incorpora flujo SDD, plan del core y convenciones del proyecto
- Añadir cliff.toml para changelog con git-cliff
- Añadir .vampus.yml para versionado (vampus)
- Workflows de CI y release + GIT_FLOW.md y CHANGELOG
- Contenedor Arch (GTK>=4.17) y release por PR
- Instalar solo el paquete rust (incluye rustfmt/clippy)

### Refactor

- Deriva Default y limpia lints de clippy
- *(ranking)* Memoizar canonicalización con IdCache y limpiar callback vacío

### Styling

- *(ui)* Estilos por defecto de filas y selección

### Testing

- *(actions)* Cobertura de Exec, Terminal y .desktop inexistente (cierra 1.6)
