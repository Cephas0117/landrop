export function formatSpeed(bps: number): string {
  if (bps >= 1024 * 1024 * 1024) return `${(bps / (1024 ** 3)).toFixed(1)} GB/s`;
  if (bps >= 1024 * 1024) return `${(bps / (1024 ** 2)).toFixed(1)} MB/s`;
  if (bps >= 1024) return `${(bps / 1024).toFixed(1)} KB/s`;
  return `${bps.toFixed(0)} B/s`;
}

export function formatSize(bytes: number): string {
  if (bytes >= 1024 * 1024 * 1024) return `${(bytes / (1024 ** 3)).toFixed(2)} GB`;
  if (bytes >= 1024 * 1024) return `${(bytes / (1024 ** 2)).toFixed(1)} MB`;
  if (bytes >= 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${bytes} B`;
}

export function formatEta(secs: number): string {
  if (secs <= 0 || !isFinite(secs)) return "—";
  if (secs < 1) return "< 1s";
  const m = Math.floor(secs / 60);
  const s = Math.floor(secs % 60);
  if (m > 0) return `${m}m ${s}s`;
  return `${s}s`;
}

export function formatPercent(fraction: number): string {
  return `${Math.min(100, Math.round(fraction * 100))}%`;
}
