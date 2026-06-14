# Context

## Domain

RTF 大文档阅读与批注工具，面向临床试验 TFL（Tables, Figures, Listings）文档的审阅场景。

## Glossary

| Term | Definition |
|------|------------|
| **TFL** | Tables, Figures, Listings — 临床试验中由 SAS 等统计软件自动生成的报告文档 |
| **块 (Block)** | TFL 文档中的独立内容单元，如一个 Table、一个 Figure、一个 Listing。块是虚拟滚动的基本单位 |
| **批注 (Annotation)** | 附加于文本范围的审阅意见，使用 Word 兼容的 RTF `\annotation` 控制词存储。锚定方式为字符偏移量 |
| **EMF** | Enhanced Metafile — SAS 在 Windows 上输出的默认图片格式，需转换为 SVG/PNG 才能在浏览器中显示 |
| **智能保存** | 首次批注时强制另存为新文件，不覆盖原始 TFL；后续编辑已批注版本时允许覆盖 |
| **作者过滤** | 按批注作者名筛选显示的功能。不同作者自动分配不同颜色 |
| **全局文本偏移量 (Global Text Offset)** | 文档文本流中从首字符开始的字符位置，用于批注锚定与搜索结果定位，与虚拟滚动块边界无关 |
| **暖色护眼主题 (Warm Paper Theme)** | MVP 默认阅读背景，使用米色/暖灰替代纯白，图片保持原色，避免颜色感知风险 |
| **外部编辑检测** | 通过保存时的 SHA-256 校验和与 sidecar 备份，检测文件是否被 Word 等外部程序修改 |
| **原型验证 (Prototype Verification)** | 为验证某一高风险设计假设而构建的极简或可弃实现，在全面投入开发前确认核心可行性 |
| **秒开** | 指用户选择 2000 页 RTF 文件后，首块内容可在 2 秒内呈现的性能目标 |
| **块索引 (Block Index)** | 由 Rust 解析器构建的、记录每个 Table/Figure/Listing 在文档中位置与类型的元数据结构，是虚拟滚动的依据 |
| **合成 TFL (Synthetic TFL)** | 由 SAS 程序或脚本生成的、不依赖真实临床数据的测试用 TFL 文档，用于可重复的性能基准测试 |

## Deferred Decisions

| Feature | Defer Reason |
|---------|-------------|
| 批注"已解决"状态 | 首轮不实现，后续版本添加 |
| 暗色模式 | MVP 用暖色护眼主题替代，收集用户反馈后再评估 |
| 多窗口/多标签 | MVP 单窗口单文档，后续版本再评估 |
| 多格式支持 (DOCX/PDF) | 核心 RTF 做好后再扩展 |
| 高级搜索（模糊/拼音/同义词） | MVP 用 Tantivy 基础全文搜索，高级语义搜索后续评估 |
| 遥测/崩溃报告 | MVP 仅本地日志，后续版本考虑可选匿名上报 |
| AI 辅助批注智能 | 超出当前核心痛点范围 |
