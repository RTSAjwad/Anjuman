//! Health-check DTOs.

use serde::Serialize;

/// Response for `GET /health`.
#[derive(Debug, Clone, Serialize)]
#[cfg_attr(feature = "openapi", derive(utoipa::ToSchema))]
pub struct Health {
    pub status: &'static str,
}
