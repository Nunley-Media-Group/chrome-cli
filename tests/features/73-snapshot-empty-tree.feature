# File: tests/features/73-snapshot-empty-tree.feature
#
# Generated from: .claude/specs/73-fix-page-snapshot-empty-accessibility-tree/requirements.md
# Issue: #73
# Type: Defect regression

@regression
Feature: page snapshot returns populated accessibility tree on real-world websites
  The `page snapshot` command previously returned only the root node with
  no children on real-world websites like google.com, because the tree builder
  relied solely on `childIds` which Chrome left empty. This was fixed by adding
  a `parentId`-based fallback for tree reconstruction.

  # --- Bug Is Fixed ---

  @regression
  Scenario: snapshot returns populated tree when CDP nodes have parentId but empty childIds
    Given Chrome is running and navigated to a page with multiple accessible elements
    And the CDP Accessibility.getFullAXTree response contains nodes with parentId but empty childIds
    When the user runs "page snapshot"
    Then the accessibility tree contains more than just the root node
    And interactive elements are annotated with snapshot UIDs

  # --- Related Behavior Still Works ---

  @regression
  Scenario: snapshot still works when CDP nodes have valid childIds
    Given Chrome is running and navigated to a simple HTML page with known interactive elements
    And the CDP Accessibility.getFullAXTree response contains nodes with valid childIds
    When the user runs "page snapshot"
    Then the accessibility tree contains the expected hierarchy
    And interactive elements receive UIDs matching their roles

  # --- Edge Case ---

  @regression
  Scenario: snapshot handles nodes with both childIds and parentId without duplication
    Given Chrome is running and navigated to a page
    And the CDP response contains nodes where both childIds and parentId are present
    When the user runs "page snapshot"
    Then the tree is built using childIds (top-down) without duplication from parentId
    And the tree structure matches the childIds-defined hierarchy
