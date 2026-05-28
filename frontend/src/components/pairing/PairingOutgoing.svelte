<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { ipc, type PairingOutgoing, type PairingResolved } from "../../lib/ipc";
  import { toasts } from "../../lib/stores/toast.svelte";

  let outgoing = $state<PairingOutgoing | null>(null);
  const unlisteners: Array<() => void> = [];

  let pinChunks = $derived(
    outgoing ? [outgoing.pin.slice(0, 3), outgoing.pin.slice(3, 6)] : []
  );

  onMount(async () => {
    unlisteners.push(
      await ipc.listenPairingOutgoing((data) => {
        outgoing = data;
      }),
      await ipc.listenPairingResolved((data: PairingResolved) => {
        if (!outgoing || outgoing.session_id !== data.session_id) return;
        if (data.accepted) {
          toasts.success("配对成功", `已与 ${outgoing.peer_name} 建立信任`);
        } else {
          toasts.warn("配对被拒绝", `${outgoing.peer_name} 拒绝了配对请求`);
        }
        outgoing = null;
      })
    );
  });

  onDestroy(() => unlisteners.forEach((fn) => fn()));

  function cancel() {
    outgoing = null;
  }
</script>

{#if outgoing}
  <div class="overlay" role="dialog" aria-modal="true" aria-label="正在配对">
    <div class="modal glass-card">
      <div class="spinner-row">
        <div class="spinner"></div>
      </div>

      <h2 class="title">等待对方确认</h2>
      <p class="subtitle">
        请让 <strong>{outgoing.peer_name}</strong> 在对方设备上确认以下 PIN 码。
      </p>

      <div class="pin-row">
        {#each pinChunks as chunk}
          <div class="pin-chunk">{chunk}</div>
        {/each}
      </div>

      <p class="hint">两台设备显示的 PIN 码必须一致</p>

      <div class="actions">
        <button class="btn btn-cancel" onclick={cancel}>取消</button>
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
    align-items: center;
    animation: slideUp 0.25s cubic-bezier(0.34, 1.56, 0.64, 1) forwards;
  }

  .spinner-row {
    display: flex;
    justify-content: center;
  }

  .spinner {
    width: 36px;
    height: 36px;
    border-radius: 50%;
    border: 3px solid rgba(99, 102, 241, 0.2);
    border-top-color: var(--accent, #6366f1);
    animation: spin 0.8s linear infinite;
  }

  @keyframes spin {
    to { transform: rotate(360deg); }
  }

  .title {
    font-size: 18px;
    font-weight: 600;
    color: rgba(241, 245, 249, 0.95);
    margin: 0;
    text-align: center;
  }

  .subtitle {
    font-size: 13px;
    color: rgba(241, 245, 249, 0.55);
    margin: 0;
    line-height: 1.6;
    text-align: center;
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

  .hint {
    font-size: 11px;
    color: rgba(241, 245, 249, 0.3);
    margin: 0;
    text-align: center;
  }

  .actions {
    width: 100%;
  }

  .btn {
    width: 100%;
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

  .btn-cancel {
    background: rgba(255, 255, 255, 0.06);
    border: 1px solid rgba(255, 255, 255, 0.12);
    color: rgba(241, 245, 249, 0.6);
  }

  .btn-cancel:hover {
    background: rgba(255, 255, 255, 0.1);
  }
</style>
