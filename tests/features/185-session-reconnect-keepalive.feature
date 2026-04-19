# File: tests/features/185-session-reconnect-keepalive.feature
#
# Issue: #185 — Session reconnection and keep-alive for long-running automation
# Covers ACs: AC21, AC22, AC23, AC24, AC25, AC26, AC27, AC28, AC29, AC30, AC31, AC32, AC33, AC34, AC35, AC36
#
# Layered with the existing session-connection-management.feature; this file
# focuses exclusively on the #185 enhancements (Layer A reconnect, Layer B
# keep-alive, structured loss kinds, observability, and discoverability surfaces).

Feature: Session reconnection and keep-alive
  As a developer or AI agent running long automation sessions
  I want commands to auto-reconnect and the WebSocket to stay alive
  So that I do not have to relaunch Chrome and lose context mid-workflow

  # --- Auto-reconnect (Layer A, invocation-level) ---

  @requires-chrome
  Scenario: Auto-reconnect on stale ws_url
    # Covers AC21
    Given a session file with a stale ws_url and Chrome running on the stored port
    When I run "agentchrome page snapshot"
    Then the tool rediscovers the current ws_url on the stored port
    And the session file is rewritten with the new ws_url
    And stdout contains only the snapshot JSON payload
    And the exit code is 0

  @requires-chrome
  Scenario Outline: Auto-reconnect applies uniformly across commands
    # Covers AC22 (path audit)
    Given a session file with a stale ws_url and Chrome still running
    When I run "<command>"
    Then the tool auto-reconnects transparently
    And "<command>" completes successfully

    Examples:
      | command                                             |
      | agentchrome tabs list                               |
      | agentchrome navigate about:blank                    |
      | agentchrome page snapshot                           |
      | agentchrome js exec "1+1"                           |
      | agentchrome network list                            |
      | agentchrome console read                            |
      | agentchrome dialog info                             |

  @requires-chrome
  Scenario: Auto-reconnect preserves pid in session file
    # Covers AC23 / FR17
    Given a session file with pid 12345 and a stale ws_url
    When auto-reconnect rewrites the session file
    Then the new session file still contains "pid": 12345
    And the new session file contains the fresh ws_url
    And the new session file's "reconnect_count" is incremented by 1

  Scenario: Per-attempt reconnect probe is bounded
    # Covers AC24 / FR14 — pure session-file logic, no Chrome required
    Given auto-reconnect is triggered against an unresponsive port
    When each individual probe is attempted
    Then each probe completes or aborts within the configured probe_timeout_ms
    And the total reconnect duration does not exceed max_attempts * (probe_timeout_ms + max_backoff)

  Scenario: Bounded reconnect attempts exhaust and error
    # Covers AC25
    Given Chrome is unreachable at the stored address
    And "reconnect.max_attempts" is 3
    When I run a command that triggers auto-reconnect
    Then exactly 3 probe attempts are made
    And an error is returned after the attempt budget is exhausted

  # --- Structured error kinds ---

  Scenario: Unrecoverable loss — Chrome terminated
    # Covers AC26 / FR16
    Given a session file with pid 99999 and no process alive at that pid
    And Chrome is not reachable at any known port
    When I run "agentchrome page snapshot"
    Then stderr contains JSON with "kind": "chrome_terminated"
    And stderr contains JSON with "recoverable": false
    And stderr suggests running "agentchrome connect --launch"
    And the session file is not deleted
    And the exit code is 2

  Scenario: Recoverable loss — transient
    # Covers AC27 / FR16
    Given a session file with pid of a currently alive process
    And Chrome is temporarily unreachable on the stored port
    When I run a command that triggers reconnect
    Then stderr contains JSON with "kind": "transient"
    And stderr contains JSON with "recoverable": true
    And stderr suggests running "agentchrome connect"
    And the exit code is 2

  # --- Keep-alive (Layer B, transport-level) ---

  @requires-chrome
  Scenario: Keep-alive ping prevents idle disconnect
    # Covers AC28 / FR15
    Given a command holds the CDP session for longer than the keep-alive interval
    When the keep-alive interval elapses with no outbound CDP traffic
    Then the client sends a WebSocket Ping frame
    And a Pong response is received within the pong timeout
    And the command continues without the connection being dropped

  Scenario Outline: Keep-alive interval resolution precedence
    # Covers AC29 / FR15
    Given "<source>" sets the keep-alive interval
    When a command runs
    Then the effective keep-alive interval matches "<source>"
    And no lower-precedence source overrides it

    Examples:
      | source                                    |
      | --keepalive-interval flag                 |
      | AGENTCHROME_KEEPALIVE_INTERVAL env var    |
      | config.toml [keepalive].interval_ms       |
      | compiled-in default (30000 ms)            |

  Scenario: Keep-alive disabled via --no-keepalive
    # Covers AC30 / FR15
    Given I pass "--no-keepalive"
    When a long-running command runs
    Then no WebSocket Ping frames are sent
    And the command still succeeds under normal conditions

  Scenario: Keep-alive disabled via interval 0
    # Covers AC30
    Given I pass "--keepalive-interval 0"
    When a long-running command runs
    Then no WebSocket Ping frames are sent

  @requires-chrome
  Scenario: Keep-alive does not collide with in-flight JSON-RPC
    # Covers AC31 / FR15
    Given keep-alive is active
    And a CDP JSON-RPC request is in flight when the keep-alive interval elapses
    When the keep-alive ping is sent
    Then the ping is delivered as a WebSocket control frame
    And the JSON-RPC response is correctly correlated to the original request id
    And no ping frame is interpreted as a JSON-RPC message

  # --- Observability ---

  @requires-chrome
  Scenario: Reconnect is silent on stdout
    # Covers AC32 / FR20
    Given a command succeeds after auto-reconnect
    When the command emits its output
    Then stdout contains only the expected JSON payload
    And stderr is empty unless "--verbose" was passed

  @requires-chrome
  Scenario: Reconnect telemetry visible via connect --status
    # Covers AC33 / FR19
    Given a reconnect has occurred during the current session's lifetime
    When I run "agentchrome connect --status"
    Then the JSON output contains "last_reconnect_at" as an ISO 8601 timestamp
    And the JSON output contains "reconnect_count" greater than 0
    And the JSON output contains "keepalive.interval_ms" and "keepalive.enabled"

  # --- Discoverability surfaces (clap / capabilities / man / README) ---

  Scenario: Clap --help lists the new flags
    # Covers AC34 / FR21
    Given agentchrome is built
    When I run "agentchrome --help"
    Then stdout should contain "--keepalive-interval"
    And stdout should contain "--no-keepalive"

  Scenario: Clap long help includes worked EXAMPLES for the new flags
    # Covers AC34 / FR21
    Given agentchrome is built
    When I run "agentchrome connect --help"
    Then stdout should contain "EXAMPLES:"
    And stdout should contain "--keepalive-interval"
    And stdout should contain "--json"

  Scenario: Capabilities manifest reflects the new flags
    # Covers AC35 / FR22
    Given agentchrome is built
    When I run "agentchrome capabilities"
    Then stdout should contain "--keepalive-interval"
    And stdout should contain "--no-keepalive"

  Scenario: Man page covers the new flags
    # Covers AC35 / FR22 — verified manually via `cargo xtask man connect`; CI
    # cannot invoke cargo from inside a cargo test process.

  Scenario: README documents session resilience
    # Covers AC36 / FR23
    When I inspect the project README.md
    Then it contains the heading "Session resilience"
    And it mentions "keepalive-interval" and the default 30000 ms
    And it mentions the error kind discriminator and the recoverable boolean
    And it includes at least one copy-pasteable "--keepalive-interval" command example
