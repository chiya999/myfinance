use clap::Args;

/// 共享交易筛选参数
#[derive(Args, Debug, Clone)]
pub struct TxnFilters {
    /// 筛选账户 ID
    #[arg(short, long)]
    pub account_id: Option<i64>,

    /// 筛选分类 ID
    #[arg(short = 'C', long)]
    pub category_id: Option<i64>,

    /// 筛选交易类型
    #[arg(short = 'T', long)]
    pub txn_type: Option<String>,

    /// 起始日期 (YYYY-MM-DD)
    #[arg(long)]
    pub from_date: Option<String>,

    /// 结束日期 (YYYY-MM-DD)
    #[arg(long)]
    pub to_date: Option<String>,

    /// 最小金额（元）
    #[arg(long)]
    pub min_amount: Option<f64>,

    /// 最大金额（元）
    #[arg(long)]
    pub max_amount: Option<f64>,

    /// 返回条数上限
    #[arg(short, long, default_value = "50")]
    pub limit: usize,
}
