/// 通用 CSV 解析工具
pub struct CsvParser;

impl CsvParser {
    /// 将 CSV 内容解析为 (header, rows) 的元组
    pub fn parse(content: &str) -> crate::error::AppResult<(Vec<String>, Vec<Vec<String>>)> {
        let mut reader = csv::ReaderBuilder::new()
            .flexible(true)
            .trim(csv::Trim::All)
            .from_reader(content.as_bytes());

        let headers: Vec<String> = reader
            .headers()
            .map_err(|e| crate::error::AppError::Validation(format!("CSV 读取错误: {e}")))?
            .iter()
            .map(|h| h.to_string())
            .collect();

        let mut rows = Vec::new();
        for result in reader.records() {
            let record = result
                .map_err(|e| crate::error::AppError::Validation(format!("CSV 行解析错误: {e}")))?;
            rows.push(record.iter().map(|f| f.to_string()).collect());
        }

        Ok((headers, rows))
    }

    /// 在 headers 中查找列的索引
    pub fn find_column(headers: &[String], candidates: &[&str]) -> Option<usize> {
        headers
            .iter()
            .position(|h| candidates.iter().any(|c| h.contains(c)))
    }

    /// 从行中安全获取指定列的值
    pub fn get_column(row: &[String], col: Option<usize>) -> Option<&str> {
        col.and_then(|i| row.get(i).map(|s| s.as_str()))
    }
}
