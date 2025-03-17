//! A (somewhat incomplete) markdown renderer written in iced.
//! Yes. It *kind-of* correctly parses and renders markdown
//! as a hierarchy of `iced` widgets.
//!
//! And yes, it even supports inline HTML (no CSS or JS though)
//! so this is a pseudo browser engine (a very bad one at that).
//! The implementation of HTML can be found in the `html` module.
//!
//! I didn't used iced's built-in `widget::markdown` because:
//! - The version of iced I am using is old and doesn't have it
//! - I don't think it would be perfectly suited for my use case.
//!
//! This can be called using [`MenuModsDownload::render_markdown()`]
//! (see function documentation for more details).

use std::cell::RefCell;

use comrak::nodes::NodeValue;
use iced::widget;

use crate::{
    launcher_state::{ImageState, MenuModsDownload, Message},
    menu_renderer::Element,
};

macro_rules! todoh {
    ($desc:expr) => {
        widget::column!(widget::text(concat!("[todo: ", $desc, "]"))).into()
    };
}

impl MenuModsDownload {
    /// Takes in markdown text and renders it.
    /// Supports inline HTML too!
    ///
    /// # Arguments
    /// - `markdown`: Any markdown formatted text.
    ///   Syntax errors will be ignored.
    /// - `images`: A reference to `ImageState`.
    ///   This will pull any mentioned images from here
    ///   and add requests for loading missing ones to here.
    pub fn render_markdown<'a>(
        markdown: &'a str,
        images: &'a ImageState,
        window_size: (f32, f32),
    ) -> Element<'a> {
        let arena = comrak::Arena::new();
        let root = comrak::parse_document(&arena, markdown, &comrak::Options::default());

        let mut element = widget::column!().into();

        _ = Self::render_element(root, 0, &mut element, images, window_size);
        element
    }

    #[must_use]
    fn render_element<'arena, 'element: 'arena>(
        md: &'element comrak::arena_tree::Node<'arena, RefCell<comrak::nodes::Ast>>,
        heading_size: usize,
        element: &mut Element<'static>,
        images: &ImageState,
        window_size: (f32, f32),
    ) -> bool {
        let data = md.data.borrow();

        let mut force_newline = false;

        *element = match &data.value {
            NodeValue::Document => render_children(md, 0, images, window_size)
                .spacing(10)
                .into(),
            NodeValue::Heading(node_heading) => {
                let heading_size = node_heading.level as usize;
                force_newline = true;
                render_children(md, heading_size, images, window_size).into()
            }
            NodeValue::Text(text) => widget::text(text.clone())
                .size(if heading_size > 0 {
                    36 - (heading_size * 4)
                } else {
                    16
                } as u16)
                .into(),
            NodeValue::Paragraph => render_children(md, 0, images, window_size).into(),
            NodeValue::Link(node_link) => render_link(md, images, node_link, window_size),
            NodeValue::FrontMatter(_) => todoh!("front matter"),
            NodeValue::BlockQuote => todoh!("block quote"),
            NodeValue::List(_list) => {
                force_newline = true;
                render_children(md, 0, images, window_size)
                    .spacing(10)
                    .into()
            }
            NodeValue::Item(item) => {
                force_newline = true;
                render_list_item(md, item, images, window_size)
            }
            NodeValue::DescriptionList => todoh!("description list"),
            NodeValue::DescriptionItem(_) => todoh!("description item"),
            NodeValue::DescriptionTerm => todoh!("description term"),
            NodeValue::DescriptionDetails => todoh!("description details"),
            NodeValue::CodeBlock(block) => widget::container(
                widget::column!(
                    widget::button("Copy").on_press(Message::CoreCopyText(block.literal.clone())),
                    widget::text(block.literal.clone())
                        .font(iced::Font::with_name("JetBrains Mono")),
                )
                .spacing(5),
            )
            .into(),
            NodeValue::HtmlBlock(node_html_block) => {
                Self::render_html(&node_html_block.literal, images, window_size)
            }
            NodeValue::ThematicBreak => widget::row!(widget::text("_____").size(20))
                .align_y(iced::Alignment::Center)
                .into(),
            NodeValue::FootnoteDefinition(_) => todoh!("footnote definition"),
            NodeValue::Table(_) => todoh!("table"),
            NodeValue::TableRow(_) => todoh!("table row"),
            NodeValue::TableCell => todoh!("table cell"),
            NodeValue::TaskItem(_) => todoh!("task item"),
            NodeValue::SoftBreak | NodeValue::LineBreak => widget::column!().into(),
            NodeValue::Code(code) => widget::column![
                widget::button("Copy").on_press(Message::CoreCopyText(code.literal.clone())),
                widget::text(code.literal.clone()).font(iced::Font::with_name("JetBrains Mono"))
            ]
            .spacing(5)
            .into(),
            NodeValue::HtmlInline(html) => Self::render_html(html, images, window_size),
            NodeValue::Strong | NodeValue::Emph => render_children(md, 4, images, window_size)
                .spacing(10)
                .into(),
            NodeValue::Strikethrough => todoh!("strikethrough"),
            NodeValue::Superscript => todoh!("superscript"),
            NodeValue::Image(link) => {
                if let Some(image) = images.bitmap.get(&link.url) {
                    // Image
                    widget::image(image.clone()).into()
                } else if let Some(image) = images.svg.get(&link.url) {
                    widget::svg(image.clone()).into()
                } else {
                    let mut images_to_load = images.to_load.lock().unwrap();
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
        };

        // WTF: I am going to commit a crime. Get ready.
        //
        // last_line_blank is a private field in the `comrak`
        // library. However, we really, really need this.
        //
        // Luckily they have made the grave mistake of exposing
        // the field when debug printing, so we debug-print
        // the value and search for and find last_line_blank
        //
        // If this breaks in the future, every element will
        // be on a newline which is suboptimal but... hey it's
        // not that bad
        let debug_text = format!("{data:?}");
        force_newline | parse_last_line_blank(&debug_text)
        // We need this to see if the markdown element ends with
        // a newline or not. Those `comrak` people just had to add
        // all the information and helpfully hide it from us
        // (unless I'm being an idiot and it's obvious, but hey,
        // open an issue or PR if there's a better way!)
    }
}

fn parse_last_line_blank(input: &str) -> bool {
    if let Some(pos) = input.find("last_line_blank:") {
        let substring = &input[pos..];
        if let Some(_) = substring.find("true") {
            return true;
        } else if let Some(_) = substring.find("false") {
            return false;
        }
    }
    true
}

fn render_children<'a>(
    md: &'a comrak::arena_tree::Node<'a, RefCell<comrak::nodes::Ast>>,
    heading_size: usize,
    images: &ImageState,
    window_size: (f32, f32),
) -> widget::Column<'static, Message, crate::stylesheet::styles::LauncherTheme> {
    let mut column = widget::column![];
    let mut row = widget::row![];

    let mut is_newline = false;

    for item in md.children() {
        if is_newline {
            column = column.push(row.wrap());
            row = widget::row![];
        }

        let mut element = widget::column!().into();
        is_newline =
            MenuModsDownload::render_element(item, heading_size, &mut element, images, window_size);
        row = row.push(element);
    }

    column = column.push(row.wrap());

    column.into()
}

fn render_list_item<'a>(
    md: &'a comrak::arena_tree::Node<'a, RefCell<comrak::nodes::Ast>>,
    item: &comrak::nodes::NodeList,
    images: &ImageState,
    window_size: (f32, f32),
) -> Element<'static> {
    widget::column(md.children().map(|n| {
        let starting = match item.list_type {
            comrak::nodes::ListType::Bullet => widget::text(char::from(item.bullet_char)),
            comrak::nodes::ListType::Ordered => widget::text!("{}.", item.start),
        };
        let mut element = widget::column!().into();
        _ = MenuModsDownload::render_element(n, 0, &mut element, images, window_size);
        widget::row!(starting, element).spacing(10).into()
    }))
    .spacing(10)
    .into()
}

fn render_link<'a>(
    md: &'a comrak::arena_tree::Node<'a, RefCell<comrak::nodes::Ast>>,
    images: &ImageState,
    node_link: &comrak::nodes::NodeLink,
    window_size: (f32, f32),
) -> Element<'static> {
    let mut i = 0;
    let mut children = widget::column(md.children().map(|n| {
        i += 1;
        let mut element = widget::column!().into();
        // TODO
        _ = MenuModsDownload::render_element(n, 0, &mut element, images, window_size);
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
