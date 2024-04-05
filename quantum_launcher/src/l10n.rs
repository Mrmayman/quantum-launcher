pub const ENGLISH: &[&str] = &[
    "Error",
    "Selected Java path contains invalid characters",
    "Could not open launcher config",
    "Selected Java path not found.",
    "No",
    "Yes, delete my data",
    "All your data, including worlds will be lost.",
    "Are you SURE you want to DELETE the Instance",
];

pub enum Entry {
    Error,
    InvalidCharsInJavaPath,
    CouldNotOpenLauncherConfig,
    SelectedJavaPathNotFound,
    No,
    YesDeleteMyData,
    AllYourDataIncludingWorldsWillBeLost,
    AreYouSUREYouWantToDeleteTheInstance,
}

#[macro_export]
macro_rules! l10n {
    ( $lang:ident, $entry:ident ) => {
        $crate::l10n::$lang[$crate::l10n::Entry::$entry as usize]
    };
}
