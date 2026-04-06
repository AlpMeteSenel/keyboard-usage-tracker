// ═══════════════════════════════════════════════
//  Plugin: Typing Streaks
//  Analyzes hourly activity to find peak typing hours and streaks
//  Shows consecutive active hours and best productivity windows
// ═══════════════════════════════════════════════
registerPlugin({
  id: 'typing-streaks',
  name: 'Typing Streaks',
  defaultWidth: 'half',
  defaultOrder: 75,
  render(slot) {
    slot.innerHTML = `<div class="chart-card">
      <div class="section-title"><span class="dot" style="background:var(--green)"></span> Typing Streaks</div>
      <div id="typing-streaks"><div class="chart-empty">Analyzing your typing patterns…</div></div>
    </div>`;
  },
  update(d) {
    const el = document.getElementById('typing-streaks');
    if (!el) return;

    const hourly = d.hourly_activity;
    if (!hourly || hourly.length === 0) {
      el.innerHTML = '<div class="chart-empty">No activity today to analyze streaks.</div>';
      return;
    }

    // Build full 24-hour map
    const hourMap = {};
    for (const h of hourly) hourMap[h.hour] = h.count;

    // Find the best consecutive streak
    let bestStreak = 0, bestStart = 0, curStreak = 0, curStart = 0;
    let bestStreakKeys = 0, curStreakKeys = 0;
    for (let h = 0; h < 24; h++) {
      if (hourMap[h] && hourMap[h] > 0) {
        if (curStreak === 0) curStart = h;
        curStreak++;
        curStreakKeys += hourMap[h];
        if (curStreak > bestStreak) {
          bestStreak = curStreak;
          bestStart = curStart;
          bestStreakKeys = curStreakKeys;
        }
      } else {
        curStreak = 0;
        curStreakKeys = 0;
      }
    }

    // Find peak hour
    let peakHour = 0, peakCount = 0;
    for (let h = 0; h < 24; h++) {
      if ((hourMap[h] || 0) > peakCount) {
        peakCount = hourMap[h];
        peakHour = h;
      }
    }

    // Total active hours
    let activeHours = 0, totalKeys = 0;
    for (let h = 0; h < 24; h++) {
      if (hourMap[h] && hourMap[h] > 0) {
        activeHours++;
        totalKeys += hourMap[h];
      }
    }

    // Average keys per active hour
    const avgPerHour = activeHours > 0 ? Math.round(totalKeys / activeHours) : 0;

    let html = '<div class="stats-grid" style="grid-template-columns:repeat(2,1fr)">';
    html += `<div class="stat-card"><div class="label">Best Streak</div><div class="value v-green">${bestStreak}h</div><div class="sub">${bestStart}:00–${bestStart + bestStreak}:00 · ${fmtNum(bestStreakKeys)} keys</div></div>`;
    html += `<div class="stat-card"><div class="label">Peak Hour</div><div class="value v-accent">${peakHour}:00</div><div class="sub">${fmtNum(peakCount)} keystrokes</div></div>`;
    html += `<div class="stat-card"><div class="label">Active Hours</div><div class="value v-cyan">${activeHours}</div><div class="sub">out of 24 today</div></div>`;
    html += `<div class="stat-card"><div class="label">Avg / Active Hour</div><div class="value v-orange">${fmtNum(avgPerHour)}</div><div class="sub">keys per active hour</div></div>`;
    html += '</div>';

    el.innerHTML = html;
  }
});
