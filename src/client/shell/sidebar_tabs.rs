//! Tabs listed as tree rows under their workspace in the expanded sidebar.
//!
//! Enabled with `ui.sidebar.spaces.tabs`. The rows are part of the workspace
//! block for layout and scrolling, but carry their own hits and navigation
//! targets so a click or Enter focuses the tab instead of the workspace.

use super::*;
use crate::config::SidebarTabsMode;

pub(super) struct SidebarTabHit {
    pub(super) rect: Rect,
    pub(super) endpoint_id: ClientEndpointId,
    pub(super) tab_id: String,
}

/// Returns the tabs to draw under `workspace`, in tab order.
///
/// A workspace with a single tab gets no rows; its own row already stands for it.
pub(super) fn sidebar_tabs<'a>(
    snapshot: &'a ClientShellSnapshot,
    workspace: &ClientShellWorkspace,
    mode: SidebarTabsMode,
    endpoint_active: bool,
) -> Vec<&'a ClientShellTab> {
    let shown = match mode {
        SidebarTabsMode::Off => false,
        SidebarTabsMode::Active => endpoint_active && workspace.focused,
        SidebarTabsMode::All => true,
    };
    if !shown {
        return Vec::new();
    }
    let tabs = snapshot
        .tabs
        .iter()
        .filter(|tab| tab.workspace_id == workspace.workspace_id)
        .collect::<Vec<_>>();
    if tabs.len() < 2 {
        return Vec::new();
    }
    tabs
}

/// Draws one tree row per tab, starting at `area.y`, and records a hit for each.
#[allow(clippy::too_many_arguments)]
pub(super) fn render_sidebar_tabs(
    buffer: &mut Buffer,
    area: Rect,
    tabs: &[&ClientShellTab],
    entry: &WorkspaceEntry,
    endpoint_id: &ClientEndpointId,
    endpoint_active: bool,
    selected_tab_id: Option<&str>,
    config: &ClientShellConfig,
    hits: &mut ShellHitMap,
) {
    let palette = &config.palette;
    let indent = match (entry.indented, entry.last_child) {
        (false, _) => "   ",
        (true, true) => "        ",
        (true, false) => "   │    ",
    };
    let right = area.right().saturating_sub(2);
    for (index, tab) in tabs.iter().enumerate() {
        let y = area.y.saturating_add(index as u16);
        if y >= area.bottom() {
            break;
        }
        let rect = Rect::new(area.x, y, area.width, 1);
        let selected = selected_tab_id == Some(tab.tab_id.as_str());
        if selected {
            buffer.set_style(
                rect,
                Style::default().bg(render::sidebar::workspace_selection_background(palette)),
            );
        }
        let branch = if index + 1 == tabs.len() {
            "└─ "
        } else {
            "├─ "
        };
        let guide = Style::default().fg(palette.overlay0);
        let mut x = render::put_segment(buffer, area.x, y, right, indent, guide);
        x = render::put_segment(buffer, x, y, right, branch, guide);
        x = render::put_segment(
            buffer,
            x,
            y,
            right,
            status_icon(tab.agent_status, config.status_indicators),
            Style::default().fg(status_color(tab.agent_status, palette)),
        );
        x = render::put_segment(
            buffer,
            x,
            y,
            right,
            &format!(" {} ", tab.number),
            Style::default().fg(palette.overlay0),
        );
        let focused = endpoint_active && tab.focused;
        render::put_segment(
            buffer,
            x,
            y,
            right,
            &tab.label,
            if focused {
                Style::default()
                    .fg(palette.text)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(palette.subtext0)
            },
        );
        hits.sidebar_tabs.push(SidebarTabHit {
            rect,
            endpoint_id: endpoint_id.clone(),
            tab_id: tab.tab_id.clone(),
        });
    }
}

impl ClientShellState {
    /// Tab IDs that workspace navigation visits right after `workspace`.
    pub(super) fn sidebar_tab_navigation_ids(
        &self,
        endpoint_id: &ClientEndpointId,
        snapshot: &ClientShellSnapshot,
        workspace: &ClientShellWorkspace,
    ) -> Vec<String> {
        sidebar_tabs(
            snapshot,
            workspace,
            self.config.spaces.tabs,
            endpoint_id == &self.active_endpoint_id,
        )
        .into_iter()
        .map(|tab| tab.tab_id.clone())
        .collect()
    }

    /// Starts navigation on the focused tab row when the focused workspace shows its tabs.
    pub(super) fn start_sidebar_tab_navigation(&mut self) {
        self.navigate_sidebar_tab = self.navigate_workspace_id.clone().and_then(|target| {
            let snapshot = self.snapshot.as_deref()?;
            let workspace = snapshot
                .workspaces
                .iter()
                .find(|workspace| workspace.workspace_id == target.workspace_id)?;
            let tab_id = sidebar_tabs(snapshot, workspace, self.config.spaces.tabs, true)
                .into_iter()
                .find(|tab| tab.focused)?
                .tab_id
                .clone();
            Some((target, tab_id))
        });
    }

    /// The tab row highlighted by workspace navigation, if the highlight is on one.
    pub(super) fn navigate_sidebar_tab_id(&self) -> Option<&str> {
        let (target, tab_id) = self.navigate_sidebar_tab.as_ref()?;
        (self.navigate_workspace_id.as_ref() == Some(target)).then_some(tab_id.as_str())
    }

    /// Focuses the highlighted tab row. Returns false when the highlight is on a workspace.
    pub(super) fn accept_navigate_sidebar_tab(&mut self, outcome: &mut ClientShellInput) -> bool {
        let Some(tab_id) = self.navigate_sidebar_tab_id().map(str::to_owned) else {
            return false;
        };
        let Some(target) = self.navigate_workspace_id.clone() else {
            return false;
        };
        if self.focus_or_activate(
            target.endpoint_id,
            ClientEndpointFocusTarget::Tab(tab_id),
            outcome,
        ) {
            self.mode = ClientShellMode::Terminal;
            self.navigate_workspace_id = None;
            self.navigate_sidebar_tab = None;
        }
        outcome.repaint = true;
        true
    }

    /// Handles a press on a sidebar tab row. Returns false when nothing was hit.
    pub(super) fn press_sidebar_tab(
        &mut self,
        point: (u16, u16),
        outcome: &mut ClientShellInput,
    ) -> bool {
        let Some((endpoint_id, tab_id)) = self
            .hits
            .sidebar_tabs
            .iter()
            .find(|hit| contains(hit.rect, point))
            .map(|hit| (hit.endpoint_id.clone(), hit.tab_id.clone()))
        else {
            return false;
        };
        self.focus_or_activate(endpoint_id, ClientEndpointFocusTarget::Tab(tab_id), outcome);
        outcome.repaint = true;
        true
    }
}
