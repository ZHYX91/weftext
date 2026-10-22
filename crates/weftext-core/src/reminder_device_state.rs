//! Device-local task-reminder decisions. Saved author data remains the only
//! task authority; a notification is a distinct external effect, never a save.
use super::*;
use crate::device_state::DeviceRecordKind;
use crate::operations::{SnapshotBinding, read_blob};
use serde::de::{MapAccess, Visitor};
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt,
    marker::PhantomData,
    time::{SystemTime, UNIX_EPOCH},
};
use weftext_data::{
    Cell, ExactDecimal, ExactInstant, Schema, TaskReminderResolution, TaskReminderState,
};
use weftext_knowledge::NodeRef;

const MAX_BYTES: usize = 16 * 1024 * 1024;
const MAX_RECORDS: usize = 32_768;
const MAX_LIBRARIES: usize = 128;
const MAX_SUMMARY_MEMBERS: usize = 64;
const GROUP: &str = "weftext.reminders.v1";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct LibrarySelection {
    root: String,
    identity: FileIdentity,
    enabled: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReminderSubscription {
    pub owner: NodeRef,
    pub reminder_id: String,
    pub enabled: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReminderNotificationRequest {
    pub app_id: String,
    pub tag: String,
    pub group: String,
    pub activation_key: String,
    /// Fixed generic UI copy selected by the trusted Desktop adapter, never a
    /// task title/body or client-supplied author field. Persisted before Show.
    pub title: String,
    pub body: String,
}
#[derive(Debug, Clone)]
pub struct ReminderNotificationOptions {
    pub app_id: String,
    pub title: String,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "state", rename_all = "camelCase", deny_unknown_fields)]
pub enum ReminderDeliveryState {
    Submitting,
    SubmittedToSystem { at: String },
    FailedBeforeCall { diagnostic: String },
    Unknown { diagnostic: String },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Attempt {
    permit_id: String,
    attempt_at: String,
    expires_at: String,
    attempt_count: u32,
    request: ReminderNotificationRequest,
    delivery: ReminderDeliveryState,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
enum Delivery {
    Individual {
        attempt: Attempt,
    },
    SummaryMember {
        summary_key: String,
        prior_failed_attempt: Option<Attempt>,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Record {
    owner: NodeRef,
    reminder_id: String,
    definition_digest: String,
    trigger_instant: String,
    delivery: Delivery,
    handled: bool,
    invalidated: Option<ReminderCancellation>,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct Summary {
    members: Vec<String>,
    attempt: Attempt,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct SettingsMutation {
    id: String,
    fingerprint: String,
    revision: u64,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct State {
    schema_version: u32,
    device_id: String,
    revision: u64,
    settings_revision: u64,
    #[serde(deserialize_with = "unique_map")]
    libraries: BTreeMap<String, LibrarySelection>,
    #[serde(deserialize_with = "unique_map")]
    subscriptions: BTreeMap<String, ReminderSubscription>,
    #[serde(deserialize_with = "unique_map")]
    records: BTreeMap<String, Record>,
    #[serde(deserialize_with = "unique_map")]
    summaries: BTreeMap<String, Summary>,
    last_settings_mutation: Option<SettingsMutation>,
}
fn unique_map<'de, D, V>(deserializer: D) -> std::result::Result<BTreeMap<String, V>, D::Error>
where
    D: serde::Deserializer<'de>,
    V: Deserialize<'de>,
{
    struct Unique<V>(PhantomData<V>);
    impl<'de, V: Deserialize<'de>> Visitor<'de> for Unique<V> {
        type Value = BTreeMap<String, V>;
        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("a map without duplicate keys")
        }
        fn visit_map<M: MapAccess<'de>>(
            self,
            mut map: M,
        ) -> std::result::Result<Self::Value, M::Error> {
            let mut out = BTreeMap::new();
            while let Some((key, value)) = map.next_entry::<String, V>()? {
                if out.insert(key, value).is_some() {
                    return Err(serde::de::Error::custom("Duplicate reminder key"));
                }
            }
            Ok(out)
        }
    }
    deserializer.deserialize_map(Unique(PhantomData))
}
fn err(message: impl Into<String>) -> CoreError {
    CoreError::new("reminderState", message)
}
fn uuid(value: &str) -> bool {
    weftext_data::uuid_v4(value)
}
fn digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn owner_valid(owner: &NodeRef) -> bool {
    uuid(&owner.workspace_id) && uuid(&owner.node_id)
}
fn subscription_key(owner: &NodeRef, id: &str) -> String {
    format!("{}/{}/{id}", owner.workspace_id, owner.node_id)
}
fn execution_key(
    device: &str,
    owner: &NodeRef,
    id: &str,
    definition: &str,
    trigger: &str,
) -> Result<String> {
    Ok(hash(&serde_json::to_vec(&(
        "weftext.reminder-effect.v1",
        device,
        owner,
        id,
        definition,
        trigger,
    ))?))
}
fn now() -> Result<ExactInstant> {
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|_| err("System clock is before the supported epoch"))?;
    let utc_seconds = i64::try_from(elapsed.as_secs()).map_err(|_| err("System clock overflow"))?;
    Ok(ExactInstant {
        utc_seconds,
        fraction: ExactDecimal::parse(&format!("0.{:09}", elapsed.subsec_nanos()))
            .map_err(|e| err(e.to_string()))?,
    })
}
fn validate_attempt(attempt: &Attempt, key: &str) -> Result<()> {
    if !uuid(&attempt.permit_id)
        || !uuid(&attempt.request.activation_key)
        || attempt.attempt_count == 0
        || attempt.request.tag != key
        || attempt.request.group != GROUP
        || attempt.request.app_id.is_empty()
        || attempt.request.app_id.len() > 128
        || attempt.request.app_id.chars().any(char::is_whitespace)
        || attempt.request.title.len() > 1024
        || attempt.request.body.len() > 2048
    {
        return Err(err("Invalid original notification request"));
    }
    for instant in [&attempt.attempt_at, &attempt.expires_at] {
        ExactInstant::parse(instant).map_err(|e| err(e.to_string()))?;
    }
    match &attempt.delivery {
        ReminderDeliveryState::SubmittedToSystem { at } => {
            ExactInstant::parse(at).map_err(|e| err(e.to_string()))?;
        }
        ReminderDeliveryState::FailedBeforeCall { diagnostic }
        | ReminderDeliveryState::Unknown { diagnostic }
            if diagnostic.len() > 1024 =>
        {
            return Err(err("Notification diagnostic exceeds budget"));
        }
        _ => {}
    }
    Ok(())
}
impl State {
    fn new(device: String) -> Self {
        Self {
            schema_version: 1,
            device_id: device,
            revision: 0,
            settings_revision: 0,
            libraries: BTreeMap::new(),
            subscriptions: BTreeMap::new(),
            records: BTreeMap::new(),
            summaries: BTreeMap::new(),
            last_settings_mutation: None,
        }
    }
    fn validate(&self) -> Result<()> {
        if self.schema_version != 1
            || !uuid(&self.device_id)
            || self.settings_revision > self.revision
            || self.libraries.len() > MAX_LIBRARIES
            || self.subscriptions.len() > MAX_RECORDS
            || self.records.len() + self.summaries.len() > MAX_RECORDS
        {
            return Err(err("Invalid or over-budget reminder state"));
        }
        for (id, library) in &self.libraries {
            if !uuid(id) || !Path::new(&library.root).is_absolute() || library.root.len() > 32_768 {
                return Err(err("Invalid selected reminder library"));
            }
        }
        for (key, subscription) in &self.subscriptions {
            if !owner_valid(&subscription.owner)
                || !uuid(&subscription.reminder_id)
                || *key != subscription_key(&subscription.owner, &subscription.reminder_id)
                || !self
                    .libraries
                    .contains_key(&subscription.owner.workspace_id)
            {
                return Err(err("Invalid reminder subscription"));
            }
        }
        let mut activation_keys = BTreeSet::new();
        for (key, record) in &self.records {
            if !owner_valid(&record.owner)
                || !uuid(&record.reminder_id)
                || !self.libraries.contains_key(&record.owner.workspace_id)
                || !digest(&record.definition_digest)
                || !digest(key)
                || *key
                    != execution_key(
                        &self.device_id,
                        &record.owner,
                        &record.reminder_id,
                        &record.definition_digest,
                        &record.trigger_instant,
                    )?
            {
                return Err(err("Invalid original reminder effect"));
            }
            ExactInstant::parse(&record.trigger_instant).map_err(|e| err(e.to_string()))?;
            match &record.delivery {
                Delivery::Individual { attempt } => {
                    validate_attempt(attempt, key)?;
                    if !activation_keys.insert(&attempt.request.activation_key) {
                        return Err(err("Duplicate notification activation key"));
                    }
                }
                Delivery::SummaryMember {
                    summary_key,
                    prior_failed_attempt,
                } => {
                    if !self
                        .summaries
                        .get(summary_key)
                        .is_some_and(|s| s.members.contains(key))
                    {
                        return Err(err("Missing original reminder summary"));
                    }
                    if let Some(attempt) = prior_failed_attempt {
                        validate_attempt(attempt, key)?;
                        if !matches!(
                            attempt.delivery,
                            ReminderDeliveryState::FailedBeforeCall { .. }
                        ) {
                            return Err(err(
                                "Only a definite pre-call failure may join a later summary",
                            ));
                        }
                    }
                }
            }
            if serde_json::to_vec(record)?.len() > 16 * 1024 {
                return Err(err("Reminder record exceeds budget"));
            }
        }
        for (key, summary) in &self.summaries {
            if summary.members.is_empty()
                || summary.members.len() > MAX_SUMMARY_MEMBERS
                || summary.members.windows(2).any(|v| v[0] >= v[1])
                || *key
                    != hash(&serde_json::to_vec(&(
                        "weftext.reminder-summary.v1",
                        &self.device_id,
                        &summary.members,
                    ))?)
            {
                return Err(err("Invalid original summary membership"));
            }
            for member in &summary.members {
                let linked=self.records.get(member).is_some_and(|record|matches!(&record.delivery,Delivery::SummaryMember{summary_key,..}if summary_key==key));
                if !self.records.contains_key(member)
                    || !linked
                        && !matches!(
                            summary.attempt.delivery,
                            ReminderDeliveryState::FailedBeforeCall { .. }
                        )
                {
                    return Err(err(
                        "Summary member does not point to its original possible external effect",
                    ));
                }
            }
            validate_attempt(&summary.attempt, key)?;
            if !activation_keys.insert(&summary.attempt.request.activation_key) {
                return Err(err("Duplicate summary activation key"));
            }
        }
        if self.last_settings_mutation.as_ref().is_some_and(|m| {
            !uuid(&m.id) || !digest(&m.fingerprint) || m.revision > self.settings_revision
        }) {
            return Err(err("Invalid settings mutation"));
        }
        Ok(())
    }
}
/// Scope-first backup classification calls this on the one known device leaf.
/// Foreign records remain valid originals, but never become current-device consent.
pub(crate) fn validate_reminder_device_record(bytes: &[u8]) -> Result<()> {
    if bytes.len() > MAX_BYTES {
        return Err(err("Reminder state exceeds 16 MiB"));
    }
    let state: State = serde_json::from_slice(bytes)?;
    // FileIdentity is an existing common type. Reject unknown fields in this
    // new format rather than inheriting that type's historical permissiveness.
    let raw: serde_json::Value = serde_json::from_slice(bytes)?;
    for library in raw["libraries"]
        .as_object()
        .ok_or_else(|| err("Missing reminder libraries"))?
        .values()
    {
        let identity = library["identity"]
            .as_object()
            .ok_or_else(|| err("Missing library identity"))?;
        if identity.keys().map(String::as_str).collect::<BTreeSet<_>>()
            != BTreeSet::from(["volume", "index", "created"])
        {
            return Err(err("Unknown library identity members"));
        }
    }
    state.validate()
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskReminderCandidate {
    pub root: String,
    pub owner: NodeRef,
    pub task_open: bool,
    pub resolution: TaskReminderResolution,
    #[serde(skip)]
    root_identity: FileIdentity,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderLibraryView {
    pub workspace_id: String,
    pub root: String,
    pub enabled: bool,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderSettings {
    pub device_id: String,
    pub settings_revision: u64,
    pub libraries: Vec<ReminderLibraryView>,
    pub subscriptions: Vec<ReminderSubscription>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderDeliveryStatus {
    pub execution_key: String,
    pub delivery: ReminderDeliveryState,
    pub handled: bool,
    pub invalidated: Option<ReminderCancellation>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum ReminderCancellation {
    LibraryDisabled,
    SubscriptionDisabled,
    TaskClosed,
    DefinitionChanged,
}
/// Not deserializable or constructible from a HTTP/activation payload.
#[derive(Debug)]
pub struct ReminderDeliveryPermit {
    key: String,
    permit_id: String,
    request: ReminderNotificationRequest,
    expires_at: String,
    issued: std::time::Instant,
    admitted: std::sync::atomic::AtomicBool,
}
impl ReminderDeliveryPermit {
    pub fn execution_key(&self) -> &str {
        &self.key
    }
    pub fn request(&self) -> &ReminderNotificationRequest {
        &self.request
    }
    pub fn expires_at(&self) -> &str {
        &self.expires_at
    }
}
#[derive(Debug)]
pub enum ReminderDeliveryStart {
    Started(ReminderDeliveryPermit),
    Retained(ReminderDeliveryStatus),
}
#[derive(Debug, Clone)]
pub enum ReminderSubmissionOutcome {
    SubmittedToSystem,
    FailedBeforeCall(String),
    Unknown(String),
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReminderScheduleCursor {
    pub settings_revision: u64,
    pub after: String,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderScheduleIssue {
    pub owner: NodeRef,
    pub reminder_id: String,
    pub code: String,
    pub message: String,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderSchedulePage {
    pub candidates: Vec<TaskReminderCandidate>,
    pub issues: Vec<ReminderScheduleIssue>,
    pub complete: bool,
    pub next: Option<ReminderScheduleCursor>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderActivation {
    pub candidates: Vec<TaskReminderCandidate>,
    pub issues: Vec<ReminderScheduleIssue>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderOriginalNotification {
    pub execution_key: String,
    pub request: ReminderNotificationRequest,
    pub delivery: ReminderDeliveryState,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderNotificationEligibility {
    pub original: ReminderOriginalNotification,
    pub eligible: bool,
    pub issues: Vec<ReminderScheduleIssue>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderRecordView {
    pub owner: NodeRef,
    pub reminder_id: String,
    pub trigger_instant: String,
    pub status: ReminderDeliveryStatus,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderRecordPage {
    pub records: Vec<ReminderRecordView>,
    pub next: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ReminderTiming {
    Upcoming,
    Due,
    Missed,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderEligibility {
    pub execution_key: String,
    pub timing: ReminderTiming,
    pub retained: Option<ReminderDeliveryStatus>,
}

impl Core {
    fn read_reminders(&mut self) -> Result<(State, Option<String>)> {
        self.sync_local_owner()?;
        let identity = self.device_identity()?;
        if !identity.is_current {
            return Err(CoreError::new(
                "foreignDeviceState",
                "Reminder state belongs to another machine, user or profile; it is not adopted",
            ));
        }
        match self.read_device_record(DeviceRecordKind::Reminders, MAX_BYTES)? {
            Some((bytes, digest)) => {
                validate_reminder_device_record(&bytes)?;
                let state: State = serde_json::from_slice(&bytes)?;
                if state.device_id != identity.id {
                    return Err(err("Reminder state has a different original device"));
                }
                Ok((state, Some(digest)))
            }
            None => Ok((State::new(identity.id), None)),
        }
    }
    fn write_reminders(&mut self, mut state: State, digest: Option<&str>) -> Result<State> {
        state.revision = state
            .revision
            .checked_add(1)
            .ok_or_else(|| err("Reminder revision overflow"))?;
        let bytes = serde_json::to_vec(&state)?;
        validate_reminder_device_record(&bytes)?;
        self.compare_and_write_device_record(DeviceRecordKind::Reminders, digest, &bytes)?;
        Ok(state)
    }
    pub fn reminder_settings(&mut self) -> Result<ReminderSettings> {
        let (state, _) = self.read_reminders()?;
        Ok(settings(&state))
    }
    pub fn reminder_records_page(&mut self, after: Option<&str>) -> Result<ReminderRecordPage> {
        if after.is_some_and(|key| !digest(key)) {
            return Err(err("Invalid original reminder record cursor"));
        }
        let (state, _) = self.read_reminders()?;
        let mut selected = state
            .records
            .iter()
            .filter(|(key, _)| after.is_none_or(|after| key.as_str() > after));
        let mut records = vec![];
        let mut last = None;
        for (key, record) in selected.by_ref().take(32) {
            last = Some(key.clone());
            records.push(ReminderRecordView {
                owner: record.owner.clone(),
                reminder_id: record.reminder_id.clone(),
                trigger_instant: record.trigger_instant.clone(),
                status: status(&state, key)?,
            });
        }
        let next = if selected.next().is_some() {
            last
        } else {
            None
        };
        Ok(ReminderRecordPage { records, next })
    }
    /// Explicit application-list navigation. This does not authorize an old
    /// notification activation or submit another notification.
    pub fn open_reminder_record(&mut self, key: &str) -> Result<OpenedDocument> {
        let (state, _) = self.read_reminders()?;
        let record = state
            .records
            .get(key)
            .ok_or_else(|| err("No original reminder record"))?;
        let library = state
            .libraries
            .get(&record.owner.workspace_id)
            .ok_or_else(|| err("No original reminder library"))?;
        let root = Path::new(&library.root);
        let root_guard = ParentGuard::acquire(&root.join(".reminder-open-root"))?;
        if root_guard.directory_identity()? != library.identity {
            return Err(CoreError::new(
                "foreignLibraryState",
                "The original reminder library physical root changed",
            ));
        }
        let path = self.reminder_task_source(root, &record.owner)?;
        let document = self.open(&path, Some(root.to_path_buf()))?;
        if document.kind != "managed"
            || document.node_id.as_deref() != Some(record.owner.node_id.as_str())
            || document.workspace_id.as_deref() != Some(record.owner.workspace_id.as_str())
            || weftext_engine::inspect_metadata(&document.source)
                .id
                .as_ref()
                != Some(&record.owner.node_id)
        {
            self.close_session(&document.session_id)?;
            return Err(CoreError::new(
                "identityConflict",
                "The original reminder target changed identity before opening",
            ));
        }
        if root_guard.directory_identity()? != library.identity {
            self.close_session(&document.session_id)?;
            return Err(CoreError::new(
                "foreignLibraryState",
                "The original reminder library changed while opening",
            ));
        }
        Ok(document)
    }
    pub fn reminder_delivery_for_saved_task(
        &mut self,
        expected: &TaskReminderCandidate,
    ) -> Result<Option<ReminderDeliveryStatus>> {
        let (state, _) = self.read_reminders()?;
        let current = self.task_reminder_candidate(Path::new(&expected.root), &expected.owner)?;
        same_candidate(expected, &current)?;
        let resolution = &current.resolution;
        if resolution.state != TaskReminderState::Ready {
            return Ok(None);
        }
        let key = execution_key(
            &state.device_id,
            &current.owner,
            resolution
                .reminder_id
                .as_deref()
                .ok_or_else(|| err("Missing reminder id"))?,
            resolution
                .definition_digest
                .as_deref()
                .ok_or_else(|| err("Missing reminder definition"))?,
            resolution
                .trigger_instant
                .as_deref()
                .ok_or_else(|| err("Missing reminder trigger"))?,
        )?;
        if state.records.contains_key(&key) {
            Ok(Some(status(&state, &key)?))
        } else {
            Ok(None)
        }
    }
    pub fn reminder_eligibility(
        &mut self,
        expected: &TaskReminderCandidate,
    ) -> Result<ReminderEligibility> {
        let (state, _) = self.read_reminders()?;
        let current = self.task_reminder_candidate(Path::new(&expected.root), &expected.owner)?;
        same_candidate(expected, &current)?;
        check_enabled(&state, &current)?;
        let (id, definition, trigger) = ready(&current)?;
        let key = execution_key(&state.device_id, &current.owner, id, definition, trigger)?;
        let trigger = ExactInstant::parse(trigger).map_err(|e| err(e.to_string()))?;
        let at = now()?;
        let timing = if at.compare(&trigger).is_lt() {
            ReminderTiming::Upcoming
        } else if missed(&current, &at)? {
            ReminderTiming::Missed
        } else {
            ReminderTiming::Due
        };
        let retained = if state.records.contains_key(&key) {
            Some(status(&state, &key)?)
        } else {
            None
        };
        Ok(ReminderEligibility {
            execution_key: key,
            timing,
            retained,
        })
    }
    /// Bounded traversal of consent records, not a second task store. Each task
    /// is re-qualified from its saved source; one unavailable scope is reported.
    pub fn reminder_schedule_page(
        &mut self,
        cursor: Option<&ReminderScheduleCursor>,
    ) -> Result<ReminderSchedulePage> {
        let (state, _) = self.read_reminders()?;
        if let Some(cursor) = cursor
            && (cursor.settings_revision != state.settings_revision || cursor.after.len() > 128)
        {
            return Err(CoreError::new(
                "staleBase",
                "Reminder selection changed; restart the bounded scan",
            ));
        }
        let mut candidates = vec![];
        let mut issues = vec![];
        let mut scanned = 0;
        let mut after = None;
        let mut complete = true;
        for (key, subscription) in &state.subscriptions {
            if cursor.is_some_and(|cursor| key <= &cursor.after) {
                continue;
            }
            if scanned == 32 {
                complete = false;
                break;
            }
            scanned += 1;
            after = Some(key.clone());
            if !subscription.enabled {
                continue;
            }
            let Some(library) = state
                .libraries
                .get(&subscription.owner.workspace_id)
                .filter(|l| l.enabled)
            else {
                continue;
            };
            match self.task_reminder_candidate(Path::new(&library.root), &subscription.owner) {
                Ok(candidate)
                    if candidate.root_identity == library.identity
                        && candidate.resolution.reminder_id.as_deref()
                            == Some(&subscription.reminder_id) =>
                {
                    candidates.push(candidate)
                }
                Ok(_) => issues.push(issue(
                    subscription,
                    "reminderUnavailable",
                    "The original physical library or saved reminder identity changed",
                )),
                Err(error) => issues.push(issue(subscription, &error.code, &error.message)),
            }
        }
        Ok(ReminderSchedulePage {
            candidates,
            issues,
            complete,
            next: if complete {
                None
            } else {
                after.map(|after| ReminderScheduleCursor {
                    settings_revision: state.settings_revision,
                    after,
                })
            },
        })
    }
    fn reminder_task_source(&mut self, root: &Path, owner: &NodeRef) -> Result<PathBuf> {
        let target = self.resolve_navigation_target(
            root,
            &format!("weftext-node://{}/{}", owner.workspace_id, owner.node_id),
        )?;
        if target.status != "managed" || target.kind != "node" {
            return Err(CoreError::new(
                "reminderUnavailable",
                "The original live task is unavailable",
            ));
        }
        // Navigation deliberately returns the Node directory. Resolve its
        // canonical source through the existing physical admission boundary;
        // the directory is never passed to a source-file API.
        let directory = target
            .path
            .ok_or_else(|| err("Task has no current admitted location"))?;
        let source = self.managed_node(root, Path::new(&directory), false)?;
        let path = PathBuf::from(source.path);
        if self.qualified_node_at(root, &path)?.owner != *owner {
            return Err(CoreError::new(
                "identityConflict",
                "The original reminder location now belongs to another object",
            ));
        }
        Ok(path)
    }
    fn capture_reminder_task(
        &mut self,
        root: &Path,
        owner: &NodeRef,
    ) -> Result<(TaskReminderCandidate, Vec<SnapshotWitness>)> {
        self.sync_local_owner()?;
        if !owner_valid(owner) {
            return Err(err("Invalid original task reference"));
        }
        let root = absolute_path(root)?;
        let workspace = self.require_workspace(&root)?;
        if workspace.workspace_id != owner.workspace_id {
            return Err(err("Task belongs to another selected library"));
        }
        let path = self.reminder_task_source(&root, owner)?;
        let qualified = self.qualified_query_node_at(&root, &path, owner, "live")?;
        let mut witnesses = vec![];
        Self::merge_query_node_feature_witness(&mut witnesses, &qualified.witness)?;
        let config = self.read_config(&root, ConfigRole::Schema)?;
        let config_witness = self.take_snapshot_witness(&config.base_id)?;
        let schema = match config.bytes {
            Some(bytes) => Schema::from_author_json(&decode(&bytes)?),
            None => Schema::bundled(),
        }
        .map_err(|e| CoreError::new("invalidSchema", e.to_string()))?;
        Self::merge_feature_witnesses(&mut witnesses, vec![config_witness])?;
        let source = witnesses
            .iter()
            .find(|w| Path::new(&w.binding.path) == path)
            .and_then(|w| w.binding.captured.as_ref())
            .ok_or_else(|| err("Missing qualified task source"))?;
        let projection = weftext_data::project(&decode(&source.bytes)?, &schema);
        if projection.qualification != "qualified"
            || projection.id.as_deref() != Some(owner.node_id.as_str())
            || !projection.resolved_types.iter().any(|t| t == "tasks/task")
        {
            return Err(CoreError::new(
                "reminderUnavailable",
                "The saved object is not a qualified task",
            ));
        }
        let cell = |id: &str| {
            projection
                .fields
                .iter()
                .find(|f| f.field_id == id)
                .map(|f| &f.cell)
        };
        let task_open = match cell("tasks/status") {
            Some(Cell::Value { value }) => match value.text() {
                Some("open") => true,
                Some("done" | "cancelled") => false,
                _ => return Err(err("Saved task status is invalid")),
            },
            _ => {
                return Err(CoreError::new(
                    "reminderUnavailable",
                    "Saved task status is unavailable",
                ));
            }
        };
        let value = |id: &str| -> Result<Option<&weftext_data::NativeValue>> {
            match cell(id) {
                Some(Cell::Value { value }) => Ok(Some(value)),
                Some(Cell::Absent) => Ok(None),
                _ => Err(CoreError::new(
                    "reminderUnavailable",
                    format!("Saved {id} is invalid or unavailable"),
                )),
            }
        };
        let reminder = value("tasks/reminder")?;
        // The field shape may be absent on an old schema; that is an explicit
        // configuration gap, never proof that the library has no reminders.
        let due = value("tasks/due")?;
        let resolution = weftext_data::resolve_task_reminder(reminder, due, &schema);
        let root_identity =
            ParentGuard::acquire(&root.join(".reminder-root"))?.directory_identity()?;
        let candidate = TaskReminderCandidate {
            root: display(&root),
            owner: owner.clone(),
            task_open,
            resolution,
            root_identity,
        };
        Ok((candidate, witnesses))
    }
    pub fn task_reminder_candidate(
        &mut self,
        root: &Path,
        owner: &NodeRef,
    ) -> Result<TaskReminderCandidate> {
        self.capture_reminder_task(root, owner)
            .map(|(candidate, _)| candidate)
    }
    pub fn task_reminder_for_session(&mut self, session_id: &str) -> Result<TaskReminderCandidate> {
        self.sync_local_owner()?;
        self.check_actor_session(session_id)?;
        let session = self
            .sessions
            .get(session_id)
            .ok_or_else(|| CoreError::new("sessionMissing", "Open the original task again"))?;
        let root = session.root.clone();
        let path = session.path.clone();
        let owner = self.qualified_node_at(&root, &path)?;
        self.task_reminder_candidate(&root, &owner.owner)
    }
    fn with_reminder_inputs<T>(
        &mut self,
        root: &Path,
        witnesses: &[SnapshotWitness],
        action: impl FnOnce(&mut Core) -> Result<T>,
    ) -> Result<T> {
        self.with_reminder_input_groups(&[(root, witnesses)], action)
    }
    fn with_reminder_input_groups<T>(
        &mut self,
        groups: &[(&Path, &[SnapshotWitness])],
        action: impl FnOnce(&mut Core) -> Result<T>,
    ) -> Result<T> {
        let mut guards = vec![];
        let mut handles = vec![];
        let mut seen = BTreeMap::<String, &SnapshotBinding>::new();
        let mut absent = vec![];
        let mut total = 0usize;
        for (root, witnesses) in groups {
            for witness in *witnesses {
                let basis = &witness.binding;
                if let Some(existing) = seen.get(&basis.path) {
                    if existing.parent != basis.parent
                        || existing.captured.as_ref().map(|f| (&f.stamp, &f.bytes))
                            != basis.captured.as_ref().map(|f| (&f.stamp, &f.bytes))
                    {
                        return Err(CoreError::new(
                            "staleBase",
                            "Competing original reminder dependencies",
                        ));
                    }
                    continue;
                }
                seen.insert(basis.path.clone(), basis);
                if seen.len() > 1024 {
                    return Err(CoreError::new(
                        "budget",
                        "Reminder input guard count exceeded",
                    ));
                }
                let path = Path::new(&basis.path);
                if crate::history::history_relative_path(path, root).is_none() {
                    return Err(err(
                        "Reminder dependency belongs outside the original library",
                    ));
                }
                self.check_pending(path)?;
                let guard = ParentGuard::acquire(path)?;
                if guard.directory_identity()? != basis.parent {
                    return Err(CoreError::new(
                        "staleBase",
                        "Original reminder input parent changed",
                    ));
                }
                let Some(expected) = basis.captured.as_ref() else {
                    reminder_absent(path)?;
                    absent.push(path);
                    guards.push(guard);
                    continue;
                };
                total = total
                    .checked_add(expected.bytes.len())
                    .ok_or_else(|| err("Reminder input budget overflow"))?;
                if total > 64 * 1024 * 1024 || seen.len() > 1024 {
                    return Err(CoreError::new(
                        "budget",
                        "Reminder input guard budget exceeded",
                    ));
                }
                let mut handle = guard.open_read_guard_file()?;
                let stamp_now = stamp(&handle)?;
                self.check_pending_identity(&stamp_now.identity)?;
                if guard.directory_identity()? != basis.parent
                    || stamp_now != expected.stamp
                    || read_blob(&mut handle)? != expected.bytes
                {
                    return Err(CoreError::new(
                        "staleBase",
                        "A reminder input changed before the durable decision",
                    ));
                }
                guards.push(guard);
                handles.push((handle, expected));
            }
        }
        let result = action(self);
        for (file, expected) in &mut handles {
            if stamp(file)? != expected.stamp || read_blob(file)? != expected.bytes {
                return Err(CoreError::new(
                    "staleBase",
                    "A reminder input changed during the durable decision; retain original outcome",
                ));
            }
        }
        for path in absent {
            reminder_absent(path)?;
        }
        drop(handles);
        drop(guards);
        result
    }
    pub fn set_reminder_library(
        &mut self,
        root: &Path,
        enabled: bool,
        expected_revision: u64,
        mutation_id: &str,
    ) -> Result<ReminderSettings> {
        let (mut state, digest) = self.read_reminders()?;
        let root = absolute_path(root)?;
        let workspace = self.require_workspace(&root)?;
        let root_guard = ParentGuard::acquire(&root.join(".reminder-root"))?;
        let identity = root_guard.directory_identity()?;
        let fp = hash(&serde_json::to_vec(&(
            "library",
            &workspace.workspace_id,
            &identity,
            enabled,
        ))?);
        if settings_retry(&state, expected_revision, mutation_id, &fp)? {
            return Ok(settings(&state));
        }
        if state
            .libraries
            .get(&workspace.workspace_id)
            .is_some_and(|old| old.identity != identity)
        {
            return Err(CoreError::new(
                "foreignLibraryState",
                "This original library selection names a different physical root",
            ));
        }
        state.libraries.insert(
            workspace.workspace_id,
            LibrarySelection {
                root: display(&root),
                identity: identity.clone(),
                enabled,
            },
        );
        finish_settings(&mut state, mutation_id, fp)?;
        let state = self.write_reminders(state, digest.as_deref())?;
        if root_guard.directory_identity()? != identity {
            return Err(CoreError::new(
                "staleBase",
                "Original library changed during the device selection",
            ));
        }
        Ok(settings(&state))
    }
    /// Revoking personal consent does not need the author library to be online.
    pub fn disable_reminder_library(
        &mut self,
        workspace_id: &str,
        expected_revision: u64,
        mutation_id: &str,
    ) -> Result<ReminderSettings> {
        let (mut state, digest) = self.read_reminders()?;
        let library = state
            .libraries
            .get(workspace_id)
            .ok_or_else(|| err("No original reminder library selection"))?;
        let fingerprint = hash(&serde_json::to_vec(&(
            "library",
            workspace_id,
            &library.identity,
            false,
        ))?);
        if settings_retry(&state, expected_revision, mutation_id, &fingerprint)? {
            return Ok(settings(&state));
        }
        state.libraries.get_mut(workspace_id).unwrap().enabled = false;
        finish_settings(&mut state, mutation_id, fingerprint)?;
        let state = self.write_reminders(state, digest.as_deref())?;
        Ok(settings(&state))
    }
    pub fn set_reminder_subscription(
        &mut self,
        root: &Path,
        owner: &NodeRef,
        reminder_id: &str,
        enabled: bool,
        expected_revision: u64,
        mutation_id: &str,
    ) -> Result<ReminderSettings> {
        let (mut state, digest) = self.read_reminders()?;
        if !owner_valid(owner) || !uuid(reminder_id) {
            return Err(err("Invalid task/reminder identity"));
        }
        let fp = hash(&serde_json::to_vec(&(
            "subscription",
            owner,
            reminder_id,
            enabled,
        ))?);
        if settings_retry(&state, expected_revision, mutation_id, &fp)? {
            return Ok(settings(&state));
        }
        let key = subscription_key(owner, reminder_id);
        if !enabled {
            let old = state
                .subscriptions
                .get_mut(&key)
                .ok_or_else(|| err("No original reminder subscription"))?;
            old.enabled = false;
            finish_settings(&mut state, mutation_id, fp)?;
            let state = self.write_reminders(state, digest.as_deref())?;
            return Ok(settings(&state));
        }
        let (candidate, witnesses) = self.capture_reminder_task(root, owner)?;
        if candidate.resolution.state != TaskReminderState::Ready
            || candidate.resolution.reminder_id.as_deref() != Some(reminder_id)
        {
            return Err(CoreError::new(
                "reminderUnavailable",
                "Save the exact valid reminder before enabling it on this device",
            ));
        }
        let library = state
            .libraries
            .get(&owner.workspace_id)
            .ok_or_else(|| err("Select this library on this device first"))?;
        if library.identity != candidate.root_identity || !library.enabled {
            return Err(err(
                "The original selected library is disabled or has changed identity",
            ));
        }
        state.subscriptions.insert(
            key,
            ReminderSubscription {
                owner: owner.clone(),
                reminder_id: reminder_id.into(),
                enabled,
            },
        );
        finish_settings(&mut state, mutation_id, fp)?;
        let state = self.with_reminder_inputs(Path::new(&candidate.root), &witnesses, |core| {
            core.write_reminders(state, digest.as_deref())
        })?;
        Ok(settings(&state))
    }
    /// Called only by the Desktop owner's coordinator, never a Server route.
    /// Its exact OS request is durable before returning the one-use permit.
    pub fn begin_reminder_delivery(
        &mut self,
        expected: &TaskReminderCandidate,
        options: ReminderNotificationOptions,
    ) -> Result<ReminderDeliveryStart> {
        self.begin_reminder_delivery_at(expected, options, now()?)
    }
    fn begin_reminder_delivery_at(
        &mut self,
        expected: &TaskReminderCandidate,
        options: ReminderNotificationOptions,
        at: ExactInstant,
    ) -> Result<ReminderDeliveryStart> {
        let (mut state, digest) = self.read_reminders()?;
        let (candidate, witnesses) =
            self.capture_reminder_task(Path::new(&expected.root), &expected.owner)?;
        same_candidate(expected, &candidate)?;
        check_enabled(&state, &candidate)?;
        let (id, definition, trigger) = ready(&candidate)?;
        let instant = ExactInstant::parse(trigger).map_err(|e| err(e.to_string()))?;
        if at.compare(&instant).is_lt() {
            return Err(CoreError::new(
                "reminderNotDue",
                "The original reminder instant has not arrived",
            ));
        }
        if missed(&candidate, &at)? {
            return Err(CoreError::new(
                "reminderMissed",
                "This reminder belongs in the bounded missed summary",
            ));
        }
        let key = execution_key(&state.device_id, &candidate.owner, id, definition, trigger)?;
        let attempts = if let Some(record) = state.records.get(&key) {
            match &record.delivery {
                Delivery::Individual { attempt }
                    if !record.handled
                        && record.invalidated.is_none()
                        && matches!(
                            attempt.delivery,
                            ReminderDeliveryState::FailedBeforeCall { .. }
                        ) =>
                {
                    attempt
                        .attempt_count
                        .checked_add(1)
                        .ok_or_else(|| err("Reminder attempt counter overflow"))?
                }
                _ => return Ok(ReminderDeliveryStart::Retained(status(&state, &key)?)),
            }
        } else {
            1
        };
        let attempt = make_attempt(&key, options, &at, attempts)?;
        let permit = permit(&key, &attempt);
        state.records.insert(
            key,
            Record {
                owner: candidate.owner.clone(),
                reminder_id: id.into(),
                definition_digest: definition.into(),
                trigger_instant: trigger.into(),
                delivery: Delivery::Individual { attempt },
                handled: false,
                invalidated: None,
            },
        );
        self.with_reminder_inputs(Path::new(&candidate.root), &witnesses, |core| {
            core.write_reminders(state, digest.as_deref())
        })?;
        Ok(ReminderDeliveryStart::Started(permit))
    }
    /// The trusted coordinator offers one bounded missed summary per resume.
    /// Membership is durably assigned before any external call. An original
    /// submitting/unknown/submitted member cannot be regrouped or replayed.
    pub fn begin_reminder_summary(
        &mut self,
        expected: &[TaskReminderCandidate],
        options: ReminderNotificationOptions,
    ) -> Result<ReminderDeliveryStart> {
        self.begin_reminder_summary_at(expected, options, &now()?)
    }
    fn begin_reminder_summary_at(
        &mut self,
        expected: &[TaskReminderCandidate],
        options: ReminderNotificationOptions,
        at: &ExactInstant,
    ) -> Result<ReminderDeliveryStart> {
        if expected.is_empty() || expected.len() > MAX_SUMMARY_MEMBERS {
            return Err(err("Missed summary requires 1–64 original reminders"));
        }
        let (mut state, digest) = self.read_reminders()?;
        let mut captured = vec![];
        let mut members = BTreeSet::new();
        for original in expected {
            let (candidate, witnesses) =
                self.capture_reminder_task(Path::new(&original.root), &original.owner)?;
            same_candidate(original, &candidate)?;
            check_enabled(&state, &candidate)?;
            let (id, definition, trigger) = ready(&candidate)?;
            if !missed(&candidate, at)? {
                return Err(CoreError::new(
                    "reminderNotMissed",
                    "A summary member is not past its normal delivery window",
                ));
            }
            let key = execution_key(&state.device_id, &candidate.owner, id, definition, trigger)?;
            if !members.insert(key.clone()) {
                return Err(err("Duplicate missed summary member"));
            }
            captured.push((candidate, witnesses, key));
        }
        let members = members.into_iter().collect::<Vec<_>>();
        let key = hash(&serde_json::to_vec(&(
            "weftext.reminder-summary.v1",
            &state.device_id,
            &members,
        ))?);
        if let Some(summary) = state.summaries.get(&key)
            && !matches!(
                summary.attempt.delivery,
                ReminderDeliveryState::FailedBeforeCall { .. }
            )
        {
            return Ok(ReminderDeliveryStart::Retained(status(&state, &key)?));
        }
        let prior_count = state
            .summaries
            .get(&key)
            .map_or(0, |s| s.attempt.attempt_count);
        for member in &members {
            if let Some(record) = state.records.get(member) {
                if record.handled || record.invalidated.is_some() {
                    return Err(CoreError::new(
                        "reminderRetained",
                        "An original summary member is handled or cancelled",
                    ));
                }
                match &record.delivery {
                    Delivery::Individual { attempt }
                        if matches!(
                            attempt.delivery,
                            ReminderDeliveryState::FailedBeforeCall { .. }
                        ) => {}
                    Delivery::SummaryMember { summary_key, .. }
                        if summary_key == &key
                            || state.summaries.get(summary_key).is_some_and(|summary| {
                                matches!(
                                    summary.attempt.delivery,
                                    ReminderDeliveryState::FailedBeforeCall { .. }
                                )
                            }) => {}
                    _ => {
                        return Err(CoreError::new(
                            "reminderRetained",
                            "An original summary member already has a possible external effect; retain its original outcome",
                        ));
                    }
                }
            }
        }
        let count = prior_count
            .checked_add(1)
            .ok_or_else(|| err("Summary attempt counter overflow"))?;
        let attempt = make_attempt(&key, options, at, count)?;
        let permit = permit(&key, &attempt);
        for (candidate, _, member) in &captured {
            let (id, definition, trigger) = ready(candidate)?;
            let prior_failed_attempt =
                state
                    .records
                    .get(member)
                    .and_then(|record| match &record.delivery {
                        Delivery::Individual { attempt } => Some(attempt.clone()),
                        Delivery::SummaryMember {
                            prior_failed_attempt,
                            ..
                        } => prior_failed_attempt.clone(),
                    });
            state.records.insert(
                member.clone(),
                Record {
                    owner: candidate.owner.clone(),
                    reminder_id: id.into(),
                    definition_digest: definition.into(),
                    trigger_instant: trigger.into(),
                    delivery: Delivery::SummaryMember {
                        summary_key: key.clone(),
                        prior_failed_attempt,
                    },
                    handled: false,
                    invalidated: None,
                },
            );
        }
        state.summaries.insert(key, Summary { members, attempt });
        let groups = captured
            .iter()
            .map(|(candidate, witnesses, _)| (Path::new(&candidate.root), witnesses.as_slice()))
            .collect::<Vec<_>>();
        self.with_reminder_input_groups(&groups, |core| {
            core.write_reminders(state, digest.as_deref())
        })?;
        Ok(ReminderDeliveryStart::Started(permit))
    }
    /// Recheck before invoking an external API if the adapter yielded since
    /// begin. A false/expired permit never authorizes a fresh key or retry.
    pub fn check_reminder_permit(&mut self, permit: &ReminderDeliveryPermit) -> Result<()> {
        self.check_reminder_permit_with_clock(permit, now)
    }
    fn check_reminder_permit_with_clock(
        &mut self,
        permit: &ReminderDeliveryPermit,
        mut clock: impl FnMut() -> Result<ExactInstant>,
    ) -> Result<()> {
        if permit.admitted.load(std::sync::atomic::Ordering::Acquire) {
            return Err(CoreError::new(
                "reminderPermitUsed",
                "The one-use external submission permit has already been admitted; retain/query its original result",
            ));
        }
        let (state, _) = self.read_reminders()?;
        let attempt = lookup_attempt(&state, &permit.key)?;
        if attempt.permit_id != permit.permit_id
            || attempt.request != permit.request
            || !matches!(attempt.delivery, ReminderDeliveryState::Submitting)
        {
            return Err(err("The original notification permit is no longer current"));
        }
        permit_age(permit, attempt, &clock()?)?;
        let summary = state.summaries.contains_key(&permit.key);
        let members = if let Some(summary) = state.summaries.get(&permit.key) {
            summary.members.clone()
        } else {
            vec![permit.key.clone()]
        };
        let mut captured = vec![];
        for key in members {
            let record = &state.records[&key];
            if record.handled || record.invalidated.is_some() {
                return Err(err("A notification member is handled or invalidated"));
            }
            let library = state
                .libraries
                .get(&record.owner.workspace_id)
                .ok_or_else(|| err("Original library selection is missing"))?;
            let (candidate, witnesses) =
                self.capture_reminder_task(Path::new(&library.root), &record.owner)?;
            check_enabled(&state, &candidate)?;
            if !summary && missed(&candidate, &clock()?)? {
                return Err(CoreError::new(
                    "reminderMissed",
                    "The target minute ended before the external call; no normal notification is authorized",
                ));
            }
            let (id, definition, trigger) = ready(&candidate)?;
            if execution_key(&state.device_id, &candidate.owner, id, definition, trigger)? != key {
                return Err(CoreError::new(
                    "staleBase",
                    "The original reminder definition has changed",
                ));
            }
            captured.push((candidate, witnesses));
        }
        let groups = captured
            .iter()
            .map(|(candidate, witnesses)| (Path::new(&candidate.root), witnesses.as_slice()))
            .collect::<Vec<_>>();
        // Prove one current input closure, including absent default-config
        // slots, before consuming the one-use permission. These guards do not
        // claim atomicity with a later author edit or the external OS call.
        let mut check_time = || {
            let at = clock()?;
            permit_age(permit, attempt, &at)?;
            if !summary && missed(&captured[0].0, &at)? {
                return Err(CoreError::new(
                    "reminderMissed",
                    "The target minute ended before the external call; no normal notification is authorized",
                ));
            }
            Ok(())
        };
        self.with_reminder_input_groups(&groups, |_| check_time())?;
        check_time()?;
        permit
            .admitted
            .compare_exchange(
                false,
                true,
                std::sync::atomic::Ordering::AcqRel,
                std::sync::atomic::Ordering::Acquire,
            )
            .map_err(|_| {
                CoreError::new(
                    "reminderPermitUsed",
                    "The original external permit was already admitted",
                )
            })?;
        Ok(())
    }
    pub fn finish_reminder_delivery(
        &mut self,
        permit: &ReminderDeliveryPermit,
        outcome: ReminderSubmissionOutcome,
    ) -> Result<ReminderDeliveryStatus> {
        let (mut state, digest) = self.read_reminders()?;
        let attempt = lookup_attempt_mut(&mut state, &permit.key)?;
        if attempt.permit_id != permit.permit_id || attempt.request != permit.request {
            return Err(CoreError::new(
                "operationConflict",
                "Notification result does not belong to the exact original permit/request",
            ));
        }
        if matches!(
            attempt.delivery,
            ReminderDeliveryState::SubmittedToSystem { .. }
        ) {
            return status(&state, &permit.key);
        }
        attempt.delivery = match outcome {
            ReminderSubmissionOutcome::SubmittedToSystem => {
                ReminderDeliveryState::SubmittedToSystem {
                    at: now()?.canonical().map_err(|e| err(e.to_string()))?,
                }
            }
            ReminderSubmissionOutcome::FailedBeforeCall(diagnostic) => {
                if !matches!(
                    attempt.delivery,
                    ReminderDeliveryState::Submitting
                        | ReminderDeliveryState::FailedBeforeCall { .. }
                ) {
                    return Err(err(
                        "An unknown external result cannot become a pre-call failure",
                    ));
                }
                ReminderDeliveryState::FailedBeforeCall { diagnostic }
            }
            ReminderSubmissionOutcome::Unknown(diagnostic) => {
                ReminderDeliveryState::Unknown { diagnostic }
            }
        };
        let state = self.write_reminders(state, digest.as_deref())?;
        status(&state, &permit.key)
    }
    pub fn reminder_delivery_status(&mut self, key: &str) -> Result<ReminderDeliveryStatus> {
        let (state, _) = self.read_reminders()?;
        status(&state, key)
    }
    /// Read one original OS effect by its exact individual or summary key.
    /// This never requires the task's current source to be available and never
    /// changes an external result. Summary member keys are not OS addresses.
    pub fn original_reminder_notification(
        &mut self,
        key: &str,
    ) -> Result<ReminderOriginalNotification> {
        let (state, _) = self.read_reminders()?;
        original_notification(&state, key)
    }
    /// Exact original addresses for OS-history reconciliation. Absence from
    /// Action Center never changes the durable unknown result into retryable.
    pub fn reminder_original_notifications(
        &mut self,
        after: Option<&str>,
    ) -> Result<Vec<ReminderOriginalNotification>> {
        let (state, _) = self.read_reminders()?;
        if after.is_some_and(|key| !digest(key)) {
            return Err(err("Invalid original-notification cursor"));
        }
        let mut keys = state
            .records
            .iter()
            .filter(|(_, r)| matches!(r.delivery, Delivery::Individual { .. }))
            .map(|(k, _)| k)
            .chain(state.summaries.keys())
            .collect::<Vec<_>>();
        keys.sort();
        keys.into_iter()
            .filter(|key| after.is_none_or(|after| key.as_str() > after))
            .take(32)
            .map(|key| original_notification(&state, key))
            .collect()
    }
    /// Re-authorize an original OS address before keeping it visible. Native
    /// code may withdraw this exact address when ineligible. Source failures
    /// produce issues; only a definite saved change writes a cancellation.
    pub fn refresh_reminder_notification(
        &mut self,
        key: &str,
    ) -> Result<ReminderNotificationEligibility> {
        let (mut state, digest) = self.read_reminders()?;
        let attempt = lookup_attempt(&state, key)?.clone();
        let members = if let Some(summary) = state.summaries.get(key) {
            summary.members.clone()
        } else {
            vec![key.into()]
        };
        let mut eligible = true;
        let mut issues = vec![];
        let mut captured = vec![];
        let mut changed = false;
        for member in members {
            let record = state
                .records
                .get(&member)
                .ok_or_else(|| err("Missing original reminder member"))?
                .clone();
            if record.handled || record.invalidated.is_some() {
                eligible = false;
                continue;
            }
            let Some(library) = state.libraries.get(&record.owner.workspace_id).cloned() else {
                return Err(err("Original reminder has no library selection"));
            };
            let reason = if !library.enabled {
                Some(ReminderCancellation::LibraryDisabled)
            } else if !state
                .subscriptions
                .get(&subscription_key(&record.owner, &record.reminder_id))
                .is_some_and(|s| s.enabled)
            {
                Some(ReminderCancellation::SubscriptionDisabled)
            } else {
                match self.capture_reminder_task(Path::new(&library.root), &record.owner) {
                    Err(error) => {
                        eligible = false;
                        issues.push(ReminderScheduleIssue {
                            owner: record.owner.clone(),
                            reminder_id: record.reminder_id.clone(),
                            code: error.code,
                            message: error.message,
                        });
                        None
                    }
                    Ok((candidate, witnesses)) => {
                        let reason = if candidate.root_identity != library.identity {
                            eligible = false;
                            issues.push(ReminderScheduleIssue {
                                owner: record.owner.clone(),
                                reminder_id: record.reminder_id.clone(),
                                code: "foreignLibraryState".into(),
                                message: "Original library physical identity changed".into(),
                            });
                            None
                        } else if !candidate.task_open {
                            Some(ReminderCancellation::TaskClosed)
                        } else if candidate.resolution.state == TaskReminderState::Absent {
                            Some(ReminderCancellation::DefinitionChanged)
                        } else if candidate.resolution.state != TaskReminderState::Ready {
                            eligible = false;
                            issues.push(ReminderScheduleIssue {
                                owner: record.owner.clone(),
                                reminder_id: record.reminder_id.clone(),
                                code: "reminderUnavailable".into(),
                                message:
                                    "Original saved reminder no longer resolves to an exact trigger"
                                        .into(),
                            });
                            None
                        } else {
                            let (id, definition, trigger) = ready(&candidate)?;
                            if execution_key(
                                &state.device_id,
                                &candidate.owner,
                                id,
                                definition,
                                trigger,
                            )? != member
                            {
                                Some(ReminderCancellation::DefinitionChanged)
                            } else {
                                None
                            }
                        };
                        captured.push((candidate, witnesses));
                        reason
                    }
                }
            };
            if let Some(reason) = reason {
                state.records.get_mut(&member).unwrap().invalidated = Some(reason);
                eligible = false;
                changed = true;
            }
        }
        if changed {
            let groups = captured
                .iter()
                .map(|(candidate, witnesses)| (Path::new(&candidate.root), witnesses.as_slice()))
                .collect::<Vec<_>>();
            state = self.with_reminder_input_groups(&groups, |core| {
                core.write_reminders(state, digest.as_deref())
            })?;
        }
        Ok(ReminderNotificationEligibility {
            original: ReminderOriginalNotification {
                execution_key: key.into(),
                request: attempt.request,
                delivery: status(&state, key)?.delivery,
            },
            eligible,
            issues,
        })
    }
    pub fn confirm_reminder_in_system_history(
        &mut self,
        key: &str,
        observed: &ReminderNotificationRequest,
    ) -> Result<ReminderDeliveryStatus> {
        let (mut state, digest) = self.read_reminders()?;
        let attempt = lookup_attempt_mut(&mut state, key)?;
        if &attempt.request != observed {
            return Err(CoreError::new(
                "operationConflict",
                "OS history does not match the complete original notification request",
            ));
        }
        if matches!(
            attempt.delivery,
            ReminderDeliveryState::Submitting | ReminderDeliveryState::Unknown { .. }
        ) {
            attempt.delivery = ReminderDeliveryState::SubmittedToSystem {
                at: now()?.canonical().map_err(|e| err(e.to_string()))?,
            };
            state = self.write_reminders(state, digest.as_deref())?;
        }
        status(&state, key)
    }
    /// Explicit personal handling never changes the Task's author status.
    pub fn handle_reminder(&mut self, key: &str) -> Result<ReminderDeliveryStatus> {
        let (mut state, digest) = self.read_reminders()?;
        let members = if let Some(summary) = state.summaries.get(key) {
            summary.members.clone()
        } else if state.records.contains_key(key) {
            vec![key.into()]
        } else {
            return Err(err("No original reminder effect"));
        };
        let mut changed = false;
        for member in members {
            let record = state
                .records
                .get_mut(&member)
                .ok_or_else(|| err("Missing original summary member"))?;
            if !record.handled {
                record.handled = true;
                changed = true;
            }
        }
        if changed {
            state = self.write_reminders(state, digest.as_deref())?;
        }
        status(&state, key)
    }
    pub fn resolve_reminder_activation(
        &mut self,
        activation_key: &str,
    ) -> Result<ReminderActivation> {
        let (state, _) = self.read_reminders()?;
        if !uuid(activation_key) {
            return Err(err("Invalid opaque reminder activation"));
        }
        let mut keys = vec![];
        for (key, record) in &state.records {
            if matches!(&record.delivery,Delivery::Individual{attempt}if attempt.request.activation_key==activation_key)
            {
                keys.push(key.clone());
            }
        }
        for summary in state.summaries.values() {
            if summary.attempt.request.activation_key == activation_key {
                keys.extend(summary.members.clone());
            }
        }
        if keys.is_empty() {
            return Err(CoreError::new(
                "reminderUnavailable",
                "No original notification belongs to this profile/device",
            ));
        }
        let mut out = vec![];
        let mut issues = vec![];
        for key in keys {
            let record = &state.records[&key];
            if record.handled || record.invalidated.is_some() {
                continue;
            }
            let Some(library) = state.libraries.get(&record.owner.workspace_id) else {
                continue;
            };
            let candidate =
                match self.task_reminder_candidate(Path::new(&library.root), &record.owner) {
                    Ok(candidate) => candidate,
                    Err(error) => {
                        issues.push(ReminderScheduleIssue {
                            owner: record.owner.clone(),
                            reminder_id: record.reminder_id.clone(),
                            code: error.code,
                            message: error.message,
                        });
                        continue;
                    }
                };
            if check_enabled(&state, &candidate).is_err() {
                continue;
            }
            let (id, definition, trigger) = ready(&candidate)?;
            if execution_key(&state.device_id, &candidate.owner, id, definition, trigger)? == key {
                out.push(candidate);
            }
        }
        Ok(ReminderActivation {
            candidates: out,
            issues,
        })
    }
}
fn settings(state: &State) -> ReminderSettings {
    ReminderSettings {
        device_id: state.device_id.clone(),
        settings_revision: state.settings_revision,
        libraries: state
            .libraries
            .iter()
            .map(|(id, l)| ReminderLibraryView {
                workspace_id: id.clone(),
                root: l.root.clone(),
                enabled: l.enabled,
            })
            .collect(),
        subscriptions: state.subscriptions.values().cloned().collect(),
    }
}
fn issue(subscription: &ReminderSubscription, code: &str, message: &str) -> ReminderScheduleIssue {
    ReminderScheduleIssue {
        owner: subscription.owner.clone(),
        reminder_id: subscription.reminder_id.clone(),
        code: code.into(),
        message: message.into(),
    }
}
fn reminder_absent(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(_) => Err(CoreError::new(
            "staleBase",
            "An original default-configuration slot is now occupied",
        )),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}
fn missed(candidate: &TaskReminderCandidate, at: &ExactInstant) -> Result<bool> {
    let end = ExactInstant::parse(
        candidate
            .resolution
            .normal_delivery_until
            .as_deref()
            .ok_or_else(|| err("Ready reminder lacks a target minute boundary"))?,
    )
    .map_err(|e| err(e.to_string()))?;
    Ok(!at.compare(&end).is_lt())
}
fn settings_retry(state: &State, expected: u64, id: &str, fingerprint: &str) -> Result<bool> {
    if !uuid(id) {
        return Err(err("Settings mutation requires a canonical UUIDv4"));
    }
    if let Some(mutation) = &state.last_settings_mutation
        && mutation.id == id
    {
        if mutation.fingerprint != fingerprint {
            return Err(CoreError::new(
                "operationConflict",
                "Settings mutation id belongs to different input",
            ));
        }
        return Ok(true);
    }
    if state.settings_revision != expected {
        return Err(CoreError::new(
            "staleBase",
            "Reminder settings changed; preserve the original pending request",
        ));
    }
    Ok(false)
}
fn finish_settings(state: &mut State, id: &str, fingerprint: String) -> Result<()> {
    state.settings_revision = state
        .settings_revision
        .checked_add(1)
        .ok_or_else(|| err("Settings revision overflow"))?;
    state.last_settings_mutation = Some(SettingsMutation {
        id: id.into(),
        fingerprint,
        revision: state.settings_revision,
    });
    Ok(())
}
fn ready(candidate: &TaskReminderCandidate) -> Result<(&str, &str, &str)> {
    if !candidate.task_open || candidate.resolution.state != TaskReminderState::Ready {
        return Err(CoreError::new(
            "reminderUnavailable",
            "The original task/reminder is not eligible",
        ));
    }
    match (
        &candidate.resolution.reminder_id,
        &candidate.resolution.definition_digest,
        &candidate.resolution.trigger_instant,
    ) {
        (Some(id), Some(definition), Some(trigger)) => Ok((id, definition, trigger)),
        _ => Err(err("Ready reminder lacks its original definition")),
    }
}
fn same_candidate(expected: &TaskReminderCandidate, current: &TaskReminderCandidate) -> Result<()> {
    if expected.owner != current.owner
        || expected.root_identity != current.root_identity
        || expected.resolution.reminder_id != current.resolution.reminder_id
        || expected.resolution.definition_digest != current.resolution.definition_digest
        || expected.resolution.trigger_instant != current.resolution.trigger_instant
    {
        return Err(CoreError::new(
            "staleBase",
            "The original reminder target or definition changed",
        ));
    }
    Ok(())
}
fn check_enabled(state: &State, candidate: &TaskReminderCandidate) -> Result<()> {
    let (id, _, _) = ready(candidate)?;
    let library = state
        .libraries
        .get(&candidate.owner.workspace_id)
        .ok_or_else(|| err("This library is not selected on this device"))?;
    if !library.enabled
        || library.identity != candidate.root_identity
        || !state
            .subscriptions
            .get(&subscription_key(&candidate.owner, id))
            .is_some_and(|s| s.enabled)
    {
        return Err(CoreError::new(
            "reminderDisabled",
            "The original library/reminder is not enabled on this device",
        ));
    }
    Ok(())
}
fn make_attempt(
    key: &str,
    options: ReminderNotificationOptions,
    at: &ExactInstant,
    count: u32,
) -> Result<Attempt> {
    let attempt = Attempt {
        permit_id: fresh(),
        attempt_at: at.canonical().map_err(|e| err(e.to_string()))?,
        expires_at: at
            .shifted_seconds(5)
            .and_then(|a| a.canonical())
            .map_err(|e| err(e.to_string()))?,
        attempt_count: count,
        request: ReminderNotificationRequest {
            app_id: options.app_id,
            tag: key.into(),
            group: GROUP.into(),
            activation_key: fresh(),
            title: options.title,
            body: options.body,
        },
        delivery: ReminderDeliveryState::Submitting,
    };
    validate_attempt(&attempt, key)?;
    Ok(attempt)
}
fn permit(key: &str, attempt: &Attempt) -> ReminderDeliveryPermit {
    ReminderDeliveryPermit {
        key: key.into(),
        permit_id: attempt.permit_id.clone(),
        request: attempt.request.clone(),
        expires_at: attempt.expires_at.clone(),
        issued: std::time::Instant::now(),
        admitted: std::sync::atomic::AtomicBool::new(false),
    }
}
fn permit_age(
    permit: &ReminderDeliveryPermit,
    attempt: &Attempt,
    current: &ExactInstant,
) -> Result<()> {
    let deadline = ExactInstant::parse(&permit.expires_at).map_err(|e| err(e.to_string()))?;
    let began = ExactInstant::parse(&attempt.attempt_at).map_err(|e| err(e.to_string()))?;
    if permit.issued.elapsed() >= std::time::Duration::from_secs(5)
        || current.compare(&began).is_lt()
        || !current.compare(&deadline).is_lt()
    {
        return Err(CoreError::new(
            "reminderPermitExpired",
            "The notification permit expired before external submission",
        ));
    }
    Ok(())
}
fn lookup_attempt<'a>(state: &'a State, key: &str) -> Result<&'a Attempt> {
    if let Some(summary) = state.summaries.get(key) {
        return Ok(&summary.attempt);
    }
    match &state
        .records
        .get(key)
        .ok_or_else(|| err("No original reminder effect"))?
        .delivery
    {
        Delivery::Individual { attempt } => Ok(attempt),
        Delivery::SummaryMember { summary_key, .. } => state
            .summaries
            .get(summary_key)
            .map(|s| &s.attempt)
            .ok_or_else(|| err("Missing original reminder summary")),
    }
}
fn original_notification(state: &State, key: &str) -> Result<ReminderOriginalNotification> {
    if !digest(key)
        || !state.summaries.contains_key(key)
            && !state
                .records
                .get(key)
                .is_some_and(|record| matches!(record.delivery, Delivery::Individual { .. }))
    {
        return Err(err("No original OS notification at this execution key"));
    }
    let attempt = lookup_attempt(state, key)?;
    Ok(ReminderOriginalNotification {
        execution_key: key.into(),
        request: attempt.request.clone(),
        delivery: status(state, key)?.delivery,
    })
}
fn lookup_attempt_mut<'a>(state: &'a mut State, key: &str) -> Result<&'a mut Attempt> {
    if state.summaries.contains_key(key) {
        return Ok(&mut state.summaries.get_mut(key).unwrap().attempt);
    }
    let summary = match &state
        .records
        .get(key)
        .ok_or_else(|| err("No original reminder effect"))?
        .delivery
    {
        Delivery::SummaryMember { summary_key, .. } => Some(summary_key.clone()),
        _ => None,
    };
    if let Some(summary) = summary {
        return state
            .summaries
            .get_mut(&summary)
            .map(|s| &mut s.attempt)
            .ok_or_else(|| err("Missing original reminder summary"));
    }
    match &mut state.records.get_mut(key).unwrap().delivery {
        Delivery::Individual { attempt } => Ok(attempt),
        _ => unreachable!(),
    }
}
fn status(state: &State, key: &str) -> Result<ReminderDeliveryStatus> {
    let attempt = lookup_attempt(state, key)?;
    let delivery=match &attempt.delivery {ReminderDeliveryState::Submitting=>ReminderDeliveryState::Unknown{diagnostic:"External submission may be pending or have completed; query the original OS notification, never retry blindly".into()},state=>state.clone()};
    let (record_handled, invalidated) = if let Some(record) = state.records.get(key) {
        (record.handled, record.invalidated.clone())
    } else if let Some(summary) = state.summaries.get(key) {
        (
            summary.members.iter().all(|key| state.records[key].handled),
            None,
        )
    } else {
        (false, None)
    };
    Ok(ReminderDeliveryStatus {
        execution_key: key.into(),
        delivery,
        handled: record_handled,
        invalidated,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    const DEVICE: &str = "ad7aaab7-0fd5-40c5-9087-13ae549f38d9";
    const LIBRARY: &str = "282c2f1d-393f-45ac-a193-78c5bf6ca59d";
    const NODE: &str = "91a6f054-741b-4a90-b17a-d70b3adf6f8a";
    const INTENT: &str = "a46e68ef-30a3-4f74-bba2-13327ce81f11";
    fn fixture() -> (State, String) {
        let mut state = State::new(DEVICE.into());
        state.libraries.insert(
            LIBRARY.into(),
            LibrarySelection {
                root: "D:/qualified-library".into(),
                identity: FileIdentity {
                    volume: 1,
                    index: 2,
                    created: 3,
                },
                enabled: true,
            },
        );
        let owner = NodeRef {
            workspace_id: LIBRARY.into(),
            node_id: NODE.into(),
        };
        let definition = "a".repeat(64);
        let trigger = "2026-10-23T01:00:12.123456789123Z";
        let key = execution_key(DEVICE, &owner, INTENT, &definition, trigger).unwrap();
        let at = ExactInstant::parse("2026-10-23T01:00:13.123456789123Z").unwrap();
        let attempt = make_attempt(
            &key,
            ReminderNotificationOptions {
                app_id: "Weftext.Test".into(),
                title: "Task reminder".into(),
                body: "Open the original saved task".into(),
            },
            &at,
            1,
        )
        .unwrap();
        state.records.insert(
            key.clone(),
            Record {
                owner,
                reminder_id: INTENT.into(),
                definition_digest: definition,
                trigger_instant: trigger.into(),
                delivery: Delivery::Individual { attempt },
                handled: false,
                invalidated: None,
            },
        );
        (state, key)
    }
    #[test]
    fn persisted_submitting_remains_original_unknown_after_restart_and_cancellation() {
        let (mut state, key) = fixture();
        let original = lookup_attempt(&state, &key).unwrap().request.clone();
        let bytes = serde_json::to_vec(&state).unwrap();
        validate_reminder_device_record(&bytes).unwrap();
        state = serde_json::from_slice(&bytes).unwrap();
        assert!(matches!(
            status(&state, &key).unwrap().delivery,
            ReminderDeliveryState::Unknown { .. }
        ));
        state.records.get_mut(&key).unwrap().invalidated = Some(ReminderCancellation::TaskClosed);
        state.records.get_mut(&key).unwrap().handled = true;
        let result = status(&state, &key).unwrap();
        assert!(result.handled);
        assert_eq!(result.invalidated, Some(ReminderCancellation::TaskClosed));
        assert!(matches!(
            result.delivery,
            ReminderDeliveryState::Unknown { .. }
        ));
        assert_eq!(lookup_attempt(&state, &key).unwrap().request, original);
        state.validate().unwrap();
    }
    #[test]
    fn strict_record_rejects_unknown_members_duplicate_keys_and_broken_original_links() {
        let (mut state, key) = fixture();
        let bytes = serde_json::to_vec(&state).unwrap();
        let mut raw: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        raw["guessedSuccess"] = true.into();
        assert!(validate_reminder_device_record(&serde_json::to_vec(&raw).unwrap()).is_err());
        let mut raw: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        raw["libraries"][LIBRARY]["identity"]["guessedOwner"] = true.into();
        assert!(validate_reminder_device_record(&serde_json::to_vec(&raw).unwrap()).is_err());
        let raw = String::from_utf8(bytes).unwrap().replace(
            "\"records\":{",
            &format!(
                "\"records\":{{\"{key}\":{},",
                serde_json::to_string(&state.records[&key]).unwrap()
            ),
        );
        assert!(validate_reminder_device_record(raw.as_bytes()).is_err());
        state.records.get_mut(&key).unwrap().delivery = Delivery::SummaryMember {
            summary_key: "b".repeat(64),
            prior_failed_attempt: None,
        };
        assert!(state.validate().is_err());
    }
    #[test]
    fn summary_membership_is_reciprocal_and_unknown_members_cannot_be_regrouped_in_record_shape() {
        let (mut state, key) = fixture();
        let members = vec![key.clone()];
        let summary =
            hash(&serde_json::to_vec(&("weftext.reminder-summary.v1", DEVICE, &members)).unwrap());
        let mut attempt = lookup_attempt(&state, &key).unwrap().clone();
        attempt.request.tag = summary.clone();
        state.records.get_mut(&key).unwrap().delivery = Delivery::SummaryMember {
            summary_key: summary.clone(),
            prior_failed_attempt: None,
        };
        state
            .summaries
            .insert(summary.clone(), Summary { members, attempt });
        state.validate().unwrap();
        assert!(matches!(
            status(&state, &key).unwrap().delivery,
            ReminderDeliveryState::Unknown { .. }
        ));
        let original = lookup_attempt(&state, &key).unwrap().clone();
        state.records.get_mut(&key).unwrap().delivery = Delivery::SummaryMember {
            summary_key: summary,
            prior_failed_attempt: Some(original),
        };
        assert!(state.validate().is_err());
    }
    #[test]
    fn missed_window_preserves_fraction_and_exact_boundary() {
        let (state, key) = fixture();
        let record = &state.records[&key];
        let candidate = TaskReminderCandidate {
            root: "D:/qualified-library".into(),
            owner: record.owner.clone(),
            task_open: true,
            root_identity: state.libraries[LIBRARY].identity.clone(),
            resolution: TaskReminderResolution {
                state: TaskReminderState::Ready,
                reminder_id: Some(INTENT.into()),
                trigger_instant: Some(record.trigger_instant.clone()),
                normal_delivery_until: Some("2026-10-23T01:01:00Z".into()),
                definition_digest: Some(record.definition_digest.clone()),
                providers: BTreeMap::new(),
                diagnostics: vec![],
            },
        };
        assert!(
            !missed(
                &candidate,
                &ExactInstant::parse("2026-10-23T01:00:59.999999999999Z").unwrap()
            )
            .unwrap()
        );
        assert!(
            missed(
                &candidate,
                &ExactInstant::parse("2026-10-23T01:01:00Z").unwrap()
            )
            .unwrap()
        );
    }
    #[test]
    fn settings_retry_retains_original_mutation_and_rejects_id_reuse_with_changed_input() {
        let (mut state, _) = fixture();
        let id = "94a02f40-e942-4615-8f04-6037e2d9f97b";
        let fingerprint = "e".repeat(64);
        finish_settings(&mut state, id, fingerprint.clone()).unwrap();
        assert!(settings_retry(&state, 0, id, &fingerprint).unwrap());
        assert_eq!(
            settings_retry(&state, 0, id, &"f".repeat(64))
                .unwrap_err()
                .code,
            "operationConflict"
        );
        assert_eq!(
            settings_retry(
                &state,
                0,
                "d93a76e5-bbf1-452d-8367-cd613aa52e1c",
                &fingerprint
            )
            .unwrap_err()
            .code,
            "staleBase"
        );
    }
    #[test]
    fn permit_rejects_clock_rollback_and_monotonic_expiration() {
        let (state, key) = fixture();
        let attempt = lookup_attempt(&state, &key).unwrap();
        assert_eq!(attempt.expires_at, "2026-10-23T01:00:18.123456789123Z");
        let mut permit = permit(&key, attempt);
        assert!(
            permit_age(
                &permit,
                attempt,
                &ExactInstant::parse("2026-10-23T01:00:14Z").unwrap()
            )
            .is_ok()
        );
        assert_eq!(
            permit_age(
                &permit,
                attempt,
                &ExactInstant::parse("2026-10-23T01:00:12Z").unwrap()
            )
            .unwrap_err()
            .code,
            "reminderPermitExpired"
        );
        assert_eq!(
            permit_age(
                &permit,
                attempt,
                &ExactInstant::parse(&attempt.expires_at).unwrap()
            )
            .unwrap_err()
            .code,
            "reminderPermitExpired"
        );
        permit.issued = std::time::Instant::now() - std::time::Duration::from_secs(6);
        assert_eq!(
            permit_age(
                &permit,
                attempt,
                &ExactInstant::parse("2026-10-23T01:00:14Z").unwrap()
            )
            .unwrap_err()
            .code,
            "reminderPermitExpired"
        );
    }
}
#[cfg(all(test, windows))]
#[path = "reminder_device_state_tests.rs"]
mod integration_tests;
