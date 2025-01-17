use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    sync::Mutex,
};

use comrak::nodes::NodeValue;
use iced::widget::{self, image::Handle};
use ql_mod_manager::mod_manager::Entry;

use crate::{
    icon_manager,
    launcher_state::{InstallModsMessage, ManageModsMessage, MenuModsDownload, Message},
};

use super::{button_with_icon, Element};

macro_rules! todoh {
    ($desc:expr) => {
        widget::column!(widget::text(concat!("[todo: ", $desc, "]"))).into()
    };
}

impl MenuModsDownload {
    /// Renders the main store page, with the search bar,
    /// back button and list of searched mods.
    fn view_main(
        &self,
        icons_bitmap: &HashMap<String, Handle>,
        icons_svg: &HashMap<String, widget::svg::Handle>,
    ) -> Element {
        let mods_list = match self.results.as_ref() {
            Some(results) => widget::column(
                results
                    .hits
                    .iter()
                    .enumerate()
                    .map(|(i, hit)| self.view_mod_entry(i, hit, icons_bitmap, icons_svg)),
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
                    widget::column!(button_with_icon(icon_manager::back(), "Back")
                        .on_press(Message::ManageMods(ManageModsMessage::ScreenOpen)))
                } else {
                    // Mods are being installed. Can't back out.
                    // Show list of mods being installed.
                    widget::column!("Installing:", {
                        widget::column(self.mods_download_in_progress.iter().filter_map(|id| {
                            let search = self.results.as_ref()?;
                            let hit = search.hits.iter().find(|hit| &hit.project_id == id)?;
                            Some(widget::text(format!("- {}", hit.title)).into())
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
    fn view_mod_entry(
        &self,
        i: usize,
        hit: &Entry,
        icons_bitmap: &HashMap<String, Handle>,
        icons_svg: &HashMap<String, widget::svg::Handle>,
    ) -> Element {
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
                    if let Some(icon) = icons_bitmap.get(&hit.icon_url) {
                        widget::column!(widget::image(icon.clone()))
                    } else if let Some(icon) = icons_svg.get(&hit.icon_url) {
                        widget::column!(widget::svg(icon.clone()).width(32))
                    } else {
                        widget::column!(widget::text(""))
                    },
                    widget::column!(
                        icon_manager::download_with_size(20),
                        widget::text(Self::format_downloads(hit.downloads)).size(12),
                    )
                    .align_items(iced::Alignment::Center)
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

    pub fn view<'a>(
        &'a self,
        icons_bitmap: &'a HashMap<String, Handle>,
        icons_svg: &'a HashMap<String, widget::svg::Handle>,
        images_to_load: &'a Mutex<HashSet<String>>,
    ) -> Element<'a> {
        // If we opened a mod (`self.opened_mod`) then
        // render the mod description page.
        // else render the main store page.
        let (Some(selection), Some(results)) = (&self.opened_mod, &self.results) else {
            return self.view_main(icons_bitmap, icons_svg);
        };
        let Some(hit) = results.hits.get(*selection) else {
            return self.view_main(icons_bitmap, icons_svg);
        };
        self.view_project_description(hit, images_to_load, icons_bitmap, icons_svg)
    }

    /// Renders the mod description page.
    fn view_project_description<'a>(
        &'a self,
        hit: &'a Entry,
        images_to_load: &'a Mutex<HashSet<String>>,
        icons_bitmap: &'a HashMap<String, Handle>,
        icons_svg: &'a HashMap<String, widget::svg::Handle>,
    ) -> Element<'a> {
        // Parses the markdown description of the mod.
        let markdown_description = if let Some(info) = self.result_data.get(&hit.project_id) {
            widget::column!(Self::parse_markdown(
                &info.body,
                images_to_load,
                icons_bitmap,
                icons_svg
            ))
        } else {
            widget::column!(widget::text("Loading..."))
        };

        widget::scrollable(
            widget::column!(
                widget::row!(
                    button_with_icon(icon_manager::back(), "Back")
                        .on_press(Message::InstallMods(InstallModsMessage::BackToMainScreen)),
                    button_with_icon(icon_manager::page(), "Open Mod Page").on_press(
                        Message::CoreOpenDir(format!("https://modrinth.com/mod/{}", hit.slug))
                    ),
                    button_with_icon(icon_manager::save(), "Copy ID")
                        .on_press(Message::CoreCopyText(hit.project_id.clone())),
                )
                .spacing(10),
                widget::row!(
                    if let Some(icon) = icons_bitmap.get(&hit.icon_url) {
                        widget::column!(widget::image(icon.clone()))
                    } else if let Some(icon) = icons_svg.get(&hit.icon_url) {
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

    pub fn parse_markdown<'a>(
        markdown: &'a str,
        images_to_load: &'a Mutex<HashSet<String>>,
        images_bitmap: &'a HashMap<String, Handle>,
        images_svg: &'a HashMap<String, widget::svg::Handle>,
    ) -> Element<'a> {
        let arena = comrak::Arena::new();
        let root = comrak::parse_document(&arena, markdown, &comrak::Options::default());

        let mut element = widget::column!().into();

        Self::render_element(
            root,
            0,
            &mut element,
            images_to_load,
            images_bitmap,
            images_svg,
        );
        element
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

    fn render_element<'arena, 'element: 'arena>(
        md: &'element comrak::arena_tree::Node<'arena, RefCell<comrak::nodes::Ast>>,
        heading_size: usize,
        element: &mut Element<'static>,
        images_to_load: &Mutex<HashSet<String>>,
        images_bitmap: &HashMap<String, Handle>,
        images_svg: &HashMap<String, widget::svg::Handle>,
    ) {
        let data = md.data.borrow();
        *element = match &data.value {
            NodeValue::Document => {
                render_children(md, 0, images_to_load, images_bitmap, images_svg)
                    .spacing(10)
                    .into()
            }
            NodeValue::Heading(node_heading) => {
                let heading_size = node_heading.level as usize;
                render_children(md, heading_size, images_to_load, images_bitmap, images_svg).into()
            }
            NodeValue::Text(text) => widget::text(text)
                .size(if heading_size > 0 {
                    36 - (heading_size * 4)
                } else {
                    16
                } as u16)
                .into(),
            NodeValue::Paragraph => {
                render_children(md, 0, images_to_load, images_bitmap, images_svg).into()
            }
            NodeValue::Link(node_link) => {
                render_link(md, images_to_load, images_bitmap, images_svg, node_link)
            }
            NodeValue::FrontMatter(_) => todoh!("front matter"),
            NodeValue::BlockQuote => todoh!("block quote"),
            NodeValue::List(_list) => {
                render_children(md, 0, images_to_load, images_bitmap, images_svg)
                    .spacing(10)
                    .into()
            }
            NodeValue::Item(item) => {
                render_list_item(md, item, images_to_load, images_bitmap, images_svg)
            }
            NodeValue::DescriptionList => todoh!("description list"),
            NodeValue::DescriptionItem(_) => todoh!("description item"),
            NodeValue::DescriptionTerm => todoh!("description term"),
            NodeValue::DescriptionDetails => todoh!("description details"),
            NodeValue::CodeBlock(block) => widget::container(
                widget::text(&block.literal).font(iced::Font::with_name("JetBrains Mono")),
            )
            .into(),
            NodeValue::HtmlBlock(node_html_block) => Self::render_html(
                &node_html_block.literal,
                images_to_load,
                images_bitmap,
                images_svg,
            ),
            NodeValue::ThematicBreak => widget::row!(widget::text("_____").size(20))
                .align_items(iced::Alignment::Center)
                .into(),
            NodeValue::FootnoteDefinition(_) => todoh!("footnote definition"),
            NodeValue::Table(_) => widget::column!(widget::text("[todo: table]")).into(),
            NodeValue::TableRow(_) => widget::column!(widget::text("[todo: table row]")).into(),
            NodeValue::TableCell => widget::column!(widget::text("[todo: table cell]")).into(),
            NodeValue::TaskItem(_) => widget::column!(widget::text("[todo: task item]")).into(),
            NodeValue::SoftBreak | NodeValue::LineBreak => widget::column!().into(),
            NodeValue::Code(code) => widget::text(&code.literal)
                .font(iced::Font::with_name("JetBrains Mono"))
                .into(),
            NodeValue::HtmlInline(html) => {
                Self::render_html(html, images_to_load, images_bitmap, images_svg)
            }
            NodeValue::Strong | NodeValue::Emph => widget::column(md.children().map(|n| {
                let mut element = widget::column!().into();
                Self::render_element(
                    n,
                    4,
                    &mut element,
                    images_to_load,
                    images_bitmap,
                    images_svg,
                );
                element
            }))
            .into(),
            NodeValue::Strikethrough => todoh!("strikethrough"),
            NodeValue::Superscript => todoh!("superscript"),
            NodeValue::Image(link) => {
                if let Some(image) = images_bitmap.get(&link.url) {
                    widget::image(image.clone()).width(300).into()
                } else if let Some(image) = images_svg.get(&link.url) {
                    widget::svg(image.clone()).width(300).into()
                } else {
                    let mut images_to_load = images_to_load.lock().unwrap();
                    images_to_load.insert(link.url.clone());
                    widget::text("(Loading image...)").into()
                }
            }
            NodeValue::FootnoteReference(_) => todoh!("footnote reference"),
            NodeValue::Math(_) => todoh!("math"),
            NodeValue::MultilineBlockQuote(_) => todoh!("multiline block quote"),
            NodeValue::Escaped => todoh!("escaped"),
            NodeValue::WikiLink(_) => todoh!("wiki link"),
            NodeValue::Underline => todoh!("underline"),
            NodeValue::SpoileredText => todoh!("spoilered text"),
            NodeValue::EscapedTag(_) => todoh!("escaped tag"),
        }
    }
}

fn render_list_item<'a>(
    md: &'a comrak::arena_tree::Node<'a, RefCell<comrak::nodes::Ast>>,
    item: &comrak::nodes::NodeList,
    images_to_load: &Mutex<HashSet<String>>,
    images_bitmap: &HashMap<String, Handle>,
    images_svg: &HashMap<String, widget::svg::Handle>,
) -> Element<'static> {
    widget::column(md.children().map(|n| {
        let starting = match item.list_type {
            comrak::nodes::ListType::Bullet => widget::text(char::from(item.bullet_char)),
            comrak::nodes::ListType::Ordered => widget::text(format!("{}.", item.start)),
        };
        let mut element = widget::column!().into();
        MenuModsDownload::render_element(
            n,
            0,
            &mut element,
            images_to_load,
            images_bitmap,
            images_svg,
        );
        widget::row!(starting, element).spacing(10).into()
    }))
    .spacing(10)
    .into()
}

fn render_link<'a>(
    md: &'a comrak::arena_tree::Node<'a, RefCell<comrak::nodes::Ast>>,
    images_to_load: &Mutex<HashSet<String>>,
    images_bitmap: &HashMap<String, Handle>,
    images_svg: &HashMap<String, widget::svg::Handle>,
    node_link: &comrak::nodes::NodeLink,
) -> Element<'static> {
    let mut i = 0;
    let mut children = widget::column(md.children().map(|n| {
        i += 1;
        let mut element = widget::column!().into();
        MenuModsDownload::render_element(
            n,
            0,
            &mut element,
            images_to_load,
            images_bitmap,
            images_svg,
        );
        element
    }));
    if i == 0 {
        children = widget::column!(widget::text(if node_link.title.is_empty() {
            node_link.url.clone()
        } else {
            node_link.title.clone()
        }));
    }
    widget::button(children)
        .on_press(Message::CoreOpenDir(node_link.url.clone()))
        .into()
}

fn render_children<'a>(
    md: &'a comrak::arena_tree::Node<'a, RefCell<comrak::nodes::Ast>>,
    heading_size: usize,
    images_to_load: &Mutex<HashSet<String>>,
    images_bitmap: &HashMap<String, Handle>,
    images_svg: &HashMap<String, widget::svg::Handle>,
) -> widget::Column<'static, Message, crate::stylesheet::styles::LauncherTheme> {
    widget::column(md.children().map(|n| {
        let mut element = widget::column!().into();
        MenuModsDownload::render_element(
            n,
            heading_size,
            &mut element,
            images_to_load,
            images_bitmap,
            images_svg,
        );
        element
    }))
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
