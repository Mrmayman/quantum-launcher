use std::fmt::Display;

/// An enum representing the progress in downloading
/// a Minecraft instance.
///
/// # Order
/// 1) Manifest Json
/// 2) Version Json
/// 3) Logging config
/// 4) Jar
/// 5) Libraries
/// 6) Assets
#[derive(Debug, Clone)]
pub enum DownloadProgress {
    Started,
    DownloadingJsonManifest,
    DownloadingVersionJson,
    DownloadingAssets { progress: usize, out_of: usize },
    DownloadingLibraries { progress: usize, out_of: usize },
    DownloadingJar,
    DownloadingLoggingConfig,
}

impl Display for DownloadProgress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DownloadProgress::Started => write!(f, "Started."),
            DownloadProgress::DownloadingJsonManifest => write!(f, "Downloading Manifest JSON."),
            DownloadProgress::DownloadingVersionJson => write!(f, "Downloading Version JSON."),
            DownloadProgress::DownloadingAssets { progress, out_of } => {
                write!(f, "Downloading asset {progress} / {out_of}.")
            }
            DownloadProgress::DownloadingLibraries { progress, out_of } => {
                write!(f, "Downloading library {progress} / {out_of}.")
            }
            DownloadProgress::DownloadingJar => write!(f, "Downloading Game Jar file."),
            DownloadProgress::DownloadingLoggingConfig => write!(f, "Downloading logging config."),
        }
    }
}

impl From<DownloadProgress> for f32 {
    fn from(val: DownloadProgress) -> Self {
        match val {
            DownloadProgress::Started => 0.0,
            DownloadProgress::DownloadingJsonManifest => 0.2,
            DownloadProgress::DownloadingVersionJson => 0.5,
            DownloadProgress::DownloadingAssets {
                progress: progress_num,
                out_of,
            } => (progress_num as f32 * 8.0 / out_of as f32) + 2.0,
            DownloadProgress::DownloadingLibraries {
                progress: progress_num,
                out_of,
            } => (progress_num as f32 / out_of as f32) + 1.0,
            DownloadProgress::DownloadingJar => 1.0,
            DownloadProgress::DownloadingLoggingConfig => 0.7,
        }
    }
}
