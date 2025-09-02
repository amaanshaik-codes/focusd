

# Focusd Backend Services - Comprehensive & Detailed Development Todo List

## Logical Flaws / Gaps & Robustness
- [x] Mechanism for handling missed, duplicate, or out-of-window core card taps (e.g., missed sleep, double wake)
- [x] Enforcement and user configuration of time windows for core card (wake/sleep) taps, with validation and warnings
- [x] State machine for locking/unlocking the app based on core card state (locked, unlocked, pending, error)
- [x] Fallback and auto-reconnect if ESP32 disconnects or RFID fails mid-session (backend auto-reconnect command, retries/logging, frontend polling recommended)
- [x] Session interruption/resume logic if app is closed/reopened or system sleeps (session state persisted, resume logic via backend command)
- [x] Data migration/upgrade path if schema changes (schema_version table, migration runner, helpers implemented)
- [x] Handling for multiple users or profiles (future-proofing, not MVP) (user table, user_id columns, query scoping in backend)
- [x] Robust error handling and fallback for hardware, database, and AI failures (standardized error returns/logging, global error handler for backend commands)

## Workspace & State Management
- [x] Directory picker (vault/workspace selection, creation, recent workspaces)
- [x] Onboarding UI and workspace selection (frontend, branding, background assets)
- [x] Daily SQLite database file management (auto-create/open per day, backend command returns today's DB path)
- [x] State file for onboarding/completion, user preferences, AI opt-in/out, and workspace config (JSON file, backend read/write commands)
- [x] User profile CRUD and onboarding state management (name, occupation, productivity, AI opt-in, etc.) (backend CRUD commands)
- [x] Productivity personality test (10 questions) and personality type assignment (answers/type stored in user profile)
- [x] Store and use user-defined core card time windows (in user profile/state, backend commands)
- [x] Store and use AI provider selection and API key (Gemini, ChatGPT, etc.) (in user profile/state, backend commands)

## RFID & Hardware Integration
- [x] ESP32 connection detection and status reporting
- [x] Serial port communication (read UIDs, check RFID health, auto-reconnect)
- [x] System card setup (core, event, distraction) and enforcement of uniqueness (one of each)
- [x] Session (pomodoro) card setup and management (multiple allowed)
- [x] Fallback and error reporting for hardware failures (RFID not working, ESP32 disconnect)
- [x] Health checks and diagnostics for hardware

## Card, Session (Pomodoro), Event & Distraction Logic
- [x] Card CRUD (add, update, delete, list, assign type/profile/config, color) (frontend, backend Tauri commands)
- [x] Session CRUD (frontend, backend Tauri commands)
- [x] Event CRUD (frontend, backend Tauri commands)
- [x] Distraction CRUD (frontend, backend Tauri commands)
- [x] Core card logic (wake/sleep tap, enforce user-defined time windows, lock/unlock app, state machine, error states)
- [x] Session/pomodoro card logic (start/end, duration, optional description, interruption/resume, link to goals; backend logic implemented)
- [x] Event card logic (log event, prompt for description, one per workspace; backend logic implemented)
- [x] Distraction card logic (pause/resume session, log cause, measure distraction time, one per workspace; backend logic implemented)
- [x] Enforce only one core/event/distraction card, allow multiple session cards (backend enforced on card creation/update)
- [x] Link goals to session cards, prioritize by deadline, highlight urgent goals (goal linking/prioritization logic in backend)
- [x] Alarm logic: allow user to set alarms (e.g., wake time), log punctuality, lateness, missed alarms (backend alarm logic implemented)

## Logging & Data Storage
- [x] Unified logs table (all actions: tap, event, distraction, alarm, goal, etc., with context fields; backend schema/logic implemented)
- [x] Sessions (pomodoros) table (start/end, duration, description, focus score, distraction time, linked goal; backend schema/logic implemented)
- [x] Events table (timestamp, description, linked session if any; backend schema/logic implemented)
- [x] Distractions table (start/end, session link, description; backend schema/logic implemented)
- [x] Card table (UID, type, name, config, color, etc.; backend schema/logic implemented)
- [x] Goals table (description, deadline, linked_card_uid, status, priority, created_at, completed_at; backend schema/logic implemented)
- [x] Alarms table (time, type, linked_card_uid, status, triggered_at, resolved_at; backend schema/logic implemented)
- [x] Punctuality log (alarm_id, actual_time, status; backend schema/logic implemented)
- [x] Daily database rotation and archival logic (auto-create/open, backup, restore; backend logic implemented)
- [x] Data export/import and backup/restore (with optional encryption; backend logic implemented)

## AI & Automation
- [ ] AI prompt management (custom prompt per user, based on onboarding answers and personality type)
- [ ] Gemini and ChatGPT API integration (user selects provider, enters API key)
- [ ] Automated journaling and lockscreen note generation (markdown beautification, rich text option)
- [ ] AI opt-in/out logic and privacy controls

## User Settings & Preferences
- [x] Theme (light/dark mode): per-user, validated, persisted in user profile; robust backend commands; audit log for changes; versioned for migration.
- [x] Goal setting, reminders, and orchestrator logic: recurrence, snooze, cross-device sync ready; robust CRUD; audit log for changes; validated input.
- [x] Minimum session time, focus/burnout thresholds: per-user, validated, sensible defaults, audit log for changes, versioned for migration.
- [x] Notification system: supports scheduling, repeat, user-defined channels, per-user preferences, robust error handling, audit log for changes.
- [x] Multi-language/localization support: all backend-generated messages localizable, per-user language, robust command, audit log for changes.
- [x] Markdown/rich text rendering options: toggleable per note/journal, per-user default, robust command, audit log for changes.

## Analytics & Visualization Data
- [x] Focus score, burnout meter, streaks, session stats, punctuality (backend computes and exposes all metrics for advanced charts/plots)
- [x] Data aggregation for dashboard (pie, line, bar charts, heatmaps, calendar) (backend provides aggregated data for all chart types)
- [x] Dashboard data aggregation: streaks, focus score, burnout, punctuality, goal progress, session analytics (backend API returns all dashboard metrics for visualization)

## Security & Privacy
- [x] Local encryption (AES-256 or OS keyring for sensitive data; backend commands implemented)
- [x] Data export/import with encryption (Tauri commands for encrypted backup/restore, password-protected)
- [x] User consent for AI and data sharing (consent flags in user profile/settings, enforced in backend)
- [x] Secure storage of API keys and sensitive preferences (encrypted at rest, never exposed in logs/exports, secure set/get/delete commands)

## Utility & Maintenance
- [x] Database migration/upgrade logic (versioning, migrations)
- [x] Data reset/clear for development and troubleshooting
- [x] Health checks (RFID, ESP32, database, AI connectivity)
- [x] Error logging and diagnostics for hardware, database, and AI failures
- [x] Modularize backend: separate modules for cards, sessions, events, distractions, AI, analytics, and settings
- [x] Plan for extensibility: config fields, plugin system, or API for future features

## API/Command Exposure
- [x] Expose all above as Tauri commands for frontend use (in progress)
- [x] Global error boundary and notification/toast system (frontend)
- [ ] Consistent error handling and status reporting (backend)

## Suggestions & Architecture
- [x] Add a state machine for core card logic (locked/unlocked, wake/sleep, error, pending)
- [x] Modularize frontend: separate components for cards, sessions, events, distractions, onboarding, error boundary, toast
- [x] Modularize backend: separate modules for cards, sessions, events, distractions, AI, analytics, alarms, goals, and settings
- [x] Implement robust error handling and fallback for hardware, database, and AI
- [x] Plan for extensibility: config fields, plugin system, or API for future features
- [x] Use versioning and migrations for all persistent data
- [x] Document all APIs and backend logic for maintainability (see docs/backend_api_reference.md)

---

Update this file as features are implemented, issues are found, or requirements change. This is your single source of truth for backend progress.
