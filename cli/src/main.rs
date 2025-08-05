use std::{
    fs::{self, write},
    path::PathBuf,
    process::exit,
};

use clap::Parser;
use console::{Emoji, style};
use git_version::git_version;
use lib::{
    install::install,
    rhai::{Script, WasaupEngine},
};

const GIR_VERSION: &str =
    git_version!(args = ["--always", "--dirty=-modified", "--tags", "--abbrev=4"]);

#[derive(Parser, Debug)]
#[clap(
    version = GIR_VERSION,
    author = "Daniel F. Hauge animcuil@gmail.com",
    about = "wasupdate - A tool for updating stuff"
)]
struct Args {
    #[clap(
        short,
        long,
        default_value = "wasaupdate.rhai",
        help = "Path to the update script file"
    )]
    script: String,

    #[clap(
        short,
        long,
        default_value = "false",
        help = "Prints current, latest and location for installing latest version, but will not make any changes."
    )]
    check: bool,

    #[clap(
        short,
        long,
        default_value = "false",
        help = "Uses json as stdout format instead of plain text."
    )]
    json: bool,

    #[clap(
        default_values = &[""],
        help = "Specify command to run after update."
    )]
    run_after: Vec<String>,

    #[clap(
        long,
        default_value = "false",
        help = "Initialize a placeholder update script if it does not exist."
    )]
    init: bool,

    #[clap(
        short,
        long,
        default_value = "false",
        help = "Specify whether command after update shall be backgrounded or not."
    )]
    background: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct CheckedVersion {
    current: String,
    latest: String,
    install_path: String,
    will_update: bool,
}

fn p_header() {
    println!(
        "{} {} - {}\n",
        Emoji("📦", "#"),
        style("Wasupdate").bold().underlined(),
        style(GIR_VERSION),
    );
}

const DEFAULT_SCRIPT: &str = r#"fn current_version() {
    return "0.1.0";
}
fn latest_version() {
    return "0.1.0";
}
fn install_version(version) {
    return "path/to/archive-" + version + ".tar.gz";
}"#;

pub fn p_error(msg: &str, etype: &str) {
    eprintln!(
        "{} {}: {}\n\n{}\n",
        Emoji("❗", "!"),
        style("Error: ").bold().underlined().red(),
        etype,
        style(msg),
    );
}

pub fn p_success(msg: &str) {
    println!("{} {}", Emoji("✅", "✔️"), style(msg).bold().underlined(),);
}

pub fn init(script: &str, json: bool) {
    if PathBuf::from(script).exists() {}
    let write_result = write(script, DEFAULT_SCRIPT);
    match write_result {
        Ok(()) => {
            if json {
                let json_output = serde_json::json!({
                    "message": "Update script initialized successfully.",
                    "script": script,
                });
                println!("{}", serde_json::to_string_pretty(&json_output).unwrap());
            } else {
                p_success("Update script initialized successfully.");
            }
            exit(0);
        }
        Err(e) => {
            if json {
                let json_output = serde_json::json!({
                    "error": "Failed to initialize update script.",
                    "script": script,
                    "message": e.to_string(),
                });
                println!("{}", serde_json::to_string_pretty(&json_output).unwrap());
            } else {
                let etype = format!("Failed to init script file {}", Emoji("📄", "📃"));
                p_error(
                    format!(
                        "Failed to create placeholder init script file because of error: {}",
                        e.to_string()
                    )
                    .as_str(),
                    &etype,
                );
            }
            exit(1);
        }
    }
}

fn main() {
    let args = Args::parse();

    if !args.json {
        p_header();
    }

    if args.init {
        init(&args.script, args.json);
    }

    let path_buf = PathBuf::from(&args.script);
    if !path_buf.exists() {
        if args.json {
            let json_output = serde_json::json!({
                "error": "The update script file does not exist.",
                "script": args.script,
            });
            println!("{}", serde_json::to_string_pretty(&json_output).unwrap());
        } else {
            let etype = format!("Script file is missing {}", Emoji("📄", "📃"));
            p_error(
                format!("The update script file at {} does not exist.", args.script).as_str(),
                &etype,
            );
        }
        std::process::exit(1);
    }
    let wasup_engine = match WasaupEngine::new(Script::File(path_buf)) {
        Ok(engine) => engine,
        Err(e) => {
            if args.json {
                let json_output = serde_json::json!({
                    "error": "Failed to initialize the update script engine.",
                    "message": e.to_string(),
                });
                println!("{}", serde_json::to_string_pretty(&json_output).unwrap());
            } else {
                let etype = format!("Engine failed to start {}", Emoji("⚙️", "⚙️"));
                p_error(
                    &format!("Failed to start script engine because of error: {}", e),
                    &etype,
                );
            }
            std::process::exit(1);
        }
    };
    let current_version = match wasup_engine.current_version() {
        Ok(current_version) => current_version,
        Err(e) => {
            if args.json {
                let json_output = serde_json::json!({
                    "error": "Failed to get current version.",
                    "message": e.to_string(),
                });
                println!("{}", serde_json::to_string_pretty(&json_output).unwrap());
            } else {
                let etype = format!("Failed to get current version {}", Emoji("🔍", "🔎"));
                p_error(&format!("Failed to get current version: {}", e), &etype);
            }
            std::process::exit(1);
        }
    };

    let latest_version = match wasup_engine.latest_version() {
        Ok(latest_version) => latest_version,
        Err(e) => {
            if args.json {
                let json_output = serde_json::json!({
                    "error": "Failed to get latest version.",
                    "message": e.to_string(),
                });
                println!("{}", serde_json::to_string_pretty(&json_output).unwrap());
            } else {
                let etype = format!("Failed to get latest version {}", Emoji("🔍", "🔎"));
                p_error(&format!("Failed to get latest version: {}", e), &etype);
            }
            std::process::exit(1);
        }
    };
    let install_path = match wasup_engine.install_version(latest_version.to_string().as_str()) {
        Ok(ip) => ip,
        Err(e) => {
            if args.json {
                let json_output = serde_json::json!({
                    "error": "Failed to evaluate install location.",
                    "message": e.to_string(),
                });
                println!("{}", serde_json::to_string_pretty(&json_output).unwrap());
            } else {
                // same as before
                let etype = format!("Failed to evaluate install location {}", Emoji("📂", "📁"));
                p_error(
                    &format!("Failed to evaluate install location: {}", e),
                    &etype,
                );
            }
            std::process::exit(1);
        }
    };
    let will_update = current_version != latest_version;
    let checked_version = CheckedVersion {
        current: current_version.to_string(),
        latest: latest_version.to_string(),
        install_path: install_path.to_string(),
        will_update,
    };

    if args.json && args.check {
        let json_output = serde_json::to_string_pretty(&checked_version).unwrap();
        println!("{}", json_output);
    } else if will_update && !args.json {
        println!(
            "{} Upgrade available: {} {} {}",
            Emoji("🚀", "🚀"),
            style(current_version).bold().strikethrough(),
            Emoji("➡️", "→"),
            style(latest_version.to_string()).bold().underlined()
        );
        if install_path.starts_with("http") {
            println!(
                "{} Downloading version from: {}",
                Emoji("📥", "↓"),
                style(install_path).bold().underlined().green()
            );
        } else {
            println!(
                "{} Extracting version from: {}",
                Emoji("📂", "📁"),
                style(install_path).bold().underlined().green()
            );
        }
    } else if !args.json {
        println!(
            "Version: {} is up to date {}",
            style(current_version).bold().underlined(),
            Emoji("✅", "✔️")
        );
    }
    if args.check {
        std::process::exit(0);
    }

    if will_update {
        match install(&checked_version.install_path) {
            Ok(()) => {
                if args.json {
                    let json_output = serde_json::json!({
                        "message": "Update completed successfully.",
                        "current_version": checked_version.current,
                        "latest_version": checked_version.latest,
                        "install_path": checked_version.install_path,
                    });
                    println!("{}", serde_json::to_string_pretty(&json_output).unwrap());
                } else {
                    p_success("Update completed successfully.");
                }
            }
            Err(e) => {
                if args.json {
                    let json_output = serde_json::json!({
                        "error": "Failed to install the latest version.",
                        "message": e.to_string(),
                    });
                    println!("{}", serde_json::to_string_pretty(&json_output).unwrap());
                } else {
                    let etype = format!("Failed to install latest version {}", Emoji("⚠️", "⚠️"));
                    p_error(&format!("Failed to install latest version: {}", e), &etype);
                }
                std::process::exit(1);
            }
        }
    }

    let run_after = args
        .run_after
        .iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();
    if !run_after.is_empty() {
        let program = run_after[0];
        let run_args = &run_after[1..];
        if args.background {
            #[allow(clippy::zombie_processes)]
            std::process::Command::new(program)
                .args(run_args)
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::inherit())
                .stdin(std::process::Stdio::null())
                .spawn()
                .unwrap_or_else(|e| {
                    let current_exe = std::env::current_exe().unwrap_or_else(|_| {
                        eprintln!("Failed to get current executable path: {}", e);
                        std::process::exit(1);
                    });
                    let current_dir = current_exe
                        .parent()
                        .expect("Current executable has no parent directory");
                    let cmd_path = current_dir.join(program);
                    std::process::Command::new(cmd_path)
                        .args(run_args)
                        .stdout(std::process::Stdio::null())
                        .stderr(std::process::Stdio::inherit())
                        .stdin(std::process::Stdio::null())
                        .spawn()
                        .unwrap_or_else(|e| {
                            eprintln!("Failed to run command '{}': {}", program, e);
                            std::process::exit(1);
                        })
                });
        } else {
            std::process::Command::new(program)
                .args(run_args)
                .stdout(std::process::Stdio::inherit())
                .stderr(std::process::Stdio::inherit())
                .stdin(std::process::Stdio::inherit())
                .spawn()
                .unwrap_or_else(|e| {
                    let current_exe = std::env::current_exe().unwrap_or_else(|_| {
                        eprintln!("Failed to get current executable path: {}", e);
                        std::process::exit(1);
                    });
                    let current_dir = current_exe
                        .parent()
                        .expect("Current executable has no parent directory");
                    let cmd_path = current_dir.join(program);
                    std::process::Command::new(cmd_path)
                        .args(run_args)
                        .stdout(std::process::Stdio::inherit())
                        .stderr(std::process::Stdio::inherit())
                        .stdin(std::process::Stdio::inherit())
                        .spawn()
                        .unwrap_or_else(|e| {
                            eprintln!("Failed to run command '{}': {}", program, e);
                            std::process::exit(1);
                        })
                })
                .wait()
                .unwrap_or_else(|e| {
                    eprintln!("Failed to wait for command '{}': {}", program, e);
                    std::process::exit(1);
                });
        }
    }
}
