use std::sync::Arc;

use landrop_discovery::DiscoveryEvent;
use landrop_transfer::TransferEventKind;
use serde::Serialize;
use tauri::{Emitter, Manager};

mod commands;
mod dto;
mod events;
mod state;

use dto::PeerDto;

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

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
                        let mut drx = init.discovery_rx;
                        tauri::async_runtime::spawn(async move {
                            while let Some(event) = drx.recv().await {
                                match event {
                                    DiscoveryEvent::PeerAdded(p)
                                    | DiscoveryEvent::PeerUpdated(p) => {
                                        let dto = PeerDto {
                                            id: p.device_id.to_string(),
                                            name: p.device_name,
                                            os: "unknown".into(),
                                            addr: p.addr.to_string(),
                                            fingerprint: String::new(),
                                            state: "Discovered".into(),
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
