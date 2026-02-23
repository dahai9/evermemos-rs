use axum::{
    extract::Request,
    http::{header, HeaderMap, StatusCode},
    middleware::Next,
    response::Response,
};

use crate::core::tenant::TenantContext;

/// Axum middleware that extracts tenant context from request headers
/// (`X-Organization-Id` + `X-Space-Id`) and injects it as an Extension.
pub async fn tenant_middleware(
    org_header: &str,
    space_header: &str,
    headers: &HeaderMap,
    request: Request,
    next: Next,
) -> Response {
    let org_id = headers
        .get(org_header)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("default")
        .to_string();

    let space_id = headers
        .get(space_header)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let tenant = TenantContext::new(org_id, space_id);
    let mut req = request;
    req.extensions_mut().insert(tenant);
    next.run(req).await
}

/// Simple Bearer token auth middleware.
/// Pass `api_key = "none"` to disable.
pub async fn auth_middleware(
    api_key: String,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if api_key == "none" || api_key.is_empty() {
        return Ok(next.run(request).await);
    }

    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    match token {
        Some(t) if t == api_key => Ok(next.run(request).await),
        _ => Err(StatusCode::UNAUTHORIZED),
    }
}
