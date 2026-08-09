//! 应用全局状态。

use chrono::Utc;
use dioxus::prelude::*;
use fox_core::curl_parser::CurlParsed;
use fox_core::model::*;
use fox_storage::repository as repo;
use std::collections::HashMap;
use uuid::Uuid;

use std::sync::Arc;

use crate::services::Services;

/// 内部导航页面。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Home,
    Workspace,
    Settings,
}

/// Toast 类型。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ToastKind {
    Success,
    Error,
    Info,
}

impl ToastKind {
    pub fn css_class(&self) -> &'static str {
        match self {
            ToastKind::Success => "rf-toast-success",
            ToastKind::Error => "rf-toast-error",
            ToastKind::Info => "rf-toast-info",
        }
    }
}

/// Toast 消息。
#[derive(Debug, Clone, PartialEq)]
pub struct Toast {
    pub id: u64,
    pub kind: ToastKind,
    pub message: String,
}

/// 全局应用状态。所有字段均为 Signal，克隆成本极低。
#[derive(Clone)]
pub struct AppState {
    pub services: Services,
    pub current_project_id: Signal<Option<Uuid>>,
    pub current_environment_id: Signal<Option<Uuid>>,
    pub current_page: Signal<Page>,
    pub open_tabs: Signal<Vec<Uuid>>,
    pub active_endpoint_id: Signal<Option<Uuid>>,
    pub projects: Signal<Vec<Project>>,
    pub folders: Signal<Vec<Folder>>,
    pub endpoints: Signal<Vec<Endpoint>>,
    pub environments: Signal<Vec<Environment>>,
    pub toasts: Signal<Vec<Toast>>,
    pub search: Signal<String>,
    pub mock_rules: Signal<Vec<MockRule>>,
    /// 运行中的 Mock 服务句柄（None 表示未启动）。
    pub mock_handle: Signal<Option<Arc<tokio::sync::Mutex<Option<fox_mock::server::MockServer>>>>>,
    pub mock_port: Signal<Option<u16>>,
    /// 最近用户操作步骤（问题反馈报告用），最多保留 60 条。
    pub steps: Signal<Vec<String>>,
}

/// 操作步骤上限。
const MAX_STEPS: usize = 60;

static TOAST_ID: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);

impl AppState {
    pub fn new(services: Services) -> Self {
        AppState {
            services,
            current_project_id: Signal::new(None),
            current_environment_id: Signal::new(None),
            current_page: Signal::new(Page::Home),
            open_tabs: Signal::new(Vec::new()),
            active_endpoint_id: Signal::new(None),
            projects: Signal::new(Vec::new()),
            folders: Signal::new(Vec::new()),
            endpoints: Signal::new(Vec::new()),
            environments: Signal::new(Vec::new()),
            toasts: Signal::new(Vec::new()),
            search: Signal::new(String::new()),
            mock_rules: Signal::new(Vec::new()),
            mock_handle: Signal::new(None),
            mock_port: Signal::new(None),
            steps: Signal::new(Vec::new()),
        }
    }

    /// 记录一条用户操作步骤（保留最近 MAX_STEPS 条）。
    pub fn record_step(&self, message: impl Into<String>) {
        push_step(self.steps, message.into());
    }

    pub fn set_current_project(&self, id: Option<Uuid>) {
        let mut c = self.current_project_id;
        c.set(id);
    }

    /// 弹出失败 Toast（中文消息）。
    pub fn toast_error(&self, message: impl Into<String>) {
        self.push_toast(ToastKind::Error, message.into());
    }

    /// 弹出成功 Toast。
    pub fn toast_success(&self, message: impl Into<String>) {
        self.push_toast(ToastKind::Success, message.into());
    }

    /// 弹出信息 Toast。
    pub fn toast_info(&self, message: impl Into<String>) {
        self.push_toast(ToastKind::Info, message.into());
    }

    fn push_toast(&self, kind: ToastKind, message: String) -> Toast {
        let id = TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let mut toasts = self.toasts;
        toasts.write().push(Toast {
            id,
            kind,
            message: message.clone(),
        });
        let toast = Toast { id, kind, message };
        // 4 秒后自动消失。
        let mut toasts_sink = toasts;
        spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(4)).await;
            toasts_sink.write().retain(|t| t.id != id);
        });
        toast
    }

    /// 刷新项目列表。
    pub fn refresh_projects(&self) {
        let db = self.services.db.clone();
        let mut projects = self.projects;
        let mut toasts = self.toasts;
        spawn(async move {
            match repo::list_projects(&db).await {
                Ok(list) => projects.set(list),
                Err(e) => toasts.write().push(Toast {
                    id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                    kind: ToastKind::Error,
                    message: format!("加载项目失败：{}", e.user_message()),
                }),
            }
        });
    }

    /// 创建项目。返回 Err 为中文提示；成功异步写入列表并提示。
    pub fn create_project(&self, name: String, description: String) -> Result<(), String> {
        let name = name.trim();
        if name.is_empty() {
            return Err("项目名称不能为空".into());
        }
        let db = self.services.db.clone();
        let mut projects = self.projects;
        let mut toasts = self.toasts;
        let name = name.to_string();
        spawn(async move {
            match repo::create_project(&db, &name, &description).await {
                Ok(p) => {
                    projects.write().push(p);
                    toasts.write().push(Toast {
                        id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                        kind: ToastKind::Success,
                        message: "项目创建成功".into(),
                    });
                }
                Err(e) => toasts.write().push(Toast {
                    id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                    kind: ToastKind::Error,
                    message: format!("创建项目失败：{}", e.user_message()),
                }),
            }
        });
        Ok(())
    }

    /// 删除项目（异步）。
    pub fn delete_project(&self, project_id: Uuid) {
        let db = self.services.db.clone();
        let mut projects = self.projects;
        let mut current = self.current_project_id;
        let mut page = self.current_page;
        let mut toasts = self.toasts;
        spawn(async move {
            match repo::delete_project(&db, project_id).await {
                Ok(()) => {
                    projects.write().retain(|p| p.id != project_id);
                    if current.read().is_some_and(|id| id == project_id) {
                        current.set(None);
                        page.set(Page::Home);
                    }
                    toasts.write().push(Toast {
                        id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                        kind: ToastKind::Success,
                        message: "项目已删除".into(),
                    });
                }
                Err(e) => toasts.write().push(Toast {
                    id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                    kind: ToastKind::Error,
                    message: format!("删除项目失败：{}", e.user_message()),
                }),
            }
        });
    }

    /// 选择并进入项目。
    pub fn select_project(&self, project_id: Uuid) {
        let mut page = self.current_page;
        self.set_current_project(Some(project_id));
        page.set(Page::Workspace);
        self.refresh_project_data(project_id);
    }

    /// 加载当前项目数据（文件夹、接口、环境）。
    pub fn refresh_project_data(&self, project_id: Uuid) {
        let db = self.services.db.clone();
        let mut folders = self.folders;
        let mut endpoints = self.endpoints;
        let mut environments = self.environments;
        let mut mock_rules = self.mock_rules;
        let mut toasts = self.toasts;
        spawn(async move {
            match repo::list_folders(&db, project_id).await {
                Ok(list) => folders.set(list),
                Err(e) => toasts.write().push(Toast {
                    id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                    kind: ToastKind::Error,
                    message: format!("加载文件夹失败：{}", e.user_message()),
                }),
            }
            match repo::list_endpoints(&db, project_id).await {
                Ok(list) => endpoints.set(list),
                Err(e) => toasts.write().push(Toast {
                    id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                    kind: ToastKind::Error,
                    message: format!("加载接口失败：{}", e.user_message()),
                }),
            }
            match repo::list_environments(&db, project_id).await {
                Ok(list) => environments.set(list),
                Err(e) => toasts.write().push(Toast {
                    id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                    kind: ToastKind::Error,
                    message: format!("加载环境失败：{}", e.user_message()),
                }),
            }
            match repo::list_mock_rules(&db, project_id).await {
                Ok(list) => mock_rules.set(list),
                Err(e) => toasts.write().push(Toast {
                    id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                    kind: ToastKind::Error,
                    message: format!("加载 Mock 规则失败：{}", e.user_message()),
                }),
            }
        });
    }

    fn current_project(&self) -> Option<Uuid> {
        *self.current_project_id.peek()
    }

    /// 新建文件夹（parent_id 为 None 表示根目录）。
    pub fn create_folder_at(&self, parent_id: Option<Uuid>, name: String) {
        let Some(project_id) = self.current_project() else {
            self.toast_error("未选择项目");
            return;
        };
        let db = self.services.db.clone();
        let mut folders = self.folders;
        let mut toasts = self.toasts;
        spawn(async move {
            match repo::create_folder(&db, project_id, parent_id, &name).await {
                Ok(folder) => {
                    folders.write().push(folder);
                    toasts.write().push(Toast {
                        id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                        kind: ToastKind::Success,
                        message: "文件夹创建成功".into(),
                    });
                }
                Err(e) => toasts.write().push(Toast {
                    id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                    kind: ToastKind::Error,
                    message: format!("创建文件夹失败：{}", e.user_message()),
                }),
            }
        });
    }

    /// 新建接口。
    pub fn create_endpoint_at(&self, folder_id: Option<Uuid>, name: String) {
        let Some(project_id) = self.current_project() else {
            self.toast_error("未选择项目");
            return;
        };
        let db = self.services.db.clone();
        let mut endpoints = self.endpoints;
        let mut toasts = self.toasts;
        spawn(async move {
            match repo::create_endpoint(&db, project_id, folder_id, &name).await {
                Ok(ep) => {
                    endpoints.write().push(ep);
                    toasts.write().push(Toast {
                        id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                        kind: ToastKind::Success,
                        message: "接口创建成功".into(),
                    });
                }
                Err(e) => toasts.write().push(Toast {
                    id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                    kind: ToastKind::Error,
                    message: format!("创建接口失败：{}", e.user_message()),
                }),
            }
        });
    }

    /// 从 cURL 解析结果创建接口（含方法、路径、请求头、Body、认证），并打开标签。
    pub fn create_endpoint_from_curl(&self, folder_id: Option<Uuid>, parsed: &CurlParsed) {
        let Some(project_id) = self.current_project() else {
            self.toast_error("未选择项目");
            return;
        };
        let now = Utc::now();
        let name = parsed
            .url
            .trim_end_matches('/')
            .rsplit('/')
            .find(|s| !s.is_empty())
            .unwrap_or("导入接口")
            .to_string();
        let model = Endpoint {
            id: Uuid::new_v4(),
            project_id,
            folder_id,
            name,
            method: parsed.method,
            path: parsed.url.clone(),
            description: String::new(),
            status: EndpointStatus::Developing,
            sort_order: 0,
            request: RequestSpec {
                headers: parsed.headers.clone(),
                body: parsed.body.clone().unwrap_or(BodySpec::None),
                auth: parsed.auth.clone(),
                ..RequestSpec::default()
            },
            created_at: now,
            updated_at: now,
        };
        let db = self.services.db.clone();
        let mut endpoints = self.endpoints;
        let mut toasts = self.toasts;
        let st = self.clone();
        spawn(async move {
            match repo::save_endpoint(&db, &model).await {
                Ok(()) => {
                    endpoints.write().push(model.clone());
                    st.open_endpoint_tab(model.id);
                    toasts.write().push(Toast {
                        id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                        kind: ToastKind::Success,
                        message: "cURL 导入成功".into(),
                    });
                }
                Err(e) => toasts.write().push(Toast {
                    id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                    kind: ToastKind::Error,
                    message: format!("导入失败：{}", e.user_message()),
                }),
            }
        });
    }

    /// 重命名文件夹。
    pub fn rename_folder(&self, folder_id: Uuid, name: String) {
        let db = self.services.db.clone();
        let mut folders = self.folders;
        let mut toasts = self.toasts;
        spawn(async move {
            match repo::get_folder(&db, folder_id).await {
                Ok(mut folder) => {
                    folder.name = name;
                    match repo::update_folder(&db, &folder).await {
                        Ok(updated) => {
                            let mut list = folders.write();
                            if let Some(f) = list.iter_mut().find(|f| f.id == folder_id) {
                                *f = updated;
                            }
                            toasts.write().push(Toast {
                                id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                                kind: ToastKind::Success,
                                message: "文件夹已重命名".into(),
                            });
                        }
                        Err(e) => toasts.write().push(Toast {
                            id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                            kind: ToastKind::Error,
                            message: format!("重命名失败：{}", e.user_message()),
                        }),
                    }
                }
                Err(e) => toasts.write().push(Toast {
                    id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                    kind: ToastKind::Error,
                    message: format!("重命名失败：{}", e.user_message()),
                }),
            }
        });
    }

    /// 重命名接口。
    pub fn rename_endpoint(&self, endpoint_id: Uuid, name: String) {
        let db = self.services.db.clone();
        let mut endpoints = self.endpoints;
        let mut toasts = self.toasts;
        spawn(async move {
            match repo::get_endpoint(&db, endpoint_id).await {
                Ok(mut ep) => {
                    ep.name = name;
                    match repo::update_endpoint(&db, &ep).await {
                        Ok(updated) => {
                            let mut list = endpoints.write();
                            if let Some(e) = list.iter_mut().find(|e| e.id == endpoint_id) {
                                *e = updated;
                            }
                            toasts.write().push(Toast {
                                id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                                kind: ToastKind::Success,
                                message: "接口已重命名".into(),
                            });
                        }
                        Err(e) => toasts.write().push(Toast {
                            id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                            kind: ToastKind::Error,
                            message: format!("重命名失败：{}", e.user_message()),
                        }),
                    }
                }
                Err(e) => toasts.write().push(Toast {
                    id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                    kind: ToastKind::Error,
                    message: format!("重命名失败：{}", e.user_message()),
                }),
            }
        });
    }

    /// 删除文件夹。
    pub fn delete_folder(&self, folder_id: Uuid) {
        let db = self.services.db.clone();
        let mut folders = self.folders;
        let mut toasts = self.toasts;
        spawn(async move {
            match repo::delete_folder(&db, folder_id).await {
                Ok(()) => {
                    folders.write().retain(|f| f.id != folder_id);
                    toasts.write().push(Toast {
                        id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                        kind: ToastKind::Success,
                        message: "文件夹已删除".into(),
                    });
                }
                Err(e) => toasts.write().push(Toast {
                    id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                    kind: ToastKind::Error,
                    message: format!("删除文件夹失败：{}", e.user_message()),
                }),
            }
        });
    }

    /// 删除接口。
    pub fn delete_endpoint(&self, endpoint_id: Uuid) {
        let db = self.services.db.clone();
        let mut endpoints = self.endpoints;
        let mut active = self.active_endpoint_id;
        let mut open_tabs = self.open_tabs;
        let mut toasts = self.toasts;
        spawn(async move {
            match repo::delete_endpoint(&db, endpoint_id).await {
                Ok(()) => {
                    endpoints.write().retain(|e| e.id != endpoint_id);
                    if active.read().is_some_and(|id| id == endpoint_id) {
                        active.set(None);
                    }
                    open_tabs.write().retain(|id| *id != endpoint_id);
                    toasts.write().push(Toast {
                        id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                        kind: ToastKind::Success,
                        message: "接口已删除".into(),
                    });
                }
                Err(e) => toasts.write().push(Toast {
                    id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                    kind: ToastKind::Error,
                    message: format!("删除接口失败：{}", e.user_message()),
                }),
            }
        });
    }

    /// 复制接口。
    pub fn duplicate_endpoint(&self, endpoint_id: Uuid) {
        let db = self.services.db.clone();
        let mut endpoints = self.endpoints;
        let mut toasts = self.toasts;
        spawn(async move {
            match repo::duplicate_endpoint(&db, endpoint_id).await {
                Ok(ep) => {
                    endpoints.write().push(ep);
                    toasts.write().push(Toast {
                        id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                        kind: ToastKind::Success,
                        message: "接口已复制".into(),
                    });
                }
                Err(e) => toasts.write().push(Toast {
                    id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                    kind: ToastKind::Error,
                    message: format!("复制接口失败：{}", e.user_message()),
                }),
            }
        });
    }

    /// 打开接口 Tab 并进入工作区。
    pub fn open_endpoint_tab(&self, endpoint_id: Uuid) {
        let mut page = self.current_page;
        let mut active = self.active_endpoint_id;
        let mut tabs = self.open_tabs;
        active.set(Some(endpoint_id));
        page.set(Page::Workspace);
        if !tabs.peek().contains(&endpoint_id) {
            tabs.write().push(endpoint_id);
        }
    }

    /// 保存接口（更新数据库并刷新列表）。
    pub fn save_endpoint(&self, ep: Endpoint) {
        let db = self.services.db.clone();
        let mut endpoints = self.endpoints;
        let mut toasts = self.toasts;
        spawn(async move {
            match repo::update_endpoint(&db, &ep).await {
                Ok(updated) => {
                    let mut list = endpoints.write();
                    if let Some(e) = list.iter_mut().find(|e| e.id == updated.id) {
                        *e = updated;
                    }
                    toasts.write().push(Toast {
                        id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                        kind: ToastKind::Success,
                        message: "保存成功".into(),
                    });
                }
                Err(e) => toasts.write().push(Toast {
                    id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                    kind: ToastKind::Error,
                    message: format!("保存失败：{}", e.user_message()),
                }),
            }
        });
    }

    /// 新建环境并设为当前环境。
    pub fn create_environment(&self, name: String) {
        let Some(project_id) = self.current_project() else {
            self.toast_error("未选择项目");
            return;
        };
        let db = self.services.db.clone();
        let mut environments = self.environments;
        let mut current = self.current_environment_id;
        let mut toasts = self.toasts;
        spawn(async move {
            match repo::create_environment(&db, project_id, &name).await {
                Ok(env) => {
                    environments.write().push(env.clone());
                    current.set(Some(env.id));
                    toasts.write().push(Toast {
                        id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                        kind: ToastKind::Success,
                        message: format!("环境「{}」已创建", env.name),
                    });
                }
                Err(e) => toasts.write().push(Toast {
                    id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                    kind: ToastKind::Error,
                    message: format!("创建环境失败：{}", e.user_message()),
                }),
            }
        });
    }

    /// 删除环境。
    pub fn delete_environment(&self, environment_id: Uuid) {
        let db = self.services.db.clone();
        let mut environments = self.environments;
        let mut current = self.current_environment_id;
        let mut toasts = self.toasts;
        spawn(async move {
            match repo::delete_environment(&db, environment_id).await {
                Ok(()) => {
                    environments.write().retain(|e| e.id != environment_id);
                    if current.peek().is_some_and(|id| id == environment_id) {
                        current.set(None);
                    }
                    toasts.write().push(Toast {
                        id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                        kind: ToastKind::Success,
                        message: "环境已删除".into(),
                    });
                }
                Err(e) => toasts.write().push(Toast {
                    id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                    kind: ToastKind::Error,
                    message: format!("删除环境失败：{}", e.user_message()),
                }),
            }
        });
    }

    /// 保存环境（名称 + 变量）。
    pub fn save_environment(&self, environment: Environment) {
        let db = self.services.db.clone();
        let mut environments = self.environments;
        let mut toasts = self.toasts;
        spawn(async move {
            match repo::update_environment(&db, &environment).await {
                Ok(updated) => {
                    let mut list = environments.write();
                    if let Some(e) = list.iter_mut().find(|e| e.id == updated.id) {
                        *e = updated;
                    }
                    toasts.write().push(Toast {
                        id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                        kind: ToastKind::Success,
                        message: "环境已保存".into(),
                    });
                }
                Err(e) => toasts.write().push(Toast {
                    id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                    kind: ToastKind::Error,
                    message: format!("保存环境失败：{}", e.user_message()),
                }),
            }
        });
    }

    /// 选择当前环境（None 表示不使用环境变量）。
    pub fn select_environment(&self, id: Option<Uuid>) {
        let mut current = self.current_environment_id;
        current.set(id);
    }

    /// 保存项目变量。
    pub fn save_project_variables(&self, variables: HashMap<String, String>) {
        let Some(project_id) = self.current_project() else {
            self.toast_error("未选择项目");
            return;
        };
        let Some(mut project) = self
            .projects
            .read()
            .iter()
            .find(|p| p.id == project_id)
            .cloned()
        else {
            self.toast_error("项目不存在");
            return;
        };
        project.variables = variables;
        let db = self.services.db.clone();
        let mut projects = self.projects;
        let mut toasts = self.toasts;
        spawn(async move {
            match repo::update_project(&db, &project).await {
                Ok(updated) => {
                    let mut list = projects.write();
                    if let Some(p) = list.iter_mut().find(|p| p.id == updated.id) {
                        *p = updated;
                    }
                    toasts.write().push(Toast {
                        id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                        kind: ToastKind::Success,
                        message: "项目变量已保存".into(),
                    });
                }
                Err(e) => toasts.write().push(Toast {
                    id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                    kind: ToastKind::Error,
                    message: format!("保存项目变量失败：{}", e.user_message()),
                }),
            }
        });
    }

    /// 导入 OpenAPI 文档。text 为 JSON/YAML 内容；返回中文错误或成功提示。
    pub fn import_openapi(&self, text: String, strategy: fox_openapi::import::ConflictStrategy) {
        let Some(project_id) = self.current_project() else {
            self.toast_error("未选择项目");
            return;
        };
        let (imported, format) = match fox_openapi::import::import_any(&text) {
            Ok(v) => v,
            Err(e) => {
                self.toast_error(e.user_message());
                return;
            }
        };
        if imported.is_empty() {
            self.toast_info("文档中没有可导入的接口");
            return;
        }
        let existing = self.endpoints.read().clone();
        let db = self.services.db.clone();
        let mut endpoints = self.endpoints;
        let mut toasts = self.toasts;
        let steps = self.steps;
        let count = imported.len();
        spawn(async move {
            let mut created = 0usize;
            let mut updated_count = 0usize;
            let mut skipped = 0usize;
            // M12：按 folder_hint 建一级文件夹（Postman 分组 / OpenAPI tags）。
            let mut folder_cache: HashMap<String, Uuid> = HashMap::new();
            for item in &imported {
                let mut folder_id = None;
                if let Some(hint) = item.folder_hint.as_ref() {
                    if let Some(cached) = folder_cache.get(hint) {
                        folder_id = Some(*cached);
                    } else {
                        if let Ok(f) = repo::create_folder(&db, project_id, None, hint).await {
                            folder_id = Some(f.id);
                            folder_cache.insert(hint.clone(), f.id);
                        }
                    }
                }
                let conflict = existing
                    .iter()
                    .find(|e| e.method == item.method && e.path == item.path)
                    .cloned();
                let has_conflict = conflict.is_some();
                match (conflict, strategy) {
                    (Some(_), fox_openapi::import::ConflictStrategy::Skip) => {
                        skipped += 1;
                        continue;
                    }
                    (Some(mut ep), fox_openapi::import::ConflictStrategy::Overwrite) => {
                        ep.name = item.name.clone();
                        ep.description = item.description.clone();
                        ep.request = item.request.clone();
                        ep.updated_at = Utc::now();
                        match repo::update_endpoint(&db, &ep).await {
                            Ok(updated) => {
                                let _ = repo::delete_response_examples(&db, updated.id).await;
                                insert_examples(&db, updated.id, &item.examples).await;
                                let mut list = endpoints.write();
                                if let Some(e) = list.iter_mut().find(|e| e.id == updated.id) {
                                    *e = updated;
                                }
                                updated_count += 1;
                            }
                            Err(e) => toasts.write().push(Toast {
                                id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                                kind: ToastKind::Error,
                                message: format!("覆盖接口失败：{}", e.user_message()),
                            }),
                        }
                    }
                    _ => {
                        let name = if has_conflict {
                            format!("{} (副本)", item.name)
                        } else {
                            item.name.clone()
                        };
                        match repo::create_endpoint(&db, project_id, folder_id, &name).await {
                            Ok(mut ep) => {
                                ep.method = item.method;
                                ep.path = item.path.clone();
                                ep.description = item.description.clone();
                                ep.request = item.request.clone();
                                match repo::update_endpoint(&db, &ep).await {
                                    Ok(updated) => {
                                        insert_examples(&db, updated.id, &item.examples).await;
                                        endpoints.write().push(updated);
                                        created += 1;
                                    }
                                    Err(e) => toasts.write().push(Toast {
                                        id: TOAST_ID
                                            .fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                                        kind: ToastKind::Error,
                                        message: format!("保存导入接口失败：{}", e.user_message()),
                                    }),
                                }
                            }
                            Err(e) => toasts.write().push(Toast {
                                id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                                kind: ToastKind::Error,
                                message: format!("创建接口失败：{}", e.user_message()),
                            }),
                        }
                    }
                }
            }
            toasts.write().push(Toast {
                id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                kind: ToastKind::Success,
                message: format!("导入完成（{}）：新建 {created}，覆盖 {updated_count}，跳过 {skipped}（共 {count} 个接口）", format.label()),
            });
            push_step(
                steps,
                format!(
                    "导入 OpenAPI（{}）：新建 {created}/覆盖 {updated_count}/跳过 {skipped}",
                    format.label()
                ),
            );
        });
    }

    /// 导出当前项目为 OpenAPI 3.0 JSON 文本（返回中文错误信息或 OK(json)）。
    pub fn export_openapi(&self, f: impl Fn(Result<String, String>) + 'static) {
        let Some(project_id) = self.current_project() else {
            self.toast_error("未选择项目");
            return;
        };
        let project_name = self
            .projects
            .read()
            .iter()
            .find(|p| p.id == project_id)
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "未命名项目".into());
        let eps = self.endpoints.read().clone();
        let db = self.services.db.clone();
        let steps = self.steps;
        spawn(async move {
            let mut examples: HashMap<Uuid, Vec<ResponseExample>> = HashMap::new();
            for ep in &eps {
                match repo::list_response_examples(&db, ep.id).await {
                    Ok(list) => {
                        examples.insert(ep.id, list);
                    }
                    Err(e) => {
                        f(Err(format!("加载响应示例失败：{}", e.user_message())));
                        return;
                    }
                }
            }
            push_step(steps, format!("导出 OpenAPI 文档（{} 个接口）", eps.len()));
            match fox_openapi::export::export_project(&project_name, &eps, &examples) {
                Ok(json) => f(Ok(json)),
                Err(e) => f(Err(e.user_message())),
            }
        });
    }

    /// 是否正在运行 Mock 服务。
    pub fn mock_running(&self) -> bool {
        self.mock_port.peek().is_some()
    }

    /// 启动 Mock 服务：收集当前项目的接口/示例/规则 → 构建定义 → 绑定端口启动。
    pub fn start_mock(&self) {
        if self.current_project().is_none() {
            self.toast_error("未选择项目");
            return;
        }
        if self.mock_running() {
            self.toast_info("Mock 服务已在运行");
            return;
        }
        let eps = self.endpoints.read().clone();
        let rules = self.mock_rules.read().clone();
        let db = self.services.db.clone();
        let mut port = self.mock_port;
        let mut handle = self.mock_handle;
        let mut toasts = self.toasts;
        let steps = self.steps;
        spawn(async move {
            // 1. 加载响应示例。
            let mut examples_by_ep: HashMap<Uuid, Vec<ResponseExample>> = HashMap::new();
            for ep in &eps {
                if ep.status == fox_core::model::EndpointStatus::Deprecated {
                    continue;
                }
                match repo::list_response_examples(&db, ep.id).await {
                    Ok(list) => {
                        examples_by_ep.insert(ep.id, list);
                    }
                    Err(e) => {
                        toasts.write().push(Toast {
                            id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                            kind: ToastKind::Error,
                            message: format!("加载响应示例失败：{}", e.user_message()),
                        });
                        return;
                    }
                }
            }
            // 2. 构建定义：规则 > 示例 > 默认。
            let mut defs: Vec<fox_mock::server::MockDefinition> = Vec::new();
            for rule in rules.iter().filter(|r| r.enabled) {
                defs.push(fox_mock::server::MockDefinition::from_rule(rule));
            }
            for ep in &eps {
                if ep.status == fox_core::model::EndpointStatus::Deprecated {
                    continue;
                }
                let example = examples_by_ep.get(&ep.id).and_then(|l| l.first());
                defs.push(fox_mock::server::MockDefinition::from_endpoint(
                    ep.method.as_str(),
                    &ep.path,
                    example,
                ));
            }
            // 3. 启动。
            let store = fox_mock::server::MockStore::new();
            store.set_definitions(defs);
            match fox_mock::server::start(store).await {
                Ok(server) => {
                    let addr = server.address();
                    let server_port = server.port;
                    tracing::info!("用户启动 Mock port={}", server_port);
                    push_step(steps, format!("启动 Mock 服务（port {server_port}）"));
                    let inner = Arc::new(tokio::sync::Mutex::new(Some(server)));
                    handle.set(Some(inner));
                    port.set(Some(server_port));
                    toasts.write().push(Toast {
                        id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                        kind: ToastKind::Success,
                        message: format!("Mock 服务已启动：{addr}"),
                    });
                }
                Err(e) => toasts.write().push(Toast {
                    id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                    kind: ToastKind::Error,
                    message: e.user_message(),
                }),
            }
        });
    }

    /// 停止 Mock 服务。
    pub fn stop_mock(&self) {
        if !self.mock_running() {
            return;
        }
        let mut port = self.mock_port;
        let mut handle = self.mock_handle;
        let mut toasts = self.toasts;
        let steps = self.steps;
        spawn(async move {
            if let Some(inner) = handle.write().take() {
                let taken = inner.lock().await.take();
                if let Some(server) = taken {
                    server.stop().await;
                }
            }
            port.set(None);
            push_step(steps, "停止 Mock 服务".into());
            toasts.write().push(Toast {
                id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                kind: ToastKind::Info,
                message: "Mock 服务已停止".into(),
            });
        });
    }

    /// 备份当前项目：收集全量数据 → JSON → 写入 {data_dir}/RustFox/backups/。
    pub fn backup_project(&self) {
        let Some(project_id) = self.current_project() else {
            self.toast_error("未选择项目");
            return;
        };
        let Some(project) = self
            .projects
            .read()
            .iter()
            .find(|p| p.id == project_id)
            .cloned()
        else {
            self.toast_error("未找到当前项目");
            return;
        };
        let folders = self.folders.read().clone();
        let endpoints = self.endpoints.read().clone();
        let environments = self.environments.read().clone();
        let mock_rules = self.mock_rules.read().clone();
        let db = self.services.db.clone();
        let st = self.clone();
        let steps = self.steps;
        spawn(async move {
            let mut examples: Vec<fox_core::model::ResponseExample> = Vec::new();
            for ep in &endpoints {
                match repo::list_response_examples(&db, ep.id).await {
                    Ok(list) => examples.extend(list),
                    Err(e) => {
                        st.toast_error(format!("加载响应示例失败：{}", e.user_message()));
                        return;
                    }
                }
            }
            let file = fox_backup::build_backup(
                &project,
                &folders,
                &endpoints,
                &environments,
                &mock_rules,
                &examples,
            );
            match file.serialize() {
                Ok(json) => {
                    let dir = fox_storage::db::data_dir().join("backups");
                    if std::fs::create_dir_all(&dir).is_err() {
                        st.toast_error("创建备份目录失败");
                        return;
                    }
                    let filename = format!(
                        "{}_{}.json",
                        project
                            .name
                            .trim()
                            .replace(['/', ':', '\\'], "_")
                            .replace(' ', "_"),
                        Utc::now().format("%Y%m%d_%H%M%S")
                    );
                    let path = dir.join(filename);
                    match std::fs::write(&path, json) {
                        Ok(()) => {
                            push_step(steps, format!("备份项目：{}", path.display()));
                            st.toast_success(format!(
                                "备份完成：{}（恢复时请粘贴该文件内容）",
                                path.display()
                            ));
                        }
                        Err(e) => st.toast_error(format!("备份写入失败：{e}")),
                    }
                }
                Err(e) => st.toast_error(format!("备份序列化失败：{}", e.user_message())),
            }
        });
    }

    /// 恢复备份：解析文本 → 重新映射 UUID → 全量写入（新项目，不覆盖现有数据）。
    pub fn restore_backup(&self, text: String) {
        let file = match fox_backup::BackupFile::parse(&text) {
            Ok(f) => f,
            Err(e) => {
                self.toast_error(e.user_message());
                return;
            }
        };
        let db = self.services.db.clone();
        let st = self.clone();
        let steps = self.steps;
        spawn(async move {
            let restored = fox_backup::restore_backup(&file);
            // 新项目先入库，再按引用关系写入子对象。
            if let Err(e) = repo::save_project(&db, &restored.project).await {
                st.toast_error(format!("恢复项目失败：{}", e.user_message()));
                return;
            }
            let mut failed: Vec<String> = Vec::new();
            for f in &restored.folders {
                if let Err(e) = repo::save_folder(&db, f).await {
                    failed.push(e.user_message());
                }
            }
            for e in &restored.environments {
                if let Err(e) = repo::save_environment(&db, e).await {
                    failed.push(e.user_message());
                }
            }
            for e in &restored.endpoints {
                if let Err(e) = repo::save_endpoint(&db, e).await {
                    failed.push(e.user_message());
                }
            }
            for r in &restored.mock_rules {
                if let Err(e) = repo::save_mock_rule(&db, r).await {
                    failed.push(e.user_message());
                }
            }
            for e in &restored.response_examples {
                if let Err(e) = repo::save_response_example(&db, e).await {
                    failed.push(e.user_message());
                }
            }
            st.refresh_projects();
            st.refresh_project_data(restored.project.id);
            st.set_current_project(Some(restored.project.id));
            push_step(
                steps,
                format!("恢复备份：项目「{}」", restored.project.name),
            );
            if failed.is_empty() {
                st.toast_success(format!("恢复完成：项目「{}」已创建", restored.project.name));
            } else {
                st.toast_error(format!(
                    "恢复完成（{} 项正常，{} 项失败：{}）",
                    restored.project.name,
                    failed.len(),
                    failed.first().unwrap_or(&String::new())
                ));
            }
        });
    }

    /// 解析表单字段并新建 Mock 规则。
    #[allow(clippy::too_many_arguments)]
    pub fn add_mock_rule(
        &self,
        name: String,
        method: String,
        path: String,
        status: String,
        priority: String,
        delay: String,
        query_lines: String,
        header_lines: String,
        body: String,
    ) {
        let name = name.trim();
        let path = path.trim();
        if name.is_empty() || path.is_empty() {
            self.toast_error("规则名称和路径不能为空");
            return;
        }
        let Ok(method) = method.parse::<HttpMethod>() else {
            self.toast_error("无效的 HTTP 方法");
            return;
        };
        let response_status = status.trim().parse::<u16>().unwrap_or(200);
        let priority = priority.trim().parse::<i64>().unwrap_or(0);
        let delay_ms = delay.trim().parse::<u64>().unwrap_or(0);
        let now = Utc::now();
        let rule = MockRule {
            id: Uuid::new_v4(),
            project_id: self.current_project().unwrap_or_default(),
            endpoint_id: None,
            name: name.to_string(),
            method,
            path: path.to_string(),
            match_query: parse_match_lines(&query_lines),
            match_headers: parse_match_lines(&header_lines),
            response_status,
            response_headers: HashMap::new(),
            response_body_template: body,
            delay_ms,
            enabled: true,
            priority,
            created_at: now,
            updated_at: now,
        };
        self.create_mock_rule(rule);
    }

    /// 新建 Mock 规则（落库 + 列表）。
    pub fn create_mock_rule(&self, rule: MockRule) {
        let Some(project_id) = self.current_project() else {
            self.toast_error("未选择项目");
            return;
        };
        let db = self.services.db.clone();
        let mut rules = self.mock_rules;
        let mut toasts = self.toasts;
        spawn(async move {
            match repo::create_mock_rule(&db, project_id, &rule).await {
                Ok(saved) => {
                    rules.write().push(saved);
                    toasts.write().push(Toast {
                        id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                        kind: ToastKind::Success,
                        message: "Mock 规则已创建（重启 Mock 后生效）".into(),
                    });
                }
                Err(e) => toasts.write().push(Toast {
                    id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                    kind: ToastKind::Error,
                    message: format!("创建 Mock 规则失败：{}", e.user_message()),
                }),
            }
        });
    }

    /// 删除 Mock 规则。
    pub fn delete_mock_rule(&self, rule_id: Uuid) {
        let db = self.services.db.clone();
        let mut rules = self.mock_rules;
        let mut toasts = self.toasts;
        spawn(async move {
            match repo::delete_mock_rule(&db, rule_id).await {
                Ok(()) => {
                    rules.write().retain(|r| r.id != rule_id);
                    toasts.write().push(Toast {
                        id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                        kind: ToastKind::Success,
                        message: "Mock 规则已删除".into(),
                    });
                }
                Err(e) => toasts.write().push(Toast {
                    id: TOAST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                    kind: ToastKind::Error,
                    message: format!("删除 Mock 规则失败：{}", e.user_message()),
                }),
            }
        });
    }
}

/// 追加步骤并裁剪到 MAX_STEPS 条（供 async 闭包内使用）。
fn push_step(mut steps: Signal<Vec<String>>, msg: String) {
    let mut guard = steps.write();
    guard.push(msg);
    if guard.len() > MAX_STEPS {
        let drop_count = guard.len() - MAX_STEPS;
        guard.drain(..drop_count);
    }
}

/// 解析 "key=value" 匹配行（空行/无等号行忽略）。
fn parse_match_lines(lines: &str) -> Vec<MockMatchItem> {
    lines
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && l.contains('='))
        .map(|l| {
            let mut it = l.splitn(2, '=');
            MockMatchItem {
                key: it.next().unwrap_or_default().trim().to_string(),
                value: it.next().unwrap_or_default().trim().to_string(),
            }
        })
        .collect()
}

/// 为接口批量写入导入的响应示例。
async fn insert_examples(
    db: &sqlx::SqlitePool,
    endpoint_id: Uuid,
    examples: &[fox_openapi::import::ImportedExample],
) {
    use fox_openapi::import::ImportedExample as Ex;
    for ex in examples {
        let Ex {
            name,
            status,
            content_type,
            headers,
            body,
        } = ex;
        let now = chrono::Utc::now();
        let example = ResponseExample {
            id: Uuid::new_v4(),
            endpoint_id,
            name: name.clone(),
            status: *status,
            headers: headers.clone(),
            body: body.clone(),
            content_type: content_type.clone(),
            created_at: now,
            updated_at: now,
        };
        if let Err(e) = repo::create_response_example(db, endpoint_id, &example).await {
            tracing::warn!("写响应示例失败: {}", e);
        }
    }
}
