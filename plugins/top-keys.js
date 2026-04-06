// ═══════════════════════════════════════════════
//  Plugin: Top Keys
//  Horizontal bar chart of most-pressed keys
// ═══════════════════════════════════════════════
registerPlugin({
  id: 'top-keys',
  name: 'Top Keys',
  defaultWidth: 'half',
  defaultOrder: 60,
  render(slot) {
    slot.innerHTML = `<div class="chart-card">
      <div class="section-title"><span class="dot" style="background:var(--accent)"></span> Top Keys</div>
      <div id="bars" style="max-height:400px;overflow-y:auto"><div class="chart-empty">Waiting for data…</div></div>
    </div>`;
  },
  update(d) {
    const barsEl = document.getElementById('bars');
    if (!barsEl) return;
    if (d.top_keys && d.top_keys.length > 0) {
      const max = d.top_keys[0].count;
      barsEl.innerHTML = d.top_keys.slice(0, 20).map(k => {
        const pct = Math.max((k.count/max)*100, 1);
        return `<div class="bar-row"><div class="bar-label">${esc(k.key_name)}</div><div class="bar-track"><div class="bar-fill" style="width:${pct}%"></div></div><div class="bar-count">${fmtNum(k.count)}</div></div>`;
      }).join('');
    }
  }
});
