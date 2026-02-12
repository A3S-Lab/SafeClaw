import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  ResizablePanelGroup,
  ResizablePanel,
  ResizableHandle,
} from "@/components/ui/resizable";
import { cn } from "@/lib/utils";
import { BUILTIN_PERSONAS } from "@/lib/builtin-personas";
import NiceAvatar, { genConfig } from "react-nice-avatar";
import { useCallback, useMemo, useState } from "react";
import {
  ArrowLeft,
  Boxes,
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  Circle,
  Clock,
  File,
  FileCode2,
  FileJson,
  FileText,
  Folder,
  FolderOpen,
  GitBranch,
  GitCommit,
  GitFork,
  GitPullRequest,
  Loader2,
  MoreHorizontal,
  Pause,
  Play,
  Plus,
  Rocket,
  Search,
  Server,
  Star,
  Tag,
} from "lucide-react";
import CodeEditor from "@/components/custom/code-editor";

// =============================================================================
// Types
// =============================================================================

type ProjectCategory = "all" | "enterprise" | "agent";
type ProjectStatus = "active" | "stable" | "archived" | "developing";
type AgentDevStatus = "planning" | "coding" | "testing" | "reviewing" | "deployed" | "paused";

interface GitInfo {
  url: string;
  branch: string;
  lastCommit: string;
  lastCommitTime: number;
  commitCount: number;
  openPRs?: number;
  stars?: number;
}

interface ProjectItem {
  id: string;
  name: string;
  description: string;
  category: "enterprise" | "agent";
  status: ProjectStatus;
  language: string;
  languages?: string[];
  git: GitInfo;
  tags?: string[];
  /** Enterprise project fields */
  team?: string;
  version?: string;
  /** Agent project fields */
  agentId?: string;
  devGoal?: string;
  devStatus?: AgentDevStatus;
  devProgress?: number;
  milestones?: Milestone[];
}

interface Milestone {
  id: string;
  title: string;
  done: boolean;
}

interface FileNode {
  name: string;
  type: "file" | "folder";
  language?: string;
  children?: FileNode[];
  content?: string;
}

// =============================================================================
// Constants
// =============================================================================

const CATEGORIES: { key: ProjectCategory; label: string }[] = [
  { key: "all", label: "全部项目" },
  { key: "enterprise", label: "企业项目" },
  { key: "agent", label: "智能体项目" },
];

const STATUS_CONFIG: Record<ProjectStatus, { label: string; color: string; icon: typeof Circle }> = {
  active: { label: "活跃", color: "text-primary", icon: Circle },
  stable: { label: "稳定", color: "text-primary", icon: CheckCircle2 },
  archived: { label: "归档", color: "text-muted-foreground", icon: Pause },
  developing: { label: "开发中", color: "text-orange-500", icon: Loader2 },
};

const AGENT_DEV_STATUS_CONFIG: Record<AgentDevStatus, { label: string; color: string }> = {
  planning: { label: "规划中", color: "bg-purple-100 text-purple-700 dark:bg-purple-900/30 dark:text-purple-400" },
  coding: { label: "编码中", color: "bg-primary/10 text-primary" },
  testing: { label: "测试中", color: "bg-yellow-100 text-yellow-700 dark:bg-yellow-900/30 dark:text-yellow-400" },
  reviewing: { label: "审核中", color: "bg-orange-100 text-orange-700 dark:bg-orange-900/30 dark:text-orange-400" },
  deployed: { label: "已部署", color: "bg-primary/10 text-primary" },
  paused: { label: "已暂停", color: "bg-muted text-muted-foreground" },
};

const LANG_COLORS: Record<string, string> = {
  Rust: "bg-orange-500",
  TypeScript: "bg-blue-500",
  Python: "bg-yellow-500",
  Go: "bg-cyan-500",
  Java: "bg-red-500",
  SQL: "bg-green-500",
  Shell: "bg-gray-500",
  Markdown: "bg-gray-400",
};

// =============================================================================
// File extension to Monaco language mapping
// =============================================================================

function extToLang(name: string): string {
  const ext = name.split(".").pop()?.toLowerCase() || "";
  const map: Record<string, string> = {
    rs: "rust", ts: "typescript", tsx: "typescript", js: "javascript", jsx: "javascript",
    py: "python", go: "go", java: "java", sql: "sql", sh: "shell", bash: "shell",
    json: "json", toml: "toml", yaml: "yaml", yml: "yaml", md: "markdown",
    css: "css", scss: "scss", html: "html", xml: "xml", dockerfile: "dockerfile",
    tf: "hcl", hcl: "hcl", proto: "protobuf", graphql: "graphql",
  };
  return map[ext] || "plaintext";
}

function fileIcon(name: string) {
  const ext = name.split(".").pop()?.toLowerCase() || "";
  if (["rs", "ts", "tsx", "js", "jsx", "py", "go", "java", "sh", "bash"].includes(ext))
    return <FileCode2 className="size-4 text-primary/70 shrink-0" />;
  if (["json", "toml", "yaml", "yml"].includes(ext))
    return <FileJson className="size-4 text-yellow-600 dark:text-yellow-400 shrink-0" />;
  if (["md", "txt", "rst"].includes(ext))
    return <FileText className="size-4 text-muted-foreground shrink-0" />;
  return <File className="size-4 text-muted-foreground shrink-0" />;
}

// =============================================================================
// Mock data
// =============================================================================

const now = Date.now();

const MOCK_PROJECTS: ProjectItem[] = [
  // === Enterprise projects ===
  {
    id: "proj-1",
    name: "safeclaw-gateway",
    description: "跨境支付核心网关服务，处理路由分发、限流熔断、协议转换与审计日志",
    category: "enterprise",
    status: "active",
    language: "Rust",
    languages: ["Rust", "SQL"],
    git: {
      url: "https://git.internal.com/fintech/safeclaw-gateway",
      branch: "main",
      lastCommit: "feat: add SWIFT gpi tracking webhook",
      lastCommitTime: now - 2 * 3600_000,
      commitCount: 1847,
      openPRs: 3,
      stars: 24,
    },
    team: "基础架构组",
    version: "v3.12.0",
    tags: ["核心服务", "支付", "网关"],
  },
  {
    id: "proj-2",
    name: "safeclaw-risk-engine",
    description: "实时风控引擎，支持规则引擎与 ML 模型混合决策，亚毫秒级响应",
    category: "enterprise",
    status: "active",
    language: "Rust",
    languages: ["Rust", "Python"],
    git: {
      url: "https://git.internal.com/fintech/safeclaw-risk-engine",
      branch: "main",
      lastCommit: "fix: credit score model v3 threshold calibration",
      lastCommitTime: now - 6 * 3600_000,
      commitCount: 2103,
      openPRs: 5,
      stars: 31,
    },
    team: "风控组",
    version: "v2.8.1",
    tags: ["风控", "ML", "实时"],
  },
  {
    id: "proj-3",
    name: "safeclaw-ui",
    description: "SafeClaw 桌面客户端，基于 Tauri v2 + React + TypeScript，支持 AI 助手与多智能体协作",
    category: "enterprise",
    status: "active",
    language: "TypeScript",
    languages: ["TypeScript", "Rust"],
    git: {
      url: "https://git.internal.com/fintech/safeclaw-ui",
      branch: "main",
      lastCommit: "feat: add enterprise knowledge base page",
      lastCommitTime: now - 30 * 60_000,
      commitCount: 436,
      openPRs: 2,
      stars: 15,
    },
    team: "前端组",
    version: "v0.9.4",
    tags: ["桌面端", "Tauri", "AI"],
  },
  {
    id: "proj-4",
    name: "safeclaw-data-platform",
    description: "数据平台，包含数据仓库、实时管道、指标计算与数据治理",
    category: "enterprise",
    status: "active",
    language: "Python",
    languages: ["Python", "SQL", "Go"],
    git: {
      url: "https://git.internal.com/fintech/safeclaw-data-platform",
      branch: "main",
      lastCommit: "chore: upgrade ClickHouse client to v0.7.x",
      lastCommitTime: now - 18 * 3600_000,
      commitCount: 967,
      openPRs: 4,
      stars: 18,
    },
    team: "数据组",
    version: "v1.5.0",
    tags: ["数据", "ETL", "ClickHouse"],
  },
  {
    id: "proj-5",
    name: "safeclaw-compliance-api",
    description: "合规服务 API，提供 KYC/AML 校验、制裁名单筛查与监管报告生成",
    category: "enterprise",
    status: "stable",
    language: "Go",
    languages: ["Go", "SQL"],
    git: {
      url: "https://git.internal.com/fintech/safeclaw-compliance-api",
      branch: "main",
      lastCommit: "docs: update FATF travel rule integration guide",
      lastCommitTime: now - 3 * 86400_000,
      commitCount: 583,
      openPRs: 1,
      stars: 12,
    },
    team: "合规组",
    version: "v2.1.3",
    tags: ["合规", "KYC", "AML"],
  },
  {
    id: "proj-6",
    name: "safeclaw-infra",
    description: "基础设施即代码，Kubernetes 集群配置、Terraform 模板与 CI/CD 流水线",
    category: "enterprise",
    status: "active",
    language: "Shell",
    languages: ["Shell", "Go"],
    git: {
      url: "https://git.internal.com/fintech/safeclaw-infra",
      branch: "main",
      lastCommit: "fix: adjust node-2 disk alert threshold to 90%",
      lastCommitTime: now - 12 * 3600_000,
      commitCount: 1245,
      openPRs: 2,
      stars: 8,
    },
    team: "SRE 组",
    version: "v4.0.0",
    tags: ["K8s", "Terraform", "CI/CD"],
  },

  // === Agent-created projects ===
  {
    id: "agent-1",
    name: "factor-lab",
    description: "多因子研究实验平台，支持因子挖掘、回测与组合优化",
    category: "agent",
    status: "developing",
    language: "Python",
    languages: ["Python", "SQL"],
    git: {
      url: "https://git.internal.com/agents/factor-lab",
      branch: "develop",
      lastCommit: "feat: add momentum factor with Fama-French adjustment",
      lastCommitTime: now - 4 * 3600_000,
      commitCount: 89,
      openPRs: 1,
    },
    agentId: "quant-researcher",
    devGoal: "构建完整的多因子研究框架，支持 alpha 因子挖掘、IC 分析、组合优化与实盘回测",
    devStatus: "coding",
    devProgress: 62,
    milestones: [
      { id: "m1", title: "因子计算引擎", done: true },
      { id: "m2", title: "IC/IR 分析模块", done: true },
      { id: "m3", title: "回测框架", done: true },
      { id: "m4", title: "组合优化器", done: false },
      { id: "m5", title: "实盘接口对接", done: false },
    ],
    tags: ["量化", "因子", "回测"],
  },
  {
    id: "agent-2",
    name: "compliance-scanner",
    description: "自动化合规扫描工具，定期检查代码库与配置中的合规风险",
    category: "agent",
    status: "developing",
    language: "Rust",
    languages: ["Rust", "Python"],
    git: {
      url: "https://git.internal.com/agents/compliance-scanner",
      branch: "main",
      lastCommit: "feat: add PII detection in log output patterns",
      lastCommitTime: now - 8 * 3600_000,
      commitCount: 156,
      openPRs: 2,
    },
    agentId: "risk-analyst",
    devGoal: "开发企业级合规自动扫描工具，覆盖 PII 泄露检测、密钥扫描、依赖漏洞与监管规则校验",
    devStatus: "testing",
    devProgress: 78,
    milestones: [
      { id: "m1", title: "PII 检测引擎", done: true },
      { id: "m2", title: "密钥/凭证扫描", done: true },
      { id: "m3", title: "依赖漏洞分析", done: true },
      { id: "m4", title: "监管规则 DSL", done: true },
      { id: "m5", title: "CI 集成插件", done: false },
      { id: "m6", title: "报告与告警", done: false },
    ],
    tags: ["合规", "安全", "扫描"],
  },
  {
    id: "agent-3",
    name: "smart-reconciliation",
    description: "AI 智能对账引擎，自动匹配跨系统交易记录并识别差异",
    category: "agent",
    status: "developing",
    language: "Python",
    languages: ["Python", "TypeScript"],
    git: {
      url: "https://git.internal.com/agents/smart-reconciliation",
      branch: "develop",
      lastCommit: "refactor: improve fuzzy matching algorithm for merchant names",
      lastCommitTime: now - 1.5 * 3600_000,
      commitCount: 234,
      openPRs: 3,
    },
    agentId: "financial-analyst",
    devGoal: "构建 AI 驱动的对账系统，自动匹配银行流水与内部交易记录，准确率目标 99.5%",
    devStatus: "reviewing",
    devProgress: 85,
    milestones: [
      { id: "m1", title: "数据源适配器（银行/支付通道）", done: true },
      { id: "m2", title: "精确匹配引擎", done: true },
      { id: "m3", title: "模糊匹配（ML）", done: true },
      { id: "m4", title: "差异分析与归类", done: true },
      { id: "m5", title: "自动调整建议", done: false },
      { id: "m6", title: "审计报告导出", done: false },
    ],
    tags: ["对账", "AI", "财务"],
  },
  {
    id: "agent-4",
    name: "market-monitor",
    description: "实时市场监控仪表板，聚合 Polymarket、外汇、加密货币等多源数据",
    category: "agent",
    status: "developing",
    language: "TypeScript",
    languages: ["TypeScript", "Python"],
    git: {
      url: "https://git.internal.com/agents/market-monitor",
      branch: "develop",
      lastCommit: "feat: add Polymarket WebSocket price streaming",
      lastCommitTime: now - 5 * 3600_000,
      commitCount: 67,
    },
    agentId: "quant-developer",
    devGoal: "构建多源市场数据实时监控平台，集成 Polymarket 预测市场、外汇行情与加密货币数据",
    devStatus: "coding",
    devProgress: 45,
    milestones: [
      { id: "m1", title: "Polymarket API 集成", done: true },
      { id: "m2", title: "外汇数据源接入", done: true },
      { id: "m3", title: "实时价格推送", done: false },
      { id: "m4", title: "告警规则引擎", done: false },
      { id: "m5", title: "可视化仪表板", done: false },
    ],
    tags: ["行情", "Polymarket", "监控"],
  },
  {
    id: "agent-5",
    name: "data-pipeline-quality",
    description: "数据管道质量监控框架，自动检测数据漂移、异常值与完整性问题",
    category: "agent",
    status: "developing",
    language: "Python",
    languages: ["Python", "SQL"],
    git: {
      url: "https://git.internal.com/agents/data-pipeline-quality",
      branch: "main",
      lastCommit: "test: add integration tests for drift detection",
      lastCommitTime: now - 20 * 3600_000,
      commitCount: 112,
    },
    agentId: "data-engineer",
    devGoal: "构建数据管道质量监控框架，自动检测 schema 变更、数据漂移、空值异常与延迟问题",
    devStatus: "deployed",
    devProgress: 100,
    milestones: [
      { id: "m1", title: "Schema 变更检测", done: true },
      { id: "m2", title: "统计漂移检测（KS/PSI）", done: true },
      { id: "m3", title: "空值/完整性校验", done: true },
      { id: "m4", title: "延迟监控", done: true },
      { id: "m5", title: "Slack/飞书告警集成", done: true },
    ],
    tags: ["数据质量", "监控", "管道"],
  },
  {
    id: "agent-6",
    name: "competitor-intel",
    description: "竞品情报收集与分析系统，自动跟踪竞争对手产品动态与舆情",
    category: "agent",
    status: "developing",
    language: "Python",
    languages: ["Python", "TypeScript"],
    git: {
      url: "https://git.internal.com/agents/competitor-intel",
      branch: "develop",
      lastCommit: "feat: add Twitter/X sentiment analysis pipeline",
      lastCommitTime: now - 36 * 3600_000,
      commitCount: 43,
    },
    agentId: "product-manager",
    devGoal: "建立自动化竞品情报体系，持续跟踪 Airwallex、Stripe 等竞品的产品更新、融资动态与社交舆情",
    devStatus: "planning",
    devProgress: 18,
    milestones: [
      { id: "m1", title: "数据源爬虫（官网/社交）", done: true },
      { id: "m2", title: "NLP 摘要与分类", done: false },
      { id: "m3", title: "情报报告自动生成", done: false },
      { id: "m4", title: "变更告警通知", done: false },
    ],
    tags: ["竞品", "情报", "NLP"],
  },
];

// =============================================================================
// Mock file trees per project
// =============================================================================

const MOCK_FILE_TREES: Record<string, FileNode[]> = {
  "proj-1": [
    { name: "Cargo.toml", type: "file", content: `[package]\nname = "safeclaw-gateway"\nversion = "3.12.0"\nedition = "2021"\n\n[dependencies]\ntokio = { version = "1", features = ["full"] }\naxum = "0.7"\ntower = "0.4"\nhyper = "1"\nserde = { version = "1", features = ["derive"] }\nserde_json = "1"\ntracing = "0.1"\ntracing-subscriber = "0.3"\nsqlx = { version = "0.7", features = ["postgres", "runtime-tokio"] }\nredis = { version = "0.24", features = ["tokio-comp"] }\n` },
    { name: "README.md", type: "file", content: `# SafeClaw Gateway\n\nCross-border payment core gateway service.\n\n## Features\n\n- Route dispatch with weighted load balancing\n- Rate limiting & circuit breaker (tower middleware)\n- Protocol conversion: SWIFT MT → ISO 20022 XML\n- Audit logging with structured tracing\n\n## Quick Start\n\n\`\`\`bash\ncargo run -- --config config/dev.toml\n\`\`\`\n` },
    { name: "src", type: "folder", children: [
      { name: "main.rs", type: "file", content: `use axum::Router;\nuse std::net::SocketAddr;\nuse tracing::info;\n\nmod config;\nmod error;\nmod handlers;\nmod middleware;\nmod routes;\nmod services;\n\n#[tokio::main]\nasync fn main() -> anyhow::Result<()> {\n    tracing_subscriber::init();\n\n    let config = config::load()?;\n    let state = services::AppState::new(&config).await?;\n    let app = routes::build(state);\n\n    let addr: SocketAddr = config.listen_addr.parse()?;\n    info!("Gateway listening on {addr}");\n    let listener = tokio::net::TcpListener::bind(addr).await?;\n    axum::serve(listener, app).await?;\n\n    Ok(())\n}\n` },
      { name: "config.rs", type: "file", content: `use serde::Deserialize;\nuse std::path::PathBuf;\n\n#[derive(Debug, Deserialize)]\npub struct Config {\n    pub listen_addr: String,\n    pub database_url: String,\n    pub redis_url: String,\n    pub rate_limit: RateLimitConfig,\n    pub circuit_breaker: CircuitBreakerConfig,\n}\n\n#[derive(Debug, Deserialize)]\npub struct RateLimitConfig {\n    pub requests_per_second: u32,\n    pub burst_size: u32,\n}\n\n#[derive(Debug, Deserialize)]\npub struct CircuitBreakerConfig {\n    pub failure_threshold: u32,\n    pub reset_timeout_secs: u64,\n}\n\npub fn load() -> anyhow::Result<Config> {\n    let path = std::env::var("CONFIG_PATH")\n        .map(PathBuf::from)\n        .unwrap_or_else(|_| PathBuf::from("config/dev.toml"));\n    let content = std::fs::read_to_string(&path)?;\n    let config: Config = toml::from_str(&content)?;\n    Ok(config)\n}\n` },
      { name: "error.rs", type: "file", content: `use axum::http::StatusCode;\nuse axum::response::{IntoResponse, Response};\nuse thiserror::Error;\n\n#[derive(Debug, Error)]\npub enum GatewayError {\n    #[error("Upstream service unavailable: {0}")]\n    UpstreamUnavailable(String),\n\n    #[error("Rate limit exceeded for client {client_id}")]\n    RateLimitExceeded { client_id: String },\n\n    #[error("Circuit breaker open for service {service}")]\n    CircuitBreakerOpen { service: String },\n\n    #[error("Protocol conversion failed: {0}")]\n    ProtocolConversion(String),\n\n    #[error("Database error: {0}")]\n    Database(#[from] sqlx::Error),\n\n    #[error(transparent)]\n    Internal(#[from] anyhow::Error),\n}\n\nimpl IntoResponse for GatewayError {\n    fn into_response(self) -> Response {\n        let status = match &self {\n            GatewayError::UpstreamUnavailable(_) => StatusCode::BAD_GATEWAY,\n            GatewayError::RateLimitExceeded { .. } => StatusCode::TOO_MANY_REQUESTS,\n            GatewayError::CircuitBreakerOpen { .. } => StatusCode::SERVICE_UNAVAILABLE,\n            GatewayError::ProtocolConversion(_) => StatusCode::BAD_REQUEST,\n            _ => StatusCode::INTERNAL_SERVER_ERROR,\n        };\n        (status, self.to_string()).into_response()\n    }\n}\n` },
      { name: "routes.rs", type: "file", content: `use axum::{routing::{get, post}, Router};\nuse crate::{handlers, middleware, services::AppState};\n\npub fn build(state: AppState) -> Router {\n    Router::new()\n        .route("/health", get(handlers::health))\n        .route("/api/v1/payments", post(handlers::create_payment))\n        .route("/api/v1/payments/:id", get(handlers::get_payment))\n        .route("/api/v1/payments/:id/status", get(handlers::get_payment_status))\n        .route("/api/v1/swift/webhook", post(handlers::swift_webhook))\n        .layer(middleware::rate_limit_layer(&state.config))\n        .layer(middleware::audit_layer())\n        .with_state(state)\n}\n` },
      { name: "handlers", type: "folder", children: [
        { name: "mod.rs", type: "file", content: `mod health;\nmod payment;\nmod webhook;\n\npub use health::health;\npub use payment::{create_payment, get_payment, get_payment_status};\npub use webhook::swift_webhook;\n` },
        { name: "payment.rs", type: "file", content: `use axum::extract::{Path, State};\nuse axum::Json;\nuse serde::{Deserialize, Serialize};\nuse crate::error::GatewayError;\nuse crate::services::AppState;\n\n#[derive(Deserialize)]\npub struct CreatePaymentRequest {\n    pub source_currency: String,\n    pub target_currency: String,\n    pub amount: f64,\n    pub beneficiary: BeneficiaryInfo,\n    pub reference: String,\n}\n\n#[derive(Deserialize)]\npub struct BeneficiaryInfo {\n    pub name: String,\n    pub account: String,\n    pub bank_code: String,\n    pub country: String,\n}\n\n#[derive(Serialize)]\npub struct PaymentResponse {\n    pub id: String,\n    pub status: String,\n    pub created_at: String,\n}\n\npub async fn create_payment(\n    State(state): State<AppState>,\n    Json(req): Json<CreatePaymentRequest>,\n) -> Result<Json<PaymentResponse>, GatewayError> {\n    let payment = state.payment_service.create(req).await?;\n    Ok(Json(payment))\n}\n\npub async fn get_payment(\n    State(state): State<AppState>,\n    Path(id): Path<String>,\n) -> Result<Json<PaymentResponse>, GatewayError> {\n    let payment = state.payment_service.get(&id).await?;\n    Ok(Json(payment))\n}\n\npub async fn get_payment_status(\n    State(state): State<AppState>,\n    Path(id): Path<String>,\n) -> Result<Json<serde_json::Value>, GatewayError> {\n    let status = state.payment_service.get_status(&id).await?;\n    Ok(Json(status))\n}\n` },
      ]},
      { name: "services", type: "folder", children: [
        { name: "mod.rs", type: "file", content: `mod payment;\n\nuse crate::config::Config;\nuse std::sync::Arc;\n\npub use payment::PaymentService;\n\n#[derive(Clone)]\npub struct AppState {\n    pub config: Arc<Config>,\n    pub payment_service: PaymentService,\n}\n\nimpl AppState {\n    pub async fn new(config: &Config) -> anyhow::Result<Self> {\n        let pool = sqlx::PgPool::connect(&config.database_url).await?;\n        let payment_service = PaymentService::new(pool);\n        Ok(Self {\n            config: Arc::new(config.clone()),\n            payment_service,\n        })\n    }\n}\n` },
      ]},
      { name: "middleware", type: "folder", children: [
        { name: "mod.rs", type: "file", content: `mod audit;\nmod rate_limit;\n\npub use audit::audit_layer;\npub use rate_limit::rate_limit_layer;\n` },
      ]},
    ]},
    { name: "config", type: "folder", children: [
      { name: "dev.toml", type: "file", content: `listen_addr = "0.0.0.0:8080"\ndatabase_url = "postgres://gateway:secret@localhost:5432/gateway"\nredis_url = "redis://localhost:6379"\n\n[rate_limit]\nrequests_per_second = 1000\nburst_size = 200\n\n[circuit_breaker]\nfailure_threshold = 5\nreset_timeout_secs = 30\n` },
    ]},
    { name: "tests", type: "folder", children: [
      { name: "integration.rs", type: "file", content: `use axum::http::StatusCode;\nuse axum_test::TestServer;\n\n#[tokio::test]\nasync fn test_health_check() {\n    let app = setup_test_app().await;\n    let server = TestServer::new(app).unwrap();\n    let resp = server.get("/health").await;\n    resp.assert_status(StatusCode::OK);\n}\n\n#[tokio::test]\nasync fn test_create_payment() {\n    let app = setup_test_app().await;\n    let server = TestServer::new(app).unwrap();\n    let resp = server\n        .post("/api/v1/payments")\n        .json(&serde_json::json!({\n            "source_currency": "USD",\n            "target_currency": "EUR",\n            "amount": 1000.0,\n            "beneficiary": {\n                "name": "Test Corp",\n                "account": "DE89370400440532013000",\n                "bank_code": "COBADEFFXXX",\n                "country": "DE"\n            },\n            "reference": "INV-2024-001"\n        }))\n        .await;\n    resp.assert_status(StatusCode::OK);\n}\n` },
    ]},
  ],
  "proj-2": [
    { name: "Cargo.toml", type: "file", content: `[package]\nname = "safeclaw-risk-engine"\nversion = "2.8.1"\nedition = "2021"\n\n[dependencies]\ntokio = { version = "1", features = ["full"] }\ntonic = "0.11"\nprost = "0.12"\nserde = { version = "1", features = ["derive"] }\nndarray = "0.15"\nlinfa = "0.7"\ntracing = "0.1"\n` },
    { name: "src", type: "folder", children: [
      { name: "main.rs", type: "file", content: `use tracing::info;\n\nmod engine;\nmod models;\nmod rules;\nmod scoring;\n\n#[tokio::main]\nasync fn main() -> anyhow::Result<()> {\n    tracing_subscriber::init();\n    info!("Risk engine starting...");\n\n    let engine = engine::RiskEngine::new().await?;\n    engine.serve().await?;\n\n    Ok(())\n}\n` },
      { name: "engine.rs", type: "file", content: `use crate::models::ModelRegistry;\nuse crate::rules::RuleEngine;\nuse crate::scoring::ScoreAggregator;\nuse std::sync::Arc;\nuse tokio::sync::RwLock;\n\npub struct RiskEngine {\n    rules: Arc<RuleEngine>,\n    models: Arc<RwLock<ModelRegistry>>,\n    aggregator: ScoreAggregator,\n}\n\nimpl RiskEngine {\n    pub async fn new() -> anyhow::Result<Self> {\n        let rules = Arc::new(RuleEngine::load_from_config().await?);\n        let models = Arc::new(RwLock::new(ModelRegistry::load().await?));\n        let aggregator = ScoreAggregator::new();\n\n        Ok(Self { rules, models, aggregator })\n    }\n\n    pub async fn evaluate(&self, request: &EvalRequest) -> anyhow::Result<RiskDecision> {\n        let rule_score = self.rules.evaluate(request).await?;\n        let model_score = {\n            let models = self.models.read().await;\n            models.predict(request).await?\n        };\n\n        let decision = self.aggregator.aggregate(rule_score, model_score);\n        Ok(decision)\n    }\n\n    pub async fn serve(&self) -> anyhow::Result<()> {\n        // gRPC server setup\n        todo!()\n    }\n}\n\npub struct EvalRequest {\n    pub transaction_id: String,\n    pub amount: f64,\n    pub currency: String,\n    pub sender_id: String,\n    pub receiver_id: String,\n    pub features: std::collections::HashMap<String, f64>,\n}\n\npub struct RiskDecision {\n    pub score: f64,\n    pub action: RiskAction,\n    pub reasons: Vec<String>,\n}\n\npub enum RiskAction {\n    Allow,\n    Review,\n    Block,\n}\n` },
      { name: "rules.rs", type: "file", content: `use serde::Deserialize;\n\n#[derive(Debug, Deserialize)]\npub struct Rule {\n    pub id: String,\n    pub name: String,\n    pub condition: String,\n    pub score_impact: f64,\n    pub enabled: bool,\n}\n\npub struct RuleEngine {\n    rules: Vec<Rule>,\n}\n\nimpl RuleEngine {\n    pub async fn load_from_config() -> anyhow::Result<Self> {\n        // Load rules from config file or database\n        let rules = vec![];\n        Ok(Self { rules })\n    }\n\n    pub async fn evaluate(&self, _request: &super::engine::EvalRequest) -> anyhow::Result<f64> {\n        let mut total_score = 0.0;\n        for rule in &self.rules {\n            if rule.enabled {\n                // Evaluate rule condition against request\n                total_score += rule.score_impact;\n            }\n        }\n        Ok(total_score)\n    }\n}\n` },
    ]},
  ],
  "proj-3": [
    { name: "package.json", type: "file", content: `{\n  "name": "safeclaw-ui",\n  "private": true,\n  "version": "0.9.4",\n  "scripts": {\n    "dev": "rsbuild dev --env-dir ./env",\n    "build": "rsbuild build --env-dir ./env",\n    "tauri:dev": "tauri dev",\n    "tauri:build": "tauri build"\n  },\n  "dependencies": {\n    "@tauri-apps/api": "^2",\n    "react": "^18.3.1",\n    "react-dom": "^18.3.1",\n    "react-router-dom": "^7.5.3",\n    "valtio": "^2.1.1",\n    "shiki": "^3.22.0"\n  }\n}\n` },
    { name: "src", type: "folder", children: [
      { name: "main.tsx", type: "file", content: `import React from "react";\nimport ReactDOM from "react-dom/client";\nimport { RouterProvider } from "react-router-dom";\nimport router from "./router";\nimport "./index.css";\n\nReactDOM.createRoot(document.getElementById("root")!).render(\n  <React.StrictMode>\n    <RouterProvider router={router} />\n  </React.StrictMode>,\n);\n` },
      { name: "router.tsx", type: "file", content: `import { createHashRouter } from "react-router-dom";\nimport ChatLayout from "@/layouts/chat";\n\nconst router = createHashRouter([\n  {\n    path: "/",\n    element: <ChatLayout />,\n    children: [\n      { index: true, lazy: async () => ({ Component: (await import("@/pages/agent")).default }) },\n      { path: "events", lazy: async () => ({ Component: (await import("@/pages/events")).default }) },\n      { path: "knowledge", lazy: async () => ({ Component: (await import("@/pages/knowledge")).default }) },\n      { path: "assets", lazy: async () => ({ Component: (await import("@/pages/assets")).default }) },\n      { path: "settings", lazy: async () => ({ Component: (await import("@/pages/settings")).default }) },\n    ],\n  },\n]);\n\nexport default router;\n` },
    ]},
    { name: "src-tauri", type: "folder", children: [
      { name: "Cargo.toml", type: "file", content: `[package]\nname = "safeclaw-ui"\nversion = "0.9.4"\nedition = "2021"\n\n[dependencies]\ntauri = { version = "2", features = [] }\ntauri-plugin-http = "2"\ntauri-plugin-shell = "2"\nserde = { version = "1", features = ["derive"] }\nserde_json = "1"\n` },
    ]},
  ],
  "proj-4": [
    { name: "pyproject.toml", type: "file", content: `[project]\nname = "safeclaw-data-platform"\nversion = "1.5.0"\nrequires-python = ">=3.11"\n\n[tool.ruff]\nline-length = 100\n\n[tool.pytest.ini_options]\nasyncio_mode = "auto"\n` },
    { name: "src", type: "folder", children: [
      { name: "pipeline", type: "folder", children: [
        { name: "__init__.py", type: "file", content: `"""Data pipeline orchestration module."""\n\nfrom .runner import PipelineRunner\nfrom .config import PipelineConfig\n\n__all__ = ["PipelineRunner", "PipelineConfig"]\n` },
        { name: "runner.py", type: "file", content: `"""Pipeline execution engine."""\nimport asyncio\nfrom dataclasses import dataclass\nfrom typing import Any\nimport clickhouse_connect\n\n\n@dataclass\nclass PipelineStep:\n    name: str\n    handler: str\n    config: dict[str, Any]\n    depends_on: list[str]\n\n\nclass PipelineRunner:\n    \"\"\"Execute data pipeline DAGs with dependency resolution.\"\"\"\n\n    def __init__(self, ch_client: clickhouse_connect.driver.Client):\n        self.ch = ch_client\n        self._steps: list[PipelineStep] = []\n\n    def add_step(self, step: PipelineStep) -> None:\n        self._steps.append(step)\n\n    async def run(self) -> dict[str, Any]:\n        results = {}\n        resolved = set()\n\n        while len(resolved) < len(self._steps):\n            ready = [\n                s for s in self._steps\n                if s.name not in resolved\n                and all(dep in resolved for dep in s.depends_on)\n            ]\n            if not ready:\n                raise RuntimeError("Circular dependency detected")\n\n            tasks = [self._execute_step(s) for s in ready]\n            outcomes = await asyncio.gather(*tasks)\n\n            for step, outcome in zip(ready, outcomes):\n                results[step.name] = outcome\n                resolved.add(step.name)\n\n        return results\n\n    async def _execute_step(self, step: PipelineStep) -> Any:\n        # Dynamic handler dispatch\n        handler = self._load_handler(step.handler)\n        return await handler(step.config, self.ch)\n\n    def _load_handler(self, handler_path: str):\n        module_path, func_name = handler_path.rsplit(".", 1)\n        import importlib\n        module = importlib.import_module(module_path)\n        return getattr(module, func_name)\n` },
      ]},
      { name: "warehouse", type: "folder", children: [
        { name: "__init__.py", type: "file", content: `"""Data warehouse query and management."""\n` },
        { name: "query.py", type: "file", content: `\"\"\"ClickHouse query builder and executor.\"\"\"\nfrom typing import Any\nimport clickhouse_connect\n\n\nclass QueryBuilder:\n    def __init__(self, client: clickhouse_connect.driver.Client):\n        self.client = client\n        self._table = \"\"\n        self._columns: list[str] = []\n        self._conditions: list[str] = []\n        self._order: list[str] = []\n        self._limit: int | None = None\n\n    def select(self, *columns: str) -> \"QueryBuilder\":\n        self._columns.extend(columns)\n        return self\n\n    def from_table(self, table: str) -> \"QueryBuilder\":\n        self._table = table\n        return self\n\n    def where(self, condition: str) -> \"QueryBuilder\":\n        self._conditions.append(condition)\n        return self\n\n    def order_by(self, *columns: str) -> \"QueryBuilder\":\n        self._order.extend(columns)\n        return self\n\n    def limit(self, n: int) -> \"QueryBuilder\":\n        self._limit = n\n        return self\n\n    def build(self) -> str:\n        cols = \", \".join(self._columns) if self._columns else \"*\"\n        sql = f\"SELECT {cols} FROM {self._table}\"\n        if self._conditions:\n            sql += \" WHERE \" + \" AND \".join(self._conditions)\n        if self._order:\n            sql += \" ORDER BY \" + \", \".join(self._order)\n        if self._limit:\n            sql += f\" LIMIT {self._limit}\"\n        return sql\n\n    def execute(self) -> Any:\n        return self.client.query(self.build())\n` },
      ]},
    ]},
  ],
  "proj-5": [
    { name: "go.mod", type: "file", content: `module github.com/safeclaw/compliance-api\n\ngo 1.22\n\nrequire (\n\tgithub.com/gin-gonic/gin v1.9.1\n\tgithub.com/jackc/pgx/v5 v5.5.0\n\tgo.uber.org/zap v1.27.0\n)\n` },
    { name: "main.go", type: "file", content: `package main\n\nimport (\n\t"log"\n\t"os"\n\n\t"github.com/safeclaw/compliance-api/internal/server"\n)\n\nfunc main() {\n\tport := os.Getenv("PORT")\n\tif port == "" {\n\t\tport = "8080"\n\t}\n\n\tsrv, err := server.New()\n\tif err != nil {\n\t\tlog.Fatalf("Failed to create server: %v", err)\n\t}\n\n\tlog.Printf("Compliance API listening on :%s", port)\n\tif err := srv.Run(":" + port); err != nil {\n\t\tlog.Fatal(err)\n\t}\n}\n` },
    { name: "internal", type: "folder", children: [
      { name: "server", type: "folder", children: [
        { name: "server.go", type: "file", content: `package server\n\nimport (\n\t"github.com/gin-gonic/gin"\n\t"github.com/safeclaw/compliance-api/internal/handlers"\n)\n\ntype Server struct {\n\trouter *gin.Engine\n}\n\nfunc New() (*Server, error) {\n\tr := gin.Default()\n\n\tapi := r.Group("/api/v1")\n\t{\n\t\tapi.POST("/kyc/verify", handlers.VerifyKYC)\n\t\tapi.POST("/aml/screen", handlers.ScreenAML)\n\t\tapi.GET("/sanctions/check/:entity", handlers.CheckSanctions)\n\t\tapi.POST("/reports/generate", handlers.GenerateReport)\n\t}\n\n\treturn &Server{router: r}, nil\n}\n\nfunc (s *Server) Run(addr string) error {\n\treturn s.router.Run(addr)\n}\n` },
      ]},
      { name: "handlers", type: "folder", children: [
        { name: "kyc.go", type: "file", content: `package handlers\n\nimport (\n\t"net/http"\n\n\t"github.com/gin-gonic/gin"\n)\n\ntype KYCRequest struct {\n\tFullName    string \`json:"full_name" binding:"required"\`\n\tDateOfBirth string \`json:"date_of_birth" binding:"required"\`\n\tNationality string \`json:"nationality" binding:"required"\`\n\tDocumentID  string \`json:"document_id" binding:"required"\`\n\tDocumentType string \`json:"document_type" binding:"required"\`\n}\n\ntype KYCResponse struct {\n\tVerified bool     \`json:"verified"\`\n\tScore    float64  \`json:"score"\`\n\tFlags    []string \`json:"flags,omitempty"\`\n}\n\nfunc VerifyKYC(c *gin.Context) {\n\tvar req KYCRequest\n\tif err := c.ShouldBindJSON(&req); err != nil {\n\t\tc.JSON(http.StatusBadRequest, gin.H{"error": err.Error()})\n\t\treturn\n\t}\n\n\t// KYC verification logic\n\tresp := KYCResponse{\n\t\tVerified: true,\n\t\tScore:    0.95,\n\t}\n\n\tc.JSON(http.StatusOK, resp)\n}\n` },
      ]},
    ]},
  ],
  "proj-6": [
    { name: "Makefile", type: "file", content: `SHELL := /bin/bash\n.PHONY: plan apply destroy\n\nplan:\n\tterraform plan -var-file=env/prod.tfvars\n\napply:\n\tterraform apply -var-file=env/prod.tfvars -auto-approve\n\ndestroy:\n\tterraform destroy -var-file=env/prod.tfvars\n\nkubectl-apply:\n\tkubectl apply -k k8s/overlays/prod\n` },
    { name: "terraform", type: "folder", children: [
      { name: "main.tf", type: "file", content: `terraform {\n  required_providers {\n    aws = {\n      source  = "hashicorp/aws"\n      version = "~> 5.0"\n    }\n    kubernetes = {\n      source  = "hashicorp/kubernetes"\n      version = "~> 2.25"\n    }\n  }\n  backend "s3" {\n    bucket = "safeclaw-terraform-state"\n    key    = "infra/terraform.tfstate"\n    region = "ap-southeast-1"\n  }\n}\n\nmodule "eks" {\n  source          = "./modules/eks"\n  cluster_name    = var.cluster_name\n  cluster_version = "1.29"\n  node_groups     = var.node_groups\n  vpc_id          = module.vpc.vpc_id\n  subnet_ids      = module.vpc.private_subnets\n}\n\nmodule "vpc" {\n  source     = "./modules/vpc"\n  cidr_block = var.vpc_cidr\n  azs        = var.availability_zones\n}\n\nmodule "rds" {\n  source        = "./modules/rds"\n  instance_class = "db.r6g.xlarge"\n  engine        = "postgres"\n  engine_version = "16.1"\n}\n` },
    ]},
    { name: "k8s", type: "folder", children: [
      { name: "base", type: "folder", children: [
        { name: "deployment.yaml", type: "file", content: `apiVersion: apps/v1\nkind: Deployment\nmetadata:\n  name: safeclaw-gateway\n  labels:\n    app: safeclaw-gateway\nspec:\n  replicas: 3\n  selector:\n    matchLabels:\n      app: safeclaw-gateway\n  template:\n    metadata:\n      labels:\n        app: safeclaw-gateway\n    spec:\n      containers:\n      - name: gateway\n        image: safeclaw/gateway:latest\n        ports:\n        - containerPort: 8080\n        resources:\n          requests:\n            cpu: "500m"\n            memory: "512Mi"\n          limits:\n            cpu: "2"\n            memory: "2Gi"\n        livenessProbe:\n          httpGet:\n            path: /health\n            port: 8080\n          initialDelaySeconds: 10\n          periodSeconds: 30\n` },
      ]},
    ]},
    { name: "scripts", type: "folder", children: [
      { name: "deploy.sh", type: "file", content: `#!/usr/bin/env bash\nset -euo pipefail\n\nENV=\${1:-staging}\nTAG=\${2:-latest}\n\necho "Deploying to $ENV with tag $TAG..."\n\n# Build and push Docker images\ndocker buildx build \\\\\n  --platform linux/amd64,linux/arm64 \\\\\n  -t safeclaw/gateway:$TAG \\\\\n  --push \\\\\n  ../safeclaw-gateway\n\n# Apply Kubernetes manifests\nkubectl apply -k k8s/overlays/$ENV\n\n# Wait for rollout\nkubectl rollout status deployment/safeclaw-gateway -n safeclaw --timeout=300s\n\necho "Deployment complete!"\n` },
    ]},
  ],
};

// =============================================================================
// Helpers
// =============================================================================

function formatTime(ts: number): string {
  const diff = Date.now() - ts;
  const mins = Math.floor(diff / 60_000);
  if (mins < 1) return "刚刚";
  if (mins < 60) return `${mins} 分钟前`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours} 小时前`;
  const days = Math.floor(hours / 24);
  if (days < 30) return `${days} 天前`;
  return new Date(ts).toLocaleDateString("zh-CN");
}

/** Flatten file tree to find a node by path */
function findFileByPath(tree: FileNode[], path: string[]): FileNode | null {
  if (path.length === 0) return null;
  const [head, ...rest] = path;
  const node = tree.find((n) => n.name === head);
  if (!node) return null;
  if (rest.length === 0) return node;
  if (node.type === "folder" && node.children) return findFileByPath(node.children, rest);
  return null;
}

/** Count total files in a tree */
function countFiles(nodes: FileNode[]): number {
  let count = 0;
  for (const n of nodes) {
    if (n.type === "file") count++;
    if (n.children) count += countFiles(n.children);
  }
  return count;
}

// =============================================================================
// Language dot
// =============================================================================

function LangDot({ lang }: { lang: string }) {
  const bg = LANG_COLORS[lang] || "bg-gray-400";
  return (
    <span className="flex items-center gap-1 text-[11px] text-muted-foreground">
      <span className={cn("inline-block size-2.5 rounded-full", bg)} />
      {lang}
    </span>
  );
}

// =============================================================================
// Progress bar
// =============================================================================

function ProgressBar({ progress, className }: { progress: number; className?: string }) {
  return (
    <div className={cn("h-1.5 bg-muted rounded-full overflow-hidden", className)}>
      <div
        className={cn(
          "h-full rounded-full transition-all",
          progress >= 100 ? "bg-green-500" : progress >= 60 ? "bg-primary" : "bg-orange-400",
        )}
        style={{ width: `${Math.min(progress, 100)}%` }}
      />
    </div>
  );
}

// =============================================================================
// Enterprise project card
// =============================================================================

function EnterpriseProjectCard({ project, onClick }: { project: ProjectItem; onClick: () => void }) {
  const statusCfg = STATUS_CONFIG[project.status];
  const StatusIcon = statusCfg.icon;

  return (
    <div
      className="rounded-lg border bg-card p-4 hover:shadow-sm transition-shadow cursor-pointer hover:border-primary/30"
      onClick={onClick}
    >
      {/* Header */}
      <div className="flex items-center gap-2 mb-2">
        <Server className="size-4 text-primary shrink-0" />
        <h3 className="text-sm font-semibold font-mono truncate">{project.name}</h3>
        <span className={cn("flex items-center gap-1 text-[10px] font-medium ml-auto shrink-0", statusCfg.color)}>
          <StatusIcon className="size-3" />
          {statusCfg.label}
        </span>
      </div>

      {/* Description */}
      <p className="text-xs text-muted-foreground leading-relaxed mb-3">{project.description}</p>

      {/* Git info */}
      <div className="rounded bg-muted/40 px-3 py-2 mb-3 space-y-1.5">
        <div className="flex items-center gap-2 text-[11px]">
          <GitBranch className="size-3 text-muted-foreground shrink-0" />
          <span className="font-mono text-muted-foreground">{project.git.branch}</span>
          {project.version && (
            <>
              <Tag className="size-3 text-muted-foreground shrink-0 ml-2" />
              <span className="font-mono text-primary font-medium">{project.version}</span>
            </>
          )}
        </div>
        <div className="flex items-center gap-2 text-[11px] text-muted-foreground">
          <GitCommit className="size-3 shrink-0" />
          <span className="truncate flex-1">{project.git.lastCommit}</span>
          <span className="shrink-0">{formatTime(project.git.lastCommitTime)}</span>
        </div>
        <div className="flex items-center gap-3 text-[10px] text-muted-foreground">
          <span className="flex items-center gap-1">
            <GitCommit className="size-3" />
            {project.git.commitCount} commits
          </span>
          {project.git.openPRs !== undefined && project.git.openPRs > 0 && (
            <span className="flex items-center gap-1">
              <GitPullRequest className="size-3" />
              {project.git.openPRs} open PRs
            </span>
          )}
          {project.git.stars !== undefined && project.git.stars > 0 && (
            <span className="flex items-center gap-1">
              <Star className="size-3" />
              {project.git.stars}
            </span>
          )}
        </div>
      </div>

      {/* Footer */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          {project.languages?.map((lang) => <LangDot key={lang} lang={lang} />) || <LangDot lang={project.language} />}
        </div>
        <div className="flex items-center gap-2">
          {project.team && (
            <span className="text-[10px] text-muted-foreground">{project.team}</span>
          )}
          {project.tags?.slice(0, 2).map((tag) => (
            <Badge key={tag} variant="secondary" className="text-[10px] px-1.5 py-0 h-4">
              {tag}
            </Badge>
          ))}
        </div>
      </div>
    </div>
  );
}

// =============================================================================
// Agent project card
// =============================================================================

function AgentProjectCard({ project }: { project: ProjectItem }) {
  const [expanded, setExpanded] = useState(false);
  const statusCfg = STATUS_CONFIG[project.status];
  const StatusIcon = statusCfg.icon;
  const devStatusCfg = project.devStatus ? AGENT_DEV_STATUS_CONFIG[project.devStatus] : null;

  const persona = project.agentId ? BUILTIN_PERSONAS.find((p) => p.id === project.agentId) : null;
  const avatarCfg = persona ? genConfig(persona.avatar) : null;

  const doneCount = project.milestones?.filter((m) => m.done).length || 0;
  const totalCount = project.milestones?.length || 0;

  return (
    <div className="rounded-lg border bg-card p-4 hover:shadow-sm transition-shadow">
      {/* Header */}
      <div className="flex items-center gap-2 mb-2">
        <Rocket className="size-4 text-primary shrink-0" />
        <h3 className="text-sm font-semibold font-mono truncate">{project.name}</h3>
        <span className={cn("flex items-center gap-1 text-[10px] font-medium ml-auto shrink-0", statusCfg.color)}>
          <StatusIcon className="size-3" />
          {statusCfg.label}
        </span>
      </div>

      {/* Agent + dev status */}
      <div className="flex items-center gap-2 mb-2">
        {persona && avatarCfg && (
          <div className="flex items-center gap-1.5">
            <NiceAvatar className="size-5 ring-1 ring-border" {...avatarCfg} />
            <span className="text-[11px] font-medium">{persona.name}</span>
          </div>
        )}
        {devStatusCfg && (
          <span className={cn("text-[10px] font-medium px-2 py-0.5 rounded-full ml-auto", devStatusCfg.color)}>
            {devStatusCfg.label}
          </span>
        )}
      </div>

      {/* Description */}
      <p className="text-xs text-muted-foreground leading-relaxed mb-2">{project.description}</p>

      {/* Dev goal */}
      {project.devGoal && (
        <div className="rounded bg-primary/5 border border-primary/10 px-3 py-2 mb-3">
          <div className="text-[10px] text-primary font-medium mb-1 flex items-center gap-1">
            <Play className="size-3" />
            开发目标
          </div>
          <p className="text-[11px] text-foreground/80 leading-relaxed">{project.devGoal}</p>
        </div>
      )}

      {/* Progress */}
      {project.devProgress !== undefined && (
        <div className="mb-3">
          <div className="flex items-center justify-between mb-1">
            <span className="text-[10px] text-muted-foreground">开发进度</span>
            <span className="text-[11px] font-medium">{project.devProgress}%</span>
          </div>
          <ProgressBar progress={project.devProgress} />
        </div>
      )}

      {/* Milestones (collapsible) */}
      {project.milestones && project.milestones.length > 0 && (
        <>
          <button
            type="button"
            className="flex items-center gap-1 text-[11px] text-muted-foreground hover:text-foreground transition-colors mb-2 w-full"
            onClick={() => setExpanded(!expanded)}
          >
            {expanded ? <ChevronDown className="size-3" /> : <ChevronRight className="size-3" />}
            <span>里程碑 ({doneCount}/{totalCount})</span>
          </button>
          {expanded && (
            <div className="space-y-1.5 mb-3">
              {project.milestones.map((ms) => (
                <div key={ms.id} className="flex items-center gap-2 text-[11px]">
                  {ms.done ? (
                    <CheckCircle2 className="size-3.5 text-primary shrink-0" />
                  ) : (
                    <Circle className="size-3.5 text-muted-foreground shrink-0" />
                  )}
                  <span className={cn(ms.done && "text-muted-foreground line-through")}>{ms.title}</span>
                </div>
              ))}
            </div>
          )}
        </>
      )}

      {/* Git info */}
      <div className="rounded bg-muted/40 px-3 py-2 mb-3 space-y-1.5">
        <div className="flex items-center gap-2 text-[11px]">
          <GitBranch className="size-3 text-muted-foreground shrink-0" />
          <span className="font-mono text-muted-foreground">{project.git.branch}</span>
        </div>
        <div className="flex items-center gap-2 text-[11px] text-muted-foreground">
          <GitCommit className="size-3 shrink-0" />
          <span className="truncate flex-1">{project.git.lastCommit}</span>
          <span className="shrink-0">{formatTime(project.git.lastCommitTime)}</span>
        </div>
        <div className="flex items-center gap-3 text-[10px] text-muted-foreground">
          <span className="flex items-center gap-1">
            <GitCommit className="size-3" />
            {project.git.commitCount} commits
          </span>
          {project.git.openPRs !== undefined && project.git.openPRs > 0 && (
            <span className="flex items-center gap-1">
              <GitPullRequest className="size-3" />
              {project.git.openPRs} open PRs
            </span>
          )}
        </div>
      </div>

      {/* Footer */}
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-3">
          {project.languages?.map((lang) => <LangDot key={lang} lang={lang} />) || <LangDot lang={project.language} />}
        </div>
        <div className="flex items-center gap-2">
          {project.tags?.slice(0, 2).map((tag) => (
            <Badge key={tag} variant="secondary" className="text-[10px] px-1.5 py-0 h-4">
              {tag}
            </Badge>
          ))}
        </div>
      </div>
    </div>
  );
}

// =============================================================================
// File tree node (recursive)
// =============================================================================

function FileTreeNode({
  node,
  depth,
  activePath,
  currentPath,
  onSelect,
}: {
  node: FileNode;
  depth: number;
  activePath: string[];
  currentPath: string[];
  onSelect: (path: string[]) => void;
}) {
  const [open, setOpen] = useState(depth < 1);
  const fullPath = [...currentPath, node.name];
  const isActive = activePath.join("/") === fullPath.join("/");
  const isFolder = node.type === "folder";

  const handleClick = () => {
    if (isFolder) {
      setOpen(!open);
    } else {
      onSelect(fullPath);
    }
  };

  return (
    <>
      <button
        type="button"
        className={cn(
          "flex items-center gap-1.5 w-full text-left text-[12px] py-1 px-2 rounded-sm transition-colors",
          isActive
            ? "bg-primary/10 text-primary font-medium"
            : "text-muted-foreground hover:bg-foreground/[0.04] hover:text-foreground",
        )}
        style={{ paddingLeft: `${depth * 16 + 8}px` }}
        onClick={handleClick}
      >
        {isFolder ? (
          open ? (
            <>
              <ChevronDown className="size-3 shrink-0" />
              <FolderOpen className="size-4 text-yellow-600 dark:text-yellow-400 shrink-0" />
            </>
          ) : (
            <>
              <ChevronRight className="size-3 shrink-0" />
              <Folder className="size-4 text-yellow-600 dark:text-yellow-400 shrink-0" />
            </>
          )
        ) : (
          <>
            <span className="size-3" />
            {fileIcon(node.name)}
          </>
        )}
        <span className="truncate">{node.name}</span>
      </button>
      {isFolder && open && node.children && (
        <div>
          {[...node.children]
            .sort((a, b) => {
              if (a.type !== b.type) return a.type === "folder" ? -1 : 1;
              return a.name.localeCompare(b.name);
            })
            .map((child) => (
              <FileTreeNode
                key={child.name}
                node={child}
                depth={depth + 1}
                activePath={activePath}
                currentPath={fullPath}
                onSelect={onSelect}
              />
            ))}
        </div>
      )}
    </>
  );
}

// =============================================================================
// Code editor view (shown when a project is selected)
// =============================================================================

function ProjectEditorView({
  project,
  onBack,
}: {
  project: ProjectItem;
  onBack: () => void;
}) {
  const fileTree = MOCK_FILE_TREES[project.id] || [];
  const [activePath, setActivePath] = useState<string[]>([]);

  const activeFile = useMemo(() => {
    if (activePath.length === 0) return null;
    return findFileByPath(fileTree, activePath);
  }, [activePath, fileTree]);

  const language = activeFile ? extToLang(activeFile.name) : "plaintext";
  const totalFiles = useMemo(() => countFiles(fileTree), [fileTree]);

  return (
    <div className="flex flex-col h-full w-full">
      {/* Top toolbar */}
      <div className="h-10 border-b flex items-center gap-2 px-3 bg-card shrink-0">
        <Button variant="ghost" size="sm" className="h-7 px-2 text-xs gap-1" onClick={onBack}>
          <ArrowLeft className="size-3.5" />
          返回
        </Button>
        <div className="w-px h-4 bg-border" />
        <Server className="size-3.5 text-primary" />
        <span className="text-xs font-semibold font-mono">{project.name}</span>
        {project.version && (
          <Badge variant="secondary" className="text-[10px] px-1.5 py-0 h-4">
            {project.version}
          </Badge>
        )}
        <div className="flex items-center gap-1.5 ml-2 text-[10px] text-muted-foreground">
          <GitBranch className="size-3" />
          <span className="font-mono">{project.git.branch}</span>
        </div>
        <div className="ml-auto flex items-center gap-2 text-[10px] text-muted-foreground">
          <span>{totalFiles} 文件</span>
          {project.team && <span>· {project.team}</span>}
        </div>
      </div>

      {/* Editor area */}
      <ResizablePanelGroup direction="horizontal" className="flex-1">
        {/* File tree sidebar */}
        <ResizablePanel defaultSize={22} minSize={15} maxSize={40}>
          <div className="h-full flex flex-col bg-card">
            <div className="px-3 py-2 border-b text-[11px] font-medium text-muted-foreground uppercase tracking-wider">
              文件浏览
            </div>
            <ScrollArea className="flex-1">
              <div className="py-1">
                {[...fileTree]
                  .sort((a, b) => {
                    if (a.type !== b.type) return a.type === "folder" ? -1 : 1;
                    return a.name.localeCompare(b.name);
                  })
                  .map((node) => (
                    <FileTreeNode
                      key={node.name}
                      node={node}
                      depth={0}
                      activePath={activePath}
                      currentPath={[]}
                      onSelect={setActivePath}
                    />
                  ))}
              </div>
            </ScrollArea>
          </div>
        </ResizablePanel>

        <ResizableHandle />

        {/* Editor panel */}
        <ResizablePanel defaultSize={78}>
          <div className="h-full flex flex-col">
            {activeFile ? (
              <>
                {/* File tab bar */}
                <div className="h-8 border-b flex items-center gap-2 px-3 bg-muted/30 shrink-0">
                  {fileIcon(activeFile.name)}
                  <span className="text-xs font-mono">
                    {activePath.join("/")}
                  </span>
                  <span className="text-[10px] text-muted-foreground ml-2 uppercase">{language}</span>
                </div>
                {/* Monaco editor */}
                <div className="flex-1">
                  <CodeEditor
                    value={activeFile.content || ""}
                    language={language}
                    options={{
                      readOnly: true,
                      fontSize: 13,
                      lineNumbers: "on",
                      minimap: { enabled: true },
                      scrollBeyondLastLine: false,
                      wordWrap: "on",
                      renderWhitespace: "selection",
                      guides: { bracketPairs: true },
                    }}
                  />
                </div>
              </>
            ) : (
              <div className="flex-1 flex flex-col items-center justify-center text-muted-foreground">
                <FileCode2 className="size-12 mb-3 opacity-20" />
                <p className="text-sm">选择左侧文件查看代码</p>
                <p className="text-[11px] mt-1 text-muted-foreground/60">
                  {project.name} · {totalFiles} 个文件
                </p>
              </div>
            )}
          </div>
        </ResizablePanel>
      </ResizablePanelGroup>
    </div>
  );
}

// =============================================================================
// Sidebar stat item
// =============================================================================

function StatItem({ label, value, icon: Icon }: { label: string; value: number | string; icon: typeof Boxes }) {
  return (
    <div className="flex items-center gap-2 text-xs text-muted-foreground">
      <Icon className="size-3.5 shrink-0" />
      <span className="flex-1">{label}</span>
      <span className="font-medium text-foreground">{value}</span>
    </div>
  );
}

// =============================================================================
// Main Assets Page
// =============================================================================

export default function AssetsPage() {
  const [activeCategory, setActiveCategory] = useState<ProjectCategory>("all");
  const [search, setSearch] = useState("");
  const [selectedProject, setSelectedProject] = useState<ProjectItem | null>(null);
  const q = search.trim().toLowerCase();

  const filtered = useMemo(() => {
    let projects = MOCK_PROJECTS;
    if (activeCategory !== "all") {
      projects = projects.filter((p) => p.category === activeCategory);
    }
    if (q) {
      projects = projects.filter(
        (p) =>
          p.name.toLowerCase().includes(q) ||
          p.description.toLowerCase().includes(q) ||
          p.language.toLowerCase().includes(q) ||
          p.tags?.some((t) => t.toLowerCase().includes(q)),
      );
    }
    return projects;
  }, [activeCategory, q]);

  const enterpriseProjects = filtered.filter((p) => p.category === "enterprise");
  const agentProjects = filtered.filter((p) => p.category === "agent");

  const stats = useMemo(() => {
    const enterprise = MOCK_PROJECTS.filter((p) => p.category === "enterprise");
    const agent = MOCK_PROJECTS.filter((p) => p.category === "agent");
    const totalCommits = MOCK_PROJECTS.reduce((s, p) => s + p.git.commitCount, 0);
    const openPRs = MOCK_PROJECTS.reduce((s, p) => s + (p.git.openPRs || 0), 0);
    const deployed = agent.filter((p) => p.devStatus === "deployed").length;
    return { enterprise: enterprise.length, agent: agent.length, totalCommits, openPRs, deployed };
  }, []);

  // Agents that own projects
  const agentOwners = useMemo(() => {
    const ids = new Set(MOCK_PROJECTS.filter((p) => p.agentId).map((p) => p.agentId!));
    return BUILTIN_PERSONAS.filter((p) => ids.has(p.id));
  }, []);

  const handleOpenProject = useCallback((project: ProjectItem) => {
    setSelectedProject(project);
  }, []);

  const handleBack = useCallback(() => {
    setSelectedProject(null);
  }, []);

  // If a project is selected, show the editor view
  if (selectedProject) {
    return <ProjectEditorView project={selectedProject} onBack={handleBack} />;
  }

  return (
    <div className="flex h-full w-full">
      {/* Left sidebar */}
      <div className="w-56 border-r flex flex-col">
        <div className="px-4 py-3 border-b">
          <h2 className="text-sm font-semibold flex items-center gap-2">
            <Boxes className="size-4 text-primary" />
            资产管理
          </h2>
          <p className="text-[11px] text-muted-foreground mt-0.5">软件项目与智能体仓库</p>
        </div>
        <ScrollArea className="flex-1">
          {/* Category filter */}
          <div className="p-2">
            {CATEGORIES.map((cat) => {
              const count =
                cat.key === "all"
                  ? MOCK_PROJECTS.length
                  : MOCK_PROJECTS.filter((p) => p.category === cat.key).length;
              return (
                <button
                  key={cat.key}
                  type="button"
                  className={cn(
                    "flex items-center gap-2.5 w-full rounded-md px-3 py-2 text-xs transition-colors",
                    activeCategory === cat.key
                      ? "bg-primary/10 text-primary font-medium"
                      : "text-muted-foreground hover:bg-foreground/[0.04] hover:text-foreground",
                  )}
                  onClick={() => setActiveCategory(cat.key)}
                >
                  {cat.key === "enterprise" ? (
                    <Server className="size-4" />
                  ) : cat.key === "agent" ? (
                    <Rocket className="size-4" />
                  ) : (
                    <Boxes className="size-4" />
                  )}
                  <span className="flex-1 text-left">{cat.label}</span>
                  <span
                    className={cn(
                      "text-[10px] rounded-full px-1.5 py-0.5 min-w-[20px] text-center",
                      activeCategory === cat.key ? "bg-primary/20 text-primary" : "bg-muted text-muted-foreground",
                    )}
                  >
                    {count}
                  </span>
                </button>
              );
            })}
          </div>

          {/* Stats */}
          <div className="px-4 py-3 border-t space-y-2">
            <div className="text-[11px] font-medium text-muted-foreground mb-2">概览</div>
            <StatItem label="企业项目" value={stats.enterprise} icon={Server} />
            <StatItem label="智能体项目" value={stats.agent} icon={Rocket} />
            <StatItem label="总提交数" value={stats.totalCommits.toLocaleString()} icon={GitCommit} />
            <StatItem label="待审 PR" value={stats.openPRs} icon={GitPullRequest} />
            <StatItem label="已部署" value={stats.deployed} icon={CheckCircle2} />
          </div>

          {/* Agent owners */}
          <div className="px-4 py-3 border-t">
            <div className="text-[11px] font-medium text-muted-foreground mb-2 flex items-center gap-1.5">
              <GitFork className="size-3" />
              智能体开发者
            </div>
            <div className="space-y-1.5">
              {agentOwners.map((persona) => {
                const cfg = genConfig(persona.avatar);
                const projCount = MOCK_PROJECTS.filter((p) => p.agentId === persona.id).length;
                return (
                  <div key={persona.id} className="flex items-center gap-2 text-xs text-muted-foreground">
                    <NiceAvatar className="size-5" {...cfg} />
                    <span className="flex-1 truncate">{persona.name}</span>
                    <span className="text-[10px]">{projCount} 项目</span>
                  </div>
                );
              })}
            </div>
          </div>
        </ScrollArea>
      </div>

      {/* Right: project list */}
      <div className="flex-1 flex flex-col">
        {/* Toolbar */}
        <div className="px-4 py-3 border-b flex items-center gap-3">
          <div className="relative flex-1 max-w-md">
            <Search className="absolute left-2.5 top-2.5 size-4 text-muted-foreground" />
            <Input
              placeholder="搜索项目名、描述或标签..."
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              className="pl-8 h-9"
            />
          </div>
          <div className="flex items-center gap-2 text-xs text-muted-foreground">
            <span>{filtered.length} 个项目</span>
            {activeCategory !== "all" && (
              <Button variant="ghost" size="sm" className="h-7 text-xs" onClick={() => setActiveCategory("all")}>
                清除筛选
              </Button>
            )}
          </div>
          <Button size="sm" className="h-8 text-xs gap-1 ml-auto">
            <Plus className="size-3" />
            导入仓库
          </Button>
        </div>

        {/* Project cards */}
        <ScrollArea className="flex-1">
          <div className="p-4 space-y-4">
            {/* Enterprise section */}
            {enterpriseProjects.length > 0 && (activeCategory === "all" || activeCategory === "enterprise") && (
              <div>
                {activeCategory === "all" && (
                  <div className="flex items-center gap-2 mb-3 text-xs text-muted-foreground">
                    <Server className="size-3.5 text-primary" />
                    <span className="font-medium text-primary">企业项目</span>
                    <span className="text-[10px]">({enterpriseProjects.length})</span>
                  </div>
                )}
                <div className="grid grid-cols-1 xl:grid-cols-2 gap-3">
                  {enterpriseProjects.map((p) => (
                    <EnterpriseProjectCard
                      key={p.id}
                      project={p}
                      onClick={() => handleOpenProject(p)}
                    />
                  ))}
                </div>
              </div>
            )}

            {/* Separator */}
            {activeCategory === "all" && enterpriseProjects.length > 0 && agentProjects.length > 0 && (
              <div className="flex items-center gap-3 py-1">
                <div className="flex-1 border-t" />
                <span className="text-[10px] text-muted-foreground">智能体项目</span>
                <div className="flex-1 border-t" />
              </div>
            )}

            {/* Agent section */}
            {agentProjects.length > 0 && (activeCategory === "all" || activeCategory === "agent") && (
              <div>
                {activeCategory === "all" && enterpriseProjects.length === 0 && (
                  <div className="flex items-center gap-2 mb-3 text-xs text-muted-foreground">
                    <Rocket className="size-3.5 text-primary" />
                    <span className="font-medium text-primary">智能体项目</span>
                    <span className="text-[10px]">({agentProjects.length})</span>
                  </div>
                )}
                <div className="grid grid-cols-1 xl:grid-cols-2 gap-3">
                  {agentProjects.map((p) => (
                    <AgentProjectCard key={p.id} project={p} />
                  ))}
                </div>
              </div>
            )}

            {/* Empty state */}
            {filtered.length === 0 && (
              <div className="flex flex-col items-center justify-center py-16 text-muted-foreground">
                <Boxes className="size-10 mb-3 opacity-30" />
                <p className="text-sm">{q ? "未找到匹配的项目" : "暂无项目"}</p>
              </div>
            )}
          </div>
        </ScrollArea>
      </div>
    </div>
  );
}
