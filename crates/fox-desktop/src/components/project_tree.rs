//! 左侧侧边栏：Header（项目选择器）| Toolbar（＋文件夹 / ＋接口）| Search | Tree | Footer（环境选择器）。

use std::cell::RefCell;
use std::rc::Rc;

use dioxus::prelude::*;
use fox_core::curl_parser::parse_curl;
use fox_core::model::{Endpoint, Folder};
use uuid::Uuid;

use crate::components::dropdown::Dropdown;
use crate::components::icons::{CaretIcon, FolderIcon, SearchIcon};
use crate::state::AppState;

/// 树操作动作。
#[derive(Debug, Clone)]
pub enum TreeAction {
    CreateFolder { parent_id: Option<Uuid> },
    CreateEndpoint { folder_id: Option<Uuid> },
    ImportCurl { folder_id: Option<Uuid> },
    RenameFolder { id: Uuid, current: String },
    RenameEndpoint { id: Uuid, current: String },
    DeleteFolder { id: Uuid },
    DeleteEndpoint { id: Uuid },
    DuplicateEndpoint { id: Uuid },
}

/// 树操作分发器。
pub type Dispatcher = Rc<RefCell<dyn FnMut(TreeAction)>>;

#[component]
pub fn SideBar() -> Element {
    let state = use_context::<AppState>();

    let folders = state.folders.read().clone();
    let endpoints = state.endpoints.read().clone();
    let search = state.search.read().clone();
    let has_project = state.current_project_id.read().is_some();

    let projects = state.projects.read().clone();
    let project_options: Vec<(String, String)> = projects
        .iter()
        .map(|p| (p.id.to_string(), p.name.clone()))
        .collect();
    let project_selected: String = state
        .current_project_id
        .read()
        .map(|id| id.to_string())
        .unwrap_or_default();
    let st_project = state.clone();

    let environments = state.environments.read().clone();
    let env_options: Vec<(String, String)> = environments
        .iter()
        .map(|e| (e.id.to_string(), e.name.clone()))
        .collect();
    let env_selected: String = state
        .current_environment_id
        .read()
        .map(|id| id.to_string())
        .unwrap_or_default();
    let st_env = state.clone();

    let modal: Signal<Option<TreeAction>> = use_signal(|| None);
    let mut modal_input: Signal<String> = use_signal(String::new);
    // Toolbar「＋ 接口」下拉（HTTP 接口 / 从 cURL 导入）。
    let mut add_menu_open: Signal<bool> = use_signal(|| false);
    // 导入 cURL 弹窗。
    let curl_open: Signal<bool> = use_signal(|| false);
    let curl_target: Signal<Option<Uuid>> = use_signal(|| None);
    let curl_input: Signal<String> = use_signal(String::new);

    let dispatcher: Dispatcher = Rc::new(RefCell::new({
        let st = state.clone();
        let mut modal_sig = modal;
        let mut input = modal_input;
        let mut co = curl_open;
        let mut target = curl_target;
        let mut ci = curl_input;
        move |action| match action {
            TreeAction::CreateFolder { .. }
            | TreeAction::CreateEndpoint { .. }
            | TreeAction::RenameFolder { .. }
            | TreeAction::RenameEndpoint { .. } => {
                let init = match &action {
                    TreeAction::CreateFolder { .. } => String::new(),
                    TreeAction::CreateEndpoint { .. } => "未命名接口".to_string(),
                    TreeAction::RenameFolder { current, .. }
                    | TreeAction::RenameEndpoint { current, .. } => current.clone(),
                    _ => String::new(),
                };
                input.set(init);
                modal_sig.set(Some(action));
            }
            TreeAction::ImportCurl { folder_id } => {
                ci.set(String::new());
                target.set(folder_id);
                co.set(true);
            }
            TreeAction::DeleteFolder { id } => st.delete_folder(id),
            TreeAction::DeleteEndpoint { id } => st.delete_endpoint(id),
            TreeAction::DuplicateEndpoint { id } => st.duplicate_endpoint(id),
        }
    }));
    let dispatcher_provided = dispatcher.clone();
    use_context_provider(move || dispatcher_provided);
    let top_btn_a = dispatcher.clone();
    let top_btn_b = dispatcher.clone();
    let top_btn_c = dispatcher.clone();

    let dialog_visible = modal.read().is_some();
    let dialog_title: &str = modal
        .read()
        .as_ref()
        .map(|a| match a {
            TreeAction::CreateFolder { .. } => "新建文件夹",
            TreeAction::CreateEndpoint { .. } => "新建接口",
            TreeAction::RenameFolder { .. } => "文件夹重命名",
            TreeAction::RenameEndpoint { .. } => "接口重命名",
            _ => "",
        })
        .unwrap_or("");

    let dialog_ok = {
        let st = state.clone();
        let mut modal_sig = modal;
        let input = modal_input;
        move || {
            let name = input.peek().trim().to_string();
            if name.is_empty() {
                st.toast_error("名称不能为空");
                return;
            }
            if let Some(action) = modal_sig.peek().as_ref().cloned() {
                match action {
                    TreeAction::CreateFolder { parent_id } => st.create_folder_at(parent_id, name),
                    TreeAction::CreateEndpoint { folder_id } => {
                        st.create_endpoint_at(folder_id, name)
                    }
                    TreeAction::RenameFolder { id, .. } => st.rename_folder(id, name),
                    TreeAction::RenameEndpoint { id, .. } => st.rename_endpoint(id, name),
                    _ => {}
                }
            }
            modal_sig.set(None);
        }
    };
    let dialog_cancel = {
        let mut modal_sig = modal;
        move || modal_sig.set(None)
    };
    // 导入 cURL：解析命令并在目标目录下创建接口（cursor_target 为 None 表示项目根目录）。
    let mut do_import = {
        let st = state.clone();
        let mut co = curl_open;
        let mut ci = curl_input;
        let mut target = curl_target;
        move || {
            let folder_id = *target.peek();
            let raw = ci.peek().clone();
            match parse_curl(&raw) {
                Ok(parsed) => {
                    st.create_endpoint_from_curl(folder_id, &parsed);
                    co.set(false);
                    ci.set(String::new());
                    target.set(None);
                }
                Err(e) => st.toast_error(format!("cURL 格式无法识别：{}", e.user_message())),
            }
        }
    };
    let curl_open_flag = *curl_open.read();
    let add_menu_open_flag = *add_menu_open.read();
    let mut ok_a = dialog_ok.clone();
    let mut ok_b = dialog_ok.clone();
    let mut cancel_a = dialog_cancel;
    let mut cancel_b = dialog_cancel;

    let root_folders: Vec<Folder> = folders
        .iter()
        .filter(|f| f.parent_id.is_none())
        .cloned()
        .collect();

    let no_match = !search.is_empty()
        && !endpoints
            .iter()
            .any(|e| e.name.contains(&search) || e.path.contains(&search));
    let show_empty = folders.is_empty() && endpoints.is_empty() && search.is_empty();

    rsx! {
        aside { class: "sidebar",
            div { class: "sb-header",
                div { class: "sb-project-icon",
                    FolderIcon {}
                }
                Dropdown {
                    class: "sb-width sb-project-dropdown",
                    options: project_options,
                    selected: project_selected,
                    placeholder: "未选择项目",
                    on_select: move |v: String| {
                        if let Ok(id) = uuid::Uuid::parse_str(&v) {
                            st_project.select_project(id);
                        }
                    },
                }
            }
            div { class: "sb-toolbar",
                button {
                    id: "sb-add-folder",
                    class: "sb-toolbar-btn",
                    onclick: move |_| top_btn_a.borrow_mut()(TreeAction::CreateFolder { parent_id: None }),
                    "＋ 文件夹",
                }
                div { class: "sb-tool-group",
                    button {
                        id: "sb-add-endpoint",
                        class: "sb-toolbar-btn sb-toolbar-btn-primary",
                        onclick: move |_| {
                            let next = !*add_menu_open.peek();
                            add_menu_open.set(next);
                        },
                        "＋ 接口"
                        CaretIcon {}
                    }
                    if add_menu_open_flag {
                        div { class: "sb-menu-backdrop", onclick: move |_| add_menu_open.set(false) }
                        div { class: "sb-menu",
                            button {
                                class: "sb-menu-item",
                                onclick: move |_| {
                                    add_menu_open.set(false);
                                    top_btn_b.borrow_mut()(TreeAction::CreateEndpoint { folder_id: None });
                                },
                                "HTTP 接口",
                            }
                            button {
                                class: "sb-menu-item",
                                onclick: move |_| {
                                    add_menu_open.set(false);
                                    top_btn_c.borrow_mut()(TreeAction::ImportCurl { folder_id: None });
                                },
                                "从 cURL 导入",
                            }
                        }
                    }
                }
            }
            div { class: "sb-search",
                SearchIcon {}
                input {
                    id: "sb-search-input",
                    class: "rf-input sb-search-input",
                    placeholder: "搜索接口",
                    value: "{search}",
                    oninput: move |e| {
                        let mut s = state.search;
                        s.set(e.data().value());
                    },
                }
            }
            div { class: "sb-tree",
                if !has_project {
                    div { class: "empty sb-empty", "未选择项目，请在顶部选择" }
                } else {
                    for ep in endpoints.iter().filter(|e| e.folder_id.is_none()).cloned() {
                        if search.is_empty() || ep.name.contains(&search) || ep.path.contains(&search) {
                            EndpointRow { ep, depth: 0 }
                        }
                    }
                    for folder in root_folders {
                        FolderNode {
                            folder,
                            folders: folders.clone(),
                            endpoints: endpoints.clone(),
                            search: search.clone(),
                            depth: 0,
                        }
                    }
                    if no_match {
                        div { class: "empty", "没有匹配的接口" }
                    } else if show_empty {
                        div { class: "empty", "暂无接口，点击上方按钮创建" }
                    }
                }
            }
            div { class: "sb-footer",
                Dropdown {
                    class: "sb-width sb-env-dropdown",
                    options: env_options,
                    selected: env_selected,
                    placeholder: "未选环境",
                    on_select: move |v: String| {
                        let id = uuid::Uuid::parse_str(&v).ok();
                        st_env.select_environment(id);
                    },
                }
            }
        }
        if dialog_visible {
            div {
                class: "modal-backdrop",
                onclick: move |_| cancel_a(),
                div {
                    class: "modal",
                    onclick: |e| { e.stop_propagation(); },
                    h3 { "{dialog_title}" }
                    input {
                        class: "rf-input",
                        value: "{modal_input}",
                        oninput: move |e| modal_input.set(e.data().value()),
                        onkeydown: move |e| {
                            if e.data().key().to_string() == "Enter" {
                                ok_a();
                            }
                        },
                    }
                    div { class: "rf-modal-actions",
                        button { class: "rf-btn", onclick: move |_| cancel_b(), "取消" }
                        button { class: "rf-btn rf-btn-primary", onclick: move |_| ok_b(), "确定" }
                    }
                }
            }
        }
        if curl_open_flag {
            div {
                class: "modal-backdrop",
                onclick: move |_| {
                    let mut co = curl_open;
                    co.set(false);
                },
                div {
                    class: "modal curl-modal",
                    onclick: |e| { e.stop_propagation(); },
                    h3 { "从 cURL 导入接口" }
                    div {
                        class: "hint",
                        "粘贴浏览器「Copy as cURL」复制的命令，自动解析方法、URL、请求头、Body 与认证，并在当前位置创建接口。",
                    }
                    textarea {
                        class: "rf-textarea curl-input",
                        rows: "10",
                        placeholder: "curl -X POST https://api.example.com/users \\\n  -H \"Content-Type: application/json\" \\\n  -u user:pass \\\n  -d \"{{\"name\":\"test\"}}\"",
                        value: "{curl_input}",
                        oninput: move |e| {
                            let v = e.data().value();
                            let mut ci = curl_input;
                            ci.set(v);
                        },
                    }
                    div { class: "rf-modal-actions",
                        button { class: "rf-btn", onclick: move |_| {
                            let mut co = curl_open;
                            co.set(false);
                        }, "取消" }
                        button { class: "rf-btn rf-btn-primary", onclick: move |_| do_import(), "解析并导入" }
                    }
                }
            }
        }
    }
}

/// 文件夹节点（递归渲染子文件夹与接口）。
#[component]
pub fn FolderNode(
    folder: Folder,
    folders: Vec<Folder>,
    endpoints: Vec<Endpoint>,
    search: String,
    depth: usize,
) -> Element {
    let dispatcher = use_context::<Dispatcher>();
    let d1 = dispatcher.clone();
    let d2 = dispatcher.clone();
    let d3 = dispatcher.clone();
    let d4 = dispatcher.clone();
    let d5 = dispatcher.clone();
    let children: Vec<Folder> = folders
        .iter()
        .filter(|f| f.parent_id == Some(folder.id))
        .cloned()
        .collect();

    rsx! {
        div { class: "tree-item folder", style: "padding-left: {8 + depth * 16}px",
            span { class: "name", "▸ {folder.name}" }
            div { class: "tree-actions",
                button {
                    class: "rf-tree-action",
                    onclick: move |e| {
                        e.stop_propagation();
                        (d1.borrow_mut())(TreeAction::CreateEndpoint { folder_id: Some(folder.id) });
                    },
                    "接口"
                }
                button {
                    class: "rf-tree-action",
                    onclick: move |e| {
                        e.stop_propagation();
                        (d2.borrow_mut())(TreeAction::CreateFolder { parent_id: Some(folder.id) });
                    },
                    "子目录"
                }
                button {
                    class: "rf-tree-action",
                    onclick: move |e| {
                        e.stop_propagation();
                        (d3.borrow_mut())(TreeAction::ImportCurl { folder_id: Some(folder.id) });
                    },
                    "导入cURL"
                }
                button {
                    class: "rf-tree-action",
                    onclick: move |e| {
                        e.stop_propagation();
                        (d4.borrow_mut())(TreeAction::RenameFolder { id: folder.id, current: folder.name.clone() });
                    },
                    "改名"
                }
                button {
                    class: "rf-tree-action",
                    onclick: move |e| {
                        e.stop_propagation();
                        (d5.borrow_mut())(TreeAction::DeleteFolder { id: folder.id });
                    },
                    "删除"
                }
            }
        }
        for ep in endpoints.iter().filter(|e| e.folder_id == Some(folder.id)).cloned() {
            if search.is_empty() || ep.name.contains(&search) || ep.path.contains(&search) {
                EndpointRow { ep, depth: depth + 1 }
            }
        }
        for child in children {
            FolderNode {
                folder: child,
                folders: folders.clone(),
                endpoints: endpoints.clone(),
                search: search.clone(),
                depth: depth + 1,
            }
        }
    }
}

/// 接口行。
#[component]
pub fn EndpointRow(ep: Endpoint, depth: usize) -> Element {
    let state = use_context::<AppState>();
    let dispatcher = use_context::<Dispatcher>();
    let d1 = dispatcher.clone();
    let d2 = dispatcher.clone();
    let d3 = dispatcher.clone();
    let d4 = dispatcher.clone();
    let is_active = state
        .active_endpoint_id
        .read()
        .is_some_and(|id| id == ep.id);
    let method_cls = ep.method.as_str().to_lowercase();
    let path = ep.path.clone();

    rsx! {
        div {
            class: if is_active { "tree-item selected" } else { "tree-item" },
            style: "padding-left: {8 + depth * 16}px",
            onclick: move |_| state.open_endpoint_tab(ep.id),
            div { class: "rf-method rf-method-chip rf-method-chip-{method_cls}", "{ep.method}" }
            div { class: "name", title: "{path}", "{ep.name}" }
            div { class: "tree-actions",
                button {
                    class: "rf-tree-action",
                    id: "import-curl-{ep.id}",
                    onclick: move |e| {
                        e.stop_propagation();
                        (d1.borrow_mut())(TreeAction::ImportCurl { folder_id: ep.folder_id });
                    },
                    "导入cURL"
                }
                button {
                    class: "rf-tree-action",
                    onclick: move |e| {
                        e.stop_propagation();
                        (d2.borrow_mut())(TreeAction::DuplicateEndpoint { id: ep.id });
                    },
                    "复制"
                }
                button {
                    class: "rf-tree-action",
                    onclick: move |e| {
                        e.stop_propagation();
                        (d3.borrow_mut())(TreeAction::RenameEndpoint { id: ep.id, current: ep.name.clone() });
                    },
                    "改名"
                }
                button {
                    class: "rf-tree-action",
                    onclick: move |e| {
                        e.stop_propagation();
                        (d4.borrow_mut())(TreeAction::DeleteEndpoint { id: ep.id });
                    },
                    "删除"
                }
            }
        }
    }
}
