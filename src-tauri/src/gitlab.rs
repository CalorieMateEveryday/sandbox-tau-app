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

pub async fn update_work_item_estimate_gql(
    gl_url: &str,
    token: &str,
    work_item_id: &str,
    estimate_seconds: i64
) -> Result<(), String> {
    let query = r#"
        mutation($id: WorkItemID!, $estimate: String!) {
          workItemUpdate(input: { id: $id, timeTrackingWidget: { timeEstimate: $estimate } }) {
            errors
          }
        }
    "#;

    let estimate_str = format!("{}s", estimate_seconds);
    let variables = serde_json::json!({
        "id": work_item_id,
        "estimate": estimate_str
    });

    let res: serde_json::Value = query_gitlab(gl_url, token, query, variables).await?;
    if let Some(errors) = res.pointer("/workItemUpdate/errors").and_then(|e| e.as_array()) {
        if !errors.is_empty() {
            return Err(errors[0].as_str().unwrap_or("Unknown error").into());
        }
    }
    Ok(())
}

pub async fn update_work_item_description_gql(
    gl_url: &str,
    token: &str,
    work_item_id: &str,
    description: &str
) -> Result<(), String> {
    let query = r#"
        mutation($id: WorkItemID!, $desc: String!) {
          workItemUpdate(input: { id: $id, descriptionWidget: { description: $desc } }) {
            errors
          }
        }
    "#;

    let variables = serde_json::json!({
        "id": work_item_id,
        "desc": description
    });

    let res: serde_json::Value = query_gitlab(gl_url, token, query, variables).await?;
    if let Some(errors) = res.pointer("/workItemUpdate/errors").and_then(|e| e.as_array()) {
        if !errors.is_empty() {
            return Err(errors[0].as_str().unwrap_or("Unknown error").into());
        }
    }
    Ok(())
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

