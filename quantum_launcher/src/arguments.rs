use clap::Command;
use colored::Colorize;
use ql_core::{
    err, file_utils,
    json::{instance_config::InstanceConfigJson, version::VersionDetails},
    LAUNCHER_VERSION_NAME,
};
use std::io::Write;

use crate::{launcher_state::get_entries, menu_renderer::DISCORD};

pub fn command() -> Command {
    Command::new(if cfg!(target_os = "windows") {
        ".\\quantum_launcher.exe"
    } else {
        "./quantum_launcher"
    })
    .arg_required_else_help(false)
    .author("Mrmayman")
    .version(LAUNCHER_VERSION_NAME)
    .long_about(long_about())
    .subcommand(
        get_list_instance_subcommands("list-instances".to_owned())
            .short_flag('l')
            .about("Lists all installed Minecraft instances")
            .long_about("Lists all installed Minecraft instances. Can be paired with hyphen-separated-flags like name-loader, name-version, loader-name-version"),
    )
    .subcommand(
        get_list_instance_subcommands("list-servers".to_owned())
            .short_flag('s')
            .about("Lists all installed Minecraft servers")
            .long_about("Lists all installed Minecraft servers. Can be paired with hyphen-separated-flags like name-loader, name-version, loader-name-version"),
    )
    .subcommand(Command::new("list-available-versions").short_flag('a').about("Lists all downloadable versions, downloading a list from Mojang/Omniarchive"))
    .subcommand(Command::new("--no-sandbox").hide(true)) // This one doesn't do anything, but on Windows i686 it's automatically passed?
}

fn get_list_instance_subcommands(name: String) -> Command {
    Command::new(name)
        // May god forgive me for what I'm about to do
        .subcommand(Command::new("name"))
        .subcommand(Command::new("version"))
        .subcommand(Command::new("loader"))
        .subcommand(Command::new("name-version"))
        .subcommand(Command::new("name-loader"))
        .subcommand(Command::new("version-name"))
        .subcommand(Command::new("version-loader"))
        .subcommand(Command::new("loader-name"))
        .subcommand(Command::new("loader-version"))
        .subcommand(Command::new("name-version-loader"))
        .subcommand(Command::new("name-loader-version"))
        .subcommand(Command::new("version-name-loader"))
        .subcommand(Command::new("version-loader-name"))
        .subcommand(Command::new("loader-name-version"))
        .subcommand(Command::new("loader-version-name"))
}

pub fn cmd_list_available_versions() {
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

    let mut stdout = std::io::stdout().lock();
    for version in versions {
        writeln!(stdout, "{version}").unwrap();
    }
}

pub fn long_about() -> String {
    format!(
        r"
QuantumLauncher: A simple, powerful Minecraft launcher by Mrmayman

Website: https://mrmayman.github.io/quantumlauncher
Github : https://github.com/Mrmayman/quantum-launcher
Discord: {DISCORD}
"
    )
}

pub enum PrintCmd {
    Name,
    Version,
    Loader,
}

pub fn cmd_list_instances(cmds: Vec<PrintCmd>, dirname: &str) {
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
                    let launcher_dir = file_utils::get_launcher_dir_s().unwrap();
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
                PrintCmd::Loader => {
                    if has_printed {
                        print!("\t");
                    }
                    let launcher_dir = file_utils::get_launcher_dir_s().unwrap();
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
}

/// Prints the "intro" to the screen
/// consisting of the **ASCII art logo**, as well as
/// **stylised text saying "QuantumLauncher"**
///
/// The actual data is `include_str!()`ed from
/// - `assets/ascii/icon.txt` for the ASCII art
/// - `assets/ascii/text.txt` for the text logo
///
/// The other files in `assets/ascii` are unused.
pub fn print_intro() {
    const TEXT_WIDTH: u16 = 39;

    const LOGO: &str = include_str!("../../assets/ascii/icon.txt");
    const LOGO_WIDTH: u16 = 30;

    // Helper function to pad lines to a fixed width
    fn pad_line(line: Option<&str>, width: usize) -> String {
        let line = line.unwrap_or_default();
        if line.len() < width {
            format!("{line:<width$}")
        } else {
            line.to_owned()
        }
    }

    let (text, text_len_old) = get_side_text();

    let logo_len: usize = LOGO.lines().count();

    let Some((terminal_size::Width(width), _)) = terminal_size::terminal_size() else {
        return;
    };

    let mut stdout = std::io::stdout().lock();

    // Ok, this code is uncomfortably ugly but bear with me...
    if width > TEXT_WIDTH + LOGO_WIDTH {
        // Screen large enough for Text and Logo
        // to fit side-by-side
        let lines_len = std::cmp::max(text.lines().count(), LOGO.lines().count());
        for i in 0..lines_len {
            let text_line = pad_line(text.lines().nth(i), TEXT_WIDTH as usize);
            let logo_line = pad_line(LOGO.lines().nth(i), LOGO_WIDTH as usize);
            if cfg!(target_os = "windows") || i >= logo_len {
                _ = write!(stdout, "{logo_line} ");
            } else {
                _ = write!(stdout, "{} ", logo_line.purple().bold());
            }
            if cfg!(target_os = "windows") || i >= text_len_old {
                _ = write!(stdout, "{text_line}");
            } else {
                _ = write!(stdout, "{}", text_line.bold());
            }
            _ = writeln!(stdout);
        }
    } else if width >= TEXT_WIDTH {
        // Screen only large enough for
        // Text and Logo to fit one after another
        // vertically
        if cfg!(target_os = "windows") {
            _ = writeln!(stdout, "{LOGO}\n{text}");
        } else {
            _ = writeln!(stdout, "{}\n{}", LOGO.purple().bold(), text.bold());
        }
    } else if width >= LOGO_WIDTH {
        // Screen only large enough for Logo,
        // not text
        if cfg!(target_os = "windows") {
            _ = writeln!(stdout, "{LOGO}");
        } else {
            _ = writeln!(stdout, "{}", LOGO.purple().bold());
        }
    } else {
        // Screen is too tiny
        _ = writeln!(stdout, "Quantum Launcher {LAUNCHER_VERSION_NAME}");
    }
    _ = writeln!(stdout);
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
