mod image_pipeline;

use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

use dicom::core::dictionary::DataDictionary;
use dicom::core::header::Header;
use dicom::core::value::{PrimitiveValue, Value};
use dicom::core::{Tag, VR};
use dicom::dictionary_std::StandardDataDictionary;
use dicom::object::{open_file, DefaultDicomObject};
use iced::border::{Border, Radius};
use iced::widget::image::Handle;
use iced::widget::text::Wrapping;
use iced::widget::{button, column, container, row, scrollable, text, Image, Space};
use iced::{application, Alignment, Background, Color, Element, Length, Shadow, Task, Theme};
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
    tree_view_mode: TreeViewMode,
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
    vr: String,
    alias: String,
    value: String,
}

#[derive(Debug, Clone)]
enum Message {
    PickFiles,
    FilesLoaded(Vec<Result<DicomEntry, String>>),
    SelectInstance(usize),
    ToggleNode(TreeNodeKey),
    SetTreeViewMode(TreeViewMode),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TreeViewMode {
    FileBrowser,
    UidTree,
}

impl Default for TreeViewMode {
    fn default() -> Self {
        TreeViewMode::FileBrowser
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SegmentedPosition {
    Left,
    Right,
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
            Message::SetTreeViewMode(mode) => {
                if self.tree_view_mode != mode {
                    self.tree_view_mode = mode;
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

        let toggle_row = row![
            segmented_toggle_option(
                "File Browser",
                TreeViewMode::FileBrowser,
                self.tree_view_mode,
                SegmentedPosition::Left,
            )
            .width(Length::FillPortion(1)),
            segmented_toggle_option(
                "UID Tree",
                TreeViewMode::UidTree,
                self.tree_view_mode,
                SegmentedPosition::Right,
            )
            .width(Length::FillPortion(1)),
        ]
        .spacing(0);

        let toggle_row = container(toggle_row)
            .padding(3)
            .width(Length::Fill)
            .style(segmented_container_style);

        root = root.push(toggle_row);

        if self.entries.is_empty() {
            return root.push(text("No files imported")).spacing(6);
        }

        match self.tree_view_mode {
            TreeViewMode::FileBrowser => {
                for (index, entry) in self.entries.iter().enumerate() {
                    let is_selected = self.selected_instance == Some(index);
                    let path_text = entry.view.file_path.display().to_string();
                    let button_label = if is_selected {
                        format!("▶ {path_text}")
                    } else {
                        path_text
                    };
                    root = root.push(
                        button(
                            text(button_label)
                                .wrapping(Wrapping::Word)
                                .width(Length::Fill),
                        )
                        .on_press(Message::SelectInstance(index)),
                    );
                }
            }
            TreeViewMode::UidTree => {
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
                    let patient_label =
                        format!("{} PatientID: {patient_id}", arrow(patient_collapsed));
                    root = root.push(row![button(text(patient_label))
                        .on_press(Message::ToggleNode(patient_key.clone())),]);

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
                            button(text(study_label))
                                .on_press(Message::ToggleNode(study_key.clone())),
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
                                        button(text(button_label))
                                            .on_press(Message::SelectInstance(index)),
                                    ]);
                                }
                            }
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

fn segmented_toggle_option<'a>(
    label: &'a str,
    mode: TreeViewMode,
    current: TreeViewMode,
    position: SegmentedPosition,
) -> iced::widget::Button<'a, Message> {
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
        border: Border {
            color: palette.background.strong.color.scale_alpha(0.6),
            width: 1.0,
            radius: Radius::new(999.0),
        },
        ..Default::default()
    }
}

fn segmented_button_style(
    theme: &Theme,
    status: iced::widget::button::Status,
    is_active: bool,
    position: SegmentedPosition,
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
        SegmentedPosition::Left => Radius {
            top_left: 999.0,
            top_right: 10.0,
            bottom_right: 10.0,
            bottom_left: 999.0,
        },
        SegmentedPosition::Right => Radius {
            top_left: 10.0,
            top_right: 999.0,
            bottom_right: 999.0,
            bottom_left: 10.0,
        },
    };

    iced::widget::button::Style {
        background: Some(Background::Color(background_color)),
        text_color,
        border: Border {
            color: Color::TRANSPARENT,
            width: 0.0,
            radius,
        },
        shadow: Shadow::default(),
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
        let vr = element.vr();
        let value = value_to_string(element.value(), vr);

        metadata.push(MetadataRow {
            tag: tag_text,
            vr: vr.to_string().to_owned(),
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

fn value_to_string<I, P>(value: &Value<I, P>, vr: VR) -> String {
    const MAX_LEN: usize = 120;
    let rendered = match value {
        Value::Primitive(primitive) => format_primitive_value(primitive, vr),
        Value::Sequence(sequence) => {
            let count = sequence.multiplicity() as usize;
            let suffix = if count == 1 { "" } else { "s" };
            format!("Sequence ({count} item{suffix})")
        }
        Value::PixelSequence(sequence) => {
            let fragments = sequence.fragments().len();
            let fragment_suffix = if fragments == 1 { "" } else { "s" };
            let offset_entries = sequence.offset_table().len();
            if offset_entries > 0 {
                let offset_suffix = if offset_entries == 1 { "" } else { "s" };
                format!(
                    "Pixel data ({fragments} fragment{fragment_suffix}, offset table {offset_entries} entry{offset_suffix})"
                )
            } else {
                format!("Pixel data ({fragments} fragment{fragment_suffix})")
            }
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

fn format_primitive_value(value: &PrimitiveValue, vr: VR) -> String {
    let mut rendered = match value {
        PrimitiveValue::Empty => String::new(),
        PrimitiveValue::Str(_)
        | PrimitiveValue::Strs(_)
        | PrimitiveValue::Date(_)
        | PrimitiveValue::Time(_)
        | PrimitiveValue::DateTime(_)
        | PrimitiveValue::I16(_)
        | PrimitiveValue::I32(_)
        | PrimitiveValue::I64(_)
        | PrimitiveValue::U16(_)
        | PrimitiveValue::U32(_)
        | PrimitiveValue::U64(_)
        | PrimitiveValue::F32(_)
        | PrimitiveValue::F64(_) => value.to_str().into_owned(),
        PrimitiveValue::Tags(values) => values
            .iter()
            .map(|tag| format_tag(*tag))
            .collect::<Vec<_>>()
            .join("\\"),
        PrimitiveValue::U8(_) => {
            if is_binary_vr(vr) {
                format!("Binary data ({} bytes)", value.calculate_byte_len())
            } else {
                value.to_str().into_owned()
            }
        }
    };

    if rendered.is_empty() && matches!(value, PrimitiveValue::Empty) {
        rendered.push_str("(empty)");
    }

    rendered
}

fn is_binary_vr(vr: VR) -> bool {
    matches!(
        vr,
        VR::OB | VR::OD | VR::OF | VR::OL | VR::OV | VR::OW | VR::UN
    )
}

fn attribute_text(object: &DefaultDicomObject, name: &str) -> Option<String> {
    object
        .element_by_name(name)
        .ok()
        .and_then(|element| element.to_str().ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
