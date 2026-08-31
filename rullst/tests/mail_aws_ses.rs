#![cfg(feature = "mail-aws-ses")]

use rullst::mail::{AwsSesDriver, DeliveryMode};

#[test]
fn umbrella_exports_native_aws_ses_boundary() {
    let driver = AwsSesDriver::try_native(
        "us-east-1",
        "AKIAIOSFODNN7EXAMPLE",
        "wJalrXUtnFEMI/K7MDENG+bPxRfiCYEXAMPLEKEY",
        None,
    )
    .expect("valid protocol fixture credentials");
    assert_eq!(driver.region(), "us-east-1");
    assert_eq!(driver.delivery_mode(), DeliveryMode::Real);
    assert!(format!("{driver:?}").contains("native_sigv4"));

    let _official_sdk_type = std::any::TypeId::of::<rullst::mail::aws_ses_sdk::Config>();
}
