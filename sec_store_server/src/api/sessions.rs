use axum::{
    extract::{Path as AxumPath, State},
    http::{header, HeaderMap, StatusCode},
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
        .route("/session", delete(close_session))
        .route("/session/records", get(list_records).post(add_record))
        .route(
            "/session/records/{record_id}",
            get(get_record).put(update_record).delete(delete_record),
        )
        .route("/session/save", post(save_session))
        .route("/session/cancel", post(cancel_session))
        .route("/session/export", get(export_repository))
}

fn session_id_from_headers(headers: &HeaderMap) -> Result<&str, ApiError> {
    let auth_header = headers
        .get(header::AUTHORIZATION)
        .ok_or_else(|| ApiError::unauthorized("Missing authorization header"))?;
    let auth_header = auth_header
        .to_str()
        .map_err(|_| ApiError::unauthorized("Invalid authorization header"))?;
    auth_header
        .strip_prefix("Bearer ")
        .filter(|session_id| !session_id.is_empty())
        .ok_or_else(|| ApiError::unauthorized("Invalid bearer token"))
}

async fn authorized_session(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<tokio::sync::OwnedMutexGuard<crate::SessionState>, ApiError> {
    let session = state.get_session(session_id_from_headers(headers)?).await?;
    Ok(session.lock_owned().await)
}

async fn close_session(State(state): State<AppState>, headers: HeaderMap) -> impl IntoResponse {
    let Ok(session_id) = session_id_from_headers(&headers) else {
        return StatusCode::UNAUTHORIZED;
    };
    let removed = state.sessions.write().await.remove(session_id);
    if removed.is_some() {
        StatusCode::NO_CONTENT
    } else {
        StatusCode::NOT_FOUND
    }
}

async fn list_records(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<Record>>, ApiError> {
    let session = authorized_session(&state, &headers).await?;
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
    headers: HeaderMap,
    AxumPath(record_id): AxumPath<String>,
) -> Result<Json<Record>, ApiError> {
    let session = authorized_session(&state, &headers).await?;
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
    headers: HeaderMap,
    Json(request): Json<AddRecordRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let mut session = authorized_session(&state, &headers).await?;
    session
        .repository
        .add_record(request.record)
        .await
        .map_err(ApiError::from_add_error)?;
    Ok((StatusCode::CREATED, Json(SimpleStatus::new("created"))))
}

async fn update_record(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(record_id): AxumPath<String>,
    Json(request): Json<UpdateRecordRequest>,
) -> Result<Json<SimpleStatus>, ApiError> {
    if request.record.id != record_id {
        return Err(ApiError::bad_request(
            "Record id in path and payload must match",
        ));
    }

    let mut session = authorized_session(&state, &headers).await?;
    session
        .repository
        .update(request.record)
        .await
        .map_err(ApiError::from_update_error)?;
    Ok(Json(SimpleStatus::new("updated")))
}

async fn delete_record(
    State(state): State<AppState>,
    headers: HeaderMap,
    AxumPath(record_id): AxumPath<RecordId>,
) -> Result<StatusCode, ApiError> {
    let mut session = authorized_session(&state, &headers).await?;
    session
        .repository
        .delete(&record_id)
        .await
        .map_err(ApiError::from_update_error)?;
    Ok(StatusCode::NO_CONTENT)
}

async fn save_session(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<SimpleStatus>, ApiError> {
    let mut session = authorized_session(&state, &headers).await?;
    let repository_lock = state
        .repository_lock(session.repository.identifier.as_str())
        .await;
    let _repository_lock = repository_lock.lock().await;
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
    headers: HeaderMap,
) -> Result<Json<SimpleStatus>, ApiError> {
    let mut session = authorized_session(&state, &headers).await?;
    session
        .repository
        .cancel()
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(SimpleStatus::new("cancelled")))
}

async fn export_repository(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, ApiError> {
    let session = authorized_session(&state, &headers).await?;
    let dump = session
        .repository
        .dump()
        .await
        .map_err(ApiError::internal)?;

    Ok(([(header::CONTENT_TYPE, "application/octet-stream")], dump))
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;
    use sec_store::record::Record;
    use sec_store::repository::file::OpenRecordsFileRepository;
    use sec_store::repository::remote::AddRecordRequest;
    use sec_store::repository::{OpenRepository, RecordsRepository};
    use tempfile::TempDir;

    use crate::test_support::{
        build_client, create_repo, open_session, spawn_test_server, test_password,
    };

    #[tokio::test]
    async fn export_uses_unsaved_session_state() {
        let server = spawn_test_server().await.expect("server");
        let client = build_client(&server, true).await.expect("client");
        let password = test_password();

        create_repo(&client, &server, "demo", &password).await;
        let session = open_session(&client, &server, "demo", &password).await;

        let record = Record::new(vec![("name".to_string(), "draft".to_string())]);
        let add_response = client
            .post(format!("{}/session/records", server.base_url))
            .bearer_auth(&session.session_id)
            .json(&AddRecordRequest {
                record: record.clone(),
            })
            .send()
            .await
            .expect("add response");
        assert_eq!(add_response.status(), StatusCode::CREATED);

        let export_response = client
            .get(format!("{}/session/export", server.base_url))
            .bearer_auth(&session.session_id)
            .send()
            .await
            .expect("export response");
        assert_eq!(export_response.status(), StatusCode::OK);
        let dump = export_response.bytes().await.expect("dump bytes");

        let temp_dir = TempDir::new().expect("temp dir");
        let dump_path = temp_dir.path().join("repo.json");
        std::fs::write(&dump_path, dump).expect("write dump");
        let dumped_repo = OpenRecordsFileRepository(dump_path)
            .open(password)
            .await
            .expect("open dumped repo");
        let records = dumped_repo.get_records().await.expect("records");
        assert_eq!(records, vec![record]);
    }

    #[tokio::test]
    async fn concurrent_save_returns_conflict_instead_of_overwriting() {
        let server = spawn_test_server().await.expect("server");
        let client = build_client(&server, true).await.expect("client");
        let password = test_password();

        create_repo(&client, &server, "demo", &password).await;
        let first = open_session(&client, &server, "demo", &password).await;
        let second = open_session(&client, &server, "demo", &password).await;

        let first_record = Record::new(vec![("name".to_string(), "first".to_string())]);
        let second_record = Record::new(vec![("name".to_string(), "second".to_string())]);

        let first_add = client
            .post(format!("{}/session/records", server.base_url))
            .bearer_auth(&first.session_id)
            .json(&AddRecordRequest {
                record: first_record.clone(),
            })
            .send()
            .await
            .expect("first add");
        assert_eq!(first_add.status(), StatusCode::CREATED);

        let second_add = client
            .post(format!("{}/session/records", server.base_url))
            .bearer_auth(&second.session_id)
            .json(&AddRecordRequest {
                record: second_record,
            })
            .send()
            .await
            .expect("second add");
        assert_eq!(second_add.status(), StatusCode::CREATED);

        let save_url = format!("{}/session/save", server.base_url);
        let first_save_url = save_url.clone();
        let second_save_url = save_url;
        let first_client = client.clone();
        let second_client = client.clone();
        let first_request = async {
            first_client
                .post(first_save_url)
                .bearer_auth(&first.session_id)
                .send()
                .await
                .expect("first save")
                .status()
        };
        let second_request = async {
            second_client
                .post(second_save_url)
                .bearer_auth(&second.session_id)
                .send()
                .await
                .expect("second save")
                .status()
        };
        let (first_status, second_status) = tokio::join!(first_request, second_request);
        let mut statuses = vec![first_status, second_status];
        statuses.sort();
        assert_eq!(statuses, vec![StatusCode::OK, StatusCode::CONFLICT]);

        let verify = open_session(&client, &server, "demo", &password).await;
        let records = client
            .get(format!("{}/session/records", server.base_url))
            .bearer_auth(&verify.session_id)
            .send()
            .await
            .expect("records response")
            .json::<Vec<Record>>()
            .await
            .expect("records json");
        assert_eq!(records, vec![first_record]);
    }
}
