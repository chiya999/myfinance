# 💰 myfinance — 终端个人财务管理器

Rust 课程项目：功能完整的命令行 + TUI 个人财务管理工具。

## 功能

- **账户管理**: 现金/储蓄卡/信用卡/投资账户，多币种余额追踪
- **收支记账**: 收入/支出/转账，全部金额以「分」存储（无浮点误差）
- **预算管理**: 分类预算 + 实时进度 + 超支告警
- **数据报表**: 收支总览 / 分类统计 / 月度趋势 / 账户快照
- **TUI 交互界面**: 键盘驱动，5 个标签页，实时数据
- **账单导入**: 支付宝 / 微信 CSV 账单自动识别和导入
- **数据导出**: CSV / JSON 格式导出

## 快速开始

```bash
# 编译
cargo build --release

# CLI 模式
cargo run -- account add "工资卡" --kind bank_card --initial-balance 10000
cargo run -- txn add 1 1 15000 income -d "工资"
cargo run -- account list
cargo run -- report summary --from 2025-01-01 --to 2025-12-31

# TUI 模式
cargo run -- tui
```

## 命令参考

```
myfinance tui                           # 启动 TUI
myfinance account list|add|show|edit|delete
myfinance txn list|add|transfer|delete
myfinance budget list|set|status|alert
myfinance report summary|category|trend|account-snapshot
myfinance import alipay|wechat|auto <file> [--account <id>]
```

## 技术栈

| 组件 | Crate |
|------|-------|
| 数据库 | `rusqlite` (bundled SQLite) |
| CLI | `clap` (derive) |
| TUI | `ratatui` + `crossterm` |
| 错误处理 | `thiserror` |
| 序列化 | `serde` + `serde_json` |
| 日期 | `chrono` |
| CSV | `csv` |
| 配置 | `toml` + `dirs` |
| 异步 | `tokio` |

## 项目结构

```
src/
  main.rs            # 入口
  error.rs           # 统一错误类型
  models/            # 领域模型 (Account, Transaction, Category, Budget)
  db/                # SQLite + Repository 层
  cli/               # clap CLI 子命令
  engine/            # 业务服务 (Account/Txn/Budget/Report)
  tui/               # ratatui 交互界面
  import/            # 账单导入适配器 (支付宝/微信)
  export/            # 数据导出 (CSV/JSON)
  config/            # 配置管理
tests/               # 集成测试
migrations/          # SQL 迁移
```

## 许可

MIT
