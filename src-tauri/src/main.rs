// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod gitlab;

use serde::{Deserialize, Serialize};
use std::fs;
use gitlab::{query_gitlab, WorkItemNode, WorkItemWidget};
use chrono::Datelike;
use tauri::{Manager, Emitter};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GitLabConfig {
    pub url: String,
    pub token: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TargetConfig {
    pub groups: Vec<String>,
    pub projects: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProductMatch {
    pub labels: Vec<String>,
    pub project_ids: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProductConfig {
    pub id: String,
    pub name: String,
    pub color: String,
    pub match_rules: ProductMatch,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserConfig {
    pub username: String,
    pub display_name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct ExcludeDays {
    pub global_holidays: Vec<String>,
    pub user_leave: std::collections::HashMap<String, Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BoardConfig {
    pub columns: Vec<String>,
    pub excluded_child_labels: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub gitlab: GitLabConfig,
    pub targets: TargetConfig,
    pub products: Vec<ProductConfig>,
    pub users: Vec<UserConfig>,
    pub board: BoardConfig,
    pub network_settings_path: String,
    pub average_capacity_months: u32,
    pub hours_per_day: u32,
}

fn load_exclude_days(app_handle: &tauri::AppHandle, path: &str) -> ExcludeDays {
    if path.is_empty() {
        return ExcludeDays::default();
    }

    match fs::read_to_string(path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_else(|_| {
            let _ = app_handle.emit("app-notification", serde_json::json!({
                "level": "warning",
                "message": "Failed to parse exclude_days.json. Falling back to weekends only."
            }));
            ExcludeDays::default()
        }),
        Err(_) => {
            let _ = app_handle.emit("app-notification", serde_json::json!({
                "level": "warning",
                "message": format!("Could not reach network path: {}. Falling back to weekends only.", path)
            }));
            ExcludeDays::default()
        }
    }
}

#[tauri::command]
fn get_config(app_handle: tauri::AppHandle) -> Result<AppConfig, String> {
    let config_path = app_handle
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?
        .join("config.json");

    if !config_path.exists() {
        let template = AppConfig {
            gitlab: GitLabConfig {
                url: "http://your-gitlab-ce.local".into(),
                token: "".into(),
            },
            targets: TargetConfig {
                groups: vec![],
                projects: vec![],
            },
            products: vec![],
            users: vec![],
            board: BoardConfig {
                columns: vec!["Status::To Do".into(), "Status::Doing".into(), "Status::Done".into()],
                excluded_child_labels: vec!["Status::Doing".into(), "Status::Done".into()],
            },
            network_settings_path: "".into(),
            average_capacity_months: 3,
            hours_per_day: 8,
        };
        fs::create_dir_all(config_path.parent().unwrap()).map_err(|e| e.to_string())?;
        fs::write(&config_path, serde_json::to_string_pretty(&template).unwrap()).map_err(|e| e.to_string())?;
        return Ok(template);
    }

    let content = fs::read_to_string(config_path).map_err(|e| e.to_string())?;
    serde_json::from_str(&content).map_err(|e| e.to_string())
}

#[tauri::command]
fn save_config(app_handle: tauri::AppHandle, config: AppConfig) -> Result<(), String> {
    let config_path = app_handle
        .path()
        .app_config_dir()
        .map_err(|e| e.to_string())?
        .join("config.json");

    fs::create_dir_all(config_path.parent().unwrap()).map_err(|e| e.to_string())?;
    fs::write(&config_path, serde_json::to_string_pretty(&config).unwrap()).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn calculate_capacities(app_handle: tauri::AppHandle) -> Result<std::collections::HashMap<String, f64>, String> {
    let config = get_config(app_handle.clone()).map_err(|e| e.to_string())?;
    let exclude_days = load_exclude_days(&app_handle, &config.network_settings_path);
    
    let end_date = chrono::Local::now().naive_local().date();
    let start_date = end_date - chrono::Duration::days(config.average_capacity_months as i64 * 30);
    
    let mut user_seconds: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    for u in &config.users {
        user_seconds.insert(u.username.clone(), 0);
    }

    let start_date_str = format!("{}T00:00:00Z", start_date);
    let end_date_str = format!("{}T23:59:59Z", end_date);

    let mut combined_paths = config.targets.projects.clone();
    combined_paths.extend(config.targets.groups.clone());

    for path in combined_paths {
        let variables = serde_json::json!({
            "fullPath": path,
            "startDate": start_date_str,
            "endDate": end_date_str
        });

        let gql_res: serde_json::Value = query_gitlab(
            &config.gitlab.url,
            &config.gitlab.token,
            gitlab::TIMELOGS_QUERY,
            variables
        ).await.unwrap_or_else(|e| {
            eprintln!("Warning: Failed to query timelogs for {}: {}", path, e);
            serde_json::json!({ "workspace": { "timelogs": { "nodes": [] } } })
        });

        let timelog_nodes = gql_res.pointer("/namespace/timelogs/nodes").and_then(|n| n.as_array()).cloned().unwrap_or_default();

        for node_val in timelog_nodes {
            if let Ok(node) = serde_json::from_value::<gitlab::TimelogNode>(node_val) {
                if let Some(user) = node.user {
                    if user_seconds.contains_key(&user.username) {
                        *user_seconds.get_mut(&user.username).unwrap() += node.time_spent;
                    }
                }
            }
        }
    }

    let mut capacities = std::collections::HashMap::new();
    for user in config.users {
        let total_seconds = user_seconds.get(&user.username).cloned().unwrap_or(0);
        
        let mut working_days = 0;
        let mut curr = start_date;
        while curr <= end_date {
            let weekday = curr.weekday();
            let is_weekend = weekday == chrono::Weekday::Sat || weekday == chrono::Weekday::Sun;
            let date_str = curr.to_string();
            let is_holiday = exclude_days.global_holidays.contains(&date_str);
            let is_leave = exclude_days.user_leave.get(&user.username).map_or(false, |l| l.contains(&date_str));

            if !is_weekend && !is_holiday && !is_leave {
                working_days += 1;
            }
            curr = curr + chrono::Duration::days(1);
        }

        let capacity = if working_days > 0 {
            (total_seconds as f64 / 3600.0) / working_days as f64
        } else {
            1.0
        };
        
        capacities.insert(user.username, capacity);
    }

    Ok(capacities)
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FlattenedWorkItem {
    pub id: String,
    pub iid: u64,
    pub title: String,
    pub description: Option<String>,
    pub state: String,
    pub web_url: String,
    pub assignees: Vec<gitlab::AssigneeNode>,
    pub time_stats: TimeStats,
    pub parent_iid: Option<u64>,
    pub labels: Vec<String>,
    pub milestone: Option<gitlab::MilestoneNode>,
    pub due_date: Option<String>,
    pub product: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct TimeStats {
    pub time_estimate: i64,
    pub total_time_spent: i64,
}

#[tauri::command]
fn get_working_days(app_handle: tauri::AppHandle, start_date: String, end_date: String) -> Result<Vec<String>, String> {
    let config = get_config(app_handle.clone()).map_err(|e| e.to_string())?;
    let exclude_days = load_exclude_days(&app_handle, &config.network_settings_path);
    
    let start = chrono::NaiveDate::parse_from_str(&start_date, "%Y-%m-%d").map_err(|e| e.to_string())?;
    let end = chrono::NaiveDate::parse_from_str(&end_date, "%Y-%m-%d").map_err(|e| e.to_string())?;
    
    let mut working_days = Vec::new();
    let mut curr = start;
    while curr <= end {
        let weekday = curr.weekday();
        let is_weekend = weekday == chrono::Weekday::Sat || weekday == chrono::Weekday::Sun;
        let date_str = curr.to_string();
        let is_holiday = exclude_days.global_holidays.contains(&date_str);
        
        if !is_weekend && !is_holiday {
            working_days.push(date_str);
        }
        curr = curr + chrono::Duration::days(1);
    }
    Ok(working_days)
}

#[tauri::command]
async fn fetch_gitlab_data(app_handle: tauri::AppHandle, force: bool) -> Result<Vec<FlattenedWorkItem>, String> {
    let cache_path = app_handle
        .path()
        .app_cache_dir()
        .map_err(|e| e.to_string())?
        .join("cache.json");

    if !force && cache_path.exists() {
        if let Ok(content) = fs::read_to_string(&cache_path) {
            if let Ok(cached_items) = serde_json::from_str::<Vec<FlattenedWorkItem>>(&content) {
                return Ok(cached_items);
            }
        }
    }

    let config = get_config(app_handle.clone()).map_err(|e| e.to_string())?;
    let mut all_items = Vec::new();

    for project_path in config.targets.projects {
        let variables = serde_json::json!({
            "namespacePath": project_path
        });
        
        let response: serde_json::Value = query_gitlab(
            &config.gitlab.url,
            &config.gitlab.token,
            gitlab::WORK_ITEMS_QUERY,
            variables
        ).await?;

        let work_item_nodes = response.pointer("/namespace/workItems/nodes").and_then(|n| n.as_array()).cloned().unwrap_or_default();

        for node_val in work_item_nodes {
            let node: WorkItemNode = serde_json::from_value(node_val).map_err(|e| e.to_string())?;
            
            let mut flattened = FlattenedWorkItem {
                id: node.id,
                iid: node.iid.parse().unwrap_or(0),
                title: node.title,
                description: node.description,
                state: node.state,
                web_url: node.web_url,
                assignees: vec![],
                time_stats: TimeStats { time_estimate: 0, total_time_spent: 0 },
                parent_iid: None,
                labels: vec![],
                milestone: None,
                due_date: None,
                product: None,
            };

                let mut has_labels = false;
                for widget in node.widgets {
                    match widget {
                        WorkItemWidget::Assignees { assignees } => {
                            flattened.assignees = assignees.nodes;
                        },
                        WorkItemWidget::Hierarchy { parent, .. } => {
                            if let Some(p) = parent {
                                flattened.parent_iid = p.iid.parse().ok();
                            }
                        },
                        WorkItemWidget::TimeTracking { time_estimate, total_time_spent } => {
                            flattened.time_stats.time_estimate = time_estimate;
                            flattened.time_stats.total_time_spent = total_time_spent;
                        },
                        WorkItemWidget::Milestone { milestone } => {
                            flattened.milestone = milestone;
                        },
                        _ => {}
                    }
                }
                
                // Check if labels were found in widgets (if we had a Labels widget match)
                // Actually the current GraphQL query doesn't have a Labels widget, 
                // so we rely on REST anyway. But I'll set it here if we add it later.
                if !flattened.labels.is_empty() {
                    has_labels = true;
                }

                if !has_labels {
                    if let Ok(labels) = gitlab::get_issue_labels_rest(
                        &config.gitlab.url,
                        &config.gitlab.token,
                        &project_path,
                        flattened.iid
                    ).await {
                        flattened.labels = labels;
                    }
                }

                for product in &config.products {
                    let project_match = product.match_rules.project_ids.iter().any(|p| p == &project_path);
                    let label_match = product.match_rules.labels.iter().any(|l| flattened.labels.contains(l));
                    if project_match || label_match {
                        flattened.product = Some(product.name.clone());
                        break;
                    }
                }

                all_items.push(flattened);
            }
        }

    fs::create_dir_all(cache_path.parent().unwrap()).map_err(|e| e.to_string())?;
    fs::write(&cache_path, serde_json::to_string_pretty(&all_items).unwrap()).map_err(|e| e.to_string())?;

    Ok(all_items)
}

#[tauri::command]
async fn create_milestone(
    app_handle: tauri::AppHandle, 
    project_path: String, 
    title: String, 
    due_date: Option<String>
) -> Result<(), String> {
    let config = get_config(app_handle).map_err(|e| e.to_string())?;
    gitlab::create_milestone_rest(
        &config.gitlab.url, 
        &config.gitlab.token, 
        &project_path, 
        &title, 
        due_date.as_deref()
    ).await
}

#[tauri::command]
async fn create_work_item(
    app_handle: tauri::AppHandle,
    project_path: String,
    title: String,
    milestone_id: Option<u64>,
    due_date: Option<String>,
    labels: Vec<String>,
    assignee_ids: Vec<u64>
) -> Result<(), String> {
    let config = get_config(app_handle).map_err(|e| e.to_string())?;
    gitlab::create_issue_rest(
        &config.gitlab.url,
        &config.gitlab.token,
        &project_path,
        &title,
        milestone_id,
        due_date.as_deref(),
        labels,
        assignee_ids
    ).await
}

#[tauri::command]
async fn create_child_work_item(
    app_handle: tauri::AppHandle,
    project_path: String,
    title: String,
    parent_id: String,
    milestone_id: Option<String>,
    labels: Vec<String>,
    assignee_ids: Vec<String>,
    estimate_seconds: i64
) -> Result<(), String> {
    let config = get_config(app_handle).map_err(|e| e.to_string())?;
    gitlab::create_child_task_rest(
        &config.gitlab.url,
        &config.gitlab.token,
        &project_path,
        &title,
        &parent_id,
        milestone_id,
        labels,
        assignee_ids,
        estimate_seconds
    ).await
}

#[tauri::command]
async fn update_work_item_estimate(
    app_handle: tauri::AppHandle,
    work_item_id: String,
    estimate_seconds: i64
) -> Result<(), String> {
    let config = get_config(app_handle).map_err(|e| e.to_string())?;
    gitlab::update_work_item_estimate_gql(
        &config.gitlab.url,
        &config.gitlab.token,
        &work_item_id,
        estimate_seconds
    ).await
}

fn main() {
  tauri::Builder::default()
    .plugin(tauri_plugin_opener::init())
    .invoke_handler(tauri::generate_handler![
        get_config,
        save_config,
        fetch_gitlab_data,
        calculate_capacities,
        get_working_days,
        create_milestone,
        create_work_item,
        create_child_work_item,
        update_work_item_estimate
    ])
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
