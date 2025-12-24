use std::process::Command;

fn main() {
    // Capture git commit SHA (full)
    let git_sha = run_git_command(&["rev-parse", "HEAD"]).unwrap_or_else(|| "unknown".to_string());

    // Capture git commit SHA (short, 8 chars)
    let git_sha_short = run_git_command(&["rev-parse", "--short=8", "HEAD"])
        .unwrap_or_else(|| "unknown".to_string());

    // Capture git branch name
    let git_branch = run_git_command(&["rev-parse", "--abbrev-ref", "HEAD"])
        .unwrap_or_else(|| "unknown".to_string());

    // Capture git describe for semantic versioning
    // Format: v0.1.0-5-g7b92aa23 (tag-commits_since_tag-short_sha)
    // or v0.1.0 if on a tag, or v0.1.0-dirty if dirty
    let git_describe = run_git_command(&["describe", "--tags", "--always", "--dirty"])
        .or_else(|| {
            // Fallback to CARGO_PKG_VERSION if no tags exist
            std::env::var("CARGO_PKG_VERSION").ok()
        })
        .unwrap_or_else(|| "unknown".to_string());

    // Check if repository has uncommitted changes
    let git_dirty = is_git_dirty();

    // Capture build timestamp in ISO 8601 format (YYYY-MM-DD)
    let build_timestamp = std::env::var("SOURCE_DATE_EPOCH")
        .ok()
        .and_then(|epoch| {
            use std::time::UNIX_EPOCH;
            let secs = epoch.parse::<u64>().ok()?;
            let time = UNIX_EPOCH + std::time::Duration::from_secs(secs);
            Some(format_timestamp(time))
        })
        .unwrap_or_else(|| format_timestamp(std::time::SystemTime::now()));

    // Set environment variables for the build
    println!("cargo:rustc-env=GIT_SHA={git_sha}");
    println!("cargo:rustc-env=GIT_SHA_SHORT={git_sha_short}");
    println!("cargo:rustc-env=GIT_BRANCH={git_branch}");
    println!("cargo:rustc-env=GIT_DESCRIBE={git_describe}");
    println!("cargo:rustc-env=GIT_DIRTY={git_dirty}");
    println!("cargo:rustc-env=BUILD_TIMESTAMP={build_timestamp}");

    // Rerun build script if git HEAD changes
    // Should point to the root of the repository
    println!("cargo:rerun-if-changed=../../.git/HEAD");
    // Also rerun if the current branch ref changes
    if let Some(branch_ref) = run_git_command(&["symbolic-ref", "HEAD"]) {
        let ref_path = format!("../../.git/{branch_ref}");
        println!("cargo:rerun-if-changed={ref_path}");
    }
}

/// Runs a git command and returns the output as a trimmed string, or None if the command fails.
fn run_git_command(args: &[&str]) -> Option<String> {
    let output = Command::new("git").args(args).output().ok()?;

    if output.status.success() {
        String::from_utf8(output.stdout)
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
    } else {
        None
    }
}

/// Checks if the git repository has uncommitted changes (modified, staged, or untracked files).
/// Returns "true" or "false" as a string.
fn is_git_dirty() -> String {
    // Check if there are any changes in the index or working tree
    // git diff-index --quiet HEAD returns non-zero if there are changes
    let has_changes = Command::new("git")
        .args(["diff-index", "--quiet", "HEAD", "--"])
        .status()
        .map(|status| !status.success())
        .unwrap_or(false);

    if has_changes {
        return "true".to_string();
    }

    // Check for untracked files
    let has_untracked = run_git_command(&["ls-files", "--others", "--exclude-standard"])
        .is_some_and(|output| !output.is_empty());

    if has_untracked {
        "true".to_string()
    } else {
        "false".to_string()
    }
}

/// Formats a `SystemTime` as an ISO 8601 date (YYYY-MM-DD).
fn format_timestamp(time: std::time::SystemTime) -> String {
    use std::time::UNIX_EPOCH;

    let duration = time
        .duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| std::time::Duration::from_secs(0));

    let total_secs = duration.as_secs();
    // Simple date calculation (not accounting for leap seconds, but good enough)
    let days_since_epoch = total_secs / 86400;

    // Start from 1970-01-01
    let mut year = 1970;
    let mut remaining_days = days_since_epoch;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }

    let days_in_months = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    let mut month = 1;
    let mut day = remaining_days + 1;

    for days_in_month in &days_in_months {
        if day <= *days_in_month {
            break;
        }
        day -= days_in_month;
        month += 1;
    }

    format!("{year:04}-{month:02}-{day:02}")
}

const fn is_leap_year(year: u64) -> bool {
    (year.is_multiple_of(4) && !year.is_multiple_of(100)) || year.is_multiple_of(400)
}
