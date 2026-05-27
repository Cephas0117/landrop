<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { ipc, type PairingRequest } from "../../lib/ipc";

  let request = $state<PairingRequest | null>(null);
  let unlisten: (() => void) | null = null;

  let pinChunks = $derived(
    request ? [request.pin.slice(0, 3), request.pin.slice(3, 6)] : []
  );

  onMount(async () => {
    unlisten = await ipc.listenPairingRequest((req) => {
      request = req;
    });
  });

  onDestroy(() => unlisten?.());

  async function accept() {
    if (!request) return;
    await ipc.acceptPair(request.peer_id, request.pin);
    request = null;
  }

  async function reject() {
    if (!request) return;
    await ipc.rejectPair(request.peer_id);
    request = null;
  }
</script>

{#if request}
  <div class="overlay" role="dialog" aria-modal="true" aria-label="配对请求">
    <div class="modal glass-card">
      <h2 class="title">收到配对请求</h2>
      <p class="subtitle">
        <strong>{request.peer_name}</strong> 请求与您配对。<br>
        请确认两台设备上显示的 PIN 码一致。
      </p>

      <div class="pin-row">
        {#each pinChunks as chunk}
          <div class="pin-chunk">{chunk}</div>
        {/each}
      </div>

      <p class="fingerprint">
        <span class="fp-label">设备指纹</span>
        <span class="fp-value">{request.peer_fingerprint.slice(0, 16)}…</span>
      </p>

      <div class="actions">
        <button class="btn btn-reject" onclick={reject}>拒绝</button>
        <button class="btn btn-accept" onclick={accept}>接受</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    z-index: 200;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(6, 7, 20, 0.75);
    backdrop-filter: blur(12px);
    animation: fadeIn 0.2s ease forwards;
  }

  .modal {
    width: 360px;
    padding: 32px;
    display: flex;
    flex-direction: column;
    gap: 20px;
    animation: slideUp 0.25s cubic-bezier(0.34, 1.56, 0.64, 1) forwards;
  }

  .title {
    font-size: 18px;
    font-weight: 600;
    color: rgba(241, 245, 249, 0.95);
    margin: 0;
  }

  .subtitle {
    font-size: 13px;
    color: rgba(241, 245, 249, 0.55);
    margin: 0;
    line-height: 1.6;
  }

  .pin-row {
    display: flex;
    justify-content: center;
    gap: 12px;
  }

  .pin-chunk {
    font-size: 32px;
    font-weight: 700;
    font-family: ui-monospace, monospace;
    letter-spacing: 6px;
    color: var(--accent);
    background: rgba(99, 102, 241, 0.1);
    border: 1px solid rgba(99, 102, 241, 0.3);
    border-radius: 12px;
    padding: 12px 20px;
    min-width: 100px;
    text-align: center;
  }

  .fingerprint {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin: 0;
  }

  .fp-label {
    font-size: 11px;
    color: rgba(241, 245, 249, 0.35);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }

  .fp-value {
    font-size: 12px;
    font-family: ui-monospace, monospace;
    color: rgba(241, 245, 249, 0.5);
    word-break: break-all;
  }

  .actions {
    display: flex;
    gap: 12px;
  }

  .btn {
    flex: 1;
    padding: 10px;
    border-radius: 10px;
    border: none;
    font-size: 14px;
    font-weight: 500;
    cursor: pointer;
    transition: opacity 0.15s, transform 0.1s;
  }

  .btn:active {
    transform: scale(0.97);
  }

  .btn-reject {
    background: rgba(239, 68, 68, 0.12);
    border: 1px solid rgba(239, 68, 68, 0.3);
    color: rgba(239, 68, 68, 0.9);
  }

  .btn-reject:hover {
    background: rgba(239, 68, 68, 0.2);
  }

  .btn-accept {
    background: rgba(99, 102, 241, 0.25);
    border: 1px solid rgba(99, 102, 241, 0.5);
    color: rgba(241, 245, 249, 0.95);
  }

  .btn-accept:hover {
    background: rgba(99, 102, 241, 0.38);
  }
</style>
