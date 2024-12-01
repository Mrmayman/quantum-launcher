use std::{
    collections::{HashMap, HashSet},
    sync::Mutex,
};

use html5ever::{driver::ParseOpts, parse_document, tendril::TendrilSink};
use iced::widget::{self, image::Handle};
use markup5ever_rcdom::{Node, RcDom};

use crate::launcher_state::{MenuModsDownload, Message};

use super::Element;

impl MenuModsDownload {
    pub fn render_html<'a>(
        input: &str,
        images_to_load: &Mutex<HashSet<String>>,
        images: &HashMap<String, Handle>,
    ) -> Element<'a> {
        let dom = parse_document(RcDom::default(), ParseOpts::default())
            .from_utf8()
            .read_from(&mut input.as_bytes())
            .unwrap();

        // println!("{:#?}", dom.document);

        let mut element = widget::column!().into();
        Self::traverse_node(&dom.document, &mut element, images_to_load, images, 0);
        element
    }

    fn traverse_node(
        node: &Node,
        element: &mut Element,
        images_to_load: &Mutex<HashSet<String>>,
        images: &HashMap<String, Handle>,
        heading_size: usize,
    ) {
        match &node.data {
            markup5ever_rcdom::NodeData::Document => {
                let children = node.children.borrow();
                *element = widget::column(children.iter().map(|node| {
                    let mut element = widget::column!().into();
                    Self::traverse_node(node, &mut element, images_to_load, images, 0);
                    element
                }))
                .into();
            }
            markup5ever_rcdom::NodeData::Text { contents } => {
                *element = widget::text(contents.borrow().to_string())
                    .size(if heading_size > 0 {
                        36 - (heading_size * 4)
                    } else {
                        16
                    } as u16)
                    .into();
            }
            markup5ever_rcdom::NodeData::Element {
                name,
                attrs,
                template_contents: _,
                mathml_annotation_xml_integration_point: _,
            } => {
                let name = name.local.to_string();
                let attrs = attrs.borrow();
                match name.as_str() {
                    "html" | "body" | "p" | "center" | "i" | "kbd" | "b" => {
                        let children = node.children.borrow();
                        *element = widget::column(children.iter().map(|node| {
                            let mut element = widget::column!().into();
                            Self::traverse_node(node, &mut element, images_to_load, images, 0);
                            element
                        }))
                        .into();
                    }
                    "h1" => {
                        let children = node.children.borrow();
                        *element = widget::column(children.iter().map(|node| {
                            let mut element = widget::column!().into();
                            Self::traverse_node(node, &mut element, images_to_load, images, 1);
                            element
                        }))
                        .into();
                    }
                    "h2" => {
                        let children = node.children.borrow();
                        *element = widget::column(children.iter().map(|node| {
                            let mut element = widget::column!().into();
                            Self::traverse_node(node, &mut element, images_to_load, images, 2);
                            element
                        }))
                        .into();
                    }
                    "h3" => {
                        let children = node.children.borrow();
                        *element = widget::column(children.iter().map(|node| {
                            let mut element = widget::column!().into();
                            Self::traverse_node(node, &mut element, images_to_load, images, 3);
                            element
                        }))
                        .into();
                    }
                    "details" | "summary" => {
                        let children = node.children.borrow();
                        *element = widget::column(children.iter().map(|node| {
                            let mut element = widget::column!().into();
                            Self::traverse_node(node, &mut element, images_to_load, images, 1);
                            element
                        }))
                        .into();
                    }
                    "a" => {
                        if let Some(attr) = attrs
                            .iter()
                            .find(|attr| attr.name.local.to_string().as_str() == "href")
                        {
                            let url = attr.value.to_string();
                            let children_nodes = node.children.borrow();

                            let mut i = 0;
                            let mut children = widget::column(children_nodes.iter().map(|node| {
                                let mut element = widget::column!().into();
                                i += 1;
                                Self::traverse_node(node, &mut element, images_to_load, images, 3);
                                element
                            }));
                            if children_nodes.is_empty() {
                                children = widget::column!(widget::text(&url));
                            }
                            *element = widget::button(children)
                                .on_press(Message::OpenDir(url.clone()))
                                .into();
                        } else {
                            *element = widget::text("[HTML error: malformed link]]").into();
                        }
                    }
                    "head" | "br" => {}
                    "img" => {
                        if let Some(attr) = attrs
                            .iter()
                            .find(|attr| attr.name.local.to_string().as_str() == "src")
                        {
                            let url = attr.value.to_string();
                            *element = if let Some(image) = images.get(&url) {
                                widget::image(image.clone()).width(300).into()
                            } else {
                                let mut images_to_load = images_to_load.lock().unwrap();
                                images_to_load.insert(url);
                                widget::text("(Loading image...)").into()
                            }
                        } else {
                            *element = widget::text("[HTML error: malformed image]]").into();
                        }
                    }
                    _ => *element = widget::text(format!("[HTML todo: {name}]")).into(),
                }
            }
            _ => {}
        }
    }
}
