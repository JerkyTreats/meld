# Runtime cost observability

The bounded repository-to-README experiment qualified correctness while repair latency grew from roughly 110 to 205 to 417 seconds. Provider calls accounted for only part of that elapsed time. This follow-up measures the remainder through the command harness before changing planning, sensory conversion or domain scope.

## Observable contract

`runtime actions --json` carries an optional `timing` product per invocation. `started_at_ms` is the actual Unix invocation time, separate from the existing supervisor-pass timestamp. `bounded_step_us` and `idle_receipt_us` use a monotonic clock. They include blocking I/O and returned failures. They do not claim CPU utilization or exclude provider waits.

The canonical foreground pass account also records the full supervisor tick, post-tick status projection and ledger health observation. `runtime accounts --instance ID --after OFFSET --json` exposes completed accounts from the managed instance's existing process log. It reads bounded complete lines and returns a byte cursor. Neither command advances a domain owner or authors Graph knowledge. Gauges remain observational output rather than domain Events.

Action records persist through the existing supervisor report store. The stored reader recognizes pre-timing actions and snapshots without manufacturing durations or losing activation lineage. New writers use the single current shape. The compatibility reader remains necessary for retained experiment products.

## Harness responsibility

Meld Eval profiles public action and account pages, retains the full command output and executable hash, and reports participant costs, completed pass costs, unknown timing coverage, history gaps, remaining page backlog and observation failures. It measures command process time, hashing time and output byte counts separately. Named journey phases align domain milestones with native reports.

The README journey selects the phases around its existing convergence and confirmation waits. It supplies its managed instance to the generic profiler. It does not prescribe runtime work, change claims or make observability success equivalent to domain acceptance.

## Measurement interpretation

Actor measurements nest inside the full supervisor tick. Provider measurements can nest inside an actor call. These values must not be summed as independent costs. A complete pass permits subtraction of its measured actor and idle-receipt durations from its supervisor duration; incomplete action coverage leaves that remainder unknown.

Status projection has its own measurement because it can consult native wake resolvers after the bounded actors have returned. Account emission, inter-pass sleep, process startup and unfinished calls remain outside the pass's three measured boundaries. Phase duration includes the harness observations performed during that phase.

Use a retained, qualified disposable product to first measure reconciliation without a new source edit or expected model work. Preserve the original qualification artifacts. Run the final capture after local builds and tests finish, and report both the native stop receipt and whether the generated document remained unchanged. This establishes cost attribution, not an optimization or a new qualification of whole-document README quality.

## Retained result

The [cost closeout](../../../../../meld-eval/evidence/runtime-cost-v1/CLOSEOUT.md) records four complete passes with 122.29 seconds in bounded actors, 4.66 seconds obtaining idle receipts, 0.41 seconds in other supervisor work, 50.95 seconds in status projection and 4.40 seconds in health observation. Source was not edited, no provider request occurred, and the README and original qualification report remained unchanged. All 14 participants have measured invocations, without history gaps or observer failures, and native shutdown completed.

Repeated reconciliation and status projection are the next measured locations to investigate. Internal attribution and the historical repair-latency growth remain unproven. This slice changes observation contracts and harness capture only; no new runtime authority or semantic publication path is introduced.
