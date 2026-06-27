use serde::{Deserialize, Serialize};

/// 账户类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccountKind {
    /// 现金钱包
    Wallet,
    /// 银行储蓄卡
    BankCard,
    /// 信用卡
    CreditCard,
    /// 投资账户
    Invest,
}

impl AccountKind {
    /// 从数据库字符串反序列化
    pub fn from_db(s: &str) -> Option<Self> {
        match s {
            "wallet" => Some(Self::Wallet),
            "bank_card" => Some(Self::BankCard),
            "credit_card" => Some(Self::CreditCard),
            "invest" => Some(Self::Invest),
            _ => None,
        }
    }

    /// 转为数据库字符串
    pub fn to_db(self) -> &'static str {
        match self {
            Self::Wallet => "wallet",
            Self::BankCard => "bank_card",
            Self::CreditCard => "credit_card",
            Self::Invest => "invest",
        }
    }

    /// 中文显示名
    pub fn display_name(self) -> &'static str {
        match self {
            Self::Wallet => "现金钱包",
            Self::BankCard => "储蓄卡",
            Self::CreditCard => "信用卡",
            Self::Invest => "投资账户",
        }
    }
}

impl std::str::FromStr for AccountKind {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "wallet" | "现金" | "钱包" => Ok(Self::Wallet),
            "bank_card" | "bankcard" | "储蓄卡" | "银行卡" => Ok(Self::BankCard),
            "credit_card" | "creditcard" | "信用卡" => Ok(Self::CreditCard),
            "invest" | "投资" | "投资账户" => Ok(Self::Invest),
            _ => Err(format!("未知账户类型: {s}")),
        }
    }
}

/// 账户
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    /// 数据库主键 (None 表示尚未持久化)
    pub id: Option<i64>,
    /// 账户名称
    pub name: String,
    /// 账户类型
    pub kind: AccountKind,
    /// 货币代码 (默认 CNY)
    pub currency: String,
    /// 余额，以分为单位
    pub balance_cents: i64,
    /// 是否启用
    pub is_active: bool,
    /// 创建时间 ISO 8601
    pub created_at: Option<String>,
    /// 更新时间 ISO 8601
    pub updated_at: Option<String>,
}

impl Account {
    /// 新建账户
    pub fn new(name: &str, kind: AccountKind, currency: &str, initial_balance_cents: i64) -> Self {
        Self {
            id: None,
            name: name.to_string(),
            kind,
            currency: currency.to_string(),
            balance_cents: initial_balance_cents,
            is_active: true,
            created_at: None,
            updated_at: None,
        }
    }
}
