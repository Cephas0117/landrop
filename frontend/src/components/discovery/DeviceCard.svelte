<script lang="ts">
  import { ipc } from "../../lib/ipc";
  import { toasts } from "../../lib/stores/toast.svelte";
  import type { Peer } from "../../lib/stores/devices.svelte";

  let { peer, selected = false, onSelect }: {
    peer: Peer;
    selected?: boolean;
    onSelect?: (peer: Peer) => void;
  } = $props();

  let pairing = $state(false);

  function osIcon(os: string): string {
    if (os.toLowerCase().includes("win")) return "🪟";
    if (os.toLowerCase().includes("mac") || os.toLowerCase().includes("darwin")) return "🍎";
    return "🐧";
  }

  function stateLabel(state: string): string {
    switch (state) {
      case "Discovered": return "可用";
      case "Paired":     return "已配对";
      case "Pairing":    return "配对中…";
      default:           return state;
    }
  }

  async function startPair(e: MouseEvent) {
    e.stopPropagation();
    if (pairing) return;
    pairing = true;
    try {
      await ipc.requestPair(peer.id);
      toasts.info(`正在与 ${peer.name} 配对…`, "请在两台设备上确认 PIN 码一致");
    } catch (err) {
      toasts.error("配对失败", String(err));
    } finally {
      pairing = false;
    }
  }
</script>

<!-- Outer wrapper is a div; inner select-area is the keyboard-accessible button -->
<div class="device-card glass-card" class:selected>
  <button
    class="card-select"
    onclick={() => onSelect?.(peer)}
    aria-pressed={selected}
    aria-label="选择 {peer.name}"
  >
    <span class="os-icon">{osIcon(peer.os)}</span>
    <div class="info">
      <span class="name">{peer.name}</span>
      <span class="addr">{peer.addr}</span>
    </div>
    <span class="state" class:trusted={peer.state === "Paired"}>{stateLabel(peer.state)}</span>
  </button>

  {#if peer.state === "Discovered"}
    <button
      class="pair-btn"
      onclick={startPair}
      disabled={pairing}
      title="与此设备配对"
      aria-label="与 {peer.name} 配对"
    >
      {pairing ? "…" : "配对"}
    </button>
  {/if}
</div>

<style>
  .device-card {
    display: flex;
    align-items: center;
    width: 100%;
    background: var(--glass-bg);
    transition: background 0.2s, box-shadow 0.2s;
    padding-right: 10px;
  }

  .device-card:has(.pair-btn):hover,
  .device-card.selected {
    background: rgba(99, 102, 241, 0.18);
  }

  .device-card.selected {
    box-shadow: inset 0 0 0 1px rgba(99, 102, 241, 0.5);
  }

  .card-select {
    display: flex;
    align-items: center;
    gap: 12px;
    flex: 1;
    padding: 12px 16px;
    background: none;
    border: none;
    cursor: pointer;
    text-align: left;
    color: inherit;
    min-width: 0;
  }

  .card-select:hover ~ .pair-btn {
    opacity: 1;
  }

  .device-card:not(.selected) .card-select:hover {
    background: rgba(99, 102, 241, 0.08);
  }

  .os-icon {
    font-size: 20px;
    flex-shrink: 0;
  }

  .info {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .name {
    font-size: 14px;
    font-weight: 500;
    color: rgba(241, 245, 249, 0.9);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .addr {
    font-size: 11px;
    color: rgba(241, 245, 249, 0.4);
    font-family: ui-monospace, monospace;
  }

  .state {
    font-size: 11px;
    color: rgba(241, 245, 249, 0.4);
    flex-shrink: 0;
  }

  .state.trusted {
    color: rgba(99, 102, 241, 0.8);
  }

  .pair-btn {
    font-size: 10px;
    font-weight: 600;
    padding: 3px 10px;
    border-radius: 6px;
    border: 1px solid rgba(99, 102, 241, 0.4);
    background: rgba(99, 102, 241, 0.15);
    color: rgba(99, 102, 241, 0.9);
    cursor: pointer;
    transition: background 0.15s, border-color 0.15s, opacity 0.15s;
    flex-shrink: 0;
    opacity: 0.7;
  }

  .device-card:hover .pair-btn {
    opacity: 1;
  }

  .pair-btn:hover:not(:disabled) {
    background: rgba(99, 102, 241, 0.3);
    border-color: rgba(99, 102, 241, 0.7);
  }

  .pair-btn:disabled {
    opacity: 0.4;
    cursor: default;
  }
</style>
