use std::path::Path;

use crate::models::scan_result::ScanResult;

pub fn export_json(result: &ScanResult, output_path: &Path) -> anyhow::Result<()> {
    let json = serde_json::to_string_pretty(result)?;
    std::fs::write(output_path, json)?;
    Ok(())
}
