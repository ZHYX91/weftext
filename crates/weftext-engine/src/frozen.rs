use crate::{
    Dependency, Diagnostic, Engine, EngineError, ErrorDescription, RenderOutput, RenderProducts,
    StyleModule,
    world::{FrozenFiles, LocalWorld},
};
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FrozenQuerySource {
    pub path: PathBuf,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FrozenQueryProduct {
    pub products: Option<RenderProducts>,
    pub dependencies: Vec<Dependency>,
    pub diagnostics: Vec<Diagnostic>,
    pub compiler_version: String,
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

    /// Compile multiple main sources in independent worlds over one qualified
    /// file set. Return only Query products and their original evidence.
    pub fn frozen_query_products(
        &self,
        root: &std::path::Path,
        files: &BTreeMap<String, Vec<u8>>,
        style: Option<&StyleModule>,
        notes: &[FrozenQuerySource],
    ) -> Result<Vec<Result<FrozenQueryProduct, ErrorDescription>>, EngineError> {
        let files = FrozenFiles::new(files)?;
        Ok(notes
            .iter()
            .map(|note| {
                let render = || -> Result<_, EngineError> {
                    let world = LocalWorld::with_frozen_files(
                        &self.library,
                        &self.fonts,
                        &note.source,
                        &note.path,
                        root,
                        style,
                        &files,
                    )?;
                    let map = match style {
                        Some(style) => crate::inspect_editor_with_style(&note.source, style),
                        None => crate::inspect_editor(&note.source),
                    };
                    let output = self.render_world(&note.source, world, map)?.into_output();
                    Ok(FrozenQueryProduct {
                        products: output.ok.then_some(output.products).flatten(),
                        dependencies: output.dependencies,
                        diagnostics: output.diagnostics,
                        compiler_version: output.compiler_version,
                    })
                };
                render().map_err(|error| error.description())
            })
            .collect())
    }
}
