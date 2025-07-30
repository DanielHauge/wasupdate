use clap::Parser;
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
        long,
        default_value = "wasaupdate.rhai",
        help = "Path to the update script file"
    )]
    script: String,

    #[clap(long, default_value = "false", help = "Prints the current version")]
    current: bool,

    #[clap(long, default_value = "false", help = "Prints the latest version")]
    latest: bool,

    #[clap(
        long,
        default_value = "false",
        help = "Prints the location for installing latest version."
    )]
    install: bool,

    #[clap(
        long,
        default_value = "false",
        help = "Prints current, latest and location for installing latest version."
    )]
    dry: bool,

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
        long,
        default_value = "false",
        help = "Specify whether command after update shall be backgrounded or not."
    )]
    background: bool,
}

fn main() {
    let args = Args::parse();
    if args.init {
        if std::path::PathBuf::from(&args.script).exists() {
            eprintln!(
                "Error: The update script file '{}' already exists.",
                args.script
            );
            std::process::exit(1);
        }
        let default_script = r#"fn current_version() {
    return "0.1.0";
}

fn latest_version() {
    return "0.1.0";
}

fn install_version(version) {
    return "path/to/archive-" + version + ".tar.gz";
}"#;
        std::fs::write(&args.script, default_script).expect("Failed to write default script");
        println!("Initialized update script at '{}'", args.script);
        return;
    }
    let path_buf = std::path::PathBuf::from(&args.script);
    if !path_buf.exists() {
        eprintln!(
            "Error: The update script file '{}' does not exist.",
            path_buf.display()
        );
        std::process::exit(1);
    }
    let wasup_engine = match WasaupEngine::new(Script::File(path_buf)) {
        Ok(engine) => engine,
        Err(e) => {
            eprintln!("Error initializing WasaupEngine: {}", e);
            std::process::exit(1);
        }
    };
    match (args.dry, args.current, args.latest, args.install) {
        (true, _, _, _) => {
            let current_version = wasup_engine.current_version().unwrap_or_else(|e| {
                eprintln!("Error getting current version: {}", e);
                std::process::exit(1);
            });
            let latest_version = wasup_engine.latest_version().unwrap_or_else(|e| {
                eprintln!("Error getting latest version: {}", e);
                std::process::exit(1);
            });
            let install_path = wasup_engine
                .install_version(latest_version.to_string().as_str())
                .unwrap_or_else(|e| {
                    eprintln!("Error getting install path: {}", e);
                    std::process::exit(1);
                });
            println!("Current version: {}", current_version);
            println!("Latest version: {}", latest_version);
            println!("Install path for latest version: {}", install_path);
            let will_update = current_version != latest_version;
            if will_update {
                println!("Update will be performed.");
            } else {
                println!("No update needed.");
            }
            std::process::exit(0);
        }
        (_, cur, latest, install) => {
            if cur {
                match wasup_engine.current_version() {
                    Ok(version) => println!("Current version: {}", version),
                    Err(e) => eprintln!("Error getting current version: {}", e),
                }
            }
            if latest {
                match wasup_engine.latest_version() {
                    Ok(version) => println!("Latest version: {}", version),
                    Err(e) => eprintln!("Error getting latest version: {}", e),
                }
            }
            if install {
                let latest_version = wasup_engine.latest_version().unwrap_or_else(|e| {
                    eprintln!("Error getting latest version: {}", e);
                    std::process::exit(1);
                });
                match wasup_engine.install_version(latest_version.to_string().as_str()) {
                    Ok(path) => println!("Install path for latest version: {}", path),
                    Err(e) => eprintln!("Error getting install path: {}", e),
                }
            }
        }
    }

    let current_version = wasup_engine.current_version().unwrap_or_else(|e| {
        eprintln!("Error getting current version: {}", e);
        std::process::exit(1);
    });
    let latest_version = wasup_engine.latest_version().unwrap_or_else(|e| {
        eprintln!("Error getting latest version: {}", e);
        std::process::exit(1);
    });

    if current_version == latest_version {
        println!("Already up to date: {}", current_version);
    } else {
        let install_loc = wasup_engine
            .install_version(latest_version.to_string().as_str())
            .unwrap_or_else(|e| {
                eprintln!("Error evaluatin install location: {}", e);
                std::process::exit(1);
            });
        install(&install_loc).unwrap_or_else(|e| {
            eprintln!("Error installing latest version: {}", e);
            std::process::exit(1);
        });
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
