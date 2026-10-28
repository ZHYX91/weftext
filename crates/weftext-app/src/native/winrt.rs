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

#[cfg(test)]
mod tests {
    const CHILD: &str = "WEFTEXT_TEST_WINRT_FACTORY_LIFETIME";

    #[test]
    fn cached_package_factory_survives_idle_mta_workers() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "native::winrt::tests::package_factory_child",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "WinRT factory child failed: {:?}\n{}\n{}",
            output.status,
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr),
        );
    }

    #[test]
    fn package_factory_child() {
        if std::env::var_os(CHILD).is_none() {
            return;
        }
        // A child process owns the factory cache and its final COM teardown.
        // No package is required: even the expected unpackaged error invokes
        // IPackageStatics::Current, whose cached pointer crashed the real host.
        let _process = crate::launch::ProcessWinRt::enter().unwrap();
        let mut original = None;
        for _ in 0..4 {
            let observed = std::thread::spawn(|| {
                let _apartment = super::Apartment::enter().unwrap();
                windows::ApplicationModel::Package::Current()
                    .and_then(|package| package.Id()?.FullName())
                    .map(|name| name.to_string())
                    .map_err(|error| error.code().0)
            })
            .join()
            .unwrap();
            if let Some(expected) = &original {
                assert_eq!(&observed, expected);
            } else {
                original = Some(observed);
            }
        }
    }
}
