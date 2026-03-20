pub mod json;
pub mod sarif;
pub mod terminal;

use crate::scanner::ScanResults;
use crate::analysis::AnalysisResults;
use anyhow::Result;

/// Output scan findings in the requested format
pub fn output(results: &ScanResults, format: &str, output_path: Option<&str>) -> Result<()> {
    let content = match format {
        "json" => json::format_scan(results)?,
        "sarif" => sarif::format_scan(results)?,
        "terminal" | _ => {
            terminal::print_scan(results);
            return Ok(());
        }
    };

    match output_path {
        Some(path) => {
            std::fs::write(path, &content)?;
            println!("Report written to: {}", path);
        }
        None => println!("{}", content),
    }

    Ok(())
}

/// Output analysis results
pub fn output_analysis(results: &AnalysisResults, format: &str, output_path: Option<&str>) -> Result<()> {
    let content = match format {
        "json" => serde_json::to_string_pretty(results)?,
        "terminal" | _ => {
            terminal::print_analysis(results);
            return Ok(());
        }
    };

    match output_path {
        Some(path) => {
            std::fs::write(path, &content)?;
            println!("Report written to: {}", path);
        }
        None => println!("{}", content),
    }

    Ok(())
}
