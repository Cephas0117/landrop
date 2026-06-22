use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use dashmap::DashMap;
use futures::{SinkExt, StreamExt};
use landrop_protocol::codec::FrameCodec;
use landrop_protocol::{WireMessage, PairRequest as WirePairRequest, PairAccept as WirePairAccept};
use landrop_security::{DeviceIdentity, TrustStore};
use landrop_state::transfer::TransferProgress;
use parking_lot::RwLock;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, oneshot};
use tokio_rustls::{TlsAcceptor, TlsConnector};
use tokio_util::codec::Framed;
use uuid::Uuid;

use crate::receiver::Receiver;
use crate::sender::Sender;
use landrop_security::verifier::TofuVerifier;

const LISTEN_PORT: u16 = 7878;

pub struct TransferEvent {
    pub transfer_id: Uuid,
    pub event: TransferEventKind,
}

pub enum TransferEventKind {
    Queued {
        peer_id: Uuid,
        peer_name: String,
        direction: TransferDirection,
        files_total: u32,
        total_bytes: u64,
    },
    Progress(TransferProgress),
    Completed,
    Failed(String),
}

#[derive(Debug, Clone, Copy)]
pub enum TransferDirection {
    Send,
    Receive,
}

pub enum PairingEvent {
    IncomingRequest {
        peer_id: Uuid,
        session_id: Uuid,
        pin: String,
        peer_fingerprint: String,
    },
    OutgoingResolved {
        session_id: Uuid,
        accepted: bool,
    },
}

pub struct TransferEngine {
    identity: Arc<DeviceIdentity>,
    trust_store: Arc<RwLock<TrustStore>>,
    receive_dir: Arc<RwLock<PathBuf>>,
    event_tx: mpsc::UnboundedSender<TransferEvent>,
    pairing_tx: mpsc::UnboundedSender<PairingEvent>,
    cancel_senders: Arc<DashMap<Uuid, mpsc::Sender<()>>>,
    pending_incoming: Arc<DashMap<Uuid, oneshot::Sender<bool>>>,
}

impl TransferEngine {
    pub fn new(
        identity: Arc<DeviceIdentity>,
        trust_store: Arc<RwLock<TrustStore>>,
        receive_dir: PathBuf,
    ) -> (Self, mpsc::UnboundedReceiver<TransferEvent>, mpsc::UnboundedReceiver<PairingEvent>) {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let (pairing_tx, pairing_rx) = mpsc::unbounded_channel();
        (
            Self {
                identity,
                trust_store,
                receive_dir: Arc::new(RwLock::new(receive_dir)),
                event_tx,
                pairing_tx,
                cancel_senders: Arc::new(DashMap::new()),
                pending_incoming: Arc::new(DashMap::new()),
            },
            event_rx,
            pairing_rx,
        )
    }

    pub fn cancel(&self, id: Uuid) {
        if let Some((_, tx)) = self.cancel_senders.remove(&id) {
            let _ = tx.try_send(());
        }
    }

    pub fn set_receive_dir(&self, dir: PathBuf) {
        *self.receive_dir.write() = dir;
    }

    /// Device B calls this to accept or reject an incoming pairing request.
    pub fn resolve_pairing(&self, session_id: Uuid, accepted: bool) {
        if let Some((_, tx)) = self.pending_incoming.remove(&session_id) {
            let _ = tx.send(accepted);
        }
    }

    /// Device A calls this to initiate pairing with a remote peer.
    /// Sends PairRequest over TLS; the response arrives via PairingEvent::OutgoingResolved.
    pub async fn initiate_pairing(
        &self,
        peer_addr: SocketAddr,
        device_id: Uuid,
        pin: String,
        my_fingerprint: String,
    ) -> Result<Uuid> {
        let session_id = Uuid::new_v4();

        // TOFU: no peer_id_hint for first-time pairing — accept any cert
        let verifier = TofuVerifier::new(self.trust_store.clone());
        let client_config = self.identity.client_config(verifier)?;
        let connector = TlsConnector::from(client_config);

        let stream = TcpStream::connect(peer_addr).await?;
        let server_name = rustls::pki_types::ServerName::try_from("landrop")
            .map_err(|e| anyhow::anyhow!("server name: {e}"))?
            .to_owned();
        let tls_stream = connector.connect(server_name, stream).await?;

        let mut framed = Framed::new(tls_stream, FrameCodec::default());

        framed
            .send(WireMessage::PairRequest(WirePairRequest {
                device_id,
                session_id,
                pin,
                peer_fingerprint: my_fingerprint,
            }))
            .await?;

        // Spawn background task to await PairAccept; result emitted via pairing_tx
        let pairing_tx = self.pairing_tx.clone();
        let trust_store = self.trust_store.clone();
        tokio::spawn(async move {
            let accepted = match framed.next().await {
                Some(Ok(WireMessage::PairAccept(accept))) => {
                    if accept.accepted {
                        let mut ts = trust_store.write();
                        ts.add_peer(accept.device_id, accept.peer_fingerprint);
                        if let Err(e) = ts.save() {
                            tracing::error!("failed to save trust store: {e}");
                        }
                    }
                    accept.accepted
                }
                _ => false,
            };
            let _ = pairing_tx.send(PairingEvent::OutgoingResolved { session_id, accepted });
        });

        Ok(session_id)
    }

    pub async fn listen(&self) -> Result<u16> {
        let server_config = self.identity.server_config()?;
        let acceptor = TlsAcceptor::from(server_config);
        let listener = TcpListener::bind(format!("0.0.0.0:{}", LISTEN_PORT)).await?;
        let bound_port = listener.local_addr()?.port();

        let receive_dir = self.receive_dir.clone();
        let event_tx = self.event_tx.clone();
        let pairing_tx = self.pairing_tx.clone();
        let pending_incoming = self.pending_incoming.clone();
        let trust_store = self.trust_store.clone();
        let device_id = self.identity.device_id;
        let my_fingerprint = self.identity.fingerprint.0.clone();

        tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else { break };
                let acceptor = acceptor.clone();
                let receive_dir = receive_dir.clone();
                let event_tx = event_tx.clone();
                let pairing_tx = pairing_tx.clone();
                let pending_incoming = pending_incoming.clone();
                let trust_store = trust_store.clone();
                let my_fingerprint = my_fingerprint.clone();

                tokio::spawn(async move {
                    let tls_stream = match acceptor.accept(stream).await {
                        Ok(s) => s,
                        Err(e) => {
                            tracing::warn!("TLS accept error: {e}");
                            return;
                        }
                    };

                    let mut framed = Framed::new(tls_stream, FrameCodec::default());

                    match framed.next().await {
                        Some(Ok(WireMessage::PairRequest(req))) => {
                            let (tx, rx) = oneshot::channel::<bool>();
                            let session_id = req.session_id;
                            pending_incoming.insert(session_id, tx);

                            let _ = pairing_tx.send(PairingEvent::IncomingRequest {
                                peer_id: req.device_id,
                                session_id,
                                pin: req.pin,
                                peer_fingerprint: req.peer_fingerprint.clone(),
                            });

                            let accepted = tokio::time::timeout(Duration::from_secs(120), rx)
                                .await
                                .unwrap_or(Ok(false))
                                .unwrap_or(false);

                            let _ = framed
                                .send(WireMessage::PairAccept(WirePairAccept {
                                    device_id,
                                    session_id,
                                    accepted,
                                    peer_fingerprint: my_fingerprint.clone(),
                                }))
                                .await;

                            if accepted {
                                let mut ts = trust_store.write();
                                ts.add_peer(req.device_id, req.peer_fingerprint);
                                if let Err(e) = ts.save() {
                                    tracing::error!("failed to save trust store: {e}");
                                }
                                tracing::info!("accepted pairing from {}", req.device_id);
                            }

                            let _ = pairing_tx.send(PairingEvent::OutgoingResolved {
                                session_id,
                                accepted,
                            });
                        }
                        Some(Ok(WireMessage::Manifest(manifest))) => {
                            let transfer_id = Uuid::new_v4();
                            let dir = receive_dir.read().clone();
                            let files_total = manifest.files.iter().filter(|f| !f.is_dir).count() as u32;

                            let _ = event_tx.send(TransferEvent {
                                transfer_id,
                                event: TransferEventKind::Queued {
                                    peer_id: Uuid::nil(),
                                    peer_name: manifest.transfer_name.clone(),
                                    direction: TransferDirection::Receive,
                                    files_total,
                                    total_bytes: manifest.total_bytes,
                                },
                            });

                            let (progress_tx, mut progress_rx) = mpsc::channel(32);

                            let tx = event_tx.clone();
                            let tid = transfer_id;
                            tokio::spawn(async move {
                                while let Some(p) = progress_rx.recv().await {
                                    let _ = tx.send(TransferEvent {
                                        transfer_id: tid,
                                        event: TransferEventKind::Progress(p),
                                    });
                                }
                            });

                            match Receiver::run_from_manifest(framed, manifest, dir, progress_tx)
                                .await
                            {
                                Ok(_) => {
                                    let _ = event_tx.send(TransferEvent {
                                        transfer_id,
                                        event: TransferEventKind::Completed,
                                    });
                                }
                                Err(e) => {
                                    let _ = event_tx.send(TransferEvent {
                                        transfer_id,
                                        event: TransferEventKind::Failed(e.to_string()),
                                    });
                                }
                            }
                        }
                        Some(Ok(other)) => {
                            tracing::warn!(
                                "unexpected first message on incoming connection: {:?}",
                                other
                            );
                        }
                        Some(Err(e)) => {
                            tracing::warn!("framing error on incoming connection: {e}");
                        }
                        None => {}
                    }
                });
            }
        });

        Ok(bound_port)
    }

    pub async fn send(
        &self,
        peer_addr: SocketAddr,
        peer_id: Uuid,
        peer_name: String,
        paths: Vec<PathBuf>,
    ) -> Result<Uuid> {
        let verifier = TofuVerifier::new(self.trust_store.clone());
        verifier.set_peer_id_hint(peer_id);

        let client_config = self.identity.client_config(verifier)?;
        let connector = TlsConnector::from(client_config);

        let stream = TcpStream::connect(peer_addr).await?;
        let server_name = rustls::pki_types::ServerName::try_from("landrop")
            .map_err(|e| anyhow::anyhow!("server name: {e}"))?
            .to_owned();
        let tls_stream = connector.connect(server_name, stream).await?;

        let files_total = paths.len() as u32;

        let transfer_id = Uuid::new_v4();
        let event_tx = self.event_tx.clone();

        let _ = event_tx.send(TransferEvent {
            transfer_id,
            event: TransferEventKind::Queued {
                peer_id,
                peer_name,
                direction: TransferDirection::Send,
                files_total,
                total_bytes: 0,
            },
        });
        let (progress_tx, mut progress_rx) = mpsc::channel(32);
        let (cancel_tx, cancel_rx) = mpsc::channel(1);

        let tx = event_tx.clone();
        let tid = transfer_id;
        tokio::spawn(async move {
            while let Some(p) = progress_rx.recv().await {
                let _ = tx.send(TransferEvent {
                    transfer_id: tid,
                    event: TransferEventKind::Progress(p),
                });
            }
        });

        tokio::spawn(async move {
            match Sender::run(tls_stream, paths, transfer_id, progress_tx, cancel_rx).await {
                Ok(()) => {
                    let _ = event_tx.send(TransferEvent {
                        transfer_id,
                        event: TransferEventKind::Completed,
                    });
                }
                Err(e) => {
                    let _ = event_tx.send(TransferEvent {
                        transfer_id,
                        event: TransferEventKind::Failed(e.to_string()),
                    });
                }
            }
        });

        self.cancel_senders.insert(transfer_id, cancel_tx);

        Ok(transfer_id)
    }
}
