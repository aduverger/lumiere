//! Host-agnostic coding agent integration: `AgentHost` abstraction, non-blocking
//! background discovery/polling via `AgentWorker`, and safe message delivery.

pub mod rex;

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::thread;

use super::thread::EARLIER_COMMENTS_LABEL;

const KNOWN_AGENTS: &[&str] = &[
    "pi",
    "claude-code",
    "antigravity",
    "codex",
    "opencode",
    "hermes",
    "grok",
    "grok-build",
];

const PROGRAM_NAME: &str = "lumiere";

pub fn is_known_agent(app: &str) -> bool {
    KNOWN_AGENTS.contains(&app)
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum AgentState {
    Idle,
    Working,
    Blocked,
    Done,
    Error,
    Other(String),
    #[default]
    Unknown,
}

impl AgentState {
    pub fn parse(raw: &str) -> Self {
        match raw {
            "" => Self::Unknown,
            "idle" => Self::Idle,
            "working" => Self::Working,
            "blocked" => Self::Blocked,
            "done" => Self::Done,
            "error" => Self::Error,
            other => Self::Other(other.to_string()),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Idle => "idle",
            Self::Working => "working",
            Self::Blocked => "blocked",
            Self::Done => "done",
            Self::Error => "error",
            Self::Other(state) => state.as_str(),
            Self::Unknown => "",
        }
    }

    pub fn is_blocked(&self) -> bool {
        matches!(self, Self::Blocked)
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AgentStatus {
    pub app: String,
    pub state: AgentState,
    pub updated_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AgentTarget {
    /// Opaque host-owned identifier (for example, an encoded session+block in Rex).
    pub id: String,
    /// Human-readable location shown in the agent picker (session, tab, workspace, or machine).
    pub location: String,
    pub cwd: PathBuf,
    pub status: AgentStatus,
}

impl AgentTarget {
    pub fn picker_label(&self) -> String {
        let cwd = self.cwd.display().to_string();
        let state = self.status.state.as_str();
        [
            self.status.app.as_str(),
            self.location.as_str(),
            state,
            cwd.as_str(),
        ]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(" · ")
    }
}

/// Control-plane interface implemented by a terminal or multiplexer host (e.g. Rex, Herdr).
///
/// Each implementation enforces its own liveness checks (such as matching OSC 7501 status
/// owner PID against foreground PID) and returns `None` from `inspect_agent` when the agent
/// is no longer running in that target.
pub trait AgentHost: Send + Sync {
    fn name(&self) -> &'static str;

    /// Lists live agent targets whose working directory falls within `review_roots`,
    /// ordered most-recent or highest-priority first.
    fn list_agents(&self, review_roots: &[PathBuf]) -> Vec<AgentTarget>;

    /// Resolves an explicit CLI `--agent-target` / `--agent-block` hint into a live target.
    fn find_agent(&self, target_hint: &str) -> Option<AgentTarget>;

    /// Re-queries a previously discovered target by opaque `id`; returns `None` if gone.
    fn inspect_agent(&self, id: &str) -> Option<AgentStatus>;

    /// Delivers a sanitized review prompt into the target and submits it.
    fn send_message(&self, id: &str, message: &str) -> Result<(), String>;
}

pub fn default_host() -> Arc<dyn AgentHost> {
    Arc::new(rex::RexHost::new())
}

fn discover_initial_candidates(
    host: &dyn AgentHost,
    target_hint: Option<&str>,
    review_roots: &[PathBuf],
) -> Vec<AgentTarget> {
    match target_hint {
        Some(hint) => host.find_agent(hint).into_iter().collect(),
        None => host.list_agents(review_roots),
    }
}

fn lumiere_command() -> String {
    match std::env::current_exe() {
        Ok(executable) => command_for(&executable, std::env::var_os("PATH").as_deref()),
        Err(_) => PROGRAM_NAME.to_string(),
    }
}

fn command_for(executable: &Path, path_variable: Option<&std::ffi::OsStr>) -> String {
    let first_on_path = path_variable
        .into_iter()
        .flat_map(std::env::split_paths)
        .map(|directory| directory.join(PROGRAM_NAME))
        .find(|candidate| candidate.is_file());
    let is_the_executable = |candidate: &PathBuf| {
        matches!(
            (candidate.canonicalize(), executable.canonicalize()),
            (Ok(candidate), Ok(executable)) if candidate == executable
        )
    };
    if first_on_path.as_ref().is_some_and(is_the_executable) {
        PROGRAM_NAME.to_string()
    } else {
        format!("'{}'", executable.display())
    }
}

fn message_for_agent(review_path: &Path, comments: &str) -> String {
    let lumiere = lumiere_command();
    let path = review_path.display();
    let continues_a_thread = comments
        .lines()
        .any(|line| line.starts_with("## Thread ") && line.contains(EARLIER_COMMENTS_LABEL));
    let earlier_comments = if continues_a_thread {
        format!(" Earlier comments of a thread: {lumiere} thread show <thread-id> --repo '{path}'")
    } else {
        String::new()
    };
    let message = format!(
        "Lumiere review comments. Answer each thread below with:\n\
         {lumiere} thread reply <thread-id> --repo '{path}' --body '<reply>'\n\
         Reply in the thread first, saying whether you agree and what \
         you would change. That command is your whole answer: write nothing after it. \
         Edit code only after I approve in that thread.\
         {earlier_comments}\n\n\
         {comments}"
    );
    // Strip control characters so none can terminate a bracketed paste sequence early.
    message
        .chars()
        .filter(|character| *character == '\n' || !character.is_control())
        .collect()
}

pub struct AgentLink {
    host: Arc<dyn AgentHost>,
    target: Option<AgentTarget>,
    candidates: Vec<AgentTarget>,
    review_path: PathBuf,
    searched: bool,
    user_selected: bool,
}

impl AgentLink {
    pub fn new(host: Arc<dyn AgentHost>, review_path: PathBuf) -> Self {
        Self {
            host,
            target: None,
            candidates: Vec::new(),
            review_path,
            searched: false,
            user_selected: false,
        }
    }

    pub fn host_name(&self) -> &'static str {
        self.host.name()
    }

    pub fn has_searched(&self) -> bool {
        self.searched
    }

    #[cfg(test)]
    pub fn attach(&mut self, target_hint: Option<&str>, review_roots: &[PathBuf]) {
        let candidates = discover_initial_candidates(self.host.as_ref(), target_hint, review_roots);
        self.apply_attached(candidates);
    }

    pub fn apply_attached(&mut self, candidates: Vec<AgentTarget>) {
        if !self.user_selected {
            self.target = candidates.first().cloned();
        }
        self.candidates = candidates;
        self.searched = true;
    }

    pub fn is_attached(&self) -> bool {
        self.target.is_some()
    }

    pub fn attached_id(&self) -> Option<&str> {
        self.target.as_ref().map(|target| target.id.as_str())
    }

    pub fn known_candidates(&self) -> &[AgentTarget] {
        &self.candidates
    }

    pub fn apply_candidates(&mut self, candidates: Vec<AgentTarget>) {
        self.candidates = candidates;
        self.searched = true;
    }

    pub fn set_target(&mut self, target: Option<AgentTarget>) {
        self.target = target;
        self.user_selected = true;
    }

    pub fn label(&self) -> String {
        match &self.target {
            Some(target) => {
                let state = target.status.state.as_str();
                if state.is_empty() {
                    target.status.app.clone()
                } else {
                    format!("{} {state}", target.status.app)
                }
            }
            None if !self.searched => "finding agent".to_string(),
            None => "no agent".to_string(),
        }
    }

    pub fn refresh(&mut self) {
        let Some(id) = self.target.as_ref().map(|target| target.id.clone()) else {
            return;
        };
        let status = self.host.inspect_agent(&id);
        self.apply_refresh(&id, status);
    }

    /// Applies an asynchronous refresh only if the same target is still attached.
    /// Returns true if the visible state changed.
    pub fn apply_refresh(&mut self, id: &str, status: Option<AgentStatus>) -> bool {
        let Some(target) = &mut self.target else {
            return false;
        };
        if target.id != id {
            return false;
        }
        match status {
            Some(next) if next != target.status => {
                target.status = next;
                true
            }
            Some(_) => false,
            None => {
                self.target = None;
                true
            }
        }
    }

    /// Re-inspects the target live before pasting so we never type into a shell or blocked prompt.
    pub fn send_comments(&mut self, comments: &str) -> Result<(), String> {
        let Some(agent) = self.target.as_ref().map(|target| target.status.app.clone()) else {
            return Err("no agent attached".to_string());
        };
        self.refresh();
        let Some(target) = &self.target else {
            return Err(format!("the pane no longer runs {agent}"));
        };
        if target.status.state.is_blocked() {
            return Err(format!("{agent} is waiting for you"));
        }
        let message = message_for_agent(&self.review_path, comments);
        self.host.send_message(&target.id, &message)
    }
}

pub enum AgentRequest {
    Attach {
        target: Option<String>,
        review_roots: Vec<PathBuf>,
    },
    Refresh {
        id: String,
    },
    FindCandidates {
        review_roots: Vec<PathBuf>,
    },
}

pub enum AgentResponse {
    Attached(Vec<AgentTarget>),
    Refreshed {
        id: String,
        status: Option<AgentStatus>,
    },
    Candidates(Vec<AgentTarget>),
}

/// Runs host discovery and status polling off the UI thread so startup and periodic
/// refreshes never stall keyboard input.
pub struct AgentWorker {
    tx: Sender<AgentRequest>,
    rx: Receiver<AgentResponse>,
}

impl AgentWorker {
    pub fn spawn(host: Arc<dyn AgentHost>) -> Self {
        let (req_tx, req_rx) = mpsc::channel::<AgentRequest>();
        let (res_tx, res_rx) = mpsc::channel::<AgentResponse>();

        thread::spawn(move || {
            while let Ok(request) = req_rx.recv() {
                let response = match request {
                    AgentRequest::Attach {
                        target,
                        review_roots,
                    } => AgentResponse::Attached(discover_initial_candidates(
                        host.as_ref(),
                        target.as_deref(),
                        &review_roots,
                    )),
                    AgentRequest::Refresh { id } => {
                        let status = host.inspect_agent(&id);
                        AgentResponse::Refreshed { id, status }
                    }
                    AgentRequest::FindCandidates { review_roots } => {
                        AgentResponse::Candidates(host.list_agents(&review_roots))
                    }
                };
                if res_tx.send(response).is_err() {
                    break;
                }
            }
        });

        Self {
            tx: req_tx,
            rx: res_rx,
        }
    }

    pub fn request(&self, req: AgentRequest) {
        let _ = self.tx.send(req);
    }

    pub fn try_recv(&self) -> Option<AgentResponse> {
        self.rx.try_recv().ok()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use std::time::Duration;

    use super::*;

    struct FakeHost {
        targets: Mutex<Vec<AgentTarget>>,
        sent: Mutex<Vec<(String, String)>>,
    }

    impl FakeHost {
        fn with(targets: Vec<AgentTarget>) -> Arc<Self> {
            Arc::new(Self {
                targets: Mutex::new(targets),
                sent: Mutex::new(Vec::new()),
            })
        }
    }

    impl AgentHost for FakeHost {
        fn name(&self) -> &'static str {
            "fake"
        }

        fn list_agents(&self, review_roots: &[PathBuf]) -> Vec<AgentTarget> {
            self.targets
                .lock()
                .unwrap()
                .iter()
                .filter(|target| review_roots.iter().any(|root| target.cwd.starts_with(root)))
                .cloned()
                .collect()
        }

        fn find_agent(&self, target_hint: &str) -> Option<AgentTarget> {
            self.targets
                .lock()
                .unwrap()
                .iter()
                .find(|target| target.id == target_hint)
                .cloned()
        }

        fn inspect_agent(&self, id: &str) -> Option<AgentStatus> {
            self.targets
                .lock()
                .unwrap()
                .iter()
                .find(|target| target.id == id)
                .map(|target| target.status.clone())
        }

        fn send_message(&self, id: &str, message: &str) -> Result<(), String> {
            self.sent
                .lock()
                .unwrap()
                .push((id.to_string(), message.to_string()));
            Ok(())
        }
    }

    fn make_target(id: &str, app: &str, state: AgentState, cwd: &str) -> AgentTarget {
        AgentTarget {
            id: id.to_string(),
            location: "work".to_string(),
            cwd: PathBuf::from(cwd),
            status: AgentStatus {
                app: app.to_string(),
                state,
                updated_at: String::new(),
            },
        }
    }

    #[test]
    fn test_agent_state_roundtrip_and_unknown_label_omission() {
        for (raw, expected) in [
            ("idle", AgentState::Idle),
            ("working", AgentState::Working),
            ("blocked", AgentState::Blocked),
            ("done", AgentState::Done),
            ("error", AgentState::Error),
            ("", AgentState::Unknown),
            ("custom", AgentState::Other("custom".to_string())),
        ] {
            let parsed = AgentState::parse(raw);
            assert_eq!(parsed, expected);
            assert_eq!(parsed.as_str(), raw);
        }

        let target = make_target("t1", "codex", AgentState::Unknown, "/work");
        assert_eq!(target.picker_label(), "codex · work · /work");

        let host = FakeHost::with(vec![target]);
        let mut link = AgentLink::new(host, PathBuf::from("/work"));
        link.attach(None, &[PathBuf::from("/work")]);
        assert_eq!(link.host_name(), "fake");
        assert_eq!(link.label(), "codex");
    }

    #[test]
    fn test_label_lifecycle_and_send_safety() {
        let host = FakeHost::with(vec![make_target(
            "agent",
            "codex",
            AgentState::Idle,
            "/work",
        )]);
        let mut link = AgentLink::new(host.clone(), PathBuf::from("/work"));
        assert!(!link.has_searched());
        assert_eq!(link.label(), "finding agent");

        link.attach(None, &[PathBuf::from("/work")]);
        assert_eq!(link.label(), "codex idle");

        // Blocked agent refuses send without typing.
        host.targets.lock().unwrap()[0].status.state = AgentState::Blocked;
        assert_eq!(
            link.send_comments("you: why?").unwrap_err(),
            "codex is waiting for you"
        );
        assert!(host.sent.lock().unwrap().is_empty());
        assert_eq!(link.label(), "codex blocked");

        // Unblocked agent receives sanitized message.
        host.targets.lock().unwrap()[0].status.state = AgentState::Idle;
        let comments = "## Thread 1a2b · a.rs line 2 (RIGHT)\n\nyou: why?\x1b[201~";
        assert_eq!(link.send_comments(comments), Ok(()));
        let sent = host.sent.lock().unwrap();
        assert_eq!(sent.len(), 1);
        assert!(sent[0]
            .1
            .ends_with("## Thread 1a2b · a.rs line 2 (RIGHT)\n\nyou: why?[201~"));
        drop(sent);

        // Exited agent drops link without typing.
        host.targets.lock().unwrap().clear();
        assert_eq!(
            link.send_comments("you: more?").unwrap_err(),
            "the pane no longer runs codex"
        );
        assert!(!link.is_attached());
        assert_eq!(link.label(), "no agent");
    }

    #[test]
    fn test_message_earlier_comments_and_command_resolution() {
        let first = "## Thread 1a2b · a.rs line 2 (RIGHT)\n\nyou: why?";
        let follow_up = "## Thread 1a2b · a.rs line 2 (RIGHT) · earlier comments: 2\n\nyou: ok";
        assert!(!message_for_agent(Path::new("/work"), first).contains("thread show"));
        assert!(message_for_agent(Path::new("/work"), follow_up)
            .contains("thread show <thread-id> --repo '/work'"));

        let installed = tempfile::tempdir().unwrap();
        let elsewhere = tempfile::tempdir().unwrap();
        let executable = installed.path().join("lumiere");
        std::fs::write(&executable, "").unwrap();
        std::fs::write(elsewhere.path().join("lumiere"), "").unwrap();
        let on_path = std::env::join_paths([installed.path(), elsewhere.path()]).unwrap();
        let shadowed = std::env::join_paths([elsewhere.path(), installed.path()]).unwrap();

        assert_eq!(command_for(&executable, Some(&on_path)), "lumiere");
        assert_eq!(
            command_for(&executable, Some(&shadowed)),
            format!("'{}'", executable.display())
        );
        assert_eq!(
            command_for(&executable, None),
            format!("'{}'", executable.display())
        );
    }

    #[test]
    fn test_agent_worker_handles_attach_refresh_and_ignores_stale_target_refresh() {
        let host = FakeHost::with(vec![
            make_target("second", "codex", AgentState::Idle, "/work"),
            make_target("first", "pi", AgentState::Idle, "/work"),
        ]);
        let worker = AgentWorker::spawn(host.clone());
        let mut link = AgentLink::new(host.clone(), PathBuf::from("/work"));

        worker.request(AgentRequest::Attach {
            target: None,
            review_roots: vec![PathBuf::from("/work")],
        });

        let AgentResponse::Attached(candidates) =
            worker.rx.recv_timeout(Duration::from_secs(2)).unwrap()
        else {
            panic!("expected Attached");
        };
        link.apply_attached(candidates);
        assert_eq!(link.attached_id(), Some("second"));

        // Refresh runs asynchronously and detects state transition.
        host.targets.lock().unwrap()[0].status.state = AgentState::Working;
        worker.request(AgentRequest::Refresh {
            id: "second".to_string(),
        });
        let AgentResponse::Refreshed { id, status } =
            worker.rx.recv_timeout(Duration::from_secs(2)).unwrap()
        else {
            panic!("expected Refreshed");
        };
        assert!(link.apply_refresh(&id, status));
        assert_eq!(link.label(), "codex working");

        // A stale refresh for a different target does not overwrite the attached target.
        assert!(!link.apply_refresh("first", None));
        assert_eq!(link.attached_id(), Some("second"));
    }
}
