//! Surface verdict derivation — §7.1 first-match-wins table, plus
//! allowed/forbidden actions, receipt obligations, and the public `derive`
//! orchestrator that produces a complete [`Outcome`].

use crate::model::*;
use crate::receipt;
use crate::rules::{self, codes};

/// Derive a complete outcome (verdict + dimensions + receipt) for one intent.
///
/// SPEC §7.1 evaluation order: unaccounted > denied > advisory_only > gap > authorized.
/// First match wins.
pub fn derive(intent: &Intent) -> Outcome {
    // --- Unaccounted gate (future provenance, §4.2). ---------------------
    if rules::has_future_provenance(intent) {
        return unaccounted_outcome(intent);
    }

    let basis = rules::derive_basis(intent);
    let precedence = rules::derive_precedence(intent);
    let standing = rules::derive_standing(intent);

    let surface = surface_verdict(intent, &basis, &precedence, &standing);
    finalize(intent, surface, basis, precedence, standing)
}

// ---------------------------------------------------------------------------
// Surface verdict (§7.1)
// ---------------------------------------------------------------------------

fn surface_verdict(
    intent: &Intent,
    basis: &DimensionResult<BasisStatus>,
    precedence: &DimensionResult<PrecedenceStatus>,
    standing: &DimensionResult<StandingStatus>,
) -> SurfaceVerdict {
    // Order of evaluation: denied > advisory_only > gap > authorized.
    // (unaccounted is handled before this function is called.)

    // (1) HARD denials.
    if basis.status.is_hard() {
        return SurfaceVerdict::Denied;
    }
    if precedence.status == PrecedenceStatus::Superseded {
        return SurfaceVerdict::Denied;
    }
    if matches!(
        standing.status,
        StandingStatus::Absent | StandingStatus::Forbidden | StandingStatus::OutOfScope
    ) {
        return SurfaceVerdict::Denied;
    }
    if standing.status == StandingStatus::Insufficient {
        return SurfaceVerdict::Denied;
    }

    // (2) advisory_only — recommend-class operation that meets the bar.
    if intent.operation_class == OperationClass::Recommend
        && basis.status == BasisStatus::Satisfied
        && precedence.status == PrecedenceStatus::Satisfied
        && standing.status == StandingStatus::Satisfied
    {
        return SurfaceVerdict::AdvisoryOnly;
    }

    // (3) gap — soft basis insufficiency or precedence ambiguity.
    if basis.status.is_soft_unsatisfied() || precedence.status == PrecedenceStatus::Ambiguous {
        return SurfaceVerdict::Gap;
    }

    // (4) authorized — all three satisfied (and §7.3 holds, which is
    //     reflected in basis.status = Satisfied).
    SurfaceVerdict::Authorized
}

// ---------------------------------------------------------------------------
// Outcome assembly
// ---------------------------------------------------------------------------

fn finalize(
    intent: &Intent,
    surface: SurfaceVerdict,
    basis: DimensionResult<BasisStatus>,
    precedence: DimensionResult<PrecedenceStatus>,
    standing: DimensionResult<StandingStatus>,
) -> Outcome {
    let class = match surface {
        SurfaceVerdict::Unaccounted => OutcomeClass::Error,
        _ => OutcomeClass::Verdict,
    };

    let obligation = match surface {
        SurfaceVerdict::Authorized => ReceiptObligation::ActionReceipt,
        SurfaceVerdict::AdvisoryOnly => ReceiptObligation::AdvisoryReceipt,
        SurfaceVerdict::Denied => ReceiptObligation::RefusalReceipt,
        SurfaceVerdict::Gap => ReceiptObligation::GapReceipt,
        SurfaceVerdict::Unaccounted => ReceiptObligation::ErrorReceipt,
    };

    let mut top_codes = union_codes(&basis, &precedence, &standing);
    if surface == SurfaceVerdict::Gap && !top_codes.iter().any(|c| c == codes::OPEN_FINDING_ACCOUNTED) {
        top_codes.push(codes::OPEN_FINDING_ACCOUNTED.to_string());
    }

    let allowed = allowed_actions(intent, surface, &basis, &precedence, &standing);
    let forbidden = forbidden_actions(surface);

    let dimensions = Dimensions { basis, precedence, standing };
    let receipt_partial = receipt::build(intent, &dimensions, &top_codes, obligation);

    let mut outcome = Outcome {
        class,
        surface_verdict: surface,
        operation_class: intent.operation_class,
        dimensions,
        reason_codes: top_codes,
        allowed,
        forbidden,
        receipt: receipt_partial,
    };
    receipt::seal(&mut outcome);
    outcome
}

fn union_codes(
    basis: &DimensionResult<BasisStatus>,
    precedence: &DimensionResult<PrecedenceStatus>,
    standing: &DimensionResult<StandingStatus>,
) -> Vec<String> {
    // SPEC §8.1: union, deduplicated, in dimension order (basis → precedence → standing).
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for c in basis
        .reason_codes
        .iter()
        .chain(precedence.reason_codes.iter())
        .chain(standing.reason_codes.iter())
    {
        if seen.insert(c.clone()) {
            out.push(c.clone());
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Unaccounted path
// ---------------------------------------------------------------------------

fn unaccounted_outcome(intent: &Intent) -> Outcome {
    let basis = DimensionResult {
        status: BasisStatus::Ambiguous,
        reason_codes: vec![codes::INPUT_FUTURE_PROVENANCE_RESERVED.to_string()],
    };
    let precedence = DimensionResult {
        status: PrecedenceStatus::Ambiguous,
        reason_codes: vec![codes::INPUT_FUTURE_PROVENANCE_RESERVED.to_string()],
    };
    let standing = DimensionResult {
        status: StandingStatus::Insufficient,
        reason_codes: vec![codes::INPUT_FUTURE_PROVENANCE_RESERVED.to_string()],
    };
    let mut top_codes = vec![
        codes::INPUT_FUTURE_PROVENANCE_RESERVED.to_string(),
        codes::UNACCOUNTED_INPUT.to_string(),
    ];
    top_codes.dedup();

    let dimensions = Dimensions { basis, precedence, standing };
    let receipt_partial =
        receipt::build(intent, &dimensions, &top_codes, ReceiptObligation::ErrorReceipt);

    let mut outcome = Outcome {
        class: OutcomeClass::Error,
        surface_verdict: SurfaceVerdict::Unaccounted,
        operation_class: intent.operation_class,
        dimensions,
        reason_codes: top_codes,
        allowed: vec!["fix_input_and_retry".into()],
        forbidden: vec!["proceed_with_unaccounted_intent".into()],
        receipt: receipt_partial,
    };
    receipt::seal(&mut outcome);
    outcome
}

// ---------------------------------------------------------------------------
// Allowed / forbidden actions — §8.1: suggestions, not commands.
// ---------------------------------------------------------------------------

fn allowed_actions(
    intent: &Intent,
    surface: SurfaceVerdict,
    basis: &DimensionResult<BasisStatus>,
    precedence: &DimensionResult<PrecedenceStatus>,
    standing: &DimensionResult<StandingStatus>,
) -> Vec<String> {
    match surface {
        SurfaceVerdict::Authorized => vec!["execute_intended_action".into()],
        SurfaceVerdict::AdvisoryOnly => vec![
            "share_recommendation".into(),
            "draft_recommendation".into(),
        ],
        SurfaceVerdict::Denied => denied_allowed(intent, basis, precedence, standing),
        SurfaceVerdict::Gap => gap_allowed(intent, basis, precedence),
        SurfaceVerdict::Unaccounted => vec!["fix_input_and_retry".into()],
    }
}

fn denied_allowed(
    intent: &Intent,
    basis: &DimensionResult<BasisStatus>,
    precedence: &DimensionResult<PrecedenceStatus>,
    standing: &DimensionResult<StandingStatus>,
) -> Vec<String> {
    let mut out = Vec::new();
    // Standing-insufficient in a higher op class → propose recommendation instead.
    if standing.status == StandingStatus::Insufficient
        && matches!(
            intent.operation_class,
            OperationClass::Authorize | OperationClass::Execute | OperationClass::Bind
        )
    {
        out.push("propose_recommendation".into());
    }
    if matches!(
        standing.status,
        StandingStatus::Absent | StandingStatus::Forbidden | StandingStatus::OutOfScope
    ) {
        out.push("escalate_to_authority_holder".into());
    }
    if basis.status == BasisStatus::Revoked {
        out.push("request_fresh_basis".into());
    }
    if basis.status == BasisStatus::Inadmissible {
        out.push("supply_admissible_basis".into());
    }
    if precedence.status == PrecedenceStatus::Superseded {
        out.push("adopt_superseding_rule".into());
    }
    if out.is_empty() {
        out.push("review_dimensional_accounting".into());
    }
    out
}

fn gap_allowed(
    intent: &Intent,
    basis: &DimensionResult<BasisStatus>,
    precedence: &DimensionResult<PrecedenceStatus>,
) -> Vec<String> {
    let mut out = Vec::new();
    match basis.status {
        BasisStatus::Stale => out.push("request_fresh_basis".into()),
        BasisStatus::Insufficient => out.push("supply_supporting_evidence".into()),
        BasisStatus::Absent => out.push("supply_basis_and_evidence".into()),
        BasisStatus::Ambiguous => out.push("clarify_basis_evidence_kinds".into()),
        _ => {}
    }
    if precedence.status == PrecedenceStatus::Ambiguous {
        out.push("resolve_precedence".into());
    }
    if intent.operation_class != OperationClass::Observe {
        out.push("draft_recommendation".into());
    }
    if out.is_empty() {
        out.push("close_open_finding".into());
    }
    out
}

fn forbidden_actions(surface: SurfaceVerdict) -> Vec<String> {
    match surface {
        SurfaceVerdict::Authorized => vec![],
        SurfaceVerdict::AdvisoryOnly => vec![
            "execute_recommendation_as_authorization".into(),
            "claim_authorization".into(),
        ],
        SurfaceVerdict::Denied => vec![
            "mutate_target".into(),
            "claim_authorization".into(),
        ],
        SurfaceVerdict::Gap => vec![
            "mutate_target".into(),
            "claim_authorization".into(),
        ],
        SurfaceVerdict::Unaccounted => vec!["proceed_with_unaccounted_intent".into()],
    }
}
