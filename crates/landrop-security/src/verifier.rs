use std::sync::Arc;

use parking_lot::{Mutex, RwLock};
use rustls::client::danger::{HandshakeSignatureValid, ServerCertVerified, ServerCertVerifier};
use rustls::pki_types::{CertificateDer, ServerName, UnixTime};
use rustls::{DigitallySignedStruct, Error, SignatureScheme};
use uuid::Uuid;

use crate::identity::CertFingerprint;
use crate::trust_store::TrustStore;

pub struct TofuVerifier {
    pub trust_store: Arc<RwLock<TrustStore>>,
    pub peer_id_hint: Arc<Mutex<Option<Uuid>>>,
    pub last_seen_fingerprint: Arc<Mutex<Option<String>>>,
}

impl TofuVerifier {
    pub fn new(trust_store: Arc<RwLock<TrustStore>>) -> Arc<Self> {
        Arc::new(Self {
            trust_store,
            peer_id_hint: Arc::new(Mutex::new(None)),
            last_seen_fingerprint: Arc::new(Mutex::new(None)),
        })
    }

    pub fn set_peer_id_hint(&self, id: Uuid) {
        *self.peer_id_hint.lock() = Some(id);
    }

    pub fn last_fingerprint(&self) -> Option<String> {
        self.last_seen_fingerprint.lock().clone()
    }
}

impl std::fmt::Debug for TofuVerifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TofuVerifier").finish()
    }
}

impl ServerCertVerifier for TofuVerifier {
    fn verify_server_cert(
        &self,
        end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<ServerCertVerified, Error> {
        let fp = CertFingerprint::from_der(end_entity.as_ref());
        *self.last_seen_fingerprint.lock() = Some(fp.0.clone());

        let peer_id = *self.peer_id_hint.lock();

        if let Some(id) = peer_id {
            if self.trust_store.read().is_trusted(id, &fp.0) {
                return Ok(ServerCertVerified::assertion());
            }
            return Err(Error::General("peer not trusted; pairing required".into()));
        }

        // No hint = initial pairing handshake; allow to proceed
        Ok(ServerCertVerified::assertion())
    }

    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &DigitallySignedStruct,
    ) -> Result<HandshakeSignatureValid, Error> {
        Ok(HandshakeSignatureValid::assertion())
    }

    fn supported_verify_schemes(&self) -> Vec<SignatureScheme> {
        vec![
            SignatureScheme::RSA_PSS_SHA256,
            SignatureScheme::RSA_PSS_SHA384,
            SignatureScheme::RSA_PSS_SHA512,
            SignatureScheme::ECDSA_NISTP256_SHA256,
            SignatureScheme::ECDSA_NISTP384_SHA384,
            SignatureScheme::ECDSA_NISTP521_SHA512,
            SignatureScheme::ED25519,
        ]
    }
}
