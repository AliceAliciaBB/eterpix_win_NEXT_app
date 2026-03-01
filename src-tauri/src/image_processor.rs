// image_processor.rs - PNG→JPG変換 + VRChatカメラグリッドデコーダー

use image::{DynamicImage, GenericImageView, ImageReader};
use std::collections::HashMap;
use std::io::Cursor;
use std::path::Path;

/// カメラデータ
pub type CameraData = HashMap<String, f64>;

/// 画像処理結果
pub struct ProcessResult {
    pub jpg_bytes: Vec<u8>,
    pub camera_data: Option<CameraData>,
    pub is_portrait: bool,
}

/// PNG→JPG変換、カメラグリッドデコード
pub fn process_screenshot(path: &Path, jpeg_quality: u8) -> anyhow::Result<ProcessResult> {
    // カメラグリッドをデコード（変換前）
    let camera_data = decode_camera_grid(path);

    let img = ImageReader::open(path)?.decode()?;
    let is_portrait = img.width() < img.height();

    // RGBA→RGB変換、必要なら回転
    let img = img.to_rgb8();
    let img = DynamicImage::ImageRgb8(img);
    let img = if is_portrait {
        img.rotate270() // -90° = 反時計回り90°
    } else {
        img
    };

    // JPGエンコード
    let mut buf = Vec::new();
    let mut cursor = Cursor::new(&mut buf);
    img.write_to(&mut cursor, image::ImageFormat::Jpeg)?;

    // jpeg_qualityを適用するためにencoderを使う
    let img_rgb = img.to_rgb8();
    let mut jpg_buf = Vec::new();
    let mut encoder =
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpg_buf, jpeg_quality);
    encoder.encode_image(&img_rgb)?;

    Ok(ProcessResult {
        jpg_bytes: jpg_buf,
        camera_data,
        is_portrait,
    })
}

// ============================================================
// VRChatカメラグリッドデコーダー
// グリッド: 67列(左マーカー1+データ65+右マーカー1) x 9行
// 各行: 1符号ビット + 32整数ビット + 32小数ビット
// 小数部: int / 10^9
// ============================================================

/// グリッド座標の固定値（画像下端からのオフセット）
const GRID_BL_X: i32 = 2;
const GRID_BL_Y_OFFSET: i32 = 3;
const GRID_TR_X: i32 = 135;
const GRID_TR_Y_OFFSET: i32 = 20;
const COLS: usize = 67;
const ROWS: usize = 9;
const PRECISION: f64 = 1_000_000_000.0; // 10^9

/// マーカーパターン: (左が白, 右が白)
const MARKER_PATTERNS: [(bool, bool); ROWS] = [
    (true, false),
    (false, true),
    (true, false),
    (false, true),
    (true, false),
    (false, true),
    (true, false),
    (false, true),
    (true, false),
];

/// 行→キー名マッピング
const ROW_KEYS: [&str; ROWS] = [
    "euler_z_yaw",
    "euler_y_pitch",
    "euler_x_roll",
    "camera_pos_z",
    "camera_pos_y",
    "camera_pos_x",
    "object_pos_z",
    "object_pos_y",
    "object_pos_x",
];

fn decode_camera_grid(path: &Path) -> Option<CameraData> {
    let img = ImageReader::open(path).ok()?.decode().ok()?;
    let (orig_w, orig_h) = img.dimensions();

    // 縦画像は時計回り90°回転してデコード
    let img = if orig_h > orig_w {
        img.rotate90()
    } else {
        img
    };

    let (w, h) = img.dimensions();
    let rgb = img.to_rgb8();

    let bl_x = GRID_BL_X;
    let bl_y = h as i32 - GRID_BL_Y_OFFSET;
    let tr_x = GRID_TR_X;
    let tr_y = h as i32 - GRID_TR_Y_OFFSET;

    for transform in &["none", "rotate180", "flip_h", "flip_v"] {
        if let Some(result) = try_decode(&rgb, w, h, bl_x, bl_y, tr_x, tr_y, transform) {
            return Some(result);
        }
    }
    None
}

fn transform_coord(x: i32, y: i32, w: i32, h: i32, transform: &str) -> (i32, i32) {
    match transform {
        "rotate180" => (w - 1 - x, h - 1 - y),
        "flip_h" => (w - 1 - x, y),
        "flip_v" => (x, h - 1 - y),
        _ => (x, y),
    }
}

fn is_white(rgb: &image::RgbImage, x: i32, y: i32, w: u32, h: u32) -> bool {
    if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 {
        return false;
    }
    let pixel = rgb.get_pixel(x as u32, y as u32);
    let brightness = (pixel[0] as u32 + pixel[1] as u32 + pixel[2] as u32) / 3;
    brightness > 127
}

fn try_decode(
    rgb: &image::RgbImage,
    w: u32,
    h: u32,
    bl_x: i32,
    bl_y: i32,
    tr_x: i32,
    tr_y: i32,
    transform: &str,
) -> Option<CameraData> {
    let (tbl_x, tbl_y) = transform_coord(bl_x, bl_y, w as i32, h as i32, transform);
    let (ttr_x, ttr_y) = transform_coord(tr_x, tr_y, w as i32, h as i32, transform);

    let spacing_x = (ttr_x - tbl_x) as f64 / (COLS - 1) as f64;
    let spacing_y = (ttr_y - tbl_y) as f64 / (ROWS - 1) as f64;

    // ドット座標を事前計算
    let mut dots = Vec::with_capacity(ROWS * COLS);
    for row in 0..ROWS {
        for col in 0..COLS {
            let dx = (tbl_x as f64 + col as f64 * spacing_x).round() as i32;
            let dy = (tbl_y as f64 + row as f64 * spacing_y).round() as i32;
            dots.push((dx, dy));
        }
    }

    let mut result = CameraData::new();

    for (row_idx, key) in ROW_KEYS.iter().enumerate() {
        let base = row_idx * COLS;
        let (exp_left, exp_right) = MARKER_PATTERNS[row_idx];

        // マーカー検証
        let left_white = is_white(rgb, dots[base].0, dots[base].1, w, h);
        let right_white = is_white(rgb, dots[base + COLS - 1].0, dots[base + COLS - 1].1, w, h);
        if left_white != exp_left || right_white != exp_right {
            return None;
        }

        // 符号ビット (列1)
        let sign: f64 = if is_white(rgb, dots[base + 1].0, dots[base + 1].1, w, h) {
            1.0
        } else {
            -1.0
        };

        // 整数部 (列2〜33: 32ビット)
        let mut integer_part: u64 = 0;
        for col in 2..34usize {
            integer_part <<= 1;
            if is_white(rgb, dots[base + col].0, dots[base + col].1, w, h) {
                integer_part |= 1;
            }
        }

        // 小数部 (列34〜65: 32ビット)
        let mut frac_bits: u64 = 0;
        for col in 34..66usize {
            frac_bits <<= 1;
            if is_white(rgb, dots[base + col].0, dots[base + col].1, w, h) {
                frac_bits |= 1;
            }
        }
        let frac_part = frac_bits as f64 / PRECISION;

        let value = sign * (integer_part as f64 + frac_part);
        result.insert(key.to_string(), value);
    }

    Some(result)
}
