# 💰 myfinance — 终端个人财务管理器

一个基于 Rust 的命令行 + TUI 个人财务管理工具。把微信、支付宝等多平台账单导入同一个本地数据库，统一聚合分析、按商户分类、追踪预算——所有数据存在本地 SQLite 文件，零网络依赖、零隐私风险。

> 《Rust语言程序设计》课程项目 · 西安交通大学 · 钟皓天

## ✨ 功能特性

- **账户管理** — 钱包 / 储蓄卡 / 信用卡 / 投资四类账户，多币种，可停用
- **收支记账** — 收入 / 支出 / 转账三类交易，全部金额以 i64 整数「分」存储，从源头消除浮点误差
- **事务性引擎** — 记账 INSERT + 余额 UPDATE 在 SQLite 事务中原子提交；删除交易自动冲正余额；转账含余额校验
- **预算追踪** — 总预算 + 各商户预算，实时进度条，颜色渐变（绿 < 70% → 黄 < 90% → 红 ≥ 90%），超支告警
- **数据报表** — 收支总览 / 分类统计 / 月度趋势（daily/weekly/monthly 三种粒度）/ 账户快照
- **账单导入** — 支付宝 / 微信 CSV 自动识别，`import auto` 自动嗅探格式，交易对方自动成为分类
- **TUI 交互界面** — ratatui 5 标签页，键盘驱动，vim 风格快捷键，时间筛选 + 账户过滤
- **数据导出** — CSV / JSON 格式，CSV 含 BOM 头确保 Windows Excel 中文兼容
- **配置管理** — TOML 配置文件，数据库路径优先级：CLI flag > 环境变量 > config > 默认值

## 📦 安装与编译

需要 Rust 工具链（rustc 1.96+，edition 2024）。`rusqlite` 使用 `bundled` 特性，无需安装系统级 SQLite。

```bash
git clone https://github.com/chiya999/myfinance.git
cd myfinance
cargo build --release        # 发布编译（启用 LTO）
# 二进制位于 target/release/rust_course.exe
```

开发模式：

```bash
cargo run -- <子命令>        # debug 编译，编译更快
cargo test                   # 运行 19 个测试（16 单元 + 3 集成）
cargo clippy --all-targets -- -D warnings   # 零警告
cargo fmt --check            # 格式检查
```

## 🚀 快速开始

```bash
# 1. 创建账户
cargo run -- account add "工资卡" --kind bank_card --initial-balance 10000
cargo run -- account add "微信钱包" --kind wallet --initial-balance 5000

# 2. 记一笔收入
cargo run -- txn add 1 12 15000 income -d "6月工资"
#            account_id=1  category_id=12(工资)  amount=15000元  type=income

# 3. 记一笔支出
cargo run -- txn add 2 1 25.5 expense -d "午餐"
#            account_id=2  category_id=1(餐饮)  amount=25.5元  type=expense

# 4. 账户间转账
cargo run -- txn transfer 1 2 2000 -d "还款"

# 5. 查看报表
cargo run -- report summary --from 2026-01-01 --to 2026-12-31
cargo run -- report category --kind expense
cargo run -- report trend --granularity monthly --months 6

# 6. 启动 TUI（推荐）
cargo run -- tui
```

## 📚 命令参考

### 全局选项

```
myfinance [--database <path>] [--verbose] <command>
          # --database 默认 myfinance.db，可用环境变量 MYFINANCE_DB 覆盖
```

### account — 账户管理

```bash
myfinance account list [--show-inactive]              # 列出账户
myfinance account add <name> --kind <kind> [--currency CNY] [--initial-balance 0]
                       # kind: wallet | bank_card | credit_card | invest
myfinance account show <id>                           # 详情（含本月收支）
myfinance account edit <id> [--name ...] [--active true|false]
myfinance account delete <id> [--force]               # force 跳过余额非零校验
```

### txn — 交易管理

```bash
myfinance txn list [--account-id N] [--category-id N] [--txn-type TYPE]
                   [--from-date YYYY-MM-DD] [--to-date YYYY-MM-DD]
                   [--min-amount N] [--max-amount N] [--limit 50]
                   # txn-type: income | expense | transfer

myfinance txn add <account_id> <category_id> <amount> <type> [-d 描述] [-D 日期]
                  # amount 单位为元（可为小数）；type: income | expense

myfinance txn transfer <from_id> <to_id> <amount> [-d 描述] [-D 日期]
myfinance txn delete <id>                             # 自动冲正余额
```

### budget — 预算管理

```bash
myfinance budget list                                 # 列出所有预算
myfinance budget set <category_id> <amount> [--period monthly|weekly]
myfinance budget status                               # 各分类已花 / 占比
myfinance budget alert [--threshold 80]               # 超过阈值告警
```

### report — 报表

```bash
myfinance report summary [--from ...] [--to ...]      # 收支总览
myfinance report category [--kind expense|income] [--from ...] [--to ...]
myfinance report trend [--granularity daily|weekly|monthly] [--months 6]
myfinance report account-snapshot                     # 账户快照
myfinance report export [--format csv|json] [--output <path>]   # 导出全部数据
```

### import — 账单导入

```bash
myfinance import auto   <file.csv> [--account <id>]   # 自动检测格式
myfinance import alipay <file.csv> [--account <id>]
myfinance import wechat <file.csv> [--account <id>]
```

导入流程：自动跳过元数据行 → 识别交易时间 / 金额 / 交易对方列 → 处理千位逗号和 ¥ 前缀 → 交易对方自动成为分类（"美团"所有消费归入"美团"分类）。

## ⌨️ TUI 键盘快捷键

| 按键 | 功能 |
|------|------|
| `1`-`5` | 切换标签页（Dashboard / Accounts / Transactions / Budget / Reports） |
| `Tab` / `Shift+Tab` | 循环切页 |
| `j` / `k` 或 `↓` / `↑` | 列表导航 |
| `a` | 添加（账户 / 交易 / 预算，按当前标签页） |
| `d` | 删除选中项（确认提示） |
| `f` | 选择账户视角（仅显示该账户数据），`0` 恢复全局 |
| `t` | 指定时间范围（输入 `2026` / `2026-07` / `2026-07-01` / 空=全部） |
| `r` | 刷新数据 |
| `h` 或 `?` | 帮助 |
| `q` | 退出 |
| `Esc` | 取消 / 退出输入模式 |

## 🏗️ 技术栈

| 组件 | Crate | 说明 |
|------|-------|------|
| 数据库 | `rusqlite` 0.32 (bundled) | 嵌入式 SQLite，C 源码编译进二进制 |
| CLI | `clap` 4 (derive) | 派生宏自动生成子命令 |
| TUI | `ratatui` 0.29 + `crossterm` 0.28 | immediate-mode 终端渲染 |
| 错误处理 | `thiserror` 2 + `anyhow` 1 | 统一 `AppError` 枚举（6 变体，`#[from]` 自动转换） |
| 序列化 | `serde` + `serde_json` | JSON 导出与配置 |
| 日期 | `chrono` 0.4 | 日期范围筛选与趋势聚合 |
| CSV | `csv` 1.3 | 账单解析 |
| 配置 | `toml` 0.8 + `dirs` 5 | TOML 配置文件，自动路径 |
| 异步 | `tokio` 1 (full) | TUI 事件循环 |

## 📂 项目结构

```
myfinance/
├── src/
│   ├── main.rs              # 入口：配置加载 + CLI 分发
│   ├── lib.rs               # 库根
│   ├── error.rs             # AppError 枚举 + AppResult<T>
│   ├── models/              # 领域模型：Account / Transaction / Category / Budget
│   ├── db/                  # Database 封装 + Repository<T> trait + 4 个仓库
│   ├── cli/                 # clap 6 组子命令 + TxnFilters 共享筛选
│   ├── engine/              # 业务服务：Account / Txn / Budget / Report Service
│   ├── tui/                 # App 状态机 + 事件循环 + UI 渲染
│   ├── import/              # BillImportAdapter trait + 支付宝/微信适配器
│   ├── export/              # Exporter trait + CSV/JSON 导出
│   └── config/              # Settings + TOML 配置
├── tests/                   # 集成测试（端到端生命周期 / 预置分类 / 边界条件）
├── migrations/001_init.sql  # 建表 DDL + 索引 + 18 个预置分类
└── Cargo.toml               # profile.dev opt-level=1, profile.release lto=true
```

### 架构分层

```
models  →  db  →  engine  →  cli / tui
（纯数据） （Repository）  （Service）   （用户界面）
```

单向依赖：上层依赖下层，下层不感知上层。Service 层通过 `Database::open_memory()` 可在内存 SQLite 中独立测试，完全不依赖 CLI 或 TUI。

## 🎯 设计要点

- **金额零误差** — 全部金额以 `i64` 整数「分」存储。IEEE 754 浮点导致的 `0.1 + 0.2 ≠ 0.3` 经典 bug 在类型系统中从源头消除。
- **事务性记账** — `BEGIN IMMEDIATE` / `COMMIT` / `ROLLBACK` 保证 INSERT 交易记录 + UPDATE 账户余额原子提交，中途失败自动回滚。
- **删除自动冲正** — 删除交易时反向调整账户余额，保证账目始终平衡。
- **迁移幂等性** — 基于 `PRAGMA user_version` 的迁移管理，多次运行不会重复执行已应用的 DDL。
- **trait 驱动可扩展** — `Repository<T>`、`BillImportAdapter`、`Exporter` 三个核心 trait。新增银行账单格式只需实现 `BillImportAdapter`，不修改任何现有代码（开闭原则）。
- **selected_index 围栏算法** — `max_list_index()` 动态查询当前标签页列表长度，`on_tick` 周期性钳制越界索引，`set_tab()` 切换时归零，彻底解决 TUI 导航越界问题。

## 🧪 测试

```bash
cargo test -- --nocapture
```

- **16 个单元测试** — 覆盖 models / db / engine 三层
- **3 个集成测试** —
  - `test_full_lifecycle`：建库 → 加账户 → 记账 → 转账 → 预算 → 报表 → 删除冲正 → 验证冲正后余额
  - `test_category_defaults`：验证迁移脚本中 18 个预置分类正确加载
  - `test_edge_cases`：自转账禁止、收入分类禁止设预算、非 force 删除失败、负数金额拒绝

## 📄 许可

MIT
