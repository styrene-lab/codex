# Host Actions Permission Review — Delta Spec

## ADDED Requirements

### Requirement: ACP permissions require operator review

Flynt MUST NOT auto-allow ACP permission requests. Permission requests MUST surface as reviewable UI state and resolve only after an operator decision.

#### Scenario: Permission request is shown for review
Given an ACP session sends a permission request with a tool call
When Flynt receives the request
Then Flynt displays a review card containing the tool title and raw input summary
And Flynt waits for an operator decision before responding to ACP

#### Scenario: Rejected permission does not execute host action
Given a pending permission review for a terminal creation request
When the operator rejects the request
Then Flynt returns a reject or cancelled permission response to ACP
And no terminal session is created

### Requirement: Approved terminal.create requests launch through TerminalManager

Flynt MUST execute recognized `terminal.create@1` requests through the reusable `TerminalManager` after approval.

#### Scenario: Approved terminal request creates local session
Given a pending permission review with raw input containing `terminal.create@1` and valid terminal params
When the operator approves the request
Then Flynt creates a terminal session through `TerminalManager`
And the review card shows the created terminal id
And Flynt returns an allow permission response to ACP

#### Scenario: Failed terminal creation prevents ACP approval
Given a pending permission review with raw input containing `terminal.create@1` and invalid terminal params
When the operator approves the request
Then Flynt marks the review card failed
And Flynt returns a reject or cancelled permission response to ACP
