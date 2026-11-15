# Git Flow — LiLa

Convenciones de ramas, releases y automatización para el repositorio **LiLa** (lanzador
de aplicaciones para Wayland, GTK4 + `gtk4-layer-shell`). Se usa **Git Flow** con
*conventional commits* + **gitmoji**.

---

## 1. Ramas

| Rama | Base | Propósito |
|---|---|---|
| `main` | — | Producción. Solo recibe merges vía PR desde `development` (o `hotfix/*`). Cada push dispara el release automático. |
| `development` | `main` | Integración. Recibe PRs de `feature/*`. |
| `feature/<nombre>` | `development` | Trabajo de features, fixes y refactors. |
| `hotfix/<nombre>` | `main` | Correcciones urgentes sobre producción. |

**Reglas:**

- **Nunca** se hace commit directo a `main` ni a `development`: todo cambio entra por PR.
- Las features parten siempre de `development`; los hotfixes, siempre de `main`.
- Al cerrar un hotfix hay que **sincronizar `development`** con `main`.

---

## 2. Flujo diario

### 2.1 Feature

```bash
git checkout development
git pull origin development
git checkout -b feature/mi-feature

# ... commits con gitmoji ...
# ✨ feat(ui): añadir filtro por escritorio

git push -u origin feature/mi-feature
# Abrir PR: feature/mi-feature → development
```

### 2.2 Hotfix

```bash
git checkout main
git pull origin main
git checkout -b hotfix/arreglo-critico

# ... commits ...
# 🚑️ hotfix: corregir cierre inesperado al lanzar app

git push -u origin hotfix/arreglo-critico
# Abrir PR: hotfix/arreglo-critico → main
```

Tras el merge del hotfix en `main` (y su release automático), sincronizar `development`:

```bash
git checkout development
git pull origin development
git merge main
git push origin development
```

---

## 3. Releases

El flujo de release está automatizado con GitHub Actions:

1. **PR `development` → `main`** (merge normal, sin commits directos).
2. Al hacer **push a `main`**, se dispara `release-prepare.yml`, que:
   - Instala `git-cliff` y `vampus`.
   - **Determina el bump** (`--major` / `--minor` / `--patch`) a partir de los commits desde el último tag.
   - Ejecuta `vampus upgrade <tipo>` para subir `version` en `Cargo.toml` y en `.vampus.yml`.
   - Genera `CHANGELOG.md` con `git-cliff`.
   - Hace commit (`🔧 chore: release vX.Y.Z`), crea el **tag `vX.Y.Z`** y lo empuja.
   - **Sincroniza `development`** con `main` vía la API de GitHub.
3. El push del tag **`vX.Y.Z`** dispara `release.yml`, que compila y publica los binarios.

> El commit de release (`chore: release ...`) está excluido mediante la condición
> `if: "!startsWith(github.event.head_commit.message, 'chore: release')"` para evitar bucles.

---

## 4. Tabla de bump por conventional commit

| Tipo | Emoji | Bump | Ejemplo |
|---|---|---|---|
| `feat` | ✨ | minor | `✨ feat(ui): añadir atajos de teclado` |
| `fix` | 🐛 | patch | `🐛 fix(finder): corregir indexado de apps` |
| `hotfix` | 🚑️ | patch | `🚑️ hotfix: evitar panic al cerrar` |
| `docs` | 📝 | patch | `📝 docs: actualizar README` |
| `refactor` | ♻️ | patch | `♻️ refactor(config): simplificar parser YAML` |
| `perf` | ⚡ | patch | `⚡ perf(search): acelerar filtrado fuzzy` |
| `style` | 💄 | patch | `💄 style: formatear con cargo fmt` |
| `test` | ✅ | patch | `✅ test: añadir tests de filter_items` |
| `chore` | 🔧 | patch | `🔧 chore(ci): actualizar actions` |
| `ci` | 👷 | patch | `👷 ci: añadir workflow de release` |
| `revert` | ⏪️ | patch | `⏪️ revert: revertir cambio de layout` |
| `deps` | ⬆️ | patch | `⬆️ deps: subir gtk4 a 0.10` |
| **breaking** (`feat!` / `💥` / `BREAKING CHANGE`) | 💥 | major | `💥 feat!: rediseñar formato de config` |

El *breaking change* se detecta en `release-prepare.yml` buscando `BREAKING CHANGE` o `!:` en el
mensaje del commit.

---

## 5. Tabla de gitmoji de referencia

| Gitmoji | Código | Uso |
|---|---|---|
| ✨ | `:sparkles:` | Nueva feature |
| 🐛 | `:bug:` | Corrección de bug |
| 🚑️ | `:ambulance:` | Hotfix crítico |
| 📝 | `:memo:` | Documentación |
| ♻️ | `:recycle:` | Refactor |
| ⚡ | `:zap:` | Mejora de rendimiento |
| 💄 | `:lipstick:` | Estilo / formato |
| ✅ | `:white_check_mark:` | Tests |
| 🔧 | `:wrench:` | Configuración / chore |
| 👷 | `:construction_worker:` | CI |
| ⬆️ | `:arrow_up:` | Dependencias |
| ⏪️ | `:rewind:` | Revertir |
| 💥 | `:boom:` | Breaking change |
| 🚀 | `:rocket:` | Deploy / release |
| 🔒 | `:lock:` | Seguridad |

---

## 6. Workflows de CI

| Fichero | Disparador | Función |
|---|---|---|
| `.github/workflows/ci.yml` | PR a `main`/`development` · push a `development` | `cargo fmt --check`, `cargo clippy -D warnings`, `cargo test`. |
| `.github/workflows/release-prepare.yml` | push a `main` · manual | Bump con `vampus`, changelog con `git-cliff`, commit + tag `vX.Y.Z`, sync de `development`. |
| `.github/workflows/release.yml` | push de tags `v*` | Compila para `x86_64-unknown-linux-gnu` y `aarch64-unknown-linux-gnu`, empaqueta `.tar.gz` y crea el GitHub Release. |

### 6.1 Dependencia del sistema: `gtk4-layer-shell`

LiLa **enlaza** contra `gtk4-layer-shell`, que no es solo una crate: necesita la librería
del sistema instalada para poder compilar.

- **Local (Arch):** `sudo pacman -S gtk4-layer-shell`
- **CI (Linux):** el paquete puede no existir en los repos de Ubuntu, así que los workflows
  intentan instalarlo con `apt` y, **si falla, lo compilan desde fuente** como fallback:

  ```bash
  sudo apt-get install -y libgtk-4-dev pkg-config meson ninja-build libwayland-dev wayland-protocols
  if ! sudo apt-get install -y libgtk4-layer-shell-dev; then
    git clone --depth 1 https://github.com/wmww/gtk4-layer-shell.git /tmp/gtk4-layer-shell
    cd /tmp/gtk4-layer-shell && meson setup build && ninja -C build && sudo ninja -C build install
    sudo ldconfig
  fi
  ```

- **Ejecución:** requiere un compositor Wayland con soporte layer-shell; no arranca en X11/headless.

---

## 7. Secretos

| Secreto | Scope / permiso | Obligatorio | Uso |
|---|---|---|---|
| `GH_PAT` | Personal Access Token con `contents: write` | Sí | Permite a `release-prepare.yml` hacer push del commit/tag y actualizar la rama `development` (el `GITHUB_TOKEN` por defecto no basta para disparar/actualizar otros workflows). |
| `CARGO_REGISTRY_TOKEN` | Token de crates.io | No | Solo si se decide publicar el crate en crates.io (LiLa es un binario; actualmente no se usa). |

> `release.yml` usa el `GITHUB_TOKEN` implícito con `permissions: contents: write` para crear el
> release, por lo que no necesita `GH_PAT`.
