# CLAUDE.md - TermCom Project Context

## プロジェクト概要

**TermCom** - 組み込み機器開発用通信デバッグツール

シリアル通信とTCP通信による組み込み機器とのコマンドやり取りのためのツール。TUIとCLIの両方のインターフェースを持ち、テストがしやすい構成を重視。

## 技術スタック

- **言語**: Rust 1.70+
- **非同期ランタイム**: tokio
- **TUI**: ratatui
- **CLI**: clap
- **シリアル通信**: serialport
- **TCP通信**: tokio::net
- **ログ**: tracing
- **設定管理**: serde + toml
- **テスト**: cargo test + proptest

## プロジェクト構造

```
src/
├── main.rs                 # エントリーポイント
├── cli/                    # CLIインターフェース
├── tui/                    # TUIインターフェース
├── core/                   # コアロジック
│   ├── communication/      # 通信エンジン
│   ├── session/           # セッション管理
│   └── config/            # 設定管理
├── domain/                # ドメインモデル
└── infrastructure/        # インフラストラクチャ
    ├── serial/            # シリアル通信
    ├── tcp/               # TCP通信
    └── logging/           # ログ機能
```

## 開発フロー

### テストコマンド
```bash
cargo test
cargo test --all-features
cargo test --doc
```

### リントとフォーマット
```bash
cargo fmt
cargo clippy
cargo clippy -- -D warnings
```

### ビルド
```bash
cargo build
cargo build --release
```

### 実行
```bash
# CLI モード
cargo run -- --help
cargo run -- serial --port /dev/ttyUSB0 --baud 9600

# TUI モード
cargo run -- tui
```

## 主要機能

### 通信機能
- シリアル通信 (RS232, RS485, UART)
- TCP通信 (クライアント/サーバー)
- 複数セッション同時管理 (最大10セッション)
- リアルタイムデータ監視

### インターフェース
- **CLI**: 自動化とスクリプト実行向け
- **TUI**: インタラクティブな開発・デバッグ向け

### データ管理
- 通信ログの保存・検索
- セッション設定の保存・復元
- カスタムコマンドテンプレート

## 設定ファイル

### グローバル設定
- 場所: `~/.config/termcom/config.toml`
- 内容: デフォルト設定、ログレベル、セキュリティ設定

### プロジェクト設定
- 場所: `.termcom/config.toml`
- 内容: プロジェクト固有の設定、デバイス定義

## セキュリティ

- 機密情報の自動マスキング
- 設定ファイルの暗号化オプション
- 安全でない通信の警告表示

## パフォーマンス目標

- 応答時間: 100ms以内
- 同時セッション: 最大10個
- メモリ使用量: 50MB以下
- CPU使用率: 10%以下（アイドル時）

## 開発段階

現在: **仕様策定完了** - 実装準備完了

### Phase 1: 基盤構築 (1-2週)
- プロジェクト構造
- 基本的なエラーハンドリング
- ログシステム

### Phase 2: 通信エンジン (2-3週)
- シリアル通信実装
- TCP通信実装
- 統一通信API

### Phase 3: セッション管理 (1-2週)
- 複数セッション管理
- 設定管理システム

### Phase 4: UI実装 (2-3週)
- CLIインターフェース
- TUIインターフェース

## 参考資料

- 要求仕様: `.tmp/requirements.md`
- 技術設計: `.tmp/design.md`
- 実装タスク: `.tmp/tasks.md`

## 注意事項

- 非同期処理を活用したパフォーマンス重視
- エラーハンドリングの統一 (thiserror/anyhow)
- テストカバレッジ80%以上を目標
- クロスプラットフォーム対応 (Windows/Linux/macOS)