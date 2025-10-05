use crate::message::Message;
use crate::model::DicomView;
use iced::widget::text::Wrapping;
use iced::widget::{column, row, scrollable, text};
use iced::{Element, Length};

pub fn metadata_panel<'a>(
    view: Option<&'a DicomView>,
    entries_empty: bool,
) -> Element<'a, Message> {
    if let Some(view) = view {
        let mut table = column![row![
            text("Tag").width(Length::FillPortion(1)),
            text("VR").width(Length::FillPortion(1)),
            text("Alias").width(Length::FillPortion(2)),
            text("Value").width(Length::FillPortion(4)),
        ]
        .spacing(12)];

        for row in &view.metadata {
            table = table.push(
                row![
                    text(&row.tag).width(Length::FillPortion(1)),
                    text(&row.vr).width(Length::FillPortion(1)),
                    text(&row.alias).width(Length::FillPortion(2)),
                    text(&row.value)
                        .width(Length::FillPortion(4))
                        .wrapping(Wrapping::Word),
                ]
                .spacing(12),
            );
        }

        column![
            text(format!("File: {}", view.file_path.display())).size(16),
            scrollable(table.spacing(8)),
        ]
        .spacing(12)
        .into()
    } else if entries_empty {
        text("Import DICOM instances to view their metadata").into()
    } else {
        text("Select an instance from the tree to inspect metadata").into()
    }
}
