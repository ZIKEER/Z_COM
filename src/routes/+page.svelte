<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen, type UnlistenFn } from "@tauri-apps/api/event";
  import { open, save } from "@tauri-apps/plugin-dialog";
  import { openPath } from "@tauri-apps/plugin-opener";
  import {
    ArrowDown,
    ArrowUp,
    ChevronLeft,
    ChevronRight,
    CircleHelp,
    Download,
    Eraser,
    FolderOpen,
    Link,
    Link2Off,
    Minus,
    Play,
    Plus,
    RefreshCw,
    Send,
    Settings,
    Square,
    Trash2,
    Upload,
  } from "@lucide/svelte";

  type TransportMode = "serial" | "socket" | "probe";
  type DisplayMode = "HEX" | "ASCII" | "MIXED";
  type SendMode = "HEX" | "ASCII";

  interface AppConfig {
    transport_mode: TransportMode;
    port: string;
    baudrate: string;
    baudrate_history: string[];
    databits: number;
    stopbits: number;
    parity: string;
    flowcontrol: string;
    frame_timeout: number;
    display_mode: DisplayMode;
    send_mode: SendMode;
    auto_scroll: boolean;
    auto_send_interval: number;
    display_ansi: boolean;
    support_probes: boolean;
    show_generic_jtag_adapters: boolean;
    jlink_sdk_path: string;
    probe_chip: string;
    probe_speed: number;
    probe_reset: boolean;
    probe_chip_history: string[];
    preset_panel_visible: boolean;
    send_panel_ratio: number;
    extended_panel_ratio: number;
    socket_host: string;
    socket_port: number;
    socket_protocol: "TCP" | "UDP";
    socket_role: "Client" | "Server";
    selected_probe: string;
  }

  interface ExtendedItem {
    id: number;
    data: string;
    is_hex: boolean;
    comment: string;
    delay: number;
    sort_order: number;
  }

  interface ExtendedConfig {
    items: ExtendedItem[];
    settings: { loop_send: boolean; multi_send: boolean; default_delay: number };
  }

  interface DeviceEntry {
    id: string;
    label: string;
    transport: "serial" | "probe";
    probeKind?: string;
    serialNumber?: string;
  }

  interface TransportEvent {
    kind: "connected" | "disconnected" | "data" | "error" | "warning" | "peer" | "info";
    direction: "received" | "sent" | "system";
    bytes: number[];
    message: string;
  }

  interface BootstrapData {
    config: AppConfig;
    extended: ExtendedConfig;
    version: string;
    buildTimestamp: number;
    dataDirectory: string;
    probeTargetDirectory: string;
    instanceId: number;
  }

  interface ReceiveLine {
    id: number;
    text: string;
    html: string;
  }

  const defaultConfig: AppConfig = {
    transport_mode: "serial",
    port: "",
    baudrate: "115200",
    baudrate_history: [],
    databits: 8,
    stopbits: 1,
    parity: "None",
    flowcontrol: "None",
    frame_timeout: 50,
    display_mode: "ASCII",
    send_mode: "ASCII",
    auto_scroll: true,
    auto_send_interval: 1000,
    display_ansi: false,
    support_probes: true,
    show_generic_jtag_adapters: false,
    jlink_sdk_path: "",
    probe_chip: "",
    probe_speed: 4000,
    probe_reset: false,
    probe_chip_history: [],
    preset_panel_visible: false,
    send_panel_ratio: 0.2,
    extended_panel_ratio: 0.37,
    socket_host: "127.0.0.1",
    socket_port: 8080,
    socket_protocol: "TCP",
    socket_role: "Client",
    selected_probe: "",
  };

  let config = $state<AppConfig>({ ...defaultConfig });
  let draftConfig = $state<AppConfig>({ ...defaultConfig });
  let extended = $state<ExtendedConfig>({
    items: [],
    settings: { loop_send: false, multi_send: true, default_delay: 1000 },
  });
  let devices = $state<DeviceEntry[]>([]);
  let mode = $state<TransportMode>("serial");
  let socketProtocol = $state<"TCP" | "UDP">("TCP");
  let socketRole = $state<"Client" | "Server">("Client");
  let selectedProbe = $state("");
  let connected = $state(false);
  let connecting = $state(false);
  let statusText = $state("未连接");
  let errorText = $state("");
  let sendText = $state("");
  let appendCrLf = $state(false);
  let autoSend = $state(false);
  let settingsOpen = $state(false);
  let aboutOpen = $state(false);
  let sidebarOpen = $state(false);
  let receiveContextMenu = $state<{ x: number; y: number } | null>(null);
  let receiveContextSelection = $state("");
  let receiveLines = $state<ReceiveLine[]>([]);
  let receiveHtml = $derived(receiveLines.map((line) => line.html).join("\n"));
  let receivedCount = $state(0);
  let sentCount = $state(0);
  let lineId = 0;
  let receiveView: HTMLPreElement;
  let extendedRunning = $state(false);
  let stopExtended = false;
  let version = $state("0.1.3");
  let buildTimestamp = $state(0);
  let dataDirectory = $state("");
  let probeTargetDirectory = $state("");
  let customProbeTargets = $state<string[]>([]);
  let localIpv4Addresses = $state<string[]>(["0.0.0.0", "127.0.0.1"]);
  let instanceId = $state(1);
  let lastValidBaudrate = $state("115200");
  let resizing = $state(false);
  let refreshingDevices = $state(false);
  let refreshingCustomTargets = $state(false);
  let jlinkSdkStatus = $state("尚未检查");

  const serialDevices = $derived(devices.filter((device) => device.transport === "serial"));
  const probeDevices = $derived(devices.filter((device) => device.transport === "probe"));
  const probeChipOptions = $derived([
    ...new Set([...config.probe_chip_history, ...customProbeTargets]),
  ]);
  const baudrateOptions = $derived([
    ...new Set([
      "1200", "2400", "4800", "9600", "19200", "38400", "57600", "115200",
      "230400", "460800", "921600", "1000000", "2000000", ...config.baudrate_history,
    ]),
  ].sort((left, right) => Number(left) - Number(right)));

  onMount(() => {
    let unlisten: UnlistenFn | undefined;
    void (async () => {
      try {
        unlisten = await listen<TransportEvent>("transport-event", ({ payload }) => handleTransportEvent(payload));
        const data = await invoke<BootstrapData>("bootstrap");
        config = data.config;
        config.send_panel_ratio = clamp(config.send_panel_ratio, 0.12, 0.5);
        config.extended_panel_ratio = clamp(config.extended_panel_ratio, 0.25, 0.6);
        try {
          config.baudrate = String(validatedBaudrate(config.baudrate));
        } catch {
          config.baudrate = "115200";
        }
        lastValidBaudrate = config.baudrate;
        draftConfig = structuredClone(config);
        extended = data.extended;
        version = data.version;
        buildTimestamp = data.buildTimestamp;
        dataDirectory = data.dataDirectory;
        probeTargetDirectory = data.probeTargetDirectory;
        instanceId = data.instanceId;
        sidebarOpen = data.config.preset_panel_visible;
        mode = data.config.transport_mode;
        socketProtocol = data.config.socket_protocol;
        socketRole = data.config.socket_role;
        selectedProbe = data.config.selected_probe;
        await refreshCustomProbeTargets();
        await refreshLocalIpv4Addresses();
        await refreshDevices();
      } catch (error) {
        statusText = "浏览器预览模式";
        errorText = `未连接到 Tauri 后端: ${stringifyError(error)}`;
      }
    })();
    return () => unlisten?.();
  });

  $effect(() => {
    if (!autoSend || !connected) return;
    const timer = window.setInterval(() => void sendCurrent("auto"), Math.max(config.auto_send_interval, 100));
    return () => window.clearInterval(timer);
  });

  async function refreshDevices() {
    if (refreshingDevices) return;
    refreshingDevices = true;
    errorText = "";
    try {
      devices = await invoke<DeviceEntry[]>("list_devices");
      let configChanged = false;
      if (serialDevices.length && !serialDevices.some((device) => device.id === config.port)) {
        config.port = serialDevices[0].id;
        configChanged = true;
      }
      if (probeDevices.length && !probeDevices.some((device) => device.id === selectedProbe)) {
        selectedProbe = probeDevices[0].id;
        config.selected_probe = selectedProbe;
        configChanged = true;
      }
      if (configChanged) await savePreferences();
    } catch (error) {
      showError(error);
    } finally {
      refreshingDevices = false;
    }
  }

  async function toggleConnection() {
    if (connected || connecting) {
      await invoke("disconnect_transport");
      return;
    }
    errorText = "";
    connecting = true;
    statusText = "正在连接...";
    try {
      if (mode === "serial") {
        config.baudrate = String(validatedBaudrate(config.baudrate));
        rememberBaudrate(config.baudrate);
        lastValidBaudrate = config.baudrate;
      }
      if (mode === "probe" && config.probe_chip && !config.probe_chip_history.includes(config.probe_chip)) {
        config.probe_chip_history = [config.probe_chip, ...config.probe_chip_history].slice(0, 20);
      }
      await invoke("save_config", { config });
      await invoke("connect_transport", {
        request: {
          transport: mode,
          deviceId: mode === "probe" ? selectedProbe : config.port,
          baudRate: Number(config.baudrate),
          dataBits: config.databits,
          stopBits: config.stopbits,
          parity: config.parity,
          flowControl: config.flowcontrol,
          frameTimeout: config.frame_timeout,
          socketHost: config.socket_host,
          socketPort: config.socket_port,
          socketProtocol,
          socketRole,
          probeChip: config.probe_chip,
          probeSpeed: config.probe_speed,
          probeReset: config.probe_reset,
          jlinkSdkPath: config.jlink_sdk_path,
        },
      });
    } catch (error) {
      connecting = false;
      statusText = "连接失败";
      showError(error);
    }
  }

  async function updateBaudrate() {
    const previousConfig = structuredClone($state.snapshot(config));
    previousConfig.baudrate = lastValidBaudrate;
    let serialReconfigured = false;
    try {
      config.baudrate = String(validatedBaudrate(config.baudrate));
      rememberBaudrate(config.baudrate);
      if (connected && mode === "serial") {
        await invoke("reconfigure_serial", { settings: serialSettings(config) });
        serialReconfigured = true;
      }
      await invoke("save_config", { config });
      lastValidBaudrate = config.baudrate;
    } catch (error) {
      if (serialReconfigured) {
        try {
          await invoke("reconfigure_serial", { settings: serialSettings(previousConfig) });
        } catch {}
      }
      config = previousConfig;
      showError(error);
    }
  }

  function validatedBaudrate(value: string) {
    const normalized = value.trim();
    if (!/^\d+$/.test(normalized)) throw new Error("波特率必须是正整数");
    const baudrate = Number(normalized);
    if (!Number.isSafeInteger(baudrate) || baudrate < 1 || baudrate > 10_000_000) {
      throw new Error("波特率必须在 1 到 10000000 之间");
    }
    return baudrate;
  }

  function rememberBaudrate(value: string) {
    config.baudrate_history = [value, ...config.baudrate_history.filter((item) => item !== value)].slice(0, 12);
  }

  function handleTransportEvent(event: TransportEvent) {
    if (event.kind !== "data" && event.message) {
      appendEvent(event.kind, event.message);
    }
    if (event.kind === "connected") {
      connected = true;
      connecting = false;
      statusText = event.message;
    } else if (event.kind === "disconnected") {
      connected = false;
      connecting = false;
      autoSend = false;
      statusText = "未连接";
    } else if (event.kind === "error") {
      connected = false;
      connecting = false;
      errorText = event.message;
      statusText = "连接错误";
    } else if (event.kind === "warning") {
      errorText = event.message;
    } else if (event.kind === "peer") {
      statusText = event.message;
    } else if (event.kind === "data") {
      if (event.direction === "received") {
        receivedCount += event.bytes.length;
        appendData(new Uint8Array(event.bytes), "received");
      } else if (event.direction === "sent") {
        sentCount += event.bytes.length;
        appendData(new Uint8Array(event.bytes), "sent");
      }
    }
  }

  function appendEvent(kind: string, message: string) {
    const labels: Record<string, string> = {
      connected: "连接",
      disconnected: "断开",
      error: "错误",
      warning: "警告",
      peer: "客户端",
    };
    const normalizedKind = Object.hasOwn(labels, kind) ? kind : "info";
    const label = labels[kind] ?? "提示";
    const timestamp = currentTimestamp();
    const text = `[${timestamp}] ◆ ${label}: ${message}`;
    const html = `<span class="display-timestamp">[${timestamp}]</span> <span class="display-event-marker">◆</span> <span class="display-event display-event-${normalizedKind}">${escapeHtml(label)}: ${escapeHtml(message)}</span>`;
    receiveLines = [...receiveLines, { id: ++lineId, text, html }].slice(-5000);
    if (config.auto_scroll) {
      requestAnimationFrame(() => receiveView?.scrollTo({ top: receiveView.scrollHeight }));
    }
  }

  function currentTimestamp() {
    const now = new Date();
    return now.toLocaleTimeString("zh-CN", { hour12: false }) + `.${String(now.getMilliseconds()).padStart(3, "0")}`;
  }

  function appendData(bytes: Uint8Array, direction: "received" | "sent") {
    const timestamp = currentTimestamp();
    const arrow = direction === "received" ? "←" : "→";
    if (config.display_mode === "MIXED") {
      appendMixedData(bytes, timestamp, arrow);
      return;
    }
    let content: string;
    if (config.display_mode === "HEX") {
      content = Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0").toUpperCase()).join(" ");
    } else {
      content = displayAscii(bytes, true);
    }

    const chunks = displayLines(content);
    const label = config.display_mode === "HEX" ? "HEX:" : "ASCII:";
    const labelClass = config.display_mode === "HEX" ? "display-label-hex" : "display-label-ascii";
    const newLines = chunks.map((chunk, index) => {
      const plainContent = stripAnsi(chunk);
      const contentHtml = config.display_ansi && config.display_mode === "ASCII"
        ? ansiToHtml(chunk)
        : escapeHtml(plainContent);
      if (index > 0) {
        return { id: ++lineId, text: plainContent, html: `<span class="display-data">${contentHtml}</span>` };
      }
      const text = `[${timestamp}] ${arrow} ${label} ${plainContent}`;
      const html = `<span class="display-timestamp">[${timestamp}]</span> <span class="display-arrow">${arrow}</span> <span class="display-label ${labelClass}">${label}</span> <span class="display-data">${contentHtml}</span>`;
      return { id: ++lineId, text, html };
    });
    receiveLines = [...receiveLines, ...newLines].slice(-5000);
    if (config.auto_scroll) {
      requestAnimationFrame(() => receiveView?.scrollTo({ top: receiveView.scrollHeight }));
    }
  }

  function appendMixedData(bytes: Uint8Array, timestamp: string, arrow: "←" | "→") {
    const hex = Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0").toUpperCase()).join(" ");
    const asciiLines = displayLines(displayAscii(bytes, config.display_ansi));
    const newLines = [
      {
        id: ++lineId,
        text: `[${timestamp}]`,
        html: `<span class="display-timestamp">[${timestamp}]</span>`,
      },
      {
        id: ++lineId,
        text: `${arrow} HEX: ${hex}`,
        html: `<span class="display-arrow">${arrow}</span> <span class="display-label display-label-hex">HEX:</span> <span class="display-data">${escapeHtml(hex)}</span>`,
      },
      ...asciiLines.map((line, index) => {
        const plainContent = stripAnsi(line);
        const contentHtml = config.display_ansi ? ansiToHtml(line) : escapeHtml(plainContent);
        return {
          id: ++lineId,
          text: index === 0 ? `${arrow} ASCII: ${plainContent}` : plainContent,
          html: index === 0
            ? `<span class="display-arrow">${arrow}</span> <span class="display-label display-label-ascii">ASCII:</span> <span class="display-data">${contentHtml}</span>`
            : `<span class="display-data">${contentHtml}</span>`,
        };
      }),
    ];
    receiveLines = [...receiveLines, ...newLines].slice(-5000);
    if (config.auto_scroll) {
      requestAnimationFrame(() => receiveView?.scrollTo({ top: receiveView.scrollHeight }));
    }
  }

  function displayLines(content: string) {
    const lines = content.replace(/\r\n/g, "\n").replace(/\r/g, "\n").split("\n");
    if (lines.length > 1 && lines.at(-1) === "") lines.pop();
    return lines;
  }

  function displayAscii(bytes: Uint8Array, retainAnsi: boolean) {
    let result = "";
    for (const byte of bytes) {
      if (retainAnsi && byte === 0x1b) result += "\x1b";
      else if (byte <= 0x1f) {
        result += String.fromCharCode(0x2400 + byte);
        if (byte === 0x0a) result += "\n";
      } else if (byte === 0x7f) result += "␡";
      else if (byte < 0x7f) result += String.fromCharCode(byte);
      else result += `\\x${byte.toString(16).padStart(2, "0")}`;
    }
    return result;
  }

  function showReceiveContextMenu(event: MouseEvent) {
    event.preventDefault();
    receiveContextSelection = window.getSelection()?.toString() ?? "";
    const menuWidth = 190;
    const menuHeight = 182;
    receiveContextMenu = {
      x: Math.min(event.clientX, window.innerWidth - menuWidth - 6),
      y: Math.min(event.clientY, window.innerHeight - menuHeight - 6),
    };
  }

  async function copyReceiveSelection() {
    const text = receiveContextSelection;
    receiveContextMenu = null;
    if (!text) return;
    try {
      await navigator.clipboard.writeText(text);
    } catch {
      const textarea = document.createElement("textarea");
      textarea.value = text;
      textarea.style.position = "fixed";
      textarea.style.opacity = "0";
      document.body.appendChild(textarea);
      textarea.select();
      document.execCommand("copy");
      textarea.remove();
    }
  }

  function selectAllReceive() {
    receiveContextMenu = null;
    const selection = window.getSelection();
    if (!selection || !receiveView) return;
    const range = document.createRange();
    range.selectNodeContents(receiveView);
    selection.removeAllRanges();
    selection.addRange(range);
  }

  function clearReceiveFromContextMenu() {
    receiveContextMenu = null;
    clearReceive();
  }

  async function toggleAnsiFromContextMenu() {
    receiveContextMenu = null;
    config.display_ansi = !config.display_ansi;
    await savePreferences();
  }

  async function sendCurrent(source: "manual" | "auto" = "manual") {
    if (!connected) return;
    try {
      const bytes = encodeSend(sendText, config.send_mode, appendCrLf);
      if (!bytes.length) throw new Error(source === "auto" ? "自动发送已停止：发送内容不能为空" : "发送内容不能为空");
      await invoke("send_bytes", { bytes: Array.from(bytes) });
    } catch (error) {
      autoSend = false;
      showError(error);
    }
  }

  async function sendExtendedItem(item: ExtendedItem) {
    if (!connected) return showError("请先建立连接");
    try {
      const content = item.is_hex ? item.data : decodeAsciiEscapes(item.data);
      const bytes = encodeSend(content, item.is_hex ? "HEX" : "ASCII", false);
      if (!bytes.length) throw new Error("扩展发送内容不能为空");
      await invoke("send_bytes", { bytes: Array.from(bytes) });
    } catch (error) {
      showError(error);
    }
  }

  async function toggleExtendedRun() {
    if (extendedRunning) {
      stopExtended = true;
      return;
    }
    if (!connected) return showError("请先建立连接");
    const ordered = extended.items.filter((item) => item.sort_order > 0).sort((a, b) => a.sort_order - b.sort_order);
    if (!ordered.length) return showError("没有序号大于 0 的扩展发送条目");
    const duplicate = ordered.find((item, index) => index > 0 && item.sort_order === ordered[index - 1].sort_order);
    if (duplicate) return showError(`发送序号 ${duplicate.sort_order} 重复`);
    const batch = extended.settings.multi_send ? ordered : [ordered[0]];
    extendedRunning = true;
    stopExtended = false;
    try {
      do {
        for (const item of batch) {
          if (stopExtended) break;
          await sendExtendedItem(item);
          await delay(Math.max(item.delay, 0));
        }
      } while (extended.settings.loop_send && !stopExtended);
    } finally {
      extendedRunning = false;
      stopExtended = false;
    }
  }

  function addExtendedItem() {
    extended.items.push({
      id: Date.now(),
      data: "",
      is_hex: false,
      comment: "",
      delay: extended.settings.default_delay,
      sort_order: 0,
    });
    extended.items = [...extended.items];
    void persistExtended();
  }

  function deleteExtendedItem(index: number) {
    extended.items.splice(index, 1);
    extended.items = [...extended.items];
    void persistExtended();
  }

  function moveExtendedItem(index: number, offset: number) {
    const target = index + offset;
    if (target < 0 || target >= extended.items.length) return;
    [extended.items[index], extended.items[target]] = [extended.items[target], extended.items[index]];
    extended.items = [...extended.items];
    void persistExtended();
  }

  async function persistExtended() {
    try {
      await invoke("save_extended", { extended });
    } catch (error) {
      showError(error);
    }
  }

  async function exportExtended() {
    try {
      const path = await save({ defaultPath: "extended_send.json", filters: [{ name: "JSON", extensions: ["json"] }] });
      if (path) await invoke("write_extended_file", { path, extended });
    } catch (error) {
      showError(error);
    }
  }

  async function importExtended() {
    try {
      const path = await open({ multiple: false, filters: [{ name: "JSON", extensions: ["json"] }] });
      if (typeof path === "string") {
        extended = await invoke<ExtendedConfig>("read_extended_file", { path });
        await persistExtended();
      }
    } catch (error) {
      showError(error);
    }
  }

  function openSettings() {
    draftConfig = structuredClone($state.snapshot(config));
    settingsOpen = true;
    void refreshJlinkSdkStatus();
  }

  async function chooseJlinkSdkDirectory() {
    const path = await open({ directory: true, multiple: false, title: "选择 SEGGER J-Link 安装目录" });
    if (typeof path === "string") {
      draftConfig.jlink_sdk_path = path;
      await refreshJlinkSdkStatus();
    }
  }

  async function chooseJlinkSdkLibrary() {
    const path = await open({
      multiple: false,
      title: "选择 SEGGER J-Link 动态库",
      filters: [{ name: "J-Link 动态库", extensions: ["dll", "so", "dylib"] }],
    });
    if (typeof path === "string") {
      draftConfig.jlink_sdk_path = path;
      await refreshJlinkSdkStatus();
    }
  }

  async function refreshJlinkSdkStatus() {
    jlinkSdkStatus = "正在检查...";
    try {
      const loadedPath = await invoke<string>("check_jlink_sdk", { configuredPath: draftConfig.jlink_sdk_path });
      jlinkSdkStatus = `已找到：${loadedPath}`;
    } catch (error) {
      jlinkSdkStatus = `未找到：${stringifyError(error)}`;
    }
  }

  async function applySettings() {
    const previousConfig = structuredClone($state.snapshot(config));
    const candidate = structuredClone($state.snapshot(draftConfig));
    candidate.preset_panel_visible = sidebarOpen;
    let serialReconfigured = false;
    try {
      if (connected && mode === "serial") {
        await invoke("reconfigure_serial", { settings: serialSettings(candidate) });
        serialReconfigured = true;
      }
      await invoke("save_config", { config: candidate });
      config = candidate;
      settingsOpen = false;
      await refreshDevices();
    } catch (error) {
      if (serialReconfigured) {
        try {
          await invoke("reconfigure_serial", { settings: serialSettings(previousConfig) });
        } catch {}
      }
      showError(error);
    }
  }

  function serialSettings(value: AppConfig) {
    return {
      baudRate: Number(value.baudrate),
      dataBits: value.databits,
      stopBits: value.stopbits,
      parity: value.parity,
      flowControl: value.flowcontrol,
      frameTimeout: value.frame_timeout,
    };
  }

  async function refreshCustomProbeTargets() {
    if (refreshingCustomTargets) return;
    refreshingCustomTargets = true;
    try {
      customProbeTargets = await invoke<string[]>("list_custom_probe_targets");
    } catch (error) {
      showError(error);
    } finally {
      refreshingCustomTargets = false;
    }
  }

  async function refreshLocalIpv4Addresses() {
    try {
      localIpv4Addresses = await invoke<string[]>("list_local_ipv4_addresses");
    } catch (error) {
      showError(error);
    }
  }

  async function openProbeTargetDirectory() {
    try {
      await openPath(probeTargetDirectory);
    } catch (error) {
      showError(error);
    }
  }

  async function openDataDirectory() {
    try {
      await openPath(dataDirectory);
    } catch (error) {
      showError(error);
    }
  }

  async function updateDisplayMode(value: DisplayMode) {
    config.display_mode = value;
    await savePreferences();
  }

  async function updateSendMode(value: SendMode) {
    config.send_mode = value;
    await savePreferences();
  }

  async function savePreferences() {
    try {
      await invoke("save_config", { config });
    } catch (error) {
      showError(error);
    }
  }

  function encodeSend(value: string, sendMode: SendMode, crlf: boolean): Uint8Array {
    let bytes: Uint8Array;
    if (sendMode === "HEX") {
      const normalized = value.trim();
      if (!normalized) return new Uint8Array();
      if (!/^(?:0x)?[0-9a-f]+(?:[\s,]+(?:0x)?[0-9a-f]+)*$/i.test(normalized)) {
        throw new Error("HEX 数据只能包含十六进制字符、0x 前缀、空格或逗号");
      }
      const compact = normalized.replace(/0x/gi, "").replace(/[\s,]+/g, "");
      if (compact.length % 2) throw new Error("HEX 数据必须由完整字节组成");
      bytes = new Uint8Array(compact.match(/.{2}/g)?.map((part) => Number.parseInt(part, 16)) ?? []);
    } else {
      bytes = new TextEncoder().encode(value);
    }
    if (!bytes.length) return bytes;
    if (!crlf) return bytes;
    const result = new Uint8Array(bytes.length + 2);
    result.set(bytes);
    result.set([13, 10], bytes.length);
    return result;
  }

  function decodeAsciiEscapes(value: string) {
    let result = "";
    for (let index = 0; index < value.length; index += 1) {
      const character = value[index];
      if (character !== "\\") {
        result += character;
        continue;
      }
      const escaped = value[++index];
      if (escaped === undefined) throw new Error(`转义字符不完整（位置 ${index}）`);
      const simple = ({ r: "\r", n: "\n", t: "\t", 0: "\0", "\\": "\\" } as Record<string, string>)[escaped];
      if (simple !== undefined) {
        result += simple;
      } else if (escaped === "x") {
        const hex = value.slice(index + 1, index + 3);
        if (!/^[0-9a-f]{2}$/i.test(hex)) throw new Error(`\\x 转义必须跟两个十六进制字符（位置 ${index}）`);
        result += String.fromCharCode(Number.parseInt(hex, 16));
        index += 2;
      } else {
        throw new Error(`不支持的转义 \\${escaped}（位置 ${index}）`);
      }
    }
    return result;
  }

  function setMode(value: TransportMode) {
    if (connected || connecting) return;
    mode = value;
    config.transport_mode = value;
    errorText = "";
    void savePreferences();
  }

  function setSocketProtocol(value: "TCP" | "UDP") {
    if (connected || connecting) return;
    socketProtocol = config.socket_protocol = value;
    void savePreferences();
  }

  function setSocketRole(value: "Client" | "Server") {
    if (connected || connecting) return;
    socketRole = config.socket_role = value;
    void savePreferences();
  }

  function persistSelectedProbe() {
    config.selected_probe = selectedProbe;
    void savePreferences();
  }

  function toggleSidebar() {
    sidebarOpen = !sidebarOpen;
    config.preset_panel_visible = sidebarOpen;
    void savePreferences();
  }

  function startSendPanelResize(event: PointerEvent) {
    event.preventDefault();
    resizing = true;
    const move = (moveEvent: PointerEvent) => {
      const usableHeight = Math.max(window.innerHeight - 69, 1);
      config.send_panel_ratio = clamp((window.innerHeight - 25 - moveEvent.clientY) / usableHeight, 0.12, 0.5);
    };
    const finish = () => {
      resizing = false;
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", finish);
      window.removeEventListener("pointercancel", finish);
      void savePreferences();
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", finish, { once: true });
    window.addEventListener("pointercancel", finish, { once: true });
  }

  function startExtendedPanelResize(event: PointerEvent) {
    event.preventDefault();
    resizing = true;
    const move = (moveEvent: PointerEvent) => {
      config.extended_panel_ratio = clamp((window.innerWidth - moveEvent.clientX) / Math.max(window.innerWidth, 1), 0.25, 0.6);
    };
    const finish = () => {
      resizing = false;
      window.removeEventListener("pointermove", move);
      window.removeEventListener("pointerup", finish);
      window.removeEventListener("pointercancel", finish);
      void savePreferences();
    };
    window.addEventListener("pointermove", move);
    window.addEventListener("pointerup", finish, { once: true });
    window.addEventListener("pointercancel", finish, { once: true });
  }

  function clamp(value: number, minimum: number, maximum: number) {
    return Math.min(Math.max(Number.isFinite(value) ? value : minimum, minimum), maximum);
  }

  function clearReceive() {
    receiveLines = [];
    receivedCount = 0;
    sentCount = 0;
    lineId = 0;
  }

  function showError(error: unknown) {
    errorText = stringifyError(error);
  }

  function stringifyError(error: unknown) {
    return error instanceof Error ? error.message : String(error);
  }

  function delay(ms: number) {
    return new Promise((resolve) => window.setTimeout(resolve, ms));
  }

  function stripAnsi(value: string) {
    return value.replace(/\x1b\[[0-?]*[ -/]*[@-~]/g, "").replace(/\x1b/g, "␛");
  }

  function escapeHtml(value: string) {
    return value.replace(/[&<>"']/g, (char) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[char] ?? char);
  }

  function ansiToHtml(value: string) {
    const state: { foreground?: string; background?: string; bold: boolean; underline: boolean } = {
      bold: false,
      underline: false,
    };
    let result = "";
    let position = 0;
    const pattern = /\x1b\[([0-?]*)([ -/]*)([@-~])/g;
    for (const match of value.matchAll(pattern)) {
      result += renderAnsiSegment(value.slice(position, match.index), state);
      if (match[3] === "m" && !match[2]) {
        applySgrCodes(state, match[1]);
      }
      position = (match.index ?? 0) + match[0].length;
    }
    result += renderAnsiSegment(value.slice(position), state);
    return result;
  }

  function renderAnsiSegment(value: string, state: { foreground?: string; background?: string; bold: boolean; underline: boolean }) {
    const content = escapeHtml(value.replace(/\x1b/g, "␛"));
    if (!content) return "";
    const styles = [
      state.foreground ? `color:${state.foreground}` : "",
      state.background ? `background-color:${state.background}` : "",
      state.bold ? "font-weight:700" : "",
      state.underline ? "text-decoration:underline" : "",
    ].filter(Boolean);
    return styles.length ? `<span style="${styles.join(";")}">${content}</span>` : content;
  }

  function applySgrCodes(
    state: { foreground?: string; background?: string; bold: boolean; underline: boolean },
    parameters: string,
  ) {
    const codes = (parameters || "0").split(";").map((value) => value === "" ? 0 : Number(value));
    for (let index = 0; index < codes.length; index += 1) {
      const code = codes[index];
      if (!Number.isInteger(code)) continue;
      if (code === 0) {
        state.foreground = undefined;
        state.background = undefined;
        state.bold = false;
        state.underline = false;
      } else if (code === 1) state.bold = true;
      else if (code === 4) state.underline = true;
      else if (code === 22) state.bold = false;
      else if (code === 24) state.underline = false;
      else if (code >= 30 && code <= 37) state.foreground = ansiColor(code - 30);
      else if (code === 39) state.foreground = undefined;
      else if (code >= 40 && code <= 47) state.background = ansiColor(code - 40);
      else if (code === 49) state.background = undefined;
      else if (code >= 90 && code <= 97) state.foreground = ansiColor(code - 90 + 8);
      else if (code >= 100 && code <= 107) state.background = ansiColor(code - 100 + 8);
      else if (code === 38 || code === 48) {
        const target = code === 38 ? "foreground" : "background";
        if (codes[index + 1] === 5 && isByte(codes[index + 2])) {
          state[target] = ansiColor(codes[index + 2]);
          index += 2;
        } else if (codes[index + 1] === 2 && codes.slice(index + 2, index + 5).every(isByte)) {
          state[target] = `rgb(${codes[index + 2]},${codes[index + 3]},${codes[index + 4]})`;
          index += 4;
        }
      }
    }
  }

  function isByte(value: number) {
    return Number.isInteger(value) && value >= 0 && value <= 255;
  }

  function ansiColor(index: number) {
    const palette = [
      "#20252b", "#c83f49", "#3c884e", "#a87500", "#3569b8", "#8a55a3", "#147d86", "#d9dde2",
      "#6b7280", "#ff6b74", "#62b875", "#e0b43c", "#6495e5", "#b981cf", "#35aab5", "#ffffff",
    ];
    if (index < 16) return palette[index];
    if (index <= 231) {
      const value = index - 16;
      const levels = [0, 95, 135, 175, 215, 255];
      return `rgb(${levels[Math.floor(value / 36)]},${levels[Math.floor(value / 6) % 6]},${levels[value % 6]})`;
    }
    const gray = 8 + (Math.min(index, 255) - 232) * 10;
    return `rgb(${gray},${gray},${gray})`;
  }
</script>

<svelte:head><title>{instanceId > 1 ? `Z_COM - 实例 ${instanceId}` : "Z_COM"}</title></svelte:head>

<main class="app-shell" class:resizing style={`--send-panel-height:calc(${config.send_panel_ratio * 100}vh - ${config.send_panel_ratio * 69}px);--extended-panel-width:${config.extended_panel_ratio * 100}%`}>
  <header class="connection-bar">
    <div class="mode-switch" aria-label="传输方式">
      <button class:active={mode === "serial"} onclick={() => setMode("serial")}>串口</button>
      <button class:active={mode === "socket"} onclick={() => setMode("socket")}>Socket</button>
      <button class:active={mode === "probe"} onclick={() => setMode("probe")}>调试探针 / RTT</button>
    </div>

    <button class="icon-button" title={refreshingDevices ? "正在扫描设备" : "刷新设备"} onclick={refreshDevices} disabled={connected || connecting || refreshingDevices}><span class:spinning={refreshingDevices}><RefreshCw size={17} /></span></button>

    <div class="connection-fields">
      {#if mode === "serial"}
        <label><span>设备</span><select bind:value={config.port} onchange={savePreferences} disabled={connected}>{#each serialDevices as device}<option value={device.id}>{device.label}</option>{/each}</select></label>
        <label class="short"><span>波特率</span><input list="baudrate-options" inputmode="numeric" bind:value={config.baudrate} onchange={updateBaudrate} disabled={connecting} /></label>
        <datalist id="baudrate-options">{#each baudrateOptions as baud}<option value={baud}></option>{/each}</datalist>
      {:else if mode === "socket"}
        <div class="mini-switch"><button class:active={socketProtocol === "TCP"} onclick={() => setSocketProtocol("TCP")}>TCP</button><button class:active={socketProtocol === "UDP"} onclick={() => setSocketProtocol("UDP")}>UDP</button></div>
        <div class="mini-switch"><button class:active={socketRole === "Client"} onclick={() => setSocketRole("Client")}>客户端</button><button class:active={socketRole === "Server"} onclick={() => setSocketRole("Server")}>服务端</button></div>
        <label class="host"><span>地址</span><input list={socketRole === "Server" ? "local-ipv4-addresses" : undefined} bind:value={config.socket_host} onblur={savePreferences} disabled={connected} /></label>
        <datalist id="local-ipv4-addresses">{#each localIpv4Addresses as address}<option value={address}></option>{/each}</datalist>
        <label class="port"><span>端口</span><input type="number" min="1" max="65535" bind:value={config.socket_port} onblur={savePreferences} disabled={connected} /></label>
      {:else}
        <label class="probe-select"><span>调试探针</span><select bind:value={selectedProbe} onchange={persistSelectedProbe} disabled={connected}>{#each probeDevices as device}<option value={device.id}>{device.label}</option>{/each}</select></label>
        <label class="chip"><span>目标芯片</span><input list="chip-history" bind:value={config.probe_chip} onblur={savePreferences} placeholder="例如 nRF52840_xxAA" disabled={connected} /></label>
        <datalist id="chip-history">{#each probeChipOptions as chip}<option value={chip}></option>{/each}</datalist>
        <label class="reset-check"><input type="checkbox" bind:checked={config.probe_reset} onchange={savePreferences} disabled={connected} />连接后复位</label>
      {/if}
    </div>

    <div class="top-actions">
      <button class:danger={connected} class="command-button" onclick={toggleConnection} disabled={connecting}>
        {#if connected}<Link2Off size={17} />断开{:else}<Link size={17} />{connecting ? "连接中" : "连接"}{/if}
      </button>
      <button class="icon-button" title="更多设置" onclick={openSettings}><Settings size={17} /></button>
      <button class="icon-button" title="关于 Z_COM" onclick={() => aboutOpen = true}><CircleHelp size={17} /></button>
      <button class="icon-button" class:active={sidebarOpen} title="扩展发送" onclick={toggleSidebar}>{#if sidebarOpen}<ChevronRight size={17} />{:else}<ChevronLeft size={17} />{/if}</button>
    </div>
  </header>

  {#if errorText}
    <div class="error-strip"><span>{errorText}</span><button title="关闭" onclick={() => errorText = ""}><Minus size={15} /></button></div>
  {/if}

  <section class="workspace" class:with-sidebar={sidebarOpen}>
    <section class="receive-pane">
      <div class="pane-toolbar">
        <strong>接收区域</strong>
        <div class="mode-switch compact">
          <button class:active={config.display_mode === "HEX"} onclick={() => updateDisplayMode("HEX")}>HEX</button>
          <button class:active={config.display_mode === "ASCII"} onclick={() => updateDisplayMode("ASCII")}>ASCII</button>
          <button class:active={config.display_mode === "MIXED"} onclick={() => updateDisplayMode("MIXED")}>HEX+ASCII</button>
        </div>
        <label class="inline-check"><input type="checkbox" bind:checked={config.auto_scroll} onchange={savePreferences} />自动滚动</label>
        <span class="toolbar-spacer"></span>
        <button class="icon-button subtle" title="清空报文并重置收发计数" onclick={clearReceive}><Eraser size={16} /></button>
      </div>
      <pre class="receive-output" bind:this={receiveView} aria-label="接收数据" oncontextmenu={showReceiveContextMenu}>{@html receiveHtml}</pre>
    </section>

    {#if sidebarOpen}
      <button class="extended-splitter" aria-label="调整扩展发送区域宽度" title="拖动调整扩展发送区域宽度" onpointerdown={startExtendedPanelResize}></button>
      <aside class="extended-pane">
        <div class="pane-toolbar">
          <strong>扩展发送</strong><span class="toolbar-spacer"></span>
          <button class="icon-button subtle" title="导入" onclick={importExtended}><Upload size={15} /></button>
          <button class="icon-button subtle" title="导出" onclick={exportExtended}><Download size={15} /></button>
          <button class="icon-button subtle" title="添加条目" onclick={addExtendedItem}><Plus size={16} /></button>
        </div>
        <div class="extended-table-wrap">
          <table>
            <thead><tr><th>HEX</th><th>数据内容 / 注释</th><th>序号</th><th>延时</th><th></th></tr></thead>
            <tbody>
              {#each extended.items as item, index (item.id)}
                <tr>
                  <td><input aria-label="HEX 格式" type="checkbox" bind:checked={item.is_hex} onchange={persistExtended} /></td>
                  <td><input class="item-data" aria-label="发送数据" title={item.is_hex ? "输入 HEX 字节，例如 01 03 00 00" : "支持转义：\\r  \\n  \\t  \\0  \\\\  \\xNN"} placeholder={item.is_hex ? "HEX 字节" : "ASCII，支持 \\r \\n \\t \\0 \\xNN"} bind:value={item.data} onblur={persistExtended} /><input class="item-comment" aria-label="注释" placeholder="注释" bind:value={item.comment} onblur={persistExtended} /></td>
                  <td><input aria-label="发送序号" type="number" min="0" bind:value={item.sort_order} onblur={persistExtended} /></td>
                  <td><input aria-label="发送延时" type="number" min="0" bind:value={item.delay} onblur={persistExtended} /></td>
                  <td class="row-actions">
                    <button title="发送此条" onclick={() => sendExtendedItem(item)}><Send size={14} /></button>
                    <button title="上移" onclick={() => moveExtendedItem(index, -1)} disabled={index === 0}><ArrowUp size={14} /></button>
                    <button title="下移" onclick={() => moveExtendedItem(index, 1)} disabled={index === extended.items.length - 1}><ArrowDown size={14} /></button>
                    <button title="删除" onclick={() => deleteExtendedItem(index)}><Trash2 size={14} /></button>
                  </td>
                </tr>
              {/each}
            </tbody>
          </table>
          {#if !extended.items.length}<div class="empty-state">暂无扩展发送条目</div>{/if}
        </div>
        <div class="extended-footer">
          <label><input type="checkbox" bind:checked={extended.settings.multi_send} onchange={persistExtended} />多条发送</label>
          <label><input type="checkbox" bind:checked={extended.settings.loop_send} onchange={persistExtended} />循环发送</label>
          <button class="run-button" onclick={toggleExtendedRun}>{#if extendedRunning}<Square size={15} fill="currentColor" />停止{:else}<Play size={15} fill="currentColor" />启动发送{/if}</button>
          <span class="help" title="ASCII 支持 \\r、\\n、\\t、\\0、\\、\\xNN 转义；序号 0 不参与发送；序号决定发送顺序；延时单位为毫秒"><CircleHelp size={15} /></span>
        </div>
      </aside>
    {/if}
  </section>

  <button class="send-splitter" aria-label="调整发送区域高度" title="拖动调整发送区域高度" onpointerdown={startSendPanelResize}></button>

  <section class="send-pane">
    <div class="send-options">
      <div class="mode-switch compact"><button class:active={config.send_mode === "ASCII"} onclick={() => updateSendMode("ASCII")}>ASCII</button><button class:active={config.send_mode === "HEX"} onclick={() => updateSendMode("HEX")}>HEX</button></div>
      <label><input type="checkbox" bind:checked={appendCrLf} />添加 CRLF</label>
      <span class="toolbar-spacer"></span>
      <label><input type="checkbox" bind:checked={autoSend} disabled={!connected} />自动发送</label>
      <input class="interval" aria-label="自动发送间隔" type="number" min="100" max="60000" bind:value={config.auto_send_interval} onblur={savePreferences} /><span>ms</span>
    </div>
    <textarea bind:value={sendText} aria-label="发送数据" placeholder={config.send_mode === "HEX" ? "输入十六进制字节，例如 01 03 00 00" : "输入要发送的文本"}></textarea>
    <button class="send-button" onclick={() => sendCurrent()} disabled={!connected}><Send size={18} />发送</button>
  </section>

  <footer class="status-bar">
    <span class:online={connected} class="status-dot"></span><span>{statusText}</span>
    <span class="status-path" title={dataDirectory}>{mode === "probe" ? "调试探针 RTT" : mode === "socket" ? `${socketProtocol} ${socketRole === "Server" ? "服务端" : "客户端"}` : config.port || "未选择串口"}</span>
    <span class="toolbar-spacer"></span><span>实例 {instanceId}</span><span>发送 {sentCount.toLocaleString()} B</span><span>接收 {receivedCount.toLocaleString()} B</span><span>v{version}</span>
  </footer>
</main>

{#if settingsOpen}
  <div class="modal-backdrop" role="presentation" onclick={(event) => event.target === event.currentTarget && (settingsOpen = false)}>
    <div class="settings-dialog" role="dialog" aria-modal="true" aria-labelledby="settings-title">
      <header><h2 id="settings-title">更多设置</h2><button class="icon-button" title="关闭" onclick={() => settingsOpen = false}><Minus size={17} /></button></header>
      <div class="settings-body">
        <fieldset>
          <legend>串口参数</legend>
          <label>数据位<select bind:value={draftConfig.databits}>{#each [5, 6, 7, 8] as value}<option value={value}>{value}</option>{/each}</select></label>
          <label>停止位<select bind:value={draftConfig.stopbits}><option value={1}>1</option><option value={2}>2</option></select></label>
          <label>校验<select bind:value={draftConfig.parity}><option value="None">无</option><option value="Odd">奇校验</option><option value="Even">偶校验</option></select></label>
          <label>流控<select bind:value={draftConfig.flowcontrol}><option value="None">无</option><option value="Software">XON/XOFF</option><option value="Hardware">RTS/CTS</option></select></label>
          <p class="parameter-note">已列出跨平台串口后端的全部通用选项。1.5 停止位、Mark/Space 校验等非通用能力不提供伪选项；具体组合仍以当前设备驱动支持情况为准。</p>
        </fieldset>
        <fieldset>
          <legend>接收与显示</legend>
          <label>帧超时 (ms)<input type="number" min="1" max="5000" bind:value={draftConfig.frame_timeout} /></label>
          <label class="check-row"><input type="checkbox" bind:checked={draftConfig.display_ansi} />ANSI 颜色显示（ASCII / HEX+ASCII）</label>
          <label class="check-row"><input type="checkbox" bind:checked={draftConfig.auto_scroll} />自动滚动</label>
        </fieldset>
        <fieldset class="probe-settings">
          <legend>调试探针 / RTT</legend>
          <label class="check-row full"><input type="checkbox" bind:checked={draftConfig.support_probes} />扫描调试探针</label>
          <label class="check-row full"><input type="checkbox" bind:checked={draftConfig.show_generic_jtag_adapters} disabled={!draftConfig.support_probes} />显示通用 FTDI/JTAG 适配器</label>
          <div class="sdk-path-setting">
            <label>J-Link SDK 路径<input bind:value={draftConfig.jlink_sdk_path} placeholder="留空自动查找" onblur={refreshJlinkSdkStatus} /></label>
            <div><button type="button" onclick={chooseJlinkSdkDirectory}>选择目录</button><button type="button" onclick={chooseJlinkSdkLibrary}>选择动态库</button><button type="button" onclick={() => { draftConfig.jlink_sdk_path = ""; void refreshJlinkSdkStatus(); }}>自动查找</button></div>
            <span title={jlinkSdkStatus}>{jlinkSdkStatus}</span>
          </div>
          <label>目标芯片<input list="chip-history" bind:value={draftConfig.probe_chip} placeholder="SEGGER / probe-rs 目标名称" /></label>
          <label>SWD 速度 (kHz)<input type="number" min="10" max="50000" bind:value={draftConfig.probe_speed} /></label>
          <p class="driver-note">
            J-Link 使用已安装的 SEGGER 官方驱动；其他探针使用 probe-rs。将 probe-rs Target YAML 放入自定义 MCU 目录即可扩展非 J-Link 目标；连接时会自动重新加载。
            当前已加载 {customProbeTargets.length} 个自定义目标。
            <button type="button" onclick={openProbeTargetDirectory}>打开目录</button>
            <button type="button" onclick={refreshCustomProbeTargets} disabled={refreshingCustomTargets}>{refreshingCustomTargets ? "加载中..." : "重新加载"}</button>
            Windows 下部分非 J-Link 探针需要 WinUSB 驱动；J-Link 无需切换 SEGGER 驱动。
          </p>
        </fieldset>
        <fieldset class="data-settings">
          <legend>数据存储</legend>
          <p>设置自动保存到 <code>settings.json</code>，扩展发送自动保存到 <code>extended_send.json</code>；通信日志自动写入配置目录同级的 <code>logs</code> 文件夹。</p>
          <div class="storage-path" title={dataDirectory}>{dataDirectory}</div>
          <button type="button" onclick={openDataDirectory}>打开配置目录</button>
        </fieldset>
      </div>
      <footer><button onclick={() => settingsOpen = false}>取消</button><button class="primary" onclick={applySettings}>应用</button></footer>
    </div>
  </div>
{/if}

{#if aboutOpen}
  <div class="modal-backdrop" role="presentation" onclick={(event) => event.target === event.currentTarget && (aboutOpen = false)}>
    <div class="about-dialog" role="dialog" aria-modal="true" aria-labelledby="about-title">
      <header><h2 id="about-title">关于 Z_COM</h2><button class="icon-button" title="关闭" onclick={() => aboutOpen = false}><Minus size={17} /></button></header>
      <div class="about-body">
        <div class="about-logo">Z</div>
        <dl>
          <dt>软件版本</dt><dd>v{version}</dd>
          <dt>构建时间</dt><dd>{buildTimestamp ? new Date(buildTimestamp * 1000).toLocaleString("zh-CN", { hour12: false }) : "未知"}</dd>
          <dt>实例索引</dt><dd>实例 {instanceId}</dd>
          <dt>通信后端</dt><dd>串口、TCP/UDP、SEGGER J-Link RTT、probe-rs RTT</dd>
          <dt>数据目录</dt><dd title={dataDirectory}>{dataDirectory}</dd>
        </dl>
      </div>
      <footer><button disabled title="远期开展计划">检查更新</button><button class="primary" onclick={() => aboutOpen = false}>确定</button></footer>
    </div>
  </div>
{/if}

{#if receiveContextMenu}
  <button class="context-menu-shield" aria-label="关闭右键菜单" onclick={() => receiveContextMenu = null} oncontextmenu={(event) => { event.preventDefault(); receiveContextMenu = null; }}></button>
  <div class="receive-context-menu" style={`left:${receiveContextMenu.x}px;top:${receiveContextMenu.y}px`} role="menu" tabindex="-1" oncontextmenu={(event) => event.preventDefault()}>
    <button role="menuitem" disabled={!receiveContextSelection} onclick={copyReceiveSelection}><span>复制</span><kbd>Ctrl+C</kbd></button>
    <button role="menuitem" disabled={!receiveLines.length} onclick={selectAllReceive}><span>全选</span><kbd>Ctrl+A</kbd></button>
    <div class="context-separator"></div>
    <button role="menuitem" disabled={!receiveLines.length} onclick={clearReceiveFromContextMenu}><span>清空报文并重置计数</span></button>
    <button role="menuitemcheckbox" aria-checked={config.display_ansi} onclick={toggleAnsiFromContextMenu}><span><i>{config.display_ansi ? "✓" : ""}</i>ANSI 颜色显示</span></button>
  </div>
{/if}

<style>
  :global(*) { box-sizing: border-box; }
  :global(html), :global(body) { margin: 0; width: 100%; height: 100%; overflow: hidden; }
  :global(body) { font-family: "Segoe UI", "Microsoft YaHei UI", sans-serif; color: #202020; background: #f0f0f0; font-size: 13px; letter-spacing: 0; }
  :global(button), :global(input), :global(select), :global(textarea) { font: inherit; letter-spacing: 0; }
  :global(button) { color: inherit; }
  :global(input), :global(select), :global(textarea) { border: 1px solid #b8b8b8; background: #fff; color: #202020; border-radius: 2px; outline: none; }
  :global(input:focus), :global(select:focus), :global(textarea:focus) { border-color: #0078d4; box-shadow: 0 0 0 1px rgba(0, 120, 212, .12); }

  .app-shell { height: 100vh; display: grid; grid-template-rows: 44px auto minmax(160px, 1fr) 5px clamp(96px, var(--send-panel-height), 50vh) 25px; background: #f0f0f0; }
  .app-shell.resizing { user-select: none; cursor: row-resize; }
  .connection-bar { grid-row: 1; display: flex; align-items: center; gap: 7px; padding: 5px 7px; background: #f5f5f5; border-bottom: 1px solid #b8b8b8; min-width: 0; }
  .connection-fields { display: flex; align-items: center; gap: 7px; min-width: 0; flex: 1; }
  .connection-fields label { display: flex; align-items: center; gap: 5px; min-width: 0; }
  .connection-fields label > span { color: #404040; white-space: nowrap; }
  .connection-fields select, .connection-fields input { height: 30px; padding: 4px 7px; min-width: 130px; }
  .connection-fields label:first-child select { width: clamp(160px, 23vw, 300px); }
  .connection-fields .short input { min-width: 90px; width: 104px; }
  .connection-fields .host input { width: 150px; }
  .connection-fields .port input { width: 82px; min-width: 0; }
  .connection-fields .probe-select select { width: clamp(190px, 27vw, 360px); }
  .connection-fields .chip input { width: clamp(145px, 18vw, 220px); }
  .connection-fields .reset-check { white-space: nowrap; }
  .connection-fields .reset-check input { width: auto; min-width: 0; height: auto; padding: 0; }
  .top-actions { display: flex; align-items: center; gap: 4px; }
  button { border: 1px solid #adadad; background: #f5f5f5; border-radius: 2px; min-height: 28px; cursor: pointer; }
  button:hover:not(:disabled) { background: #e5f1fb; border-color: #0078d4; }
  button:disabled { opacity: .48; cursor: default; }
  .icon-button { width: 30px; height: 30px; padding: 0; display: inline-grid; place-items: center; flex: 0 0 30px; }
  .icon-button.active { color: #202020; background: #e1e1e1; border-color: #777; }
  .spinning { display: grid; animation: spin .8s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }
  .icon-button.subtle { border-color: transparent; background: transparent; }
  .command-button, .run-button { display: inline-flex; align-items: center; justify-content: center; gap: 6px; background: #f44336; color: white; border-color: #d7352b; font-weight: 600; padding: 0 12px; white-space: nowrap; }
  .command-button:hover:not(:disabled) { background: #e53935; border-color: #c62828; }
  .command-button.danger, .run-button { background: #4caf50; border-color: #429846; }
  .command-button.danger:hover:not(:disabled), .run-button:hover:not(:disabled) { background: #43a047; border-color: #388e3c; }
  .send-button { display: inline-flex; align-items: center; justify-content: center; gap: 6px; background: #f5f5f5; color: #202020; border-color: #adadad; font-weight: 400; padding: 0 12px; white-space: nowrap; }
  .send-button:hover:not(:disabled) { background: #e5f1fb; border-color: #0078d4; }
  .mode-switch, .mini-switch { display: inline-flex; border: 1px solid #adadad; border-radius: 2px; overflow: hidden; flex: 0 0 auto; }
  .mode-switch button, .mini-switch button { border: 0; border-right: 1px solid #c8c8c8; border-radius: 0; min-height: 29px; padding: 0 10px; white-space: nowrap; }
  .mode-switch button:last-child, .mini-switch button:last-child { border-right: 0; }
  .mode-switch button.active, .mini-switch button.active { color: #202020; background: #e1e1e1; box-shadow: inset 0 0 0 1px #777; }
  .mode-switch.compact button { min-height: 24px; padding: 0 8px; font-size: 12px; }
  .mini-switch button { min-height: 28px; padding: 0 7px; font-size: 12px; }
  .error-strip { grid-row: 2; min-height: 29px; display: flex; align-items: center; gap: 8px; padding: 4px 9px; color: #842b32; background: #f7e4e5; border-bottom: 1px solid #dfaeb2; }
  .error-strip span { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .error-strip button { width: 24px; min-height: 20px; border: 0; background: transparent; display: grid; place-items: center; }
  .workspace { grid-row: 3; min-height: 0; display: grid; grid-template-columns: minmax(0, 1fr); padding: 4px 4px 0; }
  .workspace.with-sidebar { grid-template-columns: minmax(350px, 1fr) 5px clamp(280px, var(--extended-panel-width), 70%); }
  .receive-pane, .extended-pane, .send-pane { min-width: 0; background: #fff; border: 1px solid #a9a9a9; }
  .receive-pane, .extended-pane { min-height: 0; display: flex; flex-direction: column; }
  .pane-toolbar { height: 33px; display: flex; align-items: center; gap: 8px; padding: 3px 6px; background: #f5f5f5; border-bottom: 1px solid #c8c8c8; flex: 0 0 33px; }
  .pane-toolbar strong { font-size: 13px; }
  .toolbar-spacer { flex: 1; }
  .inline-check, .send-options label, .extended-footer label { display: flex; align-items: center; gap: 4px; white-space: nowrap; }
  .receive-output { margin: 0; padding: 7px 9px; flex: 1; min-height: 0; overflow: auto; white-space: pre-wrap; overflow-wrap: anywhere; font: 12.5px/1.48 "Cascadia Mono", Consolas, monospace; background: #fff; color: #202020; }
  :global(.display-timestamp) { color: #008b8b; font-weight: 600; }
  :global(.display-arrow) { color: #1f6feb; font-weight: 700; }
  :global(.display-label) { font-weight: 700; }
  :global(.display-label-hex) { color: #b15c00; }
  :global(.display-label-ascii) { color: #18794e; }
  :global(.display-data) { color: #202020; font-weight: 400; }
  :global(.display-event-marker) { color: #6b7280; font-weight: 700; }
  :global(.display-event) { font-weight: 600; }
  :global(.display-event-connected) { color: #18794e; }
  :global(.display-event-disconnected), :global(.display-event-info) { color: #5f6b73; }
  :global(.display-event-error) { color: #b42318; }
  :global(.display-event-warning) { color: #a15c00; }
  :global(.display-event-peer) { color: #6f42c1; }
  .context-menu-shield { position: fixed; inset: 0; z-index: 20; width: auto; height: auto; min-height: 0; padding: 0; border: 0; background: transparent; cursor: default; }
  .context-menu-shield:hover:not(:disabled) { border: 0; background: transparent; }
  .receive-context-menu { position: fixed; z-index: 21; width: 190px; padding: 4px; color: #202020; background: #fff; border: 1px solid #a8a8a8; border-radius: 3px; box-shadow: 0 5px 18px rgba(0, 0, 0, .22); }
  .receive-context-menu button { width: 100%; min-height: 29px; padding: 0 9px; display: flex; align-items: center; justify-content: space-between; border: 0; background: transparent; text-align: left; }
  .receive-context-menu button:hover:not(:disabled) { background: #e5f1fb; border-color: transparent; }
  .receive-context-menu button:disabled { opacity: .45; }
  .receive-context-menu kbd { color: #707070; font: 12px "Segoe UI", sans-serif; }
  .receive-context-menu i { display: inline-block; width: 18px; font-style: normal; }
  .context-separator { height: 1px; margin: 4px 5px; background: #dedede; }
  .extended-table-wrap { flex: 1; min-height: 0; overflow: auto; }
  table { width: 100%; border-collapse: collapse; table-layout: fixed; font-size: 12px; }
  th { position: sticky; top: 0; z-index: 1; background: #f0f0f0; border-bottom: 1px solid #b8b8b8; height: 27px; font-weight: 600; }
  th:nth-child(1) { width: 36px; } th:nth-child(3) { width: 48px; } th:nth-child(4) { width: 64px; } th:nth-child(5) { width: 92px; }
  td { border-bottom: 1px solid #e0e4e7; padding: 3px; text-align: center; vertical-align: middle; }
  td input[type="number"] { width: 100%; height: 24px; padding: 2px 3px; }
  .item-data, .item-comment { width: 100%; height: 24px; padding: 2px 5px; }
  .item-comment { border-top: 0; color: #667078; background: #fafbfb; }
  .row-actions { display: grid; grid-template-columns: repeat(4, 21px); gap: 1px; }
  .row-actions button { min-height: 21px; width: 21px; padding: 0; display: grid; place-items: center; border-color: transparent; background: transparent; }
  .empty-state { padding: 28px 10px; text-align: center; color: #7b858d; }
  .extended-footer { min-height: 37px; display: flex; align-items: center; gap: 8px; padding: 4px 6px; background: #f5f5f5; border-top: 1px solid #c8c8c8; }
  .extended-footer .run-button { margin-left: auto; min-height: 27px; }
  .help { color: #69747d; display: grid; place-items: center; }
  .extended-splitter, .send-splitter { min-width: 0; min-height: 0; padding: 0; border: 0; border-radius: 0; background: #d5dadd; touch-action: none; }
  .extended-splitter { width: 5px; height: auto; cursor: col-resize; }
  .send-splitter { grid-row: 4; width: auto; height: 5px; margin: 0 4px; cursor: row-resize; }
  .extended-splitter:hover, .send-splitter:hover { background: #0078d4; border: 0; }
  .send-pane { grid-row: 5; margin: 0 4px 4px; display: grid; grid-template-columns: minmax(0, 1fr) 82px; grid-template-rows: 32px minmax(0, 1fr); padding: 4px; gap: 3px 5px; }
  .send-options { grid-column: 1; display: flex; align-items: center; gap: 8px; min-width: 0; }
  .send-options .interval { width: 72px; height: 25px; padding: 2px 5px; }
  .send-pane textarea { grid-column: 1; grid-row: 2; resize: none; min-height: 0; padding: 6px 8px; font-family: "Cascadia Mono", Consolas, monospace; }
  .send-pane .send-button { grid-column: 2; grid-row: 1 / 3; width: 100%; }
  .status-bar { grid-row: 6; display: flex; align-items: center; gap: 7px; padding: 0 8px; color: #404040; border-top: 1px solid #b8b8b8; background: #f0f0f0; font-size: 12px; min-width: 0; }
  .status-dot { width: 8px; height: 8px; border-radius: 50%; background: #89939a; }
  .status-dot.online { background: #26834b; }
  .status-path { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; max-width: 42vw; }
  .modal-backdrop { position: fixed; inset: 0; z-index: 10; display: grid; place-items: center; padding: 18px; background: rgba(26, 32, 36, .38); }
  .settings-dialog { width: min(680px, 96vw); max-height: 90vh; display: flex; flex-direction: column; background: #f5f5f5; border: 1px solid #777; box-shadow: 0 16px 45px rgba(20, 25, 29, .24); }
  .settings-dialog > header { height: 42px; display: flex; align-items: center; padding: 0 10px 0 14px; border-bottom: 1px solid #bdc5cb; }
  .settings-dialog h2 { margin: 0; font-size: 15px; }
  .settings-dialog > header button { margin-left: auto; }
  .settings-body { overflow: auto; padding: 12px; display: grid; grid-template-columns: 1fr 1fr; gap: 10px; }
  fieldset { margin: 0; border: 1px solid #c2c9ce; padding: 10px; display: grid; grid-template-columns: 1fr 1fr; gap: 9px; }
  legend { padding: 0 5px; font-weight: 600; color: #46515a; }
  fieldset label { display: flex; flex-direction: column; gap: 4px; color: #56616a; }
  fieldset input, fieldset select { height: 29px; padding: 3px 6px; min-width: 0; width: 100%; }
  fieldset .check-row { flex-direction: row; align-items: center; grid-column: 1 / -1; }
  fieldset .check-row input { width: auto; height: auto; }
  .probe-settings, .data-settings { grid-column: 1 / -1; }
  .driver-note { grid-column: 1 / -1; margin: 2px 0 0; color: #6d5d31; background: #f5efdc; border-left: 3px solid #c49a38; padding: 6px 8px; font-size: 12px; }
  .parameter-note { grid-column: 1 / -1; margin: 0; color: #56616a; font-size: 12px; line-height: 1.45; }
  .driver-note button { margin: 0 3px; min-height: 24px; padding: 0 7px; }
  .sdk-path-setting { grid-column: 1 / -1; display: grid; grid-template-columns: minmax(0, 1fr) auto; gap: 5px 8px; align-items: end; }
  .sdk-path-setting > div { display: flex; gap: 5px; }
  .sdk-path-setting button { min-height: 29px; white-space: nowrap; }
  .sdk-path-setting > span { grid-column: 1 / -1; color: #667078; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: 12px; }
  .data-settings p { grid-column: 1 / -1; margin: 0; color: #56616a; line-height: 1.6; }
  .data-settings code { color: #7a3f00; font-family: "Cascadia Mono", Consolas, monospace; }
  .storage-path { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; padding: 6px 8px; color: #39434a; background: #fff; border: 1px solid #c2c9ce; font-family: "Cascadia Mono", Consolas, monospace; font-size: 12px; }
  .data-settings button { min-height: 29px; }
  .settings-dialog > footer { display: flex; justify-content: flex-end; gap: 7px; padding: 8px 12px; border-top: 1px solid #bdc5cb; }
  .settings-dialog > footer button { min-width: 72px; padding: 0 12px; }
  .settings-dialog > footer .primary { background: #4caf50; color: #fff; border-color: #429846; }
  .about-dialog { width: min(590px, 94vw); background: #f5f5f5; border: 1px solid #777; box-shadow: 0 16px 45px rgba(20, 25, 29, .24); }
  .about-dialog > header { height: 42px; display: flex; align-items: center; padding: 0 10px 0 14px; border-bottom: 1px solid #bdc5cb; }
  .about-dialog > header h2 { margin: 0; font-size: 15px; }
  .about-dialog > header button { margin-left: auto; }
  .about-body { display: grid; grid-template-columns: 70px minmax(0, 1fr); gap: 16px; padding: 18px; }
  .about-logo { width: 64px; height: 64px; display: grid; place-items: center; border-radius: 13px; color: #fff; background: #18794e; font-size: 34px; font-weight: 800; }
  .about-body dl { margin: 0; display: grid; grid-template-columns: 78px minmax(0, 1fr); gap: 9px 12px; align-items: start; }
  .about-body dt { color: #667078; }
  .about-body dd { margin: 0; overflow-wrap: anywhere; }
  .about-dialog > footer { display: flex; justify-content: flex-end; gap: 7px; padding: 8px 12px; border-top: 1px solid #bdc5cb; }
  .about-dialog > footer button { min-width: 82px; padding: 0 12px; }
  .about-dialog > footer .primary { background: #4caf50; color: #fff; border-color: #429846; }

  @media (max-width: 900px) {
    .connection-bar { flex-wrap: wrap; height: auto; }
    .app-shell { grid-template-rows: auto auto minmax(0, 1fr) 132px 25px; }
    .connection-fields { order: 3; flex-basis: 100%; }
    .top-actions { margin-left: auto; }
    .workspace.with-sidebar { grid-template-columns: minmax(280px, 1fr) minmax(300px, .85fr); }
  }
  @media (max-width: 700px) {
    .mode-switch:first-child button { padding: 0 6px; }
    .connection-fields { overflow-x: auto; padding-bottom: 2px; }
    .workspace.with-sidebar { grid-template-columns: 1fr; grid-template-rows: minmax(210px, 1fr) minmax(190px, .8fr); }
    .send-pane { margin-top: 4px; }
    .settings-body { grid-template-columns: 1fr; }
    .probe-settings, .data-settings { grid-column: 1; }
    .status-path { display: none; }
  }
</style>
