#[macro_export]
macro_rules! info {
    ($($arg:tt)*) => {
        println!("{} {}", colored::Colorize::yellow("[info]"), format_args!($($arg)*))
    };
}
