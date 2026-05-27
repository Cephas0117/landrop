<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { ipc, type AppInfo } from "./lib/ipc";
  import { devices, type Peer } from "./lib/stores/devices.svelte";
  import { transfers } from "./lib/stores/transfers.svelte";
  import { toasts } from "./lib/stores/toast.svelte";

  import Radar from "./components/discovery/Radar.svelte";
  import DeviceCard from "./components/discovery/DeviceCard.svelte";
  import TransferList from "./components/transfer/TransferList.svelte";
  import HistoryPanel from "./components/transfer/HistoryPanel.svelte";
  import DragDropZone from "./components/transfer/DragDropZone.svelte";
  import PinOverlay from "./components/pairing/PinOverlay.svelte";
  import ToastStack from "./components/common/ToastStack.svelte";

  let appInfo = $state<AppInfo | null>(null);
  let selectedPeer = $state<Peer | null>(null);
  let peerList = $derived(devices.asList());
  let booting = $state(true);
  let showSettings = $state(false);

  const unlisteners: Array<() => void> = [];

  onMount(async () => {
    try {
      appInfo = await ipc.appBootstrap();
      await ipc.discoveryStart();
      toasts.success("LANDrop 已就绪", appInfo.device_name);
      toasts.info("正在扫描附近设备…");
    } catch (e) {
      toasts.error("启动失败", String(e));
    } finally {
      booting = false;
    }

    unlisteners.push(
      await ipc.listenPeerUpsert((peer) => {
        const isNew = !devices.peers.has(peer.id);
        devices.addOrUpdate(peer);
        if (isNew) toasts.info(`发现 ${peer.name}`, peer.addr);
      }),
      await ipc.listenPeerExpired((id) => {
        const peer = devices.peers.get(id);
        if (peer) toasts.warn(`${peer.name} 已离线`);
        devices.remove(id);
      }),
      await ipc.listenProgress(({ transfer_id, progress }) => {
        transfers.updateProgress(transfer_id, progress);
      }),
      await ipc.listenCompleted((id) => {
        const t = transfers.map.get(id);
        transfers.updateStatus(id, "Completed");
        toasts.success("传输完成", t?.peerName);
      }),
      await ipc.listenFailed(({ transfer_id, error }) => {
        const t = transfers.map.get(transfer_id);
        transfers.updateStatus(transfer_id, "Failed", error);
        toasts.error("传输失败", t ? `${t.peerName}：${error}` : error);
      })
    );

    const peers = await ipc.listPeers();
    for (const p of peers) devices.addOrUpdate(p);
  });

  onDestroy(async () => {
    for (const fn of unlisteners) fn();
    await ipc.discoveryStop();
  });

  function selectPeer(peer: Peer) {
    selectedPeer = selectedPeer?.id === peer.id ? null : peer;
  }

  async function sendFiles() {
    if (!selectedPeer) return;
    const result = await open({ multiple: true, directory: false });
    if (!result) return;
    const paths = Array.isArray(result) ? result : [result];
    if (paths.length === 0) return;
    try {
      await ipc.queueSend(selectedPeer.id, paths);
      toasts.info(`正在发送 ${paths.length} 个文件至 ${selectedPeer.name}`);
    } catch (e) {
      toasts.error("发送失败", String(e));
    }
  }

  async function changeReceiveDir() {
    const result = await open({ directory: true, multiple: false });
    if (!result || Array.isArray(result)) return;
    try {
      await ipc.setReceiveDir(result);
      if (appInfo) appInfo = { ...appInfo, receive_dir: result };
      toasts.success("接收目录已更新");
    } catch (e) {
      toasts.error("更新目录失败", String(e));
    }
  }
</script>

<!-- Boot splash -->
{#if booting}
  <div class="boot-screen">
    <div class="boot-card">
      <div class="spinner"></div>
      <span class="boot-label">正在启动 LANDrop…</span>
    </div>
  </div>
{/if}

<div class="app-shell" class:hidden={booting}>
  <!-- Left panel: discovery -->
  <aside class="panel panel-left">
    <header class="panel-header">
      <div class="brand">
        <span class="brand-dot"></span>
        LANDrop
      </div>
      <div class="header-right">
        {#if appInfo}
          <span class="device-name">{appInfo.device_name}</span>
        {/if}
        <button
          class="icon-btn"
          class:active={showSettings}
          onclick={() => (showSettings = !showSettings)}
          aria-label="设置"
          title="设置"
        >⚙</button>
      </div>
    </header>

    {#if showSettings}
      <div class="settings-drawer">
        <div class="settings-row">
          <span class="settings-label">接收目录</span>
          <button class="settings-path-btn" onclick={changeReceiveDir} title="点击更改">
            <span class="settings-path">{appInfo?.receive_dir ?? "—"}</span>
            <span class="settings-change-hint">更改…</span>
          </button>
        </div>
        {#if appInfo}
          <div class="settings-row">
            <span class="settings-label">设备指纹</span>
            <span class="settings-mono">{appInfo.fingerprint.slice(0, 20)}…</span>
          </div>
        {/if}
      </div>
    {/if}

    <Radar onSelect={selectPeer} />

    <div class="device-list">
      {#if peerList.length === 0}
        <div class="empty-state">
          <div class="empty-icon">📡</div>
          <p class="empty-title">暂未发现设备</p>
          <ol class="empty-steps">
            <li>确保所有设备已连接到同一 Wi-Fi</li>
            <li>在对方设备上打开 LANDrop</li>
            <li>设备将自动出现在这里</li>
          </ol>
          <p class="empty-tip">提示：也可通过 IP 地址手动添加设备。</p>
        </div>
      {:else}
        {#each peerList as peer (peer.id)}
          <DeviceCard
            {peer}
            selected={selectedPeer?.id === peer.id}
            onSelect={selectPeer}
          />
        {/each}
      {/if}
    </div>
  </aside>

  <!-- Right panel: transfers -->
  <main class="panel panel-right">
    <header class="panel-header">
      <span class="panel-title">传输</span>
      <div class="header-right">
        {#if selectedPeer}
          <span class="selected-hint">
            发送至 <strong>{selectedPeer.name}</strong>
          </span>
          <button class="send-btn" onclick={sendFiles}>
            <span aria-hidden="true">📤</span> 发送文件
          </button>
        {:else}
          <span class="selected-hint muted">请先选择设备</span>
        {/if}
      </div>
    </header>

    {#if !selectedPeer && transfers.asList().length === 0}
      <div class="empty-state center">
        <div class="empty-icon large">💻</div>
        <p class="empty-title">准备就绪</p>
        <ol class="empty-steps">
          <li>从左侧面板选择一台设备</li>
          <li>拖放文件到此窗口，<br>或点击 <strong>发送文件</strong></li>
          <li>在对方设备上确认接收</li>
        </ol>
      </div>
    {:else}
      <TransferList peerSelected={!!selectedPeer} />
      <HistoryPanel />
    {/if}
  </main>

  <!-- Global overlays -->
  <DragDropZone {selectedPeer} />
  <PinOverlay />
</div>

<!-- Toast notifications -->
<ToastStack />

<style>
  .app-shell {
    display: flex;
    height: 100vh;
    overflow: hidden;
    transition: opacity 0.2s;
  }

  .app-shell.hidden {
    opacity: 0;
    pointer-events: none;
  }

  /* Boot splash */
  .boot-screen {
    position: fixed;
    inset: 0;
    z-index: 10000;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--bg-dark, #06071a);
  }

  .boot-card {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 20px;
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

  .boot-label {
    font-size: 14px;
    color: rgba(241, 245, 249, 0.45);
    letter-spacing: 0.02em;
  }

  /* Layout */
  .panel {
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .panel-left {
    width: 40%;
    min-width: 280px;
    border-right: 1px solid var(--glass-border);
    background: rgba(255, 255, 255, 0.02);
  }

  .panel-right {
    flex: 1;
    padding: 0 16px 16px;
    gap: 12px;
  }

  .panel-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 14px 16px;
    border-bottom: 1px solid var(--glass-border);
    flex-shrink: 0;
    gap: 8px;
  }

  .header-right {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }

  /* Brand */
  .brand {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 16px;
    font-weight: 700;
    color: rgba(241, 245, 249, 0.95);
    letter-spacing: -0.02em;
    flex-shrink: 0;
  }

  .brand-dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--accent);
    box-shadow: 0 0 8px var(--accent);
    flex-shrink: 0;
  }

  .device-name {
    font-size: 11px;
    color: rgba(241, 245, 249, 0.35);
    font-family: ui-monospace, monospace;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 120px;
  }

  /* Settings icon button */
  .icon-btn {
    flex-shrink: 0;
    background: none;
    border: none;
    color: rgba(241, 245, 249, 0.35);
    font-size: 14px;
    cursor: pointer;
    padding: 4px 6px;
    border-radius: 6px;
    transition: color 0.15s, background 0.15s;
    line-height: 1;
  }

  .icon-btn:hover,
  .icon-btn.active {
    color: rgba(241, 245, 249, 0.8);
    background: rgba(255, 255, 255, 0.08);
  }

  /* Settings drawer */
  .settings-drawer {
    border-bottom: 1px solid var(--glass-border);
    background: rgba(255, 255, 255, 0.025);
    padding: 10px 16px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    flex-shrink: 0;
    animation: slideDown 0.15s ease forwards;
  }

  @keyframes slideDown {
    from { opacity: 0; transform: translateY(-6px); }
    to   { opacity: 1; transform: translateY(0); }
  }

  .settings-row {
    display: flex;
    align-items: center;
    gap: 8px;
    font-size: 12px;
  }

  .settings-label {
    color: rgba(241, 245, 249, 0.4);
    flex-shrink: 0;
    min-width: 80px;
  }

  .settings-path-btn {
    display: flex;
    align-items: center;
    gap: 8px;
    background: none;
    border: none;
    cursor: pointer;
    padding: 4px 8px;
    border-radius: 6px;
    border: 1px solid rgba(255, 255, 255, 0.08);
    transition: border-color 0.15s;
    min-width: 0;
    flex: 1;
  }

  .settings-path-btn:hover {
    border-color: rgba(99, 102, 241, 0.4);
  }

  .settings-path {
    flex: 1;
    font-size: 11px;
    font-family: ui-monospace, monospace;
    color: rgba(241, 245, 249, 0.55);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    text-align: left;
  }

  .settings-change-hint {
    font-size: 11px;
    color: var(--accent);
    flex-shrink: 0;
    opacity: 0.7;
  }

  .settings-mono {
    font-size: 11px;
    font-family: ui-monospace, monospace;
    color: rgba(241, 245, 249, 0.3);
    letter-spacing: 0.03em;
  }

  /* Device list */
  .device-list {
    flex: 1;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 4px;
    padding: 8px;
  }

  /* Empty states */
  .empty-state {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 28px 20px;
    gap: 8px;
    text-align: center;
  }

  .empty-state.center {
    flex: 1;
    justify-content: center;
  }

  .empty-icon {
    font-size: 32px;
    margin-bottom: 4px;
    opacity: 0.6;
  }

  .empty-icon.large {
    font-size: 48px;
  }

  .empty-title {
    font-size: 14px;
    font-weight: 600;
    color: rgba(241, 245, 249, 0.6);
    margin: 0;
  }

  .empty-steps {
    font-size: 12px;
    color: rgba(241, 245, 249, 0.35);
    line-height: 1.7;
    text-align: left;
    padding-left: 20px;
    margin: 4px 0 0;
  }

  .empty-steps li {
    margin-bottom: 2px;
  }

  .empty-tip {
    font-size: 11px;
    color: rgba(99, 102, 241, 0.5);
    margin: 4px 0 0;
  }

  /* Panel right */
  .panel-title {
    font-size: 14px;
    font-weight: 600;
    color: rgba(241, 245, 249, 0.8);
    flex-shrink: 0;
  }

  .selected-hint {
    font-size: 12px;
    color: rgba(99, 102, 241, 0.8);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .selected-hint.muted {
    color: rgba(241, 245, 249, 0.25);
  }

  /* Send files button */
  .send-btn {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 6px 14px;
    background: rgba(99, 102, 241, 0.2);
    border: 1px solid rgba(99, 102, 241, 0.4);
    border-radius: 8px;
    color: rgba(241, 245, 249, 0.9);
    font-size: 12px;
    font-weight: 500;
    cursor: pointer;
    transition: background 0.15s, border-color 0.15s;
    flex-shrink: 0;
  }

  .send-btn:hover {
    background: rgba(99, 102, 241, 0.35);
    border-color: rgba(99, 102, 241, 0.65);
  }

  .send-btn:active {
    background: rgba(99, 102, 241, 0.45);
  }
</style>
