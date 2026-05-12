//! Wicket data model — input intent, output verdict, dimensional accounting, receipt.
//!
//! Mirrors SPEC.md §4 (input), §6 (dimensions), §8 (output). Field names and
//! enum variants are wire-stable; renaming requires a SPEC.md amendment in the
//! same commit.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Input — §4
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Intent {
    pub actor: String,
    pub actor_standing: ActorStanding,
    pub intended_action: String,
    pub operation_class: OperationClass,
    pub target: String,
    /// Optional per SPEC §4.1 / §4.6. When omitted, Wicket emits
    /// `SCOPE_NOT_ASSERTED` and downgrades the verdict for non-observe ops.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope_assertion: Option<ScopeAssertion>,
    pub claimed_basis: ClaimedBasis,
    pub precedence: Precedence,
    pub revocation: Revocation,
    pub expected_effect: String,
    pub call_timestamp: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prev_receipt_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActorStanding {
    pub class: StandingClass,
    pub provenance: Provenance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScopeAssertion {
    pub scope_includes_target: bool,
    pub provenance: Provenance,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimedBasis {
    pub rule: String,
    #[serde(default)]
    pub evidence_refs: Vec<Evidence>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    #[serde(rename = "ref")]
    pub reference: String,
    pub kind: EvidenceKind,
    pub issuer: String,
    pub subject: String,
    pub valid_from: String,
    pub valid_until: String,
    pub status: ValidityStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Precedence {
    pub resolution: PrecedenceResolution,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub superseded_by: Option<String>,
    pub provenance: Provenance,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Revocation {
    pub basis_revoked: bool,
    pub standing_forbidden: bool,
    pub provenance: Provenance,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

// ---------------------------------------------------------------------------
// Input enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OperationClass {
    Observe,
    Interpret,
    Recommend,
    Authorize,
    Execute,
    Bind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StandingClass {
    // Order matters: derive PartialOrd from declaration order so the
    // strict total order in SPEC §5.1 falls out automatically:
    // observe < interpret < recommend < authorize < execute.
    Observe,
    Interpret,
    Recommend,
    Authorize,
    Execute,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Provenance {
    CallerAsserted,
    Attested,
    Verified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrecedenceResolution {
    Active,
    Superseded,
    Ambiguous,
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    Prompt,
    FileHash,
    TestLog,
    ToolOutput,
    ToolTrace,
    CommandOutput,
    PolicyRef,
    PriorReceipt,
    HumanConfirmation,
    ActorAssertion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValidityStatus {
    Valid,
    Stale,
    Unavailable,
}

// ---------------------------------------------------------------------------
// Output — §8
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Outcome {
    pub class: OutcomeClass,
    pub surface_verdict: SurfaceVerdict,
    pub operation_class: OperationClass,
    pub dimensions: Dimensions,
    pub reason_codes: Vec<String>,
    pub allowed: Vec<String>,
    pub forbidden: Vec<String>,
    pub receipt: Receipt,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dimensions {
    pub basis: DimensionResult<BasisStatus>,
    pub precedence: DimensionResult<PrecedenceStatus>,
    pub standing: DimensionResult<StandingStatus>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionResult<S> {
    pub status: S,
    pub reason_codes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Receipt {
    pub receipt_id: String,
    pub obligation: ReceiptObligation,
    pub input_hash: String,
    pub evidence_ref_hashes: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prev_receipt_hash: Option<String>,
    pub timestamp: String,
}

// ---------------------------------------------------------------------------
// Output enums
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutcomeClass {
    Verdict,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SurfaceVerdict {
    Authorized,
    AdvisoryOnly,
    Denied,
    Gap,
    Unaccounted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BasisStatus {
    // §6.1: SOFT bucket — surface_verdict = gap
    Satisfied,
    Insufficient,
    Stale,
    Absent,
    Ambiguous,
    // HARD bucket — surface_verdict = denied
    Inadmissible,
    Revoked,
}

impl BasisStatus {
    pub fn is_hard(self) -> bool {
        matches!(self, BasisStatus::Inadmissible | BasisStatus::Revoked)
    }
    pub fn is_soft_unsatisfied(self) -> bool {
        matches!(
            self,
            BasisStatus::Insufficient
                | BasisStatus::Stale
                | BasisStatus::Absent
                | BasisStatus::Ambiguous
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrecedenceStatus {
    Satisfied,
    Superseded,
    Ambiguous,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StandingStatus {
    Satisfied,
    Insufficient,
    Absent,
    OutOfScope,
    Forbidden,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReceiptObligation {
    ActionReceipt,
    AdvisoryReceipt,
    RefusalReceipt,
    GapReceipt,
    ErrorReceipt,
}

// ---------------------------------------------------------------------------
// OperationClass helpers
// ---------------------------------------------------------------------------

impl OperationClass {
    /// SPEC §5: minimum standing required to perform this operation class.
    /// `Bind` requires `Execute` standing plus the hard `human_confirmation`
    /// rule in §7.3 (handled in `rules.rs`, not here).
    pub fn min_standing(self) -> StandingClass {
        match self {
            OperationClass::Observe => StandingClass::Observe,
            OperationClass::Interpret => StandingClass::Interpret,
            OperationClass::Recommend => StandingClass::Recommend,
            OperationClass::Authorize => StandingClass::Authorize,
            OperationClass::Execute => StandingClass::Execute,
            OperationClass::Bind => StandingClass::Execute,
        }
    }

    pub fn as_snake(self) -> &'static str {
        match self {
            OperationClass::Observe => "observe",
            OperationClass::Interpret => "interpret",
            OperationClass::Recommend => "recommend",
            OperationClass::Authorize => "authorize",
            OperationClass::Execute => "execute",
            OperationClass::Bind => "bind",
        }
    }
}
