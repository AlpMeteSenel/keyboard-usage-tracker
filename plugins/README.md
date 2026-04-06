# Dashboard Plugins

Each `.js` file in this folder is a self-contained dashboard plugin that gets automatically injected into the HTML at **compile time** by `build.rs`.

## Creating a Plugin

Create a new `.js` file in this folder (e.g. `my-widget.js`) and call `registerPlugin()`:

```js
registerPlugin({
  id: 'my-widget',           // Unique slug — must be unique across all plugins
  name: 'My Widget',         // Human-readable name shown in the settings panel
  defaultWidth: 'full',      // 'full' (spans entire row) or 'half' (side by side)
  defaultOrder: 90,          // Lower = higher in the default layout

  render(slot) {
    // Called once when the plugin is first placed on the page.
    // `slot` is the container <div> — set its innerHTML or append children.
    slot.innerHTML = `<div class="chart-card">
      <div class="section-title"><span class="dot" style="background:var(--accent)"></span> My Widget</div>
      <div id="my-widget-content">Loading…</div>
    </div>`;
  },

  init(slot) {
    // Optional — called once after render(). Bind event listeners here.
  },

  update(data) {
    // Called every ~3 seconds with the latest stats from /api/stats.
    // `data` contains: total_keys, total_clicks, keys_today, clicks_today,
    // current_wpm, avg_wpm, best_wpm, active_minutes_today,
    // top_keys[], held_keys[], recent[], hourly_activity[],
    // daily_activity[], weekly_activity[], monthly_activity[],
    // wpm_timeline[], period_stats{}
    const el = document.getElementById('my-widget-content');
    if (el) el.textContent = `Total keys: ${data.total_keys}`;
  }
});
```

Then rebuild: `cargo build` — the plugin is automatically picked up.

## Available Shared Utilities

These functions are available globally from the base template:

| Function | Description |
|---|---|
| `esc(str)` | HTML-escape a string |
| `fmtNum(n)` | Format number with locale separators |
| `animateValue(el, target, ms)` | Animate a counter element to a target value |
| `KEYBOARD_PROFILES` | Keyboard layout data (Turkish Q, US QWERTY) |
| `pluginRegistry` | Map of all registered plugins by id |
| `_statsToken` | Current CSRF token for API requests |

## Available CSS Classes

The base template includes all the CSS you need. Key classes:

- `.chart-card` — standard card container with border and padding
- `.section-title` — title with colored dot: `<span class="dot" style="background:var(--green)"></span>`
- `.stats-grid` — auto-fit grid for stat cards
- `.stat-card` — individual stat card with `.label`, `.value`, `.sub`
- `.bar-row`, `.bar-label`, `.bar-track`, `.bar-fill`, `.bar-count` — horizontal bar chart
- `.vbar-container`, `.vbar-wrapper`, `.vbar` — vertical bar chart
- `.chart-empty` — placeholder text for empty states
- Color classes: `.v-accent`, `.v-accent2`, `.v-green`, `.v-orange`, `.v-cyan`

## CSS Variables

```css
--accent:  #7c6aff   /* purple */
--accent2: #ff6b8a   /* pink */
--green:   #2dd4a8   /* teal */
--orange:  #ffb347   /* amber */
--cyan:    #22d3ee   /* cyan */
--card:    #181b28   /* card background */
--border:  #262a3d   /* border color */
--text:    #e4e7f1   /* primary text */
--muted:   #6b7094   /* secondary text */
```

## Included Plugins

| File | Description | Default Width |
|---|---|---|
| `stats-cards.js` | Key metrics overview (total keys, WPM, etc.) | full |
| `activity-breakdown.js` | Per-period stats (hour/day/week/month/all-time) | full |
| `wpm-timeline.js` | Interactive SVG line chart of WPM over the last hour | full |
| `activity-charts.js` | Bar charts with hourly/daily/weekly/monthly views | full |
| `keyboard-heatmap.js` | Visual keyboard colored by usage frequency | full |
| `finger-load.js` | Which fingers do the most work | half |
| `top-keys.js` | Horizontal bar chart of most-pressed keys | half |
| `key-combos.js` | Detected keyboard shortcuts (Ctrl+C, etc.) | half |
| `hold-duration.js` | Average & max hold times (table + histogram) | half |
| `typing-streaks.js` | Consecutive active hours and peak productivity | half |
| `recent-activity.js` | Last 50 key presses with timestamps | full |
| `click-heatmap.js` | Mouse click position heatmap (needs API endpoint) | full |
