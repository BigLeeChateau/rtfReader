# ADR 0002: EMF 图片转换策略

## 状态

已接受

## 背景

临床试验 TFL 中的图片通常以 **EMF**（Enhanced Metafile）格式嵌入 RTF。浏览器无法直接显示 EMF，需要后端转换。

## 考虑的方案

| 方案 | 优点 | 缺点 |
|---|---|---|
| **A. 所有平台统一用 libemf2svg** | 跨平台一致；原型已验证 | Windows 上不是原生路径；某些 EMF+ 记录支持不完整 |
| **B. macOS/Linux 用 libemf2svg，Windows 用 GDI+** | Windows 原生支持最好；macOS 依赖已验证 | 需要平台抽象层；Windows 路径需单独验证 |
| **C. 统一用 ImageMagick/Inkscape** | 可能覆盖更多 EMF 变体 | 依赖重、打包体积大、版本碎片化 |
| **D. 直接显示占位框，不提供转换** | 最简单 | 无法满足“完整渲染”的核心需求 |

## 决策

采用 **方案 B**，并增加 SVG→PNG fallback：

- **主路径**：`libemf2svg` 将 EMF 转为 SVG。
- **Fallback**：若 SVG 在浏览器中渲染异常或 `libemf2svg` 输出不完整，用 Rust `resvg` 将 SVG 栅格化为 300dpi PNG。
- **完全失败**：显示占位框，允许用户导出原始 EMF。
- **Windows**：优先使用 GDI+ 原生渲染 EMF；GDI+ 失败时再尝试 libemf2svg。

## 后果

- **正面**：macOS 原型已验证 1000 张图 0.027 秒转换。
- **正面**：PNG fallback 保证复杂图表仍可用，同时避免重型跨平台依赖。
- **正面**：Windows 走 GDI+ 最接近 Word 原生渲染效果。
- **负面**：需要维护两套 EMF 解码路径（libemf2svg / GDI+）和一套 SVG 栅格化路径（resvg）。
- **负面**：macOS 上若 libemf2svg 完全失败且 resvg 无法挽救，只能降级占位框。
