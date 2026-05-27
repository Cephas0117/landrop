<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { ipc } from "../../lib/ipc";
  import { toasts } from "../../lib/stores/toast.svelte";
  import type { Peer } from "../../lib/stores/devices.svelte";

  let { selectedPeer }: { selectedPeer: Peer | null } = $props();

  let hovering = $state(false);
  let message = $state("");

  let unlistenHover: UnlistenFn | null = null;
  let unlistenDrop: UnlistenFn | null = null;
  let unlistenCancel: UnlistenFn | null = null;

  onMount(async () => {
    unlistenHover = await listen<string[]>("tauri://drag-enter", () => {
      hovering = true;
      message = selectedPeer ? `拖放以发送至 ${selectedPeer.name}` : "请先选择左侧设备";
    });

    unlistenCancel = await listen("tauri://drag-leave", () => {
      hovering = false;
    });

    unlistenDrop = await listen<{ paths: string[] }>("tauri://drag-drop", async (e) => {
      hovering = false;
      const paths = e.payload.paths;
      if (!paths || paths.length === 0) return;
      if (!selectedPeer) {
        toasts.warn("未选择设备", "请先点击左侧设备，再拖放文件");
        return;
      }
      try {
        await ipc.queueSend(selectedPeer.id, paths);
      } catch (err) {
        toasts.error("发送失败", String(err));
      }
    });
  });

  onDestroy(() => {
    unlistenHover?.();
    unlistenCancel?.();
    unlistenDrop?.();
  });
</script>

{#if hovering}
  <div class="drop-overlay" class:no-peer={!selectedPeer} aria-hidden="true">
    <div class="drop-card glass-card">
      <div class="drop-icon">{selectedPeer ? "📤" : "⚠️"}</div>
      <p class="drop-message">{message}</p>
    </div>
  </div>
{/if}

<style>
  .drop-overlay {
    position: fixed;
    inset: 0;
    z-index: 100;
    display: flex;
    align-items: center;
    justify-content: center;
    background: rgba(6, 7, 20, 0.7);
    backdrop-filter: blur(8px);
    animation: fadeIn 0.15s ease forwards;
  }

  .drop-card {
    padding: 40px 60px;
    text-align: center;
    border: 2px dashed rgba(99, 102, 241, 0.6);
    animation: slideUp 0.2s cubic-bezier(0.34, 1.56, 0.64, 1) forwards;
  }

  .no-peer .drop-card {
    border-color: rgba(234, 179, 8, 0.5);
  }

  .drop-icon {
    font-size: 48px;
    margin-bottom: 16px;
  }

  .drop-message {
    font-size: 16px;
    font-weight: 500;
    color: rgba(241, 245, 249, 0.9);
    margin: 0;
  }
</style>
