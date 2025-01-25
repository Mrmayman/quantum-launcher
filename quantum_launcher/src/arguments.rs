use colored::Colorize;
use ql_core::{
    err, file_utils,
    json::{instance_config::InstanceConfigJson, version::VersionDetails},
    LAUNCHER_VERSION_NAME,
};
use std::io::{stdout, Write};

use crate::launcher_state::get_entries;

pub struct ArgumentInfo {
    pub headless: bool,
    pub program: Option<String>,
}

pub fn process_args(
    args: &mut impl Iterator<Item = String>,
    info: &mut ArgumentInfo,
) -> Option<()> {
    let mut command = args.next()?;
    if info.program.is_none() {
        info.program = Some(command.clone());
        if let Some(arg) = args.next() {
            command = arg;
        } else {
            print_intro();
            return None;
        }
    }

    process_argument(args, &command, info);

    None
}

fn process_argument(
    args: &mut impl Iterator<Item = String>,
    command: &str,
    info: &mut ArgumentInfo,
) {
    match command {
        "--help" => cmd_print_help(info),
        "--version" => cmd_print_version(),
        "--command" => {
            info.headless = true;
            process_args(args, info);
        }
        "--list-instances" => {
            cmd_list_instances(args, info, "instances");
        }
        "--list-servers" => {
            cmd_list_instances(args, info, "servers");
        }
        "--list-available-versions" => {
            cmd_list_available_versions();
        }
        _ => {
            if command.starts_with("-") && !command.starts_with("--") {
                for (i, c) in command.chars().skip(1).enumerate() {
                    match c {
                        'c' => {
                            info.headless = true;
                            if i >= command.len() - 1 {
                                process_args(args, info);
                            }
                        }
                        'h' => cmd_print_help(info),
                        'v' => cmd_print_version(),
                        'l' => {
                            cmd_list_instances(args, info, "instances");
                        }
                        's' => {
                            cmd_list_instances(args, info, "servers");
                        }
                        'a' => {
                            cmd_list_available_versions();
                        }
                        _ => {
                            err!(
                                "Unknown character flag {c}! Type {} to see all the command-line flags.",
                                get_program_name(info, Some("--help"))
                            );
                            std::process::exit(1);
                        }
                    }
                }
            } else {
                err!(
                    "Unknown flag \"{command}\"! Type {} to see all the command-line flags.",
                    get_program_name(info, Some("--help"))
                );
                std::process::exit(1);
            }
        }
    }
}

fn cmd_list_available_versions() {
    eprintln!("Listing downloadable versions...");
    let versions = match tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(ql_instances::list_versions(None))
        .map_err(|err| err.to_string())
    {
        Ok(n) => n,
        Err(err) => {
            err!("Could not list versions: {err}");
            std::process::exit(1);
        }
    };

    let mut stdout = stdout().lock();
    for version in versions {
        writeln!(stdout, "{version}").unwrap();
    }
    std::process::exit(0);
}

fn cmd_print_version() {
    println!(
        "{}",
        format!("QuantumLauncher v{LAUNCHER_VERSION_NAME} - made by Mrmayman").bold()
    );
    std::process::exit(0);
}

fn cmd_print_help(info: &mut ArgumentInfo) {
    println!(
        r#"Usage: {}
    --help        -h : Prints a list of valid command line flags
    --version     -v : Prints the launcher version
    --command <ARGS> : Runs the launcher with the following
        -c             arguments and then exits (headless mode)

    --list-available-versions : Prints a list of available versions
        -a                      that can be used to create instances

    --list-instances  -l : Prints a list of instances
    --list-servers    -s : Prints a list of servers
        Subcommands: "name", "version", "type" (Vanilla/Fabric/Forge/...)
        For example:
            {1}
            {2}   name
            {1} name version
            {2}   version type name"#,
        get_program_name(info, Some("[FLAGS]/[-hvcals]")),
        get_program_name(info, Some("--list-instances")),
        get_program_name(info, Some("--list-servers")),
    );
    std::process::exit(0);
}

fn cmd_list_instances(
    args: &mut impl Iterator<Item = String>,
    info: &mut ArgumentInfo,
    dirname: &str,
) {
    enum PrintCmd {
        Name,
        Version,
        Type,
    }

    let instances = match tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(get_entries(dirname.to_owned(), false))
        .map_err(|err| err.to_string())
    {
        Ok(n) => n.0,
        Err(err) => {
            err!("Could not list instances: {err}");
            std::process::exit(1);
        }
    };

    let mut cmds: Vec<PrintCmd> = Vec::new();

    for _ in 0..3 {
        if let Some(subcommand) = args.next() {
            match subcommand.as_str() {
                "name" => cmds.push(PrintCmd::Name),
                "version" => cmds.push(PrintCmd::Version),
                "type" => cmds.push(PrintCmd::Type),
                _ => {
                    err!(
                        "Unknown subcommand! Type {} to see all the command-line flags.",
                        get_program_name(info, Some("--help"))
                    );
                    std::process::exit(1);
                }
            }
        }
    }

    if cmds.is_empty() {
        cmds.push(PrintCmd::Name);
    }

    for instance in instances {
        let mut has_printed = false;
        for cmd in &cmds {
            match cmd {
                PrintCmd::Name => {
                    if has_printed {
                        print!("\t");
                    }
                    print!("{instance}");
                }
                PrintCmd::Version => {
                    if has_printed {
                        print!("\t");
                    }
                    let launcher_dir = file_utils::get_launcher_dir().unwrap();
                    let instance_dir = launcher_dir.join(dirname).join(&instance);

                    let json = std::fs::read_to_string(instance_dir.join("details.json")).unwrap();
                    let json: VersionDetails = serde_json::from_str(&json).unwrap();

                    let config_json =
                        std::fs::read_to_string(instance_dir.join("config.json")).unwrap();
                    let config_json: InstanceConfigJson =
                        serde_json::from_str(&config_json).unwrap();

                    if let Some(omniarchive) = config_json.omniarchive {
                        print!("{}", omniarchive.name.split('/').last().unwrap());
                    } else {
                        print!("{}", json.id);
                    }
                }
                PrintCmd::Type => {
                    if has_printed {
                        print!("\t");
                    }
                    let launcher_dir = file_utils::get_launcher_dir().unwrap();
                    let instance_dir = launcher_dir.join(dirname).join(&instance);
                    let config_json =
                        std::fs::read_to_string(instance_dir.join("config.json")).unwrap();
                    let config_json: InstanceConfigJson =
                        serde_json::from_str(&config_json).unwrap();

                    print!("{}", config_json.mod_type);
                }
            }
            has_printed = true;
        }
        if has_printed {
            println!();
        }
    }
    std::process::exit(0);
}

fn get_program_name(info: &mut ArgumentInfo, argument: Option<&str>) -> String {
    let mut program = info
        .program
        .as_deref()
        .unwrap_or("quantum_launcher")
        .to_owned();
    if let Some(arg) = argument {
        program.push(' ');
        program.push_str(arg);
    }
    if cfg!(target_os = "windows") {
        program.clone()
    } else {
        program.yellow().to_string()
    }
}

pub fn print_intro() {
    const TEXT_WIDTH: u16 = 39;
    let (text, text_len_old) = get_side_text();

    const LOGO_WIDTH: u16 = 30;
    let logo = include_str!("../../assets/ascii/icon.txt");
    let logo_len: usize = logo.lines().count();

    let Ok(terminal_width) = crossterm::terminal::window_size() else {
        return;
    };

    // Helper function to pad lines to a fixed width
    fn pad_line(line: Option<&str>, width: usize) -> String {
        let line = line.unwrap_or_default();
        if line.len() < width {
            format!("{:<width$}", line, width = width)
        } else {
            line.to_owned()
        }
    }

    let mut stdout = std::io::stdout().lock();

    if terminal_width.columns > TEXT_WIDTH + LOGO_WIDTH {
        let lines_len = std::cmp::max(text.lines().count(), logo.lines().count());
        for i in 0..lines_len {
            let text_line = pad_line(text.lines().nth(i), TEXT_WIDTH as usize);
            let logo_line = pad_line(logo.lines().nth(i), LOGO_WIDTH as usize);
            if cfg!(target_os = "windows") || i >= logo_len {
                let _ = write!(stdout, "{logo_line} ");
            } else {
                let _ = write!(stdout, "{} ", logo_line.purple().bold());
            }
            if cfg!(target_os = "windows") || i >= text_len_old {
                let _ = write!(stdout, "{text_line}");
            } else {
                let _ = write!(stdout, "{}", text_line.bold());
            }
            let _ = writeln!(stdout);
        }
    } else if terminal_width.columns >= TEXT_WIDTH {
        if cfg!(target_os = "windows") {
            let _ = writeln!(stdout, "{logo}\n{text}");
        } else {
            let _ = writeln!(stdout, "{}\n{}", logo.purple().bold(), text.bold());
        }
    } else if terminal_width.columns >= LOGO_WIDTH {
        if cfg!(target_os = "windows") {
            let _ = writeln!(stdout, "{logo}");
        } else {
            let _ = writeln!(stdout, "{}", logo.purple().bold());
        }
    } else {
        let _ = writeln!(stdout, "Quantum Launcher {LAUNCHER_VERSION_NAME}");
    }
    let _ = writeln!(stdout);
}

fn get_side_text() -> (String, usize) {
    let mut text = include_str!("../../assets/ascii/text.txt").to_owned();
    let text_len_old = text.lines().count();

    let mut message = if cfg!(target_os = "windows") {
        "\n A simple, powerful Minecraft launcher".to_owned()
    } else {
        format!(
            "\n {}",
            "A simple, powerful Minecraft launcher".green().bold(),
        )
    };

    message.push_str("\n This window just shows debug info so\n feel free to ignore it\n\n ");

    let list_of_commands = if cfg!(target_os = "windows") {
        "For a list of commands type 'quantum_launcher.exe --help'".to_owned()
    } else {
        format!(
            "For a list of commands type\n {} {}",
            "./quantum_launcher".yellow().bold(),
            "--help".yellow()
        )
    };
    message.push_str(&list_of_commands);

    text.push_str(&message);

    (text, text_len_old)
}
