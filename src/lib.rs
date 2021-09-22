/*
 * Copyright 2016-2018 Doug Goldstein <cardoe@cardoe.com>
 *
 * Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
 * http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
 * <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
 * option. This file may not be copied, modified, or distributed
 * except according to those terms.
 */

mod audit;
mod license;
mod metadata;

use anyhow::{format_err, Context, Result};
use cargo_metadata::CargoOpt;
use cargo_metadata::MetadataCommand;
use std::collections::BTreeSet;
use std::fs::OpenOptions;
use std::path::Path;

use audit::audit_package;
use license::{normalize_license, split_spdx_license};
use metadata::EbuildConfig;

pub fn gen_ebuild_data( manifest_path: Option<&Path>
                      , package_name: Option<&str>
                      , audit: bool ) -> Result<EbuildConfig> {
    let mut cmd = MetadataCommand::new();

    cmd.features(CargoOpt::AllFeatures);

    if let Some(path) = manifest_path {
        cmd.manifest_path(path);
    }

    let metadata = cmd
        .exec()
        .map_err(|e| format_err!("cargo metadata failed: {}", e))?;

    let resolve = metadata
        .resolve
        .as_ref()
        .ok_or_else(|| format_err!("cargo metadata did not resolve the depend graph"))?;

    let root = 
        if let Some(pkg_name) = package_name {
            let found_package =
                metadata.packages.iter().find(|&p| {
                    p.name == pkg_name
                }).ok_or_else(|| format_err!("cargo metadata contains no specified package"))?;
            &found_package.id
        } else {
            resolve
                .root
                .as_ref()
                .ok_or_else(|| format_err!("cargo metadata failed to resolve the root package"))?
        };

    if audit {
        audit_package(metadata.workspace_root.as_ref(), manifest_path)?;
    }

    let mut licenses = BTreeSet::new();
    let mut crates = Vec::new();
    let mut root_pkg = None;

    for pkg in &metadata.packages {
        if &pkg.id == root {
            root_pkg = Some(pkg.clone());
        }

        if let Some(lic_list) = pkg.license.as_ref().map(|l| split_spdx_license(&l)) {
            for lic in lic_list.iter() {
                if let Some(norm) = normalize_license(&lic) {
                    // Add the normalized license name
                    licenses.insert(norm.to_string());
                } else {
                    // Add the unknown license name to be corrected manually
                    println!(
                        "WARNING: unknown license \"{}\", please correct manually",
                        &lic
                    );
                    licenses.insert(lic.to_string());
                }
            }
        }

        if pkg.license_file.is_some() {
            println!("WARNING: {} uses a license-file, not handled", pkg.name);
        }

        if let Some(src) = &pkg.source {
            // Check if the crate is available at crates.io
            if src.is_crates_io() {
                crates.push(format!("\t{}-{}\n", pkg.name, pkg.version));
            }
        }
    }

    let root_pkg = root_pkg
        .ok_or_else(|| format_err!("unable to determine package to generate ebuild for"))?;

    Ok(EbuildConfig::from_package(root_pkg, crates, licenses))
}

pub fn write_ebuild(
    ebuild_data: EbuildConfig,
    ebuild_path: &Path,
    template_path: Option<&Path>,
) -> Result<()> {
    // Open the file where we'll write the ebuild
    let mut file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(ebuild_path)
        .context(format!(
            "Unable to create {}",
            ebuild_path.display()
        ))?;

    let mut tera = tera::Tera::default();
    let mut context = tera::Context::from_serialize(ebuild_data)?;

    tera.add_raw_template("base.tera", include_str!("base.tera"))?;
    if let Some(template) = template_path {
        tera.add_template_file(template, Some("ebuild.tera"))?;
    } else {
        tera.add_raw_template("ebuild.tera", include_str!("ebuild.tera"))?;
    }

    context.insert("cargo_ebuild_ver", env!("CARGO_PKG_VERSION"));
    context.insert("this_year", &time::OffsetDateTime::now_utc().year());

    tera.render_to("ebuild.tera", &context, &mut file)
        .context(format!(
            "Failed to write to {}",
            ebuild_path.display()
        ))
}
