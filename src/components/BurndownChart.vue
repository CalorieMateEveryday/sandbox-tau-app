<template>
  <v-card class="pa-4">
    <v-card-title class="d-flex align-center">
      Burndown Chart
      <v-spacer></v-spacer>
      <v-switch v-model="fixedScaleEnabled" label="Fixed Scale" hide-details class="mr-4"></v-switch>
      <v-text-field
        v-model.number="targetHorizonDays"
        label="Target Horizon (Days)"
        type="number"
        style="max-width: 150px"
        hide-details
        density="compact"
      ></v-text-field>
    </v-card-title>
    <v-card-subtitle>Workload vs. Capacity</v-card-subtitle>
    <div class="chart-container mt-4">
      <Line :data="chartData" :options="chartOptions" />
    </div>
  </v-card>
</template>

<script setup lang="ts">
import { computed, ref, onMounted, watch } from "vue";
import { Line } from "vue-chartjs";
import { invoke } from "@tauri-apps/api/core";
import {
  Chart as ChartJS,
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  Title,
  Tooltip,
  Legend,
} from "chart.js";
import annotationPlugin from "chartjs-plugin-annotation";
import { useIssueStore } from "../store/issueStore";

ChartJS.register(
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  Title,
  Tooltip,
  Legend,
  annotationPlugin
);

const issueStore = useIssueStore();
const working_days_list = ref<string[]>([]);
const targetHorizonDays = ref(30);
const fixedScaleEnabled = ref(true);

const calculateChart = async () => {
  const issues = issueStore.issues; // Use all issues to determine max range
  if (issues.length === 0) return;

  const today = new Date().toISOString().split("T")[0];
  // Find max due date across ALL issues to keep horizontal scale consistent
  let maxDate = today;
  issues.forEach(i => {
    const due = i.due_date || (i.milestone?.due_date);
    if (due && due > maxDate) maxDate = due;
  });

  // Fetch working days from Rust
  try {
    working_days_list.value = await invoke("get_working_days", { startDate: today, endDate: maxDate });
  } catch (e) {
    console.error(e);
  }
};

onMounted(calculateChart);
watch(() => issueStore.issues, calculateChart);
watch(() => issueStore.filteredIssues, () => {
    // We don't necessarily need to re-calculate working_days if we want fixed scale
    // but we might want to if the global max changed.
});

const chartData = computed(() => {
  const days = working_days_list.value;
  if (days.length === 0) return { labels: [], datasets: [] };

  const issues = issueStore.filteredIssues;
  
  // Calculate Target Date
  const todayDate = new Date();
  const targetDateObj = new Date(todayDate);
  targetDateObj.setDate(todayDate.getDate() + targetHorizonDays.value);
  const targetDateStr = targetDateObj.toISOString().split("T")[0];

  // Calculate work to be done by target date
  // We subtract work due AFTER the target date from the total.
  const totalFilteredEstimate = issues.reduce((sum, i) => sum + issueStore.getDays(i.time_stats.time_estimate), 0);
  const workDueAfterTarget = issues
    .filter(i => {
        const due = i.due_date || i.milestone?.due_date;
        return due && due > targetDateStr;
    })
    .reduce((sum, i) => sum + issueStore.getDays(i.time_stats.time_estimate), 0);
  
  const targetWorkload = Math.max(0, totalFilteredEstimate - workDueAfterTarget);

  // Find index of target date in days list
  let targetIndex = days.findIndex(d => d >= targetDateStr);
  if (targetIndex === -1) targetIndex = days.length - 1;
  if (targetIndex === 0) targetIndex = 1; // Avoid division by zero

  // Ideal Burn (Linear to Target)
  const idealData = days.map((_, index) => {
    if (index > targetIndex) return 0;
    return targetWorkload * (1 - index / targetIndex);
  });

  // Expected Burn (Capacity based)
  let remainingExpected = targetWorkload;
  const expectedData = days.map((date, index) => {
    if (index === 0) return remainingExpected;
    
    let dailyCapacity = 0;
    const activeUsernames = new Set(issues.flatMap(i => i.assignees.map(a => a.username)));
    activeUsernames.forEach(u => {
      dailyCapacity += issueStore.capacities[u] || 1.0; 
    });

    remainingExpected = Math.max(0, remainingExpected - dailyCapacity);
    return remainingExpected;
  });

  return {
    labels: days,
    datasets: [
      {
        label: `Required Burn (Target: ${targetHorizonDays.value}d)`,
        borderColor: "#f87979",
        data: idealData,
        fill: false,
        pointRadius: 0,
      },
      {
        label: "Expected Burn (Capacity)",
        borderColor: "#36A2EB",
        borderDash: [5, 5],
        data: expectedData,
        fill: false,
        pointRadius: 0,
      },
    ],
  };
});

const milestoneAnnotations = computed(() => {
  const annotations: any = {};
  const milestones = new Map<string, string>(); // title -> date
  
  issueStore.issues.forEach(i => {
    if (i.milestone && i.milestone.due_date) {
      milestones.set(i.milestone.title, i.milestone.due_date);
    }
  });

  milestones.forEach((date, title) => {
    annotations[`line-${title}`] = {
      type: "line",
      xMin: date,
      xMax: date,
      borderColor: "rgba(150, 150, 150, 0.5)",
      borderWidth: 2,
      label: {
        display: true,
        content: title,
        position: "start",
        backgroundColor: "rgba(150, 150, 150, 0.8)",
      }
    };
  });

  return annotations;
});

const chartOptions = computed(() => {
  const options: any = {
    responsive: true,
    maintainAspectRatio: false,
    scales: {
      y: {
        beginAtZero: true,
        title: {
          display: true,
          text: "Person-Days",
        },
      },
      x: {
        ticks: {
          maxRotation: 45,
          minRotation: 45,
        }
      }
    },
    plugins: {
      annotation: {
        annotations: milestoneAnnotations.value,
      },
      legend: {
        position: "top" as const,
      }
    },
  };

  if (fixedScaleEnabled.value) {
    // Calculate global max Y
    // For comparison, we want the max workload any single assignee has, 
    // or just a large enough fixed value. 
    // Let's find the max total workload of any user in the current issue set.
    const userWorkloads = new Map<string, number>();
    issueStore.issues.forEach(i => {
        i.assignees.forEach(a => {
            const days = issueStore.getDays(i.time_stats.time_estimate);
            userWorkloads.set(a.username, (userWorkloads.get(a.username) || 0) + days);
        });
    });
    const maxUserWorkload = Math.max(...Array.from(userWorkloads.values()), 0);
    
    // Set Y max to something consistent, e.g. next multiple of 10
    options.scales.y.max = Math.ceil((maxUserWorkload + 5) / 10) * 10;
  }

  return options;
});
</script>

<style scoped>
.chart-container {
  height: 400px;
}
</style>
