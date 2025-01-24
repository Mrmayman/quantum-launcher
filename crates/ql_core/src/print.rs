use std::{
    fs::OpenOptions,
    io::{BufWriter, Write},
};

use chrono::{Datelike, Timelike};

use crate::file_utils;

pub struct LoggingState {
    _thread: std::thread::JoinHandle<()>,
    sender: std::sync::mpsc::Sender<String>,
}

impl LoggingState {
    pub fn create() -> Option<LoggingState> {
        let launcher_dir = file_utils::get_launcher_dir().ok()?;

        let logs_dir = launcher_dir.join("logs");
        std::fs::create_dir_all(&logs_dir).ok()?;

        // Current date+time
        let now = chrono::Local::now();
        let log_file_name = format!(
            "{}-{}-{}-{}-{}-{}.log",
            now.year(),
            now.month(),
            now.day(),
            now.hour(),
            now.minute(),
            now.second()
        );
        let log_file_path = logs_dir.join(log_file_name);

        let file = OpenOptions::new()
            .create(true) // Create file if it doesn't exist
            .append(true) // Append to the file instead of overwriting
            .open(&log_file_path)
            .ok()?;

        let (sender, receiver) = std::sync::mpsc::channel::<String>();

        let thread = std::thread::spawn(move || {
            let mut writer = BufWriter::new(file);

            while let Ok(msg) = receiver.recv() {
                let _ = writer.write_all(msg.as_bytes());
                let _ = writer.flush();
            }
        });

        Some(LoggingState {
            _thread: thread,
            sender,
        })
    }

    pub fn write_str(&self, s: String) {
        self.sender.send(s.to_string()).ok();
    }
}

lazy_static::lazy_static! {
    pub static ref LOGGER: Option<LoggingState> =
        LoggingState::create();
}

pub fn print_to_file(msg: String) {
    if let Some(logger) = LOGGER.as_ref() {
        logger.write_str(msg);
    }
}

/// Print an informational message.
/// Saved to a log file.
#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        let plain_text = format!("[info] {}\n", format_args!($($arg)*));

        if cfg!(windows) {
            println!("{plain_text}")
        } else {
            println!("{} {}", colored::Colorize::yellow("[info]"), format_args!($($arg)*))
        }

        $crate::print::print_to_file(plain_text);
    };
}

/// Print an error message.
/// Saved to a log file.
#[macro_export]
macro_rules! err {
    ($($arg:tt)*) => {
        // Ugly hack to fix compiler error
        if true {
            let plain_text = format!("[error] {}\n", format_args!($($arg)*));

            if cfg!(windows) {
                eprintln!("{plain_text}")
            } else {
                eprintln!("{} {}", colored::Colorize::red("[error]"), format_args!($($arg)*))
            }

            $crate::print::print_to_file(plain_text);
        }
    };
}

/// Print a point message, ie. a small step in some process.
/// Saved to a log file.
#[macro_export]
macro_rules! pt {
    ($($arg:tt)*) => {
        let plain_text = format!("[plain] {}\n", format_args!($($arg)*));

        if cfg!(windows) {
            println!("- {}", format_args!($($arg)*))
        } else {
            println!("{} {}", colored::Colorize::bold("-"), format_args!($($arg)*))
        }

        $crate::print::print_to_file(plain_text);
    };
}
