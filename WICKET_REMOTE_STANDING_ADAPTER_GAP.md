# WICKET_REMOTE_STANDING_ADAPTER_GAP

**Status:** candidate / non-binding. Boundary note, not authorization to build.
**Filed:** 2026-05-27
**Trigger:** cross-constellation pressure as Standing-the-tool and sibling
tools weave together. Wicket is already the narrow gate; this filing exists
so it stays narrow when remote/assertion standing starts arriving from
outside.

## Position

Wicket is already the action/admissibility preflight kernel. There is no
missing feature here — only a vocabulary boundary that must hold once
Standing-the-tool starts emitting remote/caller assertion standing into the
constellation.

## Do not collapse

```
Standing AssertionGrant / StandingDecision
        ≠
Wicket ActorStanding / StandingClass
```

They name different things:

- **Wicket `StandingClass`** — operation-phase role of the actor performing
  the intended operation.
- **Standing-the-tool `AssertionGrant` / `StandingDecision`** — whether a
  remote speaker had standing to assert or request the thing in the first
  place.

Collapsing the two would launder assertion standing into operation authority.
That is the failure mode this note exists to block.

## Likely future addition (not yet ratified)

A separate `caller_assertion_standing` / `remote_standing_evidence` field on
`Intent`, or on wrapper-cooked evidence, carrying refs to Standing's output.
The kernel continues to see only cooked Intent — it does not call Standing,
Continuity, NQ, or Nightshift directly.

## Do not implement until

- Standing has visible-not-binding resolver output.
- NQ proves the remote-standing shape end to end.
- Wicket has a real consumer plant exercising the boundary.

Any earlier implementation would be speculative expansion. This filing is
the handle; the build is gated on the conditions above.

## Vocabulary boundary

| Term                    | Means                                |
| ----------------------- | ------------------------------------ |
| Wicket `StandingClass`  | operation-phase role of the actor    |
| Standing-the-tool       | speaker / requester assertion standing |

## Receipt convention (anticipated, not specified)

Wicket receipts may eventually include `relied_on[]` and
`remote_standing_evidence` hashes. SPEC §11 is the natural home if/when this
ratifies; do not move it there until a real consumer plant exists.

## Adapter plan (anticipated, not specified)

The wrapper / cook layer may accept `StandingDecision` and Continuity
`relied_on` refs. The kernel continues to see only the cooked Intent. The
"caller cooks context" spine is non-negotiable.

Discipline the cook must follow when it ratifies (cf. the
cook-translation-authority maxim in
`~/git/wlp/WLP_RECEIVER_GATE_CANDIDATE.md`):

> Cooking is translation under receiver authority, not ontology
> inheritance. The adapter may translate testimony into policy
> vocabulary; it may not let testimony choose the vocabulary.

Concretely: a future `caller_assertion_standing` field on cooked
Intent does not inherit Standing's wire names by reflex. The cook
picks the Wicket-vocabulary field that carries the load
Standing's field carries — even when both projects happen to use
the same noun. The receiver-gate WLP↔Wicket bridge worked
precisely because the cook treated `prior_receipt`,
`command_output`, and `policy_ref` as Wicket-owned names, not
import-aliases.

## Fixture additions (anticipated, `documentation_only/`)

- Remote standing present but not enough for action.
- Remote standing absent.
- Remote standing denied.
- Wicket refuses laundering assertion standing into operation authority.

## Containment

This note does not authorize:

- Adding a real `caller_assertion_standing` field to `Intent` or any schema.
- Calling Standing, Continuity, or NQ from the kernel.
- Editing SPEC §11 or §15 to absorb remote-standing vocabulary.
- Expanding Wicket scope to mediate constellation-wide standing.

Tiny courthouse, not Supreme Court cosplay.

## Sibling notes

- [[verifier]] (sibling Python/Z3 admissibility solver, README "Relationship
  to other projects") — overlapping vocabulary, different runtime job; same
  boundary discipline applies (composable evidence surfaces, not
  interchangeable outputs).
