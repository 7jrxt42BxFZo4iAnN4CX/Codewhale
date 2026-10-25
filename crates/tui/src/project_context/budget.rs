//! Per-scope instruction budgets, trim markers, and skip/trim notices (#6526).
//!
//! Standing instruction files are assembled into two independent scopes —
//! `global` (user-wide: the global `AGENTS.md` chain, `~/.claude/CLAUDE.md`,
//! and the configured `instructions = [...]` array) and `project` (the
//! repository-root → workspace chain, co-read foreign files, and the rules
//! block). Each scope has its own ceiling, expressed in **estimated tokens**
//! (`min(absolute_tokens, percent × usable_window)`), so a huge project can no
//! longer shrink the user's global preferences and vice versa.
//!
//! Every loss is reported. A scope that does not fit is trimmed from the
//! broadest end first — the root-most ancestor goes first, the workspace file
//! last — and a single file that alone exceeds its remaining budget keeps its
//! head and drops its tail with a marker carrying bytes and tokens. Skips are
//! not trims: an oversized file, an empty file, a rules-directory overflow, or
//! a symlinked duplicate is reported as `skipped` with the limit that produced
//! it, and never makes the workspace unopenable.
//!
//! Known limitations (deliberate, not bugs):
//!
//! - Token counts are *estimates* from
//!   [`crate::compaction::estimate_text_tokens_conservative`], not a
//!   provider tokenizer. The estimator reads multibyte text conservatively,
//!   but a CJK- or emoji-heavy file can still be off by a comparable margin;
//!   the notice says "estimated" for that reason.
//! - Opted-in foreign *fragments* (`.cursorrules`, `.clinerules`, …) are
//!   loaded by `codewhale_core::fragments` into the world-state block with
//!   their own cap. They are reported in `/context` but are not charged
//!   against the project scope here; only the instruction chain and the rules
//!   block are.
//! - The static-prefix guard is a safety net for the *stable* prefix (base
//!   prompt, instructions, skills index, tool schemas). Volatile world-state
//!   blocks — memory, environment, goals — are excluded because they have
//!   their own budgets and drift mid-session.
//! - Trimming is one pass per scope plus at most one guard-driven second
//!   pass. There is no convergence loop.

use std::path::PathBuf;

use serde::Deserialize;

/// Absolute per-scope default: 100k estimated tokens.
pub(crate) const DEFAULT_SCOPE_BUDGET_TOKENS: usize = 100_000;
/// Per-scope default as a share of the usable window.
pub(crate) const DEFAULT_SCOPE_BUDGET_PERCENT: u8 = 25;
/// Default ceiling for the whole static prefix, as a share of the usable window.
pub(crate) const DEFAULT_STATIC_PREFIX_GUARD_PERCENT: u8 = 80;
/// Hard memory-safety ceiling for one instruction file (8 MiB).
pub(crate) const DEFAULT_FILE_MAX_BYTES: u64 = 8 * 1024 * 1024;
/// Window assumed when the route's window is unknown.
pub(crate) const DEFAULT_CONTEXT_WINDOW_TOKENS: usize = 128 * 1024;
/// Output tokens assumed when the route's ceiling is not resolved here.
const DEFAULT_OUTPUT_TOKENS: usize = 2_048;
/// Share of the window kept free for the response and estimator drift.
const SAFETY_RESERVE_PERCENT: usize = 10;
/// Floor for [`SAFETY_RESERVE_PERCENT`] so small windows keep real headroom.
const SAFETY_RESERVE_MIN_TOKENS: usize = 2_048;
/// Room the guard must leave for non-instruction static content, in points.
const MIN_STATIC_RESIDUAL_PERCENT: u8 = 10;

/// What to do when an instruction scope does not fit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum InstructionBudgetPolicy {
    /// Trim the scope and continue with a warning (default).
    #[default]
    Trim,
    /// Stop the session instead of trimming or running an unsatisfiable guard.
    Refuse,
}

impl InstructionBudgetPolicy {
    #[must_use]
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Trim => "trim",
            Self::Refuse => "refuse",
        }
    }
}

/// The `[instruction_budget]` config table.
///
/// Every field is optional so the table only states what the user changed and
/// the defaults stay in one place. `deny_unknown_fields` makes a typo inside
/// the table fail loudly — the top-level config is deliberately tolerant of
/// unknown *tables* (#6516), so silently accepting `global_budget_token`
/// would hide the mistake forever.
#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InstructionBudgetConfig {
    /// Absolute ceiling for the global scope, in estimated tokens.
    pub global_budget_tokens: Option<usize>,
    /// Absolute ceiling for the project scope, in estimated tokens.
    pub project_budget_tokens: Option<usize>,
    /// Global scope ceiling as a share of the usable window (1–100).
    pub global_budget_percent: Option<u8>,
    /// Project scope ceiling as a share of the usable window (1–100).
    pub project_budget_percent: Option<u8>,
    /// Ceiling for the whole static prefix as a share of the usable window.
    pub static_prefix_guard_percent: Option<u8>,
    /// Hard per-file ceiling in bytes; a larger file is skipped, not trimmed.
    pub file_max_bytes: Option<u64>,
    /// `trim` (default) or `refuse`.
    pub policy: Option<InstructionBudgetPolicy>,
    /// Read `CLAUDE.md` when no `AGENTS.md`/codewhale file is present.
    pub claude_fallback: Option<bool>,
}

impl InstructionBudgetConfig {
    /// Reject values that cannot mean anything.
    ///
    /// Called while config is applied, so a broken table stops the launch
    /// instead of silently producing a nonsensical budget.
    pub(crate) fn validate(&self) -> Result<(), String> {
        for (name, value) in [
            ("global_budget_tokens", self.global_budget_tokens),
            ("project_budget_tokens", self.project_budget_tokens),
        ] {
            if value == Some(0) {
                return Err(format!(
                    "instruction_budget.{name} must be positive (0 is not an unlimited budget)"
                ));
            }
        }
        if self.file_max_bytes == Some(0) {
            return Err(
                "instruction_budget.file_max_bytes must be positive (a hard ceiling must exist)"
                    .to_string(),
            );
        }
        for (name, value) in [
            ("global_budget_percent", self.global_budget_percent),
            ("project_budget_percent", self.project_budget_percent),
            (
                "static_prefix_guard_percent",
                self.static_prefix_guard_percent,
            ),
        ] {
            if let Some(value) = value
                && !(1..=100).contains(&value)
            {
                return Err(format!(
                    "instruction_budget.{name} must be between 1 and 100 (got {value})"
                ));
            }
        }
        let global_percent = self
            .global_budget_percent
            .unwrap_or(DEFAULT_SCOPE_BUDGET_PERCENT);
        let project_percent = self
            .project_budget_percent
            .unwrap_or(DEFAULT_SCOPE_BUDGET_PERCENT);
        let guard = self
            .static_prefix_guard_percent
            .unwrap_or(DEFAULT_STATIC_PREFIX_GUARD_PERCENT);
        if guard < global_percent.saturating_add(project_percent) {
            return Err(format!(
                "instruction_budget.static_prefix_guard_percent ({guard}) must be at least the sum of the scope percents ({global_percent} + {project_percent})"
            ));
        }
        if guard - global_percent.saturating_add(project_percent) < MIN_STATIC_RESIDUAL_PERCENT {
            return Err(format!(
                "instruction_budget.static_prefix_guard_percent ({guard}) must leave at least {MIN_STATIC_RESIDUAL_PERCENT} points above the scope percents ({global_percent} + {project_percent}) for non-instruction static content"
            ));
        }
        Ok(())
    }
}

/// Which scope an instruction source belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum InstructionScope {
    /// User-wide authority: global files and configured `instructions` entries.
    Global,
    /// Workspace authority: the per-directory chain and the rules block.
    Project,
}

impl InstructionScope {
    #[must_use]
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Global => "global",
            Self::Project => "project",
        }
    }
}

/// Where an instruction source came from, for notices and `/context`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InstructionOrigin {
    /// An `AGENTS.md` (project chain or global chain).
    Agents,
    /// A `CLAUDE.md` / `.claude/CLAUDE.md` / `.claude/instructions.md`.
    Claude,
    /// Codewhale's own `.codewhale/instructions.md` or legacy
    /// `.deepseek/instructions.md`.
    Instructions,
    /// The assembled rules block.
    Rules,
    /// The configured `instructions = [...]` array.
    Configured,
}

impl InstructionOrigin {
    #[must_use]
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Agents => "agents",
            Self::Claude => "claude",
            Self::Instructions => "instructions",
            Self::Rules => "rules",
            Self::Configured => "configured",
        }
    }
}

/// What happened to one source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InstructionAction {
    Kept,
    Trimmed,
    Skipped,
    Refused,
}

impl InstructionAction {
    #[must_use]
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Kept => "kept",
            Self::Trimmed => "trimmed",
            Self::Skipped => "skipped",
            Self::Refused => "refused",
        }
    }
}

/// The limit that produced a loss.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum InstructionLimit {
    None,
    /// The scope's own token budget.
    ScopeBudget,
    /// The static-prefix guard trimmed the largest scope by an overage.
    StaticGuard,
    /// `file_max_bytes`.
    FileCeiling,
    /// The file was whitespace-only.
    EmptyFile,
    /// `MAX_RULES_FILES` per directory.
    RulesCap,
    /// The project scope budget was exhausted before the rules block.
    RulesBudget,
    /// The same canonical file was already loaded in another scope.
    DuplicatePath,
}

impl InstructionLimit {
    #[must_use]
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::ScopeBudget => "scope_budget",
            Self::StaticGuard => "static_guard",
            Self::FileCeiling => "file_ceiling",
            Self::EmptyFile => "empty_file",
            Self::RulesCap => "rules_cap",
            Self::RulesBudget => "rules_budget",
            Self::DuplicatePath => "duplicate_path",
        }
    }
}

/// One instruction source that was kept, trimmed, skipped, or refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InstructionNotice {
    pub(crate) path: Option<PathBuf>,
    pub(crate) scope: InstructionScope,
    pub(crate) origin: InstructionOrigin,
    pub(crate) action: InstructionAction,
    pub(crate) bytes_kept: u64,
    pub(crate) bytes_dropped: u64,
    pub(crate) tokens_kept: usize,
    pub(crate) tokens_dropped: usize,
    pub(crate) limit: InstructionLimit,
    /// Lower-precedence file selected after the preferred one was skipped.
    pub(crate) fallback_selected: Option<String>,
    /// Human-readable detail for notices that are not a budget loss
    /// (a refused symlinked rules directory, an unreadable file, a cap).
    pub(crate) detail: Option<String>,
}

impl InstructionNotice {
    pub(crate) fn loss(
        scope: InstructionScope,
        origin: InstructionOrigin,
        path: Option<PathBuf>,
        limit: InstructionLimit,
        action: InstructionAction,
    ) -> Self {
        Self {
            path,
            scope,
            origin,
            action,
            bytes_kept: 0,
            bytes_dropped: 0,
            tokens_kept: 0,
            tokens_dropped: 0,
            limit,
            fallback_selected: None,
            detail: None,
        }
    }

    /// Notice for the static-prefix guard: either the trim it performed or the
    /// terminal state where every instruction scope is already empty.
    pub(crate) fn static_guard(
        scope: InstructionScope,
        tokens_dropped: usize,
        unsatisfiable: bool,
    ) -> Self {
        Self {
            path: None,
            scope,
            origin: InstructionOrigin::Instructions,
            action: if unsatisfiable {
                InstructionAction::Skipped
            } else {
                InstructionAction::Trimmed
            },
            bytes_kept: 0,
            bytes_dropped: 0,
            tokens_kept: 0,
            tokens_dropped,
            limit: InstructionLimit::StaticGuard,
            fallback_selected: None,
            detail: None,
        }
    }

    /// Whether the notice describes a loss an operator must be told about.
    #[must_use]
    pub(crate) fn is_loss(&self) -> bool {
        self.action != InstructionAction::Kept
    }

    /// Display name of the source: the file name, or the scope name when the
    /// loss covers a whole scope (for example the rules block).
    #[must_use]
    pub(crate) fn label(&self) -> String {
        self.path
            .as_ref()
            .and_then(|path| path.file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| format!("{} instructions", self.scope.as_str()))
    }
}

impl std::fmt::Display for InstructionNotice {
    /// One line per loss, suitable for a system history cell, stderr, or a
    /// synthetic model message. Numbers are always present with their unit so
    /// "trimmed" never hides how much was dropped.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.limit == InstructionLimit::StaticGuard {
            return if self.action == InstructionAction::Skipped {
                write!(
                    f,
                    "instructions: static prefix is still ~{} estimated tokens over the static-prefix guard with every instruction scope trimmed to zero; continuing under policy `trim` — raise static_prefix_guard_percent or narrow the tool surface.",
                    self.tokens_dropped
                )
            } else {
                write!(
                    f,
                    "instructions: {} scope trimmed by ~{} estimated tokens to keep the static prefix under the guard",
                    self.scope.as_str(),
                    self.tokens_dropped
                )
            };
        }
        write!(
            f,
            "instructions: {} {} ({}, {}, {})",
            self.scope.as_str(),
            self.label(),
            self.origin.as_str(),
            self.action.as_str(),
            self.limit.as_str()
        )?;
        if self.action == InstructionAction::Trimmed {
            write!(
                f,
                " kept {} of {} bytes and ~{} of ~{} estimated tokens",
                self.bytes_kept,
                self.bytes_kept.saturating_add(self.bytes_dropped),
                self.tokens_kept,
                self.tokens_kept.saturating_add(self.tokens_dropped)
            )?;
        } else if self.action == InstructionAction::Skipped && self.bytes_dropped > 0 {
            write!(f, " dropped {} bytes", self.bytes_dropped)?;
        }
        if let Some(fallback) = self.fallback_selected.as_deref() {
            write!(f, "; fallback: {fallback}")?;
        }
        if let Some(detail) = self.detail.as_deref() {
            write!(f, "; {detail}")?;
        }
        Ok(())
    }
}

/// Render the whole loss list as the body of the startup notice.
#[must_use]
pub(crate) fn render_notice_body(notices: &[InstructionNotice]) -> Option<String> {
    let losses: Vec<&InstructionNotice> = notices.iter().filter(|n| n.is_loss()).collect();
    if losses.is_empty() {
        return None;
    }
    // Bounded on purpose: a rules directory beyond the cap yields one loss per
    // extra file, and an unbounded summary would be a first-turn message larger
    // than the budget it describes.
    const MAX_LINES: usize = 12;
    const MAX_BYTES: usize = 2_000;
    let mut body = String::from(
        "Standing instructions were trimmed or skipped before this session started:\n",
    );
    let mut shown = 0usize;
    for notice in &losses {
        let line = format!("- {notice}\n");
        if shown >= MAX_LINES || body.len() + line.len() > MAX_BYTES {
            break;
        }
        body.push_str(&line);
        shown += 1;
    }
    if shown < losses.len() {
        body.push_str(&format!(
            "- …and {} more loss(es); the full list is in /context report.\n",
            losses.len() - shown
        ));
    }
    body.push_str("Run /context report for the full per-source breakdown.");
    Some(body)
}

/// Raw instruction content, kept per source so a scope can be trimmed by
/// whole segments instead of by unmovable joined text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct InstructionSegment {
    pub(crate) scope: InstructionScope,
    pub(crate) origin: InstructionOrigin,
    /// `None` for generated content that has no file behind it.
    pub(crate) path: Option<PathBuf>,
    /// Display label used in the rendered block: the repo-relative path for
    /// chain segments, the absolute path the historical `<!-- global: … -->`
    /// label carried, or a synthetic name for inline sources.
    pub(crate) label: Option<String>,
    /// Canonical path, used to load one physical file once across scopes.
    pub(crate) canonical_path: Option<PathBuf>,
    pub(crate) text: String,
    /// Lower-precedence file chosen after the preferred one was skipped.
    pub(crate) fallback_selected: Option<String>,
}

impl InstructionSegment {
    pub(crate) fn new(
        scope: InstructionScope,
        origin: InstructionOrigin,
        path: Option<PathBuf>,
        text: String,
    ) -> Self {
        let canonical_path = path.as_deref().and_then(canonical_path_of);
        Self {
            scope,
            origin,
            path,
            label: None,
            canonical_path,
            text,
            fallback_selected: None,
        }
    }
}

/// Canonical identity of a file, used for symlink dedupe.
///
/// `fs::canonicalize` resolves every symlink on the way, so two paths that
/// reach one physical file compare equal. Hard links are distinct directory
/// entries for the same inode; they are **not** deduped here — a hard link is
/// read as its own file, a documented limitation of this pass.
fn canonical_path_of(path: &std::path::Path) -> Option<PathBuf> {
    std::fs::canonicalize(path).ok()
}

/// Resolved, per-route budgets. Percentages are of the *usable* window, never
/// the raw window: the response and the safety reserve are not available to
/// standing instructions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct InstructionBudgets {
    pub(crate) global_tokens: usize,
    pub(crate) project_tokens: usize,
    pub(crate) static_guard_tokens: usize,
    pub(crate) file_max_bytes: u64,
    pub(crate) policy: InstructionBudgetPolicy,
    pub(crate) claude_fallback: bool,
    pub(crate) usable_window_tokens: usize,
}

impl InstructionBudgets {
    /// Resolve config + route window into concrete token ceilings.
    ///
    /// `window` is the route's context window; `output` its resolved output
    /// ceiling when the caller has one. An unknown window resolves to
    /// [`DEFAULT_CONTEXT_WINDOW_TOKENS`] so budget math never divides by a
    /// guess of zero.
    pub(crate) fn resolve(
        config: Option<&InstructionBudgetConfig>,
        window: Option<usize>,
        output: Option<usize>,
    ) -> Result<Self, String> {
        if let Some(config) = config {
            config.validate()?;
        }
        let window = window.unwrap_or(DEFAULT_CONTEXT_WINDOW_TOKENS);
        let usable = usable_window_tokens(window, output);
        let global_percent = config
            .and_then(|c| c.global_budget_percent)
            .unwrap_or(DEFAULT_SCOPE_BUDGET_PERCENT);
        let project_percent = config
            .and_then(|c| c.project_budget_percent)
            .unwrap_or(DEFAULT_SCOPE_BUDGET_PERCENT);
        let guard_percent = config
            .and_then(|c| c.static_prefix_guard_percent)
            .unwrap_or(DEFAULT_STATIC_PREFIX_GUARD_PERCENT);
        let global_tokens = config
            .and_then(|c| c.global_budget_tokens)
            .unwrap_or(DEFAULT_SCOPE_BUDGET_TOKENS)
            .min(share_of(usable, global_percent));
        let project_tokens = config
            .and_then(|c| c.project_budget_tokens)
            .unwrap_or(DEFAULT_SCOPE_BUDGET_TOKENS)
            .min(share_of(usable, project_percent));
        Ok(Self {
            global_tokens,
            project_tokens,
            static_guard_tokens: share_of(usable, guard_percent),
            file_max_bytes: config
                .and_then(|c| c.file_max_bytes)
                .unwrap_or(DEFAULT_FILE_MAX_BYTES),
            policy: config
                .and_then(|c| c.policy)
                .unwrap_or(InstructionBudgetPolicy::Trim),
            claude_fallback: config.and_then(|c| c.claude_fallback).unwrap_or(true),
            usable_window_tokens: usable,
        })
    }

    /// The shipped defaults for a route window, with no user table.
    ///
    /// Infallible by construction: validation only rejects configured values,
    /// and every default is positive and consistent.
    #[must_use]
    pub(crate) fn defaults_for(window: Option<usize>) -> Self {
        let usable = usable_window_tokens(window.unwrap_or(DEFAULT_CONTEXT_WINDOW_TOKENS), None);
        Self {
            global_tokens: DEFAULT_SCOPE_BUDGET_TOKENS
                .min(share_of(usable, DEFAULT_SCOPE_BUDGET_PERCENT)),
            project_tokens: DEFAULT_SCOPE_BUDGET_TOKENS
                .min(share_of(usable, DEFAULT_SCOPE_BUDGET_PERCENT)),
            static_guard_tokens: share_of(usable, DEFAULT_STATIC_PREFIX_GUARD_PERCENT),
            file_max_bytes: DEFAULT_FILE_MAX_BYTES,
            policy: InstructionBudgetPolicy::Trim,
            claude_fallback: true,
            usable_window_tokens: usable,
        }
    }

    /// Set one scope's ceiling to an absolute value, for the static guard.
    ///
    /// Deliberately absolute rather than "ceiling minus the overage": a scope
    /// that uses less than its ceiling would otherwise keep every byte and the
    /// prefix would stay over the guard. The guard computes the new ceiling
    /// from the scope's *retained* usage minus the overage.
    pub(crate) fn with_scope_ceiling(mut self, scope: InstructionScope, tokens: usize) -> Self {
        match scope {
            InstructionScope::Global => self.global_tokens = tokens,
            InstructionScope::Project => self.project_tokens = tokens,
        }
        self
    }
}

/// How much the static-prefix guard asks a rebuild to trim, and from where.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct InstructionGuardTrim {
    pub(crate) scope: InstructionScope,
    /// New ceiling for `scope`, in estimated tokens.
    pub(crate) tokens: usize,
}

/// Outcome of comparing the measured static prefix against the guard.
///
/// Two thresholds are distinct: trimming begins when the prefix exceeds the
/// guard, and the guard is *unsatisfiable* only when nothing is left to trim
/// and the prefix is still over. The latter is a terminal state — the session
/// proceeds under `policy = "trim"` (or refuses at start under `"refuse"`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum StaticGuardOutcome {
    Within,
    Trim {
        scope: InstructionScope,
        tokens: usize,
    },
    Unsatisfiable {
        over: usize,
    },
}

/// Decide what the guard does for one measurement.
///
/// `largest_scope` picks the scope to cut; `tokens_kept` is how much
/// instruction content survived the scope budgets, which is what makes the
/// difference between "trim more" and "nothing left to trim".
#[must_use]
pub(crate) fn static_guard_outcome(
    measured_tokens: usize,
    budgets: &InstructionBudgets,
    largest_scope: Option<InstructionScope>,
    tokens_kept: usize,
) -> StaticGuardOutcome {
    let guard = budgets.static_guard_tokens;
    if measured_tokens <= guard {
        return StaticGuardOutcome::Within;
    }
    let over = measured_tokens - guard;
    if tokens_kept == 0 {
        return StaticGuardOutcome::Unsatisfiable { over };
    }
    StaticGuardOutcome::Trim {
        scope: largest_scope.unwrap_or(InstructionScope::Global),
        tokens: over,
    }
}

/// Usable window: the route window minus the response ceiling and a safety
/// reserve of 10% (minimum 2,048 tokens).
#[must_use]
pub(crate) fn usable_window_tokens(window: usize, output: Option<usize>) -> usize {
    let output = output.unwrap_or(DEFAULT_OUTPUT_TOKENS);
    let reserve = (window * SAFETY_RESERVE_PERCENT / 100).max(SAFETY_RESERVE_MIN_TOKENS);
    window.saturating_sub(output).saturating_sub(reserve)
}

fn share_of(total: usize, percent: u8) -> usize {
    total / 100 * usize::from(percent) + (total % 100) * usize::from(percent) / 100
}

/// Estimated tokens for instruction text. One estimator for budgets,
/// reports, and markers, so a number shown to the user is the number the
/// trim used.
#[must_use]
pub(crate) fn estimate_instruction_tokens(text: &str) -> usize {
    crate::compaction::estimate_text_tokens_conservative(text)
}

/// Result of trimming one piece of instruction text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TrimmedText {
    pub(crate) text: String,
    pub(crate) bytes_kept: u64,
    pub(crate) bytes_dropped: u64,
    pub(crate) tokens_kept: usize,
    pub(crate) tokens_dropped: usize,
    /// True when the marker alone consumed the budget and no source text
    /// survived.
    pub(crate) empty: bool,
}

impl TrimmedText {
    fn whole(text: &str) -> Self {
        let bytes = text.len() as u64;
        Self {
            text: text.to_string(),
            bytes_kept: bytes,
            bytes_dropped: 0,
            tokens_kept: estimate_instruction_tokens(text),
            tokens_dropped: 0,
            empty: text.is_empty(),
        }
    }
}

/// Marker inserted where a head-trim dropped the tail.
#[must_use]
pub(crate) fn instruction_trim_marker(
    scope: InstructionScope,
    kept: usize,
    dropped: usize,
) -> String {
    format!(
        "\n\n[…{} instructions trimmed: ~{kept} estimated tokens kept, ~{dropped} estimated tokens dropped at the {} scope budget…]",
        scope.as_str(),
        scope.as_str()
    )
}

/// Marker inserted when the rules block was trimmed.
#[must_use]
pub(crate) fn rules_trim_marker(kept: usize, dropped: usize) -> String {
    format!(
        "\n\n[…rules block trimmed: ~{kept} estimated tokens kept, ~{dropped} estimated tokens dropped at the project scope budget…]"
    )
}

/// Keep the longest UTF-8 prefix of `text` whose estimated token count, plus
/// the marker, fits `budget_tokens`.
///
/// The cut point is found by binary search over character boundaries, so the
/// result is deterministic and never splits a character.
#[must_use]
pub(crate) fn keep_head_within_tokens(
    text: &str,
    budget_tokens: usize,
    marker: impl Fn(usize, usize) -> String,
) -> TrimmedText {
    let total_tokens = estimate_instruction_tokens(text);
    if total_tokens <= budget_tokens {
        return TrimmedText::whole(text);
    }
    // The marker is charged to the budget, exactly as the old byte cap did.
    let marker_reserve = estimate_instruction_tokens(&marker(0, total_tokens));
    let available = budget_tokens.saturating_sub(marker_reserve);

    // Search over byte offsets, but only ever cut on a character boundary.
    // Both bounds move every iteration (`low` up to a strictly larger
    // boundary, `high` down past the probed one), so the loop terminates for
    // multibyte text — where a naive midpoint search can settle on the same
    // floored boundary forever.
    let floor_boundary = |index: usize| {
        let mut index = index.min(text.len());
        while index > 0 && !text.is_char_boundary(index) {
            index -= 1;
        }
        index
    };
    let next_boundary = |index: usize| {
        let mut index = index.min(text.len());
        while index < text.len() && !text.is_char_boundary(index) {
            index += 1;
        }
        index
    };
    let mut best = 0usize;
    let mut low = 0usize;
    let mut high = text.len();
    while low < high {
        let mut middle = floor_boundary(low + (high - low) / 2);
        if middle <= low {
            middle = next_boundary(middle + 1);
            if middle > high {
                break;
            }
        }
        if estimate_instruction_tokens(&text[..middle]) <= available {
            best = middle;
            low = middle;
        } else {
            if middle == 0 {
                break;
            }
            high = middle - 1;
        }
    }

    let head = text[..best].trim_end();
    let kept_tokens = estimate_instruction_tokens(head);
    let dropped_tokens = total_tokens.saturating_sub(kept_tokens);
    if head.is_empty() {
        return TrimmedText {
            text: String::new(),
            bytes_kept: 0,
            bytes_dropped: text.len() as u64,
            tokens_kept: 0,
            tokens_dropped: total_tokens,
            empty: true,
        };
    }
    let marker = marker(kept_tokens, dropped_tokens);
    TrimmedText {
        text: format!("{head}{marker}"),
        bytes_kept: head.len() as u64,
        bytes_dropped: (text.len() - head.len()) as u64,
        tokens_kept: kept_tokens,
        tokens_dropped: dropped_tokens,
        empty: false,
    }
}

/// Result of one budget pass over both scopes.
#[derive(Debug, Clone)]
pub(crate) struct InstructionAssembly {
    /// The rendered `<project_instructions>` body, already trimmed.
    pub(crate) block: Option<String>,
    /// Losses from trimming, plus the loader's skip notices.
    pub(crate) notices: Vec<InstructionNotice>,
    /// Estimated tokens kept per scope after trimming.
    pub(crate) global_tokens: usize,
    pub(crate) project_tokens: usize,
    /// The budgets this pass used.
    pub(crate) budgets: InstructionBudgets,
    /// Scope that holds the most instruction tokens; the static guard trims
    /// this one first.
    pub(crate) largest_scope: Option<InstructionScope>,
}

impl InstructionAssembly {
    /// Estimated tokens of instruction content that survived the pass.
    #[must_use]
    pub(crate) fn tokens_kept(&self) -> usize {
        self.global_tokens.saturating_add(self.project_tokens)
    }
}

/// Apply each scope's ceiling to the raw segments, then charge the rules
/// block to what is left of the project scope.
///
/// Segments are dropped whole from the broadest end of their scope first; a
/// single segment above its remaining budget keeps its head. Mutating the
/// segments in place is what lets the joined string be rebuilt from the
/// trimmed truth instead of being trimmed as an opaque blob.
pub(crate) fn apply_instruction_budgets(
    segments: &mut Vec<InstructionSegment>,
    rules_block: &mut Option<String>,
    budgets: &InstructionBudgets,
) -> Vec<InstructionNotice> {
    let mut notices = Vec::new();
    trim_scope(
        segments,
        InstructionScope::Global,
        budgets.global_tokens,
        &mut notices,
    );
    let project_kept = trim_scope(
        segments,
        InstructionScope::Project,
        budgets.project_tokens,
        &mut notices,
    );

    // Anything that survived without a loss notice is reported as `kept`, so
    // `/context report` can list every source's action rather than only the
    // ones that lost content.
    let loss_paths: Vec<Option<PathBuf>> =
        notices.iter().map(|notice| notice.path.clone()).collect();
    for segment in segments.iter() {
        if loss_paths.contains(&segment.path) {
            continue;
        }
        notices.push(InstructionNotice {
            path: segment.path.clone(),
            scope: segment.scope,
            origin: segment.origin,
            action: InstructionAction::Kept,
            bytes_kept: segment.text.len() as u64,
            bytes_dropped: 0,
            tokens_kept: estimate_instruction_tokens(&segment.text),
            tokens_dropped: 0,
            limit: InstructionLimit::None,
            fallback_selected: None,
            detail: None,
        });
    }

    // Rules are project-scope authority and read after the chain, so they get
    // whatever the instruction chain could not spend.
    if let Some(rules) = rules_block.as_mut() {
        let total = estimate_instruction_tokens(rules);
        let remaining = budgets.project_tokens.saturating_sub(project_kept);
        if total > remaining {
            if remaining == 0 {
                notices.push(InstructionNotice {
                    path: None,
                    scope: InstructionScope::Project,
                    origin: InstructionOrigin::Rules,
                    action: InstructionAction::Skipped,
                    bytes_kept: 0,
                    bytes_dropped: rules.len() as u64,
                    tokens_kept: 0,
                    tokens_dropped: total,
                    limit: InstructionLimit::RulesBudget,
                    fallback_selected: None,
                    detail: Some(format!(
                        "project scope budget of {} estimated tokens exhausted by instructions",
                        budgets.project_tokens
                    )),
                });
                *rules_block = None;
            } else {
                let trimmed = keep_head_within_tokens(rules, remaining, rules_trim_marker);
                notices.push(InstructionNotice {
                    path: None,
                    scope: InstructionScope::Project,
                    origin: InstructionOrigin::Rules,
                    action: if trimmed.empty {
                        InstructionAction::Skipped
                    } else {
                        InstructionAction::Trimmed
                    },
                    bytes_kept: trimmed.bytes_kept,
                    bytes_dropped: trimmed.bytes_dropped,
                    tokens_kept: trimmed.tokens_kept,
                    tokens_dropped: trimmed.tokens_dropped,
                    limit: InstructionLimit::RulesBudget,
                    fallback_selected: None,
                    detail: None,
                });
                *rules_block = if trimmed.text.is_empty() {
                    None
                } else {
                    Some(trimmed.text)
                };
            }
        } else {
            notices.push(InstructionNotice {
                path: None,
                scope: InstructionScope::Project,
                origin: InstructionOrigin::Rules,
                action: InstructionAction::Kept,
                bytes_kept: rules.len() as u64,
                bytes_dropped: 0,
                tokens_kept: estimate_instruction_tokens(rules),
                tokens_dropped: 0,
                limit: InstructionLimit::None,
                fallback_selected: None,
                detail: None,
            });
        }
    }

    notices
}

fn trim_scope(
    segments: &mut Vec<InstructionSegment>,
    scope: InstructionScope,
    budget_tokens: usize,
    notices: &mut Vec<InstructionNotice>,
) -> usize {
    let mut order: Vec<usize> = segments
        .iter()
        .enumerate()
        .filter(|(_, segment)| segment.scope == scope)
        .map(|(index, _)| index)
        .collect();
    if order.is_empty() {
        return 0;
    }
    let mut kept: usize = order
        .iter()
        .map(|&index| estimate_instruction_tokens(&segments[index].text))
        .sum();
    if kept <= budget_tokens {
        return kept;
    }

    // 1. Drop whole segments from the broadest end, keeping at least the most
    //    specific one to head-trim below.
    let mut dropped: Vec<usize> = Vec::new();
    while order.len() > 1 && kept > budget_tokens {
        let index = order.remove(0);
        kept = kept.saturating_sub(estimate_instruction_tokens(&segments[index].text));
        dropped.push(index);
    }

    let dropped_count = dropped.len();
    for index in &dropped {
        let segment = &segments[*index];
        let tokens = estimate_instruction_tokens(&segment.text);
        notices.push(InstructionNotice {
            path: segment.path.clone(),
            scope,
            origin: segment.origin,
            action: InstructionAction::Trimmed,
            bytes_kept: 0,
            bytes_dropped: segment.text.len() as u64,
            tokens_kept: 0,
            tokens_dropped: tokens,
            limit: InstructionLimit::ScopeBudget,
            fallback_selected: None,
            detail: Some("dropped as a broadest-scope segment".to_string()),
        });
    }
    // Remove them from the segment list: a notice that says content was
    // dropped while the content still renders is worse than no notice. Every
    // dropped index sits before `order[0]`, so removing in descending order
    // keeps the surviving index valid.
    for index in dropped.iter().rev() {
        segments.remove(*index);
    }

    // 2. A single segment above the remaining budget keeps its head.
    let last = order[0] - dropped_count;
    let trimmed = keep_head_within_tokens(&segments[last].text, budget_tokens, |kept, dropped| {
        instruction_trim_marker(scope, kept, dropped)
    });
    if trimmed.empty {
        let segment = &segments[last];
        notices.push(InstructionNotice {
            path: segment.path.clone(),
            scope,
            origin: segment.origin,
            action: InstructionAction::Skipped,
            bytes_kept: 0,
            bytes_dropped: segment.text.len() as u64,
            tokens_kept: 0,
            tokens_dropped: estimate_instruction_tokens(&segment.text),
            limit: InstructionLimit::ScopeBudget,
            fallback_selected: None,
            detail: Some(format!(
                "scope budget of {budget_tokens} estimated tokens left no room for content"
            )),
        });
        segments.remove(last);
    } else if trimmed.bytes_dropped > 0 {
        notices.push(InstructionNotice {
            path: segments[last].path.clone(),
            scope,
            origin: segments[last].origin,
            action: InstructionAction::Trimmed,
            bytes_kept: trimmed.bytes_kept,
            bytes_dropped: trimmed.bytes_dropped,
            tokens_kept: trimmed.tokens_kept,
            tokens_dropped: trimmed.tokens_dropped,
            limit: InstructionLimit::ScopeBudget,
            fallback_selected: None,
            detail: None,
        });
        segments[last].text = trimmed.text;
    }
    // When the surviving segment already fits, nothing is reported for it: a
    // "trimmed" notice on untouched text is exactly the kind of false receipt
    // this contract exists to prevent. The `kept` pass below covers it.

    // Report what the scope actually kept: a later scope (the rules block) is
    // charged against the remainder, so an under-count would over-allocate it.
    segments
        .iter()
        .filter(|segment| segment.scope == scope)
        .map(|segment| estimate_instruction_tokens(&segment.text))
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usable_window_subtracts_output_and_ten_percent() {
        assert_eq!(usable_window_tokens(128_000, Some(2_100)), 113_100);
        assert_eq!(usable_window_tokens(32_000, Some(2_000)), 26_800);
        // The reserve never drops below its floor.
        assert_eq!(usable_window_tokens(10_000, Some(0)), 10_000 - 2_048);
    }

    #[test]
    fn per_scope_budget_takes_the_smaller_of_absolute_and_percent() {
        let budgets =
            InstructionBudgets::resolve(None, Some(128_000), Some(2_100)).expect("resolve");
        // 25% of 113,100 is 28,275, below the 100k absolute cap; 80% is
        // 90,480.
        assert_eq!(budgets.global_tokens, 28_275);
        assert_eq!(budgets.project_tokens, 28_275);
        assert_eq!(budgets.static_guard_tokens, 90_480);

        let budgets =
            InstructionBudgets::resolve(None, Some(1_000_000), Some(2_048)).expect("resolve");
        // The absolute cap wins on a large window: 10% per scope, 20% total.
        assert_eq!(budgets.global_tokens, DEFAULT_SCOPE_BUDGET_TOKENS);
        assert_eq!(budgets.project_tokens, DEFAULT_SCOPE_BUDGET_TOKENS);
    }

    #[test]
    fn absolute_cap_is_independent_of_window() {
        let config = InstructionBudgetConfig {
            global_budget_tokens: Some(500),
            project_budget_tokens: Some(25_000),
            ..Default::default()
        };
        let budgets = InstructionBudgets::resolve(Some(&config), Some(128_000), Some(2_100))
            .expect("resolve");
        assert_eq!(budgets.global_tokens, 500);
        assert_eq!(budgets.project_tokens, 25_000);
    }

    #[test]
    fn invalid_percentages_and_zero_budgets_are_rejected() {
        for config in [
            InstructionBudgetConfig {
                global_budget_tokens: Some(0),
                ..Default::default()
            },
            InstructionBudgetConfig {
                project_budget_tokens: Some(0),
                ..Default::default()
            },
            InstructionBudgetConfig {
                file_max_bytes: Some(0),
                ..Default::default()
            },
            InstructionBudgetConfig {
                global_budget_percent: Some(0),
                ..Default::default()
            },
            InstructionBudgetConfig {
                project_budget_percent: Some(101),
                ..Default::default()
            },
            InstructionBudgetConfig {
                static_prefix_guard_percent: Some(0),
                ..Default::default()
            },
        ] {
            assert!(config.validate().is_err(), "{config:?} must be rejected");
        }
    }

    #[test]
    fn guard_must_leave_room_for_other_static_content() {
        let too_tight = InstructionBudgetConfig {
            global_budget_percent: Some(25),
            project_budget_percent: Some(25),
            static_prefix_guard_percent: Some(55),
            ..Default::default()
        };
        assert!(too_tight.validate().is_err());

        let inverted = InstructionBudgetConfig {
            global_budget_percent: Some(60),
            project_budget_percent: Some(60),
            static_prefix_guard_percent: Some(80),
            ..Default::default()
        };
        assert!(inverted.validate().is_err());

        let ok = InstructionBudgetConfig {
            global_budget_percent: Some(25),
            project_budget_percent: Some(25),
            static_prefix_guard_percent: Some(70),
            ..Default::default()
        };
        assert!(ok.validate().is_ok());
    }

    #[test]
    fn percent_and_guard_bounds_are_validated() {
        for config in [
            InstructionBudgetConfig {
                global_budget_percent: Some(0),
                ..Default::default()
            },
            InstructionBudgetConfig {
                project_budget_percent: Some(101),
                ..Default::default()
            },
            InstructionBudgetConfig {
                static_prefix_guard_percent: Some(101),
                ..Default::default()
            },
        ] {
            assert!(config.validate().is_err(), "{config:?}");
        }
    }

    #[test]
    fn unknown_window_uses_the_128k_default() {
        let budgets = InstructionBudgets::resolve(None, None, None).expect("resolve");
        assert_eq!(
            budgets.usable_window_tokens,
            usable_window_tokens(DEFAULT_CONTEXT_WINDOW_TOKENS, None)
        );
    }

    #[test]
    fn head_trim_keeps_the_head_and_reports_numbers() {
        let text = "руководство ".repeat(20_000);
        let trimmed = keep_head_within_tokens(&text, 1_000, |kept, dropped| {
            instruction_trim_marker(InstructionScope::Project, kept, dropped)
        });
        assert!(!trimmed.empty);
        assert!(trimmed.tokens_kept <= 1_000);
        assert!(trimmed.tokens_dropped > 0);
        assert!(trimmed.text.starts_with("руководство"));
        assert!(trimmed.text.contains("instructions trimmed"));
        assert_eq!(
            trimmed.bytes_kept as usize + trimmed.bytes_dropped as usize,
            text.len()
        );
    }

    #[test]
    fn static_guard_trims_by_the_overage() {
        let mut budgets = InstructionBudgets::defaults_for(Some(128_000));
        budgets.static_guard_tokens = 1_000;
        assert_eq!(
            static_guard_outcome(1_400, &budgets, Some(InstructionScope::Project), 500),
            StaticGuardOutcome::Trim {
                scope: InstructionScope::Project,
                tokens: 400,
            }
        );
    }

    #[test]
    fn guard_trim_begins_at_residual() {
        // At a 32k window the guard leaves ~8,000 tokens for non-instruction
        // static content. One token under the guard is still fine; one token
        // over starts trimming.
        let budgets = InstructionBudgets::defaults_for(Some(32_000));
        let guard = budgets.static_guard_tokens;
        assert_eq!(
            static_guard_outcome(guard.saturating_sub(1), &budgets, None, 100),
            StaticGuardOutcome::Within
        );
        assert!(matches!(
            static_guard_outcome(guard + 1, &budgets, None, 100),
            StaticGuardOutcome::Trim { tokens: 1, .. }
        ));
    }

    #[test]
    fn unsatisfiable_only_above_guard() {
        let budgets = InstructionBudgets::defaults_for(Some(32_000));
        let guard = budgets.static_guard_tokens;
        assert_eq!(
            static_guard_outcome(guard, &budgets, None, 0),
            StaticGuardOutcome::Within,
            "at the guard there is nothing to do even with no instructions left"
        );
        assert!(matches!(
            static_guard_outcome(guard + 50, &budgets, None, 0),
            StaticGuardOutcome::Unsatisfiable { over: 50 }
        ));
    }

    #[test]
    fn static_guard_unsatisfied_terminates() {
        // The guard is one pass: the same inputs give the same terminal
        // answer, never a second trim request.
        let budgets = InstructionBudgets::defaults_for(Some(32_000));
        let first = static_guard_outcome(budgets.static_guard_tokens + 7, &budgets, None, 0);
        let second = static_guard_outcome(budgets.static_guard_tokens + 7, &budgets, None, 0);
        assert_eq!(first, StaticGuardOutcome::Unsatisfiable { over: 7 });
        assert_eq!(first, second);
    }

    #[test]
    fn text_within_budget_is_returned_whole() {
        let text = "short instructions";
        let trimmed = keep_head_within_tokens(text, 1_000, |kept, dropped| {
            instruction_trim_marker(InstructionScope::Global, kept, dropped)
        });
        assert_eq!(trimmed.text, text);
        assert_eq!(trimmed.bytes_dropped, 0);
    }
}
