pub mod prompt_assembler;
pub mod personality_db;
pub mod personality_questions;
pub mod migration;

pub mod journals;
pub mod safety;

/// Backend module: exposes all domain modules for cards, sessions, events, distractions, AI, analytics, alarms, goals, settings, and utility.
pub mod cards;
pub mod sessions;
pub mod events;
pub mod distractions;
pub mod ai;
pub mod analytics;
pub mod alarms;
pub mod goals;
pub mod settings;
pub mod utility;
pub mod ai_provider;
pub mod dashboard;
pub mod orchestrator;
