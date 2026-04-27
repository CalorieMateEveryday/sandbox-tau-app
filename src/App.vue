<template>
  <v-app>
    <v-navigation-drawer v-model="drawer" permanent>
      <v-list density="compact" nav>
        <v-list-item prepend-icon="mdi-format-list-bulleted" title="Issue List" value="list" @click="currentView = 'list'"></v-list-item>
        <v-list-item prepend-icon="mdi-view-column" title="Issue Board" value="board" @click="currentView = 'board'"></v-list-item>
        <v-list-item prepend-icon="mdi-chart-line" title="Burndown" value="burndown" @click="currentView = 'burndown'"></v-list-item>
        <v-list-item prepend-icon="mdi-trending-up" title="Performance" value="performance" @click="currentView = 'performance'"></v-list-item>
        <v-list-item prepend-icon="mdi-plus-box-multiple" title="Bulk Creation" value="bulk" @click="currentView = 'bulk'"></v-list-item>
        <v-divider class="my-2"></v-divider>
        <v-list-item prepend-icon="mdi-cog" title="Settings" value="settings" @click="currentView = 'settings'"></v-list-item>
      </v-list>
    </v-navigation-drawer>

    <v-app-bar flat>
      <v-app-bar-title>GIssue Tracker</v-app-bar-title>
      
      <v-spacer></v-spacer>

      <v-select
        class="mx-1 mt-6"
        v-model="issueStore.filters.assignees"
        :items="issueStore.uniqueAssignees"
        item-title="name"
        item-value="username"
        label="Assignee"
        multiple
        chips
        clearable
        density="compact"
        max-width="200"
      ></v-select>

      <v-select
        class="mx-1 mt-6"
        v-model="issueStore.filters.products"
        :items="issueStore.uniqueProducts"
        label="Product"
        multiple
        chips
        clearable
        density="compact"
        max-width="200"
      ></v-select>

      <v-select
        class="mx-1 mt-6"
        v-model="issueStore.filters.milestones"
        :items="metaStore.milestones.map(m => m.title)"
        label="Milestone"
        multiple
        chips
        clearable
        density="compact"
        max-width="200"
      ></v-select>

      <v-btn icon @click="refreshData">
        <v-icon>mdi-refresh</v-icon>
      </v-btn>
    </v-app-bar>

    <v-main>
      <v-container fluid>
        <div v-if="loading" class="d-flex align-center justify-center fill-height">
          <v-progress-circular indeterminate size="64"></v-progress-circular>
        </div>
        <div v-else>
          <!-- Dynamic Components for different views -->
          <component :is="currentViewComponent" />
        </div>
      </v-container>
    </v-main>

    <!-- Toast Notifications -->
    <v-snackbar v-model="snackbar.show" :color="snackbar.color" :timeout="3000">
      {{ snackbar.text }}
      <template v-slot:actions>
        <v-btn variant="text" @click="snackbar.show = false">Close</v-btn>
      </template>
    </v-snackbar>
  </v-app>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, reactive } from "vue";
import { useIssueStore } from "./store/issueStore";
import { useMetaStore } from "./store/metaStore";
import { listen } from "@tauri-apps/api/event";
import IssueList from "./components/IssueList.vue";
import IssueBoard from "./components/IssueBoard.vue";
import BurndownChart from "./components/BurndownChart.vue";
import PerformanceStats from "./components/PerformanceStats.vue";
import BulkCreation from "./components/BulkCreation.vue";
import AppSettings from "./components/AppSettings.vue";

const issueStore = useIssueStore();
const metaStore = useMetaStore();
const drawer = ref(true);
const currentView = ref("list");
const loading = computed(() => issueStore.loading);

const snackbar = reactive({
  show: false,
  text: "",
  color: "info",
});

const refreshData = async () => {
  await issueStore.fetchData();
};

onMounted(async () => {
  await issueStore.init();
  await metaStore.fetchMeta();

  // Listen for backend notifications
  await listen("app-notification", (event: any) => {
    snackbar.text = event.payload.message;
    snackbar.color = event.payload.level === "warning" ? "orange" : "info";
    snackbar.show = true;
  });
});

const currentViewComponent = computed(() => {
  switch (currentView.value) {
    case "list": return IssueList;
    case "board": return IssueBoard;
    case "burndown": return BurndownChart;
    case "performance": return PerformanceStats;
    case "bulk": return BulkCreation;
    case "settings": return AppSettings;
    default: return IssueList;
  }
});
</script>

<style>
/* Global styles */
body {
  overflow: hidden;
}
</style>