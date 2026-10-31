use crate::{CommandError, CommandResponse};
use tauri::Manager;

const GRAPH_NODES: u32 = nowledge::graph::MAX_NODES;

#[tauri::command]
pub(crate) async fn read_memory_graph(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
) -> CommandResponse<nowledge::graph::Graph> {
    let result = async {
        if window.label() != "main" || app.state::<crate::app_lock::AppLock>().locked() {
            return Err(CommandError::from(
                "Unlock the main Vesper window to read memories.",
            ));
        }
        Ok(nowledge::graph::read(GRAPH_NODES).await?)
    };
    result.await.into()
}
