//! RustFox 设计系统：所有样式集中在 DESIGN_SYSTEM_CSS，
//! 由根组件挂载一次 <style>{DESIGN_SYSTEM_CSS}</style>。
//! 颜色 / 间距 / 圆角 / 字号一律取自 CSS 变量，禁止页面内私写样式。
//! 主题基于 Slate / Blue 色板，支持深色 / 浅色 / 自动（跟随系统）：
//! - `:root` 声明深色默认值；
//! - `@media (prefers-color-scheme: light)` 兜底「跟随系统」模式（JS 写入 data-theme 前的首帧）；
//! - `[data-theme="light"]` / `[data-theme="dark"]` 挂在根节点（<html>）上，
//!   显式覆盖继承值（auto 时不设置，由媒体查询跟随系统）。

pub const DESIGN_SYSTEM_CSS: &str = r#"
/* ============ 设计系统（Slate 深色 / 浅色双主题） ============ */

* { box-sizing: border-box; margin: 0; padding: 0; }
html, body { height: 100%; }
body {
  /* 系统无衬线字体优先，中文回退常用中文字体 */
  font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto,
    "Helvetica Neue", Arial, "PingFang SC", "Hiragino Sans GB",
    "Microsoft YaHei", "Noto Sans CJK SC", sans-serif;
  font-size: 13px; line-height: 1.5;
  color: var(--text); background: var(--bg);
  -webkit-font-smoothing: antialiased;
  transition: background .12s, color .12s;
}
button, input, select, textarea { font: inherit; color: inherit; }
svg { display: block; }


/* ---------- 主题 1/4：深色（默认） ---------- */
:root {
  /* 基础色板（Slate）：页面底 / 面板 / 边框 / 主次文字 */
  --bg: #0f172a;          /* slate-900  页面底 */
  --panel: #1e293b;       /* slate-800  面板 / 卡片 / 侧边栏 */
  --panel-2: #334155;     /* slate-700  hover / 次级面板 */
  --border: #334155;      /* slate-700  常规边框 */
  --border-2: #475569;    /* slate-600  强调边框（弹窗 / 下拉） */
  --text: #f8fafc;        /* slate-50   主文字 */
  --text-2: #b3c2d6;      /* 次级文字（AA 7:1+） */
  --muted: #9fb3c7;       /* 弱化文字 / 占位符（AA 4.5:1+） */
  /* 主强调色（Blue） */
  --accent: #60a5fa;      /* blue-400（深底 AA 5.7:1+） */
  --accent-2: #4c90f7;    /* hover 加深 */
  --accent-soft: rgba(59,130,246,.16);
  --accent-line: rgba(59,130,246,.45);
  /* 语义色 */
  --success: #34d399; --warning: #fbbf24; --danger: #f87171;
  --white: #ffffff;
  --hover-weak: rgba(255,255,255,.06);
  --purple: #c084fc;
  --purple-weak: rgba(192,132,252,.16);
  --danger-strong: #ff8a80;
  --danger-weak: rgba(248,113,113,.14);
  --danger-weak-2: rgba(248,113,113,.28);
  --danger-weak-3: rgba(179,38,30,.25);
  --danger-weak-4: rgba(179,38,30,.14);
  --danger-line: rgba(248,113,113,.45);
  --success-weak: rgba(52,211,153,.16);
  --success-weak-2: rgba(30,126,52,.25);
  --success-line: rgba(52,211,153,.45);
  --warning-weak: rgba(251,191,36,.12);
  --on-success: #06281d;
  /* 代码 / 输入 / 高亮 */
  --code-bg: #0a1120;
  --input-bg: #0c1526;
  --caret: #e2e8f0;
  --hl-k: #7dd3fc;
  --hl-s: #86efac;
  --hl-n: #fbbf24;
  --hl-b: #c084fc;
  --hl-p: #94a3b8;
  --tag-bg: rgba(148,163,184,.18);
  --tag-border: rgba(148,163,184,.14);
  --scroll-thumb: #475569;
  --scroll-thumb-hover: #64748b;
  /* 遮罩 / 浮层 */
  --scrim: rgba(2,6,23,.64);
  --overlay-bg: rgba(2,6,23,.55);
  --spinner-track: rgba(255,255,255,.18);
  --overlay-text: rgba(255,255,255,.92);
  /* 阴影 */
  --sh-1: 0 1px 3px rgba(2,6,23,.35);
  --sh-2: 0 12px 32px rgba(2,6,23,.5);
  --sh-card: 0 1px 3px rgba(2,6,23,.3), 0 10px 28px rgba(2,6,23,.28);
  --sh-card-hover: 0 4px 16px rgba(2,6,23,.4);
  --sh-btn-primary: 0 1px 2px rgba(2,6,23,.45), inset 0 1px 0 rgba(255,255,255,.14);
  --sh-btn-primary-hover: 0 4px 14px rgba(59,130,246,.35), inset 0 1px 0 rgba(255,255,255,.14);
  --tab-active-text: var(--white);
  /* 主题派生变量：组件只引用这些，主题块各自赋值（勿在组件里写死颜色） */
  --input-border: var(--border);
  --input-border-hover: var(--border-2);
  --topbar-shadow: none;
  --scroll-radius: 6px;
  --btn-primary-grad: linear-gradient(180deg, #2563eb 0%, #1e55d4 55%, #1d4ed8 100%);
  --btn-primary-grad-hover: linear-gradient(180deg, #2d6ee4 0%, #2563eb 55%, #1d4ed8 100%);
  --chip-base-bg: var(--panel-2); --chip-base-fg: var(--muted);
  --chip-get-bg: var(--success-weak); --chip-get-fg: var(--success); --chip-get-bd: transparent;
  --chip-post-bg: var(--warning-weak); --chip-post-fg: var(--warning); --chip-post-bd: transparent;
  --chip-put-bg: var(--accent-soft); --chip-put-fg: var(--accent); --chip-put-bd: transparent;
  --chip-delete-bg: var(--danger-weak); --chip-delete-fg: var(--danger); --chip-delete-bd: transparent;
  --chip-patch-bg: var(--purple-weak); --chip-patch-fg: var(--purple); --chip-patch-bd: transparent;
  --env-none-fg: var(--muted);
  --tab-inactive-fg: var(--muted);
  --mock-off-fg: var(--muted);
  --oauth-hint-fg: var(--text);
  --oauth-ok-fg: #86efac; --oauth-ok-bg: rgba(34,197,94,.14); --oauth-ok-bd: rgba(34,197,94,.40);
  --oauth-na-fg: #e2e8f0; --oauth-na-bg: rgba(148,163,184,.14); --oauth-na-bd: rgba(148,163,184,.35);
  --oauth-exp-fg: #fde68a; --oauth-exp-bg: rgba(245,158,11,.14); --oauth-exp-bd: rgba(245,158,11,.40);
  --oauth-err-fg: #fca5a5; --oauth-err-bg: rgba(239,68,68,.14); --oauth-err-bd: rgba(239,68,68,.40);
  /* 几何 */
  --r-s: 6px; --r-m: 10px; --r-l: 14px;
  --mono: ui-monospace, SFMono-Regular, Menlo, Consolas, "Liberation Mono", monospace;
}


/* ---------- 主题 2/4：自动模式（跟随系统，data-theme 未显式声明时生效） ---------- */
@media (prefers-color-scheme: light) {
  :root {
    /* 基础色板（Slate 浅色）：白面板 + 灰底，海拔分层 */
    --bg: #f8fafc;
    --panel: #ffffff;
    --panel-2: #f1f5f9;
    --border: #e2e8f0;
    --border-2: #cbd5e1;
    --text: #0f172a;
    --text-2: #475569;
    --muted: #5b6b83;
    /* 主强调色（Blue-600 起，保证白底 AA） */
    --accent: #2563eb;
    --accent-2: #1d4ed8;
    --accent-soft: rgba(37,99,235,.08);
    --accent-line: rgba(37,99,235,.35);
    /* 语义色（深一号，白底 / 浅底 AA 4.5:1） */
    --success: #047857; --warning: #b45309; --danger: #dc2626;
    --hover-weak: rgba(15,23,42,.05);
    --purple: #7c3aed;
    --purple-weak: rgba(124,58,237,.12);
    --danger-strong: #b91c1c;
    --danger-weak: rgba(220,38,38,.08);
    --danger-weak-2: rgba(220,38,38,.14);
    --danger-weak-3: rgba(220,38,38,.12);
    --danger-weak-4: rgba(220,38,38,.06);
    --danger-line: rgba(220,38,38,.35);
    --success-weak: rgba(4,120,87,.10);
    --success-weak-2: rgba(4,120,87,.12);
    --success-line: rgba(4,120,87,.35);
    --warning-weak: rgba(180,83,9,.12);
    --on-success: #ffffff;
    --code-bg: #f8fafc;
    --input-bg: #ffffff;
    --caret: #0f172a;
    --hl-k: #0369a1;
    --hl-s: #047857;
    --hl-n: #b45309;
    --hl-b: #6d28d9;
    --hl-p: #64748b;
    --tag-bg: rgba(71,85,105,.10);
    --tag-border: rgba(71,85,105,.16);
    --scroll-thumb: #cbd5e1;
    --scroll-thumb-hover: #94a3b8;
    --scroll-radius: 4px;
    --scrim: rgba(15,23,42,.40);
    --overlay-bg: rgba(248,250,252,.70);
    --spinner-track: rgba(15,23,42,.12);
    --overlay-text: #0f172a;
    /* 海拔系统：sh-1 卡片/顶栏，sh-2 弹窗/浮层 */
    --sh-1: 0 1px 2px rgba(15,23,42,.05);
    --sh-2: 0 8px 24px rgba(15,23,42,.08);
    --sh-card: 0 1px 2px rgba(15,23,42,.04), 0 2px 8px rgba(15,23,42,.05);
    --sh-card-hover: 0 4px 16px rgba(15,23,42,.10);
    --sh-btn-primary: 0 1px 2px rgba(15,23,42,.10), inset 0 1px 0 rgba(255,255,255,.5);
    --sh-btn-primary-hover: 0 4px 14px rgba(37,99,235,.28), inset 0 1px 0 rgba(255,255,255,.5);
    --tab-active-text: #0f172a;
    /* 派生 */
    --input-border: var(--border-2);
    --input-border-hover: #94a3b8;
    --topbar-shadow: var(--sh-1);
    --btn-primary-grad: linear-gradient(180deg, var(--accent) 0%, var(--accent-2) 100%);
    --btn-primary-grad-hover: linear-gradient(180deg, #2d6ee4 0%, var(--accent) 55%, var(--accent-2) 100%);
    --chip-base-bg: var(--panel-2); --chip-base-fg: #475569;
    --chip-get-bg: #ecfdf5; --chip-get-fg: #047857; --chip-get-bd: #a7f3d0;
    --chip-post-bg: #fffbeb; --chip-post-fg: #b45309; --chip-post-bd: #fde68a;
    --chip-put-bg: #eff6ff; --chip-put-fg: #2563eb; --chip-put-bd: #bfdbfe;
    --chip-delete-bg: #fef2f2; --chip-delete-fg: #b91c1c; --chip-delete-bd: #fecaca;
    --chip-patch-bg: #f5f3ff; --chip-patch-fg: #7c3aed; --chip-patch-bd: #ddd6fe;
    --env-none-fg: #475569;
    --tab-inactive-fg: #475569;
    --mock-off-fg: #475569;
    --oauth-hint-fg: var(--text-2);
    --oauth-ok-fg: #047857; --oauth-ok-bg: #ecfdf5; --oauth-ok-bd: #a7f3d0;
    --oauth-na-fg: #475569; --oauth-na-bg: #f1f5f9; --oauth-na-bd: #e2e8f0;
    --oauth-exp-fg: #b45309; --oauth-exp-bg: #fffbeb; --oauth-exp-bd: #fde68a;
    --oauth-err-fg: #b91c1c; --oauth-err-bg: #fef2f2; --oauth-err-bd: #fecaca;
  }
}


/* ---------- 主题 3/4：显式浅色（data-theme 挂在应用根容器） ---------- */
:root[data-theme="light"] {
  --bg: #f8fafc;
  --panel: #ffffff;
  --panel-2: #f1f5f9;
  --border: #e2e8f0;
  --border-2: #cbd5e1;
  --text: #0f172a;
  --text-2: #475569;
  --muted: #5b6b83;
  --accent: #2563eb;
  --accent-2: #1d4ed8;
  --accent-soft: rgba(37,99,235,.08);
  --accent-line: rgba(37,99,235,.35);
  --success: #047857; --warning: #b45309; --danger: #dc2626;
  --hover-weak: rgba(15,23,42,.05);
  --purple: #7c3aed;
  --purple-weak: rgba(124,58,237,.12);
  --danger-strong: #b91c1c;
  --danger-weak: rgba(220,38,38,.08);
  --danger-weak-2: rgba(220,38,38,.14);
  --danger-weak-3: rgba(220,38,38,.12);
  --danger-weak-4: rgba(220,38,38,.06);
  --danger-line: rgba(220,38,38,.35);
  --success-weak: rgba(4,120,87,.10);
  --success-weak-2: rgba(4,120,87,.12);
  --success-line: rgba(4,120,87,.35);
  --warning-weak: rgba(180,83,9,.12);
  --on-success: #ffffff;
  --code-bg: #f8fafc;
  --input-bg: #ffffff;
  --caret: #0f172a;
  --hl-k: #0369a1;
  --hl-s: #047857;
  --hl-n: #b45309;
  --hl-b: #6d28d9;
  --hl-p: #64748b;
  --tag-bg: rgba(71,85,105,.10);
  --tag-border: rgba(71,85,105,.16);
  --scroll-thumb: #cbd5e1;
  --scroll-thumb-hover: #94a3b8;
  --scroll-radius: 4px;
  --scrim: rgba(15,23,42,.40);
  --overlay-bg: rgba(248,250,252,.70);
  --spinner-track: rgba(15,23,42,.12);
  --overlay-text: #0f172a;
  --sh-1: 0 1px 2px rgba(15,23,42,.05);
  --sh-2: 0 8px 24px rgba(15,23,42,.08);
  --sh-card: 0 1px 2px rgba(15,23,42,.04), 0 2px 8px rgba(15,23,42,.05);
  --sh-card-hover: 0 4px 16px rgba(15,23,42,.10);
  --sh-btn-primary: 0 1px 2px rgba(15,23,42,.10), inset 0 1px 0 rgba(255,255,255,.5);
  --sh-btn-primary-hover: 0 4px 14px rgba(37,99,235,.28), inset 0 1px 0 rgba(255,255,255,.5);
  --tab-active-text: #0f172a;
  /* 派生 */
  --input-border: var(--border-2);
  --input-border-hover: #94a3b8;
  --topbar-shadow: var(--sh-1);
  --btn-primary-grad: linear-gradient(180deg, var(--accent) 0%, var(--accent-2) 100%);
  --btn-primary-grad-hover: linear-gradient(180deg, #2d6ee4 0%, var(--accent) 55%, var(--accent-2) 100%);
  --chip-base-bg: var(--panel-2); --chip-base-fg: #475569;
  --chip-get-bg: #ecfdf5; --chip-get-fg: #047857; --chip-get-bd: #a7f3d0;
  --chip-post-bg: #fffbeb; --chip-post-fg: #b45309; --chip-post-bd: #fde68a;
  --chip-put-bg: #eff6ff; --chip-put-fg: #2563eb; --chip-put-bd: #bfdbfe;
  --chip-delete-bg: #fef2f2; --chip-delete-fg: #b91c1c; --chip-delete-bd: #fecaca;
  --chip-patch-bg: #f5f3ff; --chip-patch-fg: #7c3aed; --chip-patch-bd: #ddd6fe;
  --env-none-fg: #475569;
  --tab-inactive-fg: #475569;
  --mock-off-fg: #475569;
  --oauth-hint-fg: var(--text-2);
  --oauth-ok-fg: #047857; --oauth-ok-bg: #ecfdf5; --oauth-ok-bd: #a7f3d0;
  --oauth-na-fg: #475569; --oauth-na-bg: #f1f5f9; --oauth-na-bd: #e2e8f0;
  --oauth-exp-fg: #b45309; --oauth-exp-bg: #fffbeb; --oauth-exp-bd: #fde68a;
  --oauth-err-fg: #b91c1c; --oauth-err-bg: #fef2f2; --oauth-err-bd: #fecaca;
}


/* ---------- 主题 4/4：显式深色（覆盖系统浅色时的媒体查询结果） ---------- */
[data-theme="dark"] {
  --bg: #0f172a;
  --panel: #1e293b;
  --panel-2: #334155;
  --border: #334155;
  --border-2: #475569;
  --text: #f8fafc;
  --text-2: #b3c2d6;
  --muted: #9fb3c7;
  --accent: #60a5fa;
  --accent-2: #4c90f7;
  --accent-soft: rgba(59,130,246,.16);
  --accent-line: rgba(59,130,246,.45);
  --success: #34d399; --warning: #fbbf24; --danger: #f87171;
  --hover-weak: rgba(255,255,255,.06);
  --purple: #c084fc;
  --purple-weak: rgba(192,132,252,.16);
  --danger-strong: #ff8a80;
  --danger-weak: rgba(248,113,113,.14);
  --danger-weak-2: rgba(248,113,113,.28);
  --danger-weak-3: rgba(179,38,30,.25);
  --danger-weak-4: rgba(179,38,30,.14);
  --danger-line: rgba(248,113,113,.45);
  --success-weak: rgba(52,211,153,.16);
  --success-weak-2: rgba(30,126,52,.25);
  --success-line: rgba(52,211,153,.45);
  --warning-weak: rgba(251,191,36,.12);
  --on-success: #06281d;
  --code-bg: #0a1120;
  --input-bg: #0c1526;
  --caret: #e2e8f0;
  --hl-k: #7dd3fc;
  --hl-s: #86efac;
  --hl-n: #fbbf24;
  --hl-b: #c084fc;
  --hl-p: #94a3b8;
  --tag-bg: rgba(148,163,184,.18);
  --tag-border: rgba(148,163,184,.14);
  --scroll-thumb: #475569;
  --scroll-thumb-hover: #64748b;
  --scrim: rgba(2,6,23,.64);
  --overlay-bg: rgba(2,6,23,.55);
  --spinner-track: rgba(255,255,255,.18);
  --overlay-text: rgba(255,255,255,.92);
  --sh-1: 0 1px 3px rgba(2,6,23,.35);
  --sh-2: 0 12px 32px rgba(2,6,23,.5);
  --sh-card: 0 1px 3px rgba(2,6,23,.3), 0 10px 28px rgba(2,6,23,.28);
  --sh-card-hover: 0 4px 16px rgba(2,6,23,.4);
  --sh-btn-primary: 0 1px 2px rgba(2,6,23,.45), inset 0 1px 0 rgba(255,255,255,.14);
  --sh-btn-primary-hover: 0 4px 14px rgba(59,130,246,.35), inset 0 1px 0 rgba(255,255,255,.14);
  --tab-active-text: var(--white);
  /* 派生（与 :root 深色一致，覆盖系统浅色时的媒体查询值） */
  --input-border: var(--border);
  --input-border-hover: var(--border-2);
  --topbar-shadow: none;
  --scroll-radius: 6px;
  --btn-primary-grad: linear-gradient(180deg, #2563eb 0%, #1e55d4 55%, #1d4ed8 100%);
  --btn-primary-grad-hover: linear-gradient(180deg, #2d6ee4 0%, #2563eb 55%, #1d4ed8 100%);
  --chip-base-bg: var(--panel-2); --chip-base-fg: var(--muted);
  --chip-get-bg: var(--success-weak); --chip-get-fg: var(--success); --chip-get-bd: transparent;
  --chip-post-bg: var(--warning-weak); --chip-post-fg: var(--warning); --chip-post-bd: transparent;
  --chip-put-bg: var(--accent-soft); --chip-put-fg: var(--accent); --chip-put-bd: transparent;
  --chip-delete-bg: var(--danger-weak); --chip-delete-fg: var(--danger); --chip-delete-bd: transparent;
  --chip-patch-bg: var(--purple-weak); --chip-patch-fg: var(--purple); --chip-patch-bd: transparent;
  --env-none-fg: var(--muted);
  --tab-inactive-fg: var(--muted);
  --mock-off-fg: var(--muted);
  --oauth-hint-fg: var(--text);
  --oauth-ok-fg: #86efac; --oauth-ok-bg: rgba(34,197,94,.14); --oauth-ok-bd: rgba(34,197,94,.40);
  --oauth-na-fg: #e2e8f0; --oauth-na-bg: rgba(148,163,184,.14); --oauth-na-bd: rgba(148,163,184,.35);
  --oauth-exp-fg: #fde68a; --oauth-exp-bg: rgba(245,158,11,.14); --oauth-exp-bd: rgba(245,158,11,.40);
  --oauth-err-fg: #fca5a5; --oauth-err-bg: rgba(239,68,68,.14); --oauth-err-bd: rgba(239,68,68,.40);
}


/* 输入控件：主题底 + 边框 + 聚焦光环 */
.rf-input, .rf-textarea {
  appearance: none; height: 32px; padding: 0 12px;
  background: var(--input-bg); border: 1px solid var(--input-border);
  border-radius: var(--r-s); color: var(--text);
  transition: border-color .12s ease, box-shadow .15s ease;
}
.rf-input::placeholder, .rf-textarea::placeholder { color: var(--muted); }
.rf-input:hover, .rf-textarea:hover { border-color: var(--input-border-hover); }
.rf-input:focus, .rf-textarea:focus {
  outline: none; border-color: var(--accent);
  box-shadow: 0 0 0 3px var(--accent-soft);
}
.rf-textarea { height: auto; padding: 8px 12px; resize: vertical;
  font-family: var(--mono); }


/* 按钮 */
.rf-btn {
  display: inline-flex; align-items: center; justify-content: center; gap: 6px;
  height: 32px; padding: 0 14px; border-radius: var(--r-s);
  border: 1px solid var(--border); font-weight: 500; cursor: pointer;
  user-select: none; background: var(--panel-2); color: var(--text);
  transition: background .12s ease, border-color .12s ease, color .12s ease,
    transform .08s ease, box-shadow .15s ease;
}
.rf-btn:hover { background: var(--border-2); border-color: var(--border-2); }
.rf-btn:active { transform: translateY(1px) scale(.99); }
.rf-btn:disabled { opacity: .5; cursor: not-allowed; }
/* 主按钮：主题渐变 + hover 上浮 + 品牌阴影 */
.rf-btn-primary {
  background: var(--btn-primary-grad);
  color: var(--white); border-color: transparent;
  box-shadow: var(--sh-btn-primary);
}
.rf-btn-primary:hover {
  background: var(--btn-primary-grad-hover);
  border-color: transparent;
  transform: translateY(-1px);
  box-shadow: var(--sh-btn-primary-hover);
}
.rf-btn-primary:active {
  transform: translateY(0);
  box-shadow: var(--sh-btn-primary);
}
.rf-btn-ghost { background: transparent; border-color: var(--border); color: var(--text-2); }
.rf-btn-ghost:hover { background: var(--panel-2); color: var(--text); border-color: var(--border-2); }
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
  animation: rf-pop-in .15s ease;
}
.rf-dropdown-item {
  display: flex; align-items: center; gap: 8px; padding: 6px 10px;
  border-radius: 6px; cursor: pointer; color: var(--text-2);
}
.rf-dropdown-item:hover { background: var(--panel-2); color: var(--text); }
.rf-dropdown-item.selected { color: var(--accent); }
.rf-dropdown-footer { padding: 6px 4px 2px; border-top: 1px solid var(--border); }
.rf-dropdown-action {
  display: block; width: 100%; text-align: center; padding: 8px 10px;
  border: none; border-radius: 6px; background: transparent;
  color: var(--accent); cursor: pointer; font-size: 13px; font-weight: 500;
}
.rf-dropdown-action:hover { background: var(--accent-soft); }


/* 顶栏：三栏 Flex（左 / 中 / 右），浅色下抬升一档海拔 */
.rf-topbar {
  height: 48px; display: flex; align-items: center;
  padding: 0 16px; background: var(--panel);
  border-bottom: 1px solid var(--border);
  box-shadow: var(--topbar-shadow);
}
.rf-logo { display: inline-flex; align-items: center; gap: 8px; font-size: 15px;
  font-weight: 700; color: var(--text); cursor: pointer; white-space: nowrap; }
.rf-logo svg { width: 16px; height: 16px; color: var(--accent); flex: 0 0 auto; }
.rf-topbar-sep { width: 1px; height: 20px; background: var(--border); }

/* 左侧：Logo + 面包屑 */
.tb-left { display: flex; align-items: center; gap: 12px; flex: 0 0 auto;
  min-width: 0; }
.tb-breadcrumb { display: flex; align-items: center; gap: 6px;
  font-size: 13.5px; font-weight: 700; color: var(--accent);
  white-space: nowrap; overflow: hidden; }
.tb-breadcrumb-sep { color: var(--muted); font-weight: 400; }
.tb-breadcrumb-current { overflow: hidden; text-overflow: ellipsis; }

/* 中间：全局搜索（居中，max-width 480px） */
.tb-center { flex: 1 1 auto; min-width: 0; display: flex; justify-content: center;
  padding: 0 24px; }
.tb-search { position: relative; width: 100%; max-width: 480px; }
.tb-search svg { position: absolute; left: 12px; top: 50%; translate: 0 -50%;
  width: 15px; height: 15px; color: var(--muted); pointer-events: none; }
.topbar-search-input { width: 100%; height: 32px; padding: 0 64px 0 34px;
  background: var(--panel); border-color: var(--border-2); color: var(--text);
  font-size: 13px; }
.topbar-search-input::placeholder { color: var(--muted); }
.topbar-search-input:focus { background: var(--panel); border-color: var(--accent);
  outline: none; box-shadow: 0 0 0 3px var(--accent-soft); }

/* 快捷键胶囊：右内侧悬浮，不拦截输入（视觉复用 .ws-kbd） */
.tb-kbd { position: absolute; right: 8px; top: 50%; translate: 0 -50%;
  pointer-events: none; user-select: none; }

/* 右侧：反馈 / 设置 */
.tb-right { display: flex; align-items: center; gap: 8px; flex: 0 0 auto; }
.tb-btn { display: inline-flex; align-items: center; justify-content: center;
  height: 30px; padding: 0 8px; font-size: 13px; }
.tb-btn svg { width: 14px; height: 14px; }


/* 卡片 / 空状态 / 表单行 */
.rf-card { background: var(--panel); border: 1px solid var(--border-2);
  border-radius: var(--r-l);
  box-shadow: var(--sh-card); }
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

/* 项目数据加载遮罩（Spinner，避免加载期间空白页 / 误操作） */
.rf-loading-overlay {
  position: fixed; inset: 0; z-index: 200;
  display: flex; flex-direction: column; align-items: center; justify-content: center;
  gap: 12px; font-size: 13px; color: var(--muted);
  background: var(--overlay-bg);
  backdrop-filter: blur(2px);
}
.rf-spinner {
  width: 30px; height: 30px; border-radius: 50%;
  border: 3px solid var(--border); border-top-color: var(--accent);
  animation: rf-spin 0.8s linear infinite;
}
@keyframes rf-spin {
  to { transform: rotate(360deg); }
}
.rf-form-row { display: flex; gap: 8px; }
.rf-form-row .rf-input { flex: 1; min-width: 0; }
.rf-hint { margin-top: 14px; display: flex; flex-wrap: wrap; gap: 4px 6px;
  align-items: center; justify-content: center;
  font-size: 12px; color: var(--muted); }
.rf-kbd { padding: 1px 6px; border-radius: 4px; border: 1px solid var(--border);
  border-bottom-width: 2px; background: var(--panel-2); color: var(--text-2);
  font: 11px var(--mono); }


/* Toast
   右下角堆叠（列反向，新 Toast 紧贴容器底部，旧 Toast 依次向上）。
   入场从右滑入 200ms；消失前由组件加 .fading 类播放 300ms 淡出。
   z-index 全站最高（Modal 500 / Dropdown 50 / Loading 200 均在其下）。 */
.rf-toast-wrap {
  position: fixed; bottom: 16px; right: 16px; z-index: 9999;
  display: flex; flex-direction: column-reverse; gap: 8px;
  align-items: flex-end; pointer-events: none;
}
.rf-toast { display: flex; align-items: center; gap: 8px; padding: 8px 12px;
  background: var(--panel); border: 1px solid var(--border-2);
  border-radius: var(--r-m); box-shadow: var(--sh-2); pointer-events: auto;
  animation: rf-toast-in .2s ease; }
.rf-toast.fading { animation: rf-toast-out .3s ease forwards; }
@keyframes rf-toast-in { from { opacity: 0; transform: translateX(16px); }
  to { opacity: 1; transform: none; } }
@keyframes rf-toast-out { from { opacity: 1; } to { opacity: 0; } }
.rf-toast-error { border-color: var(--danger-line); }
.rf-toast-success { border-color: var(--success-line); }


/* 滚动条：细窄（6px）+ 融入主题（浅色 thumb #cbd5e1、hover #94a3b8、圆角 4px） */
::-webkit-scrollbar { width: 6px; height: 6px; }
::-webkit-scrollbar-thumb { background: var(--scroll-thumb); border-radius: var(--scroll-radius); }
::-webkit-scrollbar-thumb:hover { background: var(--scroll-thumb-hover); }
::-webkit-scrollbar-track { background: transparent; }
:focus-visible { outline: none; box-shadow: 0 0 0 3px var(--accent-soft); }
.rf-method { font-weight: 700; font-size: 12px; }
.rf-method-get { color: var(--success); }
.rf-method-post { color: var(--warning); }
.rf-method-put { color: var(--accent); }
.rf-method-delete { color: var(--danger); }
.rf-method-patch { color: var(--purple); }


/* ============ 扩展变量 / 组件样式 ============ */

:root {
  /* 间距（4px 网格） */
  --s-1: 4px; --s-2: 8px; --s-3: 12px; --s-4: 16px; --s-5: 20px;
  --s-6: 24px; --s-7: 32px; --s-8: 40px; --s-9: 48px;
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
  border: 1px solid var(--border-2); border-radius: var(--r-l); padding: var(--s-7);
  box-shadow: var(--sh-card); }
.settings-header { display: flex; align-items: center; justify-content: space-between;
  gap: var(--s-3); margin-bottom: var(--s-3); }
.settings-header h2 { margin: 0; }
.settings-close { flex: 0 0 auto; color: var(--muted); }
.settings-close:hover { color: var(--danger); }
.settings-close svg { width: 14px; height: 14px; }
.hint { color: var(--muted); font-size: 12px; text-align: center; margin-top: var(--s-4); }
.empty { text-align: center; color: var(--muted); padding: var(--s-8) 0; font-size: 13px; }
a { color: var(--accent); cursor: pointer; text-decoration: none; }
a:hover { text-decoration: underline; }
code, pre { font-family: var(--mono); }
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
.rf-btn-danger { background: var(--danger); color: var(--white); font-weight: 600; }
.rf-btn-danger:hover { filter: brightness(1.06); }

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
/* 请求 Body 语法高亮编辑区：透明 textarea + 底层高亮 pre 叠加 */
.body-editor-fl { position: relative; width: 100%; }
.body-editor-fl .rf-body-editor {
  height: auto; min-height: 220px;
  width: 100%;
  color: transparent; caret-color: var(--caret);
  background: transparent;
  position: relative; z-index: 2;
  font-size: 13px; line-height: 1.5;
  border: 0;
}
.body-editor-hl {
  position: absolute; top: 0; left: 0; right: 0; bottom: 0;
  margin: 0; padding: 8px 12px;
  font-family: var(--mono); font-size: 13px; line-height: 1.5;
  color: var(--text); user-select: none;
  white-space: pre-wrap; word-break: break-word; word-wrap: break-word;
  pointer-events: none; overflow: auto; z-index: 1;
  scrollbar-width: none; -ms-overflow-style: none;
}
.body-editor-hl::-webkit-scrollbar { display: none; }
/* 未匹配到任何语法类别的字符（编辑中途的非法 JSON / 普通文本）以正文色显示 */
.body-editor-fl .rf-body-editor::selection {
  color: var(--text); background: var(--accent-soft);
}
.body-editor-hl .hl-k { color: var(--hl-k); }
.body-editor-hl .hl-s { color: var(--hl-s); }
.body-editor-hl .hl-n { color: var(--hl-n); }
.body-editor-hl .hl-b { color: var(--hl-b); }
.body-editor-hl .hl-p { color: var(--hl-p); }
.body-editor-hl .hl-c { color: var(--muted); font-style: italic; }
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
.rf-project-card { background: var(--panel); border: 1px solid var(--border-2);
  border-radius: var(--r-l); padding: var(--s-4); cursor: pointer;
  transition: border-color .12s ease, box-shadow .15s ease, transform .08s ease; }
.rf-project-card:hover { border-color: var(--accent-line);
  box-shadow: var(--sh-card-hover); transform: translateY(-1px); }
.rf-project-card:active { transform: translateY(0); }
.rf-project-name { font-size: 15px; font-weight: 600; }
.rf-project-desc { color: var(--muted); margin-top: var(--s-1); font-size: 13px;
  overflow: hidden; text-overflow: ellipsis; display: -webkit-box;
  -webkit-line-clamp: 2; -webkit-box-orient: vertical; }
.rf-project-meta { display: flex; align-items: center; gap: var(--s-2);
  margin-top: var(--s-3); font-size: 12px; color: var(--muted); }

/* 请求 URL 预览：环境名 + 拼接后的完整地址（base_url 高亮） */
.url-preview { display: flex; align-items: center; gap: 8px;
  min-height: 22px; padding: 0 2px; font-size: 12px; }
.url-preview-env { flex: 0 0 auto; padding: 1px 8px; border-radius: 999px;
  background: var(--accent-soft); color: var(--accent);
  border: 1px solid var(--accent-line); font-weight: 600;
  white-space: nowrap; user-select: none; }
.url-preview-env.none { background: var(--panel-2); color: var(--env-none-fg);
  border-color: var(--border); font-weight: 400; }
.url-preview-base { color: var(--accent); opacity: .9; font-family: var(--mono);
  user-select: none; }
.url-preview-rest { color: var(--text-2); font-family: var(--mono);
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }

/* 侧边栏：Header / Toolbar / Search / Tree / Footer 五段式 Flex 布局 */
.sidebar { width: 288px; flex: 0 0 288px; background: var(--panel);
  border-right: 1px solid var(--border); display: flex; flex-direction: column;
  min-height: 0; overflow: hidden; height: 100vh; }

/* 顶部固定区：项目选择器 */
.sb-header { flex: 0 0 auto; display: flex; align-items: center; gap: var(--s-2);
  padding: var(--s-3); border-bottom: 1px solid var(--border); }
.sb-project-icon { display: inline-flex; align-items: center; color: var(--accent);
  flex: 0 0 auto; }
.sb-project-icon svg { width: 18px; height: 18px; }
.sb-project-dropdown .rf-dropdown-trigger { height: 36px; width: 100%;
  background: var(--accent-soft); border-color: var(--accent-line);
  font-weight: 600; }
.sb-project-dropdown .rf-dropdown-trigger:hover { border-color: var(--accent);
  background: var(--accent-soft); }
.sb-project-dropdown .rf-dropdown-value { color: var(--accent); font-size: 13.5px; }
.sb-project-dropdown .rf-caret { color: var(--accent); }

/* 工具栏：两个按钮平分宽度 */
.sb-toolbar { flex: 0 0 auto; display: flex; gap: var(--s-2);
  padding: var(--s-2) var(--s-3); border-bottom: 1px solid var(--border); }
.sb-toolbar-btn { flex: 1 1 0; min-width: 0; height: 28px; display: inline-flex;
  align-items: center; justify-content: center; gap: 4px; font-size: 12.5px;
  color: var(--text-2); background: var(--panel-2); border: 1px solid var(--border);
  border-radius: var(--r-s); cursor: pointer; white-space: nowrap;
  transition: background .12s, border-color .12s, color .12s; }
.sb-toolbar-btn:hover { background: var(--hover-weak); color: var(--text);
  border-color: var(--border-2); }
.sb-toolbar-btn:active { background: var(--accent-soft); }
.sb-toolbar-btn-primary { background: var(--accent); border-color: transparent;
  color: var(--white); font-weight: 600;
  box-shadow: var(--sh-btn-primary); }
.sb-toolbar-btn-primary:hover { background: var(--accent-2); color: var(--white);
  border-color: transparent; }
.sb-toolbar-btn-primary:active { background: var(--accent); }
.sb-toolbar-btn svg { width: 12px; height: 12px; }
.sb-toolbar-btn-ghost { background: transparent; border-color: transparent; }
.sb-toolbar-btn-ghost:hover { background: var(--hover-weak);
  border-color: transparent; }
.sb-toolbar-btn-ghost:active { background: var(--accent-soft);
  border-color: transparent; }

/* 「＋ 接口」下拉（HTTP 接口 / 从 cURL 导入） */
.sb-tool-group { position: relative; flex: 1 1 0; min-width: 0;
  display: flex; }
.sb-tool-group .sb-toolbar-btn { flex: 1 1 0; }
.sb-menu-backdrop { position: fixed; inset: 0; z-index: 490; }
.sb-menu { position: absolute; top: calc(100% + 4px); left: 0; z-index: 510;
  min-width: 100%; background: var(--panel); border: 1px solid var(--border-2);
  border-radius: var(--r-s); box-shadow: var(--sh-2); padding: var(--s-1);
  display: flex; flex-direction: column; gap: var(--s-1); }
.sb-menu-item { text-align: left; padding: var(--s-1) var(--s-2);
  background: transparent; border: none; color: var(--text); cursor: pointer;
  font-size: 13px; border-radius: var(--r-s); white-space: nowrap; }
.sb-menu-item:hover { background: var(--accent-soft); }

/* 搜索框：图标 + 略深输入底 */
.sb-search { flex: 0 0 auto; position: relative; padding: var(--s-2) var(--s-3);
  border-bottom: 1px solid var(--border); }
.sb-search svg { position: absolute; left: calc(var(--s-3) + 8px); top: 50%;
  translate: 0 -50%; width: 15px; height: 15px; color: var(--muted); pointer-events: none; }
.sb-search-input { width: 100%; height: 28px; padding: 0 10px 0 30px;
  background: var(--panel-2); border-color: var(--border); font-size: 13px;
  color: var(--text); }
.sb-search-input::placeholder { color: var(--muted); }
.sb-search-input:focus { background: var(--panel-2); border-color: var(--accent);
  outline: none; }

/* 内容区：树形滚动 */
.sb-tree { flex: 1 1 auto; min-height: 0; overflow-y: auto; overflow-x: hidden;
  padding: var(--s-1) 0 var(--s-2); }
.sb-empty { padding: var(--s-4) var(--s-3); color: var(--muted);
  font-size: 13px; text-align: center; }

/* 底部固定区：环境选择器 */
.sb-footer { flex: 0 0 auto; display: flex; align-items: center;
  padding: var(--s-3); border-top: 1px solid var(--border); }
.sb-env-dropdown .rf-dropdown-trigger { height: 32px; width: 100%; }
.sb-env-dropdown .rf-dropdown-value { font-size: 13px; }
.sb-width { flex: 1 1 0; min-width: 0; }
.tree-item { position: relative; padding: var(--s-1) var(--s-3); cursor: pointer;
  display: flex; align-items: center; gap: var(--s-2); border-radius: var(--r-s);
  margin: 1px var(--s-2); user-select: none; transition: background .12s; }
.tree-item:hover { background: var(--accent-soft); }
.tree-item:active { background: var(--accent-soft); }
.tree-item.draggable { cursor: grab; }
.tree-item.dragging { opacity: .4; }
.tree-item.no-pointer { pointer-events: none; }
.tree-item.drop-over { background: var(--accent-soft);
  box-shadow: inset 0 0 0 1px var(--accent); }
.tree-root-drop-target { position: relative; }
.tree-item.selected { background: var(--accent-soft); }
.tree-item.selected::before { content: ""; position: absolute; left: 0;
  top: 8px; bottom: 8px; width: 2px; border-radius: 1px; background: var(--accent); }
.tree-item .name { flex: 1; overflow: hidden; text-overflow: ellipsis;
  white-space: nowrap; font-size: 13px; }
.tree-actions { display: none; gap: var(--s-1); }
.tree-item:hover .tree-actions { display: flex; }
.rf-tree-action { display: inline-flex; align-items: center; justify-content: center;
  background: transparent; border: none; color: var(--muted);
  cursor: pointer; font-size: 12px; border-radius: var(--r-s); padding: 2px 5px; }
.rf-tree-action svg { width: 12px; height: 12px; }
.rf-tree-action:hover { color: var(--text); background: var(--hover-weak); }
.rf-tree-action:active { background: var(--accent-soft); }
.rf-tree-action-danger:hover { color: var(--danger); background: var(--danger-weak); }
.kv-table .rf-tree-action:hover { color: var(--danger); background: var(--danger-weak); }
/* 侧边栏空状态：图标更醒目 */
.sb-tree-empty { padding: 40px 16px; }
.tree-more-menu .rf-dd-danger:hover { background: var(--danger-weak); color: var(--danger); }

/* HTTP 方法徽章：主题派生色（浅色饱和底 + 同族 1px 边框） */
.rf-method-chip { min-width: 46px; text-align: center; padding: 2px var(--s-1);
  border-radius: var(--r-s); font-size: 11px; font-weight: 700;
  border: 1px solid transparent;
  background: var(--chip-base-bg); color: var(--chip-base-fg); }
.rf-method-chip-get { background: var(--chip-get-bg); color: var(--chip-get-fg);
  border-color: var(--chip-get-bd); }
.rf-method-chip-post { background: var(--chip-post-bg); color: var(--chip-post-fg);
  border-color: var(--chip-post-bd); }
.rf-method-chip-put { background: var(--chip-put-bg); color: var(--chip-put-fg);
  border-color: var(--chip-put-bd); }
.rf-method-chip-delete { background: var(--chip-delete-bg); color: var(--chip-delete-fg);
  border-color: var(--chip-delete-bd); }
.rf-method-chip-patch { background: var(--chip-patch-bg); color: var(--chip-patch-fg);
  border-color: var(--chip-patch-bd); }

/* 弹窗：入场 150ms（fade + 轻微放大），出场 100ms 反向（.modal-exit） */
.modal-backdrop { position: fixed; inset: 0; z-index: 500; background: var(--scrim);
  backdrop-filter: blur(3px); display: flex; align-items: center; justify-content: center;
  animation: rf-fade-in .15s ease; }
.modal { min-width: 400px; max-width: 560px; background: var(--panel);
  border: 1px solid var(--border-2); border-radius: var(--r-l); padding: var(--s-5);
  box-shadow: var(--sh-2); animation: rf-pop-in .15s ease; }
.modal-backdrop.modal-exit { animation: rf-fade-out .1s ease forwards; }
.modal.modal-exit { animation: rf-modal-out .1s ease forwards; }
@keyframes rf-fade-in { from { opacity: 0; } to { opacity: 1; } }
@keyframes rf-pop-in { from { opacity: 0; transform: scale(.98); }
  to { opacity: 1; transform: none; } }
@keyframes rf-fade-out { from { opacity: 1; } to { opacity: 0; } }
@keyframes rf-modal-out { from { opacity: 1; transform: none; }
  to { opacity: 0; transform: scale(.98); } }
.modal h3 { margin-bottom: var(--s-3); font-size: 15px; font-weight: 650; }
.confirm-message { margin: 0 0 var(--s-2); color: var(--muted); font-size: 13px; line-height: 1.6; }
.rf-modal-actions { display: flex; justify-content: flex-end; gap: var(--s-2);
  margin-top: var(--s-3); }
.history-modal { display: flex; flex-direction: column; max-height: 74vh; width: 680px; }

/* 右侧抽屉（请求历史） */
.rf-drawer-backdrop { position: fixed; inset: 0; z-index: 480;
  background: var(--scrim); animation: rf-fade .15s ease; }
@keyframes rf-fade { from { opacity: 0; } to { opacity: 1; } }
.rf-drawer { position: fixed; top: 0; right: 0; bottom: 0; width: 420px; z-index: 490;
  background: var(--panel); border-left: 1px solid var(--border-2);
  box-shadow: var(--sh-2); display: flex; flex-direction: column;
  animation: rf-slide-in .18s ease; }
@keyframes rf-slide-in { from { transform: translateX(28px); opacity: 0; }
  to { transform: none; opacity: 1; } }
.rf-drawer-head { display: flex; align-items: center; gap: var(--s-2);
  padding: var(--s-3) var(--s-4); border-bottom: 1px solid var(--border); }
.rf-drawer-head h3 { font-size: 14px; margin: 0; flex: 1; }
.rf-drawer-body { flex: 1; overflow-y: auto; padding: var(--s-2); }
.update-modal { display: flex; flex-direction: column; max-height: 78vh; width: 560px; }
.update-notes { flex: 1; overflow-y: auto; min-height: 120px; max-height: 40vh;
  background: var(--panel-2); border: 1px solid var(--border); border-radius: var(--r-s);
  padding: var(--s-3); white-space: pre-wrap; word-break: break-word; font-size: 13px; }
.update-progress { display: flex; flex-direction: column; gap: var(--s-2); margin-top: var(--s-3); }
.update-bar { height: 8px; background: var(--panel-2); border: 1px solid var(--border);
  border-radius: var(--r-l); overflow: hidden; }
.update-bar-fill { height: 100%; background: var(--accent); transition: width .15s; }
.update-error { margin-top: var(--s-2); color: var(--danger); font-size: 13px; }
.about-table { display: flex; flex-direction: column; gap: var(--s-1); margin-bottom: var(--s-3); }
.about-table .row { gap: var(--s-3); }
.about-table .label-hint { min-width: 90px; }
.about-value { font-size: 13px; word-break: break-all; }
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
/* 请求栏：分段控件（方法 | URL | 发送）+ 右侧图标按钮 */
.editor .url-bar { display: flex; gap: var(--s-2); padding: var(--s-2) var(--s-3);
  border-bottom: 1px solid var(--border); align-items: center; }
.ub-group { display: flex; align-items: stretch; flex: 1; min-width: 0; height: 34px;
  background: var(--panel); border: 1px solid var(--border-2);
  border-radius: var(--r-s); }
.ub-group .rf-dropdown-trigger { height: 32px; border: none; border-radius: 0;
  border-right: 1px solid var(--border); font-weight: 700; font-size: 13px; }
.ub-group .rf-dd-method { width: auto; min-width: 108px; }
.ub-group .rf-dd-method-get .rf-dropdown-trigger { background: var(--chip-get-bg);
  color: var(--chip-get-fg); }
.ub-group .rf-dd-method-post .rf-dropdown-trigger { background: var(--chip-post-bg);
  color: var(--chip-post-fg); }
.ub-group .rf-dd-method-put .rf-dropdown-trigger { background: var(--chip-put-bg);
  color: var(--chip-put-fg); }
.ub-group .rf-dd-method-delete .rf-dropdown-trigger { background: var(--chip-delete-bg);
  color: var(--chip-delete-fg); }
.ub-group .rf-dd-method-patch .rf-dropdown-trigger { background: var(--chip-patch-bg);
  color: var(--chip-patch-fg); }
.ub-group .rf-dd-method-get .rf-dropdown-trigger:hover,
.ub-group .rf-dd-method-post .rf-dropdown-trigger:hover,
.ub-group .rf-dd-method-put .rf-dropdown-trigger:hover,
.ub-group .rf-dd-method-delete .rf-dropdown-trigger:hover,
.ub-group .rf-dd-method-patch .rf-dropdown-trigger:hover { filter: brightness(.97); }
.ub-url-input { flex: 1; min-width: 0; height: 32px; border: none; border-radius: 0;
  background: transparent; font-family: var(--mono); }
.ub-url-input:focus { border-color: transparent; box-shadow: none;
  background: var(--accent-soft); }
.ub-base-url { display: flex; align-items: center; padding: 0 12px;
  font-family: var(--mono); font-size: 13px; color: var(--accent);
  background: var(--accent-soft); border-right: 1px solid var(--border);
  white-space: nowrap; overflow: hidden; text-overflow: ellipsis;
  user-select: none; }
.ub-send { border: none; border-left: 1px solid var(--border); cursor: pointer;
  display: inline-flex; align-items: center; gap: 6px; padding: 0 18px;
  height: 32px; box-sizing: border-box;
  background: var(--success); color: var(--on-success); font-size: 13px;
  font-weight: 700; transition: filter .12s; flex-shrink: 0; }
.ub-send:hover { filter: brightness(1.07); }
.ub-send:active { filter: brightness(.96); }
.ub-send:disabled { opacity: .8; cursor: not-allowed; }
.ub-send svg { width: 14px; height: 14px; }
/* 右侧：导入 cURL / 保存 / 生成代码（ghost 图标按钮，间距 8px） */
.ub-actions { display: flex; gap: var(--s-2); flex: 0 0 auto; }
.ub-btn { width: 32px; padding: 0; flex: 0 0 auto; }
.ub-btn svg { width: 14px; height: 14px; }

/* 名称 + 描述：grid 两列等高 32px，描述聚焦后展开为多行 */
.editor-meta { display: grid; grid-template-columns: 1fr 1fr; gap: var(--s-2);
  padding: var(--s-2) var(--s-3); border-bottom: 1px solid var(--border);
  align-items: center; }
.editor-meta .rf-input { width: 100%; min-width: 0; height: 32px;
  box-sizing: border-box; }
.editor-meta .rf-textarea { grid-column: 1 / -1; }
.editor .tabs { display: flex; border-bottom: 1px solid var(--border);
  padding: 0 var(--s-2); gap: 2px; }
/* 区域 Tab：激活下划线 2px，::after + transition 滑动动画 */
.rf-tab { position: relative; background: transparent; border: none;
  padding: var(--s-2) var(--s-3); color: var(--muted); cursor: pointer; font-size: 13px;
  display: inline-flex; align-items: center; gap: 6px;
  border-radius: var(--r-s) var(--r-s) 0 0; transition: color .12s, background .12s; }
.rf-tab::after { content: ''; position: absolute; left: 10px; right: 10px; bottom: -1px;
  height: 2px; border-radius: 2px 2px 0 0; background: var(--accent);
  transform: scaleX(0); transform-origin: center;
  transition: transform .18s ease; }
.rf-tab:hover { color: var(--text); background: var(--accent-soft); }
.rf-tab.active { color: var(--tab-active-text); font-weight: 600; }
.rf-tab.active::after { transform: scaleX(1); }
.rf-tab-badge { min-width: 18px; height: 18px; padding: 0 5px; border-radius: 9px;
  background: var(--panel-2); color: var(--text-2); font-size: 11px; font-weight: 600;
  display: inline-flex; align-items: center; justify-content: center; }
.rf-tab.active .rf-tab-badge { background: var(--accent-soft); color: var(--accent); }
/* 右侧时钟图标按钮 */
.rf-tab-icon { padding: var(--s-2); margin-left: var(--s-1); }
.rf-tab-icon::after { display: none; }
.rf-tab-icon svg { width: 15px; height: 15px; }
.tab-body { flex: 1; overflow-y: auto; padding: var(--s-3) var(--s-4); }
.auth-field { display: flex; align-items: center; gap: var(--s-1); margin-bottom: var(--s-2); }
.auth-field .rf-input { flex: 1; min-width: 0; }

/* OAuth2 状态指示器（主题派生色） */
.oauth-status { display: flex; align-items: center; margin-bottom: var(--s-2); }
.oauth-badge { display: inline-flex; align-items: center; gap: 6px; padding: 3px 10px;
  border-radius: 999px; font-size: 12px; font-weight: 500; }
.oauth-valid { color: var(--oauth-ok-fg); background: var(--oauth-ok-bg);
  border: 1px solid var(--oauth-ok-bd); }
.oauth-unauthorized { color: var(--oauth-na-fg); background: var(--oauth-na-bg);
  border: 1px solid var(--oauth-na-bd); }
.oauth-expiring { color: var(--oauth-exp-fg); background: var(--oauth-exp-bg);
  border: 1px solid var(--oauth-exp-bd); }
.oauth-expired { color: var(--oauth-err-fg); background: var(--oauth-err-bg);
  border: 1px solid var(--oauth-err-bd); }
.oauth-hint { margin-top: var(--s-2); font-size: 12px; color: var(--oauth-hint-fg);
  line-height: 1.6; }

/* 多标签栏 */
.tab-bar { display: flex; align-items: flex-end; gap: var(--s-1); overflow: visible;
  padding: var(--s-2) var(--s-2) 0; border-bottom: 1px solid var(--border);
  background: var(--panel); }
.editor-tab { display: flex; align-items: center; gap: var(--s-1); padding: var(--s-2) var(--s-3);
  border: 1px solid var(--border); border-bottom: 1px solid var(--border);
  border-radius: var(--r-m) var(--r-m) 0 0;
  cursor: pointer; font-size: 13px; color: var(--tab-inactive-fg); background: var(--panel-2);
  max-width: 220px; white-space: nowrap; transition: color .12s, background .12s; }
.editor-tab:hover { color: var(--text); }
.editor-tab:active { background: var(--panel-2); }
/* 激活标签：背景与编辑区同色 + 底部透明连通 */
.editor-tab.active { background: var(--bg); color: var(--text); font-weight: 600;
  border-bottom-color: transparent; }
.editor-tab .tab-dirty { color: var(--warning); font-size: 10px; }
/* × 仅 hover 显示 */
.rf-tab-close { border: none; background: none; color: var(--muted); cursor: pointer;
  font-size: 14px; line-height: 1; padding: 2px var(--s-1); border-radius: var(--r-s);
  opacity: 0; transition: opacity .1s; }
.editor-tab:hover .rf-tab-close { opacity: 1; }
.rf-tab-close:hover { color: var(--text); background: var(--danger-weak-2); }
.rf-tab-close:active { background: var(--danger-weak); }

/* 新建接口按钮：加大尺寸，加号填充 */
.tab-add-btn {
  width: 34px; min-width: 34px; height: 34px; padding: 0;
  border-radius: var(--r-s); font-size: 18px; font-weight: 700;
  background: var(--accent-soft); border-color: var(--accent-line);
  color: var(--accent);
}
.tab-add-btn:hover { background: var(--accent); color: #fff; border-color: var(--accent); }
.tab-add-btn svg { width: 20px; height: 20px; }

/* 右侧环境下拉：推到最右 */
.tab-bar-right { margin-left: auto; }
.tab-env-dd .rf-dropdown-trigger { height: 32px; min-width: 120px; }

/* KeyValue 表 */
.kv-table { width: 100%; border-collapse: collapse; }
.kv-table th { color: var(--muted); text-align: left; font-weight: 600; font-size: 12px;
  padding: var(--s-1) var(--s-2); border-bottom: 1px solid var(--border);
  background: var(--panel-2); }
.kv-table td { padding: var(--s-1) var(--s-2); border-bottom: 1px solid var(--border);
  transition: background .1s; }
.kv-table tbody tr:hover td { background: var(--accent-soft); }
.kv-table td:first-child, .kv-table th:first-child { width: 44px; text-align: center; }
.kv-table .row-actions { width: 44px; text-align: center; }
/* 行内输入：默认透明无边框，hover 显边框，focus 显光环 */
.kv-table .rf-kv-input { background: transparent; border-color: transparent;
  box-shadow: none; height: 28px; padding: 0 8px; }
.kv-table .rf-kv-input:hover { border-color: var(--border-2); }
.kv-table .rf-kv-input:focus { background: var(--panel); border-color: var(--accent);
  box-shadow: 0 0 0 3px var(--accent-soft); }
/* 表下方追加按钮（已有行时仍可继续新增） */
.kv-add { margin-top: var(--s-2); display: flex; }
.kv-add .rf-btn { color: var(--text-2); }
/* 空表空状态 */
.rf-kv-empty { display: flex; flex-direction: column; align-items: center; gap: 8px;
  padding: 48px 16px; }

/* 响应区 */
.rf-resizer { height: 6px; flex-shrink: 0; cursor: row-resize; position: relative; }
.rf-resizer::after { content: ''; position: absolute; left: 0; right: 0; top: 50%;
  height: 2px; transform: translateY(-50%); border-radius: 2px;
  background: transparent; transition: background .12s; }
.rf-resizer:hover::after, .rf-resizer:active::after { background: var(--accent); }
.response { display: flex; flex-direction: column; flex: 1; min-height: 0;
  border-top: 1px solid var(--border); }
.resp-head { display: flex; gap: var(--s-2); align-items: center; padding: var(--s-2) var(--s-3);
  border-bottom: 1px solid var(--border); font-size: 13px; background: var(--panel); }
/* 状态码 pill：2xx 绿 / 4xx 橙 / 5xx 红 */
.rf-status-pill { display: inline-flex; align-items: center; height: 22px;
  padding: 0 10px; border-radius: 999px; font-size: 12px; font-weight: 700;
  font-family: var(--mono); }
.rf-status-2xx { background: var(--success-weak-2); color: var(--success); }
.rf-status-3xx { background: var(--panel-2); color: var(--text-2); }
.rf-status-4xx { background: var(--warning-weak); color: var(--warning); }
.rf-status-5xx { background: var(--danger-weak-3); color: var(--danger-strong); }
.resp-stats { display: flex; align-items: center; gap: var(--s-2);
  color: var(--text-2); font-size: 12px; }
.resp-ctype { color: var(--muted); font-size: 12px;
  overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 220px; }
/* Pretty / Raw 切换 */
.rf-seg { display: inline-flex; background: var(--panel-2);
  border: 1px solid var(--border); border-radius: var(--r-s); padding: 2px; gap: 2px; }
.rf-seg button { border: none; background: transparent; color: var(--text-2);
  font-size: 12px; padding: 2px 10px; border-radius: 6px; cursor: pointer; }
.rf-seg button:hover { color: var(--text); }
.rf-seg button.active { background: var(--panel); color: var(--text); font-weight: 600;
  box-shadow: var(--sh-1); }
/* 发送中 spinner */
.rf-spin-sm { width: 13px; height: 13px; border-radius: 50%;
  border: 2px solid currentColor; border-top-color: transparent;
  animation: rf-spin .7s linear infinite; display: inline-block; flex: 0 0 auto; }
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
.test-row .url { color: var(--text); font-size: 13px; font-family: var(--mono); }
.test-row .test { color: var(--muted); font-size: 12px; }
.test-badge { font-size: 11px; padding: 2px var(--s-2); border-radius: var(--r-m);
  font-weight: 700; }
.test-badge.ok { background: var(--success-weak-2); color: var(--success); }
.test-badge.bad { background: var(--danger-weak-3); color: var(--danger-strong); }
.test-badge.skip { background: var(--panel-2); color: var(--muted); }
.test-fail { font-size: 12px; color: var(--danger-strong); margin-top: var(--s-1);
  font-family: var(--mono); background: var(--danger-weak); border-radius: var(--r-s);
  padding: var(--s-1) var(--s-2); }

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
.curl-modal { display: flex; flex-direction: column; max-height: 78vh; width: 680px; }
.curl-modal .hint { text-align: left; color: var(--muted); font-size: 12px;
  margin: 0 0 var(--s-2); line-height: 1.5; }
.curl-input { flex: 1; min-height: 200px; resize: vertical; font-family: var(--mono); }
.codegen-out { flex: 1; overflow: auto; background: var(--code-bg); color: var(--text);
  border-radius: var(--r-m); padding: var(--s-3) var(--s-4); font-size: 12.5px;
  line-height: 1.6; white-space: pre; user-select: text; border: 1px solid var(--border); }

/* Docs Tab */
.docs-meta { display: flex; align-items: center; gap: var(--s-2); margin-bottom: var(--s-2); }
.method-badge { font-size: 11px; font-weight: 700; color: var(--white); padding: 2px var(--s-2);
  border-radius: var(--r-s); background: var(--purple); min-width: 56px; text-align: center; }
.doc-path { font-family: var(--mono); font-size: 13px; color: var(--text); }
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
.mock-status.off { background: var(--panel-2); color: var(--mock-off-fg); }
.mock-rule-row { border: 1px solid var(--border); border-radius: var(--r-m);
  padding: var(--s-2) var(--s-3); margin-top: var(--s-2); background: var(--panel); }
.backup-box { margin-top: var(--s-2); }
.backup-box .rf-textarea { width: 100%; min-height: 160px; box-sizing: border-box;
  margin-top: var(--s-2); font-size: 12.5px; resize: vertical; }

/* Toast 补充 */
.rf-toast-info { border-color: var(--accent-line); }

/* 全局加载遮罩 */
.loading-overlay { position: fixed; inset: 0; z-index: 9999;
  background: var(--overlay-bg); backdrop-filter: blur(2px);
  display: flex; align-items: center; justify-content: center; }
.loading-box { display: flex; flex-direction: column; align-items: center;
  gap: 16px; }
.loading-spinner { width: 44px; height: 44px; border-radius: 50%;
  border: 4px solid var(--spinner-track); border-top-color: var(--accent);
  animation: rf-spin .8s linear infinite; }
@keyframes rf-spin { to { transform: rotate(360deg); } }
.loading-text { color: var(--overlay-text); font-size: 13.5px;
  letter-spacing: .3px; user-select: none; }

/* 工作区空状态 */
.ws-empty { flex: 1; min-height: 0; display: flex; flex-direction: column;
  align-items: center; justify-content: center; gap: var(--s-2);
  padding: var(--s-6); user-select: none; }
.ws-empty-icon { width: 120px; height: 120px; color: var(--accent);
  opacity: .14; margin-bottom: var(--s-2); }
.ws-empty-icon svg { width: 100%; height: 100%; }
.ws-empty-title { font-size: 18px; font-weight: 650; color: var(--text); }
.ws-empty-sub { font-size: 13px; color: var(--muted); }
.ws-empty-actions { display: flex; gap: var(--s-2); margin-top: var(--s-4); }
.ws-empty-actions .rf-btn { height: 34px; padding: 0 18px; }
.ws-empty-tips { margin-top: var(--s-8); padding: var(--s-3) var(--s-4);
  background: var(--panel); border: 1px solid var(--border);
  border-radius: var(--r-m); display: flex; flex-direction: column;
  gap: var(--s-2); align-items: center; }
.ws-empty-tips-title { font-size: 12px; font-weight: 600; color: var(--muted); }
.ws-empty-tips-row { display: flex; gap: var(--s-4); align-items: center;
  font-size: 12px; color: var(--text-2); flex-wrap: wrap; justify-content: center; }
.ws-kbd { padding: 1px 6px; border-radius: 4px; border: 1px solid var(--border);
  border-bottom-width: 2px; background: var(--panel-2); color: var(--text-2);
  font: 11px var(--mono); }

/* 微交互：系统开启「减弱动态效果」时关闭全部动画 / 过渡 */
@media (prefers-reduced-motion: reduce) {
  *, *::before, *::after {
    animation-duration: .01ms !important;
    animation-iteration-count: 1 !important;
    transition-duration: .01ms !important;
    scroll-behavior: auto !important;
  }
}
"#;
