# Skill — calling Wicket for admissibility preflight

**Use this skill when:** you are about to perform an action that touches state
beyond reading — file mutation, deploy, delete, commit, push, irreversible
commit, durable memory write, policy mutation. Or when you are unsure whether
an action is admissible under the current basis.

**Do not use this skill for:** trivially read-only operations the user has
clearly authorized in the current turn. Wicket is preflight, not paranoia.

## What Wicket gives you

```
authorized | advisory_only | denied | gap | unaccounted
+ dimensional accounting (basis × precedence × standing)
+ allowed / forbidden actions
+ reason codes
+ a hash-chained receipt
```

It does not perform the action. It gates whether you should.

## Inputs you must supply

You are responsible for **cooking** the context. Wicket does not resolve
scope, dereference receipts, or compute precedence; it only accounts for the
cooked answer.

| Field | Source |
| --- | --- |
| `actor` | constant: your agent identifier |
| `actor_standing.class` | what authority class you hold for this op? |
| `actor_standing.provenance` | `caller_asserted` in v1 |
| `intended_action` | verb-shaped, e.g. `git.commit` |
| `operation_class` | `observe` \| `interpret` \| `recommend` \| `authorize` \| `execute` \| `bind` |
| `target` | path / repo / object |
| `scope_assertion.scope_includes_target` | does the actor's scope cover this target? |
| `claimed_basis.rule` | one concrete sentence — the rule you cite |
| `claimed_basis.evidence_refs` | structured pointers (prompt / policy_ref / file_hash / human_confirmation / tool_trace / etc.) |
| `precedence.resolution` | `active` \| `superseded` \| `ambiguous` \| `unresolved` |
| `revocation.basis_revoked` | did a prior receipt or policy revoke this basis? |
| `revocation.standing_forbidden` | did a prior receipt or policy explicitly forbid this standing here? |
| `expected_effect` | one sentence describing the effect |
| `call_timestamp` | ISO-8601 UTC; CLI defaults from system time |

## How to read the verdict

| Surface | Meaning | What to do |
| --- | --- | --- |
| `authorized` | All three dimensions satisfied. | Proceed. Honor `forbidden` (usually empty). |
| `advisory_only` | Recommend-class operation, admissible. | Produce the recommendation. **Do not execute it.** |
| `denied` | A hard failure. | Stop. Read `forbidden` and `reason_codes`. The `allowed` list names a legitimate downgrade or remediation. |
| `gap` | An open finding — soft insufficiency. | Stop, but admissibly. Read `allowed` to learn what evidence would close the gap. Do not launder a gap into authorization. |
| `unaccounted` | Wicket could not classify the input. | Treat as a bug or boundary. Do not proceed. |

## Doctrine you must honor

1. **`openFinding` is admissible.** A `gap` is not failure; it is Wicket
   accounting for what is missing. Do not retry as if you got a `denied`.
2. **`unaccounted` is failure.** Treat it as a bug, not a verdict you can act on.
3. **Recommendation never authorizes execution.** If you got `advisory_only`
   or a `denied` with `propose_recommendation` in `allowed`, that is the
   downgrade — share the recommendation; do not perform the action.
4. **Self-certification is inadmissible.** Do not cite your own assertions
   as evidence. Wicket will mark such bases `inadmissible` → `denied`.
5. **Receipts are immutable.** A new evidence packet produces a new receipt.
   Carry the prior `receipt_id` as `prev_receipt_hash` if relevant.
6. **Caller-asserted provenance is unverified.** Every v1 verdict carries
   `*_CALLER_ASSERTED_UNVERIFIED` codes. The downstream auditor reads them.

## Calling Wicket

**Prefer the verb-shaped wrapper.** It infers from the filesystem what it
can, gaps honestly when it can't, and keeps you out of hand-authoring
30-line Intent JSONs.

```bash
# Common cases:
wicket edit SPEC.md --because "document the wrapper" --standing execute --brief
wicket run "cargo test" --because "verify changes" --standing execute --brief
wicket commit --because "checkpoint" --standing execute --brief

# Irreversible commit (publishing a release tag, force-pushing):
wicket commit --irreversible --standing execute \
  --human-confirm "release-mgr-token" --because "publish v0.2.0"

# Default standing is `recommend` — most operations will be DENIED until you
# pass --standing execute (or set up a higher-trust integration).
```

The wrapper's exit code follows §8.2 (0 for any verdict, 2 for unaccounted,
64 for malformed input). Pair with `--strict-exit` for CI gates that
should fail on `denied`/`gap`/`unaccounted`.

### Raw mode (for upstream cooks)

If a higher-trust adapter (AG, an MCP server, a harness) cooks the Intent
itself, pipe it directly:

```bash
echo '{...intent...}' | wicket check --brief
```

For library use:

```rust
let outcome = wicket::check(&intent);
match outcome.surface_verdict {
    wicket::SurfaceVerdict::Authorized => proceed(),
    wicket::SurfaceVerdict::AdvisoryOnly => share_recommendation(),
    wicket::SurfaceVerdict::Gap => stop_and_address_open_finding(&outcome),
    wicket::SurfaceVerdict::Denied => stop(),
    wicket::SurfaceVerdict::Unaccounted => raise_bug(),
}
```

### Standing grants (production path)

Raw `--standing execute` is **test/dev only**. Every receipt that consumes
it carries `STANDING_CALLER_ASSERTED_UNVERIFIED` so the smell is visible —
but a downstream auditor that ignores that code is fooled.

In production, an integrator establishes a structured **standing grant**
(see [SPEC §15.1](../SPEC.md)). The wrapper validates the grant against
the requested invocation (actor, scope, operation, freshness) and:

- on success → elevates `actor_standing.class`, attaches the grant as
  `policy_ref` evidence so the receipt records what authorized this call.
- on failure → exits 65 with a clear stderr reason; **no Intent is
  constructed and no kernel receipt is emitted**. This is wrapper-level
  refusal, not a denied kernel verdict.

```bash
wicket edit SPEC.md \
  --because "..." \
  --standing-grant /path/to/grant.json
```

Sample grant: [`examples/grants/sample-grant.json`](../examples/grants/sample-grant.json).

### When the wrapper is not enough

The wrapper deliberately can't infer:

- The actor's standing (defaults to `recommend`; either supply
  `--standing-grant` in production or `--standing execute` for test/dev).
- Whether a fresh human approval exists (pass `--human-confirm <token>`
  when one does).
- Real precedence resolution (defaults to `active`; pass
  `--precedence unresolved` when you genuinely don't know).
- Anything outside `cwd` or its policy docs.

When you need finer control than the wrapper offers, build the Intent
yourself and feed it to `wicket check`.

## Anti-patterns

- **Do not** retry a `gap` as a different operation_class hoping for
  `authorized`. The gap names what is missing; supply that, then retry.
- **Do not** treat a `denied` with `propose_recommendation` in `allowed` as
  permission to execute under "advisory_only" framing. The verdict is denied.
- **Do not** mix self-certified evidence with legitimate evidence. Any
  self-cert ref poisons the basis.
- **Do not** mark `bind` as `execute` to dodge the human-confirmation rule.
  Wicket cannot detect this from input alone — it is documented as a boundary
  in `cases/documentation_only/silent_downgrade_without_waiver.json`.
