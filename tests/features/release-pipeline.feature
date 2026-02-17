Feature: Cross-platform release pipeline
  As a developer contributing to chrome-cli
  I want automated CI/CD pipelines that build, test, lint, and release cross-platform binaries
  So that every PR is validated and releases produce optimized standalone binaries for all supported platforms

  # --- CI Workflow ---

  Scenario: CI workflow triggers on push to main
    Given the CI workflow file exists
    When I inspect the trigger configuration
    Then it triggers on push to "main" branch

  Scenario: CI workflow triggers on pull requests to main
    Given the CI workflow file exists
    When I inspect the trigger configuration
    Then it triggers on pull_request to "main" branch

  Scenario: CI workflow runs formatting check
    Given the CI workflow file exists
    When I inspect the check job steps
    Then it runs "cargo fmt --check"

  Scenario: CI workflow runs clippy
    Given the CI workflow file exists
    When I inspect the check job steps
    Then it runs "cargo clippy -- -D warnings"

  Scenario: CI workflow runs tests
    Given the CI workflow file exists
    When I inspect the check job steps
    Then it runs "cargo test"

  Scenario: CI workflow runs build
    Given the CI workflow file exists
    When I inspect the check job steps
    Then it runs "cargo build"

  # --- Release Workflow Trigger ---

  Scenario: Release workflow triggered by version tags
    Given the release workflow file exists
    When I inspect the trigger configuration
    Then it triggers on push of tags matching "v*"

  Scenario: Release workflow supports manual dispatch
    Given the release workflow file exists
    When I inspect the trigger configuration
    Then it supports workflow_dispatch

  # --- Build Matrix ---

  Scenario Outline: Release builds target platform on correct runner
    Given the release workflow has a build matrix
    When I inspect the matrix entry for "<target>"
    Then the runner is "<runner>"

    Examples:
      | target                        | runner            |
      | aarch64-apple-darwin          | macos-latest      |
      | x86_64-apple-darwin           | macos-latest      |
      | x86_64-unknown-linux-gnu      | ubuntu-latest     |
      | aarch64-unknown-linux-gnu     | ubuntu-24.04-arm  |
      | x86_64-pc-windows-msvc        | windows-latest    |

  # --- Archiving ---

  Scenario Outline: Target uses correct archive format
    Given the release workflow has a build matrix
    When I inspect the matrix entry for "<target>"
    Then the archive format is "<format>"

    Examples:
      | target                        | format  |
      | aarch64-apple-darwin          | tar.gz  |
      | x86_64-apple-darwin           | tar.gz  |
      | x86_64-unknown-linux-gnu      | tar.gz  |
      | aarch64-unknown-linux-gnu     | tar.gz  |
      | x86_64-pc-windows-msvc        | zip     |

  # --- Release Jobs ---

  Scenario: Release workflow has fail-fast disabled
    Given the release workflow has a build matrix
    Then fail-fast is disabled

  Scenario: Release workflow creates a draft release first
    Given the release workflow file exists
    When I inspect the create-release job
    Then it creates a draft GitHub Release

  Scenario: Release workflow has a cleanup job for failures
    Given the release workflow file exists
    Then it has a cleanup-release job that runs on failure

  # --- Security ---

  Scenario: Release workflow uses minimal permissions
    Given the release workflow file exists
    Then the workflow permissions include "contents" as "write"

  Scenario: Actions are pinned by commit SHA
    Given the CI workflow file exists
    Then all action references use commit SHA pins
