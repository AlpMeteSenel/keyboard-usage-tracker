// ═══════════════════════════════════════════════
//  Plugin: Key Combos
//  Displays keyboard shortcut usage ranked by frequency
//  Uses server-side combo detection from the full event history
// ═══════════════════════════════════════════════
registerPlugin({
  id: 'key-combos',
  name: 'Key Combos',
  defaultWidth: 'half',
  defaultOrder: 62,
  render(slot) {
    slot.innerHTML = `<div class="chart-card">
      <div class="section-title"><span class="dot" style="background:var(--accent)"></span> Key Combinations</div>
      <div id="key-combos"><div class="chart-empty">Analyzing shortcuts…</div></div>
    </div>`;
  },
  update(d) {
    const el = document.getElementById('key-combos');
    if (!el) return;

    const combos = d.key_combos;
    if (!combos || combos.length === 0) {
      el.innerHTML = '<div class="chart-empty">No keyboard shortcuts detected yet. Try using Ctrl+C, Alt+Tab, etc.</div>';
      return;
    }

    const max = combos[0].count;
    el.innerHTML = combos.slice(0, 15).map(item => {
      const pct = Math.max((item.count / max) * 100, 1);
      return `<div class="bar-row"><div class="bar-label">${esc(item.combo)}</div><div class="bar-track"><div class="bar-fill" style="width:${pct}%;background:linear-gradient(90deg,var(--cyan),var(--green))"></div></div><div class="bar-count">${fmtNum(item.count)}</div></div>`;
    }).join('');
  }
});
