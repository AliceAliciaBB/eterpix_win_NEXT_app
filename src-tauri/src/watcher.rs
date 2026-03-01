// watcher.rs - VRChatスクリーンショットフォルダ監視

use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// スクリーンショット監視クラス
pub struct ScreenshotWatcher {
    _watcher: Option<RecommendedWatcher>,
    pub is_running: bool,
}

impl ScreenshotWatcher {
    pub fn new() -> Self {
        Self {
            _watcher: None,
            is_running: false,
        }
    }

    /// 監視開始
    /// 新しいPNGファイルを検出したら `file_queue` に追加する
    pub fn start(
        &mut self,
        watch_path: &Path,
        file_queue: Arc<Mutex<VecDeque<PathBuf>>>,
    ) -> anyhow::Result<()> {
        if self.is_running {
            self.stop();
        }

        if !watch_path.exists() {
            anyhow::bail!("監視パスが存在しません: {}", watch_path.display());
        }

        let queue = file_queue.clone();

        let mut watcher = RecommendedWatcher::new(
            move |res: notify::Result<Event>| {
                if let Ok(event) = res {
                    if matches!(event.kind, EventKind::Create(_)) {
                        for path in event.paths {
                            // PNG ファイルのみ、デバッグ画像を除外
                            let name = path
                                .file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("");
                            if !name.ends_with(".png") || name.contains("_debug_") {
                                continue;
                            }

                            // ファイルが完全に書き込まれるまで待機
                            std::thread::sleep(Duration::from_millis(600));

                            if path.exists() {
                                // 簡易検証: ファイルを開けるか
                                if std::fs::File::open(&path).is_ok() {
                                    queue.lock().unwrap().push_back(path);
                                }
                            }
                        }
                    }
                }
            },
            Config::default(),
        )?;

        watcher.watch(watch_path, RecursiveMode::Recursive)?;
        self._watcher = Some(watcher);
        self.is_running = true;
        Ok(())
    }

    /// 監視停止
    pub fn stop(&mut self) {
        self._watcher = None;
        self.is_running = false;
    }
}
