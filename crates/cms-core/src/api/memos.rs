use super::ApiError;
use crate::r2::Store;
use futures_util::stream::{self, StreamExt, TryStreamExt};
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::json;
use vesper_credentials::{ConsumerApi, Stored};

const ENDPOINT: &str = "https://memos.you-find.me/api/v1";
const READ_CONCURRENCY: usize = 8;

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Visibility {
    Public,
    Private,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Update {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<Visibility>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pinned: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub favorite: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archived: Option<bool>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Memo {
    pub id: String,
    pub r2_key: String,
    pub content: String,
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
    pub visibility: Visibility,
    pub pinned: bool,
    pub favorite: bool,
    pub archived: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemoView {
    #[serde(flatten)]
    pub memo: Memo,
    pub html: String,
    pub metadata_complete: bool,
}

#[derive(Clone)]
struct Client {
    api_key: String,
    http: reqwest::Client,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Page {
    pub memos: Vec<MemoView>,
    pub next_cursor: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemotePage {
    memos: Vec<RemoteMemo>,
    next_cursor: Option<String>,
}

#[derive(Deserialize)]
struct MemoResponse {
    memo: RemoteMemo,
}

#[derive(Deserialize)]
struct TagResponse {
    tags: Vec<TagCount>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TagCount {
    pub name: String,
    pub count: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RemoteMemo {
    id: String,
    r2_key: String,
    content: String,
    tags: Vec<String>,
    created_at: String,
    updated_at: String,
    visibility: Visibility,
    pinned: bool,
    favorite: bool,
    archived: bool,
}

impl Client {
    fn load() -> Result<Self, ApiError> {
        let api_key = match vesper_credentials::consumer_api(ConsumerApi::Memos)? {
            Stored::Ready(api_key) => api_key,
            Stored::Missing => return Err(ApiError::MissingCredentials("my-memos")),
        };
        Ok(Self {
            api_key,
            http: reqwest::Client::builder()
                .timeout(super::REQUEST_TIMEOUT)
                .build()?,
        })
    }
}

impl RemoteMemo {
    fn into_view(self, content: String) -> MemoView {
        let html = crate::markdown::render_memo(&strip_tags(&content));
        MemoView {
            metadata_complete: true,
            memo: Memo {
                id: self.id,
                r2_key: self.r2_key,
                content,
                tags: self.tags,
                created_at: self.created_at,
                updated_at: self.updated_at,
                visibility: self.visibility,
                pinned: self.pinned,
                favorite: self.favorite,
                archived: self.archived,
            },
            html,
        }
    }
}

pub async fn list(store: &Store, cursor: Option<String>) -> Result<Page, ApiError> {
    let client = Client::load()?;
    let mut request = client
        .http
        .get(format!("{ENDPOINT}/memos"))
        .bearer_auth(&client.api_key)
        .query(&[("limit", "25")]);
    if let Some(cursor) = cursor.as_deref() {
        request = request.query(&[("cursor", cursor)]);
    }
    let response = request.send().await?;
    let status = response.status();
    if !status.is_success() {
        return Err(ApiError::Status {
            operation: "list memos",
            status,
        });
    }
    let page: RemotePage = response.json().await?;
    let memos = stream::iter(page.memos)
        .map(|memo| async move {
            let bytes = store.get(&memo.r2_key).await?;
            let content = match String::from_utf8(bytes) {
                Ok(content) => content,
                Err(source) => {
                    return Err(ApiError::InvalidMemoBody {
                        key: memo.r2_key.clone(),
                        source,
                    });
                }
            };
            Ok::<MemoView, ApiError>(memo.into_view(content))
        })
        .buffered(READ_CONCURRENCY)
        .try_collect()
        .await?;
    Ok(Page {
        memos,
        next_cursor: page.next_cursor,
    })
}

pub async fn search(store: &Store, query: &str) -> Result<Page, ApiError> {
    let client = Client::load()?;
    let response = client
        .http
        .get(format!("{ENDPOINT}/memos"))
        .bearer_auth(&client.api_key)
        .query(&[("limit", "20"), ("search", query)])
        .send()
        .await?;
    let status = response.status();
    if !status.is_success() {
        return Err(ApiError::Status {
            operation: "search memos",
            status,
        });
    }
    let page: RemotePage = response.json().await?;
    let memos = stream::iter(page.memos)
        .map(|memo| async move {
            let bytes = store.get(&memo.r2_key).await?;
            let content = match String::from_utf8(bytes) {
                Ok(content) => content,
                Err(source) => {
                    return Err(ApiError::InvalidMemoBody {
                        key: memo.r2_key.clone(),
                        source,
                    });
                }
            };
            Ok::<MemoView, ApiError>(memo.into_view(content))
        })
        .buffered(READ_CONCURRENCY)
        .try_collect()
        .await?;
    Ok(Page {
        memos,
        next_cursor: page.next_cursor,
    })
}

pub async fn tags() -> Result<Vec<TagCount>, ApiError> {
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
            operation: "list memo tags",
            status,
        });
    }
    let result: TagResponse = response.json().await?;
    Ok(result.tags)
}

pub async fn create(content: &str, visibility: Visibility) -> Result<MemoView, ApiError> {
    let client = Client::load()?;
    let response = client
        .http
        .post(format!("{ENDPOINT}/memos"))
        .bearer_auth(&client.api_key)
        .json(&json!({
            "content": content,
            "tags": [],
            "visibility": visibility,
            "favorite": false
        }))
        .send()
        .await?;
    let status = response.status();
    if status != StatusCode::CREATED {
        return Err(ApiError::Status {
            operation: "create memo",
            status,
        });
    }
    let result: MemoResponse = response.json().await?;
    let content = result.memo.content.clone();
    Ok(result.memo.into_view(content))
}

pub async fn update(id: &str, input: &Update) -> Result<MemoView, ApiError> {
    let client = Client::load()?;
    let response = client
        .http
        .patch(format!("{ENDPOINT}/memos/{id}"))
        .bearer_auth(&client.api_key)
        .json(input)
        .send()
        .await?;
    let status = response.status();
    if !status.is_success() {
        return Err(ApiError::Status {
            operation: "update memo",
            status,
        });
    }
    let result: MemoResponse = response.json().await?;
    let content = result.memo.content.clone();
    Ok(result.memo.into_view(content))
}

pub async fn delete(id: &str) -> Result<(), ApiError> {
    let client = Client::load()?;
    let response = client
        .http
        .delete(format!("{ENDPOINT}/memos/{id}"))
        .bearer_auth(&client.api_key)
        .send()
        .await?;
    if response.status() == StatusCode::NO_CONTENT {
        Ok(())
    } else {
        Err(ApiError::Status {
            operation: "delete memo",
            status: response.status(),
        })
    }
}

fn strip_tags(content: &str) -> String {
    let characters: Vec<char> = content.chars().collect();
    let mut output = String::with_capacity(content.len());
    let mut index = 0;
    while index < characters.len() {
        if characters[index] == '#'
            && (index == 0 || characters[index - 1].is_whitespace())
            && characters
                .get(index + 1)
                .is_some_and(|character| *character != '#' && is_tag_character(*character))
        {
            let mut end = index + 1;
            while end < characters.len() && is_tag_character(characters[end]) {
                end += 1;
            }
            if end == characters.len() || characters[end].is_whitespace() {
                index = end;
                continue;
            }
        }
        output.push(characters[index]);
        index += 1;
    }
    let mut compact = String::with_capacity(output.len());
    let mut horizontal_space = false;
    for character in output.chars() {
        if matches!(character, ' ' | '\t') {
            if horizontal_space {
                continue;
            }
            horizontal_space = true;
        } else {
            horizontal_space = false;
        }
        compact.push(character);
    }
    compact.trim().to_owned()
}

fn is_tag_character(character: char) -> bool {
    character.is_alphanumeric() || matches!(character, '_' | '-' | '/')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_my_memos_hashtags_from_the_rendered_body() {
        let content = "Body #Rust #rust #长文\nnext line";
        assert_eq!(strip_tags(content), "Body \nnext line");
    }
}
