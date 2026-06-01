//! Event-loop scaffolding. The Cut-1 TUI is small enough that the
//! `App::on_key` method handles everything inline; this module exists
//! as a hook point for later cuts that want a richer event vocabulary
//! (channel-driven async events, ticks, paste handling, etc.).

#![allow(dead_code)]
