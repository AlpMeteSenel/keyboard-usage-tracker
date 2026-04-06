// ═══════════════════════════════════════════════
//  Plugin: WPM Timeline
//  Interactive SVG line chart of WPM over the last hour
// ═══════════════════════════════════════════════
registerPlugin({
  id: 'wpm-timeline',
  name: 'WPM Timeline',
  defaultWidth: 'full',
  defaultOrder: 30,
  render(slot) {
    slot.innerHTML = `<div class="chart-card">
      <div class="section-title"><span class="dot" style="background:var(--green)"></span> WPM Timeline (Last Hour)</div>
      <div id="wpm-chart"><div class="chart-empty">Collecting WPM data…</div></div>
    </div>`;
  },
  update(d) {
    const el = document.getElementById('wpm-chart');
    if (!el) return;
    const data = d.wpm_timeline;
    if (!data || data.length === 0) { el.innerHTML = '<div class="chart-empty">No WPM data in the last hour. Start typing!</div>'; return; }

    const W = 700, H = 160, pad = {t:20,r:20,b:30,l:45};
    const cw = W-pad.l-pad.r, ch = H-pad.t-pad.b;
    const maxWpm = Math.max(...data.map(d=>d.wpm), 10);
    const xStep = cw / Math.max(data.length - 1, 1);

    let pts = data.map((d,i) => [pad.l + i*xStep, pad.t + ch - (d.wpm/maxWpm)*ch]);
    let pathD = pts.map((p,i) => (i===0?'M':'L')+p[0]+','+p[1]).join(' ');
    let areaD = pathD + ` L${pts[pts.length-1][0]},${pad.t+ch} L${pts[0][0]},${pad.t+ch} Z`;

    let grid = '';
    for (let i=0;i<=4;i++) {
      const y = pad.t + (ch/4)*i;
      const v = Math.round(maxWpm - (maxWpm/4)*i);
      grid += `<line x1="${pad.l}" y1="${y}" x2="${W-pad.r}" y2="${y}" class="grid-line"/>`;
      grid += `<text x="${pad.l-8}" y="${y+4}" text-anchor="end" class="axis-label">${v}</text>`;
    }
    let xlabels = '';
    const step = Math.max(1, Math.floor(data.length / 8));
    for (let i=0;i<data.length;i+=step) {
      xlabels += `<text x="${pts[i][0]}" y="${H-4}" text-anchor="middle" class="axis-label">${esc(data[i].minute)}</text>`;
    }
    let dots = pts.map((p) => `<circle cx="${p[0]}" cy="${p[1]}" r="3" class="wpm-dot"/>`).join('');

    el.innerHTML = `<svg viewBox="0 0 ${W} ${H}" class="svg-chart">
      <defs><linearGradient id="wpmGrad" x1="0" y1="0" x2="0" y2="1"><stop offset="0%" stop-color="rgba(45,212,168,.3)"/><stop offset="100%" stop-color="rgba(45,212,168,0)"/></linearGradient></defs>
      ${grid}${xlabels}
      <path d="${areaD}" class="wpm-area"/>
      <path d="${pathD}" class="wpm-line"/>
      ${dots}
      <line class="wpm-crosshair" x1="0" y1="${pad.t}" x2="0" y2="${pad.t+ch}"/>
      <circle class="wpm-hover-dot" r="5"/>
      <rect class="wpm-hover-area" x="${pad.l}" y="${pad.t}" width="${cw}" height="${ch}"/>
    </svg>`;

    const svg = el.querySelector('svg');
    const crosshair = svg.querySelector('.wpm-crosshair');
    const hoverDot = svg.querySelector('.wpm-hover-dot');
    const tip = document.getElementById('wpm-tooltip');
    const hoverArea = svg.querySelector('.wpm-hover-area');

    hoverArea.addEventListener('mousemove', e => {
      const rect = svg.getBoundingClientRect();
      const scaleX = W / rect.width;
      const mouseX = (e.clientX - rect.left) * scaleX;
      let nearest = 0, minDist = Infinity;
      for (let i = 0; i < pts.length; i++) {
        const dd = Math.abs(pts[i][0] - mouseX);
        if (dd < minDist) { minDist = dd; nearest = i; }
      }
      const px = pts[nearest][0], py = pts[nearest][1];
      crosshair.setAttribute('x1', px); crosshair.setAttribute('x2', px); crosshair.style.opacity = '1';
      hoverDot.setAttribute('cx', px); hoverDot.setAttribute('cy', py); hoverDot.style.opacity = '1';
      tip.innerHTML = `<span class="tt-time">${esc(data[nearest].minute)}</span> &mdash; <span class="tt-val">${data[nearest].wpm.toFixed(1)} WPM</span>`;
      tip.style.display = 'block'; tip.style.left = e.clientX + 14 + 'px'; tip.style.top = e.clientY - 32 + 'px';
    });
    hoverArea.addEventListener('mouseleave', () => {
      crosshair.style.opacity = '0'; hoverDot.style.opacity = '0'; tip.style.display = 'none';
    });
  }
});
