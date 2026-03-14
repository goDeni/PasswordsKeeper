use axum::{
    extract::{Path as AxumPath, State},
    http::{header, StatusCode},
    response::IntoResponse,
    routing::{delete, get, post},
    Json, Router,
};
use sec_store::record::{Record, RecordId};
use sec_store::repository::remote::{AddRecordRequest, UpdateRecordRequest};
use sec_store::repository::RecordsRepository;

use super::SimpleStatus;
use crate::{ApiError, AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/sessions/{session_id}", delete(close_session))
        .route(
            "/sessions/{session_id}/records",
            get(list_records).post(add_record),
        )
        .route(
            "/sessions/{session_id}/records/{record_id}",
            get(get_record).put(update_record).delete(delete_record),
        )
        .route("/sessions/{session_id}/save", post(save_session))
        .route("/sessions/{session_id}/cancel", post(cancel_session))
        .route("/sessions/{session_id}/export", get(export_repository))
}

async fn close_session(
    State(state): State<AppState>,
    AxumPath(session_id): AxumPath<String>,
) -> impl IntoResponse {
    let removed = state.sessions.write().await.remove(&session_id);
    if removed.is_some() {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}

async fn list_records(
    State(state): State<AppState>,
    AxumPath(session_id): AxumPath<String>,
) -> Result<Json<Vec<Record>>, ApiError> {
    let session = state.get_session(&session_id).await?;
    let session = session.lock().await;
    Ok(Json(
        session
            .repository
            .get_records()
            .await
            .map_err(ApiError::internal)?,
    ))
}

async fn get_record(
    State(state): State<AppState>,
    AxumPath((session_id, record_id)): AxumPath<(String, String)>,
) -> Result<Json<Record>, ApiError> {
    let session = state.get_session(&session_id).await?;
    let session = session.lock().await;
    let record = session
        .repository
        .get(&record_id)
        .await
        .map_err(ApiError::internal)?
        .ok_or_else(|| ApiError::not_found("Record does not exist"))?;
    Ok(Json(record))
}

async fn add_record(
    State(state): State<AppState>,
    AxumPath(session_id): AxumPath<String>,
    Json(request): Json<AddRecordRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let session = state.get_session(&session_id).await?;
    let mut session = session.lock().await;
    session
        .repository
        .add_record(request.record)
        .await
        .map_err(ApiError::from_add_error)?;
    Ok((StatusCode::CREATED, Json(SimpleStatus::new("created"))))
}

async fn update_record(
    State(state): State<AppState>,
    AxumPath((session_id, record_id)): AxumPath<(String, String)>,
    Json(request): Json<UpdateRecordRequest>,
) -> Result<Json<SimpleStatus>, ApiError> {
    if request.record.id != record_id {
        return Err(ApiError::bad_request(
            "Record id in path and payload must match",
        ));
    }

    let session = state.get_session(&session_id).await?;
    let mut session = session.lock().await;
    session
        .repository
        .update(request.record)
        .await
        .map_err(ApiError::from_update_error)?;
    Ok(Json(SimpleStatus::new("updated")))
}

async fn delete_record(
    State(state): State<AppState>,
    AxumPath((session_id, record_id)): AxumPath<(String, RecordId)>,
) -> Result<StatusCode, ApiError> {
    let session = state.get_session(&session_id).await?;
    let mut session = session.lock().await;
    session
        .repository
        .delete(&record_id)
        .await
        .map_err(ApiError::from_update_error)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn save_session(
    State(state): State<AppState>,
    AxumPath(session_id): AxumPath<String>,
) -> Result<Json<SimpleStatus>, ApiError> {
    let session = state.get_session(&session_id).await?;
    let mut session = session.lock().await;
    let current_persisted = session
        .repository
        .persisted_dump()
        .await
        .map_err(ApiError::internal)?;
    if current_persisted != session.persisted_snapshot {
        return Err(ApiError::conflict(
            "Repository changed in another session. Reopen and retry.",
        ));
    }
    session
        .repository
        .save()
        .await
        .map_err(ApiError::internal)?;
    session.persisted_snapshot = session
        .repository
        .dump()
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(SimpleStatus::new("saved")))
}

async fn cancel_session(
    State(state): State<AppState>,
    AxumPath(session_id): AxumPath<String>,
) -> Result<Json<SimpleStatus>, ApiError> {
    let session = state.get_session(&session_id).await?;
    let mut session = session.lock().await;
    session
        .repository
        .cancel()
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(SimpleStatus::new("cancelled")))
}

async fn export_repository(
    State(state): State<AppState>,
    AxumPath(session_id): AxumPath<String>,
) -> Result<impl IntoResponse, ApiError> {
    let session = state.get_session(&session_id).await?;
    let session = session.lock().await;
    let dump = session
        .repository
        .dump()
        .await
        .map_err(ApiError::internal)?;

    Ok(([(header::CONTENT_TYPE, "application/octet-stream")], dump))
}
