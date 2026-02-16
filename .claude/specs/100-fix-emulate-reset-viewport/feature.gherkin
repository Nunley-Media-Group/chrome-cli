# File: tests/features/100-fix-emulate-reset-viewport.feature
#
# Generated from: .claude/specs/100-fix-emulate-reset-viewport/requirements.md
# Issue: #100
# Type: Defect regression

@regression
Feature: emulate reset restores original viewport dimensions
  The `emulate reset` command previously called `Emulation.clearDeviceMetricsOverride`
  which removed the CDP override but did not physically restore the original viewport
  dimensions. The viewport retained the overridden size (e.g., 375x667) instead of
  reverting to the baseline (e.g., 756x417).
  This was fixed by capturing baseline viewport dimensions before the first override
  and restoring them during reset via `Emulation.setDeviceMetricsOverride`.

  # --- Bug Is Fixed ---

  @regression
  Scenario: Reset restores original viewport after viewport override
    Given a Chrome session is connected
    And I note the current viewport dimensions as the baseline
    And the viewport is overridden via "emulate set --viewport 375x667"
    When I run "emulate reset"
    And I run "emulate status --json"
    Then the viewport dimensions match the baseline

  # --- Related Behavior Still Works ---

  @regression
  Scenario: Reset clears all overrides and restores viewport
    Given a Chrome session is connected
    And I note the current viewport dimensions as the baseline
    And emulation overrides are applied via "emulate set --viewport 375x667 --mobile --user-agent 'TestBot/1.0' --network slow-4g"
    When I run "emulate reset"
    And I run "emulate status --json"
    Then the viewport dimensions match the baseline
    And the JSON output field "mobile" is false
    And the JSON output does not contain field "network"

  # --- Edge Case ---

  @regression
  Scenario: Reset is idempotent when no overrides are active
    Given a Chrome session is connected
    And I note the current viewport dimensions as the baseline
    When I run "emulate reset"
    Then the exit code is 0
    And the viewport dimensions match the baseline
