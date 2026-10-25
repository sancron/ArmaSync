// SPDX-License-Identifier: AGPL-3.0-only
//! ArmaSync desktop GUI prototype (MIG-012).
//!
//! * [`model`]: all state and behaviour without drawing (unit-tested without a display).
//! * [`ui`]: the egui/eframe drawing code with AccessKit semantics.
//! * [`contrast`]: WCAG contrast helpers used to check the colours.
//!
//! The prototype runs the shared `armasync-application` sync use case against in-memory data with a small
//! artificial delay. It does not talk to real servers or touch the file system.

#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

pub mod contrast;
pub mod model;
pub mod sync_screen;
pub mod sync_ui;
pub mod ui;
pub mod upload_screen;
pub mod upload_ui;
