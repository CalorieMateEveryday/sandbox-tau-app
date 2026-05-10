import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";

// ─── External schedule config types ───────────────────────────────────────────
export interface ScheduleEntry {
  product_id: string;
  file: string;
}

export interface ScheduleConfig {
  schedules: ScheduleEntry[];
}

// ─── Parsed gantt data types ───────────────────────────────────────────────────
export type TaskModifier = "done" | "active" | "crit" | "milestone";

export interface GanttTask {
  id: string;
  name: string;
  alias: string;
  modifiers: TaskModifier[];
  start: string;    // YYYY-MM-DD (always a date)
  dependsOn?: string; // alias of a predecessor task (stored separately from start)
  duration: string; // e.g. "7d", "2w"
}

export interface GanttSection {
  id: string;
  name: string;
  tasks: GanttTask[];
}

export interface ParsedGantt {
  title: string;
  excludes?: string;
  sections: GanttSection[];
}

// ─── Store ─────────────────────────────────────────────────────────────────────
let _taskCounter = 0;
let _sectionCounter = 0;

export const useScheduleStore = defineStore("schedule", {
  state: () => ({
    scheduleConfig: null as ScheduleConfig | null,
    selectedProductId: null as string | null,
    parsedGantt: null as ParsedGantt | null,
    loading: false,
    fileSaving: false,
    showMilestonesMap: {} as Record<string, boolean>,
    errorMessage: "" as string,
  }),

  getters: {
    showMilestones(state): boolean {
      if (!state.selectedProductId) return true;
      return state.showMilestonesMap[state.selectedProductId] ?? true;
    },
    currentEntry(state): ScheduleEntry | null {
      if (!state.scheduleConfig || !state.selectedProductId) return null;
      return (
        state.scheduleConfig.schedules.find(
          (s) => s.product_id === state.selectedProductId
        ) ?? null
      );
    },
    availableProductIds(state): string[] {
      return state.scheduleConfig?.schedules.map((s) => s.product_id) ?? [];
    },
  },

  actions: {
    async loadScheduleConfig() {
      this.loading = true;
      this.errorMessage = "";
      try {
        this.scheduleConfig = await invoke<ScheduleConfig>("get_schedule_config");
        if (
          this.scheduleConfig.schedules.length > 0 &&
          (!this.selectedProductId ||
            !this.scheduleConfig.schedules.find(
              (s) => s.product_id === this.selectedProductId
            ))
        ) {
          this.selectedProductId = this.scheduleConfig.schedules[0].product_id;
        }
        if (this.selectedProductId) {
          await this.loadScheduleFile(this.selectedProductId);
        }
      } catch (e) {
        this.errorMessage = `スケジュール設定の読み込みに失敗しました: ${e}`;
        console.error(e);
      } finally {
        this.loading = false;
      }
    },

    async loadScheduleFile(productId: string) {
      const entry = this.scheduleConfig?.schedules.find(
        (s) => s.product_id === productId
      );
      if (!entry) return;
      try {
        const content = await invoke<string>("read_schedule_file", {
          path: entry.file,
        });
        this.parsedGantt = parseGantt(content);
      } catch (e) {
        // File doesn't exist yet → create empty schedule
        this.parsedGantt = {
          title: productId,
          sections: [],
        };
      }
    },

    async selectProduct(productId: string) {
      this.selectedProductId = productId;
      await this.loadScheduleFile(productId);
    },

    async saveScheduleFile() {
      const entry = this.currentEntry;
      if (!entry || !this.parsedGantt) return;
      this.fileSaving = true;
      try {
        const content = serializeGantt(this.parsedGantt);
        await invoke("save_schedule_file", {
          path: entry.file,
          content,
        });
      } catch (e) {
        this.errorMessage = `保存に失敗しました: ${e}`;
        console.error(e);
      } finally {
        this.fileSaving = false;
      }
    },

    toggleMilestones() {
      if (!this.selectedProductId) return;
      const current = this.showMilestonesMap[this.selectedProductId] ?? true;
      this.showMilestonesMap[this.selectedProductId] = !current;
    },

    async addScheduleEntry(entry: ScheduleEntry) {
      if (!this.scheduleConfig) {
        this.scheduleConfig = { schedules: [] };
      }
      this.scheduleConfig.schedules.push(entry);
      await invoke("save_schedule_config", { config: this.scheduleConfig });
    },

    async removeScheduleEntry(productId: string) {
      if (!this.scheduleConfig) return;
      this.scheduleConfig.schedules = this.scheduleConfig.schedules.filter(
        (s) => s.product_id !== productId
      );
      await invoke("save_schedule_config", { config: this.scheduleConfig });
    },

    // ─── Gantt editing actions ──────────────────────────────────────────────
    addSection(name: string) {
      if (!this.parsedGantt) return;
      _sectionCounter++;
      this.parsedGantt.sections.push({
        id: `sec-${_sectionCounter}`,
        name,
        tasks: [],
      });
    },

    removeSection(sectionId: string) {
      if (!this.parsedGantt) return;
      this.parsedGantt.sections = this.parsedGantt.sections.filter(
        (s) => s.id !== sectionId
      );
    },

    moveSectionUp(sectionId: string) {
      if (!this.parsedGantt) return;
      const idx = this.parsedGantt.sections.findIndex((s) => s.id === sectionId);
      if (idx > 0) {
        const arr = this.parsedGantt.sections;
        [arr[idx - 1], arr[idx]] = [arr[idx], arr[idx - 1]];
      }
    },

    moveSectionDown(sectionId: string) {
      if (!this.parsedGantt) return;
      const arr = this.parsedGantt.sections;
      const idx = arr.findIndex((s) => s.id === sectionId);
      if (idx < arr.length - 1) {
        [arr[idx], arr[idx + 1]] = [arr[idx + 1], arr[idx]];
      }
    },

    addTask(sectionId: string, task: Omit<GanttTask, "id">) {
      if (!this.parsedGantt) return;
      const section = this.parsedGantt.sections.find((s) => s.id === sectionId);
      if (!section) return;
      _taskCounter++;
      section.tasks.push({ ...task, id: `task-${_taskCounter}` });
    },

    updateTask(sectionId: string, taskId: string, task: Omit<GanttTask, "id">) {
      if (!this.parsedGantt) return;
      const section = this.parsedGantt.sections.find((s) => s.id === sectionId);
      if (!section) return;
      const idx = section.tasks.findIndex((t) => t.id === taskId);
      if (idx >= 0) section.tasks[idx] = { ...task, id: taskId };
    },

    removeTask(sectionId: string, taskId: string) {
      if (!this.parsedGantt) return;
      const section = this.parsedGantt.sections.find((s) => s.id === sectionId);
      if (!section) return;
      section.tasks = section.tasks.filter((t) => t.id !== taskId);
    },

    moveTaskUp(sectionId: string, taskId: string) {
      if (!this.parsedGantt) return;
      const section = this.parsedGantt.sections.find((s) => s.id === sectionId);
      if (!section) return;
      const idx = section.tasks.findIndex((t) => t.id === taskId);
      if (idx > 0) {
        [section.tasks[idx - 1], section.tasks[idx]] = [
          section.tasks[idx],
          section.tasks[idx - 1],
        ];
      }
    },

    moveTaskDown(sectionId: string, taskId: string) {
      if (!this.parsedGantt) return;
      const section = this.parsedGantt.sections.find((s) => s.id === sectionId);
      if (!section) return;
      const arr = section.tasks;
      const idx = arr.findIndex((t) => t.id === taskId);
      if (idx < arr.length - 1) {
        [arr[idx], arr[idx + 1]] = [arr[idx + 1], arr[idx]];
      }
    },
  },
});

// ─── Gantt Parser ──────────────────────────────────────────────────────────────
const KNOWN_MODIFIERS: TaskModifier[] = ["done", "active", "crit", "milestone"];

export function parseGantt(text: string): ParsedGantt {
  const lines = text.split(/\r?\n/);
  let title = "";
  let excludes = "";
  let currentSection: GanttSection | null = null;
  const sections: GanttSection[] = [];
  let taskId = 0;
  let secId = 0;

  for (const line of lines) {
    const trimmed = line.trim();
    if (!trimmed || trimmed.startsWith("%%") || trimmed === "gantt") continue;
    if (trimmed.startsWith("title ")) {
      title = trimmed.slice(6).trim();
    } else if (trimmed.startsWith("excludes ")) {
      excludes = trimmed.slice(9).trim();
    } else if (
      trimmed.startsWith("dateFormat ") ||
      trimmed.startsWith("axisFormat ") ||
      trimmed.startsWith("tickInterval ") ||
      trimmed.startsWith("weekday ") ||
      trimmed.startsWith("todayMarker")
    ) {
      // skip meta directives
    } else if (trimmed.startsWith("section ")) {
      secId++;
      currentSection = {
        id: `sec-${secId}`,
        name: trimmed.slice(8).trim(),
        tasks: [],
      };
      sections.push(currentSection);
    } else if (currentSection && trimmed.includes(":")) {
      // Strip inline mermaid comment (%% ...) before parsing
      const commentMatch = trimmed.match(/^(.*?)\s*%%\s*after:(\S+)\s*$/);
      const inlineDepends = commentMatch ? commentMatch[2] : undefined;
      const lineWithoutComment = commentMatch ? commentMatch[1] : trimmed;

      const colonIdx = lineWithoutComment.indexOf(":");
      const name = lineWithoutComment.slice(0, colonIdx).trim();
      const rest = lineWithoutComment.slice(colonIdx + 1).trim();
      // Split by comma, but keep "after <alias>" as single token
      const parts = rest.split(",").map((p) => p.trim());

      const modifiers: TaskModifier[] = [];
      let i = 0;
      while (i < parts.length && KNOWN_MODIFIERS.includes(parts[i] as TaskModifier)) {
        modifiers.push(parts[i] as TaskModifier);
        i++;
      }
      const remaining = parts.slice(i);

      let alias = "";
      let start = "";
      let dependsOn = "";
      let duration = "";

      if (remaining.length >= 3) {
        alias = remaining[0];
        start = remaining.slice(1, remaining.length - 1).join(",").trim();
        duration = remaining[remaining.length - 1];
      } else if (remaining.length === 2) {
        start = remaining[0];
        duration = remaining[1];
      } else if (remaining.length === 1) {
        duration = remaining[0];
      }

      // Migrate legacy "after <alias>" stored in start field
      if (start.startsWith("after ")) {
        dependsOn = start.slice(6).trim();
        start = "";
      }

      taskId++;
      currentSection.tasks.push({
        id: `task-${taskId}`,
        name,
        alias,
        modifiers,
        start,
        dependsOn: inlineDepends || (dependsOn || undefined),
        duration,
      });
    }
  }

  // Update counters to avoid collisions on subsequent adds
  _taskCounter = taskId;
  _sectionCounter = secId;

  return { title, excludes, sections };
}

// ─── Gantt Serializer ─────────────────────────────────────────────────────────
export function serializeGantt(gantt: ParsedGantt): string {
  const lines: string[] = [];
  lines.push("gantt");
  lines.push("    dateFormat YYYY-MM-DD");
  lines.push("    axisFormat %m/%d");
  if (gantt.excludes) lines.push(`    excludes ${gantt.excludes}`);
  if (gantt.title) lines.push(`    title ${gantt.title}`);
  lines.push("");

  for (const section of gantt.sections) {
    lines.push(`    section ${section.name}`);
    for (const task of section.tasks) {
      const parts: string[] = [...task.modifiers];
      if (task.alias) parts.push(task.alias);
      parts.push(task.start || "2099-01-01"); // fallback if date missing
      parts.push(task.duration);
      let line = `    ${task.name}    :${parts.join(", ")}`;
      // Store dependsOn as a comment so it survives round-trips
      if (task.dependsOn) line += `  %% after:${task.dependsOn}`;
      lines.push(line);
    }
    lines.push("");
  }

  return lines.join("\n");
}

// ─── Mermaid text builder (with milestone overlay + viewport anchors) ─────────
export interface MilestoneOverlay {
  title: string;
  date: string; // YYYY-MM-DD
}

/** Returns the earliest start date and latest end date across all tasks */
export function getGanttDateRange(gantt: ParsedGantt): { start: Date; end: Date } {
  const today = new Date();
  // Default range: today-1M to today+3M
  const defaultStart = new Date(today);
  defaultStart.setMonth(defaultStart.getMonth() - 1);
  const defaultEnd = new Date(today);
  defaultEnd.setMonth(defaultEnd.getMonth() + 3);

  let earliest = defaultStart;
  let latest = defaultEnd;

  for (const section of gantt.sections) {
    for (const task of section.tasks) {
      if (!task.start || task.start.startsWith("after")) continue;
      const d = new Date(task.start);
      if (isNaN(d.getTime())) continue;
      if (d < earliest) earliest = d;

      const durDays = parseDurationDays(task.duration);
      const taskEnd = new Date(d.getTime() + durDays * 86400000);
      if (taskEnd > latest) latest = taskEnd;
    }
  }

  return { start: earliest, end: latest };
}

function parseDurationDays(duration: string): number {
  const m = duration.match(/(\d+)([dwmM]?)/);
  if (!m) return 1;
  const n = parseInt(m[1]);
  switch (m[2]) {
    case "w": return n * 7;
    case "m": case "M": return n * 30;
    default: return n;
  }
}


export function buildMermaidText(
  gantt: ParsedGantt,
  milestones: MilestoneOverlay[]
): string {
  const today = new Date();
  const viewStart = new Date(today);
  viewStart.setMonth(viewStart.getMonth() - 1);
  const viewEnd = new Date(today);
  viewEnd.setMonth(viewEnd.getMonth() + 3);

  const initDirective =
    `%%{init: {'useMaxWidth': false, 'theme': 'base', 'themeVariables': {'primaryColor': '#1e293b', 'primaryTextColor': '#e2e8f0', 'lineColor': '#64748b', 'sectionBkgColor': '#0f172a', 'altSectionBkgColor': '#1e293b', 'gridColor': '#334155', 'doneTaskBkgColor': '#065f46', 'activeTaskBkgColor': '#1d4ed8', 'critBorderColor': '#ef4444', 'todayLineColor': '#ef4444', 'excludeBkgColor': '#1e293b'}, 'gantt': {'tickInterval': '1week', 'weekday': 'monday', 'numberSectionStyles': 4}} }%%`;

  const lines: string[] = [];
  lines.push(initDirective);
  lines.push("gantt");
  lines.push("    dateFormat YYYY-MM-DD");
  lines.push("    axisFormat %m/%d");
  if (gantt.excludes) lines.push(`    excludes ${gantt.excludes}`);
  if (gantt.title) lines.push(`    title ${gantt.title}`);
  lines.push("");

  for (const section of gantt.sections) {
    lines.push(`    section ${section.name}`);
    for (const task of section.tasks) {
      const parts: string[] = [...task.modifiers];
      if (task.alias) parts.push(task.alias);
      parts.push(task.start);
      parts.push(task.duration);
      lines.push(`    ${task.name}    :${parts.join(", ")}`);
    }
    lines.push("");
  }

  if (milestones.length > 0) {
    lines.push("    section GitLab Milestones");
    milestones.forEach((m, idx) => {
      const alias = `ms_gl_${idx}`;
      lines.push(`    ${m.title}    :milestone, ${alias}, ${m.date}, 0d`);
    });
    lines.push("");
  }

  return lines.join("\n");
}
