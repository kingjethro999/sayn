//! Newline-delimited JSON-RPC 2.0 client for the Hermes TUI gateway.
//!
//! The gateway is an optional agent backend. This client only transports
//! messages: it never executes anything the gateway asks for. Any request the
//! gateway sends *to* Sayn (approvals, tool calls, file access) is answered
//! with JSON-RPC error `-32601`, because Sayn has not implemented a mediated
//! path for it. Every wait is bounded, and the child process is always reaped.

use crate::RuntimeError;
use serde_json::{Value, json};
use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError};
use std::thread;
use std::time::{Duration, Instant};

/// JSON-RPC "method not found", used to refuse gateway-initiated requests.
pub const METHOD_NOT_FOUND: i64 = -32601;

/// Longest accepted line from the gateway.
const MAX_LINE_BYTES: usize = 8 * 1024 * 1024;

/// Bounds for a gateway connection.
#[derive(Debug, Clone, Copy)]
pub struct RpcTimeouts {
    /// How long to wait for the `gateway.ready` event after spawning.
    pub ready: Duration,
    /// How long to wait for the response to one request.
    pub call: Duration,
    /// How long to let the process exit by itself on shutdown before killing it.
    pub shutdown: Duration,
}

impl Default for RpcTimeouts {
    fn default() -> Self {
        Self {
            ready: Duration::from_secs(30),
            call: Duration::from_secs(120),
            shutdown: Duration::from_secs(2),
        }
    }
}

/// An event published by the gateway (`{"method": "event", ...}`).
#[derive(Debug, Clone, PartialEq)]
pub struct GatewayEvent {
    pub kind: String,
    pub payload: Value,
}

/// A request the gateway sent to Sayn, which was refused.
#[derive(Debug, Clone, PartialEq)]
pub struct DeniedServerRequest {
    pub method: String,
    pub params: Value,
}

enum Incoming {
    Message(Value),
    Closed,
}

pub struct HermesRpcClient {
    child: Child,
    stdin: Option<ChildStdin>,
    incoming: Receiver<Incoming>,
    next_id: u64,
    timeouts: RpcTimeouts,
    events: VecDeque<GatewayEvent>,
    denied: Vec<DeniedServerRequest>,
    closed: bool,
}

impl HermesRpcClient {
    /// Spawn the Hermes TUI gateway from the managed runtime and wait for it
    /// to report ready.
    pub fn spawn(hermes_dir: &std::path::Path) -> Result<Self, RuntimeError> {
        let python = crate::RuntimeManager::venv_python();
        if !python.exists() {
            crate::RuntimeManager::ensure_runtime()?;
        }
        let mut command = Command::new(&python);
        command
            .args(["-m", "tui_gateway.entry"])
            .current_dir(hermes_dir);
        Self::spawn_command(command, RpcTimeouts::default())
    }

    /// Spawn an arbitrary gateway command and wait for `gateway.ready`.
    pub fn spawn_command(
        mut command: Command,
        timeouts: RpcTimeouts,
    ) -> Result<Self, RuntimeError> {
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let stdin = child.stdin.take();
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();
        let (Some(stdin), Some(stdout)) = (stdin, stdout) else {
            let _ = child.kill();
            let _ = child.wait();
            return Err(RuntimeError::RpcError(
                "the gateway process has no stdio pipes".into(),
            ));
        };

        let (sender, incoming) = mpsc::channel();
        thread::spawn(move || {
            let mut reader = BufReader::new(stdout);
            let mut line = Vec::new();
            loop {
                line.clear();
                let read = (&mut reader)
                    .take(MAX_LINE_BYTES as u64 + 1)
                    .read_until(b'\n', &mut line);
                match read {
                    Ok(0) | Err(_) => break,
                    Ok(_) if line.len() > MAX_LINE_BYTES => break,
                    Ok(_) => {}
                }
                // Lines that are not JSON objects (startup chatter) are ignored.
                let Ok(message) = serde_json::from_slice::<Value>(&line) else {
                    continue;
                };
                if message.is_object() && sender.send(Incoming::Message(message)).is_err() {
                    return;
                }
            }
            let _ = sender.send(Incoming::Closed);
        });
        // Drain stderr so a chatty gateway can never block on a full pipe.
        if let Some(mut stderr) = stderr {
            thread::spawn(move || {
                let mut sink = [0_u8; 4096];
                while matches!(stderr.read(&mut sink), Ok(count) if count > 0) {}
            });
        }

        let mut client = Self {
            child,
            stdin: Some(stdin),
            incoming,
            next_id: 1,
            timeouts,
            events: VecDeque::new(),
            denied: Vec::new(),
            closed: false,
        };
        client.wait_ready()?;
        Ok(client)
    }

    fn wait_ready(&mut self) -> Result<(), RuntimeError> {
        let deadline = Instant::now() + self.timeouts.ready;
        loop {
            let message = self.receive(deadline, "the gateway.ready event")?;
            if let Some(event) = self.absorb(message)? {
                if event.kind == "gateway.ready" {
                    return Ok(());
                }
                self.events.push_back(event);
            }
        }
    }

    /// Declare, as the gateway contract requires, that this client answers
    /// gateway-initiated requests. It answers every one of them with
    /// `-32601`, so the agent is told "no" at once instead of waiting out a
    /// deadline for an answer that will never come.
    pub fn advertise_capabilities(&mut self) -> Result<Value, RuntimeError> {
        self.call("client.capabilities", json!({"server_requests": true}))
    }

    /// Create a gateway session rooted at `cwd` and return its identifier.
    /// The session is closed by the gateway when this connection ends.
    pub fn create_session(&mut self, cwd: &str, model: &str) -> Result<String, RuntimeError> {
        let result = self.call(
            "session.create",
            json!({
                "cwd": cwd,
                "cwd_explicit": true,
                "model": model,
                "close_on_disconnect": true,
            }),
        )?;
        result
            .get("session_id")
            .and_then(Value::as_str)
            .filter(|id| !id.is_empty())
            .map(str::to_owned)
            .ok_or_else(|| RuntimeError::RpcError("session.create returned no session_id".into()))
    }

    /// Submit a prompt. Progress arrives afterwards through [`Self::next_event`].
    pub fn submit_prompt(&mut self, session_id: &str, text: &str) -> Result<Value, RuntimeError> {
        self.call(
            "prompt.submit",
            json!({"session_id": session_id, "text": text}),
        )
    }

    /// Ask the gateway to stop the session's running turn. For a gateway that
    /// does not respond, use [`Self::cancel`].
    pub fn interrupt(&mut self, session_id: &str) -> Result<Value, RuntimeError> {
        self.call("session.interrupt", json!({"session_id": session_id}))
    }

    /// Send a request and wait, bounded, for the response with the same id.
    /// Events that arrive meanwhile are queued; responses to other ids are
    /// discarded; gateway-initiated requests are refused.
    pub fn call(&mut self, method: &str, params: Value) -> Result<Value, RuntimeError> {
        let id = self.next_id;
        self.next_id += 1;
        self.send(&json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        }))?;

        let deadline = Instant::now() + self.timeouts.call;
        let waiting_for = format!("a response to '{method}'");
        loop {
            let message = self.receive(deadline, &waiting_for)?;
            if message.get("method").is_none()
                && message.get("id").and_then(Value::as_u64) == Some(id)
            {
                if let Some(error) = message.get("error") {
                    return Err(RuntimeError::RpcRemote {
                        code: error.get("code").and_then(Value::as_i64).unwrap_or(0),
                        message: error
                            .get("message")
                            .and_then(Value::as_str)
                            .unwrap_or("unspecified gateway error")
                            .to_string(),
                    });
                }
                return Ok(message.get("result").cloned().unwrap_or(Value::Null));
            }
            if let Some(event) = self.absorb(message)? {
                self.events.push_back(event);
            }
        }
    }

    /// The next gateway event, waiting up to `timeout`. `Ok(None)` means no
    /// event arrived in time.
    pub fn next_event(&mut self, timeout: Duration) -> Result<Option<GatewayEvent>, RuntimeError> {
        if let Some(event) = self.events.pop_front() {
            return Ok(Some(event));
        }
        let deadline = Instant::now() + timeout;
        loop {
            let message = match self.receive(deadline, "a gateway event") {
                Ok(message) => message,
                Err(RuntimeError::Timeout(_)) => return Ok(None),
                Err(error) => return Err(error),
            };
            if let Some(event) = self.absorb(message)? {
                return Ok(Some(event));
            }
        }
    }

    /// Requests the gateway sent to Sayn so far, all of which were refused.
    pub fn denied_requests(&self) -> &[DeniedServerRequest] {
        &self.denied
    }

    /// Operating-system process id of the gateway.
    pub fn process_id(&self) -> u32 {
        self.child.id()
    }

    /// Cancel whatever the gateway is doing by stopping the process. This does
    /// not depend on the gateway cooperating.
    pub fn cancel(&mut self) {
        self.stdin = None;
        let _ = self.child.kill();
        let _ = self.child.wait();
        self.closed = true;
    }

    /// Close the connection: let the gateway exit on end-of-input, then stop
    /// it if it has not exited within the shutdown bound.
    pub fn shutdown(mut self) {
        self.stop();
    }

    fn stop(&mut self) {
        self.stdin = None;
        let deadline = Instant::now() + self.timeouts.shutdown;
        while Instant::now() < deadline {
            if matches!(self.child.try_wait(), Ok(Some(_))) {
                self.closed = true;
                return;
            }
            thread::sleep(Duration::from_millis(20));
        }
        let _ = self.child.kill();
        let _ = self.child.wait();
        self.closed = true;
    }

    fn send(&mut self, message: &Value) -> Result<(), RuntimeError> {
        let stdin = self.stdin.as_mut().ok_or(RuntimeError::GatewayClosed)?;
        let mut line = serde_json::to_vec(message)?;
        line.push(b'\n');
        stdin
            .write_all(&line)
            .and_then(|()| stdin.flush())
            .map_err(|_| RuntimeError::GatewayClosed)
    }

    fn receive(&mut self, deadline: Instant, waiting_for: &str) -> Result<Value, RuntimeError> {
        if self.closed {
            return Err(RuntimeError::GatewayClosed);
        }
        let remaining = deadline.saturating_duration_since(Instant::now());
        match self.incoming.recv_timeout(remaining) {
            Ok(Incoming::Message(message)) => Ok(message),
            Ok(Incoming::Closed) | Err(RecvTimeoutError::Disconnected) => {
                self.closed = true;
                Err(RuntimeError::GatewayClosed)
            }
            Err(RecvTimeoutError::Timeout) => Err(RuntimeError::Timeout(waiting_for.to_string())),
        }
    }

    /// Classify a message that is not the awaited response. Returns an event
    /// to deliver, refuses gateway-initiated requests, and drops the rest.
    fn absorb(&mut self, message: Value) -> Result<Option<GatewayEvent>, RuntimeError> {
        let Some(method) = message.get("method").and_then(Value::as_str) else {
            // A response nobody is waiting for (for example after a timeout).
            return Ok(None);
        };
        let params = message.get("params").cloned().unwrap_or(Value::Null);

        if method == "event" {
            let kind = params
                .get("type")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string();
            let payload = params.get("payload").cloned().unwrap_or(Value::Null);
            return Ok(Some(GatewayEvent { kind, payload }));
        }

        // Anything else with a method is the gateway asking Sayn to do something.
        if let Some(id) = message.get("id").filter(|id| !id.is_null()) {
            self.send(&json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": {
                    "code": METHOD_NOT_FOUND,
                    "message": format!("'{method}' is not supported by the Sayn client"),
                },
            }))?;
        }
        self.denied.push(DeniedServerRequest {
            method: method.to_string(),
            params,
        });
        Ok(None)
    }
}

impl Drop for HermesRpcClient {
    fn drop(&mut self) {
        if !self.closed {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn gateway(mode: &str, timeouts: RpcTimeouts) -> Result<HermesRpcClient, RuntimeError> {
        let script =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fake_gateway.py");
        let mut command = Command::new("python3");
        command.arg(script).arg(mode);
        HermesRpcClient::spawn_command(command, timeouts)
    }

    fn quick() -> RpcTimeouts {
        RpcTimeouts {
            ready: Duration::from_secs(10),
            call: Duration::from_secs(10),
            shutdown: Duration::from_secs(2),
        }
    }

    /// Generous startup bound (interpreter start-up varies under load), short
    /// bound on calls.
    fn impatient() -> RpcTimeouts {
        RpcTimeouts {
            ready: Duration::from_secs(10),
            call: Duration::from_millis(400),
            shutdown: Duration::from_millis(200),
        }
    }

    #[cfg(unix)]
    fn process_exists(pid: u32) -> bool {
        Command::new("kill")
            .args(["-0", &pid.to_string()])
            .stderr(Stdio::null())
            .status()
            .map(|status| status.success())
            .unwrap_or(false)
    }

    #[test]
    fn handshake_session_and_prompt_round_trip() {
        let mut client = gateway("normal", quick()).unwrap();

        let capabilities = client.advertise_capabilities().unwrap();
        assert!(
            capabilities["server_requests"]
                .as_array()
                .unwrap()
                .contains(&json!("approval"))
        );

        let session = client.create_session("/tmp/project", "model-1").unwrap();
        assert_eq!(session, "sess-1");

        let accepted = client.submit_prompt(&session, "build it").unwrap();
        assert_eq!(accepted["accepted"], true);

        let event = client.next_event(Duration::from_secs(5)).unwrap().unwrap();
        assert_eq!(event.kind, "turn.complete");
        assert_eq!(event.payload["session_id"], "sess-1");

        let interrupted = client.interrupt(&session).unwrap();
        assert_eq!(interrupted["status"], "not_interrupted");
        client.shutdown();
    }

    #[test]
    fn startup_output_that_is_not_json_is_ignored() {
        let mut client = gateway("noisy_start", quick()).unwrap();
        assert_eq!(client.create_session("/tmp/p", "m").unwrap(), "sess-1");
    }

    #[test]
    fn interleaved_events_and_stale_responses_do_not_confuse_a_call() {
        let mut client = gateway("interleaved", quick()).unwrap();
        let session = client.create_session("/tmp/p", "m").unwrap();

        let accepted = client.submit_prompt(&session, "go").unwrap();

        // The response with id 9999 was not mistaken for ours.
        assert_eq!(accepted, json!({"accepted": true}));
        let kinds: Vec<String> = std::iter::from_fn(|| {
            client
                .next_event(Duration::from_secs(5))
                .unwrap()
                .map(|event| format!("{}:{}", event.kind, event.payload["text"]))
        })
        .take(2)
        .collect();
        assert_eq!(kinds, vec!["token:\"hel\"", "token:\"lo\""]);
    }

    #[test]
    fn gateway_initiated_requests_are_refused_with_method_not_found() {
        let mut client = gateway("server_request", quick()).unwrap();
        let session = client.create_session("/tmp/p", "m").unwrap();

        let result = client.submit_prompt(&session, "go").unwrap();

        let reply = &result["server_request_reply"];
        assert_eq!(reply["id"], "srq-0123456789ab");
        assert_eq!(reply["error"]["code"], METHOD_NOT_FOUND);
        assert!(reply.get("result").is_none());
        assert_eq!(client.denied_requests().len(), 1);
        assert_eq!(client.denied_requests()[0].method, "approval");
    }

    #[test]
    fn parameters_outside_the_gateway_contract_are_rejected_by_the_fake() {
        // Guards the fixture: like the real gateway it refuses unknown keys,
        // so a client that drifts from the contract fails these tests.
        let mut client = gateway("normal", quick()).unwrap();
        let error = client
            .call("session.create", json!({"workdir": "/tmp/p"}))
            .unwrap_err();
        assert!(
            matches!(error, RuntimeError::RpcRemote { code: 4000, .. }),
            "{error}"
        );
    }

    #[test]
    fn remote_errors_keep_their_code() {
        let mut client = gateway("normal", quick()).unwrap();
        let error = client.call("explode", json!({})).unwrap_err();
        assert!(matches!(
            error,
            RuntimeError::RpcRemote { code: -32000, ref message } if message == "boom"
        ));
        // The connection is still usable afterwards.
        assert_eq!(client.create_session("/tmp/p", "m").unwrap(), "sess-1");
    }

    #[test]
    fn gateway_that_never_reports_ready_times_out() {
        let started = Instant::now();
        let timeouts = RpcTimeouts {
            ready: Duration::from_millis(400),
            ..impatient()
        };
        let error = gateway("no_ready", timeouts).err().unwrap();
        assert!(matches!(error, RuntimeError::Timeout(_)), "{error}");
        assert!(started.elapsed() < Duration::from_secs(10));
    }

    #[test]
    fn unanswered_call_times_out_instead_of_blocking() {
        let mut client = gateway("hang", impatient()).unwrap();
        let started = Instant::now();

        let error = client.create_session("/tmp/p", "m").unwrap_err();

        assert!(matches!(error, RuntimeError::Timeout(_)), "{error}");
        assert!(started.elapsed() < Duration::from_secs(10));
    }

    #[test]
    fn end_of_stream_is_reported_as_a_closed_gateway() {
        let mut client = gateway("eof_after_ready", quick()).unwrap();
        let error = client.create_session("/tmp/p", "m").unwrap_err();
        assert!(matches!(error, RuntimeError::GatewayClosed), "{error}");

        let mut client = gateway("eof_mid_call", quick()).unwrap();
        let error = client.create_session("/tmp/p", "m").unwrap_err();
        assert!(matches!(error, RuntimeError::GatewayClosed), "{error}");
    }

    #[cfg(unix)]
    #[test]
    fn dropping_or_cancelling_the_client_leaves_no_gateway_process() {
        let client = gateway("normal", quick()).unwrap();
        let pid = client.process_id();
        assert!(process_exists(pid));
        drop(client);
        assert!(!process_exists(pid));

        // A gateway stuck mid-request is stopped by cancel.
        let mut client = gateway("hang", impatient()).unwrap();
        let pid = client.process_id();
        let _ = client.create_session("/tmp/p", "m");
        client.cancel();
        assert!(!process_exists(pid));
        assert!(matches!(
            client.call("session.create", json!({})),
            Err(RuntimeError::GatewayClosed)
        ));

        // shutdown lets a healthy gateway exit on end-of-input.
        let client = gateway("normal", quick()).unwrap();
        let pid = client.process_id();
        client.shutdown();
        assert!(!process_exists(pid));
    }
}
