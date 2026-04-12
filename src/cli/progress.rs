use crate::cli::parse::{Commands, ContextCommands};
use crate::telemetry::events::ProgressEvent;
use crate::telemetry::ProgressRuntime;
use owo_colors::OwoColorize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::io::{self, IsTerminal, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

const PANEL_REFRESH_INTERVAL: Duration = Duration::from_millis(200);
const DEFAULT_RENDER_WIDTH: usize = 100;
const MAX_ACTIVE_PATH_WIDTH: usize = 72;

pub struct LiveProgressHandle {
    stop_flag: Arc<AtomicBool>,
    join_handle: Option<JoinHandle<()>>,
}

impl LiveProgressHandle {
    pub fn start_if_supported(
        runtime: Arc<ProgressRuntime>,
        session_id: &str,
        command: &Commands,
    ) -> Option<Self> {
        let panel_title = match command {
            Commands::Context {
                command: ContextCommands::Generate { .. },
            } => "meld context generate".to_string(),
            Commands::Context {
                command: ContextCommands::Regenerate { .. },
            } => "meld context regenerate".to_string(),
            _ => return None,
        };

        if !matches!(
            command,
            Commands::Context {
                command: ContextCommands::Generate { .. } | ContextCommands::Regenerate { .. }
            }
        ) {
            return None;
        }

        if !io::stderr().is_terminal() {
            return None;
        }

        let stop_flag = Arc::new(AtomicBool::new(false));
        let thread_stop = Arc::clone(&stop_flag);
        let thread_runtime = Arc::clone(&runtime);
        let thread_session = session_id.to_string();

        let join_handle = thread::spawn(move || {
            let mut reducer = LivePanelReducer::new();
            let mut renderer = StderrPanelRenderer::new();
            let mut last_seq = 0u64;

            while !thread_stop.load(Ordering::Relaxed) {
                if let Ok(events) = thread_runtime
                    .store()
                    .read_events_after(&thread_session, last_seq)
                {
                    for event in events {
                        last_seq = last_seq.max(event.seq);
                        reducer.apply(&event);
                    }
                    if reducer.has_visible_state() {
                        let width = renderer.width();
                        let panel = reducer.render_panel(&panel_title, width);
                        renderer.render(&panel);
                    }
                }
                thread::sleep(PANEL_REFRESH_INTERVAL);
            }

            if let Ok(events) = thread_runtime
                .store()
                .read_events_after(&thread_session, last_seq)
            {
                for event in events {
                    reducer.apply(&event);
                }
            }
            renderer.clear();
        });

        Some(Self {
            stop_flag,
            join_handle: Some(join_handle),
        })
    }

    pub fn stop(&mut self) {
        self.stop_flag.store(true, Ordering::Relaxed);
        if let Some(join_handle) = self.join_handle.take() {
            let _ = join_handle.join();
        }
    }
}

impl Drop for LiveProgressHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

#[derive(Default)]
struct LivePanelReducer {
    total_nodes: Option<usize>,
    total_levels: Option<usize>,
    current_level: Option<usize>,
    completed: usize,
    failed: usize,
    queue_pending: usize,
    queue_processing: usize,
    workflow_mode: bool,
    active_targets: BTreeMap<String, ActiveTargetState>,
    active_turns: BTreeMap<String, usize>,
    latest_message: Option<String>,
    started_at: Option<Instant>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ActiveTargetState {
    path: String,
    stage: String,
}

impl LivePanelReducer {
    fn new() -> Self {
        Self {
            started_at: Some(Instant::now()),
            ..Self::default()
        }
    }

    fn apply(&mut self, event: &ProgressEvent) {
        match event.event_type.as_str() {
            "plan_constructed" => {
                self.total_nodes = read_usize(&event.data, "total_nodes");
                self.total_levels = read_usize(&event.data, "total_levels");
            }
            "level_started" | "execution.control.level_started" => {
                self.current_level = read_usize(&event.data, "level_index");
            }
            "queue_stats" => {
                self.queue_pending =
                    read_usize(&event.data, "pending").unwrap_or(self.queue_pending);
                self.queue_processing =
                    read_usize(&event.data, "processing").unwrap_or(self.queue_processing);
            }
            "node_generation_started" | "execution.control.node_started" => {
                if read_string(&event.data, "program_kind").as_deref() == Some("workflow") {
                    self.workflow_mode = true;
                }
                if let (Some(node_id), Some(path)) = (
                    read_string(&event.data, "node_id"),
                    read_string(&event.data, "path"),
                ) {
                    self.active_targets.insert(
                        node_id,
                        ActiveTargetState {
                            path,
                            stage: "queued".to_string(),
                        },
                    );
                }
            }
            "workflow_turn_started" | "execution.workflow.turn_started" => {
                self.workflow_mode = true;
                if let Some(turn_id) = read_string(&event.data, "turn_id") {
                    *self.active_turns.entry(turn_id.clone()).or_insert(0) += 1;
                    if let Some(node_id) = read_string(&event.data, "node_id") {
                        let path = read_string(&event.data, "path").unwrap_or_default();
                        self.active_targets.insert(
                            node_id,
                            ActiveTargetState {
                                path,
                                stage: turn_id,
                            },
                        );
                    }
                }
            }
            "workflow_turn_completed"
            | "workflow_turn_failed"
            | "execution.workflow.turn_completed"
            | "execution.workflow.turn_failed" => {
                if let Some(turn_id) = read_string(&event.data, "turn_id") {
                    decrement_counter(&mut self.active_turns, &turn_id);
                }
                if matches!(
                    event.event_type.as_str(),
                    "workflow_turn_failed" | "execution.workflow.turn_failed"
                ) {
                    self.latest_message = read_string(&event.data, "error");
                }
            }
            "node_generation_completed" | "execution.control.node_completed" => {
                self.completed += 1;
                if let Some(node_id) = read_string(&event.data, "node_id") {
                    self.active_targets.remove(&node_id);
                }
            }
            "node_generation_failed" | "execution.control.node_failed" => {
                self.failed += 1;
                self.latest_message = read_string(&event.data, "error");
                if let Some(node_id) = read_string(&event.data, "node_id") {
                    self.active_targets.remove(&node_id);
                }
            }
            _ => {}
        }
    }

    fn has_visible_state(&self) -> bool {
        self.total_nodes.is_some()
            || !self.active_targets.is_empty()
            || self.completed > 0
            || self.failed > 0
    }

    fn render_panel(&self, title: &str, width: usize) -> String {
        let total = self.total_nodes.unwrap_or(0);
        let scheduled = self.active_targets.len();
        let pending = if self.queue_pending > 0 {
            self.queue_pending
        } else {
            total.saturating_sub(self.completed + self.failed + scheduled)
        };
        let workers = self.queue_processing;
        let elapsed = self
            .started_at
            .map(|started| format_elapsed(started.elapsed()))
            .unwrap_or_else(|| "00:00".to_string());

        let title_line = format!(
            "{} {}",
            title.bold().bright_cyan(),
            format_elapsed_badge(&elapsed, self.failed > 0)
        );
        let summary_line = join_segments(&[
            styled_metric(
                "done",
                &format_count(self.completed, total),
                MetricTone::Good,
            ),
            styled_metric(
                "failed",
                &self.failed.to_string(),
                MetricTone::Alert(self.failed > 0),
            ),
            styled_metric("queued", &pending.to_string(), MetricTone::Quiet),
            styled_metric("running", &workers.to_string(), MetricTone::Info),
        ]);
        let batch_line = join_segments(&[
            styled_metric(
                "phase",
                phase_label(self.current_level, self.total_levels),
                MetricTone::Info,
            ),
            styled_metric(
                "level",
                &format_level(self.current_level, self.total_levels),
                MetricTone::Quiet,
            ),
            styled_turn_summary(&self.active_turns),
        ]);
        let description_line = describe_current_work(
            self.current_level,
            self.total_levels,
            workers,
            scheduled,
            self.workflow_mode,
        );
        let detail_line = if let Some(active_target) = self.active_targets.values().next() {
            format_active_line(active_target)
        } else if let Some(message) = &self.latest_message {
            format!(
                "{} {}",
                "latest".bright_black().bold(),
                truncate_line(message, width.saturating_sub(8)).yellow()
            )
        } else {
            format!(
                "{} {}",
                "active".bright_black().bold(),
                "idle".bright_black()
            )
        };

        [
            title_line,
            summary_line,
            batch_line,
            description_line,
            detail_line,
        ]
        .join("\n")
    }
}

struct StderrPanelRenderer {
    rendered_line_count: usize,
    width: usize,
    last_rendered: Option<String>,
}

impl StderrPanelRenderer {
    fn new() -> Self {
        Self {
            rendered_line_count: 0,
            width: detect_width(),
            last_rendered: None,
        }
    }

    fn width(&mut self) -> usize {
        self.width = detect_width();
        self.width
    }

    fn render(&mut self, panel: &str) {
        if self.last_rendered.as_deref() == Some(panel) {
            return;
        }

        let lines: Vec<String> = panel
            .lines()
            .map(|line| truncate_line(line, self.width))
            .collect();
        let target_line_count = lines.len();
        let total_line_count = self.rendered_line_count.max(target_line_count);
        let mut stderr = io::stderr().lock();
        if self.rendered_line_count > 0 {
            let _ = write!(
                stderr,
                "\x1b[{}F",
                self.rendered_line_count.saturating_sub(1)
            );
        }

        for index in 0..total_line_count {
            let _ = write!(stderr, "\x1b[2K\r");
            if let Some(line) = lines.get(index) {
                let _ = write!(stderr, "{}", line);
            }
            if index + 1 < total_line_count {
                let _ = writeln!(stderr);
            }
        }
        let _ = stderr.flush();
        self.rendered_line_count = target_line_count;
        self.last_rendered = Some(panel.to_string());
    }

    fn clear(&mut self) {
        if self.rendered_line_count == 0 {
            return;
        }

        let mut stderr = io::stderr().lock();
        let _ = write!(
            stderr,
            "\x1b[{}F",
            self.rendered_line_count.saturating_sub(1)
        );
        for index in 0..self.rendered_line_count {
            let _ = write!(stderr, "\x1b[2K\r");
            if index + 1 < self.rendered_line_count {
                let _ = writeln!(stderr);
            }
        }
        let _ = write!(stderr, "\r");
        let _ = stderr.flush();
        self.rendered_line_count = 0;
        self.last_rendered = None;
    }
}

fn format_elapsed_badge(elapsed: &str, has_failures: bool) -> String {
    let badge = format!("{} {}", "live", elapsed);
    if has_failures {
        badge.red().bold().to_string()
    } else {
        badge.bright_black().to_string()
    }
}

fn format_active_line(active_target: &ActiveTargetState) -> String {
    let path = truncate_line(&active_target.path, MAX_ACTIVE_PATH_WIDTH);
    format!(
        "{} {} {} {}",
        "active".bright_black().bold(),
        path.bold(),
        "•".bright_black(),
        active_target.stage.cyan().bold()
    )
}

fn format_count(completed: usize, total: usize) -> String {
    if total > 0 {
        format!("{} of {}", completed, total)
    } else {
        completed.to_string()
    }
}

fn format_level(current_level: Option<usize>, total_levels: Option<usize>) -> String {
    format!(
        "{} of {}",
        current_level.map(|level| level + 1).unwrap_or(0),
        total_levels.unwrap_or(0)
    )
}

fn join_segments(segments: &[String]) -> String {
    let divider = format!(" {} ", "|".bright_black());
    segments.join(&divider)
}

enum MetricTone {
    Good,
    Info,
    Quiet,
    Alert(bool),
}

fn styled_metric(label: &str, value: &str, tone: MetricTone) -> String {
    let label = label.bright_black().bold().to_string();
    let value = match tone {
        MetricTone::Good => value.green().bold().to_string(),
        MetricTone::Info => value.cyan().bold().to_string(),
        MetricTone::Quiet => value.bright_white().to_string(),
        MetricTone::Alert(true) => value.red().bold().to_string(),
        MetricTone::Alert(false) => value.bright_black().to_string(),
    };
    format!("{} {}", label, value)
}

fn styled_turn_summary(active_turns: &BTreeMap<String, usize>) -> String {
    if active_turns.is_empty() {
        return styled_metric("turns", "idle", MetricTone::Quiet);
    }

    let parts = active_turns
        .iter()
        .take(2)
        .map(|(turn_id, count)| format!("{} {}", turn_id, count))
        .collect::<Vec<_>>()
        .join(", ");
    let suffix = if active_turns.len() > 2 {
        format!(" +{}", active_turns.len().saturating_sub(2))
    } else {
        String::new()
    };
    styled_metric("turns", &format!("{}{}", parts, suffix), MetricTone::Info)
}

fn phase_label(current_level: Option<usize>, total_levels: Option<usize>) -> &'static str {
    match (current_level, total_levels) {
        (None, _) => "planning",
        (Some(_), Some(0)) => "planning",
        (Some(level), Some(total)) if total > 0 && level + 1 < total => "deep pass",
        (Some(_), Some(_)) => "parent pass",
        (Some(_), None) => "processing",
    }
}

fn describe_current_work(
    current_level: Option<usize>,
    total_levels: Option<usize>,
    workers: usize,
    scheduled: usize,
    workflow_mode: bool,
) -> String {
    let ordering = match (current_level, total_levels, workflow_mode) {
        (None, _, _) => "building the ordered target list before generation starts",
        (Some(level), Some(total), true) if total > 0 && level + 1 < total => {
            "running deeper workflow targets first so parent folders inherit finished child docs"
        }
        (Some(_), Some(_), true) => {
            "running the parent workflow pass after deeper workflow targets completed"
        }
        (Some(level), Some(total), _) if total > 0 => {
            if level + 1 < total {
                "running deeper targets first so parent folders wait for child context"
            } else {
                "running the parent pass after deeper targets completed"
            }
        }
        _ => "processing ordered targets",
    };

    let worker_text = if workers == 0 {
        if scheduled > 0 {
            "queued targets are waiting for a worker slot"
        } else {
            "waiting for the next target update"
        }
    } else if workers == 1 {
        if workflow_mode {
            "1 workflow conversation is active"
        } else {
            "1 target is actively generating"
        }
    } else {
        if workflow_mode {
            "multiple workflow conversations are active in parallel"
        } else {
            "multiple targets are generating in parallel"
        }
    };

    format!(
        "{} {} {} {}",
        "about".bright_black().bold(),
        ordering.bright_black(),
        "•".bright_black(),
        worker_text.bright_black()
    )
}

fn read_string(data: &Value, key: &str) -> Option<String> {
    data.get(key)
        .and_then(|value| value.as_str())
        .map(ToString::to_string)
}

fn read_usize(data: &Value, key: &str) -> Option<usize> {
    data.get(key)
        .and_then(|value| value.as_u64())
        .and_then(|value| usize::try_from(value).ok())
}

fn decrement_counter(map: &mut BTreeMap<String, usize>, key: &str) {
    if let Some(count) = map.get_mut(key) {
        if *count > 1 {
            *count -= 1;
        } else {
            map.remove(key);
        }
    }
}

fn detect_width() -> usize {
    std::env::var("COLUMNS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 20)
        .unwrap_or(DEFAULT_RENDER_WIDTH)
}

fn truncate_line(line: &str, width: usize) -> String {
    if line.chars().count() <= width {
        return line.to_string();
    }

    let mut truncated = String::new();
    for ch in line.chars().take(width.saturating_sub(1)) {
        truncated.push(ch);
    }
    truncated.push('…');
    truncated
}

fn format_elapsed(elapsed: Duration) -> String {
    let total_seconds = elapsed.as_secs();
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    format!("{:02}:{:02}", minutes, seconds)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn event(seq: u64, event_type: &str, data: Value) -> ProgressEvent {
        ProgressEvent {
            ts: "2026-03-07T00:00:00.000Z".to_string(),
            session: "s1".to_string(),
            seq,
            domain_id: "telemetry".to_string(),
            stream_id: "s1".to_string(),
            event_type: event_type.to_string(),
            content_hash: None,
            data,
        }
    }

    fn strip_ansi(input: &str) -> String {
        let mut output = String::new();
        let mut chars = input.chars().peekable();
        while let Some(ch) = chars.next() {
            if ch == '\u{1b}' && matches!(chars.peek(), Some('[')) {
                let _ = chars.next();
                for esc in chars.by_ref() {
                    if esc.is_ascii_alphabetic() {
                        break;
                    }
                }
                continue;
            }
            output.push(ch);
        }
        output
    }

    #[test]
    fn reducer_tracks_context_progress_counts() {
        let mut reducer = LivePanelReducer::new();
        reducer.apply(&event(
            1,
            "plan_constructed",
            json!({ "total_nodes": 5, "total_levels": 3 }),
        ));
        reducer.apply(&event(
            2,
            "level_started",
            json!({ "level_index": 0, "total_count": 2 }),
        ));
        reducer.apply(&event(
            3,
            "node_generation_started",
            json!({ "node_id": "n1", "path": "./src/tree/a.rs" }),
        ));
        reducer.apply(&event(
            4,
            "node_generation_completed",
            json!({ "node_id": "n1" }),
        ));

        let panel = strip_ansi(&reducer.render_panel("meld context generate", 100));
        assert!(panel.contains("meld context generate"));
        assert!(panel.contains("done 1 of 5"));
        assert!(panel.contains("level 1 of 3"));
        assert!(panel.contains("running 0"));
        assert!(panel.contains("phase deep pass"));
    }

    #[test]
    fn reducer_tracks_workflow_turns_and_failures() {
        let mut reducer = LivePanelReducer::new();
        reducer.apply(&event(
            1,
            "node_generation_started",
            json!({ "node_id": "n1", "path": "./src/tree/walker.rs" }),
        ));
        reducer.apply(&event(
            2,
            "workflow_turn_started",
            json!({ "node_id": "n1", "path": "./src/tree/walker.rs", "turn_id": "verification" }),
        ));
        reducer.apply(&event(
            3,
            "queue_stats",
            json!({ "pending": 2, "processing": 2, "completed": 0, "failed": 0 }),
        ));

        let panel = strip_ansi(&reducer.render_panel("meld context generate", 100));
        assert!(panel.contains("verification 1"));
        assert!(panel.contains("walker.rs"));
        assert!(panel.contains("running 2"));
        assert!(panel.contains("multiple workflow conversations are active in parallel"));

        reducer.apply(&event(
            4,
            "workflow_turn_failed",
            json!({ "node_id": "n1", "turn_id": "verification", "error": "boom" }),
        ));
        reducer.apply(&event(
            5,
            "node_generation_failed",
            json!({ "node_id": "n1", "error": "boom" }),
        ));

        let panel = strip_ansi(&reducer.render_panel("meld context generate", 100));
        assert!(panel.contains("failed 1"));
        assert!(panel.contains("latest"));
        assert!(panel.contains("boom"));
    }
}
