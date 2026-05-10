<template>
  <div class="schedule-view">
    <!-- ─── Toolbar ─────────────────────────────────────────────────────── -->
    <v-toolbar class="schedule-toolbar" density="compact" flat>
      <!-- Product selector: ALL products from config.json -->
      <v-select
        v-model="selectedId"
        :items="productItems"
        item-title="label"
        item-value="id"
        label="Product"
        density="compact"
        variant="outlined"
        hide-details
        style="max-width: 260px"
        @update:model-value="onProductChange"
      />


      <v-divider vertical class="mx-3" />

      <!-- Milestone toggle -->
      <v-btn-toggle
        v-model="milestonesToggle"
        rounded="pill"
        density="compact"
        color="primary"
        variant="outlined"
        @update:model-value="scheduleStore.toggleMilestones()"
      >
        <v-btn value="on" size="small" prepend-icon="mdi-flag">
          マイルストーン
        </v-btn>
      </v-btn-toggle>

      <v-spacer />

      <!-- Edit schedule -->
      <v-btn
        size="small" variant="tonal" color="primary" prepend-icon="mdi-pencil"
        :disabled="!scheduleStore.parsedGantt || !currentEntryExists"
        class="mr-2"
        @click="editorDialog = true"
      >
        編集
      </v-btn>

      <!-- Reload -->
      <v-btn icon size="small" @click="reload" class="mr-1">
        <v-icon>mdi-refresh</v-icon>
      </v-btn>
    </v-toolbar>

    <!-- ─── Error banner ──────────────────────────────────────────────────── -->
    <v-alert
      v-if="scheduleStore.errorMessage"
      type="error"
      class="ma-2"
      closable
      @click:close="scheduleStore.errorMessage = ''"
    >
      {{ scheduleStore.errorMessage }}
    </v-alert>

    <!-- ─── Loading ───────────────────────────────────────────────────────── -->
    <div v-if="scheduleStore.loading" class="d-flex justify-center align-center" style="height: 300px">
      <v-progress-circular indeterminate color="primary" size="48" />
    </div>

    <!-- ─── No products in config ─────────────────────────────────────────── -->
    <v-alert
      v-else-if="productItems.length === 0"
      type="info"
      class="ma-4"
    >
      <p class="mb-2">config.json に Product が登録されていません。</p>
      <p class="text-body-2 text-medium-emphasis">
        Settings の <strong>Products</strong> にProductを追加してください。
      </p>
    </v-alert>

    <!-- ─── Selected product has no schedule mapping ──────────────────────── -->
    <v-alert
      v-else-if="selectedId && !currentEntryExists"
      type="warning"
      class="ma-4"
    >
      <p class="mb-2">「{{ selectedLabel }}」のスケジュールファイルが未登録です。</p>
      <v-btn
        color="warning" variant="flat" size="small"
        prepend-icon="mdi-file-plus"
        @click="openAddDialogForCurrent"
      >
        ファイルパスを登録する
      </v-btn>
    </v-alert>

    <!-- ─── Gantt chart ────────────────────────────────────────────────────── -->
    <div v-else-if="selectedId && currentEntryExists" class="chart-wrapper">
      <div v-if="!scheduleStore.parsedGantt" class="d-flex justify-center align-center" style="height:200px">
        <v-progress-circular indeterminate color="secondary" />
      </div>
      <div v-else ref="chartEl" class="mermaid-output" />
    </div>

    <!-- ─── Schedule Editor dialog ────────────────────────────────────────── -->
    <ScheduleEditor v-model="editorDialog" @saved="reload" />

    <!-- ─── Register file path dialog ─────────────────────────────────────── -->
    <v-dialog v-model="addDialog" max-width="540">
      <v-card class="pa-2">
        <v-card-title>
          <v-icon class="mr-2" color="primary">mdi-file-document-plus</v-icon>
          スケジュールファイル登録
        </v-card-title>
        <v-card-text>
          <v-select
            v-model="newEntry.product_id"
            :items="productItems"
            item-title="label"
            item-value="id"
            label="Product"
            density="compact"
            class="mb-3"
          />
          <v-text-field
            v-model="newEntry.file"
            label="Mermaid ファイルパス (.mmd)"
            placeholder="C:\Users\...\schedule_a.mmd"
            density="compact"
            hint="存在しない場合は自動作成されます"
            persistent-hint
          />
        </v-card-text>
        <v-card-actions>
          <v-spacer />
          <v-btn variant="text" @click="addDialog = false">キャンセル</v-btn>
          <v-btn
            color="primary" variant="flat"
            :disabled="!newEntry.product_id || !newEntry.file"
            @click="saveNewEntry"
          >
            登録
          </v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, watch, onMounted, nextTick } from "vue";
import mermaid from "mermaid";
import { useScheduleStore, buildMermaidText, getGanttDateRange, type ParsedGantt } from "../store/scheduleStore";
import { useMetaStore } from "../store/metaStore";
import { useIssueStore } from "../store/issueStore";
import { invoke } from "@tauri-apps/api/core";
import ScheduleEditor from "./ScheduleEditor.vue";

const scheduleStore = useScheduleStore();
const metaStore = useMetaStore();
const issueStore = useIssueStore();

const chartEl = ref<HTMLElement | null>(null);
const editorDialog = ref(false);
const addDialog = ref(false);
const selectedId = ref<string | null>(null);

// ─── Milestone toggle ─────────────────────────────────────────────────────────
const milestonesToggle = computed({
  get: () => (scheduleStore.showMilestones ? "on" : undefined),
  set: () => {},
});

// ─── Product list: ALL products from config.json ──────────────────────────────
const productItems = computed(() => {
  const configProducts = issueStore.config?.products ?? [];
  return configProducts.map((p) => ({ id: p.id, label: p.name }));
});

function isMapped(productId: string): boolean {
  return !!scheduleStore.scheduleConfig?.schedules.find(
    (s) => s.product_id === productId
  );
}

const currentEntryExists = computed(() =>
  selectedId.value ? isMapped(selectedId.value) : false
);

const selectedLabel = computed(
  () =>
    productItems.value.find((p) => p.id === selectedId.value)?.label ??
    selectedId.value ??
    ""
);

// ─── Add mapping dialog ───────────────────────────────────────────────────────
const newEntry = ref({ product_id: "", file: "" });

function openAddDialogForCurrent() {
  newEntry.value.product_id = selectedId.value ?? "";
  newEntry.value.file = "";
  addDialog.value = true;
}

async function saveNewEntry() {
  if (!newEntry.value.product_id || !newEntry.value.file) return;
  await scheduleStore.addScheduleEntry({ ...newEntry.value });
  const pid = newEntry.value.product_id;
  newEntry.value = { product_id: "", file: "" };
  addDialog.value = false;
  selectedId.value = pid;
  await scheduleStore.selectProduct(pid);
  await nextTick();
  await renderChart();
}

// ─── Mermaid rendering ────────────────────────────────────────────────────────
let mermaidInitialized = false;
let renderCount = 0;

function initMermaid() {
  if (mermaidInitialized) return;
  mermaid.initialize({
    startOnLoad: false,
    theme: "base",
    themeVariables: {
      background: "#0f172a",
      primaryColor: "#1e3a5f",
      primaryTextColor: "#e2e8f0",
      primaryBorderColor: "#334155",
      lineColor: "#64748b",
      secondaryColor: "#1e293b",
      tertiaryColor: "#0f172a",
    },
  });
  mermaidInitialized = true;
}

async function renderChart() {
  if (!scheduleStore.parsedGantt || !chartEl.value) return;

  const milestones = scheduleStore.showMilestones
    ? metaStore.milestones
        .filter((m) => !!m.due_date)
        .map((m) => ({ title: m.title, date: m.due_date! }))
    : [];

  const mermaidText = buildMermaidText(scheduleStore.parsedGantt, milestones);

  // ── Step 1: Mermaid render (errors here replace chart with error message) ─
  let svgEl: SVGSVGElement | null = null;
  try {
    initMermaid();
    renderCount++;
    const id = `gantt-${renderCount}`;
    const { svg } = await mermaid.render(id, mermaidText);
    if (chartEl.value) {
      chartEl.value.innerHTML = svg;
      svgEl = chartEl.value.querySelector("svg");
      if (svgEl) {
        const vb = svgEl.viewBox?.baseVal;
        if (vb && vb.width > 0) {
          svgEl.style.width = `${vb.width}px`;
          svgEl.style.minWidth = `${vb.width}px`;
        }
        svgEl.style.maxWidth = "none";
        svgEl.removeAttribute("height");
      }
    }
  } catch (e) {
    console.error("Mermaid render error:", e);
    if (chartEl.value) {
      chartEl.value.innerHTML = `<pre class="render-error">${String(e)}</pre>`;
    }
    return;
  }

  // ── Step 2: Post-processing (errors here are logged but do NOT clear chart) ─
  if (!svgEl) return;
  await nextTick();
  try { addSectionDividers(svgEl); } catch (e) { console.error("addSectionDividers error:", e); }
  try {
    if (scheduleStore.parsedGantt) addDependencyArrows(svgEl, scheduleStore.parsedGantt);
  } catch (e) { console.error("addDependencyArrows error:", e); }
  scrollToDefaultView();
}

/** Scroll the chart so today-1month is near the left edge */
function scrollToDefaultView() {
  const wrapper = chartEl.value;
  const svgEl = wrapper?.querySelector("svg");
  if (!wrapper || !svgEl || !scheduleStore.parsedGantt) return;

  // Use viewBox width as the intrinsic pixel width of the rendered SVG
  const vb = svgEl.viewBox?.baseVal;
  const svgIntrinsicWidth = vb && vb.width > 0 ? vb.width : svgEl.scrollWidth;
  if (svgIntrinsicWidth === 0) return;

  const { start: ganttStart, end: ganttEnd } = getGanttDateRange(scheduleStore.parsedGantt);
  const totalMs = ganttEnd.getTime() - ganttStart.getTime();
  if (totalMs <= 0) return;

  // Scroll so that today-1month appears at the left edge
  const viewportLeft = new Date();
  viewportLeft.setMonth(viewportLeft.getMonth() - 1);

  const offsetMs = Math.max(0, viewportLeft.getTime() - ganttStart.getTime());
  wrapper.scrollLeft = (offsetMs / totalMs) * svgIntrinsicWidth;
}

// ─── Watch for changes ────────────────────────────────────────────────────────
watch(
  [
    () => scheduleStore.parsedGantt,
    () => scheduleStore.showMilestones,
    () => metaStore.milestones,
  ],
  async () => {
    await nextTick();
    await renderChart();
  },
  { deep: true }
);

// ─── Product change handler ───────────────────────────────────────────────────
async function onProductChange(id: string) {
  selectedId.value = id;
  if (!isMapped(id)) {
    openAddDialogForCurrent();
    return;
  }
  await scheduleStore.selectProduct(id);
  await nextTick();
  await renderChart();
}

// ─── Reload ───────────────────────────────────────────────────────────────────
async function reload() {
  await scheduleStore.loadScheduleConfig();
  if (selectedId.value && isMapped(selectedId.value)) {
    await scheduleStore.selectProduct(selectedId.value);
    await nextTick();
    await renderChart();
  }
}

// ─── Init ─────────────────────────────────────────────────────────────────────
onMounted(async () => {
  // Ensure config is loaded (GitLab may be unavailable — local file read still works)
  if (!issueStore.config) {
    try {
      issueStore.config = await invoke("get_config");
    } catch (e) {
      console.error("Failed to load config:", e);
    }
  }

  await scheduleStore.loadScheduleConfig();

  // Auto-select: prefer first mapped product, else first config product
  if (productItems.value.length > 0) {
    const mapped = productItems.value.find((p) => isMapped(p.id));
    const first = mapped ?? productItems.value[0];
    selectedId.value = first.id;
    if (isMapped(first.id)) {
      await scheduleStore.selectProduct(first.id);
      await nextTick();
      await renderChart();
    }
  }
});

/** Draw horizontal divider lines BETWEEN sections (not between tasks) */
function addSectionDividers(svgEl: SVGSVGElement) {
  // sectionTitle elements are placed at the vertical center of each section row-group
  const titles = Array.from(
    svgEl.querySelectorAll<SVGTextElement>('.sectionTitle')
  );
  if (titles.length <= 1) return;

  // Get the y-coordinate (baseline) of each section title and sort ascending
  const yCenters = titles
    .map((t) => parseFloat(t.getAttribute('y') ?? '0'))
    .filter((y) => y > 0)
    .sort((a, b) => a - b);

  const svgWidth =
    svgEl.viewBox?.baseVal?.width ??
    parseFloat(svgEl.getAttribute('width') ?? '800');

  const ns = 'http://www.w3.org/2000/svg';

  // Place a divider at the midpoint between adjacent section centers
  for (let i = 1; i < yCenters.length; i++) {
    const dividerY = (yCenters[i - 1] + yCenters[i]) / 2;
    const line = document.createElementNS(ns, 'line');
    line.setAttribute('x1', '0');
    line.setAttribute('x2', String(svgWidth));
    line.setAttribute('y1', String(dividerY));
    line.setAttribute('y2', String(dividerY));
    line.setAttribute('stroke', '#475569');
    line.setAttribute('stroke-width', '2');
    line.setAttribute('class', 'section-divider');
    svgEl.appendChild(line);
  }
}

/** Draw dependency arrows (dependsOn) as dashed curved lines on the SVG */
function addDependencyArrows(svgEl: SVGSVGElement, gantt: ParsedGantt) {
  const hasDeps = gantt.sections.some((s) => s.tasks.some((t) => !!t.dependsOn));
  if (!hasDeps) return;

  /** Get absolute X/Y within the SVG container using CTM */
  const getSvgPos = (el: SVGElement) => {
    try {
      const bbox = (el as any).getBBox();
      const ctm = (el as any).getCTM();
      if (!ctm) return { x: 0, y: 0, w: 0, h: 0 };
      return {
        x: bbox.x * ctm.a + bbox.y * ctm.c + ctm.e,
        y: bbox.x * ctm.b + bbox.y * ctm.d + ctm.f,
        w: bbox.width * ctm.a,
        h: bbox.height * ctm.d,
      };
    } catch (e) {
      return { x: 0, y: 0, w: 0, h: 0 };
    }
  };

  // ── Calibrate using axis date labels (MM/DD tick marks) ──────────────────
  const allTexts = Array.from(svgEl.querySelectorAll<SVGTextElement>('text'));
  type AxisLabel = { date: Date; x: number };
  const axisLabels: AxisLabel[] = [];
  const todayYear = new Date().getFullYear();

  for (const el of allTexts) {
    const txt = el.textContent?.trim() ?? '';
    const m = txt.match(/^(\d{1,2})\/(\d{1,2})$/);
    if (!m) continue;
    const mo = parseInt(m[1]) - 1;
    const da = parseInt(m[2]);

    const pos = getSvgPos(el);
    if (pos.x <= 0) continue;

    let best: Date | null = null;
    let bestDiff = Infinity;
    const now = Date.now();
    for (const yr of [todayYear - 1, todayYear, todayYear + 1]) {
      const d = new Date(yr, mo, da);
      const diff = Math.abs(d.getTime() - now);
      if (diff < bestDiff) { bestDiff = diff; best = d; }
    }
    if (best) axisLabels.push({ date: best, x: pos.x });
  }
  axisLabels.sort((a, b) => a.x - b.x);
  if (axisLabels.length < 2) return;

  const pxPerDay = (axisLabels[1].x - axisLabels[0].x) / 7;
  if (pxPerDay <= 0) return;
  const refX = axisLabels[0].x;
  const refDate = axisLabels[0].date;

  const dateToSvgX = (dateStr: string): number => {
    const d = new Date(dateStr + 'T00:00:00');
    if (isNaN(d.getTime())) return -1;
    return refX + (d.getTime() - refDate.getTime()) / 86400000 * pxPerDay;
  };

  const parseDays = (dur: string): number => {
    const m = dur.match(/(\d+)([dwmM]?)/);
    if (!m) return 1;
    const n = parseInt(m[1]);
    switch (m[2]) {
      case 'w': return n * 7;
      case 'm': case 'M': return n * 30;
      default: return n;
    }
  };

  // ── 1. Find all potential task elements and sort them by Y (row order) ───
  const allTaskEls = Array.from(svgEl.querySelectorAll<SVGElement>('rect, polygon, path'))
    .filter(el => {
      const cls = el.getAttribute('class') ?? '';
      return cls.includes('task') || cls.includes('mile');
    });

  const rectPositions = allTaskEls.map((el) => {
    const pos = getSvgPos(el);
    const isMilestone = (el.getAttribute('class') ?? '').includes('mile');
    return {
      el,
      x: pos.x,
      y: pos.y,
      h: pos.h,
      w: pos.w,
      cx: pos.x + pos.w / 2,
      centerY: pos.y + pos.h / 2,
      isMilestone
    };
  });

  // Sort by Y position to get the logical row order
  // (Mermaid groups types in DOM, but Y positions always follow the defined order)
  rectPositions.sort((a, b) => a.y - b.y);

  // ── 2. Map logical tasks from gantt data to sorted SVG elements ──────────
  const aliasToPosMap = new Map<string, { rightX: number; leftX: number; y: number }>();
  let elementIdx = 0;

  for (const section of gantt.sections) {
    for (const task of section.tasks) {
      if (elementIdx >= rectPositions.length) break;
      
      const rp = rectPositions[elementIdx];
      if (task.alias) {
        aliasToPosMap.set(task.alias, {
          rightX: rp.isMilestone ? rp.cx : rp.x + rp.w,
          leftX: rp.isMilestone ? rp.cx : rp.x,
          y: rp.centerY,
        });
      }
      elementIdx++;
    }
  }

  // ── Setup SVG defs / arrowhead marker ─────────────────────────────────────
  const ns = 'http://www.w3.org/2000/svg';
  let defs = svgEl.querySelector('defs');
  if (!defs) { defs = document.createElementNS(ns, 'defs'); svgEl.insertBefore(defs, svgEl.firstChild); }
  if (!defs.querySelector('#dep-arrow-head')) {
    const marker = document.createElementNS(ns, 'marker');
    marker.setAttribute('id', 'dep-arrow-head');
    marker.setAttribute('markerWidth', '8'); marker.setAttribute('markerHeight', '6');
    marker.setAttribute('refX', '7'); marker.setAttribute('refY', '3');
    marker.setAttribute('orient', 'auto');
    const poly = document.createElementNS(ns, 'polygon');
    poly.setAttribute('points', '0 0, 8 3, 0 6'); poly.setAttribute('fill', '#f59e0b');
    marker.appendChild(poly); defs.appendChild(marker);
  }

  // ── Draw arrows ───────────────────────────────────────────────────────────
  for (const section of gantt.sections) {
    for (const task of section.tasks) {
      if (!task.dependsOn || !task.alias) continue;
      const src = aliasToPosMap.get(task.dependsOn);
      const dst = aliasToPosMap.get(task.alias);
      if (!src || !dst) { console.warn('[dep arrow] pos not found:', task.dependsOn, '->', task.alias); continue; }

      const sx = src.rightX;
      const sy = src.y;
      const dx = dst.leftX;
      const dy = dst.y;

      const spread = Math.max(30, Math.abs(dx - sx) * 0.35);
      const path = document.createElementNS(ns, 'path');
      path.setAttribute('d', `M ${sx} ${sy} C ${sx + spread} ${sy}, ${dx - spread} ${dy}, ${dx} ${dy}`);
      path.setAttribute('stroke', '#f59e0b');
      path.setAttribute('stroke-width', '1.8');
      path.setAttribute('fill', 'none');
      path.setAttribute('stroke-dasharray', '6,3');
      path.setAttribute('marker-end', 'url(#dep-arrow-head)');
      path.setAttribute('opacity', '0.85');
      svgEl.appendChild(path);
    }
  }
}
</script>

<style scoped>
.schedule-view {
  display: flex;
  flex-direction: column;
  height: 100%;
  background: #0f172a;
}

.schedule-toolbar {
  background: #1e293b !important;
  border-bottom: 1px solid #334155;
  flex-shrink: 0;
}

.chart-wrapper {
  flex: 1;
  overflow: auto;
  padding: 16px;
}

.mermaid-output {
  min-width: 800px;
}

.mermaid-output :deep(svg) {
  background: #0f172a !important;
  border-radius: 8px;
}

/* Alternating section backgrounds — more contrasting */
.mermaid-output :deep(.section0) { fill: #0a1628 !important; }
.mermaid-output :deep(.section1) { fill: #1a2744 !important; }
.mermaid-output :deep(.section2) { fill: #0a1628 !important; }
.mermaid-output :deep(.section3) { fill: #1a2744 !important; }

/* Section title text */
.mermaid-output :deep(.sectionTitle) {
  fill: #94a3b8 !important;
  font-weight: 700 !important;
  font-size: 14px !important;
}

.mermaid-output :deep(.today) {
  stroke: #ef4444 !important;
  stroke-width: 3px !important;
}

.render-error {
  background: #1e293b;
  color: #ef4444;
  padding: 16px;
  border-radius: 8px;
  font-size: 0.8rem;
  white-space: pre-wrap;
}
</style>
