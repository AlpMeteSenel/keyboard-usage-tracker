// ═══════════════════════════════════════════════
//  Plugin: Finger Load
//  Shows which fingers are doing the most work based on key positions
//  Uses the active keyboard profile layout to map keys to fingers
// ═══════════════════════════════════════════════
registerPlugin({
  id: 'finger-load',
  name: 'Finger Load',
  defaultWidth: 'half',
  defaultOrder: 63,
  render(slot) {
    slot.innerHTML = `<div class="chart-card">
      <div class="section-title"><span class="dot" style="background:var(--cyan)"></span> Finger Load Distribution</div>
      <div id="finger-load-chart"><div class="chart-empty">Waiting for data…</div></div>
    </div>`;
  },
  update(d) {
    const el = document.getElementById('finger-load-chart');
    if (!el || !d.top_keys || d.top_keys.length === 0) return;

    // Map keys → fingers based on standard keyboard rows
    // Row 2 (number row): pinky-4, ring-3, mid-2, index-1, index-1, index-1, index-1, mid-2, ring-3, pinky-4, pinky-4, pinky-4, pinky-4, pinky-4
    // Row 3 (QWER): Q=Lpinky, W=Lring, E=Lmid, R=Lindex, T=Lindex, Y=Rindex, U=Rindex, I=Rmid, O=Rring, P=Rpinky, ...
    // Row 4 (ASDF): A=Lpinky, S=Lring, D=Lmid, F=Lindex, G=Lindex, H=Rindex, J=Rindex, K=Rmid, L=Rring, ...
    // Row 5 (ZXCV): Z=Lpinky, X=Lring, C=Lmid, V=Lindex, B=Lindex, N=Rindex, M=Rindex, ...
    const fingerMap = {
      // Left pinky
      '`': 0, '1': 0, 'Q': 0, 'A': 0, 'Z': 0, 'Tab': 0, 'CapsLock': 0, 'Caps': 0, 'LShift': 0, 'LCtrl': 0, 'Escape': 0, 'Esc': 0,
      '"': 0, '<>': 0, '<': 0,
      // Left ring
      '2': 1, 'W': 1, 'S': 1, 'X': 1,
      // Left middle
      '3': 2, 'E': 2, 'D': 2, 'C': 2,
      // Left index
      '4': 3, '5': 3, 'R': 3, 'T': 3, 'F': 3, 'G': 3, 'V': 3, 'B': 3,
      // Left thumb
      'Space': 4, 'LAlt': 4, 'Win': 4,
      // Right thumb
      'RAlt': 5, 'AltGr': 5,
      // Right index
      '6': 6, '7': 6, 'Y': 6, 'U': 6, 'H': 6, 'J': 6, 'N': 6, 'M': 6,
      // Right middle
      '8': 7, 'I': 7, 'K': 7, ',': 7, 'Ö': 7,
      // Right ring
      '9': 8, 'O': 8, 'L': 8, '.': 8, 'Ç': 8,
      // Right pinky
      '0': 9, '-': 9, '=': 9, '*': 9, 'P': 9, ';': 9, 'Ş': 9, '/': 9,
      '[': 9, 'Ğ': 9, ']': 9, 'Ü': 9, '\\': 9, "'": 9, 'İ': 9,
      'Backspace': 9, 'Enter': 9, 'RShift': 9,
    };

    const fingerNames = ['L Pinky', 'L Ring', 'L Mid', 'L Index', 'L Thumb', 'R Thumb', 'R Index', 'R Mid', 'R Ring', 'R Pinky'];
    const fingerColors = [
      'var(--accent2)', 'var(--orange)', 'var(--green)', 'var(--accent)', 'var(--cyan)',
      'var(--cyan)', 'var(--accent)', 'var(--green)', 'var(--orange)', 'var(--accent2)'
    ];
    const fingerCounts = new Array(10).fill(0);
    let totalMapped = 0;

    for (const k of d.top_keys) {
      const finger = fingerMap[k.key_name];
      if (finger !== undefined) {
        fingerCounts[finger] += k.count;
        totalMapped += k.count;
      }
    }

    if (totalMapped === 0) { el.innerHTML = '<div class="chart-empty">Not enough data yet.</div>'; return; }

    const maxCount = Math.max(...fingerCounts, 1);
    let html = '<div class="finger-chart">';
    for (let i = 0; i < 10; i++) {
      const pct = (fingerCounts[i] / maxCount) * 100;
      const usePct = ((fingerCounts[i] / totalMapped) * 100).toFixed(1);
      html += `<div class="finger-bar-wrap">
        <div class="finger-bar" style="height:${Math.max(pct, 3)}%;background:${fingerColors[i]}">
          <span class="finger-bar-tooltip">${fingerNames[i]}: ${fingerCounts[i].toLocaleString()} (${usePct}%)</span>
        </div>
        <div class="finger-bar-label">${fingerNames[i].replace('L ','').replace('R ','')}</div>
        <div class="finger-bar-pct">${usePct}%</div>
      </div>`;
    }
    html += '</div>';

    const leftTotal = fingerCounts.slice(0,5).reduce((a,b)=>a+b,0);
    const rightTotal = fingerCounts.slice(5).reduce((a,b)=>a+b,0);
    html += `<div class="finger-hands">
      <span>Left hand: <strong style="color:var(--accent)">${((leftTotal/totalMapped)*100).toFixed(1)}%</strong></span>
      <span>Right hand: <strong style="color:var(--accent2)">${((rightTotal/totalMapped)*100).toFixed(1)}%</strong></span>
    </div>`;

    el.innerHTML = html;
  }
});
