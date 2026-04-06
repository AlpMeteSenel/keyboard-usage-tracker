// ═══════════════════════════════════════════════
//  Plugin: Hold Duration
//  Table + histogram showing average & max key hold times
// ═══════════════════════════════════════════════
registerPlugin({
  id: 'hold-duration',
  name: 'Hold Duration',
  defaultWidth: 'half',
  defaultOrder: 61,
  _view: 'table',
  render(slot) {
    slot.innerHTML = `<div class="chart-card">
      <div class="section-header">
        <div class="section-title"><span class="dot" style="background:var(--orange)"></span> Hold Duration Analysis</div>
        <div class="toggle-group">
          <button class="view-toggle active" id="hold-tbl-btn" title="Table view">&#9776;</button>
          <button class="view-toggle" id="hold-hist-btn" title="Histogram view">&#9635;</button>
        </div>
      </div>
      <div id="hold-table" style="max-height:400px;overflow-y:auto"><div class="chart-empty">No hold data yet.</div></div>
      <div id="hold-histogram" class="hold-histogram" style="max-height:400px;overflow-y:auto;display:none"><div class="chart-empty">No hold data yet.</div></div>
    </div>`;
  },
  init() {
    const self = pluginRegistry['hold-duration'];
    function setView(mode) {
      self._view = mode;
      const tbl = document.getElementById('hold-table');
      const hist = document.getElementById('hold-histogram');
      if (tbl) tbl.style.display = mode === 'table' ? '' : 'none';
      if (hist) hist.style.display = mode === 'histogram' ? '' : 'none';
      const tb = document.getElementById('hold-tbl-btn');
      const hb = document.getElementById('hold-hist-btn');
      if (tb) tb.classList.toggle('active', mode === 'table');
      if (hb) hb.classList.toggle('active', mode === 'histogram');
    }
    const tb = document.getElementById('hold-tbl-btn');
    const hb = document.getElementById('hold-hist-btn');
    if (tb) tb.addEventListener('click', () => setView('table'));
    if (hb) hb.addEventListener('click', () => setView('histogram'));
  },
  update(d) {
    const holdEl = document.getElementById('hold-table');
    if (holdEl && d.held_keys && d.held_keys.length > 0) {
      holdEl.innerHTML = `<table class="hold-table"><thead><tr><th>Key</th><th>Avg Hold</th><th>Max Hold</th><th>Presses</th></tr></thead><tbody>${d.held_keys.map(h =>
        `<tr><td class="key-col">${esc(h.key_name)}</td><td class="ms-col">${Math.round(h.avg_hold_ms)} ms</td><td class="ms-col">${fmtNum(h.max_hold_ms)} ms</td><td>${fmtNum(h.total_holds)}</td></tr>`
      ).join('')}</tbody></table>`;
    }
    const histEl = document.getElementById('hold-histogram');
    if (histEl && d.held_keys && d.held_keys.length > 0) {
      const absMax = Math.max(...d.held_keys.map(x => x.max_hold_ms), 1);
      let html = '';
      for (const h of d.held_keys) {
        const avgPct = Math.max((h.avg_hold_ms / absMax) * 100, 1);
        const maxPct = Math.max((h.max_hold_ms / absMax) * 100, 1);
        html += `<div class="hbar-row"><div class="hbar-label">${esc(h.key_name)}</div><div class="hbar-track"><div class="hbar-fill hbar-fill-max" style="width:${maxPct}%"></div><div class="hbar-fill hbar-fill-avg" style="width:${avgPct}%"></div></div><div class="hbar-value">${Math.round(h.avg_hold_ms)} ms</div></div>`;
      }
      html += `<div class="hold-legend"><span><span class="hold-legend-swatch" style="background:var(--orange)"></span>Avg hold</span><span><span class="hold-legend-swatch" style="background:var(--accent2);opacity:.45"></span>Max hold</span></div>`;
      histEl.innerHTML = html;
    }
  }
});
