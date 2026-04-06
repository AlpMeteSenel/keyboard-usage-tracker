// ═══════════════════════════════════════════════
//  Plugin: Recent Activity
//  Shows the last 50 key presses with timestamps and hold durations
// ═══════════════════════════════════════════════
registerPlugin({
  id: 'recent-activity',
  name: 'Recent Activity',
  defaultWidth: 'full',
  defaultOrder: 200,
  render(slot) {
    slot.innerHTML = `<div class="chart-card">
      <div class="section-title"><span class="dot" style="background:var(--green)"></span> Recent Activity (Last 50 Keys)</div>
      <div id="recent"><div class="chart-empty">Waiting for data…</div></div>
    </div>`;
  },
  update(d) {
    const recentEl = document.getElementById('recent');
    if (!recentEl) return;
    if (d.recent && d.recent.length > 0) {
      recentEl.innerHTML = '<div class="recent-feed">' + d.recent.map(r => {
        const time = r.timestamp ? (r.timestamp.split('T')[1] || r.timestamp) : '';
        const hold = r.hold_ms != null ? ` (${r.hold_ms}ms)` : '';
        return `<span class="recent-key">${esc(r.key_name)}${esc(hold)}<span class="ts">${esc(time)}</span></span>`;
      }).join('') + '</div>';
    }
  }
});
