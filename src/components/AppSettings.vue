<template>
  <v-card class="pa-4">
    <v-card-title>Application Settings</v-card-title>
    <v-card-text>
      <v-form v-if="config" @submit.prevent="saveAll">
        <v-alert v-if="localConfig?.shared_config_path" type="success" variant="tonal" class="mb-4">
          Using shared settings from: {{ localConfig.shared_config_path }}
        </v-alert>
        <v-alert v-else type="info" variant="tonal" class="mb-4">
          Settings are saved to your application's local config directory.
        </v-alert>

        <v-expansion-panels v-model="panels">
          <v-expansion-panel title="Local Redirection (Team Sharing)" value="local">
            <v-expansion-panel-text>
              <div class="text-subtitle-2 mb-2 text-grey">
                設定をチームで共有する場合、ネットワークフォルダ上のJSONファイルのパスを指定してください。
              </div>
              <v-text-field 
                label="Shared Config Path (e.g. \\server\share\config.json)" 
                v-model="localConfig.shared_config_path"
                placeholder="Leave empty for local storage"
                persistent-hint
                hint="ここにパスを設定すると、GitLabトークンやターゲット設定を含むすべての設定がこのファイルに保存・読み込みされるようになります。"
              ></v-text-field>
              <v-btn color="secondary" size="small" variant="outlined" @click="saveLocalOnly" :loading="savingLocal">
                Update Path Only
              </v-btn>
            </v-expansion-panel-text>
          </v-expansion-panel>

          <v-expansion-panel title="GitLab Connection" value="gitlab">
            <v-expansion-panel-text>
              <v-text-field label="GitLab URL" v-model="config.gitlab.url"></v-text-field>
              <v-text-field label="Personal Access Token" v-model="config.gitlab.token" type="password"></v-text-field>
            </v-expansion-panel-text>
          </v-expansion-panel>

          <v-expansion-panel title="Tracking Targets" value="targets">
            <v-expansion-panel-text>
              <v-text-field label="Group IDs (comma separated)" v-model="groupsText"></v-text-field>
              <v-text-field label="Project IDs (comma separated)" v-model="projectsText"></v-text-field>
            </v-expansion-panel-text>
          </v-expansion-panel>

          <v-expansion-panel title="Paths & Calculations" value="paths">
            <v-expansion-panel-text>
              <v-text-field label="Network Path (exclude_days.json)" v-model="config.network_settings_path"></v-text-field>
              <v-text-field label="Average Capacity Window (Months)" v-model.number="config.average_capacity_months" type="number"></v-text-field>
              <v-text-field label="Hours per Day" v-model.number="config.hours_per_day" type="number"></v-text-field>
            </v-expansion-panel-text>
          </v-expansion-panel>

          <v-expansion-panel title="Board Settings" value="board">
            <v-expansion-panel-text>
              <v-text-field label="Excluded Child Labels (comma separated)" v-model="excludedLabelsText"></v-text-field>
            </v-expansion-panel-text>
          </v-expansion-panel>
        </v-expansion-panels>

        <v-btn color="primary" class="mt-4" type="submit" :loading="savingAll">Save All Settings</v-btn>
      </v-form>
      <div v-else>Loading configuration...</div>
    </v-card-text>
  </v-card>
</template>

<script setup lang="ts">
import { ref, onMounted } from "vue";
import { useIssueStore } from "../store/issueStore";
import { invoke } from "@tauri-apps/api/core";

const issueStore = useIssueStore();
const config = ref<any>(null);
const localConfig = ref<any>({ shared_config_path: "" });
const groupsText = ref("");
const projectsText = ref("");
const excludedLabelsText = ref("");
const panels = ref<string[]>([]);

const savingLocal = ref(false);
const savingAll = ref(false);

onMounted(async () => {
  try {
    localConfig.value = await invoke("get_local_config");
    if (issueStore.config) {
      config.value = JSON.parse(JSON.stringify(issueStore.config));
      groupsText.value = config.value.targets.groups.join(", ");
      projectsText.value = config.value.targets.projects.join(", ");
      excludedLabelsText.value = config.value.board.excluded_child_labels.join(", ");
    }
  } catch (e) {
    console.error(e);
  }
});

const saveLocalOnly = async () => {
  savingLocal.value = true;
  try {
    await invoke("save_local_config", { config: localConfig.value });
    alert("Local redirection path updated. The app will now reload settings.");
    await issueStore.init();
    config.value = JSON.parse(JSON.stringify(issueStore.config));
  } catch (e) {
    alert("Failed to update local config: " + e);
  } finally {
    savingLocal.value = false;
  }
};

const saveAll = async () => {
  savingAll.value = true;
  config.value.targets.groups = groupsText.value.split(",").map(s => s.trim()).filter(s => s !== "");
  config.value.targets.projects = projectsText.value.split(",").map(s => s.trim()).filter(s => s !== "");
  config.value.board.excluded_child_labels = excludedLabelsText.value.split(",").map(s => s.trim()).filter(s => s !== "");
  
  try {
    // Save local config first if path was changed
    await invoke("save_local_config", { config: localConfig.value });
    
    // Save the main config (to either local or shared path)
    await invoke("save_config", { config: config.value });
    
    issueStore.config = config.value;
    alert("All settings saved successfully!");
  } catch (e) {
    alert("Failed to save settings: " + e);
  } finally {
    savingAll.value = false;
  }
};
</script>
