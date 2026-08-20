-- 项目手动拖拽排序：sort_order 越大越靠后
ALTER TABLE projects ADD COLUMN sort_order INTEGER NOT NULL DEFAULT 0;
