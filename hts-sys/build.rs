// Copyright 2014 Johannes Köster.
// Licensed under the MIT license (http://opensource.org/licenses/MIT)
// This file may not be copied, modified, or distributed
// except according to those terms.

use bindgen;
use cc;
use fs_utils::copy::copy_directory;
use glob::glob;

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn sed_htslib_makefile(out: &PathBuf, patterns: &Vec<&str>, feature: &str) {
    for pattern in patterns {
        if Command::new("sed")
            .current_dir(out.join("htslib"))
            .arg("-i")
            .arg("-e")
            .arg(pattern)
            .arg("Makefile")
            .status()
            .unwrap()
            .success()
            != true
        {
            panic!("failed to strip {} support", feature);
        }
    }
}

fn main() {
    let out = PathBuf::from(env::var("OUT_DIR").unwrap());
    let mut cfg = cc::Build::new();
    cfg.warnings(false).static_flag(true).pic(true);

    if let Ok(z_inc) = env::var("DEP_Z_INCLUDE") {
        cfg.include(z_inc);
    }

    if !out.join("htslib").exists() {
        copy_directory("htslib", &out).unwrap();
    }

    let use_bzip2 = env::var("CARGO_FEATURE_BZIP2").is_ok();
    if !use_bzip2 {
        let bzip2_patterns = vec!["s/ -lbz2//", "/#define HAVE_LIBBZ2/d"];
        sed_htslib_makefile(&out, &bzip2_patterns, "bzip2");
    } else if let Ok(inc) = env::var("DEP_BZIP2_ROOT")
        .map(PathBuf::from)
        .map(|path| path.join("include"))
    {
        cfg.include(inc);
    }

    let use_lzma = env::var("CARGO_FEATURE_LZMA").is_ok();
    if !use_lzma {
        let lzma_patterns = vec!["s/ -llzma//", "/#define HAVE_LIBLZMA/d"];
        sed_htslib_makefile(&out, &lzma_patterns, "lzma");
    } else if let Ok(inc) = env::var("DEP_LZMA_INCLUDE").map(PathBuf::from) {
        cfg.include(inc);
    }

    let tool = cfg.get_compiler();
    let (cc_path, cflags_env) = (tool.path(), tool.cflags_env());
    let cc_cflags = cflags_env.to_string_lossy().replace("-O0", "");
    if Command::new("autoheader")
        .current_dir(out.join("htslib"))
        .status()
        .unwrap()
        .success()
        != true
    {
        panic!("failed to build htslib");
    }
   println!("autoheader");
   if Command::new("autoconf")
        .current_dir(out.join("htslib"))
        .status()
        .unwrap()
        .success()
        != true
    {
        panic!("failed to build htslib");
    }
    println!("autoconf");
    if Command::new("sudo")
        .current_dir(out.join("htslib"))
        .arg("sh")
        .arg("./configure")
        .arg(format!("CC=clang"))
        .status()
        .unwrap()
        .success()
        != true
    {
        panic!("failed to build htslib");
    }
    println!("sudo ./configure");
    if Command::new("sudo")
        .current_dir(out.join("htslib"))
        .arg("make")
//        .arg(format!("CC={}", cc_path.display()))
//        .arg(format!("CFLAGS={}", cc_cflags))
//        .arg("--host")
        .status()
        .unwrap()
        .success()
        != true
    {
        panic!("failed to build htslib");
    }
    println!("sudo make");
    if Command::new("sudo")
        .current_dir(out.join("htslib"))
        .arg("make")
        .arg("install")
        .status()
        .unwrap()
        .success()
        != true
    {
        panic!("failed to build htslib");
    }
    println!("sudo make install");
    cfg.file("wrapper.c").compile("wrapper");

    bindgen::Builder::default()
        .header("wrapper.h")
        .generate_comments(false)
        .blacklist_function("strtold")
        .blacklist_type("max_align_t")
        .generate()
        .expect("Unable to generate bindings.")
        .write_to_file(out.join("bindings.rs"))
        .expect("Could not write bindings.");

    let include = out.join("include");
    fs::create_dir_all(&include).unwrap();
    if include.join("htslib").exists() {
        fs::remove_dir_all(include.join("htslib")).expect("remove exist include dir");
    }
    copy_directory(out.join("htslib").join("htslib"), &include).unwrap();
    fs::copy(out.join("htslib").join("libhts.a"), out.join("libhts.a")).unwrap();

    println!("cargo:root={}", out.display());
    println!("cargo:include={}", include.display());
    println!("cargo:libdir={}", out.display());
    println!("cargo:rustc-link-lib=static=hts");
    println!("cargo:rerun-if-changed=wrapper.c");
    println!("cargo:rerun-if-changed=wrapper.h");
    for htsfile in glob("htslib/**/*").unwrap() {
        let htsfile = htsfile.as_ref().unwrap().to_str().unwrap();
        println!("cargo:rerun-if-changed={}", htsfile);
    }
}
