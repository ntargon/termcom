# 詳細設計書 - 組み込み機器開発用通信デバッグツール (TermCom)

## 1. アーキテクチャ概要

### 1.1 システム構成図

```
┌─────────────────────────────────────────────────────────────┐
│                      TermCom Application                    │
├─────────────────────────────────────────────────────────────┤
│                    Presentation Layer                       │
│  ┌─────────────────┐           ┌─────────────────┐         │
│  │   TUI Module    │           │   CLI Module    │         │
│  │   (ratatui)     │           │  (clap/args)    │         │
│  └─────────────────┘           └─────────────────┘         │
├─────────────────────────────────────────────────────────────┤
│                     Application Layer                       │
│  ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐│
│  │ Session Manager │ │ Command Manager │ │ Logger Manager  ││
│  └─────────────────┘ └─────────────────┘ └─────────────────┘│
├─────────────────────────────────────────────────────────────┤
│                     Domain Layer                            │
│  ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐│
│  │ Communication   │ │ Data Processor  │ │ Configuration   ││
│  │ Engine          │ │                 │ │ Manager         ││
│  └─────────────────┘ └─────────────────┘ └─────────────────┘│
├─────────────────────────────────────────────────────────────┤
│                   Infrastructure Layer                      │
│  ┌─────────────────┐           ┌─────────────────┐         │
│  │ Serial Port     │           │   TCP Socket    │         │
│  │ (serialport)    │           │  (std::net)     │         │
│  └─────────────────┘           └─────────────────┘         │
└─────────────────────────────────────────────────────────────┘
```

### 1.2 技術スタック

- 言語: Rust 1.70+
- フレームワーク: 
  - TUI: ratatui 0.23+
  - CLI: clap 4.0+
- ライブラリ: 
  - シリアル通信: serialport 4.0+
  - 非同期ランタイム: tokio 1.0+
  - 設定管理: serde + toml/serde_yaml
  - ログ: tracing + tracing-subscriber
  - エラーハンドリング: anyhow + thiserror
- ツール: 
  - ビルド: Cargo
  - テスト: cargo test + proptest
  - フォーマット: rustfmt
  - Lint: clippy

## 2. コンポーネント設計

### 2.1 コンポーネント一覧

| コンポーネント名 | 責務         | 依存関係                 |
| ---------------- | ------------ | ------------------------ |
| App Controller   | アプリケーション全体の制御 | SessionManager, UIManager |
| Session Manager  | 通信セッションの管理 | CommunicationEngine, DataProcessor |
| Communication Engine | 通信プロトコルの抽象化 | SerialPort, TcpSocket |
| Data Processor   | データの変換・フォーマット | なし |
| Command Manager  | コマンドの管理・実行 | ConfigurationManager |
| Logger Manager   | ログ出力・管理 | DataProcessor |
| TUI Manager      | TUIインターフェース | SessionManager, CommandManager |
| CLI Manager      | CLIインターフェース | SessionManager, CommandManager |
| Configuration Manager | 設定ファイル管理 | なし |

### 2.2 各コンポーネントの詳細

#### App Controller

- **目的**: アプリケーション全体のライフサイクル管理とモード切り替え
- **公開インターフェース**:
  ```rust
  pub struct AppController {
      session_manager: SessionManager,
      ui_manager: Box<dyn UIManager>,
      config: Configuration,
  }
  
  impl AppController {
      pub fn new(mode: AppMode, config: Configuration) -> Result<Self>;
      pub async fn run(&mut self) -> Result<()>;
      pub fn shutdown(&mut self) -> Result<()>;
  }
  ```
- **内部実装方針**: 
  - モード（TUI/CLI）に応じたUIManagerの選択
  - グレースフルシャットダウンの実装
  - グローバル設定の管理

#### Session Manager

- **目的**: 複数の通信セッションの並行管理
- **公開インターフェース**:
  ```rust
  #[derive(Clone)]
  pub struct SessionManager {
      sessions: Arc<RwLock<HashMap<SessionId, Session>>>,
      event_sender: mpsc::Sender<SessionEvent>,
  }
  
  impl SessionManager {
      pub async fn create_session(&self, config: SessionConfig) -> Result<SessionId>;
      pub async fn close_session(&self, id: SessionId) -> Result<()>;
      pub async fn send_data(&self, id: SessionId, data: Vec<u8>) -> Result<()>;
      pub fn subscribe_events(&self) -> mpsc::Receiver<SessionEvent>;
  }
  ```
- **内部実装方針**: 
  - 非同期処理によるセッション管理
  - イベント駆動アーキテクチャ
  - スレッドセーフなセッション状態管理

#### Communication Engine

- **目的**: シリアル・TCP通信の統一インターフェース提供
- **公開インターフェース**:
  ```rust
  #[async_trait]
  pub trait CommunicationEngine: Send + Sync {
      async fn connect(&mut self, config: &ConnectionConfig) -> Result<()>;
      async fn disconnect(&mut self) -> Result<()>;
      async fn send(&mut self, data: &[u8]) -> Result<usize>;
      async fn receive(&mut self) -> Result<Vec<u8>>;
      fn is_connected(&self) -> bool;
  }
  
  pub struct SerialEngine { /* implementation */ }
  pub struct TcpEngine { /* implementation */ }
  ```
- **内部実装方針**: 
  - トレイトによる通信プロトコルの抽象化
  - 非同期I/Oによる高効率通信
  - 接続状態の監視と自動再接続

#### Data Processor

- **目的**: 送受信データの変換・フォーマット・解析
- **公開インターフェース**:
  ```rust
  pub struct DataProcessor;
  
  impl DataProcessor {
      pub fn format_display(data: &[u8], format: DisplayFormat) -> String;
      pub fn parse_input(input: &str, format: InputFormat) -> Result<Vec<u8>>;
      pub fn add_timestamp(data: &str) -> String;
      pub fn mask_sensitive_data(data: &str, patterns: &[String]) -> String;
  }
  
  #[derive(Clone, Copy)]
  pub enum DisplayFormat {
      Hex, Ascii, Binary, Mixed
  }
  ```
- **内部実装方針**: 
  - 関数型プログラミングアプローチ
  - ゼロコピー最適化
  - 設定可能なフォーマットオプション

## 3. データフロー

### 3.1 データフロー図

```
[User Input] → [CLI/TUI] → [Command Manager] → [Session Manager]
                    ↓
[Communication Engine] → [Data Processor] → [Logger Manager]
                    ↓                              ↓
[Serial/TCP Device] ← [Session Manager] ← [Display/File Output]
                    ↓
[TUI/CLI Display] ← [Data Processor] ← [Received Data]
```

### 3.2 データ変換

- **入力データ形式**: 
  - ユーザー入力: UTF-8テキスト
  - 設定ファイル: TOML/YAML
  - 機器からの受信: バイト配列
- **処理過程**: 
  - 入力パース → バイト配列変換 → 通信エンジン送信
  - 受信データ → フォーマット変換 → 表示・ログ出力
- **出力データ形式**: 
  - 画面表示: フォーマット済みテキスト
  - ログファイル: タイムスタンプ付きテキスト

## 4. APIインターフェース

### 4.1 内部API

```rust
// セッション管理API
pub trait SessionAPI {
    async fn create_session(&self, config: SessionConfig) -> Result<SessionId>;
    async fn send_command(&self, session_id: SessionId, command: Command) -> Result<()>;
    fn get_session_status(&self, session_id: SessionId) -> Option<SessionStatus>;
}

// 設定管理API
pub trait ConfigAPI {
    fn load_config(&self, path: &Path) -> Result<Configuration>;
    fn save_config(&self, config: &Configuration, path: &Path) -> Result<()>;
    fn get_default_config(&self) -> Configuration;
}

// ログ管理API
pub trait LogAPI {
    fn log_data(&self, session_id: SessionId, direction: Direction, data: &[u8]);
    fn set_log_level(&self, level: LogLevel);
    fn rotate_log_file(&self) -> Result<()>;
}
```

### 4.2 外部API

```rust
// CLI用コマンドライン引数
#[derive(Parser)]
pub struct CliArgs {
    #[arg(short, long)]
    pub config: Option<PathBuf>,
    
    #[arg(short, long)]
    pub mode: Option<String>,
    
    #[command(subcommand)]
    pub command: Option<Commands>,
}

// TUI用イベントハンドリング
pub enum TUIEvent {
    KeyPress(KeyEvent),
    Resize(u16, u16),
    SessionUpdate(SessionId, SessionStatus),
    DataReceived(SessionId, Vec<u8>),
}
```

## 5. エラーハンドリング

### 5.1 エラー分類

- **通信エラー**: 接続失敗、タイムアウト、プロトコルエラー
  - 対処方法: 自動再試行、ユーザー通知、ログ記録
- **設定エラー**: 不正な設定値、ファイル読み込み失敗
  - 対処方法: デフォルト値使用、バリデーション、ユーザー警告
- **UIエラー**: 画面描画失敗、入力処理エラー
  - 対処方法: グレースフルデグレード、エラー画面表示
- **システムエラー**: メモリ不足、ファイルI/Oエラー
  - 対処方法: 緊急シャットダウン、リソース解放

### 5.2 エラー通知

```rust
#[derive(thiserror::Error, Debug)]
pub enum TermComError {
    #[error("Communication error: {0}")]
    Communication(#[from] std::io::Error),
    
    #[error("Configuration error: {message}")]
    Configuration { message: String },
    
    #[error("Session error: {session_id} - {message}")]
    Session { session_id: SessionId, message: String },
}

// ログレベル別のエラー処理
pub fn handle_error(error: &TermComError) {
    match error {
        TermComError::Communication(_) => {
            tracing::warn!("Communication error occurred: {}", error);
            // UI notification
        },
        TermComError::Configuration { .. } => {
            tracing::error!("Configuration error: {}", error);
            // Show error dialog
        },
        TermComError::Session { .. } => {
            tracing::info!("Session error: {}", error);
            // Update session status
        }
    }
}
```

## 6. セキュリティ設計

### 6.1 認証・認可

- 設定ファイルへのアクセス権限チェック
- 通信ログの機密情報自動検出・マスキング
- 安全でない通信プロトコルの警告表示

### 6.2 データ保護

```rust
pub struct SecurityManager {
    sensitive_patterns: Vec<regex::Regex>,
    encryption_key: Option<[u8; 32]>,
}

impl SecurityManager {
    pub fn mask_sensitive_data(&self, data: &str) -> String;
    pub fn encrypt_config_file(&self, config: &Configuration) -> Result<Vec<u8>>;
    pub fn validate_connection_security(&self, config: &ConnectionConfig) -> SecurityLevel;
}
```

## 7. テスト戦略

### 7.1 単体テスト

- **カバレッジ目標**: 80%以上
- **テストフレームワーク**: 
  - `cargo test` (標準)
  - `proptest` (プロパティベーステスト)
  - `tokio-test` (非同期テスト)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    
    #[tokio::test]
    async fn test_session_creation() {
        let manager = SessionManager::new();
        let config = SessionConfig::test_config();
        let session_id = manager.create_session(config).await.unwrap();
        assert!(manager.get_session(session_id).is_some());
    }
    
    proptest! {
        #[test]
        fn test_data_processor_formats(data in any::<Vec<u8>>()) {
            let formatted = DataProcessor::format_display(&data, DisplayFormat::Hex);
            assert!(formatted.chars().all(|c| c.is_ascii_hexdigit() || c.is_whitespace()));
        }
    }
}
```

### 7.2 統合テスト

```rust
// モックデバイスを使用した統合テスト
#[tokio::test]
async fn integration_test_serial_communication() {
    let mock_device = MockSerialDevice::new();
    let session_manager = SessionManager::new();
    
    // セッション作成
    let session_id = session_manager
        .create_session(SessionConfig::serial_mock(mock_device.port()))
        .await
        .unwrap();
    
    // データ送信テスト
    let test_data = b"AT+COMMAND\r\n";
    session_manager.send_data(session_id, test_data.to_vec()).await.unwrap();
    
    // 応答確認
    let response = mock_device.wait_for_response().await;
    assert_eq!(response, test_data);
}
```

## 8. パフォーマンス最適化

### 8.1 想定される負荷

- **同時セッション数**: 最大10セッション
- **データスループット**: セッションあたり1MB/s
- **応答時間**: コマンド送信から表示まで100ms以内
- **メモリ使用量**: 通常動作時100MB以下

### 8.2 最適化方針

```rust
// リングバッファによる効率的なデータ管理
pub struct RingBuffer<T> {
    buffer: Vec<T>,
    head: usize,
    tail: usize,
    capacity: usize,
}

// 非同期処理によるI/O最適化
pub struct OptimizedCommunication {
    read_buffer: Arc<Mutex<VecDeque<u8>>>,
    write_queue: Arc<Mutex<VecDeque<Vec<u8>>>>,
}

impl OptimizedCommunication {
    pub async fn batch_write(&mut self) -> Result<()> {
        // バッチ処理による書き込み効率化
    }
    
    pub async fn buffered_read(&mut self) -> Result<Vec<u8>> {
        // バッファリングによる読み込み効率化
    }
}
```

## 9. デプロイメント

### 9.1 デプロイ構成

```toml
# Cargo.toml - リリース最適化
[profile.release]
opt-level = 3
lto = true
codegen-units = 1
panic = "abort"

# クロスコンパイル対応
[target.x86_64-pc-windows-gnu]
linker = "x86_64-w64-mingw32-gcc"

[target.x86_64-apple-darwin]
rustflags = ["-C", "link-arg=-undefined", "-C", "link-arg=dynamic_lookup"]
```

### 9.2 設定管理

```rust
// 階層設定管理
#[derive(Deserialize, Serialize)]
pub struct Configuration {
    pub app: AppConfig,
    pub ui: UIConfig,
    pub communication: CommunicationConfig,
    pub logging: LoggingConfig,
    pub security: SecurityConfig,
}

// 環境変数による設定オーバーライド
pub fn load_configuration() -> Result<Configuration> {
    let mut config = Configuration::default();
    
    // 設定ファイル読み込み
    if let Ok(file_config) = load_from_file("termcom.toml") {
        config.merge(file_config);
    }
    
    // 環境変数オーバーライド
    config.apply_env_overrides();
    
    Ok(config)
}
```

## 10. 実装上の注意事項

### 10.1 非同期処理
- tokioランタイムを使用した効率的な非同期I/O
- デッドロック防止のためのlockガード順序統一
- チャネルによるタスク間通信の活用

### 10.2 エラーハンドリング
- `Result<T, E>`型による明示的エラー処理
- `?`演算子を使用した簡潔なエラー伝播
- カスタムエラー型による詳細なエラー情報提供

### 10.3 メモリ管理
- Arc<RwLock<T>>による安全な並行アクセス
- 大きなデータのストリーミング処理
- メモリリークを防ぐためのDrop trait実装

### 10.4 クロスプラットフォーム対応
- 条件コンパイルによるOS固有機能の分離
- pathの区切り文字統一
- 改行コードの適切な処理

### 10.5 テスタビリティ
- 依存性注入によるモック可能な設計
- トレイトによる抽象化とテスト容易性向上
- プロパティベーステストによる堅牢性確保

### 10.6 保守性
- モジュール分割による責務の明確化
- 包括的なドキュメンテーション
- コード品質チェックの自動化（clippy, rustfmt）