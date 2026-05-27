<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { devices, type Peer } from "../../lib/stores/devices.svelte";

  let { onSelect }: { onSelect?: (peer: Peer) => void } = $props();

  const SIZE = 320;
  const CENTER = SIZE / 2;
  const RINGS = [0.25, 0.45, 0.65, 0.85];

  function peerPosition(index: number, total: number) {
    const ring = RINGS[index % RINGS.length];
    const angle = (index / Math.max(total, 1)) * Math.PI * 2 - Math.PI / 2;
    const r = (SIZE / 2) * ring;
    return {
      x: CENTER + r * Math.cos(angle),
      y: CENTER + r * Math.sin(angle),
    };
  }

  function osIcon(os: string): string {
    if (os.toLowerCase().includes("win")) return "🪟";
    if (os.toLowerCase().includes("mac") || os.toLowerCase().includes("darwin")) return "🍎";
    return "🐧";
  }

  let intervalId: ReturnType<typeof setInterval>;

  onMount(() => {
    intervalId = setInterval(() => devices.clearExpired(5000), 4000);
  });

  onDestroy(() => {
    clearInterval(intervalId);
  });

  let peerList = $derived(devices.asList());
</script>

<div class="radar-container">
  <svg width={SIZE} height={SIZE} viewBox={`0 0 ${SIZE} ${SIZE}`}>
    <!-- Ring definitions -->
    <defs>
      <radialGradient id="radarGrad" cx="50%" cy="50%">
        <stop offset="0%" stop-color="rgba(99,102,241,0.1)" />
        <stop offset="100%" stop-color="transparent" />
      </radialGradient>
    </defs>

    <!-- Background glow -->
    <circle cx={CENTER} cy={CENTER} r={CENTER} fill="url(#radarGrad)" />

    <!-- Concentric rings -->
    {#each RINGS as ring}
      <circle
        cx={CENTER}
        cy={CENTER}
        r={(SIZE / 2) * ring}
        fill="none"
        stroke="rgba(99,102,241,0.2)"
        stroke-width="1"
      />
    {/each}

    <!-- Cross hairs -->
    <line x1={CENTER} y1={8} x2={CENTER} y2={SIZE - 8} stroke="rgba(99,102,241,0.1)" stroke-width="1" />
    <line x1={8} y1={CENTER} x2={SIZE - 8} y2={CENTER} stroke="rgba(99,102,241,0.1)" stroke-width="1" />

    <!-- Pulse rings -->
    <circle
      class="pulse-ring"
      cx={CENTER}
      cy={CENTER}
      r={(SIZE / 2) * 0.35}
      fill="none"
      stroke="rgba(99,102,241,0.4)"
      stroke-width="1.5"
      style="animation: radarPulse 3s ease-out infinite;"
    />
    <circle
      class="pulse-ring"
      cx={CENTER}
      cy={CENTER}
      r={(SIZE / 2) * 0.35}
      fill="none"
      stroke="rgba(99,102,241,0.3)"
      stroke-width="1"
      style="animation: radarPulse 3s ease-out infinite 1.5s;"
    />

    <!-- Rotating scan arm -->
    <g style="transform-origin: {CENTER}px {CENTER}px; animation: radarSpin 4s linear infinite; will-change: transform;">
      <line
        x1={CENTER}
        y1={CENTER}
        x2={CENTER}
        y2={8}
        stroke="rgba(99,102,241,0.8)"
        stroke-width="2"
        stroke-linecap="round"
      />
      <defs>
        <linearGradient id="sweepGrad" x1="0" y1="1" x2="0" y2="0" gradientUnits="objectBoundingBox">
          <stop offset="0%" stop-color="rgba(99,102,241,0)" />
          <stop offset="100%" stop-color="rgba(99,102,241,0.15)" />
        </linearGradient>
      </defs>
      <path
        d={`M ${CENTER},${CENTER} L ${CENTER},${8} A ${CENTER - 8},${CENTER - 8} 0 0,1 ${CENTER + (CENTER - 8) * Math.sin(Math.PI / 6)},${CENTER - (CENTER - 8) * Math.cos(Math.PI / 6)} Z`}
        fill="url(#sweepGrad)"
      />
    </g>

    <!-- Device nodes -->
    {#each peerList as peer, i}
      {@const pos = peerPosition(i, peerList.length)}
      <g
        class="peer-node"
        transform={`translate(${pos.x},${pos.y})`}
        style="cursor:pointer;"
        onclick={() => onSelect?.(peer)}
        onkeydown={(e) => (e.key === "Enter" || e.key === " ") && onSelect?.(peer)}
        role="button"
        tabindex="0"
        aria-label={peer.name}
      >
        <circle r="18" fill="rgba(10,11,30,0.9)" stroke="rgba(99,102,241,0.5)" stroke-width="1.5" />
        <text text-anchor="middle" dominant-baseline="central" font-size="13">{osIcon(peer.os)}</text>
        <text
          y="26"
          text-anchor="middle"
          dominant-baseline="hanging"
          font-size="9"
          fill="rgba(241,245,249,0.7)"
          font-family="-apple-system,sans-serif"
        >{peer.name.slice(0, 10)}</text>
      </g>
    {/each}

    <!-- Center dot -->
    <circle cx={CENTER} cy={CENTER} r="4" fill="rgba(99,102,241,0.8)" />
    <circle cx={CENTER} cy={CENTER} r="8" fill="none" stroke="rgba(99,102,241,0.3)" stroke-width="1" />
  </svg>
</div>

<style>
  .radar-container {
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 16px;
  }

  svg {
    overflow: visible;
  }

  .peer-node {
    animation: fadeIn 0.4s cubic-bezier(0.34, 1.56, 0.64, 1) forwards;
  }

  .peer-node:hover circle:first-child {
    stroke: rgba(99, 102, 241, 0.9);
    fill: rgba(99, 102, 241, 0.15);
  }
</style>
