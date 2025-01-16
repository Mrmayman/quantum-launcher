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

impl From<&DownloadProgress> for f32 {
    fn from(val: &DownloadProgress) -> Self {
        match val {
            DownloadProgress::DownloadingJsonManifest => 0.1,
            DownloadProgress::DownloadingVersionJson => 0.3,
            DownloadProgress::DownloadingLoggingConfig => 0.5,
            DownloadProgress::DownloadingJar => 0.7,
            DownloadProgress::DownloadingLibraries {
                progress: progress_num,
                out_of,
            } => (*progress_num as f32 / *out_of as f32) + 1.0,
            DownloadProgress::DownloadingAssets {
                progress: progress_num,
                out_of,
            } => (*progress_num as f32 * 8.0 / *out_of as f32) + 2.0,
        }
    }
}

pub struct GenericProgress {
    pub done: usize,
    pub total: usize,
    pub message: Option<String>,
    pub has_finished: bool,
}

impl Default for GenericProgress {
    fn default() -> Self {
        Self {
            done: 0,
            total: 1,
            message: None,
            has_finished: false,
        }
    }
}

impl GenericProgress {
    pub fn finished() -> Self {
        Self {
            has_finished: true,
            done: 1,
            total: 1,
            message: None,
        }
    }
}

pub trait Progress {
    fn get_num(&self) -> f32;
    fn get_message(&self) -> Option<String>;
    fn total() -> f32;
}

impl Progress for DownloadProgress {
    fn get_num(&self) -> f32 {
        f32::from(self)
    }

    fn get_message(&self) -> Option<String> {
        Some(self.to_string())
    }

    fn total() -> f32 {
        10.0
    }
}

impl Progress for GenericProgress {
    fn get_num(&self) -> f32 {
        self.done as f32 / self.total as f32
    }

    fn get_message(&self) -> Option<String> {
        self.message.clone()
    }

    fn total() -> f32 {
        1.0
    }
}
