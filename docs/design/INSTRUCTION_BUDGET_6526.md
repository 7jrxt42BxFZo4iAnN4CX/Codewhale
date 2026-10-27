# Standing instructions: selection, budgets, and visible truncation

Plan for issue #6526 (and the comment at
https://github.com/codewhale-hq/Codewhale/issues/6526#issuecomment-6393297659).

- **Status:** v5 — implemented on this branch (see §10 for what shipped and
  where the plan was written against an older tree).
- **Branch:** `fix/instruction-budget-6526` (from `main` at `498dd5a87`, 2026-10-24).
- **Scope:** standing instruction files (`AGENTS.md`, `CLAUDE.md`, `instructions.md`,
  the top-level `instructions = [...]` array, `rules/`). Skills are a separate issue (#7073).
- **Orientation:** OpenCode's selection model (AGENTS.md wins, otherwise CLAUDE.md) with
  codewhale's own sources preserved; no content truncation while a source is within budget.

## 1. Problem

Today codewhale has four instruction sources, three ceilings, and two different trim
directions — none of them visible to the operator.

### 1.1 Global layer

`load_global_agents_context` tries six candidates in order, first existing wins, and falls
through to the next candidate on a read error (`crates/tui/src/project_context.rs:1000-1033`):

1. `~/.codewhale/AGENTS.md`
2. `~/.agents/AGENTS.md`
3. `~/.deepseek/AGENTS.md` (legacy)
4. `~/.codewhale/instructions.md`
5. `~/.agents/instructions.md`
6. `~/.deepseek/instructions.md`

`~/.claude/CLAUDE.md` is not read today.

### 1.2 Project layer

Per directory, root → workspace:

- `AGENTS.md` (canonical);
- `.agents/AGENTS.md` (always, as a cross-agent fragment, `:196-208`);
- `.codewhale/instructions.md`;
- `.deepseek/instructions.md` (legacy);
- `CLAUDE.md` and other foreign formats only when `project_instruction_imports` enables them
  (`crates/tui/src/config.rs:2346-2352`, empty by default);
- `WHALE.md` is ignored and reported as a migration diagnostic.

### 1.3 Rules

`.codewhale/rules/*.md` and `.claude/rules/*.md` (`project_context.rs:341`), plus foreign rules
directories when imports are enabled (`:254-265`). `MAX_RULES_FILES = 50` per directory
(`:390`) silently drops the rest with a `tracing::warn!` only (`:1144-1155`).

### 1.4 Configured extra files

The top-level `instructions = [...]` array (`crates/tui/src/config.rs:2327`, user config only;
the project overlay ignores it). Each entry is capped at 100 KiB and trimmed with
`[…truncated: N of M bytes omitted — …]` (`crates/tui/src/prompts.rs:113`, `:154-164`,
`:464-540`). This is not a `[context]` key: `ContextConfig` only has `project_pack`
(`config.rs:1675`) and deliberately ignores unknown fields (#6516).

### 1.5 Ceilings and trim directions

- Aggregate `MAX_PROJECT_INSTRUCTION_BYTES = 48 * 1024` (`project_context.rs:386`), shared by
  instructions and rules.
- Per-file `MAX_CONTEXT_SIZE = 100 * 1024` (`:368`): a larger file is skipped (`TooLarge`,
  `:1070-1075`); a whitespace-only file is skipped as `Empty` (`:1078-1082`).
- Instructions are trimmed with `keep_tail_within` (`:397-410`): keep the tail, drop the
  front (broadest scope first), marker `[…broader-scope project instructions dropped at the
  aggregate budget…]`.
- Rules are trimmed with `keep_head_within` (`:413-426`): keep the head, drop the tail (the
  last rule files), marker `[…rules block truncated at the aggregate budget…]` (`:456-470`).
- The global text is merged into the project text (`merge_global_and_project_instructions`,
  `:985-999`) before `enforce_project_instruction_budget` runs (`:552`, `:749`). Per-scope
  budgets require a pipeline change: keep the scopes separate until each is measured and
  trimmed, then merge.

### 1.6 Visibility

`ProjectContext.warnings` appears only in `/context report`
(`crates/tui/src/context_report.rs:560-571`) and the Runtime API source map. Nothing is shown
at session start; the model sees inline markers without sizes or file names.

### 1.7 Measured on a real setup (approximate; markers and labels excluded)

Global `~/.agents/AGENTS.md` = 71,522 B, project `AGENTS.md` = 37,571 B. Assembled 109,093 B;
the 48 KiB tail keeps the project and about 11.3 KiB (16%) of the global; about 58.5 KiB are
dropped from the top. A 26 KiB project leaves ~22.0 KiB (31%) of the global. A 120 KiB project
is skipped by the per-file ceiling and the global is still cut to 49,152 B.

### 1.8 Data model and cache today

`enforce_project_instruction_budget` sees one flattened `String` (`ctx.instructions`) plus
`ctx.rules_block`; the per-file segments (`ancestor_docs: Vec<(PathBuf, String)>`) are local to
the loader and gone by trim time. The project-context cache
(`crates/tui/src/project_context_cache.rs`, wrapper
`load_project_context_with_parents_cached_and_home`) is keyed on files, not on the context
window. Both facts constrain the design: segment-level trimming needs segments kept on
`ProjectContext`, and window-derived budgets must not be baked into a file-keyed cache.

## 2. What we compared

### Claude Code (`code.claude.com/docs/en/memory`)

- Load order, broadest → most specific: managed policy → `~/.claude/CLAUDE.md` → project
  (`./CLAUDE.md` or `./.claude/CLAUDE.md`, every ancestor) → `./CLAUDE.local.md`.
- All discovered files are concatenated, order root → cwd; **no size cap, no truncation**.
- `AGENTS.md` is read only when no `CLAUDE.md`/`.claude/CLAUDE.md`/`CLAUDE.local.md` exists in
  cwd or above; a setting can switch to both / CLAUDE-only / managed-only.
- Imports `@path` up to 4 hops; `.claude/rules/` and path-scoped rules.
- Only explicit limit: auto memory, first 200 lines or 25 KB.

### OpenCode (`opencode.ai/docs/rules`, source `anomalyco/opencode@dev`)

- Per category: local `AGENTS.md` beats `CLAUDE.md`; global
  `~/.config/opencode/AGENTS.md` beats `~/.claude/CLAUDE.md`.
- `instructions` in `opencode.json` adds paths, globs, and remote URLs (5 s timeout), combined
  with `AGENTS.md`.
- **No content truncation.** `config/plugin/instruction.ts` reads global + ancestor
  `AGENTS.md` + configured sources; `session/instructions.ts` injects each file whole as a
  synthetic message `Instructions from: <path>\n<content>`.
- Order: global → project (nearest → farthest) → configured.

### codewhale today

Concatenate global + project + rules, then trim to 48 KiB: instructions keep the tail, rules
keep the head, global is discarded first. The only one of the three that cuts content.

## 3. Decisions

### 3.1 Selection

- **Project, per directory:** `AGENTS.md` wins; if absent, `CLAUDE.md` is the default
  fallback (OpenCode parity), controlled by `claude_fallback = true`. `CLAUDE.md` is also read
  alongside `AGENTS.md` when `project_instruction_imports` includes `claude`.

  | AGENTS.md | CLAUDE.md | `claude` import | `claude_fallback` | read |
  | --- | --- | --- | --- | --- |
  | loaded | absent | any | any | AGENTS.md |
  | loaded | loaded | no | any | AGENTS.md |
  | loaded | loaded | yes | any | both |
  | absent/skipped | loaded | yes | any | CLAUDE.md (explicit import outranks the fallback flag) |
  | absent/skipped | loaded | no | true | CLAUDE.md |
  | absent/skipped | loaded | no | false | nothing (explicit opt-out) |
  | absent/skipped | absent/skipped | any | any | nothing |

  `CLAUDE.md` here means `CLAUDE.md` or `.claude/CLAUDE.md` in the directory;
  `CLAUDE.local.md` is Claude-specific and is not imported.

  **Skip counts as absent:** if the winning `AGENTS.md` is too large or empty, it is treated
  as absent and the next candidate (CLAUDE.md) is tried; the notice names both the skipped
  file and the fallback selected. The same rule applies to the global chain: a skipped
  candidate falls through to the next one. An **empty project scope after a symlink dedupe**
  is not "absent": the content is already loaded as global, so no fallback runs; the notice
  says `duplicate`.

- **Global:** the six existing candidates in order, then `~/.claude/CLAUDE.md` as the last
  resort when none of the six loads and `claude_fallback = true`. Foreign files never outrank
  codewhale's own.
- **All ancestor directories** are read, root → workspace.
- **Symlinks are followed** (current `open_read` behaviour); duplicates are detected by
  canonical path (inode only on Unix) and loaded once. A project file symlinked to the global
  file is counted as global; the project duplicate is reported, not counted twice.
- `.agents/AGENTS.md` is always a fragment; it is not deduped against a same-directory
  `AGENTS.md` unless the canonical path is identical.

### 3.2 Budgets

- Budgets are in **tokens**. Files and the per-file ceiling are in bytes; conversion uses the
  engine's token estimator on the assembled text, the same estimator the context report uses.
  No `bytes/4` shortcut (it underestimates Cyrillic by 30–50%).
- **Usable window** = `context_window − max_output_tokens − safety_reserve`
  (`safety_reserve` = 10% of the window, minimum 2,048 tokens). All percentages are of the
  usable window, not the raw window.
- Per-scope budgets, independent:
  `effective = min(absolute_tokens, percent × usable_window)`.
  Defaults: **100,000 tokens** and **25%** per scope. All values are positive; `0` is invalid
  (a finite budget is the point). `percent` is 1–100.
- **Scope assignment:** global scope = the six global candidates + `~/.claude/CLAUDE.md` + the
  configured `instructions = [...]` entries (user-level authority). Project scope = the
  per-directory chain + fragments + `CLAUDE.md` + rules. Configured entries are therefore
  global, not project; rules are project.
- **Static-prefix guard:** the static prefix = base system prompt + tool-use policy +
  environment block + instructions + skills index + tool schemas + memory. It must stay under
  `static_prefix_guard_percent` (default 80% of the usable window). The guard is the safety
  net for non-instruction static content; it is not a substitute for a total-instructions key.
- **No separate "total instructions" key.** A key equal to the sum of the two scope caps
  (50%) would be inert; a smaller one would contradict the per-scope decision. The guard
  covers the actual risk.
- Effective table (raw windows, before the output/safety reserve; the reserve only lowers the
  usable window):

  | window | usable (approx) | per scope | both scopes | guard 80% | residual for other static |
  | --- | --- | --- | --- | --- | --- |
  | 32k | ~26.8k | 6,700 | 13,400 | 21,400 | 8,000 |
  | 120k | ~105.8k | 26,450 | 52,900 | 84,600 | 31,700 |
  | 128k | ~113.1k | 28,275 | 56,550 | 90,500 | 33,950 |
  | 150k | ~132.8k | 33,200 | 66,400 | 106,200 | 39,800 |
  | 1M | ~897.9k | 100,000 (10%) | 200,000 (20%) | 718,300 | 518,300 |

  The owner's 1M expectation (10% per scope, 20% total) is preserved by the absolute cap.
  A 32k window is best-effort. Two thresholds are distinct: guard trimming begins when
  non-instruction static content exceeds the residual (~8,000 tokens at 32k); the guard is
  `unsatisfiable` only when non-instruction static content alone exceeds the guard
  (~21,400 tokens at 32k). In the first case instructions are trimmed by the overage; in the
  second they reach zero and the session proceeds (or refuses, under `policy = refuse`).

### 3.3 Per-file ceiling

- `file_max_bytes = 8 MiB` is a memory-safety limit, not truncation. The size is checked with
  `metadata()` before reading; a larger file is `skipped`, with a notice naming the file and
  the limit. There is no `0`/unlimited value: a hard ceiling must always exist.
- A whitespace-only file is `skipped (empty)`.
- `MAX_RULES_FILES = 50` stays as a safety cap, but its overflow is reported as
  `skipped (rules cap)` in the notice and `/context`; it is no longer silent.
- Files are read as a whole up to the ceiling. For files near 8 MiB, tokenization is the
  expensive step; the scope budget will trim after tokenization. A byte-side pre-cut is an
  allowed optimization, not a behaviour change.

### 3.4 Trim semantics

- `ProjectContext` keeps `instruction_segments: Vec<(PathBuf, String)>` (scope, path, text) in
  addition to the joined string. Per-scope trimming operates on segments; the joined string is
  rebuilt after trimming.
- Each scope is trimmed only against its own budget; the other scope is untouched.
- Within the project scope, drop whole chain segments from the broadest end first (root-most
  ancestors go first, the workspace file last). If a single file alone exceeds the remaining
  budget, keep its head, drop its tail with `keep_head_within`, and insert a marker with
  bytes/tokens. This reverses today's single-file direction (today `keep_tail_within` keeps
  the tail); the change is listed in §9. `keep_tail_within` is no longer used for instructions.
- Within the global scope there is normally one file; the same single-file rule applies.
- Rules are charged to the project scope after project instructions; they keep the current
  head-first trim (rules are a flat filename-sorted list, with no nearest-scope hierarchy to
  preserve), and the marker and notice carry the numbers.
- The trim amount is `over = content_tokens − budget_tokens`; trim `min(scope_tokens, over)`
  from the chosen scope, then re-measure once. There is no loop.
- The marker is included in the budget (as today: `max − marker_len`). The cut point is found
  by binary search on UTF-8 boundaries so the result is deterministic.
- If trimming leaves zero source bytes (marker excluded), report `skipped (budget)`, not
  `trimmed`. This is reachable when the budget is smaller than the marker or when the static
  guard trims a scope to zero.
- If the static guard trips, trim the largest instruction scope first, by `over`; re-render
  once, re-measure once. If both scopes reach zero and the guard is still exceeded, emit
  `static_guard_unsatisfied` and continue (policy `trim`) or stop (policy `refuse`). There is
  no retry loop and only one notice per scope per session.

### 3.5 Policy

- Default: `trim` + warning in the chat. `refuse` remains an opt-in value for operators who
  prefer a hard stop. `refuse` stops on **trim** losses (scope budget or static guard) and on
  the unsatisfiable guard; **skips** (file ceiling, empty file, rules cap) remain skips and are
  reported, because a single bad file must not make the project unopenable.
- The `warn` value is removed; "warn" is what `trim` does.
- Validation rejects unknown policy values, non-positive budgets, percents outside 1–100, a
  guard outside 1–100, and any config where `guard − (global_percent + project_percent) < 10`;
  the remaining 10 points are the minimum room for non-instruction static content.

### 3.6 Notice contract

| Trigger | Action | User | Model |
| --- | --- | --- | --- |
| file > `file_max_bytes` | skip | startup line | first-turn synthetic |
| empty file | skip | startup line | first-turn synthetic |
| rules cap 50 | skip files | startup line | first-turn synthetic |
| duplicate via symlink | skip duplicate | startup line | first-turn synthetic |
| scope > budget, `trim` | trim that scope | startup line + numbers | first-turn synthetic + marker |
| zero source bytes left | skip | "skipped", not "trimmed" | same |
| static guard, `trim` | trim largest scope by `over` | startup line | first-turn synthetic |
| static guard unsatisfiable | proceed (trim) / stop (refuse) | named line | first-turn synthetic |
| `refuse` policy on trim | stop | named error | session does not start |
| all within budgets | no notice | no | no |

Fields: `path`, `scope` (`global`/`project`), `source` (`agents`/`claude`/`instructions`/
`configured`/`rules`), `action` (`kept`/`trimmed`/`skipped`/`refused`), `bytes_kept`,
`bytes_dropped`, `tokens_kept` (estimated), `tokens_dropped`, `limit_kind` (`scope_budget`/
`static_guard`/`file_ceiling`/`empty_file`/`rules_cap`/`rules_budget`/`duplicate_path`), and
`fallback_selected` when a skipped file was replaced by a lower-precedence candidate.

Channels:

- **User:** a system history cell at session start (`app.add_message(HistoryCell::System {
  content })`), not the `startup_notice` status-line field (that one belongs to auto-resume and
  would be overwritten); stderr for headless/`exec` (text output). The notice is recomputed
  deterministically from files + config, so a resumed session states the same losses.
- **Model:** a first-turn synthetic runtime-provenance message (the LSP-diagnostics mechanism;
  see §10), which is part of the logged transcript.
- **`/context report`:** per-source action and budget, alongside the existing warnings at
  `context_report.rs:560`.

## 4. Config surface

Top-level `instructions = [...]` stays as is (array). New table **`[instruction_budget]`**
(not `[instructions]`, which would collide with the array; not `[context]`, which ignores
unknown keys and would hide typos):

```toml
[instruction_budget]
global_budget_tokens = 100000     # positive
project_budget_tokens = 100000    # positive
global_budget_percent = 25        # 1..=100
project_budget_percent = 25       # 1..=100
static_prefix_guard_percent = 80  # 1..=100
file_max_bytes = 8388608          # hard ceiling, positive
policy = "trim"                   # trim | refuse
claude_fallback = true            # AGENTS.md wins; otherwise CLAUDE.md
```

- `deny_unknown_fields` on this table so a typo inside it fails loud.
- The **project overlay ignores this table** (like `instructions`), so a cloned repository
  cannot shrink the user's global budget or force `refuse`. The overlay/base merge happens at
  `config.rs:10625-10628`; the new table follows the `instructions` rule there (user config
  only).
- Because the top-level config is tolerant of unknown tables (#6516), a misspelled table name
  (`[instruction_budjet]`) would be ignored; `codewhale doctor` inspects the raw TOML before
  deserialization and warns about unknown `instruction*` tables and keys.

## 5. Implementation plan

1. **New assembly order** in `crates/tui/src/project_context.rs`:
   `assemble_instruction_scopes(raw: &ProjectContext, configured: &[PathBuf], budgets:
   &InstructionBudgets)` reads configured entries fresh (they are user config and are not in
   the cache key), joins them to the global scope, applies per-scope trims, then merges with
   labels. Imports are already baked into the raw cache; budgets are passed separately. The
   new function owns the split; the configured entries move out of
   `prompts.rs::render_instructions_block`, so there is exactly one owner and one charge
   (global).
2. **Cache**: `project_context_cache` keeps **raw segments** keyed by the existing file
   candidate set (`project_context_cache_candidate_paths`, `project_context.rs:754-800`).
   `apply_instruction_budgets` runs after cache lookup, per request/window; configured
   `instructions` entries are read after cache lookup, so a change there is picked up without
   cache invalidation; `InstructionSource::Inline` stays outside the cache. A window change
   must not invalidate the content cache, and a content change must not reuse trimmed text.
3. **`crates/tui/src/project_context.rs`**
   - `ProjectContext` gains `instruction_segments: Vec<InstructionSegment>` (scope, path,
     text); the joined string is rebuilt after trimming.
   - Delete both `enforce_project_instruction_budget` call sites (`:552`, `:749`) and replace
     them with `apply_instruction_budgets`, called by the assembly after cache lookup.
   - `load_global_agents_context`: add `~/.claude/CLAUDE.md` as the last candidate; keep the
     six existing candidates and fall-through; report the selected fallback.
   - `load_dir_instructions` / `active_fragment_candidates`: implement the truth table from
     §3.1 (skip-as-absent, explicit import co-reads, symlink dedupe).
   - `apply_instruction_budgets`: per-scope budgets; segments dropped whole; `keep_head_within`
     for a single oversized file and the rules block; `keep_tail_within` no longer used for
     instructions.
   - `MAX_RULES_FILES` overflow → structured warning, not only `tracing::warn!`.
   - Module-level known-limitations note (repo rule) describing the estimator and trim order.
4. **`crates/tui/src/prompts.rs`**
   - `INSTRUCTIONS_FILE_MAX_BYTES` from `file_max_bytes`; split the relay-artifact cap from
     the instruction cap so `policy = refuse` does not change relay behaviour.
     `render_instructions_block` renders only the assembled instruction block (no configured
     entries).
5. **Static guard**: `prompts.rs` exposes `measure_static_prefix(&SystemPrompt, &[ToolSpec])`;
   `core/engine/preview.rs` (`compose_stable_system_prompt` + `system_prompt_text`) calls
   `enforce_static_prefix_guard` after the stable prefix is composed and the tool registry is
   known. The guard covers the **stable** prefix only (base prompt, tool policy, instructions,
   skills index, tool schemas); volatile memory/environment blocks are excluded (memory has
   its own budget). On trim, rebuild the instruction block once, recompose, re-measure once;
   if both scopes are zero and the stable prefix is still over, return `Unsatisfiable`.
6. **Startup notice**
   - Emit the user line as a `HistoryCell::System` at session start (TUI), stderr
     (headless/exec), and a session event (Runtime API); re-emit on resume from the persisted
     event. Do not reuse `startup_notice` (auto-resume status line).
7. **`crates/tui/src/context_report.rs`**
   - Show per-source action, bytes/tokens, budget and limit kind.
8. **Docs and registry**
   - `docs/CONFIGURATION.md`, `docs/zh_hans/CONFIGURATION.md` (mirror, or an explicit deferral
     note with the sync date), `config.example.toml` (replace the fixed 48 KiB comment),
     `CHANGELOG.md`, and a `docs/features.toml` row with id/name/summary/status/surfaces/
     since/flag/docs/owner (the registry test fails on drift).
9. **Tests** (one per branch, plus the boundary cases):
   - `agents_md_wins_over_claude_md`
   - `claude_md_is_default_fallback`
   - `claude_fallback_can_be_disabled`
   - `claude_import_reads_when_agents_absent_and_fallback_disabled`
   - `explicit_claude_import_reads_both`
   - `agents_md_skipped_falls_back_to_claude`
   - `global_claude_is_last_resort_after_instructions_md`
   - `symlinked_duplicate_is_loaded_once` (canonical path; no fallback)
   - `no_trim_within_scope_budget`
   - `per_scope_budgets_are_independent`
   - `trim_keeps_the_nearest_scope` (segments)
   - `single_oversized_file_keeps_the_head`
   - `zero_bytes_left_is_reported_as_skipped`
   - `rules_share_the_project_scope_budget`
   - `rules_block_is_head_trimmed_with_numbers`
   - `static_guard_trims_by_the_overage`
   - `static_guard_unsatisfied_terminates`
   - `guard_trim_begins_at_residual`
   - `unsatisfiable_only_above_guard`
   - `static_prefix_guard_measures_tool_schemas`
   - `stable_prefix_guard_ignores_memory_growth`
   - `cache_returns_untrimmed_segments_across_budgets`
   - `configured_instructions_are_read_fresh`
   - `inline_instructions_stay_outside_cache`
   - `relay_artifact_cap_unchanged`
   - `refuse_policy_stops_on_trim_but_not_on_skip`
   - `file_over_ceiling_is_skipped_not_trimmed`
   - `file_max_bytes_zero_is_rejected`
   - `empty_file_is_skipped`
   - `rules_cap_is_reported`
   - `unknown_window_uses_128k`
   - `percent_and_guard_bounds_are_validated`
   - `project_overlay_cannot_set_global_budget`
   - `instructions_array_counts_toward_global`
   - `startup_notice_is_a_system_cell_and_stderr`
   - `model_notice_is_a_session_event`
   - `resumed_session_reemits_notice`
   - `context_report_shows_actions_and_budgets`
   - update the existing 48 KiB tests to the new defaults.

## 6. Acceptance criteria

- Default behaviour: no content is trimmed while a source is within its scope budget; the
  only losses are scope-budget trims, static-guard trims, and skips (file ceiling, empty,
  rules cap) — each with a notice.
- `AGENTS.md` selection and the CLAUDE fallback match the truth table, including
  skip-as-absent and `claude_fallback = false`; `project_instruction_imports` still works for
  co-reading.
- Global and project budgets are independent; a huge project does not shrink the global; rules
  are charged to project; configured `instructions` entries are charged to global.
- Any loss (`trimmed`/`skipped`/`refused`) is reported to the user, to the model, and in
  `/context`, with path, scope, source, bytes, tokens and limit kind.
- No silent drop remains: rules cap, empty files, fall-through substitution, symlink
  duplicates and per-file skips are all in the contract; the guard has a terminal
  unsatisfiable branch and trims by a defined overage.
- All existing tests pass; the new tests above cover every branch; the old 48 KiB tests are
  updated.

## 7. Risks

- **Prompt-cache and cost:** a larger static prefix changes the cached prefix and increases
  cost per request; the PR must include a measured estimate.
- **Upstream acceptance:** the maintainers said the absolute ceiling is their decision
  (#6526); 100k tokens per scope is 6–8× the current 48 KiB. Present the defaults and the
  knobs, and be ready to keep the current default.
- **CLAUDE fallback is a policy change:** a repo with only `CLAUDE.md` previously got no
  project instructions; now it does. Migration note and `claude_fallback = false` escape.
- **Small windows:** at 32k the residual is ~8,000 tokens for skills/tool schemas; exceeding
  it starts guard trimming, and only non-instruction static content above ~21,400 tokens is
  `unsatisfiable`. The PR should include a 32k measurement and may lower the default percents
  for small windows.
- **Estimator drift:** tokens are estimated, not counted; the notice must say "estimated"
  where it matters.
- **Refactor size:** moving configured `instructions` into the assembly and keeping segments
  on `ProjectContext` touches the loader, cache and prompt builder; the PR should be split
  into a behaviour-preserving refactor and the budget change if maintainers prefer.

## 8. Out of scope

- Skills index budgets and truncation (#7073).
- Skill installation/disable policy (#7066).
- Compaction engine changes.
- Moving instructions from the system prompt into a synthetic message (decided against for
  this PR).

## 9. PR framing

- Reference OpenCode's selection model with source links (`opencode.ai/docs/rules`,
  `anomalyco/opencode` `config/plugin/instruction.ts`, `session/instructions.ts`) and Claude
  Code's no-truncation model (`code.claude.com/docs/en/memory`).
- Include the measured evidence from §1.7, the effective-budget table from §3.2, and the
  cost/cache estimate.
- State the behaviour changes (CLAUDE fallback; 48 KiB → per-scope budgets; a single
  oversized file now keeps its head instead of its tail; configured `instructions` entries
  move from `prompts.rs` rendering into the instruction assembly; the guard covers the stable
  prefix only) and the migration path (`claude_fallback = false`, `[instruction_budget]`).
- Use `Refs #6526`, not `fix`/`close`. Maintainers land verified slices on `main`; this branch
  is a contributor branch, not a maintainer PR branch.

## 10. Implementation notes (v5)

What landed, and where it differs from the plan above. The differences are
places where the plan cited a tree that has since changed, or where the plan's
mechanism did not exist; none of them changes the behaviour contract in §3.

- **Where the code lives.** Per-scope budgets, segments, notices, trim
  markers, and the static-guard decision are in
  `crates/tui/src/project_context/budget.rs`. Loader selection and the
  assembly are in `crates/tui/src/project_context.rs`; `types.rs` carries the
  segments and notices on `ProjectContext`. Tests for the behaviour named in
  §5.9 are in `project_context/instruction_budget_tests.rs` (selection, skips,
  trims, rules, configured entries, `refuse`), `budget.rs` (budget math and the
  guard decision), `prompts.rs` (stable-prefix measurement, configured-entry
  rendering), `lib.rs` (project overlay) and `config.rs`.
- **The "first-turn synthetic message" mechanism** named in §3.6 does not exist
  in this tree (`crates/tui/src/core/agent.rs` and `announce_active_skill` are
  gone). The equivalent is the runtime-provenance synthetic message the LSP
  diagnostics path already uses
  (`runtime_text_message_with_turn_metadata(..., UserInputProvenance::Runtime)`),
  delivered once from turn preparation. It is part of the transcript, so it is
  logged (model-visible ⇒ logged) and a resumed session already has it in
  history — that replaces the separate persisted event.
- **The static guard** runs in turn preparation rather than
  `core/engine/preview.rs`, because that is the first point where the pinned
  prefix and the real tool catalog are known together. It measures the first
  system block plus the tool schemas (`prompts::measure_static_prefix`), trims
  the largest scope by the overage, recomposes and re-pins once under a
  declared reason, and classifies the result with the pure
  `static_guard_outcome`. `compose_stable_system_prompt` now has an
  assembly-returning sibling used by the guard.
- **`policy = "refuse"`** is enforced at session start (TUI and `exec`) by
  `project_context::startup_instruction_notice`, which returns an error for a
  trim loss or a rules-budget loss. Skips stay skips. The engine itself never
  refuses: by the time it runs, the session has already started, and the guard
  reports `static_guard_unsatisfied` and continues. **Known limit:** the
  embed/ACP/app-server surfaces do not run the startup gate, so `refuse` is a
  no-op there — they trim and report like `trim`.
- **Symlinks.** §3.1 assumed the project loader follows them. It does not:
  `fs_confined::open_read` refuses a link that leaves the scope root, so a
  project file linked to the user's global file is now recognised as a
  **duplicate** before the read (the content is already loaded from the wider
  scope), and any other symlinked instruction file is *skipped with a notice*
  rather than followed. That refusal is pre-existing sandbox behaviour; this
  change only makes the duplicate case honest instead of an unreadable-file
  error.
- **Foreign fragments** (`.cursorrules`, `.clinerules`, …) keep their own
  bounded-fragment cap and are not charged to the project scope; they are
  reported in `/context` and named as a known limitation in `budget.rs`.
- **Window used by the startup line.** The TUI/exec startup notice resolves
  budgets for the route window they are about to launch with
  (`route_context_window_tokens` from the app's effective model, or from the
  validated CLI route in `exec`); the 128k default is only a fallback for a
  caller that has no route. A first pass used that default unconditionally,
  which could report a trim that would not happen and, under
  `policy = "refuse"`, stop a session that would have fitted.
- **The user/model channels** are: a `HistoryCell::System` cell at TUI start,
  `eprintln!` for `exec` text output, one runtime-provenance transcript
  message for the model, and per-source rows in `/context report`. The notice
  is recomputed deterministically from files + config, so a resumed session
  states the same losses.
- **`/context report`** rows cover the file-based sources and the resolved
  budgets; the TUI variant passes no configured `instructions` paths (the App
  does not hold the config), so configured entries appear only in the headless
  report and in the startup notice. `exec` prints the notice for text output
  only; a JSON consumer reads the same losses from `/context report`.
- **Build note for this environment.** The shared cargo registry and target
  directory are read-only inside the session sandbox, so the verification
  build used an isolated `CARGO_HOME`/target under `/tmp` (`cargo check -p
  codewhale-tui`), not the shared target dir. The `codewhale-tui` test binary
  is the long pole: full codegen for that crate did not finish inside this
  session's time budget, so test *execution* is the one gate still open here;
  the crate and its `cfg(test)` code were typechecked instead.
