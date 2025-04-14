//! An (incomplete) HTML renderer written in iced.
//! - Supports headings, images and links. Not much more though.
//! - Does not support CSS, JavaScript or any fancy HTML features.
//!
//! See [`MenuModsDownload::render_html`] docs for more info.

use html5ever::{driver::ParseOpts, parse_document, tendril::TendrilSink};
use iced::widget;
use markup5ever_rcdom::{Node, RcDom};

use crate::launcher_state::{ImageState, MenuModsDownload, Message};

use super::Element;

#[derive(Debug, Clone, Copy)]
struct ChildData {
    heading_weight: usize,
    monospace: bool,
}

impl Default for ChildData {
    fn default() -> Self {
        Self {
            heading_weight: 0,
            monospace: false,
        }
    }
}

impl ChildData {
    pub fn with_heading(weight: usize) -> Self {
        Self {
            heading_weight: weight,
            monospace: false,
        }
    }

    pub fn monospace() -> Self {
        Self {
            heading_weight: 0,
            monospace: true,
        }
    }
}

impl MenuModsDownload {
    /// Takes in text containing HTML and renders it to an iced `Element`.
    ///
    /// # Arguments
    /// - `input`: Text with valid HTML syntax
    ///   (any syntax errors may be brushed off or loosely handled so be careful)
    /// - `images`: A reference to `ImageState`.
    ///   This will pull any mentioned images from here
    ///   and add requests for loading missing ones to here.
    pub fn render_html<'a>(
        input: &str,
        images: &'a ImageState,
        window_size: (f32, f32),
    ) -> Element<'a> {
        let dom = parse_document(RcDom::default(), ParseOpts::default())
            .from_utf8()
            .read_from(&mut input.as_bytes())
            // Will not panic as reading from &[u8] cannot fail
            .unwrap();

        let mut element = widget::column!().into();
        _ = Self::traverse_node(
            &dom.document,
            &mut element,
            images,
            ChildData::default(),
            window_size,
        );
        element
    }

    #[must_use]
    fn traverse_node<'a>(
        node: &Node,
        element: &mut Element<'a>,
        images: &'a ImageState,
        data: ChildData,
        window_size: (f32, f32),
    ) -> bool {
        match &node.data {
            markup5ever_rcdom::NodeData::Document => {
                render_children(node, element, images, ChildData::default(), window_size);
                true
            }
            markup5ever_rcdom::NodeData::Text { contents } => {
                let text = contents.borrow().to_string();

                *element = if data.monospace {
                    widget::row![
                        widget::text(text.clone()).font(iced::Font::with_name("JetBrains Mono")),
                        widget::button(widget::text("Copy").size(12))
                            .on_press(Message::CoreCopyText(text)),
                    ]
                    .spacing(5)
                    .wrap()
                    .into()
                } else {
                    widget::text(text)
                        .size(if data.heading_weight > 0 {
                            36 - (data.heading_weight * 4)
                        } else {
                            16
                        } as u16)
                        .into()
                };

                false
            }
            markup5ever_rcdom::NodeData::Element {
                name,
                attrs,
                template_contents: _,
                mathml_annotation_xml_integration_point: _,
            } => render_html(name, attrs, node, element, images, window_size),
            _ => false,
        }
    }
}

#[must_use]
fn render_html<'a>(
    name: &html5ever::QualName,
    attrs: &std::cell::RefCell<Vec<html5ever::Attribute>>,
    node: &Node,
    element: &mut Element<'a>,
    images: &'a ImageState,
    window_size: (f32, f32),
) -> bool {
    let name = name.local.to_string();
    let attrs = attrs.borrow();
    match name.as_str() {
        "center" | "kbd" | "span" => {
            render_children(node, element, images, ChildData::default(), window_size);
            false
        }
        "html" | "body" | "p" | "div" => {
            render_children(node, element, images, ChildData::default(), window_size);
            true
        }
        "details" | "summary" | "h1" => {
            render_children(node, element, images, ChildData::default(), window_size);
            true
        }
        "h2" => {
            render_children(
                node,
                element,
                images,
                ChildData::with_heading(2),
                window_size,
            );
            true
        }
        "h3" => {
            render_children(
                node,
                element,
                images,
                ChildData::with_heading(3),
                window_size,
            );
            true
        }
        "h4" => {
            render_children(
                node,
                element,
                images,
                ChildData::with_heading(4),
                window_size,
            );
            true
        }
        "b" | "strong" | "em" | "i" => {
            render_children(
                node,
                element,
                images,
                ChildData::with_heading(4),
                window_size,
            );
            false
        }
        "a" => {
            if let Some(attr) = attrs
                .iter()
                .find(|attr| attr.name.local.to_string().as_str() == "href")
            {
                let url = attr.value.to_string();
                let children_empty = { node.children.borrow().is_empty() };

                let mut children: Element = widget::column![].into();
                _ = render_children(
                    node,
                    &mut children,
                    images,
                    ChildData::with_heading(0),
                    window_size,
                );

                if children_empty {
                    children = widget::column!(widget::text(url.clone())).into();
                }
                *element = widget::button(children)
                    .on_press(Message::CoreOpenDir(url))
                    .into();
            } else {
                *element = widget::text("[HTML error: malformed link]]").into();
            }
            false
        }
        "head" | "br" => true,
        "img" => {
            if let Some(attr) = attrs
                .iter()
                .find(|attr| attr.name.local.to_string().as_str() == "src")
            {
                let url = attr.value.to_string();
                *element = if let Some(image) = images.bitmap.get(&url) {
                    // Image
                    widget::image(image.clone()).into()
                } else if let Some(image) = images.svg.get(&url) {
                    widget::svg(image.clone()).into()
                } else {
                    let mut images_to_load = images.to_load.lock().unwrap();
                    images_to_load.insert(url);
                    widget::text("(Loading image...)").into()
                }
            } else {
                *element = widget::text("[HTML error: malformed image]]").into();
            }
            true
        }
        "code" => {
            render_children(node, element, images, ChildData::monospace(), window_size);
            false
        }
        "hr" => {
            *element = widget::horizontal_rule(4.0).into();
            true
        }
        _ => {
            *element = widget::text!("[HTML todo: {name}]").into();
            true
        }
    }
}

fn render_children<'a>(
    node: &Node,
    element: &mut Element<'a>,
    images: &'a ImageState,
    data: ChildData,
    window_size: (f32, f32),
) {
    let children = node.children.borrow();

    let mut column = widget::column![];
    let mut row = widget::row![];

    let mut is_newline = false;

    for item in children.iter() {
        if is_newline {
            column = column.push(row.wrap());
            row = widget::row![];
        }
        let mut element = widget::column!().into();
        is_newline = MenuModsDownload::traverse_node(item, &mut element, images, data, window_size);
        row = row.push(element);
    }
    column = column.push(row.wrap());
    *element = column.into();
}
