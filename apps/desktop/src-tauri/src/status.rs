#[tauri::command]
pub(crate) fn read_service_catalog()
-> crate::CommandResponse<Vec<service_status::ServiceCatalogEntry>> {
    crate::CommandResponse::Ready {
        data: service_status::read_catalog(),
    }
}
