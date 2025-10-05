use dicom::core::value::{PrimitiveValue, Value};
use dicom::core::{Tag, VR};

const MAX_VALUE_LEN: usize = 120;

pub fn value_to_string<I, P>(value: &Value<I, P>, vr: VR) -> String {
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

    if rendered.len() > MAX_VALUE_LEN {
        let mut truncated = rendered.chars().take(MAX_VALUE_LEN).collect::<String>();
        truncated.push('…');
        truncated
    } else {
        rendered
    }
}

pub fn format_tag(tag: Tag) -> String {
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
