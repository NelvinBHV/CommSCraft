//! Central local/CI entry point; no live SIP traffic.
use std::{
    env,
    process::{Command, ExitCode},
};

fn run(program: &str, args: &[&str]) -> Result<(), String> {
    println!("==> {program} {}", args.join(" "));
    let status = Command::new(program)
        .args(args)
        .status()
        .map_err(|e| format!("{program}: {e}"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("{program} exited with {status}"))
    }
}

fn ci() -> Result<(), String> {
    let py = if cfg!(windows) { "python" } else { "python3" };
    run(
        py,
        &[
            "-m", "unittest", "discover", "-s", "scripts", "-p", "test*.py",
        ],
    )?;
    run(py, &["scripts/check_release.py"])?;
    run("cargo", &["fmt", "--all", "--", "--check"])?;
    run("cargo", &["test", "--workspace", "--all-targets"])?;
    run(
        "cargo",
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ],
    )
}

fn main() -> ExitCode {
    let arg = env::args().nth(1).unwrap_or_else(|| "help".to_string());
    let py = if cfg!(windows) { "python" } else { "python3" };
    let result = match arg.as_str() {
        "ci" => ci(),
        "test" => run("cargo", &["test", "--workspace", "--all-targets"]),
        "evidence" => run(
            py,
            &[
                "-m", "unittest", "discover", "-s", "scripts", "-p", "test*.py",
            ],
        ),
        "release-check" => run(py, &["scripts/check_release.py", "--release"]),
        "help" | "--help" | "-h" => {
            println!("CommSCraft build tools: ci | test | evidence | release-check");
            Ok(())
        }
        _ => Err(format!("Unknown CommSCraft build command: {arg}")),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            eprintln!("{msg}");
            ExitCode::FAILURE
        }
    }
}
