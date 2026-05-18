# Wicket Specification

**Status:** v0.3 draft, 2026-05-09. Implementation-ready pending freeze. The spec leads; the code follows literally.

> Wicket does not decide truth. Wicket decides whether an intended operation has accounted authority to proceed.
>
> **Caller cooks context. Wicket accounts for whether that cooked context is sufficient.**

Changes from v0.2: basis status splits into soft (`insufficient`/`stale`/`absent`/`ambiguous` → gap) and hard (`revoked`/`inadmissible` → denied), resolving the openFinding-vs-§7.3 conflict; `inadmissible` added as a basis status for self-certification and bind-without-human-confirmation; top-level `revocation` field added so Wicket can derive `basis.revoked` and `standing.forbidden` from caller-cooked context without dereferencing receipts or policy; §7.3 renumbered to §7.3; §10 fixture #12 renamed `bind_without_supporting_evidence`; §7 adds explicit caller-asserted-authorized warning; reserved-provenance behavior made explicit (schema-allowed → model-unaccounted, exit 2); `evidence_ref_hashes` renamed `evidence_ref_hashes`; reason-code registry updated for soft/hard split.

Changes from v0.1: `scope_assertion` lifted to a top-level structured field with its own provenance and `evidence_refs`; `precedence` gains `evidence_refs` and the `unresolved` resolution value; reason codes `ACCOUNTING_*` renamed to `OPEN_FINDING_ACCOUNTED` / `UNACCOUNTED_INPUT`; `STANDING_SCOPE_NOT_ASSERTED` split into `SCOPE_NOT_ASSERTED` / `SCOPE_CALLER_ASSERTED_UNVERIFIED`; `PRECEDENCE_UNRESOLVED` added; precedence input value renamed `satisfied` → `active`.

Changes from v0: input model adds `call_timestamp`, structured `actor_standing`; caller-asserted scope and precedence; standing ladder decoupled with v1-debt; basis sufficiency matrix added; `advisory_only` doctrine sharpened; reason-code registry added; exit-code policy fixed; canonicalization named (RFC 8785); evidence kinds expanded with issuer/subject.

---

## 1. Purpose

Wicket is a small Rust CLI/library that performs **admissibility preflight** for a single intended agentic operation. Given a structured intent and the evidence the actor cites for it, Wicket returns a bounded verdict, the dimensional accounting that produced that verdict, the actions the actor may and may not take next, and a receipt obligation.

Wicket is the **handle**. It is called once per intended operation. It is stateless across calls. It does not orchestrate, schedule, store, route, or interpret prose authority. **Wicket is a verifier, not a resolver.** Callers cook scope and precedence; Wicket reads the cooked answer and verifies it against the operation.

Wicket sits between two existing projects:

- `nlai` — invariant kernel: language is a proposal, not authority.
- `agent_governor` (a.k.a. AG) — operational mint: full authority/regime/composition machinery.

```
nlai          → invariant kernel
wicket        → admissibility preflight surface (this project)
agent_gov     → research / constitutional substrate
LeanProofs    → formal substrate (Authority/StateTransition/Derivation/Execution/Corrective)
```

**Keeper constraint: heavy at the mint, light at the handle.**

---

## 2. Non-goals

Wicket is explicitly **not**:

- A platform, framework, or runtime.
- A workflow engine or task scheduler.
- A policy DSL, rule engine, or expression language.
- A path-glob or scope evaluator. (Caller asserts whether target is in scope.)
- A precedence resolver. (Caller asserts which rule prevails.)
- An MCP server (a thin MCP shim may exist later; it is not core).
- A multi-agent orchestrator or lease manager.
- A persistent store (no DB, no WAL, no journal).
- A dashboard, UI, or web service.
- A plugin host or extensibility framework.
- A regime detector or adaptive controller (that is AG).
- A claim history / vocabulary drift tracker (that is AG).

If a feature pushes Wicket past **~5k LOC excluding fixtures**, it is the wrong feature.

---

## 3. Core doctrine

Eight load-bearing lines. These are the only doctrine Wicket asserts; everything else is implementation.

1. **Verdict is dimensional, not atomic.** Authorized iff `Basis × Precedence × Standing` are all satisfied. The flat surface verdict is a derived projection.
2. **`openFinding` is admissible.** Refusal to launder uncertainty into closure is an admissible outcome. The failure mode is `unaccounted`: silent omission, or inability to account for the case under the model.
3. **Receipts are immutable.** Disputes produce new receipts. Wicket does not edit, replace, or delete receipts.
4. **Policy gaps do not mutate policy.** Absence of a contradicting rule is not permission. The trapdoor: only an explicit policy-amendment operation may modify policy.
5. **Same-basis corrective paths cannot launder non-authorization into authorization.** Re-entry through a fresh basis K′ via a forward operation is the legitimate path. (`Admissibility/Corrective.lean :: corrective_no_authority_laundering`.)
6. **Revoked or stale basis cannot authorize irreversible action.** Severity scales with operation class; `bind` is the strict boundary. (`Admissibility/Execution.lean :: revoked_basis_cannot_be_authorized_step`.)
7. **Recommendation does not authorize execution.** A recommend-class operation is admissible at recommend-class standing; an execute-class operation requires execute-class standing. The downgrade path ("you may instead recommend") lives in `allowed`, not in the surface verdict.
8. **Wicket may classify and gate. Wicket may not become the source of authority.** Authority lives in the substrate, the docs, the policy, the human, and the tool evidence. Wicket is the bouncer, not the legislature.

---

## 4. Input model

Wicket consumes a single JSON object describing one intended operation.

```jsonc
{
  "actor": "claude-code",                  // identifier of the requesting actor
  "actor_standing": {
    "class": "interpret",                  // see §6.3
    "provenance": "caller_asserted"        // §4.2
  },
  "intended_action": "git.commit",         // verb-shaped action name
  "operation_class": "execute",            // see §5
  "target": "docs/gaps/COVERAGE.md",       // object of the action
  "scope_assertion": {
    "scope_includes_target": true,         // caller-cooked; §4.6
    "provenance": "caller_asserted",
    "evidence_refs": ["policy://repo/CLAUDE.md#write-scope"]
  },
  "claimed_basis": {
    "rule": "user requested edit in current session",
    "evidence_refs": [
      {
        "ref": "session://prompt#current",
        "kind": "prompt",
        "issuer": "user",                  // who produced this evidence
        "subject": "claude-code",          // who the evidence is about/for
        "valid_from": "2026-05-09T22:00:00Z",
        "valid_until": "2026-05-09T23:00:00Z",
        "status": "valid"
      }
    ]
  },
  "precedence": {
    "resolution": "active",                // active | superseded | ambiguous | unresolved
    "superseded_by": null,                 // optional rule ref when superseded
    "provenance": "caller_asserted",
    "evidence_refs": ["policy://repo/SPEC.md#v1"]
  },
  "revocation": {
    "basis_revoked": false,                // caller-cooked; §4.8
    "standing_forbidden": false,           // caller-cooked; §4.8
    "provenance": "caller_asserted",
    "evidence_refs": []                    // refs to receipts/policies that establish revocation, if any
  },
  "expected_effect": "modify repository file at target path",
  "call_timestamp": "2026-05-09T23:14:02Z",
  "prev_receipt_hash": null                // optional; chains receipts within a session
}
```

### 4.1 Required fields

`actor`, `actor_standing`, `intended_action`, `operation_class`, `target`, `claimed_basis`, `precedence`, `revocation`, `expected_effect`, `call_timestamp` are required. `scope_assertion` and `prev_receipt_hash` are optional. The `revocation` field may carry `basis_revoked: false, standing_forbidden: false` (the common case); it is required so the caller cannot omit the question. `scope_assertion` is optional because a caller (especially a thin adapter / wrapper) may not have enough information to assert scope; in that case Wicket emits `SCOPE_NOT_ASSERTED` and downgrades the verdict (see §4.6).

### 4.2 Provenance fields

`actor_standing.provenance ∈ { caller_asserted, attested, verified }`. V1 supports `caller_asserted` only and emits the reason code `STANDING_CALLER_ASSERTED_UNVERIFIED` whenever it is consumed. `attested` and `verified` are **schema-valid but model-unaccounted** in v1: the JSON schema accepts them, the model returns `surface_verdict: unaccounted` with `INPUT_FUTURE_PROVENANCE_RESERVED`, and the CLI exits 2. They are not schema-rejected (exit 64). This makes the future-provenance boundary visible without requiring a schema change to surface it.

`precedence.provenance`, `scope_assertion.provenance`, and `revocation.provenance` follow the same rule, emitting `PRECEDENCE_CALLER_ASSERTED_UNVERIFIED`, `SCOPE_CALLER_ASSERTED_UNVERIFIED`, and `REVOCATION_CALLER_ASSERTED_UNVERIFIED` respectively. Wicket does not verify any of these in v1; it accounts for the fact that they are unverified.

### 4.3 Evidence kinds

```
evidence_refs[].kind ∈ {
  prompt,               // session prompt or instruction
  file_hash,            // pinned content of a file at a known hash
  test_log,             // test runner output
  tool_output,          // structured output of a named tool
  tool_trace,           // sequence/trace of tool calls (for traceability)
  command_output,       // shell/CLI output captured verbatim
  policy_ref,           // pointer to a durable policy/rule
  prior_receipt,        // a previously-emitted Wicket or AG receipt
  human_confirmation,   // explicit human approval, freshly produced
  actor_assertion       // a claim by the actor itself; see §4.4
}
```

Unknown kinds → `basis: ambiguous`.

### 4.4 Self-certification

When `evidence_refs[].issuer == evidence_refs[].subject == actor`, the evidence is **self-certifying**. In v1, **any** self-certified ref in `evidence_refs` makes the entire basis `inadmissible` — Wicket emits `BASIS_INADMISSIBLE_SELF_CERTIFIED` and the surface verdict is `denied`. This is a hard rule: an actor that mixes self-claims with legitimate evidence poisons the basis, and is told to remove the self-cert ref and resubmit. The doctrinal reason: legitimate evidence should not be diluted by self-certification, and Wicket cannot tell which refs the downstream enforcer would weight.

The previous v0.2 stance (self-cert disregarded but not failing) is recorded as resolved in §13.10.

### 4.5 Time

**Time enters as evidence, not ambient reality.**

The kernel must not consult wall-clock time. All temporal evaluation runs over caller-supplied fields:

- `call_timestamp` (Intent) — the evaluation time for this preflight.
- `valid_from` / `valid_until` (Evidence) — freshness window for each evidence ref.
- `issued_at` / `expires_at` (grant; §15.1) — window during which a grant is honored.
- `revocation.evidence_refs` — pointers to receipts/policies that established revocation; the kernel does not dereference them.

Timestamps arrive as RFC 3339 strings and are parsed to typed `DateTime<Utc>` at the kernel boundary (`parse_ts` in `rules.rs`, `parse_iso` in `grant.rs`). Comparisons run over the parsed typed values; the kernel does not perform string ordering on raw timestamp fields. Unparseable timestamps in the Intent route to `unaccounted` (exit 2); unparseable timestamps in Evidence disqualify that evidence ref.

The wrapper, CLI, and cook layers (`cook.rs`, `main.rs`) may stamp time from the system clock when populating Intent and Evidence fields. The kernel modules (`lib.rs`, `model.rs`, `rules.rs`, `grant.rs`, `verdict.rs`, `receipt.rs`) **must not** call `Utc::now`, `Local::now`, `SystemTime::now`, `Instant::now`, `OffsetDateTime::now_utc`, or any equivalent ambient-clock function. This is enforced by `tests/kernel_atemporality.rs`.

**Tripwires**

- A receipt's timestamp records evaluation time supplied by the cook layer; it cannot launder artifact time.
- Evidence freshness is evaluated against `call_timestamp`, not ambient `now()` — the same Intent produces the same verdict regardless of when the kernel re-evaluates it.
- Future-dated revocation does not affect present standing; the kernel reads `revocation.basis_revoked` as a boolean the caller cooked at evaluation time, not by dereferencing an `effective_at` field.

The CLI populates `call_timestamp` from system time at process start unless `--call-timestamp` is supplied; tests pass it explicitly.

### 4.6 Scope

`scope_assertion` is **caller-cooked**, not Wicket-resolved. Wicket does not match paths, evaluate globs, interpret scope grammars, or read repo ownership maps. The caller supplies a structured assertion with `scope_includes_target: bool`, `provenance`, and supporting `evidence_refs`. If the caller asserts `false`, standing dimension is `out_of_scope`. If `scope_assertion` is omitted entirely, Wicket emits `SCOPE_NOT_ASSERTED`, which downgrades the verdict for any non-observe operation.

The `evidence_refs` are pointers (e.g. `policy://repo/CLAUDE.md#write-scope`); Wicket records them in the receipt but does not dereference or validate them.

### 4.7 Precedence

`precedence.resolution` is **caller-cooked**. Wicket does not enumerate applicable rules or compute precedence. The caller supplies a resolution from `{ active, superseded, ambiguous, unresolved }`, optional `superseded_by`, `provenance`, and supporting `evidence_refs`. Wicket reads the resolution and accounts for it.

- `active` — the cited basis is the current governing rule.
- `superseded` — a later rule explicitly displaces it.
- `ambiguous` — multiple rules apply and no resolution order exists.
- `unresolved` — caller has not yet done the work to resolve precedence (distinct from inherently `ambiguous`; emits a different reason code but collapses to the same dimension status).

### 4.8 Revocation

`revocation` is **caller-cooked**. Wicket does not dereference `prior_receipt` or `policy_ref` evidence to discover whether a basis has been revoked or a standing forbidden — the caller resolves that and supplies the answer. The field is two booleans plus provenance and evidence pointers:

```jsonc
"revocation": {
  "basis_revoked": false,         // does any prior receipt or policy revoke the cited basis?
  "standing_forbidden": false,    // does any prior receipt or policy explicitly forbid this standing on this target?
  "provenance": "caller_asserted",
  "evidence_refs": []             // refs to the receipts/policies that establish revocation, if true
}
```

If `basis_revoked: true`, basis dimension is `revoked` (hard → denied). If `standing_forbidden: true`, standing dimension is `forbidden` (hard → denied). The `evidence_refs` are recorded in the receipt so a downstream auditor can trace the claim.

`revocation` is required so the caller cannot omit the question. The common case is two `false` values with empty `evidence_refs`; that is fine and emits no revocation reason code.

---

## 5. Operation classes

Six values. Each implies a minimum required standing and a different receipt obligation.

| Class       | Meaning                                                  | Min standing (v1) | Reversible? |
| ----------- | -------------------------------------------------------- | ----------------- | ----------- |
| `observe`   | Read-only inspection of state.                           | observe           | n/a         |
| `interpret` | Analysis or summarization over observed material.        | interpret         | n/a         |
| `recommend` | Propose action for human or downstream consideration.    | recommend         | n/a         |
| `authorize` | Grant permission for a downstream action to proceed.     | authorize         | yes         |
| `execute`   | Perform a state-mutating action with a recovery path.    | execute           | yes         |
| `bind`      | Irreversible commitment (delete, deploy, merge, sign).   | execute + fresh `human_confirmation` (§7.3) | **no** |

### 5.1 V1 standing-ladder debt

V1 treats standings as a strict total order:

```
observe < interpret < recommend < authorize < execute
```

This is **acknowledged debt**. In reality, `authorize` and `execute` are adjacent powers, not a ladder: an actor may be permitted to approve without executing, or to execute only under prior approval. The total-order assumption will not survive contact with a real authority model and is flagged as Open Question §13.7. Until resolved, every Wicket call where `operation_class ∈ {authorize, execute}` emits `STANDING_LADDER_V1_FLAT` to make the debt visible in receipts.

Operation class is supplied by the caller; Wicket does not infer it. Mislabeling (`bind` declared as `execute`) is undetectable from input alone — it is the `silent_downgrade` documentation fixture (§10.2), not an executable check.

---

## 6. Verdict dimensions

A verdict is the conjunction of three independently-evaluated dimensions. Each dimension carries a `DimensionStatus`. The flat surface verdict in §7 is derived from this triple.

### 6.1 Basis

> Does the actor have a current, non-revoked, non-stale, evidence-grounded reason to believe it may act?

Basis statuses partition into **soft** (closable; surface verdict `gap`) and **hard** (non-closable from this call; surface verdict `denied`):

```
basis.status ∈ {
  satisfied,       // rule + evidence sufficient for the operation class (§7.3)
  // SOFT — surface = gap; actor can address by supplying more/fresher evidence
  insufficient,    // rule + some evidence but not enough for the operation class
  stale,           // evidence past valid_until at call_timestamp
  absent,          // no basis cited at all
  ambiguous,       // basis cited but Wicket cannot map evidence to known kinds
  // HARD — surface = denied; not closable by adding more evidence
  inadmissible,    // structurally bad basis (self-cert sole, bind without human_confirmation)
  revoked          // caller-cooked revocation = true (§4.8)
}
```

The soft/hard split is the doctrinal hinge that lets `openFinding` (gap) coexist with `inadmissible evidence` (denied) without contradiction. See §7.1.

### 6.2 Precedence

> Among rules and prior decisions that bear on this operation, does the cited basis hold?

```
precedence.status ∈ {
  satisfied,       // caller asserted resolution = active
  superseded,      // caller asserted resolution = superseded
  ambiguous        // caller asserted resolution = ambiguous OR unresolved
}
```

Caller-input mapping:

| Caller `precedence.resolution` | Dimension status | Reason code              |
| ------------------------------ | ---------------- | ------------------------ |
| `active`                       | `satisfied`      | `PRECEDENCE_OK`          |
| `superseded`                   | `superseded`     | `PRECEDENCE_SUPERSEDED`  |
| `ambiguous`                    | `ambiguous`      | `PRECEDENCE_AMBIGUOUS`   |
| `unresolved`                   | `ambiguous`      | `PRECEDENCE_UNRESOLVED`  |

V1 has no `insufficient` here: the caller resolves precedence. `unresolved` collapses to `ambiguous` for surface derivation but emits a distinguishable reason code so receipts record the difference.

### 6.3 Standing

> Does the actor hold the authority class required by this operation, over this target?

```
standing.status ∈ {
  satisfied,       // class >= min_standing(operation_class) ∧ scope_includes_target ∧ provenance acceptable
  insufficient,    // class < min_standing(operation_class)
  absent,          // no standing claim at all
  out_of_scope,    // class adequate but caller asserts target outside scope
  forbidden        // standing explicitly denied for this operation+target by prior receipt
}
```

`forbidden` is sourced from prior `prior_receipt` evidence with explicit denial; Wicket does not invent it.

---

## 7. Surface verdict derivation

The surface verdict is a deterministic projection of the three dimensions. It exists for boring downstream consumers; doctrine lives in the dimensions.

```
surface_verdict ∈ {
  authorized,      // basis ∧ precedence ∧ standing all satisfied AND §7.3 sufficiency holds
  advisory_only,   // operation_class = recommend AND verdict admissible
  denied,          // any dimension in a hard-rejection state (see §7.1)
  gap,             // all dimensions accountable; one or more in a soft-rejection state
  unaccounted      // class:error — input did not map to the model
}
```

**Doctrinal note on `advisory_only`:** A recommend-class operation that meets the recommend-class bar is `advisory_only`. It is not `authorized` because authorization is a stronger claim than admissibility-of-recommendation. It is not `denied` because the recommendation is admissible. The "you may instead draft a recommendation" downgrade for higher operation classes is **not** `advisory_only` — that is `denied` with `propose_recommendation` in `allowed`.

**Doctrinal note on `authorized`:** In v1, `authorized` means **authorized under caller-supplied scope, precedence, standing, revocation, and evidence context.** It does **not** mean Wicket independently verified authority. The provenance-unverified reason codes (`STANDING_CALLER_ASSERTED_UNVERIFIED`, etc.) are present in every v1 verdict that consumes a caller-cooked field; they are the receipt-level signal that authority came from the wicket window, not the mint.

### 7.1 Derivation table

Order of evaluation: **`unaccounted` > `denied` > `advisory_only` > `gap` > `authorized`.** First match wins.

| Condition                                                                            | Surface       |
| ------------------------------------------------------------------------------------ | ------------- |
| Any field rejected as unmappable (unknown enum, malformed input, future provenance)  | `unaccounted` |
| `basis.status ∈ { revoked, inadmissible }` (HARD basis failures)                     | `denied`      |
| `precedence.status = superseded`                                                     | `denied`      |
| `standing.status ∈ { absent, forbidden, out_of_scope }`                              | `denied`      |
| `standing.status = insufficient` (any operation class)                               | `denied`      |
| `operation_class = recommend` ∧ all three dimensions otherwise satisfied             | `advisory_only` |
| `basis.status ∈ { insufficient, stale, absent, ambiguous }` (SOFT) or `precedence.status = ambiguous` | `gap`     |
| All three `satisfied` ∧ §7.3 sufficiency holds                                       | `authorized`  |

**Soft vs hard:** The §7.3 sufficiency matrix produces either soft (`insufficient`) or hard (`inadmissible`) basis statuses, and the derivation table routes them differently. Missing-but-closable evidence is `insufficient` → `gap`. Structurally inadmissible bases (self-cert sole, bind without human_confirmation) are `inadmissible` → `denied`. This is the conflict resolution from v0.2: the positive `open_finding_is_admissible` fixture is `gap` because its basis is soft-`insufficient`, not hard-`inadmissible`.

`gap` is doctrinally an **`openFinding`**: Wicket can fully account for why it cannot authorize, and the actor can address the missing piece. `gap` is not failure. **`surface_verdict = gap` IS the openFinding representation;** there is no separate accounting field. Every `gap` verdict carries the reason code `OPEN_FINDING_ACCOUNTED` so the doctrinal claim is machine-readable.

`unaccounted` is the only error-class verdict: Wicket received input it could not classify. The output JSON tags it `class: "error"`; the other four are `class: "verdict"`. Every `unaccounted` verdict carries the reason code `UNACCOUNTED_INPUT`.

### 7.2 Why standing-insufficient is always `denied`

In v0 this was conditionally `advisory_only` for low operation classes. That was wrong: insufficient standing for any operation is a non-grant. The downgrade option (recommend instead) is a follow-up the actor may take, recorded in `allowed`. The verdict stays about the requested operation.

---

### 7.3 Basis sufficiency by operation class

Even when basis evidence is structurally present, **what counts as sufficient** depends on the operation class. This is the V1 sufficiency matrix. It is small and explicit; sharper cuts are deferred.

The matrix produces a soft outcome (`basis.status = insufficient` → gap) **except** where a row marks a requirement as **hard** (`basis.status = inadmissible` → denied).

| Operation class | Required evidence kinds (any of, unless noted)                                                                       | Hard rules                              |
| --------------- | -------------------------------------------------------------------------------------------------------------------- | --------------------------------------- |
| `observe`       | `prompt` ∨ `policy_ref` ∨ `file_hash`                                                                                | none                                    |
| `interpret`     | (`prompt` ∨ `policy_ref`) + a target reference                                                                       | none                                    |
| `recommend`     | (`prompt` ∨ `human_confirmation` ∨ `policy_ref`) + a target reference                                                | none                                    |
| `authorize`     | `policy_ref` ∨ `human_confirmation`                                                                                  | none                                    |
| `execute`       | (`policy_ref` ∨ `human_confirmation`) + at least one of (`tool_trace` ∨ `test_log` ∨ `command_output` ∨ `file_hash`) | none                                    |
| `bind`          | fresh `human_confirmation` (within 60 minutes of `call_timestamp`) + at least one *supporting evidence ref* (any of `tool_trace` ∨ `test_log` ∨ `command_output` ∨ `file_hash` ∨ `prior_receipt`), all non-stale | **hard**: missing `human_confirmation` → `inadmissible` |

**Self-certified evidence (§4.4) is `inadmissible` (hard).** Any self-cert ref in `evidence_refs` makes the basis `inadmissible` regardless of which row applies.

When a row is unmet by missing-but-supplyable evidence, basis is `insufficient` and the surface verdict is `gap` (with reason codes like `BASIS_INSUFFICIENT_FOR_EXECUTE`). When a row's hard rule is violated, basis is `inadmissible` and the surface verdict is `denied` (with reason codes like `BASIS_INADMISSIBLE_BIND_REQUIRES_HUMAN_CONFIRMATION`).

The matrix is V1 doctrine, not eternal truth. Open Question §13.9 tracks the next refinement.

---

## 8. Output model

```jsonc
{
  "class": "verdict",                      // "verdict" | "error"
  "surface_verdict": "gap",
  "operation_class": "execute",
  "dimensions": {
    "basis":      { "status": "stale",     "reason_codes": ["BASIS_TTL_EXPIRED"] },
    "precedence": { "status": "satisfied", "reason_codes": ["PRECEDENCE_OK", "PRECEDENCE_CALLER_ASSERTED_UNVERIFIED"] },
    "standing":   { "status": "satisfied", "reason_codes": ["STANDING_OK", "STANDING_CALLER_ASSERTED_UNVERIFIED", "SCOPE_CALLER_ASSERTED_UNVERIFIED"] }
  },
  "reason_codes": [
    "BASIS_TTL_EXPIRED",
    "STANDING_CALLER_ASSERTED_UNVERIFIED",
    "PRECEDENCE_CALLER_ASSERTED_UNVERIFIED",
    "SCOPE_CALLER_ASSERTED_UNVERIFIED",
    "OPEN_FINDING_ACCOUNTED"
  ],
  "allowed": [
    "request_fresh_basis",
    "propose_recommendation"
  ],
  "forbidden": [
    "mutate_target",
    "claim_authorization"
  ],
  "receipt": {
    "receipt_id": "sha256:…",
    "obligation": "gap_receipt",
    "input_hash": "sha256:…",
    "evidence_ref_hashes": ["sha256:…"],
    "prev_receipt_hash": null,
    "timestamp": "2026-05-09T23:14:02Z"
  }
}
```

### 8.1 Hard rules

- `reason_codes` are SCREAMING_SNAKE_CASE, machine-readable, and stable across versions. The full v1 registry is §8.3.
- Per-dimension `reason_codes` is a list (a dimension may carry multiple codes, e.g. `STANDING_OK` + `STANDING_CALLER_ASSERTED_UNVERIFIED`). The top-level `reason_codes` is the union, deduplicated, in dimension order.
- Free-text rationales do not appear in v1 output. (A future `--explain` flag may add prose; it is not core.)
- `allowed` and `forbidden` are **suggestions, not commands**: they describe what the actor may try next and what is explicitly out. The downstream enforcer makes the actual decision.
- `receipt.receipt_id` is `sha256:` over canonical JSON of the verdict body excluding `receipt_id` itself, **including** `receipt.timestamp`.
- `receipt.evidence_ref_hashes` are `sha256:` hashes of each **evidence reference object** in canonical JSON form, not of the dereferenced evidence content. (Wicket does not dereference `ref` URIs.) The name is explicit so future-readers don't assume Wicket fetches and hashes the evidence itself.
- Canonical JSON is **RFC 8785 (JCS)** unless otherwise stated.
- Receipts are emitted on every call, including `unaccounted`. An `error_receipt` records that Wicket could not classify and what input refused to map.

### 8.2 Exit codes

`denied` is a successful classification, not a failure. The CLI default:

| Exit | Meaning                                                             |
| ---- | ------------------------------------------------------------------- |
| 0    | Verdict produced: `authorized`, `advisory_only`, `denied`, or `gap` |
| 2    | `unaccounted` — schema-valid input but not mappable to the model    |
| 64   | Malformed input — schema validation failure                         |
| 70   | Internal error — bug in Wicket itself                                |

`--strict-exit` makes `denied`, `gap`, and `unaccounted` all nonzero (for CI gates that want any non-authorization to fail the shell command). Default is non-strict.

### 8.3 Reason code registry (v1)

This registry is authoritative for v1. New codes require a SPEC.md amendment in the same commit that introduces them.

**Basis dimension** (SOFT codes pair with `basis.status ∈ {insufficient, stale, absent, ambiguous}` → `gap`; HARD codes pair with `basis.status ∈ {inadmissible, revoked}` → `denied`):

```
BASIS_OK                                              // status=satisfied

# SOFT — surface=gap
BASIS_ABSENT                                          // status=absent
BASIS_INSUFFICIENT_FOR_OBSERVE                        // status=insufficient
BASIS_INSUFFICIENT_FOR_INTERPRET                      // status=insufficient
BASIS_INSUFFICIENT_FOR_RECOMMEND                      // status=insufficient
BASIS_INSUFFICIENT_FOR_AUTHORIZE                      // status=insufficient
BASIS_INSUFFICIENT_FOR_EXECUTE                        // status=insufficient
BASIS_INSUFFICIENT_FOR_BIND                           // status=insufficient (missing supporting evidence; human_conf present)
BASIS_TTL_EXPIRED                                     // status=stale
BASIS_AMBIGUOUS_EVIDENCE_KIND                         // status=ambiguous

# HARD — surface=denied
BASIS_INADMISSIBLE_SELF_CERTIFIED                     // status=inadmissible (§4.4)
BASIS_INADMISSIBLE_BIND_REQUIRES_HUMAN_CONFIRMATION   // status=inadmissible (§7.3 bind row)
BASIS_REVOKED                                         // status=revoked (§4.8 revocation.basis_revoked=true)
```

**Precedence dimension:**

```
PRECEDENCE_OK
PRECEDENCE_SUPERSEDED
PRECEDENCE_AMBIGUOUS
PRECEDENCE_UNRESOLVED
PRECEDENCE_CALLER_ASSERTED_UNVERIFIED
```

**Standing dimension (includes scope-assertion and revocation-forbidden codes):**

```
STANDING_OK
STANDING_INSUFFICIENT_FOR_OPERATION
STANDING_ABSENT
STANDING_OUT_OF_SCOPE
STANDING_FORBIDDEN                              // status=forbidden (§4.8 revocation.standing_forbidden=true)
STANDING_CALLER_ASSERTED_UNVERIFIED
STANDING_LADDER_V1_FLAT
SCOPE_NOT_ASSERTED
SCOPE_CALLER_ASSERTED_UNVERIFIED
REVOCATION_CALLER_ASSERTED_UNVERIFIED
```

**Cross-cutting / accounting:**

```
OPEN_FINDING_ACCOUNTED
UNACCOUNTED_INPUT
INPUT_SCHEMA_INVALID
INPUT_UNKNOWN_ENUM_VALUE
INPUT_FUTURE_PROVENANCE_RESERVED
RECEIPT_HASH_MISMATCH
```

A code that does not appear in this registry MUST NOT be emitted by v1. CI enforces this via fixture validation.

---

## 9. Receipt obligations

| Surface        | Receipt obligation  | Notes                                              |
| -------------- | ------------------- | -------------------------------------------------- |
| `authorized`   | `action_receipt`    | Records what was authorized and the dimensional accounting. |
| `advisory_only`| `advisory_receipt`  | Records the recommendation. Not authorization.    |
| `denied`       | `refusal_receipt`   | Records refusal and the dimension that triggered it. |
| `gap`          | `gap_receipt`       | Records the open finding: what is accounted, what is missing, what would close it. |
| `unaccounted`  | `error_receipt`     | Records the unmappable input. Class: error.        |

Receipts are immutable. A dispute or new evidence produces a **new** receipt that may reference the prior via `prev_receipt_hash`. Wicket itself does not store receipts; the caller is responsible for persistence (this keeps Wicket stateless).

---

## 10. Fixture taxonomy

Fixtures are doctrine-pressure tests, not unit tests. Each fixture is a JSON file under `cases/<bucket>/<name>.json` containing:

```jsonc
{
  "name": "open_finding_is_admissible",
  "intent": { /* §4 input */ },
  "expected": {
    "surface_verdict": "gap",
    "dimensions": {
      "basis":      { "status": "insufficient" },
      "precedence": { "status": "satisfied" },
      "standing":   { "status": "satisfied" }
    },
    "must_include_reason_codes": ["BASIS_INSUFFICIENT_FOR_EXECUTE"],
    "must_forbid": ["mutate_target"]
  },
  "doctrine": ["openFinding_is_admissible"]
}
```

Each fixture is **one Wicket call**. Multi-step scenarios become multiple fixtures with `prev_receipt_hash` chaining where relevant. Wicket is stateless across calls; fixtures honor that.

### 10.1 Buckets

```
cases/
  positive/                                 # admissible outcomes (incl. open findings)
  laundering/                               # attempts to launder one verdict into another
  stale_revoked/                            # time- and revocation-based failures
  evidence/                                 # evidence sufficiency / provenance failures
  unaccounted/                              # malformed or model-unmappable inputs
  documentation_only/                       # cases Wicket cannot detect from input alone (§10.3)
```

### 10.2 V1 fixture set (acceptance-blocking)

The V1 release must ship at least these thirteen fixtures, each with stable expected output:

1. `positive/open_finding_is_admissible` → `gap` (Lean: P27 `open_finding_admissible_with_durability`)
2. `positive/observe_with_current_basis` → `authorized`
3. `positive/recommend_class_admissible` → `advisory_only` (operation_class = recommend, all dimensions satisfied)
4. `positive/bind_with_fresh_human_confirmation` → `authorized`
5. `laundering/recommend_standing_attempts_execute` → `denied` (standing insufficient for execute; `propose_recommendation` in `allowed`)
6. `laundering/recommend_standing_attempts_bind` → `denied` (same shape, harder boundary)
7. `laundering/same_basis_corrective_to_forward` → `denied` (Lean: `corrective_no_authority_laundering`)
8. `laundering/policy_gap_promoted_to_policy` → `denied`
9. `stale_revoked/revoked_basis_irreversible_action` → `denied` (Lean: `revoked_basis_cannot_be_authorized_step`)
10. `stale_revoked/stale_evidence_past_window` → `gap` (basis: stale)
11. `evidence/self_certification_as_evidence` → `denied` (basis: inadmissible + `BASIS_INADMISSIBLE_SELF_CERTIFIED`)
12. `evidence/bind_without_human_confirmation` → `denied` (basis: inadmissible + `BASIS_INADMISSIBLE_BIND_REQUIRES_HUMAN_CONFIRMATION`; HARD bind row violation)
13. `evidence/bind_with_human_confirmation_but_no_supporting_refs` → `gap` (basis: insufficient + `BASIS_INSUFFICIENT_FOR_BIND`; SOFT — actor can supply a `tool_trace` / `file_hash` / `prior_receipt` and resubmit)

Each fixture's `doctrine` field names the doctrinal line(s) it pressure-tests. CI runs all fixtures on every change.

### 10.3 Documentation-only fixtures

Some failure modes cannot be detected from a single Wicket call's input. They live in `cases/documentation_only/` with a `documentation_only: true` flag and are not executed by CI:

- `silent_downgrade_without_waiver` — caller mislabels `bind` as `execute`; Wicket cannot detect this without an external oracle. Documents the boundary.
- `closed_world_violation` — caller cites evidence from outside the declared closed world; provenance verification is out of v1 scope (`provenance: caller_asserted` only).

These remain in the doctrine library; promotion to executable status awaits a verified-provenance boundary (§13.4).

---

## 11. Relationship to Agent Governor and Lean

### 11.1 What stays in AG (the mint)

- Regime detection / control-theoretic feedback loop.
- Multi-agent leases, WAL-mode receipt kernel, concurrency.
- Composition governance (sequences and tool-call coupling).
- Domain plugins (fiction, nonfiction, writing modules).
- Supervised sessions and tool interception runtime.
- Override system, sunset clauses, scar tissue, hysteresis.
- Claim diffing, vocabulary drift, custody scoring.
- Scope-and-precedence resolution (Wicket consumes, AG resolves).

### 11.2 What Wicket owns (the handle)

- Single-call admissibility preflight: input → verdict + receipt.
- Three-dimensional verdict algebra (Basis × Precedence × Standing).
- Typed input model with structured evidence pointers (no prose authority).
- Hash-chained, immutable receipts in the same canonical form as AG.
- Fixture-driven doctrine pressure-testing.
- The reason-code registry (§8.3).

### 11.3 What the formal substrate (Lean) provides

The five-module Admissibility kernel in `~/git/lean/LeanProofs/Admissibility/` is the formal mint. Wicket is its operational shadow:

- `Authority.lean` — the `Basis × Precedence × Standing → AuthorityVerdict` algebra Wicket implements as a derivation table (§7.1).
- `StateTransition.lean` — the policy-trapdoor invariant Wicket enforces by refusing to treat policy-gap as policy-amendment.
- `Derivation.lean` — the bundled-derivation pattern Wicket models in its evidence/standing checks.
- `Execution.lean` — `revoked_basis_cannot_be_authorized_step` is fixture #9's expected verdict.
- `Corrective.lean` — `corrective_no_authority_laundering` is fixture #7's expected verdict.
- `Admissibility.lean` (P27 obligation skeleton) — `open_finding_admissible_with_durability` is fixture #1's expected verdict, and the source of the `openFinding` / `unaccounted` distinction.

### 11.4 Future kernel consumption

A future AG kernel (or Lean-checked Wicket) should be able to:

1. Read Wicket fixtures unchanged and reproduce the surface verdict.
2. Discharge each fixture as a theorem with the dimensional triple as inputs.
3. Translate Wicket verdicts into AG verdicts via a stable map (`authorized → PASS`, `denied → BLOCK`, `gap → WARN`, `advisory_only → OBSERVE`, `unaccounted → ERROR`).

Wicket fixtures are the contract surface. Both AG and Lean conform to them; Wicket does not conform to AG or Lean.

### 11.5 AG/Wicket reproducibility (future contract)

A useful long-term constraint, not a v1 promise:

> **For every AG gate-receipt class, there should be a corresponding Wicket
> fixture, or a documented reason Wicket cannot express it.**

This cuts both ways:

- If AG cannot reduce a gate to a Wicket-shaped case, AG may be hiding
  doctrine in implementation machinery rather than in admissibility logic.
- If Wicket cannot express an AG gate, Wicket may be missing a primitive.
- If AG and Wicket disagree on a fixture, the divergence is itself
  evidence — one of them is wrong, and the disagreement names where.

Wicket is small enough to be read for doctrine; AG is too large. That
asymmetry is the reason this contract is useful: Wicket can be the
ratification surface that AG tools cite when they want to argue their
gate machinery is doctrinally clean.

---

## 12. V1 acceptance criteria

V1 is "demo-able and doctrinally correct." Not "feature-complete."

- [ ] SPEC.md frozen and reviewed.
- [ ] JSON schemas (`schemas/intent.schema.json`, `schemas/verdict.schema.json`) match §4 and §8 exactly.
- [ ] Rust skeleton compiles with zero warnings on `--release`.
- [ ] No async, no DB, no plugins, no transport, no MCP.
- [ ] Total Rust LOC ≤ 5,000 (excluding fixtures and generated code).
- [ ] At least 13 executable fixtures across all five executable buckets.
- [ ] Documentation-only fixtures present and clearly marked.
- [ ] `cat cases/<any>.json | wicket check` produces the fixture's expected verdict for every shipped fixture.
- [ ] `cargo test` runs all fixtures and passes.
- [ ] Receipts are content-addressed (RFC 8785 canonical JSON, `sha256:`, prev-link supported).
- [ ] CLI emits boring JSON. No prose. No emoji.
- [ ] Exit codes follow §8.2.
- [ ] Reason codes are restricted to the §8.3 registry; CI rejects unknown codes.
- [ ] One operational skill in `skills/preflight.md` describing how an agent should call Wicket.

V1 explicitly does **not** require:

- Receipt persistence.
- Multiple actors per call.
- A policy DSL.
- An override or waiver mechanism.
- A `--explain` prose mode.
- An MCP shim.
- Any IDE integration.
- Verified or attested provenance.

---

## 13. Open questions

Tracked here, not implemented around.

1. **`unaccounted` detection scope.** V1 fires `unaccounted` only on schema/enum failures or future-provenance values. §7.1 is exhaustive over valid inputs. Future: should genuinely uncovered cross-products also be `unaccounted`? Resolution before any §7.1 expansion.
2. **Δt / freshness of `prev_receipt_hash`.** V1: independent of basis freshness. Receipt-chain expresses sequencing, not freshness. A future authority-token boundary may change this.
3. **Multiple evidence kinds for one basis.** V1: weakest kind sets the floor for §7.3 sufficiency. Sharper composition rules deferred.
4. **Provenance verification boundary.** V1 supports `caller_asserted` only. `attested` and `verified` are **schema-allowed but model-unaccounted** (exit 2 with `INPUT_FUTURE_PROVENANCE_RESERVED`). The verification boundary is where AG injects token/proof checking. Document where it lives before any v2 work.
5. **Operation class boundary between `execute` and `bind`.** V1: caller declares; mislabeling lives in `documentation_only/silent_downgrade`. Future: external oracle for irreversibility.
6. **Fixture format versioning.** V1 has no version field. Add one before fixture set crosses ~25 cases.
7. **Standing-ladder debt.** V1 uses a strict total order; reality has adjacent powers (approve-without-execute, execute-only-under-approval). Every `authorize`/`execute` call emits `STANDING_LADDER_V1_FLAT`. Resolution: a partial-order standing model, deferred to v2.
8. **Schema vs spec authority.** SPEC.md is doctrine. Schemas are encoding. Any schema change that changes meaning must update SPEC.md in the same commit. No automated regeneration is promised.
9. **Basis sufficiency matrix sharpness.** §7.3 is V1 doctrine, not eternal truth. Specific known soft spots: `interpret` requirement for "observable target" is not encoded in input; `execute` matrix may admit too many trace kinds. Sharper cuts deferred.
10. **Self-certification severity.** **Resolved (v0.3):** any self-certified ref makes the basis `inadmissible` → `denied` regardless of other evidence (§4.4). The previous v0.2 stance (disregarded but not failing) was incompatible with fixture #11 expecting `denied`. The hard rule is the doctrinally-correct stance: legitimate evidence should not be diluted by self-certification, and Wicket cannot tell which refs the downstream enforcer would weight. Future v2 may admit a tightly-scoped exception (e.g. self-cert allowed for `observe` only); v1 does not.

---

## 14. Glossary

- **Basis** — the rule and supporting evidence the actor cites for its right to act. *Not* the evidence alone; not the rule alone.
- **Precedence** — among applicable rules, which one governs. In Wicket v1, caller-resolved.
- **Standing** — the authority class the actor holds, scoped to the target.
- **Operation class** — the severity level of the intended action, from `observe` to `bind`.
- **Open finding** — an admissible outcome where Wicket cannot authorize but can fully account for why. Doctrinally not a failure.
- **Unaccounted** — an inadmissible outcome where Wicket cannot classify the case at all under its current model. The only error-class verdict.
- **Receipt** — an immutable, hash-chained record of one Wicket call. Disputes produce new receipts; receipts are never edited.
- **Trapdoor invariant** — only an explicit policy-amendment operation may modify policy. Absence of contradiction is not permission.
- **Self-certification** — evidence whose `issuer == subject == actor`. Any self-certified ref makes the basis `inadmissible` → `denied` (§4.4).
- **Soft / hard basis failure** — soft (`insufficient`/`stale`/`absent`/`ambiguous`) is closable by supplying more or fresher evidence and produces `gap`. Hard (`inadmissible`/`revoked`) is not closable from this call and produces `denied`.
- **Caller-cooked** — a field whose value the caller has resolved before calling Wicket (scope, precedence, revocation). Wicket does not verify cooked values in v1; it accounts for them.
- **Caller-asserted** — the v1 provenance value naming caller-cooked trust without verification.
- **Authorized-under-context** — the v1 meaning of `authorized`: authorized given the caller-supplied context, not independently verified.
- **Supporting evidence** — for the `bind` row of §7.3, any of `tool_trace`, `test_log`, `command_output`, `file_hash`, `prior_receipt`. Required in addition to fresh `human_confirmation`.
- **Standing grant** — a wrapper-level (§15.1) JSON document binding an issuer to an actor's standing class over a scope root, set of operation verbs, and time window. Validated by the wrapper before the kernel ever sees the resulting Intent.
- **Mint / handle** — the constitutional substrate (AG, Lean) is the mint; Wicket is the handle. Authority is minted there; passage is gated here.

---

## 15. Wrapper conventions (non-kernel)

This section documents conventions used by the verb-shaped CLI wrappers
(`wicket edit`, `wicket run`, `wicket commit`). **The kernel knows nothing
about these conventions.** They are wrapper-level mechanisms that produce
ordinary Intents the kernel evaluates with its existing rules. This section
exists so that a future re-implementation of the wrapper can match the
current behavior, and so that the audit trail in receipts is intelligible.

### 15.1 Standing grants

> **Standing must be granted, not asserted.** Raw `--standing execute` exists
> for test/dev only; in production, an integrator establishes a structured,
> bounded grant.

A standing grant is a JSON document describing one issuance of standing
authority from an issuer to an actor over a scope root, for a specified set
of operation verbs, between two timestamps:

```jsonc
{
  "schema": "wicket-grant/v1",
  "actor": "claude-code",
  "standing": "execute",
  "scope_root": "/home/jbeck/git/wicket",
  "operations": ["edit", "run", "commit"],
  "issued_by": "jmbeck",
  "issued_at": "2026-05-09T10:00:00Z",
  "expires_at": "2026-05-09T18:00:00Z",
  "basis": "interactive coding session approval",
  "grant_id": "sha256:..."   // optional; computed if absent
}
```

`grant_id` is `sha256:` over RFC 8785 canonical JSON of the body excluding
`grant_id` itself. Same construction as receipts.

### 15.2 Wrapper validation

When `--standing-grant <FILE>` is supplied to a verb subcommand, the wrapper
validates the grant against the requested invocation:

1. `schema` matches `wicket-grant/v1` (else `GRANT_SCHEMA_MISMATCH`).
2. `actor` equals the wrapper's `--actor` (else `GRANT_ACTOR_MISMATCH`).
3. The verb (`edit` / `run` / `commit`) appears in `operations`
   (else `GRANT_OPERATION_NOT_PERMITTED`).
4. `issued_at` and `expires_at` parse as ISO-8601 (else
   `GRANT_UNPARSEABLE_TIMESTAMP`).
5. `now` is in the closed interval `[issued_at, expires_at]`
   (else `GRANT_NOT_YET_VALID` or `GRANT_EXPIRED`).
6. The intent's target (or `cwd` for `run`/`commit`) is contained within
   `scope_root` (else `GRANT_OUT_OF_SCOPE`).

On success: the wrapper sets `actor_standing.class = grant.standing` and
attaches the grant as a `policy_ref` Evidence ref. The kernel sees this as
ordinary policy evidence; the receipt's `evidence_ref_hashes` records it.

On failure: the wrapper writes the validation reason to stderr and exits 65
(data error). **No Intent is constructed and no receipt is emitted by the
kernel.** This is wrapper-level refusal — the kernel-level "every call
emits a receipt" doctrine still holds for actual `wicket check` invocations.

### 15.3 Why grants belong outside the kernel

Grants are a coordination convenience. The kernel's authority model is
deliberately narrow: it accounts for whether a cooked Intent satisfies the
basis × precedence × standing triple. Grants live above that — they are how
a higher-trust adapter (a session manager, an auth proxy, an MCP server, a
future Agent Governor) hands the wrapper a bounded "you may now invoke at
this standing" envelope.

Because grants are not kernel doctrine, this spec does not mandate their
format. Different wrapper implementations may use different grant formats
or even different mechanisms (e.g., signed tokens, OS keychain entries).
The constraint is purely: **the wrapper must not allow standing claims it
cannot point to evidence for.** Raw `--standing execute` violates that
loudly via `STANDING_CALLER_ASSERTED_UNVERIFIED`; grants make the violation
auditable.

### 15.4 Cook-layer budget

The wrapper's "cooking" — gathering filesystem and environment evidence,
auto-detecting policy docs, hashing target files — is **structurally
different** from the kernel's verdict logic. Kernel doctrine is small and
formalizable; cook is heuristic and fragile.

To keep the cook from becoming the policy-discovery grammar the kernel
refuses to be, the cook layer carries its own LOC budget:

> **Cook may gather facts. Cook may not become the grammar of authority.**

V1 budget: cook layer ≤ **1,000 LOC** (separate from the kernel's 5,000 LOC
cap in §2). Anything that pushes cook past that ceiling — monorepo policy
search, extra repo layouts, submodule traversal — is the wrong feature.
The right move when cook gets crowded is to push the work onto an upstream
adapter (AG, an MCP server) that hands Wicket a fully-cooked Intent.

---

*Heavy at the mint. Light at the handle.*
