# File: tests/features/iframe-frame-targeting.feature
#
# Generated from: .claude/specs/feature-add-iframe-frame-targeting-support/requirements.md
# Issue: #189

Feature: Iframe/Frame Targeting Support
  As a browser automation engineer working with enterprise web applications
  I want to target specific iframe contexts when running AgentChrome commands
  So that I can automate applications that embed content in iframes

  Background:
    Given a Chrome instance is connected
    And the test fixture "iframe-frame-targeting.html" is loaded

  # --- Frame Enumeration ---

  Scenario: AC1 - List frames on a page
    Given the page contains nested iframes
    When "page frames" is run
    Then the output is a JSON array
    And each frame has fields "index", "id", "url", "name", "securityOrigin", "unreachable", "width", "height", "depth"
    And the array contains at least 3 entries

  Scenario: AC2 - Main frame listed at index 0
    When "page frames" is run
    Then the first frame has "index" equal to 0
    And the first frame has "depth" equal to 0
    And child iframes have "index" starting from 1

  # --- Frame-Scoped Page Commands ---

  Scenario: AC3 - Target a specific frame with page snapshot
    Given an iframe at index 1 contains a button labeled "IFrame Submit"
    When "page snapshot --frame 1" is run
    Then the accessibility tree contains "IFrame Submit"
    And the accessibility tree does not contain "Main Page Heading"

  Scenario: AC3 - Target a specific frame with page screenshot
    When "page screenshot --frame 1 --file /tmp/frame-screenshot.png" is run
    Then the exit code is 0
    And the screenshot file is created

  Scenario: AC3 - Target a specific frame with page text
    Given an iframe at index 1 contains the text "iframe-only-content"
    When "page text --frame 1" is run
    Then the output contains "iframe-only-content"
    And the output does not contain "main-page-only-content"

  # --- Frame-Scoped JS Execution ---

  Scenario: AC4 - Target a specific frame with js exec
    Given an iframe at index 1 has document title "Child Frame"
    And the main page has document title "Parent Page"
    When "js exec --frame 1 document.title" is run
    Then the result value is "Child Frame"

  # --- Frame-Scoped DOM Commands ---

  Scenario: AC5 - Target a specific frame with dom select
    Given an iframe at index 1 contains an element with id "iframe-element"
    When "dom select --frame 1 css:#iframe-element" is run
    Then the output contains the element
    And the exit code is 0

  Scenario: AC5 - Target a specific frame with dom get-text
    Given an iframe at index 1 contains a paragraph with text "iframe paragraph text"
    When "dom get-text --frame 1 css:#iframe-paragraph" is run
    Then the result contains "iframe paragraph text"

  # --- Frame-Scoped Interact Commands ---

  Scenario: AC6 - Target a specific frame with interact click
    Given a page snapshot of frame 1 assigned UID "s1" to a button
    When "interact click --frame 1 s1" is run
    Then the exit code is 0

  Scenario: AC6 - Target a specific frame with interact click-at
    When "interact click-at --frame 1 50 50" is run
    Then the exit code is 0
    And coordinates are translated relative to the iframe viewport

  Scenario: AC6 - Target a specific frame with interact type
    Given a page snapshot of frame 1 assigned UID "s2" to a text input
    When "interact type --frame 1 s2 hello" is run
    Then the exit code is 0

  # --- Frame-Scoped Form Commands ---

  Scenario: AC7 - Target a specific frame with form fill
    Given a page snapshot of frame 1 assigned UID "s2" to a text input
    When "form fill --frame 1 s2 test-value" is run
    Then the exit code is 0
    And the text input in frame 1 contains "test-value"

  Scenario: AC7 - Target a specific frame with form submit
    Given a page snapshot of frame 1 assigned UID "s3" to a form
    When "form submit --frame 1 s3" is run
    Then the exit code is 0

  # --- Cross-Origin Iframe Access ---

  Scenario: AC8 - Cross-origin iframe access
    Given a cross-origin iframe exists at index 2
    When "page snapshot --frame 2" is run
    Then the accessibility tree contains content from the cross-origin iframe
    And the exit code is 0

  Scenario: AC8 - Cross-origin iframe js exec
    Given a cross-origin iframe exists at index 2
    When "js exec --frame 2 document.title" is run
    Then the result is the cross-origin iframe's document title
    And the exit code is 0

  # --- UID Consistency ---

  Scenario: AC9 - Frame-scoped UIDs are consistent across commands
    Given "page snapshot --frame 1" assigned UID "s3" to a button
    When "interact click --frame 1 s3" is run
    Then the click targets the same button that was assigned UID "s3"
    And the exit code is 0

  # --- Default Behavior ---

  Scenario: AC10 - No --frame flag defaults to main frame
    When "page snapshot" is run without --frame
    Then the accessibility tree contains "Main Page Heading"
    And the behavior is identical to current implementation

  Scenario: AC11 - --frame 0 targets the main frame
    When "page snapshot --frame 0" is run
    Then the accessibility tree contains "Main Page Heading"
    And the result is identical to running without --frame

  # --- Error Handling ---

  Scenario: AC12 - Invalid frame index error
    When "page snapshot --frame 99" is run
    Then the exit code is 3
    And stderr contains a JSON error with "Frame index 99 not found"
    And stderr JSON contains "code" equal to 3

  Scenario: AC13 - Frame removed during command execution
    Given an iframe at index 1 is removed via JavaScript during command execution
    When a frame-targeted command encounters the missing frame
    Then the exit code is 3
    And stderr contains a JSON error about the frame being unavailable

  # --- Documentation ---

  Scenario: AC14 - Documentation and examples updated
    When "examples page" is run
    Then the output contains "page frames"
    And the output contains "--frame"
    When "examples interact" is run
    Then the output contains "--frame"

  # --- Automatic Frame Detection ---

  Scenario: AC15 - Automatic frame detection with --frame auto
    Given UID "s5" exists only inside an iframe at index 2
    When "interact click --frame auto s5" is run
    Then the exit code is 0
    And the JSON output contains "frame" equal to 2

  Scenario: AC16 - Automatic frame detection with no match
    When "interact click --frame auto s999" is run
    Then the exit code is 3
    And stderr contains "Element not found in any frame"

  # --- Nested Frame Path ---

  Scenario: AC17 - Nested iframe path syntax
    Given the iframe at index 1 contains a child iframe
    When "page snapshot --frame 1/0" is run
    Then the snapshot targets the first child of the iframe at index 1
    And the exit code is 0

  Scenario: AC18 - Invalid nested frame path error
    When "page snapshot --frame 1/5" is run
    Then the exit code is 3
    And stderr contains a JSON error about the path segment failing

  # --- Worker Targeting ---

  Scenario: AC19 - List workers on a page
    Given the page has a registered Service Worker
    When "page workers" is run
    Then the output is a JSON array
    And each worker has fields "index", "id", "type", "url", "status"

  Scenario: AC20 - Target a worker with js exec
    Given a Service Worker is registered at index 0
    When "js exec --worker 0 self.registration.scope" is run
    Then the result reflects the worker's scope
    And the exit code is 0

  Scenario: AC21 - Invalid worker index error
    When "js exec --worker 99" is run
    Then the exit code is 3
    And stderr contains "Worker index 99 not found"

  # --- Frame-Scoped Network ---

  Scenario: AC22 - Frame-scoped network monitoring
    Given an iframe at index 1 has made network requests
    When "network list --frame 1" is run
    Then only requests from frame 1 are listed
    And requests from the main frame are excluded

  Scenario: AC23 - Frame-scoped network interception
    Given network interception is enabled for frame 1 with url pattern "*.js"
    When the iframe at index 1 loads a JavaScript file
    Then the request is intercepted
    And requests from other frames pass through unmodified

  # --- Legacy Frameset ---

  Scenario: AC24 - Legacy frameset support
    Given the page contains a frameset with frame elements
    When "page frames" is run
    Then frame elements appear in the frame list with standard metadata
    And "--frame <index>" targeting works for frame elements

  # --- Shadow DOM ---

  Scenario: AC25 - Shadow DOM traversal in snapshots
    Given the page contains a web component with an open shadow DOM root
    And the shadow root contains a button labeled "Shadow Button"
    When "page snapshot --pierce-shadow" is run
    Then the accessibility tree contains "Shadow Button"
    And the shadow DOM button has a UID assigned

  Scenario: AC26 - Shadow DOM traversal in dom commands
    Given the shadow root contains an element with id "shadow-element"
    When "dom select --pierce-shadow css:#shadow-element" is run
    Then the element inside the shadow root is returned
    And the exit code is 0

  Scenario: AC27 - Shadow DOM via UID without --pierce-shadow
    Given "page snapshot --pierce-shadow" assigned UID "s7" to a shadow DOM button
    When "interact click s7" is run
    Then the shadow DOM button is clicked
    And no --pierce-shadow flag is needed on the interact command

  Scenario: AC28 - Combined frame and shadow DOM targeting
    Given an iframe at index 1 contains a web component with shadow DOM
    When "page snapshot --frame 1 --pierce-shadow" is run
    Then the accessibility tree includes shadow DOM content from within the iframe
    And UIDs assigned to shadow elements work with subsequent "--frame 1" commands
