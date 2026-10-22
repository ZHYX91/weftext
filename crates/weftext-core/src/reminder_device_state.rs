//! Device-local task-reminder decisions. Saved author data remains the only
//! task authority; a notification is a distinct external effect, never a save.
use super::*;
use crate::device_state::DeviceRecordKind;
use crate::operations::{SnapshotBinding, read_blob};
use crate::reminder_schedule::{
    InputBudget, MAX_INPUT_BYTES, MAX_INPUT_PATHS, cooling_down, summary_deadline, validate_window,
};
use std::{
    collections::{BTreeMap, BTreeSet},
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
const STATE_VERSION: u32 = 2;
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
    FailedBeforeCall { description: CoreDescription },
    Unknown { description: CoreDescription },
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
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum Delivery {
    Individual {
        attempt: Attempt,
    },
    SummaryMember {
        summary_key: String,
        prior_failed_attempt: Option<Attempt>,
    },
    SummaryOverflow {
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
    cutoff: String,
    next_allowed_at: String,
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
    libraries: BTreeMap<String, LibrarySelection>,
    subscriptions: BTreeMap<String, ReminderSubscription>,
    records: BTreeMap<String, Record>,
    summaries: BTreeMap<String, Summary>,
    last_settings_mutation: Option<SettingsMutation>,
}
fn err(message: UiMessage) -> CoreError {
    CoreError::new("reminderState", message)
}
fn temporal_error(error: weftext_data::DataError) -> CoreError {
    err(UiMessage::error_reminder_temporal_value_invalid())
        .caused_by(CoreError::from_data("data", error))
}
fn resolution_error(
    code: &str,
    explanation: UiMessage,
    resolution: &TaskReminderResolution,
) -> CoreError {
    match resolution.failure_kind {
        Some(kind) => CoreError::from_data(
            code,
            weftext_data::DataError::with_kind(kind, explanation)
                .with_diagnostics(resolution.diagnostics.clone()),
        ),
        None => CoreError::from_product(
            code,
            None,
            explanation.into(),
            resolution.diagnostics.clone(),
        ),
    }
}
fn field_error(code: &str, explanation: UiMessage, cell: Option<&Cell>) -> CoreError {
    match cell {
        Some(Cell::Invalid { diagnostics, .. }) => CoreError::from_data(
            code,
            weftext_data::DataError::new(explanation).with_diagnostics(diagnostics.clone()),
        ),
        Some(cell @ Cell::Unavailable { kind, .. }) => CoreError::from_data(
            code,
            weftext_data::DataError::with_kind(*kind, explanation)
                .with_diagnostics(cell.error_diagnostics()),
        ),
        _ => CoreError::new(code, explanation),
    }
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
        .map_err(|error| {
            err(UiMessage::error_reminder_clock_before_epoch())
                .diagnostic(DiagnosticOrigin::Dependency, error.to_string())
        })?;
    let utc_seconds = i64::try_from(elapsed.as_secs()).map_err(|error| {
        err(UiMessage::error_reminder_clock_overflow())
            .diagnostic(DiagnosticOrigin::Dependency, error.to_string())
    })?;
    Ok(ExactInstant {
        utc_seconds,
        fraction: ExactDecimal::parse(&format!("0.{:09}", elapsed.subsec_nanos()))
            .map_err(temporal_error)?,
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
        return Err(err(
            UiMessage::error_reminder_original_notification_invalid(),
        ));
    }
    for instant in [&attempt.attempt_at, &attempt.expires_at] {
        ExactInstant::parse(instant).map_err(temporal_error)?;
    }
    match &attempt.delivery {
        ReminderDeliveryState::SubmittedToSystem { at } => {
            ExactInstant::parse(at).map_err(temporal_error)?;
        }
        ReminderDeliveryState::FailedBeforeCall { description }
        | ReminderDeliveryState::Unknown { description }
            if serde_json::to_vec(description)?.len() > 16 * 1024 =>
        {
            return Err(err(UiMessage::error_reminder_delivery_description_budget()));
        }
        _ => {}
    }
    Ok(())
}
fn validate_prior_failure(prior: &Option<Attempt>, key: &str) -> Result<()> {
    if let Some(attempt) = prior {
        validate_attempt(attempt, key)?;
        if !matches!(
            attempt.delivery,
            ReminderDeliveryState::FailedBeforeCall { .. }
        ) {
            return Err(err(
                UiMessage::error_reminder_summary_requires_pre_call_failure(),
            ));
        }
    }
    Ok(())
}
impl State {
    fn new(device: String) -> Self {
        Self {
            schema_version: STATE_VERSION,
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
        if self.schema_version != STATE_VERSION {
            return Err(CoreError::new(
                "unsupportedVersion",
                UiMessage::error_reminder_state_version_unsupported(),
            ));
        }
        if !uuid(&self.device_id)
            || self.settings_revision > self.revision
            || self.libraries.len() > MAX_LIBRARIES
            || self.subscriptions.len() > MAX_RECORDS
            || self.records.len() + self.summaries.len() > MAX_RECORDS
        {
            return Err(err(UiMessage::error_reminder_state_invalid()));
        }
        for (id, library) in &self.libraries {
            if !uuid(id) || !Path::new(&library.root).is_absolute() || library.root.len() > 32_768 {
                return Err(err(UiMessage::error_reminder_library_selection_invalid()));
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
                return Err(err(UiMessage::error_reminder_subscription_invalid()));
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
                return Err(err(UiMessage::error_reminder_original_effect_invalid()));
            }
            ExactInstant::parse(&record.trigger_instant).map_err(temporal_error)?;
            match &record.delivery {
                Delivery::Individual { attempt } => {
                    validate_attempt(attempt, key)?;
                    if !activation_keys.insert(&attempt.request.activation_key) {
                        return Err(err(UiMessage::error_reminder_activation_duplicate()));
                    }
                }
                Delivery::SummaryMember {
                    summary_key,
                    prior_failed_attempt,
                } => {
                    if !self
                        .summaries
                        .get(summary_key)
                        .is_some_and(|summary| summary.members.contains(key))
                    {
                        return Err(err(UiMessage::error_reminder_original_summary_missing()));
                    }
                    validate_prior_failure(prior_failed_attempt, key)?;
                }
                Delivery::SummaryOverflow {
                    summary_key,
                    prior_failed_attempt,
                } => {
                    if !self.summaries.get(summary_key).is_some_and(|summary| {
                        summary.members.len() == MAX_SUMMARY_MEMBERS
                            && summary.members.last().is_some_and(|last| key > last)
                    }) {
                        return Err(err(UiMessage::error_reminder_summary_overflow_invalid()));
                    }
                    validate_prior_failure(prior_failed_attempt, key)?;
                }
            }
            if serde_json::to_vec(record)?.len() > 16 * 1024 {
                return Err(err(UiMessage::error_reminder_record_budget()));
            }
        }
        let mut windows = vec![];
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
                return Err(err(UiMessage::error_reminder_summary_members_invalid()));
            }
            for member in &summary.members {
                let linked=self.records.get(member).is_some_and(|record|matches!(&record.delivery,Delivery::SummaryMember{summary_key,..}if summary_key==key));
                if !linked {
                    return Err(err(UiMessage::error_reminder_summary_effect_mismatch()));
                }
            }
            let window = validate_window(&summary.cutoff, &summary.next_allowed_at)?;
            let attempt_at =
                ExactInstant::parse(&summary.attempt.attempt_at).map_err(temporal_error)?;
            if attempt_at.compare(&window.0).is_lt()
                || summary.attempt.attempt_count == 1 && !attempt_at.compare(&window.0).is_eq()
            {
                return Err(err(UiMessage::error_reminder_summary_cooldown_invalid()));
            }
            windows.push(window);
            validate_attempt(&summary.attempt, key)?;
            if !activation_keys.insert(&summary.attempt.request.activation_key) {
                return Err(err(UiMessage::error_reminder_summary_activation_duplicate()));
            }
        }
        windows.sort_by(|a, b| a.0.compare(&b.0));
        if windows
            .windows(2)
            .any(|pair| pair[1].0.compare(&pair[0].1).is_lt())
        {
            return Err(err(UiMessage::error_reminder_summary_cooldown_invalid()));
        }
        if self.last_settings_mutation.as_ref().is_some_and(|m| {
            !uuid(&m.id) || !digest(&m.fingerprint) || m.revision > self.settings_revision
        }) {
            return Err(err(UiMessage::error_reminder_settings_mutation_invalid()));
        }
        Ok(())
    }
}
fn summary_cooling_down(state: &State, at: &ExactInstant) -> Result<Option<String>> {
    // An original retry does not move the batch cutoff, but its later trusted
    // attempt time also cannot be rolled back to unlock another admission.
    for summary in state.summaries.values() {
        if at
            .compare(&ExactInstant::parse(&summary.attempt.attempt_at).map_err(temporal_error)?)
            .is_lt()
        {
            return Err(CoreError::new(
                "reminderClockRollback",
                UiMessage::error_reminder_summary_clock_rollback(),
            ));
        }
    }
    cooling_down(
        at,
        state
            .summaries
            .values()
            .map(|summary| (summary.cutoff.as_str(), summary.next_allowed_at.as_str())),
    )
}
fn validate_summary_capacity(state: &State) -> Result<()> {
    if state
        .records
        .len()
        .checked_add(state.summaries.len())
        .is_none_or(|count| count > MAX_RECORDS)
        || serde_json::to_vec(state)?.len() > MAX_BYTES
    {
        return Err(CoreError::new(
            "budget",
            UiMessage::error_reminder_summary_prepare_capacity(),
        ));
    }
    Ok(())
}
/// Scope-first backup classification calls this on the one known device leaf.
/// Foreign records remain valid originals, but never become current-device consent.
pub(crate) fn validate_reminder_device_record(bytes: &[u8]) -> Result<()> {
    decode_reminder_record(bytes).map(|_| ())
}
fn decode_reminder_record(bytes: &[u8]) -> Result<State> {
    if bytes.len() > MAX_BYTES {
        return Err(err(UiMessage::error_reminder_state_byte_budget()));
    }
    let raw = weftext_data::parse_unique_json(bytes)
        .map_err(|error| CoreError::from_data("reminderState", error))?;
    if raw["schemaVersion"]
        .as_u64()
        .is_some_and(|version| version != u64::from(STATE_VERSION))
    {
        return Err(CoreError::new(
            "unsupportedVersion",
            UiMessage::error_reminder_state_version_unsupported(),
        ));
    }
    let state: State = serde_json::from_value(raw.clone())?;
    // FileIdentity is an existing common type. Reject unknown fields in this
    // new format rather than inheriting that type's historical permissiveness.
    for library in raw["libraries"]
        .as_object()
        .ok_or_else(|| err(UiMessage::error_reminder_libraries_missing()))?
        .values()
    {
        let identity = library["identity"]
            .as_object()
            .ok_or_else(|| err(UiMessage::error_reminder_library_identity_missing()))?;
        if identity.keys().map(String::as_str).collect::<BTreeSet<_>>()
            != BTreeSet::from(["volume", "index", "created"])
        {
            return Err(err(
                UiMessage::error_reminder_library_identity_fields_invalid(),
            ));
        }
    }
    state.validate()?;
    Ok(state)
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
/// Observed record outcome. Application-only overflow is not an OS attempt;
/// a durable submitting barrier is reported as unknown until resolved.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ReminderRecordedDelivery {
    SubmittedToSystem { at: String },
    FailedBeforeCall { description: CoreDescription },
    Unknown { description: CoreDescription },
    InAppOnly { summary_key: String },
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReminderDeliveryStatus {
    pub execution_key: String,
    pub delivery: ReminderRecordedDelivery,
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
/// Trusted full-scan result; a permit is never returned for an incomplete
/// budget-limited prefix. Scope issues retain their actual typed causes.
#[derive(Debug)]
pub enum ReminderSummaryPreparation {
    Started {
        permit: Box<ReminderDeliveryPermit>,
        member_count: usize,
        overflow_count: usize,
        /// At most 32 whole original errors; never truncated causes or facts.
        issues: Vec<ReminderScheduleIssue>,
        /// Exact failures from the full consent traversal, including samples
        /// omitted from `issues`. Any nonzero count means restricted scopes.
        issue_count: usize,
        /// Whether every failure is present in `issues`. This describes the
        /// diagnostic list, not whether every author scope could be qualified.
        issues_complete: bool,
    },
    NoNewMissed {
        issues: Vec<ReminderScheduleIssue>,
        issue_count: usize,
        issues_complete: bool,
    },
    CoolingDown {
        until: String,
    },
}
#[derive(Debug, Clone)]
pub enum ReminderSubmissionOutcome {
    SubmittedToSystem,
    FailedBeforeCall(CoreDescription),
    Unknown(CoreDescription),
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
    #[serde(flatten)]
    pub error: CoreError,
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
                UiMessage::error_reminder_foreign_device(),
            ));
        }
        match self.read_device_record(DeviceRecordKind::Reminders, MAX_BYTES)? {
            Some((bytes, digest)) => {
                let state = decode_reminder_record(&bytes)?;
                if state.device_id != identity.id {
                    return Err(err(UiMessage::error_reminder_device_mismatch()));
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
            .ok_or_else(|| err(UiMessage::error_reminder_revision_overflow()))?;
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
            return Err(err(UiMessage::error_reminder_record_cursor_invalid()));
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
            .ok_or_else(|| err(UiMessage::error_reminder_original_record_missing()))?;
        let library = state
            .libraries
            .get(&record.owner.workspace_id)
            .ok_or_else(|| err(UiMessage::error_reminder_original_library_missing()))?;
        let root = Path::new(&library.root);
        let root_guard = ParentGuard::acquire(&root.join(".reminder-open-root"))?;
        if root_guard.directory_identity()? != library.identity {
            return Err(CoreError::new(
                "foreignLibraryState",
                UiMessage::error_reminder_original_library_root_changed(),
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
                UiMessage::error_reminder_open_target_identity_changed(),
            ));
        }
        if root_guard.directory_identity()? != library.identity {
            self.close_session(&document.session_id)?;
            return Err(CoreError::new(
                "foreignLibraryState",
                UiMessage::error_reminder_open_library_changed(),
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
                .ok_or_else(|| err(UiMessage::error_reminder_reminder_id_missing()))?,
            resolution
                .definition_digest
                .as_deref()
                .ok_or_else(|| err(UiMessage::error_reminder_definition_missing()))?,
            resolution
                .trigger_instant
                .as_deref()
                .ok_or_else(|| err(UiMessage::error_reminder_trigger_missing()))?,
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
        let trigger = ExactInstant::parse(trigger).map_err(temporal_error)?;
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
                UiMessage::error_reminder_selection_changed(),
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
                    CoreError::new(
                        "reminderUnavailable",
                        UiMessage::error_reminder_schedule_target_changed(),
                    ),
                )),
                Err(error) => issues.push(issue(subscription, error)),
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
                UiMessage::error_reminder_live_task_unavailable(),
            ));
        }
        // Navigation deliberately returns the Node directory. Resolve its
        // canonical source through the existing physical admission boundary;
        // the directory is never passed to a source-file API.
        let directory = target
            .path
            .ok_or_else(|| err(UiMessage::error_reminder_task_location_unavailable()))?;
        let source = self.managed_node(root, Path::new(&directory), false)?;
        let path = PathBuf::from(source.path);
        if self.qualified_node_at(root, &path)?.owner != *owner {
            return Err(CoreError::new(
                "identityConflict",
                UiMessage::error_reminder_task_location_changed(),
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
            return Err(err(UiMessage::error_reminder_task_reference_invalid()));
        }
        let root = absolute_path(root)?;
        let workspace = self.require_workspace(&root)?;
        if workspace.workspace_id != owner.workspace_id {
            return Err(err(UiMessage::error_reminder_task_library_mismatch()));
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
        .map_err(|error| CoreError::from_data("invalidSchema", error))?;
        Self::merge_feature_witnesses(&mut witnesses, vec![config_witness])?;
        let source = witnesses
            .iter()
            .find(|w| Path::new(&w.binding.path) == path)
            .and_then(|w| w.binding.captured.as_ref())
            .ok_or_else(|| err(UiMessage::error_reminder_qualified_task_source_missing()))?;
        let projection = weftext_data::project(&decode(&source.bytes)?, &schema);
        if projection.qualification != "qualified"
            || projection.id.as_deref() != Some(owner.node_id.as_str())
            || !projection.resolved_types.iter().any(|t| t == "tasks/task")
        {
            return Err(CoreError::from_product(
                "reminderUnavailable",
                None,
                UiMessage::error_reminder_saved_object_not_task().into(),
                projection.diagnostics,
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
                _ => return Err(err(UiMessage::error_reminder_saved_task_status_invalid())),
            },
            _ => {
                return Err(field_error(
                    "reminderUnavailable",
                    UiMessage::error_reminder_saved_task_status_unavailable(),
                    cell("tasks/status"),
                ));
            }
        };
        let value = |id: &str| -> Result<Option<&weftext_data::NativeValue>> {
            match cell(id) {
                Some(Cell::Value { value }) => Ok(Some(value)),
                Some(Cell::Absent) => Ok(None),
                original => Err(field_error(
                    "reminderUnavailable",
                    UiMessage::error_reminder_saved_field_unavailable(
                        weftext_messages::params::ErrorReminderSavedFieldUnavailable {
                            field: id.into(),
                        },
                    ),
                    original,
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
        let session = self.sessions.get(session_id).ok_or_else(|| {
            CoreError::new(
                "sessionMissing",
                UiMessage::error_reminder_session_unavailable(),
            )
        })?;
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
                            UiMessage::error_reminder_dependency_conflict(),
                        ));
                    }
                    continue;
                }
                seen.insert(basis.path.clone(), basis);
                if seen.len() > MAX_INPUT_PATHS {
                    return Err(CoreError::new(
                        "budget",
                        UiMessage::error_reminder_input_guard_count_budget(),
                    ));
                }
                let path = Path::new(&basis.path);
                if crate::history::history_relative_path(path, root).is_none() {
                    return Err(err(UiMessage::error_reminder_dependency_outside_library()));
                }
                self.check_pending(path)?;
                let guard = ParentGuard::acquire(path)?;
                if guard.directory_identity()? != basis.parent {
                    return Err(CoreError::new(
                        "staleBase",
                        UiMessage::error_reminder_input_parent_changed(),
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
                    .ok_or_else(|| err(UiMessage::error_reminder_input_byte_overflow()))?;
                if total > MAX_INPUT_BYTES || seen.len() > MAX_INPUT_PATHS {
                    return Err(CoreError::new(
                        "budget",
                        UiMessage::error_reminder_input_guard_byte_budget(),
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
                        UiMessage::error_reminder_input_changed_before_decision(),
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
                    UiMessage::error_reminder_input_changed_during_decision(),
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
                UiMessage::error_reminder_selected_root_mismatch(),
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
                UiMessage::error_reminder_library_changed_during_selection(),
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
            .ok_or_else(|| err(UiMessage::error_reminder_library_selection_missing()))?;
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
            return Err(err(
                UiMessage::error_reminder_subscription_identity_invalid(),
            ));
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
                .ok_or_else(|| err(UiMessage::error_reminder_subscription_missing()))?;
            old.enabled = false;
            finish_settings(&mut state, mutation_id, fp)?;
            let state = self.write_reminders(state, digest.as_deref())?;
            return Ok(settings(&state));
        }
        let (candidate, witnesses) = self.capture_reminder_task(root, owner)?;
        if candidate.resolution.state != TaskReminderState::Ready
            || candidate.resolution.reminder_id.as_deref() != Some(reminder_id)
        {
            return Err(resolution_error(
                "reminderUnavailable",
                UiMessage::error_reminder_save_before_enable(),
                &candidate.resolution,
            ));
        }
        let library = state
            .libraries
            .get(&owner.workspace_id)
            .ok_or_else(|| err(UiMessage::error_reminder_select_library_first()))?;
        if library.identity != candidate.root_identity || !library.enabled {
            return Err(err(
                UiMessage::error_reminder_selected_library_disabled_or_changed(),
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
        let instant = ExactInstant::parse(trigger).map_err(temporal_error)?;
        if at.compare(&instant).is_lt() {
            return Err(CoreError::new(
                "reminderNotDue",
                UiMessage::error_reminder_not_due(),
            ));
        }
        if missed(&candidate, &at)? {
            return Err(CoreError::new(
                "reminderMissed",
                UiMessage::error_reminder_requires_missed_summary(),
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
                        .ok_or_else(|| err(UiMessage::error_reminder_attempt_counter_overflow()))?
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
    /// Prepare one complete, guarded missed set from current saved sources.
    /// Callers cannot offer a truncated member prefix or a client-owned clock.
    pub fn prepare_reminder_summary(
        &mut self,
        options: ReminderNotificationOptions,
    ) -> Result<ReminderSummaryPreparation> {
        self.prepare_reminder_summary_with_clock(options, now)
    }
    fn prepare_reminder_summary_with_clock(
        &mut self,
        options: ReminderNotificationOptions,
        mut clock: impl FnMut() -> Result<ExactInstant>,
    ) -> Result<ReminderSummaryPreparation> {
        let (mut state, digest) = self.read_reminders()?;
        let scan_at = clock()?;
        if let Some(until) = summary_cooling_down(&state, &scan_at)? {
            return Ok(ReminderSummaryPreparation::CoolingDown { until });
        }
        let mut captured = BTreeMap::new();
        let mut budget = InputBudget::default();
        let mut issues = vec![];
        let mut issue_count = 0usize;
        for subscription in state
            .subscriptions
            .values()
            .filter(|subscription| subscription.enabled)
        {
            let Some(library) = state
                .libraries
                .get(&subscription.owner.workspace_id)
                .filter(|library| library.enabled)
            else {
                continue;
            };
            let qualified = (|| {
                let (candidate, witnesses) =
                    self.capture_reminder_task(Path::new(&library.root), &subscription.owner)?;
                if candidate.root_identity != library.identity {
                    return Err(CoreError::new(
                        "foreignLibraryState",
                        UiMessage::error_reminder_library_identity_changed(),
                    ));
                }
                if !candidate.task_open {
                    return Ok(None);
                }
                check_enabled(&state, &candidate)?;
                let (id, definition, trigger) = ready(&candidate)?;
                if id != subscription.reminder_id {
                    return Err(CoreError::new(
                        "reminderUnavailable",
                        UiMessage::error_reminder_schedule_target_changed(),
                    ));
                }
                if !missed(&candidate, &scan_at)? {
                    return Ok(None);
                }
                let key =
                    execution_key(&state.device_id, &candidate.owner, id, definition, trigger)?;
                let prior_failed_attempt = match state.records.get(&key) {
                    None => None,
                    Some(record) if !record.handled && record.invalidated.is_none() => {
                        match &record.delivery {
                            // A definitely uncalled normal attempt may enter its
                            // first missed phase once. Summary retries use only
                            // retry_reminder_summary's fixed original key.
                            Delivery::Individual { attempt }
                                if matches!(
                                    attempt.delivery,
                                    ReminderDeliveryState::FailedBeforeCall { .. }
                                ) =>
                            {
                                Some(attempt.clone())
                            }
                            _ => return Ok(None),
                        }
                    }
                    Some(_) => return Ok(None),
                };
                Ok(Some((key, candidate, witnesses, prior_failed_attempt)))
            })();
            match qualified {
                Err(error) => {
                    // A task that could not be qualified receives no record
                    // or watermark. Its complete typed failure stays scoped.
                    // The cumulative budget of qualified candidates below is
                    // different: crossing it aborts the entire admission.
                    issue_count += 1;
                    if issues.len() < 32 {
                        issues.push(issue(subscription, error));
                    }
                }
                Ok(None) => {}
                Ok(Some((key, candidate, witnesses, prior_failed_attempt))) => {
                    // This is a whole-batch limit, not a pagination limit.
                    // A budget error cannot admit the prefix gathered so far.
                    for witness in &witnesses {
                        budget.include(
                            &witness.binding.path,
                            witness
                                .binding
                                .captured
                                .as_ref()
                                .map_or(0, |file| file.bytes.len()),
                        )?;
                    }
                    if captured
                        .insert(key, (candidate, witnesses, prior_failed_attempt))
                        .is_some()
                    {
                        return Err(err(UiMessage::error_reminder_summary_member_duplicate()));
                    }
                }
            }
        }
        if captured.is_empty() {
            let issues_complete = issue_count == issues.len();
            return Ok(ReminderSummaryPreparation::NoNewMissed {
                issues,
                issue_count,
                issues_complete,
            });
        }
        let checked_at = clock()?;
        if checked_at.compare(&scan_at).is_lt() {
            return Err(CoreError::new(
                "reminderClockRollback",
                UiMessage::error_reminder_summary_clock_rollback(),
            ));
        }
        let members = captured
            .keys()
            .take(MAX_SUMMARY_MEMBERS)
            .cloned()
            .collect::<Vec<_>>();
        let key = hash(&serde_json::to_vec(&(
            "weftext.reminder-summary.v1",
            &state.device_id,
            &members,
        ))?);
        if state.summaries.contains_key(&key) {
            return Err(err(UiMessage::error_reminder_summary_members_invalid()));
        }
        let member_count = members.len();
        let overflow_count = captured.len() - member_count;
        let groups = captured
            .values()
            .map(|(candidate, witnesses, _)| (Path::new(&candidate.root), witnesses.as_slice()))
            .collect::<Vec<_>>();
        let permit = self.with_reminder_input_groups(&groups, |core| {
            // Full scan and all input guards precede the short-lived permit.
            // A large qualified set must not consume its permit while scanning.
            let admitted_at = clock()?;
            if admitted_at.compare(&checked_at).is_lt() {
                return Err(CoreError::new(
                    "reminderClockRollback",
                    UiMessage::error_reminder_summary_clock_rollback(),
                ));
            }
            let cutoff = admitted_at.canonical().map_err(temporal_error)?;
            let next_allowed_at = summary_deadline(&admitted_at)?
                .canonical()
                .map_err(temporal_error)?;
            let attempt = make_attempt(&key, options, &admitted_at, 1)?;
            let permit = permit(&key, &attempt);
            for (index, (execution, (candidate, _, prior_failed_attempt))) in
                captured.iter().enumerate()
            {
                let (id, definition, trigger) = ready(candidate)?;
                state.records.insert(
                    execution.clone(),
                    Record {
                        owner: candidate.owner.clone(),
                        reminder_id: id.into(),
                        definition_digest: definition.into(),
                        trigger_instant: trigger.into(),
                        delivery: if index < member_count {
                            Delivery::SummaryMember {
                                summary_key: key.clone(),
                                prior_failed_attempt: prior_failed_attempt.clone(),
                            }
                        } else {
                            Delivery::SummaryOverflow {
                                summary_key: key.clone(),
                                prior_failed_attempt: prior_failed_attempt.clone(),
                            }
                        },
                        handled: false,
                        invalidated: None,
                    },
                );
            }
            state.summaries.insert(
                key.clone(),
                Summary {
                    members,
                    attempt,
                    cutoff,
                    next_allowed_at,
                },
            );
            validate_summary_capacity(&state)?;
            permit_age(&permit, &state.summaries[&key].attempt, &clock()?)?;
            core.write_reminders(state, digest.as_deref())?;
            Ok(permit)
        })?;
        let issues_complete = issue_count == issues.len();
        Ok(ReminderSummaryPreparation::Started {
            permit: Box::new(permit),
            member_count,
            overflow_count,
            issues,
            issue_count,
            issues_complete,
        })
    }
    /// Only a proven pre-call summary failure may obtain another permit. Its
    /// original members, OS request, cutoff and cooldown remain fixed.
    pub fn retry_reminder_summary(&mut self, key: &str) -> Result<ReminderDeliveryStart> {
        self.retry_reminder_summary_with_clock(key, now)
    }
    fn retry_reminder_summary_with_clock(
        &mut self,
        key: &str,
        mut clock: impl FnMut() -> Result<ExactInstant>,
    ) -> Result<ReminderDeliveryStart> {
        let (mut state, digest) = self.read_reminders()?;
        let original = state
            .summaries
            .get(key)
            .ok_or_else(|| err(UiMessage::error_reminder_original_summary_missing()))?
            .clone();
        if !matches!(
            original.attempt.delivery,
            ReminderDeliveryState::FailedBeforeCall { .. }
        ) {
            return Ok(ReminderDeliveryStart::Retained(status(&state, key)?));
        }
        let at = clock()?;
        // A retry cannot use a corrected clock to precede its own first
        // admission. It consumes neither new members nor another batch slot.
        let _ = summary_cooling_down(&state, &at)?;
        if at
            .compare(&ExactInstant::parse(&original.cutoff).map_err(temporal_error)?)
            .is_lt()
        {
            return Err(CoreError::new(
                "reminderClockRollback",
                UiMessage::error_reminder_summary_clock_rollback(),
            ));
        }
        let mut captured = vec![];
        for member in &original.members {
            let record = &state.records[member];
            if record.handled || record.invalidated.is_some() {
                return Err(err(UiMessage::error_reminder_summary_member_handled()));
            }
            let library = &state.libraries[&record.owner.workspace_id];
            let (candidate, witnesses) =
                self.capture_reminder_task(Path::new(&library.root), &record.owner)?;
            check_enabled(&state, &candidate)?;
            let (id, definition, trigger) = ready(&candidate)?;
            if execution_key(&state.device_id, &candidate.owner, id, definition, trigger)?
                != *member
            {
                return Err(CoreError::new(
                    "staleBase",
                    UiMessage::error_reminder_definition_changed(),
                ));
            }
            if !missed(&candidate, &at)? {
                return Err(CoreError::new(
                    "reminderNotMissed",
                    UiMessage::error_reminder_summary_member_not_missed(),
                ));
            }
            captured.push((candidate, witnesses));
        }
        let checked_at = clock()?;
        if checked_at.compare(&at).is_lt() {
            return Err(CoreError::new(
                "reminderClockRollback",
                UiMessage::error_reminder_summary_clock_rollback(),
            ));
        }
        let groups = captured
            .iter()
            .map(|(candidate, witnesses)| (Path::new(&candidate.root), witnesses.as_slice()))
            .collect::<Vec<_>>();
        let permit = self.with_reminder_input_groups(&groups, |core| {
            let admitted_at = clock()?;
            if admitted_at.compare(&checked_at).is_lt() {
                return Err(CoreError::new(
                    "reminderClockRollback",
                    UiMessage::error_reminder_summary_clock_rollback(),
                ));
            }
            let mut attempt = original.attempt;
            attempt.permit_id = fresh();
            attempt.attempt_count = attempt
                .attempt_count
                .checked_add(1)
                .ok_or_else(|| err(UiMessage::error_reminder_summary_attempt_counter_overflow()))?;
            attempt.attempt_at = admitted_at.canonical().map_err(temporal_error)?;
            attempt.expires_at = admitted_at
                .shifted_seconds(5)
                .and_then(|instant| instant.canonical())
                .map_err(temporal_error)?;
            attempt.delivery = ReminderDeliveryState::Submitting;
            let permit = permit(key, &attempt);
            state.summaries.get_mut(key).unwrap().attempt = attempt;
            permit_age(&permit, &state.summaries[key].attempt, &clock()?)?;
            core.write_reminders(state, digest.as_deref())?;
            Ok(permit)
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
                UiMessage::error_reminder_permit_already_admitted(),
            ));
        }
        let (state, _) = self.read_reminders()?;
        let attempt = lookup_attempt(&state, &permit.key)?;
        if attempt.permit_id != permit.permit_id
            || attempt.request != permit.request
            || !matches!(attempt.delivery, ReminderDeliveryState::Submitting)
        {
            return Err(err(UiMessage::error_reminder_permit_changed()));
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
                return Err(err(UiMessage::error_reminder_notification_member_handled()));
            }
            let library = state
                .libraries
                .get(&record.owner.workspace_id)
                .ok_or_else(|| err(UiMessage::error_reminder_original_selection_missing()))?;
            let (candidate, witnesses) =
                self.capture_reminder_task(Path::new(&library.root), &record.owner)?;
            check_enabled(&state, &candidate)?;
            if !summary && missed(&candidate, &clock()?)? {
                return Err(CoreError::new(
                    "reminderMissed",
                    UiMessage::error_reminder_minute_ended_before_call(),
                ));
            }
            let (id, definition, trigger) = ready(&candidate)?;
            if execution_key(&state.device_id, &candidate.owner, id, definition, trigger)? != key {
                return Err(CoreError::new(
                    "staleBase",
                    UiMessage::error_reminder_definition_changed(),
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
                    UiMessage::error_reminder_minute_ended_before_call(),
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
                    UiMessage::error_reminder_external_permit_already_admitted(),
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
                UiMessage::error_reminder_delivery_result_mismatch(),
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
                    at: now()?.canonical().map_err(temporal_error)?,
                }
            }
            ReminderSubmissionOutcome::FailedBeforeCall(description) => {
                if !matches!(
                    attempt.delivery,
                    ReminderDeliveryState::Submitting
                        | ReminderDeliveryState::FailedBeforeCall { .. }
                ) {
                    return Err(err(
                        UiMessage::error_reminder_unknown_cannot_become_pre_call_failure(),
                    ));
                }
                ReminderDeliveryState::FailedBeforeCall { description }
            }
            ReminderSubmissionOutcome::Unknown(description) => {
                ReminderDeliveryState::Unknown { description }
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
            return Err(err(UiMessage::error_reminder_notification_cursor_invalid()));
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
        original_notification(&state, key)?;
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
                .ok_or_else(|| err(UiMessage::error_reminder_original_member_missing()))?
                .clone();
            if record.handled || record.invalidated.is_some() {
                eligible = false;
                continue;
            }
            let Some(library) = state.libraries.get(&record.owner.workspace_id).cloned() else {
                return Err(err(
                    UiMessage::error_reminder_original_member_library_missing(),
                ));
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
                            error,
                        });
                        None
                    }
                    Ok((candidate, witnesses)) => {
                        let reason = if candidate.root_identity != library.identity {
                            eligible = false;
                            issues.push(ReminderScheduleIssue {
                                owner: record.owner.clone(),
                                reminder_id: record.reminder_id.clone(),
                                error: CoreError::new(
                                    "foreignLibraryState",
                                    UiMessage::error_reminder_library_identity_changed(),
                                ),
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
                                error: resolution_error(
                                    "reminderUnavailable",
                                    UiMessage::error_reminder_saved_trigger_unavailable(),
                                    &candidate.resolution,
                                ),
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
            self.with_reminder_input_groups(&groups, |core| {
                core.write_reminders(state, digest.as_deref())
            })?;
        }
        Ok(ReminderNotificationEligibility {
            original: ReminderOriginalNotification {
                execution_key: key.into(),
                request: attempt.request.clone(),
                delivery: observed_delivery(&attempt),
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
        original_notification(&state, key)?;
        let attempt = lookup_attempt_mut(&mut state, key)?;
        if &attempt.request != observed {
            return Err(CoreError::new(
                "operationConflict",
                UiMessage::error_reminder_os_history_mismatch(),
            ));
        }
        if matches!(
            attempt.delivery,
            ReminderDeliveryState::Submitting | ReminderDeliveryState::Unknown { .. }
        ) {
            attempt.delivery = ReminderDeliveryState::SubmittedToSystem {
                at: now()?.canonical().map_err(temporal_error)?,
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
            return Err(err(UiMessage::error_reminder_original_effect_missing()));
        };
        let mut changed = false;
        for member in members {
            let record = state
                .records
                .get_mut(&member)
                .ok_or_else(|| err(UiMessage::error_reminder_original_summary_member_missing()))?;
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
            return Err(err(UiMessage::error_reminder_activation_invalid()));
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
                UiMessage::error_reminder_activation_unavailable(),
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
                            error,
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
fn issue(subscription: &ReminderSubscription, error: CoreError) -> ReminderScheduleIssue {
    ReminderScheduleIssue {
        owner: subscription.owner.clone(),
        reminder_id: subscription.reminder_id.clone(),
        error,
    }
}
fn reminder_absent(path: &Path) -> Result<()> {
    match fs::symlink_metadata(path) {
        Ok(_) => Err(CoreError::new(
            "staleBase",
            UiMessage::error_reminder_default_configuration_occupied(),
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
            .ok_or_else(|| err(UiMessage::error_reminder_minute_boundary_missing()))?,
    )
    .map_err(temporal_error)?;
    Ok(!at.compare(&end).is_lt())
}
fn settings_retry(state: &State, expected: u64, id: &str, fingerprint: &str) -> Result<bool> {
    if !uuid(id) {
        return Err(err(UiMessage::error_reminder_mutation_id_invalid()));
    }
    if let Some(mutation) = &state.last_settings_mutation
        && mutation.id == id
    {
        if mutation.fingerprint != fingerprint {
            return Err(CoreError::new(
                "operationConflict",
                UiMessage::error_reminder_mutation_input_conflict(),
            ));
        }
        return Ok(true);
    }
    if state.settings_revision != expected {
        return Err(CoreError::new(
            "staleBase",
            UiMessage::error_reminder_settings_changed(),
        ));
    }
    Ok(false)
}
fn finish_settings(state: &mut State, id: &str, fingerprint: String) -> Result<()> {
    state.settings_revision = state
        .settings_revision
        .checked_add(1)
        .ok_or_else(|| err(UiMessage::error_reminder_settings_revision_overflow()))?;
    state.last_settings_mutation = Some(SettingsMutation {
        id: id.into(),
        fingerprint,
        revision: state.settings_revision,
    });
    Ok(())
}
fn ready(candidate: &TaskReminderCandidate) -> Result<(&str, &str, &str)> {
    if !candidate.task_open || candidate.resolution.state != TaskReminderState::Ready {
        return Err(resolution_error(
            "reminderUnavailable",
            UiMessage::error_reminder_task_not_eligible(),
            &candidate.resolution,
        ));
    }
    match (
        &candidate.resolution.reminder_id,
        &candidate.resolution.definition_digest,
        &candidate.resolution.trigger_instant,
    ) {
        (Some(id), Some(definition), Some(trigger)) => Ok((id, definition, trigger)),
        _ => Err(err(UiMessage::error_reminder_ready_definition_missing())),
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
            UiMessage::error_reminder_candidate_changed(),
        ));
    }
    Ok(())
}
fn check_enabled(state: &State, candidate: &TaskReminderCandidate) -> Result<()> {
    let (id, _, _) = ready(candidate)?;
    let library = state
        .libraries
        .get(&candidate.owner.workspace_id)
        .ok_or_else(|| err(UiMessage::error_reminder_library_not_selected()))?;
    if !library.enabled
        || library.identity != candidate.root_identity
        || !state
            .subscriptions
            .get(&subscription_key(&candidate.owner, id))
            .is_some_and(|s| s.enabled)
    {
        return Err(CoreError::new(
            "reminderDisabled",
            UiMessage::error_reminder_not_enabled(),
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
        attempt_at: at.canonical().map_err(temporal_error)?,
        expires_at: at
            .shifted_seconds(5)
            .and_then(|a| a.canonical())
            .map_err(temporal_error)?,
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
    let deadline = ExactInstant::parse(&permit.expires_at).map_err(temporal_error)?;
    let began = ExactInstant::parse(&attempt.attempt_at).map_err(temporal_error)?;
    if permit.issued.elapsed() >= std::time::Duration::from_secs(5)
        || current.compare(&began).is_lt()
        || !current.compare(&deadline).is_lt()
    {
        return Err(CoreError::new(
            "reminderPermitExpired",
            UiMessage::error_reminder_permit_expired(),
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
        .ok_or_else(|| err(UiMessage::error_reminder_original_effect_missing()))?
        .delivery
    {
        Delivery::Individual { attempt } => Ok(attempt),
        Delivery::SummaryMember { summary_key, .. } => state
            .summaries
            .get(summary_key)
            .map(|s| &s.attempt)
            .ok_or_else(|| err(UiMessage::error_reminder_original_summary_missing())),
        Delivery::SummaryOverflow { .. } => {
            Err(err(UiMessage::error_reminder_os_notification_missing()))
        }
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
        return Err(err(UiMessage::error_reminder_os_notification_missing()));
    }
    let attempt = lookup_attempt(state, key)?;
    Ok(ReminderOriginalNotification {
        execution_key: key.into(),
        request: attempt.request.clone(),
        delivery: observed_delivery(attempt),
    })
}
fn lookup_attempt_mut<'a>(state: &'a mut State, key: &str) -> Result<&'a mut Attempt> {
    if state.summaries.contains_key(key) {
        return Ok(&mut state.summaries.get_mut(key).unwrap().attempt);
    }
    let summary = match &state
        .records
        .get(key)
        .ok_or_else(|| err(UiMessage::error_reminder_original_effect_missing()))?
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
            .ok_or_else(|| err(UiMessage::error_reminder_original_summary_missing()));
    }
    match &mut state.records.get_mut(key).unwrap().delivery {
        Delivery::Individual { attempt } => Ok(attempt),
        Delivery::SummaryOverflow { .. } => {
            Err(err(UiMessage::error_reminder_os_notification_missing()))
        }
        Delivery::SummaryMember { .. } => {
            Err(err(UiMessage::error_reminder_original_summary_missing()))
        }
    }
}
fn observed_delivery(attempt: &Attempt) -> ReminderDeliveryState {
    match &attempt.delivery {
        ReminderDeliveryState::Submitting => ReminderDeliveryState::Unknown {
            description: UiMessage::status_reminder_external_submission_unknown().into(),
        },
        value => value.clone(),
    }
}
fn recorded_delivery(attempt: &Attempt) -> ReminderRecordedDelivery {
    match &attempt.delivery {
        ReminderDeliveryState::Submitting => ReminderRecordedDelivery::Unknown {
            description: UiMessage::status_reminder_external_submission_unknown().into(),
        },
        ReminderDeliveryState::SubmittedToSystem { at } => {
            ReminderRecordedDelivery::SubmittedToSystem { at: at.clone() }
        }
        ReminderDeliveryState::FailedBeforeCall { description } => {
            ReminderRecordedDelivery::FailedBeforeCall {
                description: description.clone(),
            }
        }
        ReminderDeliveryState::Unknown { description } => ReminderRecordedDelivery::Unknown {
            description: description.clone(),
        },
    }
}
fn status(state: &State, key: &str) -> Result<ReminderDeliveryStatus> {
    let delivery = match state.records.get(key).map(|record| &record.delivery) {
        Some(Delivery::SummaryOverflow { summary_key, .. }) => {
            ReminderRecordedDelivery::InAppOnly {
                summary_key: summary_key.clone(),
            }
        }
        _ => recorded_delivery(lookup_attempt(state, key)?),
    };
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
    fn summary_fixture(count: usize) -> (State, String, Vec<String>) {
        assert!(count > 0);
        let (mut state, key) = fixture();
        let template = state.records.remove(&key).unwrap();
        let at = ExactInstant::parse("2026-10-23T10:00:00.123456789123Z").unwrap();
        let mut records = BTreeMap::new();
        for _ in 0..count {
            let mut record = template.clone();
            record.owner.node_id = fresh();
            let key = execution_key(
                DEVICE,
                &record.owner,
                &record.reminder_id,
                &record.definition_digest,
                &record.trigger_instant,
            )
            .unwrap();
            records.insert(key, record);
        }
        let members = records
            .keys()
            .take(MAX_SUMMARY_MEMBERS)
            .cloned()
            .collect::<Vec<_>>();
        let key =
            hash(&serde_json::to_vec(&("weftext.reminder-summary.v1", DEVICE, &members)).unwrap());
        for (index, record) in records.values_mut().enumerate() {
            record.delivery = if index < members.len() {
                Delivery::SummaryMember {
                    summary_key: key.clone(),
                    prior_failed_attempt: None,
                }
            } else {
                Delivery::SummaryOverflow {
                    summary_key: key.clone(),
                    prior_failed_attempt: None,
                }
            };
        }
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
        let keys = records.keys().cloned().collect();
        state.records = records;
        state.summaries.insert(
            key.clone(),
            Summary {
                members,
                attempt,
                cutoff: at.canonical().unwrap(),
                next_allowed_at: summary_deadline(&at).unwrap().canonical().unwrap(),
            },
        );
        (state, key, keys)
    }
    #[test]
    fn overflow_is_a_strict_non_os_record_and_cannot_alias_an_attempt() {
        let (mut state, summary_key, keys) = summary_fixture(65);
        let overflow = &keys[64];
        let bytes = serde_json::to_vec(&state).unwrap();
        validate_reminder_device_record(&bytes).unwrap();
        let mut decoded = decode_reminder_record(&bytes).unwrap();
        assert!(matches!(
            status(&decoded, overflow).unwrap().delivery,
            ReminderRecordedDelivery::InAppOnly { .. }
        ));
        assert!(lookup_attempt(&decoded, overflow).is_err());
        assert!(lookup_attempt_mut(&mut decoded, overflow).is_err());
        assert!(original_notification(&decoded, overflow).is_err());
        decoded
            .summaries
            .get_mut(&summary_key)
            .unwrap()
            .next_allowed_at = "2026-10-23T10:15:00.123456789124Z".into();
        assert!(decoded.validate().is_err());
        if let Delivery::SummaryOverflow { summary_key, .. } =
            &mut state.records.get_mut(overflow).unwrap().delivery
        {
            *summary_key = "f".repeat(64);
        }
        assert!(state.validate().is_err());
        let (mut state, summary_key, keys) = summary_fixture(2);
        state.records.get_mut(&keys[1]).unwrap().delivery = Delivery::SummaryOverflow {
            summary_key,
            prior_failed_attempt: None,
        };
        assert!(state.validate().is_err());
    }
    #[test]
    fn whole_ledger_capacity_is_checked_before_an_external_permit_can_be_published() {
        let (state, _, _) = summary_fixture(65);
        validate_summary_capacity(&state).unwrap();
        let (mut state, _, _) = summary_fixture(MAX_RECORDS);
        assert_eq!(
            validate_summary_capacity(&state).unwrap_err().code,
            "budget"
        );
        state.records.clear();
        // Only capacity is under test here; strict cross-links are separately
        // tested above. Serialized bytes also have a whole-record ceiling.
        state.libraries.get_mut(LIBRARY).unwrap().root = format!("D:/{}", "x".repeat(MAX_BYTES));
        assert_eq!(
            validate_summary_capacity(&state).unwrap_err().code,
            "budget"
        );
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
            ReminderRecordedDelivery::Unknown { .. }
        ));
        state.records.get_mut(&key).unwrap().invalidated = Some(ReminderCancellation::TaskClosed);
        state.records.get_mut(&key).unwrap().handled = true;
        let result = status(&state, &key).unwrap();
        assert!(result.handled);
        assert_eq!(result.invalidated, Some(ReminderCancellation::TaskClosed));
        assert!(matches!(
            result.delivery,
            ReminderRecordedDelivery::Unknown { .. }
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
        state.summaries.insert(
            summary.clone(),
            Summary {
                cutoff: attempt.attempt_at.clone(),
                next_allowed_at: summary_deadline(
                    &ExactInstant::parse(&attempt.attempt_at).unwrap(),
                )
                .unwrap()
                .canonical()
                .unwrap(),
                members,
                attempt,
            },
        );
        state.validate().unwrap();
        assert!(matches!(
            status(&state, &key).unwrap().delivery,
            ReminderRecordedDelivery::Unknown { .. }
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
                failure_kind: None,
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
    #[test]
    fn original_delivery_keeps_full_typed_causes_parallel_facts_and_raw_diagnostics() {
        let (mut state, key) = fixture();
        let mut original = ErrorDescription::new(UiMessage::error_core_io_failure()).diagnostic(
            DiagnosticOrigin::Os,
            "retained native diagnostic\n{original}",
        );
        for index in 0..32 {
            original = original
                .try_caused_by(
                    format!("nativeCause{index}"),
                    ErrorDescription::new(UiMessage::common_operation(
                        weftext_messages::params::CommonOperation {
                            operation_id: index.to_string(),
                        },
                    )),
                )
                .unwrap();
        }
        let parallel: Vec<_> = (0..40)
            .map(|index| {
                ErrorDescription::new(UiMessage::common_operation(
                    weftext_messages::params::CommonOperation {
                        operation_id: format!("parallel{index}"),
                    },
                ))
            })
            .collect();
        let mut error = CoreError::new("originalNativeResult", original);
        error.add_detail_group(ErrorDetailGroup::Data {
            classification: weftext_data::DataErrorKind::Coverage,
            diagnostics: parallel.clone(),
        });
        let description = CoreDescription::from(error);
        lookup_attempt_mut(&mut state, &key).unwrap().delivery = ReminderDeliveryState::Unknown {
            description: description.clone(),
        };
        let original_bytes = serde_json::to_vec(&state).unwrap();
        let decoded = decode_reminder_record(&original_bytes).unwrap();
        let result = status(&decoded, &key).unwrap();
        let ReminderRecordedDelivery::Unknown {
            description: actual,
        } = result.delivery
        else {
            panic!("Unknown external result changed category")
        };
        assert_eq!(actual, description);
        assert_eq!(actual.causes.as_slice().len(), 32);
        assert_eq!(
            actual.diagnostics[0].text,
            "retained native diagnostic\n{original}"
        );
        let Some(CoreErrorDetails::Aggregate { groups }) = &actual.details else {
            panic!("Original Data facts were discarded")
        };
        assert!(groups.iter().any(|group| matches!(group,
            ErrorDetailGroup::Data { classification: weftext_data::DataErrorKind::Coverage, diagnostics }
                if diagnostics == &parallel
        )));
        actual.explanation.render(weftext_messages::UiLocale::EnUs);
        assert_eq!(serde_json::to_vec(&decoded).unwrap(), original_bytes);

        let mut old = serde_json::to_value(&decoded).unwrap();
        old["records"][&key]["delivery"]["attempt"]["delivery"] =
            serde_json::json!({"state":"unknown", "diagnostic":"old product wording"});
        assert!(decode_reminder_record(&serde_json::to_vec(&old).unwrap()).is_err());
        let mut old_version = serde_json::to_value(&decoded).unwrap();
        old_version["schemaVersion"] = 1.into();
        let error = decode_reminder_record(&serde_json::to_vec(&old_version).unwrap())
            .err()
            .unwrap();
        assert_eq!(error.code, "unsupportedVersion");
        assert_eq!(
            error.explanation,
            UiMessage::error_reminder_state_version_unsupported()
        );
    }

    #[test]
    fn reminder_json_duplicate_is_a_typed_validation_fact_and_syntax_keeps_json_origin() {
        let (state, _) = fixture();
        let original = serde_json::to_string(&state).unwrap();
        let duplicate = original.replacen("\"records\":{", "\"records\":{},\"records\":{", 1);
        let error = decode_reminder_record(duplicate.as_bytes()).err().unwrap();
        assert!(error.diagnostics.is_empty());
        assert_eq!(
            error.explanation,
            UiMessage::error_data_schema_json_key_duplicate(
                weftext_messages::params::ErrorDataSchemaJsonKeyDuplicate {
                    key: "records".into()
                },
            )
        );
        assert!(matches!(
            error.details,
            Some(CoreErrorDetails::Aggregate { .. })
        ));
        let syntax = br#"{"schemaVersion":1,"deviceId":"ORIGINAL" "revision":0}"#;
        let error = decode_reminder_record(syntax).err().unwrap();
        assert_eq!(error.diagnostics.len(), 1);
        assert_eq!(error.diagnostics[0].origin, DiagnosticOrigin::Json);
        assert!(!error.diagnostics[0].text.contains("ORIGINAL"));
    }

    #[test]
    fn unavailable_task_fields_and_resolution_keep_actual_kind_and_all_parallel_problems() {
        let (state, key) = fixture();
        let diagnostics: Vec<_> = (0..48)
            .map(|index| {
                ErrorDescription::new(UiMessage::common_operation(
                    weftext_messages::params::CommonOperation {
                        operation_id: index.to_string(),
                    },
                ))
            })
            .collect();
        let cell = Cell::unavailable_group(
            weftext_data::DataErrorKind::Coverage,
            UiMessage::error_core_io_failure(),
            diagnostics.clone(),
        );
        let field = field_error(
            "reminderUnavailable",
            UiMessage::error_reminder_saved_task_status_unavailable(),
            Some(&cell),
        );
        let Some(CoreErrorDetails::Aggregate { groups }) = field.details else {
            panic!("Field facts missing")
        };
        assert!(groups.iter().any(|group| matches!(group,
            ErrorDetailGroup::Data { classification: weftext_data::DataErrorKind::Coverage, diagnostics: actual }
                if actual.len() == diagnostics.len() + 1 && actual[1..] == diagnostics
        )));
        let record = &state.records[&key];
        let resolution = TaskReminderResolution {
            state: TaskReminderState::Unavailable,
            reminder_id: Some(INTENT.into()),
            trigger_instant: Some(record.trigger_instant.clone()),
            normal_delivery_until: None,
            definition_digest: Some(record.definition_digest.clone()),
            providers: BTreeMap::new(),
            failure_kind: Some(weftext_data::DataErrorKind::Coverage),
            diagnostics: diagnostics.clone(),
        };
        let error = resolution_error(
            "reminderUnavailable",
            UiMessage::error_reminder_task_not_eligible(),
            &resolution,
        );
        assert!(
            matches!(error.details, Some(CoreErrorDetails::Aggregate { groups }) if groups.iter().any(|group|
                matches!(group, ErrorDetailGroup::Data { classification: weftext_data::DataErrorKind::Coverage, diagnostics: actual }
                    if actual == &diagnostics)
            ))
        );
    }
}
#[cfg(all(test, windows))]
#[path = "reminder_device_state_tests.rs"]
mod integration_tests;
