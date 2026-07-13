<!--
Adopted verbatim from cartography on 2026-05-28.
Source: ~/git/cartography/doctrine/wicket-intoto-interop.md (ARCHIVED
2026-06-14; archival committed 2026-07-13 — cartography absorbed by agent_gov).
Discipline: adopted verbatim; if amendment is needed, file feedback with
agent_gov (constellation coordination venue; topology index at
agent_gov/docs/CONSTELLATION_MAP.md) and the doctrine evolves there.
Do not fork locally.
-->

# Wicket verdicts as in-toto attestations

**Interoperate, don't reinvent.** A Wicket admissibility verdict expresses
cleanly as an in-toto v1 Statement wrapped in a DSSE envelope — the same format
`cosign attest` produces and `cosign verify-attestation` consumes. You do not
need a new receipt envelope; the ecosystem already has one.

## The mapping

| in-toto concept | Wicket concept |
|---|---|
| `subject` (the artifact) | the **intent**, addressed by its content-hash |
| `subject.digest.sha256` | **exactly Wicket's `input_hash`** |
| `predicateType` | `…/wicket-admissibility/v0.1` — "this is a Wicket verdict" |
| `predicate` | the verdict body (surface_verdict, dimensions, allowed/forbidden, receipt) |

The load-bearing detail: the subject digest *is* the `input_hash` Wicket already
mints. So any in-toto verifier asking "does this attestation's subject match my
artifact?" is checking the same content-address Wicket produced — the two
systems agree on the address of the evidence without either trusting the other.
Evidence custody survives the hop into a standard format.

## What is proven (ran locally)

- A real `denied` verdict serialized to a structurally valid in-toto v1
  Statement.
- Wrapped in a DSSE envelope, signed with Ed25519.
- **Independently verified** by a separate tool: the DSSE signature checks out,
  *and* the subject digest equals the content-address recomputed from the
  original intent with a standalone RFC-8785 canonicalizer.

```
  [1] DSSE signature verifies        : True
  [2] subject == intent content-addr : True
```

## What is deferred (honest scope)

- **Transparency log.** Recording the attestation in Rekor (or a self-hosted
  log) and verifying with the `cosign` CLI is not exercised here. The envelope
  is cosign-shaped; closing this is a packaging step, not a research one.
- **A registered, resolvable `predicateType` URI.** The placeholder above is
  yours to own and publish.

## A note on signing identity

Prefer **keyed signing** (your own Ed25519/cosign key) over Sigstore *keyless*.
Keyless (Fulcio + the public-good instance) binds every signature to an **OIDC
identity** — a GitHub account, a Google login, an email — and records it in a
public transparency log alongside every attestation. Keyed signing keeps the
identity layer on a key you control. If you want a transparency log without that
identity coupling, run a self-hosted Rekor. The interop story holds either way.

## Running the bridge

A working bridge lives at [`examples/wicket_to_intoto.py`](../examples/wicket_to_intoto.py).

```bash
pip install jcs cryptography
python examples/wicket_to_intoto.py <verdict.json> <intent.json> <out_prefix>
```

The script writes `<out_prefix>.intoto.jsonl` (the DSSE envelope) and, on
first run, generates an Ed25519 keypair under `<out_prefix>'s parent>/keys/`.
Subsequent runs reuse the same key.
