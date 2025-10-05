use dicom::object::DefaultDicomObject;
use dicom::pixeldata::{
    DecodedPixelData, PhotometricInterpretation, PixelDecoder, PlanarConfiguration,
};
use iced::widget::image::Handle;

pub struct FrameImagePipeline;

impl FrameImagePipeline {
    pub fn render_first_frame(object: &DefaultDicomObject) -> Result<Option<Handle>, String> {
        let decoded = match object.decode_pixel_data() {
            Ok(data) => data,
            Err(err) => {
                return Err(format!("Failed to decode pixel data: {err}"));
            }
        };

        if decoded.number_of_frames() == 0 {
            return Ok(None);
        }

        Self::frame_to_handle(&decoded, 0).map(Some)
    }

    pub fn frame_to_handle(
        decoded: &DecodedPixelData<'_>,
        frame_idx: u32,
    ) -> Result<Handle, String> {
        if frame_idx >= decoded.number_of_frames() {
            return Err(format!(
                "Requested frame {frame_idx}, but only {} frame(s) are available",
                decoded.number_of_frames()
            ));
        }

        match decoded.photometric_interpretation() {
            photometric if photometric.is_monochrome() => {
                Self::monochrome_to_handle(decoded, frame_idx)
            }
            PhotometricInterpretation::Rgb => Self::rgb_to_handle(decoded, frame_idx),
            other => Self::fallback_to_dynamic(decoded, frame_idx, other.as_str()),
        }
    }

    fn monochrome_to_handle(
        decoded: &DecodedPixelData<'_>,
        frame_idx: u32,
    ) -> Result<Handle, String> {
        let width = decoded.columns();
        let height = decoded.rows();
        let invert = matches!(
            decoded.photometric_interpretation(),
            PhotometricInterpretation::Monochrome1
        );

        if decoded.bits_allocated() <= 8 {
            let samples = decoded
                .to_vec_frame::<u8>(frame_idx)
                .map_err(|err| format!("Failed to materialize frame data: {err}"))?;
            let mut rgba = Vec::with_capacity(samples.len() * 4);
            for &gray in &samples {
                let value = if invert {
                    255u8.saturating_sub(gray)
                } else {
                    gray
                };
                rgba.extend_from_slice(&[value, value, value, 255]);
            }
            return Ok(Handle::from_rgba(width, height, rgba));
        }

        let samples = decoded
            .to_vec_frame::<u16>(frame_idx)
            .map_err(|err| format!("Failed to materialize frame data: {err}"))?;
        let (min, max) = min_max_u16(&samples).unwrap_or((0, 0));
        let mut rgba = Vec::with_capacity(samples.len() * 4);
        for &value in &samples {
            let mut gray = normalize_u16(value, min, max);
            if invert {
                gray = 255 - gray;
            }
            rgba.extend_from_slice(&[gray, gray, gray, 255]);
        }
        Ok(Handle::from_rgba(width, height, rgba))
    }

    fn rgb_to_handle(decoded: &DecodedPixelData<'_>, frame_idx: u32) -> Result<Handle, String> {
        let width = decoded.columns();
        let height = decoded.rows();
        let pixel_count = (width * height) as usize;

        if decoded.bits_allocated() <= 8 {
            let samples = decoded
                .to_vec_frame::<u8>(frame_idx)
                .map_err(|err| format!("Failed to materialize RGB frame: {err}"))?;
            let rgba = match decoded.planar_configuration() {
                PlanarConfiguration::Standard => rgb_interleaved_to_rgba(&samples)?,
                PlanarConfiguration::PixelFirst => rgb_planar_to_rgba_u8(&samples, pixel_count)?,
            };
            return Ok(Handle::from_rgba(width, height, rgba));
        }

        let samples = decoded
            .to_vec_frame::<u16>(frame_idx)
            .map_err(|err| format!("Failed to materialize RGB frame: {err}"))?;
        let rgba = match decoded.planar_configuration() {
            PlanarConfiguration::Standard => rgb_interleaved_u16_to_rgba(&samples)?,
            PlanarConfiguration::PixelFirst => rgb_planar_u16_to_rgba(&samples, pixel_count)?,
        };
        Ok(Handle::from_rgba(width, height, rgba))
    }

    fn fallback_to_dynamic(
        decoded: &DecodedPixelData<'_>,
        frame_idx: u32,
        interpretation: &str,
    ) -> Result<Handle, String> {
        decoded
            .to_dynamic_image(frame_idx)
            .map_err(|err| {
                format!("Unsupported photometric interpretation `{interpretation}`: {err}")
            })
            .map(|image| {
                let rgba = image.into_rgba8();
                let (width, height) = rgba.dimensions();
                Handle::from_rgba(width, height, rgba.into_raw())
            })
    }
}

fn rgb_interleaved_to_rgba(samples: &[u8]) -> Result<Vec<u8>, String> {
    if !samples.len().is_multiple_of(3) {
        return Err(format!(
            "RGB buffer length {} is not divisible by 3",
            samples.len()
        ));
    }
    let mut rgba = Vec::with_capacity(samples.len() / 3 * 4);
    for chunk in samples.chunks(3) {
        if let [r, g, b] = *chunk {
            rgba.extend_from_slice(&[r, g, b, 255]);
        }
    }
    Ok(rgba)
}

fn rgb_planar_to_rgba_u8(samples: &[u8], pixel_count: usize) -> Result<Vec<u8>, String> {
    if samples.len() < pixel_count * 3 {
        return Err(format!(
            "RGB buffer length {} is too small for {pixel_count} pixels",
            samples.len()
        ));
    }
    let (r_plane, rest) = samples.split_at(pixel_count);
    let (g_plane, b_plane) = rest.split_at(pixel_count);
    if b_plane.len() < pixel_count {
        return Err(String::from("RGB buffer does not include all color planes"));
    }

    let mut rgba = Vec::with_capacity(pixel_count * 4);
    for idx in 0..pixel_count {
        rgba.extend_from_slice(&[r_plane[idx], g_plane[idx], b_plane[idx], 255]);
    }
    Ok(rgba)
}

fn rgb_interleaved_u16_to_rgba(samples: &[u16]) -> Result<Vec<u8>, String> {
    if !samples.len().is_multiple_of(3) {
        return Err(format!(
            "RGB buffer length {} is not divisible by 3",
            samples.len()
        ));
    }

    let (mut r_min, mut r_max) = (u16::MAX, u16::MIN);
    let (mut g_min, mut g_max) = (u16::MAX, u16::MIN);
    let (mut b_min, mut b_max) = (u16::MAX, u16::MIN);

    for chunk in samples.chunks(3) {
        if let [r, g, b] = *chunk {
            r_min = r_min.min(r);
            r_max = r_max.max(r);
            g_min = g_min.min(g);
            g_max = g_max.max(g);
            b_min = b_min.min(b);
            b_max = b_max.max(b);
        }
    }

    let mut rgba = Vec::with_capacity(samples.len() / 3 * 4);
    for chunk in samples.chunks(3) {
        if let [r, g, b] = *chunk {
            rgba.extend_from_slice(&[
                normalize_u16(r, r_min, r_max),
                normalize_u16(g, g_min, g_max),
                normalize_u16(b, b_min, b_max),
                255,
            ]);
        }
    }
    Ok(rgba)
}

fn rgb_planar_u16_to_rgba(samples: &[u16], pixel_count: usize) -> Result<Vec<u8>, String> {
    if samples.len() < pixel_count * 3 {
        return Err(format!(
            "RGB buffer length {} is too small for {pixel_count} pixels",
            samples.len()
        ));
    }

    let (r_plane, rest) = samples.split_at(pixel_count);
    let (g_plane, b_plane) = rest.split_at(pixel_count);
    if b_plane.len() < pixel_count {
        return Err(String::from("RGB buffer does not include all color planes"));
    }

    let (r_min, r_max) = min_max_u16(r_plane).unwrap_or((0, 0));
    let (g_min, g_max) = min_max_u16(g_plane).unwrap_or((0, 0));
    let (b_min, b_max) = min_max_u16(b_plane).unwrap_or((0, 0));

    let mut rgba = Vec::with_capacity(pixel_count * 4);
    for idx in 0..pixel_count {
        rgba.extend_from_slice(&[
            normalize_u16(r_plane[idx], r_min, r_max),
            normalize_u16(g_plane[idx], g_min, g_max),
            normalize_u16(b_plane[idx], b_min, b_max),
            255,
        ]);
    }
    Ok(rgba)
}

fn min_max_u16(values: &[u16]) -> Option<(u16, u16)> {
    values.iter().copied().fold(None, |acc, value| match acc {
        None => Some((value, value)),
        Some((min, max)) => Some((min.min(value), max.max(value))),
    })
}

fn normalize_u16(value: u16, min: u16, max: u16) -> u8 {
    if max <= min {
        return 0;
    }

    let range = (max - min) as f32;
    let normalized = (value.saturating_sub(min)) as f32 / range;
    (normalized * 255.0).clamp(0.0, 255.0).round() as u8
}
