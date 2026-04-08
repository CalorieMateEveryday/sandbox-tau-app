<template>
  <v-row>
    <v-col cols="12" md="4">
      <v-card class="pa-4 text-center">
        <v-card-subtitle>Overall Project progress</v-card-subtitle>
        <v-card-title class="text-h4">{{ totalProgress }}%</v-card-title>
        <v-progress-linear :model-value="totalProgress" color="primary" height="10"></v-progress-linear>
      </v-card>
    </v-col>
    <v-col cols="12" md="4">
      <v-card class="pa-4 text-center">
        <v-card-subtitle>Estimated Total</v-card-subtitle>
        <v-card-title class="text-h4">{{ totalEstimate.toFixed(1) }}d</v-card-title>
      </v-card>
    </v-col>
    <v-col cols="12" md="4">
      <v-card class="pa-4 text-center">
        <v-card-subtitle>Spent Total</v-card-subtitle>
        <v-card-title class="text-h4">{{ totalSpent.toFixed(1) }}d</v-card-title>
      </v-card>
    </v-col>

    <v-col cols="12">
      <v-card class="pa-4">
        <v-card-title>Progress by Product</v-card-title>
        <v-table>
          <thead>
            <tr>
              <th>Product</th>
              <th>Estimate</th>
              <th>Spent</th>
               <th>Progress</th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="product in productStats" :key="product.name">
               <td>{{ product.name }}</td>
               <td>{{ product.estimate.toFixed(1) }}d</td>
               <td>{{ product.spent.toFixed(1) }}d</td>
               <td>
                <v-progress-linear :model-value="product.progress" color="secondary" height="20">
                  <template v-slot:default="{ value }">
                    <strong>{{ Math.ceil(value) }}%</strong>
                  </template>
                </v-progress-linear>
               </td>
            </tr>
          </tbody>
        </v-table>
      </v-card>
    </v-col>
  </v-row>
</template>

<script setup lang="ts">
import { computed } from "vue";
import { useIssueStore } from "../store/issueStore";

const issueStore = useIssueStore();

const totalEstimate = computed(() => 
  issueStore.filteredIssues.reduce((sum, i) => sum + issueStore.getDays(i.time_stats.time_estimate), 0)
);

const totalSpent = computed(() => 
  issueStore.filteredIssues.reduce((sum, i) => sum + issueStore.getDays(i.time_stats.total_time_spent), 0)
);

const totalProgress = computed(() => {
  if (totalEstimate.value === 0) return 0;
  return Math.min(100, Math.round((totalSpent.value / totalEstimate.value) * 100));
});

const productStats = computed(() => {
  const products = issueStore.uniqueProducts;
  return products.map(name => {
    const pIssues = issueStore.filteredIssues.filter(i => i.product === name);
    const estimate = pIssues.reduce((sum, i) => sum + issueStore.getDays(i.time_stats.time_estimate), 0);
    const spent = pIssues.reduce((sum, i) => sum + issueStore.getDays(i.time_stats.total_time_spent), 0);
    const progress = estimate === 0 ? 0 : Math.min(100, (spent / estimate) * 100);
    return { name, estimate, spent, progress };
  });
});
</script>
