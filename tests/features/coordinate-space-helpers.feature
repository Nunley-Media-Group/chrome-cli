Feature: Coordinate Space Helpers for Frame-Aware Coordinate Resolution
  As a browser automation engineer working with coordinate-based interactions across frames
  I want coordinate resolution helpers that translate between frame-local and page-level coordinates plus element-relative and percentage-based coordinate options
  So that I can reliably target elements without manually recalculating coordinates when viewport dimensions shift between the main page and iframes

  # --- Happy Path: page coords (requires Chrome) ---

  # Scenario: AC1 — Resolve coordinates for a selector in the main frame
  #   Given the test fixture "tests/fixtures/coordinate-space-helpers.html" is loaded
  #   And the main-frame element "#submit" has bounding client rect {x: 100, y: 200, width: 80, height: 32}
  #   When I run "page coords --selector css:#submit"
  #   Then the exit code is 0
  #   And stdout JSON field "frame.index" equals 0
  #   And stdout JSON field "frameLocal.boundingBox.x" equals 100
  #   And stdout JSON field "frameLocal.boundingBox.y" equals 200
  #   And stdout JSON field "frameLocal.boundingBox.width" equals 80
  #   And stdout JSON field "frameLocal.boundingBox.height" equals 32
  #   And stdout JSON field "frameLocal.center.x" equals 140
  #   And stdout JSON field "frameLocal.center.y" equals 216
  #   And stdout JSON field "page.boundingBox.x" equals 100
  #   And stdout JSON field "page.boundingBox.y" equals 200
  #   And stdout JSON field "page.center.x" equals 140
  #   And stdout JSON field "page.center.y" equals 216
  #   And stdout JSON field "frameOffset.x" equals 0
  #   And stdout JSON field "frameOffset.y" equals 0

  # Scenario: AC2 — Resolve coordinates for a selector in a nested iframe
  #   Given the test fixture is loaded
  #   And the iframe at index 1 is offset (50, 100) in the page
  #   And the element "#inner" inside frame 1 has bounding client rect {x: 10, y: 20, width: 80, height: 32}
  #   When I run "page --frame 1 coords --selector css:#inner"
  #   Then the exit code is 0
  #   And stdout JSON field "frame.index" equals 1
  #   And stdout JSON field "frameLocal.boundingBox.x" equals 10
  #   And stdout JSON field "frameLocal.boundingBox.y" equals 20
  #   And stdout JSON field "page.boundingBox.x" equals 60
  #   And stdout JSON field "page.boundingBox.y" equals 120
  #   And stdout JSON field "frameOffset.x" equals 50
  #   And stdout JSON field "frameOffset.y" equals 100

  # --- Happy Path: --relative-to on coord-dispatching commands (requires Chrome) ---

  # Scenario: AC4 — Click at an absolute offset within an element
  #   Given the test fixture is loaded
  #   When I run "interact click-at 10 5 --relative-to css:button"
  #   Then the exit code is 0
  #   And stdout JSON field "clicked_at.x" equals 110
  #   And stdout JSON field "clicked_at.y" equals 205

  # Scenario: AC5 — Click at a percentage position within an element
  #   Given the test fixture is loaded
  #   When I run "interact click-at 50% 50% --relative-to css:#target"
  #   Then the exit code is 0
  #   And stdout JSON field "clicked_at.x" equals 200
  #   And stdout JSON field "clicked_at.y" equals 250

  # Scenario Outline: AC5 — 0% and 100% boundary percentages resolve to element edges
  #   Given the test fixture is loaded
  #   When I run "interact click-at <x> <y> --relative-to css:#box"
  #   Then stdout JSON field "clicked_at.x" equals <px>
  #   And stdout JSON field "clicked_at.y" equals <py>
  #   Examples:
  #     | x    | y    | px  | py  |
  #     | 0%   | 0%   | 100 | 200 |
  #     | 100% | 100% | 299 | 299 |
  #     | 50%  | 0%   | 200 | 200 |
  #     | 0%   | 50%  | 100 | 250 |

  # Scenario: AC7 — drag-at with --relative-to dispatches at resolved coordinates
  #   Given the test fixture is loaded
  #   When I run "interact drag-at 0 0 100% 100% --relative-to css:div"
  #   Then the exit code is 0
  #   And stdout JSON field "dragged_at.from.x" equals 100
  #   And stdout JSON field "dragged_at.from.y" equals 200
  #   And stdout JSON field "dragged_at.to.x" equals 299
  #   And stdout JSON field "dragged_at.to.y" equals 299

  # Scenario: AC8 — --relative-to combined with --frame resolves in frame then applies offset
  #   Given the test fixture is loaded
  #   When I run "interact --frame 1 click-at 50% 50% --relative-to css:button"
  #   Then the exit code is 0
  #   And stdout JSON field "clicked_at.x" equals 100
  #   And stdout JSON field "clicked_at.y" equals 136

  # --- Error Handling (requires Chrome) ---

  # Scenario: AC9 — Missing selector on page coords produces structured error
  #   Given the test fixture is loaded
  #   When I run "page coords --selector css:#does-not-exist"
  #   Then the exit code is 3
  #   And stderr is a single JSON object with fields "error" and "code"
  #   And stdout is empty

  # Scenario: AC11 — --relative-to with missing element errors before dispatch
  #   Given the test fixture is loaded
  #   When I run "interact click-at 50% 50% --relative-to css:#missing"
  #   Then the exit code is 3
  #   And stderr contains exactly one JSON error object
  #   And no mouse event was dispatched to Chrome

  # --- Error Handling (does not require Chrome) ---

  Scenario: AC10 — Invalid percentage value rejected by clap before dispatch
    Given agentchrome is built
    When I run "agentchrome interact click-at 150% 50% --relative-to css:button"
    Then the exit code should be nonzero
    And stderr should contain "error"

  Scenario: Percentage coordinates without --relative-to are rejected
    Given agentchrome is built
    When I run "agentchrome interact click-at 50% 50%"
    Then the exit code should be nonzero
    And stderr should contain "relative-to"

  # --- Regression guard: no-connection errors ---

  Scenario: page coords with no connection returns error
    Given agentchrome is built
    When I run "agentchrome page coords --selector css:#submit"
    Then stderr should contain "connect"
    And the exit code should be nonzero

  Scenario: Absolute-coordinate click-at with no connection returns error
    Given agentchrome is built
    When I run "agentchrome interact click-at 150 250"
    Then stderr should contain "connect"
    And the exit code should be nonzero

  # --- Documentation discoverability ---

  Scenario: AC12 — examples interact includes --relative-to coordinate helper examples
    Given agentchrome is built
    When I run "agentchrome examples interact"
    Then stdout should contain "click-at"
    And stdout should contain "--relative-to"
    And stdout should contain "50%"

  Scenario: AC12 — examples page includes page coords examples
    Given agentchrome is built
    When I run "agentchrome examples page"
    Then stdout should contain "page coords --selector"
    And stdout should contain "--frame"
