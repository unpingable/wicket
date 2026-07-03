//! Diff hunks → `wicket::Intent` fan-out.
//!
//! For each authority-bearing surface touched by the diff, emit a
//! `bind`-class Intent with the diff itself as caller-asserted file_hash
//! evidence. Without explicit human confirmation, the kernel denies — and
//! that is the intended outcome for unauthorized identity/attribution
//! mutations.
//!
//! This module does **not** decide admissibility. It cooks ingredients in
//! the shape `wicket` expects.

use crate::diff::{FileChanges, ParsedDiff};
use crate::surfaces::{classify_path, AuthoritySurface};
use chrono::{Duration, Utc};
use sha2::{Digest, Sha256};
use wicket::model::{ActorStanding, ClaimedBasis, Precedence, Revocation, ValidityStatus};
use wicket::{
    Evidence, EvidenceKind, Intent, OperationClass, PrecedenceResolution, Provenance, StandingClass,
};

#[derive(Debug, Clone)]
pub struct CookedIntent {
    pub surface: AuthoritySurface,
    pub path: String,
    /// Specific sub-target detected within the surface (e.g.,
    /// `"copyright_holder"` for a LICENSE hunk that swaps the
    /// Copyright line). `None` when only the file as a whole is touched.
    pub sub_target: Option<String>,
    pub intent: Intent,
}

pub struct CookOpts<'a> {
    pub actor: &'a str,
    pub standing: StandingClass,
    pub because: &'a str,
}

pub fn cook_diff(diff: &ParsedDiff, opts: &CookOpts) -> Vec<CookedIntent> {
    let mut out = Vec::new();
    for (path, changes) in &diff.files {
        let Some(surface) = classify_path(path) else {
            continue;
        };
        let sub_target = detect_sub_target(surface, changes);
        let intent = build_intent(surface, path, sub_target.as_deref(), changes, opts);
        out.push(CookedIntent {
            surface,
            path: path.clone(),
            sub_target,
            intent,
        });
    }
    out
}

fn detect_sub_target(surface: AuthoritySurface, changes: &FileChanges) -> Option<String> {
    match surface {
        AuthoritySurface::License => {
            let removed = changes.removed.iter().any(|l| is_copyright_line(l));
            let added = changes.added.iter().any(|l| is_copyright_line(l));
            if removed && added {
                Some("copyright_holder".to_string())
            } else {
                None
            }
        }
        _ => None,
    }
}

fn is_copyright_line(line: &str) -> bool {
    let t = line.trim_start();
    t.starts_with("Copyright")
        || t.starts_with("(c)")
        || t.starts_with("(C)")
        || t.starts_with("\u{00A9}")
}

fn build_intent(
    surface: AuthoritySurface,
    path: &str,
    sub_target: Option<&str>,
    changes: &FileChanges,
    opts: &CookOpts,
) -> Intent {
    let now = Utc::now();
    let ts = fmt_ts(now);
    let target = match sub_target {
        Some(sub) => format!("{path}:{sub}"),
        None => path.to_string(),
    };

    let diff_hash = hash_changes(changes);
    let diff_evidence = Evidence {
        reference: format!("diff://{path}#sha256:{diff_hash}"),
        kind: EvidenceKind::FileHash,
        issuer: "wicket-guard".to_string(),
        subject: opts.actor.to_string(),
        valid_from: ts.clone(),
        valid_until: fmt_ts(now + Duration::minutes(60)),
        status: ValidityStatus::Valid,
    };

    let intended_action = match surface {
        AuthoritySurface::License => "diff.license_mutation",
        AuthoritySurface::Notice => "diff.notice_mutation",
        AuthoritySurface::Codeowners => "diff.codeowners_mutation",
        AuthoritySurface::SecurityPolicy => "diff.security_policy_mutation",
    };

    Intent {
        actor: opts.actor.to_string(),
        actor_standing: ActorStanding {
            class: opts.standing,
            provenance: Provenance::CallerAsserted,
        },
        intended_action: intended_action.to_string(),
        operation_class: OperationClass::Bind,
        target,
        scope_assertion: None,
        claimed_basis: ClaimedBasis {
            rule: opts.because.to_string(),
            evidence_refs: vec![diff_evidence],
        },
        precedence: Precedence {
            resolution: PrecedenceResolution::Active,
            superseded_by: None,
            provenance: Provenance::CallerAsserted,
            evidence_refs: vec![],
        },
        revocation: Revocation {
            basis_revoked: false,
            standing_forbidden: false,
            provenance: Provenance::CallerAsserted,
            evidence_refs: vec![],
        },
        expected_effect: describe_effect(surface, sub_target),
        call_timestamp: ts,
        prev_receipt_hash: None,
    }
}

fn describe_effect(surface: AuthoritySurface, sub_target: Option<&str>) -> String {
    match (surface, sub_target) {
        (AuthoritySurface::License, Some("copyright_holder")) => {
            "mutate LICENSE copyright holder".to_string()
        }
        (AuthoritySurface::License, _) => "mutate LICENSE contents".to_string(),
        (AuthoritySurface::Notice, _) => "mutate NOTICE contents".to_string(),
        (AuthoritySurface::Codeowners, _) => "mutate CODEOWNERS".to_string(),
        (AuthoritySurface::SecurityPolicy, _) => "mutate SECURITY policy".to_string(),
    }
}

fn hash_changes(c: &FileChanges) -> String {
    let mut h = Sha256::new();
    for l in &c.removed {
        h.update("-");
        h.update(l.as_bytes());
        h.update("\n");
    }
    for l in &c.added {
        h.update("+");
        h.update(l.as_bytes());
        h.update("\n");
    }
    let d = h.finalize();
    d.iter().map(|b| format!("{:02x}", b)).collect()
}

fn fmt_ts(t: chrono::DateTime<Utc>) -> String {
    t.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}
