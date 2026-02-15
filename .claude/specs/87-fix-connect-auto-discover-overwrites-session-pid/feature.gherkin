# File: tests/features/87-fix-connect-auto-discover-overwrites-session-pid.feature
#
# Generated from: .claude/specs/87-fix-connect-auto-discover-overwrites-session-pid/requirements.md
# Issue: #87
# Type: Defect regression

@regression
Feature: Connect auto-discover preserves session PID
  The connect auto-discover code path previously overwrote the session file
  with pid: None, losing the PID stored by connect --launch.
  This was fixed by reading the existing session and preserving the PID
  when the port matches.

  # --- Bug Is Fixed ---

  @regression
  Scenario: PID is preserved across reconnections
    Given Chrome was launched with "connect --launch" storing PID 54321 on port 9222
    When I run "connect" auto-discover on port 9222
    Then the session file retains PID 54321

  @regression
  Scenario: Disconnect kills launched Chrome after reconnection
    Given Chrome was launched with "connect --launch" and auto-discovered with "connect"
    When I run "connect --disconnect"
    Then the output includes "killed_pid"

  # --- Related Behavior Still Works ---

  @regression
  Scenario: PID is not injected when no prior session exists
    Given no session file exists
    When I run "connect" auto-discover to a running Chrome on port 9222
    Then the session file does not contain a PID
