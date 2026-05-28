use std::sync::Arc;

use landrop_security::PairingManager;
use serde::Serialize;
use tauri::{Emitter, State};
use uuid::Uuid;

use crate::dto::{AppInfoDto, PeerDto, TransferDto};
use crate::events;
use crate::state::TauriState;

type AppState<'a> = State<'a, Arc<TauriState>>;

#[derive(Clone, Serialize)]
struct PairingOutgoingEvent {
    peer_id: String,
    peer_name: String,
    session_id: String,
    pin: String,
}

#[tauri::command]
pub async fn app_bootstrap(state: AppState<'_>) -> Result<AppInfoDto, String> {
    let svc = &state.services;
    Ok(AppInfoDto {
        device_id: svc.identity.device_id.to_string(),
        device_name: svc.app_state.device_name.clone(),
        fingerprint: svc.identity.fingerprint.0.clone(),
        receive_dir: svc.app_state.receive_dir().to_string_lossy().to_string(),
    })
}

#[tauri::command]
pub async fn discovery_start(state: AppState<'_>) -> Result<(), String> {
    state.services.discovery.start(7878).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn discovery_stop(_state: AppState<'_>) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub async fn list_peers(state: AppState<'_>) -> Result<Vec<PeerDto>, String> {
    let peers = state.services.discovery.get_peers();
    let trust = state.services.trust_store.read();
    Ok(peers
        .into_iter()
        .map(|p| {
            let trusted = trust.is_trusted(p.device_id, "");
            PeerDto {
                id: p.device_id.to_string(),
                name: p.device_name.clone(),
                os: "unknown".into(),
                addr: p.addr.to_string(),
                fingerprint: String::new(),
                state: if trusted { "Paired" } else { "Discovered" }.into(),
            }
        })
        .collect())
}

#[tauri::command]
pub async fn probe_manual_peer(addr: String, state: AppState<'_>) -> Result<PeerDto, String> {
    let socket_addr: std::net::SocketAddr =
        addr.parse().map_err(|e: std::net::AddrParseError| e.to_string())?;
    let peer = state
        .services
        .discovery
        .probe_manual(socket_addr)
        .await
        .map_err(|e| e.to_string())?;
    Ok(PeerDto {
        id: peer.device_id.to_string(),
        name: peer.device_name,
        os: "unknown".into(),
        addr: peer.addr.to_string(),
        fingerprint: String::new(),
        state: "Discovered".into(),
    })
}

#[tauri::command]
pub async fn request_pair(
    peer_id: String,
    app_handle: tauri::AppHandle,
    state: AppState<'_>,
) -> Result<(), String> {
    let peer_uuid = Uuid::parse_str(&peer_id).map_err(|e| e.to_string())?;

    let peers = state.services.discovery.get_peers();
    let peer = peers
        .iter()
        .find(|p| p.device_id == peer_uuid)
        .ok_or_else(|| format!("peer {peer_id} not found"))?
        .clone();

    let device_id = state.services.identity.device_id;
    let pin = PairingManager::generate_pin();

    let session_id = state
        .services
        .transfer
        .initiate_pairing(peer.addr, device_id, pin.clone())
        .await
        .map_err(|e| e.to_string())?;

    let _ = app_handle.emit(
        events::PAIRING_OUTGOING,
        PairingOutgoingEvent {
            peer_id: peer.device_id.to_string(),
            peer_name: peer.device_name,
            session_id: session_id.to_string(),
            pin,
        },
    );

    Ok(())
}

#[tauri::command]
pub async fn accept_pair(session_id: String, state: AppState<'_>) -> Result<(), String> {
    let session_uuid = Uuid::parse_str(&session_id).map_err(|e| e.to_string())?;
    state.services.transfer.resolve_pairing(session_uuid, true);
    Ok(())
}

#[tauri::command]
pub async fn reject_pair(session_id: String, state: AppState<'_>) -> Result<(), String> {
    let session_uuid = Uuid::parse_str(&session_id).map_err(|e| e.to_string())?;
    state.services.transfer.resolve_pairing(session_uuid, false);
    Ok(())
}

#[tauri::command]
pub async fn queue_send(
    peer_id: String,
    paths: Vec<String>,
    state: AppState<'_>,
) -> Result<String, String> {
    let peer_uuid = Uuid::parse_str(&peer_id).map_err(|e| e.to_string())?;
    let path_bufs: Vec<std::path::PathBuf> = paths.iter().map(|p| p.into()).collect();

    let peers = state.services.discovery.get_peers();
    let peer = peers
        .iter()
        .find(|p| p.device_id == peer_uuid)
        .ok_or_else(|| format!("peer {peer_id} not found"))?
        .clone();

    let transfer_id = state
        .services
        .transfer
        .send(peer.addr, peer_uuid, path_bufs)
        .await
        .map_err(|e| e.to_string())?;

    Ok(transfer_id.to_string())
}

#[tauri::command]
pub async fn cancel_transfer(id: String, state: AppState<'_>) -> Result<(), String> {
    let uuid = Uuid::parse_str(&id).map_err(|e| e.to_string())?;
    state.services.transfer.cancel(uuid);
    Ok(())
}

#[tauri::command]
pub async fn retry_transfer(_id: String, _state: AppState<'_>) -> Result<(), String> {
    Ok(())
}

#[tauri::command]
pub async fn set_receive_dir(path: String, state: AppState<'_>) -> Result<(), String> {
    state.services.app_state.set_receive_dir(path.into());
    state
        .services
        .transfer
        .set_receive_dir(state.services.app_state.receive_dir());
    Ok(())
}

#[tauri::command]
pub async fn list_transfer_history(state: AppState<'_>) -> Result<Vec<TransferDto>, String> {
    let history = state.services.app_state.get_history();
    Ok(history
        .into_iter()
        .map(|t| TransferDto {
            id: t.id.to_string(),
            peer_id: t.peer_id.to_string(),
            peer_name: t.peer_name,
            direction: format!("{:?}", t.direction),
            status: format!("{:?}", t.status),
            bytes_sent: t.progress.bytes_sent,
            total_bytes: t.progress.total_bytes,
            speed_bps: t.progress.speed_bps,
            eta_secs: t.progress.eta_secs,
            files_done: t.progress.files_done,
            files_total: t.progress.files_total,
        })
        .collect())
}
