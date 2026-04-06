// ═══════════════════════════════════════════════
//  Plugin: Click Heatmap
//  Visual canvas-based heatmap of mouse click positions on screen
// ═══════════════════════════════════════════════
registerPlugin({
  id: 'click-heatmap',
  name: 'Click Heatmap',
  defaultWidth: 'full',
  defaultOrder: 79,
  _canvas: null,
  _loaded: false,
  render(slot) {
    slot.innerHTML = `<div class="chart-card">
      <div class="section-title"><span class="dot" style="background:var(--accent2)"></span> Click Heatmap</div>
      <canvas class="click-heatmap-canvas" id="click-heatmap-canvas" width="800" height="450"></canvas>
      <div class="click-stats-row" id="click-stats-row"></div>
    </div>`;
    const self = pluginRegistry['click-heatmap'];
    self._canvas = document.getElementById('click-heatmap-canvas');
    self._loaded = false;
  },
  update(d) {
    const self = pluginRegistry['click-heatmap'];
    const canvas = self._canvas;
    const statsRow = document.getElementById('click-stats-row');
    if (!canvas) return;

    // Show basic click stats from existing data
    if (statsRow) {
      statsRow.innerHTML = `
        <span class="click-stat">Total clicks: <span>${fmtNum(d.total_clicks)}</span></span>
        <span class="click-stat">Today: <span>${fmtNum(d.clicks_today)}</span></span>
      `;
    }

    // Fetch and draw click positions from API
    self._fetchAndDraw(canvas);
  },
  _fetchAndDraw(canvas) {
    fetch('/api/click_positions?token=' + encodeURIComponent(_statsToken))
      .then(r => {
        if (!r.ok) throw new Error('Not available');
        return r.json();
      })
      .then(clicks => {
        const self = pluginRegistry['click-heatmap'];
        self._drawHeatmap(canvas, clicks);
      })
      .catch(() => {
        // Endpoint not available yet — draw placeholder
        const ctx = canvas.getContext('2d');
        ctx.fillStyle = '#10121a';
        ctx.fillRect(0, 0, canvas.width, canvas.height);
        ctx.fillStyle = '#6b7094';
        ctx.font = '14px system-ui';
        ctx.textAlign = 'center';
        ctx.fillText('Click position tracking coming soon', canvas.width / 2, canvas.height / 2 - 10);
        ctx.font = '11px system-ui';
        ctx.fillText('No click position data available yet', canvas.width / 2, canvas.height / 2 + 15);
      });
  },
  _drawHeatmap(canvas, clicks) {
    if (!clicks || clicks.length === 0) return;
    const ctx = canvas.getContext('2d');
    const W = canvas.width, H = canvas.height;

    // Find screen bounds
    let maxX = 1920, maxY = 1080;
    for (const c of clicks) {
      if (c.x > maxX) maxX = c.x;
      if (c.y > maxY) maxY = c.y;
    }

    ctx.fillStyle = '#10121a';
    ctx.fillRect(0, 0, W, H);

    // Draw click dots with additive blending for heat effect
    ctx.globalCompositeOperation = 'lighter';
    for (const c of clicks) {
      const x = (c.x / maxX) * W;
      const y = (c.y / maxY) * H;
      const count = c.count || 1;
      const radius = Math.min(8 + Math.sqrt(count) * 2, 30);
      const alpha = Math.min(0.05 + count * 0.02, 0.4);

      const gradient = ctx.createRadialGradient(x, y, 0, x, y, radius);
      gradient.addColorStop(0, `rgba(124, 106, 255, ${alpha})`);
      gradient.addColorStop(0.5, `rgba(255, 107, 138, ${alpha * 0.5})`);
      gradient.addColorStop(1, 'rgba(0, 0, 0, 0)');
      ctx.fillStyle = gradient;
      ctx.fillRect(x - radius, y - radius, radius * 2, radius * 2);
    }
    ctx.globalCompositeOperation = 'source-over';
  }
});
