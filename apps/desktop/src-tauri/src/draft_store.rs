use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tempfile::Builder;
use weftext_core::{DocumentRevision, NodeId};

const DRAFT_SCHEMA: &str = "weftext.desktop-draft.v2";
const DRAFT_DIRECTORY: &str = "drafts";
const MAX_DRAFT_SOURCE_BYTES: usize = 64 * 1024 * 1024;

/// Device-recovery provenance is deliberately separate from Core's managed
/// document profile.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum StoredDraftProfile {
    AsciiDocV1,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StoredDraft {
    schema: String,
    pub(crate) workspace_id: NodeId,
    pub(crate) node_id: NodeId,
    pub(crate) base_revision: DocumentRevision,
    pub(crate) document_profile: StoredDraftProfile,
    pub(crate) source: String,
    pub(crate) updated_at_unix_ms: u64,
}

#[derive(Debug, Default)]
pub(crate) struct DraftInventory {
    pub(crate) drafts: Vec<StoredDraft>,
    pub(crate) issues: Vec<String>,
}

pub(crate) struct DraftStore {
    root: PathBuf,
}

impl DraftStore {
    pub(crate) fn new(config_dir: &Path) -> Self {
        Self {
            root: config_dir.join(DRAFT_DIRECTORY),
        }
    }

    pub(crate) fn save(
        &self,
        workspace_path: &Path,
        workspace_id: NodeId,
        node_id: NodeId,
        base_revision: DocumentRevision,
        source: String,
    ) -> Result<StoredDraft, String> {
        if source.len() > MAX_DRAFT_SOURCE_BYTES {
            return Err("草稿超过当前 64 MiB 的恢复上限".to_owned());
        }
        let draft = StoredDraft {
            schema: DRAFT_SCHEMA.to_owned(),
            workspace_id,
            node_id,
            base_revision,
            document_profile: StoredDraftProfile::AsciiDocV1,
            source,
            updated_at_unix_ms: updated_at_unix_ms()?,
        };
        let scope = self.scope_directory(workspace_path, workspace_id);
        fs::create_dir_all(&scope).map_err(|error| format!("无法创建草稿恢复目录：{error}"))?;
        let bytes =
            serde_json::to_vec(&draft).map_err(|error| format!("无法编码恢复草稿：{error}"))?;
        let mut staged = Builder::new()
            .prefix(".weftext-draft-")
            .tempfile_in(&scope)
            .map_err(|error| format!("无法暂存恢复草稿：{error}"))?;
        staged
            .write_all(&bytes)
            .and_then(|()| staged.flush())
            .and_then(|()| staged.as_file().sync_all())
            .map_err(|error| format!("无法写入恢复草稿：{error}"))?;
        let target = scope.join(draft_filename(node_id));
        staged
            .persist(&target)
            .map_err(|error| format!("无法提交恢复草稿：{}", error.error))?;
        let verified = self
            .load(workspace_path, workspace_id, node_id)?
            .ok_or_else(|| "恢复草稿提交后不可见".to_owned())?;
        if verified != draft {
            return Err("恢复草稿提交后校验不一致".to_owned());
        }
        let _ = File::open(&scope).and_then(|directory| directory.sync_all());
        Ok(draft)
    }

    pub(crate) fn load(
        &self,
        workspace_path: &Path,
        workspace_id: NodeId,
        node_id: NodeId,
    ) -> Result<Option<StoredDraft>, String> {
        let path = self
            .scope_directory(workspace_path, workspace_id)
            .join(draft_filename(node_id));
        if !path.exists() {
            return Ok(None);
        }
        read_draft(&path, workspace_id, Some(node_id)).map(Some)
    }

    pub(crate) fn list(
        &self,
        workspace_path: &Path,
        workspace_id: NodeId,
    ) -> Result<DraftInventory, String> {
        let scope = self.scope_directory(workspace_path, workspace_id);
        if !scope.exists() {
            return Ok(DraftInventory::default());
        }
        let mut entries = fs::read_dir(&scope)
            .map_err(|error| format!("无法读取草稿恢复目录：{error}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("无法枚举草稿恢复目录：{error}"))?;
        entries.sort_by_key(fs::DirEntry::file_name);
        let mut inventory = DraftInventory::default();
        for entry in entries {
            let name = entry.file_name().to_string_lossy().into_owned();
            let file_type = match entry.file_type() {
                Ok(value) => value,
                Err(error) => {
                    inventory
                        .issues
                        .push(format!("{name}：无法检查类型：{error}"));
                    continue;
                }
            };
            let is_json = Path::new(&name)
                .extension()
                .is_some_and(|extension| extension.eq_ignore_ascii_case("json"));
            if !file_type.is_file() || !is_json {
                inventory.issues.push(format!("{name}：不是有效的草稿记录"));
                continue;
            }
            match read_draft(&entry.path(), workspace_id, None) {
                Ok(draft) => inventory.drafts.push(draft),
                Err(error) => inventory.issues.push(format!("{name}：{error}")),
            }
        }
        inventory.drafts.sort_by_key(|draft| draft.node_id);
        Ok(inventory)
    }

    pub(crate) fn discard(
        &self,
        workspace_path: &Path,
        workspace_id: NodeId,
        node_id: NodeId,
    ) -> Result<bool, String> {
        let scope = self.scope_directory(workspace_path, workspace_id);
        let path = scope.join(draft_filename(node_id));
        match fs::remove_file(&path) {
            Ok(()) => {
                let _ = File::open(&scope).and_then(|directory| directory.sync_all());
                Ok(true)
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
            Err(error) => Err(format!("无法放弃恢复草稿：{error}")),
        }
    }

    fn scope_directory(&self, workspace_path: &Path, workspace_id: NodeId) -> PathBuf {
        let mut digest = Sha256::new();
        digest.update(workspace_id.to_string().as_bytes());
        digest.update([0]);
        digest.update(workspace_path.to_string_lossy().as_bytes());
        self.root.join(format!("{:x}", digest.finalize()))
    }
}

fn read_draft(
    path: &Path,
    workspace_id: NodeId,
    expected_node_id: Option<NodeId>,
) -> Result<StoredDraft, String> {
    let metadata = fs::metadata(path).map_err(|error| format!("无法读取草稿元数据：{error}"))?;
    if metadata.len() > u64::try_from(MAX_DRAFT_SOURCE_BYTES).unwrap_or(u64::MAX) + 64 * 1024 {
        return Err("草稿记录超过恢复上限".to_owned());
    }
    let bytes = fs::read(path).map_err(|error| format!("无法读取草稿：{error}"))?;
    let schema = serde_json::from_slice::<serde_json::Value>(&bytes)
        .map_err(|error| format!("草稿记录格式无效：{error}"))?
        .get("schema")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "草稿记录缺少版本".to_owned())?
        .to_owned();
    if schema != DRAFT_SCHEMA {
        return Err("草稿记录版本不受支持".to_owned());
    }
    let draft = serde_json::from_slice::<StoredDraft>(&bytes)
        .map_err(|error| format!("草稿记录格式无效：{error}"))?;
    if draft.workspace_id != workspace_id {
        return Err("草稿记录的工作区身份不匹配".to_owned());
    }
    if expected_node_id.is_some_and(|node_id| draft.node_id != node_id) {
        return Err("草稿记录的节点身份不匹配".to_owned());
    }
    if draft.source.len() > MAX_DRAFT_SOURCE_BYTES {
        return Err("草稿内容超过恢复上限".to_owned());
    }
    Ok(draft)
}

fn draft_filename(node_id: NodeId) -> String {
    format!("{node_id}.json")
}

fn updated_at_unix_ms() -> Result<u64, String> {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| "系统时钟早于 Unix epoch，无法记录草稿时间".to_owned())?;
    u64::try_from(elapsed.as_millis()).map_err(|_| "草稿时间超出支持范围".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn persists_and_discards_a_scoped_draft() {
        let temporary = tempdir().expect("tempdir");
        let workspace = temporary.path().join("Workspace");
        let store = DraftStore::new(&temporary.path().join("config"));
        let workspace_id = NodeId::new_v4();
        let node_id = NodeId::new_v4();
        let revision = DocumentRevision::from_source("before");

        let saved = store
            .save(
                &workspace,
                workspace_id,
                node_id,
                revision.clone(),
                "draft".to_owned(),
            )
            .expect("save");
        assert_eq!(saved.base_revision, revision);
        assert_eq!(saved.document_profile, StoredDraftProfile::AsciiDocV1);
        assert_eq!(
            store
                .load(&workspace, workspace_id, node_id)
                .expect("load")
                .expect("draft")
                .source,
            "draft"
        );
        assert_eq!(
            store
                .list(&workspace, workspace_id)
                .expect("list")
                .drafts
                .len(),
            1
        );
        store
            .save(
                &workspace,
                workspace_id,
                node_id,
                revision,
                "newer draft".to_owned(),
            )
            .expect("replace");
        assert_eq!(
            store
                .load(&workspace, workspace_id, node_id)
                .expect("load replacement")
                .expect("replacement")
                .source,
            "newer draft"
        );
        assert!(store
            .discard(&workspace, workspace_id, node_id)
            .expect("discard"));
        assert!(store
            .load(&workspace, workspace_id, node_id)
            .expect("load")
            .is_none());
    }

    #[test]
    fn interrupted_staging_is_reported_without_hiding_the_last_good_draft() {
        let temporary = tempdir().expect("tempdir");
        let workspace = temporary.path().join("Workspace");
        let store = DraftStore::new(&temporary.path().join("config"));
        let workspace_id = NodeId::new_v4();
        let node_id = NodeId::new_v4();
        store
            .save(
                &workspace,
                workspace_id,
                node_id,
                DocumentRevision::from_source("before"),
                "last good draft".to_owned(),
            )
            .expect("save");
        let scope = store.scope_directory(&workspace, workspace_id);
        fs::write(scope.join(".weftext-draft-interrupted"), b"partial").expect("interrupted stage");

        let inventory = store.list(&workspace, workspace_id).expect("list");
        assert_eq!(inventory.drafts.len(), 1);
        assert_eq!(inventory.drafts[0].source, "last good draft");
        assert_eq!(inventory.issues.len(), 1);
        assert!(inventory.issues[0].contains("不是有效的草稿记录"));
    }

    #[test]
    fn retired_draft_schema_is_rejected() {
        let temporary = tempdir().expect("tempdir");
        let workspace = temporary.path().join("Workspace");
        let store = DraftStore::new(&temporary.path().join("config"));
        let workspace_id = NodeId::new_v4();
        let node_id = NodeId::new_v4();
        let scope = store.scope_directory(&workspace, workspace_id);
        fs::create_dir_all(&scope).expect("scope");
        fs::write(
            scope.join(draft_filename(node_id)),
            serde_json::to_vec(&serde_json::json!({
                "schema": "weftext.desktop-draft.v1",
                "workspaceId": workspace_id,
                "nodeId": node_id,
                "baseRevision": DocumentRevision::from_source("before"),
                "source": "retired draft bytes\r\n",
                "updatedAtUnixMs": 42,
            }))
            .expect("legacy json"),
        )
        .expect("retired draft");

        assert!(store.load(&workspace, workspace_id, node_id).is_err());
    }
}
