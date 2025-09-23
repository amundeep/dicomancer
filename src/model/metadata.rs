struct Metadata {
    elements: Vec<String>,
    input_value: String,
}

impl Default for Metadata {
    fn default() -> Self {
        Self {
            elements: vec![
                "StudyInstanceUID".to_owned(),
                "SeriesInstanceUID".to_owned(),
                "SOPInstanceUID".to_owned(),
            ],
            input_value: String::default(),
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    InputValue(String),
    Submitted,
    DeleteItem(usize),
}
