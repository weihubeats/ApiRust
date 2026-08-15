//! 项目 JSON 备份与恢复（M10）。
//!
//! 备份 = 单个 JSON 文件，包含项目及全部子对象（含 UUID 引用关系）。
//! 恢复 = 解析备份并重新分配 UUID（新项目），保证不会与现有数据冲突。

use std::collections::HashMap;

use chrono::Utc;
use fox_core::model::{Endpoint, Environment, Folder, MockRule, Project, ResponseExample};
use fox_core::AppError;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 备份文件（顶层）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BackupFile {
    pub format: String,
    pub schema_version: u32,
    pub exported_at: String,
    pub project: Project,
    pub folders: Vec<Folder>,
    pub endpoints: Vec<Endpoint>,
    pub environments: Vec<Environment>,
    pub mock_rules: Vec<MockRule>,
    pub response_examples: Vec<ResponseExample>,
}

/// 备份格式标识。
pub const FORMAT: &str = "rustfox-project-backup";
/// 当前 schema 版本。
pub const SCHEMA_VERSION: u32 = 1;

impl BackupFile {
    pub fn serialize(&self) -> Result<String, AppError> {
        serde_json::to_string_pretty(self).map_err(AppError::Json)
    }

    pub fn parse(text: &str) -> Result<BackupFile, AppError> {
        let file: BackupFile = serde_json::from_str(text)
            .map_err(|e| AppError::Validation(format!("备份文件解析失败：{e}")))?;
        if file.format != FORMAT {
            return Err(fox_core::validation("不是有效的 RustFox 备份文件"));
        }
        if file.schema_version > SCHEMA_VERSION {
            return Err(fox_core::validation(format!(
                "备份文件版本 {} 过新，当前最高支持 {}",
                file.schema_version, SCHEMA_VERSION
            )));
        }
        Ok(file)
    }
}

/// 构建备份文件。
pub fn build_backup(
    project: &Project,
    folders: &[Folder],
    endpoints: &[Endpoint],
    environments: &[Environment],
    mock_rules: &[MockRule],
    response_examples: &[ResponseExample],
) -> BackupFile {
    BackupFile {
        format: FORMAT.to_string(),
        schema_version: SCHEMA_VERSION,
        exported_at: Utc::now().to_rfc3339(),
        project: project.clone(),
        folders: folders.to_vec(),
        endpoints: endpoints.to_vec(),
        environments: environments.to_vec(),
        mock_rules: mock_rules.to_vec(),
        response_examples: response_examples.to_vec(),
    }
}

/// 恢复结果：所有实体均已重映射到新的 UUID，且原本的引用关系保持一致。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Restored {
    pub project: Project,
    pub folders: Vec<Folder>,
    pub endpoints: Vec<Endpoint>,
    pub environments: Vec<Environment>,
    pub mock_rules: Vec<MockRule>,
    pub response_examples: Vec<ResponseExample>,
}

/// 恢复：全量重映射 UUID（新项目）。返回值与 `build_backup` 顺序对应。
pub fn restore_backup(file: &BackupFile) -> Restored {
    let mut map: HashMap<Uuid, Uuid> = HashMap::new();

    let new_project_id = Uuid::new_v4();
    map.insert(file.project.id, new_project_id);
    let mut folders: Vec<Folder> = Vec::new();
    for f in &file.folders {
        let new_id = Uuid::new_v4();
        map.insert(f.id, new_id);
        folders.push(Folder {
            id: new_id,
            project_id: new_project_id,
            parent_id: f.parent_id.map(|p| *map.get(&p).unwrap_or(&new_project_id)),
            ..f.clone()
        });
    }

    let mut endpoints: Vec<Endpoint> = Vec::new();
    for e in &file.endpoints {
        let new_id = Uuid::new_v4();
        map.insert(e.id, new_id);
        endpoints.push(Endpoint {
            id: new_id,
            project_id: new_project_id,
            folder_id: e.folder_id.map(|p| *map.get(&p).unwrap_or(&new_id)),
            ..e.clone()
        });
    }

    let mut environments: Vec<Environment> = Vec::new();
    for e in &file.environments {
        environments.push(Environment {
            id: Uuid::new_v4(),
            project_id: new_project_id,
            ..e.clone()
        });
    }

    let mut mock_rules: Vec<MockRule> = Vec::new();
    for r in &file.mock_rules {
        mock_rules.push(MockRule {
            id: Uuid::new_v4(),
            project_id: new_project_id,
            endpoint_id: r.endpoint_id.map(|p| *map.get(&p).unwrap_or(&Uuid::nil())),
            ..r.clone()
        });
    }

    let mut response_examples: Vec<ResponseExample> = Vec::new();
    for e in &file.response_examples {
        response_examples.push(ResponseExample {
            id: Uuid::new_v4(),
            endpoint_id: *map.get(&e.endpoint_id).unwrap_or(&new_project_id),
            ..e.clone()
        });
    }

    Restored {
        project: Project {
            id: new_project_id,
            ..file.project.clone()
        },
        folders,
        endpoints,
        environments,
        mock_rules,
        response_examples,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fox_core::model::{BodySpec, EndpointStatus, HttpMethod, KeyValue, RequestSpec};

    fn sample_data() -> BackupFile {
        let project = Project {
            id: Uuid::new_v4(),
            name: "示例项目".into(),
            description: "desc".into(),
            variables: HashMap::from([("base_url".into(), "http://127.0.0.1".into())]),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let folder = Folder {
            id: Uuid::new_v4(),
            project_id: project.id,
            parent_id: None,
            name: "用户".into(),
            sort_order: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let mut request = RequestSpec::default();
        request.params.push(KeyValue::new("page", "1"));
        request.body = BodySpec::Json {
            raw: r#"{"a":1}"#.into(),
        };
        let ep = Endpoint {
            id: Uuid::new_v4(),
            project_id: project.id,
            folder_id: Some(folder.id),
            name: "列表".into(),
            method: HttpMethod::GET,
            path: "/users".into(),
            description: String::new(),
            status: EndpointStatus::Released,
            sort_order: 1,
            request,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let env = Environment {
            id: Uuid::new_v4(),
            project_id: project.id,
            name: "测试".into(),
            variables: HashMap::from([("token".into(), "t1".into())]),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let rule = MockRule {
            id: Uuid::new_v4(),
            project_id: project.id,
            endpoint_id: Some(ep.id),
            name: "规则".into(),
            method: HttpMethod::GET,
            path: "/users".into(),
            match_query: Vec::new(),
            match_headers: Vec::new(),
            response_status: 200,
            response_headers: HashMap::new(),
            response_body_template: "{}".into(),
            delay_ms: 0,
            enabled: true,
            priority: 0,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let example = ResponseExample {
            id: Uuid::new_v4(),
            endpoint_id: ep.id,
            name: "成功".into(),
            status: 200,
            headers: HashMap::new(),
            body: r#"{"list":[]}"#.into(),
            content_type: "application/json".into(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        build_backup(&project, &[folder], &[ep], &[env], &[rule], &[example])
    }

    #[test]
    fn serialize_parse_roundtrip() {
        let data = sample_data();
        let text = data.serialize().unwrap();
        let parsed = BackupFile::parse(&text).unwrap();
        assert_eq!(parsed, data);
    }

    #[test]
    fn parse_rejects_wrong_format() {
        assert!(BackupFile::parse(r#"{"format":"other"}"#).is_err());
    }

    #[test]
    fn restore_remaps_all_ids_consistently() {
        let data = sample_data();
        let restored = restore_backup(&data);
        // 新的项目 id
        assert_ne!(restored.project.id, data.project.id);
        assert_eq!(restored.project.name, data.project.name);
        assert_eq!(restored.project.variables, data.project.variables);
        // 文件夹引用新项目
        assert_eq!(restored.folders.len(), 1);
        let f = &restored.folders[0];
        assert_eq!(f.project_id, restored.project.id);
        assert_ne!(f.id, data.folders[0].id);
        // 接口引用新区块
        let ep = &restored.endpoints[0];
        assert_eq!(ep.project_id, restored.project.id);
        assert_eq!(ep.folder_id, Some(f.id));
        assert_eq!(ep.method, HttpMethod::GET);
        assert_eq!(ep.request.params.len(), 1);
        // MockRule 与 ResponseExample 引用新的 endpoint id
        assert_eq!(restored.mock_rules[0].endpoint_id, Some(ep.id));
        assert_eq!(restored.response_examples[0].endpoint_id, ep.id);
        assert_eq!(restored.environments[0].project_id, restored.project.id);
        // 无交叉引用残留
        let old_ids: Vec<Uuid> = data.endpoints.iter().map(|e| e.id).collect();
        for new_ep in &restored.endpoints {
            assert!(!old_ids.contains(&new_ep.id));
        }
    }

    #[test]
    fn restore_is_idempotent_shape() {
        let data = sample_data();
        let a = restore_backup(&data);
        let b = restore_backup(&data);
        assert_ne!(a.project.id, b.project.id);
        assert_eq!(a.endpoints[0].name, b.endpoints[0].name);
    }
}
