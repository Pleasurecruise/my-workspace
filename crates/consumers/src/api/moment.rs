use super::ApiError;
use cms_core::r2::Store;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use vesper_credentials::{ConsumerApi, Stored};

mod exif;
mod media;

pub(super) use media::Error as MediaError;

const ENDPOINT: &str = "https://moment.you-find.me/api/v1";
const PAGE_SIZE: usize = 100;

#[derive(Clone)]
struct Client {
    api_key: String,
    http: reqwest::Client,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Geo {
    pub lat: f64,
    pub lng: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Photo {
    pub id: String,
    pub url: String,
    pub thumbnail_url: String,
    pub r2_key: String,
    pub thumbnail_r2_key: String,
    pub thumb_hash: Option<String>,
    pub title: String,
    pub width: u32,
    pub height: u32,
    pub aspect_ratio: Option<f64>,
    pub tags: Vec<String>,
    pub date: Option<String>,
    pub description: Option<String>,
    pub size: Option<i64>,
    pub format: Option<String>,
    pub geo: Option<Geo>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    pub photos: Vec<Photo>,
    pub total: usize,
    pub next_cursor: Option<String>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Create {
    pub r2_key: String,
    pub thumbnail_r2_key: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geo: Option<Geo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thumb_hash: Option<String>,
    pub width: u32,
    pub height: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aspect_ratio: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Upload {
    pub title: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub date: Option<String>,
    pub geo: Option<Geo>,
}

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Update {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date: Option<Option<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geo: Option<Option<Geo>>,
}

#[derive(Deserialize)]
struct PhotoList {
    photos: Vec<Photo>,
}

#[derive(Deserialize)]
struct PhotoResponse {
    photo: Photo,
}

#[derive(Deserialize)]
struct TagList {
    tags: Vec<String>,
}

impl Client {
    fn load() -> Result<Self, ApiError> {
        let api_key = match vesper_credentials::consumer_api(ConsumerApi::Moment)? {
            Stored::Ready(api_key) => api_key,
            Stored::Missing => return Err(ApiError::MissingCredentials("my-moment")),
        };
        Ok(Self {
            api_key,
            http: reqwest::Client::builder()
                .timeout(super::REQUEST_TIMEOUT)
                .build()?,
        })
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PhotoQuery {
    pub from_date: Option<String>,
    pub to_date: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub search: Option<String>,
    pub limit: Option<usize>,
}

pub async fn query(input: &PhotoQuery) -> Result<Vec<Photo>, ApiError> {
    if input.limit.is_some_and(|limit| !(1..=100).contains(&limit)) {
        return Err(ApiError::Protocol(
            "photo limit must be between 1 and 100".to_owned(),
        ));
    }
    if input
        .search
        .as_ref()
        .is_some_and(|query| query.trim().is_empty())
    {
        return Err(ApiError::Protocol(
            "photo search cannot be empty".to_owned(),
        ));
    }
    let has_filters =
        input.from_date.is_some() || input.to_date.is_some() || !input.tags.is_empty();
    if input.search.is_some() && has_filters {
        return Err(ApiError::Protocol(
            "photo search cannot be combined with date or tag filters".to_owned(),
        ));
    }
    let date_format = time::macros::format_description!("[year]-[month]-[day]");
    for date in [&input.from_date, &input.to_date].into_iter().flatten() {
        time::Date::parse(date, &date_format)
            .map_err(|_| ApiError::Protocol("photo dates must use YYYY-MM-DD".to_owned()))?;
    }
    if let (Some(from), Some(to)) = (&input.from_date, &input.to_date)
        && from > to
    {
        return Err(ApiError::Protocol(
            "photo fromDate must not follow toDate".to_owned(),
        ));
    }
    if input
        .tags
        .iter()
        .any(|tag| tag.trim().is_empty() || tag.contains(','))
    {
        return Err(ApiError::Protocol(
            "photo tags must be non-empty and contain no commas".to_owned(),
        ));
    }
    let client = Client::load()?;
    let mut request = client
        .http
        .get(format!("{ENDPOINT}/photos"))
        .bearer_auth(&client.api_key);
    if let Some(limit) = input.limit {
        request = request.query(&[("limit", limit)]);
    }
    if let Some(from) = &input.from_date {
        request = request.query(&[("fromDate", from)]);
    }
    if let Some(to) = &input.to_date {
        request = request.query(&[("toDate", to)]);
    }
    if let Some(search) = &input.search {
        request = request.query(&[("search", search)]);
    }
    if !input.tags.is_empty() {
        request = request.query(&[("tags", input.tags.join(","))]);
    }
    let response = request.send().await?;
    let status = response.status();
    if !status.is_success() {
        return Err(ApiError::Status {
            operation: "query photos",
            status,
        });
    }
    let result: PhotoList = response.json().await?;
    Ok(result.photos)
}

pub async fn get(id: &str) -> Result<Photo, ApiError> {
    if id.trim().is_empty() {
        return Err(ApiError::Protocol("a photo ID is required".to_owned()));
    }
    let mut url = reqwest::Url::parse(&format!("{ENDPOINT}/photos/"))
        .map_err(|_| ApiError::Protocol("the Moment endpoint is invalid".to_owned()))?;
    url.path_segments_mut()
        .map_err(|_| {
            ApiError::Protocol("the Moment endpoint cannot contain a photo ID".to_owned())
        })?
        .push(id);
    let client = Client::load()?;
    let response = client
        .http
        .get(url)
        .bearer_auth(&client.api_key)
        .send()
        .await?;
    let status = response.status();
    if !status.is_success() {
        return Err(ApiError::Status {
            operation: "read photo",
            status,
        });
    }
    let result: PhotoResponse = response.json().await?;
    Ok(result.photo)
}

pub async fn list(cursor: Option<String>) -> Result<Page, ApiError> {
    let client = Client::load()?;
    let response = client
        .http
        .get(format!("{ENDPOINT}/photos"))
        .bearer_auth(&client.api_key)
        .query(&[("limit", "100")])
        .send()
        .await?;
    let status = response.status();
    if !status.is_success() {
        return Err(ApiError::Status {
            operation: "list photos",
            status,
        });
    }
    let result: PhotoList = response.json().await?;
    let start = match cursor {
        Some(cursor) => {
            let position = result.photos.iter().position(|photo| photo.id == cursor);
            match position {
                Some(position) => position + 1,
                None => {
                    return Err(ApiError::Protocol(format!(
                        "invalid photo cursor: {cursor}"
                    )));
                }
            }
        }
        None => 0,
    };
    let total = result.photos.len();
    let end = total.min(start + PAGE_SIZE);
    let next_cursor = if end < total {
        Some(result.photos[end - 1].id.clone())
    } else {
        None
    };
    Ok(Page {
        photos: result.photos[start..end].to_vec(),
        total,
        next_cursor,
    })
}

pub async fn search(query: &str) -> Result<Vec<Photo>, ApiError> {
    let client = Client::load()?;
    let response = client
        .http
        .get(format!("{ENDPOINT}/photos"))
        .bearer_auth(&client.api_key)
        .query(&[("limit", "100"), ("search", query)])
        .send()
        .await?;
    let status = response.status();
    if !status.is_success() {
        return Err(ApiError::Status {
            operation: "search photos",
            status,
        });
    }
    let result: PhotoList = response.json().await?;
    Ok(result.photos)
}

pub async fn tags() -> Result<Vec<String>, ApiError> {
    let client = Client::load()?;
    let response = client
        .http
        .get(format!("{ENDPOINT}/tags"))
        .bearer_auth(&client.api_key)
        .send()
        .await?;
    let status = response.status();
    if !status.is_success() {
        return Err(ApiError::Status {
            operation: "list photo tags",
            status,
        });
    }
    let result: TagList = response.json().await?;
    Ok(result.tags)
}

pub async fn create(input: &Create) -> Result<Photo, ApiError> {
    let client = Client::load()?;
    let response = client
        .http
        .post(format!("{ENDPOINT}/photos"))
        .bearer_auth(&client.api_key)
        .json(input)
        .send()
        .await?;
    let status = response.status();
    if status != StatusCode::CREATED {
        return Err(ApiError::Status {
            operation: "create photo",
            status,
        });
    }
    let result: PhotoResponse = response.json().await?;
    Ok(result.photo)
}

pub async fn upload(store: &Store, input: Upload, source: Vec<u8>) -> Result<Photo, ApiError> {
    const INVALID_METADATA: &str = "photo upload metadata is invalid";
    if input.title.trim().is_empty() || input.title.chars().count() > 120 {
        return Err(ApiError::Protocol(INVALID_METADATA.to_owned()));
    }
    if input
        .description
        .as_ref()
        .is_some_and(|description| description.chars().count() > 500)
    {
        return Err(ApiError::Protocol(INVALID_METADATA.to_owned()));
    }
    if input.tags.len() > 10 {
        return Err(ApiError::Protocol(INVALID_METADATA.to_owned()));
    }
    for tag in &input.tags {
        if tag.trim().is_empty() || tag.chars().count() > 50 {
            return Err(ApiError::Protocol(INVALID_METADATA.to_owned()));
        }
    }
    match &input.geo {
        Some(geo) if !(-90.0..=90.0).contains(&geo.lat) || !(-180.0..=180.0).contains(&geo.lng) => {
            return Err(ApiError::Protocol(INVALID_METADATA.to_owned()));
        }
        Some(_) | None => {}
    }
    let prepare_task = tokio::task::spawn_blocking(move || media::prepare(&source))
        .await
        .map_err(|_| ApiError::Protocol("photo processor stopped unexpectedly".to_owned()))?;
    let prepared = prepare_task.map_err(ApiError::Media)?;
    let id = uuid::Uuid::new_v4();
    let r2_key = format!("img/{id}.png");
    let thumbnail_r2_key = format!("img/thumbnails/{id}.jpg");
    store.put(&r2_key, prepared.original, "image/png").await?;
    if let Err(error) = store
        .put(&thumbnail_r2_key, prepared.thumbnail, "image/jpeg")
        .await
    {
        return match store.delete(&r2_key).await {
            Ok(()) => Err(error.into()),
            Err(cleanup) => Err(ApiError::Protocol(format!(
                "thumbnail upload failed: {error}; original cleanup failed: {cleanup}"
            ))),
        };
    }
    let metadata = Create {
        r2_key: r2_key.clone(),
        thumbnail_r2_key: thumbnail_r2_key.clone(),
        title: input.title,
        description: input.description,
        tags: input
            .tags
            .into_iter()
            .map(|tag| tag.trim().to_lowercase())
            .collect(),
        date: input.date.or(prepared.captured_at),
        geo: input.geo.or(prepared.geo),
        thumb_hash: Some(prepared.thumb_hash),
        width: prepared.width,
        height: prepared.height,
        aspect_ratio: Some(f64::from(prepared.width) / f64::from(prepared.height)),
        format: Some("PNG".to_owned()),
    };
    match create(&metadata).await {
        Ok(photo) => Ok(photo),
        Err(error) => {
            let original_cleanup = store.delete(&r2_key).await;
            let thumbnail_cleanup = store.delete(&thumbnail_r2_key).await;
            match (original_cleanup, thumbnail_cleanup) {
                (Ok(()), Ok(())) => Err(error),
                (Err(cleanup), Ok(())) => Err(ApiError::Protocol(format!(
                    "photo metadata creation failed: {error}; original cleanup failed: {cleanup}"
                ))),
                (Ok(()), Err(cleanup)) => Err(ApiError::Protocol(format!(
                    "photo metadata creation failed: {error}; thumbnail cleanup failed: {cleanup}"
                ))),
                (Err(original_cleanup), Err(thumbnail_cleanup)) => {
                    Err(ApiError::Protocol(format!(
                        "photo metadata creation failed: {error}; original cleanup failed: {original_cleanup}; thumbnail cleanup failed: {thumbnail_cleanup}"
                    )))
                }
            }
        }
    }
}

pub async fn update(id: &str, input: &Update) -> Result<Photo, ApiError> {
    let client = Client::load()?;
    let response = client
        .http
        .patch(format!("{ENDPOINT}/photos/{id}"))
        .bearer_auth(&client.api_key)
        .json(input)
        .send()
        .await?;
    let status = response.status();
    if !status.is_success() {
        return Err(ApiError::Status {
            operation: "update photo",
            status,
        });
    }
    let result: PhotoResponse = response.json().await?;
    Ok(result.photo)
}

pub async fn delete(id: &str) -> Result<(), ApiError> {
    let client = Client::load()?;
    let response = client
        .http
        .delete(format!("{ENDPOINT}/photos/{id}"))
        .bearer_auth(&client.api_key)
        .send()
        .await?;
    if response.status() == StatusCode::NO_CONTENT {
        Ok(())
    } else {
        Err(ApiError::Status {
            operation: "delete photo",
            status: response.status(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::{Photo, Update, Upload};

    #[test]
    fn decodes_photo() {
        let photo: Photo = serde_json::from_value(serde_json::json!({
            "id": "photo-id",
            "url": "/api/photos/img/photo.jpg",
            "thumbnailUrl": "/api/photos/img/thumbnails/photo.webp",
            "r2Key": "img/photo.jpg",
            "thumbnailR2Key": "img/thumbnails/photo.webp",
            "thumbHash": "hash",
            "title": "Photo",
            "width": 1600,
            "height": 900,
            "aspectRatio": 1.7777777778,
            "tags": ["travel"],
            "date": "2026-08-23T00:00:00.000Z",
            "description": "Description",
            "size": 1024,
            "format": "JPG",
            "geo": { "lat": 31.2304, "lng": 121.4737 }
        }))
        .expect("photo response should match the REST contract");

        assert_eq!(photo.r2_key, "img/photo.jpg");
        assert_eq!(photo.thumbnail_r2_key, "img/thumbnails/photo.webp");
        assert_eq!(photo.tags, ["travel"]);
    }

    #[test]
    fn preserves_null_updates() {
        let update = Update {
            date: Some(None),
            geo: Some(None),
            ..Update::default()
        };
        let value = serde_json::to_value(update).expect("photo update should serialize");

        assert_eq!(value.get("date"), Some(&serde_json::Value::Null));
        assert_eq!(value.get("geo"), Some(&serde_json::Value::Null));
        assert!(value.get("title").is_none());
    }

    #[test]
    fn decodes_upload() {
        let upload: Upload = serde_json::from_value(serde_json::json!({
            "title": "Shanghai",
            "description": "Evening",
            "tags": ["city"],
            "date": "2026-08-24T12:00:00.000Z",
            "geo": { "lat": 31.2304, "lng": 121.4737 }
        }))
        .expect("desktop upload should match the command contract");

        assert_eq!(upload.title, "Shanghai");
        assert_eq!(upload.tags, ["city"]);
        assert_eq!(upload.geo.expect("coordinates").lat, 31.2304);
    }
}
