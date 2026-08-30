use super::Geo;

#[derive(Default)]
pub(super) struct Metadata {
    pub captured_at: Option<String>,
    pub geo: Option<Geo>,
    pub orientation: Option<image::metadata::Orientation>,
}

pub(super) fn read(source: &[u8], heif: bool) -> Metadata {
    let parsed = if heif {
        heic::DecoderConfig::new()
            .extract_exif(source)
            .ok()
            .flatten()
            .and_then(|bytes| exif::Reader::new().read_raw(bytes.into_owned()).ok())
    } else {
        exif::Reader::new()
            .read_from_container(&mut std::io::Cursor::new(source))
            .ok()
    };
    let Some(parsed) = parsed else {
        return Metadata::default();
    };

    let latitude = coordinate(&parsed, exif::Tag::GPSLatitude, exif::Tag::GPSLatitudeRef);
    let longitude = coordinate(&parsed, exif::Tag::GPSLongitude, exif::Tag::GPSLongitudeRef);
    let geo = match (latitude, longitude) {
        (Some(lat), Some(lng)) => Some(Geo { lat, lng }),
        _ => None,
    };
    let orientation = parsed
        .get_field(exif::Tag::Orientation, exif::In::PRIMARY)
        .and_then(|field| field.value.get_uint(0))
        .and_then(|value| u8::try_from(value).ok())
        .and_then(image::metadata::Orientation::from_exif);

    Metadata {
        captured_at: captured_at(&parsed),
        geo,
        orientation,
    }
}

fn coordinate(parsed: &exif::Exif, tag: exif::Tag, reference: exif::Tag) -> Option<f64> {
    let field = parsed.get_field(tag, exif::In::PRIMARY)?;
    let exif::Value::Rational(parts) = &field.value else {
        return None;
    };
    let [degrees, minutes, seconds, ..] = parts.as_slice() else {
        return None;
    };
    let value = degrees.to_f64() + minutes.to_f64() / 60.0 + seconds.to_f64() / 3600.0;
    let reference = parsed
        .get_field(reference, exif::In::PRIMARY)?
        .display_value()
        .to_string();
    if reference.trim_matches('"').starts_with(['S', 'W']) {
        Some(-value)
    } else {
        Some(value)
    }
}

fn captured_at(parsed: &exif::Exif) -> Option<String> {
    let field = parsed.get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY)?;
    let exif::Value::Ascii(values) = &field.value else {
        return None;
    };
    let [raw, ..] = values.as_slice() else {
        return None;
    };
    let raw = std::str::from_utf8(raw).ok()?.trim_end_matches('\0');
    let (date, clock) = raw.split_once(' ')?;
    let mut date = date.split(':');
    let year = date.next()?;
    let month = date.next()?;
    let day = date.next()?;
    if date.next().is_some() {
        return None;
    }
    let offset = match parsed.get_field(exif::Tag::OffsetTimeOriginal, exif::In::PRIMARY) {
        Some(field) => match &field.value {
            exif::Value::Ascii(values) => match values.as_slice() {
                [value, ..] => std::str::from_utf8(value).ok(),
                [] => None,
            },
            _ => None,
        },
        None => None,
    };
    let subsecond = match parsed.get_field(exif::Tag::SubSecTimeOriginal, exif::In::PRIMARY) {
        Some(field) => match &field.value {
            exif::Value::Ascii(values) => match values.as_slice() {
                [value, ..] => std::str::from_utf8(value).ok(),
                [] => None,
            },
            _ => None,
        },
        None => None,
    };
    let mut captured_at = format!("{year}-{month}-{day}T{clock}");
    if let Some(subsecond) = subsecond.filter(|value| !value.is_empty()) {
        captured_at.push('.');
        captured_at.push_str(subsecond);
    }
    if let Some(offset) = offset.filter(|value| !value.is_empty()) {
        captured_at.push_str(offset);
    }
    Some(captured_at)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_capture_time() {
        let bytes = [
            0x49, 0x49, 0x2a, 0x00, 0x08, 0x00, 0x00, 0x00, 0x01, 0x00, 0x69, 0x87, 0x04, 0x00,
            0x01, 0x00, 0x00, 0x00, 0x1a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00,
            0x03, 0x90, 0x02, 0x00, 0x14, 0x00, 0x00, 0x00, 0x2c, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, b'2', b'0', b'2', b'6', b':', b'0', b'8', b':', b'2', b'6', b' ', b'1',
            b'2', b':', b'3', b'4', b':', b'5', b'6', 0x00,
        ];
        let parsed = exif::Reader::new()
            .read_raw(bytes.to_vec())
            .expect("valid EXIF fixture");
        assert_eq!(captured_at(&parsed), Some("2026-08-26T12:34:56".to_owned()));
    }
}
