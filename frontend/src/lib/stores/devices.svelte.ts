export interface Peer {
  id: string;
  name: string;
  os: string;
  addr: string;
  fingerprint: string;
  state: string;
  lastSeenMs: number;
}

let _peers = $state<Map<string, Peer>>(new Map());

export const devices = {
  get peers() {
    return _peers;
  },

  addOrUpdate(peer: Peer) {
    peer.lastSeenMs = Date.now();
    _peers = new Map(_peers).set(peer.id, peer);
  },

  remove(id: string) {
    const m = new Map(_peers);
    m.delete(id);
    _peers = m;
  },

  clearExpired(ttlMs = 5000) {
    const now = Date.now();
    const m = new Map(_peers);
    for (const [id, peer] of m) {
      if (now - peer.lastSeenMs > ttlMs) {
        m.delete(id);
      }
    }
    _peers = m;
  },

  asList(): Peer[] {
    return [..._peers.values()].sort((a, b) => a.name.localeCompare(b.name));
  },
};
