use anyhow::Result;
use vergen_gix::{BuildBuilder, CargoBuilder, Emitter, GixBuilder, RustcBuilder, SysinfoBuilder};

pub fn main() -> Result<()> {
    nightyl();
    beta();
    stable();
    msrv();
    Emitter::default()
        .add_instructions(&BuildBuilder::all_build()?)?
        .add_instructions(&CargoBuilder::all_cargo()?)?
        .add_instructions(&GixBuilder::all_git()?)?
        .add_instructions(&RustcBuilder::all_rustc()?)?
        .add_instructions(&SysinfoBuilder::all_sysinfo()?)?
        .emit()
}

#[rustversion::nightly]
fn nightyl() {
    println!("cargo:rustc-cfg=nightly");
}

#[rustversion::not(nightly)]
fn nightly_lints() {}

#[rustversion::beta]
fn beta_lints() {
    println!("cargo:rustc-cfg=beta");
}

#[rustversion::not(beta)]
fn beta() {}

#[rustversion::stable]
fn stable_lints() {
    println!("cargo:rustc-cfg=stable");
}

#[rustversion::not(stable)]
fn stable() {}

#[rustversion::before(1.70)]
fn msrv_lints() {}

#[rustversion::since(1.70)]
fn msrv() {
    println!("cargo:rustc-cfg=msrv");
}
