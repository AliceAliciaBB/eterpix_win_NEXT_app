// config.rs - アプリケーション設定管理

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 公開範囲オプション
pub const VISIBILITY_OPTIONS: &[(&str, &str)] = &[
    ("self", "自分のみ"),
    ("friends", "フレンド"),
    ("instance_friends", "インスタンス&フレンド"),
    ("instance", "インスタンス"),
    ("public", "パブリック"),
];

/// #[serde(default)] により、JSON に存在しないフィールドは Default 値で補完される。
/// これにより旧バージョンの config.json を読み込んでも全リセットにならない（Python版の cls(**data) と同等）。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AppConfig {
    /// サーバーURL
    pub server_url: String,
    /// 監視フォルダ（空の場合はデフォルト）
    pub watch_folder: String,
    /// 自動アップロード
    pub auto_upload: bool,
    /// JPEG品質 (1-100)
    pub jpeg_quality: u8,
    /// デフォルト公開範囲
    pub default_visibility: String,
    /// 監視状態を保存
    pub watch_enabled: bool,
    /// OSC機能の有効/無効
    pub osc_enabled: bool,
    /// VRCへの送信ポート
    pub osc_send_port: u16,
    /// VRCからの受信ポート
    pub osc_recv_port: u16,
    /// 通知の有効/無効（Python版: notifications_enabled）
    pub notifications_enabled: bool,
    /// 保存済みトークン
    pub saved_token: Option<String>,
    /// 保存済みユーザー名
    pub saved_username: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server_url: "https://www.eterpix.uk".to_string(),
            watch_folder: String::new(),
            auto_upload: true,
            jpeg_quality: 85,
            default_visibility: "self".to_string(),
            watch_enabled: false,
            osc_enabled: false,
            osc_send_port: 9000,
            osc_recv_port: 9001,
            notifications_enabled: true,
            saved_token: None,
            saved_username: None,
        }
    }
}

impl AppConfig {
    /// 設定ファイルのパスを取得
    pub fn config_path() -> PathBuf {
        let base = if let Some(app_data) = dirs::data_dir() {
            app_data.join("EterPixUploader")
        } else {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".eterpix_uploader")
        };
        base.join("config.json")
    }

    /// 設定を読み込み（Python版 AppConfig.load() に相当）
    /// JSON に存在しないフィールドは Default 値で補完（#[serde(default)] により保証）
    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            match std::fs::read_to_string(&path) {
                Ok(content) => match serde_json::from_str::<Self>(&content) {
                    Ok(config) => return config,
                    Err(e) => eprintln!("設定読み込みエラー: {e}"),
                },
                Err(e) => eprintln!("設定ファイル読み取りエラー: {e}"),
            }
        }
        Self::default()
    }

    /// 設定を保存（Python版 AppConfig.save() に相当）
    /// indent=2 の pretty JSON で UTF-8 保存
    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, content.as_bytes())?;
        Ok(())
    }

    /// 監視フォルダを取得（デフォルト: Pictures/VRChat）
    pub fn get_watch_folder(&self) -> PathBuf {
        if !self.watch_folder.is_empty() {
            return PathBuf::from(&self.watch_folder);
        }
        let pictures = dirs::picture_dir()
            .unwrap_or_else(|| dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")));
        let vrchat = pictures.join("VRChat");
        if vrchat.exists() {
            vrchat
        } else {
            pictures
        }
    }

    /// オフラインキューの保存ディレクトリ
    pub fn queue_dir() -> PathBuf {
        let base = if let Some(app_data) = dirs::data_dir() {
            app_data.join("EterPixUploader")
        } else {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".eterpix_uploader")
        };
        base.join("queue")
    }
}
