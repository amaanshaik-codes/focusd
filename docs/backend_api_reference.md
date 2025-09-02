# Focusd Backend API Reference

This document provides an overview of all backend modules, Tauri commands, and extensibility points for maintainability and future development.

## Modules
- cards: Card CRUD and logic
- sessions: Pomodoro/session CRUD and logic
- events: Event CRUD and logic
- distractions: Distraction CRUD and logic
- ai: AI provider integration, consent, API key
- analytics: Metrics, trends, recommendations
- alarms: Alarm CRUD and logic
- goals: Goal CRUD and logic
- settings: User/app settings, audit log
- utility: Health checks, error logging, reset
- migration: Versioning and migrations

## Error Handling
- All Tauri commands return `Result<T, String>`
- Errors are logged via `utility::log_error`
- Hardware/AI/database errors have fallback logic and diagnostics

## Extensibility
- Each module exposes config structs for future fields
- Plugin trait pattern can be used for backend extensions
- New modules/plugins can be added by creating a Rust file and updating `backend/mod.rs`

## Versioning & Migrations
- All persistent data uses versioned schemas
- Migration logic is centralized in `backend/migration.rs`

## API Documentation
- All public functions and Tauri commands are documented with Rust doc comments
- See each module for details

---
For more, see inline Rust docs in each module and the main backend_services_todo.md.
