use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// 交易类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TxnType {
    /// 收入
    Income,
    /// 支出
    Expense,
    /// 转账（账户间）
    Transfer,
}

impl TxnType {
    pub fn from_db(s: &str) -> Option<Self> {
        match s {
            "income" => Some(Self::Income),
            "expense" => Some(Self::Expense),
            "transfer" => Some(Self::Transfer),
            _ => None,
        }
    }

    pub fn to_db(self) -> &'static str {
        match self {
            Self::Income => "income",
            Self::Expense => "expense",
            Self::Transfer => "transfer",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Income => "收入",
            Self::Expense => "支出",
            Self::Transfer => "转账",
        }
    }
}

impl std::str::FromStr for TxnType {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "income" | "收入" => Ok(Self::Income),
            "expense" | "支出" => Ok(Self::Expense),
            "transfer" | "转账" => Ok(Self::Transfer),
            _ => Err(format!("未知交易类型: {s}")),
        }
    }
}

/// 交易 / 账单记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transaction {
    pub id: Option<i64>,
    pub account_id: i64,
    pub category_id: Option<i64>,
    #[serde(rename = "type")]
    pub txn_type: TxnType,
    /// 金额（分），始终 > 0
    pub amount_cents: i64,
    /// 转账目标账户（仅 TxnType::Transfer 时有值）
    pub to_account_id: Option<i64>,
    pub description: String,
    pub txn_date: NaiveDate,
    pub created_at: Option<String>,
}

impl Transaction {
    /// 新建收入或支出记录
    pub fn new_income_expense(
        account_id: i64,
        category_id: i64,
        txn_type: TxnType,
        amount_cents: i64,
        description: &str,
        date: NaiveDate,
    ) -> Self {
        assert!(matches!(txn_type, TxnType::Income | TxnType::Expense));
        assert!(amount_cents > 0);
        Self {
            id: None,
            account_id,
            category_id: Some(category_id),
            txn_type,
            amount_cents,
            to_account_id: None,
            description: description.to_string(),
            txn_date: date,
            created_at: None,
        }
    }

    /// 新建转账记录
    pub fn new_transfer(
        from_account_id: i64,
        to_account_id: i64,
        amount_cents: i64,
        description: &str,
        date: NaiveDate,
    ) -> Self {
        assert!(amount_cents > 0);
        Self {
            id: None,
            account_id: from_account_id,
            category_id: None,
            txn_type: TxnType::Transfer,
            amount_cents,
            to_account_id: Some(to_account_id),
            description: description.to_string(),
            txn_date: date,
            created_at: None,
        }
    }
}
