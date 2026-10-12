<script lang="ts">
  import { onMount } from "svelte";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";

  type Point = { time: number; value: number };
  type Series = { name: string; points: Point[] };
  type TemplateField = { name: string; type: "number" | "int" | "hex" | "hex0x" };

  let packetTemplate = $state("");
  let activeTemplates: string[] = [];
  let templateError = $state("");
  let paused = $state(false);
  let showRuleDialog = $state(false);
  let maxPoints = $state(500);
  let yMin = $state("");
  let yMax = $state("");
  let series = $state<Series[]>([]);
  let hovered = $state<{ name: string; point: Point } | null>(null);
  let followLatest = $state(true);
  let viewX: { min: number; max: number } | null = $state(null);
  let viewY: { min: number; max: number } | null = $state(null);
  let dragStart: { x: number; y: number } | null = $state(null);
  let dragMoved = false;
  let buffer = "";
  let sequence = 0;

  const chartWidth = 1000;
  const chartHeight = 520;
  const padding = { left: 62, right: 18, top: 24, bottom: 42 };
  const plotWidth = chartWidth - padding.left - padding.right;
  const plotHeight = chartHeight - padding.top - padding.bottom;

  onMount(() => {
    const stored = localStorage.getItem("zcom-plot-template");
    if (stored) {
      packetTemplate = stored;
      activeTemplates = stored.split(/\r?\n/).map((line) => line.trim()).filter(Boolean);
    }
    let unlisten: UnlistenFn | undefined;
    void listen<{ bytes: number[] }>("plot-data", ({ payload }) => {
      if (!paused) consume(new Uint8Array(payload.bytes));
    }).then((value) => { unlisten = value; });
    return () => unlisten?.();
  });

  function consume(bytes: Uint8Array) {
    buffer += new TextDecoder().decode(bytes);
    parseBuffer();
    if (buffer.length > 65536) buffer = buffer.slice(-32768);
  }

  function parseBuffer() {
    if (!activeTemplates.length) return;
    let compiled: { matcher: RegExp; fields: TemplateField[] }[];
    try {
      compiled = compileTemplates(activeTemplates);
      templateError = "";
    } catch (error) {
      templateError = error instanceof Error ? error.message : String(error);
      return;
    }
    const next = new Map(series.map((item) => [item.name, [...item.points]]));
    let consumed = 0;
    for (const rule of compiled) {
      let match: RegExpExecArray | null;
      while ((match = rule.matcher.exec(buffer))) {
        const now = ++sequence;
        rule.fields.forEach((field, index) => {
          const raw = match?.[index + 1] ?? "";
          const value = field.type === "hex" || field.type === "hex0x"
            ? Number.parseInt(raw.replace(/^0x/i, ""), 16)
            : Number(raw);
          if (!Number.isFinite(value)) return;
          const points = next.get(field.name) ?? [];
          points.push({ time: now, value });
          next.set(field.name, points.slice(-Math.max(20, Number(maxPoints) || 500)));
        });
        consumed = Math.max(consumed, rule.matcher.lastIndex);
      }
    }
    if (consumed) buffer = buffer.slice(consumed);
    series = [...next.entries()].map(([name, points]) => ({ name, points }));
  }

  function compileTemplate(template: string) {
    const placeholder = /\{([^{}:]+?)(?::(number|int|hex|hex0x))?\}/g;
    const fields: TemplateField[] = [];
    let expression = "";
    let cursor = 0;
    let match: RegExpExecArray | null;
    while ((match = placeholder.exec(template))) {
      expression += literalExpression(template.slice(cursor, match.index));
      const type = (match[2] ?? "number") as TemplateField["type"];
      const name = match[1].trim();
      if (!name) throw new Error("占位符中的曲线名不能为空");
      fields.push({ name, type });
      expression += type === "hex0x"
        ? "(0[xX][0-9A-Fa-f]+)"
        : type === "hex"
        ? "(0[xX][0-9A-Fa-f]+|[0-9A-Fa-f]+)"
        : type === "int" ? "(-?\\d+)" : "(-?\\d+(?:\\.\\d+)?(?:[eE][+-]?\\d+)?)";
      cursor = placeholder.lastIndex;
    }
    expression += literalExpression(template.slice(cursor));
    if (!fields.length) throw new Error("模板中至少需要一个 {字段名} 占位符");
    return { matcher: new RegExp(expression, "g"), fields };
  }

  function compileTemplates(templates: string[]) {
    return templates.map((template, index) => {
      try {
        return compileTemplate(template);
      } catch (error) {
        const message = error instanceof Error ? error.message : String(error);
        throw new Error(`第 ${index + 1} 行规则错误：${message}`);
      }
    });
  }

  function literalExpression(value: string) {
    const normalized = value
      .replace(/\\r\\n/g, "\n")
      .replace(/\\r|\\n/g, "\n")
      .replace(/␍␊|␍|␊/g, "\n");
    return normalized
      .split(/(\s+)/)
      .map((part) => /^\s+$/.test(part) ? "\\s*" : part.replace(/[.*+?^${}()|[\]\\]/g, "\\$&"))
      .join("");
  }

  function saveTemplate() {
    templateError = "";
    try {
      const templates = packetTemplate.split(/\r?\n/).map((line) => line.trim()).filter(Boolean);
      compileTemplates(templates);
      activeTemplates = templates;
      localStorage.setItem("zcom-plot-template", packetTemplate);
      parseBuffer();
      showRuleDialog = false;
    } catch (error) {
      templateError = error instanceof Error ? error.message : String(error);
    }
  }

  function clearSeries() {
    series = [];
    buffer = "";
    hovered = null;
    viewX = null;
    viewY = null;
    followLatest = true;
  }

  function dataBounds() {
    const points = series.flatMap((item) => item.points);
    if (!points.length) return { min: 0, max: 1 };
    const values = points.map((point) => point.time);
    const min = Math.min(...values);
    const max = Math.max(...values);
    return min === max ? { min: min - 1, max: max + 1 } : { min, max };
  }

  function bounds() {
    const values = series.flatMap((item) => item.points.map((point) => point.value));
    let minimum = yMin === "" ? Math.min(...values) : Number(yMin);
    let maximum = yMax === "" ? Math.max(...values) : Number(yMax);
    if (!values.length) return { minimum: 0, maximum: 1 };
    if (!Number.isFinite(minimum)) minimum = Math.min(...values);
    if (!Number.isFinite(maximum)) maximum = Math.max(...values);
    if (minimum === maximum) { minimum -= 1; maximum += 1; }
    return { minimum, maximum };
  }

  function x(point: Point) {
    const domain = viewX ?? dataBounds();
    return padding.left + ((point.time - domain.min) / Math.max(domain.max - domain.min, 1)) * plotWidth;
  }

  function y(value: number) {
    const { minimum, maximum } = visibleYBounds();
    return padding.top + (1 - (value - minimum) / Math.max(maximum - minimum, 1)) * plotHeight;
  }

  function visibleYBounds() {
    const fallback = bounds();
    return { minimum: viewY?.min ?? fallback.minimum, maximum: viewY?.max ?? fallback.maximum };
  }

  function path(points: Point[]) {
    return points.map((point, index) => `${index ? "L" : "M"}${x(point).toFixed(2)},${y(point.value).toFixed(2)}`).join(" ");
  }

  function svgPosition(event: MouseEvent | WheelEvent) {
    const rect = (event.currentTarget as HTMLElement).getBoundingClientRect();
    return {
      x: ((event.clientX - rect.left) / rect.width) * chartWidth,
      y: ((event.clientY - rect.top) / rect.height) * chartHeight,
    };
  }

  function zoomDomain(domain: { min: number; max: number }, ratio: number, factor: number) {
    const center = domain.min + ratio * (domain.max - domain.min);
    const span = Math.max((domain.max - domain.min) * factor, 0.000001);
    return { min: center - ratio * span, max: center + (1 - ratio) * span };
  }

  function handleWheel(event: WheelEvent) {
    event.preventDefault();
    const position = svgPosition(event);
    const xRatio = Math.min(1, Math.max(0, (position.x - padding.left) / plotWidth));
    const yRatio = Math.min(1, Math.max(0, 1 - (position.y - padding.top) / plotHeight));
    const factor = event.deltaY > 0 ? 1.15 : 0.87;
    const zoomX = !event.ctrlKey;
    const zoomY = event.ctrlKey || event.shiftKey;
    if (zoomX) viewX = zoomDomain(viewX ?? dataBounds(), xRatio, factor);
    if (zoomY) viewY = zoomDomain(viewY ?? { min: bounds().minimum, max: bounds().maximum }, yRatio, factor);
    followLatest = false;
  }

  function handleDragStart(event: MouseEvent) {
    if (event.button !== 0) return;
    dragStart = svgPosition(event);
    dragMoved = false;
  }

  function handleDragMove(event: MouseEvent) {
    if (!dragStart) return;
    const position = svgPosition(event);
    const dx = position.x - dragStart.x;
    const dy = position.y - dragStart.y;
    if (!dragMoved && Math.hypot(dx, dy) < 3) return;
    if (!dragMoved) followLatest = false;
    dragMoved = true;
    const xDomain = viewX ?? dataBounds();
    const yBounds = viewY ?? { min: bounds().minimum, max: bounds().maximum };
    const xShift = -dx / plotWidth * (xDomain.max - xDomain.min);
    const yShift = dy / plotHeight * (yBounds.max - yBounds.min);
    viewX = { min: xDomain.min + xShift, max: xDomain.max + xShift };
    viewY = { min: yBounds.min + yShift, max: yBounds.max + yShift };
    dragStart = position;
  }

  function handleDragEnd() { dragStart = null; dragMoved = false; }

  function goLatest() {
    followLatest = true;
    viewX = null;
  }

  function resetView() {
    followLatest = true;
    viewX = null;
    viewY = null;
  }

  const colors = ["#1677ff", "#e67e22", "#18a058", "#c039a5", "#8e44ad", "#008b8b"];
</script>

<svelte:head><title>数据曲线</title></svelte:head>

<main class="plot-window">
  <header class="toolbar">
    <button onclick={() => showRuleDialog = true}>规则配置</button>
    <button onclick={() => paused = !paused}>{paused ? "继续" : "暂停"}</button>
    <button onclick={clearSeries}>清除</button>
    <button onclick={goLatest}>回到最新</button>
    <button onclick={resetView}>重置视图</button>
    <label>点数 <input class="small" type="number" min="20" max="5000" bind:value={maxPoints} onchange={() => maxPoints = Math.min(Math.max(Number(maxPoints) || 500, 20), 5000)} /></label>
    <label>Y最小 <input class="small" placeholder="自动" bind:value={yMin} /></label>
    <label>Y最大 <input class="small" placeholder="自动" bind:value={yMax} /></label>
  </header>
  {#if templateError}<p class="error">模板错误：{templateError}</p>{/if}
  <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
  <section role="application" aria-label="可缩放和拖动的数据曲线" class:dragging={dragStart !== null} class="chart-wrap" onwheel={handleWheel} onmousedown={handleDragStart} onmousemove={handleDragMove} onmouseup={handleDragEnd} onmouseleave={handleDragEnd}>
    <svg viewBox={`0 0 ${chartWidth} ${chartHeight}`} role="img" aria-label="接收数据曲线">
      <line x1={padding.left} y1={padding.top} x2={padding.left} y2={chartHeight - padding.bottom} class="axis" />
      <line x1={padding.left} y1={chartHeight - padding.bottom} x2={chartWidth - padding.right} y2={chartHeight - padding.bottom} class="axis" />
      {#each [0, .25, .5, .75, 1] as ratio}
        <line x1={padding.left} y1={padding.top + plotHeight * ratio} x2={chartWidth - padding.right} y2={padding.top + plotHeight * ratio} class="grid" />
        <text x={padding.left - 8} y={padding.top + plotHeight * ratio + 4} text-anchor="end" class="tick">{(visibleYBounds().maximum - (visibleYBounds().maximum - visibleYBounds().minimum) * ratio).toFixed(2)}</text>
      {/each}
      {#each series as item, index (item.name)}
        <path d={path(item.points)} class="line" stroke={colors[index % colors.length]} />
        {#each item.points as point (point.time)}
          <circle role="button" tabindex="0" aria-label={`${item.name}: ${point.value}`} cx={x(point)} cy={y(point.value)} r="3" fill={colors[index % colors.length]} onmouseenter={() => hovered = { name: item.name, point }} onmouseleave={() => hovered = null} onfocus={() => hovered = { name: item.name, point }} onblur={() => hovered = null} />
        {/each}
      {/each}
      {#if !series.length}<text x={chartWidth / 2} y={chartHeight / 2} text-anchor="middle" class="empty">等待接收数据…</text>{/if}
    </svg>
    {#if hovered}<div class="tooltip">{hovered.name}: {hovered.point.value}</div>{/if}
  </section>
  <footer>{#each series as item, index (item.name)}<span><i style={`background:${colors[index % colors.length]}`}></i>{item.name}</span>{/each}<span class="state">{paused ? "已暂停" : followLatest ? "实时跟随" : "历史视图"}</span></footer>
</main>

{#if showRuleDialog}
  <div class="modal-backdrop" role="presentation" onclick={(event) => event.target === event.currentTarget && (showRuleDialog = false)}>
    <div class="rule-dialog" role="dialog" aria-modal="true" aria-labelledby="rule-title">
      <header class="dialog-header"><h2 id="rule-title">规则配置</h2><button class="close" aria-label="关闭" onclick={() => showRuleDialog = false}>×</button></header>
      <p class="dialog-help">每行填写一条规则，接收的 ASCII 文本报文会依次尝试所有规则。把变化的数据写成 <code>{"{曲线名:数据格式}"}</code>，曲线名支持中文。数据格式：<code>number</code>（整数或小数）、<code>int</code>（整数）、<code>hex</code>（十六进制，可带或不带 0x）、<code>hex0x</code>（必须带 0x/0X）。报文换行请写 <code>\\r\\n</code> 或 <code>␍␊</code>。当前仅支持 ASCII 文本报文，暂不支持 HEX 原始报文。</p>
      <textarea class="rule-editor" bind:value={packetTemplate} spellcheck="false" placeholder="每行一条规则"></textarea>
      {#if templateError}<p class="error">规则错误：{templateError}</p>{/if}
      <footer class="dialog-actions"><button onclick={() => showRuleDialog = false}>取消</button><button class="primary" onclick={saveTemplate}>保存规则</button></footer>
    </div>
  </div>
{/if}

<style>
  :global(*) { box-sizing: border-box; }
  :global(html), :global(body) { margin: 0; width: 100%; height: 100%; overflow: hidden; font: 13px "Segoe UI", "Microsoft YaHei UI", sans-serif; color: #202020; background: #f5f7f8; }
  .plot-window { height: 100vh; display: flex; flex-direction: column; padding: 9px 11px 7px; gap: 5px; }
  .toolbar { display: flex; align-items: center; gap: 7px; flex-wrap: wrap; }
  label { display: inline-flex; align-items: center; gap: 4px; white-space: nowrap; }
  input, button, textarea { border: 1px solid #b7c0c5; border-radius: 3px; background: #fff; color: inherit; }
  input, button { height: 28px; }
  input { padding: 3px 6px; }
  button { padding: 0 10px; cursor: pointer; }
  button:hover { border-color: #1677ff; background: #edf5ff; }
  .small { width: 62px; }
  textarea { width: 100%; height: 92px; resize: vertical; padding: 6px 8px; font: 12px/1.4 Consolas, monospace; }
  .error { margin: 0; color: #b42318; }
  .chart-wrap { position: relative; flex: 1; min-height: 0; background: #fff; border: 1px solid #b9c2c7; overflow: hidden; cursor: grab; user-select: none; }
  .chart-wrap.dragging { cursor: grabbing; }
  svg { width: 100%; height: 100%; }
  .axis { stroke: #6d777d; stroke-width: 1.2; }
  .grid { stroke: #e2e7e9; stroke-width: 1; }
  .tick { fill: #68757d; font-size: 12px; }
  .line { fill: none; stroke-width: 2.2; stroke-linejoin: round; stroke-linecap: round; }
  circle { cursor: crosshair; }
  .empty { fill: #8b969d; font-size: 18px; }
  .tooltip { position: absolute; top: 10px; right: 12px; padding: 5px 8px; background: #202020; color: #fff; border-radius: 3px; pointer-events: none; }
  footer { display: flex; align-items: center; gap: 13px; min-height: 24px; color: #536068; }
  footer i { display: inline-block; width: 10px; height: 3px; margin-right: 4px; vertical-align: middle; }
  .state { margin-left: auto; }
  .modal-backdrop { position: fixed; inset: 0; display: grid; place-items: center; padding: 18px; background: rgb(0 0 0 / 28%); z-index: 10; }
  .rule-dialog { width: min(760px, 100%); max-height: min(680px, 100%); display: flex; flex-direction: column; gap: 10px; padding: 14px; background: #fff; border: 1px solid #aeb8bd; border-radius: 6px; box-shadow: 0 14px 38px rgb(0 0 0 / 22%); }
  .dialog-header { display: flex; align-items: center; justify-content: space-between; }
  .dialog-header h2 { margin: 0; font-size: 16px; }
  .close { width: 28px; padding: 0; font-size: 20px; line-height: 1; }
  .dialog-help { margin: 0; color: #536068; font-size: 12px; line-height: 1.55; }
  code { padding: 1px 3px; border-radius: 2px; background: #f0f3f4; font-family: Consolas, monospace; }
  .rule-editor { min-height: 220px; height: 35vh; resize: vertical; }
  .dialog-actions { display: flex; justify-content: flex-end; gap: 8px; }
  .primary { border-color: #1677ff; background: #1677ff; color: #fff; }
  .primary:hover { background: #0958d9; }
</style>
