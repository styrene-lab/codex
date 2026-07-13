+++
id = "22222222-2222-4222-8222-222222222204"
kind = "task"

[data]
title = "Ship the QBF release"
board = "564f64c1-2e9c-4c7d-bc6d-84ecf0f5c6c1"
column = "Doing"
priority = 0
status = "in_progress"
position = 1
tags = ["sentry", "release"]
external_refs = ["cron:0 9 * * 1"]

[data.execution]
model = "openai-codex:gpt-5.5"
skill = "release"
max_turns = 8
timeout_secs = 900
token_budget = 12000
cwd = "."
+++

Execute the final bounded release check after visual approval.
