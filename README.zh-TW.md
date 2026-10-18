<!-- source: README.md sha256:2712306ab5f7 -->
<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="brand/wordmark-inverted.svg">
  <img src="brand/wordmark.svg" alt="Codewhale" width="320">
</picture>

**在終端機中運作、可搭配任何模型的程式開發代理（coding agent）。**

交給 Codewhale 一項任務，它會讀取儲存庫、編輯檔案並執行指令，直到任務完成或需要你提供輸入為止。
它支援超過 40 個內建供應商（預設為 DeepSeek），也能使用在你自己機器上執行的模型。
`/receipts` 會列出它變更的檔案、執行的指令，以及它發出的網路與 MCP 呼叫。

[![CI](https://github.com/codewhale-hq/CodeWhale/actions/workflows/ci.yml/badge.svg)](https://github.com/codewhale-hq/CodeWhale/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/codewhale-cli?label=crates.io)](https://crates.io/crates/codewhale-cli)
[![npm](https://img.shields.io/npm/v/codewhale?label=npm)](https://www.npmjs.com/package/codewhale)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
[![Discord](https://img.shields.io/badge/Discord-join-5865F2?logo=discord&logoColor=white)](https://discord.gg/37gfS3ksug)

[官方網站](https://codewhale.net) · [文件](docs/README.md) · [變更記錄](CHANGELOG.md) · [參與貢獻](CONTRIBUTING.md)

[English](README.md) · [简体中文](README.zh-CN.md) · [日本語](README.ja-JP.md) · [Tiếng Việt](README.vi.md) · [Bahasa Indonesia](README.id.md) · [한국어](README.ko-KR.md) · [Español](README.es-419.md) · [Português](README.pt-BR.md) · [Русский](README.ru.md) · [Українська](README.uk.md) · [Français](README.fr.md) · [Deutsch](README.de.md) · [हिन्दी](README.hi.md) · [Türkçe](README.tr.md) · [Italiano](README.it.md) · [Polski](README.pl.md) · [العربية](README.ar.md) · [Català](README.ca.md)

<img src="web/public/codewhale-tui-0102-demo.png" alt="一次 Codewhale 終端機工作階段" width="760">

<sub>擷取自本機示範工作階段。</sub>

</div>

## 安裝

macOS 與 Linux：

```bash
curl -fsSL https://codewhale.net/install.sh | sh
```

安裝程式會把經過 checksum 驗證的 `codewhale` 及其簡短別名 `codew` 放到 `~/.local/bin`。
如果之後 shell 顯示 `command not found`，請執行安裝程式印出的那一行 PATH 指令
（[詳細說明](docs/INSTALL.md#put-it-on-your-path)）。執行 `codewhale update` 即可升級。

<details>
<summary><b>Windows、npm、Cargo 與其他安裝方式</b></summary>

```bash
winget install HunterBown.CodeWhale  # Windows x64 (or Scoop, or the installer from GitHub Releases)
npm install -g codewhale            # wraps the same release binaries
cargo install codewhale-cli --locked  # build from crates.io
```

預先建置的二進位檔涵蓋 x64 與 arm64 上的 Linux、macOS 與 Windows。Nix、Linux 上的
Homebrew、Android/Termux（預覽版）與手動下載請見[安裝指南](docs/INSTALL.md)，容器請見
[Docker](docs/DOCKER.md)。每台機器請只使用一種安裝方式；否則 `PATH` 上會出現兩份副本，
以排在前面的那份為準。

</details>

## 快速入門

1. 先提交（commit）或暫存（stash）你的工作，然後在 git 儲存庫的根目錄執行 `codewhale`。
   預設情況下，它在該儲存庫中會直接套用檔案編輯、不先詢問，並向你顯示差異（diff）。
2. 執行 `/provider`（或按 `F3`），選擇供應商、貼上它的 API 金鑰，再選擇模型。預設供應商是
   DeepSeek。如果沒有金鑰，請改選本機執行階段（local runtime）。若尚未設定任何供應商，而 Ollama 已在提供
   聊天模型，Codewhale 會自動使用它。
3. 輸入任務：

```text
Fix the failing tests and explain what changed.
```

唯讀指令會直接執行。其他任何 shell 指令，Codewhale 都會先詢問：`y` 執行這一次，
`a` 允許本次對話後續類似的指令，`n` 拒絕。`/undo` 會還原你上一個請求及其
檔案變更。`/help` 會列出指令與按鍵。

在腳本與 CI 中，`codewhale exec` 會在沒有介面的情況下執行一項任務。加上 `--auto` 後，
它會在不詢問的情況下編輯檔案並執行 shell 指令，因此請讓它在容器或用完即丟的工作副本中執行：

```bash
codewhale exec --auto "fix the failing tests and explain what changed"
```

## 模式與權限

全新安裝會以 Work 模式與「詢問」權限啟動。

| | 切換方式 | 選項 |
| --- | --- | --- |
| 模式 | `Tab`（輸入框為空時）或 `/mode` | **Plan** 會讀取與搜尋，但不做任何變更。**Work** 會編輯檔案並執行指令。**Operate** 擁有與 Work 相同的存取權限，會把你的請求當作目標，持續推進，直到目標通過驗證或需要你提供輸入為止。 |
| 權限 | `Shift+Tab` | **詢問**（Ask）會在你的儲存庫中直接套用檔案編輯、不先詢問，並在執行非唯讀的 shell 指令前先詢問。**自動審核**（Auto-Review）讓規則與審核模型核准例行呼叫，並拒絕高風險呼叫；若要自行核准這些呼叫，請切換到「詢問」。**完整存取**（Full Access）從不詢問，並關閉 Codewhale 的沙箱；儲存庫規則與受管理的政策仍然適用。 |

詳情請參閱[模式與權限指南](docs/MODES.md)。

## 功能

### 模型

內建超過 40 個供應商，包括 Anthropic、OpenAI、DeepSeek、Google、Mistral、Moonshot、
OpenRouter 與 xAI。任何相容 OpenAI 的端點都能使用，透過 Ollama、vLLM 或 SGLang 執行的
本機模型也可以。`/model` 可在回合之間切換模型。請參閱[供應商](docs/PROVIDERS.md)。

### 回條與復原

`/receipts`（或在結束後執行 `codewhale receipts --last`）會列出變更的檔案、執行的指令、
網路與 MCP 呼叫、啟動的代理、核准紀錄與核准人，以及失敗項目。`/undo` 會還原你上一個請求。
`/restore` 會把整個工作區回復到先前的快照；由於這會覆寫之後的編輯，因此需要 `/trust on`
或「完整存取」。請參閱[回條](docs/RECEIPTS.md)。

### 長時間工作

`/goal` 會設定一個跨回合延續的目標。Codewhale 可以把工作分給[子代理](docs/SUBAGENTS.md)、
執行[代理團隊（Fleet）](docs/FLEET.md)，或依照已提交到儲存庫的
[工作流程](docs/WORKFLOW_AUTHORING.md)進行。

### 從其他代理轉過來

Codewhale 會自行讀取 `AGENTS.md`。為其他工具撰寫的指示檔，例如 `CLAUDE.md`、`.cursorrules`
或 `.github/copilot-instructions.md`，只有在你於設定中以
`project_instruction_imports = ["claude"]`（或 `"cursor"`、`"cline"`、`"windsurf"`、
`"copilot"`、`"all"`）選擇啟用時才會載入；請參閱 [config.example.toml](config.example.toml)。
Codewhale 發現這類檔案卻沒有載入時，會發出警告。

`/import-claude` 會準備匯入你的 Claude Code 設定與 MCP 伺服器，供你審閱。`/plugin install`
接受包含技能、指令、代理設定檔與 MCP 伺服器的 Claude Code 外掛，並拒絕需要 Hook 或 LSP
伺服器的外掛（[詳細說明](docs/CLAUDE_PLUGIN_COMPAT.md)）。

### 擴充功能

連接 [MCP 伺服器](docs/MCP.md)、加入[技能](docs/SKILLS.md)與[外掛](docs/PLUGINS.md)，
並在工作階段與工具事件上執行 [Hook](docs/HOOKS.md)。隨附的 Computer Use 外掛（實驗性，
僅限 macOS）可以觀察並操作其他應用程式。在你檢視其存取範圍並啟用之前，它會保持關閉
（[指南](crates/tui/plugins/computer-use/README.md)）。

## 執行方式

| 指令 | 作用 |
| --- | --- |
| `codewhale` | 互動式終端機應用程式 |
| `codewhale exec "…"` | 在沒有介面的情況下執行一項任務。單獨使用時，它只回答、不使用工具；`--auto` 讓它編輯檔案並執行指令，`--output-format stream-json` 以 JSON 串流輸出事件 |
| `codewhale web` | 在 `127.0.0.1` 上的本機 Web UI（[指南](docs/WEB.md)） |
| `codewhale review --pr N` | 對 GitHub PR（Pull Request）進行僅供參考的審查。除非你加上 `--post`，否則不會張貼任何內容；[GitHub Action](docs/GITHUB_ACTION.md) 會在 CI 中執行相同的審查 |
| `codewhale app-server --http` | [Runtime API](docs/RUNTIME_API.md)，用於執行緒、事件與核准的本機 HTTP API |
| `codewhale serve --acp` | 透過 stdio 提供的 Agent Client Protocol，供 Zed 等編輯器使用 |

終端機應用程式、`exec`、`web` 與 Runtime API 共用同一個引擎（[架構](docs/ARCHITECTURE.md)）。
由社群維護的
[VS Code 擴充功能](https://marketplace.visualstudio.com/items?itemName=HengQuWorld.brotherwhale-vscode)
可從側邊欄連接到它（[原始碼](https://github.com/HengQuWorld/CodeWhale-VSCode)）。
桌面應用程式正在開發中；請參閱[產品頁面](https://codewhale.net/en/product)。

## 費用與帳號

Codewhale 免費，並採用 MIT 授權。使用你自己的 API 金鑰時，費用直接付給供應商；本機模型
不需要金鑰。價格已知時，底部狀態列與 `/cost` 會顯示工作階段目前的花費；價格未知時會顯示為
未知，絕不會顯示為免費。

你不需要帳號。

## 安全與隱私

在「詢問」與「自動審核」權限下，只要系統提供作業系統沙箱，指令就會在其中執行：macOS 上為
Seatbelt，Linux 上則在已安裝 bubblewrap 時使用它。Windows 目前還沒有作業系統沙箱，而
「完整存取」會關閉沙箱。`/preview-request` 會顯示下一個請求經過遮蔽的預覽，但不會送出。

遙測預設為開啟。它會傳送彙總計數（版本與平台、工作階段長度與結果、功能與錯誤次數），並附上
每 90 天輪替一次的隨機安裝 ID；絕不會傳送對話、程式碼、提示詞、檔案或儲存庫名稱，或憑證。
執行 `codewhale config set telemetry false` 即可關閉。Codewhale 啟動時也會向 GitHub 檢查
是否有新版本；設定 `CODEWHALE_NO_UPDATE_CHECK=1` 即可略過。更多資訊請參閱
[授權順序](docs/AUTHORIZATION_ORDER.md)、[沙箱](docs/SANDBOX.md)與[遙測](docs/TELEMETRY.md)。

## 文件

| 從這裡開始 | 參考資料 |
| --- | --- |
| [安裝](docs/INSTALL.md) | [設定](docs/CONFIGURATION.md) |
| [供應商與本機模型](docs/PROVIDERS.md) | [架構](docs/ARCHITECTURE.md) |
| [模式與權限](docs/MODES.md) | [Runtime API](docs/RUNTIME_API.md) |
| [鍵盤快速鍵](docs/KEYBINDINGS.md) | [外掛開發](docs/PLUGIN_AUTHORING.md) |
| [GitHub PR 審查](docs/GITHUB_ACTION.md) | [所有文件](docs/README.md) |

## 社群

如果缺少某個供應商，或某個工作流程不順手，
請[提出 issue](https://github.com/codewhale-hq/CodeWhale/issues/new/choose)或
[送出 PR](CONTRIBUTING.md)。被合併的成果會保留貢獻者的署名；
[儲存庫結構](CONTRIBUTING.md#project-structure)說明了各部分放在哪裡。

歡迎到 [Discord](https://discord.gg/37gfS3ksug) 交流，或在微信上加維護者 Hunter
（`hunterbown`）為好友，申請加入 Whale Brothers 群組。

## 歷史與授權

Codewhale 最初名為 `deepseek-tui`，至今仍會讀取該專案的設定與工作階段。它是獨立維護的專案，
與 DeepSeek 或任何其他模型供應商均無隸屬關係。感謝[每一位貢獻者](docs/CONTRIBUTORS.md)。

[![Codewhale contributors](docs/assets/contributors.png)](docs/CONTRIBUTORS.md#everyone)

[MIT](LICENSE)。改編自其他開源專案的程式碼列於[第三方聲明](docs/THIRD_PARTY_NOTICES.md)中。
