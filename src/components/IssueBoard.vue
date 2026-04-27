<template>
  <div class="d-flex flex-row overflow-x-auto fill-height pa-4">
    <div
      v-for="column in issueStore.config?.board.columns || []"
      :key="column"
      class="board-column mx-2"
    >
      <v-card variant="outlined" class="fill-height" width="300px">
        <v-toolbar color="grey-darken-3" density="compact">
          <v-toolbar-title class="text-subtitle-1">{{ column }}</v-toolbar-title>
        </v-toolbar>
        <v-list class="pa-2 bg-transparent">
          <v-card
            v-for="issue in getItemsInColumn(column)"
            :key="issue.id"
            class="mb-2 pa-2"
            elevation="2"
            :style="{ borderLeft: `4px solid ${getProductColor(issue.product)}` }"
            @contextmenu.prevent="onContextMenu($event, issue)"
          >
            <div class="text-caption d-flex justify-space-between align-center">
              <span>#{{ issue.iid }}</span>
              <v-chip v-if="issue.product" size="x-small" label :color="getProductColor(issue.product)" variant="tonal">
                {{ issue.product }}
              </v-chip>
            </div>
            <div class="text-body-2 font-weight-bold">{{ issue.title }}</div>
            <v-chip-group>
              <v-chip
                v-for="label in issue.labels.filter(l => l !== column)"
                :key="label"
                size="x-small"
                variant="outlined"
                :color="getLabelColor(label)"
              >
                {{ label }}
              </v-chip>
            </v-chip-group>
            
            <div class="mt-2 d-flex justify-space-between text-caption text-grey">
              <span>Est: {{ issueStore.getDays(issue.time_stats.time_estimate) }}d</span>
              <span>Spent: {{ issueStore.getDays(issue.time_stats.total_time_spent) }}d</span>
            </div>
          </v-card>
        </v-list>
      </v-card>
    </div>

    <AddChildDialog 
      v-model="childDialog.show" 
      :parent="childDialog.item" 
      @created="issueStore.fetchData"
    />
  </div>
</template>

<script setup lang="ts">
import { reactive } from "vue";
import { useIssueStore } from "../store/issueStore";
import { useMetaStore } from "../store/metaStore";
import AddChildDialog from "./AddChildDialog.vue";

const issueStore = useIssueStore();
const metaStore = useMetaStore();
const childDialog = reactive({ show: false, item: null as any });

const onContextMenu = (_e: MouseEvent, item: any) => {
  childDialog.item = item;
  childDialog.show = true;
};

const getItemsInColumn = (column: string) => {
  return issueStore.filteredIssues.filter(issue => issue.labels.includes(column));
};

const getProductColor = (productName?: string) => {
  if (!productName) return "#666";
  const colors = ["#42A5F5", "#66BB6A", "#FFA726", "#AB47BC", "#26A69A", "#EC407A"];
  let hash = 0;
  for (let i = 0; i < productName.length; i++) {
    hash = productName.charCodeAt(i) + ((hash << 5) - hash);
  }
  return colors[Math.abs(hash) % colors.length];
};

const getLabelColor = (labelName: string) => {
  const label = metaStore.labels.find(l => l.name === labelName);
  return label ? label.color : undefined;
};
</script>

<style scoped>
.board-column {
  min-width: 300px;
  height: calc(100vh - 150px);
}
</style>
