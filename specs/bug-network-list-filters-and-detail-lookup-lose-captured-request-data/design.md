# Root Cause Analysis: Network list filters and detail lookup lose captured request data

**Issue**: #285
**Date**: 2026-04-28
**Status**: Draft
**Author**: Codex (write-spec)

---

## Root Cause

`src/network.rs::collect_and_correlate()` collects CDP Network and Page events through separate per-method receivers, pushes them into `RawNetworkEvent`, then performs a second correlation pass. The second pass creates a `NetworkRequestBuilder` only when it sees `Network.requestWillBeSent`. `Network.responseReceived`, `Network.loadingFinished`, and `Network.loadingFailed` update a builder only if one already exists for that request ID. Because `tokio::select!` drains whichever per-method receiver is ready, the local `raw_events` vector can process response or finish events before the matching request event even though Chrome emitted a valid request lifecycle. Those early response/finish events are dropped, leaving a listed request with method, URL, and type but without `status`, `size`, or `duration_ms`.

The collector also tags raw events with a mutable `current_nav_id` at receive time and, unless `--include-preserved` is set, keeps only builders whose request event navigation ID equals the final navigation ID. The page reload used for capture can deliver `Page.frameNavigated`, request, response, loading, and load events close together. A document request can therefore be associated inconsistently with the final navigation filter across separate list invocations. The filter implementation itself compares lowercase normalized resource types, but it is filtering a lossy and newly captured request set, so `network list --type document` can lose the same document request that appeared in the prior unfiltered list.

`network get` has a separate stability problem: `execute_get()` calls `collect_and_correlate()` again and searches the newly captured builders by `assigned_id`. The ID is an in-memory sequence number assigned during the current correlation pass, not a stable request identity and not persisted from the preceding `network list`. A request ID returned by list can therefore be absent in the next invocation's recaptured set, producing `Network request <id> not found` even though the ID came from the supported workflow.

### Affected Code

| File | Lines | Role |
|------|-------|------|
| `src/network.rs` | 122-148 | `NetworkRequestBuilder` stores captured request state, assigned numeric IDs, response metadata, timing, and navigation association. |
| `src/network.rs` | 469-748 | `collect_and_correlate()` subscribes to Network/Page events, reloads the page, drains per-method receivers, correlates request state, and filters by navigation. |
| `src/network.rs` | 563-625 | `tokio::select!` receives per-method events without preserving a total cross-method CDP event order. |
| `src/network.rs` | 633-735 | The second-pass correlator creates builders from `requestWillBeSent` and drops response/finish/fail data when the builder is not already present. |
| `src/network.rs` | 737-744 | Current-navigation filtering compares builder navigation IDs against the final mutable navigation counter. |
| `src/network.rs` | 777-792 | `builder_to_summary()` serializes `status`, `size`, `duration_ms`, and `timestamp`; missing correlated data becomes `null`. |
| `src/network.rs` | 856-957 | `execute_list()` recaptures, filters, paginates, and emits summaries for every list invocation. |
| `src/network.rs` | 963-1117 | `execute_get()` recaptures independently and looks up the supplied numeric ID in the new capture set. |
| `tests/bdd.rs` | 7479-7487, 7709-7727 | Existing network regression features are registered as Chrome-dependent documentation scenarios, but no focused test covers list, document filter, metadata, and detail lookup as one workflow. |

### Triggering Conditions

- The capture reload produces Network and Page events close enough together that separate event receivers are ready in the same collection window.
- Response or loading-finished events are drained before their matching request event in the collector's local event vector.
- The document request lifecycle overlaps `Page.frameNavigated`/`Page.loadEventFired`, making the final navigation filter sensitive to receive ordering.
- The user runs `network list`, `network list --type document`, and `network get <id-from-list>` as separate CLI invocations.
- Regression coverage checks pieces of the network feature but does not currently enforce list/filter/get as one stable, metadata-preserving workflow.

---

## Fix Strategy

### Approach

Keep the fix inside the existing network command boundary. Replace the lossy second-pass correlation behavior with an order-insensitive request-state aggregator keyed by CDP `requestId`. The aggregator must retain partial request, response, finish, and failure fragments regardless of which event arrives first, then normalize each completed request into one internal representation before summary filtering, ID assignment, and detail lookup. A response or finish event that arrives before `requestWillBeSent` should be stored as pending state and merged when the request event appears, not discarded.

Move CLI numeric ID assignment to the normalized request set after correlation and deterministic sorting. The list path and get path need a stable bridge: after `network list` builds the normalized capture, persist a short-lived network snapshot for the active Chrome target in AgentChrome-owned session storage. `network get <id>` should first resolve the supplied ID against that snapshot, validating that it belongs to the same host/port/target context. If the snapshot is missing or stale, `network get` may recapture, but it must use the same deterministic normalization and ID assignment before deciding that a request is genuinely unavailable.

For navigation scoping, avoid using the final mutable `current_nav_id` as the only inclusion test for requests observed during the capture reload. The current capture window should include the completed document request produced by the reload, and `--include-preserved` should remain the control for intentionally including requests outside the current capture/navigation scope. Type filtering should continue to run against lowercase normalized resource types after the capture set is complete.

### Changes

| File | Change | Rationale |
|------|--------|-----------|
| `src/network.rs` | Introduce an order-insensitive capture aggregator that stores request, response, finish, and failure fragments keyed by CDP `requestId` and merges them into `NetworkRequestBuilder`/successor state after the drain window. | Prevents response metadata, size, and timing data from being dropped when per-method receivers are drained out of lifecycle order. |
| `src/network.rs` | Assign CLI numeric IDs after normalization and deterministic sorting rather than while the collector happens to see `requestWillBeSent`. | Makes list output stable enough for filtering and later detail lookup. |
| `src/network.rs` | Persist the last normalized list capture for the active target and teach `network get` to resolve IDs from that snapshot before recapturing or returning not found. | Preserves the list-to-get workflow without adding a long-running daemon. |
| `src/network.rs` | Scope current-capture filtering around the reload/capture window so the document request generated by capture is retained even when Page navigation events interleave with Network events. | Prevents `--type document` from losing the document request that unfiltered list can show. |
| `src/network.rs` | Add focused unit tests for out-of-order event fragments, document type filtering, non-null completed metadata, and stable list-to-get ID resolution. | Provides fast regression coverage for the root causes without requiring live Chrome in every unit test. |
| `tests/features/285-network-list-filters-and-detail-lookup-lose-captured-request-data.feature` | Add BDD regression scenarios for AC1-AC4. | Documents and exercises the observed list/filter/get workflow. |
| `tests/bdd.rs` | Register the new feature file and add/reuse step bindings for focused network regression execution. | Keeps the BDD suite aware of the new scenarios and compatible with the existing Chrome-dependent feature filtering pattern. |
| `tests/fixtures/285-network-list-detail.html` | Add a deterministic local page for the live smoke/BDD path if existing fixtures cannot produce the required document request and subresource metadata. | Avoids depending on `qaplayground.vercel.app` in CI while preserving the live-regression workflow for manual verification. |

### Blast Radius

- **Direct impact**: `network list`, `network list --type/--url/--status/--method`, and `network get <id>`.
- **Indirect impact**: shared network output helpers, request size/timestamp/duration serialization, and any tests that assume request IDs are assigned during raw event collection.
- **Not impacted**: command-line argument parsing, JSON error shape, `network follow` streaming behavior unless a shared helper is deliberately reused, console/performance/page/tab command behavior, and network interception/mutation features.
- **Risk level**: Medium. The change touches the core network capture path and adds small persisted state, but it is contained to the network command module and session-owned cache behavior.

---

## Regression Risk

| Risk | Likelihood | Mitigation |
|------|------------|------------|
| `network follow` output changes accidentally if capture helpers are shared too broadly. | Low | Keep follow's streaming loop unchanged unless extracting a pure formatting helper; run existing follow scenarios and focused manual smoke. |
| Snapshot data becomes stale after tab changes or a new navigation. | Medium | Key snapshots by host, port, target/page ID, and capture timestamp; invalidate on context mismatch and recapture before returning not found. |
| Deterministic ID assignment changes existing expectations for simple list output. | Medium | Preserve the existing `id` field shape and add tests that IDs are stable within a normalized capture. |
| Response body retrieval behavior differs when `network get` resolves from a cached snapshot. | Low | Continue treating request/response bodies as optional where CDP no longer exposes them; ensure `request`, `response`, and `timing` sections still exist. |
| Navigation scoping includes stale preserved requests by default. | Low | Unit-test current-capture filtering separately from `--include-preserved`, and use the capture window/target context as the default inclusion boundary. |
| Missing response metadata is hidden by filling defaults. | Low | Preserve `null` for genuinely unavailable CDP fields; only populate fields from captured response/finish/timing data or documented fallbacks. |

---

## Alternatives Considered

| Option | Description | Why Not Selected |
|--------|-------------|------------------|
| Longer event drain timeout | Increase the post-load idle window so more response/finish events might arrive. | Does not fix out-of-order correlation or list-to-get ID instability; it only makes the race less frequent. |
| Background network daemon | Keep Network enabled continuously and serve list/get from a long-running process. | Larger operational change, conflicts with the zero-config CLI model, and is explicitly out of scope for this defect. |
| Re-run capture in `network get` and hope IDs match | Leave IDs as per-capture sequence numbers and recapture on every get. | This is the current failure mode; IDs from a previous list are not stable across invocations. |
| Document that `network get` IDs are invocation-local only | Narrow the contract instead of fixing the workflow. | Contradicts the feature spec and user-facing examples where list is the discovery step for get. |

---

## Validation Checklist

Before moving to TASKS phase:

- [x] Root cause is identified with specific code references
- [x] Fix is minimal - no unrelated refactoring
- [x] Blast radius is assessed
- [x] Regression risks are documented with mitigations
- [x] Fix follows existing project patterns (per `structure.md`)

---

## Change History

| Issue | Date | Summary |
|-------|------|---------|
| #285 | 2026-04-28 | Initial defect design |
