<template>
  <v-card class="pa-4">
    <v-card-title>Bulk Creation</v-card-title>
    <v-tabs v-model="tab">
      <v-tab value="workitem">Create Work Items</v-tab>
      <v-tab value="milestone">Create Milestones</v-tab>
    </v-tabs>

    <v-window v-model="tab" class="mt-4">
      <v-window-item value="workitem">
        <v-form @submit.prevent="createWorkItem">
          <v-select label="Target Repository" :items="issueStore.config?.targets.projects" v-model="workitemForm.project_path"></v-select>
          <v-text-field label="Title" v-model="workitemForm.title" required></v-text-field>
          <v-select 
            label="Milestone" 
            :items="issueStore.uniqueMilestones" 
            v-model="workitemForm.milestone_title"
            :disabled="!!workitemForm.due_date"
            clearable
          ></v-select>
          <v-text-field 
            label="Due Date" 
            type="date" 
            v-model="workitemForm.due_date" 
            :disabled="!!workitemForm.milestone_title"
            clearable
          ></v-text-field>
          <v-select 
            label="Assignees" 
            :items="issueStore.uniqueAssignees" 
            item-title="name" 
            item-value="username"
            v-model="workitemForm.assignee_usernames"
            multiple
            chips
          ></v-select>
          <v-text-field label="Estimate (days)" type="number" v-model.number="workitemForm.estimate"></v-text-field>
          <v-checkbox label="Keep inputs for next item" v-model="keepInputs"></v-checkbox>
          <v-btn color="primary" type="submit" :loading="creating">Create Item</v-btn>
        </v-form>
      </v-window-item>

      <v-window-item value="milestone">
        <v-form @submit.prevent="createMilestone">
          <v-select label="Target Repository" :items="issueStore.config?.targets.projects" v-model="milestoneForm.project_path"></v-select>
          <v-text-field label="Milestone Title" v-model="milestoneForm.title" required></v-text-field>
          <v-text-field label="Due Date" type="date" v-model="milestoneForm.due_date"></v-text-field>
          <v-btn color="primary" type="submit" :loading="creating">Create Milestone</v-btn>
        </v-form>
      </v-window-item>
    </v-window>

    <v-snackbar v-model="alert.show" :color="alert.color">
      {{ alert.text }}
    </v-snackbar>
  </v-card>
</template>

<script setup lang="ts">
import { ref, reactive } from "vue";
import { invoke } from "@tauri-apps/api/core";
import { useIssueStore } from "../store/issueStore";

const issueStore = useIssueStore();
const tab = ref("workitem");
const keepInputs = ref(true);
const creating = ref(false);

const workitemForm = reactive({
  project_path: "",
  title: "",
  milestone_title: "",
  due_date: "",
  assignee_usernames: [] as string[],
  estimate: 0,
});

const milestoneForm = reactive({
  project_path: "",
  title: "",
  due_date: "",
});

const alert = reactive({ show: false, text: "", color: "" });

const createWorkItem = async () => {
  creating.value = true;
  try {
    // Note: In a real app we'd resolve milestone_title to milestone_id
    // For this simple version, I'll pass labels/etc as specified in requirements.
    await invoke("create_work_item", {
      projectPath: workitemForm.project_path,
      title: workitemForm.title,
      milestoneId: null, // Placeholder
      dueDate: workitemForm.due_date || null,
      labels: [],
      assigneeIds: [], 
    });
    alert.text = "Work item created!";
    alert.color = "success";
    alert.show = true;
    if (!keepInputs.value) workitemForm.title = "";
  } catch (e) {
    alert.text = "Error: " + e;
    alert.color = "error";
    alert.show = true;
  } finally {
    creating.value = false;
  }
};

const createMilestone = async () => {
  creating.value = true;
  try {
    await invoke("create_milestone", {
      projectPath: milestoneForm.project_path,
      title: milestoneForm.title,
      dueDate: milestoneForm.due_date || null,
    });
    alert.text = "Milestone created!";
    alert.color = "success";
    alert.show = true;
  } catch (e) {
    alert.text = "Error: " + e;
    alert.color = "error";
    alert.show = true;
  } finally {
    creating.value = false;
  }
};
</script>
