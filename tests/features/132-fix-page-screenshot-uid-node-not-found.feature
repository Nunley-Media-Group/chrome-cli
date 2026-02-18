# File: tests/features/132-fix-page-screenshot-uid-node-not-found.feature
#
# Generated from: .claude/specs/132-fix-page-screenshot-uid-node-not-found/requirements.md
# Issue: #132
# Type: Defect regression

@regression
Feature: Page screenshot --uid no longer fails with 'Could not find node'
  The `page screenshot --uid` command previously failed with "Could not find node
  with given id" because `resolve_uid_clip()` did not call `DOM.getDocument` before
  `DOM.describeNode`, leaving Chrome's DOM agent without document context.
  This was fixed by adding a `DOM.getDocument` call before `DOM.describeNode`.

  Background:
    Given chrome-cli is built

  # --- Bug Is Fixed ---

  # These scenarios require a running Chrome instance with CDP enabled.
  # They are documented here for completeness but run only when integration
  # test infrastructure is available.

  # @regression
  # Scenario: AC1 — Screenshot by UID succeeds after snapshot
  #   Given a page has been loaded and "page snapshot" has assigned UIDs
  #   When I run "page screenshot --uid s1 --file /tmp/element.png"
  #   Then a PNG file is written to "/tmp/element.png" with exit code 0

  # --- Related Behavior Still Works ---

  # @regression
  # Scenario: AC2 — js exec --uid still works after snapshot
  #   Given a page has been loaded and "page snapshot" has assigned UIDs
  #   When I run "js exec --uid s1 \"(el) => el.tagName\""
  #   Then the element tag name is returned with exit code 0

  # --- Source-level regression: resolve_uid_clip uses backendNodeId directly ---

  @regression
  Scenario: resolve_uid_clip passes backendNodeId directly to DOM.getBoxModel
    Given chrome-cli is built
    When I check the resolve_uid_clip implementation
    Then it should pass backendNodeId directly to DOM.getBoxModel
