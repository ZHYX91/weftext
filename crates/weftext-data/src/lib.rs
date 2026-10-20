//! Pure structured-data and Query transformations. Persistence and authorization
//! belong to Core; this crate accepts frozen bytes and never opens files.
mod embedded_query;
mod native;
mod projection;
pub use embedded_query::*;
mod query;
mod query_native_products;
pub use query_native_products::native_query_products;
mod query_providers;
pub use query_providers::*;
pub mod query_advanced;
pub use query_advanced::*;
mod recurrence;
mod relations;
mod schema;
mod tag_forest;
mod temporal;
pub use tag_forest::*;
mod tag_remap;
pub use native::*;
pub use projection::*;
pub use query::*;
pub use recurrence::*;
pub use relations::*;
pub use schema::*;
pub use tag_remap::*;
pub use temporal::*;

use std::fmt;
#[derive(Debug, Clone)]
pub struct DataError(pub String);
impl fmt::Display for DataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for DataError {}
impl From<serde_json::Error> for DataError {
    fn from(e: serde_json::Error) -> Self {
        Self(e.to_string())
    }
}
pub type Result<T> = std::result::Result<T, DataError>;
