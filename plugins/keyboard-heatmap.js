// ═══════════════════════════════════════════════
//  Plugin: Keyboard Heatmap
//  Visual keyboard with heat colors based on key usage
//  Supports multiple keyboard profiles (Turkish Q, US QWERTY, etc.)
// ═══════════════════════════════════════════════
registerPlugin({
  id: 'keyboard-heatmap',
  name: 'Keyboard Heatmap',
  defaultWidth: 'full',
  defaultOrder: 50,
  _currentProfile: localStorage.getItem('kb-profile') || 'turkish-q',
  _lastTopKeys: null,
  render(slot) {
    const self = pluginRegistry['keyboard-heatmap'];
    slot.innerHTML = `<div class="chart-card">
      <div class="section-header">
        <div class="section-title"><span class="dot" style="background:var(--accent2)"></span> Keyboard Heatmap</div>
        <select class="profile-select" id="kb-profile-select"></select>
      </div>
      <div class="keyboard-container"><div class="keyboard" id="keyboard-heatmap"></div></div>
      <div class="legend"><span>Less</span><div class="legend-bar"></div><span>More</span></div>
    </div>`;
    // Build profile selector
    const sel = document.getElementById('kb-profile-select');
    for (const [id, prof] of Object.entries(KEYBOARD_PROFILES)) {
      const opt = document.createElement('option');
      opt.value = id; opt.textContent = prof.name;
      if (id === self._currentProfile) opt.selected = true;
      sel.appendChild(opt);
    }
    self._buildKeyboard();
  },
  init() {
    const self = pluginRegistry['keyboard-heatmap'];
    const sel = document.getElementById('kb-profile-select');
    if (sel) sel.addEventListener('change', function() {
      self._currentProfile = this.value;
      localStorage.setItem('kb-profile', this.value);
      self._buildKeyboard();
      if (self._lastTopKeys) self._applyHeatmap(self._lastTopKeys);
    });
  },
  _createSpacer(w, flex) {
    const sp = document.createElement('div');
    if (flex) { sp.style.flex = '1'; sp.style.minWidth = '14px'; }
    else { sp.style.width = w + 'px'; sp.style.flexShrink = '0'; }
    return sp;
  },
  _createKeyEl(key, tooltipBelow) {
    const el = document.createElement('div');
    const cls = ['kb-key'];
    if (tooltipBelow) cls.push('tt-below');
    if (key.sub) cls.push('kb-key-sub');
    if (key.h > 1) cls.push('kb-key-tall');
    el.className = cls.join(' ');
    el.style.width = key.w + 'px';
    if (key.h > 1) el.style.height = (44 * key.h + 5 * (key.h - 1)) + 'px';
    el.dataset.key = key.d;
    el.dataset.label = key.tooltip || key.k;
    const label = esc(key.k);
    const tooltip = esc(key.tooltip || key.k);
    el.innerHTML =
      `<span class="kb-label">${label}</span>` +
      (key.sub ? `<span class="kb-sub-label">${esc(key.sub)}</span>` : '') +
      `<span class="kb-count">0</span>` +
      `<div class="kb-tooltip"><span class="tt-key">${tooltip}</span> &mdash; <span class="tt-count">0</span> presses</div>`;
    return el;
  },
  _appendKeys(keys, rowEl, tooltipBelow) {
    const self = pluginRegistry['keyboard-heatmap'];
    for (const key of keys) {
      if (key.gap) { rowEl.appendChild(self._createSpacer(key.w)); continue; }
      rowEl.appendChild(self._createKeyEl(key, tooltipBelow));
    }
  },
  _buildKeyboard() {
    const self = pluginRegistry['keyboard-heatmap'];
    const profile = KEYBOARD_PROFILES[self._currentProfile] || KEYBOARD_PROFILES['turkish-q'];
    const el = document.getElementById('keyboard-heatmap');
    if (!el) return;
    el.innerHTML = '';

    for (let rowIdx = 0; rowIdx < profile.rows.length; rowIdx++) {
      const row = profile.rows[rowIdx];
      const rowEl = document.createElement('div');
      rowEl.className = 'kb-row';

      // Overlap row if previous row has tall keys (e.g. Num+, NumEnter)
      if (rowIdx > 0 && profile.rows[rowIdx - 1].some(k => !k.gap && k.h > 1))
        rowEl.classList.add('kb-row-overlap');

      // Split at first separator gap (w>=14) to flex-align the numpad
      const sepIdx = row.findIndex(k => k.gap && k.w >= 14);
      const mainKeys = sepIdx >= 0 ? row.slice(0, sepIdx) : row;
      const numpadKeys = sepIdx >= 0 ? row.slice(sepIdx + 1) : [];
      const isTopRow = rowIdx === 0;

      self._appendKeys(mainKeys, rowEl, isTopRow);
      if (numpadKeys.length) {
        rowEl.appendChild(self._createSpacer(0, true));
        self._appendKeys(numpadKeys, rowEl, isTopRow);
      }

      el.appendChild(rowEl);
    }
  },
  _heatColor(ratio) {
    if (ratio <= 0) return '#1e2235';
    const h = 260 - ratio * 260, s = 55 + ratio * 35, l = 18 + ratio * 30;
    return `hsl(${h},${s}%,${l}%)`;
  },
  _applyHeatmap(topKeys) {
    const self = pluginRegistry['keyboard-heatmap'];
    if (!topKeys || topKeys.length === 0) return;
    const counts = {};
    for (const k of topKeys) counts[k.key_name] = (counts[k.key_name] || 0) + k.count;
    if (counts['LShift'] || counts['RShift']) counts['Shift'] = (counts['Shift'] || 0) + (counts['LShift'] || 0) + (counts['RShift'] || 0);
    if (counts['LCtrl'] || counts['RCtrl']) counts['Ctrl'] = (counts['Ctrl'] || 0) + (counts['LCtrl'] || 0) + (counts['RCtrl'] || 0);
    if (counts['LAlt'] || counts['RAlt']) counts['Alt'] = (counts['Alt'] || 0) + (counts['LAlt'] || 0) + (counts['RAlt'] || 0);
    const maxCount = Math.max(...Object.values(counts), 1);
    document.querySelectorAll('.kb-key').forEach(el => {
      const name = el.dataset.key;
      const c = counts[name] || 0;
      const ratio = maxCount > 0 ? c / maxCount : 0;
      el.style.background = self._heatColor(ratio);
      el.querySelector('.kb-count').textContent = c > 0 ? c.toLocaleString() : '';
      const tt = el.querySelector('.kb-tooltip');
      if (tt) {
        const label = el.dataset.label || el.querySelector('.kb-label').textContent;
        tt.innerHTML = '<span class="tt-key">' + esc(label) + '</span> &mdash; <span class="tt-count">' + (c > 0 ? c.toLocaleString() : '0') + '</span> presses';
      }
    });
  },
  update(d) {
    const self = pluginRegistry['keyboard-heatmap'];
    self._lastTopKeys = d.top_keys;
    self._applyHeatmap(d.top_keys);
  }
});
