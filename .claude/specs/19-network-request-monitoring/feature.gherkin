# File: tests/features/network.feature
#
# Generated from: .claude/specs/19-network-request-monitoring/requirements.md
# Issue: #19

Feature: Network request monitoring
  As a developer / automation engineer
  I want to monitor and inspect HTTP network requests from the command line
  So that I can debug network issues, audit API calls, and automate network analysis in scripts and CI pipelines

  Background:
    Given Chrome is running with CDP enabled

  # --- Happy Path: List ---

  Scenario: List network requests from the current page
    Given a page is loaded that has made network requests
    When I run "chrome-cli network list"
    Then the output is a JSON array
    And each entry contains "id", "method", "url", "status", "type", "size", "duration_ms", and "timestamp"
    And the exit code should be 0

  Scenario: List network requests targeting a specific tab
    Given multiple tabs are open with network activity
    When I run "chrome-cli network list --tab <TAB_ID>"
    Then only requests from the specified tab are returned

  # --- Filtering ---

  Scenario Outline: Filter by resource type
    Given a page has made requests of various resource types
    When I run "chrome-cli network list --type <types>"
    Then only requests of type <types> are returned

    Examples:
      | types       |
      | xhr         |
      | fetch       |
      | xhr,fetch   |
      | document    |
      | script      |

  Scenario: Filter by URL pattern
    Given a page has made requests to "https://api.example.com/data" and "https://cdn.example.com/image.png"
    When I run "chrome-cli network list --url api.example.com"
    Then only requests whose URL contains "api.example.com" are returned

  Scenario: Filter by exact HTTP status code
    Given a page has responses with status codes 200, 301, and 404
    When I run "chrome-cli network list --status 404"
    Then only requests with status 404 are returned

  Scenario: Filter by wildcard HTTP status code
    Given a page has responses with status codes 200, 301, 404, and 500
    When I run "chrome-cli network list --status 4xx"
    Then only requests with status codes 400-499 are returned

  Scenario: Filter by HTTP method
    Given a page has made GET and POST requests
    When I run "chrome-cli network list --method POST"
    Then only POST requests are returned

  # --- Pagination ---

  Scenario: Paginate network request results
    Given a page has made more than 50 network requests
    When I run "chrome-cli network list --limit 20 --page 1"
    Then 20 requests are returned starting from offset 20

  # --- Navigation Preservation ---

  Scenario: Include preserved requests from previous navigations
    Given a page has navigated away (clearing current requests)
    When I run "chrome-cli network list --include-preserved"
    Then requests from both before and after the navigation are included

  # --- Detail: Get ---

  Scenario: Get detailed network request info
    Given a page has a completed network request
    When I run "chrome-cli network get <REQ_ID>"
    Then the output is a JSON object with "request", "response", and "timing" sections
    And "request" contains "method", "url", and "headers"
    And "response" contains "status", "status_text", "headers", and "body"
    And "timing" contains "dns_ms", "connect_ms", "tls_ms", "ttfb_ms", and "download_ms"
    And the exit code should be 0

  Scenario: Get request with redirect chain
    Given a page has a network request that was redirected
    When I run "chrome-cli network get <REQ_ID>"
    Then the output includes a "redirect_chain" array with each redirect hop

  Scenario: Save request body to file
    Given a page has a POST request with a body
    When I run "chrome-cli network get <REQ_ID> --save-request /tmp/req.txt"
    Then the request body is saved to "/tmp/req.txt"

  Scenario: Save response body to file
    Given a page has a completed request with a response body
    When I run "chrome-cli network get <REQ_ID> --save-response /tmp/resp.json"
    Then the response body is saved to "/tmp/resp.json"

  Scenario: Large body is truncated in inline output
    Given a page has a response body larger than 10000 characters
    When I run "chrome-cli network get <REQ_ID>"
    Then the inline body is truncated to 10000 characters
    And the "truncated" field is true

  Scenario: Binary response is not inlined
    Given a page has a network request for a binary resource
    When I run "chrome-cli network get <REQ_ID>"
    Then the response body is null
    And the "binary" field is true

  # --- Streaming: Follow ---

  Scenario: Stream network requests in real-time
    Given a page is open
    When I run "chrome-cli network follow --timeout 5000"
    And the page makes new network requests
    Then each completed request is printed as a JSON line
    And each line contains "method", "url", "status", "size", and "duration_ms"

  Scenario: Follow with type and method filters
    Given a page is open
    When I run "chrome-cli network follow --type xhr --method POST --timeout 5000"
    And the page makes XHR POST and GET requests
    Then only the XHR POST requests appear in the stream

  Scenario: Follow with URL filter
    Given a page is open
    When I run "chrome-cli network follow --url api/ --timeout 5000"
    And the page makes requests to various URLs
    Then only requests with URLs containing "api/" appear in the stream

  Scenario: Follow exits after timeout
    When I run "chrome-cli network follow --timeout 2000"
    Then the command exits after approximately 2 seconds
    And the exit code should be 0

  Scenario: Follow exits on Ctrl+C
    When I start "chrome-cli network follow"
    And I send SIGINT after 1 second
    Then the command exits cleanly
    And the exit code should be 0

  Scenario: Follow verbose mode includes headers
    Given a page is open
    When I run "chrome-cli network follow --verbose --timeout 5000"
    And the page makes network requests
    Then each streamed event includes "request_headers" and "response_headers"

  # --- Edge Cases ---

  Scenario: No network requests available
    Given a freshly loaded blank page with no requests
    When I run "chrome-cli network list"
    Then the output is an empty JSON array "[]"
    And the exit code should be 0

  Scenario: Request not found
    When I run "chrome-cli network get 99999"
    Then an error is returned indicating the request was not found
    And the exit code should be non-zero
