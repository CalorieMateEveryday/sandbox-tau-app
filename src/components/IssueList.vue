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
        <tr @contextmenu.prevent="onContextMenu($event, item)" @click="showDescription(item)" style="cursor: pointer;">
          <td>{{ item.iid }}</td>
          <td>
            <a :href="item.web_url" target="_blank" class="text-decoration-none text-primary" @click.stop>
              {{ item.title }}
            </a>
          </td>
          <td>
            <v-chip
              v-for="label in item.labels"
              :key="label"
              size="x-small"
              class="mr-1"
              :color="getLabelColor(label)"
              label
            >
              {{ label }}
            </v-chip>
          </td>
          <td>{{ issueStore.getDays(item.time_stats.time_estimate) }}d</td>
          <td>{{ issueStore.getDays(item.time_stats.total_time_spent) }}d</td>
          <td>{{ item.milestone?.title }}</td>
          <td>{{ getDisplayAssignees(item) }}</td>
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

    <v-dialog v-model="descDialog.show" max-width="800">
      <v-card>
        <v-card-title>Description: {{ descDialog.item?.title }}</v-card-title>
        <v-card-text>
          <v-textarea
            v-model="descDialog.description"
            rows="10"
            variant="outlined"
            auto-grow
          ></v-textarea>
        </v-card-text>
        <v-card-actions>
          <v-spacer></v-spacer>
          <v-btn text @click="descDialog.show = false">Cancel</v-btn>
          <v-btn color="primary" @click="saveDescription">Save</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, reactive } from "vue";
import { useIssueStore } from "../store/issueStore";
import { useMetaStore } from "../store/metaStore";
import AddChildDialog from "./AddChildDialog.vue";
import { invoke } from "@tauri-apps/api/core";

const issueStore = useIssueStore();
const metaStore = useMetaStore();
const search = ref("");
const childDialog = reactive({ show: false, item: null as any });
const descDialog = reactive({ show: false, item: null as any, description: "" });

const onContextMenu = (_e: MouseEvent, item: any) => {
  childDialog.item = item;
  childDialog.show = true;
};

const showDescription = (item: any) => {
  descDialog.item = item;
  descDialog.description = item.description || "";
  descDialog.show = true;
};

const saveDescription = async () => {
  if (descDialog.item) {
    try {
      await invoke("update_work_item_description", {
        workItemId: descDialog.item.id,
        description: descDialog.description
      });
      descDialog.item.description = descDialog.description;
      descDialog.show = false;
    } catch (e) {
      console.error("Failed to update description", e);
    }
  }
};

const getLabelColor = (labelName: string) => {
  const label = metaStore.labels.find(l => l.name === labelName);
  return label ? label.color : undefined;
};

const getDisplayAssignees = (item: any) => {
  if (!issueStore.config) return "";
  const validUsers = issueStore.config.users.map(u => u.username);
  const assignees = item.assignees.filter((a: any) => validUsers.includes(a.username));
  if (assignees.length === 0) return "Unassigned";
  return assignees.map((a: any) => a.name).join(', ');
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
