# Wicket

Admissibility preflight for agentic operations. A small Rust CLI/library that
classifies one intended agentic operation, accounts for whether the cited
basis / precedence / standing are sufficient, and emits an immutable receipt.

> **Wicket may classify and gate. Wicket may not become the source of authority.**
>
> Wicket does not decide truth. Wicket decides whether an intended operation
> has accounted authority to proceed.
>
> **Caller cooks context. Wicket accounts for whether that cooked context is sufficient.**

Heavy at the mint. Light at the handle.

The full specification is [SPEC.md](./SPEC.md). Wicket is the embarrassingly
literal Rust translation of that spec; if the two disagree, SPEC.md is
authoritative and the code is the bug.

## What it is

- **Single-call.** One intent in, one verdict + receipt out. Stateless across
  calls.
- **Dimensional.** The verdict is a projection of three independently-evaluated
  dimensions: `basis × precedence × standing`. The flat surface is for boring
  consumers; doctrine lives in the dimensions.
- **Soft / hard split.** A missing-but-closable basis is `gap`
  (`OPEN_FINDING_ACCOUNTED`); a structurally inadmissible basis is `denied`.
  An open finding is admissible.
- **Caller-cooked.** Wicket does not resolve scope, evaluate globs, dereference
  receipts, or compute precedence. The caller cooks scope/precedence/revocation
  and Wicket accounts for those answers.
- **Receipts are content-addressed and immutable.** RFC 8785 canonical JSON,
  `sha256:` over the verdict body excluding `receipt_id`.

## What it is not

A platform. A workflow engine. A policy DSL. An MCP server. A multi-agent
orchestrator. A persistent store. A glob matcher. A precedence resolver. A
regime detector. A claim history tracker. See [SPEC.md §2](./SPEC.md).

## Build

```bash
cargo build --release
```

## CLI usage

Wicket has two modes: a **verb-shaped wrapper** (the handle most users want) and **raw `check`** (for fully-formed Intents).

### Verb-shaped wrapper

The wrapper gathers what it can from the filesystem (auto-detects `CLAUDE.md` /
`SPEC.md` / `README.md` as `policy_ref` evidence, hashes the target file as
`file_hash`, runs `git status --porcelain` for `command_output`) and gaps
honestly when context is missing. It never makes you assert the thing Wicket
is supposed to test.

```bash
# Filesystem edit
wicket edit SPEC.md --because "document the wrapper" --standing execute

# Command run (the command is described, not executed by Wicket)
wicket run "cargo test --release" --because "verify v0.1.1" --standing execute

# Git commit (default execute; --irreversible makes it bind)
wicket commit --because "checkpoint v0.1.1" --standing execute
wicket commit --because "publish release" --irreversible --standing execute \
    --human-confirm "release-mgr-2026-05-09"

# --brief: one-line verdict + top reason codes + receipt prefix
wicket edit SPEC.md --because "..." --standing execute --brief

# --show-intent: print the cooked Intent JSON instead of running check
wicket edit SPEC.md --because "..." --standing execute --show-intent
```

Wrapper flags shared across `edit` / `run` / `commit`:

| Flag | Default | Meaning |
| --- | --- | --- |
| `--because` | (required) | One-sentence rule the actor cites. |
| `--standing` | `recommend` | Actor's standing class. Pass `execute` if the operator authorized the agent to act. **Test/dev only**; use `--standing-grant` in production. |
| `--standing-grant <FILE>` | none | Path to a JSON grant ([SPEC §15.1](./SPEC.md)). When validated, the grant's standing class supersedes `--standing` and the grant is attached as `policy_ref` evidence. Invalid grants exit 65. |
| `--precedence` | `active` | Caller's precedence resolution. Use `unresolved` for genuine uncertainty. |
| `--human-confirm <token>` | none | Token identifying a fresh human approval. Required for `--irreversible` commits. |
| `--policy <PATH>` | auto | Additional policy doc paths beyond auto-detected `CLAUDE.md`/`SPEC.md`/`README.md`. |
| `--actor <id>` | `claude-code` | Override the actor identifier. |
| `--brief` | off | One-line summary instead of full JSON. |
| `--strict-exit` | off | Treat `denied`/`gap`/`unaccounted` as nonzero exit. |

### Raw `check`

For fully-formed Intent JSON (e.g., constructed by an upstream cooking layer
or replayed from a fixture):

```bash
wicket check --input intent.json
cat fixture.json | jq .intent | wicket check
wicket check --input intent.json --canonical          # RFC 8785 JSON
wicket check --input intent.json --strict-exit
```

### Exit codes ([SPEC §8.2](./SPEC.md))

| Exit | Meaning                                                             |
| ---- | ------------------------------------------------------------------- |
| 0    | `authorized` \| `advisory_only` \| `denied` \| `gap`                |
| 2    | `unaccounted` (model-unmappable; reserved provenance, etc.)         |
| 64   | Malformed input (schema validation failure)                         |
| 70   | Internal error                                                      |

`--strict-exit` makes `denied` / `gap` / `unaccounted` all nonzero.

## Library usage

```rust
use wicket::{check, Intent};

let intent: Intent = serde_json::from_str(raw_json)?;
let outcome = check(&intent);
println!("{}", serde_json::to_string_pretty(&outcome)?);
```

## Layout

```
wicket/
  SPEC.md                  # canonical specification
  Cargo.toml
  src/
    lib.rs                 # public API: check()
    main.rs                # CLI: check / edit / run / commit
    model.rs               # input/output types and enums
    rules.rs               # per-dimension derivation, sufficiency matrix
    verdict.rs             # surface verdict, allowed/forbidden, orchestrator
    receipt.rs             # RFC 8785 canonical JSON, sha256 hashing
    cook.rs                # thin adapter: verb packets → Intent
  schemas/
    intent.schema.json
    verdict.schema.json
  cases/                   # fixture-driven doctrine tests
    positive/
    laundering/
    stale_revoked/
    evidence/
    unaccounted/
    dogfood/               # captured from real wrapper invocations
    documentation_only/    # not executed by CI; marks boundaries
  skills/
    preflight.md           # how an agent should call wicket
  tests/
    fixtures.rs            # walks cases/ and asserts expected verdicts
```

## Standing grants

A standing grant is a JSON document binding an issuer to an actor's standing
class over a scope root, set of operation verbs, and time window. The wrapper
validates the grant before constructing an Intent; invalid grants are refused
at the wrapper layer (exit 65) without invoking the kernel. See
[SPEC §15.1](./SPEC.md) for the full grant doctrine and `examples/grants/`
for a sample.

```bash
wicket edit SPEC.md \
  --because "document grant flow" \
  --standing-grant examples/grants/sample-grant.json
```

Wrapper-level grant rejection reasons (printed to stderr, exit 65):

- `GRANT_SCHEMA_MISMATCH` — file does not declare `wicket-grant/v1`.
- `GRANT_ACTOR_MISMATCH` — grant `actor` ≠ `--actor`.
- `GRANT_OPERATION_NOT_PERMITTED` — verb not in grant `operations`.
- `GRANT_OUT_OF_SCOPE` — target is not under grant `scope_root`.
- `GRANT_NOT_YET_VALID` / `GRANT_EXPIRED` — `now` outside the grant window.
- `GRANT_UNPARSEABLE_TIMESTAMP` — `issued_at` or `expires_at` malformed.

Grants are a **wrapper convention**, not kernel doctrine. The kernel never
sees a grant directly — it only sees the resulting Intent with the grant
attached as a `policy_ref` evidence ref.

## Tests

```bash
cargo test
```

The fixture runner (2 integration tests, both green) walks `cases/`, skips
the `documentation_only/` bucket, and asserts each fixture's `expected`
block matches `wicket::check`'s output. Receipts are also asserted to be
content-addressed and stable across repeated calls.

Fixture inventory (current):

- 4 `positive/`
- 4 `laundering/`
- 2 `stale_revoked/`
- 3 `evidence/`
- 1 `unaccounted/`
- 4 `dogfood/` (captured from real wrapper invocations)
- 3 `documentation_only/` (boundary fixtures; not executed)

That's **18 executable fixtures + 3 documentation-only = 21 total** as of
this writing.

## AG / Wicket reproducibility

A long-term contract, not a v1 promise (see [SPEC §11.5](./SPEC.md)):

> For every AG gate-receipt class, there should be a corresponding Wicket
> fixture, or a documented reason Wicket cannot express it.

Wicket is small enough to be read for doctrine; AG is too large. That
asymmetry is the reason the contract is useful — Wicket can be the
ratification surface AG tools cite when arguing their gate machinery is
doctrinally clean.

## Doctrine

See [SPEC.md §3](./SPEC.md). Eight load-bearing lines. Don't add a ninth.

## Relationship to other projects

- `nlai` — invariant kernel: language is a proposal, not authority.
- `wicket` — admissibility preflight surface (this repo).
- `agent_governor` — research / constitutional substrate.
- `~/git/lean/LeanProofs/Admissibility/` — formal substrate; Wicket fixtures
  should be reproducible by the kernel.

## License

Apache-2.0. See [LICENSE](./LICENSE) and [NOTICE](./NOTICE).
