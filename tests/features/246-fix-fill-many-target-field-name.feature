# File: tests/features/246-fix-fill-many-target-field-name.feature
#
# Generated from: specs/bug-fix-fill-many-target-field-name/requirements.md
# Issue: #246
# Type: Defect regression

@regression
Feature: form fill-many accepts `target` field name consistently with the rest of the form API
  `form fill-many` previously required a `uid` key in each JSON entry, while every
  other `form` subcommand documents and accepts `target`. This was fixed by
  renaming `FillEntry.uid` to `target` with a `#[serde(alias = "uid")]` for
  backward compatibility, and by updating the error message, clap help, and
  inline examples.

  # --- Bug Is Fixed ---

  @regression
  Scenario: fill-many accepts `target` as the primary key
    Given agentchrome is built
    When I run "agentchrome form fill-many '[{\"target\":\"s1\",\"value\":\"hello\"}]' --help"
    Then the exit code should be 0
    And stdout should contain "target"

  @regression
  Scenario: error message for malformed payload references `target`, not `uid`
    Given agentchrome is built
    When I run "agentchrome form fill-many '[{\"nope\":\"s1\",\"value\":\"x\"}]'"
    Then the exit code should not be 0
    And stderr should contain "target"
    And stderr should not contain "expected array of {uid, value}"

  # --- Related Behavior Still Works ---

  @regression
  Scenario: legacy `uid` key is still accepted (silent alias, no deprecation warning)
    Given agentchrome is built
    When I run "agentchrome form fill-many '[{\"uid\":\"s1\",\"value\":\"hello\"}]' --help"
    Then the exit code should be 0
    And stderr should not contain "deprecated"

  @regression
  Scenario: fill-many --help documents `target` consistently
    Given agentchrome is built
    When I run "agentchrome form fill-many --help"
    Then the exit code should be 0
    And stdout should contain "target"
    And stdout should contain "--file"
    And stdout should contain "--include-snapshot"
