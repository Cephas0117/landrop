<script lang="ts">
  import { toasts, type ToastKind } from "../../lib/stores/toast.svelte";

  const icons: Record<ToastKind, string> = {
    success: "✓",
    error: "✕",
    warn: "⚠",
    info: "ℹ",
  };

  const colors: Record<ToastKind, string> = {
    success: "var(--success)",
    error: "var(--danger)",
    warn: "var(--warn)",
    info: "var(--accent)",
  };
</script>

<div class="toast-stack" aria-live="polite">
  {#each toasts.list as toast (toast.id)}
    <div class="toast" style="--color: {colors[toast.kind]}">
      <span class="toast-icon">{icons[toast.kind]}</span>
      <div class="toast-body">
        <span class="toast-msg">{toast.message}</span>
        {#if toast.detail}
          <span class="toast-detail">{toast.detail}</span>
        {/if}
      </div>
      <button class="toast-close" onclick={() => toasts.dismiss(toast.id)} aria-label="Dismiss">✕</button>
    </div>
  {/each}
</div>

<style>
  .toast-stack {
    position: fixed;
    bottom: 24px;
    right: 24px;
    z-index: 9999;
    display: flex;
    flex-direction: column;
    gap: 8px;
    pointer-events: none;
  }

  .toast {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    min-width: 260px;
    max-width: 380px;
    padding: 12px 14px;
    background: rgba(10, 11, 30, 0.92);
    backdrop-filter: blur(20px);
    border: 1px solid rgba(255, 255, 255, 0.1);
    border-left: 3px solid var(--color);
    border-radius: 10px;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
    pointer-events: all;
    animation: slideInRight 0.25s cubic-bezier(0.34, 1.56, 0.64, 1) forwards;
  }

  .toast-icon {
    flex-shrink: 0;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: var(--color);
    color: #fff;
    font-size: 10px;
    font-weight: 700;
    display: flex;
    align-items: center;
    justify-content: center;
    margin-top: 1px;
  }

  .toast-body {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 2px;
    min-width: 0;
  }

  .toast-msg {
    font-size: 13px;
    font-weight: 500;
    color: rgba(241, 245, 249, 0.95);
    line-height: 1.4;
  }

  .toast-detail {
    font-size: 11px;
    color: rgba(241, 245, 249, 0.45);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .toast-close {
    flex-shrink: 0;
    background: none;
    border: none;
    color: rgba(241, 245, 249, 0.3);
    font-size: 11px;
    cursor: pointer;
    padding: 2px;
    transition: color 0.15s;
    line-height: 1;
    margin-top: 1px;
  }

  .toast-close:hover {
    color: rgba(241, 245, 249, 0.7);
  }

  @keyframes slideInRight {
    from { opacity: 0; transform: translateX(24px) scale(0.95); }
    to   { opacity: 1; transform: translateX(0) scale(1); }
  }
</style>
