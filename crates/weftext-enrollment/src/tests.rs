use super::*;

#[test]
fn physical_identity_requires_one_strict_platform_tag() {
    assert!(
        serde_json::from_str::<PhysicalIdentity>(r#"{"volume":1,"index":2,"created":3}"#).is_err()
    );
    let windows = r#"{"platform":"windows","volume":1,"index":2,"created":3}"#;
    assert_eq!(
        serde_json::to_string(&serde_json::from_str::<PhysicalIdentity>(windows).unwrap()).unwrap(),
        windows
    );
    assert!(
        serde_json::from_str::<PhysicalIdentity>(
            r#"{"platform":"linux","volume":1,"filesystem":[1,2],"handle_type":1,"handle":[3]}"#
        )
        .is_err()
    );
    assert!(
        serde_json::from_str::<PhysicalIdentity>(
            r#"{"platform":"unknown","volume":1,"index":2,"created":3}"#
        )
        .is_err()
    );
}

#[cfg(target_os = "linux")]
#[test]
fn traversal_and_ordinary_paths_cannot_supply_administrator_authority() {
    assert!(matches!(
        open_administrator_file(std::path::Path::new("relative"), 4096),
        Err(EnrollmentError::Rejected(
            EnrollmentRejection::AbsoluteAdministratorPathRequired
        ))
    ));
    assert!(open_administrator_file(std::path::Path::new("/tmp/../etc/machine-id"), 4096).is_err());
    assert!(inspect_enrollment(std::path::Path::new("/proc/self/status")).is_err());
}

#[cfg(target_os = "linux")]
#[test]
#[ignore = "requires an actual enrolled non-root service with empty supplementary groups and no capabilities"]
fn actual_service_process_has_current_administrator_authority() {
    let path = std::env::var_os("WEFTEXT_LINUX_TEST_ENROLLMENT").expect("explicit enrollment");
    let authority = inspect_enrollment(std::path::Path::new(&path)).unwrap();
    authority.check_current().unwrap();
    authority.verify_service_process().unwrap();
    assert_eq!(unsafe { libc::getuid() }, authority.service_uid());
    assert_ne!(authority.service_uid(), authority.worker_uid());
    assert_ne!(authority.service_uid(), authority.client_uid());
    assert_eq!(authority.read_service_key().unwrap().len(), 32);
    assert!(matches!(
        authority.root_identity(),
        PhysicalIdentity::Linux { .. }
    ));
}
