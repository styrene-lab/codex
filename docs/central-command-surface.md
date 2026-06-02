---
id: central-command-surface
title: "Central command surface and execution registry"
status: exploring
tags: [commands, ui, command-palette, architecture, diagnostics]
open_questions: []
dependencies: []
related: []
---

# Central command surface and execution registry

## Overview

Centralize Flynt's command/action surface behind a typed command registry. The command palette should discover and invoke commands from this registry, while sidebar context menus, toolbar buttons, settings buttons, and future UI affordances should call the same command IDs/handlers. This prevents duplicated execution logic and makes command requirements, permissions, background execution, diagnostics, and tests explicit.
