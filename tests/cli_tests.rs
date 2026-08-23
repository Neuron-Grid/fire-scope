use std::process::Command;

fn fire_scope(args: &[&str]) -> Result<std::process::Output, std::io::Error> {
    Command::new(env!("CARGO_BIN_EXE_fire-scope"))
        .args(args)
        .output()
}

#[test]
fn nested_help_is_available_without_network_access() -> Result<(), Box<dyn std::error::Error>> {
    let cases: &[(&[&str], &str)] = &[
        (&["--help"], "Commands:"),
        (&["list", "--help"], "country"),
        (&["list", "country", "--help"], "COUNTRY_CODES"),
        (&["list", "asn", "--help"], "AS_NUMBERS"),
        (&["overlap", "--help"], "--country"),
    ];

    for (args, expected) in cases {
        let output = fire_scope(args)?;
        let stdout = String::from_utf8(output.stdout)?;
        assert!(output.status.success(), "command failed: {args:?}");
        assert!(stdout.contains(expected), "missing {expected:?}: {stdout}");
    }
    Ok(())
}

#[test]
fn reports_version_0_2_0() -> Result<(), Box<dyn std::error::Error>> {
    for args in [
        &["--version"][..],
        &["list", "--version"][..],
        &["list", "country", "--version"][..],
        &["list", "asn", "-V"][..],
        &["overlap", "--version"][..],
    ] {
        let output = fire_scope(args)?;
        let stdout = String::from_utf8(output.stdout)?;
        assert!(output.status.success(), "command failed: {args:?}");
        assert!(stdout.contains("0.2.0"), "version missing: {stdout}");
    }
    Ok(())
}

#[test]
fn rejects_legacy_flat_flags() -> Result<(), std::io::Error> {
    for args in [&["-c", "jp"][..], &["-a", "1234"][..], &["-o"][..]] {
        assert!(!fire_scope(args)?.status.success());
    }
    Ok(())
}
