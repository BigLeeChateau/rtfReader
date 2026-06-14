import { createSignal, For, createMemo, Show } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import "./App.css";

interface Block {
  block_type: string;
  start_byte: number;
  end_byte: number;
  estimated_lines: number;
}

interface ParseResult {
  file_size: number;
  parse_nanos: number;
  peak_memory_kb: number;
  block_count: number;
  blocks: Block[];
  skipped_control_words: string[];
  page_count_estimate: number;
}

interface ConvertedImage {
  index: number;
  format: string;
  svg: string;
}

interface ImageParseResult {
  parse: ParseResult;
  convert_nanos: number;
  converted_images: ConvertedImage[];
  total_images: number;
  converted_count: number;
}

interface TableParseResult {
  row_count: number;
  column_count: number;
  html: string;
}

function App() {
  const [result, setResult] = createSignal<ParseResult | null>(null);
  const [imageResult, setImageResult] = createSignal<ImageParseResult | null>(null);
  const [tableResult, setTableResult] = createSignal<TableParseResult | null>(null);
  const [error, setError] = createSignal("");
  const [loading, setLoading] = createSignal(false);
  const [endToEndMs, setEndToEndMs] = createSignal<number | null>(null);
  const [scrollFps, setScrollFps] = createSignal<number | null>(null);
  const [scrollMemGrowth, setScrollMemGrowth] = createSignal<number | null>(null);

  const throughputPagesPerSec = createMemo(() => {
    const r = result();
    if (!r) return null;
    const pages = r.page_count_estimate;
    const secs = r.parse_nanos / 1e9;
    if (secs === 0) return null;
    return Math.round(pages / secs);
  });

  const convertThroughputPerSec = createMemo(() => {
    const ir = imageResult();
    if (!ir) return null;
    const secs = ir.convert_nanos / 1e9;
    if (secs === 0) return null;
    return Math.round(ir.total_images / secs);
  });

  async function handleOpen(withImages: boolean) {
    setError("");
    setResult(null);
    setImageResult(null);
    setTableResult(null);
    setEndToEndMs(null);
    setScrollFps(null);
    setScrollMemGrowth(null);

    const startMs = performance.now();
    setLoading(true);
    try {
      const selected = await open({
        multiple: false,
        directory: false,
        filters: [{ name: "RTF", extensions: ["rtf"] }],
      });
      if (!selected || Array.isArray(selected)) {
        setLoading(false);
        return;
      }

      if (withImages) {
        const res: ImageParseResult = await invoke("parse_and_convert_rtf", { path: selected });
        setImageResult(res);
        setResult(res.parse);
      } else {
        const res: ParseResult = await invoke("parse_rtf", { path: selected });
        setResult(res);
      }
      setEndToEndMs(Math.round(performance.now() - startMs));
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  async function handleParseTables() {
    setError("");
    setTableResult(null);
    setLoading(true);
    const startMs = performance.now();
    try {
      const selected = await open({
        multiple: false,
        directory: false,
        filters: [{ name: "RTF", extensions: ["rtf"] }],
      });
      if (!selected || Array.isArray(selected)) {
        setLoading(false);
        return;
      }
      const res: TableParseResult = await invoke("parse_tables", { path: selected });
      setTableResult(res);
      setEndToEndMs(Math.round(performance.now() - startMs));
    } catch (e) {
      setError(String(e));
    } finally {
      setLoading(false);
    }
  }

  async function runScrollStress() {
    const r = result();
    if (!r) return;

    const container = document.getElementById("scroll-container");
    if (!container) return;

    const initialHeap = (performance as any).memory?.usedJSHeapSize ?? 0;
    let frames = 0;
    let start = performance.now();

    const step = () => {
      const now = performance.now();
      frames++;
      const maxScroll = container.scrollHeight - container.clientHeight;
      const next = (container.scrollTop + 800) % (maxScroll + 1);
      container.scrollTop = next;
      if (now - start < 3000) {
        requestAnimationFrame(step);
      } else {
        const fps = Math.round((frames * 1000) / (now - start));
        setScrollFps(fps);
        const finalHeap = (performance as any).memory?.usedJSHeapSize ?? 0;
        setScrollMemGrowth(Math.round((finalHeap - initialHeap) / 1024 / 1024));
      }
    };
    requestAnimationFrame(step);
  }

  const blockHeights = createMemo(() => {
    const r = result();
    if (!r) return [];
    return r.blocks.map((b) => Math.max(80, b.estimated_lines * 24));
  });

  const svgByIndex = createMemo(() => {
    const ir = imageResult();
    const map = new Map<number, string>();
    if (ir) {
      for (const img of ir.converted_images) {
        map.set(img.index, img.svg);
      }
    }
    return map;
  });

  return (
    <main class="container">
      <h1>RTF Reader 原型 — 图片验证</h1>

      <section class="controls">
        <button onClick={() => handleOpen(false)} disabled={loading()}>
          {loading() ? "解析中..." : "选择 RTF（仅解析）"}
        </button>
        <button onClick={() => handleOpen(true)} disabled={loading()}>
          {loading() ? "解析中..." : "选择 RTF（解析 + 转换图片）"}
        </button>
        <button onClick={handleParseTables} disabled={loading()}>
          {loading() ? "解析中..." : "选择 RTF（提取表格）"}
        </button>
        <Show when={result()}>
          <button onClick={runScrollStress} disabled={scrollFps() !== null}>
            运行虚拟滚动压力测试（3 秒）
          </button>
        </Show>
      </section>

      {error() && <div class="error">{error()}</div>}

      <Show when={result()}>
        <section class="metrics">
          <h2>性能指标</h2>
          <div class="grid">
            <div class="card">
              <div class="label">文件大小</div>
              <div class="value">{formatBytes(result()!.file_size)}</div>
            </div>
            <div class="card">
              <div class="label">预估页数</div>
              <div class="value">{result()!.page_count_estimate.toLocaleString()}</div>
            </div>
            <div class="card">
              <div class="label">解析耗时</div>
              <div class="value">{(result()!.parse_nanos / 1e6).toFixed(2)} ms</div>
            </div>
            <div class="card">
              <div class="label">端到端首屏</div>
              <div class="value">{endToEndMs() !== null ? `${endToEndMs()} ms` : "—"}</div>
            </div>
            <div class="card">
              <div class="label">解析吞吐量</div>
              <div class="value">
                {throughputPagesPerSec() !== null
                  ? `${throughputPagesPerSec()!.toLocaleString()} 页/秒`
                  : "—"}
              </div>
            </div>
            <div class="card">
              <div class="label">峰值 RSS</div>
              <div class="value">
                {result()!.peak_memory_kb > 0
                  ? `${Math.round(result()!.peak_memory_kb / 1024)} MB`
                  : "未获取"}
              </div>
            </div>
            <div class="card">
              <div class="label">块数</div>
              <div class="value">{result()!.block_count.toLocaleString()}</div>
            </div>
            <div class="card">
              <div class="label">跳过控制词</div>
              <div class="value">{result()!.skipped_control_words.length} 个</div>
            </div>
          </div>

          <Show when={imageResult()}>
            <h3>图片转换</h3>
            <div class="grid">
              <div class="card">
                <div class="label">图片总数</div>
                <div class="value">{imageResult()!.total_images.toLocaleString()}</div>
              </div>
              <div class="card">
                <div class="label">转换成功</div>
                <div class="value">{imageResult()!.converted_count.toLocaleString()}</div>
              </div>
              <div class="card">
                <div class="label">转换耗时</div>
                <div class="value">{(imageResult()!.convert_nanos / 1e6).toFixed(2)} ms</div>
              </div>
              <div class="card">
                <div class="label">转换吞吐量</div>
                <div class="value">
                  {convertThroughputPerSec() !== null
                    ? `${convertThroughputPerSec()!.toLocaleString()} 图/秒`
                    : "—"}
                </div>
              </div>
            </div>
          </Show>

          <Show when={scrollFps() !== null}>
            <div class="scroll-metrics">
              <h3>虚拟滚动压力测试</h3>
              <div class="grid">
                <div class="card">
                  <div class="label">滚动帧率</div>
                  <div class="value">{scrollFps()} fps</div>
                </div>
                <div class="card">
                  <div class="label">JS 堆内存增长</div>
                  <div class="value">{scrollMemGrowth() ?? "—"} MB</div>
                </div>
              </div>
            </div>
          </Show>

          <Show when={result()!.skipped_control_words.length > 0}>
            <div class="skipped">
              <h3>前 20 个被跳过的控制词</h3>
              <code>{result()!.skipped_control_words.join(", ")}</code>
            </div>
          </Show>
        </section>
      </Show>

      <Show when={imageResult()}>
        <section class="preview">
          <h2>转换后的 SVG 预览（前 20 张）</h2>
          <div class="svg-grid">
            <For each={imageResult()!.converted_images}>
              {(img, i) => (
                <div class="svg-card">
                  <div class="svg-label">
                    #{i() + 1} ({img.format})
                  </div>
                  <div class="svg-content" innerHTML={img.svg} />
                </div>
              )}
            </For>
          </div>
        </section>
      </Show>

      <Show when={tableResult()}>
        <section class="preview">
          <h2>表格解析结果</h2>
          <div class="grid" style={{ "margin-bottom": "16px" }}>
            <div class="card">
              <div class="label">行数</div>
              <div class="value">{tableResult()!.row_count.toLocaleString()}</div>
            </div>
            <div class="card">
              <div class="label">列数</div>
              <div class="value">{tableResult()!.column_count.toLocaleString()}</div>
            </div>
          </div>
          <div class="table-preview" innerHTML={tableResult()!.html} />
        </section>
      </Show>

      <Show when={result()}>
        <section class="preview">
          <h2>块预览（虚拟滚动占位）</h2>
          <div id="scroll-container" class="scroll-container">
            <For each={result()!.blocks}>
              {(block, i) => (
                <div
                  class={`block ${block.block_type.toLowerCase()}`}
                  style={{ height: `${blockHeights()[i()]}px` }}
                >
                  <div class="block-header">
                    #{i() + 1} {block.block_type} · {formatBytes(block.end_byte - block.start_byte)}
                  </div>
                  <Show when={block.block_type === "Figure"}>
                    <div class="block-svg" innerHTML={svgByIndex().get(i()) ?? ""} />
                  </Show>
                </div>
              )}
            </For>
          </div>
        </section>
      </Show>
    </main>
  );
}

function formatBytes(n: number): string {
  if (n < 1024) return `${n} B`;
  if (n < 1024 * 1024) return `${(n / 1024).toFixed(1)} KB`;
  return `${(n / 1024 / 1024).toFixed(2)} MB`;
}

export default App;
