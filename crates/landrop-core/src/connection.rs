use std::net::SocketAddr;
use std::time::Duration;

use anyhow::Result;
use futures::{SinkExt, StreamExt};
use landrop_protocol::codec::FrameCodec;
use landrop_protocol::{Hello, WireMessage};
use landrop_security::verifier::TofuVerifier;
use tokio::net::TcpStream;
use tokio_rustls::TlsConnector;
use tokio_util::codec::Framed;
use uuid::Uuid;

use crate::service::ServiceContainer;

pub struct PeerConnection {
    pub peer_id: Uuid,
    pub peer_name: String,
    pub peer_fingerprint: String,
    pub requires_pairing: bool,
}

impl ServiceContainer {
    pub async fn connect_peer(
        &self,
        addr: SocketAddr,
        peer_id_hint: Option<Uuid>,
    ) -> Result<PeerConnection> {
        let verifier = TofuVerifier::new(self.trust_store.clone());
        if let Some(id) = peer_id_hint {
            verifier.set_peer_id_hint(id);
        }

        let client_config = self.identity.client_config(verifier.clone())?;
        let connector = TlsConnector::from(client_config);
        let stream = tokio::time::timeout(
            Duration::from_secs(10),
            TcpStream::connect(addr),
        )
        .await??;

        let server_name = rustls::pki_types::ServerName::try_from("landrop")
            .map_err(|e| anyhow::anyhow!("server name: {}", e))?
            .to_owned();
        let tls = connector.connect(server_name, stream).await?;

        let mut framed = Framed::new(tls, FrameCodec::default());

        // Send Hello
        let hello = WireMessage::Hello(Hello {
            device_id: self.identity.device_id,
            session_id: Uuid::new_v4(),
            device_name: self.app_state.device_name.clone(),
            os: std::env::consts::OS.to_string(),
            cert_fingerprint: self.identity.fingerprint.0.clone(),
            capabilities: Default::default(),
        });
        framed.send(hello).await?;

        // Wait for CapsAck
        let caps_ack = match framed.next().await {
            Some(Ok(WireMessage::CapsAck(c))) => c,
            Some(Ok(other)) => anyhow::bail!("expected CapsAck, got {:?}", other),
            Some(Err(e)) => return Err(e.into()),
            None => anyhow::bail!("connection closed after Hello"),
        };

        let peer_fingerprint = verifier.last_fingerprint()
            .unwrap_or_default();

        Ok(PeerConnection {
            peer_id: caps_ack.device_id,
            peer_name: caps_ack.device_name,
            peer_fingerprint,
            requires_pairing: caps_ack.requires_pairing,
        })
    }
}
