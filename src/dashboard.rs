use std::path::PathBuf;
use std::sync::{Arc, RwLock};
use std::thread;
use std::time::Duration;

use rusqlite::Connection;
use serde::Serialize;

// ---------------------------------------------------------------------------
// Stats data types
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct DashboardStats {
    total_keys: u64,
    total_clicks: u64,
    keys_today: u64,
    clicks_today: u64,
    current_wpm: f64,
    avg_wpm: f64,
    best_wpm: f64,
    active_minutes_today: u64,
    top_keys: Vec<KeyCount>,
    held_keys: Vec<HeldKeyStat>,
    recent: Vec<RecentEvent>,
    hourly_activity: Vec<HourlyActivity>,
    daily_activity: Vec<DailyActivity>,
    weekly_activity: Vec<WeeklyActivity>,
    monthly_activity: Vec<MonthlyActivity>,
    wpm_timeline: Vec<WpmPoint>,
    period_stats: PeriodStats,
    key_combos: Vec<ComboCount>,
}

#[derive(Serialize)]
struct KeyCount {
    key_name: String,
    count: u64,
}

#[derive(Serialize)]
struct HeldKeyStat {
    key_name: String,
    avg_hold_ms: f64,
    max_hold_ms: u64,
    total_holds: u64,
}

#[derive(Serialize)]
struct RecentEvent {
    timestamp: String,
    key_name: String,
    hold_ms: Option<u64>,
}

#[derive(Serialize)]
struct HourlyActivity {
    hour: u32,
    count: u64,
}

#[derive(Serialize)]
struct DailyActivity {
    date: String,
    count: u64,
}

#[derive(Serialize)]
struct WeeklyActivity {
    week: String,
    count: u64,
}

#[derive(Serialize)]
struct MonthlyActivity {
    month: String,
    count: u64,
}

#[derive(Serialize)]
struct WpmPoint {
    minute: String,
    wpm: f64,
}

#[derive(Serialize)]
pub struct PeriodStat {
    keys: u64,
    clicks: u64,
    wpm: f64,
    active_minutes: u64,
}

#[derive(Serialize)]
struct PeriodStats {
    this_hour: PeriodStat,
    today: PeriodStat,
    this_week: PeriodStat,
    this_month: PeriodStat,
    all_time: PeriodStat,
}

#[derive(Serialize)]
struct ComboCount {
    combo: String,
    count: u64,
}

#[derive(Serialize)]
struct ClickPosition {
    x: i32,
    y: i32,
    count: u64,
}

// ---------------------------------------------------------------------------
// Stats queries
// ---------------------------------------------------------------------------

fn default_stats() -> DashboardStats {
    DashboardStats {
        total_keys: 0,
        total_clicks: 0,
        keys_today: 0,
        clicks_today: 0,
        current_wpm: 0.0,
        avg_wpm: 0.0,
        best_wpm: 0.0,
        active_minutes_today: 0,
        top_keys: vec![],
        held_keys: vec![],
        recent: vec![],
        hourly_activity: vec![],
        daily_activity: vec![],
        weekly_activity: vec![],
        monthly_activity: vec![],
        wpm_timeline: vec![],
        period_stats: PeriodStats {
            this_hour: PeriodStat {
                keys: 0,
                clicks: 0,
                wpm: 0.0,
                active_minutes: 0,
            },
            today: PeriodStat {
                keys: 0,
                clicks: 0,
                wpm: 0.0,
                active_minutes: 0,
            },
            this_week: PeriodStat {
                keys: 0,
                clicks: 0,
                wpm: 0.0,
                active_minutes: 0,
            },
            this_month: PeriodStat {
                keys: 0,
                clicks: 0,
                wpm: 0.0,
                active_minutes: 0,
            },
            all_time: PeriodStat {
                keys: 0,
                clicks: 0,
                wpm: 0.0,
                active_minutes: 0,
            },
        },
        key_combos: vec![],
    }
}

fn query_stats(db_path: &PathBuf) -> DashboardStats {
    let conn = match Connection::open(db_path) {
        Ok(c) => c,
        Err(_) => return default_stats(),
    };

    let total_keys: u64 = conn
        .query_row("SELECT COUNT(*) FROM key_events", [], |r| r.get(0))
        .unwrap_or(0);
    let total_clicks: u64 = conn
        .query_row(
            "SELECT COUNT(*) FROM mouse_events WHERE event_type = 'click'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let keys_today: u64 = conn
        .query_row(
            "SELECT COUNT(*) FROM key_events \
             WHERE timestamp >= strftime('%Y-%m-%dT00:00:00','now','localtime')",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let clicks_today: u64 = conn
        .query_row(
            "SELECT COUNT(*) FROM mouse_events \
             WHERE event_type='click' \
             AND timestamp >= strftime('%Y-%m-%dT00:00:00','now','localtime')",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let current_wpm: f64 = conn
        .query_row(
            "SELECT COUNT(*) FROM key_events \
             WHERE timestamp >= strftime('%Y-%m-%dT%H:%M:%S','now','localtime','-60 seconds') \
             AND key_name NOT IN ('Shift','Ctrl','Alt','CapsLock','Win','LShift','RShift','LCtrl','RCtrl','LAlt','RAlt')",
            [],
            |r| Ok(r.get::<_, u64>(0)? as f64 / 5.0),
        )
        .unwrap_or(0.0);

    let avg_wpm: f64 = conn
        .query_row(
            "SELECT COALESCE(AVG(cnt),0) FROM ( \
               SELECT COUNT(*)/5.0 AS cnt FROM key_events \
               WHERE key_name NOT IN ('Shift','Ctrl','Alt','CapsLock','Win','LShift','RShift','LCtrl','RCtrl','LAlt','RAlt') \
               GROUP BY strftime('%Y-%m-%d %H:%M',timestamp) \
               HAVING COUNT(*)>=5 \
             )",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0.0);

    let best_wpm: f64 = conn
        .query_row(
            "SELECT COALESCE(MAX(cnt),0) FROM ( \
               SELECT COUNT(*)/5.0 AS cnt FROM key_events \
               WHERE key_name NOT IN ('Shift','Ctrl','Alt','CapsLock','Win','LShift','RShift','LCtrl','RCtrl','LAlt','RAlt') \
               GROUP BY strftime('%Y-%m-%d %H:%M',timestamp) \
               HAVING COUNT(*)>=10 \
             )",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0.0);

    let active_minutes_today: u64 = conn
        .query_row(
            "SELECT COUNT(DISTINCT strftime('%Y-%m-%d %H:%M',timestamp)) FROM key_events \
             WHERE timestamp >= strftime('%Y-%m-%dT00:00:00','now','localtime')",
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);

    let top_keys: Vec<KeyCount> = if let Ok(mut stmt) = conn.prepare(
        "SELECT key_name, COUNT(*) AS cnt FROM key_events \
             GROUP BY key_name ORDER BY cnt DESC",
    ) {
        stmt.query_map([], |row| {
            Ok(KeyCount {
                key_name: row.get(0)?,
                count: row.get(1)?,
            })
        })
        .ok()
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default()
    } else {
        vec![]
    };

    let held_keys: Vec<HeldKeyStat> = if let Ok(mut stmt) = conn.prepare(
        "SELECT key_name, AVG(hold_ms), MAX(hold_ms), COUNT(*) \
             FROM key_events WHERE hold_ms IS NOT NULL AND hold_ms > 0 \
             GROUP BY key_name ORDER BY AVG(hold_ms) DESC LIMIT 20",
    ) {
        stmt.query_map([], |row| {
            Ok(HeldKeyStat {
                key_name: row.get(0)?,
                avg_hold_ms: row.get(1)?,
                max_hold_ms: row.get(2)?,
                total_holds: row.get(3)?,
            })
        })
        .ok()
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default()
    } else {
        vec![]
    };

    let recent: Vec<RecentEvent> = if let Ok(mut stmt) = conn
        .prepare("SELECT timestamp, key_name, hold_ms FROM key_events ORDER BY id DESC LIMIT 50")
    {
        stmt.query_map([], |row| {
            Ok(RecentEvent {
                timestamp: row.get(0)?,
                key_name: row.get(1)?,
                hold_ms: row.get(2)?,
            })
        })
        .ok()
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default()
    } else {
        vec![]
    };

    let hourly_activity: Vec<HourlyActivity> = if let Ok(mut stmt) = conn.prepare(
        "SELECT CAST(strftime('%H',timestamp) AS INTEGER), COUNT(*) \
             FROM key_events \
             WHERE timestamp >= strftime('%Y-%m-%dT00:00:00','now','localtime') \
             GROUP BY strftime('%H',timestamp) ORDER BY 1",
    ) {
        stmt.query_map([], |row| {
            Ok(HourlyActivity {
                hour: row.get(0)?,
                count: row.get(1)?,
            })
        })
        .ok()
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default()
    } else {
        vec![]
    };

    let daily_activity: Vec<DailyActivity> = if let Ok(mut stmt) = conn.prepare(
        "SELECT substr(timestamp,1,10), COUNT(*) \
             FROM key_events \
             WHERE timestamp >= strftime('%Y-%m-%dT00:00:00','now','localtime','-30 days') \
             GROUP BY substr(timestamp,1,10) ORDER BY 1",
    ) {
        stmt.query_map([], |row| {
            Ok(DailyActivity {
                date: row.get(0)?,
                count: row.get(1)?,
            })
        })
        .ok()
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default()
    } else {
        vec![]
    };

    let weekly_activity: Vec<WeeklyActivity> = if let Ok(mut stmt) = conn.prepare(
        "SELECT strftime('%Y-W%W',timestamp), COUNT(*) \
             FROM key_events \
             WHERE timestamp >= strftime('%Y-%m-%dT00:00:00','now','localtime','-84 days') \
             GROUP BY strftime('%Y-W%W',timestamp) ORDER BY 1",
    ) {
        stmt.query_map([], |row| {
            Ok(WeeklyActivity {
                week: row.get(0)?,
                count: row.get(1)?,
            })
        })
        .ok()
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default()
    } else {
        vec![]
    };

    let monthly_activity: Vec<MonthlyActivity> = if let Ok(mut stmt) = conn.prepare(
        "SELECT strftime('%Y-%m',timestamp), COUNT(*) \
             FROM key_events \
             WHERE timestamp >= strftime('%Y-%m-%dT00:00:00','now','localtime','-365 days') \
             GROUP BY strftime('%Y-%m',timestamp) ORDER BY 1",
    ) {
        stmt.query_map([], |row| {
            Ok(MonthlyActivity {
                month: row.get(0)?,
                count: row.get(1)?,
            })
        })
        .ok()
        .map(|rows| rows.filter_map(|r| r.ok()).collect())
        .unwrap_or_default()
    } else {
        vec![]
    };

    let wpm_timeline: Vec<WpmPoint> = if let Ok(mut stmt) = conn
        .prepare(
            "SELECT strftime('%H:%M',timestamp), COUNT(*)/5.0 \
             FROM key_events \
             WHERE timestamp >= strftime('%Y-%m-%dT%H:%M:%S','now','localtime','-60 minutes') \
             AND key_name NOT IN ('Shift','Ctrl','Alt','CapsLock','Win','LShift','RShift','LCtrl','RCtrl','LAlt','RAlt') \
             GROUP BY strftime('%H:%M',timestamp) ORDER BY 1",
        ) {
        stmt.query_map([], |row| {
            Ok(WpmPoint {
                minute: row.get(0)?,
                wpm: row.get(1)?,
            })
        }).ok().map(|rows| rows.filter_map(|r| r.ok()).collect()).unwrap_or_default()
    } else { vec![] };

    let period_stat = |time_filter: &str| -> PeriodStat {
        let keys: u64 = conn
            .query_row(
                &format!("SELECT COUNT(*) FROM key_events WHERE {time_filter}"),
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let clicks: u64 = conn
            .query_row(
                &format!(
                    "SELECT COUNT(*) FROM mouse_events WHERE event_type='click' AND {time_filter}"
                ),
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let wpm: f64 = conn
            .query_row(
                &format!(
                    "SELECT COALESCE(AVG(cnt),0) FROM ( \
                       SELECT COUNT(*)/5.0 AS cnt FROM key_events \
                       WHERE {time_filter} \
                       AND key_name NOT IN ('Shift','Ctrl','Alt','CapsLock','Win','LShift','RShift','LCtrl','RCtrl','LAlt','RAlt') \
                       GROUP BY strftime('%Y-%m-%d %H:%M',timestamp) \
                       HAVING COUNT(*)>=5 \
                     )"
                ),
                [],
                |r| r.get(0),
            )
            .unwrap_or(0.0);
        let active_minutes: u64 = conn
            .query_row(
                &format!(
                    "SELECT COUNT(DISTINCT strftime('%Y-%m-%d %H:%M',timestamp)) FROM key_events WHERE {time_filter}"
                ),
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        PeriodStat {
            keys,
            clicks,
            wpm,
            active_minutes,
        }
    };

    let period_stats = PeriodStats {
        this_hour: period_stat("timestamp >= strftime('%Y-%m-%dT%H:00:00','now','localtime')"),
        today: period_stat("timestamp >= strftime('%Y-%m-%dT00:00:00','now','localtime')"),
        this_week: period_stat(
            "timestamp >= strftime('%Y-%m-%dT00:00:00','now','localtime','weekday 0','-7 days')",
        ),
        this_month: period_stat("timestamp >= strftime('%Y-%m-01T00:00:00','now','localtime')"),
        all_time: period_stat("1=1"),
    };

    let key_combos = query_key_combos(&conn);

    DashboardStats {
        total_keys,
        total_clicks,
        keys_today,
        clicks_today,
        current_wpm,
        avg_wpm,
        best_wpm,
        active_minutes_today,
        top_keys,
        held_keys,
        recent,
        hourly_activity,
        daily_activity,
        weekly_activity,
        monthly_activity,
        wpm_timeline,
        period_stats,
        key_combos,
    }
}

fn query_key_combos(conn: &Connection) -> Vec<ComboCount> {
    use std::collections::HashMap;

    const MODIFIERS: &[&str] = &["LCtrl", "RCtrl", "LShift", "RShift", "LAlt", "RAlt", "Win"];

    let mut combos: HashMap<String, u64> = HashMap::new();

    // Only scan the last 100k events to keep this query fast on large databases
    let mut stmt = match conn.prepare(
        "SELECT key_name, \
         CAST((julianday(timestamp) - 2440587.5) * 86400000 AS INTEGER) AS ts_ms \
         FROM key_events WHERE id > (SELECT MAX(id) - 100000 FROM key_events) ORDER BY id",
    ) {
        Ok(s) => s,
        Err(_) => return vec![],
    };

    struct ModEntry {
        key: String,
        ts_ms: i64,
        used: bool,
    }
    let mut recent_mods: Vec<ModEntry> = Vec::new();

    let mut rows = match stmt.query([]) {
        Ok(r) => r,
        Err(_) => return vec![],
    };

    while let Ok(Some(row)) = rows.next() {
        let key_name: String = match row.get(0) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let ts_ms: i64 = match row.get(1) {
            Ok(v) => v,
            Err(_) => continue,
        };

        if MODIFIERS.contains(&key_name.as_str()) {
            recent_mods.push(ModEntry {
                key: key_name,
                ts_ms,
                used: false,
            });
            if recent_mods.len() > 4 {
                recent_mods.remove(0);
            }
        } else {
            for m in recent_mods.iter_mut().rev() {
                if m.used {
                    continue;
                }
                let diff = ts_ms - m.ts_ms;
                if diff < 0 || diff > 200 {
                    continue;
                }
                let label = match m.key.as_str() {
                    "LCtrl" | "RCtrl" => "Ctrl",
                    "LShift" | "RShift" => "Shift",
                    "LAlt" | "RAlt" => "Alt",
                    "Win" => "Win",
                    _ => continue,
                };
                *combos.entry(format!("{}+{}", label, key_name)).or_insert(0) += 1;
                m.used = true;
                break;
            }
        }
    }

    let mut sorted: Vec<_> = combos
        .into_iter()
        .map(|(combo, count)| ComboCount { combo, count })
        .collect();
    sorted.sort_by(|a, b| b.count.cmp(&a.count));
    sorted.truncate(15);
    sorted
}

fn query_click_positions(db_path: &PathBuf) -> Vec<ClickPosition> {
    let conn = match Connection::open(db_path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    // Bucket clicks into 10x10 pixel grid cells to aggregate nearby clicks
    let mut stmt = match conn.prepare(
        "SELECT (x / 10) * 10 AS bx, (y / 10) * 10 AS by, COUNT(*) AS cnt \
         FROM mouse_events WHERE event_type = 'click' \
         GROUP BY bx, by ORDER BY cnt DESC LIMIT 5000",
    ) {
        Ok(s) => s,
        Err(_) => return vec![],
    };
    stmt.query_map([], |row| {
        Ok(ClickPosition {
            x: row.get(0)?,
            y: row.get(1)?,
            count: row.get(2)?,
        })
    })
    .ok()
    .map(|rows| rows.filter_map(|r| r.ok()).collect())
    .unwrap_or_default()
}

fn query_hour_stats(db_path: &PathBuf, offset: u32) -> PeriodStat {
    let offset = offset.min(168);
    let conn = match Connection::open(db_path) {
        Ok(c) => c,
        Err(_) => {
            return PeriodStat {
                keys: 0,
                clicks: 0,
                wpm: 0.0,
                active_minutes: 0,
            }
        }
    };
    let time_filter = if offset == 0 {
        "timestamp >= strftime('%Y-%m-%dT%H:00:00','now','localtime')".to_string()
    } else {
        format!(
            "timestamp >= strftime('%Y-%m-%dT%H:00:00','now','localtime','-{offset} hours') \
             AND timestamp < strftime('%Y-%m-%dT%H:00:00','now','localtime','-{} hours')",
            offset - 1
        )
    };
    let keys: u64 = conn
        .query_row(
            &format!("SELECT COUNT(*) FROM key_events WHERE {time_filter}"),
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let clicks: u64 = conn
        .query_row(
            &format!(
                "SELECT COUNT(*) FROM mouse_events WHERE event_type='click' AND {time_filter}"
            ),
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    let wpm: f64 = conn
        .query_row(
            &format!(
                "SELECT COALESCE(AVG(cnt),0) FROM ( \
                   SELECT COUNT(*)/5.0 AS cnt FROM key_events \
                   WHERE {time_filter} \
                   AND key_name NOT IN ('Shift','Ctrl','Alt','CapsLock','Win','LShift','RShift','LCtrl','RCtrl','LAlt','RAlt') \
                   GROUP BY strftime('%Y-%m-%d %H:%M',timestamp) \
                   HAVING COUNT(*)>=5 \
                 )"
            ),
            [],
            |r| r.get(0),
        )
        .unwrap_or(0.0);
    let active_minutes: u64 = conn
        .query_row(
            &format!(
                "SELECT COUNT(DISTINCT strftime('%Y-%m-%d %H:%M',timestamp)) FROM key_events WHERE {time_filter}"
            ),
            [],
            |r| r.get(0),
        )
        .unwrap_or(0);
    PeriodStat {
        keys,
        clicks,
        wpm,
        active_minutes,
    }
}

// ---------------------------------------------------------------------------
// Hardware bounded rotating token system
// ---------------------------------------------------------------------------

fn derive_token(fingerprint: &str, purpose: &str, bucket: u64) -> String {
    use std::hash::{Hash, Hasher};
    let mut h1 = std::collections::hash_map::DefaultHasher::new();
    fingerprint.hash(&mut h1);
    purpose.hash(&mut h1);
    bucket.hash(&mut h1);
    0xA1B2C3D4u64.hash(&mut h1);
    let a = h1.finish();

    let mut h2 = std::collections::hash_map::DefaultHasher::new();
    a.hash(&mut h2);
    fingerprint.hash(&mut h2);
    purpose.hash(&mut h2);
    0xE5F60718u64.hash(&mut h2);
    let b = h2.finish();

    format!("{a:016x}{b:016x}")
}

const STATS_TOKEN_CYCLE_SECS: u64 = 15 * 60;
const RESTART_TOKEN_CYCLE_SECS: u64 = 60 * 60;

fn current_epoch_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn current_stats_tokens(fingerprint: &str) -> (String, String) {
    let now = current_epoch_secs();
    let bucket = now / STATS_TOKEN_CYCLE_SECS;
    (
        derive_token(fingerprint, "stats", bucket),
        derive_token(fingerprint, "stats", bucket.wrapping_sub(1)),
    )
}

fn current_restart_tokens(fingerprint: &str) -> (String, String) {
    let now = current_epoch_secs();
    let bucket = now / RESTART_TOKEN_CYCLE_SECS;
    (
        derive_token(fingerprint, "restart", bucket),
        derive_token(fingerprint, "restart", bucket.wrapping_sub(1)),
    )
}

fn current_stop_tokens(fingerprint: &str) -> (String, String) {
    let now = current_epoch_secs();
    let bucket = now / RESTART_TOKEN_CYCLE_SECS;
    (
        derive_token(fingerprint, "stop", bucket),
        derive_token(fingerprint, "stop", bucket.wrapping_sub(1)),
    )
}

fn validate_token(submitted: &str, current: &str, previous: &str) -> bool {
    submitted == current || submitted == previous
}

/// Validate a stats token against the current hardware fingerprint.
/// Exposed for use by the WebSocket server in main.rs.
pub fn validate_stats_token(submitted: &str) -> bool {
    let fp = crate::platform::hardware_fingerprint();
    let (cur, prev) = current_stats_tokens(&fp);
    validate_token(submitted, &cur, &prev)
}

// ---------------------------------------------------------------------------
// Dashboard HTTP server (tiny_http on localhost:9898)
// ---------------------------------------------------------------------------

const DASHBOARD_HTML: &str = include_str!(concat!(env!("OUT_DIR"), "/dashboard.html"));
const FAVICON_ICO: &[u8] = include_bytes!("../keyboard_logo.ico");

pub fn start_dashboard(db_path: Arc<PathBuf>) {
    let fingerprint = Arc::new(crate::platform::hardware_fingerprint());

    // Shared cache for the stats JSON, refreshed by a background thread.
    let default_json = serde_json::to_string(&default_stats()).unwrap_or_else(|_| "{}".into());
    let stats_cache: Arc<RwLock<String>> = Arc::new(RwLock::new(default_json));

    // Background thread: refresh cache every 2 seconds.
    {
        let db_path = Arc::clone(&db_path);
        let cache = Arc::clone(&stats_cache);
        thread::spawn(move || loop {
            let stats = query_stats(&db_path);
            let json = serde_json::to_string(&stats).unwrap_or_default();
            if let Ok(mut w) = cache.write() {
                *w = json;
            }
            thread::sleep(Duration::from_secs(2));
        });
    }

    thread::spawn(move || {
        let server = tiny_http::Server::http("127.0.0.1:9898")
            .expect("Failed to start dashboard server on :9898");
        let json_header: tiny_http::Header = "Content-Type: application/json".parse().unwrap();
        let html_header: tiny_http::Header =
            "Content-Type: text/html; charset=utf-8".parse().unwrap();
        let ico_header: tiny_http::Header = "Content-Type: image/x-icon".parse().unwrap();
        for request in server.incoming_requests() {
            let url = request.url().to_string();
            let path = url.split('?').next().unwrap_or(&url);
            match path {
                "/favicon.ico" => {
                    let response =
                        tiny_http::Response::from_data(FAVICON_ICO).with_header(ico_header.clone());
                    let _ = request.respond(response);
                }
                "/api/tokens" => {
                    let (stats_tok, _) = current_stats_tokens(&fingerprint);
                    let (restart_tok, _) = current_restart_tokens(&fingerprint);
                    let (stop_tok, _) = current_stop_tokens(&fingerprint);
                    let json = format!(
                        r#"{{"stats":"{}","restart":"{}","stop":"{}"}}"#,
                        stats_tok, restart_tok, stop_tok
                    );
                    let response =
                        tiny_http::Response::from_string(json).with_header(json_header.clone());
                    let _ = request.respond(response);
                }
                "/api/stats" => {
                    let qs = url.split('?').nth(1).unwrap_or("");
                    let submitted = qs
                        .split('&')
                        .find(|p| p.starts_with("token="))
                        .and_then(|p| p.strip_prefix("token="))
                        .unwrap_or("");
                    let (cur, prev) = current_stats_tokens(&fingerprint);
                    if !validate_token(submitted, &cur, &prev) {
                        let response = tiny_http::Response::from_string(r#"{"error":"forbidden"}"#)
                            .with_status_code(403)
                            .with_header(json_header.clone());
                        let _ = request.respond(response);
                        continue;
                    }
                    let json = stats_cache
                        .read()
                        .unwrap_or_else(|e| e.into_inner())
                        .clone();
                    let response =
                        tiny_http::Response::from_string(json).with_header(json_header.clone());
                    let _ = request.respond(response);
                }
                "/api/touchpad_fingers" => {
                    let qs = url.split('?').nth(1).unwrap_or("");
                    let submitted = qs
                        .split('&')
                        .find(|p| p.starts_with("token="))
                        .and_then(|p| p.strip_prefix("token="))
                        .unwrap_or("");
                    let (cur, prev) = current_stats_tokens(&fingerprint);
                    if !validate_token(submitted, &cur, &prev) {
                        let _ = request.respond(
                            tiny_http::Response::from_string(r#"{"error":"forbidden"}"#)
                                .with_status_code(403)
                                .with_header(json_header.clone()),
                        );
                        continue;
                    }
                    let mut data = std::collections::HashMap::new();
                    if let Ok(conn) = Connection::open(&*db_path) {
                        if let Ok(mut stmt) =
                            conn.prepare("SELECT fingers, count FROM touchpad_fingers")
                        {
                            if let Ok(map) = stmt.query_map([], |row| {
                                Ok((row.get::<_, u32>(0)?, row.get::<_, u32>(1)?))
                            }) {
                                for item in map.filter_map(Result::ok) {
                                    data.insert(item.0, item.1);
                                }
                            }
                        }
                    }
                    let _ = request.respond(
                        tiny_http::Response::from_string(
                            serde_json::to_string(&data).unwrap_or_else(|_| "{}".to_string()),
                        )
                        .with_header(json_header.clone()),
                    );
                }
                "/api/touchpad_heatmap" => {
                    let qs = url.split('?').nth(1).unwrap_or("");
                    let submitted = qs
                        .split('&')
                        .find(|p| p.starts_with("token="))
                        .and_then(|p| p.strip_prefix("token="))
                        .unwrap_or("");
                    let (cur, prev) = current_stats_tokens(&fingerprint);
                    if !validate_token(submitted, &cur, &prev) {
                        let _ = request.respond(
                            tiny_http::Response::from_string(r#"{"error":"forbidden"}"#)
                                .with_status_code(403)
                                .with_header(json_header.clone()),
                        );
                        continue;
                    }
                    let mut points = Vec::new();
                    if let Ok(conn) = Connection::open(&*db_path) {
                        if let Ok(mut stmt) =
                            conn.prepare("SELECT x_pos, y_pos, hit_count FROM touchpad_heatmap")
                        {
                            let iter = stmt.query_map([], |row| {
                                Ok(serde_json::json!({
                                    "x": row.get::<_, i32>(0)?,
                                    "y": row.get::<_, i32>(1)?,
                                    "count": row.get::<_, i32>(2)?
                                }))
                            });
                            if let Ok(mapped) = iter {
                                for item in mapped.filter_map(Result::ok) {
                                    points.push(item);
                                }
                            }
                        }
                    }
                    let _ = request.respond(
                        tiny_http::Response::from_string(
                            serde_json::to_string(&points).unwrap_or_else(|_| "[]".to_string()),
                        )
                        .with_header(json_header.clone()),
                    );
                }
                "/api/live_touchpad" => {
                    let qs = url.split('?').nth(1).unwrap_or("");
                    let submitted = qs
                        .split('&')
                        .find(|p| p.starts_with("token="))
                        .and_then(|p| p.strip_prefix("token="))
                        .unwrap_or("");
                    let (cur, prev) = current_stats_tokens(&fingerprint);
                    if !validate_token(submitted, &cur, &prev) {
                        let response = tiny_http::Response::from_string(r#"{"error":"forbidden"}"#)
                            .with_status_code(403)
                            .with_header(json_header.clone());
                        let _ = request.respond(response);
                        continue;
                    }
                    let contacts = crate::platform::live_touchpad();
                    let json =
                        serde_json::to_string(&contacts).unwrap_or_else(|_| "[]".to_string());
                    let response =
                        tiny_http::Response::from_string(json).with_header(json_header.clone());
                    let _ = request.respond(response);
                }
                "/api/hour_stats" => {
                    let qs = url.split('?').nth(1).unwrap_or("");
                    let submitted = qs
                        .split('&')
                        .find(|p| p.starts_with("token="))
                        .and_then(|p| p.strip_prefix("token="))
                        .unwrap_or("");
                    let (cur, prev) = current_stats_tokens(&fingerprint);
                    if !validate_token(submitted, &cur, &prev) {
                        let response = tiny_http::Response::from_string(r#"{"error":"forbidden"}"#)
                            .with_status_code(403)
                            .with_header(json_header.clone());
                        let _ = request.respond(response);
                        continue;
                    }
                    let offset: u32 = qs
                        .split('&')
                        .find(|p| p.starts_with("offset="))
                        .and_then(|p| p.strip_prefix("offset="))
                        .and_then(|v| v.parse().ok())
                        .unwrap_or(0);
                    let stat = query_hour_stats(&db_path, offset);
                    let json = serde_json::to_string(&stat).unwrap_or_default();
                    let response =
                        tiny_http::Response::from_string(json).with_header(json_header.clone());
                    let _ = request.respond(response);
                }
                "/api/click_positions" => {
                    let qs = url.split('?').nth(1).unwrap_or("");
                    let submitted = qs
                        .split('&')
                        .find(|p| p.starts_with("token="))
                        .and_then(|p| p.strip_prefix("token="))
                        .unwrap_or("");
                    let (cur, prev) = current_stats_tokens(&fingerprint);
                    if !validate_token(submitted, &cur, &prev) {
                        let response = tiny_http::Response::from_string(r#"{"error":"forbidden"}"#)
                            .with_status_code(403)
                            .with_header(json_header.clone());
                        let _ = request.respond(response);
                        continue;
                    }
                    let positions = query_click_positions(&db_path);
                    let json = serde_json::to_string(&positions).unwrap_or_default();
                    let response =
                        tiny_http::Response::from_string(json).with_header(json_header.clone());
                    let _ = request.respond(response);
                }
                "/api/restart" => {
                    let method = request.method().as_str();
                    let qs = url.split('?').nth(1).unwrap_or("");
                    let submitted = qs
                        .split('&')
                        .find(|p| p.starts_with("token="))
                        .and_then(|p| p.strip_prefix("token="))
                        .unwrap_or("");
                    let (cur, prev) = current_restart_tokens(&fingerprint);

                    if method != "POST" || !validate_token(submitted, &cur, &prev) {
                        let response = tiny_http::Response::from_string(r#"{"error":"forbidden"}"#)
                            .with_status_code(403)
                            .with_header(json_header.clone());
                        let _ = request.respond(response);
                        continue;
                    }

                    crate::platform::signal_restart();
                    let response = tiny_http::Response::from_string(r#"{"status":"restarting"}"#)
                        .with_header(json_header.clone());
                    let _ = request.respond(response);
                }
                "/api/stop" => {
                    let method = request.method().as_str();
                    let qs = url.split('?').nth(1).unwrap_or("");
                    let submitted = qs
                        .split('&')
                        .find(|p| p.starts_with("token="))
                        .and_then(|p| p.strip_prefix("token="))
                        .unwrap_or("");
                    let (cur, prev) = current_stop_tokens(&fingerprint);

                    if method != "POST" || !validate_token(submitted, &cur, &prev) {
                        let response = tiny_http::Response::from_string(r#"{"error":"forbidden"}"#)
                            .with_status_code(403)
                            .with_header(json_header.clone());
                        let _ = request.respond(response);
                        continue;
                    }

                    crate::platform::signal_stop();
                    let response = tiny_http::Response::from_string(r#"{"status":"stopping"}"#)
                        .with_header(json_header.clone());
                    let _ = request.respond(response);
                }
                _ => {
                    let response = tiny_http::Response::from_string(DASHBOARD_HTML)
                        .with_header(html_header.clone());
                    let _ = request.respond(response);
                }
            }
        }
    });
}
