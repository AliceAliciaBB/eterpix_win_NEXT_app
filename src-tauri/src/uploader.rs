// uploader.rs - EterPixサーバーとのHTTP通信クライアント

use crate::image_processor::CameraData;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse {
    pub status: String,
    pub message: Option<String>,
    pub data: Option<serde_json::Value>,
}

impl ApiResponse {
    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            status: "error".to_string(),
            message: Some(msg.into()),
            data: None,
        }
    }
    pub fn is_success(&self) -> bool {
        self.status == "success"
    }
}

pub struct UploaderClient {
    pub base_url: String,
    pub token: Option<String>,
    client: reqwest::Client,
}

impl UploaderClient {
    pub fn new(base_url: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .danger_accept_invalid_certs(false)
            .build()
            .unwrap_or_default();

        Self {
            base_url: base_url.trim_end_matches('/').to_string(),
            token: None,
            client,
        }
    }

    fn auth_header(&self) -> Option<(reqwest::header::HeaderName, reqwest::header::HeaderValue)> {
        self.token.as_ref().map(|t| {
            (
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", t).parse().unwrap(),
            )
        })
    }

    fn format_error(e: &reqwest::Error) -> String {
        if e.is_connect() {
            "サーバーに接続できません。ネットワークとサーバーURLを確認してください。".to_string()
        } else if e.is_timeout() {
            "接続がタイムアウトしました。".to_string()
        } else if let Some(status) = e.status() {
            match status.as_u16() {
                530 | 521..=524 => "サーバーに接続できません。しばらく待ってから再試行してください。".to_string(),
                500..=599 => format!("サーバーエラーが発生しました ({})。", status),
                401 => "ユーザー名またはパスワードが正しくありません。".to_string(),
                403 => "アクセスが拒否されました。".to_string(),
                404 => "サーバーのエンドポイントが見つかりません。サーバーURLを確認してください。".to_string(),
                429 => "リクエストが多すぎます。しばらく待ってから再試行してください。".to_string(),
                c => format!("HTTPエラー ({})", c),
            }
        } else {
            e.to_string()
        }
    }

    async fn post_json(&self, path: &str, body: serde_json::Value) -> ApiResponse {
        let mut req = self.client.post(format!("{}{}", self.base_url, path)).json(&body);
        if let Some((k, v)) = self.auth_header() {
            req = req.header(k, v);
        }
        match req.send().await {
            Ok(resp) => resp.json::<ApiResponse>().await.unwrap_or_else(|e| ApiResponse::error(e.to_string())),
            Err(e) => ApiResponse::error(Self::format_error(&e)),
        }
    }

    /// ログイン
    pub async fn login(&mut self, username: &str, password: &str) -> ApiResponse {
        let resp = self
            .post_json(
                "/vrc/api/auth/login",
                serde_json::json!({ "username": username, "password": password }),
            )
            .await;
        if resp.is_success() {
            if let Some(token) = resp.data.as_ref()
                .and_then(|d| d.get("token"))
                .and_then(|t| t.as_str())
            {
                self.token = Some(token.to_string());
            }
        }
        resp
    }

    /// 新規登録
    pub async fn register(&mut self, username: &str, password: &str) -> ApiResponse {
        let resp = self
            .post_json(
                "/vrc/api/auth/register",
                serde_json::json!({ "username": username, "password": password }),
            )
            .await;
        if resp.is_success() {
            if let Some(token) = resp
                .data
                .as_ref()
                .and_then(|d| d.get("token"))
                .and_then(|t| t.as_str())
            {
                self.token = Some(token.to_string());
            }
        }
        resp
    }

    /// ユーザー情報取得
    pub async fn get_me(&self) -> ApiResponse {
        let mut req = self.client.get(format!("{}/vrc/api/auth/me", self.base_url));
        if let Some((k, v)) = self.auth_header() {
            req = req.header(k, v);
        }
        match req.send().await {
            Ok(resp) => resp.json::<ApiResponse>().await.unwrap_or_else(|e| ApiResponse::error(e.to_string())),
            Err(e) => ApiResponse::error(Self::format_error(&e)),
        }
    }

    /// 写真をアップロード
    pub async fn upload_photo(
        &self,
        jpg_bytes: Vec<u8>,
        filename: String,
        world_id: Option<String>,
        instance_id: Option<String>,
        visibility: String,
        camera_data: Option<CameraData>,
        image_rotation: i32,
    ) -> ApiResponse {
        let part = match reqwest::multipart::Part::bytes(jpg_bytes)
            .file_name(filename)
            .mime_str("image/jpeg")
        {
            Ok(p) => p,
            Err(e) => return ApiResponse::error(e.to_string()),
        };

        let mut form = reqwest::multipart::Form::new()
            .part("image", part)
            .text("visibility", visibility)
            .text(
                "taken_at",
                chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            )
            .text("image_rotation", image_rotation.to_string());

        if let Some(wid) = world_id {
            form = form.text("world_id", wid);
        }
        if let Some(iid) = instance_id {
            form = form.text("instance_id", iid);
        }
        if let Some(cam) = camera_data {
            for (k, v) in cam {
                form = form.text(k, v.to_string());
            }
        }

        let mut req = self
            .client
            .post(format!("{}/vrc/api/photos/upload", self.base_url))
            .multipart(form);
        if let Some((k, v)) = self.auth_header() {
            req = req.header(k, v);
        }

        match req.send().await {
            Ok(resp) => resp.json::<ApiResponse>().await.unwrap_or_else(|e| ApiResponse::error(e.to_string())),
            Err(e) => ApiResponse::error(Self::format_error(&e)),
        }
    }

    /// インスタンス参加を報告
    pub async fn report_instance_join(
        &self,
        world_id: &str,
        instance_id: &str,
        vrc_user_id: Option<&str>,
        vrc_display_name: Option<&str>,
    ) -> ApiResponse {
        self.post_json(
            "/vrc/api/instance/join",
            serde_json::json!({
                "world_id": world_id,
                "instance_id": instance_id,
                "vrc_user_id": vrc_user_id,
                "vrc_display_name": vrc_display_name,
            }),
        )
        .await
    }

    /// インスタンス退出を報告
    pub async fn report_instance_leave(&self) -> ApiResponse {
        let mut req = self
            .client
            .post(format!("{}/vrc/api/instance/leave", self.base_url));
        if let Some((k, v)) = self.auth_header() {
            req = req.header(k, v);
        }
        match req.send().await {
            Ok(resp) => resp.json::<ApiResponse>().await.unwrap_or_else(|e| ApiResponse::error(e.to_string())),
            Err(e) => ApiResponse::error(Self::format_error(&e)),
        }
    }

    /// サーバー死活確認
    pub async fn health_check(&self) -> bool {
        let req = self
            .client
            .get(format!("{}/vrc/api/health", self.base_url))
            .timeout(std::time::Duration::from_secs(5));
        match req.send().await {
            Ok(resp) => resp.status().is_success(),
            Err(_) => false,
        }
    }
}
