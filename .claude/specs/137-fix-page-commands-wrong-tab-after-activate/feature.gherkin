# File: tests/features/137-fix-page-commands-wrong-tab-after-activate.feature
#
# Generated from: .claude/specs/137-fix-page-commands-wrong-tab-after-activate/requirements.md
# Issue: #137
# Type: Defect regression

@regression
Feature: Page commands target wrong tab after tabs activate
  The page commands (text, screenshot, etc.) previously attached a CDP session
  to whichever tab Chrome's /json/list happened to list first, ignoring the
  tab activated by `tabs activate`. This was fixed by persisting the activated
  tab ID in the session file and preferring it in resolve_target().

  # --- Bug Is Fixed ---

  @regression
  Scenario: Page text reads from the activated tab
    Given a headless Chrome instance with three tabs
    And tab 1 is navigated to "https://example.com"
    And tab 2 is navigated to "https://httpbin.org"
    When I activate tab 1
    And I run page text as a separate invocation
    Then the page text output URL contains "example.com"

  @regression
  Scenario: Page screenshot captures the activated tab
    Given a headless Chrome instance with three tabs
    And tab 1 is navigated to "https://example.com"
    And tab 2 is navigated to "https://httpbin.org"
    When I activate tab 1
    And I run page screenshot as a separate invocation
    Then the screenshot is captured from the activated tab

  # --- Related Behavior Still Works ---

  @regression
  Scenario: Explicit --tab flag overrides persisted active tab
    Given a headless Chrome instance with three tabs
    And tab 1 is navigated to "https://example.com"
    And tab 2 is navigated to "https://httpbin.org"
    When I activate tab 1
    And I run page text with --tab targeting tab 2
    Then the page text output URL contains "httpbin.org"

  # --- Cross-Invocation Persistence ---

  @regression
  Scenario: Active tab persists across CLI invocations
    Given a headless Chrome instance with two tabs
    And tab 1 is navigated to "https://example.com"
    When I activate tab 1
    And the CLI process exits
    And a new CLI invocation runs page text
    Then the page text output URL contains "example.com"
