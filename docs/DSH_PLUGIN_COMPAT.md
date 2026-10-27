# DeepSeek Harness Plugin Compatibility

Codewhale runs reviewed DeepSeek Harness (DSH) plugins in its experimental
Native extension host ([EXTENSIONS.md](EXTENSIONS.md)). The target is DSH
**0.2.1-rc.2**, pinned in `crates/tui/extension-host/src/dsh/compatibility.json`.
The Rust Engine keeps turns, sessions, approvals, credentials and receipts; a
DSH service is offered only where it maps onto one of those existing owners.
Everything else is refused by name when the plugin activates, never faked.

This page is the equivalence checklist. "Verified" names the check that was
run against the commit that landed the behavior. It is not a release
qualification, and hosted CI, Linux and Windows runs are listed separately.

For the separate launcher that runs your own installed `dsh`, see
[DeepSeek Harness integration](INTEGRATIONS_DSH.md).

## Packages, review and trust

| DSH behavior | In Codewhale | Status |
| --- | --- | --- |
| Install a bundle package (`dsh.bundle.patch`, Cordis patch YAML) | `/plugin import dsh`, `POST /v1/apps/plugins/import/dsh/preview`, then install with the previewed content hash | Verified on macOS over the Runtime API |
| A new plugin does nothing until reviewed | installed disabled and untrusted; trust needs the review token; enable is separate | Verified on macOS over the Runtime API |
| Trust covers the bytes and the compatibility table | changing either invalidates earlier trust | Verified by focused Engine tests |
| Bytes changed after review | install refused with a 409 conflict, nothing installed | Verified by focused Engine tests (`17ebeb5f61`) |
| Update and rollback walkthrough | existing update/revoke/remove paths | Not verified |
| Desktop app preview → install → trust → enable | App plugin settings | Built; not walked through live |

## Headless services

| DSH service | In Codewhale | Status |
| --- | --- | --- |
| `tools.register` | Engine tool registry; input and output checked against the declared schema; every call goes through an approval card | Verified on macOS over the Runtime API |
| Plugin tools on the **first** turn | the first tool build waits up to 10 s for the plugin host to finish loading | Verified on macOS over the Runtime API (`25225ef051`); before the fix, `tool_search` could not find the tool on turn one |
| `commands.register` | thread-scoped Runtime commands with replay-safe operation keys; a command may submit one text prompt with plugin attribution | Verified (command runs and replays) |
| `sessions.flush(exec.agent.session)` | Engine captures the conversation so far and waits for the Runtime, terminal or `exec` store to confirm the save | Verified on macOS over the Runtime API, including reload after an Engine restart (`17ebeb5f61`); other `sessions` methods refused |
| `userQuestions.ask` | the Engine's own question flow; answers are typed (choice, your own text, or skipped) | Verified on macOS over the Runtime API (`fff5a476f9`); `wait` and timed asks refused |
| `prompt`, `storage`, `skills`, `avatars`, `mcp`, `shellHooks`, `logger`, `events`, `reflect`, `registry` | provided | Supported subset; full API inventory not qualified |
| `subagents.startActivation`, caller delivery, disposal | refused | Not implemented |
| `tools.get`, `tools.restrict` | refused | Not implemented |
| `agents`, `systemPrompt`, `llm`, `approval`, `credentials`, `fs`, `subprocess` | refused by design until each maps to Engine authority | Not implemented |
| Rich or media tool output | dropped | Not implemented |
| Direct `git` from a plugin | Native profile denies repository reads | Not implemented |

## Plugin UI (`dsh.client`)

Not supported. The DSH import receipt says so: the UI half of a package is not
converted and stays off until Codewhale has a renderer that can keep plugin UI
off the network. A macOS probe showed WebRTC traffic leaving a WebKit view
despite every available restriction (content rules, CSP, an opaque sandboxed
frame and a restrictive Permissions-Policy), so a process-level boundary is
required first.

## Containment

| Behavior | Status |
| --- | --- |
| Native tier runs only under an OS sandbox (Seatbelt, bubblewrap, LPAC) | Verified on macOS with the compiled host (`17ebeb5f61`) |
| Plugins in the Native tier are isolated from each other | No: they share one trusted host process |
| Detached descendants on macOS | Can outlive the host; they keep the sandbox |
| Windows, current source | Not verified |

## Published plugins

`@michengai/dsh-code-review` 1.0.3 injects `commands`, `subagents`, `agents`,
`tools`, `systemPrompt`, `sessions` and `userQuestions`. `subagents`, `agents`
and `systemPrompt` are still refused, so the unmodified plugin cannot activate
in Codewhale yet.
