use super::*;
use base64::Engine as _;
use std::collections::BTreeMap;
use weftext_engine::{FrozenQueryProduct, FrozenQuerySource, StyleModule};

pub(super) const BATCH_NOTES: usize = 96;
const MAX_FILES_BYTES: usize = 64 * 1024 * 1024;

pub(super) fn build_identity() -> &'static str {
    #[cfg(windows)]
    if let Ok(catalog) = crate::artifact_catalog::Catalog::get() {
        return catalog.build_id();
    }
    env!("CARGO_PKG_VERSION")
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct BatchRequest<Files = BTreeMap<String, String>> {
    root: PathBuf,
    style: Option<StyleModule>,
    frozen_files: Files,
    query_sources: Vec<FrozenQuerySource>,
}

type Product = std::result::Result<FrozenQueryProduct, ErrorDescription>;

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(super) struct BatchOutput {
    query_products: Vec<Product>,
}

impl BatchRequest {
    pub(super) fn render(self) -> Result<BatchOutput, Box<dyn std::error::Error>> {
        if self.query_sources.is_empty()
            || self.query_sources.len() > BATCH_NOTES
            || self
                .query_sources
                .iter()
                .any(|note| note.source.len() > weftext_core::MAX_SOURCE_BYTES)
        {
            return Err(
                ErrorDescription::new(UiMessage::error_api_worker_render_input_budget()).into(),
            );
        }
        let mut size = 0;
        let mut files = BTreeMap::new();
        for (name, value) in self.frozen_files {
            let bytes = base64::engine::general_purpose::STANDARD.decode(value)?;
            if bytes.len() > MAX_FILES_BYTES - size {
                return Err(ErrorDescription::new(
                    UiMessage::error_api_worker_render_frozen_files_budget(),
                )
                .into());
            }
            size += bytes.len();
            files.insert(name, bytes);
        }
        let engine = weftext_engine::Engine::new_with_system_fonts(false)?;
        Ok(BatchOutput {
            query_products: engine.frozen_query_products(
                &self.root,
                &files,
                self.style.as_ref(),
                &self.query_sources,
            )?,
        })
    }
}

pub(super) fn encode_files(files: &BTreeMap<String, Vec<u8>>) -> Arc<BTreeMap<String, String>> {
    Arc::new(
        files
            .iter()
            .map(|(path, bytes)| {
                (
                    path.clone(),
                    base64::engine::general_purpose::STANDARD.encode(bytes),
                )
            })
            .collect(),
    )
}

pub(super) async fn run(
    state: &AppState,
    root: PathBuf,
    style: Option<StyleModule>,
    frozen_files: Arc<BTreeMap<String, String>>,
    query_sources: Vec<FrozenQuerySource>,
) -> Result<Vec<Product>, ApiError> {
    let length = query_sources.len();
    let permit = tokio::select! {
        biased;
        () = crate::query_runs::cancellation() => return Err(crate::query_runs::cancelled()),
        permit = state.render_gate.clone().acquire_owned() => permit
            .map_err(|_| ApiError::internal(UiMessage::error_compile_service_unavailable()))?,
    };
    let payload = value(BatchRequest {
        root,
        style,
        frozen_files: frozen_files.as_ref(),
        query_sources,
    })?;
    // Keep the render permit until the owned process has finished, including
    // when the HTTP caller gives up waiting for this batch.
    let output = crate::work::spawn(async move {
        let _permit = permit;
        owned_compiler_worker(payload, false, compiler_worker::Transport::default()).await
    })
    .await
    .map_err(|_| ApiError::internal(UiMessage::error_compile_service_unavailable()))??;
    let output: BatchOutput = serde_json::from_value(output)
        .map_err(|_| ApiError::internal(UiMessage::error_compiler_output_invalid()))?;
    if output.query_products.len() != length {
        return Err(ApiError::internal(
            UiMessage::error_compiler_output_invalid(),
        ));
    }
    Ok(output.query_products)
}
