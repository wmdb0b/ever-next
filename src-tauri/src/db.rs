// Compvisor - embedded SQLite database layer (fully offline, no server, no external DB)
use chrono::Local;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use uuid::Uuid;

pub struct Db {
    pub conn: Mutex<Connection>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct Activity {
    pub id: String,
    pub source: String,
    pub duration: i64,
    #[serde(rename = "dateTime")]
    pub date_time: String,
    #[serde(rename = "projectId")]
    pub project_id: Option<String>,
    #[serde(rename = "taskId")]
    pub task_id: Option<String>,
    #[serde(rename = "entityId")]
    pub entity_id: Option<String>,
    #[serde(rename = "entityName")]
    pub entity_name: Option<String>,
    #[serde(rename = "keyboardCount")]
    pub keyboard_count: i64,
    #[serde(rename = "mouseCount")]
    pub mouse_count: i64,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TimeSlot {
    pub id: String,
    #[serde(rename = "tenantId")]
    pub tenant_id: String,
    #[serde(rename = "employeeId")]
    pub employee_id: String,
    #[serde(rename = "timeSpansCount")]
    pub time_spans_count: i64,
    pub duration: i64,
    #[serde(rename = "keyboardKey")]
    pub keyboard_key: String,
    #[serde(rename = "mouseKey")]
    pub mouse_key: String,
    #[serde(rename = "localDateTimeStart")]
    pub local_date_time_start: String,
    #[serde(rename = "localDateTimeEnd")]
    pub local_date_time_end: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct DailyStat {
    pub date: String,
    pub duration: i64,
    pub activities: i64,
    #[serde(rename = "keyboardCount")]
    pub keyboard_count: i64,
    #[serde(rename = "mouseCount")]
    pub mouse_count: i64,
}

pub fn now() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S").to_string()
}

impl Db {
    pub fn new(path: &str) -> Result<Db, String> {
        let conn = Connection::open(path).map_err(|e| e.to_string())?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             CREATE TABLE IF NOT EXISTS settings(key TEXT PRIMARY KEY, value TEXT);
             CREATE TABLE IF NOT EXISTS activities(
                id TEXT PRIMARY KEY, source TEXT, duration INTEGER, dateTime TEXT,
                projectId TEXT, taskId TEXT, entityId TEXT, entityName TEXT,
                keyboardCount INTEGER, mouseCount INTEGER);
             CREATE TABLE IF NOT EXISTS time_slots(
                id TEXT PRIMARY KEY, tenantId TEXT, employeeId TEXT, timeSpansCount INTEGER,
                duration INTEGER, keyboardKey TEXT, mouseKey TEXT,
                localDateTimeStart TEXT, localDateTimeEnd TEXT);
             CREATE INDEX IF NOT EXISTS idx_act_date ON activities(dateTime);
             CREATE INDEX IF NOT EXISTS idx_slot_date ON time_slots(localDateTimeStart);",
        )
        .map_err(|e| e.to_string())?;
        Ok(Db {
            conn: Mutex::new(conn),
        })
    }

    // ---------- settings ----------
    pub fn get_setting(&self, key: &str) -> Option<String> {
        let c = self.conn.lock().unwrap();
        c.query_row("SELECT value FROM settings WHERE key=?1", params![key], |r| {
            r.get::<_, String>(0)
        })
        .ok()
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), String> {
        let c = self.conn.lock().unwrap();
        c.execute(
            "INSERT INTO settings(key,value) VALUES(?1,?2)
             ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            params![key, value],
        )
        .map(|_| ())
        .map_err(|e| e.to_string())
    }

    pub fn all_settings(&self) -> Vec<(String, String)> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c.prepare("SELECT key,value FROM settings").unwrap();
        stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
            .unwrap()
            .filter_map(|x| x.ok())
            .collect()
    }

    // ---------- activities ----------
    pub fn add_activity(&self, a: &Activity) -> Result<String, String> {
        let id = if a.id.is_empty() {
            Uuid::new_v4().to_string()
        } else {
            a.id.clone()
        };
        let c = self.conn.lock().unwrap();
        c.execute(
            "INSERT INTO activities(id,source,duration,dateTime,projectId,taskId,entityId,entityName,keyboardCount,mouseCount)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
            params![id, a.source, a.duration, a.date_time, a.project_id, a.task_id, a.entity_id, a.entity_name, a.keyboard_count, a.mouse_count],
        )
        .map_err(|e| e.to_string())?;
        Ok(id)
    }

    pub fn list_activities(&self, limit: i64) -> Vec<Activity> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c
            .prepare(
                "SELECT id,source,duration,dateTime,projectId,taskId,entityId,entityName,keyboardCount,mouseCount
                 FROM activities ORDER BY dateTime DESC LIMIT ?1",
            )
            .unwrap();
        stmt.query_map(params![limit], |r| {
            Ok(Activity {
                id: r.get(0)?,
                source: r.get(1)?,
                duration: r.get(2)?,
                date_time: r.get(3)?,
                project_id: r.get(4)?,
                task_id: r.get(5)?,
                entity_id: r.get(6)?,
                entity_name: r.get(7)?,
                keyboard_count: r.get(8)?,
                mouse_count: r.get(9)?,
            })
        })
        .unwrap()
        .filter_map(|x| x.ok())
        .collect()
    }

    // ---------- time slots ----------
    pub fn add_timeslot(&self, t: &TimeSlot) -> Result<String, String> {
        let id = if t.id.is_empty() {
            Uuid::new_v4().to_string()
        } else {
            t.id.clone()
        };
        let c = self.conn.lock().unwrap();
        c.execute(
            "INSERT INTO time_slots(id,tenantId,employeeId,timeSpansCount,duration,keyboardKey,mouseKey,localDateTimeStart,localDateTimeEnd)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)",
            params![id, t.tenant_id, t.employee_id, t.time_spans_count, t.duration, t.keyboard_key, t.mouse_key, t.local_date_time_start, t.local_date_time_end],
        )
        .map_err(|e| e.to_string())?;
        Ok(id)
    }

    pub fn list_timeslots(&self, limit: i64) -> Vec<TimeSlot> {
        let c = self.conn.lock().unwrap();
        let mut stmt = c
            .prepare("SELECT id,tenantId,employeeId,timeSpansCount,duration,keyboardKey,mouseKey,localDateTimeStart,localDateTimeEnd FROM time_slots ORDER BY localDateTimeStart DESC LIMIT ?1")
            .unwrap();
        stmt.query_map(params![limit], |r| {
            Ok(TimeSlot {
                id: r.get(0)?,
                tenant_id: r.get(1)?,
                employee_id: r.get(2)?,
                time_spans_count: r.get(3)?,
                duration: r.get(4)?,
                keyboard_key: r.get(5)?,
                mouse_key: r.get(6)?,
                local_date_time_start: r.get(7)?,
                local_date_time_end: r.get(8)?,
            })
        })
        .unwrap()
        .filter_map(|x| x.ok())
        .collect()
    }

    pub fn delete_timeslot(&self, id: &str) -> Result<(), String> {
        let c = self.conn.lock().unwrap();
        c.execute("DELETE FROM time_slots WHERE id=?1", params![id])
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    // ---------- stats ----------
    pub fn daily_stats(&self, days: i64) -> Vec<DailyStat> {
        // last `days` local dates, aggregated with plain SQL per date (no extensions needed)
        let today = Local::today();
        let mut out = Vec::new();
        for i in 0..days {
            let d = today - chrono::Duration::days(i);
            let ds = d.format("%Y-%m-%d").to_string();
            let c = self.conn.lock().unwrap();
            let dur: i64 = c
                .query_row(
                    "SELECT COALESCE(SUM(duration),0) FROM time_slots WHERE substr(localDateTimeStart,1,10)=?1",
                    params![ds],
                    |r| r.get(0),
                )
                .unwrap_or(0);
            let cnt: i64 = c
                .query_row(
                    "SELECT COUNT(*) FROM activities WHERE substr(dateTime,1,10)=?1",
                    params![ds],
                    |r| r.get(0),
                )
                .unwrap_or(0);
            let kb: i64 = c
                .query_row(
                    "SELECT COALESCE(SUM(keyboardCount),0) FROM activities WHERE substr(dateTime,1,10)=?1",
                    params![ds],
                    |r| r.get(0),
                )
                .unwrap_or(0);
            let ms: i64 = c
                .query_row(
                    "SELECT COALESCE(SUM(mouseCount),0) FROM activities WHERE substr(dateTime,1,10)=?1",
                    params![ds],
                    |r| r.get(0),
                )
                .unwrap_or(0);
            drop(c);
            out.push(DailyStat { date: ds, duration: dur, activities: cnt, keyboard_count: kb, mouse_count: ms });
        }
        out.reverse();
        out
    }

    pub fn totals(&self) -> (i64, i64, i64) {
        let c = self.conn.lock().unwrap();
        let dur: i64 = c
            .query_row("SELECT COALESCE(SUM(duration),0) FROM time_slots", [], |r| r.get(0))
            .unwrap_or(0);
        let kb: i64 = c
            .query_row(
                "SELECT COALESCE(SUM(keyboardCount),0) FROM activities",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        let ms: i64 = c
            .query_row(
                "SELECT COALESCE(SUM(mouseCount),0) FROM activities",
                [],
                |r| r.get(0),
            )
            .unwrap_or(0);
        (dur, kb, ms)
    }

    pub fn clear_all(&self) -> Result<(), String> {
        let c = self.conn.lock().unwrap();
        c.execute_batch("DELETE FROM activities; DELETE FROM time_slots;")
            .map_err(|e| e.to_string())
    }
}
