//! Energy over the load window: RAPL package counters when readable,
//! otherwise an explicitly labelled CPU-time estimate, otherwise nothing.
use std::path::{Path, PathBuf};

pub(super) const POWERCAP_ROOT: &str = "/sys/class/powercap";
pub(super) const RAPL_LABEL: &str = "measured (RAPL, whole package, includes other processes)";
pub(super) const RAPL_HINT: &str = "RAPL energy counters usually need root since Linux 5.10 \
    (energy_uj is readable by root only); pass --cpu-watts for a labelled estimate";
const JOULES_PER_KWH: f64 = 3_600_000.0;

/// One package-level powercap zone.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Zone {
    pub name: String,
    pub energy: PathBuf,
    pub max_range_uj: u64,
}

/// `intel-rapl:0` or `amd-rapl:1`: a top-level package zone, not a
/// sub-zone such as `intel-rapl:0:0`.
pub(super) fn is_package_zone(directory: &str) -> bool {
    ["intel-rapl:", "amd-rapl:"].iter().any(|prefix| {
        directory.strip_prefix(prefix).is_some_and(|index| {
            !index.is_empty() && index.chars().all(|character| character.is_ascii_digit())
        })
    })
}

/// A counter file's value.
pub(super) fn parse_counter(text: &str) -> Option<u64> {
    text.trim().parse().ok()
}

/// Microjoules between two readings; a counter that wrapped restarts at 0
/// after `max_range_uj`.
pub(super) fn delta_uj(start: u64, end: u64, max_range_uj: u64) -> u64 {
    if end >= start {
        end - start
    } else {
        max_range_uj.saturating_sub(start).saturating_add(end)
    }
}

/// The readable package zones under `root`, or why there are none.
pub(super) fn discover(root: &Path) -> Result<Vec<Zone>, String> {
    if !cfg!(target_os = "linux") && root == Path::new(POWERCAP_ROOT) {
        return Err("RAPL powercap counters are a Linux interface".to_string());
    }
    let Ok(entries) = std::fs::read_dir(root) else {
        return Err(format!(
            "no powercap interface at {}; {RAPL_HINT}",
            root.display()
        ));
    };
    let mut directories: Vec<PathBuf> = entries
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(is_package_zone)
        })
        .collect();
    directories.sort();
    let mut zones = Vec::new();
    let mut denied = false;
    for directory in directories {
        let name = std::fs::read_to_string(directory.join("name")).unwrap_or_default();
        if !name.trim().starts_with("package") {
            continue;
        }
        let max_range = std::fs::read_to_string(directory.join("max_energy_range_uj"))
            .ok()
            .as_deref()
            .and_then(parse_counter);
        let energy = directory.join("energy_uj");
        match std::fs::read_to_string(&energy) {
            Ok(text) if parse_counter(&text).is_some() => {
                if let Some(max_range_uj) = max_range {
                    zones.push(Zone {
                        name: name.trim().to_string(),
                        energy,
                        max_range_uj,
                    });
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => denied = true,
            _ => {}
        }
    }
    if zones.is_empty() {
        return Err(if denied {
            format!("RAPL counters exist but are not readable; {RAPL_HINT}")
        } else {
            format!("no readable RAPL package zone; {RAPL_HINT}")
        });
    }
    Ok(zones)
}

/// One reading of every zone, in the zones' order.
pub(super) fn read(zones: &[Zone]) -> Option<Vec<u64>> {
    zones
        .iter()
        .map(|zone| {
            std::fs::read_to_string(&zone.energy)
                .ok()
                .as_deref()
                .and_then(parse_counter)
        })
        .collect()
}

/// Joules across all zones between two readings.
pub(super) fn joules(zones: &[Zone], start: &[u64], end: &[u64]) -> Option<f64> {
    if zones.len() != start.len() || zones.len() != end.len() {
        return None;
    }
    let microjoules: u64 = zones
        .iter()
        .zip(start.iter().zip(end))
        .map(|(zone, (start, end))| delta_uj(*start, *end, zone.max_range_uj))
        .fold(0_u64, u64::saturating_add);
    Some(microjoules as f64 / 1_000_000.0)
}

/// The energy figure of a run and how it was obtained.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Energy {
    Measured {
        joules: f64,
        method: String,
        zones: Vec<String>,
    },
    Estimate {
        joules: f64,
        method: String,
        cpu_seconds: f64,
        cpu_watts: f64,
    },
    NotMeasured {
        reason: String,
    },
}

impl Energy {
    /// RAPL wins; otherwise CPU time × watts when both are known.
    pub(super) fn decide(
        rapl: Result<(f64, Vec<String>), String>,
        cpu_seconds: Option<f64>,
        cpu_watts: Option<f64>,
    ) -> Self {
        let reason = match rapl {
            Ok((joules, zones)) => {
                return Self::Measured {
                    joules,
                    method: RAPL_LABEL.to_string(),
                    zones,
                };
            }
            Err(reason) => reason,
        };
        match (cpu_seconds, cpu_watts) {
            (Some(cpu_seconds), Some(cpu_watts)) => Self::Estimate {
                joules: cpu_seconds * cpu_watts,
                method: format!(
                    "estimate: E = process CPU time ({cpu_seconds:.3} s) × --cpu-watts ({cpu_watts} W); \
                     assumes the given wattage per busy CPU second"
                ),
                cpu_seconds,
                cpu_watts,
            },
            (None, Some(_)) => Self::NotMeasured {
                reason: "--cpu-watts needs the process CPU time, which was not measured"
                    .to_string(),
            },
            _ => Self::NotMeasured { reason },
        }
    }

    pub(super) fn joules(&self) -> Option<f64> {
        match self {
            Self::Measured { joules, .. } | Self::Estimate { joules, .. } => Some(*joules),
            Self::NotMeasured { .. } => None,
        }
    }

    pub(super) fn is_estimate(&self) -> bool {
        matches!(self, Self::Estimate { .. })
    }
}

pub(super) fn kwh(joules: f64) -> f64 {
    joules / JOULES_PER_KWH
}

#[cfg(test)]
mod tests {
    use super::*;

    const MAX: u64 = 262_143_328_850;

    #[test]
    fn counters_and_package_zone_names_are_parsed() {
        assert_eq!(
            parse_counter(include_str!("fixtures/rapl_energy_uj")),
            Some(123_456_789)
        );
        assert_eq!(
            parse_counter(include_str!("fixtures/rapl_max_energy_range_uj")),
            Some(MAX)
        );
        assert_eq!(parse_counter("not a number"), None);
        assert!(is_package_zone("intel-rapl:0"));
        assert!(is_package_zone("amd-rapl:12"));
        assert!(!is_package_zone("intel-rapl:0:0"));
        assert!(!is_package_zone("intel-rapl"));
        assert!(!is_package_zone("intel-rapl:"));
        assert!(!is_package_zone("dtpm"));
    }

    #[test]
    fn deltas_handle_counter_wraparound() {
        assert_eq!(delta_uj(1_000, 6_000, MAX), 5_000);
        assert_eq!(delta_uj(MAX - 1_000, 4_000, MAX), 5_000);
        assert_eq!(delta_uj(7, 7, MAX), 0);
        // A reading above the advertised range never underflows.
        assert_eq!(delta_uj(MAX + 10, 5, MAX), 5);
    }

    #[cfg(unix)]
    #[test]
    fn discovery_reads_package_zones_and_sums_joules() {
        let root = tempfile::tempdir().unwrap();
        let zone = |directory: &str, name: &str, energy: &str| {
            let path = root.path().join(directory);
            std::fs::create_dir_all(&path).unwrap();
            std::fs::write(path.join("name"), name).unwrap();
            std::fs::write(
                path.join("max_energy_range_uj"),
                include_str!("fixtures/rapl_max_energy_range_uj"),
            )
            .unwrap();
            std::fs::write(path.join("energy_uj"), energy).unwrap();
        };
        zone(
            "intel-rapl:0",
            include_str!("fixtures/rapl_name"),
            include_str!("fixtures/rapl_energy_uj"),
        );
        zone("intel-rapl:0:0", "core\n", "5\n");
        zone("intel-rapl:1", "psys\n", "5\n");
        zone("amd-rapl:1", "package-1\n", "10\n");
        let zones = discover(root.path()).unwrap();
        let names: Vec<&str> = zones.iter().map(|zone| zone.name.as_str()).collect();
        assert_eq!(names, ["package-1", "package-0"]);
        let start = read(&zones).unwrap();
        assert_eq!(start, [10, 123_456_789]);
        // One zone wrapped (1 J), the other did not (2 J).
        let wrapped = [MAX - 500_000, 123_456_789 + 2_000_000];
        let after = [500_000, 123_456_789 + 4_000_000];
        assert_eq!(joules(&zones, &wrapped, &after), Some(3.0));
        assert_eq!(joules(&zones, &start, &start), Some(0.0));
        assert_eq!(joules(&zones, &start, &start[..1]), None);

        let empty = tempfile::tempdir().unwrap();
        assert!(
            discover(empty.path())
                .unwrap_err()
                .contains("no readable RAPL package zone")
        );
        assert!(
            discover(&empty.path().join("missing"))
                .unwrap_err()
                .contains("root since Linux 5.10")
        );
    }

    #[cfg(unix)]
    #[test]
    fn unparsable_missing_and_unreadable_counters_are_told_apart() {
        use std::os::unix::fs::PermissionsExt;

        let package = |root: &Path, energy: Option<&str>| {
            let path = root.join("intel-rapl:0");
            std::fs::create_dir_all(&path).unwrap();
            std::fs::write(path.join("name"), "package-0\n").unwrap();
            std::fs::write(path.join("max_energy_range_uj"), "1000\n").unwrap();
            if let Some(energy) = energy {
                std::fs::write(path.join("energy_uj"), energy).unwrap();
            }
            path.join("energy_uj")
        };
        for energy in [Some("not a number\n"), None] {
            let root = tempfile::tempdir().unwrap();
            package(root.path(), energy);
            let error = discover(root.path()).unwrap_err();
            assert!(error.contains("no readable RAPL package zone"), "{error}");
        }

        let root = tempfile::tempdir().unwrap();
        let counter = package(root.path(), Some("5\n"));
        std::fs::set_permissions(&counter, std::fs::Permissions::from_mode(0o000)).unwrap();
        // A privileged test user can still read the file; nothing to assert then.
        if std::fs::read_to_string(&counter).is_err() {
            let error = discover(root.path()).unwrap_err();
            assert!(error.contains("exist but are not readable"), "{error}");
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn the_system_powercap_root_is_inspected_on_linux() {
        let outcome = discover(Path::new(POWERCAP_ROOT));
        assert!(
            outcome
                .as_ref()
                .err()
                .is_none_or(|error| !error.contains("are a Linux interface")),
            "{outcome:?}"
        );
    }

    #[test]
    fn energy_prefers_rapl_then_a_labelled_estimate() {
        let measured = Energy::decide(Ok((12.5, vec!["package-0".into()])), Some(2.0), Some(15.0));
        assert_eq!(measured.joules(), Some(12.5));
        assert!(!measured.is_estimate());

        let estimate = Energy::decide(Err("denied".into()), Some(2.0), Some(15.0));
        assert_eq!(estimate.joules(), Some(30.0));
        assert!(estimate.is_estimate());
        let Energy::Estimate { method, .. } = &estimate else {
            panic!("estimate expected");
        };
        assert!(method.starts_with("estimate: E = process CPU time"));

        let missing_cpu = Energy::decide(Err("denied".into()), None, Some(15.0));
        assert_eq!(missing_cpu.joules(), None);
        assert_eq!(
            missing_cpu,
            Energy::NotMeasured {
                reason: "--cpu-watts needs the process CPU time, which was not measured".into()
            }
        );
        assert_eq!(
            Energy::decide(Err("denied".into()), Some(2.0), None),
            Energy::NotMeasured {
                reason: "denied".into()
            }
        );
        assert!((kwh(3_600_000.0) - 1.0).abs() < f64::EPSILON);
    }
}
