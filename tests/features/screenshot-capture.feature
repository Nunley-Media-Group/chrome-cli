# File: tests/features/screenshot-capture.feature
#
# Generated from: .claude/specs/screenshot-capture/requirements.md
# Issue: #12

Feature: Screenshot capture
  As a developer / automation engineer
  I want to capture screenshots of browser pages via the CLI
  So that I can visually verify page state and debug rendering issues

  Background:
    Given Chrome is running with CDP enabled

  # --- Happy Path ---

  Scenario: Capture viewport screenshot (default)
    Given a page is loaded at "https://example.com"
    When I run "chrome-cli page screenshot"
    Then stdout contains JSON with keys "format", "data", "width", "height"
    And the "format" field is "png"
    And the "data" field is a non-empty base64 string
    And the "width" and "height" are positive integers

  Scenario: Save screenshot to file
    Given a page is loaded at "https://example.com"
    When I run "chrome-cli page screenshot --file /tmp/test-screenshot.png"
    Then a valid PNG file exists at "/tmp/test-screenshot.png"
    And stdout contains JSON with keys "format", "file", "width", "height"
    And the "file" field is "/tmp/test-screenshot.png"
    And the "data" key is not present in the JSON output

  Scenario: Target a specific tab
    Given multiple tabs are open
    When I run "chrome-cli page screenshot --tab <ID>"
    Then the screenshot is captured from the specified tab

  # --- Full-Page ---

  Scenario: Full-page screenshot
    Given a page with content exceeding the viewport height
    When I run "chrome-cli page screenshot --full-page"
    Then stdout contains JSON with a "height" greater than the viewport height
    And the screenshot captures the entire scrollable page content

  # --- Element Targeting ---

  Scenario: Element screenshot by CSS selector
    Given a page with an element matching "#logo"
    When I run "chrome-cli page screenshot --selector '#logo'"
    Then the screenshot captures only the bounding box of that element
    And the "width" and "height" reflect the element's dimensions

  Scenario: Element screenshot by accessibility UID
    Given a page with a snapshot captured and element UID "s1" assigned
    When I run "chrome-cli page screenshot --uid s1"
    Then the screenshot captures only the element with UID "s1"
    And the "width" and "height" reflect the element's dimensions

  # --- Format Options ---

  Scenario: JPEG format
    Given a page is loaded
    When I run "chrome-cli page screenshot --format jpeg"
    Then the "format" field is "jpeg"

  Scenario: WebP format
    Given a page is loaded
    When I run "chrome-cli page screenshot --format webp"
    Then the "format" field is "webp"

  Scenario: Custom quality for JPEG
    Given a page is loaded
    When I run "chrome-cli page screenshot --format jpeg --quality 50"
    Then the screenshot is captured with JPEG quality 50

  # --- Region Clipping ---

  Scenario: Region clipping
    Given a page is loaded
    When I run "chrome-cli page screenshot --clip 10,20,200,100"
    Then the screenshot captures region (10, 20, 200, 100)
    And the "width" is 200 and "height" is 100

  # --- Error Handling ---

  Scenario: Conflicting --full-page with --selector
    Given a page is loaded
    When I run "chrome-cli page screenshot --full-page --selector '#logo'"
    Then stderr contains a JSON error about mutually exclusive flags
    And the exit code is non-zero

  Scenario: Conflicting --full-page with --uid
    Given a page is loaded
    When I run "chrome-cli page screenshot --full-page --uid s1"
    Then stderr contains a JSON error about mutually exclusive flags
    And the exit code is non-zero

  Scenario: Non-existent CSS selector
    Given a page is loaded
    When I run "chrome-cli page screenshot --selector '#does-not-exist'"
    Then stderr contains a JSON error about element not found
    And the exit code is non-zero

  Scenario: Non-existent UID
    Given a page with a snapshot captured
    When I run "chrome-cli page screenshot --uid s999"
    Then stderr contains a JSON error about UID not found
    And the exit code is non-zero

  # --- Edge Cases ---

  Scenario: Blank page screenshot
    Given a blank page is loaded at "about:blank"
    When I run "chrome-cli page screenshot"
    Then a screenshot is captured successfully
    And the exit code is 0

  Scenario: Quality ignored for PNG format
    Given a page is loaded
    When I run "chrome-cli page screenshot --format png --quality 50"
    Then the screenshot is captured normally
    And the "format" field is "png"
