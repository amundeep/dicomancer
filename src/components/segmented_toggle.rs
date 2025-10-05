use crate::message::Message;
use crate::model::TreeViewMode;
use iced::widget::text::Wrapping;
use iced::widget::{button, container, row, text, Container};
use iced::{Alignment, Background, Color, Length, Shadow, Theme};

pub fn tree_view_mode_toggle(current: TreeViewMode) -> Container<'static, Message> {
    let toggle_row = row![
        segmented_toggle_option(
            "File Browser",
            TreeViewMode::FileBrowser,
            current,
            SegmentPosition::Left
        )
        .width(Length::FillPortion(1)),
        segmented_toggle_option(
            "UID Tree",
            TreeViewMode::UidTree,
            current,
            SegmentPosition::Right
        )
        .width(Length::FillPortion(1)),
    ]
    .spacing(0);

    container(toggle_row)
        .padding(3)
        .width(Length::Fill)
        .style(segmented_container_style)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SegmentPosition {
    Left,
    Right,
}

fn segmented_toggle_option(
    label: &'static str,
    mode: TreeViewMode,
    current: TreeViewMode,
    position: SegmentPosition,
) -> iced::widget::Button<'static, Message> {
    let is_active = mode == current;
    let content = container(text(label).size(14).wrapping(Wrapping::None))
        .width(Length::Fill)
        .height(Length::Fixed(32.0))
        .align_x(Alignment::Center)
        .align_y(Alignment::Center)
        .padding([6, 16]);

    button(content)
        .padding(0)
        .on_press(Message::SetTreeViewMode(mode))
        .style(move |theme, status| segmented_button_style(theme, status, is_active, position))
}

fn segmented_container_style(theme: &Theme) -> iced::widget::container::Style {
    let palette = theme.extended_palette();

    iced::widget::container::Style {
        background: Some(Background::Color(palette.background.strong.color)),
        border: iced::border::Border {
            color: palette.background.strong.color.scale_alpha(0.6),
            width: 1.0,
            radius: iced::border::Radius::new(999.0),
        },
        ..Default::default()
    }
}

fn segmented_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
    is_active: bool,
    position: SegmentPosition,
) -> iced::widget::button::Style {
    let palette = theme.extended_palette();

    let mut background_color = if is_active {
        palette.primary.strong.color
    } else {
        palette.background.strong.color.scale_alpha(0.4)
    };

    match status {
        iced::widget::button::Status::Hovered => {
            background_color = if is_active {
                palette.primary.base.color
            } else {
                palette.background.base.color.scale_alpha(0.8)
            };
        }
        iced::widget::button::Status::Pressed => {
            background_color = if is_active {
                palette.primary.base.color.scale_alpha(0.9)
            } else {
                palette.background.base.color.scale_alpha(0.9)
            };
        }
        iced::widget::button::Status::Disabled => {
            background_color = background_color.scale_alpha(0.5);
        }
        iced::widget::button::Status::Active => {}
    }

    let text_color = if is_active {
        palette.primary.strong.text
    } else {
        palette.background.base.text
    };

    let radius = match position {
        SegmentPosition::Left => iced::border::Radius {
            top_left: 999.0,
            top_right: 10.0,
            bottom_right: 10.0,
            bottom_left: 999.0,
        },
        SegmentPosition::Right => iced::border::Radius {
            top_left: 10.0,
            top_right: 999.0,
            bottom_right: 999.0,
            bottom_left: 10.0,
        },
    };

    iced::widget::button::Style {
        background: Some(Background::Color(background_color)),
        text_color,
        border: iced::border::Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius,
        },
        shadow: Shadow::default(),
    }
}
