//! 左侧侧边栏：Header（项目选择器）| Toolbar（＋文件夹 / ＋接口）| Search | Tree。

use std::cell::RefCell;
use std::rc::Rc;

use dioxus::events::eval;
use dioxus::prelude::*;
use fox_codegen::Lang;
use fox_core::curl_parser::parse_curl;
use fox_core::model::{Endpoint, Folder};
use serde::Deserialize;
use uuid::Uuid;

use crate::components::confirm_dialog::{ConfirmDialog, ConfirmInfo};
use crate::components::dropdown::Dropdown;
use crate::components::icons::{
    CaretIcon, CopyIcon, FolderIcon, PencilIcon, PlusIcon, SearchIcon, TrashIcon, TreeFolderIcon,
};
use crate::components::modal::RFModal;
use crate::pages::workspace::build_codegen_code;
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
}

/// 树操作分发器。
pub type Dispatcher = Rc<RefCell<dyn FnMut(TreeAction)>>;

/// JS 拖放回调消息。
#[derive(Debug, Deserialize)]
struct DropMessage {
    ep_id: String,
    folder_id: Option<String>,
}

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

    let modal: Signal<Option<TreeAction>> = use_signal(|| None);
    let mut modal_input: Signal<String> = use_signal(String::new);
    // Toolbar「＋ 接口」下拉（HTTP 接口 / 从 cURL 导入）。
    let mut add_menu_open: Signal<bool> = use_signal(|| false);
    // 导入 cURL 弹窗。
    let curl_open: Signal<bool> = use_signal(|| false);
    let curl_target: Signal<Option<Uuid>> = use_signal(|| None);
    let curl_input: Signal<String> = use_signal(String::new);
    // 删除二次确认弹窗（保存待删除的动作，确认后才真正执行）。
    let confirm: Signal<Option<(ConfirmInfo, TreeAction)>> = use_signal(|| None);

    let dispatcher: Dispatcher = Rc::new(RefCell::new({
        let mut modal_sig = modal;
        let mut input = modal_input;
        let mut co = curl_open;
        let mut target = curl_target;
        let mut ci = curl_input;
        let mut confirm_sig = confirm;
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
            TreeAction::DeleteFolder { id } => confirm_sig.set(Some((
                ConfirmInfo::new(
                    "删除文件夹",
                    "确定要删除该文件夹吗？其下的所有子文件夹和接口将一并删除，且不可恢复。",
                ),
                TreeAction::DeleteFolder { id },
            ))),
            TreeAction::DeleteEndpoint { id } => confirm_sig.set(Some((
                ConfirmInfo::new("删除接口", "确定要删除该接口吗？此操作不可恢复。"),
                TreeAction::DeleteEndpoint { id },
            ))),
        }
    }));
    let dispatcher_provided = dispatcher.clone();
    use_context_provider(move || dispatcher_provided);
    let top_btn_a = dispatcher.clone();
    let top_btn_b = dispatcher.clone();
    let top_btn_c = dispatcher.clone();

    // 拖放：通过 JS 注入全局 drag/drop 监听，drop 事件通过 eval channel 回传 Rust。
    {
        let st = state.clone();
        let dragger_init = use_hook(|| std::cell::Cell::new(false));
        use_effect(move || {
            if dragger_init.get() {
                return;
            }
            dragger_init.set(true);
            let js = r#"
console.log('fox-drag-js-init');
var foxDrag = window.__foxDrag = window.__foxDrag || {
    epId: null, dragging: false, moved: false, startX: 0, startY: 0, ghost: null
};

function makeGhost(el) {
    var name = el.querySelector('.name');
    var g = document.createElement('div');
    g.className = 'fox-drag-ghost';
    g.textContent = name ? name.textContent.trim() : el.dataset.foxEpId;
    g.style.position = 'fixed';
    g.style.pointerEvents = 'none';
    g.style.zIndex = '10000';
    g.style.opacity = '0.85';
    g.style.background = '#1e40af';
    g.style.color = '#fff';
    g.style.padding = '4px 10px';
    g.style.borderRadius = '6px';
    g.style.fontSize = '12px';
    g.style.fontWeight = '600';
    g.style.boxShadow = '0 4px 12px rgba(0,0,0,0.4)';
    g.style.transform = 'translate(-50%, -50%)';
    g.style.whiteSpace = 'nowrap';
    document.body.appendChild(g);
    return g;
}

function clearDragging() {
    document.querySelectorAll('.tree-item.dragging').forEach(function(el){el.classList.remove('dragging')});
}

function doStart(e) {
    if (e.button !== 0) return;
    if (e.target.tagName === 'BUTTON' || e.target.closest('button')) return;
    if (e.target.tagName === 'INPUT' || e.target.tagName === 'TEXTAREA') return;
    var epEl = e.target.closest('[data-fox-ep-id]');
    if (!epEl) return;
    foxDrag.epId = epEl.dataset.foxEpId;
    foxDrag.startX = e.clientX;
    foxDrag.startY = e.clientY;
    foxDrag.moved = false;
    foxDrag.dragging = false;
    epEl.classList.add('dragging');
}

function doMove(e) {
    if (!foxDrag.epId) return;
    var dx = Math.abs(e.clientX - foxDrag.startX);
    var dy = Math.abs(e.clientY - foxDrag.startY);
    if (dx < 8 && dy < 8) return;
    foxDrag.moved = true;
    foxDrag.dragging = true;
    if (!foxDrag.ghost) {
        foxDrag.ghost = makeGhost(document.querySelector('[data-fox-ep-id="' + foxDrag.epId + '"]'));
    }
    foxDrag.ghost.style.left = e.clientX + 'px';
    foxDrag.ghost.style.top = e.clientY + 'px';
    var target = e.target.closest('[data-fox-drop-target]');
    document.querySelectorAll('[data-fox-drop-target].drop-over').forEach(function(el){el.classList.remove('drop-over')});
    if (target) {
        target.classList.add('drop-over');
    }
}

function doEnd(e) {
    if (!foxDrag.epId) return;
    var epId = foxDrag.epId;
    var target = null;
    if (foxDrag.moved && foxDrag.dragging) {
        target = e.target.closest('[data-fox-drop-target]');
    }
    var folderId = target ? target.dataset.foxFolderId : null;
    foxDrag.epId = null;
    foxDrag.dragging = false;
    foxDrag.moved = false;
    document.querySelectorAll('[data-fox-drop-target].drop-over').forEach(function(el){el.classList.remove('drop-over')});
    clearDragging();
    if (foxDrag.ghost) { foxDrag.ghost.remove(); foxDrag.ghost = null; }
    if (target) {
        foxDrag.noClick = true;
        console.log('fox-drop:', epId, folderId);
        dioxus.send({ep_id: epId, folder_id: folderId});
    }
}

document.addEventListener('mousedown', doStart);
document.addEventListener('mousemove', doMove);
document.addEventListener('mouseup', doEnd);
document.addEventListener('click', function(e) {
    if (foxDrag.noClick) {
        foxDrag.noClick = false;
        e.preventDefault();
        e.stopImmediatePropagation();
        return false;
    }
}, true);
console.log('fox-drag-js-done');
"#;
            let mut handle = eval(js);
            let st_spawn = st.clone();
            // 使用 spawn_forever：任务绑定根 scope，不会被组件 re-render 丢弃。
            spawn_forever(async move {
                loop {
                    match handle.recv().await {
                        Ok(v) => {
                            tracing::info!("fox-drag-recv: {}", serde_json::to_string(&v).unwrap_or_default());
                            let msg: DropMessage = match serde_json::from_value(v) {
                                Ok(m) => m,
                                Err(_) => continue,
                            };
                            if let Ok(ep_id) = Uuid::parse_str(&msg.ep_id) {
                                let folder_id: Option<Uuid> = msg
                                    .folder_id
                                    .and_then(|s| Uuid::parse_str(&s).ok());
                                st_spawn.move_endpoint_to_folder(ep_id, folder_id);
                            }
                        }
                        Err(_) => break,
                    }
                }
            });
        });
    }

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
    let st_del = state.clone();
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
                    class: "sb-toolbar-btn sb-toolbar-btn-ghost",
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
                        div {
                            class: "tree-root-drop-target",
                            "data-fox-drop-target": "",
                            for ep in endpoints.iter().filter(|e| e.folder_id.is_none()).cloned() {
                                if search.is_empty() || ep.name.contains(&search) || ep.path.contains(&search) {
                                    EndpointRow { ep, depth: 0 }
                                }
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
                            div { class: "rf-empty sb-tree-empty",
                                div { class: "rf-empty-icon", TreeFolderIcon {} }
                                div { class: "rf-empty-title", "还没有接口" }
                                div { class: "rf-empty-desc", "点击上方「+ 接口」创建第一个 API" }
                            }
                        }
                    }
            }
        }
        if dialog_visible {
            RFModal {
                on_close: move |_| cancel_a(),
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
        if curl_open_flag {
            RFModal {
                on_close: move |_| {
                    let mut co = curl_open;
                    co.set(false);
                },
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
        if let Some((info, _)) = confirm.read().as_ref() {
            ConfirmDialog {
                info: Some(info.clone()),
                on_confirm: move |_| {
                    if let Some((_, action)) = confirm.peek().as_ref() {
                        match action {
                            TreeAction::DeleteFolder { id } => st_del.delete_folder(*id),
                            TreeAction::DeleteEndpoint { id } => st_del.delete_endpoint(*id),
                            _ => {}
                        }
                    }
                    let mut c = confirm;
                    c.set(None);
                },
                on_cancel: move |_| {
                    let mut c = confirm;
                    c.set(None);
                },
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
    let mut expanded = use_signal(|| true);
    let children: Vec<Folder> = folders
        .iter()
        .filter(|f| f.parent_id == Some(folder.id))
        .cloned()
        .collect();
    let has_sub = !children.is_empty()
        || endpoints.iter().any(|e| e.folder_id == Some(folder.id));
    let f_id = folder.id;
    let folder_name = folder.name.clone();
    let expand_class = if has_sub { "tree-item folder expandable" } else { "tree-item folder" };
    let chevron = if has_sub {
        if *expanded.read() { "▾" } else { "▸" }
    } else { "  " };

    let is_expanded = *expanded.read();
    let fold_children = is_expanded;

    rsx! {
        div {
            class: expand_class,
            style: "padding-left: {8 + depth * 16}px",
            "data-fox-drop-target": "",
            "data-fox-folder-id": "{f_id}",
            onclick: move |_| {
                if fold_children && has_sub {
                    let mut e = expanded;
                    e.set(false);
                } else if has_sub {
                    let mut e = expanded;
                    e.set(true);
                }
            },
            span { class: "name", "{chevron} {folder_name}" }
            div { class: "tree-actions",
                button {
                    class: "rf-tree-action",
                    title: "新建接口",
                    onclick: move |e| {
                        e.stop_propagation();
                        (d1.borrow_mut())(TreeAction::CreateEndpoint { folder_id: Some(f_id) });
                    },
                    PlusIcon {}
                }
                button {
                    class: "rf-tree-action",
                    title: "新建子目录",
                    onclick: move |e| {
                        e.stop_propagation();
                        (d2.borrow_mut())(TreeAction::CreateFolder { parent_id: Some(f_id) });
                    },
                    FolderIcon {}
                }
                button {
                    class: "rf-tree-action",
                    title: "重命名",
                    onclick: move |e| {
                        e.stop_propagation();
                        (d3.borrow_mut())(TreeAction::RenameFolder { id: f_id, current: folder_name.clone() });
                    },
                    PencilIcon {}
                }
                button {
                    class: "rf-tree-action rf-tree-action-danger",
                    title: "删除",
                    onclick: move |e| {
                        e.stop_propagation();
                        (d4.borrow_mut())(TreeAction::DeleteFolder { id: f_id });
                    },
                    TrashIcon {}
                }
            }
        }
        if is_expanded {
            for ep in endpoints.iter().filter(|e| e.folder_id == Some(f_id)).cloned() {
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
}

/// 复制文本到剪贴板（webview 内 execCommand 兜底，兼容非安全上下文）。
fn copy_text(text: &str) {
    let quoted = serde_json::to_string(text).unwrap_or_else(|_| "\"\"".into());
    eval(&format!(
        "(function(){{var ta=document.createElement('textarea');ta.value={quoted};ta.style.position='fixed';ta.style.opacity='0';document.body.appendChild(ta);ta.focus();ta.select();try{{document.execCommand('copy');}}catch(e){{}}document.body.removeChild(ta);}})();"
    ));
}

/// 接口行。
#[component]
pub fn EndpointRow(ep: Endpoint, depth: usize) -> Element {
    let state = use_context::<AppState>();
    let dispatcher = use_context::<Dispatcher>();
    let d_rename = dispatcher.clone();
    let d_delete = dispatcher.clone();
    let is_active = state
        .active_endpoint_id
        .read()
        .is_some_and(|id| id == ep.id);
    let method_cls = ep.method.as_str().to_lowercase();
    let path = ep.path.clone();
    let ep_id = ep.id;
    let ep_name = ep.name.clone();
    let ep_for_curl = ep.clone();
    let st_curl = state.clone();

    rsx! {
        div {
            class: if is_active { "tree-item selected draggable" } else { "tree-item draggable" },
            style: "padding-left: {8 + depth * 16}px",
            "data-fox-ep-id": "{ep_id}",
            onclick: move |_| state.open_endpoint_tab(ep.id),
            div { class: "rf-method rf-method-chip rf-method-chip-{method_cls}", "{ep.method}" }
            div { class: "name", title: "{path}", "{ep.name}" }
            div { class: "tree-actions",
                button {
                    class: "rf-tree-action",
                    title: "重命名",
                    onclick: move |e| {
                        e.stop_propagation();
                        (d_rename.borrow_mut())(TreeAction::RenameEndpoint { id: ep_id, current: ep_name.clone() });
                    },
                    PencilIcon {}
                }
                button {
                    class: "rf-tree-action",
                    title: "复制 Curl",
                    onclick: move |e| {
                        e.stop_propagation();
                        if let Some(code) = build_codegen_code(&st_curl, &ep_for_curl, Lang::Curl) {
                            copy_text(&code);
                            st_curl.toast_success("cURL 命令已复制到剪贴板");
                        }
                    },
                    CopyIcon {}
                }
                button {
                    class: "rf-tree-action rf-tree-action-danger",
                    title: "删除",
                    onclick: move |e| {
                        e.stop_propagation();
                        (d_delete.borrow_mut())(TreeAction::DeleteEndpoint { id: ep_id });
                    },
                    TrashIcon {}
                }
            }
        }
    }
}
