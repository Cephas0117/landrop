const ALPHA = 0.25;

export class EwmaCalculator {
  private lastBytes = 0;
  private lastTimestamp = Date.now();
  private speedBps = 0;

  update(totalBytes: number, remainingBytes: number): { speedMbSec: number; etaSecs: number } {
    const now = Date.now();
    const dt = (now - this.lastTimestamp) / 1000;

    if (dt >= 0.05) {
      const delta = totalBytes - this.lastBytes;
      const instantSpeed = delta / dt;
      this.speedBps = ALPHA * instantSpeed + (1 - ALPHA) * this.speedBps;
      this.lastBytes = totalBytes;
      this.lastTimestamp = now;
    }

    const speedMbSec = this.speedBps / (1024 * 1024);
    const etaSecs = this.speedBps > 0 ? remainingBytes / this.speedBps : 0;

    return { speedMbSec, etaSecs };
  }

  reset() {
    this.lastBytes = 0;
    this.lastTimestamp = Date.now();
    this.speedBps = 0;
  }
}
