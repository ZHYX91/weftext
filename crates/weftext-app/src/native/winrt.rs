//! One balanced MTA section owns every WinRT pointer it creates. Only plain
//! results cross the thread boundary; dropping a caller does not drop its work.
use windows::Win32::System::WinRT::{RO_INIT_MULTITHREADED, RoInitialize, RoUninitialize};
pub(super) struct Apartment;
impl Apartment {
    pub fn enter() -> windows::core::Result<Self> {
        unsafe { RoInitialize(RO_INIT_MULTITHREADED) }?;
        Ok(Self)
    }
}
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe { RoUninitialize() };
    }
}

pub(super) async fn call<R: Send + 'static, E>(
    operation: impl FnOnce() -> Result<R, E> + Send + 'static,
) -> std::io::Result<R>
where
    E: std::error::Error + Send + Sync + 'static,
{
    crate::work::spawn_blocking(move || {
        let _apartment = Apartment::enter().map_err(std::io::Error::other)?;
        operation().map_err(std::io::Error::other)
    })
    .await
    .map_err(std::io::Error::other)?
}
