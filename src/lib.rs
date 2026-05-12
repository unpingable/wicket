//! Wicket — admissibility preflight for agentic operations.
//!
//! See SPEC.md for the canonical specification. This library is the
//! embarrassingly literal Rust translation of the spec.
//!
//! Public entry point: [`check`].

pub mod cook;
pub mod grant;
pub mod model;
pub mod receipt;
pub mod rules;
pub mod verdict;

pub use model::{
    BasisStatus, DimensionResult, Evidence, EvidenceKind, Intent, OperationClass, Outcome,
    OutcomeClass, PrecedenceResolution, PrecedenceStatus, Provenance, Receipt, ReceiptObligation,
    StandingClass, StandingStatus, SurfaceVerdict,
};

/// Run admissibility preflight on a single intent.
///
/// Returns a fully-formed [`Outcome`] including verdict, dimensional
/// accounting, allowed/forbidden actions, and a hash-chained receipt.
pub fn check(intent: &Intent) -> Outcome {
    verdict::derive(intent)
}
