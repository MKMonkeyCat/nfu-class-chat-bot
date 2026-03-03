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

- `./config -> /app/config`
- `./data -> /app/data`

其中 SQLite 會儲存到 `data/db.db`。

## Helm 部署

本專案已提供 Helm chart：`charts/class-chat-bot`。

```bash
helm upgrade --install class-chat-bot ./charts/class-chat-bot
```

預設設定：

- Service 對外 Port：`8080`（targetPort: `8080`）
- `config` 掛載到 `/app/config`
- `data` 掛載到 `/app/data`

若要指定既有 PVC，可在 `values.yaml` 設定：

- `persistence.config.existingClaim`
- `persistence.data.existingClaim`

## 設定重點

- Bot 啟動時會讀取 `config.toml`
- 啟動後會監聽檔案變更並熱更新
- LINE Webhook 入口預設為 `POST /line/webhook`
- 可在 `config/crawlers.toml` 的 `[google_calendar]` 啟用 Google Calendar（服務帳戶）同步

## Google Calendar（服務帳戶）

1. 在 Google Cloud 建立 Service Account 並啟用 Calendar API。
2. 下載 Service Account JSON，放到 `config/google-service-account.json`（或自訂路徑）。
3. 將私人行事曆「分享給服務帳戶 email」（至少讀取權限）。
4. 在 `config/crawlers.toml` 設定：
   - `[google_calendar].enabled = true`
   - `service_account_json_path`
   - `[[google_calendar.calendars]]` 的 `calendar_id` 與 `enabled = true`

啟用後，Bot 會定期抓取未來活動並推播到通知目標，且同一活動只會推播一次。

## 指令

- `!setup class_info`：發送身分設定按鈕（需管理權限）

## 專案結構

- `src/main.rs`：應用程式入口
- `src/handler.rs`：Discord 互動與身分綁定邏輯
- `src/link_chat/`：LINE -> Discord 轉發實作
- `src/app_config.rs`：設定讀取與熱重載
- `src/db.rs`：SQLite 初始化
- `k8s/manifest.yaml`：Kubernetes 範例部署
