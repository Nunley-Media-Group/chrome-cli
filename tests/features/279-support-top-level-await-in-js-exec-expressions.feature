# File: tests/features/279-support-top-level-await-in-js-exec-expressions.feature
#
# Generated from: specs/bug-support-top-level-await-in-js-exec-expressions/requirements.md
# Issue: #279
# Type: Defect regression

@regression
Feature: js exec supports top-level await expressions
  The `agentchrome js exec` expression path previously wrapped user code in a
  plain block and sent it to `Runtime.evaluate` without enabling evaluation
  semantics that permit direct top-level await syntax. This caused
  `await Promise.resolve("done")` to fail during parsing even though returned
  Promise values were awaited correctly.

  Background:
    Given Chrome is connected and a page is loaded

  @regression
  Scenario: AC1 - direct top-level await evaluates successfully
    When I run "agentchrome js exec --code await/**/Promise.resolve('done')"
    Then the command exits with code 0
    And stdout is valid JSON containing "\"result\":\"done\""
    And stdout is valid JSON containing "\"type\":\"string\""

  @regression
  Scenario: AC2 - returned Promise values are still awaited by default
    When I run "agentchrome js exec --code new/**/Promise(r=>setTimeout(()=>r('done'),100))"
    Then the command exits with code 0
    And stdout is valid JSON containing "\"result\":\"done\""
    And stdout is valid JSON containing "\"type\":\"string\""

  @regression
  Scenario: AC3 - let declarations remain isolated across consecutive invocations
    When I run "agentchrome js exec --code let/**/topLevelAwaitRegressionValue=1;topLevelAwaitRegressionValue"
    Then the command exits with code 0
    And stdout is valid JSON containing "\"result\":1"
    When I run "agentchrome js exec --code let/**/topLevelAwaitRegressionValue=2;topLevelAwaitRegressionValue"
    Then the command exits with code 0
    And stdout is valid JSON containing "\"result\":2"

  @regression
  Scenario: AC3 - const declarations remain isolated across consecutive invocations
    When I run "agentchrome js exec --code const/**/topLevelAwaitRegressionConst=10;topLevelAwaitRegressionConst"
    Then the command exits with code 0
    And stdout is valid JSON containing "\"result\":10"
    When I run "agentchrome js exec --code const/**/topLevelAwaitRegressionConst=20;topLevelAwaitRegressionConst"
    Then the command exits with code 0
    And stdout is valid JSON containing "\"result\":20"
