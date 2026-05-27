<script lang="ts">
  let { class: className = "", children }: { class?: string; children?: any } = $props();
</script>

<div class="glass-card {className}">
  {@render children?.()}
</div>
