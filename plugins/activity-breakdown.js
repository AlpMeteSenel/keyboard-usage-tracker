// ═══════════════════════════════════════════════
//  Plugin: Activity Breakdown
//  Per-period stats: this hour (navigable), today, week, month, all time
// ═══════════════════════════════════════════════
registerPlugin({
  id: 'activity-breakdown',
  name: 'Activity Breakdown',
  defaultWidth: 'full',
  defaultOrder: 20,
  _hourOffset: 0,
  render(slot) {
    const self = pluginRegistry['activity-breakdown'];
    self._hourOffset = 0;
    slot.innerHTML = `
    <div class="section-title" style="margin-bottom:.8rem"><span class="dot" style="background:var(--orange)"></span> Activity Breakdown</div>
    <div class="period-grid" id="period-grid">
      <div class="period-card" id="period-hour">
        <div class="period-nav">
          <button class="period-nav-btn" id="hour-prev" title="Previous hour">&#9664;</button>
          <div class="period-name" id="hour-label">This Hour</div>
          <button class="period-nav-btn" id="hour-next" title="Next hour" disabled>&#9654;</button>
        </div>
        <div class="period-row"><span class="period-label">Keys</span><span class="period-value pv-keys" data-p="keys">—</span></div>
        <div class="period-row"><span class="period-label">Clicks</span><span class="period-value pv-clicks" data-p="clicks">—</span></div>
        <div class="period-row"><span class="period-label">Avg WPM</span><span class="period-value pv-wpm" data-p="wpm">—</span></div>
        <div class="period-row"><span class="period-label">Active</span><span class="period-value pv-active" data-p="active">—</span></div>
      </div>
      <div class="period-card" id="period-today">
        <div class="period-name">Today</div>
        <div class="period-row"><span class="period-label">Keys</span><span class="period-value pv-keys" data-p="keys">—</span></div>
        <div class="period-row"><span class="period-label">Clicks</span><span class="period-value pv-clicks" data-p="clicks">—</span></div>
        <div class="period-row"><span class="period-label">Avg WPM</span><span class="period-value pv-wpm" data-p="wpm">—</span></div>
        <div class="period-row"><span class="period-label">Active</span><span class="period-value pv-active" data-p="active">—</span></div>
      </div>
      <div class="period-card" id="period-week">
        <div class="period-name">This Week</div>
        <div class="period-row"><span class="period-label">Keys</span><span class="period-value pv-keys" data-p="keys">—</span></div>
        <div class="period-row"><span class="period-label">Clicks</span><span class="period-value pv-clicks" data-p="clicks">—</span></div>
        <div class="period-row"><span class="period-label">Avg WPM</span><span class="period-value pv-wpm" data-p="wpm">—</span></div>
        <div class="period-row"><span class="period-label">Active</span><span class="period-value pv-active" data-p="active">—</span></div>
      </div>
      <div class="period-card" id="period-month">
        <div class="period-name">This Month</div>
        <div class="period-row"><span class="period-label">Keys</span><span class="period-value pv-keys" data-p="keys">—</span></div>
        <div class="period-row"><span class="period-label">Clicks</span><span class="period-value pv-clicks" data-p="clicks">—</span></div>
        <div class="period-row"><span class="period-label">Avg WPM</span><span class="period-value pv-wpm" data-p="wpm">—</span></div>
        <div class="period-row"><span class="period-label">Active</span><span class="period-value pv-active" data-p="active">—</span></div>
      </div>
      <div class="period-card" id="period-alltime">
        <div class="period-name">All Time</div>
        <div class="period-row"><span class="period-label">Keys</span><span class="period-value pv-keys" data-p="keys">—</span></div>
        <div class="period-row"><span class="period-label">Clicks</span><span class="period-value pv-clicks" data-p="clicks">—</span></div>
        <div class="period-row"><span class="period-label">Avg WPM</span><span class="period-value pv-wpm" data-p="wpm">—</span></div>
        <div class="period-row"><span class="period-label">Active</span><span class="period-value pv-active" data-p="active">—</span></div>
      </div>
    </div>`;
  },
  init() {
    const self = pluginRegistry['activity-breakdown'];
    function hourLabel(offset) {
      if (offset === 0) return 'This Hour';
      if (offset === 1) return '1 Hour Ago';
      return offset + ' Hours Ago';
    }
    function hourNav(delta) {
      self._hourOffset = Math.max(0, self._hourOffset + delta);
      const lbl = document.getElementById('hour-label');
      if (lbl) lbl.textContent = hourLabel(self._hourOffset);
      const nb = document.getElementById('hour-next');
      if (nb) nb.disabled = (self._hourOffset === 0);
      fetch('/api/hour_stats?offset=' + self._hourOffset + '&token=' + encodeURIComponent(_statsToken))
        .then(r => r.json())
        .then(stat => {
          const card = document.getElementById('period-hour');
          if (!card) return;
          card.querySelectorAll('.period-value').forEach(v => {
            const p = v.dataset.p;
            if (p === 'keys') v.textContent = fmtNum(stat.keys);
            else if (p === 'clicks') v.textContent = fmtNum(stat.clicks);
            else if (p === 'wpm') v.textContent = stat.wpm ? stat.wpm.toFixed(1) : '0';
            else if (p === 'active') v.textContent = stat.active_minutes + ' min';
          });
        }).catch(() => {});
    }
    const prevBtn = document.getElementById('hour-prev');
    const nextBtn = document.getElementById('hour-next');
    if (prevBtn) prevBtn.addEventListener('click', () => hourNav(1));
    if (nextBtn) nextBtn.addEventListener('click', () => hourNav(-1));
  },
  update(d) {
    const self = pluginRegistry['activity-breakdown'];
    const ps = d.period_stats;
    if (!ps) return;
    const map = {
      'period-today': ps.today,
      'period-week': ps.this_week,
      'period-month': ps.this_month,
      'period-alltime': ps.all_time,
    };
    if (self._hourOffset === 0) map['period-hour'] = ps.this_hour;
    for (const [id, stat] of Object.entries(map)) {
      if (!stat) continue;
      const card = document.getElementById(id);
      if (!card) continue;
      card.querySelectorAll('.period-value').forEach(v => {
        const p = v.dataset.p;
        if (p === 'keys') v.textContent = fmtNum(stat.keys);
        else if (p === 'clicks') v.textContent = fmtNum(stat.clicks);
        else if (p === 'wpm') v.textContent = stat.wpm ? stat.wpm.toFixed(1) : '0';
        else if (p === 'active') v.textContent = stat.active_minutes + ' min';
      });
    }
  }
});
