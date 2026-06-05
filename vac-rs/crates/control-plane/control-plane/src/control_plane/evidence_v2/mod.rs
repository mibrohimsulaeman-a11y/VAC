//! Signed evidence v2 support.
//!
//! Evidence v2 keeps hashes on JSON canonical payloads and stores records as
//! per-capability chains. YAML remains a persistence format, not the hashing
//! authority.

pub mod doctor;
pub mod hash;
pub mod jcs;
pub mod merkle;
pub mod migration;
pub mod signing;
pub mod store;
pub mod types;

pub use doctor::EvidenceV2DoctorReport;
pub use doctor::load_evidence_v2_doctor_report;
pub use hash::hash_evidence_v2;
pub use hash::hash_merkle_root;
pub use hash::hash_xref_marker;
pub use migration::render_evidence_v1_to_v2_migration_yaml;
pub use signing::EvidenceSigner;
pub use signing::SigningIdentity;
pub use signing::verify_signature_payload;
pub use store::EvidenceStore;
pub use store::EvidenceV2ResignReport;
pub use store::GitRefEvidenceStore;
pub use types::*;
