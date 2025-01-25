//! # Omniarchive API
//! This crate provides an API to scrape the Omniarchive website for Minecraft versions.
//!
//! It supports both client and server versions of the following categories:
//!
//! - PreClassic
//! - Classic
//! - Indev
//! - Infdev
//! - Alpha
//! - Beta
//!
//! ## Example
//! ```no_run
//! use omniarchive_api::{MinecraftVersionCategory};
//!
//! let list_of_version_urls =
//!     MinecraftVersionCategory::Alpha.download_index(None, false).await.unwrap();
//! ```

use std::{
    collections::HashSet,
    fmt::Display,
    rc::Rc,
    sync::{mpsc::Sender, Arc},
};

use html5ever::tendril::TendrilSink;
use markup5ever_rcdom::Node;
use ql_core::file_utils;

mod entry;
mod error;
pub use entry::ListEntry;
pub use error::WebScrapeError;

/// Represents a category of Minecraft versions.
#[derive(Clone, Debug, Copy)]
pub enum MinecraftVersionCategory {
    PreClassic,
    Classic,
    Indev,
    Infdev,
    Alpha,
    Beta,
}

impl Display for MinecraftVersionCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                MinecraftVersionCategory::PreClassic => "PreClassic",
                MinecraftVersionCategory::Classic => "Classic",
                MinecraftVersionCategory::Indev => "Indev",
                MinecraftVersionCategory::Infdev => "Infdev",
                MinecraftVersionCategory::Alpha => "Alpha",
                MinecraftVersionCategory::Beta => "Beta",
            }
        )
    }
}

impl MinecraftVersionCategory {
    /// Returns a list of all client versions.
    pub fn all_client() -> Vec<MinecraftVersionCategory> {
        vec![
            MinecraftVersionCategory::PreClassic,
            MinecraftVersionCategory::Classic,
            MinecraftVersionCategory::Indev,
            MinecraftVersionCategory::Infdev,
            MinecraftVersionCategory::Alpha,
            MinecraftVersionCategory::Beta,
        ]
    }

    /// Returns a list of all server versions.
    ///
    /// Note: PreClassic, Indev, and Infdev do not have server versions.
    pub fn all_server() -> Vec<MinecraftVersionCategory> {
        vec![
            MinecraftVersionCategory::Classic,
            MinecraftVersionCategory::Alpha,
            MinecraftVersionCategory::Beta,
        ]
    }

    /// Returns a URL to the `index.html` page of the category.
    ///
    /// # Arguments
    /// - `server`: Whether to get the server download.
    ///   `false` for client, `true` for server.
    pub fn get_index_url(&self, server: bool) -> String {
        format!(
            "https://vault.omniarchive.uk/archive/java/{}-{}/index.html",
            if server { "server" } else { "client" },
            match self {
                MinecraftVersionCategory::PreClassic => "preclassic",
                MinecraftVersionCategory::Classic => "classic",
                MinecraftVersionCategory::Indev => "indev",
                MinecraftVersionCategory::Infdev => "infdev",
                MinecraftVersionCategory::Alpha => "alpha",
                MinecraftVersionCategory::Beta => "beta",
            }
        )
    }

    /// Scrapes the Omniarchive `index.html` page for
    /// a list of version URLs.
    ///
    /// # Arguments
    /// - `progress`: An optional progress sender.
    /// - `download_server`: Whether to download server versions.
    ///   `false` for client, `true` for server.
    pub async fn download_index(
        &self,
        progress: Option<Arc<Sender<()>>>,
        download_server: bool,
    ) -> Result<Vec<String>, WebScrapeError> {
        let url = self.get_index_url(download_server);

        let client = reqwest::Client::new();

        let mut buffer = Vec::new();
        let mut deeper_buffer = Vec::new();
        let mut visited = HashSet::new();

        if let Some(progress) = &progress {
            progress.send(()).unwrap();
        }

        let mut i = 1;

        let links = self
            .get_links(
                &client,
                url,
                &mut buffer,
                &mut visited,
                progress.as_deref(),
                &mut deeper_buffer,
                &mut i,
            )
            .await?;

        Ok(links.into_iter().map(|n| n.1).collect())
    }

    #[allow(clippy::too_many_arguments)]
    async fn get_links(
        &self,
        client: &reqwest::Client,
        url: String,
        buffer: &mut Vec<String>,
        visited: &mut HashSet<String>,
        progress: Option<&Sender<()>>,
        deeper_buffer: &mut Vec<String>,
        i: &mut usize,
    ) -> Result<Vec<(MinecraftVersionCategory, String)>, WebScrapeError> {
        let mut links = self
            .scrape_links(client, &url, buffer, visited, progress, i)
            .await?;
        while !buffer.is_empty() || !deeper_buffer.is_empty() {
            for link in buffer.iter() {
                let scraped_links = self
                    .scrape_links(client, link, deeper_buffer, visited, progress, i)
                    .await?;
                links.extend_from_slice(&scraped_links);
            }
            buffer.clear();
            for link in deeper_buffer.iter() {
                let scraped_links = self
                    .scrape_links(client, link, buffer, visited, progress, i)
                    .await?;
                links.extend_from_slice(&scraped_links);
            }
            deeper_buffer.clear();
        }
        if let Some(progress) = &progress {
            progress.send(()).unwrap();
        }
        Ok(links)
    }

    pub async fn download_all(
        progress: Option<Arc<Sender<()>>>,
        download_server: bool,
    ) -> Result<Vec<(Self, String)>, WebScrapeError> {
        let mut links = Vec::new();
        let client = reqwest::Client::new();
        let mut visited = HashSet::new();

        let mut buffer = Vec::new();
        let mut deeper_buffer = Vec::new();

        let mut i = 1;

        for category in MinecraftVersionCategory::all_client().into_iter().rev() {
            let url = category.get_index_url(download_server);

            let versions = category
                .get_links(
                    &client,
                    url,
                    &mut buffer,
                    &mut visited,
                    progress.as_deref(),
                    &mut deeper_buffer,
                    &mut i,
                )
                .await?;

            links.extend(versions.into_iter().rev());
        }
        Ok(links)
    }

    async fn scrape_links(
        &self,
        client: &reqwest::Client,
        url: &str,
        deeper_buffer: &mut Vec<String>,
        visited: &mut HashSet<String>,
        progress: Option<&Sender<()>>,
        i: &mut usize,
    ) -> Result<Vec<(Self, String)>, WebScrapeError> {
        if !visited.insert(url.to_owned()) {
            return Ok(Vec::new());
        }

        let file = file_utils::download_file_to_string(client, url, true).await?;

        if let Some(progress) = progress {
            progress.send(()).unwrap();
        } else {
            eprintln!("- Progress ({self}): {i} / 20");
            *i += 1;
        }

        let dom = html5ever::parse_document(
            markup5ever_rcdom::RcDom::default(),
            html5ever::ParseOpts::default(),
        )
        .from_utf8()
        .read_from(&mut file.as_bytes())
        .unwrap();
        let e_html = find_elem(&dom.document, "html")?;
        let e_body = find_elem(&e_html, "body")?;
        let e_code = find_elem(&e_body, "code")?;

        let mut links = Vec::new();
        for n in e_code.children.borrow().iter().skip(1) {
            if let markup5ever_rcdom::NodeData::Element { name, attrs, .. } = &n.data {
                if name.local.to_string() != "a" {
                    continue;
                }
                if let Some(a) = attrs
                    .borrow()
                    .iter()
                    .find(|a| a.name.local.to_string() == "href")
                {
                    let link = a.value.to_string();
                    if link.ends_with("index.html") {
                        if link != "https://vault.omniarchive.uk/archive/java/index.html" {
                            deeper_buffer.push(link);
                        }
                    } else if !ends_with_extension(&link, ".exe") {
                        links.push((*self, link));
                    }
                }
            }
        }

        Ok(links)
    }
}

fn ends_with_extension(link: &str, extension: &str) -> bool {
    link.to_lowercase().ends_with(&extension.to_lowercase())
}

fn find_elem(dom: &Node, element_name: &str) -> Result<Rc<Node>, WebScrapeError> {
    dom.children
        .borrow()
        .iter()
        .find(|n| match &n.data {
            markup5ever_rcdom::NodeData::Element { name, .. } => {
                name.local.to_string() == element_name
            }
            _ => false,
        })
        .cloned()
        .ok_or(WebScrapeError::ElementNotFound(element_name.to_owned()))
}
