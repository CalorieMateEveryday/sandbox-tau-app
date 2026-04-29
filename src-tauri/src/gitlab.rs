use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize)]
pub struct GraphQLResponse<T> {
    pub data: Option<T>,
    pub errors: Option<Vec<GraphQLError>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GraphQLError {
    pub message: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkItemNode {
    pub id: String,
    pub iid: String,
    pub title: String,
    pub description: Option<String>,
    pub state: String,
    #[serde(rename = "webUrl")]
    pub web_url: String,
    pub widgets: Vec<WorkItemWidget>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")] 
pub enum WorkItemWidget {
    #[serde(alias = "WorkItemWidgetAssignees")]
    Assignees {
        assignees: AssigneeNodes,
    },
    #[serde(alias = "WorkItemWidgetHierarchy")]
    Hierarchy {
        parent: Option<ParentWorkItem>,
        children: Option<ChildNodes>,
    },
    #[serde(alias = "WorkItemWidgetTimeTracking")]
    TimeTracking {
        #[serde(rename = "timeEstimate")]
        time_estimate: i64,
        #[serde(rename = "totalTimeSpent")]
        total_time_spent: i64,
    },
    #[serde(alias = "WorkItemWidgetMilestone")]
    Milestone {
        milestone: Option<MilestoneNode>,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AssigneeNodes {
    pub nodes: Vec<AssigneeNode>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AssigneeNode {
    pub id: String,
    pub username: String,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MilestoneNode {
    pub id: String,
    pub title: String,
    #[serde(rename = "dueDate")]
    pub due_date: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ParentWorkItem {
    pub id: String,
    pub iid: String,
    pub title: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChildNodes {
    pub nodes: Vec<ChildNode>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChildNode {
    pub id: String,
    pub iid: String,
    pub title: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TimelogNode {
    #[serde(rename = "timeSpent")]
    pub time_spent: i64,
    #[serde(rename = "spentAt")]
    pub spent_at: String,
    pub user: Option<TimelogUser>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TimelogUser {
    pub username: String,
}

pub const TIMELOGS_QUERY: &str = r#"
query($fullPath: ID!, $startDate: Time!, $endDate: Time!) {
  project(fullPath: $fullPath) {
    timelogs(startDate: $startDate, endDate: $endDate) {
      nodes {
        timeSpent
        spentAt
        user { username }
      }
    }
  }
  group(fullPath: $fullPath) {
    timelogs(startDate: $startDate, endDate: $endDate) {
      nodes {
        timeSpent
        spentAt
        user { username }
      }
    }
  }
}
"#;

pub async fn get_issue_labels_rest(url: &str, token: &str, project_id: &str, issue_iid: u64) -> Result<Vec<String>, String> {
    let client = reqwest::Client::new();
    let res = client
        .get(format!("{}/api/v4/projects/{}/issues/{}", url, project_id, issue_iid))
        .header("PRIVATE-TOKEN", token)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        return Err(format!("GitLab REST error: {}", res.status()));
    }

    let issue: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
    let labels = issue["labels"]
        .as_array()
        .ok_or("No labels field in REST response")?
        .iter()
        .map(|l| l.as_str().unwrap_or_default().to_string())
        .collect();

    Ok(labels)
}

pub async fn create_milestone_rest(url: &str, token: &str, project_id: &str, title: &str, due_date: Option<&str>) -> Result<(), String> {
    let client = reqwest::Client::new();
    let mut map = std::collections::HashMap::new();
    map.insert("title", title);
    if let Some(due) = due_date {
        map.insert("due_date", due);
    }

    let res = client
        .post(format!("{}/api/v4/projects/{}/milestones", url, project_id))
        .header("PRIVATE-TOKEN", token)
        .json(&map)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        return Err(format!("GitLab Milestone creation error: {}", res.status()));
    }
    Ok(())
}

pub async fn create_issue_rest(
    url: &str, 
    token: &str, 
    project_id: &str, 
    title: &str, 
    milestone_id: Option<u64>, 
    due_date: Option<&str>,
    labels: Vec<String>,
    assignee_ids: Vec<u64>
) -> Result<(), String> {
    let client = reqwest::Client::new();
    let mut body = serde_json::json!({
        "title": title,
        "labels": labels.join(","),
        "assignee_ids": assignee_ids,
    });

    if let Some(m_id) = milestone_id {
        body["milestone_id"] = serde_json::json!(m_id);
    }
    if let Some(due) = due_date {
        body["due_date"] = serde_json::json!(due);
    }

    let res = client
        .post(format!("{}/api/v4/projects/{}/issues", url, project_id))
        .header("PRIVATE-TOKEN", token)
        .json(&body)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        return Err(format!("GitLab Issue creation error: {}", res.status()));
    }
    Ok(())
}

pub async fn query_gitlab<T>(url: &str, token: &str, query: &str, variables: serde_json::Value) -> Result<T, String> 
where T: for<'de> Deserialize<'de> 
{
    let client = reqwest::Client::new();
    let res = client
        .post(format!("{}/api/graphql", url))
        .header("PRIVATE-TOKEN", token) 
        .json(&serde_json::json!({
            "query": query,
            "variables": variables
        }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !res.status().is_success() {
        return Err(format!("GitLab (HTTP {}) error: {}", res.status(), res.text().await.unwrap_or_default()));
    }

    let gql_res: GraphQLResponse<T> = res.json().await.map_err(|e| e.to_string())?;

    // If we have data, return it even if there are errors (partial success)
    if let Some(data) = gql_res.data {
        return Ok(data);
    }

    if let Some(errors) = gql_res.errors {
        return Err(errors.into_iter().map(|e| e.message).collect::<Vec<_>>().join(", "));
    }

    Err("No data or errors in response".to_string())
}

fn check_gql_errors(res: &serde_json::Value, pointer_base: &str) -> Result<(), String> {
    let path = format!("/{}/errors", pointer_base);
    if let Some(errors) = res.pointer(&path).and_then(|e| e.as_array()) {
        if !errors.is_empty() {
            return Err(errors[0].as_str().unwrap_or("Unknown error").into());
        }
    }
    Ok(())
}

// Example query for Work Items
pub const WORK_ITEMS_QUERY: &str = r#"
query($namespacePath: ID!) {
  project(fullPath: $namespacePath) {
    workItems(first: 100) {
      nodes {
        id
        iid
        title
        description
        state
        webUrl
        widgets {
          ... on WorkItemWidgetAssignees {
            assignees {
              nodes {
                id
                username
                name
              }
            }
          }
          ... on WorkItemWidgetMilestone {
            milestone {
              id
              title
              dueDate
            }
          }
          ... on WorkItemWidgetHierarchy {
            parent {
              id
              iid
              title
            }
            children {
              nodes {
                id
                iid
                title
              }
            }
          }
          ... on WorkItemWidgetProgress {
            progress
          }
          ... on WorkItemWidgetTimeTracking {
            timeEstimate
            totalTimeSpent
          }
        }
      }
    }
  }
  group(fullPath: $namespacePath) {
    workItems(first: 100) {
      nodes {
        id
        iid
        title
        description
        state
        webUrl
        widgets {
          ... on WorkItemWidgetAssignees {
            assignees {
              nodes {
                id
                username
                name
              }
            }
          }
          ... on WorkItemWidgetMilestone {
            milestone {
              id
              title
              dueDate
            }
          }
          ... on WorkItemWidgetHierarchy {
            parent {
              id
              iid
              title
            }
            children {
              nodes {
                id
                iid
                title
              }
            }
          }
          ... on WorkItemWidgetProgress {
            progress
          }
          ... on WorkItemWidgetTimeTracking {
            timeEstimate
            totalTimeSpent
          }
        }
      }
    }
  }
}
"#;


#[derive(Debug, Serialize, Deserialize)]
pub struct WorkItemUpdateInput {
    pub id: String,
    pub title: Option<String>,
    pub description: Option<String>,
    pub estimate_seconds: Option<i64>,
    pub labels: Option<Vec<String>>,
    pub milestone_id: Option<String>,
    pub assignee_ids: Option<Vec<String>>,
    pub due_date: Option<String>,
}

pub async fn update_work_item_gql(
    gl_url: &str,
    token: &str,
    input: WorkItemUpdateInput
) -> Result<(), String> {
    let mut mutation_parts: Vec<String> = Vec::new();
    let mut variables = serde_json::json!({ "id": input.id });

    if let Some(title) = input.title {
        mutation_parts.push("title: $title".to_string());
        variables["title"] = serde_json::json!(title);
    }

    if let Some(desc) = input.description {
        mutation_parts.push("descriptionWidget: { description: $desc }".to_string());
        variables["desc"] = serde_json::json!(desc);
    }

    if let Some(est) = input.estimate_seconds {
        mutation_parts.push("timeTrackingWidget: { timeEstimate: $estimate }".to_string());
        variables["estimate"] = serde_json::json!(format!("{}s", est));
    }

    if let Some(target_labels) = input.labels {
        // 現在のラベルを取得して差分を計算
        let current_labels = get_work_item_labels_gql(gl_url, token, &input.id).await?;
        
        let current_set: std::collections::HashSet<_> = current_labels.into_iter().collect();
        let target_set: std::collections::HashSet<_> = target_labels.into_iter().collect();

        let labels_to_add: Vec<_> = target_set.difference(&current_set).cloned().collect();
        let labels_to_remove: Vec<_> = current_set.difference(&target_set).cloned().collect();

        let mut labels_widget_parts = Vec::new();
        if !labels_to_add.is_empty() {
            labels_widget_parts.push("addLabelNames: $addLabels");
            variables["addLabels"] = serde_json::json!(labels_to_add);
        }
        if !labels_to_remove.is_empty() {
            labels_widget_parts.push("removeLabelNames: $removeLabels");
            variables["removeLabels"] = serde_json::json!(labels_to_remove);
        }

        if !labels_widget_parts.is_empty() {
            mutation_parts.push(format!("labelsWidget: {{ {} }}", labels_widget_parts.join(", ")));
        }
    }

    if let Some(ms_id) = input.milestone_id {
        mutation_parts.push("milestoneWidget: { milestoneId: $msId }".to_string());
        variables["msId"] = serde_json::json!(ms_id);
    }

    if let Some(assignees) = input.assignee_ids {
        mutation_parts.push("assigneesWidget: { assigneeIds: $assignees }".to_string());
        variables["assignees"] = serde_json::json!(assignees);
    }

    if let Some(due) = input.due_date {
        mutation_parts.push("startAndDueDateWidget: { dueDate: $due }".to_string());
        variables["due"] = serde_json::json!(due);
    }

    if mutation_parts.is_empty() {
        return Ok(());
    }

    // 動的に変数の型定義を構築
    let mut var_defs = vec!["$id: WorkItemID!"];
    if variables.get("title").is_some() { var_defs.push("$title: String"); }
    if variables.get("desc").is_some() { var_defs.push("$desc: String"); }
    if variables.get("estimate").is_some() { var_defs.push("$estimate: String"); }
    if variables.get("addLabels").is_some() { var_defs.push("$addLabels: [String!]"); }
    if variables.get("removeLabels").is_some() { var_defs.push("$removeLabels: [String!]"); }
    if variables.get("msId").is_some() { var_defs.push("$msId: MilestoneID"); }
    if variables.get("assignees").is_some() { var_defs.push("$assignees: [UserID!]"); }
    if variables.get("due").is_some() { var_defs.push("$due: Date"); }

    let query = format!(
        r#"
        mutation({}) {{
          workItemUpdate(input: {{
            id: $id,
            {}
          }}) {{
            errors
          }}
        }}
        "#,
        var_defs.join(", "),
        mutation_parts.join(",\n            ")
    );

    let res: serde_json::Value = query_gitlab(gl_url, token, &query, variables).await?;
    check_gql_errors(&res, "workItemUpdate")
}

async fn get_work_item_labels_gql(
    gl_url: &str,
    token: &str,
    work_item_id: &str
) -> Result<Vec<String>, String> {
    let query = r#"
        query($id: WorkItemID!) {
          workItem(id: $id) {
            widgets {
              ... on WorkItemWidgetLabels {
                labels {
                  nodes {
                    title
                  }
                }
              }
            }
          }
        }
    "#;
    
    let res: serde_json::Value = query_gitlab(gl_url, token, query, serde_json::json!({ "id": work_item_id })).await?;
    
    let mut labels = Vec::new();
    if let Some(widgets) = res.pointer("/workItem/widgets").and_then(|w| w.as_array()) {
        for widget in widgets {
            if widget["type"] == "LABELS" {
                if let Some(nodes) = widget.pointer("/labels/nodes").and_then(|n| n.as_array()) {
                    for node in nodes {
                        if let Some(title) = node["title"].as_str() {
                            labels.push(title.to_string());
                        }
                    }
                }
            }
        }
    }
    Ok(labels)
}

pub async fn create_note_gql(
    gl_url: &str,
    token: &str,
    noteable_id: &str,
    body: &str
) -> Result<(), String> {
    let query = r#"
        mutation($id: NoteableID!, $body: String!) {
          createNote(input: { noteableId: $id, body: $body }) {
            errors
          }
        }
    "#;
    let variables = serde_json::json!({ "id": noteable_id, "body": body });
    let res: serde_json::Value = query_gitlab(gl_url, token, query, variables).await?;
    check_gql_errors(&res, "createNote")
}

pub async fn update_note_gql(
    gl_url: &str,
    token: &str,
    note_id: &str,
    body: &str
) -> Result<(), String> {
    let query = r#"
        mutation($id: NoteID!, $body: String!) {
          updateNote(input: { id: $id, body: $body }) {
            errors
          }
        }
    "#;
    let variables = serde_json::json!({ "id": note_id, "body": body });
    let res: serde_json::Value = query_gitlab(gl_url, token, query, variables).await?;
    check_gql_errors(&res, "updateNote")
}

pub async fn delete_note_gql(
    gl_url: &str,
    token: &str,
    note_id: &str
) -> Result<(), String> {
    let query = r#"
        mutation($id: NoteID!) {
          destroyNote(input: { id: $id }) {
            errors
          }
        }
    "#;
    let variables = serde_json::json!({ "id": note_id });
    let res: serde_json::Value = query_gitlab(gl_url, token, query, variables).await?;
    check_gql_errors(&res, "destroyNote")
}

pub async fn create_child_task_rest(
    gl_url: &str,
    token: &str,
    project_path: &str,
    title: &str,
    parent_id: &str,
    milestone_id: Option<String>,
    labels: Vec<String>,
    assignee_ids: Vec<String>,
    estimate_seconds: i64
) -> Result<(), String> {
    let type_query = r#"
        query($projectPath: ID!) {
          project(fullPath: $projectPath) {
            workItemTypes {
              nodes {
                id
                name
              }
            }
          }
        }
    "#;

    let type_res: serde_json::Value = query_gitlab(gl_url, token, type_query, serde_json::json!({"projectPath": project_path})).await?;
    let task_type_id = type_res.pointer("/project/workItemTypes/nodes")
        .and_then(|n| n.as_array())
        .and_then(|arr| arr.iter().find(|t| t["name"] == "Task"))
        .and_then(|t| t["id"].as_str())
        .ok_or("Could not find Work Item Type 'Task'")?;

    let mutation = r#"
        mutation($projectPath: ID!, $title: String!, $typeId: WorkItemTypeID!, $parentId: WorkItemID!, $labels: [String!], $estimate: String!, $milestoneId: MilestoneID, $assigneeIds: [UserID!]) {
          workItemCreate(input: {
            namespacePath: $projectPath,
            title: $title,
            workItemTypeId: $typeId,
            hierarchyWidget: { parentId: $parentId },
            labelsWidget: { addLabelNames: $labels },
            timeTrackingWidget: { timeEstimate: $estimate },
            milestoneWidget: { milestoneId: $milestoneId },
            assigneesWidget: { assigneeIds: $assigneeIds }
          }) {
            workItem { id }
            errors
          }
        }
    "#;

    let variables = serde_json::json!({
        "projectPath": project_path,
        "title": title,
        "typeId": task_type_id,
        "parentId": parent_id,
        "labels": labels,
        "estimate": format!("{}s", estimate_seconds),
        "milestoneId": milestone_id,
        "assigneeIds": assignee_ids
    });

    let res: serde_json::Value = query_gitlab(gl_url, token, mutation, variables).await?;
    
    if let Some(errors) = res.pointer("/workItemCreate/errors").and_then(|e| e.as_array()) {
        if !errors.is_empty() {
            return Err(errors[0].as_str().unwrap_or("Unknown error").into());
        }
    }
    Ok(())
}

