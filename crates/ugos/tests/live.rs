#[tokio::test]
#[cfg(debug_assertions)]
#[ignore = "requires the configured local UGOS device and operating-system credential store"]
async fn reads_live_task_manager() {
    vesper_credentials::load_dev_environment().expect("development environment should be readable");
    let snapshot = ugos::task_manager()
        .await
        .expect("configured UGOS Task Manager should be available");
    assert!(snapshot.cpu.is_some(), "UGOS should return a CPU sample");
    assert!(
        snapshot.memory.is_some(),
        "UGOS should return a memory sample"
    );
    assert!(
        snapshot.storage.is_some(),
        "UGOS should return usable volume capacity"
    );
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    let next = ugos::task_manager()
        .await
        .expect("a second UGOS Task Manager sample should be available");
    assert!(next.cpu_history.len() >= 2, "CPU history should accumulate");
    assert!(
        next.memory_history.len() >= 2,
        "memory history should accumulate"
    );
}
