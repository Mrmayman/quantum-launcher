use std::{fmt::Display, time::Instant};

use image::ImageReader;
use ql_core::{err, file_utils, IoError, RequestError};
use serde::{Deserialize, Serialize};
use zip_extract::ZipError;

use crate::rate_limiter::RATE_LIMITER;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Search {
    pub hits: Vec<Entry>,
    pub offset: usize,
    pub limit: usize,
    pub total_hits: usize,
}

#[derive(Clone)]
pub struct ImageResult {
    pub url: String,
    pub image: Vec<u8>,
    pub is_svg: bool,
}

impl std::fmt::Debug for ImageResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ImageResult")
            .field("url", &self.url)
            .field("image", &format_args!("{} bytes", self.image.len()))
            .field("is_svg", &self.is_svg)
            .finish()
    }
}

impl Search {
    fn get_search_url(query: &Query) -> String {
        let mut url = "https://api.modrinth.com/v2/search?index=relevance&limit=100".to_owned();
        if !query.name.is_empty() {
            url.push_str("&query=");
            url.push_str(&query.name);
        }

        let mut filters: Vec<Vec<String>> = Vec::new();

        filters.push(vec!["project_type:mod".to_owned()]);

        if query.loaders.is_empty() {
            filters.push(vec![
                "categories:'forge'".to_owned(),
                "categories:'fabric'".to_owned(),
                "categories:'quilt'".to_owned(),
                "categories:'liteloader'".to_owned(),
                "categories:'modloader'".to_owned(),
                "categories:'rift'".to_owned(),
                "categories:'neoforge'".to_owned(),
            ]);
        } else {
            filters.push(
                query
                    .loaders
                    .iter()
                    .map(|loader| format!("categories:'{loader}'"))
                    .collect(),
            );
        }

        if query.open_source {
            filters.push(vec!["open_source:true".to_owned()]);
        }
        // if query.client_side {
        //     filters.push(vec!["client_side:required".to_owned()]);
        // }
        if query.server_side {
            filters.push(vec!["server_side:required".to_owned()]);
        }

        filters.push(
            query
                .versions
                .iter()
                .map(|version| format!("versions:{version}"))
                .collect(),
        );

        url.push_str("&facets=[");

        let num_filters = filters.len();
        for (idx, filter) in filters.iter().enumerate() {
            if !filter.is_empty() {
                url.push('[');
            }

            let num_subfilters = filter.len();
            for (sub_idx, subfilter) in filter.iter().enumerate() {
                url.push_str(&format!("\"{subfilter}\""));
                url.push(if sub_idx + 1 < num_subfilters {
                    ','
                } else {
                    ']'
                });
            }

            url.push(if idx + 1 < num_filters { ',' } else { ']' });
        }

        url
    }

    pub async fn search(query: Query) -> Result<(Self, Instant), ModError> {
        let _lock = RATE_LIMITER.lock().await;
        let instant = Instant::now();
        let url = Search::get_search_url(&query);
        // println!("searching: {url}");

        let client = reqwest::Client::new();
        let json = file_utils::download_file_to_string(&client, &url, true).await?;
        let json: Self = serde_json::from_str(&json)?;

        Ok((json, instant))
    }

    pub async fn search_w(query: Query) -> Result<(Self, Instant), String> {
        Self::search(query).await.map_err(|err| err.to_string())
    }

    pub async fn download_image(url: String, icon: bool) -> Result<ImageResult, String> {
        if url.starts_with("https://cdn.modrinth.com/") {
            // Does Modrinth CDN have a rate limit like their API?
            // I have no idea but from my testing it doesn't seem like they do.

            // let _lock = ql_instances::RATE_LIMITER.lock().await;
        }
        if url.is_empty() {
            return Err("url is empty".to_owned());
        }

        let client = reqwest::Client::new();
        let image = file_utils::download_file_to_bytes(&client, &url, true)
            .await
            .map_err(|err| format!("{url}: {err}"))?;

        if url.to_lowercase().ends_with(".svg") {
            return Ok(ImageResult {
                url,
                image,
                is_svg: true,
            });
        }

        if let Ok(text) = std::str::from_utf8(&image) {
            if text.starts_with("<svg") {
                return Ok(ImageResult {
                    url,
                    image,
                    is_svg: true,
                });
            }
        }

        let img = ImageReader::new(std::io::Cursor::new(image))
            .with_guessed_format()
            .map_err(|err| format!("{url}: {err}"))?
            .decode()
            .map_err(|err| format!("{url}: {err}"))?;

        let img = img.thumbnail(if icon { 32 } else { 240 }, 426);
        // let img = img.resize(32, 32, image::imageops::FilterType::Nearest);

        let mut buffer = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut buffer);
        img.write_to(&mut cursor, image::ImageFormat::Png)
            .map_err(|err| format!("{url}: {err}"))?;

        Ok(ImageResult {
            url,
            image: buffer,
            is_svg: false,
        })
    }
}

pub enum ModError {
    RequestError(RequestError),
    Serde(serde_json::Error),
    Io(IoError),
    NoCompatibleVersionFound,
    NoFilesFound,
    ZipIoError(std::io::Error, String),
    Zip(ZipError),
}

impl Display for ModError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "could not perform mod action: ")?;
        match self {
            ModError::RequestError(err) => write!(f, "(request) {err}"),
            ModError::Serde(err) => write!(f, "(json) {err}"),
            ModError::Io(err) => write!(f, "(io) {err}"),
            ModError::NoCompatibleVersionFound => {
                write!(f, "no compatible version found when downloading mod")
            }
            ModError::NoFilesFound => write!(f, "no files found for mod"),
            ModError::ZipIoError(err, path) => {
                write!(f, "couldn't add entry {path} to zip: {err}")
            }
            ModError::Zip(err) => write!(f, "(zip) {err}"),
        }
    }
}

impl From<RequestError> for ModError {
    fn from(value: RequestError) -> Self {
        Self::RequestError(value)
    }
}

impl From<ZipError> for ModError {
    fn from(value: ZipError) -> Self {
        Self::Zip(value)
    }
}

impl From<serde_json::Error> for ModError {
    fn from(value: serde_json::Error) -> Self {
        Self::Serde(value)
    }
}

impl From<IoError> for ModError {
    fn from(value: IoError) -> Self {
        Self::Io(value)
    }
}

pub struct Query {
    pub name: String,
    pub versions: Vec<String>,
    pub loaders: Vec<Loader>,
    pub server_side: bool,
    pub open_source: bool,
}

#[derive(Debug, Clone)]
pub enum Loader {
    Forge,
    Fabric,
    Quilt,
    Liteloader,
    Modloader,
    Rift,
    Neoforge,
    // Note: Modrinth doesn't support the below:
    OptiFine,
    Paper,
}

impl TryFrom<&str> for Loader {
    type Error = ();

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "Forge" => Ok(Loader::Forge),
            "Fabric" => Ok(Loader::Fabric),
            "Quilt" => Ok(Loader::Quilt),
            loader => {
                err!("Unknown loader: {loader}");
                Err(())
            }
        }
    }
}

impl Display for Loader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Loader::Forge => "forge",
                Loader::Fabric => "fabric",
                Loader::Quilt => "quilt",
                Loader::Liteloader => "liteloader",
                Loader::Modloader => "modloader",
                Loader::Rift => "rift",
                Loader::Neoforge => "neoforge",
                Loader::OptiFine => "optifine",
                Loader::Paper => "paper",
            }
        )
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Entry {
    pub project_id: String,
    pub project_type: String,
    pub slug: String,
    pub author: String,
    pub title: String,
    pub description: String,
    pub categories: Vec<String>,
    pub display_categories: Vec<String>,
    pub versions: Vec<String>,
    pub downloads: usize,
    pub follows: usize,
    pub icon_url: String,
    pub date_created: String,
    pub date_modified: String,
    pub latest_version: String,
    pub license: String,
    pub client_side: String,
    pub server_side: String,
    pub gallery: Vec<String>,
    pub featured_gallery: Option<String>,
    pub color: Option<usize>,
    pub thread_id: Option<String>,
    pub monetization_status: Option<String>,
}
