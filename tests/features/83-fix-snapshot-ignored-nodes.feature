# File: tests/features/83-fix-snapshot-ignored-nodes.feature
#
# Generated from: .claude/specs/83-fix-snapshot-ignored-nodes/requirements.md
# Issue: #83
# Type: Defect regression

@regression
Feature: Snapshot promotes children of ignored accessibility nodes
  The `build_subtree` function previously returned `None` for ignored nodes,
  discarding their entire subtree. This caused page snapshots to return empty
  trees on real-world pages where Chrome wraps visible content in ignored
  structural containers. The fix makes ignored nodes transparent by promoting
  their children to the nearest non-ignored ancestor.

  # --- Bug Is Fixed ---

  @regression
  Scenario: Ignored ancestor nodes do not discard visible descendants
    Given CDP returns an accessibility tree where the root's children are ignored nodes
    And those ignored nodes have non-ignored descendants (headings, paragraphs, links)
    When the snapshot tree is built
    Then the root node's children include the promoted non-ignored descendants
    And interactive elements among the promoted descendants have UIDs assigned

  # --- Related Behavior Still Works ---

  @regression
  Scenario: Non-ignored trees render identically to previous behavior
    Given CDP returns an accessibility tree with no ignored intermediate nodes
    When the snapshot tree is built
    Then the tree structure matches the CDP hierarchy exactly
    And interactive elements have UIDs assigned in depth-first order

  # --- Edge Case ---

  @regression
  Scenario: Deeply nested ignored chains promote through all levels
    Given CDP returns an accessibility tree with three consecutive ignored ancestor nodes
    And the deepest ignored node has non-ignored children
    When the snapshot tree is built
    Then the non-ignored children appear as direct children of the nearest non-ignored ancestor
    And the children appear in depth-first traversal order
