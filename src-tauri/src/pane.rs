//! The review pane: a web view of its own in a project's window, of the project the window shows, over the viewer;
//! above the pages (the tabs), and so above the PDF view that lies in a page. It lies along a side of the viewer (in
//! the middle at the top or the bottom, as tall as the viewer on the left or the right) or free, where its header was
//! dragged ([`Dock`]); its edges dragged set its size. The page in front tells where the viewer is (`pane_area`); the
//! pane's place, its size in each place and whether it is shown are kept per project (state.json). Collapsed, it is
//! no more than its header bar (`HEADER`), where it lies, or a strip down the left or right side.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, EventTarget, LogicalPosition, LogicalSize, Manager, Rect, WebviewBuilder, WebviewUrl, Window};

use crate::state::{Dock, Frame, PaneLayout};
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
    /// the pane of each window being dragged: where it was as the drag began
    pub drags: HashMap<String, Bounds>,
}

/// Where the pane lies: x, y, width and height, in points from the top left of the page.
pub type Bounds = (f64, f64, f64, f64);

/// What of the pane is dragged: its header (the pane moved), or an edge or a corner (by the compass).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Grip {
    Move,
    N,
    S,
    E,
    W,
    Ne,
    Nw,
    Se,
    Sw,
}

impl Grip {
    /// The edges it moves: top, bottom, left, right.
    fn edges(self) -> (bool, bool, bool, bool) {
        use Grip::*;
        (
            matches!(self, N | Ne | Nw),
            matches!(self, S | Se | Sw),
            matches!(self, W | Nw | Sw),
            matches!(self, E | Ne | Se),
        )
    }
}

/// What the pane's page learns of where it is, to draw its edge.
#[derive(Debug, Clone, Serialize)]
pub struct Placed {
    dock: Dock,
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

/// The pane's bounds in the viewer, its size kept within it, `MARGIN` from its edges; collapsed, the header bar alone
/// at the same place (a strip on the left or the right).
fn bounds(area: Area, layout: &PaneLayout) -> Bounds {
    let (x, y) = (area.x + MARGIN, area.y + MARGIN);
    let (width, height) = ((area.width - 2.0 * MARGIN).max(0.0), (area.height - 2.0 * MARGIN).max(0.0));
    // along a side, it leaves `LEFT` of the viewer
    let tall = |want: f64| want.clamp(SMALLEST, (height - LEFT).max(SMALLEST));
    let wide = |want: f64| want.clamp(SMALLEST, (width - LEFT).max(SMALLEST));
    let collapsed = |size: f64| if layout.collapsed { HEADER } else { size };
    match layout.dock {
        Dock::Bottom | Dock::Top => {
            let h = collapsed(tall(layout.height));
            let w = layout.across.max(SMALLEST).min(width);
            let top = if layout.dock == Dock::Top { y } else { y + height - h };
            (x + (width - w) / 2.0, top, w, h)
        }
        Dock::Right | Dock::Left => {
            let w = collapsed(wide(layout.width));
            let left = if layout.dock == Dock::Left { x } else { x + width - w };
            (left, y, w, height)
        }
        Dock::Free => {
            let free = layout.free;
            let w = free.width.max(SMALLEST).min(width);
            let h = collapsed(free.height.max(SMALLEST).min(height));
            let left = (area.x + free.x).min(x + width - w).max(x);
            let top = (area.y + free.y).min(y + height - h).max(y);
            (left, top, w, h)
        }
    }
}

/// Where the pane lies, at its size when expanded: a collapsed pane dragged free keeps its size for when it opens.
fn expanded_at(area: Area, layout: &PaneLayout) -> Bounds {
    let (x, y, ..) = bounds(area, layout);
    let (_, _, width, height) = bounds(area, &PaneLayout { collapsed: false, ..layout.clone() });
    (x, y, width, height)
}

/// The pane's layout as dragged from `start` (where it was as the drag began, at its size when expanded) by `dx`,
/// `dy` (points). The header moves it, free; at the top or the bottom its sides widen it on both sides, to stay in
/// the middle.
fn dragged(layout: &mut PaneLayout, area: Area, start: Bounds, grip: Grip, dx: f64, dy: f64) {
    let (x, y, w, h) = start;
    let (top, bottom, left, right) = grip.edges();
    let free = |x: f64, y: f64, width: f64, height: f64| Frame { x: x - area.x, y: y - area.y, width, height };
    match layout.dock {
        _ if grip == Grip::Move => {
            layout.dock = Dock::Free;
            layout.free = free(x + dx, y + dy, w, h);
        }
        Dock::Bottom | Dock::Top => {
            if left || right {
                layout.across = w + 2.0 * if left { -dx } else { dx };
            }
            if top && layout.dock == Dock::Bottom {
                layout.height = h - dy;
            }
            if bottom && layout.dock == Dock::Top {
                layout.height = h + dy;
            }
        }
        Dock::Right if left => layout.width = w - dx,
        Dock::Left if right => layout.width = w + dx,
        Dock::Right | Dock::Left => {}
        Dock::Free => {
            let width = (if left { w - dx } else if right { w + dx } else { w }).max(SMALLEST);
            let height = (if top { h - dy } else if bottom { h + dy } else { h }).max(SMALLEST);
            // the edge across from the one dragged stays
            let x = if left { x + w - width } else { x };
            let y = if top { y + h - height } else { y };
            layout.free = free(x, y, width, height);
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
    let placed = Placed { dock: layout.dock, collapsed: layout.collapsed };
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

/// Whether the pane of `window` is shown and expanded: the page's button for it is on.
pub fn shown(app: &AppHandle, window: &str) -> bool {
    layout_of(app, window).is_some_and(|(_, layout)| layout.shown && !layout.collapsed)
}

/// The page in front told whether the pane is shown and expanded (`pane-shown`), for its button; a page brought to
/// the front asks (`pane_shown`).
fn tell(app: &AppHandle, window: &str) {
    if let Some(front) = papers_tabs::front_of(app, window) {
        let _ = app.emit_to(EventTarget::webview(front), "pane-shown", shown(app, window));
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
    tell(app, window);
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
    tell(app, window);
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
    tell(app, window);
    if collapsed {
        keys_to_page(app, window);
    } else if let Some(pane) = app.get_webview(&label(window)) {
        let _ = pane.set_focus();
    }
}

/// ⌘⇧J: the pane to its next place.
pub fn next_dock(app: &AppHandle, window: &str) {
    if let Some((_, layout)) = layout_of(app, window) {
        dock(app, window, layout.dock.next());
    }
}

/// The pane to `dock` (the menu of its header).
pub fn dock(app: &AppHandle, window: &str, dock: Dock) {
    let Some((root, mut layout)) = layout_of(app, window) else {
        return;
    };
    layout.dock = dock;
    keep(app, root, layout);
    place(app, window);
}

/// The pane's header, an edge or a corner dragged by `dx`, `dy` (points) since the drag began; `done` when let go,
/// kept then.
pub fn drag(app: &AppHandle, window: &str, grip: Grip, dx: f64, dy: f64, done: bool) {
    let Some((root, mut layout)) = layout_of(app, window) else {
        return;
    };
    let state = app.state::<AppState>();
    let start = {
        let mut panes = state.panes.lock().unwrap();
        let Some(area) = panes.areas.get(window).copied() else {
            return;
        };
        let start = *panes.drags.entry(window.to_string()).or_insert_with(|| expanded_at(area, &layout));
        if done {
            panes.drags.remove(window);
        }
        (area, start)
    };
    let (area, start) = start;
    dragged(&mut layout, area, start, grip, dx, dy);
    if done {
        // free, it is kept where it lies, within the viewer, not where it was dropped beyond it
        if layout.dock == Dock::Free {
            let (x, y, width, height) = expanded_at(area, &layout);
            layout.free = Frame { x: x - area.x, y: y - area.y, width, height };
        }
        keep(app, root, layout);
    } else {
        state.saved.lock().unwrap().panes.insert(root, layout);
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
    panes.drags.remove(window);
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

    fn layout(dock: Dock) -> PaneLayout {
        PaneLayout {
            dock,
            height: 280.0,
            across: 600.0,
            width: 400.0,
            free: Frame { x: 100.0, y: 50.0, width: 500.0, height: 300.0 },
            ..PaneLayout::default()
        }
    }

    #[test]
    fn at_the_top_or_the_bottom_it_is_in_the_middle_on_the_left_or_the_right_as_tall_as_the_viewer() {
        let middle = 240.0 + (960.0 - 600.0) / 2.0;
        assert_eq!(bounds(AREA, &layout(Dock::Bottom)), (middle, 30.0 + 770.0 - 8.0 - 280.0, 600.0, 280.0));
        assert_eq!(bounds(AREA, &layout(Dock::Top)), (middle, 38.0, 600.0, 280.0));
        assert_eq!(bounds(AREA, &layout(Dock::Right)), (240.0 + 960.0 - 8.0 - 400.0, 38.0, 400.0, 754.0));
        assert_eq!(bounds(AREA, &layout(Dock::Left)), (248.0, 38.0, 400.0, 754.0));
    }

    #[test]
    fn free_it_is_where_it_was_put_kept_within_the_viewer() {
        assert_eq!(bounds(AREA, &layout(Dock::Free)), (340.0, 80.0, 500.0, 300.0));
        let mut away = layout(Dock::Free);
        away.free.x = 5000.0;
        away.free.y = -100.0;
        assert_eq!(bounds(AREA, &away), (240.0 + 960.0 - 8.0 - 500.0, 38.0, 500.0, 300.0));
    }

    #[test]
    fn collapsed_it_is_its_header_where_it_lies() {
        let collapsed = |dock| bounds(AREA, &PaneLayout { collapsed: true, ..layout(dock) });
        assert_eq!(collapsed(Dock::Bottom), (420.0, 30.0 + 770.0 - 8.0 - HEADER, 600.0, HEADER));
        assert_eq!(collapsed(Dock::Top), (420.0, 38.0, 600.0, HEADER));
        assert_eq!(collapsed(Dock::Right), (240.0 + 960.0 - 8.0 - HEADER, 38.0, HEADER, 754.0));
        assert_eq!(collapsed(Dock::Left), (248.0, 38.0, HEADER, 754.0));
        assert_eq!(collapsed(Dock::Free), (340.0, 80.0, 500.0, HEADER));
    }

    #[test]
    fn its_size_stays_within_the_viewer() {
        let mut big = layout(Dock::Bottom);
        big.height = 5000.0;
        big.across = 5000.0;
        let (_, _, width, height) = bounds(AREA, &big);
        assert_eq!((width, height), (944.0, 770.0 - 16.0 - LEFT));
        big.height = 10.0;
        assert_eq!(bounds(AREA, &big).3, SMALLEST);
    }

    #[test]
    fn its_header_dragged_frees_it_where_it_was_dropped() {
        let mut pane = layout(Dock::Bottom);
        let start = bounds(AREA, &pane);
        dragged(&mut pane, AREA, start, Grip::Move, -50.0, -200.0);
        assert_eq!(pane.dock, Dock::Free);
        assert_eq!(bounds(AREA, &pane), (start.0 - 50.0, start.1 - 200.0, 600.0, 280.0));
    }

    #[test]
    fn a_side_dragged_at_the_bottom_widens_it_on_both_sides() {
        let mut pane = layout(Dock::Bottom);
        let start = bounds(AREA, &pane);
        dragged(&mut pane, AREA, start, Grip::E, 30.0, 0.0);
        assert_eq!(bounds(AREA, &pane).0 + bounds(AREA, &pane).2, start.0 + start.2 + 30.0);
        dragged(&mut pane, AREA, start, Grip::Nw, 30.0, -40.0);
        assert_eq!((pane.across, pane.height), (540.0, 320.0));
    }

    #[test]
    fn free_an_edge_dragged_leaves_the_edge_across_from_it() {
        let mut pane = layout(Dock::Free);
        let start = bounds(AREA, &pane);
        dragged(&mut pane, AREA, start, Grip::Nw, 20.0, 30.0);
        assert_eq!(bounds(AREA, &pane), (360.0, 110.0, 480.0, 270.0));
        dragged(&mut pane, AREA, start, Grip::W, 1000.0, 0.0);
        let (x, _, width, _) = bounds(AREA, &pane);
        assert_eq!((x + width, width), (840.0, SMALLEST));
    }
}
