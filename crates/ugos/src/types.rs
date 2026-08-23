use crate::UgosError;
use serde::Deserialize;
use serde::de::DeserializeOwned;

#[derive(Deserialize)]
struct ApiStatus {
    code: i32,
    msg: String,
}

#[derive(Deserialize)]
pub(crate) struct ApiResponse<T> {
    data: T,
}

impl<T> ApiResponse<T> {
    pub(crate) fn decode(body: &str, endpoint: &str) -> Result<T, UgosError>
    where
        T: DeserializeOwned,
    {
        let status: ApiStatus = serde_json::from_str(body).map_err(|error| UgosError::Decode {
            endpoint: endpoint.to_owned(),
            message: error.to_string(),
        })?;
        if status.code != 200 {
            return Err(UgosError::Api {
                endpoint: endpoint.to_owned(),
                code: status.code,
                message: status.msg,
            });
        }
        let response: Self = serde_json::from_str(body).map_err(|error| UgosError::Decode {
            endpoint: endpoint.to_owned(),
            message: error.to_string(),
        })?;
        Ok(response.data)
    }
}

#[derive(Deserialize)]
pub(crate) struct TaskManagerAll {
    pub(crate) cpu: MetricSeries<CpuStat>,
    pub(crate) mem: MetricSeries<MemoryStat>,
    pub(crate) net: MetricSeries<NetworkStat>,
    #[serde(default)]
    pub(crate) vol: Vec<VolumeStat>,
}

#[derive(Deserialize)]
pub(crate) struct MetricSeries<T> {
    pub(crate) series: Vec<T>,
}

#[derive(Deserialize)]
pub(crate) struct CpuStat {
    pub(crate) used_percent: f64,
    pub(crate) temp: f64,
    pub(crate) time: i64,
}

#[derive(Deserialize)]
pub(crate) struct MemoryStat {
    pub(crate) used_percent: f64,
    pub(crate) time: i64,
}

#[derive(Deserialize)]
pub(crate) struct NetworkStat {
    pub(crate) name: String,
    pub(crate) recv_rate: f64,
    pub(crate) send_rate: f64,
    pub(crate) time: i64,
}

#[derive(Deserialize)]
pub(crate) struct VolumeStat {
    pub(crate) total: f64,
    pub(crate) used: f64,
}

#[derive(Deserialize)]
pub(crate) struct VolumeList {
    pub(crate) result: Vec<VolumeStat>,
}

#[cfg(test)]
#[path = "../tests/unit/types.rs"]
mod tests;
