//! The review pane: a web view of its own in a project's window, of the project the window shows, over the viewer,
//! at the bottom or on the right; above the pages (the tabs), and so above the PDF view that lies in a page. The
//! page in front tells where the viewer is (`pane_area`); the pane's side, size and whether it is shown are kept per
//! project (state.json). Collapsed, it is no more than its header bar (`HEADER`), along the bottom of the viewer or
//! down its right side.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, EventTarget, LogicalPosition, LogicalSize, Manager, Rect, WebviewBuilder, WebviewUrl, Window};

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
/// The pane collapsed: its header bar alone (points), the height of the header in
/// `routes/pane/+page.svelte`, the width of the strip on the right.
const HEADER: f64 = 32.0;

/// The viewer of a page, in points from the top left of the window's content.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct Area {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    /// the height of the page itself
    pub viewport: f64,
}

/// What the app knows of the panes beyond what it keeps between sessions.
#[derive(Debug, Default)]
pub struct Panes {
    /// the viewer of each window, as its page in front told
    pub areas: HashMap<String, Area>,
    /// the windows whose pane has the keys
    pub focused: HashSet<String>,
    /// where the pane of each window was placed last, for its page as it loads
    pub placed: HashMap<String, Placed>,
    /// the pages that show a card, which the pane would cover
    pub covered: HashSet<String>,
}

/// What the pane's page learns of where it is, to draw its edge.
#[derive(Debug, Clone, Serialize)]
pub struct Placed {
    side: Side,
    size: f64,
    collapsed: bool,
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

/// The pane's bounds in the viewer, its size kept within it; collapsed, the
/// header bar alone at the same place.
fn bounds(area: Area, layout: &PaneLayout) -> (f64, f64, f64, f64) {
    match layout.side {
        Side::Bottom => {
            let room = (area.height - 2.0 * MARGIN - LEFT).max(SMALLEST);
            let height = if layout.collapsed { HEADER } else { layout.bottom.clamp(SMALLEST, room) };
            let y = area.y + area.height - MARGIN - height;
            (area.x + MARGIN, y, (area.width - 2.0 * MARGIN).max(0.0), height)
        }
        Side::Right => {
            let room = (area.width - 2.0 * MARGIN - LEFT).max(SMALLEST);
            let width = if layout.collapsed { HEADER } else { layout.right.clamp(SMALLEST, room) };
            let x = area.x + area.width - MARGIN - width;
            (x, area.y + MARGIN, width, (area.height - 2.0 * MARGIN).max(0.0))
        }
    }
}

/// The pane of `window` where its layout and the viewer say; hidden while the page in front shows a card.
pub fn place(app: &AppHandle, window: &str) {
    let Some((_, layout)) = layout_of(app, window) else {
        return;
    };
    let Some(pane) = app.get_webview(&label(window)) else {
        return;
    };
    let state = app.state::<AppState>();
    let front = papers_tabs::front_of(app, window);
    let (area, covered) = {
        let panes = state.panes.lock().unwrap();
        (panes.areas.get(window).copied(), front.is_some_and(|f| panes.covered.contains(&f)))
    };
    let Some(area) = area.filter(|_| layout.shown && !covered) else {
        let _ = pane.hide();
        return;
    };
    let (x, y, width, height) = bounds(area, &layout);
    // wry places a child web view in the window's content view, which runs under the title bar; the page's
    // coordinates start below it
    let y = y + title_bar(app, window, area.viewport);
    let _ = pane.set_bounds(Rect {
        position: LogicalPosition::new(x, y).into(),
        size: LogicalSize::new(width, height).into(),
    });
    let _ = pane.show();
    let size = match layout.side {
        Side::Bottom => height,
        Side::Right => width,
    };
    let placed = Placed { side: layout.side, size, collapsed: layout.collapsed };
    state.panes.lock().unwrap().placed.insert(window.to_string(), placed.clone());
    let _ = app.emit_to(EventTarget::webview(label(window)), "pane-placed", placed);
}

/// The height of the title bar of `window` (points): its content, which runs under the title bar, less the
/// height of the page (`viewport`), which starts below it.
fn title_bar(app: &AppHandle, window: &str, viewport: f64) -> f64 {
    let Some(window) = app.get_window(window) else {
        return 0.0;
    };
    let scale = window.scale_factor().unwrap_or(1.0);
    window
        .inner_size()
        .map(|inner| (f64::from(inner.height) / scale - viewport).max(0.0))
        .unwrap_or(0.0)
}

/// Where the pane of `window` was placed last: its page loads after the first placing.
pub fn placement(app: &AppHandle, window: &str) -> Option<Placed> {
    app.state::<AppState>().panes.lock().unwrap().placed.get(window).cloned()
}

/// The pane made for a window as its first page comes (above it), or brought above the pages as another comes.
pub fn over_pages(app: &AppHandle, window: &Window) {
    match app.get_webview(&label(window.label())) {
        Some(pane) => raise(&pane),
        None => {
            let made = window.add_child(
                WebviewBuilder::new(label(window.label()), WebviewUrl::App("pane".into())),
                LogicalPosition::new(0.0, 0.0),
                LogicalSize::new(1.0, 1.0),
            );
            // placed and shown once the page tells where its viewer is
            match made {
                Ok(pane) => {
                    let _ = pane.hide();
                }
                Err(e) => eprintln!("{e}"),
            }
        }
    }
}

/// A web view brought above the others of its window: AppKit stacks a window's views in the order they were added.
fn raise(webview: &tauri::Webview) {
    #[cfg(target_os = "macos")]
    let _ = webview.with_webview(|platform| {
        use objc2_app_kit::{NSView, NSWindowOrderingMode};
        // the web view lives while its window does, and this runs on the main thread
        let view = unsafe { &*(platform.inner() as *const NSView) };
        if let Some(parent) = unsafe { view.superview() } {
            parent.addSubview_positioned_relativeTo(view, NSWindowOrderingMode::Above, None);
        }
    });
    #[cfg(not(target_os = "macos"))]
    let _ = webview;
}

/// The viewer of the page `page` moved or changed size: the pane follows, if the page is in front.
pub fn set_area(app: &AppHandle, page: &str, area: Area) {
    let Some(window) = papers_tabs::window_of(app, page) else {
        return;
    };
    if papers_tabs::front_of(app, &window).as_deref() != Some(page) {
        return;
    }
    let window = window.as_str();
    let changed = {
        let state = app.state::<AppState>();
        let mut panes = state.panes.lock().unwrap();
        panes.areas.insert(window.to_string(), area) != Some(area)
    };
    if changed {
        place(app, window);
    }
}

/// ⌘J: the pane hidden when it is open in this window, wherever the keys are;
/// else shown with them (expanded if collapsed).
pub fn toggle(app: &AppHandle, window: &str) {
    let Some((_, layout)) = layout_of(app, window) else {
        return;
    };
    if layout.shown && !layout.collapsed {
        hide(app, window);
    } else {
        show(app, window);
    }
}

/// The pane shown and expanded in this window, with the keys.
pub fn show(app: &AppHandle, window: &str) {
    let Some((root, mut layout)) = layout_of(app, window) else {
        return;
    };
    layout.shown = true;
    layout.collapsed = false;
    keep(app, root, layout);
    place(app, window);
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
    place(app, window);
    keys_to_page(app, window);
}

/// Escape in the pane: the keys back to the page, the pane left shown.
pub fn release(app: &AppHandle, window: &str) {
    keys_to_page(app, window);
}

/// A quote into the review at the pane's cursor: the pane shown and expanded
/// in this window, with the keys, to write what it is about.
pub fn insert(app: &AppHandle, window: &str, text: &str) {
    show(app, window);
    let _ = app.emit_to(EventTarget::webview(label(window)), "review-insert", text);
}

/// A card of a page shown or gone: the pane hidden meanwhile when the page is in front, then placed again as it
/// was.
pub fn set_covered(app: &AppHandle, page: &str, covered: bool) {
    {
        let state = app.state::<AppState>();
        let mut panes = state.panes.lock().unwrap();
        if covered {
            panes.covered.insert(page.to_string());
        } else {
            panes.covered.remove(page);
        }
    }
    if let Some(window) = papers_tabs::window_of(app, page) {
        place(app, &window);
    }
}

/// The keys from the pane back to the page in front (and its PDF).
fn keys_to_page(app: &AppHandle, window: &str) {
    app.state::<AppState>().panes.lock().unwrap().focused.remove(window);
    let Some(front) = papers_tabs::front_of(app, window) else {
        return;
    };
    if let Some(page) = app.get_webview(&front) {
        let _ = page.set_focus();
    }
    let _ = app.emit_to(EventTarget::webview(front), "pane-hidden", ());
}

/// The pane's collapse button: the pane collapsed to its header bar, the keys
/// back to the page (and its PDF), or expanded again.
pub fn collapse(app: &AppHandle, window: &str, collapsed: bool) {
    let Some((root, mut layout)) = layout_of(app, window) else {
        return;
    };
    layout.collapsed = collapsed;
    keep(app, root, layout);
    place(app, window);
    if collapsed {
        keys_to_page(app, window);
    } else if let Some(pane) = app.get_webview(&label(window)) {
        let _ = pane.set_focus();
    }
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

/// A window switched to another project: the pane placed as that project keeps it.
pub fn switched(app: &AppHandle, window: &str) {
    place(app, window);
}

/// A page gone: whether it showed a card.
pub fn page_closed(app: &AppHandle, page: &str) {
    app.state::<AppState>().panes.lock().unwrap().covered.remove(page);
}

/// A window gone: what was known of its pane goes.
pub fn closed(app: &AppHandle, window: &str) {
    let state = app.state::<AppState>();
    let mut panes = state.panes.lock().unwrap();
    panes.areas.remove(window);
    panes.focused.remove(window);
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
        viewport: 800.0,
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
    fn collapsed_it_is_its_header_at_the_bottom_or_the_right_of_the_viewer() {
        let mut layout = PaneLayout {
            collapsed: true,
            bottom: 280.0,
            right: 400.0,
            ..PaneLayout::default()
        };
        assert_eq!(bounds(AREA, &layout), (248.0, 30.0 + 770.0 - 8.0 - HEADER, 944.0, HEADER));
        layout.side = Side::Right;
        assert_eq!(bounds(AREA, &layout), (240.0 + 960.0 - 8.0 - HEADER, 38.0, HEADER, 754.0));
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
