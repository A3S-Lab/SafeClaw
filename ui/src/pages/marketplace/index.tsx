/**
 * Agent Marketplace — browse, hire, and publish agents on the agent internet.
 * Features: agent listings, pricing, capabilities, bounty tasks.
 */
import { cn } from "@/lib/utils";
import { BUILTIN_PERSONAS } from "@/lib/builtin-personas";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import NiceAvatar, { genConfig } from "react-nice-avatar";
import { useCallback, useMemo, useState } from "react";
import {
	ArrowLeft,
	ArrowUpRight,
	BadgeCheck,
	Banknote,
	BookOpen,
	Bot,
	Briefcase,
	CheckCircle2,
	Clock,
	Code2,
	Crown,
	DollarSign,
	ExternalLink,
	Eye,
	Filter,
	Flame,
	Globe,
	Handshake,
	Layers,
	MessageCircle,
	Plus,
	Rocket,
	Search,
	Shield,
	ShieldCheck,
	Sparkles,
	Star,
	Store,
	Target,
	TrendingUp,
	Trophy,
	Users,
	Zap,
} from "lucide-react";

// =============================================================================
// Types
// =============================================================================

type AgentTier = "free" | "basic" | "pro" | "enterprise";
type MarketCategory = "all" | "finance" | "risk" | "dev" | "data" | "ops" | "legal";
type BountyStatus = "open" | "in_progress" | "completed";
type BountyDifficulty = "easy" | "medium" | "hard" | "expert";
type TabKey = "browse" | "my_agents" | "bounties";

interface MarketAgent {
	id: string;
	name: string;
	provider: string;
	providerVerified: boolean;
	description: string;
	category: MarketCategory;
	avatar: string;
	tier: AgentTier;
	price: string;
	priceUnit: string;
	rating: number;
	reviews: number;
	hires: number;
	capabilities: string[];
	tags: string[];
	featured?: boolean;
	teeSupported?: boolean;
}

interface MyPublishedAgent {
	id: string;
	personaId: string;
	name: string;
	description: string;
	tier: AgentTier;
	price: string;
	priceUnit: string;
	hires: number;
	revenue: string;
	rating: number;
	reviews: number;
	status: "published" | "draft" | "paused";
}

interface BountyTask {
	id: string;
	title: string;
	description: string;
	reward: string;
	difficulty: BountyDifficulty;
	category: MarketCategory;
	status: BountyStatus;
	poster: string;
	posterVerified: boolean;
	deadline: string;
	applicants: number;
	requirements: string[];
	tags: string[];
}

// =============================================================================
// Config
// =============================================================================

const TIER_CONFIG: Record<AgentTier, { label: string; color: string; icon: typeof Star }> = {
	free: { label: "Free", color: "bg-zinc-100 text-zinc-600 dark:bg-zinc-800 dark:text-zinc-400", icon: Zap },
	basic: { label: "Basic", color: "bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400", icon: Star },
	pro: { label: "Pro", color: "bg-purple-100 text-purple-700 dark:bg-purple-900/30 dark:text-purple-400", icon: Crown },
	enterprise: { label: "Enterprise", color: "bg-amber-100 text-amber-700 dark:bg-amber-900/30 dark:text-amber-400", icon: Shield },
};

const CATEGORY_CONFIG: Record<Exclude<MarketCategory, "all">, { label: string }> = {
	finance: { label: "金融" },
	risk: { label: "风控" },
	dev: { label: "开发" },
	data: { label: "数据" },
	ops: { label: "运维" },
	legal: { label: "法务" },
};

const DIFFICULTY_CONFIG: Record<BountyDifficulty, { label: string; color: string }> = {
	easy: { label: "简单", color: "bg-sky-100 text-sky-700 dark:bg-sky-900/30 dark:text-sky-400" },
	medium: { label: "中等", color: "bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400" },
	hard: { label: "困难", color: "bg-orange-100 text-orange-700 dark:bg-orange-900/30 dark:text-orange-400" },
	expert: { label: "专家", color: "bg-red-100 text-red-700 dark:bg-red-900/30 dark:text-red-400" },
};

const BOUNTY_STATUS_CONFIG: Record<BountyStatus, { label: string; color: string }> = {
	open: { label: "招募中", color: "text-primary" },
	in_progress: { label: "进行中", color: "text-blue-600 dark:text-blue-400" },
	completed: { label: "已完成", color: "text-muted-foreground" },
};

const TABS: { key: TabKey; label: string; icon: typeof Store }[] = [
	{ key: "browse", label: "浏览市场", icon: Store },
	{ key: "my_agents", label: "我的智能体", icon: Bot },
	{ key: "bounties", label: "赏金任务", icon: Trophy },
];

// =============================================================================
// Mock data — marketplace agents
// =============================================================================

const MARKET_AGENTS: MarketAgent[] = [
	{
		id: "ma-1",
		name: "QuantAlpha Pro",
		provider: "DeepQuant Labs",
		providerVerified: true,
		description: "专业量化因子挖掘与策略回测智能体，支持 A 股/港股/美股多市场，内置 200+ 预定义因子库，自动因子组合优化。",
		category: "finance",
		avatar: "quant-researcher",
		tier: "pro",
		price: "¥2,800",
		priceUnit: "/月",
		rating: 4.8,
		reviews: 127,
		hires: 342,
		capabilities: ["多因子挖掘", "IC/IR 分析", "回测引擎", "组合优化", "风险归因"],
		tags: ["量化", "因子", "A股"],
		featured: true,
		teeSupported: true,
	},
	{
		id: "ma-2",
		name: "RiskGuard",
		provider: "SecureAI Inc.",
		providerVerified: true,
		description: "企业级实时风控决策引擎，亚毫秒级响应，支持规则引擎与 ML 模型混合决策，覆盖交易反欺诈、信用评估、AML 筛查。",
		category: "risk",
		avatar: "risk-analyst",
		tier: "enterprise",
		price: "¥8,500",
		priceUnit: "/月",
		rating: 4.9,
		reviews: 89,
		hires: 156,
		capabilities: ["交易反欺诈", "信用评估", "AML 筛查", "规则引擎", "模型监控"],
		tags: ["风控", "反欺诈", "AML"],
		featured: true,
		teeSupported: true,
	},
	{
		id: "ma-3",
		name: "CodePilot X",
		provider: "AgentForge",
		providerVerified: true,
		description: "全栈开发智能体，精通 Rust/TypeScript/Python/Go，支持从需求分析到代码实现、测试、部署全流程。",
		category: "dev",
		avatar: "fullstack-engineer",
		tier: "pro",
		price: "¥1,500",
		priceUnit: "/月",
		rating: 4.7,
		reviews: 312,
		hires: 891,
		capabilities: ["需求分析", "架构设计", "代码实现", "代码审查", "CI/CD"],
		tags: ["全栈", "Rust", "TypeScript"],
		featured: false,
	},
	{
		id: "ma-4",
		name: "DataFlow Agent",
		provider: "PipelineAI",
		providerVerified: false,
		description: "数据工程智能体，自动构建 ETL 管道、数据质量监控、Schema 变更检测，支持 ClickHouse/Spark/Flink。",
		category: "data",
		avatar: "data-engineer",
		tier: "basic",
		price: "¥680",
		priceUnit: "/月",
		rating: 4.5,
		reviews: 67,
		hires: 203,
		capabilities: ["ETL 构建", "数据质量", "Schema 检测", "管道编排", "告警集成"],
		tags: ["数据", "ETL", "ClickHouse"],
	},
	{
		id: "ma-5",
		name: "CompliBot",
		provider: "LegalTech AI",
		providerVerified: true,
		description: "合规审查智能体，自动解读最新监管政策，对比企业制度差异，生成合规改进建议与影响评估报告。",
		category: "legal",
		avatar: "compliance-officer",
		tier: "pro",
		price: "¥3,200",
		priceUnit: "/月",
		rating: 4.6,
		reviews: 45,
		hires: 98,
		capabilities: ["政策解读", "合规差距分析", "影响评估", "报告生成", "制度审查"],
		tags: ["合规", "监管", "法规"],
		teeSupported: true,
	},
	{
		id: "ma-6",
		name: "SRE Guardian",
		provider: "CloudOps Team",
		providerVerified: false,
		description: "SRE 运维智能体，7x24 小时监控告警处理、自动故障诊断与根因分析，支持 K8s/Terraform/Prometheus。",
		category: "ops",
		avatar: "devops-engineer",
		tier: "basic",
		price: "¥980",
		priceUnit: "/月",
		rating: 4.4,
		reviews: 156,
		hires: 412,
		capabilities: ["告警处理", "故障诊断", "根因分析", "容量规划", "变更管理"],
		tags: ["SRE", "K8s", "监控"],
	},
	{
		id: "ma-7",
		name: "LegalEagle",
		provider: "LawAI Corp",
		providerVerified: true,
		description: "法律文档智能体，自动起草/审查合同、条款风险标注、合规对比分析，支持中英双语法律文本处理。",
		category: "legal",
		avatar: "legal-counsel",
		tier: "pro",
		price: "¥2,200",
		priceUnit: "/月",
		rating: 4.7,
		reviews: 78,
		hires: 167,
		capabilities: ["合同起草", "条款审查", "风险标注", "合规对比", "双语处理"],
		tags: ["合同", "法务", "双语"],
		teeSupported: true,
	},
	{
		id: "ma-8",
		name: "InsightMiner",
		provider: "DataVision",
		providerVerified: false,
		description: "数据分析智能体，自动发现数据洞察、生成可视化报表、支持自然语言查询 SQL 转换。",
		category: "data",
		avatar: "data-scientist",
		tier: "free",
		price: "免费",
		priceUnit: "",
		rating: 4.2,
		reviews: 234,
		hires: 1205,
		capabilities: ["自然语言查询", "数据可视化", "异常检测", "趋势分析", "报表生成"],
		tags: ["BI", "可视化", "SQL"],
	},
	{
		id: "ma-9",
		name: "FinAdvisor",
		provider: "WealthTech AI",
		providerVerified: true,
		description: "投资顾问智能体，提供宏观经济分析、行业研究、个股评价，生成结构化投研报告。",
		category: "finance",
		avatar: "financial-analyst",
		tier: "enterprise",
		price: "¥6,800",
		priceUnit: "/月",
		rating: 4.8,
		reviews: 56,
		hires: 89,
		capabilities: ["宏观分析", "行业研究", "个股评价", "投研报告", "风险提示"],
		tags: ["投研", "宏观", "行业"],
		teeSupported: true,
	},
];

// =============================================================================
// Mock data — my published agents
// =============================================================================

const MY_AGENTS: MyPublishedAgent[] = [
	{
		id: "my-1",
		personaId: "quant-researcher",
		name: "SafeClaw 量化研究员",
		description: "多因子研究与策略回测，支持 A 股/港股/美股市场",
		tier: "pro",
		price: "¥2,000",
		priceUnit: "/月",
		hires: 23,
		revenue: "¥46,000",
		rating: 4.7,
		reviews: 15,
		status: "published",
	},
	{
		id: "my-2",
		personaId: "risk-analyst",
		name: "SafeClaw 风险分析师",
		description: "实时风控决策与合规扫描，支持 TEE 安全执行",
		tier: "enterprise",
		price: "¥5,000",
		priceUnit: "/月",
		hires: 8,
		revenue: "¥40,000",
		rating: 4.9,
		reviews: 7,
		status: "published",
	},
	{
		id: "my-3",
		personaId: "data-engineer",
		name: "SafeClaw 数据工程师",
		description: "数据管道构建与质量监控，支持多数据源集成",
		tier: "basic",
		price: "¥500",
		priceUnit: "/月",
		hires: 45,
		revenue: "¥22,500",
		rating: 4.5,
		reviews: 28,
		status: "published",
	},
	{
		id: "my-4",
		personaId: "security-engineer",
		name: "SafeClaw 安全审计员",
		description: "代码安全审计与渗透测试，TEE 环境安全验证",
		tier: "pro",
		price: "¥3,000",
		priceUnit: "/月",
		hires: 0,
		revenue: "¥0",
		rating: 0,
		reviews: 0,
		status: "draft",
	},
];

// =============================================================================
// Mock data — bounty tasks
// =============================================================================

const BOUNTIES: BountyTask[] = [
	{
		id: "b-1",
		title: "构建跨境支付路由优选引擎",
		description: "需要一个智能路由引擎，根据费率、时效、成功率等多维指标实时选择最优支付通道。要求支持 12+ 个通道、实时费率同步、TEE 安全执行。",
		reward: "¥25,000",
		difficulty: "expert",
		category: "finance",
		status: "open",
		poster: "SafeClaw Finance",
		posterVerified: true,
		deadline: "2025-03-15",
		applicants: 7,
		requirements: ["精通 Rust/Go", "支付行业经验", "TEE 开发经验"],
		tags: ["支付", "路由", "TEE"],
	},
	{
		id: "b-2",
		title: "设备指纹采集与关联图谱分析",
		description: "开发设备指纹采集 SDK 与后端关联分析系统，识别多账户关联、设备代理群组，支持实时图谱查询。",
		reward: "¥18,000",
		difficulty: "hard",
		category: "risk",
		status: "open",
		poster: "SecureAI Inc.",
		posterVerified: true,
		deadline: "2025-02-28",
		applicants: 12,
		requirements: ["图数据库经验", "指纹算法", "实时计算"],
		tags: ["指纹", "图谱", "反欺诈"],
	},
	{
		id: "b-3",
		title: "K8s 集群自动弹性伸缩优化",
		description: "基于历史负载模式与实时指标的 K8s HPA/VPA 智能调参系统，目标降低 30% 资源浪费并保证 SLA。",
		reward: "¥12,000",
		difficulty: "hard",
		category: "ops",
		status: "in_progress",
		poster: "CloudOps Team",
		posterVerified: false,
		deadline: "2025-03-01",
		applicants: 5,
		requirements: ["K8s 深度经验", "时序预测", "Python"],
		tags: ["K8s", "弹性伸缩", "成本优化"],
	},
	{
		id: "b-4",
		title: "合规政策变更自动影响评估系统",
		description: "开发监管政策文本的 NLP 解析引擎，自动对比新旧规则差异，生成对企业现有制度的影响评估报告。",
		reward: "¥15,000",
		difficulty: "hard",
		category: "legal",
		status: "open",
		poster: "LegalTech AI",
		posterVerified: true,
		deadline: "2025-03-20",
		applicants: 3,
		requirements: ["NLP 经验", "法规知识", "报告自动生成"],
		tags: ["合规", "NLP", "政策"],
	},
	{
		id: "b-5",
		title: "ClickHouse 实时数据质量告警插件",
		description: "开发 ClickHouse 数据质量实时监控插件，支持空值率、分布漂移、延迟检测，集成飞书/Slack 告警。",
		reward: "¥6,000",
		difficulty: "medium",
		category: "data",
		status: "open",
		poster: "PipelineAI",
		posterVerified: false,
		deadline: "2025-02-20",
		applicants: 8,
		requirements: ["ClickHouse", "数据质量", "告警集成"],
		tags: ["数据质量", "ClickHouse", "监控"],
	},
	{
		id: "b-6",
		title: "React 组件性能审计工具",
		description: "构建 React 组件性能审计智能体，自动检测不必要的重渲染、大 bundle、内存泄漏，并给出优化建议。",
		reward: "¥8,000",
		difficulty: "medium",
		category: "dev",
		status: "in_progress",
		poster: "AgentForge",
		posterVerified: true,
		deadline: "2025-03-10",
		applicants: 15,
		requirements: ["React 性能优化", "AST 分析", "Chrome DevTools"],
		tags: ["React", "性能", "审计"],
	},
	{
		id: "b-7",
		title: "多语言合同条款风险智能标注",
		description: "开发多语言合同条款风险自动标注系统，支持中英日三语，标注高风险条款并提供修改建议。",
		reward: "¥20,000",
		difficulty: "expert",
		category: "legal",
		status: "open",
		poster: "LawAI Corp",
		posterVerified: true,
		deadline: "2025-04-01",
		applicants: 4,
		requirements: ["法律 NLP", "多语言处理", "合同分析"],
		tags: ["合同", "多语言", "NLP"],
	},
	{
		id: "b-8",
		title: "基于 TEE 的隐私计算联邦学习框架",
		description: "在 Intel TDX 环境中实现联邦学习框架，支持多方安全计算、梯度加密聚合、模型完整性验证。",
		reward: "¥35,000",
		difficulty: "expert",
		category: "data",
		status: "open",
		poster: "SafeClaw Research",
		posterVerified: true,
		deadline: "2025-04-30",
		applicants: 2,
		requirements: ["TEE 开发", "联邦学习", "密码学"],
		tags: ["TEE", "联邦学习", "隐私计算"],
	},
];

// =============================================================================
// Stars component
// =============================================================================

function StarRating({ rating }: { rating: number }) {
	return (
		<div className="flex items-center gap-0.5">
			{Array.from({ length: 5 }).map((_, i) => (
				<Star
					key={i}
					className={cn(
						"size-3",
						i < Math.floor(rating)
							? "fill-amber-400 text-amber-400"
							: i < rating
								? "fill-amber-400/50 text-amber-400"
								: "text-zinc-300 dark:text-zinc-600",
					)}
				/>
			))}
			<span className="text-[11px] font-medium ml-1">{rating.toFixed(1)}</span>
		</div>
	);
}

// =============================================================================
// Agent card (browse view)
// =============================================================================

function AgentCard({ agent, onClick }: { agent: MarketAgent; onClick: () => void }) {
	const tierCfg = TIER_CONFIG[agent.tier];
	const TierIcon = tierCfg.icon;
	const persona = BUILTIN_PERSONAS.find((p) => p.id === agent.avatar);
	const avatarCfg = persona ? genConfig(persona.avatar) : null;

	return (
		<div
			className={cn(
				"group rounded-xl border bg-card hover:shadow-lg transition-all cursor-pointer active:scale-[0.99]",
				agent.featured ? "ring-1 ring-primary/20 hover:ring-primary/40" : "hover:border-primary/30",
			)}
			onClick={onClick}
			role="button"
			tabIndex={0}
			onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") onClick(); }}
		>
			{/* Featured badge */}
			{agent.featured && (
				<div className="flex items-center gap-1 px-4 py-1.5 bg-primary/5 rounded-t-xl border-b border-primary/10">
					<Flame className="size-3 text-primary" />
					<span className="text-[10px] text-primary font-medium">Featured</span>
				</div>
			)}

			<div className="p-4">
				{/* Header */}
				<div className="flex items-start gap-3 mb-3">
					<div className="relative shrink-0">
						{avatarCfg ? (
							<NiceAvatar className="size-10 ring-2 ring-border" {...avatarCfg} />
						) : (
							<div className="size-10 rounded-full bg-primary/10 flex items-center justify-center">
								<Bot className="size-5 text-primary" />
							</div>
						)}
						{agent.teeSupported && (
							<div className="absolute -bottom-0.5 -right-0.5 size-4 rounded-full bg-primary flex items-center justify-center ring-2 ring-card">
								<ShieldCheck className="size-2.5 text-white" />
							</div>
						)}
					</div>
					<div className="flex-1 min-w-0">
						<div className="flex items-center gap-1.5">
							<h3 className="text-sm font-bold truncate">{agent.name}</h3>
							<ArrowUpRight className="size-3 text-muted-foreground/0 group-hover:text-muted-foreground transition-colors shrink-0" />
						</div>
						<div className="flex items-center gap-1.5 mt-0.5">
							<span className="text-[10px] text-muted-foreground">{agent.provider}</span>
							{agent.providerVerified && <BadgeCheck className="size-3 text-blue-500" />}
						</div>
					</div>
					<span className={cn("text-[10px] font-medium px-2 py-0.5 rounded-full shrink-0 flex items-center gap-1", tierCfg.color)}>
						<TierIcon className="size-2.5" />
						{tierCfg.label}
					</span>
				</div>

				{/* Description */}
				<p className="text-[11px] text-muted-foreground leading-relaxed mb-3 line-clamp-2">{agent.description}</p>

				{/* Capabilities */}
				<div className="flex flex-wrap gap-1 mb-3">
					{agent.capabilities.slice(0, 4).map((cap) => (
						<span key={cap} className="rounded-md bg-muted px-1.5 py-0.5 text-[9px] font-medium text-muted-foreground">{cap}</span>
					))}
					{agent.capabilities.length > 4 && (
						<span className="text-[9px] text-muted-foreground">+{agent.capabilities.length - 4}</span>
					)}
				</div>

				{/* Footer */}
				<div className="flex items-center justify-between pt-3 border-t">
					<div className="flex items-center gap-3">
						<StarRating rating={agent.rating} />
						<span className="text-[10px] text-muted-foreground">({agent.reviews})</span>
					</div>
					<div className="text-right">
						<span className="text-sm font-bold text-primary">{agent.price}</span>
						<span className="text-[10px] text-muted-foreground">{agent.priceUnit}</span>
					</div>
				</div>

				{/* Stats row */}
				<div className="flex items-center gap-4 mt-2 text-[10px] text-muted-foreground">
					<span className="flex items-center gap-1">
						<Users className="size-3" />
						{agent.hires} 次雇佣
					</span>
					{agent.teeSupported && (
						<span className="flex items-center gap-1 text-primary">
							<ShieldCheck className="size-3" />
							TEE 安全
						</span>
					)}
				</div>
			</div>
		</div>
	);
}

// =============================================================================
// Agent detail view
// =============================================================================

function AgentDetail({ agent, onBack }: { agent: MarketAgent; onBack: () => void }) {
	const tierCfg = TIER_CONFIG[agent.tier];
	const TierIcon = tierCfg.icon;
	const persona = BUILTIN_PERSONAS.find((p) => p.id === agent.avatar);
	const avatarCfg = persona ? genConfig(persona.avatar) : null;

	return (
		<div className="flex flex-col h-full overflow-hidden">
			{/* Header */}
			<div className="flex items-center gap-3 px-6 py-4 border-b shrink-0">
				<button
					type="button"
					className="flex items-center justify-center size-8 rounded-lg border hover:bg-foreground/[0.04] transition-colors"
					onClick={onBack}
					aria-label="Back"
				>
					<ArrowLeft className="size-4" />
				</button>
				<div className="relative shrink-0">
					{avatarCfg ? (
						<NiceAvatar className="size-10 ring-2 ring-border" {...avatarCfg} />
					) : (
						<div className="size-10 rounded-full bg-primary/10 flex items-center justify-center">
							<Bot className="size-5 text-primary" />
						</div>
					)}
				</div>
				<div className="flex-1 min-w-0">
					<div className="flex items-center gap-2">
						<h1 className="text-base font-bold">{agent.name}</h1>
						<span className={cn("text-[10px] font-medium px-2 py-0.5 rounded-full flex items-center gap-1", tierCfg.color)}>
							<TierIcon className="size-2.5" />
							{tierCfg.label}
						</span>
						{agent.teeSupported && (
							<Badge variant="outline" className="text-[9px] px-1.5 py-0 h-4 bg-primary/10 text-primary border-primary/20">
								<ShieldCheck className="size-2.5 mr-0.5" />TEE
							</Badge>
						)}
					</div>
					<div className="flex items-center gap-2 mt-0.5">
						<span className="text-xs text-muted-foreground">{agent.provider}</span>
						{agent.providerVerified && <BadgeCheck className="size-3 text-blue-500" />}
					</div>
				</div>
				<div className="text-right shrink-0">
					<div className="text-lg font-bold text-primary">{agent.price}<span className="text-xs text-muted-foreground font-normal">{agent.priceUnit}</span></div>
					<Button size="sm" className="mt-1 h-8 text-xs gap-1">
						<Handshake className="size-3" />
						雇佣智能体
					</Button>
				</div>
			</div>

			<ScrollArea className="flex-1">
				<div className="px-6 py-5 space-y-6">
					{/* Stats */}
					<div className="grid grid-cols-4 gap-3">
						{[
							{ label: "综合评分", value: agent.rating.toFixed(1), icon: Star },
							{ label: "用户评价", value: `${agent.reviews} 条`, icon: MessageCircle },
							{ label: "累计雇佣", value: `${agent.hires} 次`, icon: Users },
							{ label: "能力数量", value: `${agent.capabilities.length} 项`, icon: Layers },
						].map((s) => (
							<div key={s.label} className="rounded-lg border bg-card px-4 py-3 text-center">
								<s.icon className="size-4 text-primary mx-auto mb-1" />
								<div className="text-lg font-bold">{s.value}</div>
								<div className="text-[10px] text-muted-foreground">{s.label}</div>
							</div>
						))}
					</div>

					{/* Description */}
					<div>
						<h2 className="text-sm font-bold mb-2 flex items-center gap-2">
							<BookOpen className="size-4 text-primary" />详细介绍
						</h2>
						<p className="text-xs text-muted-foreground leading-relaxed">{agent.description}</p>
					</div>

					{/* Capabilities */}
					<div>
						<h2 className="text-sm font-bold mb-3 flex items-center gap-2">
							<Sparkles className="size-4 text-primary" />核心能力
						</h2>
						<div className="grid grid-cols-2 gap-2">
							{agent.capabilities.map((cap) => (
								<div key={cap} className="flex items-center gap-2 rounded-lg border bg-card px-3 py-2">
									<CheckCircle2 className="size-3.5 text-primary shrink-0" />
									<span className="text-xs">{cap}</span>
								</div>
							))}
						</div>
					</div>

					{/* Tags */}
					<div className="flex flex-wrap gap-1.5">
						{agent.tags.map((tag) => (
							<Badge key={tag} variant="secondary" className="text-[10px] px-2 py-0.5">{tag}</Badge>
						))}
					</div>
				</div>
			</ScrollArea>
		</div>
	);
}

// =============================================================================
// My agents tab
// =============================================================================

function MyAgentsTab() {
	const totalRevenue = MY_AGENTS.reduce((sum, a) => {
		const num = Number.parseFloat(a.revenue.replace(/[¥,]/g, ""));
		return sum + (Number.isNaN(num) ? 0 : num);
	}, 0);
	const totalHires = MY_AGENTS.reduce((s, a) => s + a.hires, 0);
	const publishedCount = MY_AGENTS.filter((a) => a.status === "published").length;

	return (
		<div className="flex flex-col h-full">
			{/* Stats bar */}
			<div className="grid grid-cols-3 gap-3 px-6 py-4 border-b shrink-0">
				{[
					{ label: "累计收入", value: `¥${totalRevenue.toLocaleString()}`, icon: DollarSign, color: "text-primary" },
					{ label: "总雇佣次数", value: `${totalHires} 次`, icon: Users, color: "text-blue-500" },
					{ label: "已发布", value: `${publishedCount} 个`, icon: Rocket, color: "text-purple-500" },
				].map((s) => (
					<div key={s.label} className="flex items-center gap-3 rounded-lg border bg-card px-4 py-3">
						<s.icon className={cn("size-8", s.color)} />
						<div>
							<div className="text-lg font-bold">{s.value}</div>
							<div className="text-[10px] text-muted-foreground">{s.label}</div>
						</div>
					</div>
				))}
			</div>

			{/* Agent list */}
			<ScrollArea className="flex-1">
				<div className="px-6 py-4 space-y-3">
					{MY_AGENTS.map((agent) => {
						const persona = BUILTIN_PERSONAS.find((p) => p.id === agent.personaId);
						const avatarCfg = persona ? genConfig(persona.avatar) : null;
						const tierCfg = TIER_CONFIG[agent.tier];
						const statusColor = agent.status === "published" ? "text-primary" : agent.status === "draft" ? "text-muted-foreground" : "text-amber-600 dark:text-amber-400";
						const statusLabel = agent.status === "published" ? "已发布" : agent.status === "draft" ? "草稿" : "已暂停";

						return (
							<div key={agent.id} className="rounded-xl border bg-card p-4 hover:shadow-sm transition-shadow">
								<div className="flex items-start gap-3">
									{avatarCfg && <NiceAvatar className="size-10 ring-2 ring-border shrink-0" {...avatarCfg} />}
									<div className="flex-1 min-w-0">
										<div className="flex items-center gap-2">
											<h3 className="text-sm font-bold truncate">{agent.name}</h3>
											<span className={cn("text-[10px] font-medium px-2 py-0.5 rounded-full", tierCfg.color)}>
												{tierCfg.label}
											</span>
											<span className={cn("text-[10px] font-medium ml-auto", statusColor)}>{statusLabel}</span>
										</div>
										<p className="text-[11px] text-muted-foreground mt-0.5">{agent.description}</p>
									</div>
								</div>

								<div className="grid grid-cols-4 gap-3 mt-3 pt-3 border-t">
									<div className="text-center">
										<div className="text-sm font-bold">{agent.price}<span className="text-[10px] text-muted-foreground font-normal">{agent.priceUnit}</span></div>
										<div className="text-[9px] text-muted-foreground">定价</div>
									</div>
									<div className="text-center">
										<div className="text-sm font-bold">{agent.hires}</div>
										<div className="text-[9px] text-muted-foreground">雇佣次数</div>
									</div>
									<div className="text-center">
										<div className="text-sm font-bold">{agent.revenue}</div>
										<div className="text-[9px] text-muted-foreground">收入</div>
									</div>
									<div className="text-center">
										{agent.rating > 0 ? <StarRating rating={agent.rating} /> : <span className="text-[11px] text-muted-foreground">暂无评分</span>}
										<div className="text-[9px] text-muted-foreground mt-0.5">{agent.reviews} 条评价</div>
									</div>
								</div>
							</div>
						);
					})}
				</div>
			</ScrollArea>
		</div>
	);
}

// =============================================================================
// Bounty card
// =============================================================================

function BountyCard({ bounty }: { bounty: BountyTask }) {
	const diffCfg = DIFFICULTY_CONFIG[bounty.difficulty];
	const statusCfg = BOUNTY_STATUS_CONFIG[bounty.status];

	return (
		<div className={cn(
			"rounded-xl border bg-card p-4 hover:shadow-sm transition-shadow",
			bounty.status === "completed" && "opacity-60",
		)}>
			{/* Header */}
			<div className="flex items-start gap-2 mb-2">
				<Target className="size-4 text-primary shrink-0 mt-0.5" />
				<div className="flex-1 min-w-0">
					<h3 className="text-sm font-bold">{bounty.title}</h3>
					<div className="flex items-center gap-2 mt-1">
						<span className={cn("text-[10px] font-medium px-2 py-0.5 rounded-full", diffCfg.color)}>{diffCfg.label}</span>
						<span className={cn("text-[10px] font-medium", statusCfg.color)}>{statusCfg.label}</span>
						<span className="text-[10px] text-muted-foreground flex items-center gap-1">
							<Clock className="size-3" />截止 {bounty.deadline}
						</span>
					</div>
				</div>
				<div className="text-right shrink-0">
					<div className="text-base font-bold text-primary flex items-center gap-1">
						<Banknote className="size-4" />
						{bounty.reward}
					</div>
				</div>
			</div>

			{/* Description */}
			<p className="text-[11px] text-muted-foreground leading-relaxed mb-3 line-clamp-2">{bounty.description}</p>

			{/* Requirements */}
			<div className="flex flex-wrap gap-1 mb-3">
				{bounty.requirements.map((req) => (
					<span key={req} className="rounded-md bg-primary/5 border border-primary/10 px-1.5 py-0.5 text-[9px] font-medium text-primary">{req}</span>
				))}
			</div>

			{/* Footer */}
			<div className="flex items-center justify-between pt-2.5 border-t">
				<div className="flex items-center gap-2 text-[10px] text-muted-foreground">
					<span className="flex items-center gap-1">
						{bounty.poster}
						{bounty.posterVerified && <BadgeCheck className="size-3 text-blue-500" />}
					</span>
				</div>
				<div className="flex items-center gap-3 text-[10px] text-muted-foreground">
					<span className="flex items-center gap-1">
						<Users className="size-3" />
						{bounty.applicants} 人申请
					</span>
					{bounty.status === "open" && (
						<Button size="sm" variant="outline" className="h-6 text-[10px] px-2 gap-1">
							<Rocket className="size-3" />
							接取任务
						</Button>
					)}
				</div>
			</div>
		</div>
	);
}

// =============================================================================
// Main page
// =============================================================================

export default function MarketplacePage() {
	const [activeTab, setActiveTab] = useState<TabKey>("browse");
	const [search, setSearch] = useState("");
	const [categoryFilter, setCategoryFilter] = useState<MarketCategory>("all");
	const [selectedAgent, setSelectedAgent] = useState<MarketAgent | null>(null);
	const [bountyFilter, setBountyFilter] = useState<BountyStatus | "all">("all");

	const filteredAgents = useMemo(() => {
		return MARKET_AGENTS.filter((a) => {
			if (categoryFilter !== "all" && a.category !== categoryFilter) return false;
			if (search) {
				const q = search.toLowerCase();
				return a.name.toLowerCase().includes(q) || a.description.toLowerCase().includes(q) || a.provider.toLowerCase().includes(q);
			}
			return true;
		});
	}, [search, categoryFilter]);

	const filteredBounties = useMemo(() => {
		return BOUNTIES.filter((b) => {
			if (bountyFilter !== "all" && b.status !== bountyFilter) return false;
			if (search) {
				const q = search.toLowerCase();
				return b.title.toLowerCase().includes(q) || b.description.toLowerCase().includes(q);
			}
			return true;
		});
	}, [search, bountyFilter]);

	const marketStats = useMemo(() => ({
		totalAgents: MARKET_AGENTS.length,
		totalBounties: BOUNTIES.filter((b) => b.status === "open").length,
		totalReward: BOUNTIES.filter((b) => b.status === "open").reduce((sum, b) => {
			const num = Number.parseFloat(b.reward.replace(/[¥,]/g, ""));
			return sum + (Number.isNaN(num) ? 0 : num);
		}, 0),
	}), []);

	// If viewing agent detail
	if (selectedAgent) {
		return <AgentDetail agent={selectedAgent} onBack={() => setSelectedAgent(null)} />;
	}

	return (
		<div className="flex flex-col h-full overflow-hidden">
			{/* Header */}
			<div className="flex items-center gap-3 px-6 py-4 border-b shrink-0">
				<div className="flex items-center justify-center size-8 rounded-lg bg-primary/10">
					<Globe className="size-4 text-primary" />
				</div>
				<div className="flex-1">
					<h1 className="text-base font-bold">智能体市场</h1>
					<p className="text-xs text-muted-foreground">
						{marketStats.totalAgents} 个智能体可雇佣 · {marketStats.totalBounties} 个赏金任务 · 总赏金池 ¥{marketStats.totalReward.toLocaleString()}
					</p>
				</div>
				<div className="relative">
					<Search className="absolute left-2.5 top-1/2 -translate-y-1/2 size-3.5 text-muted-foreground" />
					<input
						type="text"
						placeholder="搜索智能体、服务商或任务..."
						className="h-8 rounded-lg border bg-background pl-8 pr-3 text-xs focus:outline-none focus:ring-1 focus:ring-primary w-[240px]"
						value={search}
						onChange={(e) => setSearch(e.target.value)}
					/>
				</div>
			</div>

			{/* Tabs */}
			<div className="flex items-center gap-1 px-6 py-2 border-b shrink-0">
				{TABS.map((tab) => (
					<button
						key={tab.key}
						type="button"
						className={cn(
							"flex items-center gap-1.5 rounded-lg px-3 py-1.5 text-xs font-medium transition-colors",
							activeTab === tab.key
								? "bg-primary text-primary-foreground"
								: "text-muted-foreground hover:bg-foreground/[0.04]",
						)}
						onClick={() => setActiveTab(tab.key)}
					>
						<tab.icon className="size-3.5" />
						{tab.label}
					</button>
				))}
			</div>

			{/* Content */}
			{activeTab === "browse" && (
				<>
					{/* Category filter */}
					<div className="flex items-center gap-1.5 px-6 py-2.5 border-b shrink-0 overflow-x-auto">
						<button
							type="button"
							className={cn(
								"rounded-full px-3 py-1 text-[11px] font-medium transition-colors whitespace-nowrap",
								categoryFilter === "all"
									? "bg-primary text-primary-foreground"
									: "bg-muted text-muted-foreground hover:bg-foreground/[0.06]",
							)}
							onClick={() => setCategoryFilter("all")}
						>
							全部 <span className={cn("text-[9px]", categoryFilter === "all" ? "text-primary-foreground/70" : "text-muted-foreground/60")}>{MARKET_AGENTS.length}</span>
						</button>
						{Object.entries(CATEGORY_CONFIG).map(([key, cfg]) => {
							const count = MARKET_AGENTS.filter((a) => a.category === key).length;
							if (count === 0) return null;
							return (
								<button
									key={key}
									type="button"
									className={cn(
										"rounded-full px-3 py-1 text-[11px] font-medium transition-colors whitespace-nowrap",
										categoryFilter === key
											? "bg-primary text-primary-foreground"
											: "bg-muted text-muted-foreground hover:bg-foreground/[0.06]",
									)}
									onClick={() => setCategoryFilter(key as MarketCategory)}
								>
									{cfg.label} <span className={cn("text-[9px]", categoryFilter === key ? "text-primary-foreground/70" : "text-muted-foreground/60")}>{count}</span>
								</button>
							);
						})}
					</div>

					{/* Agent grid */}
					<ScrollArea className="flex-1">
						<div className="px-6 py-5">
							{/* Featured section */}
							{categoryFilter === "all" && !search && (
								<div className="mb-5">
									<h2 className="text-xs font-bold mb-3 flex items-center gap-2 text-muted-foreground">
										<Flame className="size-3.5 text-primary" />
										<span className="text-primary">精选推荐</span>
									</h2>
									<div className="grid grid-cols-2 xl:grid-cols-3 gap-4">
										{MARKET_AGENTS.filter((a) => a.featured).map((agent) => (
											<AgentCard key={agent.id} agent={agent} onClick={() => setSelectedAgent(agent)} />
										))}
									</div>
								</div>
							)}

							{/* All agents */}
							<div>
								{categoryFilter === "all" && !search && (
									<h2 className="text-xs font-bold mb-3 flex items-center gap-2 text-muted-foreground">
										<Layers className="size-3.5 text-primary" />
										<span className="text-primary">全部智能体</span>
									</h2>
								)}
								<div className="grid grid-cols-2 xl:grid-cols-3 gap-4">
									{filteredAgents.filter((a) => search || categoryFilter !== "all" || !a.featured).map((agent) => (
										<AgentCard key={agent.id} agent={agent} onClick={() => setSelectedAgent(agent)} />
									))}
								</div>
								{filteredAgents.length === 0 && (
									<div className="flex flex-col items-center justify-center py-20 text-muted-foreground">
										<Search className="size-10 mb-3 opacity-30" />
										<p className="text-sm">没有找到匹配的智能体</p>
									</div>
								)}
							</div>
						</div>
					</ScrollArea>
				</>
			)}

			{activeTab === "my_agents" && <MyAgentsTab />}

			{activeTab === "bounties" && (
				<>
					{/* Bounty filter */}
					<div className="flex items-center gap-1.5 px-6 py-2.5 border-b shrink-0">
						{[
							{ key: "all" as const, label: "全部" },
							{ key: "open" as const, label: "招募中" },
							{ key: "in_progress" as const, label: "进行中" },
							{ key: "completed" as const, label: "已完成" },
						].map((f) => (
							<button
								key={f.key}
								type="button"
								className={cn(
									"rounded-full px-3 py-1 text-[11px] font-medium transition-colors",
									bountyFilter === f.key
										? "bg-primary text-primary-foreground"
										: "bg-muted text-muted-foreground hover:bg-foreground/[0.06]",
								)}
								onClick={() => setBountyFilter(f.key)}
							>
								{f.label}
							</button>
						))}
					</div>

					<ScrollArea className="flex-1">
						<div className="px-6 py-5 space-y-3">
							{filteredBounties.map((bounty) => (
								<BountyCard key={bounty.id} bounty={bounty} />
							))}
							{filteredBounties.length === 0 && (
								<div className="flex flex-col items-center justify-center py-20 text-muted-foreground">
									<Trophy className="size-10 mb-3 opacity-30" />
									<p className="text-sm">没有找到匹配的赏金任务</p>
								</div>
							)}
						</div>
					</ScrollArea>
				</>
			)}
		</div>
	);
}
