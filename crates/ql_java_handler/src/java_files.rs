use std::collections::BTreeMap;

use serde::Deserialize;

#[derive(Deserialize)]
pub struct JavaFilesJson {
    pub files: BTreeMap<String, JavaFile>,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
#[allow(non_camel_case_types)]
pub enum JavaFile {
    file {
        downloads: JavaFileDownload,
        executable: bool,
    },
    directory {},
    link {
        // target: String,
    },
}

impl JavaFile {
    pub fn get_kind_name(&self) -> &'static str {
        match self {
            JavaFile::file { .. } => "file",
            JavaFile::directory {} => "directory",
            JavaFile::link { .. } => "symlink (TODO)",
        }
    }
}

#[derive(Deserialize)]
pub struct JavaFileDownload {
    pub lzma: Option<JavaFileDownloadDetails>,
    pub raw: JavaFileDownloadDetails,
}

#[derive(Deserialize)]
pub struct JavaFileDownloadDetails {
    // sha1: String,
    // size: usize,
    pub url: String,
}
