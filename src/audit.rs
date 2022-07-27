use std::env;
use std::path::Path;
use std::process::Command;

use anyhow::{format_err, Context, Result};
use rustsec::Lockfile;
use rustsec::report::{Settings, VulnerabilityInfo};
use rustsec::{Database, Report, Vulnerability};

fn generate_lockfile(workspace_root: &Path, manifest_path: Option<&Path>) -> Result<Lockfile> {
    let lockfile = workspace_root.join("Cargo.lock");
    let mut command = Command::new(env::var("CARGO").unwrap_or_else(|_| "cargo".into()));

    if lockfile.exists() {
        return Lockfile::load(lockfile).context("Failed to load lockfile");
    }

    command.arg("generate-lockfile");

    if let Some(path) = manifest_path {
        command.arg("--manifest-path");
        command.arg(path.as_os_str());
    }

    let status = command
        .status()
        .context("Failed to run `cargo generate-lockfile`")?;

    match status.code() {
        Some(0) => Lockfile::load(lockfile).context("Failed to load lockfile"),
        Some(code) => Err(format_err!(
            "Non-zero status ({}) on `cargo generate-lockfile`",
            code,
        )),
        None => Err(format_err!(
            "Unexpected termination on `cargo generate-lockfile`",
        )),
    }
}

pub fn audit_package(workspace_root: &Path, manifest_path: Option<&Path>) -> Result<()> {
    let database = Database::fetch().context("Failed to fetch security advisory database")?;
    let lockfile = generate_lockfile(workspace_root, manifest_path)?;
    let settings = Settings::default();
    let report = Report::generate(&database, &lockfile, &settings);

    if report.vulnerabilities.found {
        let VulnerabilityInfo { count, list, .. } = report.vulnerabilities;

        let mut message = match count {
            1 => format!("Found {} vulnerability:\n", count),
            _ => format!("Found {} vulnerabilities:\n", count),
        };

        for Vulnerability {
            package,
            versions,
            advisory,
            ..
        } in list
        {
            message.push('\n');
            message.push_str(&format!("Crate:    {}\n", package.name));
            message.push_str(&format!("Version:  {}\n", package.version.to_string()));
            message.push_str(&format!("Title:    {}\n", advisory.title));
            message.push_str(&format!("Date:     {}\n", advisory.date.as_str()));
            message.push_str(&format!("ID:       {}\n", advisory.id));

            if let Some(url) = advisory.id.url() {
                message.push_str(&format!("URL:      {}\n", url));
            } else if let Some(url) = &advisory.url {
                message.push_str(&format!("URL:      {}\n", url));
            }

            if versions.patched().is_empty() {
                message.push_str("Solution: No solution available\n");
            } else {
                let patched = versions
                    .patched()
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .as_slice()
                    .join(" or ");

                message.push_str(&format!("Solution: Upgrade to {}\n", patched));
            }
        }

        message.push_str("\nPlease fix the issues or use \"--noaudit\" flag.\n");

        return Err(format_err!(message));
    }

    Ok(())
}
