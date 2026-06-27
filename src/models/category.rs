use serde::{Deserialize, Serialize};

/// 分类类别（收入 / 支出）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CategoryKind {
    Income,
    Expense,
}

impl CategoryKind {
    pub fn from_db(s: &str) -> Option<Self> {
        match s {
            "income" => Some(Self::Income),
            "expense" => Some(Self::Expense),
            _ => None,
        }
    }

    pub fn to_db(self) -> &'static str {
        match self {
            Self::Income => "income",
            Self::Expense => "expense",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Income => "收入",
            Self::Expense => "支出",
        }
    }
}

/// 收支分类
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Category {
    pub id: Option<i64>,
    pub name: String,
    pub kind: CategoryKind,
    /// emoji 图标
    pub icon: String,
    /// 父分类 ID（支持二级分类）
    pub parent_id: Option<i64>,
    pub sort_order: i32,
    pub is_default: bool,
}
