use rullst_supervision::{Context, Limits, OpaqueId, Operator, Revision, Scope, StoreConfig};

#[test]
fn public_inputs_bound_identifiers_revisions_configuration_and_debug_output() {
    for bad in [
        "",
        "a b",
        "a/b",
        "alice@example.com",
        "\n",
        "é",
        &"a".repeat(129),
    ] {
        assert!(OpaqueId::new(bad).is_err());
    }
    assert!(OpaqueId::new("a".repeat(128)).is_ok());
    assert!(Revision::new(0).is_err());
    assert!(Revision::new(-1).is_err());
    assert_eq!(Revision::new(i64::MAX).unwrap().value(), i64::MAX);
    assert!(Limits::new(0, 1, 1, 1).is_err());
    assert!(Limits::new(1, 1025, 1, 1).is_err());
    assert!(Limits::new(1, 1, 16385, 1).is_err());
    assert!(Limits::new(1, 1, 1, 1025).is_err());
    let limits = Limits::new(8, 8, 8, 8).unwrap();
    assert!(limits.clone().event_budget(1, 0).is_err());
    assert!(limits.clone().event_budget(2049, 1).is_err());
    assert!(StoreConfig::new("epoch", limits.clone(), 3599, 60).is_err());
    assert!(StoreConfig::new("epoch", limits.clone(), 604801, 60).is_err());
    assert!(StoreConfig::new("epoch", limits.clone(), 3600, 28801).is_err());
    let context = Context::new("private-school", "private-actor").unwrap();
    let operator = Operator::new(context, "private-evidence").unwrap();
    let scope = Scope::new("private-school", "private-learner", "private-resource").unwrap();
    let config = StoreConfig::new("private-epoch", limits, 3600, 60).unwrap();
    assert!(!format!("{operator:?} {scope:?} {config:?}").contains("private-"));
}

#[cfg(feature = "exam")]
#[test]
fn acknowledgement_is_explicit_and_session_policy_has_a_hard_lifetime() {
    use rullst_supervision::exam::{Acknowledgement, ExamPolicy};
    assert!(Acknowledgement::new("policy", "notice", false).is_err());
    assert!(Acknowledgement::new("", "notice", true).is_err());
    assert!(ExamPolicy::new("policy", "notice", 0).is_err());
    assert!(ExamPolicy::new("policy", "notice", 28801).is_err());
    assert_eq!(
        ExamPolicy::new("policy", "notice", 28800)
            .unwrap()
            .lifetime_seconds(),
        28800
    );
}

#[cfg(feature = "parental")]
#[test]
fn parental_policy_bounds_courses_and_absolute_windows() {
    use rullst_supervision::parental::CoursePolicy;
    assert!(CoursePolicy::new(["a", "a"], 1, 10).is_err());
    assert!(CoursePolicy::new((0..65).map(|n| format!("course-{n}")), 1, 10).is_err());
    assert!(CoursePolicy::new(["a"], -1, 10).is_err());
    assert!(CoursePolicy::new(["a"], 10, 10).is_err());
    assert!(CoursePolicy::new(["a"], 0, 2_592_001).is_err());
    assert!(CoursePolicy::new(["a"], 0, i64::MAX).is_err());
    assert!(CoursePolicy::new(Vec::<String>::new(), 0, 2_592_000).is_ok());
}

#[cfg(feature = "exam")]
#[test]
fn collection_selection_and_request_bindings_are_bounded_and_inspectable_without_storage() {
    use rullst_supervision::exam::{Capability, Collection, ObservationRequest};
    assert_eq!(Collection::visibility_only().bits(), 1);
    assert_eq!(Collection::new(Capability::ALL).unwrap().bits(), 511);
    assert!(Collection::from_bits(512).is_err());
    assert!(Collection::new([Capability::Visibility, Capability::Visibility]).is_err());
    assert_eq!(Collection::none().capabilities().count(), 0);
    let actor = Context::new("school", "learner").unwrap();
    let scope = Scope::new("school", "learner", "exam").unwrap();
    let session = OpaqueId::new("session").unwrap();
    let revision = Revision::new(7).unwrap();
    assert!(ObservationRequest::new(&actor, &scope, &session, revision, 0).is_err());
    let request = ObservationRequest::new(&actor, &scope, &session, revision, 1).unwrap();
    assert_eq!(request.context(), &actor);
    assert_eq!(request.scope(), &scope);
    assert_eq!(request.session(), &session);
    assert_eq!(request.revision(), revision);
    assert_eq!(request.sequence(), 1);
}

#[cfg(feature = "analysis")]
#[test]
fn borrowed_media_contract_checks_format_and_size_and_withholds_debug_contents() {
    use rullst_supervision::analysis::{
        AnalysisKind, AnalysisOptions, AnalyzerDescriptor, MediaSample, SampleFormat,
    };
    for format in [
        SampleFormat::JpegFrame,
        SampleFormat::PngFrame,
        SampleFormat::Pcm16LeMono16Khz,
    ] {
        assert!(MediaSample::new(format, &[]).is_err());
    }
    assert!(MediaSample::new(SampleFormat::JpegFrame, b"wrong-header").is_err());
    assert!(MediaSample::new(SampleFormat::PngFrame, b"wrong-header").is_err());
    assert!(MediaSample::new(SampleFormat::Pcm16LeMono16Khz, &[0]).is_err());
    assert!(MediaSample::new(SampleFormat::Pcm16LeMono16Khz, &vec![0; 160002]).is_err());
    assert!(MediaSample::new(SampleFormat::Pcm16LeMono16Khz, &vec![0; 160000]).is_ok());
    let mut image = vec![0; 1024 * 1024 + 1];
    image[..3].copy_from_slice(&[255, 216, 255]);
    assert!(MediaSample::new(SampleFormat::JpegFrame, &image).is_err());
    assert!(MediaSample::new(SampleFormat::JpegFrame, &image[..1024 * 1024]).is_ok());
    let png = MediaSample::new(SampleFormat::PngFrame, b"\x89PNG\r\n\x1a\nprivate-marker").unwrap();
    assert_eq!(png.kind(), AnalysisKind::CameraPresence);
    assert_eq!(png.format(), SampleFormat::PngFrame);
    assert!(!format!("{png:?}").contains("private-marker"));
    assert!(AnalyzerDescriptor::new("", "v1", AnalysisKind::CameraPresence, true).is_err());
    assert!(AnalysisOptions::new(0).is_err());
    assert!(AnalysisOptions::new(16).is_err());
    let options = AnalysisOptions::new(15).unwrap();
    assert_eq!(options.timeout_seconds(), 15);
    assert!(!options.permits_simulation());
    assert!(options.allow_simulated_for_testing().permits_simulation());
}
