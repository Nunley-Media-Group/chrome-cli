# File: tests/features/227-fix-interact-key-keyup-event.feature
#
# Generated from: specs/bug-fix-interact-key-keyup-event/requirements.md
# Issue: #227
# Type: Defect regression

@regression
Feature: interact key fires keyup listeners on the target page
  `agentchrome interact key <KEY>` previously sent CDP `Input.dispatchKeyEvent`
  payloads missing `windowsVirtualKeyCode` and `text`, and mis-mapped `Enter`
  to `"\r"` on the DOM `key` field. As a result, page listeners bound via
  `document.addEventListener('keyup', …)` or `$(document).keyup(…)` never
  observed a usable event. This was fixed by enriching the CDP payload with
  `windowsVirtualKeyCode` + `text` (on keyDown for printable keys) and by
  correcting the `Enter` / `Tab` values in `cdp_key_value`.

  Background:
    Given agentchrome is connected to a headless Chrome instance
    And the page at tests/fixtures/interact-key-keyup-event.html is loaded
    And the element with id "target" is focused via `interact click target`

  # --- Bug Is Fixed: letter key ---

  @regression
  Scenario: interact key A fires keyup with event.key === "A"
    When I run `agentchrome interact key A`
    Then the element with id "result" reads "You entered: A"
    And the captured keyup event has `event.key` equal to "A"
    And the captured keyup event has `event.keyCode` equal to 65
    And the captured keyup event has `event.which` equal to 65

  # --- Bug Is Fixed: Enter ---

  @regression
  Scenario: interact key Enter fires keyup with event.key === "Enter"
    When I run `agentchrome interact key Enter`
    Then the element with id "result" reads "You entered: ENTER"
    And the captured keyup event has `event.key` equal to "Enter"
    And the captured keyup event has `event.keyCode` equal to 13

  # --- No Regression: modifier combination ---

  @regression
  Scenario: interact key Shift+A preserves modifier behavior
    When I run `agentchrome interact key "Shift+A"`
    Then the captured keydown event has `event.key` equal to "A"
    And the captured keydown event has `event.shiftKey` equal to true
    And the captured keydown event has `event.code` equal to "KeyA"

  # --- No Regression: interact type (char-synthesis path) ---

  @regression
  Scenario: interact type hello still writes characters via the char path
    When I run `agentchrome interact type "hello"`
    Then the element with id "target" has value "hello"
