<template>
  <v-card class="pa-4">
    <v-card-title>Application Settings</v-card-title>
    <v-card-text>
      <v-form v-if="config" @submit.prevent="saveConfig">
        <v-alert type="info" class="mb-4">
          Settings are saved to your application's local config directory.
        </v-alert>

        <v-expansion-panels>
          <v-expansion-panel title="GitLab Connection">
            <v-expansion-panel-text>
              <v-text-field label="GitLab URL" v-model="config.gitlab.url"></v-text-field>
              <v-text-field label="Personal Access Token" v-model="config.gitlab.token" type="password"></v-text-field>
            </v-expansion-panel-text>
          </v-expansion-panel>

          <v-expansion-panel title="Tracking Targets">
            <v-expansion-panel-text>
              <v-text-field label="Group IDs (comma separated)" v-model="groupsText"></v-text-field>
              <v-text-field label="Project IDs (comma separated)" v-model="projectsText"></v-text-field>
            </v-expansion-panel-text>
          </v-expansion-panel>

          <v-expansion-panel title="Paths & Calculations">
            <v-expansion-panel-text>
              <v-text-field label="Network Path (exclude_days.json)" v-model="config.network_settings_path"></v-text-field>
              <v-text-field label="Average Capacity Window (Months)" v-model.number="config.average_capacity_months" type="number"></v-text-field>
              <v-text-field label="Hours per Day" v-model.number="config.hours_per_day" type="number"></v-text-field>
            </v-expansion-panel-text>
          </v-expansion-panel>

          <v-expansion-panel title="Board Settings">
            <v-expansion-panel-text>
              <v-text-field label="Excluded Child Labels (comma separated)" v-model="excludedLabelsText"></v-text-field>
            </v-expansion-panel-text>
          </v-expansion-panel>
        </v-expansion-panels>

        <v-btn color="primary" class="mt-4" type="submit">Save Settings</v-btn>
      </v-form>
      <div v-else>Loading configuration...</div>
    </v-card-text>
  </v-card>
</template>

<script setup lang="ts">
import { ref, watch, onMounted } from "vue";
import { useIssueStore } from "../store/issueStore";
import { invoke } from "@tauri-apps/api/core";

const issueStore = useIssueStore();
const config = ref<any>(null);
const groupsText = ref("");
const projectsText = ref("");
const excludedLabelsText = ref("");

onMounted(() => {
  if (issueStore.config) {
    config.value = JSON.parse(JSON.stringify(issueStore.config));
    groupsText.value = config.value.targets.groups.join(", ");
    projectsText.value = config.value.targets.projects.join(", ");
    excludedLabelsText.value = config.value.board.excluded_child_labels.join(", ");
  }
});

const saveConfig = async () => {
  config.value.targets.groups = groupsText.value.split(",").map(s => s.trim()).filter(s => s !== "");
  config.value.targets.projects = projectsText.value.split(",").map(s => s.trim()).filter(s => s !== "");
  config.value.board.excluded_child_labels = excludedLabelsText.value.split(",").map(s => s.trim()).filter(s => s !== "");
  
  try {
    await invoke("save_config", { config: config.value });
    issueStore.config = config.value;
    alert("Settings saved successfully!");
  } catch (e) {
    alert("Failed to save settings: " + e);
  }
};
</script>
