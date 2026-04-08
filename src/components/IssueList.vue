<template>
  <div>
    <v-data-table
      v-model:search="search"
      :headers="headers"
      :items="treeIssues"
      :loading="issueStore.loading"
      density="compact"
      item-value="id"
      show-expand
    >
      <template v-slot:item="{ item }">
        <tr @contextmenu.prevent="onContextMenu($event, item)">
          <td>{{ item.iid }}</td>
          <td>
            <a :href="item.web_url" target="_blank" class="text-decoration-none text-primary">
              {{ item.title }}
            </a>
          </td>
          <td>
            <v-chip
              v-for="label in item.labels"
              :key="label"
              size="x-small"
              class="mr-1"
              label
            >
              {{ label }}
            </v-chip>
          </td>
          <td>{{ issueStore.getDays(item.time_stats.time_estimate) }}d</td>
          <td>{{ issueStore.getDays(item.time_stats.total_time_spent) }}d</td>
          <td>{{ item.milestone?.title }}</td>
          <td>{{ item.assignees.map(a => a.name).join(', ') }}</td>
        </tr>
      </template>
      <template v-slot:expanded-row="{ columns, item }">
        <tr>
          <td :colspan="columns.length">
            <v-table density="compact" class="bg-grey-darken-4">
              <tbody>
                <tr v-for="child in item.children" :key="child.id">
                  <td width="80px">#{{ child.iid }}</td>
                  <td>{{ child.title }}</td>
                  <td>{{ child.labels.join(', ') }}</td>
                  <td>{{ child.assignees.map(a => a.name).join(', ') }}</td>
                </tr>
              </tbody>
            </v-table>
          </td>
        </tr>
      </template>
    </v-data-table>

    <AddChildDialog 
      v-model="childDialog.show" 
      :parent="childDialog.item" 
      @created="issueStore.fetchData"
    />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, reactive } from "vue";
import { useIssueStore } from "../store/issueStore";
import AddChildDialog from "./AddChildDialog.vue";

const issueStore = useIssueStore();
const search = ref("");
const childDialog = reactive({ show: false, item: null as any });

const onContextMenu = (e: MouseEvent, item: any) => {
  childDialog.item = item;
  childDialog.show = true;
};

const treeIssues = computed(() => {
  const all = issueStore.filteredIssues;
  const roots = all.filter(i => !i.parent_iid);
  const children = all.filter(i => i.parent_iid);

  return roots.map(root => {
    return {
      ...root,
      children: children.filter(c => c.parent_iid === root.iid)
    };
  });
});

const headers = [
  { title: "IID", key: "iid", width: "80px" },
  { title: "Title", key: "title" },
  { title: "Labels", key: "labels" },
  { title: "Estimate (d)", key: "time_stats.time_estimate" },
  { title: "Spent (d)", key: "time_stats.total_time_spent" },
  { title: "Milestone", key: "milestone.title" },
  { title: "Assignee", key: "assignees" },
];
</script>
