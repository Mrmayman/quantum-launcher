//! # Omniarchive API
//! This crate provides an API to scrape the Omniarchive website
//! for Minecraft versions.
//!
//! It's used by [Quantum Launcher](https://mrmayman.github.io/quantumlauncher).
//!
//! **Not recommended to use in your own projects, but go ahead if you wish**
//!
//! It supports both client and server versions of the following categories:
//!
//! - Pre-Classic
//! - Classic
//! - Indev
//! - Infdev
//! - Alpha
//! - Beta
//!
//! ## Example
//! ```no_run
//! # async fn get() {
//! use omniarchive_api::{MinecraftVersionCategory};
//!
//! let list_of_version_urls =
//!     MinecraftVersionCategory::Alpha.download_index(None, false).await.unwrap();
//! # }
//! ```

use std::{
    collections::HashSet,
    fmt::Display,
    rc::Rc,
    sync::{mpsc::Sender, Arc, Mutex},
};

use html5ever::tendril::TendrilSink;
use markup5ever_rcdom::Node;
use ql_core::{do_jobs, file_utils};

mod entry;
mod error;
pub use entry::ListEntry;
pub use error::{ListError, WebScrapeError};

pub struct VersionEntry {
    pub url: String,
    pub category: MinecraftVersionCategory,
}

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
    pub const ALL_CLIENT: [Self; 6] = [
        MinecraftVersionCategory::PreClassic,
        MinecraftVersionCategory::Classic,
        MinecraftVersionCategory::Indev,
        MinecraftVersionCategory::Infdev,
        MinecraftVersionCategory::Alpha,
        MinecraftVersionCategory::Beta,
    ];

    // Note: Pre-Classic, Indev, and Infdev
    // don't have server versions
    // (unless you use unofficial mods)
    pub const ALL_SERVER: [Self; 3] = [
        MinecraftVersionCategory::Classic,
        MinecraftVersionCategory::Alpha,
        MinecraftVersionCategory::Beta,
    ];

    /// Returns a URL to the `index.html` page of the category.
    ///
    /// # Arguments
    /// - `server`: Whether to get the server download.
    ///   `false` for client, `true` for server.
    #[must_use]
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
    ///
    /// # Errors
    /// If:
    /// - an `index.html` file was unable to be downloaded
    ///   (internet or server issue)
    /// - a required html element was not found in the data
    pub async fn download_index(
        &self,
        progress: Option<Arc<Sender<()>>>,
        download_server: bool,
    ) -> Result<Vec<String>, WebScrapeError> {
        let url = self.get_index_url(download_server);

        let buffer = Mutex::new(Vec::new());
        let deeper_buffer = Mutex::new(Vec::new());
        let visited = Mutex::new(HashSet::new());

        if let Some(progress) = &progress {
            _ = progress.send(());
        }

        let i = Mutex::new(1);

        let links = self
            .get_links(
                url,
                &buffer,
                &visited,
                progress.as_deref(),
                &deeper_buffer,
                &i,
            )
            .await?;

        Ok(links.into_iter().map(|n| n.url).collect())
    }

    async fn get_links(
        &self,
        url: String,
        buffer: &Mutex<Vec<String>>,
        visited: &Mutex<HashSet<String>>,
        progress: Option<&Sender<()>>,
        deeper_buffer: &Mutex<Vec<String>>,
        i: &Mutex<usize>,
    ) -> Result<Vec<VersionEntry>, WebScrapeError> {
        let mut links = self.scrape_links(url, buffer, visited, progress, i).await?;

        let mut is_buffer_empty = buffer.lock().unwrap().is_empty();

        while !is_buffer_empty {
            {
                let buffer_new = buffer.lock().unwrap().clone();
                let new = do_jobs(
                    buffer_new
                        .into_iter()
                        .map(|link| self.scrape_links(link, deeper_buffer, visited, progress, i)),
                )
                .await?;
                buffer.lock().unwrap().clear();
                links.extend(new.into_iter().flatten());
            }

            {
                let deeper_buffer_clone = deeper_buffer.lock().unwrap().clone();
                let new = do_jobs(
                    deeper_buffer_clone
                        .into_iter()
                        .map(|link| self.scrape_links(link, buffer, visited, progress, i)),
                )
                .await?;
                deeper_buffer.lock().unwrap().clear();
                links.extend(new.into_iter().flatten());
            }

            is_buffer_empty = buffer.lock().unwrap().is_empty();
        }
        if let Some(progress) = &progress {
            _ = progress.send(());
        }
        Ok(links)
    }

    async fn scrape_links(
        &self,
        url: String,
        deeper_buffer: &Mutex<Vec<String>>,
        visited: &Mutex<HashSet<String>>,
        progress: Option<&Sender<()>>,
        i: &Mutex<usize>,
    ) -> Result<Vec<VersionEntry>, WebScrapeError> {
        if !visited.lock().unwrap().insert(url.clone()) {
            return Ok(Vec::new());
        }

        let file = file_utils::download_file_to_string(&url, true).await?;

        if let Some(progress) = progress {
            progress.send(()).unwrap();
        } else {
            let mut i = i.lock().unwrap();
            eprintln!("- Progress ({self}): {i} / 20");
            *i += 1;
        }

        let dom = html5ever::parse_document(
            markup5ever_rcdom::RcDom::default(),
            html5ever::ParseOpts::default(),
        )
        .from_utf8()
        .read_from(&mut file.as_bytes())
        // Will not panic as `file` is in-memory and fully readable
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
                    let url = a.value.to_string();
                    if url.ends_with("index.html") {
                        if url != "https://vault.omniarchive.uk/archive/java/index.html" {
                            deeper_buffer.lock().unwrap().push(url);
                        }
                    } else if !ends_with_extension(&url, ".exe") {
                        links.push(VersionEntry {
                            category: *self,
                            url,
                        });
                    }
                }
            }
        }

        Ok(links)
    }
}

/// Batch-downloads the version list for all versions.
///
/// If you want fine grained control over which category
/// of versions to download, check out
/// `MinecraftVersionCategory::download_index`
///
/// # Arguments
/// - `progress`: An optional progress sender.
/// - `download_server`: Whether to download server versions.
///   `false` for client, `true` for server.
///
/// # Errors
/// If:
/// - an `index.html` file was unable to be downloaded
///   (internet or server issue)
/// - a required html element was not found in the data
pub async fn download_all(
    progress: Option<Arc<Sender<()>>>,
    download_server: bool,
) -> Result<Vec<VersionEntry>, WebScrapeError> {
    let mut links = Vec::new();
    let visited = Mutex::new(HashSet::new());

    let buffer = Mutex::new(Vec::new());
    let deeper_buffer = Mutex::new(Vec::new());

    let i = Mutex::new(1);

    for category in MinecraftVersionCategory::ALL_CLIENT.into_iter().rev() {
        let url = category.get_index_url(download_server);

        let versions = category
            .get_links(
                url,
                &buffer,
                &visited,
                progress.as_deref(),
                &deeper_buffer,
                &i,
            )
            .await?;

        links.extend(versions.into_iter().rev());
    }
    Ok(links)
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
