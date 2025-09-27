mod image_pipeline;

use std::path::PathBuf;

use dicom::core::dictionary::DataDictionary;
use dicom::core::header::Header;
use dicom::core::value::Value;
use dicom::core::Tag;
use dicom::dictionary_std::StandardDataDictionary;
use dicom::object::{open_file, DefaultDicomObject};
use iced::widget::image::Handle;
use iced::widget::text::Wrapping;
use iced::widget::{button, column, container, row, scrollable, text, Image};
use iced::{application, Alignment, Element, Length, Task, Theme};
use rfd::AsyncFileDialog;

use crate::image_pipeline::FrameImagePipeline;

pub fn main() -> iced::Result {
    let _ = env_logger::Builder::from_default_env()
        .format_timestamp_secs()
        .try_init();

    application("Dicomancer", App::update, App::view)
        .theme(App::theme)
        .run()
}

#[derive(Default)]
struct App {
    dicom: Option<DicomView>,
    last_error: Option<String>,
}

#[derive(Debug, Clone)]
struct DicomView {
    file_path: PathBuf,
    metadata: Vec<MetadataRow>,
    tree_entries: Vec<String>,
    image: Option<Handle>,
}

#[derive(Debug, Clone)]
struct MetadataRow {
    tag: String,
    alias: String,
    value: String,
}

#[derive(Debug, Clone)]
enum Message {
    PickFile,
    FileLoaded(Result<DicomView, String>),
}

impl App {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::PickFile => Task::perform(
                async {
                    let picked = AsyncFileDialog::new().pick_file().await;
                    match picked {
                        Some(handle) => load_dicom(handle.path().to_path_buf()),
                        None => Err(String::from("No file selected")),
                    }
                },
                Message::FileLoaded,
            ),
            Message::FileLoaded(result) => {
                match result {
                    Ok(view) => {
                        self.last_error = None;
                        self.dicom = Some(view);
                    }
                    Err(err) => {
                        self.last_error = Some(err);
                        self.dicom = None;
                    }
                }
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let pick_button = button("Pick File").on_press(Message::PickFile);

        let content: Element<'_, Message> = match (&self.dicom, &self.last_error) {
            (Some(view), _) => {
                let mut tree_list = column![text("Top-level Elements").size(20)];
                for entry in &view.tree_entries {
                    tree_list = tree_list.push(text(entry));
                }

                let tree_panel = container(scrollable(tree_list.spacing(6)))
                    .padding(16)
                    .width(Length::FillPortion(2));

                let mut table = column![row![
                    text("Tag").width(Length::FillPortion(1)),
                    text("Alias").width(Length::FillPortion(1)),
                    text("Value").width(Length::FillPortion(2)),
                ]
                .spacing(12)];

                for row in &view.metadata {
                    table = table.push(
                        row![
                            text(&row.tag).width(Length::FillPortion(1)),
                            text(&row.alias).width(Length::FillPortion(1)),
                            text(&row.value)
                                .width(Length::FillPortion(2))
                                .wrapping(Wrapping::Word),
                        ]
                        .spacing(12),
                    );
                }

                let metadata_panel = container(scrollable(table.spacing(8)))
                    .padding(16)
                    .width(Length::FillPortion(3));

                let image_element: Element<'_, Message> = if let Some(handle) = &view.image {
                    Image::new(handle.clone())
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .into()
                } else {
                    text("No frame preview available").into()
                };

                let image_panel = container(image_element)
                    .padding(16)
                    .width(Length::FillPortion(4))
                    .height(Length::Fill)
                    .align_x(Alignment::Center)
                    .align_y(Alignment::Center);

                column![
                    text(view.file_path.display().to_string()).size(16),
                    row![tree_panel, metadata_panel, image_panel]
                        .spacing(16)
                        .width(Length::Fill)
                        .height(Length::Fill)
                ]
                .spacing(16)
                .into()
            }
            (None, Some(err)) => text(format!("Error: {err}")).into(),
            (None, None) => text("Select a DICOM file to preview its contents").into(),
        };

        column![pick_button, content]
            .padding(20)
            .spacing(20)
            .align_x(Alignment::Start)
            .into()
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }
}

fn load_dicom(path: PathBuf) -> Result<DicomView, String> {
    log::info!("Loading DICOM file: {}", path.display());
    let object = open_file(&path).map_err(|err| {
        let message = format!("Failed to open DICOM file: {err}");
        log::error!("{message}");
        message
    })?;

    let mut metadata = Vec::new();
    let mut tree_entries = Vec::new();

    for element in object.iter() {
        let tag = element.tag();
        let tag_text = format_tag(tag);
        let alias = StandardDataDictionary
            .by_tag(tag)
            .map(|entry| entry.alias)
            .unwrap_or("Unknown")
            .to_string();
        let value = value_to_string(element.value());

        tree_entries.push(format!("{tag_text} {alias}"));
        metadata.push(MetadataRow {
            tag: tag_text,
            alias,
            value,
        });
    }

    let image = extract_image_handle(&object);

    Ok(DicomView {
        file_path: path,
        metadata,
        tree_entries,
        image,
    })
}

fn extract_image_handle(object: &DefaultDicomObject) -> Option<Handle> {
    match FrameImagePipeline::render_first_frame(object) {
        Ok(handle) => handle,
        Err(err) => {
            log::warn!("Unable to build frame preview: {err}");
            None
        }
    }
}

fn value_to_string<D: std::fmt::Debug>(value: &Value<D>) -> String {
    const MAX_LEN: usize = 120;
    let rendered = match value {
        Value::Primitive(_) | Value::Sequence { .. } | Value::PixelSequence { .. } => {
            format!("{value:?}")
        }
    };

    if rendered.len() > MAX_LEN {
        let mut truncated = rendered.chars().take(MAX_LEN).collect::<String>();
        truncated.push('…');
        truncated
    } else {
        rendered
    }
}

fn format_tag(tag: Tag) -> String {
    format!("{:04X},{:04X}", tag.group(), tag.element())
}
