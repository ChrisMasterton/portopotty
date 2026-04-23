# Task File Formats

These formats are the shared local schema for the ranking and execution skills.
Use these field names and status values consistently.

## Allowed local statuses
- `queued`
- `in_progress`
- `blocked`
- `skipped_due_to_blocker`
- `in_review`

## Source values
- `linear`
- `github-issues`
- `glowing-gdd`

## Project archetype values
- `web-frontend`
- `fullstack-web`
- `backend-service`
- `unity-game`
- `library-package`
- `desktop-tool`
- `unknown`

## Confidence values
- `Confirmed`
- `Inferred`
- `Needs verification`

## .agents/repo-fingerprint.md

Use this structure and preserve the generated markers.

```md
# Repo Fingerprint

<!-- BEGIN GENERATED FINGERPRINT -->

## Identity
- Repo type: `Not yet generated`
- Project archetype: `unknown`
- Primary stack: `Not yet generated`
- Monorepo: `Not determined`
- Preferred task source: `Not determined`
- Last refreshed: `YYYY-MM-DD HH:MM`

## Capabilities
- Browser E2E available: `true|false|unknown`
- Playwright present: `true|false|unknown`
- Unity project: `true|false|unknown`
- Unity edit-mode tests present: `true|false|unknown`
- Unity play-mode tests present: `true|false|unknown`
- API/backend tests present: `true|false|unknown`
- Frontend unit tests present: `true|false|unknown`
- Source writeback confidence: `low|medium|high`

## Strategy defaults
- Bug repro preference: `playwright|unit-or-integration|unity-tests|manual-first|mixed|unknown`
- Verification preference: `browser-flow|unit-or-integration|unity-tests|mixed|unknown`
- Review surface: `browser|api|unity|mixed|unknown`

## Evidence
- `Short bullet about key files or config that drove the fingerprint`
- `Another short bullet`

## Uncertainty
- `Anything that still needs human verification`

<!-- END GENERATED FINGERPRINT -->

## Manual notes
- Optional hand-written notes.
```

## .agents/repo-conventions.md

Use this structure and preserve the generated markers.

```md
# Repo Conventions

<!-- BEGIN GENERATED SECTION -->

## Overview
- Repo type: `...`
- Project archetype: `...`
- Primary task source: `...`
- Monorepo: `...`
- Primary stack: `...`
- Browser E2E available: `...`
- Unity project: `...`
- Last refreshed by agent: `...`

## App or game startup
- Frontend: `...` — Confirmed/Inferred/Needs verification
- Backend: `...` — Confirmed/Inferred/Needs verification
- Worker: `...` — Confirmed/Inferred/Needs verification
- Unity/editor run notes: `...` — Confirmed/Inferred/Needs verification
- Startup order: `...` — Confirmed/Inferred/Needs verification

## Local URLs and ports
- Frontend URL: `...` — Confirmed/Inferred/Needs verification
- Backend URL: `...` — Confirmed/Inferred/Needs verification
- Other service: `...` — Confirmed/Inferred/Needs verification

## Tests
- Unit tests: `...` — Confirmed/Inferred/Needs verification
- Integration tests: `...` — Confirmed/Inferred/Needs verification
- E2E / Playwright: `...` — Confirmed/Inferred/Needs verification
- Unity edit-mode tests: `...` — Confirmed/Inferred/Needs verification
- Unity play-mode tests: `...` — Confirmed/Inferred/Needs verification
- Single test file: `...` — Confirmed/Inferred/Needs verification
- Test name filter: `...` — Confirmed/Inferred/Needs verification

## Validation
- Build: `...` — Confirmed/Inferred/Needs verification
- Lint: `...` — Confirmed/Inferred/Needs verification
- Typecheck: `...` — Confirmed/Inferred/Needs verification
- Format check: `...` — Confirmed/Inferred/Needs verification

## Required services / setup
- Database: `...`
- Cache/message bus: `...`
- Seed data required: `...`
- Env files expected: `...`
- Auth/setup notes: `...`

## QA / review handoff notes
- Preferred reviewer path: `...`
- Recommended validation surface: `...`
- Review-packet quirks: `...`

## Repo-specific debugging notes
- `...`

## Conflicts or uncertainty
- `...`

<!-- END GENERATED SECTION -->

## Manual notes
- Optional hand-written notes.
```

## .agents/task-work-queue.md

Use this structure and preserve the generated markers.

```md
# Task Work Queue

- Source: `linear`
- Assignee aliases: `Chris`, `Chris-HS`
- Project archetype: `fullstack-web`
- Last refreshed: `YYYY-MM-DD HH:MM`
- Notes: `optional short note about source quality or ambiguity`

<!-- BEGIN QUEUE ITEMS -->

### 1. LIN-123 - Fix login redirect
- Source state: `Backlog`
- Source priority: `High`
- Explicit dependencies: `none`
- Inferred dependencies: `LIN-120 (shared auth helper must be fixed first)`
- Ranking reason: `User-facing bug in login flow; blocks other auth tickets`
- Local execution status: `queued`

<!-- END QUEUE ITEMS -->

## Manual notes
- Optional hand-written notes.
```

## .agents/task-blocked.md

Use this structure and preserve the generated markers.

```md
# Task Blocked List

<!-- BEGIN BLOCKED ITEMS -->

### LIN-145 - Clarify workout completion rule
- Source: `linear`
- Why blocked: `Acceptance criteria unclear`
- Dependency causing block: `none`
- Questions:
  - `Should partial completion count as done?`
- What I tried:
  - `Read ticket`
- What would unblock it:
  - `Answer the questions above`

<!-- END BLOCKED ITEMS -->

## Manual notes
- Optional hand-written notes.
```

## .agents/current-task-plan.md

Use this structure and preserve the generated markers.

```md
# Current Task Plan

<!-- BEGIN CURRENT TASK -->

- Source: `linear`
- Task ID: `LIN-123`
- Title: `Fix login redirect`
- Project archetype: `fullstack-web`
- Source state: `In Progress`
- Local status: `in_progress`
- Work classification: `bug`

## Goal
Fix the redirect after login so users land on the dashboard instead of returning to the login page.

## Success criteria
- Successful login lands on dashboard
- Existing auth flow still works
- Relevant tests pass
- Review packet is generated for QA

## Relevant code areas
- `src/auth/...`
- `src/routes/...`
- `tests/e2e/login.spec.ts`

## Risks / unknowns
- Session cookie timing
- Route guard interaction

## Test strategy
- Reproduce with the cheapest credible path for this repo
- Add/update focused regression coverage
- Run relevant targeted tests

<!-- END CURRENT TASK -->

## Manual notes
- Optional hand-written notes.
```

## .agents/review-packets/<TASK-ID>.md

Use this structure and preserve the generated markers.

```md
# Review Packet - LIN-123

<!-- BEGIN REVIEW PACKET -->

- Source: `linear`
- Task ID: `LIN-123`
- Title: `Fix login redirect`
- Project archetype: `fullstack-web`
- Local status: `in_review`
- Source state after work: `In Review`
- Commit: `abc1234`
- Branch: `current-branch-name`

## Summary
Users now land on the dashboard after successful login instead of bouncing back to the login screen.

## What changed
- Updated post-login redirect logic
- Tightened auth guard handling
- Added regression coverage

## How QA should test
1. Start from the normal local or staging environment for this repo.
2. Log in with a valid test account.
3. Confirm the app lands on the dashboard.

## Expected results
- Successful login redirects to dashboard
- No login loop

## Verification performed by implementation
- `Ran targeted tests`
- `Verified the bug repro path`

## Risks / follow-up
- `List any remaining caveats or none`

<!-- END REVIEW PACKET -->

## Manual notes
- Optional hand-written notes.
```
