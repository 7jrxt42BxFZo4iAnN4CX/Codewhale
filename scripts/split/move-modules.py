#!/usr/bin/env python3
"""Move top-level TUI modules into crates/runtime (the runtime/TUI split).

For each named module this:

1. `git mv`s `crates/tui/src/<m>.rs` and/or `crates/tui/src/<m>/` into
   `crates/runtime/src/`;
2. deletes its `mod <m>;` / `pub mod <m>;` line from the TUI `lib.rs` and adds
   `pub mod <m>;` to the runtime `lib.rs` (sorted);
3. rewrites every `crate::<m>` path in `crates/tui/src` to
   `codewhale_runtime::<m>` and, when the TUI `lib.rs` uses `<m>::` unqualified,
   adds `use codewhale_runtime::<m>;` there;
4. sweeps every string occurrence of the old path across the repository
   (`crates/tui/src/<m>` and `tui/src/<m>`), found with `git grep`.
   CHANGELOG files are history and are left alone.

It changes no item visibility; `scripts/split/widen.py` does that, driven by
the compiler.

Usage:
    python3 scripts/split/move-modules.py <module> [<module> ...]
    python3 scripts/split/move-modules.py --dry-run <module> ...
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[2]
TUI_SRC = REPO_ROOT / "crates" / "tui" / "src"
RUNTIME_SRC = REPO_ROOT / "crates" / "runtime" / "src"
TUI_LIB = TUI_SRC / "lib.rs"
RUNTIME_LIB = RUNTIME_SRC / "lib.rs"


def git(*args: str, check: bool = True) -> str:
    return subprocess.run(
        ["git", *args], cwd=REPO_ROOT, capture_output=True, text=True, check=check
    ).stdout


def module_paths(module: str) -> list[Path]:
    paths = [p for p in (TUI_SRC / f"{module}.rs", TUI_SRC / module) if p.exists()]
    if not paths:
        raise SystemExit(f"move-modules: no crates/tui/src/{module}.rs or {module}/")
    return paths


def remove_mod_line(text: str, module: str) -> tuple[str, bool]:
    pattern = re.compile(rf"^(?:pub(?:\([^)]*\))?\s+)?mod\s+{re.escape(module)}\s*;\n", re.M)
    match = pattern.search(text)
    if not match:
        return text, False
    # Refuse to drop a mod line that carries attributes (cfg, path): that
    # module needs a human decision, not a mechanical move.
    before = text[: match.start()].rstrip("\n").rsplit("\n", 1)[-1].strip()
    if before.startswith("#["):
        raise SystemExit(f"move-modules: `mod {module};` carries an attribute ({before}); move it by hand")
    return text[: match.start()] + text[match.end() :], True


def add_runtime_mod(text: str, module: str) -> str:
    line = f"pub mod {module};\n"
    if line in text:
        return text
    lines = text.splitlines(keepends=True)
    mods = [i for i, l in enumerate(lines) if re.match(r"pub mod \w+;\n", l)]
    if not mods:
        return text.rstrip("\n") + "\n\n" + line
    insert = mods[-1] + 1
    for i in mods:
        if lines[i] > line:
            insert = i
            break
    lines.insert(insert, line)
    return "".join(lines)


def rewrite_callers(modules: list[str], dry_run: bool) -> list[str]:
    alt = "|".join(re.escape(m) for m in modules)
    pattern = re.compile(rf"\bcrate::({alt})(?![A-Za-z0-9_])")
    touched: list[str] = []
    for rel in git("ls-files", "crates/tui/src", check=False).split():
        if not rel.endswith(".rs"):
            continue
        path = REPO_ROOT / rel
        text = path.read_text(encoding="utf-8")
        updated = pattern.sub(lambda m: f"codewhale_runtime::{m.group(1)}", text)
        if updated != text:
            touched.append(rel)
            if not dry_run:
                path.write_text(updated, encoding="utf-8")
    return touched


def import_unqualified(text: str, modules: list[str]) -> str:
    needed = [
        m
        for m in modules
        if re.search(rf"(?<![\w:]){re.escape(m)}::", text)
        and f"use codewhale_runtime::{m};" not in text
    ]
    if not needed:
        return text
    anchor = re.search(r"^use codewhale_runtime::.*\n", text, re.M) or re.search(r"^use .*\n", text, re.M)
    lines = "".join(f"use codewhale_runtime::{m};\n" for m in sorted(needed))
    return text[: anchor.start()] + lines + text[anchor.start() :]


def sweep_paths(module: str, dry_run: bool) -> list[str]:
    patterns = [
        (f"crates/tui/src/{module}", f"crates/runtime/src/{module}"),
        (f"tui/src/{module}", f"runtime/src/{module}"),
    ]
    touched: list[str] = []
    for old, new in patterns:
        # The module as a whole path segment: `elapsed.rs`, `elapsed/`,
        # `elapsed::`, never `elapsed_time.rs`; `tui/src/x` only when it is not
        # already the tail of `crates/tui/src/x`.
        regex = re.compile(
            rf"(?<![A-Za-z0-9_/]){re.escape(old)}(?![A-Za-z0-9_])"
            if not old.startswith("crates/")
            else rf"{re.escape(old)}(?![A-Za-z0-9_])"
        )
        for rel in git("grep", "-l", "-F", old, check=False).split():
            if Path(rel).name.startswith("CHANGELOG"):
                continue
            path = REPO_ROOT / rel
            text = path.read_text(encoding="utf-8")
            updated = regex.sub(new, text)
            if updated != text:
                touched.append(rel)
                if not dry_run:
                    path.write_text(updated, encoding="utf-8")
    return sorted(set(touched))


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n\n")[0])
    parser.add_argument("modules", nargs="+")
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args(argv)

    if not RUNTIME_LIB.is_file():
        raise SystemExit("move-modules: crates/runtime/src/lib.rs does not exist yet")
    tui_lib = TUI_LIB.read_text(encoding="utf-8")
    runtime_lib = RUNTIME_LIB.read_text(encoding="utf-8")
    for module in args.modules:
        paths = module_paths(module)
        tui_lib, found = remove_mod_line(tui_lib, module)
        if not found:
            raise SystemExit(f"move-modules: no `mod {module};` line in crates/tui/src/lib.rs")
        runtime_lib = add_runtime_mod(runtime_lib, module)
        if not args.dry_run:
            for path in paths:
                git("mv", str(path.relative_to(REPO_ROOT)), str((RUNTIME_SRC / path.name).relative_to(REPO_ROOT)))
    if not args.dry_run:
        tui_lib = import_unqualified(tui_lib, args.modules)
        TUI_LIB.write_text(tui_lib, encoding="utf-8")
        RUNTIME_LIB.write_text(runtime_lib, encoding="utf-8")
    for rel in rewrite_callers(args.modules, args.dry_run):
        print(f"callers rewritten: {rel}")
    swept: set[str] = set()
    for module in args.modules:
        swept.update(sweep_paths(module, args.dry_run))
    for rel in sorted(swept):
        print(f"path sweep: {rel}")
    print(f"moved {len(args.modules)} module(s){' (dry run)' if args.dry_run else ''}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
