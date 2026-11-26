//! Rex multiplexer implementation of `AgentHost` using `rex` CLI and OSC 7501 status records.

use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use serde_json::Value;

use super::{is_known_agent, AgentHost, AgentState, AgentStatus, AgentTarget};

const TARGET_ID_SEP: char = '\x1f';

const PANES_SCRIPT: &str = r#"
local session = rex.session_id
local blocks
if rex.args.only_block then
  blocks = { { block_id = rex.block_id, creator_name = "com.superlogical.terminal" } }
else
  blocks = (rex.call("session.list_blocks", { session_id = session }) or {}).blocks or {}
end
local panes = {}
for _, block in ipairs(blocks) do
  if block.creator_name == "com.superlogical.terminal" then
    local function ask(method)
      local ok, answer = pcall(rex.call, "block.method",
        { session_id = session, block_id = block.block_id, method = method })
      return ok and answer or nil
    end
    panes[#panes + 1] =
      { block = block.block_id, process = ask("process"), status = ask("program_status") }
  end
end
return panes
"#;

struct RexSession {
    id: String,
    label: String,
}

struct RexStatusRecord {
    app: String,
    state: String,
    updated_at: String,
    owner_pid: Option<u64>,
}

struct RexPane {
    block: String,
    cwd: PathBuf,
    foreground_pid: Option<u64>,
    statuses: Vec<RexStatusRecord>,
}

trait RexTransport: Send + Sync {
    fn sessions(&self) -> Vec<RexSession>;
    fn terminal_panes(&self, session: &str) -> Vec<RexPane>;
    fn terminal_pane(&self, session: &str, block: &str) -> Option<RexPane>;
    fn submit_message(&self, session: &str, block: &str, message: &str) -> Result<(), String>;
}

#[derive(Clone, Copy, Default)]
pub struct RexCli;

impl RexCli {
    fn command(args: &[&str]) -> Command {
        let mut command = Command::new("rex");
        command
            .args(["--autostart=false", "--timeout", "3s"])
            .args(args)
            .stdin(Stdio::null());
        command
    }

    fn json(args: &[&str]) -> Option<Value> {
        let output = Self::command(args).arg("--json").output().ok()?;
        if !output.status.success() {
            return None;
        }
        serde_json::from_slice(&output.stdout).ok()
    }

    fn panes(session: &str, only_block: Option<&str>) -> Vec<RexPane> {
        let mut args = vec!["do", "-s", session];
        if let Some(block) = only_block {
            args.extend(["-b", block, "--args", r#"{"only_block":true}"#]);
        }
        args.extend(["-e", PANES_SCRIPT]);
        Self::json(&args)
            .map(|panes| panes_from(&panes))
            .unwrap_or_default()
    }

    fn run(args: &[&str], input: &str) -> Result<(), String> {
        let cannot_run = |error: std::io::Error| format!("cannot run rex: {error}");
        let mut child = Self::command(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(cannot_run)?;
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(input.as_bytes()).map_err(cannot_run)?;
        }
        let output = child.wait_with_output().map_err(cannot_run)?;
        if output.status.success() {
            Ok(())
        } else {
            Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
        }
    }
}

impl RexTransport for RexCli {
    fn sessions(&self) -> Vec<RexSession> {
        let Some(json) = Self::json(&["ls"]) else {
            return Vec::new();
        };
        json["sessions"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|session| RexSession {
                id: text(session, "session_id"),
                label: text(session, "label"),
            })
            .collect()
    }

    fn terminal_panes(&self, session: &str) -> Vec<RexPane> {
        Self::panes(session, None)
    }

    fn terminal_pane(&self, session: &str, block: &str) -> Option<RexPane> {
        Self::panes(session, Some(block)).into_iter().next()
    }

    fn submit_message(&self, session: &str, block: &str, message: &str) -> Result<(), String> {
        // Bracketed paste delivers newlines as prompt text rather than submitting early.
        let paste = format!("\x1b[200~{message}\x1b[201~");
        Self::run(&["send", "-s", session, "-b", block], &paste)?;
        Self::run(&["send-key", "-s", session, "-b", block, "enter"], "")
    }
}

fn text(value: &Value, key: &str) -> String {
    value[key].as_str().unwrap_or_default().to_string()
}

fn panes_from(panes: &Value) -> Vec<RexPane> {
    let pane_from = |pane: &Value| {
        let foreground = &pane["process"]["foreground"];
        let first_program = &pane["process"]["child"];
        let cwd = [foreground, first_program]
            .into_iter()
            .map(|program| text(program, "cwd"))
            .find(|cwd| !cwd.is_empty())
            .unwrap_or_default();
        let status_from = |record: &Value| RexStatusRecord {
            app: text(record, "app"),
            state: text(record, "state"),
            updated_at: text(record, "updated_at"),
            owner_pid: record["owner"]["pid"].as_u64(),
        };
        let records = pane["status"]["records"].as_array();
        RexPane {
            block: text(pane, "block"),
            cwd: PathBuf::from(cwd),
            foreground_pid: foreground["pid"].as_u64(),
            statuses: records.into_iter().flatten().map(status_from).collect(),
        }
    };
    panes
        .as_array()
        .into_iter()
        .flatten()
        .map(pane_from)
        .collect()
}

fn encode_target_id(session: &str, block: &str) -> String {
    format!("{session}{TARGET_ID_SEP}{block}")
}

fn decode_target_id(id: &str) -> Option<(&str, &str)> {
    id.split_once(TARGET_ID_SEP)
}

/// Only statuses reported by the current foreground PID count so exited agents never
/// let comments get pasted into an interactive shell.
fn running_agent(pane: &RexPane) -> Option<AgentStatus> {
    let record = pane.statuses.iter().find(|status| {
        is_known_agent(&status.app)
            && status.owner_pid.is_some()
            && status.owner_pid == pane.foreground_pid
    })?;
    Some(AgentStatus {
        app: record.app.clone(),
        state: AgentState::parse(&record.state),
        updated_at: record.updated_at.clone(),
    })
}

fn to_target(session: &RexSession, pane: RexPane) -> Option<AgentTarget> {
    let status = running_agent(&pane)?;
    Some(AgentTarget {
        id: encode_target_id(&session.id, &pane.block),
        location: session.label.clone(),
        cwd: pane.cwd,
        status,
    })
}

pub struct RexHost<T = RexCli> {
    transport: T,
    current_session: Option<Option<String>>,
}

impl RexHost<RexCli> {
    pub fn new() -> Self {
        Self {
            transport: RexCli,
            current_session: None,
        }
    }
}

impl Default for RexHost<RexCli> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> RexHost<T> {
    #[cfg(test)]
    fn with_transport(transport: T, current_session: Option<String>) -> Self {
        Self {
            transport,
            current_session: Some(current_session),
        }
    }

    fn active_session(&self) -> Option<String> {
        match &self.current_session {
            Some(configured) => configured.clone(),
            None => std::env::var("REX_SESSION").ok(),
        }
    }
}

impl<T: RexTransport> AgentHost for RexHost<T> {
    fn name(&self) -> &'static str {
        "rex"
    }

    fn list_agents(&self, review_roots: &[PathBuf]) -> Vec<AgentTarget> {
        let mut targets = Vec::new();
        for session in self.transport.sessions() {
            for pane in self.transport.terminal_panes(&session.id) {
                let target = to_target(&session, pane)
                    .filter(|target| review_roots.iter().any(|root| target.cwd.starts_with(root)));
                targets.extend(target);
            }
        }
        targets.sort_by(|a, b| b.status.updated_at.cmp(&a.status.updated_at));
        targets
    }

    fn find_agent(&self, target_hint: &str) -> Option<AgentTarget> {
        let (session_id, block) = match decode_target_id(target_hint) {
            Some((session, block)) => (session.to_string(), block),
            None => (self.active_session()?, target_hint),
        };
        let session = RexSession {
            id: session_id.clone(),
            label: String::new(),
        };
        self.transport
            .terminal_pane(&session_id, block)
            .and_then(|pane| to_target(&session, pane))
    }

    fn inspect_agent(&self, id: &str) -> Option<AgentStatus> {
        let (session, block) = decode_target_id(id)?;
        self.transport
            .terminal_pane(session, block)
            .as_ref()
            .and_then(running_agent)
    }

    fn send_message(&self, id: &str, message: &str) -> Result<(), String> {
        let Some((session, block)) = decode_target_id(id) else {
            return Err("invalid rex target".to_string());
        };
        self.transport.submit_message(session, block, message)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;
    use crate::review::agent::AgentLink;

    const AGENT_PID: u64 = 7;
    const SHELL_PID: u64 = 1;

    #[derive(Clone)]
    struct FakePane {
        session: &'static str,
        block: &'static str,
        app: &'static str,
        state: &'static str,
        cwd: &'static str,
        updated_at: &'static str,
        foreground_pid: u64,
    }

    struct FakeTransport {
        panes: Mutex<Vec<FakePane>>,
        typed: Mutex<Vec<(String, String)>>,
    }

    impl FakeTransport {
        fn with(panes: Vec<FakePane>) -> Self {
            Self {
                panes: Mutex::new(panes),
                typed: Mutex::new(Vec::new()),
            }
        }
    }

    fn described(pane: &FakePane) -> RexPane {
        let status = RexStatusRecord {
            app: pane.app.to_string(),
            state: pane.state.to_string(),
            updated_at: pane.updated_at.to_string(),
            owner_pid: Some(AGENT_PID),
        };
        RexPane {
            block: pane.block.to_string(),
            cwd: PathBuf::from(pane.cwd),
            foreground_pid: Some(pane.foreground_pid),
            statuses: if pane.app.is_empty() {
                Vec::new()
            } else {
                vec![status]
            },
        }
    }

    impl RexTransport for FakeTransport {
        fn sessions(&self) -> Vec<RexSession> {
            let panes = self.panes.lock().unwrap();
            let mut ids: Vec<&str> = panes.iter().map(|pane| pane.session).collect();
            ids.dedup();
            ids.into_iter()
                .map(|id| RexSession {
                    id: id.to_string(),
                    label: format!("label of {id}"),
                })
                .collect()
        }

        fn terminal_panes(&self, session: &str) -> Vec<RexPane> {
            self.panes
                .lock()
                .unwrap()
                .iter()
                .filter(|pane| pane.session == session)
                .map(described)
                .collect()
        }

        fn terminal_pane(&self, session: &str, block: &str) -> Option<RexPane> {
            self.panes
                .lock()
                .unwrap()
                .iter()
                .find(|pane| pane.session == session && pane.block == block)
                .map(described)
        }

        fn submit_message(&self, _session: &str, block: &str, message: &str) -> Result<(), String> {
            self.typed
                .lock()
                .unwrap()
                .push((block.to_string(), message.to_string()));
            Ok(())
        }
    }

    fn pane(
        session: &'static str,
        block: &'static str,
        app: &'static str,
        cwd: &'static str,
        updated_at: &'static str,
    ) -> FakePane {
        FakePane {
            session,
            block,
            app,
            state: "idle",
            cwd,
            updated_at,
            foreground_pid: AGENT_PID,
        }
    }

    fn roots() -> Vec<PathBuf> {
        vec![PathBuf::from("/work/api"), PathBuf::from("/work")]
    }

    fn blocks(targets: &[AgentTarget]) -> Vec<&str> {
        targets
            .iter()
            .filter_map(|target| decode_target_id(&target.id).map(|(_, block)| block))
            .collect()
    }

    #[test]
    fn test_attaches_to_the_latest_agent_pane_in_or_under_a_review_root() {
        let host = Arc::new(RexHost::with_transport(
            FakeTransport::with(vec![
                pane("s1", "older", "pi", "/work/api", "2026-11-18T10:00:00Z"),
                pane("s1", "shell", "", "/work", "2026-11-18T13:00:00Z"),
                pane("s1", "build", "cargo", "/work", "2026-11-18T13:00:00Z"),
                pane(
                    "s1",
                    "subdirectory",
                    "codex",
                    "/work/api/src",
                    "2026-11-18T12:00:00Z",
                ),
                pane("s2", "elsewhere", "pi", "/other", "2026-11-18T13:00:00Z"),
                pane("s2", "sibling", "pi", "/workshop", "2026-11-18T13:00:00Z"),
                pane(
                    "s2",
                    "at-the-root",
                    "claude-code",
                    "/work",
                    "2026-11-18T11:00:00Z",
                ),
            ]),
            Some("s1".to_string()),
        ));

        let mut link = AgentLink::new(host, PathBuf::from("/work"));
        link.attach(None, &roots());

        assert_eq!(link.host_name(), "rex");
        assert_eq!(
            link.attached_id().and_then(decode_target_id),
            Some(("s1", "subdirectory"))
        );
        assert_eq!(link.label(), "codex idle");
        assert_eq!(
            blocks(link.known_candidates()),
            vec!["subdirectory", "at-the-root", "older"]
        );
        assert_eq!(
            link.known_candidates()[1].picker_label(),
            "claude-code · label of s2 · idle · /work"
        );
    }

    #[test]
    fn test_exited_agent_and_target_hint_guards() {
        let mut back_at_shell = pane("s1", "quit", "pi", "/work", "2026-11-18T12:00:00Z");
        back_at_shell.state = "done";
        back_at_shell.foreground_pid = SHELL_PID;
        let host = Arc::new(RexHost::with_transport(
            FakeTransport::with(vec![back_at_shell]),
            Some("s1".to_string()),
        ));
        let mut link = AgentLink::new(host, PathBuf::from("/work"));
        link.attach(None, &roots());
        assert!(!link.is_attached());

        let make_host = |session: Option<&str>| {
            Arc::new(RexHost::with_transport(
                FakeTransport::with(vec![
                    pane(
                        "s1",
                        "given",
                        "pi",
                        "/somewhere/else",
                        "2026-11-18T10:00:00Z",
                    ),
                    pane("s1", "shell", "", "/work", "2026-11-18T10:00:00Z"),
                    pane("s1", "matching", "pi", "/work", "2026-11-18T12:00:00Z"),
                ]),
                session.map(str::to_string),
            ))
        };

        let mut given_link = AgentLink::new(make_host(Some("s1")), PathBuf::from("/work"));
        given_link.attach(Some("given"), &roots());
        assert_eq!(
            given_link.attached_id().and_then(decode_target_id),
            Some(("s1", "given"))
        );

        let mut shell_link = AgentLink::new(make_host(Some("s1")), PathBuf::from("/work"));
        shell_link.attach(Some("shell"), &roots());
        assert_eq!(shell_link.attached_id(), None);

        let mut no_session_link = AgentLink::new(make_host(None), PathBuf::from("/work"));
        no_session_link.attach(Some("given"), &roots());
        assert_eq!(no_session_link.attached_id(), None);
    }

    #[test]
    fn test_panes_are_read_from_what_rex_returns() {
        let panes = serde_json::json!([
            {
                "block": "block:agent",
                "process": {
                    "child": { "pid": 1, "argv0": "zsh", "cwd": "/work" },
                    "foreground": { "pid": 7, "argv0": "pi", "cwd": "/work/api" }
                },
                "status": { "records": [ {
                    "state": "working", "app": "pi", "updated_at": "2026-11-23T16:18:13Z",
                    "owner": { "pid": 7, "name": "node" }
                } ] }
            },
            {
                "block": "block:wrapped",
                "process": {
                    "child": { "pid": 1, "argv0": "zsh", "cwd": "/work" },
                    "foreground": { "pid": 9, "argv0": "bash" }
                },
                "status": { "records": {} }
            }
        ]);

        let panes = panes_from(&panes);
        assert_eq!(panes.len(), 2);
        assert_eq!(panes[0].cwd, PathBuf::from("/work/api"));
        let agent = running_agent(&panes[0]).unwrap();
        assert_eq!(
            (agent.app.as_str(), agent.state),
            ("pi", AgentState::Working)
        );
        assert_eq!(panes[1].cwd, PathBuf::from("/work"));
        assert!(running_agent(&panes[1]).is_none());
        assert!(panes_from(&serde_json::json!({})).is_empty());
    }
}
