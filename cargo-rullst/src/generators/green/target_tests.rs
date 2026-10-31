//! URL validation, socket lookup and the lifecycle of a started app.
use super::*;

#[test]
fn only_loopback_http_origins_are_accepted() {
    for (raw, normalized) in [
        ("http://127.0.0.1:3000", "http://127.0.0.1:3000/"),
        ("http://127.10.0.1:8080/", "http://127.10.0.1:8080/"),
        ("http://[::1]:3000", "http://[::1]:3000/"),
        ("http://LOCALHOST:3000", "http://127.0.0.1:3000/"),
        (
            "http://[::ffff:127.0.0.1]:3000",
            "http://[::ffff:7f00:1]:3000/",
        ),
    ] {
        assert_eq!(validate_url(raw).unwrap().as_str(), normalized, "{raw}");
    }
    for (raw, error) in [
        (
            "http://example.com",
            UrlError::NotLoopback("example.com".into()),
        ),
        (
            "http://10.0.0.1:80",
            UrlError::NotLoopback("10.0.0.1".into()),
        ),
        (
            "http://0.0.0.0:3000",
            UrlError::NotLoopback("0.0.0.0".into()),
        ),
        ("http://[::]:3000", UrlError::NotLoopback("[::]".into())),
        (
            "http://localhost.evil.example",
            UrlError::NotLoopback("localhost.evil.example".into()),
        ),
        ("https://127.0.0.1:3000", UrlError::Scheme),
        ("ftp://127.0.0.1", UrlError::Scheme),
        ("http://user:secret@127.0.0.1:3000", UrlError::Extra),
        ("http://user@127.0.0.1:3000", UrlError::Extra),
        ("http://:secret@127.0.0.1:3000", UrlError::Extra),
        ("http://127.0.0.1:3000/?q=1", UrlError::Extra),
        ("http://127.0.0.1:3000/#x", UrlError::Extra),
        ("http://127.0.0.1:3000/health", UrlError::Path),
        ("localhost:3000", UrlError::Scheme),
        ("127.0.0.1:3000", UrlError::Invalid("127.0.0.1:3000".into())),
        ("not a url", UrlError::Invalid("not a url".into())),
    ] {
        assert_eq!(validate_url(raw), Err(error), "{raw}");
    }
}

#[test]
fn request_paths_keep_the_loopback_host() {
    let base = validate_url("http://127.0.0.1:3000").unwrap();
    assert_eq!(
        request_url(&base, "/search?q=rust").as_str(),
        "http://127.0.0.1:3000/search?q=rust"
    );
    let odd = request_url(&base, "/@evil.example/x");
    assert_eq!(odd.host_str(), Some("127.0.0.1"));
}

#[test]
fn listening_sockets_are_found_by_port_and_state() {
    let table = include_str!("fixtures/proc_net_tcp");
    assert_eq!(listening_inodes(table, 3000), [424242]);
    // Port 8080 only has an established connection.
    assert!(listening_inodes(table, 8080).is_empty());
    assert!(listening_inodes(table, 9).is_empty());
    let table6 = include_str!("fixtures/proc_net_tcp6");
    assert_eq!(listening_inodes(table6, 3000), [515151]);
}

#[test]
fn a_free_port_is_a_bindable_loopback_port() {
    // A parallel test may bind the freed port first, so try a few fresh
    // ports; one of them must bind.
    let bound = (0..5).find_map(|_| {
        let port = free_port().unwrap();
        // The kernel picks it from the unprivileged ephemeral range.
        assert!(port >= 1024, "{port}");
        std::net::TcpListener::bind(("127.0.0.1", port))
            .ok()
            .map(|listener| (port, listener))
    });
    let (port, listener) = bound.expect("no free port could be bound");
    assert_eq!(listener.local_addr().unwrap().port(), port);
}

#[test]
fn only_a_single_owner_identifies_the_app() {
    assert_eq!(single_owner(&[4242]), Listener::Found(4242));
    assert_eq!(
        single_owner(&[]),
        Listener::Unknown("the listening process belongs to another user or is not visible")
    );
    assert_eq!(
        single_owner(&[4242, 4243]),
        Listener::Unknown("several processes share the listening socket")
    );
}

#[cfg(unix)]
mod owned {
    use super::*;
    use crate::generators::green::tests::support::{TestServer, script};

    /// Starts `binary`, retrying while another test's fork still holds the
    /// freshly written script open (ETXTBSY).
    fn start(binary: &Path, port: u16, environment: &str) -> OwnedApp {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            match OwnedApp::spawn(binary, port, environment) {
                Err(error) if error.raw_os_error() == Some(26) && Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(20));
                }
                result => return result.unwrap(),
            }
        }
    }

    /// Waits until the app's stderr tail contains `text`.
    fn wait_for_stderr(app: &OwnedApp, text: &str) -> String {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            let tail = app.stderr_tail();
            if tail.contains(text) || Instant::now() >= deadline {
                return tail;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    fn block_on<T>(future: impl std::future::Future<Output = T>) -> T {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap()
            .block_on(future)
    }

    fn alive(pid: u32) -> bool {
        let pid = rustix::process::Pid::from_raw(i32::try_from(pid).unwrap()).unwrap();
        rustix::process::test_kill_process(pid).is_ok()
    }

    #[test]
    fn the_app_gets_its_environment_and_its_stderr_tail_is_kept() {
        let directory = tempfile::tempdir().unwrap();
        let app = script(
            directory.path(),
            "app",
            "echo \"env=$RULLST_ENV host=$HOST port=$PORT\" >&2\nexec sleep 30",
        );
        let mut started = start(&app, 4321, "staging");
        assert_eq!(started.pid(), started.child.id());
        let tail = wait_for_stderr(&started, "port=4321");
        assert_eq!(tail, "env=staging host=127.0.0.1 port=4321");
        started.stop();
        assert!(started.child.try_wait().unwrap().is_some());
    }

    #[test]
    fn stopping_lets_the_app_exit_on_sigterm_within_the_grace_period() {
        let directory = tempfile::tempdir().unwrap();
        let app = script(
            directory.path(),
            "app",
            "trap 'sleep 0.3; exit 7' TERM\necho ready >&2\nwhile :; do sleep 0.05; done",
        );
        let mut started = start(&app, 4321, "production");
        assert_eq!(wait_for_stderr(&started, "ready"), "ready");
        let stopping = Instant::now();
        started.stop();
        // The trap needs about 0.3 s; killing at once would leave no exit code.
        assert!(
            stopping.elapsed() < Duration::from_secs(3),
            "{:?}",
            stopping.elapsed()
        );
        let status = started.child.try_wait().unwrap().unwrap();
        assert_eq!(status.code(), Some(7), "{status}");
    }

    #[test]
    fn dropping_the_app_stops_it() {
        let directory = tempfile::tempdir().unwrap();
        let app = script(directory.path(), "app", "exec sleep 30");
        let started = start(&app, 4321, "production");
        let pid = started.pid();
        assert!(alive(pid));
        drop(started);
        assert!(!alive(pid), "{pid} still runs");
    }

    #[test]
    fn readiness_is_an_answer_on_the_url_while_the_app_runs() {
        let server = TestServer::start();
        let directory = tempfile::tempdir().unwrap();
        let app = script(directory.path(), "app", "exec sleep 30");
        let mut started = start(&app, 4321, "production");
        let client = super::super::super::load::client().unwrap();
        block_on(started.wait_ready(&client, &server.url("/health"))).unwrap();
        assert_eq!(server.paths(), ["/health"]);
    }

    #[test]
    fn an_app_that_exits_during_start_up_is_reported() {
        // Accepts connections but never answers, so only the exit ends the wait.
        let silent = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let url =
            reqwest::Url::parse(&format!("http://{}/", silent.local_addr().unwrap())).unwrap();
        let directory = tempfile::tempdir().unwrap();
        let app = script(directory.path(), "app", "exit 3");
        let mut started = start(&app, 4321, "production");
        let client = super::super::super::load::client().unwrap();
        let error = block_on(started.wait_ready(&client, &url)).unwrap_err();
        assert!(
            matches!(&error, StartError::Exited(status) if status.code() == Some(3)),
            "{error}"
        );
    }
}
