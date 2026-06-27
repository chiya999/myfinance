//! 图表数据准备层
//! Phase 6 核心：将报表数据转为图表友好结构

use serde::Serialize;

/// 柱状图数据
#[derive(Debug, Clone, Serialize)]
pub struct ChartData {
    pub title: String,
    pub labels: Vec<String>,
    pub values: Vec<f64>,
}

/// 趋势数据
#[derive(Debug, Clone, Serialize)]
pub struct TrendData {
    pub title: String,
    pub x_labels: Vec<String>,
    pub income_values: Vec<f64>,
    pub expense_values: Vec<f64>,
}
