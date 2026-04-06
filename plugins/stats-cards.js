// ═══════════════════════════════════════════════
//  Plugin: Stats Cards
//  Shows key metrics: total keys, clicks, WPM, active time, unique keys
// ═══════════════════════════════════════════════
registerPlugin({
  id: 'stats-cards',
  name: 'Stats Cards',
  defaultWidth: 'full',
  defaultOrder: 10,
  render(slot) {
    slot.innerHTML = `<div class="stats-grid">
      <div class="stat-card"><div class="label">Total Keystrokes</div><div class="value v-accent" id="total-keys">—</div><div class="sub" id="keys-today-sub">Today: —</div></div>
      <div class="stat-card"><div class="label">Total Clicks</div><div class="value v-accent2" id="total-clicks">—</div><div class="sub" id="clicks-today-sub">Today: —</div></div>
      <div class="stat-card"><div class="label">Current WPM</div><div class="value v-green" id="current-wpm">—</div><div class="sub">Last 60 seconds</div></div>
      <div class="stat-card"><div class="label">Average WPM</div><div class="value v-cyan" id="avg-wpm">—</div><div class="sub">Per active minute</div></div>
      <div class="stat-card"><div class="label">Best WPM</div><div class="value v-orange" id="best-wpm">—</div><div class="sub">Peak 1-minute burst</div></div>
      <div class="stat-card"><div class="label">Active Time Today</div><div class="value v-accent" id="active-time">—</div><div class="sub">Minutes of typing</div></div>
      <div class="stat-card"><div class="label">Unique Keys</div><div class="value v-green" id="unique-keys">—</div><div class="sub">Distinct keys used</div></div>
    </div>`;
  },
  update(d) {
    animateValue(document.getElementById('total-keys'), d.total_keys, 400);
    animateValue(document.getElementById('total-clicks'), d.total_clicks, 400);
    const ktSub = document.getElementById('keys-today-sub');
    if (ktSub) ktSub.textContent = 'Today: ' + fmtNum(d.keys_today);
    const ctSub = document.getElementById('clicks-today-sub');
    if (ctSub) ctSub.textContent = 'Today: ' + fmtNum(d.clicks_today);
    const cw = document.getElementById('current-wpm');
    if (cw) cw.textContent = d.current_wpm ? d.current_wpm.toFixed(1) : '0';
    const aw = document.getElementById('avg-wpm');
    if (aw) aw.textContent = d.avg_wpm ? d.avg_wpm.toFixed(1) : '0';
    const bw = document.getElementById('best-wpm');
    if (bw) bw.textContent = d.best_wpm ? d.best_wpm.toFixed(1) : '0';
    const at = document.getElementById('active-time');
    if (at) at.textContent = d.active_minutes_today || '0';
    const uk = document.getElementById('unique-keys');
    if (uk) uk.textContent = d.top_keys ? d.top_keys.length : '0';
  }
});
