use iced::widget;

use super::Element;

pub fn dynamic_box<'a>(
    items: impl IntoIterator<Item = Element<'a>>,
    item_width: f32,
    item_height: f32,
    total_width: f32,
    padding: f32,
    spacing: f32,
) -> (Element<'a>, f32) {
    let mut current_column = widget::Column::new();

    if total_width < (item_width + item_width + padding + spacing) {
        return shrunk_box(item_height, padding, items, current_column, spacing);
    }

    let mut out_h = padding + item_height;

    let mut current_row = widget::Row::new().spacing(spacing);

    let mut x = padding;
    for item in items {
        x += item_width + spacing;
        if x >= total_width {
            out_h += item_height + spacing;
            current_column = current_column.push(current_row);
            current_row = widget::Row::new();
        }
        current_row = current_row.push(item);
    }
    current_column = current_column.push(current_row);

    out_h += padding;

    (
        current_column.padding(padding).spacing(spacing).into(),
        out_h,
    )
}

fn shrunk_box<'a>(
    item_height: f32,
    padding: f32,
    items: impl IntoIterator<Item = Element<'a>>,
    mut current_column: widget::Column<
        'a,
        crate::launcher_state::Message,
        crate::stylesheet::styles::LauncherTheme,
    >,
    spacing: f32,
) -> (Element<'a>, f32) {
    let mut out_h = padding;
    for item in items {
        current_column = current_column.push(item);
        out_h += item_height + spacing;
    }
    out_h += padding;
    (
        current_column.padding(padding).spacing(spacing).into(),
        out_h,
    )
}
