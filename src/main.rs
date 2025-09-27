mod image_pipeline;

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use dicom::core::dictionary::DataDictionary;
use dicom::core::header::Header;
use dicom::core::value::Value;
use dicom::core::Tag;
use dicom::dictionary_std::StandardDataDictionary;
use dicom::object::{open_file, DefaultDicomObject};
use iced::widget::image::Handle;
use iced::widget::text::Wrapping;
use iced::widget::{button, column, container, row, scrollable, text, Image, Space};
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
    entries: Vec<DicomEntry>,
    selected_instance: Option<usize>,
    collapsed_nodes: BTreeSet<TreeNodeKey>,
    last_error: Option<String>,
}

#[derive(Debug, Clone)]
struct DicomView {
    file_path: PathBuf,
    metadata: Vec<MetadataRow>,
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
    PickFiles,
    FilesLoaded(Vec<Result<DicomEntry, String>>),
    SelectInstance(usize),
    ToggleNode(TreeNodeKey),
}

#[derive(Debug, Clone)]
struct DicomEntry {
    patient_id: String,
    study_instance_uid: String,
    series_instance_uid: String,
    sop_instance_uid: String,
    view: DicomView,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum TreeNodeKey {
    Patient(String),
    Study {
        patient: String,
        study: String,
    },
    Series {
        patient: String,
        study: String,
        series: String,
    },
}

impl TreeNodeKey {
    fn patient(id: &str) -> Self {
        Self::Patient(id.to_string())
    }

    fn study(patient: &str, study: &str) -> Self {
        Self::Study {
            patient: patient.to_string(),
            study: study.to_string(),
        }
    }

    fn series(patient: &str, study: &str, series: &str) -> Self {
        Self::Series {
            patient: patient.to_string(),
            study: study.to_string(),
            series: series.to_string(),
        }
    }
}

impl App {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::PickFiles => Task::perform(
                async {
                    match AsyncFileDialog::new().pick_files().await {
                        Some(handles) if !handles.is_empty() => handles
                            .into_iter()
                            .map(|handle| load_dicom(handle.path().to_path_buf()))
                            .collect(),
                        _ => Vec::new(),
                    }
                },
                Message::FilesLoaded,
            ),
            Message::FilesLoaded(results) => {
                let mut errors = Vec::new();
                for result in results {
                    match result {
                        Ok(entry) => {
                            let index = self.entries.len();
                            self.entries.push(entry);
                            self.selected_instance = Some(index);
                        }
                        Err(err) => errors.push(err),
                    }
                }

                if errors.is_empty() {
                    if self.entries.is_empty() {
                        self.selected_instance = None;
                    }
                    if self.last_error.is_some() {
                        self.last_error = None;
                    }
                } else {
                    self.last_error = Some(errors.join("\n"));
                }

                Task::none()
            }
            Message::SelectInstance(index) => {
                if index < self.entries.len() {
                    self.selected_instance = Some(index);
                }
                Task::none()
            }
            Message::ToggleNode(key) => {
                if !self.collapsed_nodes.remove(&key) {
                    self.collapsed_nodes.insert(key);
                }
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let pick_button = button("Import DICOM Files").on_press(Message::PickFiles);

        let tree_column = self.build_tree_column();

        let selected_view = self
            .selected_instance
            .and_then(|index| self.entries.get(index))
            .map(|entry| &entry.view);

        let tree_panel = container(scrollable(tree_column.spacing(6)))
            .padding(16)
            .width(Length::FillPortion(2));

        let metadata_content: Element<'_, Message> = if let Some(view) = selected_view {
            let mut table = column![row![
                text("Tag").width(Length::FillPortion(1)),
                text("Alias").width(Length::FillPortion(1)),
                text("Value").width(Length::FillPortion(3)),
            ]
            .spacing(12)];

            for row in &view.metadata {
                table = table.push(
                    row![
                        text(&row.tag).width(Length::FillPortion(1)),
                        text(&row.alias).width(Length::FillPortion(1)),
                        text(&row.value)
                            .width(Length::FillPortion(3))
                            .wrapping(Wrapping::Word),
                    ]
                    .spacing(12),
                );
            }

            column![
                text(format!("File: {}", view.file_path.display())).size(16),
                scrollable(table.spacing(8))
            ]
            .spacing(12)
            .into()
        } else if self.entries.is_empty() {
            text("Import DICOM instances to view their metadata").into()
        } else {
            text("Select an instance from the tree to inspect metadata").into()
        };

        let metadata_panel = container(metadata_content)
            .padding(16)
            .width(Length::FillPortion(5));

        let image_element: Element<'_, Message> = if let Some(view) = selected_view {
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
        };

        let image_panel = container(image_element)
            .padding(16)
            .width(Length::FillPortion(3))
            .height(Length::Fill)
            .align_x(Alignment::Center)
            .align_y(Alignment::Center);

        let mut content = column![row![tree_panel, metadata_panel, image_panel]
            .spacing(16)
            .width(Length::Fill)
            .height(Length::Fill)]
        .spacing(16);

        if let Some(error) = &self.last_error {
            content = content.push(text(error).size(16).wrapping(Wrapping::Word));
        }

        column![pick_button, content]
            .padding(20)
            .spacing(20)
            .align_x(Alignment::Start)
            .into()
    }

    fn build_tree_column(&self) -> iced::widget::Column<'_, Message> {
        const INDENT: f32 = 18.0;

        let mut root = column![text("Imported Instances").size(20)];

        if self.entries.is_empty() {
            return root.push(text("No files imported"));
        }

        let mut grouped: BTreeMap<
            &str,
            BTreeMap<&str, BTreeMap<&str, BTreeMap<&str, Vec<usize>>>>,
        > = BTreeMap::new();

        for (idx, entry) in self.entries.iter().enumerate() {
            let patient_map = grouped
                .entry(entry.patient_id.as_str())
                .or_insert_with(BTreeMap::new);
            let study_map = patient_map
                .entry(entry.study_instance_uid.as_str())
                .or_insert_with(BTreeMap::new);
            let series_map = study_map
                .entry(entry.series_instance_uid.as_str())
                .or_insert_with(BTreeMap::new);
            series_map
                .entry(entry.sop_instance_uid.as_str())
                .or_insert_with(Vec::new)
                .push(idx);
        }

        let arrow = |collapsed: bool| if collapsed { "▶" } else { "▼" };

        for (patient_id, studies) in grouped {
            let patient_key = TreeNodeKey::patient(patient_id);
            let patient_collapsed = self.collapsed_nodes.contains(&patient_key);
            let patient_label = format!("{} PatientID: {patient_id}", arrow(patient_collapsed));
            root = root.push(row![
                button(text(patient_label)).on_press(Message::ToggleNode(patient_key.clone())),
            ]);

            if patient_collapsed {
                continue;
            }

            for (study_uid, series_map) in studies {
                let study_key = TreeNodeKey::study(patient_id, study_uid);
                let study_collapsed = self.collapsed_nodes.contains(&study_key);
                let study_label =
                    format!("{} StudyInstanceUID: {study_uid}", arrow(study_collapsed));
                root = root.push(row![
                    Space::with_width(Length::Fixed(INDENT)),
                    button(text(study_label)).on_press(Message::ToggleNode(study_key.clone())),
                ]);

                if study_collapsed {
                    continue;
                }

                for (series_uid, sop_map) in series_map {
                    let series_key = TreeNodeKey::series(patient_id, study_uid, series_uid);
                    let series_collapsed = self.collapsed_nodes.contains(&series_key);
                    let series_label = format!(
                        "{} SeriesInstanceUID: {series_uid}",
                        arrow(series_collapsed)
                    );
                    root = root.push(row![
                        Space::with_width(Length::Fixed(INDENT * 2.0)),
                        button(text(series_label))
                            .on_press(Message::ToggleNode(series_key.clone())),
                    ]);

                    if series_collapsed {
                        continue;
                    }

                    for (sop_uid, indices) in sop_map {
                        for index in indices {
                            let label = format!("SOPInstanceUID: {sop_uid}");
                            let is_selected = self.selected_instance == Some(index);
                            let button_label = if is_selected {
                                format!("▶ {label}")
                            } else {
                                label
                            };
                            root = root.push(row![
                                Space::with_width(Length::Fixed(INDENT * 3.0)),
                                button(text(button_label)).on_press(Message::SelectInstance(index)),
                            ]);
                        }
                    }
                }
            }
        }

        root.spacing(6)
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }
}

fn load_dicom(path: PathBuf) -> Result<DicomEntry, String> {
    log::info!("Loading DICOM file: {}", path.display());
    let object = open_file(&path).map_err(|err| {
        let message = format!("{}: failed to open DICOM file ({err})", path.display());
        log::error!("{message}");
        message
    })?;

    let patient_id = attribute_text(&object, "PatientID");
    let study_uid = attribute_text(&object, "StudyInstanceUID");
    let series_uid = attribute_text(&object, "SeriesInstanceUID");
    let sop_uid = attribute_text(&object, "SOPInstanceUID");

    let mut metadata = Vec::new();
    for element in object.iter() {
        let tag = element.tag();
        let tag_text = format_tag(tag);
        let alias = StandardDataDictionary
            .by_tag(tag)
            .map(|entry| entry.alias)
            .unwrap_or("Unknown")
            .to_string();
        let value = value_to_string(element.value());

        metadata.push(MetadataRow {
            tag: tag_text,
            alias,
            value,
        });
    }

    let image = extract_image_handle(&object);

    let view = DicomView {
        file_path: path,
        metadata,
        image,
    };

    Ok(DicomEntry {
        patient_id: patient_id.unwrap_or_else(|| "Unknown".to_string()),
        study_instance_uid: study_uid.unwrap_or_else(|| "Unknown".to_string()),
        series_instance_uid: series_uid.unwrap_or_else(|| "Unknown".to_string()),
        sop_instance_uid: sop_uid.unwrap_or_else(|| "Unknown".to_string()),
        view,
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

fn attribute_text(object: &DefaultDicomObject, name: &str) -> Option<String> {
    object
        .element_by_name(name)
        .ok()
        .and_then(|element| element.to_str().ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
