use rusqlite::Connection;

// ---------------------------------------------------------------------------
// SQLite setup
// ---------------------------------------------------------------------------

pub fn init_db(conn: &Connection) {
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA synchronous  = NORMAL;
         CREATE TABLE IF NOT EXISTS key_events (
             id         INTEGER PRIMARY KEY,
             timestamp  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f','now','localtime')),
             vk_code    INTEGER NOT NULL,
             key_name   TEXT    NOT NULL,
             shift_held INTEGER NOT NULL,
             caps_on    INTEGER NOT NULL,
             hold_ms    INTEGER
         );
         CREATE TABLE IF NOT EXISTS mouse_events (
             id         INTEGER PRIMARY KEY,
             timestamp  TEXT    NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%f','now','localtime')),
             event_type TEXT    NOT NULL,
             button     TEXT,
             x          INTEGER NOT NULL,
             y          INTEGER NOT NULL,
             delta      INTEGER
         );
         CREATE TABLE IF NOT EXISTS touchpad_heatmap (
             x_pos INTEGER,
             y_pos INTEGER,
             hit_count INTEGER NOT NULL DEFAULT 1,
             PRIMARY KEY (x_pos, y_pos)
         );
         CREATE TABLE IF NOT EXISTS touchpad_fingers (
             fingers INTEGER PRIMARY KEY,
             count INTEGER NOT NULL DEFAULT 1
         );",
    )
    .expect("Failed to initialize database");
}

// ---------------------------------------------------------------------------
// Batch flushing helpers
// ---------------------------------------------------------------------------

pub fn flush_keys(conn: &Connection, buf: &mut Vec<(u32, String, bool, bool, Option<u64>)>) {
    if buf.is_empty() {
        return;
    }
    let Ok(tx) = conn.unchecked_transaction() else {
        return;
    };
    {
        let Ok(mut stmt) = tx
            .prepare_cached(
                "INSERT INTO key_events (vk_code, key_name, shift_held, caps_on, hold_ms) VALUES (?1, ?2, ?3, ?4, ?5)",
            ) else { return };
        for (vk, name, shift, caps, hold_ms) in buf.iter() {
            let _ = stmt.execute(rusqlite::params![
                vk,
                name,
                *shift as i32,
                *caps as i32,
                hold_ms
            ]);
        }
    }
    let _ = tx.commit();
    buf.clear();
}

pub fn flush_mouse(conn: &Connection, buf: &mut Vec<(String, i32, i32)>) {
    if buf.is_empty() {
        return;
    }
    let Ok(tx) = conn.unchecked_transaction() else {
        return;
    };
    {
        let Ok(mut stmt) = tx.prepare_cached(
            "INSERT INTO mouse_events (event_type, button, x, y) VALUES ('click', ?1, ?2, ?3)",
        ) else {
            return;
        };
        for (btn, x, y) in buf.iter() {
            let _ = stmt.execute(rusqlite::params![btn, x, y]);
        }
    }
    let _ = tx.commit();
    buf.clear();
}

pub fn flush_touchpad(conn: &Connection, buf: &mut std::collections::HashMap<(i32, i32), u32>) {
    if buf.is_empty() {
        return;
    }
    let Ok(tx) = conn.unchecked_transaction() else {
        return;
    };
    {
        let Ok(mut stmt) = tx.prepare_cached(
            "INSERT INTO touchpad_heatmap (x_pos, y_pos, hit_count) VALUES (?1, ?2, ?3)
             ON CONFLICT(x_pos, y_pos) DO UPDATE SET hit_count = hit_count + ?3",
        ) else {
            return;
        };
        for ((x, y), count) in buf.iter() {
            let _ = stmt.execute(rusqlite::params![x, y, count]);
        }
    }
    let _ = tx.commit();
    buf.clear();
}

pub fn flush_touchpad_fingers(conn: &Connection, buf: &mut std::collections::HashMap<u32, u32>) {
    if buf.is_empty() {
        return;
    }
    let Ok(tx) = conn.unchecked_transaction() else {
        return;
    };
    {
        let Ok(mut stmt) = tx.prepare_cached(
            "INSERT INTO touchpad_fingers (fingers, count) VALUES (?1, ?2)
             ON CONFLICT(fingers) DO UPDATE SET count = count + ?2",
        ) else {
            return;
        };
        for (fingers, count) in buf.iter() {
            let _ = stmt.execute(rusqlite::params![fingers, count]);
        }
    }
    let _ = tx.commit();
    buf.clear();
}
