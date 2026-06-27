use std::io::Write;

use super::{ExportData, Exporter};
use crate::error::AppResult;

pub struct JsonExporter;

impl Exporter for JsonExporter {
    fn format_name(&self) -> &'static str {
        "JSON"
    }

    fn export(&self, data: &ExportData, writer: &mut dyn Write) -> AppResult<()> {
        let json = serde_json::to_string_pretty(data)?;
        writer.write_all(json.as_bytes())?;
        Ok(())
    }
}
