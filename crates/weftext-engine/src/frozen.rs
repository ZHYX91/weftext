use crate::{Engine, EngineError, RenderOutput, StyleModule, world::LocalWorld};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

/// Explicit immutable input admitted by Core. No missing dependency is filled
/// from disk, packages, network, or a later source observation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrozenRenderRequest {
    pub source: String,
    pub path: PathBuf,
    pub root: PathBuf,
    pub files: BTreeMap<String, Vec<u8>>,
    pub style: Option<StyleModule>,
}

impl Engine {
    pub fn render_frozen(&self, input: &FrozenRenderRequest) -> Result<RenderOutput, EngineError> {
        self.render_frozen_compilation(input)
            .map(crate::render::RenderCompilation::into_output)
    }
    pub(crate) fn render_frozen_compilation(
        &self,
        input: &FrozenRenderRequest,
    ) -> Result<crate::render::RenderCompilation, EngineError> {
        let world = LocalWorld::frozen(&self.library, &self.fonts, input)?;
        let map = match &input.style {
            Some(style) => crate::inspect_editor_with_style(&input.source, style),
            None => crate::inspect_editor(&input.source),
        };
        self.render_world(&input.source, world, map)
    }
}
