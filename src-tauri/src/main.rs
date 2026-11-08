// Compvisor - standalone offline Windows desktop app (Rust + Tauri, no Electron).
mod db;
mod monitor;

use db::{Activity, Db, TimeSlot};
use serde_json::json;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{Manager, State};

struct AppState {
    db: Arc<Db>,
    running: Arc<AtomicBool>, // timer active?
    start_time: Arc<std::sync::Mutex<Option<i64>>>, // unix seconds when tracking started
}

fn db_path(handle: &tauri::AppHandle) -> PathBuf {
    // Embedded database lives in the user's app-data folder (Windows: %APPDATA%\com.compvisor.desktop)
    let dir = handle
        .path()
        .app_config_dir()
        .or_else(|_| handle.path().data_dir())
        .unwrap_or_else(|_| {
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|d| d.to_path_buf()))
                .unwrap_or_else(|| PathBuf::from("."))
        });
    std::fs::create_dir_all(&dir).ok();
    dir.join("compvisor.db")
}

#[tauri::command]
fn get_status(state: State<AppState>) -> Result<serde_json::Value, String> {
    let dur_total = state.db.totals().0;
    let st = monitor::current_status();
    let running = state.running.load(Ordering::Relaxed);
    let elapsed = match *state.start_time.lock().unwrap() {
        Some(s) => chrono::Local::now().timestamp() - s,
        None => 0,
    };
    Ok(json!({
        "running": running,
        "elapsed": elapsed,
        "totalDuration": dur_total,
        "keyboard": st.keyboard,
        "mouse": st.mouse,
        "activeWindow": st.active_window,
        "activeApp": st.active_app,
    }))
}

#[tauri::command]
fn start_tracking(state: State<AppState>) -> Result<(), String> {
    let mut guard = state.start_time.lock().unwrap();
    if guard.is_none() {
        *guard = Some(chrono::Local::now().timestamp());
    }
    state.running.store(true, Ordering::Relaxed);
    state.db.set_setting("running", "1")?;
    Ok(())
}

#[tauri::command]
fn stop_tracking(state: State<AppState>) -> Result<TimeSlot, String> {
    let start = state.start_time.lock().unwrap().take();
    state.running.store(false, Ordering::Relaxed);
    state.db.set_setting("running", "0")?;
    let now_ts = chrono::Local::now().timestamp();
    let duration = start.map(|s| now_ts - s).unwrap_or(0);
    let mon = monitor::status_and_reset();
    let slot = TimeSlot {
        id: String::new(),
        tenant_id: "local".into(),
        employee_id: state
            .db
            .get_setting("employeeId")
            .unwrap_or_else(|| "me".into()),
        time_spans_count: 1,
        duration,
        keyboard_key: mon.keyboard.to_string(),
        mouse_key: mon.mouse.to_string(),
        local_date_time_start: start
            .map(|s| {
                chrono::DateTime::<chrono::Local>::from_timestamp(s, 0)
                    .map(|d| d.format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_default()
            })
            .unwrap_or_else(db::now),
        local_date_time_end: db::now(),
    };
    let id = state.db.add_timeslot(&slot)?;
    Ok(TimeSlot { id, ..slot })
}

#[tauri::command]
fn add_activity(a: Activity, state: State<AppState>) -> Result<String, String> {
    state.db.add_activity(&a)
}

#[tauri::command]
fn list_activities(limit: Option<i64>, state: State<AppState>) -> Result<Vec<Activity>, String> {
    Ok(state.db.list_activities(limit.unwrap_or(200)))
}

#[tauri::command]
fn list_timeslots(limit: Option<i64>, state: State<AppState>) -> Result<Vec<TimeSlot>, String> {
    Ok(state.db.list_timeslots(limit.unwrap_or(200)))
}

#[tauri::command]
fn delete_timeslot(id: String, state: State<AppState>) -> Result<(), String> {
    state.db.delete_timeslot(&id)
}

#[tauri::command]
fn daily_stats(days: Option<i64>, state: State<AppState>) -> Result<Vec<db::DailyStat>, String> {
    Ok(state.db.daily_stats(days.unwrap_or(7)))
}

#[tauri::command]
fn get_setting(key: String, state: State<AppState>) -> Option<String> {
    state.db.get_setting(&key)
}

#[tauri::command]
fn set_setting(key: String, value: String, state: State<AppState>) -> Result<(), String> {
    state.db.set_setting(&key, &value)
}

#[tauri::command]
fn clear_data(state: State<AppState>) -> Result<(), String> {
    state.db.clear_all()
}

#[tauri::command]
fn export_data(state: State<AppState>) -> Result<serde_json::Value, String> {
    Ok(json!({
        "activities": state.db.list_activities(100000),
        "timeSlots": state.db.list_timeslots(100000),
        "settings": state.db.all_settings(),
    }))
}

fn main() {
    // single instance of the app directory/db
    tauri::Builder::default()
        .setup(|app| {
            let path = db_path(&app.handle());
            let database = Db::new(path.to_str().unwrap_or("compvisor.db"))
                .map_err(|e| format!("Failed to open embedded database: {}", e))?;
            let state = AppState {
                db: Arc::new(database),
                running: Arc::new(AtomicBool::new(false)),
                start_time: Arc::new(std::sync::Mutex::new(None)),
            };
            app.manage(state);
            monitor::spawn_monitor_thread();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_status,
            start_tracking,
            stop_tracking,
            add_activity,
            list_activities,
            list_timeslots,
            delete_timeslot,
            daily_stats,
            get_setting,
            set_setting,
            clear_data,
            export_data
        ])
        .run(tauri::generate_context!())
        .expect("error running Compvisor");
}
