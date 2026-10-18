<!-- source: README.md sha256:2712306ab5f7 -->
<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="brand/wordmark-inverted.svg">
  <img src="brand/wordmark.svg" alt="Codewhale" width="320">
</picture>

**Un agent de code pour votre terminal qui fonctionne avec n’importe quel modèle.**

Donnez une tâche à Codewhale : il lit le dépôt, modifie des fichiers et exécute
des commandes jusqu’à ce que la tâche soit terminée ou qu’il ait besoin d’une
réponse de votre part. Il fonctionne avec plus de 40 fournisseurs intégrés,
DeepSeek par défaut, ainsi qu’avec des modèles qui tournent sur votre propre
machine. `/receipts` liste les fichiers qu’il a modifiés, les commandes qu’il a
exécutées et les appels web et MCP qu’il a effectués.

[![CI](https://github.com/codewhale-hq/CodeWhale/actions/workflows/ci.yml/badge.svg)](https://github.com/codewhale-hq/CodeWhale/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/codewhale-cli?label=crates.io)](https://crates.io/crates/codewhale-cli)
[![npm](https://img.shields.io/npm/v/codewhale?label=npm)](https://www.npmjs.com/package/codewhale)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
[![Discord](https://img.shields.io/badge/Discord-join-5865F2?logo=discord&logoColor=white)](https://discord.gg/37gfS3ksug)

[Site web](https://codewhale.net) · [Documentation](docs/README.md) · [Journal des modifications](CHANGELOG.md) · [Contribuer](CONTRIBUTING.md)

[English](README.md) · [简体中文](README.zh-CN.md) · [日本語](README.ja-JP.md) · [Tiếng Việt](README.vi.md) · [Bahasa Indonesia](README.id.md) · [한국어](README.ko-KR.md) · [Español](README.es-419.md) · [Português](README.pt-BR.md) · [Русский](README.ru.md) · [Українська](README.uk.md) · [Deutsch](README.de.md) · [繁體中文](README.zh-TW.md) · [हिन्दी](README.hi.md) · [Türkçe](README.tr.md) · [Italiano](README.it.md) · [Polski](README.pl.md) · [العربية](README.ar.md) · [Català](README.ca.md)

<img src="web/public/codewhale-tui-0102-demo.png" alt="Une session de terminal Codewhale" width="760">

<sub>Capture d’une session de démonstration locale.</sub>

</div>

## Installation

macOS et Linux :

```bash
curl -fsSL https://codewhale.net/install.sh | sh
```

L’installeur place `codewhale` et son alias court `codew` dans `~/.local/bin`,
après vérification de leur somme de contrôle. Si votre shell répond ensuite
`command not found`, exécutez la ligne PATH qu’affiche l’installeur
([détails](docs/INSTALL.md#put-it-on-your-path)). `codewhale update` le met à
jour.

<details>
<summary><b>Windows, npm, Cargo et autres méthodes d’installation</b></summary>

```bash
winget install HunterBown.CodeWhale  # Windows x64 (or Scoop, or the installer from GitHub Releases)
npm install -g codewhale            # wraps the same release binaries
cargo install codewhale-cli --locked  # build from crates.io
```

Des binaires précompilés sont fournis pour Linux, macOS et Windows, en x64 et
arm64. Nix, Homebrew sous Linux, Android/Termux (préversion) et les
téléchargements manuels sont décrits dans le
[guide d’installation](docs/INSTALL.md), et les conteneurs dans
[Docker](docs/DOCKER.md). Tenez-vous-en à une seule méthode d’installation par
machine ; sinon, deux copies se retrouvent dans le `PATH` et c’est la première
trouvée qui l’emporte.

</details>

## Démarrage rapide

1. Commitez ou mettez de côté (stash) votre travail, puis lancez `codewhale` à
   la racine d’un dépôt git. Par défaut, il y applique les modifications de
   fichiers sans demander et vous montre le diff.
2. Lancez `/provider` (ou appuyez sur `F3`), choisissez un fournisseur, collez
   sa clé API et choisissez un modèle. DeepSeek est le fournisseur par défaut.
   Sans clé, choisissez plutôt un runtime local. Si aucun fournisseur n’est
   configuré et qu’Ollama sert déjà un modèle de chat, Codewhale l’utilise
   automatiquement.
3. Saisissez une tâche :

```text
Fix the failing tests and explain what changed.
```

Les commandes en lecture seule s’exécutent immédiatement. Pour toute autre
commande shell, Codewhale demande d’abord : `y` l’exécute une fois, `a` autorise
les commandes similaires pour le reste de la conversation, `n` refuse. `/undo`
annule votre dernière demande et ses modifications de fichiers. `/help` liste
les commandes et les touches.

Pour les scripts et la CI, `codewhale exec` exécute une tâche sans l’interface.
Avec `--auto`, il modifie des fichiers et exécute des commandes shell sans
demander ; lancez-le donc dans un conteneur ou une copie de travail jetable :

```bash
codewhale exec --auto "fix the failing tests and explain what changed"
```

## Modes et permissions

Une nouvelle installation démarre en mode Work avec la permission Ask.

| | Changer avec | Options |
| --- | --- | --- |
| Mode | `Tab` (quand la saisie est vide) ou `/mode` | **Plan** lit et effectue des recherches, mais ne modifie rien. **Work** modifie des fichiers et exécute des commandes. **Operate** a les mêmes accès que Work, traite votre demande comme un objectif et continue jusqu’à ce que l’objectif soit vérifié ou qu’il ait besoin d’une réponse de votre part. |
| Permission | `Shift+Tab` | **Ask** applique sans demander les modifications de fichiers dans votre dépôt et demande avant les commandes shell qui ne sont pas en lecture seule. **Auto-Review** laisse des règles et un modèle de revue approuver les appels courants et refuse ceux à haut risque ; passez en Ask pour les approuver vous-même. **Full Access** ne demande jamais et désactive le bac à sable de Codewhale ; les règles du dépôt et la politique gérée s’appliquent toujours. |

Le [guide des modes et des permissions](docs/MODES.md) donne les détails.

## Ce qu’il fait

### Modèles

Plus de 40 fournisseurs sont intégrés, dont Anthropic, OpenAI, DeepSeek,
Google, Mistral, Moonshot, OpenRouter et xAI. Tout point d’accès compatible
OpenAI fonctionne, tout comme les modèles locaux via Ollama, vLLM ou SGLang.
`/model` change de modèle entre deux tours. Voir
[Fournisseurs](docs/PROVIDERS.md).

### Reçus et annulation

`/receipts`, ou `codewhale receipts --last` après avoir quitté, liste les
fichiers modifiés, les commandes exécutées, les appels web et MCP, les agents
lancés, les approbations et qui les a données, ainsi que les échecs. `/undo`
annule votre dernière demande. `/restore` ramène tout l’espace de travail à un
instantané antérieur ; comme cela écrase les modifications ultérieures, il faut
`/trust on` ou Full Access. Voir [Reçus](docs/RECEIPTS.md).

### Travail de longue durée

`/goal` fixe un objectif qui se maintient d’un tour à l’autre. Codewhale peut
répartir le travail entre des [sous-agents](docs/SUBAGENTS.md), faire tourner
une [flotte](docs/FLEET.md) d’agents ou suivre un
[workflow](docs/WORKFLOW_AUTHORING.md) versionné dans le dépôt.

### Venir d’un autre agent

Codewhale lit `AGENTS.md` de lui-même. Les fichiers d’instructions écrits pour
d’autres outils, comme `CLAUDE.md`, `.cursorrules` ou
`.github/copilot-instructions.md`, ne sont chargés que si vous l’activez
explicitement avec `project_instruction_imports = ["claude"]` (ou `"cursor"`,
`"cline"`, `"windsurf"`, `"copilot"`, `"all"`) dans votre configuration ; voir
[config.example.toml](config.example.toml). Codewhale vous avertit lorsqu’il en
trouve un qu’il n’a pas chargé.

`/import-claude` prépare une importation de vos paramètres Claude Code et de vos
serveurs MCP, pour que vous la vérifiiez. `/plugin install` accepte les plugins
Claude Code contenant des skills, des commandes, des profils d’agent et des
serveurs MCP, et refuse ceux qui ont besoin de hooks ou de serveurs LSP
([détails](docs/CLAUDE_PLUGIN_COMPAT.md)).

### Extensions

Connectez des [serveurs MCP](docs/MCP.md), ajoutez des [skills](docs/SKILLS.md)
et des [plugins](docs/PLUGINS.md), et exécutez des [hooks](docs/HOOKS.md) sur
les événements de session et d’outil. Le plugin Computer Use fourni
(expérimental, macOS uniquement) peut observer et utiliser d’autres
applications. Il reste désactivé tant que vous n’avez pas examiné ses accès et
ne l’avez pas activé ([guide](crates/tui/plugins/computer-use/README.md)).

## Façons de l’exécuter

| Commande | Ce qu’elle fait |
| --- | --- |
| `codewhale` | L’application interactive dans le terminal |
| `codewhale exec "…"` | Une tâche sans l’interface. Sans option, il répond sans utiliser d’outils ; `--auto` lui permet de modifier des fichiers et d’exécuter des commandes, et `--output-format stream-json` diffuse les événements en JSON |
| `codewhale web` | Une interface web locale sur `127.0.0.1` ([guide](docs/WEB.md)) |
| `codewhale review --pr N` | Une revue consultative d’une pull request GitHub. Elle ne publie rien sauf si vous ajoutez `--post` ; la [GitHub Action](docs/GITHUB_ACTION.md) exécute la même revue en CI |
| `codewhale app-server --http` | La [Runtime API](docs/RUNTIME_API.md), une API HTTP locale pour les fils, les événements et les approbations |
| `codewhale serve --acp` | L’Agent Client Protocol sur stdio, pour des éditeurs comme Zed |

L’application de terminal, `exec`, `web` et la Runtime API partagent un seul
moteur ([architecture](docs/ARCHITECTURE.md)). Une
[extension VS Code](https://marketplace.visualstudio.com/items?itemName=HengQuWorld.brotherwhale-vscode)
maintenue par la communauté s’y connecte depuis la barre latérale
([source](https://github.com/HengQuWorld/CodeWhale-VSCode)). Une application de
bureau est en cours de développement ; voir la
[page produit](https://codewhale.net/en/product).

## Coût et comptes

Codewhale est gratuit et sous licence MIT. Avec votre propre clé API, vous payez
directement le fournisseur ; les modèles locaux ne nécessitent aucune clé. Le
pied de page et `/cost` indiquent ce qu’une session a coûté lorsque le prix est
connu ; un prix inconnu est affiché comme inconnu, jamais comme gratuit.

Aucun compte n’est nécessaire.

## Sécurité et confidentialité

En Ask et en Auto-Review, les commandes s’exécutent dans un bac à sable du
système d’exploitation lorsque le système en propose un : Seatbelt sur macOS, et
bubblewrap sous Linux lorsqu’il est installé. Windows n’a pas encore de bac à
sable du système, et Full Access désactive le bac à sable. `/preview-request`
affiche un aperçu expurgé de la prochaine requête sans l’envoyer.

La télémétrie est activée par défaut. Elle envoie des compteurs agrégés (version
et plateforme, durée et résultat des sessions, compteurs de fonctionnalités et
d’erreurs) avec un identifiant d’installation aléatoire renouvelé tous les
90 jours, et jamais de conversations, de code, de prompts, de noms de fichiers
ou de dépôts, ni d’informations d’identification.
`codewhale config set telemetry false` la désactive. Au démarrage, Codewhale
vérifie aussi sur GitHub si une nouvelle version est disponible ; définissez
`CODEWHALE_NO_UPDATE_CHECK=1` pour l’éviter. Pour en savoir plus :
[ordre d’autorisation](docs/AUTHORIZATION_ORDER.md),
[bac à sable](docs/SANDBOX.md) et [télémétrie](docs/TELEMETRY.md).

## Documentation

| Pour commencer | Référence |
| --- | --- |
| [Installation](docs/INSTALL.md) | [Configuration](docs/CONFIGURATION.md) |
| [Fournisseurs et modèles locaux](docs/PROVIDERS.md) | [Architecture](docs/ARCHITECTURE.md) |
| [Modes et permissions](docs/MODES.md) | [Runtime API](docs/RUNTIME_API.md) |
| [Raccourcis clavier](docs/KEYBINDINGS.md) | [Création de plugins](docs/PLUGIN_AUTHORING.md) |
| [Revue de PR GitHub](docs/GITHUB_ACTION.md) | [Toute la documentation](docs/README.md) |

## Communauté

S’il manque un fournisseur ou si un workflow est peu pratique,
[ouvrez une issue](https://github.com/codewhale-hq/CodeWhale/issues/new/choose) ou
[envoyez une pull request](CONTRIBUTING.md). Les personnes qui contribuent
restent créditées pour le travail intégré, et
l’[organisation du dépôt](CONTRIBUTING.md#project-structure) montre où se trouve
chaque chose.

Discutez sur [Discord](https://discord.gg/37gfS3ksug), ou ajoutez le mainteneur,
Hunter, sur WeChat (`hunterbown`) et demandez à rejoindre le groupe Whale
Brothers.

## Historique et licence

Codewhale a commencé sous le nom de `deepseek-tui` et lit toujours la
configuration et les sessions de ce projet. Il est maintenu de manière
indépendante et n’est affilié ni à DeepSeek ni à aucun autre fournisseur de
modèles. Merci à [toutes les personnes qui ont contribué](docs/CONTRIBUTORS.md).

[![Codewhale contributors](docs/assets/contributors.png)](docs/CONTRIBUTORS.md#everyone)

[MIT](LICENSE). Le code adapté d’autres projets open source est répertorié dans
les [mentions tierces](docs/THIRD_PARTY_NOTICES.md).
