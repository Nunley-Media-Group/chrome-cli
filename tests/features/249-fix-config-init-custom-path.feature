Feature: config init honors custom destination path (issue #249)
  As a developer provisioning agentchrome for a CI environment
  I want `config init --config /path/to/file.toml` to write the template to the specified path
  So that I can initialize config files in custom locations without editing them manually

  @regression
  Scenario: --config is honored as the init destination
    Given a writable target path "custom.toml" that does not exist
    When I run agentchrome config init with --config pointing at "custom.toml"
    Then the "custom.toml" target file exists
    And the JSON output's "created" field equals the "custom.toml" target path
    And the process exits with code 0

  @regression
  Scenario: default path is preserved when no path flag is supplied
    Given no config file exists at the XDG default path
    When I run agentchrome config init with no path flags
    Then the file at the XDG default path exists
    And the JSON output's "created" field equals the XDG default path
    And the process exits with code 0

  @regression
  Scenario: unwritable path errors clearly without falling back to the default
    Given a regular file blocks the parent directory of the target path
    When I run agentchrome config init with --config pointing at the blocked target path
    Then no file is created at the XDG default path
    And the process exits with code 1
    And stderr contains the blocked target path

  @regression
  Scenario: --path wins when both flags are supplied with different values
    Given a writable target path "from-path.toml" that does not exist
    And a writable target path "from-config.toml" that does not exist
    When I run agentchrome config init with --path on "from-path.toml" and --config on "from-config.toml"
    Then the "from-path.toml" target file exists
    And the "from-config.toml" target file does not exist
    And stderr notes that --path overrode --config
    And the process exits with code 0
