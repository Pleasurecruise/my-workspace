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
        (Some(lat), Some(lng))
            if (-90.0..=90.0).contains(&lat) && (-180.0..=180.0).contains(&lng) =>
        {
            Some(Geo { lat, lng })
        }
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

    fn build_capture_exif(latitude: (u32, u32), longitude: (u32, u32)) -> Vec<u8> {
        use ::exif::{Field, In, Rational, Tag, Value};
        let fields = [
            Field {
                tag: Tag::DateTimeOriginal,
                ifd_num: In::PRIMARY,
                value: Value::Ascii(vec![b"2026:09:18 12:34:56".to_vec()]),
            },
            Field {
                tag: Tag::OffsetTimeOriginal,
                ifd_num: In::PRIMARY,
                value: Value::Ascii(vec![b"+08:00".to_vec()]),
            },
            Field {
                tag: Tag::GPSLatitudeRef,
                ifd_num: In::PRIMARY,
                value: Value::Ascii(vec![b"S".to_vec()]),
            },
            Field {
                tag: Tag::GPSLatitude,
                ifd_num: In::PRIMARY,
                value: Value::Rational(vec![
                    Rational::from(latitude),
                    Rational::from((12, 1)),
                    Rational::from((0, 1)),
                ]),
            },
            Field {
                tag: Tag::GPSLongitudeRef,
                ifd_num: In::PRIMARY,
                value: Value::Ascii(vec![b"W".to_vec()]),
            },
            Field {
                tag: Tag::GPSLongitude,
                ifd_num: In::PRIMARY,
                value: Value::Rational(vec![
                    Rational::from(longitude),
                    Rational::from((30, 1)),
                    Rational::from((0, 1)),
                ]),
            },
        ];
        let mut writer = ::exif::experimental::Writer::new();
        for field in &fields {
            writer.push_field(field);
        }
        let mut output = std::io::Cursor::new(Vec::new());
        writer
            .write(&mut output, true)
            .expect("encode EXIF fixture");
        output.into_inner()
    }

    fn build_container_box(kind: &[u8; 4], content: &[u8]) -> Vec<u8> {
        let mut result = u32::try_from(content.len() + 8)
            .expect("small fixture")
            .to_be_bytes()
            .to_vec();
        result.extend_from_slice(kind);
        result.extend_from_slice(content);
        result
    }

    #[tokio::test]
    async fn reads_heic_metadata() {
        // An EXIF-only HEIF container using idat construction; deliberately no image bitstream.
        // The 6-byte prefix exercises the HEIF offset to the TIFF header.
        let mut exif_data = 6_u32.to_be_bytes().to_vec();
        exif_data.extend_from_slice(b"Exif\0\0");
        exif_data.extend_from_slice(&build_capture_exif((31, 1), (121, 1)));
        let mut info = vec![0, 0, 0, 0, 0, 1];
        info.extend(build_container_box(b"infe", b"\x02\0\0\0\0\x01\0\0Exif\0"));
        let mut location = vec![1, 0, 0, 0, 0x44, 0, 0, 1, 0, 1, 0, 1, 0, 0, 0, 1];
        location.extend_from_slice(&0_u32.to_be_bytes());
        location.extend_from_slice(
            &u32::try_from(exif_data.len())
                .expect("small fixture")
                .to_be_bytes(),
        );
        let mut meta = vec![0, 0, 0, 0];
        meta.extend(build_container_box(b"iinf", &info));
        meta.extend(build_container_box(b"iloc", &location));
        meta.extend(build_container_box(b"idat", &exif_data));
        let mut source = build_container_box(b"ftyp", b"heic\0\0\0\0mif1heic");
        source.extend(build_container_box(b"meta", &meta));
        let metadata = super::super::read_metadata(source)
            .await
            .expect("read HEIC metadata");
        assert_eq!(
            metadata.captured_at.as_deref(),
            Some("2026-09-18T12:34:56+08:00")
        );
        let geo = metadata.geo.expect("GPS metadata");
        assert!((geo.lat + 31.2).abs() < 0.000001);
        assert!((geo.lng + 121.5).abs() < 0.000001);
    }

    #[tokio::test]
    async fn reads_jpeg_metadata() {
        let mut payload = b"Exif\0\0".to_vec();
        payload.extend(build_capture_exif((31, 1), (121, 1)));
        let mut source = vec![0xff, 0xd8, 0xff, 0xe1];
        source.extend_from_slice(
            &u16::try_from(payload.len() + 2)
                .expect("small fixture")
                .to_be_bytes(),
        );
        source.extend(payload);
        source.extend_from_slice(&[0xff, 0xd9]);
        let metadata = super::super::read_metadata(source)
            .await
            .expect("read JPEG metadata");
        assert_eq!(
            metadata.captured_at.as_deref(),
            Some("2026-09-18T12:34:56+08:00")
        );
        assert!(metadata.geo.is_some());
        let empty = super::super::read_metadata(vec![0xff, 0xd8, 0xff, 0xd9])
            .await
            .expect("no EXIF");
        assert!(empty.captured_at.is_none());
        assert!(empty.geo.is_none());
        assert!(super::super::read_metadata(vec![]).await.is_err());
        assert!(
            super::super::read_metadata(vec![0; 20 * 1024 * 1024 + 1])
                .await
                .is_err()
        );
    }

    #[tokio::test]
    async fn rejects_invalid_gps() {
        for (latitude, longitude) in [
            ((1, 0), (121, 1)),
            ((31, 1), (1, 0)),
            ((91, 1), (121, 1)),
            ((31, 1), (181, 1)),
        ] {
            let metadata = super::super::read_metadata(build_capture_exif(latitude, longitude))
                .await
                .expect("read photo metadata");
            assert!(metadata.captured_at.is_some());
            assert!(
                metadata.geo.is_none(),
                "invalid GPS must not cross the transport boundary"
            );
        }
    }

    #[test]
    fn applies_metadata_policy() {
        use super::super::{MetadataPolicy, PhotoMetadata, Upload};
        let mut input = Upload {
            title: "Photo".into(),
            description: None,
            tags: vec![],
            date: None,
            geo: None,
        };
        let fixture = || PhotoMetadata {
            captured_at: Some("2026-09-18T12:34:56+08:00".into()),
            geo: Some(Geo {
                lat: 31.2,
                lng: 121.5,
            }),
        };
        input.apply_metadata(fixture(), MetadataPolicy::Reviewed);
        assert!(input.date.is_none());
        assert!(input.geo.is_none());
        input.apply_metadata(fixture(), MetadataPolicy::SourceDefaults);
        assert_eq!(input.date.as_deref(), Some("2026-09-18T12:34:56+08:00"));
        assert_eq!(input.geo.as_ref().expect("source GPS").lat, 31.2);
        input.date = Some("2025-01-01T10:00:00Z".into());
        input.geo = Some(Geo { lat: 0.0, lng: 0.0 });
        input.apply_metadata(fixture(), MetadataPolicy::SourceDefaults);
        assert_eq!(input.date.as_deref(), Some("2025-01-01T10:00:00Z"));
        assert_eq!(input.geo.as_ref().expect("manual GPS").lat, 0.0);
    }
}
