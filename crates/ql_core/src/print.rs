use std::{
    fs::OpenOptions,
    io::{BufWriter, Write},
    sync::Mutex,
};

use chrono::{Datelike, Timelike};

use crate::file_utils;

pub struct LoggingState {
    writer: BufWriter<std::fs::File>,
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

        Some(LoggingState {
            writer: BufWriter::new(file),
        })
    }

    pub fn write_str(&mut self, s: &str) {
        let _ = self.writer.write_all(s.as_bytes());
        let _ = self.writer.flush();
    }
}

lazy_static::lazy_static! {
    pub static ref LOGGER: Mutex<Option<LoggingState>> =
        Mutex::new(LoggingState::create());
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

        {
            let mut logger = $crate::print::LOGGER.lock().unwrap();
            if let Some(logger) = &mut *logger {
                logger.write_str(&plain_text);
            }
        }
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

            {
                let mut logger = $crate::print::LOGGER.lock().unwrap();
                if let Some(logger) = &mut *logger {
                    logger.write_str(&plain_text);
                }
            }
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

        {
            let mut logger = $crate::print::LOGGER.lock().unwrap();
            if let Some(logger) = &mut *logger {
                logger.write_str(&plain_text);
            }
        }
    };
}
