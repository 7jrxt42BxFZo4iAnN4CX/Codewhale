<!-- source: README.md sha256:2712306ab5f7 -->
<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="brand/wordmark-inverted.svg">
  <img src="brand/wordmark.svg" alt="Codewhale" width="320">
</picture>

**Tác nhân lập trình chạy trong terminal, hoạt động với mọi mô hình.**

Giao cho Codewhale một tác vụ, nó sẽ đọc kho mã, chỉnh sửa tệp và chạy lệnh cho
đến khi tác vụ hoàn thành hoặc cần bạn cung cấp thêm thông tin. Codewhale hoạt
động với hơn 40 nhà cung cấp tích hợp sẵn, mặc định là DeepSeek, và với các mô
hình chạy trên chính máy của bạn. `/receipts` liệt kê các tệp nó đã thay đổi,
các lệnh nó đã chạy, cùng các lệnh gọi web và MCP nó đã thực hiện.

[![CI](https://github.com/codewhale-hq/CodeWhale/actions/workflows/ci.yml/badge.svg)](https://github.com/codewhale-hq/CodeWhale/actions/workflows/ci.yml)
[![crates.io](https://img.shields.io/crates/v/codewhale-cli?label=crates.io)](https://crates.io/crates/codewhale-cli)
[![npm](https://img.shields.io/npm/v/codewhale?label=npm)](https://www.npmjs.com/package/codewhale)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
[![Discord](https://img.shields.io/badge/Discord-join-5865F2?logo=discord&logoColor=white)](https://discord.gg/37gfS3ksug)

[Trang web](https://codewhale.net) · [Tài liệu](docs/README.md) · [Nhật ký thay đổi](CHANGELOG.md) · [Đóng góp](CONTRIBUTING.md)

[English](README.md) · [简体中文](README.zh-CN.md) · [日本語](README.ja-JP.md) · [Bahasa Indonesia](README.id.md) · [한국어](README.ko-KR.md) · [Español](README.es-419.md) · [Português](README.pt-BR.md) · [Русский](README.ru.md) · [Українська](README.uk.md) · [Français](README.fr.md) · [Deutsch](README.de.md) · [繁體中文](README.zh-TW.md) · [हिन्दी](README.hi.md) · [Türkçe](README.tr.md) · [Italiano](README.it.md) · [Polski](README.pl.md) · [العربية](README.ar.md) · [Català](README.ca.md)

<img src="web/public/codewhale-tui-0102-demo.png" alt="Một phiên terminal của Codewhale" width="760">

<sub>Ảnh chụp từ một phiên demo cục bộ.</sub>

</div>

## Cài đặt

macOS và Linux:

```bash
curl -fsSL https://codewhale.net/install.sh | sh
```

Trình cài đặt đặt `codewhale` và bí danh ngắn `codew` của nó, đã được xác minh
checksum, vào `~/.local/bin`. Nếu sau đó shell báo `command not found`, hãy chạy
dòng PATH mà trình cài đặt in ra
([chi tiết](docs/INSTALL.md#put-it-on-your-path)). Dùng `codewhale update` để
nâng cấp.

<details>
<summary><b>Windows, npm, Cargo và các cách cài khác</b></summary>

```bash
winget install HunterBown.CodeWhale  # Windows x64 (or Scoop, or the installer from GitHub Releases)
npm install -g codewhale            # wraps the same release binaries
cargo install codewhale-cli --locked  # build from crates.io
```

Có sẵn tệp nhị phân dựng sẵn cho Linux, macOS và Windows trên x64 và arm64. Nix,
Homebrew trên Linux, Android/Termux (bản xem trước) và tải thủ công được trình
bày trong [hướng dẫn cài đặt](docs/INSTALL.md), còn container thì ở
[Docker](docs/DOCKER.md). Mỗi máy chỉ nên dùng một cách cài đặt; nếu không, hai
bản sẽ cùng nằm trên `PATH` và bản nào đứng trước sẽ được dùng.

</details>

## Bắt đầu nhanh

1. Commit hoặc stash công việc của bạn, rồi chạy `codewhale` từ thư mục gốc của
   một kho git. Theo mặc định, nó áp dụng các chỉnh sửa tệp ở đó mà không hỏi
   và hiển thị diff cho bạn.
2. Chạy `/provider` (hoặc nhấn `F3`), chọn một nhà cung cấp, dán khóa API của
   nhà cung cấp đó và chọn một mô hình. DeepSeek là nhà cung cấp mặc định. Nếu
   không có khóa, hãy chọn một runtime cục bộ. Nếu chưa cấu hình nhà cung cấp
   nào và Ollama đang phục vụ sẵn một mô hình trò chuyện, Codewhale sẽ tự động
   dùng mô hình đó.
3. Nhập một tác vụ:

```text
Fix the failing tests and explain what changed.
```

Các lệnh chỉ đọc chạy ngay. Với mọi lệnh shell khác, Codewhale hỏi trước: `y`
chạy lệnh một lần, `a` cho phép các lệnh tương tự trong phần còn lại của cuộc
trò chuyện, `n` từ chối. `/undo` hoàn tác yêu cầu gần nhất của bạn cùng các thay
đổi tệp của nó. `/help` liệt kê các lệnh và phím tắt.

Với script và CI, `codewhale exec` chạy một tác vụ mà không cần giao diện. Với
`--auto`, nó chỉnh sửa tệp và chạy lệnh shell mà không hỏi, vì vậy hãy chạy nó
trong một container hoặc một bản checkout dùng một lần:

```bash
codewhale exec --auto "fix the failing tests and explain what changed"
```

## Chế độ và quyền

Bản cài đặt mới khởi động ở chế độ Work với quyền Ask.

| | Chuyển bằng | Tùy chọn |
| --- | --- | --- |
| Chế độ | `Tab` (khi ô nhập trống) hoặc `/mode` | **Plan** đọc và tìm kiếm nhưng không thay đổi gì. **Work** chỉnh sửa tệp và chạy lệnh. **Operate** có cùng quyền truy cập như Work, coi yêu cầu của bạn là một mục tiêu và tiếp tục làm cho đến khi mục tiêu được xác minh hoặc cần bạn cung cấp thêm thông tin. |
| Quyền | `Shift+Tab` | **Ask** áp dụng các chỉnh sửa tệp trong kho mã của bạn mà không hỏi, và hỏi trước khi chạy các lệnh shell không phải chỉ đọc. **Auto-Review** để các quy tắc và một mô hình đánh giá phê duyệt các lệnh gọi thông thường, và từ chối các lệnh gọi rủi ro cao; hãy chuyển sang Ask để tự phê duyệt những lệnh gọi đó. **Full Access** không bao giờ hỏi và tắt sandbox của Codewhale; các quy tắc của kho mã và chính sách được quản lý vẫn được áp dụng. |

Xem chi tiết trong [hướng dẫn về chế độ và quyền](docs/MODES.md).

## Codewhale làm được gì

### Mô hình

Hơn 40 nhà cung cấp được tích hợp sẵn, gồm Anthropic, OpenAI, DeepSeek, Google,
Mistral, Moonshot, OpenRouter và xAI. Mọi endpoint tương thích OpenAI đều dùng
được, mô hình cục bộ qua Ollama, vLLM hoặc SGLang cũng vậy. `/model` đổi mô hình
giữa các lượt. Xem [Nhà cung cấp](docs/PROVIDERS.md).

### Biên nhận và hoàn tác

`/receipts`, hoặc `codewhale receipts --last` sau khi bạn thoát, liệt kê các tệp
đã thay đổi, các lệnh đã chạy, các lệnh gọi web và MCP, các tác nhân đã khởi
chạy, các phê duyệt và người đã phê duyệt, cùng các lần thất bại. `/undo` hoàn
tác yêu cầu gần nhất của bạn. `/restore` đưa toàn bộ không gian làm việc về một
snapshot trước đó; vì thao tác này ghi đè các chỉnh sửa sau đó, nó cần
`/trust on` hoặc Full Access. Xem [Biên nhận](docs/RECEIPTS.md).

### Công việc dài hơi

`/goal` đặt một mục tiêu được giữ qua các lượt. Codewhale có thể chia việc cho
các [tác nhân phụ](docs/SUBAGENTS.md), chạy một [nhóm tác nhân](docs/FLEET.md),
hoặc làm theo một [workflow](docs/WORKFLOW_AUTHORING.md) đã được commit vào kho
mã.

### Chuyển từ tác nhân khác

Codewhale tự đọc `AGENTS.md`. Các tệp chỉ dẫn viết cho công cụ khác, như
`CLAUDE.md`, `.cursorrules` hoặc `.github/copilot-instructions.md`, chỉ được nạp
khi bạn chủ động bật bằng `project_instruction_imports = ["claude"]` (hoặc
`"cursor"`, `"cline"`, `"windsurf"`, `"copilot"`, `"all"`) trong cấu hình; xem
[config.example.toml](config.example.toml). Codewhale sẽ cảnh báo khi tìm thấy
một tệp như vậy mà nó không nạp.

`/import-claude` chuẩn bị việc nhập cài đặt Claude Code và các máy chủ MCP của
bạn để bạn xem lại. `/plugin install` chấp nhận các plugin Claude Code chứa
skill, lệnh, hồ sơ tác nhân và máy chủ MCP, và từ chối các plugin cần hook hoặc
máy chủ LSP ([chi tiết](docs/CLAUDE_PLUGIN_COMPAT.md)).

### Tiện ích mở rộng

Kết nối [máy chủ MCP](docs/MCP.md), thêm [skill](docs/SKILLS.md) và
[plugin](docs/PLUGINS.md), và chạy [hook](docs/HOOKS.md) theo sự kiện phiên và
công cụ. Plugin Computer Use đi kèm (thử nghiệm, chỉ có trên macOS) có thể quan
sát và điều khiển các ứng dụng khác. Plugin này luôn tắt cho đến khi bạn xem lại
quyền truy cập của nó và bật nó lên
([hướng dẫn](crates/tui/plugins/computer-use/README.md)).

## Các cách chạy

| Lệnh | Chức năng |
| --- | --- |
| `codewhale` | Ứng dụng terminal tương tác |
| `codewhale exec "…"` | Một tác vụ không có giao diện. Khi không kèm cờ, nó trả lời mà không dùng công cụ; `--auto` cho phép nó chỉnh sửa tệp và chạy lệnh, còn `--output-format stream-json` phát trực tiếp các sự kiện dưới dạng JSON |
| `codewhale web` | Giao diện web cục bộ tại `127.0.0.1` ([hướng dẫn](docs/WEB.md)) |
| `codewhale review --pr N` | Đánh giá mang tính tham khảo cho một pull request trên GitHub. Nó không đăng gì trừ khi bạn thêm `--post`; [GitHub Action](docs/GITHUB_ACTION.md) chạy cùng kiểu đánh giá đó trong CI |
| `codewhale app-server --http` | [Runtime API](docs/RUNTIME_API.md), một API HTTP cục bộ cho thread, sự kiện và phê duyệt |
| `codewhale serve --acp` | Agent Client Protocol qua stdio, dành cho các trình soạn thảo như Zed |

Ứng dụng terminal, `exec`, `web` và Runtime API dùng chung một engine
([kiến trúc](docs/ARCHITECTURE.md)). Một
[tiện ích mở rộng VS Code](https://marketplace.visualstudio.com/items?itemName=HengQuWorld.brotherwhale-vscode)
do cộng đồng duy trì kết nối với engine này từ thanh bên
([mã nguồn](https://github.com/HengQuWorld/CodeWhale-VSCode)). Ứng dụng desktop
đang được phát triển; xem [trang sản phẩm](https://codewhale.net/en/product).

## Chi phí và tài khoản

Codewhale miễn phí và được cấp phép theo MIT. Khi dùng khóa API của riêng mình,
bạn trả tiền trực tiếp cho nhà cung cấp; mô hình cục bộ không cần khóa. Thanh
dưới cùng và `/cost` hiển thị chi phí của một phiên khi biết giá, còn giá chưa
biết được hiển thị là chưa biết, không bao giờ là miễn phí.

Bạn không cần tài khoản.

## An toàn và quyền riêng tư

Ở Ask và Auto-Review, lệnh chạy trong sandbox của hệ điều hành nếu hệ thống có:
Seatbelt trên macOS, và bubblewrap trên Linux khi đã được cài đặt. Windows hiện
chưa có sandbox của hệ điều hành, và Full Access tắt sandbox.
`/preview-request` hiển thị bản xem trước của yêu cầu tiếp theo, đã che thông
tin nhạy cảm, mà không gửi yêu cầu đó đi.

Telemetry được bật theo mặc định. Nó gửi các số liệu tổng hợp (phiên bản và nền
tảng, thời lượng và kết quả của phiên, số đếm tính năng và lỗi) kèm một ID cài
đặt ngẫu nhiên được thay mới sau mỗi 90 ngày, và không bao giờ gửi cuộc trò
chuyện, mã nguồn, prompt, tên tệp hay tên kho mã, hoặc thông tin xác thực.
`codewhale config set telemetry false` sẽ tắt telemetry. Khi khởi động,
Codewhale cũng kiểm tra GitHub xem có bản phát hành mới không; đặt
`CODEWHALE_NO_UPDATE_CHECK=1` để bỏ qua bước này. Xem thêm tại
[thứ tự cấp quyền](docs/AUTHORIZATION_ORDER.md), [sandbox](docs/SANDBOX.md) và
[telemetry](docs/TELEMETRY.md).

## Tài liệu

| Bắt đầu tại đây | Tham khảo |
| --- | --- |
| [Cài đặt](docs/INSTALL.md) | [Cấu hình](docs/CONFIGURATION.md) |
| [Nhà cung cấp và mô hình cục bộ](docs/PROVIDERS.md) | [Kiến trúc](docs/ARCHITECTURE.md) |
| [Chế độ và quyền](docs/MODES.md) | [Runtime API](docs/RUNTIME_API.md) |
| [Phím tắt](docs/KEYBINDINGS.md) | [Viết plugin](docs/PLUGIN_AUTHORING.md) |
| [Đánh giá PR trên GitHub](docs/GITHUB_ACTION.md) | [Toàn bộ tài liệu](docs/README.md) |

## Cộng đồng

Nếu thiếu một nhà cung cấp hoặc một quy trình làm việc còn bất tiện, hãy
[mở một issue](https://github.com/codewhale-hq/CodeWhale/issues/new/choose) hoặc
[gửi pull request](CONTRIBUTING.md). Người đóng góp được ghi nhận cho phần việc
được tích hợp, và [cấu trúc kho mã](CONTRIBUTING.md#project-structure) cho biết
mọi thứ nằm ở đâu.

Trò chuyện trên [Discord](https://discord.gg/37gfS3ksug), hoặc thêm người duy
trì dự án, Hunter, trên WeChat (`hunterbown`) và xin vào nhóm Whale Brothers.

## Lịch sử và giấy phép

Codewhale khởi đầu là `deepseek-tui` và vẫn đọc cấu hình cùng các phiên của dự
án đó. Dự án được duy trì độc lập và không liên kết với DeepSeek hay bất kỳ nhà
cung cấp mô hình nào khác. Cảm ơn
[mọi người đã đóng góp](docs/CONTRIBUTORS.md).

[![Codewhale contributors](docs/assets/contributors.png)](docs/CONTRIBUTORS.md#everyone)

[MIT](LICENSE). Mã được điều chỉnh từ các dự án mã nguồn mở khác được liệt kê
trong [thông báo bên thứ ba](docs/THIRD_PARTY_NOTICES.md).
