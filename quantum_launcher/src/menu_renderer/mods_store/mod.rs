use iced::widget;
use ql_mod_manager::mod_manager::Entry;

use crate::{
    icon_manager,
    launcher_state::{
        ImageState, InstallModsMessage, ManageModsMessage, MenuModsDownload, Message,
    },
};

use super::{button_with_icon, Element};

mod html;
mod markdown;

impl MenuModsDownload {
    /// Renders the main store page, with the search bar,
    /// back button and list of searched mods.
    fn view_main<'a>(&'a self, images: &'a ImageState) -> Element<'a> {
        let mods_list = match self.results.as_ref() {
            Some(results) => widget::column(
                results
                    .hits
                    .iter()
                    .enumerate()
                    .map(|(i, hit)| self.view_mod_entry(i, hit, images)),
            ),
            None => {
                widget::column!(widget::text(if self.is_loading_search {
                    "Loading..."
                } else {
                    "Search something to get started..."
                }))
            }
        };
        widget::row!(
            widget::column!(
                widget::text_input("Search...", &self.query)
                    .on_input(|n| Message::InstallMods(InstallModsMessage::SearchInput(n))),
                if self.mods_download_in_progress.is_empty() {
                    widget::column!(button_with_icon(icon_manager::back(), "Back", 16)
                        .on_press(Message::ManageMods(ManageModsMessage::ScreenOpen)))
                } else {
                    // Mods are being installed. Can't back out.
                    // Show list of mods being installed.
                    widget::column!("Installing:", {
                        widget::column(self.mods_download_in_progress.iter().filter_map(|id| {
                            let search = self.results.as_ref()?;
                            let hit = search.hits.iter().find(|hit| &hit.project_id == id)?;
                            Some(widget::text!("- {}", hit.title).into())
                        }))
                    })
                },
            )
            .padding(10)
            .spacing(10)
            .width(200),
            widget::scrollable(mods_list.spacing(10).padding(10)),
        )
        .padding(10)
        .spacing(10)
        .into()
    }

    /// Renders a single mod entry (and button) in the search results.
    fn view_mod_entry<'a>(
        &'a self,
        i: usize,
        hit: &'a Entry,
        images: &'a ImageState,
    ) -> Element<'a> {
        widget::row!(
            widget::button(
                widget::row![icon_manager::download()]
                    .spacing(10)
                    .padding(5)
            )
            .height(70)
            .on_press_maybe(
                (!self.mods_download_in_progress.contains(&hit.project_id)
                    && !self.mod_index.mods.contains_key(&hit.project_id))
                .then_some(Message::InstallMods(InstallModsMessage::Download(i)))
            ),
            widget::button(
                widget::row!(
                    if let Some(icon) = images.bitmap.get(&hit.icon_url) {
                        widget::column!(widget::image(icon.clone()))
                    } else if let Some(icon) = images.svg.get(&hit.icon_url) {
                        widget::column!(widget::svg(icon.clone()).width(32))
                    } else {
                        widget::column!(widget::text(""))
                    },
                    widget::column!(
                        icon_manager::download_with_size(20),
                        widget::text(Self::format_downloads(hit.downloads)).size(12),
                    )
                    .align_x(iced::Alignment::Center)
                    .width(40)
                    .height(60)
                    .spacing(5),
                    widget::column!(
                        widget::text(&hit.title).size(16),
                        widget::text(safe_slice(&hit.description, 50)).size(12),
                    )
                    .spacing(5),
                    widget::horizontal_space()
                )
                .padding(5)
                .spacing(10),
            )
            .height(70)
            .on_press(Message::InstallMods(InstallModsMessage::Click(i)))
        )
        .spacing(5)
        .into()
    }

    pub fn view<'a>(&'a self, images: &'a ImageState) -> Element<'a> {
        // If we opened a mod (`self.opened_mod`) then
        // render the mod description page.
        // else render the main store page.
        let (Some(selection), Some(results)) = (&self.opened_mod, &self.results) else {
            return self.view_main(images);
        };
        let Some(hit) = results.hits.get(*selection) else {
            return self.view_main(images);
        };
        self.view_project_description(hit, images)
    }

    /// Renders the mod description page.
    fn view_project_description<'a>(
        &'a self,
        hit: &'a Entry,
        images: &'a ImageState,
    ) -> Element<'a> {
        // Parses the markdown description of the mod.
        let markdown_description = if let Some(info) = self.result_data.get(&hit.project_id) {
            widget::column!(Self::render_markdown(&info.body, images))
        } else {
            widget::column!(widget::text("Loading..."))
        };

        widget::scrollable(
            widget::column!(
                widget::row!(
                    button_with_icon(icon_manager::back(), "Back", 16)
                        .on_press(Message::InstallMods(InstallModsMessage::BackToMainScreen)),
                    button_with_icon(icon_manager::page(), "Open Mod Page", 16).on_press(
                        Message::CoreOpenDir(format!("https://modrinth.com/mod/{}", hit.slug))
                    ),
                    button_with_icon(icon_manager::save(), "Copy ID", 16)
                        .on_press(Message::CoreCopyText(hit.project_id.clone())),
                )
                .spacing(10),
                widget::row!(
                    if let Some(icon) = images.bitmap.get(&hit.icon_url) {
                        widget::column!(widget::image(icon.clone()))
                    } else if let Some(icon) = images.svg.get(&hit.icon_url) {
                        widget::column!(widget::svg(icon.clone()))
                    } else {
                        widget::column!(widget::text(""))
                    },
                    widget::text(&hit.title).size(24)
                )
                .spacing(10),
                widget::text(&hit.description).size(20),
                markdown_description
            )
            .padding(20)
            .spacing(20),
        )
        .into()
    }

    fn format_downloads(downloads: usize) -> String {
        if downloads < 999 {
            downloads.to_string()
        } else if downloads < 10000 {
            format!("{:.1}K", downloads as f32 / 1000.0)
        } else if downloads < 1_000_000 {
            format!("{}K", downloads / 1000)
        } else if downloads < 10_000_000 {
            format!("{:.1}M", downloads as f32 / 1_000_000.0)
        } else {
            format!("{}M", downloads / 1_000_000)
        }
    }
}

fn safe_slice(s: &str, max_len: usize) -> &str {
    let mut end = 0;
    for (i, _) in s.char_indices().take(max_len) {
        end = i;
    }
    if end == 0 {
        s
    } else {
        &s[..end]
    }
}
