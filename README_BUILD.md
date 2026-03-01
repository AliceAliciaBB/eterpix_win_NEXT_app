# EterPix VRC Uploader - Tauri + Vue ビルドガイド

## 必要なもの

| ツール | バージョン | 備考 |
|--------|-----------|------|
| Node.js | 18以上 | https://nodejs.org |
| Rust | stable | https://rustup.rs |
| Tauri CLI | v2 | npm install で自動取得 |
| Visual Studio Build Tools | 2019+ | Windows のみ必須 |

## セットアップ

```bash
# 1. 新コード/ フォルダに移動
cd 新コード

# 2. Node 依存関係をインストール
npm install

# 3. アイコンを確認 (すでにコピー済み)
ls src-tauri/icons/
```

## 開発サーバー起動

```bash
npm run tauri dev
```

## exe ビルド

```bash
npm run tauri build
```

ビルド成功後、以下にexeが生成されます：
```
src-tauri/target/release/bundle/msi/   ← Windows インストーラー (.msi)
src-tauri/target/release/bundle/nsis/  ← NSIS インストーラー
src-tauri/target/release/eterpix-vrc-uploader.exe  ← 単体exe
```

## アーキテクチャ概要

```
新コード/
├── src/                     # Vue 3 フロントエンド
│   ├── main.ts              # エントリポイント
│   ├── App.vue              # ルートコンポーネント + トレイ連携
│   ├── style.css            # グローバルCSS (Light/Dark対応)
│   ├── stores/
│   │   └── appStore.ts      # Pinia ストア (Tauri IPC呼び出し)
│   └── components/
│       ├── LoginPage.vue    # ログイン/新規登録画面
│       ├── HomePage.vue     # ホーム (監視・状態・履歴)
│       └── SettingsPage.vue # 設定 (OSC・スタートアップ等)
│
└── src-tauri/               # Rust バックエンド
    ├── Cargo.toml
    ├── tauri.conf.json
    └── src/
        ├── main.rs          # エントリポイント
        ├── lib.rs           # Tauri コマンド・AppState・バックグラウンドタスク
        ├── config.rs        # JSON設定ファイル管理
        ├── startup.rs       # Windows レジストリ スタートアップ登録
        ├── log_parser.rs    # VRChatログ解析 (ワールド参加/退出)
        ├── watcher.rs       # ファイル監視 (notify crate)
        ├── image_processor.rs  # PNG→JPG変換 + カメラグリッドデコーダー
        ├── uploader.rs      # HTTP通信クライアント (reqwest)
        ├── offline_queue.rs # オフラインキュー (SQLite)
        └── osc_handler.rs   # OSC通信 (VRC公開範囲変更)
```

## Python版からの移行対応表

| Python (旧) | Rust (新) |
|-------------|-----------|
| Flask + SSE | Tauri イベント (`app.emit()`) |
| pystray | Tauri built-in tray |
| PyInstaller | `tauri build` |
| watchdog | notify crate |
| Pillow | image crate |
| python-osc | rosc crate |
| httpx | reqwest crate |
| CSV offline queue | SQLite (rusqlite) |
| winreg (Python) | winreg crate |

## 設定ファイルの場所

```
%APPDATA%\EterPixUploader\config.json    ← アプリ設定
%APPDATA%\EterPixUploader\queue\         ← オフラインキュー (SQLite + 画像)
```
