pub mod coordinates;
pub mod editor;
pub mod input;
pub mod keybindings;
pub mod markdown;
pub mod state;
pub mod theme;
pub mod views;
pub mod workspace;

use std::collections::VecDeque;
use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Once};
use std::time::{Duration, Instant};

use crossterm::{
    cursor,
    event::{
        self, DisableMouseCapture, EnableMouseCapture, Event, KeyboardEnhancementFlags,
        PopKeyboardEnhancementFlags, PushKeyboardEnhancementFlags,
    },
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::prelude::*;
use spinoff::{spinners, Color, Spinner};

use crate::diff::github::{
    detect_current_branch_pr, fetch_pr_info, fetch_viewed_files, load_pr_file_diffs,
    load_pr_file_diffs_silent, parse_pr_input, PrInfo,
};
use crate::diff::watcher::{setup_watcher, WatchEvent, WatchHandle};
use crate::diff::{
    load_lazy_file_diffs, load_lazy_single_commit_diffs, spawn_stats_job, validate_reference,
    DiffOptions, DiffSource, FileDiff, StatsUpdate,
};
use crate::error::LumiereError;
use crate::review::agent::{default_host, AgentLink, AgentRequest, AgentResponse, AgentWorker};
use crate::review::Workspace;
use crate::ui::editor::AnnotationEditor;
use crate::ui::input::{
    agent_picker_with_selection, close_deleted_thread_modal, count_of, editor_words,
};
use crate::ui::state::AppState;
use crate::ui::views::{
    render_diff, render_empty_state, render_selection_tooltip, DiffRenderInput, Modal,
    ModalContent, ReviewStatus,
};
use crate::ui::workspace::WorkspaceSession;
use crate::vcs::{CommitReference, StackedCommitInfo, VcsBackend};

const LARGE_FILE_NOTICE: &str = "large file: colours are approximate";

static TERMINAL_ACTIVE: AtomicBool = AtomicBool::new(false);
static PANIC_HOOK_INSTALLED: Once = Once::new();

fn open_tui_writer() -> io::Result<Box<dyn Write + Send>> {
    if io::stdout().is_terminal() {
        return Ok(Box::new(io::stdout()));
    }
    #[cfg(unix)]
    if let Ok(f) = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .open("/dev/tty")
    {
        return Ok(Box::new(f));
    }
    Ok(Box::new(io::stdout()))
}

fn restore_writer<W: Write>(writer: &mut W) {
    let _ = execute!(writer, PopKeyboardEnhancementFlags);
    let _ = execute!(
        writer,
        DisableMouseCapture,
        LeaveAlternateScreen,
        cursor::Show
    );
}

fn restore_terminal_best_effort() {
    if let Ok(mut writer) = open_tui_writer() {
        restore_writer(&mut writer);
    }
    let _ = disable_raw_mode();
}

/// Owns raw mode, alternate screen, mouse capture, and kitty keyboard enhancement flags,
/// restoring the terminal deterministically on drop or panic unwind.
pub struct TerminalGuard {
    terminal: Terminal<CrosstermBackend<Box<dyn Write + Send>>>,
}

impl TerminalGuard {
    pub fn enter() -> io::Result<Self> {
        PANIC_HOOK_INSTALLED.call_once(|| {
            let prev = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |info| {
                if TERMINAL_ACTIVE.swap(false, Ordering::SeqCst) {
                    restore_terminal_best_effort();
                }
                prev(info);
            }));
        });

        enable_raw_mode()?;
        TERMINAL_ACTIVE.store(true, Ordering::SeqCst);
        let mut tui_writer = match open_tui_writer() {
            Ok(w) => w,
            Err(e) => {
                if TERMINAL_ACTIVE.swap(false, Ordering::SeqCst) {
                    let _ = disable_raw_mode();
                }
                return Err(e);
            }
        };
        if let Err(e) = execute!(tui_writer, EnterAlternateScreen, EnableMouseCapture) {
            if TERMINAL_ACTIVE.swap(false, Ordering::SeqCst) {
                restore_writer(&mut tui_writer);
                let _ = disable_raw_mode();
            }
            return Err(e);
        }
        let _ = execute!(
            tui_writer,
            PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
        );
        let terminal = match Terminal::new(CrosstermBackend::new(tui_writer)) {
            Ok(t) => t,
            Err(e) => {
                if TERMINAL_ACTIVE.swap(false, Ordering::SeqCst) {
                    restore_terminal_best_effort();
                }
                return Err(e);
            }
        };
        Ok(Self { terminal })
    }

    pub fn suspend(&mut self) -> io::Result<()> {
        TERMINAL_ACTIVE.store(false, Ordering::SeqCst);
        restore_writer(self.terminal.backend_mut());
        disable_raw_mode()
    }

    pub fn resume(&mut self) -> io::Result<()> {
        enable_raw_mode()?;
        TERMINAL_ACTIVE.store(true, Ordering::SeqCst);
        if let Err(e) = execute!(
            self.terminal.backend_mut(),
            EnterAlternateScreen,
            EnableMouseCapture
        ) {
            if TERMINAL_ACTIVE.swap(false, Ordering::SeqCst) {
                restore_writer(self.terminal.backend_mut());
                let _ = disable_raw_mode();
            }
            return Err(e);
        }
        let _ = execute!(
            self.terminal.backend_mut(),
            PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::DISAMBIGUATE_ESCAPE_CODES)
        );
        self.terminal.clear()
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        if TERMINAL_ACTIVE.swap(false, Ordering::SeqCst) {
            restore_writer(self.terminal.backend_mut());
            let _ = disable_raw_mode();
        }
    }
}

pub enum Picker {
    Repository,
    Agent,
    Quit,
}

pub enum LoopAction {
    Continue,
    Quit,
    OpenExternalEditor,
}

pub struct App<'a> {
    pub state: AppState,
    pub options: DiffOptions,
    pub pr_info: Option<PrInfo>,
    pub backend: &'a dyn VcsBackend,
    pub workspace_session: Option<WorkspaceSession<'a>>,
    pub active_modal: Option<Modal>,
    pub open_picker: Option<Picker>,
    pub annotation_editor: Option<AnnotationEditor<'static>>,
    pub pending_watch_event: Option<WatchEvent>,
    pub pending_events: VecDeque<Event>,
    pub send_annotations_on_exit: bool,
    pub show_resolved_threads: bool,
    pub watch_handle: Option<WatchHandle>,
    pub watch_rx: Option<Receiver<WatchEvent>>,
    pub stats_tx: Sender<StatsUpdate>,
    pub stats_rx: Receiver<StatsUpdate>,
    pub stats_generation: u64,
    pub review_roots: Vec<PathBuf>,
    pub agent: AgentLink,
    pub agent_worker: AgentWorker,
    pub agent_refreshed_at: Instant,
    pub agent_refreshes_in_flight: usize,
    pub notice: Option<(String, Instant)>,
    pub noticed_file: Option<String>,
    pub dirty: bool,
}

impl<'a> App<'a> {
    fn new(
        options: DiffOptions,
        pr_info: Option<PrInfo>,
        file_diffs: Vec<FileDiff>,
        diff_source: DiffSource,
        stacked_commits: Option<Vec<StackedCommitInfo>>,
        backend: &'a dyn VcsBackend,
        workspace_session: Option<WorkspaceSession<'a>>,
    ) -> Self {
        let mut state = AppState::for_repo_lazy(
            &options,
            file_diffs,
            diff_source,
            backend,
            options.focus.as_deref(),
        );
        state.attach_thread_store(backend);
        if let Some(pr) = &pr_info {
            state.set_diff_reference(Some(format!(
                "PR #{} ({}...{})",
                pr.number, pr.base_ref, pr.head_ref
            )));
        }
        if let Some(commits) = stacked_commits {
            state.init_stacked_mode(commits);
        }

        if let Some(ref pr) = pr_info {
            let mut spinner = Spinner::new(
                spinners::Dots,
                format!("Syncing viewed status for {} files", state.file_diffs.len()),
                Color::Cyan,
            );
            sync_viewed_files_from_github(pr, &mut state);
            let viewed_count = state.viewed_files.len();
            spinner.success(&format!("{viewed_count} files marked as viewed"));
        }

        state.watching = options.watch && pr_info.is_none();
        let (watch_handle, watch_rx) = if state.watching {
            match setup_watcher(backend.working_tree_root()) {
                Some((handle, rx)) => (Some(handle), Some(rx)),
                None => {
                    state.watching = false;
                    (None, None)
                }
            }
        } else {
            (None, None)
        };

        let canonical = |path: PathBuf| path.canonicalize().unwrap_or(path);
        let launch_dir = std::env::current_dir().ok().map(canonical);
        let mut review_roots: Vec<PathBuf> = match &workspace_session {
            Some(session) => session.repository_roots(),
            None => vec![backend.working_tree_root().to_path_buf()],
        }
        .into_iter()
        .map(canonical)
        .collect();
        let review_path = match (&workspace_session, &launch_dir) {
            (Some(_), Some(dir)) => dir.clone(),
            _ => review_roots[0].clone(),
        };
        review_roots.extend(launch_dir);

        let host = default_host();
        let agent = AgentLink::new(Arc::clone(&host), review_path);
        let agent_worker = AgentWorker::spawn(host);
        agent_worker.request(AgentRequest::Attach {
            target: options.agent_target.clone(),
            review_roots: review_roots.clone(),
        });

        let (stats_tx, stats_rx) = mpsc::channel();
        let mut app = Self {
            state,
            options,
            pr_info,
            backend,
            workspace_session,
            active_modal: None,
            open_picker: None,
            annotation_editor: None,
            pending_watch_event: None,
            pending_events: VecDeque::new(),
            send_annotations_on_exit: false,
            show_resolved_threads: false,
            watch_handle,
            watch_rx,
            stats_tx,
            stats_rx,
            stats_generation: 0,
            review_roots,
            agent,
            agent_worker,
            agent_refreshed_at: Instant::now(),
            agent_refreshes_in_flight: 0,
            notice: None,
            noticed_file: None,
            dirty: true,
        };
        app.start_stats_job();
        app
    }

    pub fn start_stats_job(&mut self) {
        self.stats_generation = self.stats_generation.wrapping_add(1);
        if self.state.stats_ready {
            return;
        }
        spawn_stats_job(
            self.stats_generation,
            self.backend.working_tree_root().to_path_buf(),
            &self.state.diff_source,
            self.options.file.clone(),
            self.stats_tx.clone(),
        );
    }

    pub fn queued_total(&self) -> usize {
        match &self.workspace_session {
            Some(session) => session.queued_count(&self.state),
            None => self.state.queued_count(),
        }
    }

    pub fn send_queued(&mut self, only: Option<(&str, &str)>) -> Result<String, String> {
        let comments = match &self.workspace_session {
            Some(session) => session.queued_text_for_agent(&self.state, only),
            None => self.state.queued_text_for_agent(only),
        };
        self.agent
            .send_comments(&comments)
            .map_err(|error| format!("{error}: still queued"))?;
        let sent = match only {
            Some(_) => self.state.mark_sent(only),
            None => {
                self.state.mark_sent(None)
                    + self
                        .workspace_session
                        .as_mut()
                        .map_or(0, WorkspaceSession::mark_parked_sent)
            }
        };
        if let Some(id) = self.agent.attached_id() {
            self.agent_worker
                .request(AgentRequest::Refresh { id: id.to_string() });
            self.agent_refreshes_in_flight += 1;
            self.agent_refreshed_at = Instant::now();
        }
        Ok(format!("sent {} to the agent", count_of(sent, "comment")))
    }

    fn highlighted_agent_picker_target(&self) -> Option<Option<String>> {
        if !matches!(self.open_picker, Some(Picker::Agent)) {
            return None;
        }
        let Some(Modal {
            content: ModalContent::Select { selected, .. },
        }) = &self.active_modal
        else {
            return None;
        };
        let candidates = self.agent.known_candidates();
        if candidates.is_empty() {
            return None;
        }
        if *selected == 0 {
            Some(None)
        } else {
            candidates
                .get(selected - 1)
                .map(|target| Some(target.id.clone()))
        }
    }

    fn refresh_open_agent_picker(&mut self, highlighted: Option<Option<String>>) {
        if matches!(self.open_picker, Some(Picker::Agent)) {
            let target_ref = highlighted.as_ref().map(|opt| opt.as_deref());
            self.active_modal = Some(agent_picker_with_selection(&self.agent, target_ref));
            self.dirty = true;
        }
    }

    fn poll_background_updates(&mut self) {
        if self.state.reload_threads_if_changed() {
            self.dirty = true;
        }

        while let Some(response) = self.agent_worker.try_recv() {
            match response {
                AgentResponse::Attached(candidates) => {
                    let highlighted = self.highlighted_agent_picker_target();
                    self.agent.apply_attached(candidates);
                    self.agent_refreshed_at = Instant::now();
                    self.refresh_open_agent_picker(highlighted);
                    self.dirty = true;
                }
                AgentResponse::Refreshed { id, status } => {
                    self.agent_refreshes_in_flight =
                        self.agent_refreshes_in_flight.saturating_sub(1);
                    if self.agent.apply_refresh(&id, status) {
                        self.dirty = true;
                    }
                }
                AgentResponse::Candidates(candidates) => {
                    let highlighted = self.highlighted_agent_picker_target();
                    self.agent.apply_candidates(candidates);
                    self.refresh_open_agent_picker(highlighted);
                }
            }
        }

        if self.agent.has_searched()
            && self.agent_refreshes_in_flight == 0
            && self.agent_refreshed_at.elapsed() > Duration::from_secs(3)
        {
            if let Some(id) = self.agent.attached_id() {
                self.agent_worker
                    .request(AgentRequest::Refresh { id: id.to_string() });
                self.agent_refreshes_in_flight += 1;
            }
            self.agent_refreshed_at = Instant::now();
        }

        if self
            .notice
            .as_ref()
            .is_some_and(|(_, shown_at)| shown_at.elapsed() > Duration::from_secs(6))
        {
            self.notice = None;
            self.dirty = true;
        }

        if self.active_modal.is_none() {
            if let Some(error) = self.state.thread_store_error.take() {
                self.active_modal = Some(Modal::info(
                    "Threads are not saved",
                    format!("{error}\n\nComments written from now on are lost when lumiere exits."),
                ));
                self.dirty = true;
            }
        }

        while let Ok(update) = self.stats_rx.try_recv() {
            if update.generation == self.stats_generation {
                self.state.total_added = update.added;
                self.state.total_removed = update.removed;
                self.state.stats_ready = true;
                self.dirty = true;
            }
        }

        if let Some(ref rx) = self.watch_rx {
            while let Ok(event) = rx.try_recv() {
                self.state.needs_reload = true;
                match &mut self.pending_watch_event {
                    Some(existing) => existing.changed_files.extend(event.changed_files),
                    None => self.pending_watch_event = Some(event),
                }
                self.dirty = true;
            }
        }

        if self.state.needs_reload {
            let changed_files = self.pending_watch_event.take().map(|e| e.changed_files);
            if let Some(ref pr) = self.pr_info {
                match load_pr_file_diffs_silent(pr) {
                    Ok(file_diffs) => {
                        self.state.reload(file_diffs, changed_files.as_ref());
                        self.state.refresh_branch(self.backend);
                        sync_viewed_files_from_github(pr, &mut self.state);
                    }
                    Err(e) => {
                        self.state.needs_reload = false;
                        self.set_notice(format!("reload failed: {e}"));
                    }
                }
            } else if self.state.stacked_mode {
                if let Some(commit) = self.state.current_commit().cloned() {
                    self.state.save_stacked_viewed_files();
                    let (file_diffs, diff_source) = load_lazy_single_commit_diffs(
                        &commit.commit_id,
                        &self.options.file,
                        self.backend,
                    );
                    self.state.reload_lazy(
                        file_diffs,
                        diff_source,
                        changed_files.as_ref(),
                        self.backend,
                    );
                    self.state.refresh_branch(self.backend);
                    self.state.load_stacked_viewed_files();
                    self.start_stats_job();
                } else {
                    self.state.needs_reload = false;
                }
            } else {
                let (file_diffs, diff_source) = load_lazy_file_diffs(&self.options, self.backend);
                self.state.reload_lazy(
                    file_diffs,
                    diff_source,
                    changed_files.as_ref(),
                    self.backend,
                );
                self.state.refresh_branch(self.backend);
                self.start_stats_job();
            }
            self.dirty = true;
        }

        if close_deleted_thread_modal(
            &self.state,
            &mut self.active_modal,
            &mut self.annotation_editor,
            self.show_resolved_threads,
        ) {
            self.dirty = true;
        }

        if let Some(diff) = self.state.file_diffs.get(self.state.current_file) {
            if self.noticed_file.as_deref() != Some(diff.filename.as_str()) {
                self.noticed_file = Some(diff.filename.clone());
                if self.state.current_file_is_too_large_to_parse() {
                    self.set_notice(LARGE_FILE_NOTICE);
                    self.dirty = true;
                }
            }
        }
    }

    fn draw<B: Backend>(&mut self, terminal: &mut Terminal<B>) -> Result<(), B::Error> {
        if self.state.file_diffs.is_empty() {
            let repo_name = self
                .workspace_session
                .as_ref()
                .map(WorkspaceSession::active_name);
            terminal.draw(|frame| {
                render_empty_state(frame, self.state.watching, repo_name);
                if let Some(ref modal) = self.active_modal {
                    modal.render(frame, &self.state.threads, self.annotation_editor.as_ref());
                }
            })?;
            self.state.editor_rect = self.active_modal.as_ref().and_then(Modal::editor_rect);
            return Ok(());
        }

        self.state.update_search_matches();
        self.state.get_highlighters();
        let (old_hl, new_hl) = self.state.highlighters_ref().unwrap();

        let base_ref = self
            .state
            .diff_reference
            .as_deref()
            .unwrap_or(&self.state.current_branch);
        let workspace_commit_ref = self
            .workspace_session
            .as_ref()
            .map(|session| format!("{} · {base_ref}", session.active_name()));
        let commit_ref = workspace_commit_ref.as_deref().unwrap_or(base_ref);

        let agent_label = self.agent.label();
        let review_status = ReviewStatus {
            agent: &agent_label,
            queued_comments: self.queued_total(),
            notice: self.notice.as_ref().map(|(text, _)| text.as_str()),
        };

        let thread_modal_open = self
            .active_modal
            .as_ref()
            .is_some_and(Modal::is_thread_view);
        let inline_editor = (!thread_modal_open)
            .then_some(self.annotation_editor.as_ref())
            .flatten();

        let mut render_output = views::DiffRenderOutput::default();
        let mut modal_editor_rect = None;

        terminal.draw(|frame| {
            render_output = render_diff(
                frame,
                &DiffRenderInput {
                    state: &self.state,
                    commit_ref,
                    pr_info: self.pr_info.as_ref(),
                    old_highlighter: old_hl,
                    new_highlighter: new_hl,
                    editor: inline_editor,
                    review: &review_status,
                },
            );

            if self.state.show_selection_tooltip
                && self.state.selection.is_active()
                && !self.state.is_dragging
                && self.annotation_editor.is_none()
                && self.active_modal.is_none()
            {
                render_selection_tooltip(
                    frame,
                    &self.state,
                    render_output.content_row_offset,
                    &render_output.overlay_gaps,
                );
            }

            if let Some(ref modal) = self.active_modal {
                modal.render(frame, &self.state.threads, self.annotation_editor.as_ref());
                modal_editor_rect = modal.editor_rect();
            }
        })?;

        self.state.content_row_offset = render_output.content_row_offset;
        self.state.annotation_overlay_gaps = render_output.overlay_gaps;
        self.state.annotation_rects = render_output.annotation_rects;
        self.state.editor_rect = modal_editor_rect.or(render_output.editor_rect);
        Ok(())
    }

    fn run_external_editor(&mut self) {
        let Some(diff) = self.state.file_diffs.get(self.state.current_file) else {
            return;
        };
        let filename = diff.filename.clone();
        let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vim".to_string());

        let line_arg = if let Some(hunk_idx) = self.state.focused_hunk {
            self.state.ensure_cache();
            self.state
                .hunks_ref()
                .get(hunk_idx)
                .and_then(|&hunk_start| self.state.side_by_side_ref().get(hunk_start))
                .and_then(|dl| {
                    dl.new_line
                        .as_ref()
                        .map(|(n, _)| *n)
                        .or(dl.old_line.as_ref().map(|(n, _)| *n))
                })
        } else {
            None
        };

        let words = editor_words(&editor);
        let mut cmd = std::process::Command::new(&words[0]);
        cmd.args(&words[1..]);
        cmd.current_dir(self.backend.working_tree_root());
        #[cfg(unix)]
        if !io::stdout().is_terminal() {
            if let Ok(tty) = std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open("/dev/tty")
            {
                cmd.stdout(tty);
            }
        }
        if let Some(line) = line_arg {
            cmd.arg(format!("+{line}"));
        }
        if let Err(error) = cmd.arg(filename).status() {
            self.set_notice(format!("cannot run {editor}: {error}"));
        }
    }

    fn export_queued_to_stdout(&mut self) -> io::Result<()> {
        let formatted = match &self.workspace_session {
            Some(session) => session.format_threads_for_export(&self.state, true),
            None => self.state.format_threads_for_export(true),
        };
        let stdout = io::stdout();
        let mut handle = stdout.lock();
        handle.write_all(formatted.as_bytes())?;
        handle.write_all(b"\n")?;
        self.state.mark_sent(None);
        if let Some(session) = self.workspace_session.as_mut() {
            session.mark_parked_sent();
        }
        Ok(())
    }
}

fn sync_viewed_files_from_github(pr_info: &PrInfo, state: &mut AppState) {
    if let Ok(viewed_paths) = fetch_viewed_files(pr_info) {
        state.viewed_files.clear();
        for (idx, diff) in state.file_diffs.iter().enumerate() {
            if viewed_paths.contains(&diff.filename) {
                state.viewed_files.insert(idx);
            }
        }
    }
}

fn run_loop<'a>(mut app: App<'a>) -> Result<(), LumiereError> {
    let mut guard = TerminalGuard::enter()?;

    'main: loop {
        app.poll_background_updates();

        if app.dirty {
            app.draw(&mut guard.terminal)?;
            app.dirty = false;
        }

        if app.pending_events.is_empty() && event::poll(Duration::from_millis(50))? {
            app.pending_events.push_back(event::read()?);
            while event::poll(Duration::ZERO)? {
                app.pending_events.push_back(event::read()?);
            }
        }

        while let Some(ev) = app.pending_events.pop_front() {
            let size = guard.terminal.size()?;
            let rect = Rect::new(0, 0, size.width, size.height);
            match app.handle_event(ev, rect) {
                LoopAction::Continue => {}
                LoopAction::Quit => break 'main,
                LoopAction::OpenExternalEditor => {
                    guard.suspend()?;
                    app.run_external_editor();
                    guard.resume()?;
                    app.dirty = true;
                }
            }
        }
    }

    drop(guard);

    if app.send_annotations_on_exit {
        app.export_queued_to_stdout()?;
    }

    Ok(())
}

fn fetch_pr_with_spinner(pr_input: &str, origin: Option<&str>) -> Result<PrInfo, LumiereError> {
    let spinner_msg = match parse_pr_input(pr_input) {
        Some((Some(owner), Some(repo), number)) => format!("Fetching PR {owner}/{repo}#{number}"),
        Some((_, _, number)) => format!("Fetching PR #{number}"),
        None => "Fetching PR".to_string(),
    };
    let mut spinner = Spinner::new(spinners::Dots, spinner_msg, Color::Cyan);
    match fetch_pr_info(pr_input, origin) {
        Ok(pr_info) => {
            spinner.success("Fetched PR metadata");
            Ok(pr_info)
        }
        Err(e) => {
            spinner.clear();
            Err(LumiereError::CommandError(e))
        }
    }
}

pub fn run_workspace_diff_ui(
    options: DiffOptions,
    workspace: &Workspace,
) -> Result<(), LumiereError> {
    if options.pr.is_some() || options.detect_pr || options.stacked {
        return Err(LumiereError::CommandError(
            "--pr, --detect-pr and --stacked need a single repository; this folder holds several"
                .to_string(),
        ));
    }

    let initial_repository = workspace
        .initial_repository(&options)
        .map_err(LumiereError::CommandError)?;
    let backend = workspace
        .backend(initial_repository)
        .map_err(LumiereError::CommandError)?;
    theme::init(options.theme.as_deref());
    let (file_diffs, diff_source) = load_lazy_file_diffs(&options, backend);
    let session = WorkspaceSession::new(workspace, initial_repository);
    let app = App::new(
        options,
        None,
        file_diffs,
        diff_source,
        None,
        backend,
        Some(session),
    );
    run_loop(app)
}

pub fn run_diff_ui(mut options: DiffOptions, backend: &dyn VcsBackend) -> Result<(), LumiereError> {
    if options.detect_pr && options.pr.is_none() {
        let mut spinner = Spinner::new(
            spinners::Dots,
            "Detecting PR for current branch",
            Color::Cyan,
        );
        match detect_current_branch_pr() {
            Ok(number) => {
                spinner.success(&format!("Detected PR #{number}"));
                options.pr = Some(number);
            }
            Err(e) => {
                spinner.clear();
                return Err(LumiereError::CommandError(e));
            }
        }
    }

    let pr_input = options.pr.clone().or_else(|| {
        if let Some(CommitReference::Single(ref input)) = options.reference {
            if input.contains("/pull/") || input.parse::<u64>().is_ok() {
                return Some(input.clone());
            }
        }
        None
    });

    if let Some(pr_input) = pr_input {
        let pr_info = fetch_pr_with_spinner(&pr_input, options.origin.as_deref())?;
        theme::init(options.theme.as_deref());
        let file_diffs = load_pr_file_diffs(&pr_info).map_err(LumiereError::CommandError)?;
        let app = App::new(
            options,
            Some(pr_info),
            file_diffs,
            DiffSource::Eager,
            None,
            backend,
            None,
        );
        return run_loop(app);
    }

    if let Some(CommitReference::MergeBaseToWorkingTree { .. }) = options.reference {
        validate_reference(options.reference.as_ref(), backend)
            .map_err(LumiereError::CommandError)?;
    }

    if options.stacked {
        let Some(ref reference) = options.reference else {
            return Err(LumiereError::CommandError(
                "--stacked requires a range (e.g., main..feature)".to_string(),
            ));
        };
        let (from, to) = match reference {
            CommitReference::Range { from, to } => (from.clone(), to.clone()),
            CommitReference::TripleDots { from, to } => {
                let merge_base = backend
                    .get_merge_base(from, to)
                    .unwrap_or_else(|_| from.clone());
                (merge_base, to.clone())
            }
            CommitReference::Single(_)
            | CommitReference::RangeToWorkingTree { .. }
            | CommitReference::MergeBaseToWorkingTree { .. } => {
                return Err(LumiereError::CommandError(
                    "--stacked requires a range (e.g., main..feature)".to_string(),
                ));
            }
        };

        let commits = backend.get_commits_in_range(&from, &to)?;
        if commits.is_empty() {
            return Err(LumiereError::CommandError(format!(
                "No commits found in range {from}..{to}"
            )));
        }

        theme::init(options.theme.as_deref());
        let (file_diffs, diff_source) =
            load_lazy_single_commit_diffs(&commits[0].commit_id, &options.file, backend);
        let app = App::new(
            options,
            None,
            file_diffs,
            diff_source,
            Some(commits),
            backend,
            None,
        );
        return run_loop(app);
    }

    theme::init(options.theme.as_deref());
    let (file_diffs, diff_source) = load_lazy_file_diffs(&options, backend);
    let app = App::new(options, None, file_diffs, diff_source, None, backend, None);
    run_loop(app)
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crossterm::event::{
        KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind,
    };
    use ratatui::backend::TestBackend;
    use tempfile::TempDir;

    use super::*;
    use crate::review::agent::{AgentHost, AgentState, AgentStatus, AgentTarget};
    use crate::vcs::test_utils::git;
    use crate::vcs::GitBackend;

    struct StubHost {
        root: PathBuf,
    }

    impl StubHost {
        fn target(&self, id: &str) -> AgentTarget {
            AgentTarget {
                id: id.to_string(),
                location: "work".to_string(),
                cwd: self.root.clone(),
                status: AgentStatus {
                    app: "pi".to_string(),
                    state: AgentState::Idle,
                    updated_at: "2026-11-25T10:00:00Z".to_string(),
                },
            }
        }
    }

    impl AgentHost for StubHost {
        fn name(&self) -> &'static str {
            "stub"
        }

        fn list_agents(&self, _review_roots: &[PathBuf]) -> Vec<AgentTarget> {
            vec![self.target("b1")]
        }

        fn find_agent(&self, target_hint: &str) -> Option<AgentTarget> {
            (target_hint == "b1").then(|| self.target("b1"))
        }

        fn inspect_agent(&self, id: &str) -> Option<AgentStatus> {
            (id == "b1").then(|| self.target("b1").status)
        }

        fn send_message(&self, _id: &str, _message: &str) -> Result<(), String> {
            Ok(())
        }
    }

    fn init_repo_with_diff() -> (TempDir, GitBackend) {
        let dir = TempDir::new().unwrap();
        let p = dir.path();
        git(p, &["init"]);
        git(p, &["config", "user.email", "test@example.com"]);
        git(p, &["config", "user.name", "Test User"]);
        let base: String = (1..=60).map(|n| format!("line {n}\n")).collect();
        std::fs::write(p.join("a.rs"), &base).unwrap();
        git(p, &["add", "."]);
        git(p, &["commit", "-m", "init"]);
        let updated = base.replace("line 5\n", "line 5 edited\n");
        std::fs::write(p.join("a.rs"), updated).unwrap();
        let backend = GitBackend::new(p).unwrap();
        (dir, backend)
    }

    fn key(code: KeyCode, modifiers: KeyModifiers) -> Event {
        Event::Key(KeyEvent::new(code, modifiers))
    }

    fn test_app<'a>(backend: &'a GitBackend, host: Arc<dyn AgentHost>, root: &Path) -> App<'a> {
        let options = DiffOptions {
            hunks_only: true,
            ..DiffOptions::default()
        };
        let (file_diffs, diff_source) = load_lazy_file_diffs(&options, backend);
        let state = AppState::for_repo_lazy(&options, file_diffs, diff_source, backend, None);
        let review_roots = vec![root.to_path_buf()];
        let agent = AgentLink::new(Arc::clone(&host), root.to_path_buf());
        let agent_worker = AgentWorker::spawn(host);
        let (stats_tx, stats_rx) = mpsc::channel();
        App {
            state,
            options,
            pr_info: None,
            backend,
            workspace_session: None,
            active_modal: None,
            open_picker: None,
            annotation_editor: None,
            pending_watch_event: None,
            pending_events: VecDeque::new(),
            send_annotations_on_exit: false,
            show_resolved_threads: false,
            watch_handle: None,
            watch_rx: None,
            stats_tx,
            stats_rx,
            stats_generation: 0,
            review_roots,
            agent,
            agent_worker,
            agent_refreshed_at: Instant::now(),
            agent_refreshes_in_flight: 0,
            notice: None,
            noticed_file: None,
            dirty: true,
        }
    }

    #[test]
    fn test_app_comment_queue_quit_prompt_and_global_search_jump_to_hidden_line() {
        let (dir, backend) = init_repo_with_diff();
        let host: Arc<dyn AgentHost> = Arc::new(StubHost {
            root: dir.path().to_path_buf(),
        });
        let mut app = test_app(&backend, host, dir.path());
        let screen = Rect::new(0, 0, 120, 30);

        // Open comment editor on focused hunk, type, and queue with Alt+Enter.
        app.handle_event(key(KeyCode::Char('i'), KeyModifiers::NONE), screen);
        assert!(app.annotation_editor.is_some());
        for c in "needs test".chars() {
            app.handle_event(key(KeyCode::Char(c), KeyModifiers::NONE), screen);
        }
        app.handle_event(key(KeyCode::Enter, KeyModifiers::ALT), screen);
        assert!(app.annotation_editor.is_none());
        assert_eq!(app.queued_total(), 1);

        // Pressing q with queued comments asks before exiting; Esc cancels.
        assert!(matches!(
            app.handle_event(key(KeyCode::Char('q'), KeyModifiers::NONE), screen),
            LoopAction::Continue
        ));
        assert!(matches!(app.open_picker, Some(Picker::Quit)));
        app.handle_event(key(KeyCode::Esc, KeyModifiers::NONE), screen);
        assert!(app.active_modal.is_none());

        // Global search jumping to line 55 (folded in hunks-only view) switches to full file.
        assert!(app.state.settings.hunks_only);
        app.handle_event(key(KeyCode::Char('f'), KeyModifiers::CONTROL), screen);
        for c in "line 55".chars() {
            app.handle_event(key(KeyCode::Char(c), KeyModifiers::NONE), screen);
        }
        app.handle_event(key(KeyCode::Enter, KeyModifiers::NONE), screen);
        assert!(!app.state.settings.hunks_only);
        assert!(app.active_modal.is_none());
        assert!(app
            .notice
            .as_ref()
            .is_some_and(|(m, _)| m.contains("full file")));
    }

    #[test]
    fn test_app_agent_picker_refreshes_asynchronously_and_scroll_coalesces() {
        let (dir, backend) = init_repo_with_diff();
        let host: Arc<dyn AgentHost> = Arc::new(StubHost {
            root: dir.path().to_path_buf(),
        });
        let mut app = test_app(&backend, host, dir.path());
        let screen = Rect::new(0, 0, 120, 30);

        // `A` opens picker immediately and requests candidates off-thread.
        app.handle_event(key(KeyCode::Char('A'), KeyModifiers::NONE), screen);
        assert!(matches!(app.open_picker, Some(Picker::Agent)));
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            app.poll_background_updates();
            if !app.agent.known_candidates().is_empty() {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(app.agent.known_candidates().len(), 1);
        app.handle_event(key(KeyCode::Esc, KeyModifiers::NONE), screen);

        // Switch to full-file view and coalesce multiple queued wheel events into one scroll.
        app.state.set_hunks_only(false, 25);
        app.state.scroll = 0;
        let wheel = |kind| {
            Event::Mouse(MouseEvent {
                kind,
                column: 80,
                row: 10,
                modifiers: KeyModifiers::NONE,
            })
        };
        app.pending_events
            .push_back(wheel(MouseEventKind::ScrollDown));
        app.pending_events
            .push_back(wheel(MouseEventKind::ScrollDown));
        app.handle_event(wheel(MouseEventKind::ScrollDown), screen);
        assert_eq!(app.state.scroll, 9);
        assert!(app.pending_events.is_empty());
    }

    #[test]
    fn test_late_attach_respects_user_choice_and_picker_preserves_highlight() {
        let (dir, backend) = init_repo_with_diff();
        let host = Arc::new(StubHost {
            root: dir.path().to_path_buf(),
        });
        let mut app = test_app(&backend, host.clone(), dir.path());

        // Explicit disconnect before initial attach finishes must not be clobbered.
        app.agent.set_target(None);
        app.agent
            .apply_attached(vec![host.target("b1"), host.target("b2")]);
        assert!(!app.agent.is_attached());

        // Highlighting b2 in the open agent picker survives a reordered candidate list.
        app.open_picker = Some(Picker::Agent);
        app.active_modal = Some(agent_picker_with_selection(&app.agent, Some(Some("b2"))));
        let highlighted = app.highlighted_agent_picker_target();
        app.agent
            .apply_candidates(vec![host.target("b2"), host.target("b1")]);
        app.refresh_open_agent_picker(highlighted);
        match &app.active_modal {
            Some(Modal {
                content: ModalContent::Select { selected, .. },
            }) => assert_eq!(*selected, 1),
            _ => panic!("expected agent picker select modal"),
        }
    }

    #[test]
    fn test_clicking_inline_thread_opens_modal_and_reply_box_focuses_on_click_or_enter() {
        let (dir, backend) = init_repo_with_diff();
        let host: Arc<dyn AgentHost> = Arc::new(StubHost {
            root: dir.path().to_path_buf(),
        });
        let mut app = test_app(&backend, host, dir.path());
        let screen = Rect::new(0, 0, 120, 30);
        let mut terminal = Terminal::new(TestBackend::new(screen.width, screen.height)).unwrap();

        let (thread_id, _) = app.state.add_thread(
            "a.rs".to_string(),
            crate::review::AnnotationTarget::File,
            "initial note".to_string(),
        );
        app.state.mark_sent(None);
        app.draw(&mut terminal).unwrap();

        let click = |column, row| {
            Event::Mouse(MouseEvent {
                kind: MouseEventKind::Down(MouseButton::Left),
                column,
                row,
                modifiers: KeyModifiers::NONE,
            })
        };

        let thread_rect = app.state.annotation_rects[0].area;
        app.handle_event(click(thread_rect.x + 2, thread_rect.y + 1), screen);
        assert!(app.active_modal.as_ref().is_some_and(Modal::is_thread_view));
        assert!(app.annotation_editor.is_none());

        app.draw(&mut terminal).unwrap();
        let reply_rect = match &app.active_modal {
            Some(Modal {
                content: ModalContent::ThreadView(view),
            }) => {
                assert_eq!(view.thread_id, thread_id);
                view.last_reply_box_rect.get().expect("reply box rect")
            }
            _ => panic!("expected thread modal"),
        };

        app.handle_event(click(0, 0), screen);
        assert!(app.active_modal.as_ref().is_some_and(Modal::is_thread_view));
        assert!(app.annotation_editor.is_none());

        app.handle_event(click(reply_rect.x + 2, reply_rect.y + 1), screen);
        assert!(app.annotation_editor.is_some());

        app.draw(&mut terminal).unwrap();
        let editor_rect = app.state.editor_rect.expect("editor rect");
        app.handle_event(click(editor_rect.x + 2, editor_rect.y - 1), screen);
        assert!(app.annotation_editor.is_none());
        assert!(app.active_modal.as_ref().is_some_and(Modal::is_thread_view));

        app.handle_event(key(KeyCode::Enter, KeyModifiers::NONE), screen);
        assert!(app.annotation_editor.is_some());

        app.handle_event(key(KeyCode::Esc, KeyModifiers::NONE), screen);
        assert!(app.annotation_editor.is_none());
        assert!(app.active_modal.as_ref().is_some_and(Modal::is_thread_view));
    }
}
