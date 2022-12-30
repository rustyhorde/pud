use anyhow::Result;
use vergen::Vergen;

pub fn main() -> Result<()> {
    nightly_lints();
    beta_lints();
    stable_lints();
    msrv_lints();
    Vergen::default()
        .all_build()
        .all_cargo()
        .all_git()
        .all_rustc()
        .all_sysinfo()
        .gen()
}

#[rustversion::nightly]
fn nightly_lints() {
    println!("cargo:rustc-cfg=nightly");
}

#[rustversion::not(nightly)]
fn nightly_lints() {}

#[rustversion::beta]
fn beta_lints() {
    println!("cargo:rustc-cfg=beta");
}

#[rustversion::not(beta)]
fn beta_lints() {}

#[rustversion::stable]
fn stable_lints() {
    println!("cargo:rustc-cfg=stable");
}

#[rustversion::not(stable)]
fn stable_lints() {}

#[rustversion::before(1.65)]
fn msrv_lints() {}

#[rustversion::since(1.65)]
fn msrv_lints() {
    println!("cargo:rustc-cfg=msrv");
}
