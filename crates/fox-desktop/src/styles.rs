//! RustFox 设计系统：所有样式集中在 DESIGN_SYSTEM_CSS，
//! 由根组件挂载一次 <style>{DESIGN_SYSTEM_CSS}</style>。
//! 颜色 / 间距 / 圆角 / 字号一律取自 CSS 变量，禁止页面内私写样式。

pub const DESIGN_SYSTEM_CSS: &str = r#"
/* ============ 设计系统（M2.5 视觉重构） ============ */

* { box-sizing: border-box; margin: 0; padding: 0; }
html, body { height: 100%; }
body {
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", "PingFang SC",
    "Hiragino Sans GB", "Microsoft YaHei", Roboto, sans-serif;
  font-size: 13px; line-height: 1.5;
  color: var(--text); background: var(--bg);
  -webkit-font-smoothing: antialiased;
}
button, input, select, textarea { font: inherit; color: inherit; }
svg { display: block; }


:root {
  --bg: #0b1120;          /* 页面底 */
  --panel: #111a2c;       /* 面板 */
  --panel-2: #1a2438;     /* hover/次级面板 */
  --border: #223049;
  --border-2: #31415f;
  --text: #e8ecf4;
  --text-2: #a7b1c6;
  --muted: #64708c;
  --accent: #3b82f6;
  --accent-2: #2f6fe0;
  --accent-soft: rgba(59,130,246,.16);
  --success: #34d399; --warning: #fbbf24; --danger: #f87171;
  --r-s: 6px; --r-m: 10px; --r-l: 14px;
  --sh-1: 0 1px 2px rgba(0,0,0,.35);
  --sh-2: 0 12px 32px rgba(0,0,0,.45);
}


/* 输入控件：深色底 + 边框 + 聚焦光环 */
.rf-input, .rf-textarea {
  appearance: none; height: 32px; padding: 0 12px;
  background: #0d1526; border: 1px solid var(--border);
  border-radius: var(--r-s); color: var(--text);
  transition: border-color .15s, box-shadow .15s;
}
.rf-input::placeholder, .rf-textarea::placeholder { color: var(--muted); }
.rf-input:hover { border-color: var(--border-2); }
.rf-input:focus, .rf-textarea:focus {
  outline: none; border-color: var(--accent);
  box-shadow: 0 0 0 3px var(--accent-soft);
}
.rf-textarea { height: auto; padding: 8px 12px; resize: vertical;
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; }


/* 按钮 */
.rf-btn {
  display: inline-flex; align-items: center; justify-content: center; gap: 6px;
  height: 32px; padding: 0 14px; border-radius: var(--r-s);
  border: 1px solid transparent; font-weight: 500; cursor: pointer;
  user-select: none;
  transition: background .15s, border-color .15s, color .15s, transform .05s;
}
.rf-btn:active { transform: translateY(1px); }
.rf-btn:disabled { opacity: .5; cursor: not-allowed; }
.rf-btn-primary { background: var(--accent); color: #fff; }
.rf-btn-primary:hover { background: var(--accent-2); }
.rf-btn-ghost { background: transparent; border-color: var(--border); color: var(--text-2); }
.rf-btn-ghost:hover { background: var(--panel-2); color: var(--text); }
.rf-btn-sm { height: 26px; padding: 0 10px; font-size: 12px; }


/* 自定义下拉（替代原生 select） */
.rf-dropdown { position: relative; }
.rf-dropdown-trigger {
  display: inline-flex; align-items: center; gap: 8px; height: 32px;
  padding: 0 10px 0 12px; background: var(--panel-2);
  border: 1px solid var(--border); border-radius: var(--r-s);
  color: var(--text); cursor: pointer;
}
.rf-dropdown-trigger:hover { border-color: var(--border-2); }
.rf-dropdown-trigger .rf-caret { color: var(--muted); transition: transform .15s; }
.rf-dropdown.open .rf-caret { transform: rotate(180deg); }
.rf-dropdown-backdrop { position: fixed; inset: 0; z-index: 40; }
.rf-dropdown-menu {
  position: absolute; top: calc(100% + 4px); left: 0; z-index: 50;
  min-width: 100%; max-height: 280px; overflow: auto;
  background: var(--panel); border: 1px solid var(--border-2);
  border-radius: var(--r-m); box-shadow: var(--sh-2); padding: 4px;
}
.rf-dropdown-item {
  display: flex; align-items: center; gap: 8px; padding: 6px 10px;
  border-radius: 6px; cursor: pointer; color: var(--text-2);
}
.rf-dropdown-item:hover { background: var(--panel-2); color: var(--text); }
.rf-dropdown-item.selected { color: var(--accent); }


/* 顶栏 */
.rf-topbar {
  height: 48px; display: flex; align-items: center; gap: 12px;
  padding: 0 16px; background: var(--panel);
  border-bottom: 1px solid var(--border);
}
.rf-logo { font-size: 15px; font-weight: 700; color: var(--accent); }
.rf-topbar-sep { width: 1px; height: 20px; background: var(--border); }
.rf-topbar-spacer { flex: 1; }
.rf-search { position: relative; width: 280px; }
.rf-search .rf-input { width: 100%; height: 30px; padding-left: 30px; }
.rf-search svg { position: absolute; left: 10px; top: 50%; translate: 0 -50%; color: var(--muted); }


/* 卡片 / 空状态 / 表单行 */
.rf-card { background: var(--panel); border: 1px solid var(--border);
  border-radius: var(--r-l); box-shadow: var(--sh-1); }
.rf-home { height: 100%; display: flex; justify-content: center;
  padding: 10vh 24px 24px; }
.rf-home-card { width: 100%; max-width: 620px; padding: 24px; }
.rf-card-title { font-size: 16px; font-weight: 600; }
.rf-divider { height: 1px; background: var(--border); margin: 16px 0; }
.rf-empty { display: flex; flex-direction: column; align-items: center;
  gap: 6px; padding: 32px 16px; }
.rf-empty-icon { width: 48px; height: 48px; border-radius: 12px; margin-bottom: 6px;
  display: flex; align-items: center; justify-content: center;
  background: var(--accent-soft); color: var(--accent); }
.rf-empty-title { font-size: 14px; font-weight: 600; }
.rf-empty-desc { font-size: 12px; color: var(--muted); }
.rf-form-row { display: flex; gap: 8px; }
.rf-form-row .rf-input { flex: 1; min-width: 0; }
.rf-hint { margin-top: 14px; display: flex; flex-wrap: wrap; gap: 4px 6px;
  align-items: center; justify-content: center;
  font-size: 12px; color: var(--muted); }
.rf-kbd { padding: 1px 6px; border-radius: 4px; border: 1px solid var(--border);
  border-bottom-width: 2px; background: var(--panel-2); color: var(--text-2);
  font: 11px ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; }


/* Toast */
.rf-toast-wrap { position: fixed; top: 60px; right: 16px; z-index: 100;
  display: flex; flex-direction: column; gap: 8px; }
.rf-toast { display: flex; align-items: center; gap: 8px; padding: 8px 12px;
  background: var(--panel); border: 1px solid var(--border-2);
  border-radius: var(--r-m); box-shadow: var(--sh-2); }
.rf-toast-error { border-color: rgba(248,113,113,.45); }
.rf-toast-success { border-color: rgba(52,211,153,.45); }


/* 滚动条 / 焦点 / HTTP 方法色 */
::-webkit-scrollbar { width: 10px; height: 10px; }
::-webkit-scrollbar-thumb { background: #2b3a57; border-radius: 8px;
  border: 2px solid var(--bg); }
::-webkit-scrollbar-thumb:hover { background: #3a4c6f; }
::-webkit-scrollbar-track { background: transparent; }
:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
.rf-method { font-weight: 700; font-size: 12px; }
.rf-method-get { color: var(--success); }
.rf-method-post { color: var(--warning); }
.rf-method-put { color: var(--accent); }
.rf-method-delete { color: var(--danger); }
.rf-method-patch { color: #c084fc; }


/* ============ 扩展变量 / 组件样式（M2.5+） ============ */

:root {
  /* 间距（4px 网格） */
  --s-1: 4px; --s-2: 8px; --s-3: 12px; --s-4: 16px; --s-5: 20px;
  --s-6: 24px; --s-7: 32px; --s-8: 40px; --s-9: 48px;
  /* 语义色 */
  --white: #ffffff;
  --on-success: #06281d;
  --code-bg: #0a0f1c;
  --purple: #c084fc;
  --purple-weak: rgba(192, 132, 252, .16);
  --danger-strong: #ff8a80;
  --danger-weak: rgba(248, 113, 113, .14);
  --danger-weak-2: rgba(248, 113, 113, .28);
  --danger-weak-3: rgba(179, 38, 30, .25);
  --danger-weak-4: rgba(179, 38, 30, .14);
  --success-weak: rgba(52, 211, 153, .16);
  --success-weak-2: rgba(30, 126, 52, .25);
  --warning-weak: rgba(251, 191, 36, .12);
  --accent-line: rgba(59, 130, 246, .45);
  --scrim: rgba(4, 8, 17, .62);
  --white-weak: rgba(255, 255, 255, .06);
}

/* 布局骨架 */
#main { height: 100%; }
.app { display: flex; flex-direction: column; height: 100%; }
.body { flex: 1; display: flex; min-height: 0; }
.main { flex: 1; min-width: 0; display: flex; flex-direction: column; background: var(--bg); }
.page-pad { padding: var(--s-5) var(--s-6); overflow-y: auto; flex: 1; }
.row { display: flex; gap: var(--s-2); align-items: center; }
.grow { flex: 1; }
.spacer { flex: 1; }
.center-box { max-width: 760px; margin: var(--s-9) auto; background: var(--panel);
  border: 1px solid var(--border); border-radius: var(--r-l); padding: var(--s-7); }
.hint { color: var(--muted); font-size: 12px; text-align: center; margin-top: var(--s-4); }
.empty { text-align: center; color: var(--muted); padding: var(--s-8) 0; font-size: 13px; }
a { color: var(--accent); cursor: pointer; text-decoration: none; }
a:hover { text-decoration: underline; }
code, pre { font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; }
.status-ok { color: var(--success); font-weight: 700; }
.status-err { color: var(--danger); font-weight: 700; }
.label-hint { color: var(--muted); font-size: 13px; margin-right: var(--s-2); }
.hint-inline { color: var(--muted); font-size: 12px; }
.warn-text { color: var(--warning); font-size: 12px; margin-top: var(--s-2); }
.warn-pill { color: var(--warning); font-size: 12px; background: var(--warning-weak);
  padding: 2px var(--s-2); border-radius: var(--r-m); }

/* 间距工具（全部取自变量） */
.rf-mt-1 { margin-top: var(--s-1); }
.rf-mt-2 { margin-top: var(--s-2); }
.rf-mt-3 { margin-top: var(--s-3); }
.rf-mt-4 { margin-top: var(--s-4); }
.rf-mb-2 { margin-bottom: var(--s-2); }
.rf-mb-3 { margin-bottom: var(--s-3); }
.rf-hint-flat { margin-top: 0; text-align: left; }

/* 按钮补充 */
.rf-btn-send { background: var(--success); color: var(--on-success); font-weight: 700; }
.rf-btn-send:hover { filter: brightness(1.06); }
.rf-btn-danger { background: var(--danger); color: var(--white); font-weight: 600; }
.rf-btn-danger:hover { filter: brightness(1.06); }
.rf-btn-sm.rf-btn-danger:hover, .rf-btn-sm.rf-btn-send:hover { filter: brightness(1.06); }

/* 输入控件补充 */
.rf-input-sm { height: 26px; padding: 0 var(--s-2); font-size: 12px; }
.rf-in-short { width: 96px; }
.rf-in-72 { width: 72px; }
.rf-in-96 { width: 96px; }
.rf-check { accent-color: var(--accent); width: 14px; height: 14px; }
.rf-kv-input {
  height: 26px; padding: 0 var(--s-2); background: transparent;
  border: 1px solid transparent; color: var(--text); border-radius: var(--r-s);
}
.rf-kv-input:hover { border-color: var(--border-2); background: var(--bg); }
.rf-kv-input:focus {
  outline: none; border-color: var(--accent); background: var(--bg);
  box-shadow: 0 0 0 2px var(--accent-soft);
}
.rf-body-editor { width: 100%; min-height: 220px; height: 70%; }
.rf-editor-desc { resize: none; }
.rf-oapi-input { width: 100%; height: 180px; margin-top: var(--s-2); box-sizing: border-box; }
.rf-mock-field { flex: 1; box-sizing: border-box; font-size: 12.5px; resize: vertical; }
.rf-mock-body { width: 100%; box-sizing: border-box; margin-top: var(--s-2); font-size: 12.5px; resize: vertical; }

/* 下拉补充 */
.rf-dropdown { display: inline-flex; max-width: 100%; }
.rf-dropdown-trigger { max-width: 220px; }
.rf-dropdown-value { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.rf-dropdown-value.rf-placeholder { color: var(--muted); }
.rf-dropdown-item:active { background: var(--accent-soft); }
.rf-dd-method { width: 108px; }
.rf-dd-lang { width: 168px; }

/* 首页：项目卡片网格 */
.rf-project-grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(240px, 1fr));
  gap: var(--s-3); margin-top: var(--s-4); }
.rf-project-card { background: var(--panel); border: 1px solid var(--border);
  border-radius: var(--r-l); padding: var(--s-4); cursor: pointer;
  transition: border-color .15s, box-shadow .15s, transform .05s; }
.rf-project-card:hover { border-color: var(--border-2); box-shadow: var(--sh-1); }
.rf-project-card:active { transform: translateY(1px); }
.rf-project-name { font-size: 15px; font-weight: 600; }
.rf-project-desc { color: var(--muted); margin-top: var(--s-1); font-size: 13px;
  overflow: hidden; text-overflow: ellipsis; display: -webkit-box;
  -webkit-line-clamp: 2; -webkit-box-orient: vertical; }
.rf-project-meta { display: flex; align-items: center; gap: var(--s-2);
  margin-top: var(--s-3); font-size: 12px; color: var(--muted); }

/* 侧边栏 */
.sidebar { width: 288px; flex: 0 0 288px; background: var(--panel);
  border-right: 1px solid var(--border); display: flex; flex-direction: column;
  min-height: 0; overflow-y: auto; }
.sidebar .section-title { padding: var(--s-3) var(--s-4) var(--s-1); color: var(--muted);
  font-size: 12px; font-weight: 600; letter-spacing: .4px;
  display: flex; justify-content: space-between; align-items: center; user-select: none; }
.sidebar .rf-toolbar { padding: var(--s-1) var(--s-3); gap: var(--s-2); flex-wrap: wrap; }
.tree-item { padding: var(--s-1) var(--s-3); cursor: pointer;
  display: flex; align-items: center; gap: var(--s-2); border-radius: var(--r-s);
  margin: 1px var(--s-2); user-select: none; transition: background .12s; }
.tree-item:hover { background: var(--accent-soft); }
.tree-item:active { background: var(--accent-soft); }
.tree-item.selected { background: var(--accent-soft); outline: 1px solid var(--accent); }
.tree-item .name { flex: 1; overflow: hidden; text-overflow: ellipsis;
  white-space: nowrap; font-size: 13px; }
.tree-actions { display: none; gap: var(--s-1); }
.tree-item:hover .tree-actions { display: flex; }
.rf-tree-action { background: transparent; border: none; color: var(--muted);
  cursor: pointer; font-size: 12px; border-radius: var(--r-s); padding: 2px var(--s-1); }
.rf-tree-action:hover { color: var(--text); background: var(--white-weak); }
.rf-tree-action:active { background: var(--accent-soft); }
.kv-table .rf-tree-action:hover { color: var(--danger); background: var(--danger-weak); }

/* HTTP 方法徽章 */
.rf-method-chip { min-width: 46px; text-align: center; padding: 2px var(--s-1);
  border-radius: var(--r-s); font-size: 11px; font-weight: 700;
  background: var(--panel-2); color: var(--muted); }
.rf-method-chip-get { background: var(--success-weak); color: var(--success); }
.rf-method-chip-post { background: var(--warning-weak); color: var(--warning); }
.rf-method-chip-put { background: var(--accent-soft); color: var(--accent); }
.rf-method-chip-delete { background: var(--danger-weak); color: var(--danger); }
.rf-method-chip-patch { background: var(--purple-weak); color: var(--purple); }

/* 弹窗 */
.modal-backdrop { position: fixed; inset: 0; z-index: 500; background: var(--scrim);
  backdrop-filter: blur(3px); display: flex; align-items: center; justify-content: center; }
.modal { min-width: 400px; max-width: 560px; background: var(--panel);
  border: 1px solid var(--border-2); border-radius: var(--r-l); padding: var(--s-5);
  box-shadow: var(--sh-2); }
.modal h3 { margin-bottom: var(--s-3); font-size: 15px; font-weight: 650; }
.rf-modal-actions { display: flex; justify-content: flex-end; gap: var(--s-2);
  margin-top: var(--s-3); }
.history-modal { display: flex; flex-direction: column; max-height: 74vh; width: 680px; }
.history-list { flex: 1; overflow-y: auto; min-height: 140px; }
.history-item { padding: var(--s-2) var(--s-3); border-bottom: 1px solid var(--border);
  cursor: pointer; border-radius: var(--r-s); margin: 0 var(--s-1); }
.history-item:hover { background: var(--accent-soft); }
.history-item:active { background: var(--accent-soft); }
.history-item .history-meta { display: flex; gap: var(--s-2); align-items: center; }
.history-item .history-meta .url { flex: 1; overflow: hidden; text-overflow: ellipsis;
  white-space: nowrap; color: var(--text); font-size: 13px; }
.history-item .history-time { color: var(--muted); font-size: 12px; margin-top: var(--s-1); }
.history-detail { border-top: 1px solid var(--border); padding: var(--s-3);
  max-height: 280px; overflow-y: auto; }
.history-detail pre { font-size: 12.5px; line-height: 1.6; }

/* 编辑器 */
.editor { display: flex; flex-direction: column; flex: 1; min-height: 0; background: var(--bg); }
.editor .url-bar { display: flex; gap: var(--s-2); padding: var(--s-2) var(--s-3);
  border-bottom: 1px solid var(--border); align-items: center; }
.editor .url-bar .rf-input { flex: 1; min-width: 0; font-family: ui-monospace,
  SFMono-Regular, Menlo, Consolas, monospace; }
.editor .tabs { display: flex; border-bottom: 1px solid var(--border);
  padding: 0 var(--s-2); gap: 2px; }
.rf-tab { background: transparent; border: none; border-bottom: 2px solid transparent;
  padding: var(--s-2) var(--s-3); color: var(--muted); cursor: pointer; font-size: 13px;
  border-radius: var(--r-s) var(--r-s) 0 0; transition: color .12s, background .12s; }
.rf-tab:hover { color: var(--text); background: var(--accent-soft); }
.rf-tab.active { color: var(--white); border-bottom-color: var(--accent); font-weight: 600; }
.tab-body { flex: 1; overflow-y: auto; padding: var(--s-3) var(--s-4); }
.editor-meta { display: flex; gap: var(--s-2); padding: var(--s-2) var(--s-3);
  border-bottom: 1px solid var(--border); align-items: stretch; }
.editor-meta .rf-input, .editor-meta .rf-textarea { flex: 1; min-width: 0; }
.auth-field { display: flex; align-items: center; gap: var(--s-1); margin-bottom: var(--s-2); }
.auth-field .rf-input { flex: 1; min-width: 0; }

/* 多标签栏 */
.tab-bar { display: flex; align-items: flex-end; gap: var(--s-1); overflow-x: auto;
  padding: var(--s-2) var(--s-2) 0; border-bottom: 1px solid var(--border);
  background: var(--panel); }
.editor-tab { display: flex; align-items: center; gap: var(--s-1); padding: var(--s-2) var(--s-3);
  border: 1px solid var(--border); border-bottom: none; border-radius: var(--r-m) var(--r-m) 0 0;
  cursor: pointer; font-size: 13px; color: var(--muted); background: var(--panel-2);
  max-width: 220px; white-space: nowrap; transition: color .12s, background .12s; }
.editor-tab:hover { color: var(--text); }
.editor-tab:active { background: var(--panel-2); }
.editor-tab.active { background: var(--bg); color: var(--text); font-weight: 600; }
.editor-tab .tab-dirty { color: var(--warning); font-size: 10px; }
.rf-tab-close { border: none; background: none; color: var(--muted); cursor: pointer;
  font-size: 14px; line-height: 1; padding: 2px var(--s-1); border-radius: var(--r-s); }
.rf-tab-close:hover { color: var(--white); background: var(--danger-weak-2); }
.rf-tab-close:active { background: var(--danger-weak); }

/* KeyValue 表 */
.kv-table { width: 100%; border-collapse: collapse; }
.kv-table th { color: var(--muted); text-align: left; font-weight: 600; font-size: 12px;
  padding: var(--s-1) var(--s-2); border-bottom: 1px solid var(--border); }
.kv-table td { padding: var(--s-1) var(--s-2); border-bottom: 1px solid var(--border); }
.kv-table td:first-child, .kv-table th:first-child { width: 44px; text-align: center; }
.kv-table .row-actions { width: 72px; text-align: center; }

/* 响应区 */
.response { display: flex; flex-direction: column; flex: 1; min-height: 0;
  border-top: 1px solid var(--border); }
.resp-head { display: flex; gap: var(--s-3); align-items: center; padding: var(--s-2) var(--s-3);
  border-bottom: 1px solid var(--border); font-size: 13px; background: var(--panel); }
.response .resp-head .status-ok, .response .resp-head .status-err { font-size: 14px; }
.response pre { margin: 0; padding: var(--s-3) var(--s-4); overflow: auto; flex: 1;
  font-size: 12.5px; line-height: 1.6; color: var(--text); background: var(--code-bg); }
.resp-summary { padding: var(--s-2) var(--s-3); border-bottom: 1px solid var(--border);
  max-height: 130px; overflow-y: auto; background: var(--panel); }
.resp-summary label { color: var(--muted); font-size: 12px; display: block;
  margin-bottom: var(--s-1); font-weight: 600; }
.resp-hdr-row { display: flex; gap: var(--s-3); font-size: 12px; padding: 2px 0; }
.resp-hdr-row code { color: var(--accent); min-width: 180px; }

/* 测试 */
.test-run { border: 1px solid var(--border-2); border-radius: var(--r-l);
  margin-top: var(--s-3); background: var(--panel); overflow: hidden; }
.test-run > .resp-head { padding: var(--s-2) var(--s-3); border-bottom: 1px solid var(--border);
  display: flex; gap: var(--s-2); align-items: center; }
.test-summary { font-size: 12px; padding: 2px var(--s-2); border-radius: var(--r-m);
  font-weight: 700; }
.test-summary.ok { background: var(--success-weak-2); color: var(--success); }
.test-summary.bad { background: var(--danger-weak-3); color: var(--danger-strong); }
.test-summary.skip { background: var(--panel-2); color: var(--muted); }
.test-row { padding: var(--s-2) var(--s-3); border-bottom: 1px solid var(--border); }
.test-row:last-child { border-bottom: none; }
.test-row.fail { background: var(--danger-weak-4); border-left: 3px solid var(--danger); }
.test-row.skip { opacity: .55; }
.test-row-main { display: flex; gap: var(--s-2); align-items: center; flex-wrap: wrap; }
.test-row .url { color: var(--text); font-size: 13px; font-family: ui-monospace,
  SFMono-Regular, Menlo, Consolas, monospace; }
.test-row .test { color: var(--muted); font-size: 12px; }
.test-badge { font-size: 11px; padding: 2px var(--s-2); border-radius: var(--r-m);
  font-weight: 700; }
.test-badge.ok { background: var(--success-weak-2); color: var(--success); }
.test-badge.bad { background: var(--danger-weak-3); color: var(--danger-strong); }
.test-badge.skip { background: var(--panel-2); color: var(--muted); }
.test-fail { font-size: 12px; color: var(--danger-strong); margin-top: var(--s-1);
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  background: var(--danger-weak); border-radius: var(--r-s); padding: var(--s-1) var(--s-2); }

/* 测试历史 */
.history-box { margin-top: var(--s-4); }
.history-box .kv-title { margin-bottom: var(--s-2); }
.kv-title { color: var(--muted); font-size: 12px; font-weight: 600;
  margin: var(--s-2) 0 var(--s-1); letter-spacing: .3px; }
.hist-row { border: 1px solid var(--border); border-radius: var(--r-m);
  padding: var(--s-2) var(--s-3); margin-top: var(--s-2); display: flex; gap: var(--s-3);
  align-items: center; font-size: 12px; background: var(--panel); }
.hist-time { color: var(--muted); min-width: 108px; }
.hist-name { font-weight: 650; min-width: 120px; }
.hist-detail { width: 100%; padding: var(--s-2); border-top: 1px dashed var(--border);
  margin-top: var(--s-2); max-height: 240px; overflow-y: auto; }
.hist-detail .kv-row { display: flex; gap: var(--s-2); align-items: center;
  padding: var(--s-1) 0; font-size: 12px; }
.hist-detail .url { flex: 1; overflow: hidden; text-overflow: ellipsis;
  white-space: nowrap; color: var(--text); }

/* 压测 */
.load-box { border-top: 1px solid var(--border); margin-top: var(--s-4);
  padding-top: var(--s-3); }
.load-result { border: 1px solid var(--border-2); border-radius: var(--r-m);
  padding: var(--s-3); background: var(--panel); }
.load-grid { display: grid; grid-template-columns: 120px 1fr; gap: var(--s-2) var(--s-4);
  font-size: 13px; margin-top: var(--s-2); }
.load-grid > span:nth-child(odd) { color: var(--muted); }
.load-ok { color: var(--success); font-weight: 700; }
.load-bad { color: var(--danger); font-weight: 700; }
.load-err { font-size: 12px; color: var(--danger); word-break: break-all; }

/* 代码生成 */
.codegen-modal { display: flex; flex-direction: column; max-height: 76vh; width: 680px; }
.codegen-out { flex: 1; overflow: auto; background: var(--code-bg); color: var(--text);
  border-radius: var(--r-m); padding: var(--s-3) var(--s-4); font-size: 12.5px;
  line-height: 1.6; white-space: pre; user-select: text; border: 1px solid var(--border); }

/* Docs Tab */
.docs-meta { display: flex; align-items: center; gap: var(--s-2); margin-bottom: var(--s-2); }
.method-badge { font-size: 11px; font-weight: 700; color: var(--white); padding: 2px var(--s-2);
  border-radius: var(--r-s); background: var(--purple); min-width: 56px; text-align: center; }
.doc-path { font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 13px; color: var(--text); }
.doc-desc { color: var(--text); font-size: 13px; margin: var(--s-2) 0 var(--s-3);
  padding: var(--s-2) var(--s-3); background: var(--panel-2); border-radius: var(--r-m);
  border: 1px solid var(--border); }
.docs-block h4 { margin: var(--s-4) 0 var(--s-2); color: var(--muted);
  font-size: 13px; font-weight: 650; }
.docs-block .kv-row { display: flex; gap: var(--s-3); align-items: center;
  padding: 2px var(--s-1); font-size: 12px; }
.docs-block .kv-row code { color: var(--accent); min-width: 150px; }
.doc-body { margin: 0; padding: var(--s-2) var(--s-3); border: 1px solid var(--border);
  border-radius: var(--r-m); font-size: 12.5px; white-space: pre-wrap;
  word-break: break-all; max-height: 200px; overflow-y: auto; background: var(--code-bg); }
.ex-row { display: flex; gap: var(--s-2); align-items: center; padding: var(--s-2);
  border: 1px solid var(--border); border-radius: var(--r-m); margin-top: var(--s-2);
  font-size: 12px; background: var(--panel); }
.ex-row .hint { color: var(--muted); font-size: 12px; }

/* 设置页 */
.settings-section { margin-top: var(--s-4); }
.settings-section > h3 { font-size: 15px; margin-bottom: var(--s-2); }
.section-title { font-weight: 700; margin-bottom: var(--s-2); font-size: 15px; }
.settings-section > .hint { text-align: left; margin: 0 0 var(--s-2); }
.env-item { border: 1px solid var(--border); border-radius: var(--r-m); padding: var(--s-3);
  margin-bottom: var(--s-2); background: var(--panel); }
.env-item .env-name { font-weight: 650; }
.env-current { background: var(--accent); color: var(--white); border-radius: var(--r-m);
  padding: 2px var(--s-2); font-size: 11px; font-weight: 600; }
.env-editor { margin-top: var(--s-3); padding: var(--s-3); background: var(--bg);
  border-radius: var(--r-m); border: 1px solid var(--border); }
.env-editor .row { margin-top: var(--s-1); }
.mock-status { font-size: 12px; padding: 2px var(--s-2); border-radius: var(--r-m);
  margin-right: var(--s-2); font-weight: 700; }
.mock-status.ok { background: var(--success-weak-2); color: var(--success); }
.mock-status.off { background: var(--panel-2); color: var(--muted); }
.mock-rule-row { border: 1px solid var(--border); border-radius: var(--r-m);
  padding: var(--s-2) var(--s-3); margin-top: var(--s-2); background: var(--panel); }
.backup-box { margin-top: var(--s-2); }
.backup-box .rf-textarea { width: 100%; min-height: 160px; box-sizing: border-box;
  margin-top: var(--s-2); font-size: 12.5px; resize: vertical; }

/* Toast 补充 */
.rf-toast-info { border-color: var(--accent-line); }
"#;
