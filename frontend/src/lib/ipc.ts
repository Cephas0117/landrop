import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { Peer } from "./stores/devices.svelte";
import type { TransferProgress } from "./stores/transfers.svelte";

export interface AppInfo {
  device_id: string;
  device_name: string;
  fingerprint: string;
  receive_dir: string;
}

export interface PairingRequest {
  peer_id: string;
  peer_name: string;
  peer_fingerprint: string;
  pin: string;
}

export const ipc = {
  async appBootstrap(): Promise<AppInfo> {
    return invoke("app_bootstrap");
  },

  async discoveryStart(): Promise<void> {
    return invoke("discovery_start");
  },

  async discoveryStop(): Promise<void> {
    return invoke("discovery_stop");
  },

  async listPeers(): Promise<Peer[]> {
    return invoke("list_peers");
  },

  async probeManualPeer(addr: string): Promise<Peer> {
    return invoke("probe_manual_peer", { addr });
  },

  async requestPair(peerId: string): Promise<void> {
    return invoke("request_pair", { peerId });
  },

  async acceptPair(peerId: string, pin: string): Promise<void> {
    return invoke("accept_pair", { peerId, pin });
  },

  async rejectPair(peerId: string): Promise<void> {
    return invoke("reject_pair", { peerId });
  },

  async queueSend(peerId: string, paths: string[]): Promise<string> {
    return invoke("queue_send", { peerId, paths });
  },

  async cancelTransfer(id: string): Promise<void> {
    return invoke("cancel_transfer", { id });
  },

  async setReceiveDir(path: string): Promise<void> {
    return invoke("set_receive_dir", { path });
  },

  async listTransferHistory() {
    return invoke("list_transfer_history");
  },

  // Event listeners
  async listenPeerUpsert(cb: (peer: Peer) => void): Promise<UnlistenFn> {
    return listen("discovery://peer-upsert", (e) => cb(e.payload as Peer));
  },

  async listenPeerExpired(cb: (id: string) => void): Promise<UnlistenFn> {
    return listen("discovery://peer-expired", (e) => cb(e.payload as string));
  },

  async listenPairingRequest(cb: (req: PairingRequest) => void): Promise<UnlistenFn> {
    return listen("pairing://request", (e) => cb(e.payload as PairingRequest));
  },

  async listenProgress(
    cb: (data: { transfer_id: string; progress: TransferProgress }) => void
  ): Promise<UnlistenFn> {
    return listen("transfer://progress", (e) =>
      cb(e.payload as { transfer_id: string; progress: TransferProgress })
    );
  },

  async listenCompleted(cb: (id: string) => void): Promise<UnlistenFn> {
    return listen("transfer://completed", (e) => cb(e.payload as string));
  },

  async listenFailed(
    cb: (data: { transfer_id: string; error: string }) => void
  ): Promise<UnlistenFn> {
    return listen("transfer://failed", (e) =>
      cb(e.payload as { transfer_id: string; error: string })
    );
  },
};
