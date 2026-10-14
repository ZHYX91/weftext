use crate::{
    Dependency, Diagnostic, Engine, EngineError, StyleModule, render::diagnostic, world::LocalWorld,
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde::{Deserialize, Serialize};
use std::path::Path;
use typst_layout::PagedDocument;
use typst_pdf::PdfOptions;

const PDF_LIMIT: usize = 16 * 1024 * 1024;
const PAGE_LIMIT: usize = 1000;

/// The official Paged target's output. It has no author-source edit addresses;
/// the host binds delivery to the exact Draft and compilation environment.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PagedOutput {
    pub ok: bool,
    pub pdf_base64: Option<String>,
    pub page_count: usize,
    pub diagnostics: Vec<Diagnostic>,
    pub dependencies: Vec<Dependency>,
    pub font_families: Vec<String>,
    pub compiler_version: String,
    pub target: String,
}

impl Engine {
    pub fn render_paged(
        &self,
        source: &str,
        path: &Path,
        root: &Path,
    ) -> Result<PagedOutput, EngineError> {
        let world = LocalWorld::new(&self.library, &self.fonts, source, path, root)?;
        self.paged_world(&world)
    }

    pub fn render_paged_with_style(
        &self,
        source: &str,
        path: &Path,
        root: &Path,
        style: &StyleModule,
    ) -> Result<PagedOutput, EngineError> {
        let world = LocalWorld::with_style(&self.library, &self.fonts, source, path, root, style)?;
        self.paged_world(&world)
    }

    fn paged_world(&self, world: &LocalWorld<'_>) -> Result<PagedOutput, EngineError> {
        // This is a separate native target, not a reconstruction from HTML or
        // a browser print of Flow. The World and style entry are shared.
        let compiled = typst::compile::<PagedDocument>(world);
        let mut diagnostics: Vec<_> = compiled
            .warnings
            .iter()
            .map(|d| diagnostic(d, world))
            .collect();
        let mut pdf_base64 = None;
        let mut page_count = 0;
        match compiled.output {
            Ok(document) => {
                page_count = document.pages().len();
                if page_count > PAGE_LIMIT {
                    diagnostics.push(budget_diagnostic("分页超过 1,000 页输出上限"));
                } else {
                    // Default export keeps the native document metadata,
                    // embedded fonts, links and tagged PDF. No invented stable
                    // document ID or wall-clock timestamp is supplied.
                    match typst_pdf::pdf(&document, &PdfOptions::default()) {
                        Ok(bytes) if bytes.len() <= PDF_LIMIT => {
                            pdf_base64 = Some(STANDARD.encode(bytes));
                        }
                        Ok(_) => diagnostics.push(budget_diagnostic("PDF 超过 16 MiB 输出上限")),
                        Err(errors) => {
                            diagnostics.extend(errors.iter().map(|d| diagnostic(d, world)));
                        }
                    }
                }
            }
            Err(errors) => {
                diagnostics.extend(errors.iter().map(|d| diagnostic(d, world)));
            }
        }
        Ok(PagedOutput {
            ok: pdf_base64.is_some(),
            pdf_base64,
            page_count,
            diagnostics,
            dependencies: world.dependencies(),
            font_families: self.families.clone(),
            compiler_version: "0.15.1".into(),
            target: "paged".into(),
        })
    }
}

fn budget_diagnostic(message: &str) -> Diagnostic {
    Diagnostic {
        severity: "error".into(),
        message: message.into(),
        origin: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, sync::OnceLock};
    use tempfile::TempDir;

    fn engine() -> &'static Engine {
        static ENGINE: OnceLock<Engine> = OnceLock::new();
        ENGINE.get_or_init(|| Engine::new().unwrap())
    }

    #[test]
    fn native_paged_target_preserves_pagebreak_and_returns_pdf() {
        let root = TempDir::new().unwrap();
        let source = "#context if target() == \"paged\" [第一页#pagebreak()第二页] else [连续内容]";
        let output = engine()
            .render_paged(source, &root.path().join("note.typ"), root.path())
            .unwrap();
        assert!(output.ok, "{:?}", output.diagnostics);
        assert_eq!(output.target, "paged");
        assert_eq!(output.compiler_version, "0.15.1");
        assert_eq!(output.page_count, 2);
        let pdf = STANDARD.decode(output.pdf_base64.unwrap()).unwrap();
        assert!(pdf.starts_with(b"%PDF-"));
        assert!(pdf.ends_with(b"%%EOF"));
    }

    #[test]
    fn invalid_draft_has_diagnostics_and_no_pdf() {
        let root = TempDir::new().unwrap();
        let source = "中文\n#let incomplete = (";
        let output = engine()
            .render_paged(source, &root.path().join("note.typ"), root.path())
            .unwrap();
        assert!(!output.ok);
        assert!(output.pdf_base64.is_none());
        assert_eq!(output.page_count, 0);
        assert!(output.diagnostics.iter().any(|d| {
            d.severity == "error"
                && d.origin.as_ref().is_some_and(|origin| {
                    origin.is_main && source.get(origin.from..origin.to) == Some(&origin.raw)
                })
        }));
    }

    #[test]
    fn paged_uses_exact_draft_and_source_relative_authorized_dependencies() {
        let root = TempDir::new().unwrap();
        fs::create_dir(root.path().join("notes")).unwrap();
        fs::write(root.path().join("notes/part.typ"), "依赖内容").unwrap();
        fs::write(root.path().join("notes/note.typ"), "磁盘旧内容").unwrap();
        let output = engine()
            .render_paged(
                "#include \"part.typ\"\n#pagebreak()当前草稿",
                &root.path().join("notes/note.typ"),
                root.path(),
            )
            .unwrap();
        assert!(output.ok, "{:?}", output.diagnostics);
        assert_eq!(output.page_count, 2);
        assert!(
            output
                .dependencies
                .iter()
                .any(|dep| { dep.path == "notes/part.typ" && dep.status == "loaded" })
        );
        let denied = engine()
            .render_paged(
                "#read(\"../../outside.txt\")",
                &root.path().join("notes/note.typ"),
                root.path(),
            )
            .unwrap();
        assert!(!denied.ok);
        assert!(denied.pdf_base64.is_none());
    }

    #[test]
    fn native_library_style_is_shared_without_changing_note_bytes() {
        let root = TempDir::new().unwrap();
        fs::create_dir(root.path().join("notes")).unwrap();
        fs::create_dir(root.path().join("styles")).unwrap();
        fs::write(root.path().join("styles/decoration.txt"), "库页眉").unwrap();
        fs::write(root.path().join("notes/body.txt"), "笔记相对依赖").unwrap();
        let source = "#read(\"body.txt\")";
        let path = root.path().join("notes/note.typ");
        let style = StyleModule {
            path: "styles/library-style.typ".into(),
            source:
                "#let library-style(body) = { set page(header: read(\"decoration.txt\")); body }"
                    .into(),
            enabled: true,
            entry_function: "library-style".into(),
        };
        let plain = engine().render_paged(source, &path, root.path()).unwrap();
        let styled = engine()
            .render_paged_with_style(source, &path, root.path(), &style)
            .unwrap();
        assert!(styled.ok, "{:?}", styled.diagnostics);
        assert_ne!(plain.pdf_base64, styled.pdf_base64);
        assert!(
            styled
                .dependencies
                .iter()
                .any(|dep| dep.path == "styles/decoration.txt")
        );
        assert!(
            styled
                .dependencies
                .iter()
                .any(|dep| dep.path == "notes/body.txt")
        );
        assert!(
            !path.exists(),
            "derived entry must never create or rewrite a note"
        );
    }
}
