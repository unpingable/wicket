#!/usr/bin/env python3
"""Express a Wicket verdict as a signed in-toto v1 attestation (DSSE envelope).

The mapping is the whole argument for "interoperate, don't reinvent":

  in-toto concept      <-  Wicket concept
  -----------------------------------------------------------------
  subject (artifact)   <-  the INTENT, addressed by its content-hash
                           (exactly the `input_hash` Wicket already mints)
  predicateType        <-  "this is a Wicket admissibility verdict"
  predicate            <-  the verdict body (surface_verdict, dimensions,
                           allowed/forbidden, receipt)

Because the subject digest IS Wicket's own input_hash, any in-toto verifier
checking "does this attestation's subject match my artifact?" is checking the
same content-address Wicket minted. The two systems agree on the address of the
evidence without either one trusting the other.

Output: a DSSE envelope (the format `cosign attest` produces and
`cosign verify-attestation` consumes), written as <out>.intoto.jsonl.

Usage: wicket_to_intoto.py <verdict.json> <intent.json> <out_prefix>
"""
import base64
import hashlib
import json
import sys
from pathlib import Path

import jcs
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey
from cryptography.hazmat.primitives import serialization

PREDICATE_TYPE = "https://neutral.zone/predicates/wicket-admissibility/v0.1"
PAYLOAD_TYPE = "application/vnd.in-toto+json"


def pae(payload_type: str, payload: bytes) -> bytes:
    """DSSE Pre-Authentication Encoding (the bytes that actually get signed)."""
    t = payload_type.encode()
    return b"DSSEv1 %d %s %d %s" % (len(t), t, len(payload), payload)


def main():
    verdict = json.load(open(sys.argv[1]))
    intent = json.load(open(sys.argv[2]))
    out_prefix = sys.argv[3]

    # Subject = the intent, addressed by the SAME hash Wicket put in the receipt.
    input_hash = verdict["receipt"]["input_hash"]          # "sha256:<hex>"
    algo, hexdigest = input_hash.split(":", 1)
    subject_name = f"intent:{intent.get('intended_action','?')}:{intent.get('target','?')}"

    statement = {
        "_type": "https://in-toto.io/Statement/v1",
        "subject": [{"name": subject_name, "digest": {algo: hexdigest}}],
        "predicateType": PREDICATE_TYPE,
        "predicate": verdict,
    }
    payload = jcs.canonicalize(statement)  # deterministic bytes

    # Sign with a local Ed25519 key (keyed cosign style; prefer keyed signing
    # over keyless/Fulcio — see the signing-identity note in INTEROP.md).
    keydir = Path(out_prefix).parent / "keys"
    keydir.mkdir(parents=True, exist_ok=True)
    priv_path = keydir / "ed25519.key"
    pub_path = keydir / "ed25519.pub"
    if priv_path.exists():
        priv = serialization.load_pem_private_key(priv_path.read_bytes(), password=None)
    else:
        priv = Ed25519PrivateKey.generate()
        priv_path.write_bytes(priv.private_bytes(
            serialization.Encoding.PEM,
            serialization.PrivateFormat.PKCS8,
            serialization.NoEncryption()))
        pub = priv.public_key()
        pub_path.write_bytes(pub.public_bytes(
            serialization.Encoding.PEM, serialization.PublicFormat.SubjectPublicKeyInfo))

    sig = priv.sign(pae(PAYLOAD_TYPE, payload))
    keyid = "sha256:" + hashlib.sha256(
        priv.public_key().public_bytes(
            serialization.Encoding.Raw, serialization.PublicFormat.Raw)).hexdigest()

    envelope = {
        "payload": base64.b64encode(payload).decode(),
        "payloadType": PAYLOAD_TYPE,
        "signatures": [{"keyid": keyid, "sig": base64.b64encode(sig).decode()}],
    }

    out = f"{out_prefix}.intoto.jsonl"
    Path(out).write_text(json.dumps(envelope) + "\n")
    print("wrote DSSE envelope:", out)
    print("  payloadType  :", PAYLOAD_TYPE)
    print("  predicateType:", PREDICATE_TYPE)
    print("  subject      :", subject_name)
    print("  subject digest:", algo + ":" + hexdigest, "  <- == Wicket input_hash")
    print("  public key   :", pub_path)
    print()
    print("--- the in-toto Statement (predicate truncated) ---")
    shown = dict(statement)
    shown["predicate"] = {k: statement["predicate"][k]
                          for k in ("class", "surface_verdict", "operation_class")}
    shown["predicate"]["…"] = "(full verdict body)"
    print(json.dumps(shown, indent=2))


if __name__ == "__main__":
    main()
