use super::Finding;
use regex::Regex;
use std::collections::HashMap;
use std::path::Path;
use std::sync::LazyLock;

struct DbPattern {
    regex: Regex,
    db_type: &'static str,
    severity: &'static str,
    description: &'static str,
}

static PATTERNS: LazyLock<Vec<DbPattern>> = LazyLock::new(|| {
    vec![
        // Connection strings with credentials (high severity)
        DbPattern {
            regex: Regex::new(r#"(?i)(?:postgres|postgresql)://\w+:\w+@[^\s'"]+?"#).unwrap(),
            db_type: "PostgreSQL",
            severity: "high",
            description: "PostgreSQL connection string with embedded credentials",
        },
        DbPattern {
            regex: Regex::new(r#"(?i)mongodb(?:\+srv)?://\w+:\w+@[^\s'"]+?"#).unwrap(),
            db_type: "MongoDB",
            severity: "high",
            description: "MongoDB connection string with embedded credentials",
        },
        DbPattern {
            regex: Regex::new(r#"(?i)mysql://\w+:\w+@[^\s'"]+?"#).unwrap(),
            db_type: "MySQL",
            severity: "high",
            description: "MySQL connection string with embedded credentials",
        },
        DbPattern {
            regex: Regex::new(r#"(?i)redis://(?:\w+:\w+@)?[^\s'"]+?"#).unwrap(),
            db_type: "Redis",
            severity: "medium",
            description: "Redis connection string",
        },
        // Connection strings without credentials (info)
        DbPattern {
            regex: Regex::new(r#"(?i)(?:DATABASE_URL|DB_URL|MONGO_URI|REDIS_URL)\s*=\s*[^\s]+"#).unwrap(),
            db_type: "Generic",
            severity: "medium",
            description: "Database URL environment variable assignment",
        },
        // ORM/driver configurations
        DbPattern {
            regex: Regex::new(r#"(?i)(?:create_engine|sqlalchemy|sequelize|prisma|mongoose\.connect|knex|typeorm)\s*\("#).unwrap(),
            db_type: "ORM",
            severity: "info",
            description: "ORM/database driver initialization",
        },
        // Raw SQL queries (potential injection risk)
        DbPattern {
            regex: Regex::new(r#"(?i)(?:execute|query|raw)\s*\(\s*(?:f['""]|['""].*?\{|.*?\+\s*\w+\s*\+)"#).unwrap(),
            db_type: "SQL",
            severity: "high",
            description: "Potential SQL injection: string interpolation in query",
        },
        // SQLite file references
        DbPattern {
            regex: Regex::new(r#"(?i)sqlite:///[^\s'"]+?"#).unwrap(),
            db_type: "SQLite",
            severity: "info",
            description: "SQLite database file reference",
        },
    ]
});

/// Scan a file for database connections and queries
pub fn scan(path: &Path, source: &str, language: &str) -> Vec<Finding> {
    let mut findings = Vec::new();
    let file_str = path.to_string_lossy().to_string();

    for (line_num, line) in source.lines().enumerate() {
        for pattern in PATTERNS.iter() {
            if pattern.regex.is_match(line) {
                let mut metadata = HashMap::new();
                metadata.insert("db_type".into(), pattern.db_type.into());

                findings.push(Finding {
                    category: "db_connection".into(),
                    severity: pattern.severity.into(),
                    file: file_str.clone(),
                    line: line_num + 1,
                    title: format!("{} connection detected", pattern.db_type),
                    description: pattern.description.into(),
                    snippet: line.trim().to_string(),
                    language: language.into(),
                    metadata,
                });
            }
        }
    }

    findings
}
