<script lang="ts">
  import type { Transfer } from "../../lib/stores/transfers.svelte";
  import ProgressBar from "../common/ProgressBar.svelte";
  import { formatSpeed, formatSize, formatEta } from "../../lib/utils/format";
  import { ipc } from "../../lib/ipc";

  let { transfer }: { transfer: Transfer } = $props();

  let pct = $derived(
    transfer.progress.totalBytes > 0
      ? transfer.progress.bytesSent / transfer.progress.totalBytes
      : 0
  );

  let active = $derived(
    transfer.status === "Queued" ||
    transfer.status === "Connecting" ||
    transfer.status === "Transferring"
  );

  function statusLabel(s: string): string {
    const map: Record<string, string> = {
      Queued: "队列中", Connecting: "连接中", Transferring: "传输中",
      Completed: "已完成", Failed: "失败", Canceled: "已取消",
    };
    return map[s] ?? s;
  }

  async function cancel() {
    await ipc.cancelTransfer(transfer.id);
  }
</script>

<div class="transfer-item glass-card" class:completed={transfer.status === "Completed"} class:failed={transfer.status === "Failed"}>
  <div class="header">
    <span class="direction">{transfer.direction === "Send" ? "↑" : "↓"}</span>
    <span class="peer">{transfer.peerName}</span>
    <span class="status" class:active class:error={transfer.status === "Failed"}>
      {statusLabel(transfer.status)}
    </span>
    {#if active}
      <button class="cancel-btn" onclick={cancel} aria-label="取消传输">✕</button>
    {/if}
  </div>

  <div class="files">
    {#each transfer.paths.slice(0, 2) as path}
      <span class="file-path">{path.split("/").pop() ?? path}</span>
    {/each}
    {#if transfer.paths.length > 2}
      <span class="more">另 {transfer.paths.length - 2} 个文件</span>
    {/if}
  </div>

  {#if transfer.status === "Transferring"}
    <ProgressBar value={pct} />
    <div class="stats">
      <span>{formatSize(transfer.progress.bytesSent)} / {formatSize(transfer.progress.totalBytes)}</span>
      <span>{formatSpeed(transfer.progress.speedBps)}</span>
      <span>预计剩余 {formatEta(transfer.progress.etaSecs)}</span>
    </div>
  {/if}

  {#if transfer.error}
    <div class="error-msg">{transfer.error}</div>
  {/if}
</div>

<style>
  .transfer-item {
    padding: 12px 16px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    animation: slideUp 0.3s cubic-bezier(0.34, 1.56, 0.64, 1) forwards;
  }

  .header {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .direction {
    font-size: 14px;
    color: var(--accent);
    flex-shrink: 0;
  }

  .peer {
    flex: 1;
    font-size: 13px;
    font-weight: 500;
    color: rgba(241, 245, 249, 0.9);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .status {
    font-size: 11px;
    color: rgba(241, 245, 249, 0.4);
    flex-shrink: 0;
  }

  .status.active {
    color: var(--accent);
  }

  .status.error {
    color: rgba(239, 68, 68, 0.8);
  }

  .cancel-btn {
    background: none;
    border: none;
    color: rgba(241, 245, 249, 0.3);
    cursor: pointer;
    padding: 2px 4px;
    font-size: 12px;
    line-height: 1;
    flex-shrink: 0;
    transition: color 0.15s;
  }

  .cancel-btn:hover {
    color: rgba(239, 68, 68, 0.8);
  }

  .files {
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }

  .file-path {
    font-size: 11px;
    font-family: ui-monospace, monospace;
    color: rgba(241, 245, 249, 0.5);
    background: rgba(255, 255, 255, 0.04);
    padding: 2px 6px;
    border-radius: 4px;
    max-width: 200px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .more {
    font-size: 11px;
    color: rgba(241, 245, 249, 0.3);
    padding: 2px 6px;
  }

  .stats {
    display: flex;
    gap: 12px;
    font-size: 11px;
    color: rgba(241, 245, 249, 0.4);
  }

  .error-msg {
    font-size: 11px;
    color: rgba(239, 68, 68, 0.7);
    padding: 4px 8px;
    background: rgba(239, 68, 68, 0.08);
    border-radius: 4px;
    border: 1px solid rgba(239, 68, 68, 0.2);
  }
</style>
