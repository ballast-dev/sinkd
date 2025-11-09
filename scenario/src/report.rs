use log::info;
use std::fs;
use std::path::Path;
use std::time::SystemTime;

#[derive(serde::Serialize)]
struct ReportMetadata {
    timestamp: String,
    instance_name: String,
    report_number: u32,
    base_path: String,
}

#[derive(serde::Serialize)]
struct FileInfo {
    path: String,
    size: u64,
    modified: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    checksum: Option<String>,
}

#[derive(serde::Serialize)]
struct DirectoryInfo {
    path: String,
    file_count: usize,
    total_size: u64,
}

#[derive(serde::Serialize)]
struct Report {
    metadata: ReportMetadata,
    summary: SummaryInfo,
    directories: Vec<DirectoryInfo>,
    files: Vec<FileInfo>,
}

#[derive(serde::Serialize)]
struct SummaryInfo {
    files: usize,
    directories: usize,
    size: u64,
}

/// Generates a TOML report of the test scenarios directory
pub fn generate_toml_report(base_path: &str, report_number: u32) -> Result<(), String> {
    info!("Generating TOML report #{report_number} for path: {base_path}");

    let base = Path::new(base_path);
    if !base.exists() {
        return Err(format!("Base path does not exist: {base_path}"));
    }

    // Scan directory recursively
    let mut files = Vec::new();
    let mut directories = Vec::new();
    let mut total_size = 0u64;

    scan_directory(base, base, &mut files, &mut directories, &mut total_size)?;

    // Create report structure
    let metadata = ReportMetadata {
        timestamp: chrono::Utc::now().to_rfc3339(),
        instance_name: "delta".to_string(),
        report_number,
        base_path: base_path.to_string(),
    };

    let summary = SummaryInfo {
        files: files.len(),
        directories: directories.len(),
        size: total_size,
    };

    let report = Report {
        metadata,
        summary,
        directories,
        files,
    };

    // Serialize to TOML
    let toml_string = toml::to_string_pretty(&report)
        .map_err(|e| format!("Failed to serialize report to TOML: {e}"))?;

    // Write to reports directory
    let reports_dir = base.join("_reports");
    fs::create_dir_all(&reports_dir)
        .map_err(|e| format!("Failed to create reports directory: {e}"))?;

    let report_file = reports_dir.join(format!("delta_report_{report_number}.toml"));
    fs::write(&report_file, toml_string)
        .map_err(|e| format!("Failed to write report file: {e}"))?;

    info!("Report written to: {}", report_file.display());

    // Also create a latest symlink/reference
    let latest_file = reports_dir.join("delta_report_latest.toml");
    if latest_file.exists() {
        let _ = fs::remove_file(&latest_file);
    }
    fs::write(
        &latest_file,
        format!(
            "# Latest report: delta_report_{report_number}.toml\n# Generated: {}\n",
            chrono::Utc::now().to_rfc3339()
        ),
    )
    .ok();

    info!("Latest report reference updated");

    Ok(())
}

/// Recursively scans a directory and collects file/directory information
fn scan_directory(
    root: &Path,
    current: &Path,
    files: &mut Vec<FileInfo>,
    directories: &mut Vec<DirectoryInfo>,
    total_size: &mut u64,
) -> Result<(), String> {
    // Skip reports directory
    if current
        .file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|n| n == "_reports")
    {
        return Ok(());
    }

    let entries = fs::read_dir(current)
        .map_err(|e| format!("Failed to read directory {}: {e}", current.display()))?;

    let mut dir_file_count = 0;
    let mut dir_total_size = 0u64;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Failed to read directory entry: {e}"))?;
        let path = entry.path();

        if path.is_dir() {
            // Recursively scan subdirectories
            scan_directory(root, &path, files, directories, total_size)?;
        } else if path.is_file() {
            // Get file metadata
            let metadata = fs::metadata(&path)
                .map_err(|e| format!("Failed to get metadata for {}: {e}", path.display()))?;

            let size = metadata.len();
            *total_size += size;
            dir_total_size += size;
            dir_file_count += 1;

            let modified = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                .and_then(|d| {
                    i64::try_from(d.as_secs())
                        .ok()
                        .and_then(|secs| chrono::DateTime::<chrono::Utc>::from_timestamp(secs, 0))
                })
                .map_or_else(|| "unknown".to_string(), |dt| dt.to_rfc3339());

            let relative_path = path
                .strip_prefix(root)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();

            let file_info = FileInfo {
                path: relative_path,
                size,
                modified,
                checksum: None, // Can add checksum calculation later if needed
            };

            files.push(file_info);
        }
    }

    // Add directory info (only for non-root directories with files)
    if current != root && dir_file_count > 0 {
        let relative_path = current
            .strip_prefix(root)
            .unwrap_or(current)
            .to_string_lossy()
            .to_string();

        directories.push(DirectoryInfo {
            path: relative_path,
            file_count: dir_file_count,
            total_size: dir_total_size,
        });
    }

    Ok(())
}
