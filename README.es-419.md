<!-- source: README.md sha256:4102e79c1672 -->
<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="brand/wordmark-inverted.svg">
  <img src="brand/wordmark.svg" alt="Codewhale" width="320">
</picture>

**Un agente de programación para tu terminal que funciona con cualquier modelo.**

Dale una tarea a Codewhale y lee el repositorio, edita archivos y ejecuta
comandos hasta terminar la tarea o hasta que necesite una respuesta tuya.
Funciona con más de 40 proveedores integrados, con DeepSeek de forma
predeterminada, y con modelos que se ejecutan en tu propia máquina. `/receipts`
enumera los archivos que cambió, los comandos que ejecutó y las llamadas web y
MCP que hizo.

[![CI](https://github.com/codewhale-hq/CodeWhale/actions/workflows/ci.yml/badge.svg)](https://github.com/codewhale-hq/CodeWhale/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/codewhale-cli?label=crates.io)](https://crates.io/crates/codewhale-cli)
[![npm](https://img.shields.io/npm/v/codewhale?label=npm)](https://www.npmjs.com/package/codewhale)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
[![Discord](https://img.shields.io/badge/Discord-join-5865F2?logo=discord&logoColor=white)](https://discord.gg/37gfS3ksug)

[Sitio web](https://codewhale.net) · [Documentación](docs/README.md) · [Registro de cambios](CHANGELOG.md) · [Contribuir](CONTRIBUTING.md)

[English](README.md) · [简体中文](README.zh-CN.md) · [日本語](README.ja-JP.md) · [Tiếng Việt](README.vi.md) · [Bahasa Indonesia](README.id.md) · [한국어](README.ko-KR.md) · [Português](README.pt-BR.md) · [Русский](README.ru.md) · [Українська](README.uk.md) · [Français](README.fr.md) · [Deutsch](README.de.md) · [繁體中文](README.zh-TW.md) · [हिन्दी](README.hi.md) · [Türkçe](README.tr.md) · [Italiano](README.it.md) · [Polski](README.pl.md) · [العربية](README.ar.md) · [Català](README.ca.md)

<img src="web/public/codewhale-tui-0102-demo.png" alt="Una sesión de terminal de Codewhale" width="760">

<sub>Captura de una sesión de demostración local.</sub>

</div>

## Instalación

macOS y Linux:

```bash
curl -fsSL https://codewhale.net/install.sh | sh
```

El instalador coloca `codewhale` y su alias corto `codew`, con la suma de
verificación comprobada, en `~/.local/bin`. Si después tu shell responde
`command not found`, ejecuta la línea de PATH que muestra el instalador
([detalles](docs/INSTALL.md#put-it-on-your-path)). `codewhale update` lo
actualiza.

<details>
<summary><b>Windows, npm, Cargo y otras formas de instalarlo</b></summary>

```bash
winget install HunterBown.CodeWhale  # Windows x64 (or Scoop, or the installer from GitHub Releases)
npm install -g codewhale            # wraps the same release binaries
cargo install codewhale-cli --locked  # build from crates.io
```

Hay binarios precompilados para Linux, macOS y Windows en x64 y arm64. Nix,
Homebrew en Linux, Android/Termux (versión preliminar) y las descargas manuales
están en la [guía de instalación](docs/INSTALL.md), y los contenedores, en
[Docker](docs/DOCKER.md). Usa un solo método de instalación por máquina; de lo
contrario, terminas con dos copias en el `PATH` y gana la que aparezca primero.

</details>

## Inicio rápido

1. Haz commit de tu trabajo o guárdalo con stash, y luego ejecuta `codewhale`
   desde la raíz de un repositorio git. De forma predeterminada, ahí aplica las
   ediciones de archivos sin preguntar y te muestra el diff.
2. Ejecuta `/provider` (o presiona `F3`), elige un proveedor, pega su clave API
   y selecciona un modelo. DeepSeek es el proveedor predeterminado. Si no tienes
   clave, elige un entorno local. Si no hay ningún proveedor configurado y
   Ollama ya está sirviendo un modelo de chat, Codewhale lo usa automáticamente.
3. Escribe una tarea:

```text
Fix the failing tests and explain what changed.
```

Los comandos de solo lectura se ejecutan de inmediato. Para cualquier otro
comando de shell, Codewhale pregunta primero: `y` lo ejecuta una vez, `a`
permite comandos similares durante el resto de la conversación y `n` lo
rechaza. `/undo` revierte tu última solicitud y sus cambios en archivos. `/help`
muestra los comandos y las teclas.

Para scripts y CI, `codewhale exec` ejecuta una tarea sin la interfaz. Con
`--auto` edita archivos y ejecuta comandos de shell sin preguntar, así que dale
un contenedor o un checkout desechable:

```bash
codewhale exec --auto "fix the failing tests and explain what changed"
```

## Modos y permisos

Una instalación nueva arranca en el modo Work con el permiso Ask.

| | Se cambia con | Opciones |
| --- | --- | --- |
| Modo | `Tab` (cuando la entrada está vacía) o `/mode` | **Plan** lee y busca, pero no cambia nada. **Work** edita archivos y ejecuta comandos. **Operate** tiene el mismo acceso que Work, trata tu solicitud como una meta y sigue hasta que la meta queda verificada o necesita una respuesta tuya. |
| Permiso | `Shift+Tab` | **Ask** aplica las ediciones de archivos en tu repositorio sin preguntar y pregunta antes de los comandos de shell que no son de solo lectura. **Auto-Review** deja que unas reglas y un modelo revisor aprueben las llamadas rutinarias y rechaza las de alto riesgo; cambia a Ask para aprobar esas tú mismo. **Full Access** nunca pregunta y desactiva el sandbox de Codewhale; las reglas del repositorio y la política administrada siguen aplicándose. |

Consulta la [guía de modos y permisos](docs/MODES.md) para ver los detalles.

## Qué hace

### Modelos

Hay más de 40 proveedores integrados, entre ellos Anthropic, OpenAI, DeepSeek,
Google, Mistral, Moonshot, OpenRouter y xAI. Funciona cualquier endpoint
compatible con OpenAI, y también los modelos locales mediante Ollama, vLLM o
SGLang. `/model` cambia de modelo entre turnos. Consulta
[Proveedores](docs/PROVIDERS.md).

### Recibos y deshacer

`/receipts`, o `codewhale receipts --last` después de salir, enumera los
archivos modificados, los comandos ejecutados, las llamadas web y MCP, los
agentes iniciados, las aprobaciones y quién las dio, y los fallos. `/undo`
revierte tu última solicitud. `/restore` regresa todo el espacio de trabajo a
una instantánea anterior; como eso sobrescribe las ediciones posteriores,
requiere `/trust on` o Full Access. Consulta [Recibos](docs/RECEIPTS.md).

### Trabajos más largos

`/goal` define un objetivo que se mantiene entre turnos. Codewhale puede
repartir el trabajo entre [sub-agentes](docs/SUBAGENTS.md), ejecutar una
[flota](docs/FLEET.md) de agentes o seguir un
[flujo de trabajo](docs/WORKFLOW_AUTHORING.md) versionado en el repositorio.

### Si vienes de otro agente

Codewhale lee `AGENTS.md` por su cuenta. Los archivos de instrucciones escritos
para otras herramientas, como `CLAUDE.md`, `.cursorrules` o
`.github/copilot-instructions.md`, solo se cargan si los habilitas con
`project_instruction_imports = ["claude"]` (o `"cursor"`, `"cline"`,
`"windsurf"`, `"copilot"`, `"all"`) en tu configuración; consulta
[config.example.toml](config.example.toml). Codewhale te avisa cuando encuentra
uno que no cargó.

`/import-claude` prepara una importación de tu configuración y tus servidores
MCP de Claude Code para que la revises. `/plugin install` acepta plugins de
Claude Code que contengan skills, comandos, perfiles de agente y servidores MCP,
y rechaza los plugins que necesitan hooks o servidores LSP
([detalles](docs/CLAUDE_PLUGIN_COMPAT.md)).

### Extensiones

Conecta [servidores MCP](docs/MCP.md), agrega [skills](docs/SKILLS.md) y
[plugins](docs/PLUGINS.md), y ejecuta [hooks](docs/HOOKS.md) en eventos de
sesión y de herramientas. El plugin Computer Use incluido (experimental, solo
macOS) puede observar y operar otras aplicaciones. Permanece desactivado hasta
que revises su acceso y lo habilites
([guía](crates/tui/plugins/computer-use/README.md)).

## Formas de ejecutarlo

| Comando | Qué hace |
| --- | --- |
| `codewhale` | La aplicación interactiva de terminal |
| `codewhale exec "…"` | Una tarea sin la interfaz. Por sí solo responde sin usar herramientas; `--auto` le permite editar archivos y ejecutar comandos, y `--output-format stream-json` transmite los eventos como JSON |
| `codewhale web` | Una interfaz web local en `127.0.0.1` ([guía](docs/WEB.md)) |
| `codewhale review --pr N` | Una revisión orientativa de un pull request de GitHub. No publica nada a menos que agregues `--post`; la [GitHub Action](docs/GITHUB_ACTION.md) ejecuta la misma revisión en CI |
| `codewhale app-server --http` | La [Runtime API](docs/RUNTIME_API.md), una API HTTP local para hilos, eventos y aprobaciones |
| `codewhale serve --acp` | El Agent Client Protocol sobre stdio, para editores como Zed |

La aplicación de terminal, `exec`, `web` y la Runtime API comparten un mismo
motor ([arquitectura](docs/ARCHITECTURE.md)). Una
[extensión de VS Code](https://marketplace.visualstudio.com/items?itemName=HengQuWorld.brotherwhale-vscode)
mantenida por la comunidad se conecta a él desde la barra lateral
([código fuente](https://github.com/HengQuWorld/CodeWhale-VSCode)).
Hay una aplicación de escritorio en desarrollo; consulta la
[página del producto](https://codewhale.net/en/product).

## Costo y cuentas

Codewhale es gratuito y tiene licencia MIT. Con tu propia clave API le pagas
directamente al proveedor; los modelos locales no necesitan clave. La barra
inferior y `/cost` muestran cuánto ha costado una sesión cuando se conoce el
precio; un precio desconocido se muestra como desconocido, nunca como gratuito.

No necesitas una cuenta. Con una (`codewhale login`), puedes guardar las claves
de tus proveedores en tu
[cuenta de Codewhale](docs/CONFIGURATION.md#account-provider-keys); en otro
dispositivo, inicia sesión y elige Codewhale en `/provider` para usarlas.
Iniciar sesión no sube las claves que ya están en tu máquina.

## Seguridad y privacidad

En Ask y Auto-Review, los comandos se ejecutan en un sandbox del sistema
operativo cuando el sistema tiene uno: Seatbelt en macOS, y bubblewrap en Linux
cuando está instalado. Windows todavía no tiene sandbox del sistema operativo, y
Full Access desactiva el sandbox. `/preview-request` muestra una vista previa
de la siguiente solicitud, con los datos sensibles ocultos, sin enviarla.

La telemetría está activada de forma predeterminada. Envía conteos agregados
(versión y plataforma, duración y resultado de la sesión, conteos de funciones
y de errores) con un ID de instalación aleatorio que cambia cada 90 días, y
nunca conversaciones, código, prompts, nombres de archivos o de repositorios,
ni credenciales. `codewhale config set telemetry false` la desactiva. Al
iniciar, Codewhale también consulta GitHub para ver si hay una versión nueva;
define `CODEWHALE_NO_UPDATE_CHECK=1` para omitir esa consulta. Más información
en [orden de autorización](docs/AUTHORIZATION_ORDER.md),
[sandboxing](docs/SANDBOX.md) y [telemetría](docs/TELEMETRY.md).

## Documentación

| Empieza aquí | Referencia |
| --- | --- |
| [Instalación](docs/INSTALL.md) | [Configuración](docs/CONFIGURATION.md) |
| [Proveedores y modelos locales](docs/PROVIDERS.md) | [Arquitectura](docs/ARCHITECTURE.md) |
| [Modos y permisos](docs/MODES.md) | [Runtime API](docs/RUNTIME_API.md) |
| [Atajos de teclado](docs/KEYBINDINGS.md) | [Creación de plugins](docs/PLUGIN_AUTHORING.md) |
| [Revisión de PR en GitHub](docs/GITHUB_ACTION.md) | [Toda la documentación](docs/README.md) |

## Comunidad

Si falta un proveedor o un flujo de trabajo resulta incómodo,
[abre un issue](https://github.com/codewhale-hq/CodeWhale/issues/new/choose) o
[envía un pull request](CONTRIBUTING.md). Quienes contribuyen conservan el
crédito por el trabajo que se integra, y la
[estructura del repositorio](CONTRIBUTING.md#project-structure) muestra dónde
está cada cosa.

Conversa en [Discord](https://discord.gg/37gfS3ksug), o agrega al mantenedor,
Hunter, en WeChat (`hunterbown`) y pide unirte al grupo Whale Brothers.

## Historia y licencia

Codewhale comenzó como `deepseek-tui` y todavía lee la configuración y las
sesiones de ese proyecto. Se mantiene de forma independiente y no está afiliado
a DeepSeek ni a ningún otro proveedor de modelos. Gracias a
[todas las personas que han contribuido](docs/CONTRIBUTORS.md).

[![Codewhale contributors](docs/assets/contributors.png)](docs/CONTRIBUTORS.md#everyone)

[MIT](LICENSE). El código adaptado de otros proyectos de código abierto se
enumera en los [avisos de terceros](docs/THIRD_PARTY_NOTICES.md).
