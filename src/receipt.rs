//! Receipts — RFC 8785 canonical JSON, sha256 hashing, content addressing.
//!
//! SPEC.md §8.1:
//! - `receipt.receipt_id` = sha256 of the verdict body excluding `receipt_id`,
//!   including `receipt.timestamp`.
//! - `receipt.evidence_ref_hashes` = sha256 of each evidence reference object
//!   (NOT the dereferenced evidence content; Wicket does not dereference).
//! - Canonical JSON is RFC 8785 (JCS).

use crate::model::*;
use serde::Serialize;
use sha2::{Digest, Sha256};

/// Hash any serde-serializable value via RFC 8785 canonical JSON.
pub fn hash_value<T: Serialize>(value: &T) -> String {
    let canonical = serde_jcs::to_vec(value).expect("canonical JSON serialization");
    sha256_hex(&canonical)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(7 + 64);
    hex.push_str("sha256:");
    for b in digest {
        hex.push_str(&format!("{:02x}", b));
    }
    hex
}

/// Build a partial receipt with `receipt_id` left empty. Caller fills it
/// via [`seal`] after assembling the full [`Outcome`].
pub fn build(
    intent: &Intent,
    _dimensions: &Dimensions,
    _top_codes: &[String],
    obligation: ReceiptObligation,
) -> Receipt {
    let input_hash = hash_value(intent);
    let evidence_ref_hashes: Vec<String> = intent
        .claimed_basis
        .evidence_refs
        .iter()
        .map(hash_value)
        .collect();
    Receipt {
        receipt_id: String::new(),
        obligation,
        input_hash,
        evidence_ref_hashes,
        prev_receipt_hash: intent.prev_receipt_hash.clone(),
        timestamp: intent.call_timestamp.clone(),
    }
}

/// Compute and set `outcome.receipt.receipt_id` over the canonical JSON of
/// the entire outcome with `receipt_id` elided.
pub fn seal(outcome: &mut Outcome) {
    outcome.receipt.receipt_id = String::new();
    let canonical =
        serde_jcs::to_vec(outcome).expect("canonical JSON serialization of outcome");
    outcome.receipt.receipt_id = sha256_hex(&canonical);
}
