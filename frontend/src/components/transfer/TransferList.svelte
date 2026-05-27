<script lang="ts">
  import { transfers } from "../../lib/stores/transfers.svelte";
  import TransferItem from "./TransferItem.svelte";

  let { peerSelected = false }: { peerSelected?: boolean } = $props();
  let activeList = $derived(transfers.active());
</script>

<div class="transfer-list">
  {#if activeList.length === 0}
    <div class="empty">
      <span class="empty-icon">📂</span>
      {#if peerSelected}
        <p>拖放文件到此，或点击 <strong>发送文件</strong></p>
      {:else}
        <p>请先选择左侧设备，再拖放文件</p>
      {/if}
    </div>
  {:else}
    {#each activeList as transfer (transfer.id)}
      <TransferItem {transfer} />
    {/each}
  {/if}
</div>

<style>
  .transfer-list {
    display: flex;
    flex-direction: column;
    gap: 8px;
    flex: 1;
    overflow-y: auto;
    padding: 4px 0;
  }

  .empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 12px;
    flex: 1;
    color: rgba(241, 245, 249, 0.25);
    padding: 40px 20px;
    text-align: center;
  }

  .empty-icon {
    font-size: 40px;
    opacity: 0.4;
  }

  .empty p {
    font-size: 13px;
    margin: 0;
    line-height: 1.5;
  }
</style>
