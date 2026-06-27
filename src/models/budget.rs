use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// 预算周期
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BudgetPeriod {
    Monthly,
    Weekly,
}

impl BudgetPeriod {
    pub fn from_db(s: &str) -> Option<Self> {
        match s {
            "monthly" => Some(Self::Monthly),
            "weekly" => Some(Self::Weekly),
            _ => None,
        }
    }

    pub fn to_db(self) -> &'static str {
        match self {
            Self::Monthly => "monthly",
            Self::Weekly => "weekly",
        }
    }
}

/// 预算
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budget {
    pub id: Option<i64>,
    pub category_id: i64,
    /// 预算金额（分）
    pub amount_cents: i64,
    pub period: BudgetPeriod,
    pub start_date: NaiveDate,
    /// None 表示永久有效
    pub end_date: Option<NaiveDate>,
    pub created_at: Option<String>,
}
