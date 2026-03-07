# CLI Feedback Technical Specification

Date: 2026-03-07
Status: active

## Intent

Define one executable technical specification for compact CLI feedback during long running capability execution.

This specification is normative for the next implementation step.

## Source Synthesis

This specification synthesizes:

- current telemetry and workflow execution behavior in the codebase
- validated `context generate` workflow backed execution on `src/tree`
- the design direction that workflows orchestrate capabilities
- the immediate product goal of compact command feedback without scroll spam

## Problem Statement

Long running generation work needs operator feedback that stays visible in place.

Current command execution emits strong telemetry, but the terminal experience still depends on sparse final summaries rather than compact live state.

The next outcome should improve operator confidence without binding the rendering layer to any one capability implementation.

## Scope

- compact live CLI feedback for capability execution
- rendering for `context generate` as the first consumer
- telemetry driven state reduction for batch progress and workflow turn progress
- stable non interactive behavior for redirected output and scripts

## Non Goals

- full screen terminal UI
- rich key driven interaction
- redesign of workflow or context domain ownership
- remote telemetry dashboards
- replacing final command summaries

## Design Principles

### Workflow owns orchestration intent

Workflow decides which capability runs next.

CLI feedback must represent workflow and capability progress without re implementing orchestration logic.

### Capability owns execution details

Each capability emits telemetry that describes its own progress in a stable schema.

CLI feedback consumes telemetry only.

### Ordering is external input

If a command uses bottom up merkle ordering, the feedback layer treats that as submitted plan data.

CLI feedback does not compute ordering.

### No scroll spam

The active display should stay visible near the command line with in place refresh.

### Script safety

Human feedback goes to `stderr` only.

Machine consumable command results remain on `stdout`.

## Primary User Experience

For an interactive terminal, the command shows a compact live panel with a fixed height.

Recommended default height:

- four lines for active progress
- one optional line for the latest warning or failure

Example:

```text
meld context generate --path src/workflow --agent docs-writer --provider local
19 of 87 done | 2 failed | 4 active | 00:42
batch 2 of 5 | ordering bottom_up | pending 61 | running 4
capability context.generate 19 | workflow.turn 54 | file.write 0
active src/workflow/record_contracts/schema_version.rs :: style_refine
```

For a non interactive terminal, live rendering is disabled and only final summaries are emitted.

## Functional Requirements

### FR1 Telemetry Driven Rendering

The renderer consumes telemetry events from the existing session store.

Rules:

- no direct coupling to workflow executor internals
- no direct coupling to context queue internals
- all state shown to the operator must be derivable from telemetry

### FR2 Fixed Height In Place Panel

Interactive rendering uses carriage return and cursor movement to redraw one fixed block on `stderr`.

Rules:

- panel height remains stable after first draw
- updates replace prior lines rather than append new lines
- renderer flushes at a bounded rate

Recommended default refresh rate:

- five updates per second maximum

### FR3 Graceful Degradation

If the terminal does not support interactive rendering, the renderer disables itself.

Disabled conditions:

- `stderr` is not a terminal
- `NO_COLOR` policy disables dynamic rendering if later configured to do so
- rendering initialization fails

### FR4 Command Summary Preservation

The final command summary remains the source of truth for success and failure.

Rules:

- live panel disappears or finalizes cleanly before the command exits
- final summary still appears once
- errors still appear in the command result surface

### FR5 Capability Neutral State Model

The reducer tracks generic execution state that can represent many capabilities.

Required reducer concepts:

- session status
- plan or submitted work id
- total submitted targets
- completed targets
- failed targets
- skipped targets
- active targets
- active capabilities by type
- active workflow turns by turn id
- latest warning or failure message

### FR6 Context Generate First Consumer

`context generate` is the first command wired to the live renderer.

Required visible state for this command:

- overall target progress
- current plan level or batch index if present in telemetry
- active target path list with truncation
- workflow turn distribution for active workflow backed items

## Event Consumption Model

The reducer should consume existing events first and require only minimal additions.

Primary event sources already present:

- `plan_constructed`
- `generation_started`
- `level_started`
- `node_generation_started`
- `node_generation_completed`
- `node_generation_failed`
- `workflow_target_started`
- `workflow_target_completed`
- `workflow_turn_started`
- `workflow_turn_completed`
- `workflow_turn_failed`
- `workflow_target_force_reset`
- `command_summary`
- `session_ended`

Reducer rules:

- target identity is `node_id`
- active target state starts at target start and ends at target complete or fail
- active turn state starts at turn start and ends at turn complete or fail
- failure banner uses the latest target or turn failure event

## Proposed Renderer Architecture

### Domain split

- telemetry domain stores events and exposes session reads
- CLI domain owns session follow, reduction, rendering, and terminal lifecycle
- capability domains emit events only

### CLI modules

Suggested files:

- `src/cli/progress/session_follow.rs`
- `src/cli/progress/reducer.rs`
- `src/cli/progress/render.rs`
- `src/cli/progress/live_panel.rs`

### Runtime flow

1. command starts and receives session id
2. CLI creates a session follower bound to that session id
3. follower polls or tails new events from the telemetry store
4. reducer updates compact state
5. renderer redraws fixed panel on `stderr`
6. final summary arrives and renderer exits cleanly

## State Reduction Contract

Suggested reducer struct:

```rust
pub struct LiveCommandState {
    pub command: String,
    pub plan_id: Option<String>,
    pub total_targets: Option<usize>,
    pub total_levels: Option<usize>,
    pub current_level: Option<usize>,
    pub completed_targets: usize,
    pub failed_targets: usize,
    pub skipped_targets: usize,
    pub active_targets: Vec<ActiveTargetState>,
    pub active_capabilities: BTreeMap<String, usize>,
    pub active_turns: BTreeMap<String, usize>,
    pub latest_message: Option<String>,
    pub started_at_ms: u64,
}
```

Target state should keep only the small amount of data needed for rendering:

- path
- capability kind
- optional workflow turn id
- started time

## Rendering Rules

### Line 1

Overall progress and elapsed time.

Format target:

`done | failed | active | elapsed`

### Line 2

Plan context if available.

Format target:

`batch or level | ordering if known | pending | running`

### Line 3

Capability distribution.

Format target:

`context.generate | workflow.turn | file.write`

### Line 4

Most relevant active target.

Format target:

`path :: turn or step`

### Optional line 5

Latest failure or warning, truncated to terminal width.

## Configuration Surface

No new user facing flag is required for the first version.

Optional future flags:

- `--quiet-progress`
- `--progress-format text`
- `--progress-format json`

Default policy for the first version:

- interactive terminal enables live panel
- non interactive terminal disables live panel

## Verification Plan

### Unit verification

- reducer builds correct aggregate state from ordered events
- renderer truncates long paths deterministically
- fixed height redraw logic preserves terminal cleanliness

### Integration verification

- `context generate` with workflow backed agent shows progress without newline growth
- final summary remains present after live panel exits
- redirected command output remains stable

### Manual verification

- run `meld context generate --path src/tree --agent docs-writer --provider local`
- verify active panel stays near the command line
- verify no scroll spam during long workflow turns

## Open Questions

- should the renderer read directly from the store or subscribe through a small runtime facade
- should warning line persistence be time bounded or event bounded
- should active target display rotate or remain pinned to the oldest running target
- should command specific reducers extend the generic reducer or compose into it

## Recommended Delivery Order

1. build reducer and renderer behind an internal feature seam
2. wire `context generate` session follow into the live panel
3. add workflow turn aggregation for active items
4. add final cleanup behavior and snapshot tests
5. decide whether to generalize to other long running commands
