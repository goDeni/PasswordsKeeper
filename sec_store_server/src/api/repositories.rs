use axum::{
    extract::{Path as AxumPath, State},
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Json, Router,
};
use sec_store::repository::remote::{
    CreateRepositoryRequest, OpenRepositoryRequest, OpenRepositoryResponse,
};
use sec_store::repository::RepositoriesSource;

use super::SimpleStatus;
use crate::{ApiError, AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/repositories/{repository_name}", post(create_repository))
        .route(
            "/repositories/{repository_name}/sessions",
            post(open_repository),
        )
}

async fn create_repository(
    State(state): State<AppState>,
    AxumPath(repository_name): AxumPath<String>,
    Json(request): Json<CreateRepositoryRequest>,
) -> Result<impl IntoResponse, ApiError> {
    state
        .repositories
        .create_repository(&repository_name, request.password)
        .await
        .map_err(ApiError::from_create_error)?;

    Ok((StatusCode::CREATED, Json(SimpleStatus::new("created"))))
}

async fn open_repository(
    State(state): State<AppState>,
    AxumPath(repository_name): AxumPath<String>,
    Json(request): Json<OpenRepositoryRequest>,
) -> Result<Json<OpenRepositoryResponse>, ApiError> {
    let repository = state
        .repositories
        .open_repository(&repository_name, request.password)
        .await
        .map_err(ApiError::from_open_error)?;
    let session_id = state
        .insert_session(repository)
        .await
        .map_err(ApiError::internal)?;
    Ok(Json(OpenRepositoryResponse { session_id }))
}

#[cfg(test)]
mod tests {
    use axum::http::StatusCode;
    use sec_store::record::Record;
    use sec_store::repository::remote::{
        AddRecordRequest, CreateRepositoryRequest, OpenRepositoryRequest, OpenRepositoryResponse,
    };

    use crate::test_support::{build_client, spawn_test_server};

    #[tokio::test]
    async fn mtls_server_rejects_unknown_client_and_persists_records() {
        let server = spawn_test_server().await.expect("server");
        let allowed_client = build_client(&server, true).await.expect("allowed client");

        let create_response = allowed_client
            .post(format!("{}/repositories/demo", server.base_url))
            .json(&CreateRepositoryRequest {
                password: "secret".to_string(),
            })
            .send()
            .await
            .expect("create response");
        assert_eq!(create_response.status(), StatusCode::CREATED);

        let session = allowed_client
            .post(format!("{}/repositories/demo/sessions", server.base_url))
            .json(&OpenRepositoryRequest {
                password: "secret".to_string(),
            })
            .send()
            .await
            .expect("open response")
            .json::<OpenRepositoryResponse>()
            .await
            .expect("open json");

        let record = Record::new(vec![("name".to_string(), "mail".to_string())]);
        let add_response = allowed_client
            .post(format!(
                "{}/sessions/{}/records",
                server.base_url, session.session_id
            ))
            .json(&AddRecordRequest {
                record: record.clone(),
            })
            .send()
            .await
            .expect("add response");
        assert_eq!(add_response.status(), StatusCode::CREATED);

        let save_response = allowed_client
            .post(format!(
                "{}/sessions/{}/save",
                server.base_url, session.session_id
            ))
            .send()
            .await
            .expect("save response");
        assert_eq!(save_response.status(), StatusCode::OK);

        let second_session = allowed_client
            .post(format!("{}/repositories/demo/sessions", server.base_url))
            .json(&OpenRepositoryRequest {
                password: "secret".to_string(),
            })
            .send()
            .await
            .expect("second open response")
            .json::<OpenRepositoryResponse>()
            .await
            .expect("second open json");

        let records = allowed_client
            .get(format!(
                "{}/sessions/{}/records",
                server.base_url, second_session.session_id
            ))
            .send()
            .await
            .expect("records response")
            .json::<Vec<Record>>()
            .await
            .expect("records json");
        assert_eq!(records, vec![record]);

        let denied_client = build_client(&server, false).await.expect("denied client");
        let denied_result = denied_client
            .get(format!(
                "{}/sessions/{}/records",
                server.base_url, second_session.session_id
            ))
            .send()
            .await;
        assert!(denied_result.is_err());
    }

    #[tokio::test]
    async fn invalid_repository_names_return_bad_request() {
        let server = spawn_test_server().await.expect("server");
        let client = build_client(&server, true).await.expect("client");

        let create_response = client
            .post(format!("{}/repositories/invalid%3Aname", server.base_url))
            .json(&CreateRepositoryRequest {
                password: "secret".to_string(),
            })
            .send()
            .await
            .expect("create response");
        assert_eq!(create_response.status(), StatusCode::BAD_REQUEST);

        let open_response = client
            .post(format!(
                "{}/repositories/invalid%3Aname/sessions",
                server.base_url
            ))
            .json(&OpenRepositoryRequest {
                password: "secret".to_string(),
            })
            .send()
            .await
            .expect("open response");
        assert_eq!(open_response.status(), StatusCode::BAD_REQUEST);
    }
}
