use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use ed25519_dalek::Signature;
use ed25519_dalek::Signer as _;
use ed25519_dalek::SigningKey;
use ed25519_dalek::Verifier as _;
use ed25519_dalek::VerifyingKey;

use super::types::EvidenceV2;
use super::types::MerkleRoot;
use super::types::SigAlgorithm;
use super::types::SigMode;
use super::types::SignatureEnvelope;
use super::types::XrefMarker;

const BROKER_SECRET_ENV: &str = "VAC_EVIDENCE_V2_BROKER_ED25519_SECRET_BASE64";
const OPERATOR_SECRET_ENV: &str = "VAC_EVIDENCE_V2_OPERATOR_ED25519_SECRET_BASE64";
const KEY_ID_PREFIX: &str = "ed25519:";

#[derive(Debug, Clone)]
pub struct SigningIdentity {
    signing_key: SigningKey,
}

impl SigningIdentity {
    pub fn from_raw_secret_base64(secret_base64: &str) -> Result<Self, String> {
        let bytes = STANDARD
            .decode(secret_base64)
            .map_err(|err| format!("invalid ed25519 secret base64: {err}"))?;
        let secret: [u8; 32] = bytes
            .as_slice()
            .try_into()
            .map_err(|_| "ed25519 secret must decode to exactly 32 bytes".to_string())?;
        Ok(Self {
            signing_key: SigningKey::from_bytes(&secret),
        })
    }

    pub fn from_raw_secret_bytes(secret: [u8; 32]) -> Self {
        Self {
            signing_key: SigningKey::from_bytes(&secret),
        }
    }

    pub fn key_id(&self) -> String {
        format!(
            "{KEY_ID_PREFIX}{}",
            STANDARD.encode(self.signing_key.verifying_key().as_bytes())
        )
    }

    pub fn sign_payload(&self, payload: &str) -> SignatureEnvelope {
        SignatureEnvelope::signed_ed25519(
            self.key_id(),
            STANDARD.encode(self.signing_key.sign(payload.as_bytes()).to_bytes()),
        )
    }
}

#[derive(Debug, Clone, Default)]
pub struct EvidenceSigner {
    broker: Option<SigningIdentity>,
    operator: Option<SigningIdentity>,
}

impl EvidenceSigner {
    pub fn integrity_hint_only() -> Self {
        Self::default()
    }

    pub fn from_env() -> Self {
        let broker = std::env::var(BROKER_SECRET_ENV)
            .ok()
            .and_then(|value| match SigningIdentity::from_raw_secret_base64(&value) {
                Ok(identity) => Some(identity),
                Err(err) => {
                    tracing::warn!(%err, env = BROKER_SECRET_ENV, "ignoring invalid evidence v2 broker signing key");
                    None
                }
            });
        let operator = std::env::var(OPERATOR_SECRET_ENV)
            .ok()
            .and_then(|value| match SigningIdentity::from_raw_secret_base64(&value) {
                Ok(identity) => Some(identity),
                Err(err) => {
                    tracing::warn!(%err, env = OPERATOR_SECRET_ENV, "ignoring invalid evidence v2 operator signing key");
                    None
                }
            });
        Self { broker, operator }
    }

    pub fn require_broker_and_operator_from_env() -> Result<Self, String> {
        let signer = Self::from_env();
        if signer.broker.is_none() {
            return Err(format!(
                "{BROKER_SECRET_ENV} must be set to a base64-encoded 32-byte Ed25519 secret"
            ));
        }
        if signer.operator.is_none() {
            return Err(format!(
                "{OPERATOR_SECRET_ENV} must be set to a base64-encoded 32-byte Ed25519 secret"
            ));
        }
        Ok(signer)
    }

    pub fn has_broker(&self) -> bool {
        self.broker.is_some()
    }

    pub fn has_operator(&self) -> bool {
        self.operator.is_some()
    }

    pub fn with_broker_for_tests(secret: [u8; 32]) -> Self {
        Self {
            broker: Some(SigningIdentity::from_raw_secret_bytes(secret)),
            operator: None,
        }
    }

    pub fn with_broker_and_operator_for_tests(broker: [u8; 32], operator: [u8; 32]) -> Self {
        Self {
            broker: Some(SigningIdentity::from_raw_secret_bytes(broker)),
            operator: Some(SigningIdentity::from_raw_secret_bytes(operator)),
        }
    }

    pub fn sign_evidence(&self, record: &mut EvidenceV2) {
        record.broker_sig = self
            .broker
            .as_ref()
            .map_or_else(broker_integrity_hint, |identity| {
                identity.sign_payload(&record.sub_chain.self_hash)
            });
        record.operator_sig = self
            .operator
            .as_ref()
            .map(|identity| identity.sign_payload(&record.sub_chain.self_hash))
            .or_else(|| record.operator_sig.take());
    }

    pub fn sign_xref(&self, marker: &mut XrefMarker) {
        marker.broker_sig = self
            .broker
            .as_ref()
            .map_or_else(broker_integrity_hint, |identity| {
                identity.sign_payload(&marker.sub_chain.self_hash)
            });
    }

    pub fn sign_merkle_root(&self, root: &mut MerkleRoot) {
        root.broker_sig = self
            .broker
            .as_ref()
            .map_or_else(broker_integrity_hint, |identity| {
                identity.sign_payload(&root.root_hash)
            });
    }
}

pub fn broker_integrity_hint() -> SignatureEnvelope {
    SignatureEnvelope::integrity_hint("broker.local")
}

pub fn operator_integrity_hint() -> SignatureEnvelope {
    SignatureEnvelope::integrity_hint("operator.local")
}

pub fn verify_signature_payload(payload: &str, envelope: &SignatureEnvelope) -> Result<(), String> {
    match envelope.mode {
        SigMode::IntegrityHint => {
            if envelope.algorithm != SigAlgorithm::None {
                return Err("integrity-hint signature must use algorithm none".to_string());
            }
            if envelope.value.is_some() {
                return Err("integrity-hint signature must not carry a value".to_string());
            }
            Ok(())
        }
        SigMode::Signed => {
            if envelope.algorithm != SigAlgorithm::Ed25519 {
                return Err("signed evidence v2 envelope must use ed25519".to_string());
            }
            let public_key = envelope
                .key_id
                .strip_prefix(KEY_ID_PREFIX)
                .ok_or_else(|| "ed25519 key_id must be prefixed with ed25519:".to_string())?;
            let public_key = STANDARD
                .decode(public_key)
                .map_err(|err| format!("invalid ed25519 public key in key_id: {err}"))?;
            let public_key: [u8; 32] = public_key
                .as_slice()
                .try_into()
                .map_err(|_| "ed25519 public key must decode to exactly 32 bytes".to_string())?;
            let verifying_key = VerifyingKey::from_bytes(&public_key)
                .map_err(|err| format!("invalid ed25519 public key: {err}"))?;
            let signature = envelope
                .value
                .as_deref()
                .ok_or_else(|| "signed evidence v2 envelope is missing value".to_string())?;
            let signature = STANDARD
                .decode(signature)
                .map_err(|err| format!("invalid ed25519 signature base64: {err}"))?;
            let signature = Signature::from_slice(&signature)
                .map_err(|err| format!("invalid ed25519 signature bytes: {err}"))?;
            verifying_key
                .verify(payload.as_bytes(), &signature)
                .map_err(|err| format!("ed25519 signature verification failed: {err}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ed25519_signature_round_trips_and_rejects_tamper() {
        let identity = SigningIdentity::from_raw_secret_bytes([7u8; 32]);
        let envelope = identity.sign_payload("payload-hash");

        assert_eq!(envelope.algorithm, SigAlgorithm::Ed25519);
        assert_eq!(envelope.mode, SigMode::Signed);
        verify_signature_payload("payload-hash", &envelope).expect("valid signature");
        assert!(verify_signature_payload("other-payload", &envelope).is_err());
    }

    #[test]
    fn integrity_hint_is_structurally_verified_without_claiming_signature() {
        verify_signature_payload("payload-hash", &broker_integrity_hint())
            .expect("valid integrity hint");

        let mut invalid = broker_integrity_hint();
        invalid.algorithm = SigAlgorithm::Ed25519;
        assert!(verify_signature_payload("payload-hash", &invalid).is_err());
    }
}
