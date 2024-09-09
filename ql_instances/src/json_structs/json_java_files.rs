use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct JavaFilesJson {
    pub files: BTreeMap<String, JavaFile>,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "type")]
#[allow(non_camel_case_types)]
pub enum JavaFile {
    file {
        downloads: JavaFileDownload,
        executable: bool,
    },
    directory {},
    link {
        target: String,
    },
}

#[derive(Serialize, Deserialize)]
pub struct JavaFileDownload {
    pub lzma: Option<JavaFileDownloadDetails>,
    pub raw: JavaFileDownloadDetails,
}

#[derive(Serialize, Deserialize)]
pub struct JavaFileDownloadDetails {
    pub sha1: String,
    pub size: usize,
    pub url: String,
}
