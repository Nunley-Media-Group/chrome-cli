# File: tests/features/media-control.feature
#
# Generated from: .claude/specs/feature-add-media-control-commands/requirements.md
# Issue: #193

Feature: Media Control Commands
  As a browser automation engineer working with media-heavy web applications
  I want built-in commands to list, play, pause, and seek audio/video elements
  So that I can control media playback without writing repetitive js exec boilerplate

  Background:
    Given agentchrome is built

  # --- Happy Path ---

  # AC1: List media elements
  Scenario: List media elements on a page
    Given a connected Chrome session on a page with audio and video elements
    When I run "agentchrome media list"
    Then the output is a JSON array
    And each object contains "index", "tag", "src", "currentSrc", "duration", "currentTime", "state", "muted", "volume", "loop", and "readyState" fields
    And each "tag" field is "audio" or "video"
    And each "index" is a zero-based integer
    And the exit code should be 0

  # AC2: Play a media element
  Scenario: Play a paused media element by index
    Given a connected Chrome session on a page with a paused audio element at index 0
    When I run "agentchrome media play 0"
    Then the output JSON should contain "state" equal to "playing"
    And the output JSON should contain "index" equal to 0
    And the output JSON should contain "tag" equal to "audio"
    And the exit code should be 0

  # AC3: Pause a media element
  Scenario: Pause a playing media element by index
    Given a connected Chrome session on a page with a playing audio element at index 0
    When I run "agentchrome media pause 0"
    Then the output JSON should contain "state" equal to "paused"
    And the output JSON should contain "index" equal to 0
    And the exit code should be 0

  # AC4: Seek a media element to a specific time
  Scenario: Seek a media element to a specific time
    Given a connected Chrome session on a page with an audio element at index 0 with duration 30 seconds
    When I run "agentchrome media seek 0 15.5"
    Then the output JSON should contain "currentTime" approximately equal to 15.5
    And the output JSON should contain "index" equal to 0
    And the exit code should be 0

  # AC5: Seek a media element to its end
  Scenario: Seek a media element to its end
    Given a connected Chrome session on a page with an audio element at index 0 with duration 30 seconds
    When I run "agentchrome media seek-end 0"
    Then the output JSON should contain "state" equal to "ended"
    And the output JSON should contain "currentTime" approximately equal to 30.0
    And the exit code should be 0

  # --- Bulk Operations ---

  # AC6: Bulk media control with --all flag
  Scenario: Seek all media elements to end with --all flag
    Given a connected Chrome session on a page with 3 audio elements
    When I run "agentchrome media seek-end --all"
    Then the output is a JSON array with 3 elements
    And each element has "state" equal to "ended"
    And the exit code should be 0

  Scenario: Play all media elements with --all flag
    Given a connected Chrome session on a page with 3 paused audio elements
    When I run "agentchrome media play --all"
    Then the output is a JSON array with 3 elements
    And each element has "state" equal to "playing"
    And the exit code should be 0

  # --- Frame Scoping ---

  # AC7: Frame-scoped media control
  Scenario: List media elements within a specific frame
    Given a connected Chrome session on a page with an iframe containing an audio element
    When I run "agentchrome media --frame 0 list"
    Then the output is a JSON array
    And the array contains only media elements from the specified frame
    And the exit code should be 0

  # --- Cross-validation ---

  # AC8: Cross-validation of state mutation via list
  Scenario: Play then list shows updated state
    Given a connected Chrome session on a page with a paused audio element at index 0
    And I have run "agentchrome media play 0"
    When I run "agentchrome media list"
    Then the output JSON array element at index 0 has "state" equal to "playing"

  # --- Edge Cases ---

  # AC9: No media elements on page
  Scenario: List media elements on a page with no media
    Given a connected Chrome session on a page with no audio or video elements
    When I run "agentchrome media list"
    Then the output is an empty JSON array
    And the exit code should be 0

  # AC10: Invalid media index
  Scenario: Play with an out-of-bounds index returns error
    Given a connected Chrome session on a page with 2 media elements
    When I run "agentchrome media play 5"
    Then a JSON error is output on stderr
    And the error message contains "not found"
    And the exit code should be non-zero

  # AC11: Seek beyond duration clamps to duration
  Scenario: Seek beyond duration clamps to end
    Given a connected Chrome session on a page with an audio element at index 0 with duration 30 seconds
    When I run "agentchrome media seek 0 999"
    Then the output JSON should contain "currentTime" less than or equal to 30.0
    And the exit code should be 0

  # --- Documentation ---

  # AC12: Documentation updated
  Scenario: Examples include media command group
    When I run "agentchrome examples media"
    Then the output contains media command examples
    And the exit code should be 0

  # --- Selector Targeting ---

  # AC13: Media element with selector targeting
  Scenario: Play a media element by CSS selector
    Given a connected Chrome session on a page with an audio element with class "narration"
    When I run "agentchrome media play css:audio.narration"
    Then the output JSON should contain "state" equal to "playing"
    And the output JSON should contain "tag" equal to "audio"
    And the exit code should be 0
