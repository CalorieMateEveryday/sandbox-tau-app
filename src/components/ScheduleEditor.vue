<template>
  <v-dialog v-model="dialog" max-width="960" scrollable>
    <v-card class="schedule-editor">
      <v-card-title class="d-flex align-center pa-4">
        <v-icon class="mr-2" color="primary">mdi-calendar-edit</v-icon>
        スケジュール編集
        <v-spacer />
        <v-btn icon variant="text" @click="dialog = false"><v-icon>mdi-close</v-icon></v-btn>
      </v-card-title>
      <v-divider />

      <v-card-text class="pa-4">
        <v-text-field v-model="localTitle" label="スケジュールタイトル"
          prepend-inner-icon="mdi-format-title" density="compact" class="mb-4" />

        <v-text-field v-model="localExcludes" label="除外設定 (excludes)"
          prepend-inner-icon="mdi-calendar-remove" density="compact" class="mb-4"
          hint="例: weekends (週末を除外)・2026-01-01, 2026-01-02 (特定日を除外)" persistent-hint />

        <div v-for="(section, sIdx) in localSections" :key="section.id" class="section-block mb-4">
          <!-- Section header -->
          <div class="d-flex align-center section-header pa-2 rounded-t">
            <v-icon size="small" class="mr-2" color="secondary">mdi-folder</v-icon>
            <v-text-field v-model="section.name" density="compact" variant="plain"
              hide-details class="section-name-field" placeholder="セクション名" />
            <v-btn icon size="x-small" variant="text" :disabled="sIdx === 0" @click="moveSectionUp(sIdx)">
              <v-icon size="small">mdi-arrow-up</v-icon>
            </v-btn>
            <v-btn icon size="x-small" variant="text" :disabled="sIdx === localSections.length - 1" @click="moveSectionDown(sIdx)">
              <v-icon size="small">mdi-arrow-down</v-icon>
            </v-btn>
            <v-btn icon size="x-small" variant="text" color="error" @click="removeSection(sIdx)">
              <v-icon size="small">mdi-delete</v-icon>
            </v-btn>
          </div>

          <!-- Task list -->
          <v-table density="compact" class="task-table rounded-b">
            <thead>
              <tr>
                <th style="width:30%">タスク名</th>
                <th style="width:15%">種別</th>
                <th style="width:30%">開始</th>
                <th style="width:10%">期間</th>
                <th style="width:15%">操作</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="(task, tIdx) in section.tasks" :key="task.id"
                class="task-row" @click="openTaskEdit(sIdx, tIdx)">
                <td class="font-weight-medium">{{ task.name || '(未設定)' }}</td>
                <td>
                  <v-chip v-for="m in task.modifiers" :key="m" size="x-small" class="mr-1">{{ m }}</v-chip>
                  <span v-if="!task.modifiers.length" class="text-medium-emphasis text-caption">task</span>
                </td>
                <td class="text-caption">
                  <v-icon v-if="task.start.startsWith('after')" size="x-small" color="warning" class="mr-1">mdi-arrow-right</v-icon>
                  {{ task.start || '-' }}
                </td>
                <td class="text-caption">{{ task.duration }}</td>
                <td class="text-no-wrap" @click.stop>
                  <v-btn icon size="x-small" variant="text" @click="openTaskEdit(sIdx, tIdx)">
                    <v-icon size="small">mdi-pencil</v-icon>
                  </v-btn>
                  <v-btn icon size="x-small" variant="text" :disabled="tIdx === 0" @click="moveTaskUp(sIdx, tIdx)">
                    <v-icon size="small">mdi-arrow-up</v-icon>
                  </v-btn>
                  <v-btn icon size="x-small" variant="text" :disabled="tIdx === section.tasks.length - 1" @click="moveTaskDown(sIdx, tIdx)">
                    <v-icon size="small">mdi-arrow-down</v-icon>
                  </v-btn>
                  <v-btn icon size="x-small" variant="text" color="error" @click="removeTask(sIdx, tIdx)">
                    <v-icon size="small">mdi-delete</v-icon>
                  </v-btn>
                </td>
              </tr>
              <tr>
                <td colspan="5">
                  <v-btn size="small" variant="text" color="primary" prepend-icon="mdi-plus" @click="addTask(sIdx)">
                    タスク追加
                  </v-btn>
                </td>
              </tr>
            </tbody>
          </v-table>
        </div>

        <v-btn variant="outlined" color="secondary" prepend-icon="mdi-folder-plus" block @click="addSection">
          セクション追加
        </v-btn>
      </v-card-text>

      <v-divider />
      <v-card-actions class="pa-4">
        <v-btn variant="text" @click="dialog = false">キャンセル</v-btn>
        <v-spacer />
        <v-btn color="primary" variant="flat" prepend-icon="mdi-content-save" @click="save">保存</v-btn>
      </v-card-actions>
    </v-card>

    <!-- ─── Task Edit Dialog ──────────────────────────────────────────────── -->
    <v-dialog v-model="taskEditDialog" max-width="540" :persistent="true">
      <v-card v-if="editingTask" class="pa-1">
        <v-card-title class="text-subtitle-1 pa-4">
          <v-icon class="mr-2" color="primary" size="small">mdi-pencil-box</v-icon>
          タスク編集
        </v-card-title>
        <v-divider />
        <v-card-text class="pa-4">
          <!-- Task name -->
          <v-text-field v-model="editingTask.name" label="タスク名"
            density="compact" class="mb-3" autofocus />

          <!-- Alias -->
          <v-text-field v-model="editingTask.alias" label="エイリアス (ID)"
            density="compact" class="mb-3"
            hint="依存関係の参照ID。省略可（英数字・ハイフン推奨）" persistent-hint
            placeholder="例: task-a1" />

          <!-- Modifiers -->
          <v-select v-model="editingTask.modifiers" label="種別"
            :items="modifierItems" item-title="label" item-value="value"
            multiple chips density="compact" class="mb-4"
            :chip-props="{ size: 'small' }" />

          <!-- Date picker -->
          <v-menu v-model="dateMenu" :close-on-content-click="false">
            <template #activator="{ props: menuProps }">
              <v-text-field v-bind="menuProps"
                :model-value="editingTask.start"
                label="開始日 (YYYY-MM-DD)"
                prepend-inner-icon="mdi-calendar"
                readonly density="compact" class="mb-3" />
            </template>
            <v-date-picker
              :model-value="parsedDate"
              color="primary"
              @update:model-value="onDatePicked"
            />
          </v-menu>

          <!-- Dependency (optional, stored alongside date) -->
          <v-select v-model="editingTask.dependsOn"
            :items="availableAliases"
            item-title="label" item-value="value"
            label="前提タスク（参考）"
            density="compact" class="mb-1"
            clearable
            no-data-text="エイリアスが設定されたタスクがありません" />
          <div class="text-caption text-medium-emphasis mb-3">
            <v-icon size="x-small">mdi-information-outline</v-icon>
            Mermaid 記法では日付と前提タスクは排他仕様のため、チャートは日付で描画されます。前提タスクは .mmd ファイルにコメントとして保存されます。
          </div>

          <!-- Duration -->
          <v-text-field v-model="editingTask.duration" label="期間"
            density="compact" hint="例: 7d (7日)・2w (2週間)" persistent-hint
            placeholder="7d" />
        </v-card-text>
        <v-divider />
        <v-card-actions class="pa-3">
          <v-spacer />
          <v-btn variant="text" @click="taskEditDialog = false">キャンセル</v-btn>
          <v-btn color="primary" variant="flat" @click="saveTaskEdit">OK</v-btn>
        </v-card-actions>
      </v-card>
    </v-dialog>
  </v-dialog>
</template>

<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { useScheduleStore } from "../store/scheduleStore";
import type { GanttSection, TaskModifier } from "../store/scheduleStore";

const props = defineProps<{ modelValue: boolean }>();
const emit = defineEmits<{ "update:modelValue": [v: boolean]; saved: [] }>();

const scheduleStore = useScheduleStore();
const dialog = ref(props.modelValue);

watch(() => props.modelValue, (v) => { dialog.value = v; if (v) initLocal(); });
watch(dialog, (v) => emit("update:modelValue", v));

// ─── Local copy ───────────────────────────────────────────────────────────────
const localTitle = ref("");
const localExcludes = ref("");
const localSections = ref<GanttSection[]>([]);
let _idCounter = 10000;
const newId = () => `local-${_idCounter++}`;

function initLocal() {
  if (!scheduleStore.parsedGantt) return;
  localTitle.value = scheduleStore.parsedGantt.title;
  localExcludes.value = scheduleStore.parsedGantt.excludes || "";
  localSections.value = scheduleStore.parsedGantt.sections.map((s) => ({
    ...s,
    tasks: s.tasks.map((t) => ({ ...t })),
  }));
}

// ─── Task edit dialog ─────────────────────────────────────────────────────────
const taskEditDialog = ref(false);
const editingTask = ref<any>(null);
const editingSIdx = ref(0);
const editingTIdx = ref(0);
const dateMenu = ref(false);

const modifierItems = [
  { label: "milestone", value: "milestone" },
  { label: "crit（重要）", value: "crit" },
  { label: "active（進行中）", value: "active" },
  { label: "done（完了）", value: "done" },
] as { label: string; value: TaskModifier }[];

/** All task aliases in the gantt, excluding the currently-edited task */
const availableAliases = computed(() => {
  const result: { label: string; value: string }[] = [];
  localSections.value.forEach((section) => {
    section.tasks.forEach((task) => {
      if (task.alias && task.id !== editingTask.value?.id) {
        result.push({ label: `${task.name}（${task.alias}）`, value: task.alias });
      }
    });
  });
  return result;
});

/** Parse the current start string into a Date for v-date-picker */
const parsedDate = computed<Date | undefined>(() => {
  const s = editingTask.value?.start;
  if (!s || s.startsWith("after")) return undefined;
  const d = new Date(s + "T00:00:00");
  return isNaN(d.getTime()) ? undefined : d;
});

function openTaskEdit(sIdx: number, tIdx: number) {
  editingTask.value = { ...localSections.value[sIdx].tasks[tIdx] };
  editingSIdx.value = sIdx;
  editingTIdx.value = tIdx;
  taskEditDialog.value = true;
}

function onDatePicked(val: unknown) {
  if (!val || !editingTask.value) return;
  const d = val instanceof Date ? val : new Date(val as string);
  if (!isNaN(d.getTime())) {
    editingTask.value.start = d.toISOString().slice(0, 10);
  }
  dateMenu.value = false;
}

function saveTaskEdit() {
  if (!editingTask.value) return;
  localSections.value[editingSIdx.value].tasks[editingTIdx.value] = { ...editingTask.value };
  taskEditDialog.value = false;
}

// ─── Section operations ───────────────────────────────────────────────────────
function addSection() {
  localSections.value.push({ id: newId(), name: "新規セクション", tasks: [] });
}
function removeSection(idx: number) { localSections.value.splice(idx, 1); }
function moveSectionUp(idx: number) {
  if (idx > 0) {
    const a = localSections.value;
    [a[idx - 1], a[idx]] = [a[idx], a[idx - 1]];
  }
}
function moveSectionDown(idx: number) {
  const a = localSections.value;
  if (idx < a.length - 1) [a[idx], a[idx + 1]] = [a[idx + 1], a[idx]];
}

// ─── Task operations ──────────────────────────────────────────────────────────
function addTask(sIdx: number) {
  const today = new Date().toISOString().slice(0, 10);
  localSections.value[sIdx].tasks.push({
    id: newId(), name: "新規タスク", alias: "",
    modifiers: [], start: today, duration: "7d",
  });
  openTaskEdit(sIdx, localSections.value[sIdx].tasks.length - 1);
}
function removeTask(sIdx: number, tIdx: number) { localSections.value[sIdx].tasks.splice(tIdx, 1); }
function moveTaskUp(sIdx: number, tIdx: number) {
  const a = localSections.value[sIdx].tasks;
  if (tIdx > 0) [a[tIdx - 1], a[tIdx]] = [a[tIdx], a[tIdx - 1]];
}
function moveTaskDown(sIdx: number, tIdx: number) {
  const a = localSections.value[sIdx].tasks;
  if (tIdx < a.length - 1) [a[tIdx], a[tIdx + 1]] = [a[tIdx + 1], a[tIdx]];
}

// ─── Save ─────────────────────────────────────────────────────────────────────
async function save() {
  if (!scheduleStore.parsedGantt) return;
  scheduleStore.parsedGantt.title = localTitle.value;
  scheduleStore.parsedGantt.excludes = localExcludes.value;
  scheduleStore.parsedGantt.sections = localSections.value;
  await scheduleStore.saveScheduleFile();
  emit("saved");
  dialog.value = false;
}
</script>

<style scoped>
.schedule-editor { background: #0f172a; }
.section-header { background: #1e293b; border: 1px solid #334155; }
.section-name-field { flex: 1; font-weight: 600; }
.task-table { border: 1px solid #334155; border-top: none; }
.task-table :deep(th) { background: #1e293b !important; color: #94a3b8 !important; font-size: 0.75rem !important; }
.task-row { cursor: pointer; }
.task-row:hover :deep(td) { background: rgba(59, 130, 246, 0.08) !important; }
</style>
