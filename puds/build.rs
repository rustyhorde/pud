use anyhow::Result;
use vergen::{vergen, Config};

pub fn main() -> Result<()> {
    println!("cargo:rerun-if-changed=build.rs");
    nightly_lints();
    beta_lints();
    stable_lints();
    msrv_lints();
    vergen(Config::default())
}

#[rustversion::nightly]
fn nightly_lints() {
    println!("cargo:rustc-cfg=nightly_lints");
}

#[rustversion::not(nightly)]
fn nightly_lints() {}

#[rustversion::beta]
fn beta_lints() {
    println!("cargo:rustc-cfg=beta_lints");
}

#[rustversion::not(beta)]
fn beta_lints() {}

#[rustversion::stable]
fn stable_lints() {
    println!("cargo:rustc-cfg=stable_lints");
}

#[rustversion::not(stable)]
fn stable_lints() {}

#[rustversion::before(1.65)]
fn msrv_lints() {}

#[rustversion::since(1.65)]
fn msrv_lints() {
    println!("cargo:rustc-cfg=msrv");
}
