use std::fs;

use iced::widget::{button, column, text};
use iced::{application, Alignment, Element, Length, Settings, Task, Theme};
use rfd::AsyncFileDialog;

pub fn main() -> iced::Result {
    application("Dicomancer", App::update, App::view)
        .theme(App::theme)
        .run()
}

#[derive(Default)]
struct App {
    file_content: Option<String>,
    last_error: Option<String>,
}

#[derive(Debug, Clone)]
enum Message {
    PickFile,
    FileLoaded(Result<String, String>),
}

impl App {
    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::PickFile => {
                // Spawn async task to show file dialog and read content
                Task::perform(
                    async {
                        let picked = AsyncFileDialog::new().pick_file().await;
                        match picked {
                            Some(handle) => {
                                let path = handle.path().to_owned();
                                match fs::read_to_string(&path) {
                                    Ok(content) => Ok(content),
                                    Err(e) => Err(format!("Failed to read file: {e}")),
                                }
                            }
                            None => Err("No file selected".into()),
                        }
                    },
                    Message::FileLoaded,
                )
            }
            Message::FileLoaded(result) => {
                match result {
                    Ok(content) => {
                        self.file_content = Some(content);
                        self.last_error = None;
                    }
                    Err(e) => {
                        self.last_error = Some(e);
                        self.file_content = None;
                    }
                }
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<Message> {
        column![
            button("Pick File").on_press(Message::PickFile),
            if let Some(content) = &self.file_content {
                text(format!("Loaded file with {} bytes", content.len()))
            } else if let Some(err) = &self.last_error {
                text(format!("Error: {err}"))
            } else {
                text("No file loaded")
            }
        ]
        .padding(20)
        .align_x(Alignment::Center)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn theme(&self) -> Theme {
        Theme::Dark
    }
}
