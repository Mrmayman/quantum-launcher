//! An (incomplete) HTML renderer written in iced.
//! - Supports headings, images and links. Not much more though.
//! - Does not support CSS, JavaScript or any fancy HTML features.
//!
//! See [`MenuModsDownload::render_html`] docs for more info.

use html5ever::{driver::ParseOpts, parse_document, tendril::TendrilSink};
use iced::widget;
use markup5ever_rcdom::{Node, RcDom};

use crate::{
    draw_children,
    launcher_state::{ImageState, MenuModsDownload, Message},
};

use super::{
    helpers::{ChildData, ElementProperties},
    Element,
};

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
        // WTF: If you can't fix the chaos, embrace the chaos.
        let input = input
            .replace("<ul>", "<br><ul>")
            .replace("<ol>", "<br><ol>");

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
            ElementProperties::default(),
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
        properties: ElementProperties,
    ) -> bool {
        let info = (node, images, window_size);
        match &node.data {
            markup5ever_rcdom::NodeData::Document => {
                draw_children!(info, element);
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
            } => render_html(name, attrs, node, element, images, window_size, properties),
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
    properties: ElementProperties,
) -> bool {
    let name = name.local.to_string();
    let attrs = attrs.borrow();

    let info = (node, images, window_size);

    match name.as_str() {
        "center" | "kbd" | "span" => {
            draw_children!(info, element);
            false
        }
        "html" | "body" | "p" | "div" => {
            draw_children!(info, element);
            true
        }
        "details" | "summary" | "h1" => {
            draw_children!(info, element, ChildData::with_heading(1));
            true
        }
        "h2" => {
            draw_children!(info, element, ChildData::with_heading(2));
            true
        }
        "h3" => {
            draw_children!(info, element, ChildData::with_heading(3));
            true
        }
        "h4" => {
            draw_children!(info, element, ChildData::with_heading(4));
            true
        }
        "b" | "strong" | "em" | "i" => {
            draw_children!(info, element, ChildData::with_heading(4));
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
                draw_children!(info, &mut children);

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
            draw_children!(info, element, ChildData::monospace());
            false
        }
        "hr" => {
            *element = widget::horizontal_rule(4.0).into();
            true
        }
        "ul" => {
            draw_children!(info, element, ChildData::with_indent());
            true
        }
        "ol" => {
            draw_children!(info, element, ChildData::with_indent_ordered());
            true
        }
        "li" => {
            let bullet = if let Some(num) = properties.li_ordered_number {
                widget::text!("{num}. ")
            } else {
                widget::text("- ")
            };
            let mut children: Element = widget::column![].into();
            draw_children!(info, &mut children);
            *element = widget::row![bullet, children].into();
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
    properties: ElementProperties,
) {
    let children = node.children.borrow();

    let mut column = widget::column![];
    let mut row = widget::row![].push_maybe(data.indent.then_some(widget::Space::with_width(16)));

    let mut is_newline = false;

    let mut i = 0;
    for item in children.iter() {
        if is_newline {
            column = column.push(row.wrap());
            row = widget::row![].push_maybe(data.indent.then_some(widget::Space::with_width(16)));
        }
        if is_node_useless(item) {
            continue;
        }

        let mut element = widget::column!().into();

        let mut properties = properties;
        if data.li_ordered {
            properties.li_ordered_number = Some(i + 1);
        }

        is_newline = MenuModsDownload::traverse_node(
            item,
            &mut element,
            images,
            data,
            window_size,
            properties,
        );
        row = row.push(element);

        i += 1;
    }
    column = column.push(row.wrap());
    *element = column.into();
}

fn is_node_useless(node: &Node) -> bool {
    if let markup5ever_rcdom::NodeData::Text { contents } = &node.data {
        let contents = contents.borrow();
        let contents = contents.to_string();
        contents.trim().is_empty()
    } else {
        false
    }
}
