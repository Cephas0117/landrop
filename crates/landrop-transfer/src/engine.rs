use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use dashmap::DashMap;
use landrop_security::{DeviceIdentity, TrustStore};
use landrop_state::transfer::TransferProgress;
use parking_lot::RwLock;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;
use tokio_rustls::{TlsAcceptor, TlsConnector};
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
    Progress(TransferProgress),
    Completed,
    Failed(String),
}

pub struct TransferEngine {
    identity: Arc<DeviceIdentity>,
    trust_store: Arc<RwLock<TrustStore>>,
    receive_dir: Arc<RwLock<PathBuf>>,
    event_tx: mpsc::UnboundedSender<TransferEvent>,
    cancel_senders: Arc<DashMap<Uuid, mpsc::Sender<()>>>,
}

impl TransferEngine {
    pub fn new(
        identity: Arc<DeviceIdentity>,
        trust_store: Arc<RwLock<TrustStore>>,
        receive_dir: PathBuf,
    ) -> (Self, mpsc::UnboundedReceiver<TransferEvent>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (
            Self {
                identity,
                trust_store,
                receive_dir: Arc::new(RwLock::new(receive_dir)),
                event_tx: tx,
                cancel_senders: Arc::new(DashMap::new()),
            },
            rx,
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

    pub async fn listen(&self) -> Result<u16> {
        let server_config = self.identity.server_config()?;
        let acceptor = TlsAcceptor::from(server_config);
        let listener = TcpListener::bind(format!("0.0.0.0:{}", LISTEN_PORT)).await?;
        let bound_port = listener.local_addr()?.port();

        let receive_dir = self.receive_dir.clone();
        let event_tx = self.event_tx.clone();

        tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = listener.accept().await else { break };
                let acceptor = acceptor.clone();
                let receive_dir = receive_dir.clone();
                let event_tx = event_tx.clone();

                tokio::spawn(async move {
                    let tls_stream = match acceptor.accept(stream).await {
                        Ok(s) => s,
                        Err(e) => {
                            tracing::warn!("TLS accept error: {e}");
                            return;
                        }
                    };

                    let transfer_id = Uuid::new_v4();
                    let dir = receive_dir.read().clone();
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

                    match Receiver::run(tls_stream, dir, transfer_id, progress_tx).await {
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
                });
            }
        });

        Ok(bound_port)
    }

    pub async fn send(
        &self,
        peer_addr: SocketAddr,
        peer_id: Uuid,
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

        let transfer_id = Uuid::new_v4();
        let event_tx = self.event_tx.clone();
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
