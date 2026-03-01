// log_parser.rs - VRChatログ解析

use regex::Regex;
use std::path::{Path, PathBuf};

/// VRChatログ解析結果イベント
#[derive(Debug, Clone)]
pub enum LogEvent {
    WorldJoined { world_id: String, instance_id: String },
    WorldLeft { world_id: String, instance_id: String },
}

/// VRChatログ解析クラス
pub struct VRChatLogParser {
    log_dir: PathBuf,
    pub current_world: Option<(String, String)>,
    last_position: u64,
    current_log_file: Option<PathBuf>,
    re_world_join: Regex,
    re_world_leave: Regex,
}

impl VRChatLogParser {
    pub fn new() -> Self {
        let log_dir = Self::get_log_dir();
        Self {
            log_dir,
            current_world: None,
            last_position: 0,
            current_log_file: None,
            re_world_join: Regex::new(r"Joining (wrld_[a-zA-Z0-9\-]+):(\d+)").unwrap(),
            re_world_leave: Regex::new(r"Leaving wrld_").unwrap(),
        }
    }

    fn get_log_dir() -> PathBuf {
        // %APPDATA%\..\LocalLow\VRChat\VRChat
        if let Ok(appdata) = std::env::var("APPDATA") {
            let base = Path::new(&appdata).parent().unwrap_or(Path::new(&appdata));
            let path = base.join("LocalLow").join("VRChat").join("VRChat");
            if path.exists() {
                return path;
            }
        }
        // フォールバック
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("AppData")
            .join("LocalLow")
            .join("VRChat")
            .join("VRChat")
    }

    /// 最新のログファイルを取得
    fn get_latest_log(&self) -> Option<PathBuf> {
        if !self.log_dir.exists() {
            return None;
        }
        let mut logs: Vec<PathBuf> = std::fs::read_dir(&self.log_dir)
            .ok()?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("output_log_") && n.ends_with(".txt"))
                    .unwrap_or(false)
            })
            .collect();

        logs.sort_by_key(|p| {
            p.metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
        });
        logs.into_iter().last()
    }

    /// 新しいログ行を解析してイベントを返す
    pub fn parse_new_lines(&mut self) -> Vec<LogEvent> {
        let mut events = Vec::new();

        let log_file = match self.get_latest_log() {
            Some(f) => f,
            None => return events,
        };

        // ファイルが変わったらリセット
        if Some(&log_file) != self.current_log_file.as_ref() {
            self.current_log_file = Some(log_file.clone());
            self.last_position = 0;
        }

        let content = match std::fs::read_to_string(&log_file) {
            Ok(c) => c,
            Err(_) => return events,
        };

        // 前回の位置から新しい行だけ処理
        let bytes = content.as_bytes();
        if self.last_position as usize >= bytes.len() {
            return events;
        }

        let new_content = &content[self.last_position as usize..];
        self.last_position = bytes.len() as u64;

        for line in new_content.lines() {
            // ワールド参加
            if let Some(cap) = self.re_world_join.captures(line) {
                let world_id = cap[1].to_string();
                let instance_id = cap[2].to_string();
                self.current_world = Some((world_id.clone(), instance_id.clone()));
                events.push(LogEvent::WorldJoined { world_id, instance_id });
            }
            // ワールド退出
            else if self.re_world_leave.is_match(line) {
                if let Some((world_id, instance_id)) = self.current_world.take() {
                    events.push(LogEvent::WorldLeft { world_id, instance_id });
                }
            }
        }

        events
    }
}
