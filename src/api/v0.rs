use crate::api::AppState;
use axum::Router;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi()]
// #[openapi(paths(demo_for_text), components(schemas(DemoResponse)))]
pub struct ApiDoc;

pub fn app() -> Router<AppState> {
    Router::new()
}
