// Copyright (C) 2024-2025 Fred Clausen
// Use of this source code is governed by an MIT-style
// license that can be found in the LICENSE file or at
// https://opensource.org/licenses/MIT.

use std::{fs::OpenOptions, io::BufWriter, path::Path, process::Command};

fn main() {
    println!("Building....");
    let out_dir = std::env::var("OUT_DIR").expect("no out dir");
    let out_dir = Path::new(&out_dir);
    let terminfo_out_dir = out_dir.join("terminfo");
    let terminfo_definition = "../res/freminal.ti";
    eprintln!("cargo:rerun-if-changed={terminfo_definition}");

    let mut child = Command::new("tic")
        .arg("-o")
        .arg(&terminfo_out_dir)
        .arg("-x")
        .arg(terminfo_definition)
        .spawn()
        .expect("Failed to spawn terminfo compiler");
    let status = child
        .wait()
        .expect("failed to get terminfo compiler result");
    assert!(status.success());

    let terminfo_tarball_path = out_dir.join("terminfo.tar");
    let terminfo_tarball_file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(terminfo_tarball_path)
        .expect("Failed to open terminfo tarball for writing");

    let mut tar_builder = tar::Builder::new(BufWriter::new(terminfo_tarball_file));
    tar_builder
        .append_dir_all(".", terminfo_out_dir)
        .expect("Failed to add terminfo to tarball");
    tar_builder
        .finish()
        .expect("Failed to write terminfo tarball");
}
