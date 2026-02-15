# File: tests/features/70-fix-enable-automation-flag.feature
#
# Generated from: .claude/specs/70-fix-enable-automation-flag/requirements.md
# Issue: #70
# Type: Defect regression

@regression
Feature: Chrome launched via connect --launch includes --enable-automation flag
  The Chrome launcher previously omitted the --enable-automation flag when
  spawning Chrome, so the automation infobar never appeared and certain
  automation behaviors were not enabled.
  This was fixed by adding --enable-automation to the hardcoded launch arguments.

  # --- Bug Is Fixed ---

  @regression
  Scenario: Automation flag is included on launch
    Given a LaunchConfig with default settings
    When Chrome is spawned via launch_chrome()
    Then the Chrome command-line arguments include "--enable-automation"

  # --- Related Behavior Still Works ---

  @regression
  Scenario: Headless mode is unaffected
    Given a LaunchConfig with headless mode enabled
    When Chrome is spawned via launch_chrome()
    Then the Chrome command-line arguments include "--enable-automation"
    And the Chrome command-line arguments include "--headless=new"

  # --- Edge Case ---

  @regression
  Scenario: Extra args do not conflict with automation flag
    Given a LaunchConfig with extra_args containing "--enable-automation"
    When Chrome is spawned via launch_chrome()
    Then Chrome launches without error
    And the Chrome command-line arguments include "--enable-automation"
