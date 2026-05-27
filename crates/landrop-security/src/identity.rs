use std::sync::Arc;

use anyhow::Result;
use rcgen::{generate_simple_self_signed, Certificate, CertifiedKey, KeyPair};
use rustls::client::danger::ServerCertVerifier;
use rustls::pki_types::{PrivateKeyDer, PrivatePkcs8KeyDer};
use rustls::{ClientConfig, ServerConfig};
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
    pub cert: Certificate,
    pub fingerprint: CertFingerprint,
    pub device_id: Uuid,
    key_pair: KeyPair,
}

impl DeviceIdentity {
    pub fn generate() -> Result<Self> {
        let device_id = Uuid::new_v4();
        let subject_alt_names = vec!["localhost".to_string(), format!("landrop-{}", device_id)];
        let CertifiedKey { cert, key_pair } = generate_simple_self_signed(subject_alt_names)?;
        let der = cert.der().to_vec();
        let fingerprint = CertFingerprint::from_der(&der);

        Ok(Self { cert, fingerprint, device_id, key_pair })
    }

    pub fn cert_der(&self) -> Vec<u8> {
        self.cert.der().to_vec()
    }

    pub fn server_config(&self) -> Result<Arc<ServerConfig>> {
        let config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(
                vec![self.cert.der().clone()],
                PrivateKeyDer::from(PrivatePkcs8KeyDer::from(self.key_pair.serialize_der())),
            )?;
        Ok(Arc::new(config))
    }

    pub fn client_config(&self, verifier: Arc<dyn ServerCertVerifier>) -> Result<Arc<ClientConfig>> {
        let config = ClientConfig::builder()
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
}
