-- myfinance 初始化迁移: 建表 + 索引 + 预置分类
PRAGMA journal_mode = WAL;
PRAGMA foreign_keys = ON;

-- 账户表
CREATE TABLE IF NOT EXISTS accounts (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    name            TEXT    NOT NULL,
    kind            TEXT    NOT NULL CHECK (kind IN ('wallet', 'bank_card', 'credit_card', 'invest')),
    currency        TEXT    NOT NULL DEFAULT 'CNY',
    balance_cents   INTEGER NOT NULL DEFAULT 0,
    is_active       INTEGER NOT NULL DEFAULT 1,
    created_at      TEXT    NOT NULL DEFAULT (datetime('now', 'localtime')),
    updated_at      TEXT    NOT NULL DEFAULT (datetime('now', 'localtime'))
);

-- 分类表（支持树形子分类）
CREATE TABLE IF NOT EXISTS categories (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    name        TEXT    NOT NULL,
    kind        TEXT    NOT NULL CHECK (kind IN ('income', 'expense')),
    icon        TEXT    NOT NULL DEFAULT '',
    parent_id   INTEGER REFERENCES categories(id),
    sort_order  INTEGER NOT NULL DEFAULT 0,
    is_default  INTEGER NOT NULL DEFAULT 0
);

-- 交易表
CREATE TABLE IF NOT EXISTS transactions (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    account_id      INTEGER NOT NULL REFERENCES accounts(id),
    category_id     INTEGER REFERENCES categories(id),
    type            TEXT    NOT NULL CHECK (type IN ('income', 'expense', 'transfer')),
    amount_cents    INTEGER NOT NULL CHECK (amount_cents > 0),
    to_account_id   INTEGER REFERENCES accounts(id),
    description     TEXT    NOT NULL DEFAULT '',
    txn_date        TEXT    NOT NULL,
    created_at      TEXT    NOT NULL DEFAULT (datetime('now', 'localtime'))
);

-- 预算表
CREATE TABLE IF NOT EXISTS budgets (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    category_id INTEGER NOT NULL REFERENCES categories(id),
    amount_cents INTEGER NOT NULL CHECK (amount_cents > 0),
    period      TEXT    NOT NULL CHECK (period IN ('monthly', 'weekly')),
    start_date  TEXT    NOT NULL,
    end_date    TEXT,
    created_at  TEXT    NOT NULL DEFAULT (datetime('now', 'localtime'))
);

-- 索引
CREATE INDEX IF NOT EXISTS idx_txn_account   ON transactions(account_id);
CREATE INDEX IF NOT EXISTS idx_txn_category  ON transactions(category_id);
CREATE INDEX IF NOT EXISTS idx_txn_date      ON transactions(txn_date);
CREATE INDEX IF NOT EXISTS idx_txn_type      ON transactions(type);
CREATE INDEX IF NOT EXISTS idx_budget_category ON budgets(category_id);
CREATE INDEX IF NOT EXISTS idx_account_active ON accounts(is_active);

-- 预置支出分类
INSERT INTO categories (name, kind, icon, parent_id, sort_order, is_default) VALUES
    ('餐饮',   'expense', '🍜', NULL, 1,  1),
    ('交通',   'expense', '🚇', NULL, 2,  1),
    ('购物',   'expense', '🛒', NULL, 3,  1),
    ('房租',   'expense', '🏠', NULL, 4,  1),
    ('水电',   'expense', '💡', NULL, 5,  1),
    ('通讯',   'expense', '📱', NULL, 6,  1),
    ('医疗',   'expense', '🏥', NULL, 7,  1),
    ('教育',   'expense', '📚', NULL, 8,  1),
    ('娱乐',   'expense', '🎵', NULL, 9,  1),
    ('人情',   'expense', '🎁', NULL, 10, 1),
    ('其他支出','expense', '💸', NULL, 11, 1);

-- 预置收入分类
INSERT INTO categories (name, kind, icon, parent_id, sort_order, is_default) VALUES
    ('工资',   'income', '💰', NULL, 1,  1),
    ('奖金',   'income', '🎉', NULL, 2,  1),
    ('兼职',   'income', '💼', NULL, 3,  1),
    ('投资收益','income', '📈', NULL, 4,  1),
    ('退款',   'income', '↩️',  NULL, 5,  1),
    ('红包',   'income', '🧧', NULL, 6,  1),
    ('其他收入','income', '💵', NULL, 7,  1);
