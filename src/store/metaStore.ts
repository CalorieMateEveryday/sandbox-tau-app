import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";

export interface GitlabLabel {
  id: number;
  name: string;
  color: string;
}

export interface GitlabMilestone {
  id: number;
  title: string;
  due_date: string | null;
}

export interface TargetsMeta {
  labels: GitlabLabel[];
  milestones: GitlabMilestone[];
}

export const useMetaStore = defineStore("meta", {
  state: () => ({
    labels: [] as GitlabLabel[],
    milestones: [] as GitlabMilestone[],
    loading: false,
  }),
  actions: {
    async fetchMeta() {
      this.loading = true;
      try {
        const meta: TargetsMeta = await invoke("fetch_targets_meta");
        this.labels = meta.labels;
        this.milestones = meta.milestones;
      } catch (e) {
        console.error("Failed to fetch meta:", e);
      } finally {
        this.loading = false;
      }
    },
  },
});
