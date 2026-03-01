// offline_queue.rs - オフライン時のデータキュー管理 (SQLite)

use crate::image_processor::CameraData;
use rusqlite::{params, Connection, Result as SqlResult};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedPhoto {
    pub id: String,
    pub filename: String,
    pub world_id: Option<String>,
    pub instance_id: Option<String>,
    pub visibility: String,
    pub taken_at: String,
    pub camera_data: Option<String>, // JSON文字列
    pub image_rotation: i32,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueuedWorldJoin {
    pub id: String,
    pub world_id: String,
    pub instance_id: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Default)]
pub struct QueueCounts {
    pub photos: usize,
    pub worlds: usize,
}

pub struct OfflineQueueManager {
    db_path: PathBuf,
    images_dir: PathBuf,
}

impl OfflineQueueManager {
    pub fn new(base_dir: PathBuf) -> Self {
        let images_dir = base_dir.join("images");
        std::fs::create_dir_all(&images_dir).ok();
        let db_path = base_dir.join("queue.db");
        let mgr = Self { db_path, images_dir };
        mgr.init_db().ok();
        mgr
    }

    fn open(&self) -> SqlResult<Connection> {
        let conn = Connection::open(&self.db_path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL;")?;
        Ok(conn)
    }

    fn init_db(&self) -> SqlResult<()> {
        let conn = self.open()?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS queued_photos (
                id TEXT PRIMARY KEY,
                filename TEXT NOT NULL,
                world_id TEXT,
                instance_id TEXT,
                visibility TEXT NOT NULL DEFAULT 'self',
                taken_at TEXT NOT NULL,
                camera_data TEXT,
                image_rotation INTEGER NOT NULL DEFAULT 0,
                created_at TEXT NOT NULL
            );
            CREATE TABLE IF NOT EXISTS queued_world_joins (
                id TEXT PRIMARY KEY,
                world_id TEXT NOT NULL,
                instance_id TEXT NOT NULL,
                created_at TEXT NOT NULL
            );",
        )
    }

    // ============================================================
    // 写真キュー
    // ============================================================

    pub fn queue_photo(
        &self,
        jpg_bytes: &[u8],
        filename: &str,
        world_id: Option<&str>,
        instance_id: Option<&str>,
        visibility: &str,
        camera_data: Option<&CameraData>,
        image_rotation: i32,
    ) -> anyhow::Result<String> {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();

        // 画像を保存
        let img_path = self.images_dir.join(format!("{}.jpg", id));
        std::fs::write(&img_path, jpg_bytes)?;

        let cam_json = camera_data
            .map(|c| serde_json::to_string(c).unwrap_or_default());

        let conn = self.open()?;
        conn.execute(
            "INSERT INTO queued_photos
             (id, filename, world_id, instance_id, visibility, taken_at, camera_data, image_rotation, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                id,
                filename,
                world_id,
                instance_id,
                visibility,
                now,
                cam_json,
                image_rotation,
                now,
            ],
        )?;
        Ok(id)
    }

    pub fn get_queued_photos(&self) -> Vec<(QueuedPhoto, Vec<u8>)> {
        let conn = match self.open() {
            Ok(c) => c,
            Err(_) => return vec![],
        };
        let mut stmt = match conn.prepare(
            "SELECT id, filename, world_id, instance_id, visibility, taken_at,
                    camera_data, image_rotation, created_at FROM queued_photos ORDER BY created_at",
        ) {
            Ok(s) => s,
            Err(_) => return vec![],
        };

        let mut results = Vec::new();
        let rows = stmt.query_map([], |row| {
            Ok(QueuedPhoto {
                id: row.get(0)?,
                filename: row.get(1)?,
                world_id: row.get(2)?,
                instance_id: row.get(3)?,
                visibility: row.get(4)?,
                taken_at: row.get(5)?,
                camera_data: row.get(6)?,
                image_rotation: row.get(7)?,
                created_at: row.get(8)?,
            })
        });

        if let Ok(rows) = rows {
            for photo in rows.flatten() {
                let img_path = self.images_dir.join(format!("{}.jpg", photo.id));
                if let Ok(bytes) = std::fs::read(&img_path) {
                    results.push((photo, bytes));
                }
            }
        }
        results
    }

    pub fn remove_photo(&self, id: &str) -> bool {
        let img_path = self.images_dir.join(format!("{}.jpg", id));
        let _ = std::fs::remove_file(img_path);
        let conn = match self.open() {
            Ok(c) => c,
            Err(_) => return false,
        };
        conn.execute("DELETE FROM queued_photos WHERE id = ?1", params![id])
            .is_ok()
    }

    // ============================================================
    // ワールド参加キュー
    // ============================================================

    pub fn queue_world_join(
        &self,
        world_id: &str,
        instance_id: &str,
    ) -> anyhow::Result<String> {
        let id = Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let conn = self.open()?;
        conn.execute(
            "INSERT INTO queued_world_joins (id, world_id, instance_id, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![id, world_id, instance_id, now],
        )?;
        Ok(id)
    }

    pub fn get_queued_world_joins(&self) -> Vec<QueuedWorldJoin> {
        let conn = match self.open() {
            Ok(c) => c,
            Err(_) => return vec![],
        };
        let mut stmt = match conn
            .prepare("SELECT id, world_id, instance_id, created_at FROM queued_world_joins ORDER BY created_at")
        {
            Ok(s) => s,
            Err(_) => return vec![],
        };
        stmt.query_map([], |row| {
            Ok(QueuedWorldJoin {
                id: row.get(0)?,
                world_id: row.get(1)?,
                instance_id: row.get(2)?,
                created_at: row.get(3)?,
            })
        })
        .ok()
        .map(|rows| rows.flatten().collect())
        .unwrap_or_default()
    }

    pub fn remove_world_join(&self, id: &str) -> bool {
        let conn = match self.open() {
            Ok(c) => c,
            Err(_) => return false,
        };
        conn.execute("DELETE FROM queued_world_joins WHERE id = ?1", params![id])
            .is_ok()
    }

    // ============================================================
    // ユーティリティ
    // ============================================================

    pub fn get_queue_counts(&self) -> QueueCounts {
        let conn = match self.open() {
            Ok(c) => c,
            Err(_) => return QueueCounts::default(),
        };
        let photos: usize = conn
            .query_row("SELECT COUNT(*) FROM queued_photos", [], |r| r.get(0))
            .unwrap_or(0);
        let worlds: usize = conn
            .query_row("SELECT COUNT(*) FROM queued_world_joins", [], |r| r.get(0))
            .unwrap_or(0);
        QueueCounts { photos, worlds }
    }

    pub fn has_pending_data(&self) -> bool {
        let counts = self.get_queue_counts();
        counts.photos > 0 || counts.worlds > 0
    }
}
