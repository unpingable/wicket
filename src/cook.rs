//! Thin adapter — turns verb-shaped fact packets into [`Intent`]s for the kernel.
//!
//! Doctrine: **never make the operator manufacture facts Wicket is supposed
//! to test.** The cook module gathers what it can from the filesystem and
//! environment, omits what it can't determine (so the kernel emits an
//! honest gap), and tags every assertion as `caller_asserted` so the
//! existing `*_CALLER_ASSERTED_UNVERIFIED` codes carry the unverified-ness.
//!
//! This module does *not* expand the kernel's authority; it is a small
//! kitchen that prepares ingredients in the shape the kernel expects.

use crate::model::*;
use chrono::{Duration, Utc};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Fact packets — what the user supplies
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct EditPacket {
    pub actor: String,
    pub standing: StandingClass,
    pub because: String,
    pub target: PathBuf,
    pub cwd: PathBuf,
    pub policy_paths: Vec<PathBuf>,
    pub human_confirmation: Option<String>,
    pub precedence: PrecedenceResolution,
    /// Optional pre-validated grant evidence (caller has already validated
    /// the grant; the wrapper just attaches it as `policy_ref` evidence).
    pub grant_evidence: Option<Evidence>,
}

#[derive(Debug, Clone)]
pub struct RunPacket {
    pub actor: String,
    pub standing: StandingClass,
    pub because: String,
    pub command: String,
    pub operation_class: OperationClass,
    pub cwd: PathBuf,
    pub policy_paths: Vec<PathBuf>,
    pub human_confirmation: Option<String>,
    pub precedence: PrecedenceResolution,
    pub grant_evidence: Option<Evidence>,
}

#[derive(Debug, Clone)]
pub struct CommitPacket {
    pub actor: String,
    pub standing: StandingClass,
    pub because: String,
    pub operation_class: OperationClass,
    pub cwd: PathBuf,
    pub policy_paths: Vec<PathBuf>,
    pub human_confirmation: Option<String>,
    pub precedence: PrecedenceResolution,
    pub grant_evidence: Option<Evidence>,
}

// ---------------------------------------------------------------------------
// Cook functions
// ---------------------------------------------------------------------------

pub fn cook_edit(p: EditPacket) -> Intent {
    let now = Utc::now();
    let call_ts = format_ts(now);
    let mut evidence = gather_policy_refs(&p.policy_paths, now, &p.actor);

    // Read target file (if it exists) and pin its hash.
    if let Some(ev) = file_hash_evidence(&p.target, now, &p.actor) {
        evidence.push(ev);
    }

    if let Some(token) = p.human_confirmation {
        evidence.push(human_conf_evidence(&token, now, &p.actor));
    }

    if let Some(g) = p.grant_evidence {
        evidence.push(g);
    }

    Intent {
        actor: p.actor.clone(),
        actor_standing: ActorStanding {
            class: p.standing,
            provenance: Provenance::CallerAsserted,
        },
        intended_action: "fs.edit".into(),
        operation_class: OperationClass::Execute,
        target: display_path(&p.target),
        scope_assertion: derive_scope(&p.target, &p.cwd, &p.policy_paths),
        claimed_basis: ClaimedBasis {
            rule: p.because,
            evidence_refs: evidence,
        },
        precedence: precedence_for(p.precedence),
        revocation: default_revocation(),
        expected_effect: format!("modify {}", display_path(&p.target)),
        call_timestamp: call_ts,
        prev_receipt_hash: None,
    }
}

pub fn cook_run(p: RunPacket) -> Intent {
    let now = Utc::now();
    let call_ts = format_ts(now);
    let mut evidence = gather_policy_refs(&p.policy_paths, now, &p.actor);

    // The command itself is not evidence; we record it as the intended
    // action. Caller can supply human_confirmation for stronger ops.
    if let Some(token) = p.human_confirmation {
        evidence.push(human_conf_evidence(&token, now, &p.actor));
    }

    if let Some(g) = p.grant_evidence {
        evidence.push(g);
    }

    Intent {
        actor: p.actor.clone(),
        actor_standing: ActorStanding {
            class: p.standing,
            provenance: Provenance::CallerAsserted,
        },
        intended_action: format!("shell.run: {}", p.command),
        operation_class: p.operation_class,
        target: p.command.clone(),
        scope_assertion: scope_for_cwd(&p.cwd, &p.policy_paths),
        claimed_basis: ClaimedBasis {
            rule: p.because,
            evidence_refs: evidence,
        },
        precedence: precedence_for(p.precedence),
        revocation: default_revocation(),
        expected_effect: format!("execute command: {}", p.command),
        call_timestamp: call_ts,
        prev_receipt_hash: None,
    }
}

pub fn cook_commit(p: CommitPacket) -> Intent {
    let now = Utc::now();
    let call_ts = format_ts(now);
    let mut evidence = gather_policy_refs(&p.policy_paths, now, &p.actor);

    // Capture `git status --porcelain` as command_output evidence if available.
    if let Some(ev) = git_status_evidence(&p.cwd, now, &p.actor) {
        evidence.push(ev);
    }

    if let Some(token) = p.human_confirmation {
        evidence.push(human_conf_evidence(&token, now, &p.actor));
    }

    if let Some(g) = p.grant_evidence {
        evidence.push(g);
    }

    Intent {
        actor: p.actor.clone(),
        actor_standing: ActorStanding {
            class: p.standing,
            provenance: Provenance::CallerAsserted,
        },
        intended_action: "git.commit".into(),
        operation_class: p.operation_class,
        target: format!("{}@HEAD", display_path(&p.cwd)),
        scope_assertion: scope_for_cwd(&p.cwd, &p.policy_paths),
        claimed_basis: ClaimedBasis {
            rule: p.because,
            evidence_refs: evidence,
        },
        precedence: precedence_for(p.precedence),
        revocation: default_revocation(),
        expected_effect: "create a git commit on the current branch".into(),
        call_timestamp: call_ts,
        prev_receipt_hash: None,
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Look for repo-shaped policy docs (CLAUDE.md, SPEC.md, README.md) in
/// `cwd` and any user-supplied `policy_paths`. Returns auto-detected paths
/// in cwd that are not already in `extras`.
pub fn auto_detect_policy_paths(cwd: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for name in ["CLAUDE.md", "SPEC.md", "README.md"] {
        let p = cwd.join(name);
        if p.exists() {
            out.push(p);
        }
    }
    out
}

fn gather_policy_refs(
    policy_paths: &[PathBuf],
    now: chrono::DateTime<Utc>,
    actor: &str,
) -> Vec<Evidence> {
    let mut out = Vec::new();
    for p in policy_paths {
        if let Some(ev) = policy_ref_evidence(p, now, actor) {
            out.push(ev);
        }
    }
    out
}

fn policy_ref_evidence(
    path: &Path,
    now: chrono::DateTime<Utc>,
    actor: &str,
) -> Option<Evidence> {
    if !path.exists() {
        return None;
    }
    Some(Evidence {
        reference: format!("policy://{}", display_path(path)),
        kind: EvidenceKind::PolicyRef,
        issuer: "repo-owner".into(),
        subject: actor.into(),
        valid_from: format_ts(now - Duration::minutes(60)),
        valid_until: format_ts(now + Duration::minutes(60)),
        status: ValidityStatus::Valid,
    })
}

fn file_hash_evidence(
    path: &Path,
    now: chrono::DateTime<Utc>,
    actor: &str,
) -> Option<Evidence> {
    let bytes = std::fs::read(path).ok()?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let digest = hasher.finalize();
    let hex: String = digest.iter().map(|b| format!("{:02x}", b)).collect();
    Some(Evidence {
        reference: format!("file://{}#sha256:{}", display_path(path), hex),
        kind: EvidenceKind::FileHash,
        issuer: "filesystem".into(),
        subject: actor.into(),
        valid_from: format_ts(now),
        valid_until: format_ts(now + Duration::minutes(60)),
        status: ValidityStatus::Valid,
    })
}

fn human_conf_evidence(
    token: &str,
    now: chrono::DateTime<Utc>,
    actor: &str,
) -> Evidence {
    Evidence {
        reference: format!("human://{token}"),
        kind: EvidenceKind::HumanConfirmation,
        issuer: "operator".into(),
        subject: actor.into(),
        valid_from: format_ts(now - Duration::minutes(5)),
        valid_until: format_ts(now + Duration::minutes(60)),
        status: ValidityStatus::Valid,
    }
}

fn git_status_evidence(
    cwd: &Path,
    now: chrono::DateTime<Utc>,
    actor: &str,
) -> Option<Evidence> {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(cwd)
        .args(["status", "--porcelain"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let body = out.stdout;
    let mut hasher = Sha256::new();
    hasher.update(&body);
    let digest = hasher.finalize();
    let hex: String = digest.iter().map(|b| format!("{:02x}", b)).collect();
    Some(Evidence {
        reference: format!("git://{}#status:sha256:{}", display_path(cwd), hex),
        kind: EvidenceKind::CommandOutput,
        issuer: "git".into(),
        subject: actor.into(),
        valid_from: format_ts(now),
        valid_until: format_ts(now + Duration::minutes(60)),
        status: ValidityStatus::Valid,
    })
}

/// Caller-cooked scope: target inside cwd AND cwd has at least one
/// policy doc → assert true. Otherwise omit (so kernel emits SCOPE_NOT_ASSERTED).
fn derive_scope(
    target: &Path,
    cwd: &Path,
    policy_paths: &[PathBuf],
) -> Option<ScopeAssertion> {
    let target_abs = match target.canonicalize() {
        Ok(p) => p,
        // Target doesn't exist yet (e.g., new file); use lexical containment.
        Err(_) => match cwd.canonicalize() {
            Ok(c) => c.join(target),
            Err(_) => return None,
        },
    };
    let cwd_abs = cwd.canonicalize().ok()?;
    let inside = target_abs.starts_with(&cwd_abs);
    if !inside || policy_paths.is_empty() {
        return None;
    }
    Some(ScopeAssertion {
        scope_includes_target: true,
        provenance: Provenance::CallerAsserted,
        evidence_refs: policy_paths
            .iter()
            .map(|p| format!("policy://{}", display_path(p)))
            .collect(),
    })
}

fn scope_for_cwd(cwd: &Path, policy_paths: &[PathBuf]) -> Option<ScopeAssertion> {
    if policy_paths.is_empty() {
        return None;
    }
    Some(ScopeAssertion {
        scope_includes_target: true,
        provenance: Provenance::CallerAsserted,
        evidence_refs: policy_paths
            .iter()
            .map(|p| format!("policy://{}", display_path(p)))
            .chain(std::iter::once(format!(
                "cwd://{}",
                display_path(cwd)
            )))
            .collect(),
    })
}

/// The wrapper-supplied precedence assertion. Default is `active` — the
/// invoker implicitly asserts "I am acting under what I believe is the
/// active rule set." For genuine uncertainty, the caller passes `unresolved`.
fn precedence_for(resolution: PrecedenceResolution) -> Precedence {
    Precedence {
        resolution,
        superseded_by: None,
        provenance: Provenance::CallerAsserted,
        evidence_refs: vec![],
    }
}

fn default_revocation() -> Revocation {
    Revocation {
        basis_revoked: false,
        standing_forbidden: false,
        provenance: Provenance::CallerAsserted,
        evidence_refs: vec![],
    }
}

fn format_ts(t: chrono::DateTime<Utc>) -> String {
    t.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

fn display_path(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}
