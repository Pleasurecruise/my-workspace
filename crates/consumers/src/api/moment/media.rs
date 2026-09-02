use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;
use image::{DynamicImage, ImageFormat, ImageReader};
use std::io::Cursor;

const SOURCE_LIMIT: usize = 20 * 1024 * 1024;
const DIMENSION_LIMIT: u32 = 16_384;
const PIXEL_LIMIT: u64 = 100_000_000;
const MEMORY_LIMIT: u64 = 512 * 1024 * 1024;

pub(super) struct Prepared {
    pub original: Vec<u8>,
    pub thumbnail: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub thumb_hash: String,
    pub captured_at: Option<String>,
    pub geo: Option<super::Geo>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("photo source is empty")]
    Empty,
    #[error("photo source exceeds the 20 MB limit")]
    SourceTooLarge,
    #[error("photo dimensions or decoded size exceed the processing limit")]
    ImageTooLarge,
    #[error("photo source is not a supported PNG, JPEG, WebP, AVIF, or HEIC image")]
    Decode,
    #[error("photo variants could not be encoded")]
    Encode(#[source] image::ImageError),
}

pub(super) fn prepare(source: &[u8]) -> Result<Prepared, Error> {
    if source.is_empty() {
        return Err(Error::Empty);
    }
    if source.len() > SOURCE_LIMIT {
        return Err(Error::SourceTooLarge);
    }

    let mut heif = false;
    let brands = match (source.get(4..8), source.get(8..)) {
        (Some(b"ftyp"), Some(brands)) => brands,
        _ => &[],
    };
    for brand in brands.as_chunks::<4>().0 {
        if matches!(
            brand,
            b"heic"
                | b"heix"
                | b"hevc"
                | b"hevx"
                | b"heim"
                | b"heis"
                | b"hevm"
                | b"hevs"
                | b"avif"
                | b"avis"
                | b"mif1"
                | b"msf1"
        ) {
            heif = true;
            break;
        }
    }
    let metadata = super::exif::read(source, heif);
    let mut image = if heif {
        let mut limits = heic::Limits::default();
        limits.max_width = Some(u64::from(DIMENSION_LIMIT));
        limits.max_height = Some(u64::from(DIMENSION_LIMIT));
        limits.max_pixels = Some(PIXEL_LIMIT);
        limits.max_memory_bytes = Some(MEMORY_LIMIT);
        let decoded = match heic::DecoderConfig::new()
            .decode_request(source)
            .with_output_layout(heic::PixelLayout::Rgba8)
            .with_limits(&limits)
            .decode()
        {
            Ok(decoded) => decoded,
            Err(error)
                if matches!(
                    error.error(),
                    heic::HeicError::LimitExceeded(_) | heic::HeicError::OutOfMemory
                ) =>
            {
                return Err(Error::ImageTooLarge);
            }
            Err(_) => return Err(Error::Decode),
        };
        image::RgbaImage::from_raw(decoded.width, decoded.height, decoded.data)
            .map(DynamicImage::ImageRgba8)
            .ok_or(Error::Decode)?
    } else {
        let mut reader = ImageReader::new(Cursor::new(source))
            .with_guessed_format()
            .map_err(|_| Error::Decode)?;
        let mut limits = image::Limits::default();
        limits.max_image_width = Some(DIMENSION_LIMIT);
        limits.max_image_height = Some(DIMENSION_LIMIT);
        limits.max_alloc = Some(MEMORY_LIMIT);
        reader.limits(limits);
        match reader.decode() {
            Ok(image) => image,
            Err(image::ImageError::Limits(_)) => return Err(Error::ImageTooLarge),
            Err(_) => return Err(Error::Decode),
        }
    };
    if u64::from(image.width()) * u64::from(image.height()) > PIXEL_LIMIT {
        return Err(Error::ImageTooLarge);
    }
    if let (false, Some(orientation)) = (heif, metadata.orientation) {
        image.apply_orientation(orientation);
    }

    let width = image.width();
    let height = image.height();
    let mut original = Cursor::new(Vec::new());
    image
        .write_to(&mut original, ImageFormat::Png)
        .map_err(Error::Encode)?;
    let thumbnail = image.resize(600, 600, FilterType::Lanczos3);
    let mut thumbnail_bytes = Vec::new();
    JpegEncoder::new_with_quality(&mut thumbnail_bytes, 90)
        .encode_image(&thumbnail)
        .map_err(Error::Encode)?;
    let placeholder = image.resize(100, 100, FilterType::Lanczos3).to_rgba8();
    let thumb_hash = hex::encode(thumbhash::rgba_to_thumb_hash(
        usize::try_from(placeholder.width()).map_err(|_| Error::ImageTooLarge)?,
        usize::try_from(placeholder.height()).map_err(|_| Error::ImageTooLarge)?,
        placeholder.as_raw(),
    ));

    Ok(Prepared {
        original: original.into_inner(),
        thumbnail: thumbnail_bytes,
        width,
        height,
        thumb_hash,
        captured_at: metadata.captured_at,
        geo: metadata.geo,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{GenericImageView, Rgba, RgbaImage};

    #[test]
    fn normalizes_images() {
        let mut source = Cursor::new(Vec::new());
        DynamicImage::ImageRgba8(RgbaImage::from_pixel(1200, 800, Rgba([21, 34, 55, 255])))
            .write_to(&mut source, ImageFormat::Png)
            .expect("fixture should encode");

        let prepared = prepare(source.get_ref()).expect("fixture should prepare");

        assert_eq!((prepared.width, prepared.height), (1200, 800));
        assert_eq!(
            image::load_from_memory(&prepared.original)
                .expect("original should decode")
                .dimensions(),
            (1200, 800)
        );
        assert_eq!(
            image::load_from_memory(&prepared.thumbnail)
                .expect("thumbnail should decode")
                .dimensions(),
            (600, 400)
        );
        assert!(!prepared.thumb_hash.is_empty());
    }

    #[test]
    fn rejects_bad_image() {
        assert!(matches!(prepare(b"not an image"), Err(Error::Decode)));
    }

    #[test]
    #[ignore = "requires MOMENT_HEIF_FIXTURE"]
    fn loads_heif_fixture() {
        let path = std::env::var("MOMENT_HEIF_FIXTURE").expect("HEIF fixture path");
        let source = std::fs::read(path).expect("HEIF fixture should be readable");

        let prepared = prepare(&source).expect("HEIF fixture should prepare");

        assert!(prepared.width > 0);
        assert!(prepared.height > 0);
        assert!(!prepared.original.is_empty());
        assert!(!prepared.thumbnail.is_empty());
    }
}
