## Default Permission

RustFox 核心插件的默认权限：允许前端调用全部 fox 命令
（项目 / 接口 / 文件夹 / 环境 / cURL 解析 / 请求执行）。
命令的 allow-* 权限由 tauri-build 依据 invoke_handler 自动生成。

#### This default permission set includes the following:

- `allow-get-projects`
- `allow-save-project`
- `allow-delete-project`
- `allow-set-active-project`
- `allow-get-active-project`
- `allow-list-endpoints`
- `allow-get-endpoint`
- `allow-save-endpoint`
- `allow-delete-endpoint`
- `allow-duplicate-endpoint`
- `allow-list-folders`
- `allow-save-folder`
- `allow-delete-folder`
- `allow-parse-curl-command`
- `allow-list-environments`
- `allow-save-environment`
- `allow-set-active-environment`
- `allow-get-active-environment`
- `allow-delete-environment`
- `allow-execute-request`
- `allow-cancel-request`

## Permission Table

<table>
<tr>
<th>Identifier</th>
<th>Description</th>
</tr>


<tr>
<td>

`fox-tauri:allow-backup-export`

</td>
<td>

Enables the backup_export command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:deny-backup-export`

</td>
<td>

Denies the backup_export command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:allow-backup-restore`

</td>
<td>

Enables the backup_restore command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:deny-backup-restore`

</td>
<td>

Denies the backup_restore command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:allow-cancel-request`

</td>
<td>

Enables the cancel_request command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:deny-cancel-request`

</td>
<td>

Denies the cancel_request command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:allow-codegen-render`

</td>
<td>

Enables the codegen_render command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:deny-codegen-render`

</td>
<td>

Denies the codegen_render command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:allow-delete-endpoint`

</td>
<td>

Enables the delete_endpoint command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:deny-delete-endpoint`

</td>
<td>

Denies the delete_endpoint command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:allow-delete-environment`

</td>
<td>

Enables the delete_environment command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:deny-delete-environment`

</td>
<td>

Denies the delete_environment command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:allow-delete-example`

</td>
<td>

Enables the delete_example command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:deny-delete-example`

</td>
<td>

Denies the delete_example command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:allow-delete-folder`

</td>
<td>

Enables the delete_folder command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:deny-delete-folder`

</td>
<td>

Denies the delete_folder command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:allow-delete-mock-rule`

</td>
<td>

Enables the delete_mock_rule command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:deny-delete-mock-rule`

</td>
<td>

Denies the delete_mock_rule command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:allow-delete-project`

</td>
<td>

Enables the delete_project command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:deny-delete-project`

</td>
<td>

Denies the delete_project command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:allow-duplicate-endpoint`

</td>
<td>

Enables the duplicate_endpoint command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:deny-duplicate-endpoint`

</td>
<td>

Denies the duplicate_endpoint command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:allow-execute-request`

</td>
<td>

Enables the execute_request command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:deny-execute-request`

</td>
<td>

Denies the execute_request command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:allow-export-openapi`

</td>
<td>

Enables the export_openapi command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:deny-export-openapi`

</td>
<td>

Denies the export_openapi command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:allow-get-active-environment`

</td>
<td>

Enables the get_active_environment command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:deny-get-active-environment`

</td>
<td>

Denies the get_active_environment command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:allow-get-active-project`

</td>
<td>

Enables the get_active_project command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:deny-get-active-project`

</td>
<td>

Denies the get_active_project command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:allow-get-endpoint`

</td>
<td>

Enables the get_endpoint command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:deny-get-endpoint`

</td>
<td>

Denies the get_endpoint command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:allow-get-projects`

</td>
<td>

Enables the get_projects command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:deny-get-projects`

</td>
<td>

Denies the get_projects command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:allow-import-document`

</td>
<td>

Enables the import_document command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:deny-import-document`

</td>
<td>

Denies the import_document command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:allow-list-endpoints`

</td>
<td>

Enables the list_endpoints command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:deny-list-endpoints`

</td>
<td>

Denies the list_endpoints command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:allow-list-environments`

</td>
<td>

Enables the list_environments command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:deny-list-environments`

</td>
<td>

Denies the list_environments command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:allow-list-examples`

</td>
<td>

Enables the list_examples command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:deny-list-examples`

</td>
<td>

Denies the list_examples command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:allow-list-folders`

</td>
<td>

Enables the list_folders command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:deny-list-folders`

</td>
<td>

Denies the list_folders command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:allow-list-mock-rules`

</td>
<td>

Enables the list_mock_rules command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:deny-list-mock-rules`

</td>
<td>

Denies the list_mock_rules command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:allow-list-request-histories`

</td>
<td>

Enables the list_request_histories command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:deny-list-request-histories`

</td>
<td>

Denies the list_request_histories command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:allow-load-test`

</td>
<td>

Enables the load_test command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:deny-load-test`

</td>
<td>

Denies the load_test command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:allow-mock-start`

</td>
<td>

Enables the mock_start command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:deny-mock-start`

</td>
<td>

Denies the mock_start command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:allow-mock-status`

</td>
<td>

Enables the mock_status command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:deny-mock-status`

</td>
<td>

Denies the mock_status command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:allow-mock-stop`

</td>
<td>

Enables the mock_stop command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:deny-mock-stop`

</td>
<td>

Denies the mock_stop command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:allow-oauth-access-token`

</td>
<td>

Enables the oauth_access_token command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:deny-oauth-access-token`

</td>
<td>

Denies the oauth_access_token command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:allow-oauth-authorize`

</td>
<td>

Enables the oauth_authorize command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:deny-oauth-authorize`

</td>
<td>

Denies the oauth_authorize command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:allow-parse-curl-command`

</td>
<td>

Enables the parse_curl_command command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:deny-parse-curl-command`

</td>
<td>

Denies the parse_curl_command command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:allow-save-endpoint`

</td>
<td>

Enables the save_endpoint command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:deny-save-endpoint`

</td>
<td>

Denies the save_endpoint command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:allow-save-environment`

</td>
<td>

Enables the save_environment command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:deny-save-environment`

</td>
<td>

Denies the save_environment command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:allow-save-example`

</td>
<td>

Enables the save_example command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:deny-save-example`

</td>
<td>

Denies the save_example command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:allow-save-folder`

</td>
<td>

Enables the save_folder command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:deny-save-folder`

</td>
<td>

Denies the save_folder command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:allow-save-mock-rule`

</td>
<td>

Enables the save_mock_rule command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:deny-save-mock-rule`

</td>
<td>

Denies the save_mock_rule command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:allow-save-project`

</td>
<td>

Enables the save_project command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:deny-save-project`

</td>
<td>

Denies the save_project command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:allow-set-active-environment`

</td>
<td>

Enables the set_active_environment command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:deny-set-active-environment`

</td>
<td>

Denies the set_active_environment command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:allow-set-active-project`

</td>
<td>

Enables the set_active_project command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:deny-set-active-project`

</td>
<td>

Denies the set_active_project command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:allow-test-endpoint`

</td>
<td>

Enables the test_endpoint command without any pre-configured scope.

</td>
</tr>

<tr>
<td>

`fox-tauri:deny-test-endpoint`

</td>
<td>

Denies the test_endpoint command without any pre-configured scope.

</td>
</tr>
</table>
