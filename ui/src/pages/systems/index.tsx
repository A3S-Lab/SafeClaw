/**
 * Enterprise Systems page — browse and open internal business systems
 * developed by AI agents.
 */
import { cn } from "@/lib/utils";
import { BUILTIN_PERSONAS } from "@/lib/builtin-personas";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Badge } from "@/components/ui/badge";
import NiceAvatar, { genConfig } from "react-nice-avatar";
import {
  ArrowLeft,
  ArrowUpRight,
  BarChart3,
  Briefcase,
  Building2,
  CheckCircle2,
  CircleDollarSign,
  Clock,
  CreditCard,
  FileBarChart,
  GitBranch,
  Globe,
  HandCoins,
  Layers,
  LayoutGrid,
  Loader2,
  Lock,
  Receipt,
  Search,
  Server,
  Shield,
  ShieldCheck,
  Users,
  Wallet,
} from "lucide-react";
import { useCallback, useMemo, useState } from "react";

// =============================================================================
// Types
// =============================================================================

type SystemStatus = "running" | "deploying" | "maintenance" | "offline";
type SystemCategory = "finance" | "risk" | "crm" | "compliance" | "data" | "internal";

interface EnterpriseSystem {
  id: string;
  name: string;
  description: string;
  category: SystemCategory;
  status: SystemStatus;
  icon: typeof CreditCard;
  iconColor: string;
  iconBg: string;
  /** Agents that built / maintain this system */
  agentIds: string[];
  version: string;
  lastDeploy: string;
  uptime: string;
  /** Tech stack tags */
  stack: string[];
  /** System-specific metrics */
  metrics: { label: string; value: string }[];
  /** Modules / features within the system */
  modules: SystemModule[];
}

interface SystemModule {
  name: string;
  description: string;
  status: "active" | "beta" | "planned";
  route?: string;
}

// =============================================================================
// Category config
// =============================================================================

const CATEGORY_CONFIG: Record<SystemCategory, { label: string; color: string }> = {
  finance: { label: "财务", color: "bg-primary/10 text-primary border-primary/20" },
  risk: { label: "风控", color: "bg-red-500/10 text-red-600 dark:text-red-400 border-red-500/20" },
  crm: { label: "CRM", color: "bg-blue-500/10 text-blue-600 dark:text-blue-400 border-blue-500/20" },
  compliance: { label: "合规", color: "bg-amber-500/10 text-amber-600 dark:text-amber-400 border-amber-500/20" },
  data: { label: "数据", color: "bg-purple-500/10 text-purple-600 dark:text-purple-400 border-purple-500/20" },
  internal: { label: "内部", color: "bg-zinc-500/10 text-zinc-600 dark:text-zinc-400 border-zinc-500/20" },
};

const STATUS_CONFIG: Record<SystemStatus, { label: string; color: string; dot: string }> = {
  running: { label: "运行中", color: "text-primary", dot: "bg-primary" },
  deploying: { label: "部署中", color: "text-blue-600 dark:text-blue-400", dot: "bg-blue-500 animate-pulse" },
  maintenance: { label: "维护中", color: "text-amber-600 dark:text-amber-400", dot: "bg-amber-500" },
  offline: { label: "已下线", color: "text-muted-foreground", dot: "bg-zinc-400" },
};

// =============================================================================
// Mock enterprise systems
// =============================================================================

const SYSTEMS: EnterpriseSystem[] = [
  {
    id: "finance-core",
    name: "财务核心系统",
    description: "企业核心财务管理平台，涵盖总账、应收应付、费用报销、税务管理等全流程自动化财务处理。",
    category: "finance",
    status: "running",
    icon: CircleDollarSign,
    iconColor: "text-primary",
    iconBg: "bg-primary/10",
    agentIds: ["financial-analyst", "fullstack-engineer"],
    version: "3.2.1",
    lastDeploy: "2 小时前",
    uptime: "99.97%",
    stack: ["React", "Rust", "PostgreSQL", "Redis", "TEE"],
    metrics: [
      { label: "本月处理", value: "¥3.8 亿" },
      { label: "自动对账率", value: "95.7%" },
      { label: "平均审批时效", value: "2.1 小时" },
      { label: "异常检测", value: "12 条" },
    ],
    modules: [
      { name: "总账管理", description: "多账簿、多币种科目管理与自动记账", status: "active" },
      { name: "应收应付", description: "发票管理、账龄分析、自动催收", status: "active" },
      { name: "费用报销", description: "OCR 识别发票、智能审批流、合规校验", status: "active" },
      { name: "税务管理", description: "增值税自动计算、纳税申报表生成", status: "active" },
      { name: "智能对账", description: "AI 驱动的银行流水与发票自动匹配", status: "active" },
      { name: "资金预测", description: "基于历史数据的现金流预测模型", status: "beta" },
    ],
  },
  {
    id: "payment-gateway",
    name: "支付结算平台",
    description: "跨境支付核心平台，支持多渠道收付款、实时汇率、路由优选、TEE 安全支付执行。",
    category: "finance",
    status: "running",
    icon: CreditCard,
    iconColor: "text-blue-500",
    iconBg: "bg-blue-500/10",
    agentIds: ["fullstack-engineer", "devops-engineer", "security-engineer"],
    version: "5.1.0",
    lastDeploy: "昨天 18:30",
    uptime: "99.99%",
    stack: ["React", "Go", "Kafka", "ClickHouse", "TEE", "HSM"],
    metrics: [
      { label: "日交易量", value: "42,891 笔" },
      { label: "成功率", value: "99.7%" },
      { label: "P99 延迟", value: "47ms" },
      { label: "支持币种", value: "12 种" },
    ],
    modules: [
      { name: "收单网关", description: "多渠道支付接入：银行卡、电子钱包、银行转账", status: "active" },
      { name: "跨境付款", description: "SWIFT、本地清算网络、实时汇率定价", status: "active" },
      { name: "交易路由", description: "智能渠道路由、费率优选、容灾切换", status: "active" },
      { name: "清结算", description: "T+0/T+1 清算、手续费分账、对账单生成", status: "active" },
      { name: "虚拟账户", description: "多币种虚拟收款账户", status: "beta" },
      { name: "加密资产", description: "稳定币支付通道（USDT/USDC）", status: "planned" },
    ],
  },
  {
    id: "risk-engine",
    name: "风控决策引擎",
    description: "实时风控决策平台，覆盖交易反欺诈、信用评估、AML 筛查、模型监控全链路。",
    category: "risk",
    status: "running",
    icon: Shield,
    iconColor: "text-red-500",
    iconBg: "bg-red-500/10",
    agentIds: ["risk-analyst", "data-scientist", "fullstack-engineer"],
    version: "4.0.3",
    lastDeploy: "3 天前",
    uptime: "99.98%",
    stack: ["Rust", "Python", "Flink", "Redis", "TensorFlow", "TEE"],
    metrics: [
      { label: "日评估量", value: "129 万次" },
      { label: "P99 延迟", value: "12.8ms" },
      { label: "拦截率", value: "0.23%" },
      { label: "误报率", value: "3.2%" },
    ],
    modules: [
      { name: "交易反欺诈", description: "实时特征计算 + ML 模型评分 + 规则引擎", status: "active" },
      { name: "信用评估", description: "XGBoost 评分模型（AUC 0.79）+ SHAP 解释", status: "active" },
      { name: "AML 筛查", description: "PEP 名单、制裁名单、高风险区域筛查", status: "active" },
      { name: "模型监控", description: "PSI 漂移检测、AUC 衰减告警、自动重训练触发", status: "active" },
      { name: "设备指纹", description: "多维度设备特征采集与关联分析", status: "beta" },
      { name: "图谱分析", description: "账户关联图谱与团伙欺诈检测", status: "planned" },
    ],
  },
  {
    id: "crm-platform",
    name: "客户关系管理系统",
    description: "全生命周期客户管理平台，从获客、转化到服务、留存的智能化客户运营体系。",
    category: "crm",
    status: "running",
    icon: Users,
    iconColor: "text-indigo-500",
    iconBg: "bg-indigo-500/10",
    agentIds: ["product-manager", "fullstack-engineer", "data-scientist"],
    version: "2.5.0",
    lastDeploy: "5 天前",
    uptime: "99.95%",
    stack: ["React", "Node.js", "PostgreSQL", "Elasticsearch", "Redis"],
    metrics: [
      { label: "活跃商户", value: "1,247 家" },
      { label: "本月新增", value: "89 家" },
      { label: "NPS 评分", value: "72" },
      { label: "工单解决率", value: "94.3%" },
    ],
    modules: [
      { name: "客户档案", description: "商户 360 度画像、KYC 资料、交易概览", status: "active" },
      { name: "销售漏斗", description: "线索管理、商机跟踪、转化分析", status: "active" },
      { name: "工单系统", description: "智能工单分配、SLA 监控、满意度调查", status: "active" },
      { name: "客户分群", description: "RFM 模型分群、个性化营销策略", status: "active" },
      { name: "智能外呼", description: "AI 外呼助手、自动回访、语音质检", status: "beta" },
      { name: "流失预警", description: "基于行为数据的客户流失预测模型", status: "planned" },
    ],
  },
  {
    id: "compliance-hub",
    name: "合规管理中心",
    description: "企业合规一站式管理平台，涵盖监管报送、内审管理、政策追踪、合规培训。",
    category: "compliance",
    status: "running",
    icon: ShieldCheck,
    iconColor: "text-amber-500",
    iconBg: "bg-amber-500/10",
    agentIds: ["compliance-officer", "legal-counsel", "fullstack-engineer"],
    version: "1.8.2",
    lastDeploy: "1 周前",
    uptime: "99.93%",
    stack: ["React", "Rust", "PostgreSQL", "MinIO", "TEE"],
    metrics: [
      { label: "监管报表", value: "23 份/月" },
      { label: "合规通过率", value: "98.2%" },
      { label: "未决事项", value: "3 件" },
      { label: "政策更新", value: "7 条" },
    ],
    modules: [
      { name: "监管报送", description: "央行、银保监会报表自动生成与提交", status: "active" },
      { name: "AML 管理", description: "反洗钱案件管理、可疑交易报告", status: "active" },
      { name: "内审管理", description: "审计计划、检查清单、整改跟踪", status: "active" },
      { name: "政策追踪", description: "监管政策变更自动追踪与影响分析", status: "active" },
      { name: "合规培训", description: "课程管理、考试评估、学习记录", status: "beta" },
    ],
  },
  {
    id: "data-platform",
    name: "数据中台",
    description: "企业级数据资产管理与分析平台，统一数据源、数据治理、报表 BI 与机器学习工作台。",
    category: "data",
    status: "running",
    icon: BarChart3,
    iconColor: "text-purple-500",
    iconBg: "bg-purple-500/10",
    agentIds: ["data-engineer", "data-scientist", "fullstack-engineer"],
    version: "2.1.0",
    lastDeploy: "4 天前",
    uptime: "99.96%",
    stack: ["React", "Python", "Flink", "ClickHouse", "Spark", "Airflow"],
    metrics: [
      { label: "数据表", value: "1,842 张" },
      { label: "日任务数", value: "3,271 个" },
      { label: "成功率", value: "99.4%" },
      { label: "数据量", value: "47.2 TB" },
    ],
    modules: [
      { name: "数据集成", description: "MySQL、Kafka、API 等多源数据接入", status: "active" },
      { name: "数据治理", description: "元数据管理、数据血缘、质量规则", status: "active" },
      { name: "BI 报表", description: "拖拽式报表构建、自动刷新、权限管理", status: "active" },
      { name: "数据市场", description: "数据资产目录、申请审批、使用统计", status: "active" },
      { name: "ML 工作台", description: "特征工程、模型训练、在线推理", status: "beta" },
    ],
  },
  {
    id: "supplier-portal",
    name: "供应商管理门户",
    description: "供应商全生命周期管理，涵盖准入评审、采购招标、合同管理、绩效评估。",
    category: "internal",
    status: "deploying",
    icon: HandCoins,
    iconColor: "text-teal-500",
    iconBg: "bg-teal-500/10",
    agentIds: ["financial-analyst", "product-manager", "legal-counsel"],
    version: "1.2.0-rc.1",
    lastDeploy: "部署中...",
    uptime: "—",
    stack: ["React", "Node.js", "PostgreSQL", "S3"],
    metrics: [
      { label: "供应商", value: "218 家" },
      { label: "待审合同", value: "7 份" },
      { label: "本月采购", value: "¥892 万" },
      { label: "平均账期", value: "45 天" },
    ],
    modules: [
      { name: "供应商准入", description: "资质审核、背调报告、评审打分", status: "active" },
      { name: "采购管理", description: "需求汇总、比价招标、订单跟踪", status: "active" },
      { name: "合同管理", description: "合同模板、电子签章、到期提醒", status: "active" },
      { name: "绩效评估", description: "交期达成率、质量评分、综合排名", status: "beta" },
    ],
  },
  {
    id: "invoice-system",
    name: "电子发票系统",
    description: "全流程电子发票管理，支持发票开具、OCR 验真、进项认证、税务合规。",
    category: "finance",
    status: "running",
    icon: Receipt,
    iconColor: "text-orange-500",
    iconBg: "bg-orange-500/10",
    agentIds: ["financial-analyst", "fullstack-engineer"],
    version: "2.0.1",
    lastDeploy: "6 天前",
    uptime: "99.94%",
    stack: ["React", "Go", "MySQL", "OCR", "国税接口"],
    metrics: [
      { label: "本月开票", value: "4,821 张" },
      { label: "OCR 识别率", value: "99.1%" },
      { label: "验真通过率", value: "99.8%" },
      { label: "进项认证", value: "2,130 张" },
    ],
    modules: [
      { name: "发票开具", description: "增值税专票/普票在线开具与打印", status: "active" },
      { name: "OCR 验真", description: "发票图片识别、税局接口验真查重", status: "active" },
      { name: "进项管理", description: "进项发票认证、抵扣计算、到期提醒", status: "active" },
      { name: "电子归档", description: "发票电子化存档、合规检索", status: "active" },
    ],
  },
];

// =============================================================================
// Helpers
// =============================================================================

function getPersona(agentId: string) {
  return BUILTIN_PERSONAS.find((p) => p.id === agentId);
}

// =============================================================================
// System card (grid view)
// =============================================================================

function SystemCard({ system, onClick }: { system: EnterpriseSystem; onClick: () => void }) {
  const Icon = system.icon;
  const statusCfg = STATUS_CONFIG[system.status];
  const catCfg = CATEGORY_CONFIG[system.category];

  return (
    <div
      className="group rounded-xl border bg-card hover:shadow-lg hover:border-primary/30 transition-all cursor-pointer active:scale-[0.99]"
      onClick={onClick}
      role="button"
      tabIndex={0}
      onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") onClick(); }}
    >
      {/* Header */}
      <div className="px-4 pt-4 pb-3">
        <div className="flex items-start gap-3">
          <div className={cn("flex items-center justify-center size-10 rounded-xl shrink-0", system.iconBg)}>
            <Icon className={cn("size-5", system.iconColor)} />
          </div>
          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2">
              <h3 className="text-sm font-bold truncate">{system.name}</h3>
              <ArrowUpRight className="size-3 text-muted-foreground/0 group-hover:text-muted-foreground transition-colors shrink-0" />
            </div>
            <div className="flex items-center gap-2 mt-0.5">
              <Badge variant="outline" className={cn("text-[9px] px-1.5 py-0 h-4 font-medium", catCfg.color)}>{catCfg.label}</Badge>
              <span className="text-[10px] text-muted-foreground">v{system.version}</span>
            </div>
          </div>
        </div>
        <p className="text-[11px] text-muted-foreground leading-relaxed mt-2.5 line-clamp-2">{system.description}</p>
      </div>

      {/* Metrics */}
      <div className="grid grid-cols-2 gap-px bg-border/50 border-t">
        {system.metrics.slice(0, 4).map((m, i) => (
          <div key={m.label} className={cn("px-3 py-2 bg-card", i < 2 ? "" : "")}>
            <div className="text-xs font-bold">{m.value}</div>
            <div className="text-[9px] text-muted-foreground">{m.label}</div>
          </div>
        ))}
      </div>

      {/* Footer */}
      <div className="flex items-center justify-between px-4 py-2.5 border-t">
        <div className="flex items-center gap-1">
          {system.agentIds.slice(0, 3).map((aid) => {
            const persona = getPersona(aid);
            if (!persona) return null;
            const cfg = genConfig(persona.avatar);
            return <NiceAvatar key={aid} className="size-4 -ml-0.5 first:ml-0 ring-1 ring-card" {...cfg} />;
          })}
          {system.agentIds.length > 3 && (
            <span className="text-[9px] text-muted-foreground ml-1">+{system.agentIds.length - 3}</span>
          )}
        </div>
        <div className={cn("flex items-center gap-1.5 text-[10px] font-medium", statusCfg.color)}>
          <span className={cn("size-1.5 rounded-full", statusCfg.dot)} />
          {statusCfg.label}
        </div>
      </div>
    </div>
  );
}

// =============================================================================
// System detail page
// =============================================================================

function SystemDetail({ system, onBack }: { system: EnterpriseSystem; onBack: () => void }) {
  const Icon = system.icon;
  const statusCfg = STATUS_CONFIG[system.status];
  const catCfg = CATEGORY_CONFIG[system.category];

  return (
    <div className="flex flex-col h-full overflow-hidden">
      {/* Header bar */}
      <div className="flex items-center gap-3 px-6 py-4 border-b shrink-0">
        <button
          type="button"
          className="flex items-center justify-center size-8 rounded-lg border hover:bg-foreground/[0.04] transition-colors"
          onClick={onBack}
          aria-label="Back"
        >
          <ArrowLeft className="size-4" />
        </button>
        <div className={cn("flex items-center justify-center size-9 rounded-xl shrink-0", system.iconBg)}>
          <Icon className={cn("size-5", system.iconColor)} />
        </div>
        <div className="flex-1 min-w-0">
          <div className="flex items-center gap-2">
            <h1 className="text-base font-bold">{system.name}</h1>
            <Badge variant="outline" className={cn("text-[9px] px-1.5 py-0 h-4 font-medium", catCfg.color)}>{catCfg.label}</Badge>
            <div className={cn("flex items-center gap-1.5 text-[10px] font-medium", statusCfg.color)}>
              <span className={cn("size-1.5 rounded-full", statusCfg.dot)} />
              {statusCfg.label}
            </div>
          </div>
          <p className="text-xs text-muted-foreground mt-0.5">{system.description}</p>
        </div>
        <button
          type="button"
          className="flex items-center gap-1.5 rounded-lg bg-primary px-4 py-2 text-xs font-medium text-primary-foreground hover:bg-primary/90 transition-colors shrink-0"
        >
          <Globe className="size-3.5" />
          进入系统
        </button>
      </div>

      <ScrollArea className="flex-1">
        <div className="px-6 py-5 space-y-6">
          {/* Overview metrics */}
          <div>
            <h2 className="text-sm font-bold mb-3 flex items-center gap-2">
              <BarChart3 className="size-4 text-primary" />核心指标
            </h2>
            <div className="grid grid-cols-4 gap-3">
              {system.metrics.map((m) => (
                <div key={m.label} className="rounded-lg border bg-card px-4 py-3 text-center">
                  <div className="text-lg font-bold">{m.value}</div>
                  <div className="text-[10px] text-muted-foreground mt-0.5">{m.label}</div>
                </div>
              ))}
            </div>
          </div>

          {/* System info */}
          <div className="grid grid-cols-2 gap-4">
            <div className="rounded-xl border bg-card p-4">
              <h3 className="text-xs font-bold mb-3 flex items-center gap-2">
                <Server className="size-3.5 text-primary" />系统信息
              </h3>
              <div className="space-y-2.5">
                {[
                  { label: "版本", value: `v${system.version}` },
                  { label: "上次部署", value: system.lastDeploy },
                  { label: "可用性", value: system.uptime },
                  { label: "部署模式", value: "TEE 安全容器 + 多可用区" },
                ].map((row) => (
                  <div key={row.label} className="flex items-center justify-between">
                    <span className="text-[11px] text-muted-foreground">{row.label}</span>
                    <span className="text-[11px] font-medium">{row.value}</span>
                  </div>
                ))}
              </div>
              <div className="flex flex-wrap gap-1.5 mt-3 pt-3 border-t">
                {system.stack.map((t) => (
                  <span key={t} className="rounded-md bg-muted px-2 py-0.5 text-[10px] font-medium text-muted-foreground">{t}</span>
                ))}
              </div>
            </div>

            <div className="rounded-xl border bg-card p-4">
              <h3 className="text-xs font-bold mb-3 flex items-center gap-2">
                <Users className="size-3.5 text-primary" />负责智能体
              </h3>
              <div className="space-y-2">
                {system.agentIds.map((aid) => {
                  const persona = getPersona(aid);
                  if (!persona) return null;
                  const cfg = genConfig(persona.avatar);
                  return (
                    <div key={aid} className="flex items-center gap-2.5 rounded-lg border px-3 py-2">
                      <NiceAvatar className="size-6 shrink-0" {...cfg} />
                      <div className="flex-1 min-w-0">
                        <div className="text-[11px] font-medium">{persona.name}</div>
                        <div className="text-[10px] text-muted-foreground truncate">{persona.description}</div>
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          </div>

          {/* Modules */}
          <div>
            <h2 className="text-sm font-bold mb-3 flex items-center gap-2">
              <Layers className="size-4 text-primary" />功能模块
            </h2>
            <div className="grid grid-cols-2 gap-3">
              {system.modules.map((mod) => (
                <div
                  key={mod.name}
                  className={cn(
                    "rounded-lg border bg-card p-3.5 transition-all",
                    mod.status === "active" ? "hover:shadow-sm hover:border-primary/30 cursor-pointer" : "",
                    mod.status === "planned" && "opacity-60",
                  )}
                >
                  <div className="flex items-center justify-between mb-1">
                    <span className="text-xs font-medium">{mod.name}</span>
                    {mod.status === "active" && (
                      <span className="inline-flex items-center gap-0.5 text-[9px] text-primary font-medium">
                        <CheckCircle2 className="size-2.5" />已上线
                      </span>
                    )}
                    {mod.status === "beta" && (
                      <Badge className="text-[9px] px-1.5 py-0 h-4 bg-blue-500/10 text-blue-600 dark:text-blue-400 border-blue-500/20" variant="outline">Beta</Badge>
                    )}
                    {mod.status === "planned" && (
                      <span className="text-[9px] text-muted-foreground">规划中</span>
                    )}
                  </div>
                  <p className="text-[10px] text-muted-foreground leading-relaxed">{mod.description}</p>
                </div>
              ))}
            </div>
          </div>
        </div>
      </ScrollArea>
    </div>
  );
}

// =============================================================================
// Main page
// =============================================================================

export default function SystemsPage() {
  const [search, setSearch] = useState("");
  const [categoryFilter, setCategoryFilter] = useState<SystemCategory | "all">("all");
  const [selectedSystem, setSelectedSystem] = useState<EnterpriseSystem | null>(null);

  const filtered = useMemo(() => {
    return SYSTEMS.filter((s) => {
      if (categoryFilter !== "all" && s.category !== categoryFilter) return false;
      if (search) {
        const q = search.toLowerCase();
        return s.name.toLowerCase().includes(q) || s.description.toLowerCase().includes(q);
      }
      return true;
    });
  }, [search, categoryFilter]);

  const categories: { key: SystemCategory | "all"; label: string; count: number }[] = useMemo(() => {
    const counts: Record<string, number> = { all: SYSTEMS.length };
    for (const s of SYSTEMS) {
      counts[s.category] = (counts[s.category] || 0) + 1;
    }
    return [
      { key: "all", label: "全部", count: counts.all },
      ...Object.entries(CATEGORY_CONFIG).map(([key, cfg]) => ({
        key: key as SystemCategory,
        label: cfg.label,
        count: counts[key] || 0,
      })),
    ];
  }, []);

  if (selectedSystem) {
    return <SystemDetail system={selectedSystem} onBack={() => setSelectedSystem(null)} />;
  }

  return (
    <div className="flex flex-col h-full overflow-hidden">
      {/* Header */}
      <div className="flex items-center gap-3 px-6 py-4 border-b shrink-0">
        <div className="flex items-center justify-center size-8 rounded-lg bg-primary/10">
          <Building2 className="size-4 text-primary" />
        </div>
        <div className="flex-1">
          <h1 className="text-base font-bold">企业系统</h1>
          <p className="text-xs text-muted-foreground">智能体开发与维护的企业级业务系统</p>
        </div>
        <div className="relative">
          <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 size-3.5 text-muted-foreground" />
          <input
            type="text"
            placeholder="搜索系统..."
            className="h-8 rounded-lg border bg-background pl-8 pr-3 text-xs focus:outline-none focus:ring-1 focus:ring-primary w-[200px]"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
          />
        </div>
      </div>

      {/* Category tabs */}
      <div className="flex items-center gap-1.5 px-6 py-2.5 border-b shrink-0 overflow-x-auto">
        {categories.map((cat) => (
          <button
            key={cat.key}
            type="button"
            className={cn(
              "flex items-center gap-1.5 rounded-full px-3 py-1 text-[11px] font-medium transition-colors whitespace-nowrap",
              categoryFilter === cat.key
                ? "bg-primary text-primary-foreground"
                : "bg-muted text-muted-foreground hover:bg-foreground/[0.06]",
            )}
            onClick={() => setCategoryFilter(cat.key)}
          >
            {cat.label}
            <span className={cn(
              "text-[9px]",
              categoryFilter === cat.key ? "text-primary-foreground/70" : "text-muted-foreground/60",
            )}>
              {cat.count}
            </span>
          </button>
        ))}
      </div>

      {/* System grid */}
      <ScrollArea className="flex-1">
        <div className="px-6 py-5">
          <div className="grid grid-cols-2 xl:grid-cols-3 gap-4">
            {filtered.map((sys) => (
              <SystemCard key={sys.id} system={sys} onClick={() => setSelectedSystem(sys)} />
            ))}
          </div>
          {filtered.length === 0 && (
            <div className="flex flex-col items-center justify-center py-20 text-muted-foreground">
              <Search className="size-10 mb-3 opacity-30" />
              <p className="text-sm">没有找到匹配的系统</p>
            </div>
          )}
        </div>
      </ScrollArea>
    </div>
  );
}
