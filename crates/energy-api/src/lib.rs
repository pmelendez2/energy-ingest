// SPDX-License-Identifier: MIT

//! Authenticated, read-only local HTTP API (ADR-0014, ADR-0024).
//! Never log Authorization headers or serialize database errors.

use axum::{
    Json, Router,
    body::Bytes,
    extract::DefaultBodyLimit,
    extract::{Path, Query, State},
    http::{HeaderMap, StatusCode, header},
    routing::{get, post},
};
use chrono::NaiveDateTime;
use energy_store::{
    AcquisitionSourceId, HistoryWindow, MAX_WINDOW_CANDIDATES, StoreError,
    authenticate_api_token, create_ingestion_run, get_acquisition_source,
    import_gb_document_atomic_with_hash, plan_history_refresh,
    resolve_meter_window, resolve_physical_meter_stream,
    summarize_meter_validation, window_fingerprint,
};
use gb::parse_reader;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{MySql, Pool};
use utoipa::{OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;

#[derive(Clone)]
pub struct AppState {
    pub pool: Pool<MySql>,
}

#[derive(Debug, Serialize, ToSchema)]
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

#[derive(Serialize, ToSchema, sqlx::FromRow)]
struct AcquisitionSourceResponse {
    id: u64,
    name: String,
    data_custodian_id: u64,
}

#[utoipa::path(
    get,
    path = "/v1/acquisition-sources",
    responses(
        (status = 200, description = "Configured acquisition sources",
         body = Vec<AcquisitionSourceResponse>),
        (status = 401, description = "Bearer token required",
         body = ErrorBody),
    ),
    security(("bearer_auth" = [])),
)]
async fn acquisition_sources(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Vec<AcquisitionSourceResponse>>, ApiError> {
    require_auth(&state.pool, &headers).await?;
    let sources = sqlx::query_as::<_, AcquisitionSourceResponse>(
        "SELECT id, name, data_custodian_id \
         FROM acquisition_source ORDER BY name, id",
    )
    .fetch_all(&state.pool)
    .await
    .map_err(|_| failure(StatusCode::INTERNAL_SERVER_ERROR, "server_error"))?;
    Ok(Json(sources))
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
    physical_meter_id: u64,
    status: &'static str,
    candidate_count: usize,
    selected_interval_count: usize,
    content_fingerprint: String,
    issues: Vec<IssueResponse>,
}

fn validation_response(
    summary: energy_store::MeterValidation,
) -> ValidationResponse {
    ValidationResponse {
        physical_meter_id: summary.physical_meter_id,
        status: summary.status.as_str(),
        candidate_count: summary.candidate_count,
        selected_interval_count: summary.selected_interval_count,
        content_fingerprint: summary.content_fingerprint,
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

/// Maximum number of date ranges in one history planning request.
const MAX_HISTORY_RANGES: usize = 12;

/// Internal UTC date range. Customers select calendar dates in the UI.
#[derive(Deserialize, ToSchema)]
struct HistoryRangeInput {
    start: String,
    end: String,
}

#[derive(Deserialize, ToSchema)]
struct HistoryPreviousInput {
    start: String,
    end: String,
    fingerprint: Option<String>,
}

#[derive(Deserialize, ToSchema)]
struct HistoryPlanRequest {
    ranges: Vec<HistoryRangeInput>,
    /// Omit for first synchronization; otherwise include every same range.
    previous: Option<Vec<HistoryPreviousInput>>,
}

#[derive(Serialize, ToSchema)]
struct HistoryActionResponse {
    start: String,
    end: String,
    action: &'static str,
    /// Absent for unresolved periods or periods without observations.
    current_fingerprint: Option<String>,
}

#[derive(Serialize, ToSchema)]
struct HistoryPlanResponse {
    physical_meter_id: u64,
    ranges: Vec<HistoryActionResponse>,
}

fn parse_history_range(
    start: &str,
    end: &str,
) -> Result<(NaiveDateTime, NaiveDateTime), ApiError> {
    parse_window(&WindowQuery { start: start.to_owned(), end: end.to_owned() })
}

/// Check the partition before performing any database work.
/// Previously saved fingerprints are hints, never validation evidence.
fn history_inputs(
    request: &HistoryPlanRequest,
) -> Result<(Vec<HistoryWindow>, Option<Vec<HistoryWindow>>), ApiError> {
    if request.ranges.is_empty() || request.ranges.len() > MAX_HISTORY_RANGES {
        return Err(failure(StatusCode::BAD_REQUEST, "invalid_ranges"));
    }
    let mut current = Vec::with_capacity(request.ranges.len());
    for range in &request.ranges {
        let (start, end) = parse_history_range(&range.start, &range.end)?;
        current.push(HistoryWindow { start, end, fingerprint: None });
    }
    let previous = request
        .previous
        .as_ref()
        .map(|entries| {
            entries
                .iter()
                .map(|entry| {
                    let (start, end) =
                        parse_history_range(&entry.start, &entry.end)?;
                    Ok(HistoryWindow {
                        start,
                        end,
                        fingerprint: entry.fingerprint.clone(),
                    })
                })
                .collect::<Result<Vec<_>, ApiError>>()
        })
        .transpose()?;
    plan_history_refresh(previous.as_deref(), &current)
        .map_err(|_| failure(StatusCode::BAD_REQUEST, "invalid_ranges"))?;
    Ok((current, previous))
}

fn history_plan_response(
    meter_id: u64,
    previous: Option<&[HistoryWindow]>,
    current: &[HistoryWindow],
) -> Result<HistoryPlanResponse, ApiError> {
    let actions = plan_history_refresh(previous, current)
        .map_err(|_| failure(StatusCode::BAD_REQUEST, "invalid_ranges"))?;
    let ranges = actions
        .into_iter()
        .map(|item| {
            let fingerprint = current
                .iter()
                .find(|range| {
                    range.start == item.start && range.end == item.end
                })
                .and_then(|range| range.fingerprint.clone());
            HistoryActionResponse {
                start: item.start.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
                end: item.end.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
                action: item.action.as_str(),
                current_fingerprint: fingerprint,
            }
        })
        .collect();
    Ok(HistoryPlanResponse { physical_meter_id: meter_id, ranges })
}

/// Read-only, conservative historical date-range reconciliation.
/// No records are changed, and an unresolved range is never published.
#[utoipa::path(
    post,
    path = "/v1/meters/{id}/history/plan",
    params(("id" = u64, Path, description = "Physical meter ID")),
    request_body = HistoryPlanRequest,
    responses(
        (status = 200, description = "Actions for each date range",
         body = HistoryPlanResponse),
        (status = 400, description = "Invalid date ranges", body = ErrorBody),
        (status = 401, description = "Bearer token required", body = ErrorBody),
        (status = 404, description = "Meter not found", body = ErrorBody),
        (status = 413, description = "Reduce range size", body = ErrorBody),
    ),
    security(("bearer_auth" = [])),
)]
async fn history_plan(
    State(state): State<AppState>,
    Path(id): Path<u64>,
    headers: HeaderMap,
    Json(request): Json<HistoryPlanRequest>,
) -> Result<Json<HistoryPlanResponse>, ApiError> {
    require_auth(&state.pool, &headers).await?;
    let (mut current, previous) = history_inputs(&request)?;
    for range in &mut current {
        let selected =
            resolve_meter_window(&state.pool, id, range.start, range.end)
                .await
                .map_err(store_failure)?
                .ok_or_else(|| failure(StatusCode::NOT_FOUND, "not_found"))?;
        if selected.candidate_count > MAX_WINDOW_CANDIDATES as usize {
            return Err(failure(
                StatusCode::PAYLOAD_TOO_LARGE,
                "window_too_large",
            ));
        }
        // An empty or conflicting range is blocked, not zero usage.
        if selected.candidate_count > 0 && selected.issues.is_empty() {
            range.fingerprint = Some(window_fingerprint(
                id,
                range.start,
                range.end,
                &selected.intervals,
            ));
        }
    }
    Ok(Json(history_plan_response(id, previous.as_deref(), &current)?))
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
    paths(healthz, meters, acquisition_sources, stream, window, validation, history_plan,
          import_green_button),
    components(schemas(
        ErrorBody, Meter, AcquisitionSourceResponse, IntervalResponse, IssueResponse, StreamResponse,
        ValidationResponse, HistoryRangeInput, HistoryPreviousInput,
        HistoryPlanRequest, HistoryActionResponse, HistoryPlanResponse,
        ImportResult
    )),
    modifiers(&BearerSecurity),
    info(
        title = "Energy Ingest HTTP API",
        version = "0.1.0",
        description = "Authenticated meter history and Green Button import"
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

/// A single authenticated Green Button XML import. The source ID is the
/// existing acquisition-source identity, not a physical meter ID.
/// The same atomic importer used by energy-store-import commits the file.
const MAX_IMPORT_BYTES: usize = 8 * 1024 * 1024;

#[derive(Debug, Deserialize)]
struct ImportQuery {
    source_id: u64,
}

#[derive(Serialize, ToSchema)]
struct ImportResult {
    observations: usize,
    unchanged: usize,
    accepted: usize,
}

#[utoipa::path(
    post,
    path = "/v1/imports/green-button",
    params(("source_id" = u64, Query,
             description = "Existing acquisition source ID")),
    responses(
        (status = 200, description = "XML committed", body = ImportResult),
        (status = 400, description = "Invalid XML or source", body = ErrorBody),
        (status = 401, description = "Bearer token required", body = ErrorBody),
        (status = 404, description = "Source not found", body = ErrorBody),
        (status = 413, description = "XML too large", body = ErrorBody),
    ),
    security(("bearer_auth" = [])),
)]
async fn import_green_button(
    State(state): State<AppState>,
    headers: HeaderMap,
    Query(query): Query<ImportQuery>,
    body: Bytes,
) -> Result<Json<ImportResult>, ApiError> {
    require_auth(&state.pool, &headers).await?;
    if query.source_id == 0 || body.is_empty() {
        return Err(failure(StatusCode::BAD_REQUEST, "invalid_import"));
    }
    if body.len() > MAX_IMPORT_BYTES {
        return Err(failure(StatusCode::PAYLOAD_TOO_LARGE, "file_too_large"));
    }
    let source = AcquisitionSourceId(query.source_id);
    if get_acquisition_source(&state.pool, source)
        .await
        .map_err(store_failure)?
        .is_none()
    {
        return Err(failure(StatusCode::NOT_FOUND, "source_not_found"));
    }
    let document = parse_reader(body.as_ref())
        .map_err(|_| failure(StatusCode::BAD_REQUEST, "invalid_xml"))?;
    let hash = format!("{:x}", Sha256::digest(&body));
    let run = create_ingestion_run(&state.pool, source)
        .await
        .map_err(store_failure)?;
    let report = import_gb_document_atomic_with_hash(
        &state.pool,
        source,
        run,
        &document,
        Some(&hash),
    )
    .await
    .map_err(|_| {
        failure(StatusCode::UNPROCESSABLE_ENTITY, "import_rejected")
    })?;
    Ok(Json(ImportResult {
        observations: report.observations,
        unchanged: report.unchanged,
        accepted: report.accepted,
    }))
}

/// The health probe reveals no database or customer information.
pub fn router(pool: Pool<MySql>) -> Router {
    Router::new()
        .route("/healthz", get(healthz))
        .route("/v1/meters", get(meters))
        .route("/v1/acquisition-sources", get(acquisition_sources))
        .route("/v1/imports/green-button", post(import_green_button))
        .layer(DefaultBodyLimit::max(MAX_IMPORT_BYTES))
        .route("/v1/meters/{id}/stream", get(stream))
        .route("/v1/meters/{id}/validation", get(validation))
        .route("/v1/meters/{id}/window", get(window))
        .route("/v1/meters/{id}/history/plan", post(history_plan))
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
        assert_eq!(response.physical_meter_id, summary.physical_meter_id);
        assert_eq!(response.candidate_count, summary.candidate_count);
        assert_eq!(response.selected_interval_count, 0);
        assert_eq!(response.issues[0].kind, summary.issues[0].kind.as_str());
        let payload = serde_json::to_value(&response).unwrap();
        assert!(payload.get("contract_version").is_none());
        assert_eq!(payload["status"], "unresolved");
        assert_eq!(payload["content_fingerprint"].as_str().unwrap().len(), 64);
        assert_eq!(payload["issues"][0]["kind"], "conflicting_duplicate");
    }

    fn date_range(start: &str, end: &str) -> HistoryRangeInput {
        HistoryRangeInput { start: start.to_owned(), end: end.to_owned() }
    }

    #[test]
    fn first_sync_uses_only_server_fingerprints() {
        let input = HistoryPlanRequest {
            ranges: vec![date_range(
                "2026-10-01T00:00:00Z",
                "2026-10-02T00:00:00Z",
            )],
            previous: None,
        };
        let (mut current, old) = history_inputs(&input).unwrap();
        current[0].fingerprint = Some("server-value".to_owned());
        let report =
            history_plan_response(5, old.as_deref(), &current).unwrap();
        let payload = serde_json::to_value(&report).unwrap();
        assert!(payload.get("contract_version").is_none());
        assert_eq!(report.physical_meter_id, 5);
        assert_eq!(report.ranges[0].action, "refresh");
        assert_eq!(
            report.ranges[0].current_fingerprint.as_deref(),
            Some("server-value"),
        );
    }

    #[test]
    fn unchanged_and_corrected_ranges_are_distinguished() {
        let input = HistoryPlanRequest {
            ranges: vec![
                date_range("2026-10-01T00:00:00Z", "2026-10-02T00:00:00Z"),
                date_range("2026-10-02T00:00:00Z", "2026-10-03T00:00:00Z"),
            ],
            previous: Some(vec![
                HistoryPreviousInput {
                    start: "2026-10-01T00:00:00Z".to_owned(),
                    end: "2026-10-02T00:00:00Z".to_owned(),
                    fingerprint: Some("same".to_owned()),
                },
                HistoryPreviousInput {
                    start: "2026-10-02T00:00:00Z".to_owned(),
                    end: "2026-10-03T00:00:00Z".to_owned(),
                    fingerprint: Some("before".to_owned()),
                },
            ]),
        };
        let (mut current, old) = history_inputs(&input).unwrap();
        current[0].fingerprint = Some("same".to_owned());
        current[1].fingerprint = Some("after".to_owned());
        let report =
            history_plan_response(1, old.as_deref(), &current).unwrap();
        assert_eq!(report.ranges[0].action, "unchanged");
        assert_eq!(report.ranges[1].action, "refresh");
    }

    #[test]
    fn unresolved_and_empty_ranges_are_blocked() {
        let input = HistoryPlanRequest {
            ranges: vec![date_range(
                "2026-10-01T00:00:00Z",
                "2026-10-02T00:00:00Z",
            )],
            previous: None,
        };
        let (current, previous) = history_inputs(&input).unwrap();
        let report =
            history_plan_response(1, previous.as_deref(), &current).unwrap();
        assert_eq!(report.ranges[0].action, "blocked");
        assert!(report.ranges[0].current_fingerprint.is_none());
    }

    #[test]
    fn invalid_partitions_fail_before_database_queries() {
        let input = HistoryPlanRequest {
            ranges: vec![
                date_range("2026-10-01T00:00:00Z", "2026-10-03T00:00:00Z"),
                date_range("2026-10-02T00:00:00Z", "2026-10-04T00:00:00Z"),
            ],
            previous: None,
        };
        assert!(history_inputs(&input).is_err());
    }

    #[test]
    fn changed_partitions_and_excessive_ranges_are_rejected() {
        let one = date_range("2026-10-01T00:00:00Z", "2026-10-02T00:00:00Z");
        let mismatch =
            HistoryPlanRequest { ranges: vec![one], previous: Some(vec![]) };
        assert!(history_inputs(&mismatch).is_err());
        let excessive = HistoryPlanRequest {
            ranges: (0..=MAX_HISTORY_RANGES)
                .map(|_| {
                    date_range("2026-10-01T00:00:00Z", "2026-10-02T00:00:00Z")
                })
                .collect(),
            previous: None,
        };
        assert!(history_inputs(&excessive).is_err());
    }
}

#[cfg(test)]
mod acquisition_source_api_contract_tests {
    use super::*;

    #[test]
    fn source_catalog_is_documented() {
        let doc = serde_json::to_value(openapi()).expect("OpenAPI JSON");
        let path = &doc["paths"]["/v1/acquisition-sources"]["get"];
        assert!(path.is_object());
        assert!(path["security"].is_array());
    }
}
