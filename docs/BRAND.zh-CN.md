# 品牌资源

[English](BRAND.md)

Weftext 保留既定古文字“文”意象，使用墨蓝黑 `#07172D` 与暖纸色 `#F4EFE4`。图形不是特定甲骨文字形的严格复刻。技术重构和小尺寸适配不得改成 W 或占位图。

[主图形](../brand/logo-mark.svg)与[应用图标](../brand/app-icon.svg)是从项目留存品牌原件显式导入的原始 SVG；路径逐字保留，两者共享同一条 `evenodd` 主路径。主图形使用 `currentColor`，可在浅色背景用墨蓝黑、深色背景用暖纸色；应用图标有原定圆角底色，角外透明。私有历史记录没有导入产品库。

应用图标是交付资源的生成源：

- [生成脚本](../packaging/windows/generate_assets.py)只用 Python 标准库，按原始直线路径的奇偶填充与圆角矩形生成 4×4 子像素采样的 RGBA 图像；遇到不支持的路径命令或结构会失败，不尝试近似重设计。
- Windows [资源目录](../packaging/windows/assets)含 50、150、44 像素的 PNG 和包含 32、16、48、256 像素的 ICO。ICO 首帧为 32 像素，对应当前托盘加载尺寸；程序构建资源与托盘都引用同一 ICO，打包入口复制同一组资源，MSIX 底色同步为墨蓝黑。
- WebUI [副本](../app/public/brand/app-icon.svg)逐字复制生成源，由 Vite 复制进 dist 并由程序静态资源处理器提供。普通界面的首页／侧栏、远端界面页首，以及普通／远端／Capture 三个 HTML 的 favicon 使用该资源。

运行 `python packaging/windows/generate_assets.py` 更新派生资源；`--check` 按 PNG 解码内容和 ICO 目录／各帧内容核对，避免把不同 zlib 运行时的压缩字节差异误判为图形变化。`python scripts/check.py` 包含资源一致性检查；`python scripts/test_brand_assets.py` 验证压缩等价、损坏拒绝与实际消费者引用，后者也进入 `scripts/mvp.py check` 的现有检查组。前端改动另运行 `npm --prefix app run build`。

小尺寸、透明角和深浅背景的实际渲染检查只说明这些资源的可见效果。静态引用和生成检查不代替新程序的实际窗口／托盘、系统缩放、安装／升级或整体版本验收；这些仍按 [WORKFLOW](WORKFLOW.zh-CN.md) 和 [PACKAGING](PACKAGING.zh-CN.md) 的候选边界记录。
