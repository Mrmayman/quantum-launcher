#[derive(Debug, Default, Clone, Copy)]
pub struct ChildData {
    pub heading_weight: usize,
    pub indent: bool,
    pub monospace: bool,
    pub li_ordered: bool,
}

impl ChildData {
    pub fn with_heading(weight: usize) -> Self {
        Self {
            heading_weight: weight,
            ..Default::default()
        }
    }

    pub fn with_indent() -> Self {
        Self {
            indent: true,
            ..Default::default()
        }
    }

    pub fn with_indent_ordered() -> Self {
        Self {
            indent: true,
            li_ordered: true,
            ..Default::default()
        }
    }

    pub fn monospace() -> Self {
        Self {
            monospace: true,
            ..Default::default()
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct ElementProperties {
    pub li_ordered_number: Option<usize>,
}

#[macro_export]
macro_rules! draw_children {
    ($info:expr, $element:expr, $child_data:expr, $element_properties:expr) => {
        let (node, images, window_size) = $info;
        render_children(
            node,
            $element,
            images,
            $child_data,
            window_size,
            $element_properties,
        );
    };

    ($info:expr, $element:expr, $child_data:expr) => {
        let (node, images, window_size) = $info;
        render_children(
            node,
            $element,
            images,
            $child_data,
            window_size,
            ElementProperties::default(),
        );
    };

    ($info:expr, $element:expr) => {
        let (node, images, window_size) = $info;
        render_children(
            node,
            $element,
            images,
            ChildData::default(),
            window_size,
            ElementProperties::default(),
        );
    };
}
