// ═══════════════════════════════════════════════
//  Plugin: Typo Analyzer
//  Tracks Backspace/Delete usage to estimate typing accuracy
//  Shows correction rate, true WPM, and recent correction patterns
// ═══════════════════════════════════════════════
registerPlugin({
  id: 'typo-analyzer',
  name: 'Typo Analyzer',
  defaultWidth: 'half',
  defaultOrder: 76,

  render(slot) {
    slot.innerHTML = `<div class="chart-card">
      <div class="section-title"><span class="dot" style="background:var(--orange)"></span> Typo Analyzer</div>
      <div id="typo-analyzer-content"><div class="chart-empty">Analyzing typing accuracy…</div></div>
    </div>`;
  },

  update(d) {
    const el = document.getElementById('typo-analyzer-content');
    if (!el) return;

    // --- Gather correction key counts from top_keys ---
    const CORRECTION_KEYS = ['Backspace', 'Delete'];
    let totalCorrections = 0;
    let todayCorrections = 0;
    const keyMap = {};
    if (d.top_keys) {
      for (const k of d.top_keys) {
        keyMap[k.key_name] = k.count;
        if (CORRECTION_KEYS.includes(k.key_name)) {
          totalCorrections += k.count;
        }
      }
    }

    // --- All-time accuracy ---
    const totalKeys = d.total_keys || 0;
    // Characters "intended" = total keys minus correction keys, since each correction
    // means one keystroke wasted (the mistake) + one correction key.
    // Accuracy = 1 - (corrections / productive_keystrokes)
    const productiveKeys = Math.max(totalKeys - totalCorrections, 1);
    const accuracyPct = Math.max(0, ((1 - totalCorrections / productiveKeys) * 100)).toFixed(1);

    // --- True WPM (effective WPM adjusted for corrections) ---
    // Each correction "cancels" a character, so effective chars = raw chars - 2*corrections
    // (one for the wrong char, one for the correction key itself)
    const avgWpm = d.avg_wpm || 0;
    const correctionRatio = totalKeys > 0 ? totalCorrections / totalKeys : 0;
    const trueWpm = Math.max(0, avgWpm * (1 - 2 * correctionRatio)).toFixed(1);

    // --- Recent corrections (from last 50 events) ---
    let recentTotal = 0;
    let recentCorrections = 0;
    if (d.recent && d.recent.length > 0) {
      recentTotal = d.recent.length;
      for (const ev of d.recent) {
        if (CORRECTION_KEYS.includes(ev.key_name)) {
          recentCorrections++;
        }
      }
    }
    const recentPct = recentTotal > 0
      ? ((recentCorrections / recentTotal) * 100).toFixed(1)
      : '0.0';

    // --- Correction ratio label ---
    let accuracyLabel, accuracyColor;
    const acc = parseFloat(accuracyPct);
    if (acc >= 97) { accuracyLabel = 'Excellent'; accuracyColor = 'var(--green)'; }
    else if (acc >= 93) { accuracyLabel = 'Good'; accuracyColor = 'var(--cyan)'; }
    else if (acc >= 88) { accuracyLabel = 'Average'; accuracyColor = 'var(--orange)'; }
    else { accuracyLabel = 'Needs work'; accuracyColor = 'var(--accent)'; }

    // --- Build visual bar for recent 50 events ---
    let recentBar = '';
    if (d.recent && d.recent.length > 0) {
      // Show a mini bar of the last 50 events: colored dots
      const dots = d.recent.slice().reverse().map(ev =>
        CORRECTION_KEYS.includes(ev.key_name)
          ? '<span style="display:inline-block;width:6px;height:6px;border-radius:50%;background:var(--accent);margin:1px"></span>'
          : '<span style="display:inline-block;width:6px;height:6px;border-radius:50%;background:var(--green);margin:1px;opacity:.3"></span>'
      ).join('');
      recentBar = `<div style="margin-top:10px;padding:10px;background:rgba(255,255,255,.03);border-radius:8px">
        <div style="font-size:.75rem;color:var(--muted);margin-bottom:6px">Last ${recentTotal} keystrokes <span style="opacity:.6">(red = correction)</span></div>
        <div style="line-height:10px">${dots}</div>
      </div>`;
    }

    let html = '<div class="stats-grid" style="grid-template-columns:repeat(2,1fr)">';
    html += `<div class="stat-card"><div class="label">Accuracy</div><div class="value" style="color:${accuracyColor}">${accuracyPct}%</div><div class="sub">${accuracyLabel}</div></div>`;
    html += `<div class="stat-card"><div class="label">True WPM</div><div class="value v-cyan">${trueWpm}</div><div class="sub">Adjusted for corrections</div></div>`;
    html += `<div class="stat-card"><div class="label">Total Corrections</div><div class="value v-orange">${fmtNum(totalCorrections)}</div><div class="sub">Backspace + Delete</div></div>`;
    html += `<div class="stat-card"><div class="label">Recent Correction Rate</div><div class="value v-accent">${recentPct}%</div><div class="sub">Last ${recentTotal} keystrokes</div></div>`;
    html += '</div>';
    html += recentBar;

    el.innerHTML = html;
  }
});
