# Complex Change Workflow Governance

Date: 2026-03-01
Status: active
Scope: user triggered workflow for complex change requests

## Purpose

Define one optional workflow for complex work that benefits from phased execution, dependency tracking, and explicit verification gates.

This workflow is opt in and not the default for routine change requests.

## Activation

Workflow activation requires explicit user intent.

Activation paths:
- user requests complex workflow mode
- agent recommends complex workflow mode and user confirms

Without explicit user confirmation, work remains on the default workflow path.

## CI Enforcement

CI ignores this workflow.

Non enforcement rules:
- CI does not require workflow artifacts
- CI does not block merges for missing workflow metadata
- workflow compliance is a reviewer and author agreement, not an automation gate

## Required Artifacts When Active

When this workflow is active, create and maintain one PLAN document under the relevant `design/` scope.

Required PLAN structure:
- overview with objective and outcome
- development phases with dependency order
- per phase goal tasks exit criteria and key seams
- verification strategy and gate definitions
- implementation order summary
- related documentation links
- short exception list for non default path command behavior

## Branch and Commit Model When Active

Branch model:
- one feature branch for the full plan scope
- optional short lived phase working branches for parallel execution

Commit model:
- each phase task should land as one atomic commit by default
- tiny coupled doc updates may share one commit when split cost is higher than value
- mixed runtime and design updates should use runtime focused commit type and include design impact in commit body

## Evidence and Completion Tracking

Tracking requirements:
- update task completion status in PLAN as work lands
- capture verification evidence for each phase gate
- add phase completion notes with unresolved risks when present

Completion requirement:
- final plan state shows gate pass status for all active phases before closeout

## Deactivation

This workflow deactivates when either condition is true:
- user requests return to default workflow
- scoped complex work is complete and closed out in the plan
