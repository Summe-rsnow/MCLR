//! Task IPC commands.

use mclr_core::{AppError, TaskCenter, TaskNode};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
#[specta::specta]
pub async fn tasks_list(state: State<'_, Arc<TaskCenter>>) -> Result<Vec<TaskNode>, AppError> {
    Ok(state.list_tasks().await)
}

#[tauri::command]
#[specta::specta]
pub async fn task_cancel(state: State<'_, Arc<TaskCenter>>, id: String) -> Result<(), AppError> {
    state.cancel_task(&id).await.map_err(AppError::from)?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn task_pause(state: State<'_, Arc<TaskCenter>>, id: String) -> Result<(), AppError> {
    state.pause_task(&id).await.map_err(AppError::from)?;
    Ok(())
}

#[tauri::command]
#[specta::specta]
pub async fn task_resume(state: State<'_, Arc<TaskCenter>>, id: String) -> Result<(), AppError> {
    state.resume_task(&id).await.map_err(AppError::from)?;
    Ok(())
}
