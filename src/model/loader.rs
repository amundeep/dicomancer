use super::{DicomEntry, DicomView, MetadataRow};
use crate::image_pipeline::FrameImagePipeline;
use crate::utils::{format_tag, value_to_string};
use dicom::core::dictionary::DataDictionary;
use dicom::core::header::Header;
use dicom::dictionary_std::StandardDataDictionary;
use dicom::object::{open_file, DefaultDicomObject};
use iced::widget::image::Handle;
use std::path::PathBuf;

pub fn load_dicom(path: PathBuf) -> Result<DicomEntry, String> {
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

fn attribute_text(object: &DefaultDicomObject, name: &str) -> Option<String> {
    object
        .element_by_name(name)
        .ok()
        .and_then(|element| element.to_str().ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}
