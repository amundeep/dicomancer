use crate::message::Message;
use crate::model::DicomView;
use iced::widget::{text, Image};
use iced::{Element, Length};

pub fn image_panel(view: Option<&DicomView>) -> Element<'static, Message> {
    if let Some(view) = view {
        if let Some(handle) = &view.image {
            Image::new(handle.clone())
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        } else {
            text("No frame preview available").into()
        }
    } else {
        text("Select an instance to preview its first frame").into()
    }
}
