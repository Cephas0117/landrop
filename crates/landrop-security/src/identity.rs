use std::io::Write;
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};
use directories::ProjectDirs;
use rcgen::{generate_simple_self_signed, CertifiedKey, KeyPair};
use rustls::client::danger::ServerCertVerifier;
use rustls::pki_types::{CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::{ClientConfig, ServerConfig};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct CertFingerprint(pub String);

impl CertFingerprint {
    pub fn from_der(der: &[u8]) -> Self {
        let hash = Sha256::digest(der);
        Self(encode_hex(&hash))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub struct DeviceIdentity {
    pub cert: CertificateDer<'static>,
    pub fingerprint: CertFingerprint,
    pub device_id: Uuid,
    key_pair: KeyPair,
}

#[derive(Serialize, Deserialize)]
struct StoredIdentity {
    device_id: Uuid,
    certificate: Vec<u8>,
    private_key: Vec<u8>,
}

impl DeviceIdentity {
    pub fn load_or_create() -> Result<Self> {
        let dirs = ProjectDirs::from("com", "landrop", "LANDrop")
            .context("cannot determine data directory")?;
        Self::load_or_create_at(&dirs.data_dir().join("identity.json"))
    }

    pub fn load_or_create_at(path: &Path) -> Result<Self> {
        match std::fs::read(path) {
            Ok(bytes) => Self::from_stored(&bytes),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let identity = Self::generate()?;
                let parent = path.parent().context("identity path has no parent")?;
                std::fs::create_dir_all(parent)?;
                let stored = StoredIdentity {
                    device_id: identity.device_id,
                    certificate: identity.cert_der(),
                    private_key: identity.key_pair.serialize_der(),
                };
                // NamedTempFile uses private permissions; publishing without
                // clobbering keeps concurrent launches on the same identity.
                let mut file = tempfile::NamedTempFile::new_in(parent)?;
                file.write_all(&serde_json::to_vec(&stored)?)?;
                file.as_file().sync_all()?;
                match file.persist_noclobber(path) {
                    Ok(_) => Ok(identity),
                    Err(e) if e.error.kind() == std::io::ErrorKind::AlreadyExists => {
                        Self::from_stored(&std::fs::read(path)?)
                    }
                    Err(e) => Err(e.error.into()),
                }
            }
            Err(e) => Err(e).context("failed to read device identity"),
        }
    }

    fn from_stored(bytes: &[u8]) -> Result<Self> {
        let stored: StoredIdentity =
            serde_json::from_slice(bytes).context("saved device identity is invalid")?;
        let identity = Self {
            fingerprint: CertFingerprint::from_der(&stored.certificate),
            cert: CertificateDer::from(stored.certificate),
            device_id: stored.device_id,
            key_pair: KeyPair::try_from(stored.private_key)?,
        };
        identity
            .server_config()
            .context("saved certificate/key is invalid")?;
        Ok(identity)
    }

    pub fn generate() -> Result<Self> {
        let device_id = Uuid::new_v4();
        let subject_alt_names = vec!["localhost".to_string(), format!("landrop-{}", device_id)];
        let CertifiedKey { cert, key_pair } = generate_simple_self_signed(subject_alt_names)?;
        let der = cert.der().to_vec();
        let fingerprint = CertFingerprint::from_der(&der);

        Ok(Self {
            cert: cert.der().clone(),
            fingerprint,
            device_id,
            key_pair,
        })
    }

    pub fn cert_der(&self) -> Vec<u8> {
        self.cert.to_vec()
    }

    pub fn server_config(&self) -> Result<Arc<ServerConfig>> {
        let config =
            ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
                .with_protocol_versions(&[&rustls::version::TLS13])?
                .with_no_client_auth()
                .with_single_cert(
                    vec![self.cert.clone()],
                    PrivateKeyDer::from(PrivatePkcs8KeyDer::from(self.key_pair.serialize_der())),
                )?;
        Ok(Arc::new(config))
    }

    pub fn client_config(
        &self,
        verifier: Arc<dyn ServerCertVerifier>,
    ) -> Result<Arc<ClientConfig>> {
        let config =
            ClientConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
                .with_protocol_versions(&[&rustls::version::TLS13])?
                .dangerous()
                .with_custom_certificate_verifier(verifier)
                .with_no_client_auth();
        Ok(Arc::new(config))
    }
}

fn encode_hex(bytes: &[u8]) -> String {
    const LUT: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        out.push(LUT[(byte >> 4) as usize] as char);
        out.push(LUT[(byte & 0x0f) as usize] as char);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_stable_fingerprint_format() {
        let identity = DeviceIdentity::generate().unwrap();
        assert_eq!(identity.fingerprint.0.len(), 64);
    }

    #[test]
    fn identity_survives_restart_with_private_permissions() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("identity.json");
        let first = DeviceIdentity::load_or_create_at(&path).unwrap();
        let second = DeviceIdentity::load_or_create_at(&path).unwrap();
        assert_eq!(first.device_id, second.device_id);
        assert_eq!(first.cert_der(), second.cert_der());
        assert_eq!(first.fingerprint.0, second.fingerprint.0);
        assert_eq!(
            first.key_pair.serialize_der(),
            second.key_pair.serialize_der()
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                std::fs::metadata(path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
    }

    #[test]
    fn corrupt_identity_is_not_silently_replaced() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("identity.json");
        std::fs::write(&path, b"invalid").unwrap();
        assert!(DeviceIdentity::load_or_create_at(&path).is_err());
        assert_eq!(std::fs::read(path).unwrap(), b"invalid");
    }
}
