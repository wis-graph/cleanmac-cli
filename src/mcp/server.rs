use crate::config::Config;
use crate::metadata;
use crate::output::{
    CategoryScanResult as JsonCategoryScanResult, ScanItem, ScanResult as JsonScanResult,
};
use crate::plugin::{PluginRegistry, ScanConfig};
use rmcp::{
    ErrorData as McpError, ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    schemars::{self, JsonSchema},
    tool, tool_handler, tool_router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ScanInput {
    #[serde(default)]
    pub categories: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ScanCategoryInput {
    pub category: String,
    #[serde(default)]
    pub collect_metadata: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AnalyzeDiskInput {
    pub path: String,
    #[serde(default = "default_depth")]
    pub depth: usize,
}

fn default_depth() -> usize {
    2
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PreviewCleanInput {
    pub categories: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ScanOutput {
    pub categories: Vec<CategoryOutput>,
    pub total_size_bytes: u64,
    pub total_items: usize,
    pub cli_command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CategoryOutput {
    pub id: String,
    pub name: String,
    pub size_bytes: u64,
    pub item_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiskAnalysisOutput {
    pub path: String,
    pub total_size_bytes: u64,
    pub children: Vec<DiskChildOutput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct DiskChildOutput {
    pub name: String,
    pub size_bytes: u64,
    pub percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AppOutput {
    pub name: String,
    pub path: String,
    pub size_bytes: u64,
    pub bundle_id: Option<String>,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PreviewOutput {
    pub items: Vec<PreviewItemOutput>,
    pub total_size_bytes: u64,
    pub cli_command: String,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PreviewItemOutput {
    pub path: String,
    pub size_bytes: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_used: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HistoryOutput {
    pub entries: Vec<HistoryEntryOutput>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HistoryEntryOutput {
    pub timestamp: String,
    pub action: String,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct CleanMacServer {
    tool_router: ToolRouter<Self>,
}

#[tool_router]
impl CleanMacServer {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Scan the system for cleanable items")]
    pub async fn scan_system(
        &self,
        input: Parameters<ScanInput>,
    ) -> Result<CallToolResult, McpError> {
        let input = input.0;
        let config = Config::load().map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let registry = PluginRegistry::default();
        let scan_config = ScanConfig {
            min_size: config.scan.min_size_bytes,
            max_depth: config.scan.max_depth,
            excluded_paths: config.scan.excluded_paths.iter().map(|s| s.into()).collect(),
            progress_callback: None,
            item_callback: None,
        };

        let report = registry
            .scan_all(&scan_config)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let filter_categories = input.categories.unwrap_or_default();

        let categories: Vec<CategoryOutput> = report
            .categories
            .iter()
            .filter(|cat| {
                filter_categories.is_empty()
                    || filter_categories
                        .iter()
                        .any(|c| cat.scanner_id.contains(&c.to_lowercase()))
            })
            .map(|cat| CategoryOutput {
                id: cat.scanner_id.clone(),
                name: cat.name.clone(),
                size_bytes: cat.total_size(),
                item_count: cat.items.len(),
            })
            .collect();

        let total_size: u64 = categories.iter().map(|c| c.size_bytes).sum();
        let total_items: usize = categories.iter().map(|c| c.item_count).sum();

        let output = ScanOutput {
            categories,
            total_size_bytes: total_size,
            total_items,
            cli_command: "cleanmac scan --format json".to_string(),
        };

        Ok(CallToolResult::success(vec![Content::json(output)?]))
    }

    #[tool(description = "Scan a specific category for cleanable items with metadata")]
    pub async fn scan_category(
        &self,
        input: Parameters<ScanCategoryInput>,
    ) -> Result<CallToolResult, McpError> {
        let input = input.0;
        let config = Config::load().map_err(|e| McpError::internal_error(e.to_string(), None))?;
        let start = std::time::Instant::now();

        let registry = PluginRegistry::default();
        let scan_config = ScanConfig {
            min_size: config.scan.min_size_bytes,
            max_depth: config.scan.max_depth,
            excluded_paths: config.scan.excluded_paths.iter().map(|s| s.into()).collect(),
            progress_callback: None,
            item_callback: None,
        };

        let report = registry
            .scan_all(&scan_config)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let collect_metadata = input.collect_metadata.unwrap_or(false);
        let categories: Vec<JsonCategoryScanResult> = report
            .categories
            .iter()
            .filter(|cat| cat.scanner_id.contains(&input.category.to_lowercase()))
            .map(|cat_result| {
                let items: Vec<ScanItem> = cat_result
                    .items
                    .iter()
                    .map(|item| {
                        let (last_used, use_count) = if collect_metadata {
                            match metadata::get_file_metadata(&item.path) {
                                Some(meta) => (meta.last_used, meta.use_count),
                                None => (None, None),
                            }
                        } else {
                            (None, None)
                        };

                        ScanItem {
                            path: item.path.clone(),
                            size_bytes: item.size,
                            modified: item.last_modified.unwrap_or_else(chrono::Utc::now),
                            last_used,
                            use_count,
                        }
                    })
                    .collect();

                JsonCategoryScanResult {
                    id: cat_result.scanner_id.clone(),
                    name: cat_result.name.clone(),
                    description: String::new(),
                    size_bytes: cat_result.total_size(),
                    item_count: items.len(),
                    items,
                }
            })
            .collect();

        let output = JsonScanResult::new(categories, start.elapsed().as_millis() as u64);
        Ok(CallToolResult::success(vec![Content::json(output)?]))
    }

    #[tool(description = "Analyze disk usage for a given path")]
    pub async fn analyze_disk(
        &self,
        input: Parameters<AnalyzeDiskInput>,
    ) -> Result<CallToolResult, McpError> {
        let input = input.0;
        use std::path::Path;
        use walkdir::WalkDir;

        let path = Path::new(&input.path);
        if !path.exists() {
            return Err(McpError::invalid_params(
                format!("Path does not exist: {}", input.path),
                None,
            ));
        }

        let mut children: std::collections::HashMap<String, u64> =
            std::collections::HashMap::new();

        for entry in WalkDir::new(path)
            .min_depth(1)
            .max_depth(input.depth)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                if let Ok(metadata) = entry.metadata() {
                    let depth = entry.depth();
                    if depth <= input.depth {
                        let relative = entry.path().strip_prefix(path).unwrap_or(entry.path());
                        let first_component = relative
                            .components()
                            .next()
                            .map(|c| c.as_os_str().to_string_lossy().to_string())
                            .unwrap_or_else(|| "unknown".to_string());

                        *children.entry(first_component).or_insert(0) += metadata.len();
                    }
                }
            }
        }

        let total_size: u64 = children.values().sum();

        let mut children_output: Vec<DiskChildOutput> = children
            .into_iter()
            .map(|(name, size)| DiskChildOutput {
                name,
                size_bytes: size,
                percent: if total_size > 0 {
                    (size as f64 / total_size as f64) * 100.0
                } else {
                    0.0
                },
            })
            .collect();

        children_output.sort_by(|a, b| b.size_bytes.cmp(&a.size_bytes));

        let output = DiskAnalysisOutput {
            path: input.path,
            total_size_bytes: total_size,
            children: children_output.into_iter().take(20).collect(),
        };

        Ok(CallToolResult::success(vec![Content::json(output)?]))
    }

    #[tool(description = "List installed applications")]
    pub async fn list_apps(&self) -> Result<CallToolResult, McpError> {
        use crate::uninstaller::AppDetector;

        let detector = AppDetector::new();
        let apps = detector.list_all();

        let output: Vec<AppOutput> = apps
            .iter()
            .map(|app| AppOutput {
                name: app.name().to_string(),
                path: app.path.to_string_lossy().to_string(),
                size_bytes: app.size(),
                bundle_id: app.info().map(|i| i.bundle_id.clone()),
                version: app.info().map(|i| i.version.clone()),
            })
            .collect();

        Ok(CallToolResult::success(vec![Content::json(output)?]))
    }

    #[tool(description = "Get deletion history")]
    pub async fn get_history(&self) -> Result<CallToolResult, McpError> {
        use crate::history::HistoryLogger;

        let logger = HistoryLogger::new();
        let entries = logger
            .read_history(Some(50))
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let output = HistoryOutput {
            entries: entries
                .into_iter()
                .map(|e| HistoryEntryOutput {
                    timestamp: e.timestamp.to_rfc3339(),
                    action: e.action,
                    path: e.path.to_string_lossy().to_string(),
                    size: e.size,
                })
                .collect(),
        };

        Ok(CallToolResult::success(vec![Content::json(output)?]))
    }

    #[tool(description = "Preview what would be cleaned (dry-run) and get CLI command to execute")]
    pub async fn preview_clean(
        &self,
        input: Parameters<PreviewCleanInput>,
    ) -> Result<CallToolResult, McpError> {
        let input = input.0;
        let config = Config::load().map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let registry = PluginRegistry::default();
        let scan_config = ScanConfig {
            min_size: config.scan.min_size_bytes,
            max_depth: config.scan.max_depth,
            excluded_paths: config.scan.excluded_paths.iter().map(|s| s.into()).collect(),
            progress_callback: None,
            item_callback: None,
        };

        let report = registry
            .scan_all(&scan_config)
            .map_err(|e| McpError::internal_error(e.to_string(), None))?;

        let mut items: Vec<PreviewItemOutput> = Vec::new();
        let mut total_size: u64 = 0;
        let mut warnings: HashSet<String> = HashSet::new();

        for cat in &report.categories {
            if input
                .categories
                .iter()
                .any(|c| cat.scanner_id.contains(&c.to_lowercase()))
            {
                for item in &cat.items {
                    let last_used = metadata::get_file_metadata(&item.path)
                        .and_then(|m| m.last_used.map(|d| d.to_rfc3339()));

                    items.push(PreviewItemOutput {
                        path: item.path.to_string_lossy().to_string(),
                        size_bytes: item.size,
                        last_used,
                    });
                    total_size += item.size;

                    if cat.scanner_id.contains("browser") {
                        warnings.insert(
                            "Browser cache deletion may require re-login to websites".to_string(),
                        );
                    }
                }
            }
        }

        let category_list = input.categories.join(",");
        let cli_command = format!("cleanmac apply --category {} --yes", category_list);

        let output = PreviewOutput {
            items: items.into_iter().take(100).collect(),
            total_size_bytes: total_size,
            cli_command,
            warnings: warnings.into_iter().collect(),
        };

        Ok(CallToolResult::success(vec![Content::json(output)?]))
    }
}

#[tool_handler]
impl ServerHandler for CleanMacServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

impl Default for CleanMacServer {
    fn default() -> Self {
        Self::new()
    }
}

pub async fn run_mcp_server() -> anyhow::Result<()> {
    use rmcp::transport::stdio;

    let server = CleanMacServer::new();
    let service = server.serve(stdio()).await?;

    service.waiting().await?;

    Ok(())
}
