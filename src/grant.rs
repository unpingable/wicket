//! Standing grants — a wrapper-level convention, NOT kernel doctrine.
//!
//! A grant is a structured, bounded, content-addressed assertion that some
//! issuer has authorized an actor to operate at a given standing class over
//! a scope root, for a specified set of operation verbs, between two
//! timestamps. The wrapper validates the grant before cooking an Intent;
//! the kernel never sees the grant directly — it only sees the resulting
//! Intent (with the grant attached as `policy_ref` evidence).
//!
//! Doctrine: **standing must be granted, not asserted.** Raw `--standing
//! execute` remains available for test/dev but always carries
//! `STANDING_CALLER_ASSERTED_UNVERIFIED` in the receipt. Grants give the
//! receipt something concrete to point at.

use crate::model::{Evidence, EvidenceKind, StandingClass, ValidityStatus};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

pub const GRANT_SCHEMA_V1: &str = "wicket-grant/v1";

#[derive(Debug, Clone, Deserialize)]
pub struct Grant {
    pub schema: String,
    pub actor: String,
    pub standing: StandingClass,
    pub scope_root: PathBuf,
    pub operations: Vec<String>,
    pub issued_by: String,
    pub issued_at: String,
    pub expires_at: String,
    pub basis: String,
    #[serde(default)]
    pub grant_id: Option<String>,
}

#[derive(Debug)]
pub enum GrantLoadError {
    Io(std::io::Error),
    Parse(serde_json::Error),
}

impl std::fmt::Display for GrantLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GrantLoadError::Io(e) => write!(f, "grant file: {e}"),
            GrantLoadError::Parse(e) => write!(f, "grant parse: {e}"),
        }
    }
}

impl Grant {
    pub fn load(path: &Path) -> Result<Self, GrantLoadError> {
        let raw = std::fs::read_to_string(path).map_err(GrantLoadError::Io)?;
        serde_json::from_str(&raw).map_err(GrantLoadError::Parse)
    }
}

#[derive(Debug)]
pub enum GrantValidation {
    Valid,
    SchemaMismatch { found: String, expected: String },
    ActorMismatch { grant: String, intent: String },
    ScopeOutOfRange { scope_root: PathBuf, target: PathBuf },
    OperationNotPermitted { permitted: Vec<String>, requested: String },
    Expired { expires_at: String, now: String },
    NotYetValid { issued_at: String, now: String },
    UnparseableTimestamp { field: &'static str, value: String },
}

impl std::fmt::Display for GrantValidation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GrantValidation::Valid => write!(f, "valid"),
            GrantValidation::SchemaMismatch { found, expected } => {
                write!(f, "GRANT_SCHEMA_MISMATCH: found {found:?}, expected {expected:?}")
            }
            GrantValidation::ActorMismatch { grant, intent } => write!(
                f,
                "GRANT_ACTOR_MISMATCH: grant is for {grant:?}, intent actor is {intent:?}"
            ),
            GrantValidation::ScopeOutOfRange { scope_root, target } => write!(
                f,
                "GRANT_OUT_OF_SCOPE: scope_root={} does not contain target={}",
                scope_root.display(),
                target.display()
            ),
            GrantValidation::OperationNotPermitted { permitted, requested } => write!(
                f,
                "GRANT_OPERATION_NOT_PERMITTED: requested {requested:?} not in permitted {permitted:?}"
            ),
            GrantValidation::Expired { expires_at, now } => {
                write!(f, "GRANT_EXPIRED: expires_at={expires_at}, now={now}")
            }
            GrantValidation::NotYetValid { issued_at, now } => {
                write!(f, "GRANT_NOT_YET_VALID: issued_at={issued_at}, now={now}")
            }
            GrantValidation::UnparseableTimestamp { field, value } => {
                write!(f, "GRANT_UNPARSEABLE_TIMESTAMP: field={field} value={value:?}")
            }
        }
    }
}

/// Validate a grant against an intended invocation.
///
/// `target` is the path being acted on (for `edit`) or `cwd` (for `run`/`commit`).
/// `op` is one of the wrapper verbs: `"edit"`, `"run"`, `"commit"`.
pub fn validate(
    grant: &Grant,
    actor: &str,
    op: &str,
    target: &Path,
    now: DateTime<Utc>,
) -> GrantValidation {
    if grant.schema != GRANT_SCHEMA_V1 {
        return GrantValidation::SchemaMismatch {
            found: grant.schema.clone(),
            expected: GRANT_SCHEMA_V1.to_string(),
        };
    }
    if grant.actor != actor {
        return GrantValidation::ActorMismatch {
            grant: grant.actor.clone(),
            intent: actor.to_string(),
        };
    }
    if !grant.operations.iter().any(|o| o == op) {
        return GrantValidation::OperationNotPermitted {
            permitted: grant.operations.clone(),
            requested: op.to_string(),
        };
    }

    let issued_at = match parse_iso(&grant.issued_at) {
        Some(t) => t,
        None => {
            return GrantValidation::UnparseableTimestamp {
                field: "issued_at",
                value: grant.issued_at.clone(),
            };
        }
    };
    let expires_at = match parse_iso(&grant.expires_at) {
        Some(t) => t,
        None => {
            return GrantValidation::UnparseableTimestamp {
                field: "expires_at",
                value: grant.expires_at.clone(),
            };
        }
    };
    if now < issued_at {
        return GrantValidation::NotYetValid {
            issued_at: grant.issued_at.clone(),
            now: now.to_rfc3339(),
        };
    }
    if now > expires_at {
        return GrantValidation::Expired {
            expires_at: grant.expires_at.clone(),
            now: now.to_rfc3339(),
        };
    }

    let scope_root = match grant.scope_root.canonicalize() {
        Ok(p) => p,
        Err(_) => grant.scope_root.clone(),
    };
    let target_canon = target.canonicalize().unwrap_or_else(|_| target.to_path_buf());
    if !target_canon.starts_with(&scope_root) {
        return GrantValidation::ScopeOutOfRange {
            scope_root,
            target: target_canon,
        };
    }

    GrantValidation::Valid
}

fn parse_iso(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s).ok().map(|dt| dt.with_timezone(&Utc))
}

/// Build a `policy_ref` Evidence pointing at a validated grant. The
/// kernel sees this as ordinary policy evidence; the audit trail in the
/// receipt records that a grant was the basis for elevated standing.
pub fn grant_evidence(
    grant: &Grant,
    grant_path: &Path,
    actor: &str,
    now: DateTime<Utc>,
) -> Evidence {
    let id = grant
        .grant_id
        .clone()
        .unwrap_or_else(|| compute_grant_id(grant));
    let valid_until = parse_iso(&grant.expires_at).unwrap_or(now);
    Evidence {
        reference: format!(
            "wicket-grant://{}#{}",
            grant_path.display(),
            id
        ),
        kind: EvidenceKind::PolicyRef,
        issuer: grant.issued_by.clone(),
        subject: actor.to_string(),
        valid_from: grant.issued_at.clone(),
        valid_until: format_ts(valid_until),
        status: ValidityStatus::Valid,
    }
}

fn compute_grant_id(grant: &Grant) -> String {
    // Content-address the grant body (excluding grant_id itself).
    let body = serde_json::json!({
        "schema": grant.schema,
        "actor": grant.actor,
        "standing": grant.standing,
        "scope_root": grant.scope_root,
        "operations": grant.operations,
        "issued_by": grant.issued_by,
        "issued_at": grant.issued_at,
        "expires_at": grant.expires_at,
        "basis": grant.basis,
    });
    let canonical = serde_jcs::to_vec(&body).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(&canonical);
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(7 + 64);
    hex.push_str("sha256:");
    for b in digest {
        hex.push_str(&format!("{:02x}", b));
    }
    hex
}

fn format_ts(t: DateTime<Utc>) -> String {
    t.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}
