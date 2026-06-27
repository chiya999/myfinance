use std::io::Write;

use super::{ExportData, Exporter};
use crate::error::AppResult;

pub struct CsvExporter;

impl Exporter for CsvExporter {
    fn format_name(&self) -> &'static str {
        "CSV"
    }

    fn export(&self, data: &ExportData, writer: &mut dyn Write) -> AppResult<()> {
        writeln!(writer, "type,date,amount,description,category,account")?;

        for txn in &data.transactions {
            writeln!(
                writer,
                "{},{},{},{},{},{}",
                txn.txn_type.display_name(),
                txn.txn_date,
                txn.amount_cents,
                txn.description,
                txn.category_id.map_or("-".to_string(), |c| c.to_string()),
                txn.account_id,
            )?;
        }

        Ok(())
    }
}
