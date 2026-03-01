# NFU Class Chat Bot

一個以 Rust 撰寫的 Discord 班級驗證機器人，並支援將 LINE 訊息轉發到 Discord。

## 功能

- Discord 身分驗證流程（本班學生 / 學長姐 / 老師 / 訪客）
- 依 `config.toml` 進行角色綁定、學號姓名驗證
- 使用 SQLite（`db.db`）儲存綁定資料
- 內建 LINE Webhook（Axum）接收訊息並轉發至 Discord
- 支援 `config.toml` 熱重載

## 專案需求

- Rust 1.93+
- Discord Bot Token（環境變數 `DISCORD_TOKEN`）
- 時區可由環境變數 `TZ` 指定（預設 `Asia/Taipei`，即 UTC+8）
- `config.toml`（可由 `config.example.toml` 複製）

## 本機啟動

1. 建立設定檔

   ```bash
   cp config.example.toml config.toml
   ```

2. 建立 `.env`

   ```env
   DISCORD_TOKEN=your_discord_bot_token
   TZ=Asia/Taipei
   ```

3. 執行

   ```bash
   cargo run
   ```

## Docker 啟動

```bash
docker compose up -d --build
```

`docker-compose.yml` 會掛載：

- `./config.toml -> /app/config.toml`
- `./db.db -> /app/db.db`

## Kubernetes 部署

已提供範例檔：`k8s/manifest.yaml`

部署前請先修改：

- `Secret` 中的 `DISCORD_TOKEN`
- `ConfigMap` 中的 `config.toml` 內容（角色 ID、LINE 憑證、群組對應）

套用：

```bash
kubectl apply -f k8s/manifest.yaml
```

查看狀態：

```bash
kubectl get pods,svc -l app=class-chat-bot
```

## 設定重點

- Bot 啟動時會讀取 `config.toml`
- 啟動後會監聽檔案變更並熱更新
- LINE Webhook 入口預設為 `POST /line/webhook`

## 指令

- `!setup class_info`：發送身分設定按鈕（需管理權限）

## 專案結構

- `src/main.rs`：應用程式入口
- `src/handler.rs`：Discord 互動與身分綁定邏輯
- `src/link_chat/`：LINE -> Discord 轉發實作
- `src/app_config.rs`：設定讀取與熱重載
- `src/db.rs`：SQLite 初始化
- `k8s/manifest.yaml`：Kubernetes 範例部署
