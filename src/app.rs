use crate::message::Message;
use crate::model::loader::load_dicom;
use crate::model::{DicomEntry, TreeNodeKey, TreeViewMode};
use crate::views::{image_panel, metadata_panel, tree_panel};
use iced::widget::text::Wrapping;
use iced::widget::{button, column, container, row, scrollable, text};
use iced::{application, Alignment, Element, Length, Task, Theme};
use rfd::AsyncFileDialog;
use std::collections::BTreeSet;

const APP_TITLE: &str = "Dicomancer";

pub fn run() -> iced::Result {
    let _ = env_logger::Builder::from_default_env()
        .format_timestamp_secs()
        .try_init();

    application(APP_TITLE, App::update, App::view)
        .theme(App::theme)
        .run()
}

#[derive(Default)]
pub struct App {
    entries: Vec<DicomEntry>,
    selected_instance: Option<usize>,
    collapsed_nodes: BTreeSet<TreeNodeKey>,
    tree_view_mode: TreeViewMode,
    last_error: Option<String>,
}

impl App {
    pub fn update(&mut self, message: Message) -> Task<Message> {
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

    pub fn view(&self) -> Element<'_, Message> {
        let pick_button = button("Import DICOM Files").on_press(Message::PickFiles);

        let tree_column = tree_panel(
            &self.entries,
            self.tree_view_mode,
            &self.collapsed_nodes,
            self.selected_instance,
        );
        let tree_panel = container(scrollable(tree_column))
            .padding(16)
            .width(Length::FillPortion(2));

        let selected_view = self
            .selected_instance
            .and_then(|index| self.entries.get(index))
            .map(|entry| &entry.view);

        let metadata_content = metadata_panel(selected_view, self.entries.is_empty());
        let metadata_panel = container(metadata_content)
            .padding(16)
            .width(Length::FillPortion(5));

        let image_content = image_panel(selected_view);
        let image_panel = container(image_content)
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

    pub fn theme(&self) -> Theme {
        Theme::Dark
    }
}
