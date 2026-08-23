use super::{ApiResponse, TaskManagerAll, VolumeList};
use crate::UgosError;

#[test]
fn decodes_data_from_the_ugos_response_envelope() {
    let stats = ApiResponse::<TaskManagerAll>::decode(
        r#"{"code":200,"msg":"success","data":{"cpu":{"series":[]},"mem":{"series":[]},"net":{"series":[]},"vol":[]}}"#,
        "taskmgr/stat/get_all",
    )
    .unwrap();
    assert!(stats.cpu.series.is_empty());
    assert!(stats.mem.series.is_empty());
    assert!(stats.net.series.is_empty());
}

#[test]
fn reports_an_ugos_api_error_before_decoding_data() {
    let result = ApiResponse::<TaskManagerAll>::decode(
        r#"{"code":401,"msg":"expired","data":[]}"#,
        "taskmgr/stat/get_all",
    );
    assert!(matches!(
        result,
        Err(UgosError::Api {
            endpoint,
            code: 401,
            message
        }) if endpoint == "taskmgr/stat/get_all" && message == "expired"
    ));
}

#[test]
fn rejects_incomplete_task_manager_telemetry() {
    let stats = serde_json::from_str::<TaskManagerAll>(r#"{"vol":[]}"#);
    assert!(stats.is_err());
}

#[test]
fn decodes_volume_capacity_from_task_manager_all() {
    let stats = ApiResponse::<TaskManagerAll>::decode(
		r#"{"code":200,"msg":"success","data":{"cpu":{"series":[]},"mem":{"series":[]},"net":{"series":[]},"vol":[{"name":"VOLUME1","total":1000,"used":375}]}}"#,
        "taskmgr/stat/get_all",
    )
    .unwrap();

    assert_eq!(stats.vol.len(), 1);
    assert_eq!(stats.vol[0].total, 1000.0);
    assert_eq!(stats.vol[0].used, 375.0);
}

#[test]
fn decodes_current_task_manager_samples() {
    let stats = ApiResponse::<TaskManagerAll>::decode(
		r#"{"code":200,"msg":"success","data":{"overview":{"cpu":[{"used_percent":2.76,"temp":58}],"mem":[{"used_percent":39}],"net":[{"send_rate":1,"recv_rate":2}]},"cpu":{"series":[{"used_percent":12.5,"temp":49,"time":100}]},"mem":{"series":[{"used_percent":32,"time":100}]},"net":{"series":[{"name":"overview","send_rate":1700,"recv_rate":2400,"time":100},{"name":"eth0","send_rate":1700,"recv_rate":2400,"time":100}]},"vol":[]}}"#,
		"taskmgr/stat/get_all",
	)
	.unwrap();
    assert_eq!(stats.cpu.series[0].used_percent, 12.5);
    assert_eq!(stats.mem.series[0].used_percent, 32.0);
    assert_eq!(stats.net.series[0].recv_rate, 2400.0);
    assert_eq!(stats.net.series[0].name, "overview");
}

#[test]
fn decodes_wrapped_volume_list() {
    let volumes = ApiResponse::<VolumeList>::decode(
        r#"{"code":200,"msg":"success","data":{"result":[{"total":1000,"used":250}]}}"#,
        "storage/volume/list",
    )
    .unwrap();
    assert_eq!(volumes.result[0].used, 250.0);
}
