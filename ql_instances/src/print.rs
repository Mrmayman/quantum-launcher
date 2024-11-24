#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        if cfg!(windows) {
            println!("[info] {}", format_args!($($arg)*))
        } else {
            println!("{} {}", colored::Colorize::yellow("[info]"), format_args!($($arg)*))
        }
    };
}

#[macro_export]
macro_rules! err {
    ($($arg:tt)*) => {
        if cfg!(windows) {
            eprintln!("[error] {}", format_args!($($arg)*))
        } else {
            eprintln!("{} {}", colored::Colorize::red("[error]"), format_args!($($arg)*))
        }
    };
}

#[macro_export]
macro_rules! pt {
    ($($arg:tt)*) => {
        if cfg!(windows) {
            println!("- {}", format_args!($($arg)*))
        } else {
            println!("{} {}", colored::Colorize::bold("-"), format_args!($($arg)*))
        }
    };
}
