# File: tests/features/full-page-screenshot-scrollable-containers.feature
#
# Generated from: .claude/specs/feature-fix-full-page-screenshot-to-capture-scrollable-inner-containers/requirements.md
# Issue: #184
#
# CLI-testable scenarios: validation errors (AC5, AC6) that fail before Chrome connection.
# Chrome-dependent scenarios: AC1 (inner scroll capture), AC2 (default behavior),
#   AC3 (auto-detect warning), AC4 (invalid selector), AC7 (viewport restoration)
#   — verified via manual smoke test during /verifying-specs.

Feature: Full-page screenshot with scrollable inner containers
  As a developer or AI agent taking screenshots of pages with scrollable inner containers
  I want page screenshot --full-page to capture the full scrollable content of inner containers
  So that I get a complete visual record even when the page uses inner scrollable regions

  # --- Happy Path (Chrome-dependent) ---

  Scenario: Capture inner scrollable content with --scroll-container
    Given a page where the scrollable content is inside an inner container ".main-content"
    And the document.documentElement.scrollHeight equals the viewport height
    When I run page screenshot with --full-page and --scroll-container ".main-content"
    Then the command exits with code 0
    And the output JSON contains "format", "width", and "height" fields
    And the output JSON "height" is greater than the viewport height

  # --- No Regression (Chrome-dependent) ---

  Scenario: Default full-page behavior unchanged
    Given a standard page where the document body scrolls beyond the viewport
    When I run page screenshot with --full-page
    Then the command exits with code 0
    And the output JSON "height" reflects the full document scrollHeight
    And the output JSON "height" is greater than the viewport height

  # --- Auto-Detection (Chrome-dependent) ---

  Scenario: Auto-detect warning when full-page dimensions match viewport
    Given a page where the document scrollHeight equals the viewport height
    And the page has an inner scrollable container
    When I run page screenshot with --full-page
    Then the command exits with code 0
    And stderr contains "warning" and "scroll-container"
    And the screenshot is captured at viewport dimensions

  # --- Error Handling: CLI-testable ---

  Scenario: --scroll-container requires --full-page
    Given agentchrome is built
    When I run "agentchrome page screenshot --scroll-container .main-content"
    Then the exit code should be nonzero
    And stderr should contain "--scroll-container requires --full-page"

  Scenario: --scroll-container conflicts with --selector
    Given agentchrome is built
    When I run "agentchrome page screenshot --full-page --scroll-container .main --selector #logo"
    Then the exit code should be nonzero
    And stderr should contain "Cannot combine --scroll-container"

  Scenario: --scroll-container conflicts with --uid
    Given agentchrome is built
    When I run "agentchrome page screenshot --full-page --scroll-container .main --uid s1"
    Then the exit code should be nonzero
    And stderr should contain "Cannot combine --scroll-container"

  Scenario: --scroll-container conflicts with --clip
    Given agentchrome is built
    When I run "agentchrome page screenshot --full-page --scroll-container .main --clip 0,0,100,100"
    Then the exit code should be nonzero
    And stderr should contain "Cannot combine --scroll-container"

  # --- Error Handling: Chrome-dependent ---

  Scenario: Invalid scroll container selector
    Given a page is loaded
    When I run page screenshot with --full-page and --scroll-container ".nonexistent"
    Then the command exits with a non-zero code
    And stderr contains a JSON error with "Element not found"

  # --- State Restoration (Chrome-dependent) ---

  Scenario: Viewport restored after scroll-container capture
    Given a page with an inner scrollable container ".main-content"
    And the viewport is at default dimensions
    When I run page screenshot with --full-page and --scroll-container ".main-content"
    And then I run page screenshot without --full-page
    Then the second screenshot dimensions match the original viewport dimensions
