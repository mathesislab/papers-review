//! The review pane: a web view of its own in a project's window, over the
//! viewer, at the bottom or on the right; above the page, and so above the
//! PDF view that lies in the page. The page tells where the viewer is
//! (`pane_area`); the pane's side, size and whether it is shown are kept per
//! project (state.json). One window of a project holds its pane at a time.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, EventTarget, LogicalPosition, LogicalSize, Manager, Rect};

use crate::state::{PaneLayout, Side};
use crate::{AppState, windows};

/// The pane of a window, by the window's label.
pub fn label(window: &str) -> String {
    format!("{window}-pane")
}

/// The space between the pane and the edges of the viewer (points).
const MARGIN: f64 = 8.0;
/// The smallest pane (points), and the least of the viewer it leaves.
const SMALLEST: f64 = 120.0;
const LEFT: f64 = 60.0;

/// The viewer of a page, in points from the top left of the window's content.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct Area {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

/// What the app knows of the panes beyond what it keeps between sessions.
#[derive(Debug, Default)]
pub struct Panes {
    /// the viewer of each window, as its page told
    pub areas: HashMap<String, Area>,
    /// the windows whose pane has the keys
    pub focused: HashSet<String>,
    /// the window of each project that holds its pane
    pub holder: HashMap<PathBuf, String>,
    /// where the pane of each window was placed last, for its page as it loads
    pub placed: HashMap<String, Placed>,
}

/// What the pane's page learns of where it is, to draw its edge.
#[derive(Debug, Clone, Serialize)]
pub struct Placed {
    side: Side,
    size: f64,
}

fn layout_of(app: &AppHandle, window: &str) -> Option<(PathBuf, PaneLayout)> {
    let root = windows::root_of(app, window)?;
    let layout = app
        .state::<AppState>()
        .saved
        .lock()
        .unwrap()
        .panes
        .get(&root)
        .cloned()
        .unwrap_or_default();
    Some((root, layout))
}

fn keep(app: &AppHandle, root: PathBuf, layout: PaneLayout) {
    let state = app.state::<AppState>();
    state.saved.lock().unwrap().panes.insert(root, layout);
    state.save();
}

/// The pane's bounds in the viewer, its size kept within it.
fn bounds(area: Area, layout: &PaneLayout) -> (f64, f64, f64, f64) {
    match layout.side {
        Side::Bottom => {
            let room = (area.height - 2.0 * MARGIN - LEFT).max(SMALLEST);
            let height = layout.bottom.clamp(SMALLEST, room);
            let y = area.y + area.height - MARGIN - height;
            (area.x + MARGIN, y, (area.width - 2.0 * MARGIN).max(0.0), height)
        }
        Side::Right => {
            let room = (area.width - 2.0 * MARGIN - LEFT).max(SMALLEST);
            let width = layout.right.clamp(SMALLEST, room);
            let x = area.x + area.width - MARGIN - width;
            (x, area.y + MARGIN, width, (area.height - 2.0 * MARGIN).max(0.0))
        }
    }
}

/// The pane of `window` where its layout and the viewer say: shown in the
/// window that holds the project's pane, hidden elsewhere.
pub fn place(app: &AppHandle, window: &str) {
    let Some((root, layout)) = layout_of(app, window) else {
        return;
    };
    let Some(pane) = app.get_webview(&label(window)) else {
        return;
    };
    let state = app.state::<AppState>();
    let (area, held) = {
        let mut panes = state.panes.lock().unwrap();
        // a pane to show and no window holding it yet: this one
        if layout.shown && !panes.holder.contains_key(&root) {
            panes.holder.insert(root.clone(), window.to_string());
        }
        (panes.areas.get(window).copied(), panes.holder.get(&root).is_some_and(|w| w == window))
    };
    let Some(area) = area.filter(|_| layout.shown && held) else {
        let _ = pane.hide();
        return;
    };
    let (x, y, width, height) = bounds(area, &layout);
    let _ = pane.set_bounds(Rect {
        position: LogicalPosition::new(x, y).into(),
        size: LogicalSize::new(width, height).into(),
    });
    let _ = pane.show();
    let size = match layout.side {
        Side::Bottom => height,
        Side::Right => width,
    };
    let placed = Placed { side: layout.side, size };
    state.panes.lock().unwrap().placed.insert(window.to_string(), placed.clone());
    let _ = app.emit_to(EventTarget::webview(label(window)), "pane-placed", placed);
}

/// Where the pane of `window` was placed last: its page loads after the first placing.
pub fn placement(app: &AppHandle, window: &str) -> Option<Placed> {
    app.state::<AppState>().panes.lock().unwrap().placed.get(window).cloned()
}

/// The windows of the project of `window`, placed again.
fn place_project(app: &AppHandle, window: &str) {
    let Some(root) = windows::root_of(app, window) else {
        return;
    };
    for other in windows::of_project(app, &root) {
        place(app, &other);
    }
}

/// The viewer of a window's page moved or changed size.
pub fn set_area(app: &AppHandle, window: &str, area: Area) {
    let changed = {
        let state = app.state::<AppState>();
        let mut panes = state.panes.lock().unwrap();
        panes.areas.insert(window.to_string(), area) != Some(area)
    };
    if changed {
        place(app, window);
    }
}

/// ⌘J: the pane shown in this window with the keys, or hidden when it has them.
pub fn toggle(app: &AppHandle, window: &str) {
    let Some((root, layout)) = layout_of(app, window) else {
        return;
    };
    let state = app.state::<AppState>();
    let focused = state.panes.lock().unwrap().focused.contains(window);
    let held = state.panes.lock().unwrap().holder.get(&root).is_some_and(|w| w == window);
    if layout.shown && held && focused {
        hide(app, window);
    } else {
        show(app, window);
    }
}

/// The pane shown in this window, with the keys (hidden in the project's other windows).
pub fn show(app: &AppHandle, window: &str) {
    let Some((root, mut layout)) = layout_of(app, window) else {
        return;
    };
    let state = app.state::<AppState>();
    layout.shown = true;
    state.panes.lock().unwrap().holder.insert(root.clone(), window.to_string());
    keep(app, root, layout);
    place_project(app, window);
    if let Some(pane) = app.get_webview(&label(window)) {
        let _ = pane.set_focus();
    }
}

/// The pane hidden; the keys back to the page (and its PDF).
pub fn hide(app: &AppHandle, window: &str) {
    let Some((root, mut layout)) = layout_of(app, window) else {
        return;
    };
    layout.shown = false;
    keep(app, root, layout);
    app.state::<AppState>().panes.lock().unwrap().focused.remove(window);
    place_project(app, window);
    if let Some(page) = app.get_webview(&windows::page(window)) {
        let _ = page.set_focus();
    }
    let _ = app.emit_to(EventTarget::webview(windows::page(window)), "pane-hidden", ());
}

/// ⌘⇧J: the pane to the other side.
pub fn switch_side(app: &AppHandle, window: &str) {
    let Some((root, mut layout)) = layout_of(app, window) else {
        return;
    };
    layout.side = match layout.side {
        Side::Bottom => Side::Right,
        Side::Right => Side::Bottom,
    };
    keep(app, root, layout);
    place(app, window);
}

/// The pane's edge dragged: its size (points); `done` when let go, kept then.
pub fn resize(app: &AppHandle, window: &str, size: f64, done: bool) {
    let Some((root, mut layout)) = layout_of(app, window) else {
        return;
    };
    match layout.side {
        Side::Bottom => layout.bottom = size,
        Side::Right => layout.right = size,
    }
    if done {
        keep(app, root, layout);
    } else {
        app.state::<AppState>().saved.lock().unwrap().panes.insert(root, layout);
    }
    place(app, window);
}

pub fn set_focused(app: &AppHandle, window: &str, focused: bool) {
    let state = app.state::<AppState>();
    let mut panes = state.panes.lock().unwrap();
    if focused {
        panes.focused.insert(window.to_string());
    } else {
        panes.focused.remove(window);
    }
}

/// A window gone: what was known of its pane goes.
pub fn closed(app: &AppHandle, window: &str) {
    let state = app.state::<AppState>();
    let mut panes = state.panes.lock().unwrap();
    panes.areas.remove(window);
    panes.focused.remove(window);
    panes.holder.retain(|_, w| w != window);
    panes.placed.remove(window);
}

#[cfg(test)]
mod tests {
    use super::*;

    const AREA: Area = Area {
        x: 240.0,
        y: 30.0,
        width: 960.0,
        height: 770.0,
    };

    #[test]
    fn the_pane_lies_at_the_bottom_or_the_right_of_the_viewer() {
        let mut layout = PaneLayout {
            bottom: 280.0,
            right: 400.0,
            ..PaneLayout::default()
        };
        assert_eq!(bounds(AREA, &layout), (248.0, 30.0 + 770.0 - 8.0 - 280.0, 944.0, 280.0));
        layout.side = Side::Right;
        assert_eq!(bounds(AREA, &layout), (240.0 + 960.0 - 8.0 - 400.0, 38.0, 400.0, 754.0));
    }

    #[test]
    fn its_size_stays_within_the_viewer() {
        let mut layout = PaneLayout {
            bottom: 5000.0,
            ..PaneLayout::default()
        };
        assert_eq!(bounds(AREA, &layout).3, 770.0 - 16.0 - LEFT);
        layout.bottom = 10.0;
        assert_eq!(bounds(AREA, &layout).3, SMALLEST);
    }
}
