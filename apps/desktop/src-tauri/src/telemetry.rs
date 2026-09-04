use std::collections::VecDeque;
use std::sync::{LazyLock, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use sysinfo::{Disks, Networks, System};

const HISTORY_LIMIT: usize = 60;
static MONITOR: LazyLock<Mutex<Monitor>> = LazyLock::new(|| Mutex::new(Monitor::new()));

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Snapshot {
    cpu: PercentSample,
    cpu_history: Vec<PercentSample>,
    memory: MemorySample,
    memory_history: Vec<PercentSample>,
    storage: Option<crate::storage::Capacity>,
    network: NetworkSample,
    network_history: Vec<NetworkSample>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct PercentSample {
    used_percent: f64,
    sampled_at: i64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MemorySample {
    used_percent: f64,
    used_bytes: u64,
    total_bytes: u64,
    sampled_at: i64,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct NetworkSample {
    receive_rate: f64,
    send_rate: f64,
    sampled_at: i64,
}

struct Monitor {
    system: System,
    disks: Disks,
    networks: Networks,
    last_network_refresh: Instant,
    cpu_history: VecDeque<PercentSample>,
    memory_history: VecDeque<PercentSample>,
    network_history: VecDeque<NetworkSample>,
}

impl Monitor {
    fn new() -> Self {
        let system = System::new_all();
        std::thread::sleep(sysinfo::MINIMUM_CPU_UPDATE_INTERVAL);
        Self {
            system,
            disks: Disks::new_with_refreshed_list(),
            networks: Networks::new_with_refreshed_list(),
            last_network_refresh: Instant::now(),
            cpu_history: VecDeque::new(),
            memory_history: VecDeque::new(),
            network_history: VecDeque::new(),
        }
    }

    fn sample(&mut self) -> Result<Snapshot, String> {
        self.system.refresh_cpu_usage();
        self.system.refresh_memory();
        self.disks.refresh(true);
        self.networks.refresh(true);

        let sampled_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|error| format!("Could not read the current-device clock: {error}"))?
            .as_secs() as i64;
        let elapsed = self.last_network_refresh.elapsed().as_secs_f64().max(0.001);
        self.last_network_refresh = Instant::now();
        let cpu = PercentSample {
            used_percent: f64::from(self.system.global_cpu_usage()).clamp(0.0, 100.0),
            sampled_at,
        };
        let total_memory = self.system.total_memory();
        let used_memory = self.system.used_memory();
        let memory_percent = if total_memory == 0 {
            0.0
        } else {
            used_memory as f64 / total_memory as f64 * 100.0
        };
        let memory = MemorySample {
            used_percent: memory_percent.clamp(0.0, 100.0),
            used_bytes: used_memory,
            total_bytes: total_memory,
            sampled_at,
        };
        let storage = crate::storage::capacity(&self.disks, sampled_at);
        let network = NetworkSample {
            receive_rate: self
                .networks
                .values()
                .map(|data| data.received() as f64)
                .sum::<f64>()
                / elapsed,
            send_rate: self
                .networks
                .values()
                .map(|data| data.transmitted() as f64)
                .sum::<f64>()
                / elapsed,
            sampled_at,
        };
        push_sample(&mut self.cpu_history, cpu.clone());
        push_sample(
            &mut self.memory_history,
            PercentSample {
                used_percent: memory.used_percent,
                sampled_at,
            },
        );
        push_sample(&mut self.network_history, network.clone());
        Ok(Snapshot {
            cpu,
            cpu_history: self.cpu_history.iter().cloned().collect(),
            memory,
            memory_history: self.memory_history.iter().cloned().collect(),
            storage,
            network,
            network_history: self.network_history.iter().cloned().collect(),
        })
    }
}

fn push_sample<T>(history: &mut VecDeque<T>, sample: T) {
    history.push_back(sample);
    while history.len() > HISTORY_LIMIT {
        history.pop_front();
    }
}

pub(crate) async fn read() -> Result<Snapshot, String> {
    tokio::task::spawn_blocking(|| {
        MONITOR
            .lock()
            .map_err(|error| format!("Current-device monitor state is unavailable: {error}"))?
            .sample()
    })
    .await
    .map_err(|error| format!("Current-device monitoring task failed: {error}"))?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn reads_current_device_snapshot() {
        let snapshot = read().await.expect("current-device snapshot");

        assert!((0.0..=100.0).contains(&snapshot.cpu.used_percent));
        assert!((0.0..=100.0).contains(&snapshot.memory.used_percent));
        assert!(snapshot.memory.total_bytes > 0);
        assert!(!snapshot.cpu_history.is_empty());
        assert!(!snapshot.memory_history.is_empty());
        assert!(!snapshot.network_history.is_empty());
    }
}
