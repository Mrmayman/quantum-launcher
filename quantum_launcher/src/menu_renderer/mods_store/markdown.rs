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
    pub fn render_markdown<'a>(markdown: &'a str, images: &'a ImageState) -> Element<'a> {
        let arena = comrak::Arena::new();
        let root = comrak::parse_document(&arena, markdown, &comrak::Options::default());

        let mut element = widget::column!().into();

        Self::render_element(root, 0, &mut element, images);
        element
    }

    fn render_element<'arena, 'element: 'arena>(
        md: &'element comrak::arena_tree::Node<'arena, RefCell<comrak::nodes::Ast>>,
        heading_size: usize,
        element: &mut Element<'static>,
        images: &ImageState,
    ) {
        let data = md.data.borrow();
        *element = match &data.value {
            NodeValue::Document => render_children(md, 0, images).spacing(10).into(),
            NodeValue::Heading(node_heading) => {
                let heading_size = node_heading.level as usize;
                render_children(md, heading_size, images).into()
            }
            NodeValue::Text(text) => widget::text(text.clone())
                .size(if heading_size > 0 {
                    36 - (heading_size * 4)
                } else {
                    16
                } as u16)
                .into(),
            NodeValue::Paragraph => render_children(md, 0, images).into(),
            NodeValue::Link(node_link) => render_link(md, images, node_link),
            NodeValue::FrontMatter(_) => todoh!("front matter"),
            NodeValue::BlockQuote => todoh!("block quote"),
            NodeValue::List(_list) => render_children(md, 0, images).spacing(10).into(),
            NodeValue::Item(item) => render_list_item(md, item, images),
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
                Self::render_html(&node_html_block.literal, images)
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
            NodeValue::HtmlInline(html) => Self::render_html(html, images),
            NodeValue::Strong | NodeValue::Emph => widget::column(md.children().map(|n| {
                let mut element = widget::column!().into();
                Self::render_element(n, 4, &mut element, images);
                element
            }))
            .into(),
            NodeValue::Strikethrough => todoh!("strikethrough"),
            NodeValue::Superscript => todoh!("superscript"),
            NodeValue::Image(link) => {
                if let Some(image) = images.bitmap.get(&link.url) {
                    widget::image(image.clone()).width(300).into()
                } else if let Some(image) = images.svg.get(&link.url) {
                    widget::svg(image.clone()).width(300).into()
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
        }
    }
}

fn render_children<'a>(
    md: &'a comrak::arena_tree::Node<'a, RefCell<comrak::nodes::Ast>>,
    heading_size: usize,
    images: &ImageState,
) -> widget::Column<'static, Message, crate::stylesheet::styles::LauncherTheme> {
    widget::column(md.children().map(|n| {
        let mut element = widget::column!().into();
        MenuModsDownload::render_element(n, heading_size, &mut element, images);
        element
    }))
}

fn render_list_item<'a>(
    md: &'a comrak::arena_tree::Node<'a, RefCell<comrak::nodes::Ast>>,
    item: &comrak::nodes::NodeList,
    images: &ImageState,
) -> Element<'static> {
    widget::column(md.children().map(|n| {
        let starting = match item.list_type {
            comrak::nodes::ListType::Bullet => widget::text(char::from(item.bullet_char)),
            comrak::nodes::ListType::Ordered => widget::text!("{}.", item.start),
        };
        let mut element = widget::column!().into();
        MenuModsDownload::render_element(n, 0, &mut element, images);
        widget::row!(starting, element).spacing(10).into()
    }))
    .spacing(10)
    .into()
}

fn render_link<'a>(
    md: &'a comrak::arena_tree::Node<'a, RefCell<comrak::nodes::Ast>>,
    images: &ImageState,
    node_link: &comrak::nodes::NodeLink,
) -> Element<'static> {
    let mut i = 0;
    let mut children = widget::column(md.children().map(|n| {
        i += 1;
        let mut element = widget::column!().into();
        MenuModsDownload::render_element(n, 0, &mut element, images);
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
