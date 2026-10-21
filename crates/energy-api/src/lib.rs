// SPDX-License-Identifier: MIT

//! Authenticated, read-only local HTTP API (ADR-0014, ADR-0024).
//! Never log Authorization headers or serialize database errors.

use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    routing::get,
};
use chrono::NaiveDateTime;
use energy_store::{
    MAX_WINDOW_CANDIDATES, StoreError, authenticate_api_token,
    resolve_meter_window, resolve_physical_meter_stream,
    summarize_meter_validation, window_fingerprint,
};
use serde::{Deserialize, Serialize};
use sqlx::{MySql, Pool};
use utoipa::{OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;

#[derive(Clone)]
pub struct AppState {
    pub pool: Pool<MySql>,
}

#[derive(Serialize, ToSchema)]
struct ErrorBody {
    error: &'static str,
}

type ApiError = (StatusCode, Json<ErrorBody>);

fn failure(code: StatusCode, message: &'static str) -> ApiError {
    (code, Json(ErrorBody { error: message }))
}

fn store_failure(error: StoreError) -> ApiError {
    match error {
        StoreError::NotFound => failure(StatusCode::NOT_FOUND, "not_found"),
        _ => failure(StatusCode::INTERNAL_SERVER_ERROR, "server_error"),
    }
}

async fn require_auth(
    pool: &Pool<MySql>,
    headers: &HeaderMap,
) -> Result<(), ApiError> {
    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    let Some(token) = token else {
        return Err(failure(StatusCode::UNAUTHORIZED, "unauthorized"));
    };
    match authenticate_api_token(pool, token).await {
        Ok(Some(_)) => Ok(()),
        Ok(None) => Err(failure(StatusCode::UNAUTHORIZED, "unauthorized")),
        Err(_) => {
            Err(failure(StatusCode::SERVICE_UNAVAILABLE, "auth_unavailable"))
        }
    }
}

#[derive(Serialize, ToSchema, sqlx::FromRow)]
struct Meter {
    id: u64,
    display_name: String,
    commodity: u32,
}

#[utoipa::path(
    get,
    path = "/v1/meters",
    responses(
        (status = 200, description = "Configured physical meters",
         body = Vec<Meter>),
        (status = 401, description = "Invalid or absent bearer token",
         body = ErrorBody),
        (status = 503, description = "Authentication store unavailable",
         body = ErrorBody),
    ),
    security(("bearer_auth" = [])),
)]
async fn meters(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<Meter>>, ApiError> {
    require_auth(&state.pool, &headers).await?;
    let items = sqlx::query_as::<_, Meter>(
        "SELECT id, display_name, commodity \
         FROM physical_meter ORDER BY id",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|_| failure(StatusCode::INTERNAL_SERVER_ERROR, "server_error"))?;
    Ok(Json(items))
}

#[derive(Serialize, ToSchema)]
struct IntervalResponse {
    start_utc: String,
    duration_seconds: u32,
    raw_value: i64,
    meter_reading_id: u64,
    accepted_revision_id: u64,
    power_of_ten_multiplier: i16,
    unit_of_measure: u32,
}

#[derive(Serialize, ToSchema)]
struct IssueResponse {
    kind: &'static str,
    interval_reading_id: u64,
    other_interval_reading_id: Option<u64>,
}

#[derive(Serialize, ToSchema)]
struct StreamResponse {
    physical_meter_id: u64,
    candidate_count: usize,
    resolved: bool,
    intervals: Vec<IntervalResponse>,
    issues: Vec<IssueResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    window_fingerprint: Option<String>,
}

#[derive(Serialize, ToSchema)]
struct ValidationResponse {
    contract_version: u32,
    physical_meter_id: u64,
    status: &'static str,
    candidate_count: usize,
    selected_interval_count: usize,
    issues: Vec<IssueResponse>,
}

fn validation_response(
    summary: energy_store::MeterValidation,
) -> ValidationResponse {
    ValidationResponse {
        contract_version: summary.contract_version,
        physical_meter_id: summary.physical_meter_id,
        status: summary.status.as_str(),
        candidate_count: summary.candidate_count,
        selected_interval_count: summary.selected_interval_count,
        issues: summary
            .issues
            .into_iter()
            .map(|item| IssueResponse {
                kind: item.kind.as_str(),
                interval_reading_id: item.interval_reading_id,
                other_interval_reading_id: item.other_interval_reading_id,
            })
            .collect(),
    }
}

/// Canonical read-only validation facts, without usage totals.
#[utoipa::path(
    get,
    path = "/v1/meters/{id}/validation",
    params(("id" = u64, Path, description = "Physical meter ID")),
    responses(
        (status = 200, description = "Canonical validation summary",
         body = ValidationResponse),
        (status = 401, description = "Bearer token required",
         body = ErrorBody),
        (status = 404, description = "Meter not found", body = ErrorBody),
    ),
    security(("bearer_auth" = [])),
)]
async fn validation(
    State(state): State<AppState>,
    Path(id): Path<u64>,
    headers: HeaderMap,
) -> Result<Json<ValidationResponse>, ApiError> {
    require_auth(&state.pool, &headers).await?;
    let resolution = resolve_physical_meter_stream(&state.pool, id)
        .await
        .map_err(store_failure)?;
    let summary = summarize_meter_validation(id, &resolution);
    Ok(Json(validation_response(summary)))
}

#[utoipa::path(
    get,
    path = "/v1/meters/{id}/stream",
    params(("id" = u64, Path, description = "Physical meter ID")),
    responses(
        (status = 200, description = "Selected intervals or conflicts",
         body = StreamResponse),
        (status = 401, description = "Invalid or absent bearer token",
         body = ErrorBody),
        (status = 404, description = "Meter not found", body = ErrorBody),
        (status = 503, description = "Authentication store unavailable",
         body = ErrorBody),
    ),
    security(("bearer_auth" = [])),
)]
async fn stream(
    State(state): State<AppState>,
    Path(id): Path<u64>,
    headers: HeaderMap,
) -> Result<Json<StreamResponse>, ApiError> {
    require_auth(&state.pool, &headers).await?;
    let selected = resolve_physical_meter_stream(&state.pool, id)
        .await
        .map_err(store_failure)?;
    let resolved = selected.issues.is_empty();
    let intervals = selected
        .intervals
        .into_iter()
        .map(|item| IntervalResponse {
            start_utc: item.start.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            duration_seconds: item.duration_seconds,
            raw_value: item.raw_value,
            meter_reading_id: item.meter_reading_id,
            accepted_revision_id: item.accepted_revision_id,
            power_of_ten_multiplier: item.power_of_ten_multiplier,
            unit_of_measure: item.unit_of_measure,
        })
        .collect();
    let issues = selected
        .issues
        .into_iter()
        .map(|item| IssueResponse {
            kind: item.kind.as_str(),
            interval_reading_id: item.interval_reading_id,
            other_interval_reading_id: item.other_interval_reading_id,
        })
        .collect();
    Ok(Json(StreamResponse {
        physical_meter_id: id,
        candidate_count: selected.candidate_count,
        resolved,
        intervals,
        issues,
        window_fingerprint: None,
    }))
}

/// Explicit UTC window, limited to 31 days per request.
#[derive(Deserialize)]
struct WindowQuery {
    start: String,
    end: String,
}

fn parse_window(
    query: &WindowQuery,
) -> Result<(NaiveDateTime, NaiveDateTime), ApiError> {
    fn parse(value: &str) -> Option<NaiveDateTime> {
        if !value.ends_with('Z') {
            return None;
        }
        NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%SZ").ok()
    }
    let start = parse(&query.start)
        .ok_or_else(|| failure(StatusCode::BAD_REQUEST, "invalid_window"))?;
    let end = parse(&query.end)
        .ok_or_else(|| failure(StatusCode::BAD_REQUEST, "invalid_window"))?;
    let seconds = end.signed_duration_since(start).num_seconds();
    if seconds <= 0 || seconds > 31 * 86_400 {
        return Err(failure(StatusCode::BAD_REQUEST, "invalid_window"));
    }
    Ok((start, end))
}

#[utoipa::path(
    get,
    path = "/v1/meters/{id}/window",
    params(
        ("id" = u64, Path, description = "Physical meter ID"),
        ("start" = String, Query, description = "UTC start, inclusive"),
        ("end" = String, Query, description = "UTC end, exclusive"),
    ),
    responses(
        (status = 200, description = "Resolved window", body = StreamResponse),
        (status = 400, description = "Invalid UTC range", body = ErrorBody),
        (status = 401, description = "Bearer token required", body = ErrorBody),
        (status = 404, description = "Meter not found", body = ErrorBody),
        (status = 409, description = "Ambiguous observations", body = ErrorBody),
        (status = 413, description = "Reduce window size", body = ErrorBody),
    ),
    security(("bearer_auth" = [])),
)]
async fn window(
    State(state): State<AppState>,
    Path(id): Path<u64>,
    Query(query): Query<WindowQuery>,
    headers: HeaderMap,
) -> Result<Json<StreamResponse>, ApiError> {
    require_auth(&state.pool, &headers).await?;
    let (start, end) = parse_window(&query)?;
    let selected = resolve_meter_window(&state.pool, id, start, end)
        .await
        .map_err(store_failure)?
        .ok_or_else(|| failure(StatusCode::NOT_FOUND, "not_found"))?;
    if selected.candidate_count > MAX_WINDOW_CANDIDATES as usize {
        return Err(failure(StatusCode::PAYLOAD_TOO_LARGE, "window_too_large"));
    }
    if !selected.issues.is_empty() {
        return Err(failure(StatusCode::CONFLICT, "unresolved_stream"));
    }
    let count = selected.candidate_count;
    let fingerprint = window_fingerprint(id, start, end, &selected.intervals);
    let intervals = selected
        .intervals
        .into_iter()
        .map(|item| IntervalResponse {
            start_utc: item.start.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
            duration_seconds: item.duration_seconds,
            raw_value: item.raw_value,
            meter_reading_id: item.meter_reading_id,
            accepted_revision_id: item.accepted_revision_id,
            power_of_ten_multiplier: item.power_of_ten_multiplier,
            unit_of_measure: item.unit_of_measure,
        })
        .collect();
    Ok(Json(StreamResponse {
        physical_meter_id: id,
        candidate_count: count,
        resolved: true,
        intervals,
        issues: Vec::new(),
        window_fingerprint: Some(fingerprint),
    }))
}

#[utoipa::path(
    get,
    path = "/healthz",
    responses((status = 204, description = "Process is alive")),
)]
async fn healthz() -> StatusCode {
    StatusCode::NO_CONTENT
}

#[derive(OpenApi)]
#[openapi(
    paths(healthz, meters, stream, window, validation),
    components(schemas(
        ErrorBody, Meter, IntervalResponse, IssueResponse, StreamResponse,
        ValidationResponse
    )),
    modifiers(&BearerSecurity),
    info(
        title = "Energy Ingest HTTP API",
        version = "0.1.0",
        description = "Read-only physical meter and raw usage observations"
    ),
)]
struct ApiDoc;

struct BearerSecurity;

impl utoipa::Modify for BearerSecurity {
    fn modify(&self, api: &mut utoipa::openapi::OpenApi) {
        use utoipa::openapi::security::{
            HttpAuthScheme, HttpBuilder, SecurityScheme,
        };
        let components = api.components.get_or_insert_with(Default::default);
        components.add_security_scheme(
            "bearer_auth",
            SecurityScheme::Http(
                HttpBuilder::new().scheme(HttpAuthScheme::Bearer).build(),
            ),
        );
    }
}

/// Contract derived from the actual Rust HTTP handlers and response types.
pub fn openapi() -> utoipa::openapi::OpenApi {
    ApiDoc::openapi()
}

/// The health probe reveals no database or customer information.
pub fn router(pool: Pool<MySql>) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/meters", get(meters))
        .route("/v1/meters/{id}/stream", get(stream))
        .route("/v1/meters/{id}/validation", get(validation))
        .route("/v1/meters/{id}/window", get(window))
        .with_state(AppState { pool })
}

/// Enable Swagger UI only for explicitly opted-in development instances.
/// Deployment must additionally bind to a loopback address.
pub fn developer_router(pool: Pool<MySql>) -> Router {
    router(pool).merge(
        SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", openapi()),
    )
}

#[cfg(test)]
mod validation_contract_tests {
    use super::*;
    use energy_store::{StreamIssue, StreamIssueKind, StreamResolution};

    #[test]
    fn http_projection_preserves_canonical_state() {
        let resolution = StreamResolution {
            candidate_count: 2,
            intervals: Vec::new(),
            issues: vec![StreamIssue {
                kind: StreamIssueKind::ConflictingDuplicate,
                interval_reading_id: 7,
                other_interval_reading_id: Some(6),
            }],
        };
        let summary = summarize_meter_validation(3, &resolution);
        let response = validation_response(summary.clone());
        assert_eq!(response.status, summary.status.as_str());
        assert_eq!(response.contract_version, summary.contract_version);
        assert_eq!(response.physical_meter_id, summary.physical_meter_id);
        assert_eq!(response.candidate_count, summary.candidate_count);
        assert_eq!(response.selected_interval_count, 0);
        assert_eq!(response.issues[0].kind, summary.issues[0].kind.as_str());
        let payload = serde_json::to_value(&response).unwrap();
        assert_eq!(payload["status"], "unresolved");
        assert_eq!(payload["issues"][0]["kind"], "conflicting_duplicate");
    }
}
