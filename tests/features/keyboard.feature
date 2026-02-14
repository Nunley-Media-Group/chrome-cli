# File: tests/features/keyboard.feature
#
# Generated from: GitHub Issue #15
# Issue: #15 - [cli] Keyboard input (typing, key presses, shortcuts)

Feature: Keyboard Input
  As a developer / automation engineer
  I want to simulate keyboard input via the CLI
  So that my automation scripts can type text and press keys programmatically

  # --- CLI Argument Validation (no Chrome required) ---

  Scenario: Type requires a text argument
    Given chrome-cli is built
    When I run "chrome-cli interact type"
    Then the exit code should be nonzero
    And stderr should contain "required"

  Scenario: Key requires a keys argument
    Given chrome-cli is built
    When I run "chrome-cli interact key"
    Then the exit code should be nonzero
    And stderr should contain "required"

  Scenario: Type help displays all options
    Given chrome-cli is built
    When I run "chrome-cli interact type --help"
    Then the exit code should be 0
    And stdout should contain "--delay"
    And stdout should contain "--include-snapshot"

  Scenario: Key help displays all options
    Given chrome-cli is built
    When I run "chrome-cli interact key --help"
    Then the exit code should be 0
    And stdout should contain "--repeat"
    And stdout should contain "--include-snapshot"

  Scenario: Interact help includes type and key subcommands
    Given chrome-cli is built
    When I run "chrome-cli interact --help"
    Then the exit code should be 0
    And stdout should contain "type"
    And stdout should contain "key"

  Scenario: Key rejects invalid key name
    Given chrome-cli is built
    When I run "chrome-cli interact key InvalidKeyName"
    Then the exit code should be nonzero
    And stderr should contain "Invalid key"

  Scenario: Key rejects duplicate modifier
    Given chrome-cli is built
    When I run "chrome-cli interact key Control+Control+A"
    Then the exit code should be nonzero
    And stderr should contain "Duplicate"

  # --- Type: Happy Paths (require Chrome) ---

  Scenario: Type text into focused element
    Given Chrome is running with CDP enabled
    And a page is loaded with a text input
    When I run "chrome-cli interact type 'Hello World'"
    Then the output JSON should contain "typed" equal to "Hello World"
    And the output JSON should contain "length" equal to 11
    And the exit code should be 0

  Scenario: Type with delay between keystrokes
    Given Chrome is running with CDP enabled
    And a page is loaded with a text input
    When I run "chrome-cli interact type 'abc' --delay 50"
    Then the output JSON should contain "typed" equal to "abc"
    And the output JSON should contain "length" equal to 3

  Scenario: Type with include-snapshot flag
    Given Chrome is running with CDP enabled
    And a page is loaded with a text input
    When I run "chrome-cli interact type 'test' --include-snapshot"
    Then the output JSON should contain "typed" equal to "test"
    And the output JSON should contain a "snapshot" field

  Scenario: Type handles Unicode and special characters
    Given Chrome is running with CDP enabled
    And a page is loaded with a text input
    When I run "chrome-cli interact type 'café'"
    Then the output JSON should contain "typed" equal to "café"
    And the output JSON should contain "length" equal to 4

  # --- Key: Happy Paths (require Chrome) ---

  Scenario: Press Enter key
    Given Chrome is running with CDP enabled
    And a page is loaded with interactive elements
    When I run "chrome-cli interact key Enter"
    Then the output JSON should contain "pressed" equal to "Enter"
    And the exit code should be 0

  Scenario: Press key combination
    Given Chrome is running with CDP enabled
    And a page is loaded with interactive elements
    When I run "chrome-cli interact key Control+A"
    Then the output JSON should contain "pressed" equal to "Control+A"

  Scenario: Press key with multiple modifiers
    Given Chrome is running with CDP enabled
    And a page is loaded with interactive elements
    When I run "chrome-cli interact key Control+Shift+ArrowDown"
    Then the output JSON should contain "pressed" equal to "Control+Shift+ArrowDown"

  Scenario: Press key with repeat
    Given Chrome is running with CDP enabled
    And a page is loaded with interactive elements
    When I run "chrome-cli interact key ArrowDown --repeat 3"
    Then the output JSON should contain "pressed" equal to "ArrowDown"
    And the output JSON should contain "repeat" equal to 3

  Scenario: Press key with include-snapshot flag
    Given Chrome is running with CDP enabled
    And a page is loaded with interactive elements
    When I run "chrome-cli interact key Tab --include-snapshot"
    Then the output JSON should contain "pressed" equal to "Tab"
    And the output JSON should contain a "snapshot" field

  # --- Supported Key Categories (require Chrome) ---

  Scenario Outline: Press supported keys from various categories
    Given Chrome is running with CDP enabled
    And a page is loaded with interactive elements
    When I run "chrome-cli interact key <key>"
    Then the output JSON should contain "pressed" equal to "<key>"
    And the exit code should be 0

    Examples:
      | key           |
      | a             |
      | Z             |
      | 5             |
      | F1            |
      | F12           |
      | ArrowUp       |
      | Home          |
      | Backspace     |
      | Tab           |
      | Escape        |
      | Space         |
      | Numpad0       |
      | NumpadAdd     |
      | Minus         |
      | Period        |
      | CapsLock      |

  # --- Plain Text Output ---

  Scenario: Plain text output for type
    Given Chrome is running with CDP enabled
    And a page is loaded with a text input
    When I run "chrome-cli interact type 'hello' --plain"
    Then the output should be plain text "Typed 5 characters"

  Scenario: Plain text output for key
    Given Chrome is running with CDP enabled
    And a page is loaded with interactive elements
    When I run "chrome-cli interact key Enter --plain"
    Then the output should be plain text "Pressed Enter"

  # --- Tab Targeting (require Chrome) ---

  Scenario: Type with tab targeting
    Given Chrome is running with CDP enabled
    And a specific tab with a focused text input
    When I run "chrome-cli interact type 'Hello' --tab ABC123"
    Then the text is typed in the specified tab
    And the exit code should be 0

  Scenario: Key press with tab targeting
    Given Chrome is running with CDP enabled
    And a specific tab with a focused element
    When I run "chrome-cli interact key Enter --tab ABC123"
    Then the key is pressed in the specified tab
    And the exit code should be 0
