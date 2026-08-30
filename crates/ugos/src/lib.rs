mod auth;
mod client;
mod tls;
mod types;

use serde::Serialize;
use std::collections::VecDeque;
use std::sync::{LazyLock, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};
use vesper_credentials::{Stored, UgosCredentials};

const HOST: &str = "ugreen";
const PORT: u16 = 9443;
static CLIENT: LazyLock<Mutex<Option<client::Client>>> = LazyLock::new(|| Mutex::new(None));
static HISTORY: LazyLock<Mutex<History>> = LazyLock::new(|| Mutex::new(History::default()));
const HISTORY_LIMIT: usize = 60;

#[derive(Default)]
struct History {
    cpu: VecDeque<CpuSample>,
    memory: VecDeque<MemorySample>,
    network: VecDeque<NetworkSample>,
}

trait Timestamped {
    fn sampled_at(&self) -> i64;
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskManagerSnapshot {
    pub cpu: Option<CpuSample>,
    pub cpu_history: Vec<CpuSample>,
    pub memory: Option<MemorySample>,
    pub memory_history: Vec<MemorySample>,
    pub storage: Option<StorageSample>,
    pub network: Option<NetworkSample>,
    pub network_history: Vec<NetworkSample>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CpuSample {
    pub used_percent: f64,
    pub temperature: f64,
    pub sampled_at: i64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MemorySample {
    pub used_percent: f64,
    pub sampled_at: i64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StorageSample {
    pub used_percent: f64,
    pub sampled_at: i64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkSample {
    pub receive_rate: f64,
    pub send_rate: f64,
    pub sampled_at: i64,
}

impl Timestamped for CpuSample {
    fn sampled_at(&self) -> i64 {
        self.sampled_at
    }
}

impl Timestamped for MemorySample {
    fn sampled_at(&self) -> i64 {
        self.sampled_at
    }
}

impl Timestamped for NetworkSample {
    fn sampled_at(&self) -> i64 {
        self.sampled_at
    }
}

#[derive(Debug, thiserror::Error)]
pub enum UgosError {
    #[error("{0}")]
    Credentials(#[from] vesper_credentials::CredentialError),
    #[error("UGOS credentials are not configured")]
    MissingCredentials,
    #[error("UGOS HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),
    #[error("UGOS response from {endpoint} could not be decoded: {message}")]
    Decode { endpoint: String, message: String },
    #[error("UGOS encryption failed: {0}")]
    Encryption(String),
    #[error("UGOS API request to {endpoint} failed ({code}): {message}")]
    Api {
        endpoint: String,
        code: i32,
        message: String,
    },
}

pub fn configure(username: String, password: String) -> Result<(), UgosError> {
    vesper_credentials::save_ugos(UgosCredentials { username, password })?;
    *CLIENT
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = None;
    *HISTORY
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner()) = History::default();
    Ok(())
}

pub async fn task_manager() -> Result<TaskManagerSnapshot, UgosError> {
    let cached = {
        CLIENT
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    };
    let client = match cached {
        Some(client) => client,
        None => {
            let UgosCredentials { username, password } = match vesper_credentials::ugos()? {
                Stored::Ready(credentials) => credentials,
                Stored::Missing => return Err(UgosError::MissingCredentials),
            };
            #[cfg(debug_assertions)]
            let fingerprint = tls::probe_fingerprint(HOST, PORT).await?;
            #[cfg(not(debug_assertions))]
            let fingerprint = match vesper_credentials::ugos_certificate()? {
                Stored::Ready(fingerprint) => tls::CertFingerprint::from_hex(&fingerprint)?,
                Stored::Missing => {
                    let fingerprint = tls::probe_fingerprint(HOST, PORT).await?;
                    vesper_credentials::save_ugos_certificate(&fingerprint.to_hex())?;
                    fingerprint
                }
            };
            let client =
                client::Client::connect(HOST, PORT, &username, &password, fingerprint).await?;
            *CLIENT
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(client.clone());
            client
        }
    };
    let all = client
        .get::<types::TaskManagerAll>("taskmgr/stat/get_all")
        .await?;
    let sampled_at = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| UgosError::Decode {
            endpoint: "local clock".to_owned(),
            message: error.to_string(),
        })?
        .as_secs() as i64;
    let volumes = if all.vol.is_empty() {
        client
            .get::<types::VolumeList>("storage/volume/list?start=0&size=50")
            .await?
            .result
    } else {
        all.vol
    };
    let total_capacity = volumes.iter().map(|volume| volume.total).sum::<f64>();
    let used_capacity = volumes.iter().map(|volume| volume.used).sum::<f64>();
    let storage = (total_capacity > 0.0).then(|| StorageSample {
        used_percent: (used_capacity / total_capacity * 100.0).clamp(0.0, 100.0),
        sampled_at,
    });
    let cpu_samples: Vec<CpuSample> = all
        .cpu
        .series
        .iter()
        .map(|sample| CpuSample {
            used_percent: sample.used_percent,
            temperature: sample.temp,
            sampled_at: sample.time,
        })
        .collect();
    let memory_samples: Vec<MemorySample> = all
        .mem
        .series
        .iter()
        .map(|sample| MemorySample {
            used_percent: sample.used_percent,
            sampled_at: sample.time,
        })
        .collect();
    let network_samples: Vec<NetworkSample> = all
        .net
        .series
        .iter()
        .filter(|sample| sample.name == "overview")
        .map(|sample| NetworkSample {
            receive_rate: sample.recv_rate,
            send_rate: sample.send_rate,
            sampled_at: sample.time,
        })
        .collect();
    let mut history = HISTORY
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    for sample in cpu_samples {
        push_sample(&mut history.cpu, sample);
    }
    for sample in memory_samples {
        push_sample(&mut history.memory, sample);
    }
    for sample in network_samples {
        push_sample(&mut history.network, sample);
    }
    Ok(TaskManagerSnapshot {
        cpu: history.cpu.back().cloned(),
        cpu_history: history.cpu.iter().cloned().collect(),
        memory: history.memory.back().cloned(),
        memory_history: history.memory.iter().cloned().collect(),
        storage,
        network: history.network.back().cloned(),
        network_history: history.network.iter().cloned().collect(),
    })
}

fn push_sample<T: Timestamped>(history: &mut VecDeque<T>, sample: T) {
    if let Some(current) = history.back() {
        if sample.sampled_at() < current.sampled_at() {
            return;
        }
        if sample.sampled_at() == current.sampled_at() {
            history.pop_back();
        }
    }
    if history.len() == HISTORY_LIMIT {
        history.pop_front();
    }
    history.push_back(sample);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn history_orders_samples() {
        let mut history = VecDeque::new();
        push_sample(
            &mut history,
            MemorySample {
                used_percent: 30.0,
                sampled_at: 10,
            },
        );
        push_sample(
            &mut history,
            MemorySample {
                used_percent: 31.0,
                sampled_at: 10,
            },
        );
        push_sample(
            &mut history,
            MemorySample {
                used_percent: 29.0,
                sampled_at: 9,
            },
        );
        push_sample(
            &mut history,
            MemorySample {
                used_percent: 32.0,
                sampled_at: 11,
            },
        );

        assert_eq!(history.len(), 2);
        assert_eq!(history[0].used_percent, 31.0);
        assert_eq!(history[1].used_percent, 32.0);
    }
}
