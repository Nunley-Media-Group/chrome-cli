Feature: Configuration file support
  As a developer or automation engineer
  I want to set default values in a configuration file
  So that I don't repeat common flags on every invocation

  # --- Config File Loading ---

  Scenario: Load config from explicit --config path
    Given a config file at "explicit.toml" with content:
      """
      [connection]
      port = 9333
      """
    When I run chrome-cli with "--config {config_path} config show"
    Then the exit code should be 0
    And the JSON output field "connection.port" should be 9333

  Scenario: Load config from CHROME_CLI_CONFIG environment variable
    Given a config file at "env-config.toml" with content:
      """
      [connection]
      port = 9444
      """
    When I run chrome-cli with env CHROME_CLI_CONFIG="{config_path}" and args "config show"
    Then the exit code should be 0
    And the JSON output field "connection.port" should be 9444

  Scenario: Load config from project-local file
    Given a project-local config file ".chrome-cli.toml" with content:
      """
      [connection]
      port = 9555
      """
    When I run chrome-cli with "config show"
    Then the exit code should be 0
    And the JSON output field "connection.port" should be 9555

  Scenario: Load config from XDG standard path
    Given an XDG config file "chrome-cli/config.toml" with content:
      """
      [connection]
      host = "10.0.0.1"
      """
    When I run chrome-cli with "config show"
    Then the exit code should be 0
    And the JSON output field "connection.host" should be "10.0.0.1"

  Scenario: Load config from home directory fallback
    Given a home directory config file ".chrome-cli.toml" with content:
      """
      [output]
      format = "pretty"
      """
    When I run chrome-cli with "config show"
    Then the exit code should be 0
    And the JSON output field "output.format" should be "pretty"

  Scenario: Config file priority - project-local wins over home directory
    Given a project-local config file ".chrome-cli.toml" with content:
      """
      [connection]
      port = 1111
      """
    And a home directory config file ".chrome-cli.toml" with content:
      """
      [connection]
      port = 2222
      """
    When I run chrome-cli with "config show"
    Then the exit code should be 0
    And the JSON output field "connection.port" should be 1111

  Scenario: CLI flags override config file values
    Given a config file at "override.toml" with content:
      """
      [connection]
      port = 9333
      """
    When I run chrome-cli with "--config {config_path} --port 9444 config show"
    Then the exit code should be 0
    And the JSON output field "connection.port" should be 9444

  Scenario: Environment variables override config file values
    Given a config file at "env-override.toml" with content:
      """
      [connection]
      port = 9333
      """
    When I run chrome-cli with env CHROME_CLI_PORT="9555" and args "--config {config_path} config show"
    Then the exit code should be 0
    And the JSON output field "connection.port" should be 9555

  # --- Config Sections ---

  Scenario: Connection defaults from config file
    Given a config file at "conn.toml" with content:
      """
      [connection]
      host = "192.168.1.100"
      port = 9333
      timeout_ms = 60000
      """
    When I run chrome-cli with "--config {config_path} config show"
    Then the exit code should be 0
    And the JSON output field "connection.host" should be "192.168.1.100"
    And the JSON output field "connection.port" should be 9333
    And the JSON output field "connection.timeout_ms" should be 60000

  Scenario: Chrome launch defaults from config file
    Given a config file at "launch.toml" with content:
      """
      [launch]
      executable = "/usr/bin/chromium"
      channel = "beta"
      headless = true
      extra_args = ["--disable-gpu", "--no-sandbox"]
      """
    When I run chrome-cli with "--config {config_path} config show"
    Then the exit code should be 0
    And the JSON output field "launch.executable" should be "/usr/bin/chromium"
    And the JSON output field "launch.channel" should be "beta"
    And the JSON output field "launch.headless" should be true

  Scenario: Output format defaults from config file
    Given a config file at "output.toml" with content:
      """
      [output]
      format = "pretty"
      """
    When I run chrome-cli with "--config {config_path} config show"
    Then the exit code should be 0
    And the JSON output field "output.format" should be "pretty"

  Scenario: Tab behavior defaults from config file
    Given a config file at "tabs.toml" with content:
      """
      [tabs]
      auto_activate = false
      filter_internal = true
      """
    When I run chrome-cli with "--config {config_path} config show"
    Then the exit code should be 0
    And the JSON output field "tabs.auto_activate" should be false
    And the JSON output field "tabs.filter_internal" should be true

  # --- Config Subcommands ---

  Scenario: config show displays resolved configuration
    Given a config file at "show.toml" with content:
      """
      [connection]
      port = 9333
      """
    When I run chrome-cli with "--config {config_path} config show"
    Then the exit code should be 0
    And the JSON output should contain key "config_path"
    And the JSON output should contain key "connection"
    And the JSON output should contain key "launch"
    And the JSON output should contain key "output"
    And the JSON output should contain key "tabs"

  Scenario: config show with no config file uses defaults
    When I run chrome-cli with "config show"
    Then the exit code should be 0
    And the JSON output field "connection.port" should be 9222
    And the JSON output field "connection.host" should be "127.0.0.1"

  Scenario: config init creates a default config file
    Given no config file exists at the init target path
    When I run chrome-cli with "config init --path {init_path}"
    Then the exit code should be 0
    And the JSON output should contain key "created"
    And the init target file should exist

  Scenario: config init refuses to overwrite existing file
    Given a config file already exists at the init target path
    When I run chrome-cli with "config init --path {init_path}"
    Then the exit code should be non-zero
    And stderr should contain "already exists"

  Scenario: config path shows active config file
    Given a config file at "path-test.toml" with content:
      """
      [connection]
      port = 9222
      """
    When I run chrome-cli with "--config {config_path} config path"
    Then the exit code should be 0
    And the JSON output field "config_path" should contain "path-test.toml"

  Scenario: config path when no config file exists
    When I run chrome-cli with "config path"
    Then the exit code should be 0
    And the JSON output field "config_path" should be null

  # --- Error Handling ---

  Scenario: Invalid TOML config file - graceful degradation
    Given a config file at "invalid.toml" with content:
      """
      this is not valid toml [[[
      """
    When I run chrome-cli with "--config {config_path} config show"
    Then the exit code should be 0
    And stderr should contain "warning"
    And the JSON output field "connection.port" should be 9222

  Scenario: Config file with unknown keys - warn and continue
    Given a config file at "unknown.toml" with content:
      """
      [connection]
      port = 9333
      unknown_key = "hello"
      """
    When I run chrome-cli with "--config {config_path} config show"
    Then the exit code should be 0
    And stderr should contain "unknown"
    And the JSON output field "connection.port" should be 9333
