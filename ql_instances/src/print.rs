#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        println!("{} {}", colored::Colorize::yellow("[info]"), format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! err {
    ($($arg:tt)*) => {
        eprintln!("{} {}", colored::Colorize::red("[error]"), format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! pt {
    ($($arg:tt)*) => {
        println!("{} {}", colored::Colorize::bold("-"), format_args!($($arg)*))
    };
}
