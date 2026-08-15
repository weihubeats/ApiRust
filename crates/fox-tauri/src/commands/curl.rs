//! cURL 导入 Command：解析命令字符串为请求模型，由前端组装草稿并落库。

use fox_core::curl_parser::{self, CurlParsed};

use crate::error::CommandResult;

/// 解析 cURL 命令，返回可编辑的请求模型（URL / 方法 / 头 / Body / 认证）。
///
/// 前端拿到结果后组装 `Endpoint` 草稿（未保存），用户编辑完再调
/// `save_endpoint` 落库——与桌面端「导入为未保存草稿」流程一致。
#[tauri::command(rename_all = "camelCase")]
pub async fn parse_curl_command(command: String) -> CommandResult<CurlParsed> {
    curl_parser::parse_curl(&command).map_err(Into::into)
}
