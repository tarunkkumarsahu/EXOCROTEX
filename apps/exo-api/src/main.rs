//! Loopback-only HTTP view of the existing cognitive-core SQLite store.
//! V0.4 exposes explicit memory and evidence operations; it never calls a model or executes tools.

use std::{env, path::PathBuf, sync::Arc};

use axum::{
    extract::{DefaultBodyLimit, Path as UrlPath, Query, State},
    http::StatusCode,
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use cognitive_core::{
    CognitiveEvent, EpistemicType, EventSource, FactKind, MemoryConflict, MemoryKind, Observation,
    PersistentMemoryStore, StoreError, StoredMemory, WorkingFact,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone)]
struct AppState {
    db_path: Arc<PathBuf>,
}

#[derive(Debug)]
enum ApiError {
    BadRequest(String),
    NotFound,
    Conflict(String),
    Internal,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::BadRequest(s) => (StatusCode::BAD_REQUEST, s),
            Self::NotFound => (StatusCode::NOT_FOUND, "Resource not found".into()),
            Self::Conflict(s) => (StatusCode::CONFLICT, s),
            Self::Internal => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Local storage error".into(),
            ),
        };
        (status, Json(serde_json::json!({ "error": message }))).into_response()
    }
}

impl From<StoreError> for ApiError {
    fn from(value: StoreError) -> Self {
        match value {
            StoreError::InvalidInput(msg) => Self::BadRequest(msg.into()),
            StoreError::NotFound => Self::NotFound,
            StoreError::InactiveMemory | StoreError::PathExists => {
                Self::Conflict("Record is no longer active or the destination exists".into())
            }
            other => {
                eprintln!("EXOCORTEX store error: {other}");
                Self::Internal
            }
        }
    }
}

// Each blocking handler opens a short-lived SQLite connection. rusqlite::Connection
// is never shared across async tasks, and existing V0.2/V0.3 schema remains canonical.
async fn store_call<T, F>(state: AppState, op: F) -> Result<Json<T>, ApiError>
where
    T: Serialize + Send + 'static,
    F: FnOnce(&mut PersistentMemoryStore) -> Result<T, StoreError> + Send + 'static,
{
    tokio::task::spawn_blocking(move || {
        let mut store = PersistentMemoryStore::open(&*state.db_path)?;
        op(&mut store)
    })
    .await
    .map_err(|_| ApiError::Internal)?
    .map(Json)
    .map_err(ApiError::from)
}

#[derive(Serialize)]
struct Health {
    ok: bool,
    service: &'static str,
    version: &'static str,
}

async fn health() -> Json<Health> {
    Json(Health {
        ok: true,
        service: "exocortex",
        version: "0.4",
    })
}

#[derive(Serialize)]
struct Status {
    memories: i64,
    events: i64,
    observations: i64,
    facts: i64,
    ai_connected: bool,
    jarvis_connected: bool,
}

async fn status(State(state): State<AppState>) -> Result<Json<Status>, ApiError> {
    store_call(state, |s| {
        Ok(Status {
            memories: s.memory_count()?,
            events: s.event_count()?,
            observations: s.observation_count()?,
            facts: s.working_fact_count()?,
            ai_connected: false,
            jarvis_connected: false,
        })
    })
    .await
}

#[derive(Deserialize, Default)]
struct MemoryQuery {
    #[serde(default)]
    q: String,
}

async fn memories(
    State(state): State<AppState>,
    Query(query): Query<MemoryQuery>,
) -> Result<Json<Vec<StoredMemory>>, ApiError> {
    if query.q.len() > 250 {
        return Err(ApiError::BadRequest(
            "Search must be 250 characters or less".into(),
        ));
    }
    store_call(state, move |s| s.recall(&query.q)).await
}

#[derive(Deserialize)]
struct MemoryInput {
    text: String,
    #[serde(default = "semantic")]
    kind: MemoryKind,
    topic: Option<String>,
}
fn semantic() -> MemoryKind {
    MemoryKind::Semantic
}

async fn remember(
    State(state): State<AppState>,
    Json(input): Json<MemoryInput>,
) -> Result<(StatusCode, Json<StoredMemory>), ApiError> {
    if input.text.len() > 8000 || input.topic.as_ref().is_some_and(|t| t.len() > 120) {
        return Err(ApiError::BadRequest("Memory or topic too long".into()));
    }
    let result = store_call(state, move |s| {
        s.create_memory(
            &input.text,
            input.kind,
            EpistemicType::UserConfirmedFact,
            input.topic.as_deref(),
        )
    })
    .await?;
    Ok((StatusCode::CREATED, result))
}

#[derive(Deserialize)]
struct Correction {
    text: String,
}

async fn correct(
    State(state): State<AppState>,
    UrlPath(id): UrlPath<Uuid>,
    Json(input): Json<Correction>,
) -> Result<Json<StoredMemory>, ApiError> {
    if input.text.len() > 8000 {
        return Err(ApiError::BadRequest("Correction too long".into()));
    }
    store_call(state, move |s| s.correct(id, &input.text)).await
}

#[derive(Serialize)]
struct Redaction {
    redacted_revisions: usize,
}

async fn forget(
    State(state): State<AppState>,
    UrlPath(id): UrlPath<Uuid>,
) -> Result<Json<Redaction>, ApiError> {
    store_call(state, move |s| {
        s.forget(id)
            .map(|redacted_revisions| Redaction { redacted_revisions })
    })
    .await
}

async fn source(
    State(state): State<AppState>,
    UrlPath(id): UrlPath<Uuid>,
) -> Result<Json<CognitiveEvent>, ApiError> {
    store_call(state, move |s| s.source(id)?.ok_or(StoreError::NotFound)).await
}

async fn conflicts(
    State(state): State<AppState>,
    UrlPath(id): UrlPath<Uuid>,
) -> Result<Json<Vec<MemoryConflict>>, ApiError> {
    store_call(state, move |s| s.possible_conflicts(id)).await
}

#[derive(Deserialize, Default)]
struct FactQuery {
    #[serde(default)]
    entity: String,
}

async fn facts(
    State(state): State<AppState>,
    Query(query): Query<FactQuery>,
) -> Result<Json<Vec<WorkingFact>>, ApiError> {
    if query.entity.len() > 120 {
        return Err(ApiError::BadRequest("Entity too long".into()));
    }
    store_call(state, move |s| {
        if query.entity.trim().is_empty() {
            s.all_working_facts()
        } else {
            s.working_facts(&query.entity)
        }
    })
    .await
}

#[derive(Deserialize)]
struct ObservationInput {
    source_key: String,
    version: i64,
    value: String,
}

async fn observe(
    State(state): State<AppState>,
    Json(input): Json<ObservationInput>,
) -> Result<(StatusCode, Json<Observation>), ApiError> {
    if input.source_key.len() > 120 || input.value.len() > 8000 {
        return Err(ApiError::BadRequest("Source key or value too long".into()));
    }
    let result = store_call(state, move |s| {
        s.record_observation(
            &input.source_key,
            input.version,
            &input.value,
            EventSource::User,
        )
    })
    .await?;
    Ok((StatusCode::CREATED, result))
}

#[derive(Deserialize)]
struct ObservationQuery {
    source_key: String,
}

async fn latest_observation(
    State(state): State<AppState>,
    Query(query): Query<ObservationQuery>,
) -> Result<Json<Observation>, ApiError> {
    store_call(state, move |s| {
        s.latest_observation(&query.source_key)?
            .ok_or(StoreError::NotFound)
    })
    .await
}

#[derive(Deserialize)]
struct FactInput {
    entity: String,
    attribute: String,
    value: String,
    kind: FactKind,
    evidence_ids: Vec<Uuid>,
}

async fn add_fact(
    State(state): State<AppState>,
    Json(input): Json<FactInput>,
) -> Result<(StatusCode, Json<WorkingFact>), ApiError> {
    if input.entity.len() > 120
        || input.attribute.len() > 120
        || input.value.len() > 8000
        || input.evidence_ids.len() > 20
    {
        return Err(ApiError::BadRequest(
            "Fact fields or evidence list too long".into(),
        ));
    }
    let result = store_call(state, move |s| {
        s.create_working_fact(
            &input.entity,
            &input.attribute,
            &input.value,
            input.kind,
            &input.evidence_ids,
        )
    })
    .await?;
    Ok((StatusCode::CREATED, result))
}

// Browser cross-origin requests cannot include this non-simple header unless CORS
// grants preflight. We intentionally do not enable broad CORS. This is NOT client
// authentication: a local process can still access a loopback service.
async fn require_client_header(request: axum::extract::Request, next: Next) -> Response {
    if request
        .headers()
        .get("x-exocortex-client")
        .is_none_or(|h| h != "local-ui-v1")
    {
        return ApiError::BadRequest("Missing local UI client header".into()).into_response();
    }
    let allowed = ["http://localhost:5173", "http://127.0.0.1:5173"];
    if request
        .headers()
        .get(axum::http::header::ORIGIN)
        .is_some_and(|h| !allowed.iter().any(|o| h == *o))
    {
        return ApiError::BadRequest("Unrecognized browser origin".into()).into_response();
    }
    next.run(request).await
}

fn router(state: AppState) -> Router {
    let routes = Router::new()
        .route("/status", get(status))
        .route("/memories", get(memories).post(remember))
        .route(
            "/memories/:id",
            axum::routing::patch(correct).delete(forget),
        )
        .route("/sources/:id", get(source))
        .route("/conflicts/:id", get(conflicts))
        .route("/facts", get(facts).post(add_fact))
        .route("/observations", post(observe))
        .route("/observations/latest", get(latest_observation))
        .route_layer(middleware::from_fn(require_client_header));
    Router::new()
        .route("/api/health", get(health))
        .nest("/api", routes)
        .layer(DefaultBodyLimit::max(16 * 1024))
        .with_state(state)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args_os().skip(1);
    let db = match args.next() {
        None => {
            let home = env::var_os("USERPROFILE")
                .or_else(|| env::var_os("HOME"))
                .ok_or("Missing USERPROFILE/HOME; pass --db PATH")?;
            PathBuf::from(home)
                .join(".exocortex")
                .join("memory.sqlite3")
        }
        Some(flag) if flag == "--db" => PathBuf::from(args.next().ok_or("--db requires a path")?),
        _ => return Err("Usage: exo-api [--db PATH]".into()),
    };
    if args.next().is_some() {
        return Err("Usage: exo-api [--db PATH]".into());
    }
    if let Some(parent) = db.parent().filter(|p| !p.as_os_str().is_empty()) {
        std::fs::create_dir_all(parent)?;
    }
    PersistentMemoryStore::open(&db)?;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:8787").await?;
    println!("EXOCORTEX API ready at http://127.0.0.1:8787 (loopback only)");
    println!("Using existing cognitive database: {}", db.display());
    axum::serve(
        listener,
        router(AppState {
            db_path: Arc::new(db),
        }),
    )
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn saved_memory_is_available_after_new_request_and_reopen() {
        let temp = tempfile::tempdir().unwrap();
        let db = Arc::new(temp.path().join("test.sqlite3"));
        let app = router(AppState {
            db_path: Arc::clone(&db),
        });
        let create = Request::builder()
            .method("POST")
            .uri("/api/memories")
            .header("content-type", "application/json")
            .header("x-exocortex-client", "local-ui-v1")
            .body(Body::from(
                r#"{"text":"Persistent cognitive test","kind":"Semantic"}"#,
            ))
            .unwrap();
        let response = app.clone().oneshot(create).await.unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        drop(app);
        let reopened = router(AppState {
            db_path: Arc::clone(&db),
        });
        let search = Request::builder()
            .uri("/api/memories?q=Persistent")
            .header("x-exocortex-client", "local-ui-v1")
            .body(Body::empty())
            .unwrap();
        let response = reopened.oneshot(search).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let payload = to_bytes(response.into_body(), 1024 * 32).await.unwrap();
        assert!(String::from_utf8_lossy(&payload).contains("Persistent cognitive test"));
    }

    #[tokio::test]
    async fn mutation_without_client_header_is_blocked() {
        let temp = tempfile::tempdir().unwrap();
        let app = router(AppState {
            db_path: Arc::new(temp.path().join("test.sqlite3")),
        });
        let request = Request::builder()
            .method("POST")
            .uri("/api/memories")
            .header("content-type", "application/json")
            .body(Body::from(r#"{"text":"should not be stored"}"#))
            .unwrap();
        assert_eq!(
            app.oneshot(request).await.unwrap().status(),
            StatusCode::BAD_REQUEST
        );
    }
}
