//! OS startup state is an observation distinct from Core's persisted intent.
//! The portable delivery has no StartupTask capability and never writes Run.
use windows::{
    ApplicationModel::{StartupTask, StartupTaskState},
    Win32::Foundation::{E_ACCESSDENIED, E_UNEXPECTED},
    core::{Error, HSTRING, Result},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum State {
    Unavailable,
    Unknown,
    Disabled,
    Enabled,
    DisabledByUser,
    DisabledByPolicy,
    EnabledByPolicy,
}
#[derive(Clone, Copy, Debug)]
pub(super) struct Observation {
    pub state: State,
    pub selected: bool,
}
impl Default for Observation {
    fn default() -> Self {
        Self {
            state: State::Unavailable,
            selected: false,
        }
    }
}
impl Observation {
    pub fn can_request(self) -> bool {
        matches!(self.state, State::Disabled | State::Enabled)
    }
    pub fn enabled(self) -> bool {
        self.selected && matches!(self.state, State::Enabled | State::EnabledByPolicy)
    }
}
fn state(value: StartupTaskState) -> Result<State> {
    Ok(match value {
        StartupTaskState::Disabled => State::Disabled,
        StartupTaskState::Enabled => State::Enabled,
        StartupTaskState::DisabledByUser => State::DisabledByUser,
        StartupTaskState::DisabledByPolicy => State::DisabledByPolicy,
        StartupTaskState::EnabledByPolicy => State::EnabledByPolicy,
        _ => return Err(Error::from(E_UNEXPECTED)),
    })
}
fn task() -> Result<Option<StartupTask>> {
    let Some(_) = super::package::current()? else {
        return Ok(None);
    };
    let contract = crate::host_contract::get().map_err(|_| Error::from(E_UNEXPECTED))?;
    let task =
        StartupTask::GetAsync(&HSTRING::from(&contract.activation.startup_task_id))?.join()?;
    if task.TaskId()? != contract.activation.startup_task_id {
        return Err(Error::from(E_ACCESSDENIED));
    }
    Ok(Some(task))
}
fn system(request: Option<bool>) -> Result<State> {
    let Some(task) = task()? else {
        return Ok(State::Unavailable);
    };
    let previous = state(task.State()?)?;
    match (request, previous) {
        (Some(true), State::Disabled) => {
            state(task.RequestEnableAsync()?.join()?)?;
        }
        (Some(false), State::Enabled) => {
            task.Disable()?;
        }
        // The user's Task Manager choice and system policy are never forced.
        _ => {}
    }
    state(task.State()?)
}
pub(super) async fn observe(
    profile: &weftext_core::QualifiedProfile,
    device: &weftext_core::DeviceIdentity,
) -> Observation {
    let selected = crate::host_state::Root::qualify(false).and_then(|root| root.startup_profile());
    let selected = match selected {
        Ok(Some(original)) => {
            original.profile.physical_identity() == profile.physical_identity()
                && device.is_current
                && original.device_id == device.id
        }
        Ok(None) => false,
        Err(_) => {
            return Observation {
                state: State::Unknown,
                selected: false,
            };
        }
    };
    Observation {
        state: super::winrt::call(|| system(None))
            .await
            .unwrap_or(State::Unknown),
        selected,
    }
}
pub(super) async fn request(
    profile: &weftext_core::QualifiedProfile,
    device: &weftext_core::DeviceIdentity,
    enabled: bool,
) -> std::io::Result<Observation> {
    let before = observe(profile, device).await;
    if !device.is_current || !before.can_request() {
        return Err(std::io::Error::other(
            "StartupTask is unavailable, unqualified, disabled by the user, or controlled by policy",
        ));
    }
    let path = profile.path().to_owned();
    let physical = profile.physical_identity().to_owned();
    let original_device = device.clone();
    super::winrt::call(move || {
        let profile =
            weftext_core::qualify_existing_profile(path).map_err(std::io::Error::other)?;
        if profile.physical_identity() != physical {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "The original startup profile identity changed",
            ));
        }
        crate::host_state::Root::qualify(false)?.change_startup(
            &profile,
            &original_device,
            enabled,
            || system(Some(enabled)).map_err(std::io::Error::other),
        )
    })
    .await?;
    // Selection can change after the request returns. Never manufacture a
    // checked state from the request or retain an earlier profile selection.
    Ok(observe(profile, device).await)
}
