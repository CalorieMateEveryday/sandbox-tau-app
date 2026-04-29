<template>
  <v-card class="pa-0 workload-card" elevation="2">
    <v-card-title class="pa-4 d-flex align-center bg-grey-lighten-4">
      <v-icon start color="primary">mdi-table-large</v-icon>
      Workload Matrix
      <v-spacer></v-spacer>
      <v-btn-toggle
        v-model="rangePreset"
        density="compact"
        color="primary"
        variant="outlined"
        mandatory
        class="mr-4"
      >
        <v-btn value="7">7d</v-btn>
        <v-btn value="14">14d</v-btn>
        <v-btn value="30">30d</v-btn>
        <v-btn value="90">90d</v-btn>
        <v-btn value="all">All</v-btn>
      </v-btn-toggle>
      <v-text-field
        v-model.number="daysLimit"
        label="Custom Days"
        type="number"
        density="compact"
        hide-details
        variant="outlined"
        class="max-width-120"
        clearable
      ></v-text-field>
    </v-card-title>

    <div class="table-container">
      <v-table density="compact" hover class="workload-table">
        <thead>
          <tr>
            <th class="text-left sticky-column bg-white">Assignee</th>
            <th v-for="product in products" :key="product" class="text-center product-header">
              {{ product }}
            </th>
            <th class="text-center font-weight-bold total-header">Total</th>
          </tr>
        </thead>
        <tbody>
          <tr v-for="assignee in matrixData" :key="assignee.username" class="data-row">
            <td class="sticky-column font-weight-medium assignee-cell">
              <v-avatar size="24" class="mr-2" color="grey-lighten-2">
                <v-icon size="16">mdi-account</v-icon>
              </v-avatar>
              {{ assignee.name }}
            </td>
            <td v-for="product in products" :key="product" class="text-center">
              <div v-if="assignee.values[product] > 0" class="estimate-value">
                {{ assignee.values[product].toFixed(1) }}<span class="unit">d</span>
              </div>
              <span v-else class="text-grey-lighten-2">-</span>
            </td>
            <td class="text-center font-weight-bold total-cell">
              {{ assignee.total.toFixed(1) }}<span class="unit">d</span>
            </td>
          </tr>
        </tbody>
        <tfoot>
          <tr class="grand-total-row">
            <td class="sticky-column font-weight-bold">Total</td>
            <td v-for="product in products" :key="product" class="text-center font-weight-bold">
              {{ productTotals[product].toFixed(1) }}<span class="unit">d</span>
            </td>
            <td class="text-center font-weight-bold text-primary total-cell">
              {{ grandTotal.toFixed(1) }}<span class="unit text-primary">d</span>
            </td>
          </tr>
        </tfoot>
      </v-table>
    </div>
  </v-card>
</template>

<script setup lang="ts">
import { ref, computed, watch } from "vue";
import { useIssueStore } from "../store/issueStore";

const issueStore = useIssueStore();
const rangePreset = ref("all");
const daysLimit = ref<number | null>(null);

watch(rangePreset, (val) => {
  if (val === "all") {
    daysLimit.value = null;
  } else {
    daysLimit.value = parseInt(val);
  }
});

watch(daysLimit, (val) => {
  if (val === null) {
    rangePreset.value = "all";
  } else if ([7, 14, 30, 90].includes(val)) {
    rangePreset.value = val.toString();
  } else {
    rangePreset.value = "";
  }
});

const products = computed(() => {
  if (!issueStore.config) return [];
  return issueStore.config.products.map(p => p.name).sort();
});

const matrixData = computed(() => {
  if (!issueStore.config) return [];
  
  const configUsers = issueStore.config.users.map(u => ({
    username: u.username,
    name: u.display_name
  }));

  // Add "unassigned" to the list of rows
  const allRows = [...configUsers, { username: "unassigned", name: "Unassigned" }];
  
  const issues = issueStore.filteredIssues;
  const now = new Date();
  now.setHours(0, 0, 0, 0);
  
  return allRows.map(a => {
    const values: Record<string, number> = {};
    products.value.forEach(p => {
      values[p] = 0;
    });

    let total = 0;

    issues.forEach(issue => {
      // Check if this assignee is on the issue
      const isAssigned = (a.username === "unassigned" && issue.assignees.length === 0) ||
                         issue.assignees.some(as => as.username === a.username);
      
      if (!isAssigned) return;

      // Check due date
      if (daysLimit.value !== null && daysLimit.value !== undefined && daysLimit.value >= 0) {
        if (!issue.due_date) return;
        const dueDate = new Date(issue.due_date);
        const diffTime = dueDate.getTime() - now.getTime();
        const diffDays = Math.floor(diffTime / (1000 * 60 * 60 * 24));
        if (diffDays > daysLimit.value) return;
      }

      const estimate = issueStore.getDays(issue.time_stats.time_estimate);
      const product = issue.product || "Unknown";
      
      if (products.value.includes(product)) {
        values[product] += estimate;
        total += estimate;
      }
    });

    return {
      username: a.username,
      name: a.name,
      values,
      total
    };
  }).filter(a => {
    // Keep config users always, but filter unassigned if it has no issues
    if (a.username === "unassigned") return a.total > 0;
    return true;
  });
});

const productTotals = computed(() => {
  const totals: Record<string, number> = {};
  products.value.forEach(p => {
    totals[p] = matrixData.value.reduce((sum, a) => sum + a.values[p], 0);
  });
  return totals;
});

const grandTotal = computed(() => {
  return matrixData.value.reduce((sum, a) => sum + a.total, 0);
});
</script>

<style scoped>
.workload-card {
  border-radius: 12px;
  overflow: hidden;
}

.max-width-120 {
  max-width: 120px;
}

.table-container {
  overflow-x: auto;
  max-height: calc(100vh - 200px);
}

.workload-table {
  border-collapse: separate;
  border-spacing: 0;
}

.sticky-column {
  position: sticky;
  left: 0;
  z-index: 10;
  border-right: 1px solid rgba(0,0,0,0.05);
}

.assignee-cell {
  background: #fafafa !important;
  min-width: 180px;
}

.product-header {
  min-width: 120px;
  background: #f5f5f5;
  color: #666;
  font-size: 0.75rem;
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.total-header {
  background: #eeeeee;
  min-width: 100px;
}

.data-row:hover .assignee-cell {
  background: #f0f0f0 !important;
}

.estimate-value {
  color: #1976D2;
  font-weight: 500;
}

.unit {
  font-size: 0.8em;
  color: #999;
  margin-left: 2px;
}

.total-cell {
  background: #fcfcfc;
}

.grand-total-row {
  background: #f5f5f5;
}

.grand-total-row td {
  border-top: 2px solid #ddd !important;
}
</style>
