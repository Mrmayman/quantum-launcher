pub const ENGLISH: &[&str] = &[
    "Error",
    "Selected Java path contains invalid characters",
    "Could not open launcher config",
    "Selected Java path not found.",
];

pub enum Entry {
    Error,
    InvalidCharsInJavaPath,
    CouldNotOpenLauncherConfig,
    SelectedJavaPathNotFound,
}

#[macro_export]
macro_rules! l10n {
    ( $lang:ident, $entry:ident ) => {
        crate::l10n::$lang[crate::l10n::Entry::$entry as usize]
    };
}
