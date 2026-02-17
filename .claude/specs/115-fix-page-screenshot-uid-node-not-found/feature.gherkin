# File: tests/features/115-fix-page-screenshot-uid-node-not-found.feature
#
# Generated from: .claude/specs/115-fix-page-screenshot-uid-node-not-found/requirements.md
# Issue: #115
# Type: Defect regression

@regression
Feature: Page screenshot --uid resolves element correctly
  The `page screenshot --uid` command previously failed with
  "Could not find node with given id" because `resolve_uid_clip()`
  did not ensure the DOM domain was active before issuing CDP commands.
  This was fixed by moving `ensure_domain("DOM")` into `resolve_uid_clip()`.

  Background:
    Given a Chrome instance is connected
    And I have navigated to a page with interactive elements
    And I have run "page snapshot" to assign UIDs

  # --- Bug Is Fixed ---

  @regression
  Scenario: Screenshot by UID works after snapshot
    When I run "page screenshot" with a valid UID and an output file
    Then the command exits with code 0
    And the screenshot file is created

  # --- Related Behavior Still Works ---

  @regression
  Scenario: JS exec by UID continues to work
    When I run "js exec" with a valid UID and expression "(el) => el.tagName"
    Then the command exits with code 0
    And the result contains a valid tag name
