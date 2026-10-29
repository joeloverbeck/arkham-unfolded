use super::*;
use std::{
    io::{BufRead, BufReader},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
};
use zbus::{message::Header, zvariant::Value};

#[derive(Default)]
struct Seen {
    calls: Vec<String>,
}
type Shared = Arc<Mutex<Seen>>;

struct Session(Shared);
#[zbus::interface(name = "org.freedesktop.portal.Session")]
impl Session {
    fn close(&self) {
        self.0.lock().unwrap().calls.push("session-close".into());
    }
}

struct Request(Shared);
#[zbus::interface(name = "org.freedesktop.portal.Request")]
impl Request {
    fn close(&self) {
        self.0.lock().unwrap().calls.push("request-close".into());
    }
}

fn sender(header: &Header<'_>) -> String {
    header
        .sender()
        .unwrap()
        .as_str()
        .trim_start_matches(':')
        .replace('.', "_")
}

async fn respond(
    connection: &zbus::Connection,
    header: &Header<'_>,
    options: Values,
    shared: Shared,
    code: u32,
    values: Values,
    silent: bool,
) -> OwnedObjectPath {
    let token = <&str>::try_from(options.get("handle_token").unwrap()).unwrap();
    let path =
        OwnedObjectPath::try_from(format!("{DESKTOP}/request/{}/{token}", sender(header))).unwrap();
    connection
        .object_server()
        .at(path.clone(), Request(shared))
        .await
        .unwrap();
    if !silent {
        // Deliberately emit before the method reply: this catches subscribe-after-call races.
        connection
            .emit_signal(
                None::<&str>,
                path.as_str(),
                "org.freedesktop.portal.Request",
                "Response",
                &(code, values),
            )
            .await
            .unwrap();
    }
    path
}

struct Remote {
    shared: Shared,
    mode: &'static str,
}
#[zbus::interface(name = "org.freedesktop.portal.RemoteDesktop")]
impl Remote {
    async fn create_session(
        &self,
        options: Values,
        #[zbus(connection)] connection: &zbus::Connection,
        #[zbus(header)] header: Header<'_>,
    ) -> OwnedObjectPath {
        self.shared.lock().unwrap().calls.push("create".into());
        let token = <&str>::try_from(options.get("session_handle_token").unwrap()).unwrap();
        let session = format!("{DESKTOP}/session/{}/{token}", sender(&header));
        connection
            .object_server()
            .at(session.clone(), Session(self.shared.clone()))
            .await
            .unwrap();
        let values = HashMap::from([("session_handle".into(), Str::from(session).into())]);
        respond(
            connection,
            &header,
            options,
            self.shared.clone(),
            0,
            values,
            false,
        )
        .await
    }
    async fn select_devices(
        &self,
        _session: OwnedObjectPath,
        options: Values,
        #[zbus(connection)] connection: &zbus::Connection,
        #[zbus(header)] header: Header<'_>,
    ) -> OwnedObjectPath {
        assert_eq!(u32::try_from(options.get("types").unwrap()).unwrap(), 2);
        assert_eq!(
            u32::try_from(options.get("persist_mode").unwrap()).unwrap(),
            0
        );
        self.shared
            .lock()
            .unwrap()
            .calls
            .push("pointer-only-nonpersistent".into());
        respond(
            connection,
            &header,
            options,
            self.shared.clone(),
            0,
            Values::new(),
            false,
        )
        .await
    }
    async fn start(
        &self,
        _session: OwnedObjectPath,
        parent: String,
        options: Values,
        #[zbus(connection)] connection: &zbus::Connection,
        #[zbus(header)] header: Header<'_>,
    ) -> OwnedObjectPath {
        assert!(parent.is_empty());
        self.shared.lock().unwrap().calls.push("start".into());
        let properties: Values = HashMap::from([
            (
                "logical_size".into(),
                Value::new((1920i32, 1080i32)).try_to_owned().unwrap(),
            ),
            ("source_type".into(), 1u32.into()),
        ]);
        let values = HashMap::from([
            (
                "devices".into(),
                (if self.mode == "keyboard" { 3u32 } else { 2u32 }).into(),
            ),
            (
                "streams".into(),
                Value::new(vec![(42u32, properties)])
                    .try_to_owned()
                    .unwrap(),
            ),
        ]);
        respond(
            connection,
            &header,
            options,
            self.shared.clone(),
            if self.mode == "cancel" { 1 } else { 0 },
            values,
            self.mode == "timeout",
        )
        .await
    }
    fn notify_pointer_motion_absolute(
        &self,
        _session: OwnedObjectPath,
        _options: Values,
        stream: u32,
        x: f64,
        y: f64,
    ) {
        assert_eq!(stream, 42);
        self.shared
            .lock()
            .unwrap()
            .calls
            .push(format!("move:{x},{y}"));
    }
    fn notify_pointer_button(
        &self,
        _session: OwnedObjectPath,
        _options: Values,
        button: i32,
        state: u32,
    ) {
        self.shared
            .lock()
            .unwrap()
            .calls
            .push(format!("button:{button},{state}"));
    }
}

struct Cast {
    shared: Shared,
    mode: &'static str,
}
#[zbus::interface(name = "org.freedesktop.portal.ScreenCast")]
impl Cast {
    async fn select_sources(
        &self,
        _session: OwnedObjectPath,
        options: Values,
        #[zbus(connection)] connection: &zbus::Connection,
        #[zbus(header)] header: Header<'_>,
    ) -> OwnedObjectPath {
        assert_eq!(u32::try_from(options.get("types").unwrap()).unwrap(), 1);
        assert!(!bool::try_from(options.get("multiple").unwrap()).unwrap());
        self.shared.lock().unwrap().calls.push("one-monitor".into());
        respond(
            connection,
            &header,
            options,
            self.shared.clone(),
            if self.mode == "partial" { 2 } else { 0 },
            Values::new(),
            false,
        )
        .await
    }
}

struct Bus {
    child: Child,
    _directory: tempfile::TempDir,
    server: Connection,
    address: String,
    shared: Shared,
}
impl Bus {
    fn new(mode: &'static str) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let mut child = Command::new("dbus-daemon")
            .args(["--session", "--nofork", "--print-address=1"])
            .arg(format!(
                "--address=unix:path={}",
                directory.path().join("bus").display()
            ))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut address = String::new();
        BufReader::new(child.stdout.take().unwrap())
            .read_line(&mut address)
            .unwrap();
        if address.is_empty() {
            let output = child.wait_with_output().unwrap();
            panic!(
                "private test bus did not start: {}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        let address = address.trim().to_owned();
        let shared = Arc::new(Mutex::new(Seen::default()));
        let server = Builder::address(address.as_str())
            .unwrap()
            .name(DESTINATION)
            .unwrap()
            .serve_at(
                DESKTOP,
                Remote {
                    shared: shared.clone(),
                    mode,
                },
            )
            .unwrap()
            .serve_at(
                DESKTOP,
                Cast {
                    shared: shared.clone(),
                    mode,
                },
            )
            .unwrap()
            .build()
            .unwrap();
        Self {
            child,
            _directory: directory,
            server,
            address,
            shared,
        }
    }
    fn portal(&self, seconds: f64) -> DesktopPortal {
        let connection = Builder::address(self.address.as_str())
            .unwrap()
            .method_timeout(Duration::from_secs(2))
            .build()
            .unwrap();
        DesktopPortal::new(
            connection,
            Instant::now() + Duration::from_secs_f64(seconds),
            Arc::new(AtomicBool::new(false)),
        )
    }
}
impl Drop for Bus {
    fn drop(&mut self) {
        let _ = self.server.clone().close();
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[test]
fn persistent_dbus_transport_requests_scoped_grant_and_releases_on_close() {
    let bus = Bus::new("success");
    let mut portal = bus.portal(5.0);
    let mut events = Vec::new();
    portal
        .start(&mut |event| {
            events.push(event);
            Ok(())
        })
        .unwrap();
    assert_eq!(portal.size(), [1920.0, 1080.0]);
    portal.move_to([123.0, 234.0]).unwrap();
    portal.button(true, 273).unwrap();
    portal.close().unwrap();
    portal.close().unwrap();
    assert_eq!(
        bus.shared.lock().unwrap().calls,
        [
            "create",
            "pointer-only-nonpersistent",
            "one-monitor",
            "start",
            "move:123,234",
            "button:273,1",
            "button:273,0",
            "session-close"
        ]
    );
    assert_eq!(events[0]["status"], "awaiting-desktop-consent");
    assert_eq!(events[1]["status"], "granted");
}

#[test]
fn canceled_partial_or_overbroad_grants_close_the_created_session() {
    for mode in ["cancel", "partial", "keyboard"] {
        let bus = Bus::new(mode);
        let mut portal = bus.portal(5.0);
        assert!(portal.start(&mut |_| Ok(())).is_err(), "{mode}");
        portal.close().unwrap();
        assert_eq!(
            bus.shared.lock().unwrap().calls.last().unwrap(),
            "session-close"
        );
    }
}

#[test]
fn missing_response_times_out_and_closes_request_and_session() {
    let bus = Bus::new("timeout");
    let mut portal = bus.portal(0.25);
    let result = portal.start(&mut |_| Ok(()));
    assert!(format!("{:#}", result.unwrap_err()).contains("timed out"));
    portal.close().unwrap();
    let state = bus.shared.lock().unwrap();
    assert!(state.calls.iter().any(|call| call == "request-close"));
    assert_eq!(state.calls.last().unwrap(), "session-close");
}

#[test]
fn closing_command_input_cancels_pending_consent_and_closes_resources() {
    let bus = Bus::new("timeout");
    let mut portal = bus.portal(2.0);
    let (reader, writer) = std::os::unix::net::UnixStream::pair().unwrap();
    let (_commands, input_ended) = crate::native::input_lines(BufReader::new(reader));
    portal.input_ended = input_ended;
    let shared = bus.shared.clone();
    let closer = std::thread::spawn(move || {
        let deadline = Instant::now() + Duration::from_secs(1);
        while !shared
            .lock()
            .unwrap()
            .calls
            .iter()
            .any(|call| call == "start")
        {
            assert!(Instant::now() < deadline, "consent never started");
            std::thread::sleep(Duration::from_millis(5));
        }
        drop(writer);
    });
    let start = Instant::now();
    let result = portal.start(&mut |_| Ok(()));
    portal.close().unwrap();
    closer.join().unwrap();
    assert!(format!("{:#}", result.unwrap_err()).contains("input ended"));
    assert!(start.elapsed() < Duration::from_secs(1));
    let calls = &bus.shared.lock().unwrap().calls;
    assert!(calls.iter().any(|call| call == "request-close"));
    assert_eq!(calls.last().unwrap(), "session-close");
    assert!(!calls.iter().any(|call| call.starts_with("move:")));
}
