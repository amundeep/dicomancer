use iced::widget::image::Handle;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct DicomView {
    pub file_path: PathBuf,
    pub metadata: Vec<MetadataRow>,
    pub image: Option<Handle>,
}

#[derive(Debug, Clone)]
pub struct MetadataRow {
    pub tag: String,
    pub vr: String,
    pub alias: String,
    pub value: String,
}

#[derive(Debug, Clone)]
pub struct DicomEntry {
    pub patient_id: String,
    pub study_instance_uid: String,
    pub series_instance_uid: String,
    pub sop_instance_uid: String,
    pub view: DicomView,
}
