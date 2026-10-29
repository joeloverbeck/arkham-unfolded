use crate::{interaction::Portal, process};
use anyhow::{Context, Result, bail, ensure};
use futures_lite::StreamExt;
use serde::Serialize;
use serde_json::{Value as Json, json};
use std::{
    collections::{BTreeSet, HashMap},
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};
use zbus::{
    MatchRule,
    blocking::{Connection, MessageIterator, connection::Builder},
    message::Type,
    zvariant::{DynamicType, OwnedObjectPath, OwnedValue, Str},
};

const DESTINATION: &str = "org.freedesktop.portal.Desktop";
const DESKTOP: &str = "/org/freedesktop/portal/desktop";
const REMOTE: &str = "org.freedesktop.portal.RemoteDesktop";
const CAST: &str = "org.freedesktop.portal.ScreenCast";
type Values = HashMap<String, OwnedValue>;

fn token() -> String {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    format!(
        "arkham_{}_{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    )
}

fn options(token: &str) -> Values {
    HashMap::from([("handle_token".into(), Str::from(token.to_owned()).into())])
}

pub struct DesktopPortal {
    connection: Option<Connection>,
    session: Option<OwnedObjectPath>,
    stream: u32,
    size: [f64; 2],
    pressed: BTreeSet<i32>,
    deadline: Instant,
    input_ended: Arc<AtomicBool>,
}

impl DesktopPortal {
    pub fn connect(deadline: Instant, input_ended: Arc<AtomicBool>) -> Result<Self> {
        let connection = Builder::session()?
            .method_timeout(Duration::from_secs(5))
            .build()?;
        Ok(Self::new(connection, deadline, input_ended))
    }

    fn new(connection: Connection, deadline: Instant, input_ended: Arc<AtomicBool>) -> Self {
        Self {
            connection: Some(connection),
            session: None,
            stream: 0,
            size: [0.0; 2],
            pressed: BTreeSet::new(),
            deadline,
            input_ended,
        }
    }

    fn connection(&self) -> Result<&Connection> {
        self.connection
            .as_ref()
            .context("Portal connection is closed")
    }

    fn check_active(&self) -> Result<()> {
        process::check_interrupt()?;
        ensure!(
            !self.input_ended.load(Ordering::SeqCst),
            "Command input ended or its reader stopped"
        );
        ensure!(
            Instant::now() < self.deadline,
            "Portal request or session timed out"
        );
        Ok(())
    }

    fn call<B: Serialize + DynamicType>(
        &self,
        path: &str,
        interface: &str,
        method: &str,
        body: &B,
    ) -> Result<zbus::Message> {
        Ok(self.connection()?.call_method(
            Some(DESTINATION),
            path,
            Some(interface),
            method,
            body,
        )?)
    }

    fn request<B: Serialize + DynamicType>(
        &self,
        interface: &str,
        method: &str,
        body: impl FnOnce(&str) -> B,
    ) -> Result<Values> {
        self.check_active()?;
        let connection = self.connection()?;
        let token = token();
        let sender = connection
            .unique_name()
            .context("Portal requires a session bus connection")?
            .as_str()
            .trim_start_matches(':')
            .replace('.', "_");
        let path = format!("{DESKTOP}/request/{sender}/{token}");
        let rule = MatchRule::builder()
            .msg_type(Type::Signal)
            .sender(DESTINATION)?
            .interface("org.freedesktop.portal.Request")?
            .member("Response")?
            .path(path.as_str())?
            .build();
        // Subscribe before invoking: a portal may emit Response before returning its handle.
        let mut responses =
            MessageIterator::for_match_rule(rule, connection, Some(1))?.into_inner();
        let mut completed = false;
        let result = (|| {
            let reply = self.call(DESKTOP, interface, method, &body(&token))?;
            let handle: OwnedObjectPath = reply.body().deserialize()?;
            ensure!(
                handle.as_str() == path,
                "Portal returned an unexpected request identity"
            );
            loop {
                self.check_active()?;
                let message = async_io::block_on(futures_lite::future::or(
                    async { Some(responses.next().await) },
                    async {
                        async_io::Timer::after(Duration::from_millis(50)).await;
                        None
                    },
                ));
                if let Some(message) = message {
                    let message = message.context("Portal response stream closed")??;
                    let (code, values): (u32, Values) = message.body().deserialize()?;
                    completed = true;
                    ensure!(code == 0, "Portal request denied or canceled: {code}");
                    return Ok(values);
                }
            }
        })();
        if !completed {
            // Close any consent dialog even if the method or response was interrupted.
            let _ = self.call(&path, "org.freedesktop.portal.Request", "Close", &());
        }
        result
    }

    fn session(&self) -> Result<&OwnedObjectPath> {
        self.session.as_ref().context("Portal session is not open")
    }
}

impl Portal for DesktopPortal {
    fn start(&mut self, emit: &mut dyn FnMut(Json) -> Result<()>) -> Result<()> {
        let session_token = token();
        let sender = self
            .connection()?
            .unique_name()
            .context("Session bus has no unique name")?
            .as_str()
            .trim_start_matches(':')
            .replace('.', "_");
        let expected =
            OwnedObjectPath::try_from(format!("{DESKTOP}/session/{sender}/{session_token}"))?;
        // Keep the predicted identity before CreateSession so partial startup can close it.
        self.session = Some(expected.clone());
        let mut values = self.request(REMOTE, "CreateSession", |token| {
            let mut options = options(token);
            options.insert(
                "session_handle_token".into(),
                Str::from(session_token).into(),
            );
            (options,)
        })?;
        let returned = String::try_from(
            values
                .remove("session_handle")
                .context("Portal supplied no session handle")?,
        )?;
        ensure!(
            returned == expected.as_str(),
            "Portal returned an unexpected session identity"
        );
        self.request(REMOTE, "SelectDevices", |token| {
            let mut options = options(token);
            options.insert("types".into(), 2u32.into());
            options.insert("persist_mode".into(), 0u32.into());
            (expected.clone(), options)
        })?;
        self.request(CAST, "SelectSources", |token| {
            let mut options = options(token);
            options.insert("types".into(), 1u32.into());
            options.insert("multiple".into(), false.into());
            (expected.clone(), options)
        })?;
        emit(json!({"status":"awaiting-desktop-consent"}))?;
        let mut grant = self.request(REMOTE, "Start", |token| {
            (expected.clone(), "", options(token))
        })?;
        let devices = u32::try_from(
            grant
                .remove("devices")
                .context("Portal supplied no devices")?,
        )?;
        let mut streams: Vec<(u32, Values)> = grant
            .remove("streams")
            .context("Portal supplied no monitor")?
            .try_into()?;
        ensure!(
            devices == 2 && streams.len() == 1,
            "Grant must include only pointer access and exactly one monitor"
        );
        let (stream, mut properties) = streams.pop().unwrap();
        if let Some(source) = properties.remove("source_type") {
            ensure!(
                u32::try_from(source)? == 1,
                "Selected source is not a monitor"
            );
        }
        let extent = properties
            .remove("logical_size")
            .or_else(|| properties.remove("size"))
            .context("Portal supplied no monitor extent")?;
        let (width, height): (i32, i32) = extent.try_into()?;
        ensure!(
            width >= 100 && height >= 100,
            "Portal supplied an unusable monitor extent"
        );
        self.stream = stream;
        self.size = [width.into(), height.into()];
        emit(json!({"status":"granted", "stream_size":[width,height]}))?;
        Ok(())
    }

    fn size(&self) -> [f64; 2] {
        self.size
    }

    fn move_to(&mut self, point: [f64; 2]) -> Result<()> {
        self.check_active()?;
        ensure!(
            point
                .iter()
                .zip(self.size)
                .all(|(value, bound)| value.is_finite() && *value >= 0.0 && *value < bound),
            "Pointer coordinate is outside the selected monitor"
        );
        self.call(
            DESKTOP,
            REMOTE,
            "NotifyPointerMotionAbsolute",
            &(
                self.session()?,
                Values::new(),
                self.stream,
                point[0],
                point[1],
            ),
        )?;
        Ok(())
    }

    fn button(&mut self, pressed: bool, button: i32) -> Result<()> {
        ensure!([272, 273].contains(&button), "Unsupported pointer button");
        if pressed {
            self.check_active()?;
            // A failed reply does not prove the press was undelivered.
            self.pressed.insert(button);
        }
        self.call(
            DESKTOP,
            REMOTE,
            "NotifyPointerButton",
            &(self.session()?, Values::new(), button, u32::from(pressed)),
        )?;
        if !pressed {
            self.pressed.remove(&button);
        }
        Ok(())
    }

    fn close(&mut self) -> Result<()> {
        if self.connection.is_none() {
            return Ok(());
        }
        let mut failure = None;
        for button in self.pressed.clone() {
            if let Err(error) = self.button(false, button) {
                failure = Some(error);
            }
        }
        if let Some(session) = self.session.take()
            && let Err(error) = self.call(
                session.as_str(),
                "org.freedesktop.portal.Session",
                "Close",
                &(),
            )
        {
            let already_closed = error.downcast_ref::<zbus::Error>().is_some_and(|e| matches!(e, zbus::Error::MethodError(name, _, _) if name.as_str() == "org.freedesktop.DBus.Error.UnknownObject"));
            if !already_closed {
                failure = Some(error);
            }
        }
        if let Some(connection) = self.connection.take()
            && let Err(error) = connection.close()
        {
            failure = Some(error.into());
        }
        if let Some(error) = failure {
            bail!("Portal cleanup failed: {error:#}");
        }
        Ok(())
    }
}

impl Drop for DesktopPortal {
    fn drop(&mut self) {
        if let Err(error) = self.close() {
            eprintln!("Portal cleanup failed: {error:#}");
        }
    }
}

#[cfg(test)]
#[path = "portal_tests.rs"]
mod tests;
