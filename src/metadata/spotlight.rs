use chrono::{DateTime, Utc};
use std::path::Path;
use std::process::Command;

#[derive(Debug, Clone, Default)]
pub struct FileMetadata {
    pub last_used: Option<DateTime<Utc>>,
    pub use_count: Option<u32>,
}

pub fn get_file_metadata(path: &Path) -> Option<FileMetadata> {
    let output = Command::new("mdls")
        .args(["-name", "kMDItemLastUsedDate", "-name", "kMDItemUseCount"])
        .arg(path)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    parse_mdls_output(&stdout)
}

fn parse_mdls_output(output: &str) -> Option<FileMetadata> {
    let mut last_used = None;
    let mut use_count = None;

    for line in output.lines() {
        let line = line.trim();

        if line.starts_with("kMDItemLastUsedDate") {
            last_used = parse_date_value(line);
        } else if line.starts_with("kMDItemUseCount") {
            use_count = parse_int_value(line);
        }
    }

    if last_used.is_some() || use_count.is_some() {
        Some(FileMetadata {
            last_used,
            use_count,
        })
    } else {
        None
    }
}

fn parse_date_value(line: &str) -> Option<DateTime<Utc>> {
    let parts: Vec<&str> = line.splitn(2, '=').collect();
    if parts.len() != 2 {
        return None;
    }

    let value = parts[1].trim();

    if value == "(null)" {
        return None;
    }

    value.trim_matches('"').parse().ok()
}

fn parse_int_value(line: &str) -> Option<u32> {
    let parts: Vec<&str> = line.splitn(2, '=').collect();
    if parts.len() != 2 {
        return None;
    }

    let value = parts[1].trim();

    if value == "(null)" {
        return None;
    }

    value.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_date_value() {
        let line = r#"kMDItemLastUsedDate = "2024-01-15 10:30:00 +0000""#;
        let result = parse_date_value(line);
        assert!(result.is_some());
    }

    #[test]
    fn test_parse_int_value() {
        let line = "kMDItemUseCount = 5";
        let result = parse_int_value(line);
        assert_eq!(result, Some(5));
    }

    #[test]
    fn test_parse_null_value() {
        let line = "kMDItemUseCount = (null)";
        let result = parse_int_value(line);
        assert_eq!(result, None);
    }
}
