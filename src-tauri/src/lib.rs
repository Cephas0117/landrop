use std::sync::Arc;

use landrop_discovery::DiscoveryEvent;
use landrop_platform::firewall::ensure_windows_firewall_rules;
use landrop_transfer::{PairingEvent, TransferEventKind};
use serde::Serialize;
use tauri::{Emitter, Manager};

mod commands;
mod dto;
mod events;
mod state;

use dto::PeerDto;

#[derive(Clone, Serialize)]
struct QueuedPayload {
    transfer_id: String,
    peer_id: String,
    peer_name: String,
    direction: String,
    files_total: u32,
    total_bytes: u64,
}

#[derive(Clone, Serialize)]
struct ProgressPayload {
    transfer_id: String,
    bytes_sent: u64,
    total_bytes: u64,
    speed_bps: f64,
    eta_secs: f64,
    files_done: u32,
    files_total: u32,
}

#[derive(Clone, Serialize)]
struct FailedPayload {
    transfer_id: String,
    error: String,
}

#[derive(Clone, Serialize)]
struct PairingRequestEvent {
    peer_id: String,
    peer_name: String,
    peer_fingerprint: String,
    session_id: String,
    pin: String,
}

#[derive(Clone, Serialize)]
struct PairingResolvedEvent {
    session_id: String,
    accepted: bool,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    ensure_windows_firewall_rules();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                match state::init_app_state().await {
                    Ok(init) => {
                        let services = Arc::clone(&init.state.services);
                        handle.manage(Arc::new(init.state));

                        // Start TCP transfer listener
                        let svc = services.clone();
                        tauri::async_runtime::spawn(async move {
                            if let Err(e) = svc.transfer.listen().await {
                                tracing::error!("transfer listener: {e}");
                            }
                        });

                        // Discovery event bridge
                        let h = handle.clone();
                        let ts = services.trust_store.clone();
                        let mut drx = init.discovery_rx;
                        tauri::async_runtime::spawn(async move {
                            while let Some(event) = drx.recv().await {
                                match event {
                                    DiscoveryEvent::PeerAdded(p)
                                    | DiscoveryEvent::PeerUpdated(p) => {
                                        let trusted = ts.read().contains(p.device_id);
                                        let dto = PeerDto {
                                            id: p.device_id.to_string(),
                                            name: p.device_name,
                                            os: "unknown".into(),
                                            addr: p.addr.to_string(),
                                            fingerprint: String::new(),
                                            state: if trusted { "Paired" } else { "Discovered" }.into(),
                                        };
                                        let _ = h.emit(events::PEER_UPSERT, dto);
                                    }
                                    DiscoveryEvent::PeerExpired(id) => {
                                        let _ = h.emit(events::PEER_EXPIRED, id.to_string());
                                    }
                                }
                            }
                        });

                        // Transfer event bridge
                        let h = handle.clone();
                        let mut trx = init.transfer_rx;
                        tauri::async_runtime::spawn(async move {
                            while let Some(event) = trx.recv().await {
                                let tid = event.transfer_id.to_string();
                                match event.event {
                                    TransferEventKind::Queued {
                                        peer_id,
                                        peer_name,
                                        direction,
                                        files_total,
                                        total_bytes,
                                    } => {
                                        let _ = h.emit(
                                            events::TRANSFER_QUEUED,
                                            QueuedPayload {
                                                transfer_id: tid,
                                                peer_id: peer_id.to_string(),
                                                peer_name,
                                                direction: format!("{:?}", direction),
                                                files_total,
                                                total_bytes,
                                            },
                                        );
                                    }
                                    TransferEventKind::Progress(p) => {
                                        let _ = h.emit(
                                            events::TRANSFER_PROGRESS,
                                            ProgressPayload {
                                                transfer_id: tid,
                                                bytes_sent: p.bytes_sent,
                                                total_bytes: p.total_bytes,
                                                speed_bps: p.speed_bps,
                                                eta_secs: p.eta_secs,
                                                files_done: p.files_done,
                                                files_total: p.files_total,
                                            },
                                        );
                                    }
                                    TransferEventKind::Completed => {
                                        let _ = h.emit(events::TRANSFER_COMPLETED, tid);
                                    }
                                    TransferEventKind::Failed(err) => {
                                        let _ = h.emit(
                                            events::TRANSFER_FAILED,
                                            FailedPayload { transfer_id: tid, error: err },
                                        );
                                    }
                                }
                            }
                        });

                        // Pairing event bridge
                        let h = handle.clone();
                        let svc = services.clone();
                        let mut prx = init.pairing_rx;
                        tauri::async_runtime::spawn(async move {
                            while let Some(event) = prx.recv().await {
                                match event {
                                    PairingEvent::IncomingRequest {
                                        peer_id,
                                        session_id,
                                        pin,
                                        peer_fingerprint,
                                    } => {
                                        let peer_name = svc
                                            .discovery
                                            .get_peers()
                                            .into_iter()
                                            .find(|p| p.device_id == peer_id)
                                            .map(|p| p.device_name)
                                            .unwrap_or_else(|| peer_id.to_string());

                                        let _ = h.emit(
                                            events::PAIRING_REQUEST,
                                            PairingRequestEvent {
                                                peer_id: peer_id.to_string(),
                                                peer_name,
                                                peer_fingerprint,
                                                session_id: session_id.to_string(),
                                                pin,
                                            },
                                        );
                                    }
                                    PairingEvent::OutgoingResolved { session_id, accepted } => {
                                        let _ = h.emit(
                                            events::PAIRING_RESOLVED,
                                            PairingResolvedEvent {
                                                session_id: session_id.to_string(),
                                                accepted,
                                            },
                                        );

                                        // Refresh all peer states so UI reflects pairing result
                                        if accepted {
                                            for p in svc.discovery.get_peers() {
                                                let trusted = svc.trust_store.read().contains(p.device_id);
                                                let _ = h.emit(
                                                    events::PEER_UPSERT,
                                                    PeerDto {
                                                        id: p.device_id.to_string(),
                                                        name: p.device_name,
                                                        os: "unknown".into(),
                                                        addr: p.addr.to_string(),
                                                        fingerprint: String::new(),
                                                        state: if trusted { "Paired" } else { "Discovered" }.into(),
                                                    },
                                                );
                                            }
                                        }
                                    }
                                }
                            }
                        });

                        tracing::info!("LANDrop initialized");
                    }
                    Err(e) => {
                        tracing::error!("init failed: {e}");
                    }
                }
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::app_bootstrap,
            commands::discovery_start,
            commands::discovery_stop,
            commands::list_peers,
            commands::probe_manual_peer,
            commands::request_pair,
            commands::accept_pair,
            commands::reject_pair,
            commands::queue_send,
            commands::cancel_transfer,
            commands::retry_transfer,
            commands::set_receive_dir,
            commands::list_transfer_history,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
