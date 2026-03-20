use crate::scanner::ScanResults;
use crate::analysis::{AnalysisResults, AnalysisReport};
use colored::*;

/// Print scan findings to terminal with color
pub fn print_scan(results: &ScanResults) {
    println!("\n{}", "=== Recon Scan Results ===".bold());
    println!("Files scanned: {}", results.files_scanned);
    println!("Scan time: {}ms", results.scan_duration_ms);
    println!("Findings: {}\n", results.findings.len());

    // Group by severity
    let critical: Vec<_> = results.findings.iter().filter(|f| f.severity == "critical").collect();
    let high: Vec<_> = results.findings.iter().filter(|f| f.severity == "high").collect();
    let medium: Vec<_> = results.findings.iter().filter(|f| f.severity == "medium").collect();
    let low: Vec<_> = results.findings.iter().filter(|f| f.severity == "low").collect();
    let info: Vec<_> = results.findings.iter().filter(|f| f.severity == "info").collect();

    if !critical.is_empty() {
        println!("{} ({}):", "CRITICAL".red().bold(), critical.len());
        for f in &critical {
            print_finding(f);
        }
    }
    if !high.is_empty() {
        println!("{} ({}):", "HIGH".red(), high.len());
        for f in &high {
            print_finding(f);
        }
    }
    if !medium.is_empty() {
        println!("{} ({}):", "MEDIUM".yellow(), medium.len());
        for f in &medium {
            print_finding(f);
        }
    }
    if !low.is_empty() {
        println!("{} ({}):", "LOW".blue(), low.len());
        for f in &low {
            print_finding(f);
        }
    }
    if !info.is_empty() {
        println!("{} ({}):", "INFO".dimmed(), info.len());
        for f in &info {
            print_finding(f);
        }
    }

    // Summary bar
    println!("\n{}", "--- Summary ---".bold());
    println!(
        "  {} critical, {} high, {} medium, {} low, {} info",
        critical.len().to_string().red().bold(),
        high.len().to_string().red(),
        medium.len().to_string().yellow(),
        low.len().to_string().blue(),
        info.len().to_string().dimmed(),
    );
}

fn print_finding(f: &crate::scanner::Finding) {
    println!(
        "  {} {}:{} - {}",
        severity_icon(&f.severity),
        f.file.dimmed(),
        f.line,
        f.title,
    );
    println!("    {}", f.snippet.dimmed());
}

fn severity_icon(severity: &str) -> ColoredString {
    match severity {
        "critical" => "!!".red().bold(),
        "high" => "! ".red(),
        "medium" => "* ".yellow(),
        "low" => "- ".blue(),
        _ => ". ".dimmed(),
    }
}

/// Print analysis results to terminal
pub fn print_analysis(results: &AnalysisResults) {
    println!("\n{}", "=== Recon Analysis Results ===".bold());

    if let Some(ref report) = results.security {
        print_report(report);
    }
    if let Some(ref report) = results.api_efficiency {
        print_report(report);
    }
    if let Some(ref report) = results.agent_data {
        print_report(report);
    }
}

fn print_report(report: &AnalysisReport) {
    let score_color = match report.risk_score {
        0..=20 => report.risk_score.to_string().green(),
        21..=50 => report.risk_score.to_string().yellow(),
        51..=80 => report.risk_score.to_string().red(),
        _ => report.risk_score.to_string().red().bold(),
    };

    println!("\n{} (Risk: {}/100)", report.category.to_uppercase().bold(), score_color);
    println!("  {}", report.summary);
    println!("  {}", report.details.dimmed());

    if !report.recommendations.is_empty() {
        println!("  {}:", "Recommendations".underline());
        for rec in &report.recommendations {
            println!("    > {}", rec);
        }
    }
}
