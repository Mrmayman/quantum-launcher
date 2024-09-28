use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::Mutex,
};

use comrak::nodes::NodeValue;
use iced::widget;

use crate::{
    icon_manager,
    launcher_state::{MenuModsDownload, Message},
};

use super::{button_with_icon, Element};

impl MenuModsDownload {
    pub fn view_main(&self, icons: &HashMap<String, PathBuf>) -> Element {
        let mods_list = match self.results.as_ref() {
            Some(results) => widget::column(results.hits.iter().enumerate().map(|(i, hit)| {
                widget::button(
                    widget::row!(
                        widget::column!(
                            icon_manager::download_with_size(16),
                            widget::text(hit.downloads).size(12)
                        )
                        .align_items(iced::Alignment::Center)
                        .spacing(5),
                        if let Some(icon) = icons.get(&hit.title) {
                            widget::column!(widget::image(icon))
                        } else {
                            widget::column!(widget::text(""))
                        },
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
                .on_press(Message::InstallModsClick(i))
                .into()
            })),
            None => widget::column!(widget::text("Search something to get started...")),
        };
        widget::row!(
            widget::column!(
                button_with_icon(icon_manager::back(), "Back")
                    .on_press(Message::ManageModsScreenOpen),
                widget::text_input("Search...", &self.query)
                    .on_input(Message::InstallModsSearchInput)
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

    pub fn view(
        &self,
        icons: &HashMap<String, PathBuf>,
        images_to_load: &Mutex<HashSet<String>>,
    ) -> Element {
        if let (Some(selection), Some(results)) = (&self.opened_mod, &self.results) {
            if let Some(hit) = results.hits.get(*selection) {
                let project_info = if let Some(info) = self.result_data.get(&hit.project_id) {
                    widget::column!(self.parse_markdown(&info.body, images_to_load, icons))
                } else {
                    widget::column!(widget::text("Loading..."))
                };

                widget::scrollable(
                    widget::column!(
                        button_with_icon(icon_manager::back(), "Back")
                            .on_press(Message::InstallModsBackToMainScreen),
                        widget::row!(
                            if let Some(icon) = icons.get(&hit.title) {
                                widget::column!(widget::image(icon))
                            } else {
                                widget::column!(widget::text(""))
                            },
                            widget::text(&hit.title).size(24)
                        )
                        .spacing(10),
                        widget::text(&hit.description).size(20),
                        project_info
                    )
                    .padding(20)
                    .spacing(20),
                )
                .into()
            } else {
                self.view_main(icons)
            }
        } else {
            self.view_main(icons)
        }
    }

    pub fn parse_markdown(
        &self,
        markdown: &str,
        images_to_load: &Mutex<HashSet<String>>,
        images: &HashMap<String, PathBuf>,
    ) -> Element {
        let arena = comrak::Arena::new();
        let root = comrak::parse_document(&arena, markdown, &comrak::Options::default());

        // println!("Start of markdown print\n{root:#?}");

        let mut element = widget::column!().into();

        Self::render_element(root, 0, &mut element, images_to_load, images);
        element
    }

    fn render_element<'arena, 'element: 'arena>(
        md: &'element comrak::arena_tree::Node<'arena, RefCell<comrak::nodes::Ast>>,
        heading_size: usize,
        element: &mut Element,
        images_to_load: &Mutex<HashSet<String>>,
        images: &HashMap<String, PathBuf>,
    ) {
        let data = md.data.borrow();
        *element = match &data.value {
            NodeValue::Document => widget::column(md.children().map(|n| {
                let mut element = widget::column!().into();
                Self::render_element(n, 0, &mut element, images_to_load, images);
                element
            }))
            .spacing(10)
            .into(),
            NodeValue::Heading(node_heading) => widget::column(md.children().map(|n| {
                let mut element = widget::column!().into();
                Self::render_element(
                    n,
                    node_heading.level as usize,
                    &mut element,
                    images_to_load,
                    images,
                );
                element
            }))
            .into(),
            NodeValue::Text(text) => widget::text(text)
                .size(if heading_size > 0 {
                    32 - (heading_size * 4)
                } else {
                    16
                } as u16)
                .into(),
            NodeValue::Paragraph => widget::column(md.children().map(|n| {
                let mut element = widget::column!().into();
                Self::render_element(n, 0, &mut element, images_to_load, images);
                element
            }))
            .into(),
            NodeValue::Link(node_link) => {
                let mut i = 0;
                let mut children = widget::column(md.children().map(|n| {
                    i += 1;
                    let mut element = widget::column!().into();
                    Self::render_element(n, 0, &mut element, images_to_load, images);
                    element
                }));
                if i == 0 {
                    children = widget::column!(widget::text(if node_link.title.is_empty() {
                        node_link.url.to_owned()
                    } else {
                        node_link.title.to_owned()
                    }))
                }
                widget::button(children)
                    .on_press(Message::OpenDir(node_link.url.to_owned()))
                    .into()
            }
            NodeValue::FrontMatter(_) => {
                widget::column!(widget::text("[todo: front matter]")).into()
            }
            NodeValue::BlockQuote => widget::column!(widget::text("[todo: block quote]")).into(),
            NodeValue::List(_) => widget::column!(widget::text("[todo: list]")).into(),
            NodeValue::Item(_) => widget::column!(widget::text("[todo: list item]")).into(),
            NodeValue::DescriptionList => {
                widget::column!(widget::text("[todo: description list]")).into()
            }
            NodeValue::DescriptionItem(_) => {
                widget::column!(widget::text("[todo: description item]")).into()
            }
            NodeValue::DescriptionTerm => {
                widget::column!(widget::text("[todo: description term]")).into()
            }
            NodeValue::DescriptionDetails => {
                widget::column!(widget::text("[todo: description details]")).into()
            }
            NodeValue::CodeBlock(_) => widget::column!(widget::text("[todo: code block]")).into(),
            NodeValue::HtmlBlock(node_html_block) => {
                let mut rendered_images = Vec::new();
                for lines in node_html_block.literal.lines() {
                    let line = lines.split_whitespace().fold("".to_owned(), |mut n, v| {
                        n.push_str(v);
                        n.push(' ');
                        n
                    });
                    if line.starts_with("<img src=\"") {
                        let url = line.split('"').nth(1);
                        if let Some(url) = url {
                            if let Some(name) = url.rsplit('/').next() {
                                if let Some(image) = images.get(name) {
                                    rendered_images.push(widget::image(image).width(300).into());
                                } else {
                                    let mut images_to_load = images_to_load.lock().unwrap();
                                    images_to_load.insert(url.to_owned());
                                }
                            }
                        }
                    }
                }
                widget::column(rendered_images)
                    .spacing(10)
                    .align_items(iced::Alignment::Center)
                    .into()
            }
            NodeValue::ThematicBreak => {
                widget::column!(widget::text("[todo: thematic break]")).into()
            }
            NodeValue::FootnoteDefinition(_) => {
                widget::column!(widget::text("[todo: footnote definition]")).into()
            }
            NodeValue::Table(_) => widget::column!(widget::text("[todo: table]")).into(),
            NodeValue::TableRow(_) => widget::column!(widget::text("[todo: table row]")).into(),
            NodeValue::TableCell => widget::column!(widget::text("[todo: table cell]")).into(),
            NodeValue::TaskItem(_) => widget::column!(widget::text("[todo: task item]")).into(),
            NodeValue::SoftBreak | NodeValue::LineBreak => widget::column!().into(),
            NodeValue::Code(_) => widget::column!(widget::text("[todo: code]")).into(),
            NodeValue::HtmlInline(_) => widget::column!(widget::text("[todo: html inline]")).into(),
            NodeValue::Emph => widget::column!(widget::text("[todo: emphasis]")).into(),
            NodeValue::Strong => widget::column!(widget::text("[todo: strong]")).into(),
            NodeValue::Strikethrough => {
                widget::column!(widget::text("[todo: strikethrough]")).into()
            }
            NodeValue::Superscript => widget::column!(widget::text("[todo: superscript]")).into(),
            NodeValue::Image(link) => {
                if let Some(image) = images.get(&link.url) {
                    widget::image(image).width(300).into()
                } else {
                    let mut images_to_load = images_to_load.lock().unwrap();
                    images_to_load.insert(link.url.to_owned());
                    widget::text("(Loading image...)").into()
                }
            }
            NodeValue::FootnoteReference(_) => {
                widget::column!(widget::text("[todo: footnote reference]")).into()
            }
            NodeValue::Math(_) => widget::column!(widget::text("[todo: math]")).into(),
            NodeValue::MultilineBlockQuote(_) => {
                widget::column!(widget::text("[todo: multiline block quote]")).into()
            }
            NodeValue::Escaped => widget::column!(widget::text("[todo: escaped]")).into(),
            NodeValue::WikiLink(_) => widget::column!(widget::text("[todo: wiki link]")).into(),
            NodeValue::Underline => widget::column!(widget::text("[todo: underline]")).into(),
            NodeValue::SpoileredText => {
                widget::column!(widget::text("[todo: spoilered text]")).into()
            }
            NodeValue::EscapedTag(_) => widget::column!(widget::text("[todo: escaped tag]")).into(),
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
