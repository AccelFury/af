---
name: af-add-subcommand
description: Add a new CLI subcommand to `af` with all four required touch-points wired up at once — clap enum, lifecycle name function, docs/cli-reference.md, and a starter integration test. Leaves a TODO stub for the actual business logic but verifies the skeleton compiles and clap-help renders. Use when the user says "add `af <ns> <cmd>` subcommand", "stub a new af command", or contributes a CLI surface change. Do NOT use to modify existing subcommands or to author business logic — only structural scaffolding.
allowed-tools: Bash, Read, Edit, Write, Grep
---

# af-add-subcommand

Rule #5 of `CLAUDE.md` says every new CLI surface touches **four** files together: the clap enum, the lifecycle-name function, `docs/cli-reference.md`, and an integration test. Forgetting any of the four creates a regression that `af-cli-contract-guard` will catch — but it is cheaper to wire all four up at the same time. This skill is the deterministic scaffolder.

## When to invoke

User says:

- "add `af <namespace> <name>` subcommand"
- "stub a new af command for X"
- "wire up `af project export` / `af signoff render` / etc."
- "contribute a CLI surface change"

Do NOT invoke for:

- editing/renaming an existing subcommand (use `af-cli-contract-guard` to plan that as a breaking change first)
- adding flags to an existing subcommand (different scope; smaller skill possible later)
- generating actual business logic (handler is a TODO stub; user fills it)

## Required inputs

1. **`namespace`** — top-level command. One of the existing groups (`core`, `manifest`, `project`, `architecture`, `resource`, `compatibility`, `constructor`, `signoff`, `dependency`, `registry`, `board`, `backend`, `evidence`, `vectors`, `wrapper`, `ci`, `tooling`, `self`) **or** the literal `top` for a new root subcommand.
2. **`name`** — the new subcommand name. Verilog-friendly snake-case is preferred (`verify`, `audit`, `migrate`); clap will convert to kebab-case for `--`-flags automatically.
3. **`description`** — one-line description (becomes clap `about` and the cli-reference.md row).
4. **Optional positional / flag specs** — e.g. `core_dir: PathBuf`, `--tier: String`. If omitted, skill stubs a minimal handler that takes no args beyond globals.

If `namespace` is occupied but `<namespace> <name>` is free, the skill extends the existing `<Namespace>Command` enum. If the namespace itself does not exist, the skill refuses with: "create the parent command group first, then re-invoke".

## Procedure

### Step 1 — collision check

```bash
cargo run --quiet -p af-cli --bin af -- --help 2>&1 | grep -E '^\s+(<namespace>|help)\s' || true
cargo run --quiet -p af-cli --bin af -- <namespace> --help 2>&1 | head -20
```

If `af <namespace> <name>` already exists in clap's help output, refuse: `"AF_CORE_NEW_SUBCOMMAND_TAKEN: af <namespace> <name> already registered"`. Do not overwrite.

If the namespace does not exist (`af --help` does not list it), refuse with the parent-first message.

### Step 2 — locate the four touch-points

Each touch-point is a precise edit, not a search-and-pray.

#### 2a. Clap enum (`crates/af-cli/src/main.rs`)

Find the `<Namespace>Command` enum:

```bash
grep -n '^enum <PascalCase namespace>Command' crates/af-cli/src/main.rs
```

For top-level subcommands, find `enum Commands` (line ~73).

Insert the new variant just before the closing brace. Use this template (with `<Name>` PascalCase and `name_snake` snake_case):

```rust
    <Name> {
        // TODO: positional / flag args
        // Example:
        // core_dir: PathBuf,
        // #[arg(long)]
        // tier: String,
    },
```

#### 2b. Dispatch (`crates/af-cli/src/main.rs`)

Find the existing `match` arm for this namespace group. Typically:

```rust
Commands::<Namespace> { command } => match command {
    <Namespace>Command::Existing { ... } => existing_handler(...),
    ...
}
```

Insert a new arm calling a placeholder handler:

```rust
    <Namespace>Command::<Name> { /* args */ } => <namespace>_<name_snake>(/* args */, &cli.build_root),
```

Define the handler stub near the other `<namespace>_*` functions:

```rust
fn <namespace>_<name_snake>(/* args */ _build_root: &Path) -> Result<CliOutput, CliError> {
    // TODO: implement
    Err(CliError::new(
        "AF_<NAMESPACE>_<NAME>_UNIMPLEMENTED",
        "af <namespace> <name>: handler is a TODO stub",
        "Implement the body of <namespace>_<name_snake> in crates/af-cli/src/main.rs.",
        1,
    ))
}
```

The error code follows the project convention `AF_<DOMAIN>_<CONDITION>` (rule #3 in CLAUDE.md). Use exit code 1 (generic) until the real semantics are known.

#### 2c. Lifecycle name (`crates/af-cli/src/main.rs`)

Find `<namespace>_command_name` (e.g. `core_command_name`, `manifest_command_name`):

```bash
grep -n 'fn <namespace>_command_name' crates/af-cli/src/main.rs
```

Add the new arm:

```rust
        <Namespace>Command::<Name> { .. } => "<name>",
```

(Use the same kebab-case form as the clap variant — for nested commands like `core registry list`, the lifecycle function returns `"registry list"`.)

#### 2d. CLI reference (`docs/cli-reference.md`)

Find the canonical command block:

```bash
grep -n '```bash' docs/cli-reference.md | head -3
```

Insert a new line in the block, alphabetically within the namespace group:

```
af <namespace> <name> [<args>]                # one-line description
```

If the description is non-trivial (more than a single flag), add a one-paragraph description after the table-style blocks, near the other `<namespace>` paragraphs.

#### 2e. Integration test (`crates/af-cli/tests/cli.rs`)

Append a test at the end of the file:

```rust
#[test]
fn <namespace>_<name_snake>_subcommand_is_registered() {
    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.args(["<namespace>", "<name>", "--help"])
        .assert()
        .success();
}

#[test]
fn <namespace>_<name_snake>_stub_returns_unimplemented_error() {
    // Until the handler is implemented, the stub returns
    // AF_<NAMESPACE>_<NAME>_UNIMPLEMENTED. Once implemented, replace
    // this test with real behavior coverage.
    let mut cmd = Command::cargo_bin("af").unwrap();
    cmd.args(["<namespace>", "<name>", "--json"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("AF_<NAMESPACE>_<NAME>_UNIMPLEMENTED"));
}
```

(Adjust the second test if the user specified args that change the failure path.)

### Step 3 — compile + clap-help smoke

```bash
cargo build -p af-cli
```

If build fails, surface the compiler error verbatim and abort. The most common cause is a typo in the namespace's PascalCase.

If build succeeds:

```bash
cargo run --quiet -p af-cli --bin af -- <namespace> <name> --help
```

This must succeed (exit 0 from clap-help). If clap rejects the subcommand, the enum variant was inserted incorrectly — surface the clap error.

### Step 4 — integration test smoke

```bash
cargo test -p af-cli --test cli <namespace>_<name_snake>_subcommand_is_registered
```

Must pass. If it does not, the dispatch arm or the lifecycle name function is missing the arm.

### Step 5 — clippy/fmt sanity

```bash
cargo fmt --all -- --check
cargo clippy -p af-cli -- -D warnings
```

If clippy flags the TODO stub (e.g. "unused variable"), address it by prefixing args with `_` (as in the template).

## Required output

```
## Added `af <namespace> <name>`

Four touch-points wired:

- `crates/af-cli/src/main.rs:<line>` — `<Namespace>Command::<Name>` variant
- `crates/af-cli/src/main.rs:<line>` — dispatch arm calling `<namespace>_<name_snake>`
- `crates/af-cli/src/main.rs:<line>` — `<namespace>_command_name` arm
- `docs/cli-reference.md:<line>` — command reference row
- `crates/af-cli/tests/cli.rs:<line>` — registration + stub tests

Handler stub at `crates/af-cli/src/main.rs:<line>`:
```rust
fn <namespace>_<name_snake>(...) -> Result<CliOutput, CliError> {
    Err(CliError::new("AF_<NAMESPACE>_<NAME>_UNIMPLEMENTED", ...))
}
```

## Smoke checks

- `cargo build -p af-cli` ✓
- `af <namespace> <name> --help` ✓
- `cargo test -p af-cli --test cli <test>` ✓
- `cargo fmt --check` ✓
- `cargo clippy -p af-cli -- -D warnings` ✓

## Next

Implement the body of `<namespace>_<name_snake>` in `crates/af-cli/src/main.rs`. After implementation, replace `<namespace>_<name_snake>_stub_returns_unimplemented_error` with real behavior coverage and add the appropriate `command_payload` variant in `crates/af-report/src/lib.rs` if the subcommand emits structured JSON.

If the subcommand's surface or output changes the contract (new flag default, new JSON shape, new error code), run `af-cli-contract-guard` before commit.
```

## Test Design Obligation

When this skill modifies `af`, it must add thoughtful tests for the touched
behavior. Cover success, failure, deterministic JSON/error output, and evidence
boundaries where applicable; if no direct test is possible, state the reason
and cite the closest existing coverage.

## Hard rules

- **Never touch an existing subcommand.** This skill only adds. Renames or removals are breaking and go through `af-cli-contract-guard`.
- **Never write business logic.** The handler stub is intentionally a TODO returning a 5-tuple error. The user implements the body.
- **Never invent a namespace.** If the parent (`af <namespace>`) does not exist, refuse and tell the user to create it first.
- **All four touch-points or none.** If any step fails (compile, clap-help, test), revert the patches you applied so the tree is left clean. Use `git diff` to inspect; if there are partial edits, `git checkout -p` selectively. Surface the failure clearly.
- **Error code follows the convention.** `AF_<NAMESPACE>_<NAME>_<CONDITION>`. Do not invent ad-hoc strings.
- **Stay within `af-cli`.** This skill does not touch other crates. If the subcommand needs new library logic in `af-manifest` or `af-report`, that is a separate change the user makes after the scaffold is in.
- **No mass refactoring.** If the namespace group is large and the dispatch `match` arm is messy, do not "clean it up" while inserting your variant. Add only.

## Edge cases

| Situation | Treatment |
|---|---|
| Subcommand needs sub-subcommands (e.g. `af core registry list`) | Two-step: first add the namespace (`af core registry`) with an empty inner enum, then re-invoke for each `list`/`add`/... |
| Subcommand needs `--json` global only — no args | Stub has empty args block; tests skip the `--json` test (or just test `--help`) |
| Description needs the registry / a board / a tier reference | Use the same reference language as the user request; do not invent new vocabulary |
| Namespace command does not have a `<Namespace>Command` enum (e.g. `af doctor` is flat) | Refuse: "doctor is a flat command; cannot add subcommands without restructuring". Suggest a new namespace instead. |
| Cargo build error suggests a renamed type | Surface and stop. Do not guess. |

## Example session

User: `add af core audit subcommand for read-only audit of a core`.

1. Collision check: `af core audit --help` returns "unrecognised subcommand". Free.
2. Locate enum: `CoreCommand` in `crates/af-cli/src/main.rs:~216`.
3. Insert variant `Audit { core_dir: PathBuf }`.
4. Insert dispatch arm `CoreCommand::Audit { core_dir } => core_audit(core_dir, &cli.build_root)`.
5. Define `core_audit(core_dir: &Path, _build_root: &Path)` returning `AF_CORE_AUDIT_UNIMPLEMENTED`.
6. Add `CoreCommand::Audit { .. } => "audit",` to `core_command_name`.
7. Add `af core audit <core_dir>                # read-only audit` to `docs/cli-reference.md`.
8. Add two integration tests in `crates/af-cli/tests/cli.rs`.
9. `cargo build -p af-cli` ✓.
10. `af core audit --help` ✓.
11. `cargo test core_audit_subcommand_is_registered` ✓.
12. `cargo fmt --check` ✓; `cargo clippy -p af-cli -- -D warnings` ✓.

Output as per template.
