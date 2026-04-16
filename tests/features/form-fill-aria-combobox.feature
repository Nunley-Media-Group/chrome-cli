# File: tests/features/form-fill-aria-combobox.feature
#
# Generated from: .claude/specs/feature-enhance-form-fill-to-handle-aria-combobox-elements/requirements.md
# Issue: #196

Feature: Form Fill ARIA Combobox Support
  As a browser automation engineer working with modern web applications
  I want form fill to automatically handle ARIA combobox elements
  So that I can fill combobox fields with a single command instead of manually composing click-type-confirm sequences

  Background:
    Given a connected Chrome session
    And a page with ARIA combobox elements is loaded

  # --- Happy Path ---

  Scenario: Auto-handle combobox elements
    Given an element with role "combobox" at UID "s5" with options "Acme Corp", "Beta Inc", "Gamma LLC"
    When I run form fill on "s5" with value "Acme Corp"
    Then the command exits with code 0
    And stdout JSON has "filled" equal to "s5"
    And stdout JSON has "value" equal to "Acme Corp"
    And the combobox displays "Acme Corp" as the selected value

  # --- Regression Guard ---

  Scenario: Preserve existing select behavior
    Given a standard select element at UID "s3" with options "Red", "Green", "Blue"
    When I run form fill on "s3" with value "Green"
    Then the command exits with code 0
    And stdout JSON has "filled" equal to "s3"
    And stdout JSON has "value" equal to "Green"
    And the select element shows "Green" as selected

  # --- Error Handling ---

  Scenario: Combobox value not found
    Given an element with role "combobox" at UID "s5" with options "Acme Corp", "Beta Inc", "Gamma LLC"
    When I run form fill on "s5" with value "Nonexistent"
    Then the command exits with code 1
    And stderr JSON has "error" containing "No matching option found in combobox"

  # --- Alternative Path ---

  Scenario: Configurable confirmation key
    Given an element with role "combobox" at UID "s5" that confirms selection on Tab
    When I run form fill on "s5" with value "Acme Corp" and confirm-key "Tab"
    Then the command exits with code 0
    And stdout JSON has "filled" equal to "s5"
    And stdout JSON has "value" equal to "Acme Corp"

  # --- Documentation ---

  Scenario: Documentation updated with combobox examples
    When I run examples for the "form" command group
    Then the output includes a combobox fill example
    And the output includes a confirm-key example

  # --- Cross-Feature Integration ---

  Scenario: Combobox in fill-many batch
    Given a text input at UID "s3" and a combobox at UID "s5"
    When I run form fill-many with JSON '[{"uid":"s3","value":"John"},{"uid":"s5","value":"Acme Corp"}]'
    Then the command exits with code 0
    And the output contains fill results for both "s3" and "s5"
    And the text input "s3" shows value "John"
    And the combobox "s5" displays "Acme Corp" as the selected value

  # --- Edge Case ---

  Scenario: Combobox with dropdown render delay
    Given an element with role "combobox" at UID "s7" with delayed option loading
    When I run form fill on "s7" with value "Async Result"
    Then the command exits with code 0
    And stdout JSON has "filled" equal to "s7"
    And stdout JSON has "value" equal to "Async Result"
    And the combobox waited for options to appear before confirming
