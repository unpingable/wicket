//! Per-dimension derivation rules and the §7.3 basis-sufficiency matrix.
//!
//! Mirrors SPEC.md §6 (per-dimension status), §7.3 (basis sufficiency),
//! and §8.3 (reason code registry).

use crate::model::*;
use chrono::{DateTime, Utc};

// ---------------------------------------------------------------------------
// Reason codes — §8.3
// ---------------------------------------------------------------------------

pub mod codes {
    // Basis — soft (→ gap)
    pub const BASIS_OK: &str = "BASIS_OK";
    pub const BASIS_ABSENT: &str = "BASIS_ABSENT";
    pub const BASIS_INSUFFICIENT_FOR_OBSERVE: &str = "BASIS_INSUFFICIENT_FOR_OBSERVE";
    pub const BASIS_INSUFFICIENT_FOR_INTERPRET: &str = "BASIS_INSUFFICIENT_FOR_INTERPRET";
    pub const BASIS_INSUFFICIENT_FOR_RECOMMEND: &str = "BASIS_INSUFFICIENT_FOR_RECOMMEND";
    pub const BASIS_INSUFFICIENT_FOR_AUTHORIZE: &str = "BASIS_INSUFFICIENT_FOR_AUTHORIZE";
    pub const BASIS_INSUFFICIENT_FOR_EXECUTE: &str = "BASIS_INSUFFICIENT_FOR_EXECUTE";
    pub const BASIS_INSUFFICIENT_FOR_BIND: &str = "BASIS_INSUFFICIENT_FOR_BIND";
    pub const BASIS_TTL_EXPIRED: &str = "BASIS_TTL_EXPIRED";
    pub const BASIS_AMBIGUOUS_EVIDENCE_KIND: &str = "BASIS_AMBIGUOUS_EVIDENCE_KIND";
    // Basis — hard (→ denied)
    pub const BASIS_INADMISSIBLE_SELF_CERTIFIED: &str = "BASIS_INADMISSIBLE_SELF_CERTIFIED";
    pub const BASIS_INADMISSIBLE_BIND_REQUIRES_HUMAN_CONFIRMATION: &str =
        "BASIS_INADMISSIBLE_BIND_REQUIRES_HUMAN_CONFIRMATION";
    pub const BASIS_REVOKED: &str = "BASIS_REVOKED";

    // Precedence
    pub const PRECEDENCE_OK: &str = "PRECEDENCE_OK";
    pub const PRECEDENCE_SUPERSEDED: &str = "PRECEDENCE_SUPERSEDED";
    pub const PRECEDENCE_AMBIGUOUS: &str = "PRECEDENCE_AMBIGUOUS";
    pub const PRECEDENCE_UNRESOLVED: &str = "PRECEDENCE_UNRESOLVED";
    pub const PRECEDENCE_CALLER_ASSERTED_UNVERIFIED: &str = "PRECEDENCE_CALLER_ASSERTED_UNVERIFIED";

    // Standing & scope & revocation provenance
    pub const STANDING_OK: &str = "STANDING_OK";
    pub const STANDING_INSUFFICIENT_FOR_OPERATION: &str = "STANDING_INSUFFICIENT_FOR_OPERATION";
    pub const STANDING_ABSENT: &str = "STANDING_ABSENT";
    pub const STANDING_OUT_OF_SCOPE: &str = "STANDING_OUT_OF_SCOPE";
    pub const STANDING_FORBIDDEN: &str = "STANDING_FORBIDDEN";
    pub const STANDING_CALLER_ASSERTED_UNVERIFIED: &str = "STANDING_CALLER_ASSERTED_UNVERIFIED";
    pub const STANDING_LADDER_V1_FLAT: &str = "STANDING_LADDER_V1_FLAT";
    pub const SCOPE_NOT_ASSERTED: &str = "SCOPE_NOT_ASSERTED";
    pub const SCOPE_CALLER_ASSERTED_UNVERIFIED: &str = "SCOPE_CALLER_ASSERTED_UNVERIFIED";
    pub const REVOCATION_CALLER_ASSERTED_UNVERIFIED: &str = "REVOCATION_CALLER_ASSERTED_UNVERIFIED";

    // Cross-cutting
    pub const OPEN_FINDING_ACCOUNTED: &str = "OPEN_FINDING_ACCOUNTED";
    pub const UNACCOUNTED_INPUT: &str = "UNACCOUNTED_INPUT";
    pub const INPUT_SCHEMA_INVALID: &str = "INPUT_SCHEMA_INVALID";
    pub const INPUT_UNKNOWN_ENUM_VALUE: &str = "INPUT_UNKNOWN_ENUM_VALUE";
    pub const INPUT_FUTURE_PROVENANCE_RESERVED: &str = "INPUT_FUTURE_PROVENANCE_RESERVED";
    pub const RECEIPT_HASH_MISMATCH: &str = "RECEIPT_HASH_MISMATCH";
}

// ---------------------------------------------------------------------------
// Time helpers
// ---------------------------------------------------------------------------

pub(crate) fn parse_ts(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s).ok().map(|dt| dt.with_timezone(&Utc))
}

/// An evidence ref is "fresh" when its `status` is `valid` AND `valid_until`
/// is at or after `call_timestamp`. Stale or unparseable timestamps disqualify.
fn is_fresh(ev: &Evidence, call_ts: DateTime<Utc>) -> bool {
    if ev.status != ValidityStatus::Valid {
        return false;
    }
    let until = match parse_ts(&ev.valid_until) {
        Some(t) => t,
        None => return false,
    };
    until >= call_ts
}

fn is_self_cert(ev: &Evidence, actor: &str) -> bool {
    ev.issuer == ev.subject && ev.subject == actor
}

// ---------------------------------------------------------------------------
// Basis derivation — §6.1, §7.3
// ---------------------------------------------------------------------------

pub fn derive_basis(intent: &Intent) -> DimensionResult<BasisStatus> {
    let mut codes: Vec<String> = Vec::new();

    // (1) Hard: revocation.basis_revoked = true → revoked → denied.
    if intent.revocation.basis_revoked {
        codes.push(codes::BASIS_REVOKED.to_string());
        return DimensionResult { status: BasisStatus::Revoked, reason_codes: codes };
    }

    // (2) Hard: any self-certified evidence ref → inadmissible → denied.
    if intent
        .claimed_basis
        .evidence_refs
        .iter()
        .any(|e| is_self_cert(e, &intent.actor))
    {
        codes.push(codes::BASIS_INADMISSIBLE_SELF_CERTIFIED.to_string());
        return DimensionResult { status: BasisStatus::Inadmissible, reason_codes: codes };
    }

    let call_ts = match parse_ts(&intent.call_timestamp) {
        Some(t) => t,
        None => {
            // Unparseable call_timestamp is an unaccounted error path; treat as
            // schema-shaped failure for the basis dimension to surface upstream.
            codes.push(codes::BASIS_AMBIGUOUS_EVIDENCE_KIND.to_string());
            return DimensionResult { status: BasisStatus::Ambiguous, reason_codes: codes };
        }
    };

    // (3) Absent: rule empty AND no evidence refs → absent → gap.
    if intent.claimed_basis.rule.trim().is_empty()
        && intent.claimed_basis.evidence_refs.is_empty()
    {
        codes.push(codes::BASIS_ABSENT.to_string());
        return DimensionResult { status: BasisStatus::Absent, reason_codes: codes };
    }

    // (4) Apply §7.3 sufficiency matrix using fresh, non-self-cert evidence.
    let fresh: Vec<&Evidence> = intent
        .claimed_basis
        .evidence_refs
        .iter()
        .filter(|e| !is_self_cert(e, &intent.actor))
        .filter(|e| is_fresh(e, call_ts))
        .collect();
    let any_supplied_evidence = !intent.claimed_basis.evidence_refs.is_empty();
    let any_fresh = !fresh.is_empty();

    let has = |k: EvidenceKind| fresh.iter().any(|e| e.kind == k);

    use EvidenceKind::*;
    use OperationClass::*;
    let result = match intent.operation_class {
        Observe => {
            if has(Prompt) || has(PolicyRef) || has(FileHash) {
                Sufficiency::Ok
            } else {
                Sufficiency::Soft(codes::BASIS_INSUFFICIENT_FOR_OBSERVE)
            }
        }
        Interpret => {
            let has_basis_kind = has(Prompt) || has(PolicyRef);
            let has_target_ref = has(FileHash) || has(ToolOutput) || has(PriorReceipt);
            if has_basis_kind && has_target_ref {
                Sufficiency::Ok
            } else {
                Sufficiency::Soft(codes::BASIS_INSUFFICIENT_FOR_INTERPRET)
            }
        }
        Recommend => {
            let has_basis_kind = has(Prompt) || has(HumanConfirmation) || has(PolicyRef);
            let has_target_ref =
                has(FileHash) || has(ToolOutput) || has(PriorReceipt) || has(CommandOutput);
            if has_basis_kind && has_target_ref {
                Sufficiency::Ok
            } else {
                Sufficiency::Soft(codes::BASIS_INSUFFICIENT_FOR_RECOMMEND)
            }
        }
        Authorize => {
            if has(PolicyRef) || has(HumanConfirmation) {
                Sufficiency::Ok
            } else {
                Sufficiency::Soft(codes::BASIS_INSUFFICIENT_FOR_AUTHORIZE)
            }
        }
        Execute => {
            let has_basis = has(PolicyRef) || has(HumanConfirmation);
            let has_trace =
                has(ToolTrace) || has(TestLog) || has(CommandOutput) || has(FileHash);
            if has_basis && has_trace {
                Sufficiency::Ok
            } else {
                Sufficiency::Soft(codes::BASIS_INSUFFICIENT_FOR_EXECUTE)
            }
        }
        Bind => {
            // Hard rule first: must have a fresh human_confirmation within 60 min.
            let fresh_human_conf_within_window = fresh.iter().any(|e| {
                e.kind == HumanConfirmation
                    && parse_ts(&e.valid_from)
                        .map(|vf| (call_ts - vf).num_minutes().abs() <= 60)
                        .unwrap_or(false)
            });
            if !fresh_human_conf_within_window {
                Sufficiency::Hard(codes::BASIS_INADMISSIBLE_BIND_REQUIRES_HUMAN_CONFIRMATION)
            } else {
                let has_support = has(ToolTrace)
                    || has(TestLog)
                    || has(CommandOutput)
                    || has(FileHash)
                    || has(PriorReceipt);
                if has_support {
                    Sufficiency::Ok
                } else {
                    Sufficiency::Soft(codes::BASIS_INSUFFICIENT_FOR_BIND)
                }
            }
        }
    };

    match result {
        Sufficiency::Ok => {
            codes.push(codes::BASIS_OK.to_string());
            DimensionResult { status: BasisStatus::Satisfied, reason_codes: codes }
        }
        Sufficiency::Soft(code) => {
            // Distinguish stale-only from absent-of-right-kind from genuine absence.
            let status = if !any_supplied_evidence {
                BasisStatus::Absent
            } else if !any_fresh {
                codes.push(codes::BASIS_TTL_EXPIRED.to_string());
                BasisStatus::Stale
            } else {
                BasisStatus::Insufficient
            };
            codes.push(code.to_string());
            DimensionResult { status, reason_codes: codes }
        }
        Sufficiency::Hard(code) => {
            codes.push(code.to_string());
            DimensionResult { status: BasisStatus::Inadmissible, reason_codes: codes }
        }
    }
}

enum Sufficiency {
    Ok,
    Soft(&'static str),
    Hard(&'static str),
}

// ---------------------------------------------------------------------------
// Precedence derivation — §6.2
// ---------------------------------------------------------------------------

pub fn derive_precedence(intent: &Intent) -> DimensionResult<PrecedenceStatus> {
    let mut codes = Vec::new();

    let (status, code) = match intent.precedence.resolution {
        PrecedenceResolution::Active => (PrecedenceStatus::Satisfied, codes::PRECEDENCE_OK),
        PrecedenceResolution::Superseded => {
            (PrecedenceStatus::Superseded, codes::PRECEDENCE_SUPERSEDED)
        }
        PrecedenceResolution::Ambiguous => {
            (PrecedenceStatus::Ambiguous, codes::PRECEDENCE_AMBIGUOUS)
        }
        PrecedenceResolution::Unresolved => {
            (PrecedenceStatus::Ambiguous, codes::PRECEDENCE_UNRESOLVED)
        }
    };
    codes.push(code.to_string());

    if intent.precedence.provenance == Provenance::CallerAsserted {
        codes.push(codes::PRECEDENCE_CALLER_ASSERTED_UNVERIFIED.to_string());
    }

    DimensionResult { status, reason_codes: codes }
}

// ---------------------------------------------------------------------------
// Standing derivation — §6.3
// ---------------------------------------------------------------------------

pub fn derive_standing(intent: &Intent) -> DimensionResult<StandingStatus> {
    let mut codes = Vec::new();

    // (1) Hard: revocation.standing_forbidden = true → forbidden.
    if intent.revocation.standing_forbidden {
        codes.push(codes::STANDING_FORBIDDEN.to_string());
        if intent.revocation.provenance == Provenance::CallerAsserted {
            codes.push(codes::REVOCATION_CALLER_ASSERTED_UNVERIFIED.to_string());
        }
        return DimensionResult { status: StandingStatus::Forbidden, reason_codes: codes };
    }

    // (2a) Scope omitted: emit SCOPE_NOT_ASSERTED. For non-observe ops, this
    //      downgrades the verdict by marking standing as out_of_scope.
    let Some(scope) = intent.scope_assertion.as_ref() else {
        codes.push(codes::SCOPE_NOT_ASSERTED.to_string());
        if matches!(intent.operation_class, OperationClass::Observe) {
            // Read-only ops can proceed even without scope assertion.
        } else {
            return DimensionResult {
                status: StandingStatus::OutOfScope,
                reason_codes: codes,
            };
        }
        // Fall through with scope_omitted noted.
        return derive_standing_inner(intent, None, codes);
    };

    // (2b) Scope check: caller-asserted scope_includes_target=false → out_of_scope.
    if !scope.scope_includes_target {
        codes.push(codes::STANDING_OUT_OF_SCOPE.to_string());
        if scope.provenance == Provenance::CallerAsserted {
            codes.push(codes::SCOPE_CALLER_ASSERTED_UNVERIFIED.to_string());
        }
        return DimensionResult { status: StandingStatus::OutOfScope, reason_codes: codes };
    }

    derive_standing_inner(intent, Some(scope), codes)
}

fn derive_standing_inner(
    intent: &Intent,
    scope: Option<&ScopeAssertion>,
    mut codes: Vec<String>,
) -> DimensionResult<StandingStatus> {

    // (3) Standing class vs operation min_standing.
    let min = intent.operation_class.min_standing();
    if intent.actor_standing.class < min {
        codes.push(codes::STANDING_INSUFFICIENT_FOR_OPERATION.to_string());
        // Provenance flags emitted on the insufficient path for receipt completeness.
        if intent.actor_standing.provenance == Provenance::CallerAsserted {
            codes.push(codes::STANDING_CALLER_ASSERTED_UNVERIFIED.to_string());
        }
        if matches!(
            intent.operation_class,
            OperationClass::Authorize | OperationClass::Execute | OperationClass::Bind
        ) {
            codes.push(codes::STANDING_LADDER_V1_FLAT.to_string());
        }
        return DimensionResult { status: StandingStatus::Insufficient, reason_codes: codes };
    }

    // (4) Satisfied path.
    codes.push(codes::STANDING_OK.to_string());
    if intent.actor_standing.provenance == Provenance::CallerAsserted {
        codes.push(codes::STANDING_CALLER_ASSERTED_UNVERIFIED.to_string());
    }
    if let Some(s) = scope {
        if s.provenance == Provenance::CallerAsserted {
            codes.push(codes::SCOPE_CALLER_ASSERTED_UNVERIFIED.to_string());
        }
    }
    if intent.revocation.provenance == Provenance::CallerAsserted {
        codes.push(codes::REVOCATION_CALLER_ASSERTED_UNVERIFIED.to_string());
    }
    if matches!(
        intent.operation_class,
        OperationClass::Authorize | OperationClass::Execute | OperationClass::Bind
    ) {
        codes.push(codes::STANDING_LADDER_V1_FLAT.to_string());
    }

    DimensionResult { status: StandingStatus::Satisfied, reason_codes: codes }
}

// ---------------------------------------------------------------------------
// Future-provenance gate (drives unaccounted)
// ---------------------------------------------------------------------------

/// SPEC §4.2: `attested` and `verified` are schema-allowed but model-unaccounted
/// in v1. Returns true if any provenance field uses a reserved value.
pub fn has_future_provenance(intent: &Intent) -> bool {
    let reserved = |p: Provenance| matches!(p, Provenance::Attested | Provenance::Verified);
    reserved(intent.actor_standing.provenance)
        || intent
            .scope_assertion
            .as_ref()
            .map(|s| reserved(s.provenance))
            .unwrap_or(false)
        || reserved(intent.precedence.provenance)
        || reserved(intent.revocation.provenance)
}
