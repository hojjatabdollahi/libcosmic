// SPDX-License-Identifier: MPL-2.0

//! A keyed wrapping flex row with drag-to-reorder animation.
//!
//! `reorderable_flex_row` lays out arbitrary item elements in wrapping rows and
//! lets the user drag an item to reorder it. While dragging, the active item
//! lifts above the layout and sibling items slide into their wrapped insertion
//! positions.
//!
//! # Example
//!
//! ```ignore
//! use libcosmic::widget::{self, reorderable_flex_row};
//!
//! let row = reorderable_flex_row(|from, to| Message::Reordered { from, to })
//!     .spacing(12.0)
//!     .push_locked("home", widget::container("Home").padding(12))
//!     .push("alpha", widget::container("Alpha").padding(12))
//!     .push("beta", widget::container("Beta").padding(12))
//!     .push("gamma", widget::container("Gamma").padding(12))
//!     .push_locked("new", widget::container("New").padding(12));
//! ```
//!
//! Locked items stay in place and never participate in reordering. Draggable
//! items use their full surface as the drag handle, but normal clicks are
//! still delivered until the pointer crosses the drag threshold.

mod widget;

pub use widget::{ReorderableFlexRow, reorderable_flex_row};
