<template>
  <v-card class="pa-4">
    <v-card-title>Burndown Chart</v-card-title>
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
import { useIssueStore } from "../store/issueStore";

ChartJS.register(
  CategoryScale,
  LinearScale,
  PointElement,
  LineElement,
  Title,
  Tooltip,
  Legend
);

const issueStore = useIssueStore();
const workingDays = ref<string[]>([]);

const calculateChart = async () => {
  const issues = issueStore.filteredIssues;
  if (issues.length === 0) return;

  const today = new Date().toISOString().split("T")[0];
  // Find max due date
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

const working_days_list = ref<string[]>([]);

onMounted(calculateChart);
watch(() => issueStore.filteredIssues, calculateChart);

const chartData = computed(() => {
  const days = working_days_list.value;
  if (days.length === 0) return { labels: [], datasets: [] };

  const issues = issueStore.filteredIssues;
  let totalEstimateDays = issues.reduce((sum, i) => sum + issueStore.getDays(i.time_stats.time_estimate), 0);
  
  // Ideal Burn (Linear)
  const idealData = days.map((_, index) => {
    return totalEstimateDays * (1 - index / (days.length - 1));
  });

  // Expected Burn (Capacity based)
  let remainingExpected = totalEstimateDays;
  const expectedData = days.map(date => {
    let dailyCapacity = 0;
    // Calculate total capacity for all assigned users on this day
    const activeUsernames = new Set(issues.flatMap(i => i.assignees.map(a => a.username)));
    activeUsernames.forEach(u => {
      // capacities[u] is already "days per person-day"
      dailyCapacity += issueStore.capacities[u] || 1.0; 
    });

    remainingExpected = Math.max(0, remainingExpected - dailyCapacity);
    return remainingExpected;
  });

  return {
    labels: days,
    datasets: [
      {
        label: "Remaining Work (Ideal)",
        borderColor: "#f87979",
        data: idealData,
        fill: false,
      },
      {
        label: "Expected Work Line (Capacity)",
        borderColor: "#36A2EB",
        borderDash: [5, 5],
        data: expectedData,
        fill: false,
      },
    ],
  };
});

const chartOptions = {
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
  },
};
</script>

<style scoped>
.chart-container {
  height: 400px;
}
</style>
