// ═══════════════════════════════════════════════
//  Plugin: Activity Charts
//  Vertical bar charts: hourly / daily / weekly / monthly views
// ═══════════════════════════════════════════════
registerPlugin({
  id: 'activity-charts',
  name: 'Activity Charts',
  defaultWidth: 'full',
  defaultOrder: 40,
  _view: 'hourly',
  render(slot) {
    slot.innerHTML = `<div class="chart-card">
      <div class="section-header">
        <div class="section-title"><span class="dot" id="activity-dot" style="background:var(--accent)"></span> <span id="activity-chart-title">Today's Keystrokes by Hour</span></div>
        <div class="toggle-group">
          <button class="view-toggle active" id="chart-hourly-btn" title="Today by Hour">&#128339;</button>
          <button class="view-toggle" id="chart-daily-btn" title="Last 30 Days">&#128197;</button>
          <button class="view-toggle" id="chart-weekly-btn" title="Last 12 Weeks">&#128200;</button>
          <button class="view-toggle" id="chart-monthly-btn" title="Last 12 Months">&#128198;</button>
        </div>
      </div>
      <div id="hourly-chart"><div class="chart-empty">No activity yet today. Start typing!</div></div>
      <div id="daily-chart" style="display:none"><div class="chart-empty">No daily data yet.</div></div>
      <div id="weekly-chart" style="display:none"><div class="chart-empty">No weekly data yet.</div></div>
      <div id="monthly-chart" style="display:none"><div class="chart-empty">No monthly data yet.</div></div>
    </div>`;
  },
  init() {
    const self = pluginRegistry['activity-charts'];
    const titles = { hourly:"Today's Keystrokes by Hour", daily:'Daily Keystrokes — Last 30 Days', weekly:'Weekly Keystrokes — Last 12 Weeks', monthly:'Monthly Keystrokes — Last 12 Months' };
    const colors = { hourly:'var(--accent)', daily:'var(--cyan)', weekly:'var(--orange)', monthly:'var(--accent2)' };
    function setView(view) {
      self._view = view;
      ['hourly','daily','weekly','monthly'].forEach(v => {
        const cel = document.getElementById(v + '-chart');
        if (cel) cel.style.display = v === view ? '' : 'none';
        const btn = document.getElementById('chart-' + v + '-btn');
        if (btn) btn.classList.toggle('active', v === view);
      });
      const title = document.getElementById('activity-chart-title');
      if (title) title.textContent = titles[view];
      const dot = document.getElementById('activity-dot');
      if (dot) dot.style.background = colors[view];
    }
    ['hourly','daily','weekly','monthly'].forEach(v => {
      const btn = document.getElementById('chart-' + v + '-btn');
      if (btn) btn.addEventListener('click', () => setView(v));
    });
  },
  update(d) {
    // Hourly
    const hEl = document.getElementById('hourly-chart');
    if (hEl) {
      const data = d.hourly_activity;
      if (!data || data.length === 0) { hEl.innerHTML = '<div class="chart-empty">No activity today yet. Start typing!</div>'; }
      else {
        const maxC = Math.max(...data.map(x=>x.count), 1);
        const hourMap = {}; for (const x of data) hourMap[x.hour] = x.count;
        let html = '<div class="vbar-container">';
        for (let h=0;h<24;h++) { const c=hourMap[h]||0; const pct=(c/maxC)*100; const color=c>0?'var(--accent)':'rgba(124,106,255,.1)'; html+=`<div class="vbar-wrapper"><div class="vbar" style="height:${Math.max(pct,2)}%;background:${color}"><span class="vbar-tooltip">${h}:00 — ${c.toLocaleString()}</span></div><div class="vbar-label">${h}</div></div>`; }
        html += '</div>'; hEl.innerHTML = html;
      }
    }
    // Daily
    const dEl = document.getElementById('daily-chart');
    if (dEl) {
      const dataMap = {}; if (d.daily_activity) for (const x of d.daily_activity) dataMap[x.date] = x.count;
      const days = []; const today = new Date();
      for (let i=29;i>=0;i--) { const dt=new Date(today); dt.setDate(dt.getDate()-i); const key=dt.toISOString().slice(0,10); days.push({date:key,count:dataMap[key]||0}); }
      const maxC=Math.max(...days.map(x=>x.count),1);
      let html='<div class="vbar-container">';
      for (const x of days) { const pct=(x.count/maxC)*100; const label=x.date.slice(5); const color=x.count>0?'var(--cyan)':'rgba(34,211,238,.1)'; html+=`<div class="vbar-wrapper"><div class="vbar" style="height:${Math.max(pct,2)}%;background:${color}"><span class="vbar-tooltip">${x.date} — ${x.count.toLocaleString()}</span></div><div class="vbar-label">${label}</div></div>`; }
      html+='</div>'; dEl.innerHTML=html;
    }
    // Weekly
    const wEl = document.getElementById('weekly-chart');
    if (wEl) {
      const dataMap={}; if (d.weekly_activity) for (const x of d.weekly_activity) dataMap[x.week]=x.count;
      const weeks=[]; const today=new Date();
      for (let i=11;i>=0;i--) { const dt=new Date(today); dt.setDate(dt.getDate()-i*7); const yr=dt.getFullYear(); const jan1=new Date(yr,0,1); const doy=Math.floor((dt-jan1)/86400000); const wn=Math.floor((doy+jan1.getDay())/7); const key=yr+'-W'+String(wn).padStart(2,'0'); if (!weeks.find(w=>w.week===key)) weeks.push({week:key,count:dataMap[key]||0}); }
      const maxC=Math.max(...weeks.map(x=>x.count),1);
      let html='<div class="vbar-container">';
      for (const x of weeks) { const pct=(x.count/maxC)*100; const label=x.week.replace(/^\d{4}-/,''); const color=x.count>0?'var(--orange)':'rgba(255,179,71,.1)'; html+=`<div class="vbar-wrapper"><div class="vbar" style="height:${Math.max(pct,2)}%;background:${color}"><span class="vbar-tooltip">${x.week} — ${x.count.toLocaleString()}</span></div><div class="vbar-label">${label}</div></div>`; }
      html+='</div>'; wEl.innerHTML=html;
    }
    // Monthly
    const mEl = document.getElementById('monthly-chart');
    if (mEl) {
      const dataMap={}; if (d.monthly_activity) for (const x of d.monthly_activity) dataMap[x.month]=x.count;
      const monthNames=['Jan','Feb','Mar','Apr','May','Jun','Jul','Aug','Sep','Oct','Nov','Dec'];
      const months=[]; const today=new Date();
      for (let i=11;i>=0;i--) { const dt=new Date(today.getFullYear(),today.getMonth()-i,1); const key=dt.getFullYear()+'-'+String(dt.getMonth()+1).padStart(2,'0'); months.push({month:key,count:dataMap[key]||0}); }
      const maxC=Math.max(...months.map(x=>x.count),1);
      let html='<div class="vbar-container">';
      for (const x of months) { const pct=(x.count/maxC)*100; const mIdx=parseInt(x.month.slice(5))-1; const label=monthNames[mIdx]||x.month.slice(5); const color=x.count>0?'var(--accent2)':'rgba(255,107,138,.1)'; html+=`<div class="vbar-wrapper"><div class="vbar" style="height:${Math.max(pct,2)}%;background:${color}"><span class="vbar-tooltip">${x.month} — ${x.count.toLocaleString()}</span></div><div class="vbar-label">${label}</div></div>`; }
      html+='</div>'; mEl.innerHTML=html;
    }
  }
});
