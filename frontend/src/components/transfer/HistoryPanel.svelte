<script lang="ts">
  import { transfers, type Transfer } from "../../lib/stores/transfers.svelte";
  import { formatSize } from "../../lib/utils/format";

  let collapsed = $state(true);
  let history = $derived(
    transfers.asList().filter(
      (t) => t.status === "Completed" || t.status === "Failed" || t.status === "Canceled"
    )
  );
</script>

<div class="history-panel glass-card">
  <button class="toggle" onclick={() => (collapsed = !collapsed)}>
    <span>传输记录</span>
    <span class="badge">{history.length}</span>
    <span class="chevron" class:open={!collapsed}>›</span>
  </button>

  {#if !collapsed}
    <div class="history-list">
      {#if history.length === 0}
        <p class="empty">暂无传输记录</p>
      {:else}
        {#each history as t (t.id)}
          <div class="history-row" class:failed={t.status === "Failed"}>
            <span class="dir">{t.direction === "Send" ? "↑" : "↓"}</span>
            <span class="name">{t.paths[0]?.split("/").pop() ?? "–"}</span>
            <span class="size">{formatSize(t.progress.totalBytes)}</span>
            <span class="peer">{t.peerName}</span>
            <span class="st">{t.status}</span>
          </div>
        {/each}
      {/if}
    </div>
  {/if}
</div>

<style>
  .history-panel {
    overflow: hidden;
  }

  .toggle {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    background: none;
    border: none;
    color: rgba(241, 245, 249, 0.7);
    font-size: 13px;
    font-weight: 500;
    cursor: pointer;
    padding: 12px 16px;
    text-align: left;
    transition: color 0.15s;
  }

  .toggle:hover {
    color: rgba(241, 245, 249, 0.9);
  }

  .badge {
    background: rgba(99, 102, 241, 0.3);
    color: var(--accent);
    font-size: 10px;
    padding: 1px 6px;
    border-radius: 10px;
    min-width: 20px;
    text-align: center;
  }

  .chevron {
    margin-left: auto;
    font-size: 16px;
    transform: rotate(90deg);
    transition: transform 0.2s;
    display: inline-block;
  }

  .chevron.open {
    transform: rotate(-90deg);
  }

  .history-list {
    border-top: 1px solid var(--glass-border);
    max-height: 200px;
    overflow-y: auto;
  }

  .history-row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 8px 16px;
    font-size: 12px;
    color: rgba(241, 245, 249, 0.5);
    border-bottom: 1px solid rgba(255, 255, 255, 0.04);
  }

  .history-row.failed .st {
    color: rgba(239, 68, 68, 0.7);
  }

  .dir {
    color: var(--accent);
    flex-shrink: 0;
  }

  .name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    font-family: ui-monospace, monospace;
    font-size: 11px;
  }

  .size, .peer, .st {
    flex-shrink: 0;
    font-size: 11px;
  }

  .empty {
    padding: 16px;
    text-align: center;
    font-size: 12px;
    color: rgba(241, 245, 249, 0.25);
    margin: 0;
  }
</style>
