#[tauri::command]
pub(crate) fn read_service_status_catalog()
-> crate::CommandResponse<Vec<quotes::status::ServiceCatalogEntry>> {
    crate::CommandResponse::Ready {
        data: quotes::status::catalog(),
    }
}
