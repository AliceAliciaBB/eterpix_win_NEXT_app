// lib.rs - Tauriアプリケーション本体
// コマンド定義・AppState・バックグラウンドタスク・トレイアイコン

mod config;
mod image_processor;
mod log_parser;
mod offline_queue;
mod osc_handler;
mod startup;
mod uploader;
mod watcher;

use config::{AppConfig, VISIBILITY_OPTIONS};
use image_processor::process_screenshot;
use log_parser::{LogEvent, VRChatLogParser};
use offline_queue::OfflineQueueManager;
use osc_handler::{OscEvent, OscHandler};
use startup::{is_startup_registered, register_startup, unregister_startup};
use uploader::UploaderClient;
use watcher::ScreenshotWatcher;

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use tauri::{AppHandle, Emitter, Manager};

// ============================================================
// アプリケーション状態
// ============================================================

pub struct AppState {
    pub config: Mutex<AppConfig>,
    pub uploader: Mutex<UploaderClient>,
    pub watcher: Mutex<ScreenshotWatcher>,
    pub log_parser: Mutex<VRChatLogParser>,
    pub offline_queue: Mutex<OfflineQueueManager>,
    pub osc_handler: Mutex<OscHandler>,
    pub is_offline: AtomicBool,
    pub upload_history: Mutex<Vec<UploadHistoryItem>>,
    pub last_status: Mutex<String>,
    pub current_world: Mutex<Option<(String, String)>>,
    pub file_queue: Arc<Mutex<VecDeque<PathBuf>>>,
    pub osc_event_rx: Mutex<Option<std::sync::mpsc::Receiver<OscEvent>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UploadHistoryItem {
    pub filename: String,
    pub photo_uuid: Option<String>,
    pub time: String,
}

impl AppState {
    fn new(config: AppConfig) -> Self {
        let token = config.saved_token.clone();
        let server_url = config.server_url.clone();
        let osc_send = config.osc_send_port;
        let osc_recv = config.osc_recv_port;
        let queue_dir = AppConfig::queue_dir();

        let mut uploader = UploaderClient::new(server_url);
        uploader.token = token;

        Self {
            config: Mutex::new(config),
            uploader: Mutex::new(uploader),
            watcher: Mutex::new(ScreenshotWatcher::new()),
            log_parser: Mutex::new(VRChatLogParser::new()),
            offline_queue: Mutex::new(OfflineQueueManager::new(queue_dir)),
            osc_handler: Mutex::new(OscHandler::new(osc_send, osc_recv)),
            is_offline: AtomicBool::new(false),
            upload_history: Mutex::new(Vec::new()),
            last_status: Mutex::new("準備完了".to_string()),
            current_world: Mutex::new(None),
            file_queue: Arc::new(Mutex::new(VecDeque::new())),
            osc_event_rx: Mutex::new(None),
        }
    }
}

// ============================================================
// イベント送信ヘルパー
// ============================================================

fn emit_status(app: &AppHandle, msg: &str) {
    let _ = app.emit("status", serde_json::json!({ "message": msg }));
    if let Some(state) = app.try_state::<AppState>() {
        *state.last_status.lock().unwrap() = msg.to_string();
    }
}

fn set_offline(app: &AppHandle, state: &AppState, offline: bool) {
    let was = state.is_offline.load(Ordering::Relaxed);
    if was != offline {
        state.is_offline.store(offline, Ordering::Relaxed);
        let _ = app.emit("offline_mode", serde_json::json!({ "is_offline": offline }));
    }
}

// ============================================================
// フロントエンドに返す全状態
// ============================================================

#[derive(Serialize)]
struct AppStatus {
    logged_in: bool,
    username: Option<String>,
    server_url: String,
    watch_folder: String,
    default_visibility: String,
    visibility_options: Vec<(String, String)>,
    is_watching: bool,
    is_offline: bool,
    queue_count: usize,
    osc_running: bool,
    osc_recv: Option<i32>,
    osc_current: Option<String>,
    world: Option<WorldInfo>,
    upload_history: Vec<UploadHistoryItem>,
    last_status: String,
    startup_registered: bool,
    auto_upload: bool,
    jpeg_quality: u8,
    osc_send_port: u16,
    osc_recv_port: u16,
}

#[derive(Serialize)]
struct WorldInfo {
    world_id: String,
    instance_id: String,
}

// ============================================================
// Tauriコマンド
// ============================================================

#[tauri::command]
fn get_status(state: tauri::State<AppState>) -> AppStatus {
    let config = state.config.lock().unwrap();
    let uploader = state.uploader.lock().unwrap();
    let watcher = state.watcher.lock().unwrap();
    let osc = state.osc_handler.lock().unwrap();
    let queue = state.offline_queue.lock().unwrap();
    let history = state.upload_history.lock().unwrap();
    let world = state.current_world.lock().unwrap();
    let last_status = state.last_status.lock().unwrap();

    let counts = queue.get_queue_counts();
    let queue_count = counts.photos + counts.worlds;

    AppStatus {
        logged_in: uploader.token.is_some(),
        username: config.saved_username.clone(),
        server_url: config.server_url.clone(),
        watch_folder: config.get_watch_folder().to_string_lossy().to_string(),
        default_visibility: config.default_visibility.clone(),
        visibility_options: VISIBILITY_OPTIONS
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        is_watching: watcher.is_running,
        is_offline: state.is_offline.load(Ordering::Relaxed),
        queue_count,
        osc_running: osc.is_running,
        osc_recv: if osc.is_running { Some(osc.last_recv_value) } else { None },
        osc_current: if osc.is_running { Some(osc.current_visibility.clone()) } else { None },
        world: world.as_ref().map(|(wid, iid)| WorldInfo {
            world_id: wid.clone(),
            instance_id: iid.clone(),
        }),
        upload_history: history.clone(),
        last_status: last_status.clone(),
        startup_registered: is_startup_registered(),
        auto_upload: config.auto_upload,
        jpeg_quality: config.jpeg_quality,
        osc_send_port: config.osc_send_port,
        osc_recv_port: config.osc_recv_port,
    }
}

#[tauri::command]
async fn login(
    _app: AppHandle,
    state: tauri::State<'_, AppState>,
    username: String,
    password: String,
) -> Result<uploader::ApiResponse, String> {
    let resp = {
        // ブロッキング回避: Mutexをロックしたままawaitしない
        drop(state.uploader.lock().unwrap());
        let url = state.config.lock().unwrap().server_url.clone();
        let mut tmp = UploaderClient::new(url);
        let r = tmp.login(&username, &password).await;
        if r.is_success() {
            state.uploader.lock().unwrap().token = tmp.token.clone();
            let mut config = state.config.lock().unwrap();
            config.saved_token = tmp.token;
            config.saved_username = Some(username);
            config.save().ok();
        }
        r
    };
    Ok(resp)
}

#[tauri::command]
async fn register_user(
    _app: AppHandle,
    state: tauri::State<'_, AppState>,
    username: String,
    password: String,
) -> Result<uploader::ApiResponse, String> {
    let url = state.config.lock().unwrap().server_url.clone();
    let mut tmp = UploaderClient::new(url);
    let resp = tmp.register(&username, &password).await;
    if resp.is_success() {
        state.uploader.lock().unwrap().token = tmp.token.clone();
        let mut config = state.config.lock().unwrap();
        config.saved_token = tmp.token;
        config.saved_username = Some(username);
        config.save().ok();
    }
    Ok(resp)
}

#[tauri::command]
fn logout(app: AppHandle, state: tauri::State<AppState>) {
    state.uploader.lock().unwrap().token = None;
    let mut config = state.config.lock().unwrap();
    config.saved_token = None;
    config.saved_username = None;
    config.save().ok();
    emit_status(&app, "ログアウトしました");
}

#[tauri::command]
fn toggle_watch(app: AppHandle, state: tauri::State<AppState>) -> bool {
    let is_running = state.watcher.lock().unwrap().is_running;
    if is_running {
        state.watcher.lock().unwrap().stop();
        state.config.lock().unwrap().watch_enabled = false;
        state.config.lock().unwrap().save().ok();
        emit_status(&app, "監視停止");
    } else {
        let watch_path = state.config.lock().unwrap().get_watch_folder();
        let queue = state.file_queue.clone();
        let result = state.watcher.lock().unwrap().start(&watch_path, queue);
        if let Err(e) = result {
            emit_status(&app, &format!("監視開始失敗: {}", e));
        } else {
            state.config.lock().unwrap().watch_enabled = true;
            state.config.lock().unwrap().save().ok();
            emit_status(&app, "監視開始");
        }
    }
    state.watcher.lock().unwrap().is_running
}

#[tauri::command]
fn toggle_osc(app: AppHandle, state: tauri::State<AppState>) -> bool {
    let is_running = state.osc_handler.lock().unwrap().is_running;
    if is_running {
        state.osc_handler.lock().unwrap().stop();
        state.config.lock().unwrap().osc_enabled = false;
        state.config.lock().unwrap().save().ok();
        let _ = app.emit("osc_stopped", serde_json::json!({ "is_running": false }));
    } else {
        let (tx, rx) = std::sync::mpsc::channel::<OscEvent>();
        *state.osc_event_rx.lock().unwrap() = Some(rx);
        let vis = state.config.lock().unwrap().default_visibility.clone();
        let result = state.osc_handler.lock().unwrap().start(tx);
        if let Err(e) = result {
            emit_status(&app, &format!("OSC開始失敗: {}", e));
        } else {
            state.osc_handler.lock().unwrap().send_visibility(&vis);
            state.config.lock().unwrap().osc_enabled = true;
            state.config.lock().unwrap().save().ok();
            let _ = app.emit("osc_started", serde_json::json!({ "is_running": true }));
        }
    }
    state.osc_handler.lock().unwrap().is_running
}

#[tauri::command]
async fn check_server(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<bool, String> {
    let url = state.config.lock().unwrap().server_url.clone();
    let tmp = UploaderClient::new(url);
    let alive = tmp.health_check().await;
    set_offline(&app, &state, !alive);
    Ok(alive)
}

#[tauri::command]
async fn resend_queue(
    app: AppHandle,
    state: tauri::State<'_, AppState>,
) -> Result<bool, String> {
    let has_token = state.uploader.lock().unwrap().token.is_some();
    if !has_token {
        emit_status(&app, "ログインしていません");
        return Ok(false);
    }

    let counts = state.offline_queue.lock().unwrap().get_queue_counts();
    if counts.photos == 0 && counts.worlds == 0 {
        emit_status(&app, "送信待ちデータがありません");
        return Ok(true);
    }

    emit_status(&app, "サーバー確認中...");
    let url = state.config.lock().unwrap().server_url.clone();
    let token = state.uploader.lock().unwrap().token.clone();
    let mut tmp = UploaderClient::new(url);
    tmp.token = token;

    let alive = tmp.health_check().await;
    if !alive {
        emit_status(&app, "サーバーに接続できません");
        return Ok(false);
    }

    set_offline(&app, &state, false);
    emit_status(&app, "再送信中...");
    process_offline_queue(&app, &state, &tmp).await;
    emit_status(&app, "再送信完了");
    Ok(true)
}

#[tauri::command]
fn save_settings(
    app: AppHandle,
    state: tauri::State<AppState>,
    server_url: Option<String>,
    watch_folder: Option<String>,
    default_visibility: Option<String>,
    auto_upload: Option<bool>,
    jpeg_quality: Option<u8>,
    osc_send_port: Option<u16>,
    osc_recv_port: Option<u16>,
) {
    let mut config = state.config.lock().unwrap();
    if let Some(url) = server_url {
        config.server_url = url.clone();
        state.uploader.lock().unwrap().base_url = url;
    }
    if let Some(folder) = watch_folder {
        config.watch_folder = folder;
    }
    if let Some(vis) = default_visibility {
        config.default_visibility = vis.clone();
        // OSC送信
        if state.osc_handler.lock().unwrap().is_running {
            state.osc_handler.lock().unwrap().send_visibility(&vis);
        }
    }
    if let Some(au) = auto_upload {
        config.auto_upload = au;
    }
    if let Some(q) = jpeg_quality {
        config.jpeg_quality = q;
    }
    if let Some(p) = osc_send_port {
        config.osc_send_port = p;
    }
    if let Some(p) = osc_recv_port {
        config.osc_recv_port = p;
    }
    config.save().ok();
    emit_status(&app, "設定を保存しました");
}

#[tauri::command]
fn toggle_startup(_app: AppHandle) -> bool {
    if is_startup_registered() {
        unregister_startup();
        false
    } else {
        let exe = std::env::current_exe()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        register_startup(&exe)
    }
}

// ============================================================
// オフラインキュー処理 (内部関数)
// ============================================================

async fn process_offline_queue(app: &AppHandle, state: &AppState, uploader: &UploaderClient) {
    // ワールド参加を処理
    let world_joins = state.offline_queue.lock().unwrap().get_queued_world_joins();
    for wj in world_joins {
        let resp = uploader
            .report_instance_join(&wj.world_id, &wj.instance_id, None, None)
            .await;
        if resp.is_success() {
            state.offline_queue.lock().unwrap().remove_world_join(&wj.id);
            let _ = app.emit(
                "queue_item_sent",
                serde_json::json!({ "type": "world_join", "world_id": wj.world_id }),
            );
        } else {
            set_offline(app, state, true);
            return;
        }
    }

    // 写真を処理
    let photos = state.offline_queue.lock().unwrap().get_queued_photos();
    for (photo, bytes) in photos {
        let cam: Option<image_processor::CameraData> = photo
            .camera_data
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok());
        let resp = uploader
            .upload_photo(
                bytes,
                photo.filename.clone(),
                photo.world_id.clone(),
                photo.instance_id.clone(),
                photo.visibility.clone(),
                cam,
                photo.image_rotation,
            )
            .await;
        if resp.is_success() {
            state.offline_queue.lock().unwrap().remove_photo(&photo.id);
            let photo_uuid = resp
                .data
                .as_ref()
                .and_then(|d| d.get("photo_uuid"))
                .and_then(|v| v.as_str())
                .map(str::to_string);

            // 履歴に追加
            {
                let mut hist = state.upload_history.lock().unwrap();
                hist.insert(
                    0,
                    UploadHistoryItem {
                        filename: photo.filename.clone(),
                        photo_uuid: photo_uuid.clone(),
                        time: chrono::Local::now().format("%H:%M:%S").to_string(),
                    },
                );
                if hist.len() > 10 {
                    hist.truncate(10);
                }
            }

            let _ = app.emit(
                "queue_item_sent",
                serde_json::json!({
                    "type": "photo",
                    "filename": photo.filename,
                    "photo_uuid": photo_uuid,
                }),
            );
        } else {
            set_offline(app, state, true);
            return;
        }
    }

    let counts = state.offline_queue.lock().unwrap().get_queue_counts();
    let _ = app.emit(
        "queue_processed",
        serde_json::json!({
            "remaining_photos": counts.photos,
            "remaining_worlds": counts.worlds,
        }),
    );
}

// ============================================================
// バックグラウンドワーカー (毎秒実行)
// ============================================================

fn start_background_worker(app: AppHandle) {
    std::thread::spawn(move || {
        let mut health_counter = 0u64;
        const HEALTH_INTERVAL: u64 = 600; // 10分 (秒)

        loop {
            std::thread::sleep(std::time::Duration::from_secs(1));

            let state = match app.try_state::<AppState>() {
                Some(s) => s,
                None => break,
            };

            // ① VRChatログ解析
            let events = state.log_parser.lock().unwrap().parse_new_lines();
            for event in events {
                match event {
                    LogEvent::WorldJoined { world_id, instance_id } => {
                        *state.current_world.lock().unwrap() =
                            Some((world_id.clone(), instance_id.clone()));
                        let _ = app.emit(
                            "world_joined",
                            serde_json::json!({
                                "world_id": world_id,
                                "instance_id": instance_id,
                            }),
                        );
                        // ログイン中ならサーバーに報告
                        let token = state.uploader.lock().unwrap().token.clone();
                        if token.is_some() {
                            let app2 = app.clone();
                            let wid = world_id.clone();
                            let iid = instance_id.clone();
                            tauri::async_runtime::spawn(async move {
                                let st = app2.state::<AppState>();
                                let url = st.config.lock().unwrap().server_url.clone();
                                let token = st.uploader.lock().unwrap().token.clone();
                                let mut tmp = UploaderClient::new(url);
                                tmp.token = token;
                                let resp = tmp.report_instance_join(&wid, &iid, None, None).await;
                                if resp.is_success() {
                                    set_offline(&app2, &st, false);
                                } else {
                                    set_offline(&app2, &st, true);
                                    st.offline_queue.lock().unwrap().queue_world_join(&wid, &iid).ok();
                                }
                            });
                        }
                    }
                    LogEvent::WorldLeft { world_id, instance_id } => {
                        *state.current_world.lock().unwrap() = None;
                        let _ = app.emit(
                            "world_left",
                            serde_json::json!({ "world_id": world_id, "instance_id": instance_id }),
                        );
                        // 退出報告
                        let token = state.uploader.lock().unwrap().token.clone();
                        if token.is_some() {
                            let app2 = app.clone();
                            tauri::async_runtime::spawn(async move {
                                let st = app2.state::<AppState>();
                                let url = st.config.lock().unwrap().server_url.clone();
                                let token = st.uploader.lock().unwrap().token.clone();
                                let mut tmp = UploaderClient::new(url);
                                tmp.token = token;
                                let _ = tmp.report_instance_leave().await;
                            });
                        }
                    }
                }
            }

            // ② OSCイベント処理
            let osc_event = {
                let rx = state.osc_event_rx.lock().unwrap();
                rx.as_ref().and_then(|r| r.try_recv().ok())
            };
            if let Some(OscEvent::VisibilityChanged(vis)) = osc_event {
                state.config.lock().unwrap().default_visibility = vis.clone();
                state.config.lock().unwrap().save().ok();
                let _ = app.emit(
                    "osc_visibility_changed",
                    serde_json::json!({ "visibility": vis }),
                );
            }

            // ③ スクリーンショットキュー処理
            let files: Vec<PathBuf> = {
                let mut q = state.file_queue.lock().unwrap();
                q.drain(..).collect()
            };
            for path in files {
                let app2 = app.clone();
                tauri::async_runtime::spawn(async move {
                    handle_screenshot(app2, path).await;
                });
            }

            // ④ ヘルスチェック (10分ごと)
            health_counter += 1;
            if health_counter >= HEALTH_INTERVAL {
                health_counter = 0;
                let has_queue = state.offline_queue.lock().unwrap().has_pending_data();
                if has_queue {
                    let app2 = app.clone();
                    tauri::async_runtime::spawn(async move {
                        let st = app2.state::<AppState>();
                        let token = st.uploader.lock().unwrap().token.clone();
                        if token.is_none() {
                            return;
                        }
                        let url = st.config.lock().unwrap().server_url.clone();
                        let mut tmp = UploaderClient::new(url);
                        tmp.token = token;
                        if tmp.health_check().await {
                            set_offline(&app2, &st, false);
                            process_offline_queue(&app2, &st, &tmp).await;
                        }
                    });
                }
            }
        }
    });
}

// ============================================================
// スクリーンショット処理
// ============================================================

async fn handle_screenshot(app: AppHandle, path: PathBuf) {
    let state = app.state::<AppState>();

    let has_token = state.uploader.lock().unwrap().token.is_some();
    if !has_token {
        emit_status(&app, "ログインしていません");
        return;
    }

    let auto_upload = state.config.lock().unwrap().auto_upload;
    if !auto_upload {
        return;
    }

    let _ = app.emit(
        "upload_start",
        serde_json::json!({ "path": path.to_string_lossy() }),
    );

    let quality = state.config.lock().unwrap().jpeg_quality;
    let result = match tauri::async_runtime::spawn_blocking({
        let p = path.clone();
        move || process_screenshot(&p, quality)
    })
    .await
    {
        Ok(Ok(r)) => r,
        Ok(Err(e)) => {
            let _ = app.emit(
                "upload_error",
                serde_json::json!({ "path": path.to_string_lossy(), "error": e.to_string() }),
            );
            return;
        }
        Err(e) => {
            let _ = app.emit(
                "upload_error",
                serde_json::json!({ "path": path.to_string_lossy(), "error": e.to_string() }),
            );
            return;
        }
    };

    let world_info = state.current_world.lock().unwrap().clone();
    let (world_id, instance_id) = match world_info {
        Some((w, i)) => (Some(w), Some(i)),
        None => (None, None),
    };
    let visibility = state.config.lock().unwrap().default_visibility.clone();
    let image_rotation = if result.is_portrait { 1 } else { 0 };
    let filename = path.file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .replace(".png", ".jpg");

    // オフラインモードなら直接キューに入れる
    if state.is_offline.load(Ordering::Relaxed) {
        let queue_id = state.offline_queue.lock().unwrap().queue_photo(
            &result.jpg_bytes,
            &filename,
            world_id.as_deref(),
            instance_id.as_deref(),
            &visibility,
            result.camera_data.as_ref(),
            image_rotation,
        );
        if let Ok(qid) = queue_id {
            let count = state.offline_queue.lock().unwrap().get_queue_counts().photos;
            let _ = app.emit(
                "photo_queued",
                serde_json::json!({ "queue_id": qid, "filename": filename, "pending_count": count }),
            );
        }
        return;
    }

    // アップロード試行
    let url = state.config.lock().unwrap().server_url.clone();
    let token = state.uploader.lock().unwrap().token.clone();
    let mut tmp = UploaderClient::new(url);
    tmp.token = token;

    let resp = tmp
        .upload_photo(
            result.jpg_bytes.clone(),
            filename.clone(),
            world_id.clone(),
            instance_id.clone(),
            visibility.clone(),
            result.camera_data.clone(),
            image_rotation,
        )
        .await;

    if resp.is_success() {
        set_offline(&app, &state, false);
        let photo_uuid = resp
            .data
            .as_ref()
            .and_then(|d| d.get("photo_uuid"))
            .and_then(|v| v.as_str())
            .map(str::to_string);

        // 履歴に追加
        {
            let mut hist = state.upload_history.lock().unwrap();
            hist.insert(
                0,
                UploadHistoryItem {
                    filename: filename.clone(),
                    photo_uuid: photo_uuid.clone(),
                    time: chrono::Local::now().format("%H:%M:%S").to_string(),
                },
            );
            if hist.len() > 10 {
                hist.truncate(10);
            }
        }

        let _ = app.emit(
            "upload_complete",
            serde_json::json!({
                "path": path.to_string_lossy(),
                "photo_uuid": photo_uuid,
            }),
        );
    } else {
        // アップロード失敗 → オフラインモードへ
        set_offline(&app, &state, true);
        let queue_id = state.offline_queue.lock().unwrap().queue_photo(
            &result.jpg_bytes,
            &filename,
            world_id.as_deref(),
            instance_id.as_deref(),
            &visibility,
            result.camera_data.as_ref(),
            image_rotation,
        );
        if let Ok(qid) = queue_id {
            let count = state.offline_queue.lock().unwrap().get_queue_counts().photos;
            let _ = app.emit(
                "photo_queued",
                serde_json::json!({ "queue_id": qid, "filename": filename, "pending_count": count }),
            );
        }
    }
}

// ============================================================
// トレイアイコン設定
// ============================================================

fn setup_tray(app: &tauri::App) -> tauri::Result<()> {
    use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
    use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};

    let open_item = MenuItem::with_id(app, "open", "ウィンドウを開く", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "終了", true, None::<&str>)?;
    let sep = PredefinedMenuItem::separator(app)?;
    let menu = Menu::with_items(app, &[&open_item, &sep, &quit_item])?;

    let icon = app.default_window_icon().cloned()
        .or_else(|| {
            let path = app.path().resource_dir().ok()?.join("icons/icon.png");
            tauri::image::Image::from_path(&path).ok()
        })
        .expect("アイコンが見つかりません");

    TrayIconBuilder::new()
        .icon(icon)
        .tooltip("EterPix VRC Uploader")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => {
                if let Some(win) = app.get_webview_window("main") {
                    win.show().ok();
                    win.set_focus().ok();
                }
            }
            "quit" => {
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(win) = app.get_webview_window("main") {
                    if win.is_visible().unwrap_or(false) {
                        win.hide().ok();
                    } else {
                        win.show().ok();
                        win.set_focus().ok();
                    }
                }
            }
        })
        .build(app)?;

    Ok(())
}

// ============================================================
// アプリケーションエントリー
// ============================================================

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            if let Some(win) = app.get_webview_window("main") {
                win.show().ok();
                win.set_focus().ok();
            }
        }))
        .setup(|app| {
            // アプリ状態を初期化
            let config = AppConfig::load();

            // 起動フラグ確認 (--minimized)
            let start_minimized = std::env::args().any(|a| a == "--minimized");

            // 保存された監視/OSC状態を復元するために情報だけ取り出す
            let watch_enabled = config.watch_enabled;
            let osc_enabled = config.osc_enabled;

            let state = AppState::new(config);

            // オフラインキューに未送信データがあれば offline モードに
            let has_pending = state.offline_queue.lock().unwrap().has_pending_data();
            if has_pending {
                state.is_offline.store(true, Ordering::Relaxed);
            }

            app.manage(state);

            // トレイアイコン設定
            setup_tray(app)?;

            // ウィンドウ表示制御
            if start_minimized {
                if let Some(win) = app.get_webview_window("main") {
                    win.hide().ok();
                }
            }

            let app_handle = app.handle().clone();

            // 監視状態を復元
            if watch_enabled {
                let state = app_handle.state::<AppState>();
                let path = state.config.lock().unwrap().get_watch_folder();
                let queue = state.file_queue.clone();
                state.watcher.lock().unwrap().start(&path, queue).ok();
            }

            // OSC状態を復元
            if osc_enabled {
                let state = app_handle.state::<AppState>();
                let vis = state.config.lock().unwrap().default_visibility.clone();
                let (tx, rx) = std::sync::mpsc::channel::<OscEvent>();
                *state.osc_event_rx.lock().unwrap() = Some(rx);
                if state.osc_handler.lock().unwrap().start(tx).is_ok() {
                    state.osc_handler.lock().unwrap().send_visibility(&vis);
                }
            }

            // バックグラウンドワーカー開始
            start_background_worker(app_handle.clone());

            // 起動後3秒でキュー再送を試みる
            if has_pending {
                let app2 = app_handle.clone();
                tauri::async_runtime::spawn(async move {
                    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
                    let st = app2.state::<AppState>();
                    let token = st.uploader.lock().unwrap().token.clone();
                    if token.is_none() {
                        return;
                    }
                    let url = st.config.lock().unwrap().server_url.clone();
                    let mut tmp = UploaderClient::new(url);
                    tmp.token = token;
                    if tmp.health_check().await {
                        set_offline(&app2, &st, false);
                        process_offline_queue(&app2, &st, &tmp).await;
                    }
                });
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_status,
            login,
            register_user,
            logout,
            toggle_watch,
            toggle_osc,
            check_server,
            resend_queue,
            save_settings,
            toggle_startup,
        ])
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                // 閉じるボタンでトレイに隠す
                window.hide().ok();
                api.prevent_close();
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
