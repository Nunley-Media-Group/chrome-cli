Feature: Page Hit Test Command
  As a browser automation engineer debugging failed click interactions
  I want to see the full event delivery path for a click at specific coordinates
  So that I can identify overlays intercepting my clicks and find the correct interaction target

  # --- Happy Path (requires Chrome) ---

  # Scenario: Hit test at coordinates returns structured element info
  #   Given a page is loaded with a button at known coordinates covered by an overlay
  #   When "page hittest 100 200" is run
  #   Then the JSON output contains a "frame" field set to "main"
  #   And the JSON output contains a "hitTarget" object with "tag", "id", "class", and "uid" fields
  #   And the JSON output contains an "interceptedBy" field
  #   And the JSON output contains a "stack" array ordered by z-index highest first
  #   And each stack element has "tag", "id", "class", "uid", and "zIndex" fields

  # Scenario: Workaround suggestions for overlay interception
  #   Given a page is loaded with coordinates that hit an overlay above a button
  #   When "page hittest 100 200" is run at overlay-covered coordinates
  #   Then the "interceptedBy" field is not null
  #   And the "suggestion" field contains actionable text referencing the overlay selector
  #   And the "suggestion" field references the underlying target element

  # --- Alternative Paths ---

  # Scenario: Frame-scoped hit test
  #   Given a page is loaded with an iframe containing a form input
  #   When "page --frame 1 hittest 50 50" is run
  #   Then the "frame" field reflects the targeted frame
  #   And the "hitTarget" describes the element within that frame's context

  Scenario: Documentation includes page hittest examples
    Given agentchrome is built
    When I run "agentchrome examples page"
    Then stdout should contain "page hittest"

  # --- Error Handling ---

  # Scenario: Coordinates outside viewport return error
  #   Given a page is loaded with a viewport of 1280x720
  #   When "page hittest 5000 5000" is run
  #   Then a JSON error is written to stderr
  #   And the error message mentions coordinates are outside viewport bounds
  #   And the exit code is non-zero

  Scenario: No connection returns error
    Given agentchrome is built
    When I run "agentchrome page hittest 100 200"
    Then stderr should contain "connect"
    And the exit code should be nonzero

  # --- Edge Cases (requires Chrome) ---

  # Scenario: Null UID for non-accessible elements
  #   Given a page is loaded with a bare div that has no accessibility role
  #   When "page hittest" is run at the div's coordinates
  #   Then the "hitTarget" object has a "uid" field set to null
  #   And the "uid" field is present in the JSON (not omitted)

  # Scenario: Empty stack at bare coordinates
  #   Given a page is loaded with an area containing only html and body elements
  #   When "page hittest" is run at those bare coordinates
  #   Then the "stack" array contains document-level elements
  #   And the "interceptedBy" field is null
  #   And the "suggestion" field is null
