/*
 * Copyright 2016-2017 Doug Goldstein <cardoe@cardoe.com>
 *
 * Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
 * http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
 * <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
 * option. This file may not be copied, modified, or distributed
 * except according to those terms.
 */

extern crate cargo_ebuild;
extern crate structopt;

use anyhow::Result;
use cargo_ebuild::{gen_ebuild_data, write_ebuild};
use std::path::PathBuf;
use structopt::clap::AppSettings;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
struct Args {
    #[structopt(name = "PATH", long = "manifest-path", parse(from_os_str))]
    /// Path to Cargo.toml.
    manifest_path: Option<PathBuf>,
    #[structopt(name = "TEMPLATE", long = "template-path", short)]
    /// Non-standard template
    template_path: Option<PathBuf>,
}

#[derive(StructOpt, Debug)]
#[structopt(
    name = "cargo-ebuild",
    bin_name = "cargo",
    author,
    about = "Generates an ebuild for a given Cargo project",
    global_settings(&[AppSettings::ColoredHelp])
)]
enum Opt {
    #[structopt(name = "ebuild")]
    /// Generates an ebuild for a given Cargo project
    Ebuild(Args),
}

fn main() -> Result<()> {
    let Opt::Ebuild(opt) = Opt::from_args();

    // compute the data from the package that the build needs
    let ebuild_data = gen_ebuild_data(opt.manifest_path.as_deref())?;

    let ebuild_path = format!("{}-{}.ebuild", ebuild_data.name, ebuild_data.version);

    write_ebuild(ebuild_data, ebuild_path.as_ref(), opt.template_path.as_deref())?;

    println!("Wrote: {}", ebuild_path);

    Ok(())
}
