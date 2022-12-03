use assert_cmd::Command;

#[test]
fn success() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("puds")?;

    cmd.assert().success();

    Ok(())
}
