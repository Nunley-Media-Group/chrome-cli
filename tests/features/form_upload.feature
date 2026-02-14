# File: tests/features/form_upload.feature
#
# Generated from: .claude/specs/23-file-upload-to-page-elements/requirements.md
# Issue: #23

Feature: File upload to page elements
  As a developer or automation engineer
  I want to upload files to file input elements via the CLI
  So that my automation scripts can programmatically test file upload forms

  Background:
    Given Chrome is running with CDP enabled
    And a page is loaded with a file upload form

  # --- Happy Path ---

  Scenario: Upload a single file by UID
    Given an accessibility snapshot has been taken with UIDs assigned
    And a file input element has UID "s5"
    And a readable file exists at "/tmp/test-photo.jpg" with size 24576 bytes
    When I run "chrome-cli form upload s5 /tmp/test-photo.jpg"
    Then the exit code should be 0
    And the JSON output should contain "uploaded" equal to "s5"
    And the JSON output should contain "files" with 1 entry
    And the JSON output should contain "size" equal to 24576
    And a change event should have been dispatched on the element

  Scenario: Upload multiple files
    Given an accessibility snapshot has been taken with UIDs assigned
    And a file input element with multiple attribute has UID "s3"
    And readable files exist at "/tmp/doc1.pdf" and "/tmp/doc2.pdf"
    When I run "chrome-cli form upload s3 /tmp/doc1.pdf /tmp/doc2.pdf"
    Then the exit code should be 0
    And the JSON output should contain "uploaded" equal to "s3"
    And the JSON output should contain "files" with 2 entries
    And the JSON output "size" should equal the combined file sizes

  # --- Alternative Paths ---

  Scenario: Upload with --tab flag targets specific tab
    Given multiple tabs are open
    And a file input element exists in tab "ABCDEF" with UID "s5"
    And a readable file exists at "/tmp/test-file.txt"
    When I run "chrome-cli form upload s5 /tmp/test-file.txt --tab ABCDEF"
    Then the exit code should be 0
    And the file is uploaded in the specified tab

  Scenario: Upload with --include-snapshot flag
    Given an accessibility snapshot has been taken with UIDs assigned
    And a file input element has UID "s5"
    And a readable file exists at "/tmp/test-photo.jpg"
    When I run "chrome-cli form upload s5 /tmp/test-photo.jpg --include-snapshot"
    Then the exit code should be 0
    And the JSON output should contain a "snapshot" field
    And the snapshot state file should be updated

  Scenario: Upload by CSS selector
    Given the page has a file input with id "file-upload"
    And a readable file exists at "/tmp/document.pdf"
    When I run "chrome-cli form upload css:#file-upload /tmp/document.pdf"
    Then the exit code should be 0
    And the JSON output should contain "uploaded" equal to "css:#file-upload"

  # --- Error Handling ---

  Scenario: Error when file not found
    Given an accessibility snapshot has been taken with UIDs assigned
    And a file input element has UID "s5"
    When I run "chrome-cli form upload s5 /nonexistent/file.txt"
    Then the exit code should be nonzero
    And stderr should contain "File not found"

  Scenario: Error when element is not a file input
    Given an accessibility snapshot has been taken with UIDs assigned
    And a text input element has UID "s2"
    And a readable file exists at "/tmp/test-file.txt"
    When I run "chrome-cli form upload s2 /tmp/test-file.txt"
    Then the exit code should be nonzero
    And stderr should contain "not a file input"

  Scenario: Error when UID not found
    Given an accessibility snapshot has been taken with UIDs assigned
    And a readable file exists at "/tmp/test-file.txt"
    When I run "chrome-cli form upload s999 /tmp/test-file.txt"
    Then the exit code should be nonzero
    And stderr should contain "not found"

  Scenario: Error when file is not readable
    Given an accessibility snapshot has been taken with UIDs assigned
    And a file input element has UID "s5"
    And a file exists at "/tmp/secret.txt" that is not readable
    When I run "chrome-cli form upload s5 /tmp/secret.txt"
    Then the exit code should be nonzero
    And stderr should contain "not readable"

  Scenario: Error when required arguments missing
    When I run "chrome-cli form upload"
    Then the exit code should be nonzero
    And stderr should contain usage information

  # --- Edge Cases ---

  Scenario: Warning for large file upload
    Given an accessibility snapshot has been taken with UIDs assigned
    And a file input element has UID "s5"
    And a readable file exists at "/tmp/huge.bin" with size greater than 100MB
    When I run "chrome-cli form upload s5 /tmp/huge.bin"
    Then the exit code should be 0
    And the file should be uploaded successfully
    And stderr should contain a warning about the file being large
