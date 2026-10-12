use super::*;

const MAX: u64 = MAX_OBSERVATION_COUNT;

fn window(seconds: u64) -> Duration {
    Duration::from_secs(seconds)
}

fn rejection(
    total: u64,
    failed: u64,
    accounts: u64,
    paths: u64,
    sources: u64,
    correlated: u64,
) -> SentinelError {
    SentinelObservation::try_new(
        window(60),
        total,
        failed,
        accounts,
        paths,
        sources,
        correlated,
    )
    .expect_err("observation must be rejected")
}

#[test]
fn window_accepts_one_second_to_one_hour() {
    for seconds in [1, 60 * 60] {
        let observation = SentinelObservation::try_new(window(seconds), 1, 0, 0, 0, 1, 0)
            .expect("window inside the bound");
        assert_eq!(observation.window_seconds(), seconds);
    }
    assert_eq!(
        SentinelObservation::try_new(window(60 * 60 + 1), 1, 0, 0, 0, 1, 0),
        Err(SentinelError::InvalidObservation("window"))
    );
}

#[test]
fn request_count_bounds_are_checked_before_consistency() {
    assert_eq!(
        rejection(0, 0, 0, 0, 0, 0),
        SentinelError::InvalidObservation("request count")
    );
    assert_eq!(
        rejection(MAX + 1, 0, 0, 0, 1, 0),
        SentinelError::InvalidObservation("request count")
    );
    assert_eq!(
        rejection(5, 0, 0, MAX + 1, 1, 0),
        SentinelError::InvalidObservation("request count")
    );
    let largest = SentinelObservation::try_new(window(60), MAX, MAX, MAX, MAX, MAX, MAX)
        .expect("every count may reach the bound");
    assert_eq!(largest.total_requests(), MAX);
}

#[test]
fn each_inconsistent_count_is_rejected_on_its_own() {
    let inconsistent = SentinelError::InvalidObservation("inconsistent counts");
    // failures > requests
    assert_eq!(rejection(5, 6, 0, 0, 1, 0), inconsistent);
    // accounts > failures
    assert_eq!(rejection(5, 2, 3, 0, 1, 0), inconsistent);
    // paths > requests
    assert_eq!(rejection(5, 0, 0, 6, 1, 0), inconsistent);
    // no source
    assert_eq!(rejection(5, 0, 0, 0, 0, 0), inconsistent);
    // sources > requests
    assert_eq!(rejection(5, 0, 0, 0, 6, 0), inconsistent);
    // correlated sources > sources
    assert_eq!(rejection(5, 0, 0, 0, 2, 3), inconsistent);
}

#[test]
fn counts_may_equal_their_upper_neighbour() {
    let observation = SentinelObservation::try_new(window(60), 5, 5, 5, 5, 5, 5)
        .expect("equal counts are consistent");
    assert_eq!(observation.failed_auth_attempts(), 5);
    assert_eq!(observation.distinct_sources(), 5);
}

#[test]
fn accessors_return_each_validated_count() {
    let observation = SentinelObservation::try_new(window(120), 900, 700, 300, 200, 50, 30)
        .expect("valid observation");
    assert_eq!(
        [
            observation.window_seconds(),
            observation.total_requests(),
            observation.failed_auth_attempts(),
            observation.distinct_accounts(),
            observation.distinct_paths(),
            observation.distinct_sources(),
            observation.correlated_sources(),
        ],
        [120, 900, 700, 300, 200, 50, 30]
    );
}

#[test]
fn thresholds_accept_one_to_the_observation_bound() {
    for (requests, paths) in [(1, 1), (5, 3), (MAX, MAX)] {
        assert!(
            SentinelPolicy::default()
                .try_with_api_scraping(requests, paths)
                .is_ok()
        );
    }
    for (requests, paths, name) in [
        (0, 1, "scraping requests"),
        (MAX + 1, 1, "scraping requests"),
        (1, 0, "scraping paths"),
        (1, MAX + 1, "scraping paths"),
    ] {
        assert_eq!(
            SentinelPolicy::default().try_with_api_scraping(requests, paths),
            Err(SentinelError::InvalidConfiguration(name))
        );
    }
}

#[test]
fn custom_scraping_thresholds_change_the_assessment() {
    let observation =
        SentinelObservation::try_new(window(60), 10, 0, 0, 3, 1, 0).expect("valid observation");
    assert!(
        ThreatClassifier::default()
            .assess(observation)
            .patterns()
            .is_empty()
    );

    let policy = SentinelPolicy::default()
        .try_with_api_scraping(10, 3)
        .expect("valid scraping policy");
    let assessment = ThreatClassifier::new(policy).assess(observation);
    assert_eq!(assessment.patterns(), &[ThreatPattern::ApiScraping]);
    assert_eq!(assessment.risk_score(), 60);
}

#[test]
fn correlated_sources_may_equal_distinct_sources() {
    assert!(
        SentinelPolicy::default()
            .try_with_distributed_automation(1, 2, 2)
            .is_ok()
    );
}
