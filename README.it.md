<!-- source: README.md sha256:4102e79c1672 -->
<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="brand/wordmark-inverted.svg">
  <img src="brand/wordmark.svg" alt="Codewhale" width="320">
</picture>

**Un agente di programmazione per il terminale che funziona con qualsiasi modello.**

Dai un compito a Codewhale: legge il repository, modifica i file ed esegue
comandi finché il compito non è completato o finché non ha bisogno di un tuo
input. Funziona con più di 40 provider integrati, DeepSeek come predefinito, e
con modelli eseguiti sulla tua macchina. `/receipts` elenca i file che ha
modificato, i comandi che ha eseguito e le chiamate web e MCP che ha fatto.

[![CI](https://github.com/codewhale-hq/CodeWhale/actions/workflows/ci.yml/badge.svg)](https://github.com/codewhale-hq/CodeWhale/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/codewhale-cli?label=crates.io)](https://crates.io/crates/codewhale-cli)
[![npm](https://img.shields.io/npm/v/codewhale?label=npm)](https://www.npmjs.com/package/codewhale)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
[![Discord](https://img.shields.io/badge/Discord-join-5865F2?logo=discord&logoColor=white)](https://discord.gg/37gfS3ksug)

[Sito web](https://codewhale.net) · [Documentazione](docs/README.md) · [Changelog](CHANGELOG.md) · [Contribuire](CONTRIBUTING.md)

[English](README.md) · [简体中文](README.zh-CN.md) · [日本語](README.ja-JP.md) · [Tiếng Việt](README.vi.md) · [Bahasa Indonesia](README.id.md) · [한국어](README.ko-KR.md) · [Español](README.es-419.md) · [Português](README.pt-BR.md) · [Русский](README.ru.md) · [Українська](README.uk.md) · [Français](README.fr.md) · [Deutsch](README.de.md) · [繁體中文](README.zh-TW.md) · [हिन्दी](README.hi.md) · [Türkçe](README.tr.md) · [Polski](README.pl.md) · [العربية](README.ar.md) · [Català](README.ca.md)

<img src="web/public/codewhale-tui-0102-demo.png" alt="Una sessione di terminale di Codewhale" width="760">

<sub>Acquisita da una sessione demo locale.</sub>

</div>

## Installazione

macOS e Linux:

```bash
curl -fsSL https://codewhale.net/install.sh | sh
```

Il programma di installazione mette in `~/.local/bin` `codewhale` e il suo alias
breve `codew`, con checksum verificato. Se poi la shell risponde
`command not found`, esegui la riga PATH che l’installer stampa
([dettagli](docs/INSTALL.md#put-it-on-your-path)). `codewhale update` lo
aggiorna.

<details>
<summary><b>Windows, npm, Cargo e altri modi per installarlo</b></summary>

```bash
winget install HunterBown.CodeWhale  # Windows x64 (or Scoop, or the installer from GitHub Releases)
npm install -g codewhale            # wraps the same release binaries
cargo install codewhale-cli --locked  # build from crates.io
```

I binari precompilati coprono Linux, macOS e Windows su x64 e arm64. Nix,
Homebrew su Linux, Android/Termux (anteprima) e i download manuali sono descritti
nella [guida all’installazione](docs/INSTALL.md), i container in
[Docker](docs/DOCKER.md). Usa un solo metodo di installazione per macchina;
altrimenti nel `PATH` finiscono due copie e vince quella che compare per prima.

</details>

## Avvio rapido

1. Fai commit o stash del tuo lavoro, poi esegui `codewhale` dalla radice di un
   repository git. Per impostazione predefinita lì applica le modifiche ai file
   senza chiedere e ti mostra il diff.
2. Esegui `/provider` (o premi `F3`), scegli un provider, incolla la sua chiave
   API e scegli un modello. DeepSeek è il provider predefinito. Se non hai una
   chiave, scegli invece un runtime locale. Se non è configurato alcun provider
   e Ollama sta già servendo un modello di chat, Codewhale lo usa
   automaticamente.
3. Scrivi un compito:

```text
Fix the failing tests and explain what changed.
```

I comandi in sola lettura vengono eseguiti subito. Per qualsiasi altro comando
di shell Codewhale chiede prima: `y` lo esegue una volta, `a` consente i comandi
simili per il resto della conversazione, `n` lo rifiuta. `/undo` annulla la tua
ultima richiesta e le relative modifiche ai file. `/help` elenca comandi e
tasti.

Per script e CI, `codewhale exec` esegue un compito senza interfaccia. Con
`--auto` modifica i file ed esegue comandi di shell senza chiedere, quindi
dagli un container o un checkout usa e getta:

```bash
codewhale exec --auto "fix the failing tests and explain what changed"
```

## Modalità e permessi

Una nuova installazione parte in modalità Work con il permesso Ask.

| | Si cambia con | Opzioni |
| --- | --- | --- |
| Modalità | `Tab` (quando l’input è vuoto) o `/mode` | **Plan** legge e cerca, ma non modifica nulla. **Work** modifica file ed esegue comandi. **Operate** ha gli stessi accessi di Work, tratta la tua richiesta come un obiettivo e va avanti finché l’obiettivo non è verificato o finché non ha bisogno di un tuo input. |
| Permesso | `Shift+Tab` | **Ask** applica senza chiedere le modifiche ai file nel tuo repository e chiede prima dei comandi di shell che non sono in sola lettura. **Auto-Review** lascia che regole e un modello revisore approvino le chiamate di routine e rifiuta quelle ad alto rischio; passa ad Ask per approvarle tu stesso. **Full Access** non chiede mai e disattiva la sandbox di Codewhale; le regole del repository e le policy gestite restano comunque valide. |

I dettagli sono nella [guida a modalità e permessi](docs/MODES.md).

## Cosa fa

### Modelli

Sono integrati più di 40 provider, tra cui Anthropic, OpenAI, DeepSeek, Google,
Mistral, Moonshot, OpenRouter e xAI. Funziona qualsiasi endpoint compatibile con
OpenAI, e anche i modelli locali tramite Ollama, vLLM o SGLang. `/model` cambia
modello tra un turno e l’altro. Vedi [Provider](docs/PROVIDERS.md).

### Ricevute e annullamento

`/receipts`, oppure `codewhale receipts --last` dopo l’uscita, elenca i file
modificati, i comandi eseguiti, le chiamate web e MCP, gli agenti avviati, le
approvazioni e chi le ha date, e gli errori. `/undo` annulla la tua ultima
richiesta. `/restore` riporta l’intera area di lavoro a uno snapshot precedente;
poiché sovrascrive le modifiche successive, richiede `/trust on` o Full Access.
Vedi [Ricevute](docs/RECEIPTS.md).

### Lavori più lunghi

`/goal` imposta un obiettivo che resta valido da un turno all’altro. Codewhale
può suddividere il lavoro tra [sub-agenti](docs/SUBAGENTS.md), avviare una
[flotta](docs/FLEET.md) di agenti o seguire un
[workflow](docs/WORKFLOW_AUTHORING.md) versionato nel repository.

### Se arrivi da un altro agente

Codewhale legge `AGENTS.md` da solo. I file di istruzioni scritti per altri
strumenti, come `CLAUDE.md`, `.cursorrules` o
`.github/copilot-instructions.md`, vengono caricati solo se li abiliti
esplicitamente con `project_instruction_imports = ["claude"]` (oppure
`"cursor"`, `"cline"`, `"windsurf"`, `"copilot"`, `"all"`) nella tua
configurazione; vedi [config.example.toml](config.example.toml). Codewhale
avvisa quando ne trova uno che non ha caricato.

`/import-claude` prepara un’importazione delle tue impostazioni e dei tuoi
server MCP di Claude Code, che puoi poi rivedere. `/plugin install` accetta
plugin di Claude Code che contengono skill, comandi, profili di agente e server
MCP, e rifiuta i plugin che richiedono hook o server LSP
([dettagli](docs/CLAUDE_PLUGIN_COMPAT.md)).

### Estensioni

Collega [server MCP](docs/MCP.md), aggiungi [skill](docs/SKILLS.md) e
[plugin](docs/PLUGINS.md) ed esegui [hook](docs/HOOKS.md) sugli eventi di
sessione e degli strumenti. Il plugin Computer Use incluso (sperimentale, solo
macOS) può osservare e usare altre applicazioni. Resta disattivato finché non ne
controlli gli accessi e lo abiliti
([guida](crates/tui/plugins/computer-use/README.md)).

## Modi di eseguirlo

| Comando | Cosa fa |
| --- | --- |
| `codewhale` | L’app interattiva nel terminale |
| `codewhale exec "…"` | Un compito senza interfaccia. Da solo risponde senza usare strumenti; `--auto` gli consente di modificare file ed eseguire comandi, e `--output-format stream-json` trasmette gli eventi in streaming come JSON |
| `codewhale web` | Un’interfaccia web locale su `127.0.0.1` ([guida](docs/WEB.md)) |
| `codewhale review --pr N` | Una revisione consultiva di una pull request GitHub. Non pubblica nulla a meno che tu non aggiunga `--post`; la [GitHub Action](docs/GITHUB_ACTION.md) esegue la stessa revisione in CI |
| `codewhale app-server --http` | La [Runtime API](docs/RUNTIME_API.md), un’API HTTP locale per thread, eventi e approvazioni |
| `codewhale serve --acp` | L’Agent Client Protocol su stdio, per editor come Zed |

L’app nel terminale, `exec`, `web` e la Runtime API condividono un unico motore
([architettura](docs/ARCHITECTURE.md)). Un’[estensione VS Code](https://marketplace.visualstudio.com/items?itemName=HengQuWorld.brotherwhale-vscode)
mantenuta dalla comunità vi si collega dalla barra laterale ([sorgente](https://github.com/HengQuWorld/CodeWhale-VSCode)).
Un’app desktop è in sviluppo; vedi la
[pagina del prodotto](https://codewhale.net/en/product).

## Costi e account

Codewhale è gratuito e rilasciato con licenza MIT. Con la tua chiave API paghi
direttamente il provider; i modelli locali non richiedono una chiave. Il piè di
pagina e `/cost` mostrano quanto è costata una sessione quando il prezzo è
noto; un prezzo sconosciuto viene mostrato come sconosciuto, mai come gratuito.

Non ti serve un account. Con un account (`codewhale login`) puoi conservare le
chiavi dei provider nel tuo
[account Codewhale](docs/CONFIGURATION.md#account-provider-keys); su un altro
dispositivo, accedi e scegli Codewhale in `/provider` per usarle. L’accesso non
carica le chiavi già presenti sulla tua macchina.

## Sicurezza e privacy

In Ask e Auto-Review i comandi vengono eseguiti in una sandbox del sistema
operativo, dove il sistema ne ha una: Seatbelt su macOS e bubblewrap su Linux
quando è installato. Windows non ha ancora una sandbox del sistema operativo, e
Full Access disattiva la sandbox. `/preview-request` mostra un’anteprima della
richiesta successiva con i dati oscurati, senza inviarla.

La telemetria è attiva per impostazione predefinita. Invia conteggi aggregati
(versione e piattaforma, durata ed esito della sessione, conteggi di funzioni ed
errori) con un ID di installazione casuale che cambia ogni 90 giorni, e non
invia mai conversazioni, codice, prompt, nomi di file o di repository, né
credenziali. `codewhale config set telemetry false` la disattiva. All’avvio
Codewhale controlla anche su GitHub se c’è una nuova release; imposta
`CODEWHALE_NO_UPDATE_CHECK=1` per saltare questo controllo. Altri dettagli in
[ordine di autorizzazione](docs/AUTHORIZATION_ORDER.md),
[sandbox](docs/SANDBOX.md) e [telemetria](docs/TELEMETRY.md).

## Documentazione

| Per iniziare | Per approfondire |
| --- | --- |
| [Installazione](docs/INSTALL.md) | [Configurazione](docs/CONFIGURATION.md) |
| [Provider e modelli locali](docs/PROVIDERS.md) | [Architettura](docs/ARCHITECTURE.md) |
| [Modalità e permessi](docs/MODES.md) | [Runtime API](docs/RUNTIME_API.md) |
| [Scorciatoie da tastiera](docs/KEYBINDINGS.md) | [Creare plugin](docs/PLUGIN_AUTHORING.md) |
| [Revisione PR su GitHub](docs/GITHUB_ACTION.md) | [Tutta la documentazione](docs/README.md) |

## Comunità

Se manca un provider o un workflow è scomodo,
[apri una issue](https://github.com/codewhale-hq/CodeWhale/issues/new/choose) o
[invia una pull request](CONTRIBUTING.md). Chi contribuisce mantiene il
riconoscimento per il lavoro che viene integrato, e la
[struttura del repository](CONTRIBUTING.md#project-structure) mostra dove si
trova ogni cosa.

Scrivici su [Discord](https://discord.gg/37gfS3ksug), oppure aggiungi il
maintainer, Hunter, su WeChat (`hunterbown`) e chiedi di entrare nel gruppo
Whale Brothers.

## Storia e licenza

Codewhale è nato come `deepseek-tui` e legge ancora la configurazione e le
sessioni di quel progetto. È mantenuto in modo indipendente e non è affiliato a
DeepSeek né ad alcun altro fornitore di modelli. Grazie a
[tutti coloro che hanno contribuito](docs/CONTRIBUTORS.md).

[![Codewhale contributors](docs/assets/contributors.png)](docs/CONTRIBUTORS.md#everyone)

[MIT](LICENSE). Il codice adattato da altri progetti open source è elencato
negli [avvisi di terze parti](docs/THIRD_PARTY_NOTICES.md).
