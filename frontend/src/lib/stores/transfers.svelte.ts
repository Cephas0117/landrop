export interface TransferProgress {
  bytesSent: number;
  totalBytes: number;
  speedBps: number;
  etaSecs: number;
  filesDone: number;
  filesTotal: number;
}

export interface Transfer {
  id: string;
  peerId: string;
  peerName: string;
  direction: "Send" | "Receive";
  status: "Queued" | "Connecting" | "Transferring" | "Completed" | "Failed" | "Canceled";
  progress: TransferProgress;
  paths: string[];
  error?: string;
}

let _transfers = $state<Map<string, Transfer>>(new Map());

export const transfers = {
  get map() {
    return _transfers;
  },

  asList(): Transfer[] {
    return [..._transfers.values()].sort((a, b) => a.id.localeCompare(b.id));
  },

  active(): Transfer[] {
    return this.asList().filter(
      (t) => t.status !== "Completed" && t.status !== "Failed" && t.status !== "Canceled"
    );
  },

  start(t: Transfer) {
    _transfers = new Map(_transfers).set(t.id, t);
  },

  updateProgress(id: string, progress: TransferProgress) {
    const t = _transfers.get(id);
    if (!t) return;
    _transfers = new Map(_transfers).set(id, { ...t, status: "Transferring", progress });
  },

  updateStatus(id: string, status: Transfer["status"], error?: string) {
    const t = _transfers.get(id);
    if (!t) return;
    _transfers = new Map(_transfers).set(id, { ...t, status, error });
  },

  remove(id: string) {
    const m = new Map(_transfers);
    m.delete(id);
    _transfers = m;
  },
};
