// ═══════════════════════════════════════════════
//  Plugin: Touchpad Heatmap
//  Visual canvas-based heatmap of touchpad contact zones
// ═══════════════════════════════════════════════
registerPlugin({
  id: 'touchpad-heatmap',
  name: 'Touchpad Heatmap',
  defaultWidth: 'half',
  defaultOrder: 80, // Right after click-heatmap
  _canvas: null,
  render(slot) {
    slot.innerHTML = `<div class="chart-card">
      <div class="section-title"><span class="dot" style="background:var(--accent2)"></span> Touchpad Heatmap</div>
      <div style="position: relative; width: 100%; aspect-ratio: 1.5; background: #10121a; border-radius: 12px; overflow: hidden; border: 1px solid rgba(255,255,255,0.04);">
        <canvas class="touchpad-heatmap-canvas" id="touchpad-heatmap-canvas" width="900" height="600" style="width: 100%; height: 100%;"></canvas>
      </div>
      <div class="click-stats-row" id="touchpad-stats-row" style="margin-top: 15px; display: flex; gap: 15px;">
        <span class="click-stat" style="color: var(--muted); font-size: 0.8rem;">Avg Fingers: <span id="tp-avg-fingers" style="color: var(--text); font-weight: 600;">-</span></span>
        <span class="click-stat" style="color: var(--muted); font-size: 0.8rem;">Time Tracked: <span id="tp-samples" style="color: var(--text); font-weight: 600;">-</span> samples</span>
      </div>
    </div>`;
    const self = pluginRegistry['touchpad-heatmap'];
    self._canvas = document.getElementById('touchpad-heatmap-canvas');
  },
  update(d) {
    const self = pluginRegistry['touchpad-heatmap'];
    const canvas = self._canvas;
    if (!canvas) return;

    self._fetchAndDraw(canvas);
  },
  _fetchAndDraw(canvas) {
    if(!_statsToken) return;
    
    // Fetch heatmap dots
    fetch('/api/touchpad_heatmap?token=' + encodeURIComponent(_statsToken))
      .then(r => {
        if (!r.ok) throw new Error('Not available');
        return r.json();
      })
      .then(points => {
        const self = pluginRegistry['touchpad-heatmap'];
        self._drawHeatmap(canvas, points);
      })
      .catch(() => {});

    // Fetch finger analytics
    fetch('/api/touchpad_fingers?token=' + encodeURIComponent(_statsToken))
      .then(r => r.json())
      .then(data => {
        let totalFingers = 0;
        let totalSamples = 0;
        for (const [fingerCount, hits] of Object.entries(data)) {
            totalFingers += parseInt(fingerCount) * hits;
            totalSamples += hits;
        }
        
        let avg = totalSamples > 0 ? (totalFingers / totalSamples).toFixed(2) : '-';
        document.getElementById('tp-avg-fingers').innerText = avg;
        document.getElementById('tp-samples').innerText = fmtNum(totalSamples);
      })
      .catch(() => {});
  },
  _drawHeatmap(canvas, points) {
    const ctx = canvas.getContext('2d');
    const W = canvas.width, H = canvas.height;

    ctx.fillStyle = '#10121a';
    ctx.fillRect(0, 0, W, H);

    if (!points || points.length === 0) {
      ctx.fillStyle = '#6b7094';
      ctx.font = '14px system-ui';
      ctx.textAlign = 'center';
      ctx.fillText("No touchpad data recorded yet...", W / 2, H / 2);
      return;
    }

    let maxX = 3000, maxY = 2000;
    
    // Auto-scale depending on the maximum bounds seen in the heatmap historical usage so far
    for (const c of points) {
      if (c.x > maxX) maxX = c.x;
      if (c.y > maxY) maxY = c.y;
    }

    ctx.globalCompositeOperation = 'lighter';
    for (const c of points) {
      const x = (c.x / maxX) * W;
      const y = (c.y / maxY) * H;
      const count = c.count || 1;
      
      const radius = Math.min(20 + Math.sqrt(count) * 2, 80); 
      const alpha = Math.min(0.08 + count * 0.01, 0.7);

      const gradient = ctx.createRadialGradient(x, y, 0, x, y, radius);
      gradient.addColorStop(0, `rgba(255, 60, 90, ${alpha})`);     // Inner Hot Pink
      gradient.addColorStop(0.5, `rgba(255, 100, 0, ${alpha * 0.4})`); // Outer Orange glow
      gradient.addColorStop(1, 'rgba(0, 0, 0, 0)');
      
      ctx.fillStyle = gradient;
      ctx.fillRect(x - radius, y - radius, radius * 2, radius * 2);
    }
    ctx.globalCompositeOperation = 'source-over';
  }
});
