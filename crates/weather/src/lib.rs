mod forecast;
mod location;

pub mod astronomy;

pub use forecast::{Current, HourlyForecast, Weather, WeatherFailure, WeatherReport, read};

// Buffering preserves query order while bounding concurrent location requests.
pub(crate) async fn collect_locations<T, E>(
    requests: impl IntoIterator<Item = impl std::future::Future<Output = Result<T, E>>>,
) -> (Vec<T>, Vec<E>) {
    use futures_util::stream::{self, StreamExt};

    let results = stream::iter(requests).buffered(4).collect::<Vec<_>>().await;
    let mut locations = Vec::new();
    let mut failures = Vec::new();
    for result in results {
        match result {
            Ok(location) => locations.push(location),
            Err(failure) => failures.push(failure),
        }
    }
    (locations, failures)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;

    #[tokio::test]
    async fn collects_locations_in_order() {
        let active = AtomicUsize::new(0);
        let peak = AtomicUsize::new(0);
        let (locations, failures) = collect_locations((0..8).map(|index| {
            let active = &active;
            let peak = &peak;
            async move {
                let count = active.fetch_add(1, Ordering::SeqCst) + 1;
                peak.fetch_max(count, Ordering::SeqCst);
                tokio::time::sleep(Duration::from_millis(if index == 0 { 20 } else { 1 })).await;
                active.fetch_sub(1, Ordering::SeqCst);
                if index % 3 == 0 {
                    Err(index)
                } else {
                    Ok(index)
                }
            }
        }))
        .await;
        assert_eq!(locations, vec![1, 2, 4, 5, 7]);
        assert_eq!(failures, vec![0, 3, 6]);
        assert_eq!(peak.load(Ordering::SeqCst), 4);
        assert_eq!(active.load(Ordering::SeqCst), 0);
    }
}
