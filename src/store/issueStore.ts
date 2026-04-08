import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";

export interface GitLabWorkItem {
  id: string;
  iid: number;
  title: string;
  description: string;
  state: string;
  project_id: string;
  web_url: string;
  assignees: { id: string; username: string; name: string }[];
  labels: string[];
  milestone?: { id: string; title: string; due_date: string };
  due_date?: string;
  time_stats: {
    time_estimate: number;
    total_time_spent: number;
  };
  parent_iid?: number;
  product?: string;
}

export interface AppConfig {
  gitlab: { url: string; token: string };
  targets: { groups: string[]; projects: string[] };
  products: { id: string; name: string; color: string; match_rules: { labels: string[]; project_ids: string[] } }[];
  users: { username: string; display_name: string }[];
  board: { columns: string[]; excluded_child_labels: string[] };
  network_settings_path: string;
  average_capacity_months: number;
  hours_per_day: number;
}

export const useIssueStore = defineStore("issue", {
  state: () => ({
    config: null as AppConfig | null,
    issues: [] as GitLabWorkItem[],
    capacities: {} as Record<string, number>,
    loading: false,
    filters: {
      assignees: [] as string[],
      products: [] as string[],
      milestones: [] as string[],
      labels: [] as string[],
    },
  }),

  getters: {
    filteredIssues(state) {
      return state.issues.filter((issue) => {
        const matchAssignee = state.filters.assignees.length === 0 || 
          issue.assignees.some(a => state.filters.assignees.includes(a.username));
        const matchProduct = state.filters.products.length === 0 || 
          (issue.product && state.filters.products.includes(issue.product));
        const matchMilestone = state.filters.milestones.length === 0 || 
          (issue.milestone && state.filters.milestones.includes(issue.milestone.title));
        const matchLabel = state.filters.labels.length === 0 || 
          issue.labels.some(l => state.filters.labels.includes(l));

        return matchAssignee && matchProduct && matchMilestone && matchLabel;
      });
    },

    uniqueAssignees(state) {
      const names = new Map<string, string>();
      state.issues.forEach(i => i.assignees.forEach(a => names.set(a.username, a.name)));
      return Array.from(names.entries()).map(([username, name]) => ({ username, name }));
    },

    uniqueProducts(state) {
      return Array.from(new Set(state.issues.map(i => i.product).filter(Boolean)));
    },

    uniqueMilestones(state) {
      return Array.from(new Set(state.issues.map(i => i.milestone?.title).filter(Boolean)));
    },

    uniqueLabels(state) {
      return Array.from(new Set(state.issues.flatMap(i => i.labels)));
    },
    
    getDays: (state) => (seconds: number) => {
        const hours = seconds / 3600;
        const hpd = state.config?.hours_per_day || 8;
        return Number((hours / hpd).toFixed(2));
    },

    formatDayUnit(): (seconds: number) => string {
      return (seconds: number) => `${this.getDays(seconds)}d`;
    }
  },

  actions: {
    async init() {
      this.loading = true;
      try {
        this.config = await invoke("get_config");
        await this.fetchData();
        await this.fetchCapacities();
      } catch (e) {
        console.error("Failed to init store:", e);
      } finally {
        this.loading = false;
      }
    },

    async fetchData() {
      this.loading = true;
      try {
        this.issues = await invoke("fetch_gitlab_data", { force: false });
      } catch (e) {
        console.error("Failed to fetch data:", e);
      } finally {
        this.loading = false;
      }
    },

    async fetchCapacities() {
      try {
        this.capacities = await invoke("calculate_capacities");
      } catch (e) {
        console.error("Failed to fetch capacities:", e);
      }
    },

    setFilter(key: keyof typeof this.filters, values: string[]) {
      this.filters[key] = values;
    },

    async createChildTasks(parent: GitLabWorkItem, children: { title: string, estimate_days: number }[]) {
      if (!this.config) return;

      this.loading = true;
      try {
        // 1. Reset parent estimate to 0
        await invoke("update_work_item_estimate", { 
          workItemId: parent.id, 
          estimateSeconds: 0 
        });

        // 2. Prepare inherited data
        const filteredLabels = parent.labels.filter(
          l => !this.config?.board.excluded_child_labels.includes(l)
        );
        const assigneeIds = parent.assignees.map(a => a.id);
        const milestoneId = parent.milestone?.id || null;
        
        // 3. Create children
        for (const child of children) {
          const estimateSeconds = Math.round(child.estimate_days * (this.config.hours_per_day || 8) * 3600);

          await invoke("create_child_work_item", {
            projectPath: parent.project_id.toString(),
            title: child.title,
            parentId: parent.id,
            milestoneId,
            labels: filteredLabels,
            assigneeIds,
            estimateSeconds
          });
        }

        await this.fetchData();
      } catch (e) {
        console.error("Failed to create child tasks:", e);
        throw e;
      } finally {
        this.loading = false;
      }
    }
  },
});
