<template>
  <v-dialog v-model="show" max-width="600px">
    <v-card v-if="parent">
      <v-card-title>Add Tasks to #{{ parent.iid }}</v-card-title>
      <v-card-text>
        <div class="text-caption mb-4">
          Inherits: 
          <v-chip size="x-small" class="mr-1" color="primary" v-if="parent.milestone">
            Milestone: {{ parent.milestone.title }}
          </v-chip>
          <v-chip 
            v-for="label in inheritedLabels" 
            :key="label" 
            size="x-small" 
            class="mr-1"
          >
            {{ label }}
          </v-chip>
        </div>

        <v-row v-for="(child, index) in children" :key="index" dense align="center">
          <v-col cols="8">
            <v-text-field
              v-model="child.title"
              label="Task Title"
              density="compact"
              hide-details
            ></v-text-field>
          </v-col>
          <v-col cols="3">
            <v-text-field
              v-model.number="child.estimate_days"
              label="Est (d)"
              type="number"
              density="compact"
              hide-details
            ></v-text-field>
          </v-col>
          <v-col cols="1">
            <v-btn icon="mdi-delete" variant="text" size="small" @click="removeRow(index)"></v-btn>
          </v-col>
        </v-row>
        
        <v-btn 
          prepend-icon="mdi-plus" 
          variant="text" 
          class="mt-2" 
          @click="addRow"
          block
        >
          Add Row
        </v-btn>
      </v-card-text>

      <v-card-actions>
        <v-spacer></v-spacer>
        <v-btn variant="text" @click="show = false">Cancel</v-btn>
        <v-btn 
          color="primary" 
          @click="submit" 
          :loading="loading"
          :disabled="children.length === 0 || !children[0].title"
        >
          Create Tasks
        </v-btn>
      </v-card-actions>
    </v-card>
  </v-dialog>
</template>

<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { useIssueStore } from "../store/issueStore";

const props = defineProps<{
  modelValue: boolean;
  parent: any;
}>();

const emit = defineEmits(["update:modelValue", "created"]);

const issueStore = useIssueStore();
const loading = ref(false);
const children = ref([{ title: "", estimate_days: 0 }]);

const show = computed({
  get: () => props.modelValue,
  set: (val) => emit("update:modelValue", val)
});

const inheritedLabels = computed(() => {
  if (!props.parent || !issueStore.config) return [];
  return props.parent.labels.filter(
    (l: string) => !issueStore.config?.board.excluded_child_labels.includes(l)
  );
});

watch(() => props.modelValue, (newVal) => {
  if (newVal) {
    children.value = [{ title: "", estimate_days: 0 }];
  }
});

const addRow = () => {
  children.value.push({ title: "", estimate_days: 0 });
};

const removeRow = (index: number) => {
  children.value.splice(index, 1);
};

const submit = async () => {
  loading.value = true;
  try {
    const validChildren = children.value.filter(c => c.title.trim() !== "");
    await issueStore.createChildTasks(props.parent, validChildren);
    emit("created");
    show.value = false;
  } catch (e) {
    console.error(e);
  } finally {
    loading.value = false;
  }
};
</script>
