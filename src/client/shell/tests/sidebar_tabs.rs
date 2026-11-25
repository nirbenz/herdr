use super::*;
use crate::api::schema::Method;
use crate::config::SidebarTabsMode;

fn tree_snapshot() -> ClientShellSnapshot {
    let mut projected = snapshot();
    let mut second = projected.workspaces[0].clone();
    second.workspace_id = "ws_2".into();
    second.number = 2;
    second.label = "second".into();
    second.focused = false;
    projected.workspaces.push(second);
    projected.tabs = [
        ("tab_1", "ws_1", 1, "editor"),
        ("tab_2", "ws_1", 2, "server"),
        ("tab_3", "ws_1", 3, "logs"),
        ("tab_4", "ws_2", 1, "api"),
        ("tab_5", "ws_2", 2, "tests"),
    ]
    .into_iter()
    .map(|(tab_id, workspace_id, number, label)| ClientShellTab {
        tab_id: tab_id.into(),
        workspace_id: workspace_id.into(),
        number,
        label: label.into(),
        custom_label: true,
        zoomed: false,
        focused: tab_id == "tab_1",
        agent_status: AgentStatus::Idle,
    })
    .collect();
    projected
}

fn tree_state(mode: SidebarTabsMode, projected: ClientShellSnapshot) -> ClientShellState {
    let mut config = ClientShellConfig::from_config(&Config::default());
    config.spaces.tabs = mode;
    let mut state = ClientShellState::new(config);
    state.set_snapshot(Box::new(projected));
    state.set_pane_surface(surface());
    state.compose(100, 28).unwrap();
    state
}

fn tab_hit_ids(state: &ClientShellState) -> Vec<&str> {
    state
        .hits
        .sidebar_tabs
        .iter()
        .map(|hit| hit.tab_id.as_str())
        .collect()
}

fn tab_rect(state: &ClientShellState, tab_id: &str) -> Rect {
    state
        .hits
        .sidebar_tabs
        .iter()
        .find(|hit| hit.tab_id == tab_id)
        .map(|hit| hit.rect)
        .expect("visible tab row")
}

fn row_text(buffer: &Buffer, rect: Rect) -> String {
    (rect.x..rect.right())
        .map(|x| buffer[(x, rect.y)].symbol())
        .collect()
}

fn focused_method(outcome: &ClientShellInput) -> &Method {
    let [ClientShellAction::Endpoint { request, .. }] = &outcome.actions[..] else {
        panic!("expected one endpoint request, got {:?}", outcome.actions);
    };
    &request.method
}

fn key(state: &mut ClientShellState, bytes: &[u8]) -> ClientShellInput {
    state.handle_input_bytes(bytes)
}

#[test]
fn tab_rows_are_off_by_default() {
    let mut state = ClientShellState::new(ClientShellConfig::from_config(&Config::default()));
    state.set_snapshot(Box::new(tree_snapshot()));
    state.set_pane_surface(surface());
    state.compose(100, 28).unwrap();
    assert!(state.hits.sidebar_tabs.is_empty());
}

#[test]
fn active_mode_draws_focused_workspace_tabs_as_a_tree_below_it() {
    let mut state = tree_state(SidebarTabsMode::Active, tree_snapshot());
    let buffer = state.compose(100, 28).unwrap().to_ratatui_buffer().unwrap();
    assert_eq!(tab_hit_ids(&state), ["tab_1", "tab_2", "tab_3"]);

    let workspace = state
        .hits
        .workspaces
        .iter()
        .find(|hit| hit.workspace_id == "ws_1")
        .unwrap()
        .rect;
    let first = tab_rect(&state, "tab_1");
    assert_eq!(first.y, workspace.bottom());
    assert_eq!(tab_rect(&state, "tab_3").y, first.y + 2);
    let second_workspace = state
        .hits
        .workspaces
        .iter()
        .find(|hit| hit.workspace_id == "ws_2")
        .unwrap()
        .rect;
    assert!(second_workspace.y > tab_rect(&state, "tab_3").y);

    assert!(row_text(&buffer, first).contains("├─ ○ 1 editor"));
    assert!(row_text(&buffer, tab_rect(&state, "tab_3")).contains("└─ ○ 3 logs"));
}

#[test]
fn all_mode_lists_every_workspace_with_more_than_one_tab() {
    let mut projected = tree_snapshot();
    let mut third = projected.workspaces[1].clone();
    third.workspace_id = "ws_3".into();
    third.number = 3;
    projected.workspaces.push(third);
    let mut lone = projected.tabs[0].clone();
    lone.tab_id = "tab_6".into();
    lone.workspace_id = "ws_3".into();
    lone.focused = false;
    projected.tabs.push(lone);

    let state = tree_state(SidebarTabsMode::All, projected);
    assert_eq!(
        tab_hit_ids(&state),
        ["tab_1", "tab_2", "tab_3", "tab_4", "tab_5"]
    );
}

#[test]
fn clicking_a_tab_row_focuses_that_tab() {
    let mut state = tree_state(SidebarTabsMode::All, tree_snapshot());
    let rect = tab_rect(&state, "tab_5");
    let outcome = state.handle_raw_events(vec![RawInputEvent::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: rect.x + 4,
        row: rect.y,
        modifiers: KeyModifiers::empty(),
    })]);
    assert!(matches!(
        focused_method(&outcome),
        Method::TabFocus(target) if target.tab_id == "tab_5"
    ));
}

#[test]
fn workspace_navigation_walks_tab_rows_and_enter_focuses_the_tab() {
    let mut state = tree_state(SidebarTabsMode::Active, tree_snapshot());
    key(&mut state, &[0x02]);
    key(&mut state, b"w");
    assert_eq!(state.mode, ClientShellMode::Navigate);
    assert_eq!(state.navigate_sidebar_tab_id(), Some("tab_1"));

    for expected in [Some("tab_2"), Some("tab_3"), None] {
        key(&mut state, b"\x1b[B");
        assert_eq!(state.navigate_sidebar_tab_id(), expected);
    }
    assert_eq!(
        state.navigate_workspace_id,
        state.navigation_target(&ClientEndpointId::Local, "ws_2")
    );

    key(&mut state, b"\x1b[A");
    assert_eq!(state.navigate_sidebar_tab_id(), Some("tab_3"));
    let enter = key(&mut state, b"\r");
    assert!(matches!(
        focused_method(&enter),
        Method::TabFocus(target) if target.tab_id == "tab_3"
    ));
    assert_eq!(state.mode, ClientShellMode::Terminal);
    assert!(state.navigate_sidebar_tab_id().is_none());
}

#[test]
fn highlighted_tab_row_takes_the_selection_instead_of_its_workspace() {
    let mut state = tree_state(SidebarTabsMode::Active, tree_snapshot());
    key(&mut state, &[0x02]);
    key(&mut state, b"w");
    key(&mut state, b"\x1b[B");
    let buffer = state.compose(100, 28).unwrap().to_ratatui_buffer().unwrap();
    let selection = state.config.palette.selection_bg;
    let selected = tab_rect(&state, "tab_2");
    assert_eq!(buffer[(selected.x, selected.y)].bg, selection);
    let workspace = state
        .hits
        .workspaces
        .iter()
        .find(|hit| hit.workspace_id == "ws_1")
        .unwrap()
        .rect;
    assert_ne!(buffer[(workspace.x, workspace.y)].bg, selection);
}

#[test]
fn spaces_tabs_setting_parses_and_rejects_unknown_values() {
    let config: Config = toml::from_str("[ui.sidebar.spaces]\ntabs = \"all\"\n").unwrap();
    assert_eq!(config.ui.sidebar.spaces.tabs, SidebarTabsMode::All);
    assert!(toml::from_str::<Config>("[ui.sidebar.spaces]\ntabs = \"some\"\n").is_err());
}

#[test]
fn machine_sidebar_lists_tabs_per_machine_and_clicks_target_that_machine() {
    let (mut state, remote) = super::endpoints::state_with_remote();
    state.config.spaces.tabs = SidebarTabsMode::All;
    state.set_snapshot(Box::new(tree_snapshot()));
    let mut remote_snapshot = tree_snapshot();
    remote_snapshot.boot_id = "remote-boot".into();
    state.set_endpoint_snapshot(&remote, Box::new(remote_snapshot));
    state.compose(100, 70).unwrap();

    let remote_hits = state
        .hits
        .sidebar_tabs
        .iter()
        .filter(|hit| hit.endpoint_id == remote)
        .map(|hit| (hit.tab_id.clone(), hit.rect))
        .collect::<Vec<_>>();
    assert_eq!(remote_hits.len(), 5);
    assert_eq!(state.hits.sidebar_tabs.len(), 10);

    let (_, rect) = &remote_hits[1];
    let outcome = state.handle_raw_events(vec![RawInputEvent::Mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: rect.x + 4,
        row: rect.y,
        modifiers: KeyModifiers::empty(),
    })]);
    assert!(matches!(
        outcome.actions.as_slice(),
        [ClientShellAction::ActivateEndpoint {
            endpoint_id,
            target: Some(ClientEndpointFocusTarget::Tab(tab_id)),
        }] if endpoint_id == &remote && tab_id == "tab_2"
    ));
}
