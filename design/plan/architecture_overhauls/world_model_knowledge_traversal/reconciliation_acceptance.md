# Minimum reconciliation flywheel acceptance

Scope corrected by the user on 2026-09-07.

The mission is a working world-model reconciliation flywheel for CVE, Docs freshness and Startup nonce, with no domain theory embedded in Rust runtime code. It is not implementation of the full cognitive architecture. The previous assessment incorrectly made Causation and Regime prerequisites and expanded acceptance beyond this mission. This record replaces that assessment.

The delivery program remains directional and non-binding. The [outcome audit](reconciliation_outcome_audit.md) retains implementation history and executable evidence. Broader architectural documents explain ownership and intent; their unrelated feature ambitions do not expand this acceptance scope.

## What must work

| Product | Acceptance behavior |
| --- | --- |
| CVE and dependency security | Observe dependency inventory and advisory state, admit meaningful evidence, detect the configured mismatch, construct and authorize the work selected by the installed theory, execute authorized mitigation where configured, independently verify the result, and return evidence to Agent. Changed inventory or advisory knowledge must change the next decision. |
| Docs freshness | Observe actual source and README claims, distinguish correct from absent or incorrect documentation, perform authorized repair, observe and confirm the result, and satisfy or maintain the configured condition. An already-correct README must not force executable work. |
| Startup nonce | Prepare and activate the installed product, establish the current epoch's expected nonce, execute its Task, observe Graph visibility, perform planned confirmation, and reach separate Agent acceptance and Goal satisfaction. A predecessor nonce cannot satisfy the successor epoch. |

All three must use the shared native reconciliation path: owner observation, admitted knowledge, Planner cut, Strategy construction, Agent authorization, Curation or Execution, owner return and Agent judgment. Currentness, separate authorization, retained history and native lifecycle ownership remain correctness requirements. A product-specific coordinator that scripts a successful demonstration does not satisfy this.

## No domain theory in Rust runtime code

Desired conditions, thresholds, domain judgments, rules, prompts, mechanisms selected for these products, evidence interpretation and product-level Plan policy must come from the exact installed theory and owner products. Runtime composition must not recreate them, invent a breach Goal, choose the intended semantic result, or substitute generic operational status for returned domain evidence.

Rust may implement native evaluators, Capability behavior, contract validation, persistence, algorithms and structural wiring. The distinction is between executing an installed rule and hard-coding the product's rule. A product name, a typed Capability contract or a test fixture is not by itself an invariant violation. Moving constants into another Rust module would not correct a real theory violation.

The first source review covers [runtime composition](../../../../src/runtime/assembly.rs), [runtime ports](../../../../src/runtime/ports.rs), [epoch preparation](../../../../src/runtime/epoch.rs), [theory loading](../../../../src/runtime/theory.rs), and the native consumers of [Docs theory](../../../../theory/docs_freshness/README.md), [Security theory](../../../../theory/dependency_security/pds-package.json), [mitigation theory](../../../../theory/dependency_security_mitigation/README.md) and [Startup theory](../../../../theory/startup/README.md). Follow decisions back to installed revisions, including any Rust defaults or fallbacks. Separate production code from test declarations before recording a finding.

For each real violation, retain one concrete account: the decision being hard-coded, its proper theory or native owner, the corrected production caller and the proof that installed theory now controls it. Use existing policy-variation tests where they already establish that relationship. Extend them only for uncovered decisions. Acceptance should demonstrate that changing a supported installed policy changes the relevant judgment without editing or recompiling runtime Rust.

## Remediation into acceptance

1. Finish the focused theory-ownership review across the three product paths. Correct actual Rust-embedded domain policy and remove its superseded implementation. Preserve working native primitives and installed theory formats unless a demonstrated correction needs a change.
2. Correct shared-runtime defects that can break these flywheels. One identified candidate is root's terminal-failure substring classifier in `is_terminal_claimed_failure`. Reproduce its ambiguity, carry an owner-declared typed result, and remove semantic text matching. This is a runtime correctness issue, not a requirement to add more domain theory.
3. Run the three products through ordinary installation and runtime entrypoints with isolated external state. Reuse the existing production-composition proofs, local provider and advisory fixtures. Confirm actual effects and returned owner evidence, not just passing internal states. Add a small executable smoke proof where current coverage stops at an in-process CLI handler.
4. Exercise the already relevant shared behaviors: changed knowledge, visible confirmation, no-action, independent compatible work, rejected or failed work, uncertain returns and epoch restart. Credit existing evidence and repeat affected cases after corrections. Fix defects that prevent the three products from working; do not turn every imaginable failure or hardware condition into a new prerequisite.
5. Validate the final source and record the result. Run the complete workspace regression suite and established static checks, identify meaningful exclusions, and tie the three product results and invariant review to the tested commit. Commit and push verified checkpoints under the standing authorization.

## Done means

CVE, Docs freshness and nonce work end to end through the native shared runtime. Their applicable installed theory controls semantic decisions. No known domain-theory-in-runtime violation or reproduced flywheel-breaking defect remains. The final evidence names the tested source, the three product scenarios, native owner returns, theory-variation evidence and relevant recovery results. Inspection can explain an incomplete run using actual missing owner positions.

Causation and Regime implementation, general cognitive inference, unrestricted autonomous code generation, additional products and an exhaustive disaster-recovery qualification are not acceptance prerequisites. Existing declared code-change machinery may support configured CVE mitigation, but a general code-change product is not an additional acceptance target. No new frozen gates or approval hierarchy are required.

## Evidence status

Existing source and test anchors for all three products are recorded in the outcome audit. Recent checkpoints corrected native intake accounting and provider rejection recovery. They remain valid evidence. This scope correction does not declare the invariant review complete or a new final test run passed. The initial search found substantial theory-shaped declarations inside test sections, and installed-theory loading in production; it is not a completed semantic ownership audit.
