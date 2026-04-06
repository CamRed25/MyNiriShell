// ShellState — shared compositor state updated from niri IPC events.
// Consumed by panel_ui (workspace dots) and dock_ui (active window list).

use std::cell::RefCell;
use std::rc::Rc;

use crate::dock_backend::{DockItem, DockState};
use crate::ipc::types::{NiriEvent, Window, Workspace};
use crate::panel_backend::PanelState;

pub struct ShellState {
    pub panel: Rc<RefCell<PanelState>>,
    pub dock: Rc<RefCell<DockState>>,
    /// Cached workspace list — updated on WorkspacesChanged, read on WorkspaceActivated.
    workspaces: RefCell<Vec<Workspace>>,
    /// ID of the currently focused window.
    focused_window_id: RefCell<Option<u64>>,
}

impl ShellState {
    pub fn new() -> Self {
        Self {
            panel: Rc::new(RefCell::new(PanelState::new())),
            dock: Rc::new(RefCell::new(DockState::new())),
            workspaces: RefCell::new(Vec::new()),
            focused_window_id: RefCell::new(None),
        }
    }

    /// Apply an event from the niri compositor to the shared state.
    pub fn apply_event(&self, event: NiriEvent) {
        match event {
            NiriEvent::WorkspacesChanged { workspaces } => {
                log::info!("IPC: WorkspacesChanged ({} workspaces)", workspaces.len());
                self.on_workspaces(workspaces);
            }
            NiriEvent::WindowsChanged { windows } => {
                log::info!("IPC: WindowsChanged ({} windows)", windows.len());
                self.on_windows(windows);
            }
            NiriEvent::WindowOpenedOrChanged { window } => {
                log::info!("IPC: WindowOpenedOrChanged id={}", window.id);
                self.on_window_changed(window);
            }
            NiriEvent::WindowClosed { id } => {
                log::info!("IPC: WindowClosed id={id}");
                self.on_window_closed(id);
            }
            NiriEvent::WindowFocusChanged { id } => {
                log::info!("IPC: WindowFocusChanged id={id:?}");
                self.on_focus_changed(id);
            }
            NiriEvent::WorkspaceActivated { id, focused } => {
                log::info!("IPC: WorkspaceActivated id={id} focused={focused}");
                self.on_workspace_activated(id);
            }
        }
    }

    fn on_workspaces(&self, workspaces: Vec<Workspace>) {
        if workspaces.is_empty() {
            return;
        }
        self.update_panel_workspaces(&workspaces);
        *self.workspaces.borrow_mut() = workspaces;
    }

    fn on_workspace_activated(&self, activated_id: u64) {
        // Mark the activated workspace as focused in the cache and refresh panel.
        let mut ws = self.workspaces.borrow_mut();
        for w in ws.iter_mut() {
            w.is_focused = w.id == activated_id;
        }
        self.update_panel_workspaces(&ws);
    }

    fn update_panel_workspaces(&self, workspaces: &[Workspace]) {
        let focused = workspaces.iter().position(|w| w.is_focused).unwrap_or(0);
        let names: Vec<String> = workspaces
            .iter()
            .map(|w| w.name.clone().unwrap_or_else(|| w.idx.to_string()))
            .collect();
        let occupied: Vec<bool> =
            workspaces.iter().map(|w| w.active_window_id.is_some()).collect();
        if let Err(e) = self.panel.borrow_mut().update_workspaces(focused, names, occupied) {
            log::warn!("panel workspace update failed: {e}");
        }
    }

    fn on_windows(&self, windows: Vec<Window>) {
        let active: Vec<DockItem> = windows.iter().map(window_to_dock_item).collect();
        self.dock.borrow_mut().set_active_windows(active);
    }

    fn on_window_changed(&self, window: Window) {
        let item = window_to_dock_item(&window);
        let mut dock = self.dock.borrow_mut();
        let mut active = dock.active.clone();
        if let Some(pos) = active.iter().position(|a| a.niri_id == item.niri_id) {
            active[pos] = item;
        } else {
            active.push(item);
        }
        dock.set_active_windows(active);
    }

    fn on_window_closed(&self, id: u64) {
        let mut dock = self.dock.borrow_mut();
        let active: Vec<DockItem> =
            dock.active.iter().filter(|a| a.niri_id != id).cloned().collect();
        dock.set_active_windows(active);
    }

    fn on_focus_changed(&self, focused_id: Option<u64>) {
        *self.focused_window_id.borrow_mut() = focused_id;
        let mut dock = self.dock.borrow_mut();
        let active: Vec<DockItem> = dock
            .active
            .iter()
            .map(|a| {
                let mut item = a.clone();
                item.is_active = focused_id == Some(a.niri_id);
                item
            })
            .collect();
        dock.set_active_windows(active);
    }
}

impl Default for ShellState {
    fn default() -> Self {
        Self::new()
    }
}

fn window_to_dock_item(w: &Window) -> DockItem {
    let app_id = w.app_id.clone().unwrap_or_default();
    let title = w.title.clone().unwrap_or_else(|| app_id.clone());
    let mut item = DockItem::active(app_id.clone(), title, app_id, w.id);
    item.workspace_id = w.workspace_id.unwrap_or(0);
    item
}
