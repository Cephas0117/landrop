<script lang="ts">
  let { value = 0, class: className = "" }: { value?: number; class?: string } = $props();
  let pct = $derived(Math.min(100, Math.max(0, value * 100)));
</script>

<div class="progress-track {className}" role="progressbar" aria-valuenow={pct} aria-valuemin={0} aria-valuemax={100}>
  <div class="progress-fill" style="width: {pct}%"></div>
</div>

<style>
  .progress-track {
    height: 4px;
    background: rgba(255, 255, 255, 0.08);
    border-radius: 2px;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background: linear-gradient(90deg, var(--accent), var(--accent2));
    border-radius: 2px;
    transition: width 0.3s ease;
  }
</style>
