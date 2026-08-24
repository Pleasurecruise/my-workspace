use aws_config::BehaviorVersion;
use aws_credential_types::Credentials;
use aws_sdk_s3::Client;
use aws_types::region::Region;
use std::error::Error;
use std::fmt::{self, Display, Formatter};
use std::path::Path;
use std::time::Duration;
use vesper_credentials::{R2Credentials, Stored};

pub const BUCKET: &str = "cherry-studio";
pub const PUBLISH_PREFIX: &str = "blog";
const REGION: &str = "auto";
const ENDPOINT: &str = "https://fb71c4eceaf623ae1b19b8b37d7a38cf.r2.cloudflarestorage.com";
const OBJECT_READ_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Clone)]
pub struct Store {
    client: Client,
}

#[derive(Debug)]
pub enum StoreError {
    Credentials(vesper_credentials::CredentialError),
    MissingCredentials,
    MissingObject(String),
    ReadTimeout(String),
    Request(String),
    Body(String),
    Read {
        path: std::path::PathBuf,
        source: std::io::Error,
    },
}

impl Display for StoreError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Credentials(source) => Display::fmt(source, formatter),
            Self::MissingCredentials => write!(formatter, "R2 credentials are not configured"),
            Self::MissingObject(key) => write!(formatter, "R2 object does not exist: {key}"),
            Self::ReadTimeout(key) => write!(formatter, "R2 object read timed out: {key}"),
            Self::Request(message) => write!(formatter, "R2 request failed: {message}"),
            Self::Body(message) => write!(formatter, "could not read R2 response body: {message}"),
            Self::Read { path, source } => {
                write!(formatter, "could not read {}: {source}", path.display())
            }
        }
    }
}

impl Error for StoreError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Credentials(source) => Some(source),
            Self::Read { source, .. } => Some(source),
            Self::MissingCredentials
            | Self::MissingObject(..)
            | Self::ReadTimeout(..)
            | Self::Request(..)
            | Self::Body(..) => None,
        }
    }
}

pub fn configure(access_key_id: String, secret_access_key: String) -> Result<(), StoreError> {
    vesper_credentials::save_r2(R2Credentials {
        access_key_id,
        secret_access_key,
    })?;
    Ok(())
}

impl From<vesper_credentials::CredentialError> for StoreError {
    fn from(source: vesper_credentials::CredentialError) -> Self {
        Self::Credentials(source)
    }
}

impl Store {
    pub async fn from_credentials() -> Result<Self, StoreError> {
        let R2Credentials {
            access_key_id,
            secret_access_key,
        } = match vesper_credentials::r2()? {
            Stored::Ready(credentials) => credentials,
            Stored::Missing => return Err(StoreError::MissingCredentials),
        };
        let credentials = Credentials::new(
            access_key_id,
            secret_access_key,
            None,
            None,
            "vesper-credentials",
        );
        let shared = aws_config::defaults(BehaviorVersion::latest())
            .credentials_provider(credentials)
            .endpoint_url(ENDPOINT)
            .region(Region::new(REGION))
            .load()
            .await;
        Ok(Self {
            client: Client::new(&shared),
        })
    }

    pub async fn get(&self, key: &str) -> Result<Vec<u8>, StoreError> {
        tokio::time::timeout(OBJECT_READ_TIMEOUT, async {
            let response = self
                .client
                .get_object()
                .bucket(BUCKET)
                .key(key)
                .send()
                .await
                .map_err(|error| {
                    if error.as_service_error().is_some_and(
                        aws_sdk_s3::operation::get_object::GetObjectError::is_no_such_key,
                    ) || error
                        .raw_response()
                        .is_some_and(|response| response.status().as_u16() == 404)
                    {
                        StoreError::MissingObject(key.to_owned())
                    } else {
                        StoreError::Request(error.to_string())
                    }
                })?;
            let bytes = response
                .body
                .collect()
                .await
                .map_err(|error| StoreError::Body(error.to_string()))?;
            Ok(bytes.into_bytes().to_vec())
        })
        .await
        .map_err(|_| StoreError::ReadTimeout(key.to_owned()))?
    }

    pub async fn put_file(&self, key: &str, path: &Path) -> Result<(), StoreError> {
        let bytes = tokio::fs::read(path)
            .await
            .map_err(|source| StoreError::Read {
                path: path.to_owned(),
                source,
            })?;
        self.client
            .put_object()
            .bucket(BUCKET)
            .key(key)
            .body(bytes.into())
            .send()
            .await
            .map_err(|error| StoreError::Request(error.to_string()))?;
        Ok(())
    }

    pub async fn put(
        &self,
        key: &str,
        bytes: Vec<u8>,
        content_type: &str,
    ) -> Result<(), StoreError> {
        self.client
            .put_object()
            .bucket(BUCKET)
            .key(key)
            .content_type(content_type)
            .body(bytes.into())
            .send()
            .await
            .map_err(|error| StoreError::Request(error.to_string()))?;
        Ok(())
    }

    pub async fn delete(&self, key: &str) -> Result<(), StoreError> {
        self.client
            .delete_object()
            .bucket(BUCKET)
            .key(key)
            .send()
            .await
            .map_err(|error| StoreError::Request(error.to_string()))?;
        Ok(())
    }
}
