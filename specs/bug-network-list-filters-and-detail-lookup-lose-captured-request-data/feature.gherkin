# File: tests/features/285-network-list-filters-and-detail-lookup-lose-captured-request-data.feature
#
# Generated from: specs/bug-network-list-filters-and-detail-lookup-lose-captured-request-data/requirements.md
# Issue: #285
# Type: Defect regression

@regression
Feature: Network list filters and detail lookup retain captured request data
  The `network list` command previously returned partially captured document
  requests, `network list --type document` could lose those document requests,
  and `network get <id-from-list>` could fail to resolve a listed request ID.
  The fix preserves correlated request data and stabilizes the list-to-get
  workflow for the same active Chrome target.

  Background:
    Given Chrome is connected in headless mode
    And a deterministic network test page has completed a document request

  # --- Bug Is Fixed ---

  @regression @requires-chrome
  Scenario: AC1 - document type filter returns captured document requests
    When I run `agentchrome network list --type document --pretty`
    Then the command exits with code 0
    And stdout is a JSON array containing at least one request
    And every returned request has `"type":"document"`

  @regression @requires-chrome
  Scenario: AC2 - completed request metadata is populated when available
    When I run `agentchrome network list --pretty`
    Then the command exits with code 0
    And stdout is a JSON array containing at least one completed request
    And each completed request includes non-null `"status"`, `"size"`, `"duration_ms"`, and `"timestamp"` when CDP provides those values

  @regression @requires-chrome
  Scenario: AC3 - detail lookup resolves an ID returned by list
    Given `agentchrome network list --pretty` returned a request id
    When I run `agentchrome network get <request-id> --pretty`
    Then the command exits with code 0
    And stdout is valid JSON with `"request"`, `"response"`, and `"timing"` sections

  # --- Regression Is Automatable ---

  @regression @requires-chrome
  Scenario: AC4 - focused regression covers list, filter, and get without manual inspection
    When I run the issue 285 network list/filter/get regression workflow
    Then the document filter, completed metadata, and detail lookup assertions all pass
    And no manual browser inspection is required

  # --- Related Behavior Still Works ---

  @regression @requires-chrome
  Scenario: Genuine missing request IDs still return typed JSON errors
    Given the latest network list output does not contain request id 999999
    When I run `agentchrome network get 999999 --pretty`
    Then the command exits with code 1
    And stderr is valid JSON containing `"error":"Network request 999999 not found"`
