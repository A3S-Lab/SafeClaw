import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";
import { BUILTIN_PERSONAS } from "@/lib/builtin-personas";
import {
  fetchTrendingEvents,
  searchEvents,
  formatVolume,
  formatProbability,
  type ParsedEvent,
  type ParsedMarket,
} from "@/lib/polymarket";
import NiceAvatar, { genConfig } from "react-nice-avatar";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  ArrowDown,
  ArrowUp,
  Bell,
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  ExternalLink,
  FileText,
  Filter,
  Globe,
  Loader2,
  RefreshCw,
  Search,
  ShieldCheck,
  Terminal,
  TrendingUp,
  Zap,
} from "lucide-react";

// =============================================================================
// Types
// =============================================================================

/** Top-level view tab: Polymarket vs local events */
type ViewTab = "local" | "polymarket";

type LocalCategory = "all" | "market" | "news" | "social" | "task" | "system" | "compliance";

interface EventItem {
  id: string;
  category: Exclude<LocalCategory, "all">;
  topic: string;
  summary: string;
  detail?: string;
  timestamp: number;
  source: string;
  subscribers: string[];
  reacted?: boolean;
  reactedAgent?: string;
}

// =============================================================================
// Category config (local events only)
// =============================================================================

const LOCAL_CATEGORIES: { key: LocalCategory; label: string; icon: typeof Bell }[] = [
  { key: "all", label: "全部", icon: Bell },
  { key: "market", label: "市场行情", icon: TrendingUp },
  { key: "news", label: "新闻资讯", icon: FileText },
  { key: "social", label: "社交媒体", icon: Bell },
  { key: "task", label: "任务事件", icon: CheckCircle2 },
  { key: "system", label: "系统事件", icon: Terminal },
  { key: "compliance", label: "合规监控", icon: ShieldCheck },
];

const categoryIconMap: Record<string, typeof Bell> = {
  market: TrendingUp,
  news: FileText,
  social: Bell,
  task: CheckCircle2,
  system: Terminal,
  compliance: ShieldCheck,
};

const categoryLabelMap: Record<string, string> = {
  market: "市场行情",
  news: "新闻资讯",
  social: "社交媒体",
  task: "任务事件",
  system: "系统事件",
  compliance: "合规监控",
};

// =============================================================================
// Local mock events
// =============================================================================

const now = Date.now();

const LOCAL_EVENTS: EventItem[] = [
  {
    id: "local-1",
    category: "market",
    topic: "forex.usd_cny",
    summary: "USD/CNY 突破 7.35 关口（7.3521），创近 3 个月新高",
    detail: "Rate: 7.3521 (+0.42%)\n24h range: 7.3180 - 7.3558\nTrigger: US non-farm payroll beat",
    timestamp: now - 10 * 60_000,
    source: "Reuters Forex",
    subscribers: ["financial-analyst"],
    reacted: true,
    reactedAgent: "财务分析师",
  },
  {
    id: "local-2",
    category: "news",
    topic: "regulation.pboc.update",
    summary: "央行发布《个人信息保护与信用评估管理办法（征求意见稿）》",
    detail: "Document: 银发〔2025〕18号\nEffective: 2025-07-01\nKey: 信用评分需可解释、禁止社交数据、变更需报备",
    timestamp: now - 12 * 60_000,
    source: "中国人民银行",
    subscribers: ["risk-analyst", "product-manager"],
    reacted: true,
    reactedAgent: "风控分析师",
  },
  {
    id: "local-3",
    category: "news",
    topic: "industry.fintech_funding",
    summary: "蚂蚁集团数字科技板块完成 50 亿元 B 轮融资，估值 200 亿",
    detail: "Company: Ant Group Digital Tech\nRound: Series B, ¥5B\nValuation: ¥20B\nLead: CIC, Temasek",
    timestamp: now - 8 * 3600_000,
    source: "36氪",
    subscribers: ["product-manager"],
  },
  {
    id: "local-4",
    category: "social",
    topic: "twitter.competitor.alert",
    summary: "Airwallex 宣布推出「AI 智能对账」功能，准确率 99.2%",
    detail: "Source: @Airwallex (Twitter/X)\nEngagement: 2.3K likes, 891 retweets\nSentiment: 82% positive",
    timestamp: now - 2 * 3600_000,
    source: "Twitter/X",
    subscribers: ["product-manager"],
    reacted: true,
    reactedAgent: "产品经理",
  },
  {
    id: "local-5",
    category: "social",
    topic: "weibo.fintech_trend",
    summary: "#数字人民币跨境支付# 话题阅读量突破 2 亿，热度上升 340%",
    detail: "Platform: 微博\nHashtag: #数字人民币跨境支付#\nViews: 2.1亿\nTrend: +340% in 24h",
    timestamp: now - 5 * 3600_000,
    source: "微博",
    subscribers: ["product-manager", "financial-analyst"],
  },
  {
    id: "local-6",
    category: "task",
    topic: "skill.factor_analysis.registered",
    summary: "新技能「因子检验」已注册成功，可供所有量化研究员使用",
    timestamp: now - 5 * 60_000,
    source: "技能系统",
    subscribers: ["quant-researcher", "data-scientist"],
  },
  {
    id: "local-7",
    category: "task",
    topic: "skill.model_monitor.completed",
    summary: "模型监控执行完成：信用评分模型 PSI=0.18（超预警线），AUC 降至 0.74",
    detail: "Model: credit_score_v3\nPSI: 0.18 (warn=0.1)\nAUC: 0.78 → 0.74\nDrift: income_level KS=0.12",
    timestamp: now - 9 * 60_000,
    source: "技能调度",
    subscribers: ["risk-analyst", "financial-analyst"],
    reacted: true,
    reactedAgent: "财务分析师",
  },
  {
    id: "local-8",
    category: "task",
    topic: "agent.fullstack.skill_created",
    summary: "全栈工程师完成「数据管道质量监控」技能开发，已注册到技能系统",
    timestamp: now - 15 * 60_000,
    source: "智能体协作",
    subscribers: ["data-engineer"],
  },
  {
    id: "local-9",
    category: "system",
    topic: "pipeline.trades.latency_spike",
    summary: "实时交易管道延迟飙升至 1200ms（阈值 500ms），可能影响数据时效性",
    detail: "Pipeline: trades-realtime\nLatency: 1200ms (threshold: 500ms)\nCause: Kafka consumer group rebalance",
    timestamp: now - 22 * 60_000,
    source: "管道监控",
    subscribers: ["data-engineer", "devops-engineer"],
  },
  {
    id: "local-10",
    category: "system",
    topic: "k8s.node.disk_pressure",
    summary: "K8s node-2 磁盘使用率达 87%，触发 DiskPressure 告警",
    detail: "Node: node-2\nDisk: 87% used (threshold: 85%)\nAction needed: Clean up or expand volume",
    timestamp: now - 45 * 60_000,
    source: "Kubernetes",
    subscribers: ["devops-engineer"],
  },
  {
    id: "local-11",
    category: "compliance",
    topic: "audit.data_access.anomaly",
    summary: "检测到异常数据访问模式：非工作时间批量查询客户信用记录（32 次/分钟）",
    detail: "Time: 02:34 AM\nAPI: /api/v1/credit-records\nRate: 32 req/min (normal: 2-5)\nRisk: Potential data exfiltration",
    timestamp: now - 3 * 3600_000,
    source: "安全审计",
    subscribers: ["risk-analyst", "devops-engineer"],
  },
  {
    id: "local-12",
    category: "compliance",
    topic: "regulation.fatf.travel_rule",
    summary: "FATF 更新旅行规则指引：跨境支付 $1000 以上需完整收付款人信息",
    detail: "Organization: FATF\nThreshold: $1000 USD equivalent\nDeadline: 2025-06-30",
    timestamp: now - 12 * 3600_000,
    source: "FATF",
    subscribers: ["product-manager", "financial-analyst", "risk-analyst"],
  },
];

// =============================================================================
// Shared: Subscriber badges
// =============================================================================

function SubscriberBadges({ personaIds }: { personaIds: string[] }) {
  return (
    <div className="flex items-center gap-1">
      {personaIds.slice(0, 4).map((pid) => {
        const persona = BUILTIN_PERSONAS.find((p) => p.id === pid);
        if (!persona) return null;
        const cfg = genConfig(persona.avatar);
        return (
          <div key={pid} className="relative group/avatar">
            <NiceAvatar className="size-5 ring-2 ring-background" {...cfg} />
            <div className="absolute bottom-full left-1/2 -translate-x-1/2 mb-1 hidden group-hover/avatar:block z-10">
              <div className="bg-popover text-popover-foreground text-[10px] px-2 py-1 rounded shadow-lg border whitespace-nowrap">
                {persona.name}
              </div>
            </div>
          </div>
        );
      })}
      {personaIds.length > 4 && (
        <span className="text-[10px] text-muted-foreground">+{personaIds.length - 4}</span>
      )}
    </div>
  );
}

// =============================================================================
// Polymarket card
// =============================================================================

function PolymarketCard({ event, markets }: { event: ParsedEvent; markets: ParsedMarket[] }) {
  const [expanded, setExpanded] = useState(false);
  const topMarket = markets[0];

  return (
    <div className="rounded-lg border bg-card p-4 hover:shadow-sm transition-shadow">
      <div className="flex items-start gap-3 mb-3">
        {event.image && (
          <img
            src={event.image}
            alt=""
            className="size-10 rounded object-cover shrink-0 bg-muted"
            crossOrigin="anonymous"
            onError={(e) => { (e.target as HTMLImageElement).style.display = "none"; }}
          />
        )}
        <div className="flex-1 min-w-0">
          <h3 className="text-sm font-medium leading-snug mb-1">{event.title}</h3>
          {event.description && (
            <p className="text-[11px] text-muted-foreground line-clamp-2">{event.description}</p>
          )}
        </div>
        <a
          href={`https://polymarket.com/event/${event.slug}`}
          target="_blank"
          rel="noopener noreferrer"
          className="text-muted-foreground hover:text-primary transition-colors shrink-0"
        >
          <ExternalLink className="size-3.5" />
        </a>
      </div>

      {topMarket && (
        <div className="mb-3">
          <div className="flex items-center justify-between text-xs mb-1.5">
            <span className="text-muted-foreground truncate mr-2">{topMarket.question || event.title}</span>
            {topMarket.oneDayPriceChange !== 0 && (
              <span
                className={cn(
                  "flex items-center gap-0.5 text-[11px] font-medium shrink-0",
                  topMarket.oneDayPriceChange > 0 ? "text-green-600 dark:text-green-400" : "text-red-600 dark:text-red-400",
                )}
              >
                {topMarket.oneDayPriceChange > 0 ? <ArrowUp className="size-3" /> : <ArrowDown className="size-3" />}
                {Math.abs(topMarket.oneDayPriceChange * 100).toFixed(1)}%
              </span>
            )}
          </div>
          <div className="flex gap-1.5 h-7">
            <div
              className="bg-primary/80 rounded-l flex items-center justify-center text-[11px] font-semibold text-primary-foreground transition-all"
              style={{ width: `${Math.max(topMarket.yesPrice * 100, 8)}%` }}
            >
              Yes {formatProbability(topMarket.yesPrice)}
            </div>
            <div
              className="bg-muted rounded-r flex items-center justify-center text-[11px] font-semibold text-muted-foreground transition-all"
              style={{ width: `${Math.max(topMarket.noPrice * 100, 8)}%` }}
            >
              No {formatProbability(topMarket.noPrice)}
            </div>
          </div>
        </div>
      )}

      <div className="flex items-center gap-4 text-[11px] text-muted-foreground mb-2">
        <span>总成交量 {formatVolume(event.volume)}</span>
        <span>24h {formatVolume(event.volume24hr)}</span>
        {event.liquidity > 0 && <span>流动性 {formatVolume(event.liquidity)}</span>}
      </div>

      {markets.length > 1 && (
        <>
          <button
            type="button"
            className="flex items-center gap-1 text-[11px] text-muted-foreground hover:text-foreground transition-colors mb-2"
            onClick={() => setExpanded(!expanded)}
          >
            {expanded ? <ChevronDown className="size-3" /> : <ChevronRight className="size-3" />}
            <span>{markets.length} 个子市场</span>
          </button>
          {expanded && (
            <div className="space-y-1.5 mb-2">
              {markets.slice(1, 10).map((m) => (
                <div key={m.id} className="flex items-center gap-2 text-[11px] rounded bg-muted/40 px-2.5 py-1.5">
                  <span className="flex-1 truncate">{m.question}</span>
                  <span className="font-medium text-primary">{formatProbability(m.yesPrice)}</span>
                  {m.oneDayPriceChange !== 0 && (
                    <span
                      className={cn(
                        "flex items-center gap-0.5 text-[10px]",
                        m.oneDayPriceChange > 0 ? "text-green-600 dark:text-green-400" : "text-red-600 dark:text-red-400",
                      )}
                    >
                      {m.oneDayPriceChange > 0 ? "+" : ""}
                      {(m.oneDayPriceChange * 100).toFixed(1)}%
                    </span>
                  )}
                  <span className="text-muted-foreground">{formatVolume(m.volume24hr)}</span>
                </div>
              ))}
            </div>
          )}
        </>
      )}
    </div>
  );
}

// =============================================================================
// Local event card
// =============================================================================

function LocalEventCard({ event }: { event: EventItem }) {
  const [expanded, setExpanded] = useState(false);
  const Icon = categoryIconMap[event.category] || Bell;
  const categoryLabel = categoryLabelMap[event.category] || event.category;

  const timeStr = (() => {
    const diff = Date.now() - event.timestamp;
    const mins = Math.floor(diff / 60_000);
    if (mins < 1) return "刚刚";
    if (mins < 60) return `${mins} 分钟前`;
    const hours = Math.floor(mins / 60);
    if (hours < 24) return `${hours} 小时前`;
    return `${Math.floor(hours / 24)} 天前`;
  })();

  return (
    <div className="rounded-lg border bg-card p-4 hover:shadow-sm transition-shadow">
      <div className="flex items-center gap-2 mb-2">
        <div className="flex items-center justify-center size-6 rounded bg-primary/10">
          <Icon className="size-3.5 text-primary" />
        </div>
        <span className="text-[10px] font-medium text-primary uppercase tracking-wide">{categoryLabel}</span>
        <span className="text-[10px] text-muted-foreground">·</span>
        <span className="text-[10px] text-muted-foreground font-mono">{event.topic}</span>
        <time className="text-[10px] text-muted-foreground ml-auto">{timeStr}</time>
      </div>

      <p className="text-sm leading-relaxed mb-2">{event.summary}</p>

      {event.detail && (
        <>
          <button
            type="button"
            className="flex items-center gap-1 text-[11px] text-muted-foreground hover:text-foreground transition-colors mb-2"
            onClick={() => setExpanded(!expanded)}
          >
            {expanded ? <ChevronDown className="size-3" /> : <ChevronRight className="size-3" />}
            <span>原始数据</span>
          </button>
          {expanded && (
            <pre className="rounded bg-muted/50 p-2 text-[11px] font-mono overflow-x-auto max-h-32 whitespace-pre-wrap text-muted-foreground mb-2">
              {event.detail}
            </pre>
          )}
        </>
      )}

      <div className="flex items-center justify-between pt-2 border-t">
        <span className="text-[10px] text-muted-foreground">来源: {event.source}</span>
        <div className="flex items-center gap-3">
          {event.reacted && event.reactedAgent && (
            <span className="text-[10px] text-primary font-medium flex items-center gap-1">
              <Zap className="size-3" />
              {event.reactedAgent} 已响应
            </span>
          )}
          <div className="flex items-center gap-1.5">
            <span className="text-[10px] text-muted-foreground">订阅:</span>
            <SubscriberBadges personaIds={event.subscribers} />
          </div>
        </div>
      </div>
    </div>
  );
}

// =============================================================================
// Main Events Page
// =============================================================================

export default function EventsPage() {
  const [viewTab, setViewTab] = useState<ViewTab>("local");
  const [localCategory, setLocalCategory] = useState<LocalCategory>("all");
  const [localSearch, setLocalSearch] = useState("");
  const [pmSearch, setPmSearch] = useState("");
  const [pmEvents, setPmEvents] = useState<ParsedEvent[]>([]);
  const [pmLoading, setPmLoading] = useState(false);
  const [pmError, setPmError] = useState<string | null>(null);
  const [pmLoaded, setPmLoaded] = useState(false);
  const searchTimer = useRef<ReturnType<typeof setTimeout>>();

  const localQ = localSearch.trim().toLowerCase();
  const pmQ = pmSearch.trim();

  // Fetch Polymarket data
  const loadPolymarket = useCallback(async (query?: string) => {
    setPmLoading(true);
    setPmError(null);
    try {
      const data = query
        ? await searchEvents(query, 16)
        : await fetchTrendingEvents(16);
      setPmEvents(data);
      setPmLoaded(true);
    } catch (err) {
      setPmError(err instanceof Error ? err.message : "Failed to fetch Polymarket data");
    } finally {
      setPmLoading(false);
    }
  }, []);

  // Load Polymarket when switching to that tab for the first time
  useEffect(() => {
    if (viewTab === "polymarket" && !pmLoaded && !pmLoading) {
      loadPolymarket();
    }
  }, [viewTab, pmLoaded, pmLoading, loadPolymarket]);

  // Debounced Polymarket search
  useEffect(() => {
    if (viewTab !== "polymarket") return;
    if (searchTimer.current) clearTimeout(searchTimer.current);
    if (!pmQ) {
      loadPolymarket();
      return;
    }
    searchTimer.current = setTimeout(() => {
      loadPolymarket(pmQ);
    }, 600);
    return () => {
      if (searchTimer.current) clearTimeout(searchTimer.current);
    };
  }, [pmQ, viewTab, loadPolymarket]);

  // Filtered local events
  const filteredLocal = useMemo(() => {
    let events = LOCAL_EVENTS;
    if (localCategory !== "all") {
      events = events.filter((e) => e.category === localCategory);
    }
    if (localQ) {
      events = events.filter(
        (e) =>
          e.summary.toLowerCase().includes(localQ) ||
          e.topic.toLowerCase().includes(localQ) ||
          e.source.toLowerCase().includes(localQ),
      );
    }
    return events.sort((a, b) => b.timestamp - a.timestamp);
  }, [localCategory, localQ]);

  // Local event counts
  const localCounts = useMemo(() => {
    const map: Record<string, number> = { all: LOCAL_EVENTS.length };
    for (const e of LOCAL_EVENTS) {
      map[e.category] = (map[e.category] || 0) + 1;
    }
    return map;
  }, []);

  return (
    <div className="flex h-full w-full">
      {/* Left sidebar */}
      <div className="w-56 border-r flex flex-col">
        <div className="px-4 py-3 border-b">
          <h2 className="text-sm font-semibold">事件中心</h2>
          <p className="text-[11px] text-muted-foreground mt-0.5">智能体订阅的事件与数据</p>
        </div>

        {/* Tab switch */}
        <div className="p-2 border-b">
          <div className="flex rounded-md border bg-muted/30 p-0.5">
            <button
              type="button"
              className={cn(
                "flex-1 flex items-center justify-center gap-1.5 rounded px-2 py-1.5 text-xs transition-colors",
                viewTab === "local"
                  ? "bg-background shadow-sm text-foreground font-medium"
                  : "text-muted-foreground hover:text-foreground",
              )}
              onClick={() => setViewTab("local")}
            >
              <Bell className="size-3.5" />
              本地事件
            </button>
            <button
              type="button"
              className={cn(
                "flex-1 flex items-center justify-center gap-1.5 rounded px-2 py-1.5 text-xs transition-colors",
                viewTab === "polymarket"
                  ? "bg-background shadow-sm text-foreground font-medium"
                  : "text-muted-foreground hover:text-foreground",
              )}
              onClick={() => setViewTab("polymarket")}
            >
              <Globe className="size-3.5" />
              Polymarket
            </button>
          </div>
        </div>

        <ScrollArea className="flex-1">
          {viewTab === "local" ? (
            <>
              {/* Local category filter */}
              <div className="p-2">
                {LOCAL_CATEGORIES.map((cat) => {
                  const Icon = cat.icon;
                  const count = localCounts[cat.key] || 0;
                  return (
                    <button
                      key={cat.key}
                      type="button"
                      className={cn(
                        "flex items-center gap-2.5 w-full rounded-md px-3 py-2 text-xs transition-colors",
                        localCategory === cat.key
                          ? "bg-primary/10 text-primary font-medium"
                          : "text-muted-foreground hover:bg-foreground/[0.04] hover:text-foreground",
                      )}
                      onClick={() => setLocalCategory(cat.key)}
                    >
                      <Icon className="size-4" />
                      <span className="flex-1 text-left">{cat.label}</span>
                      <span className={cn(
                        "text-[10px] rounded-full px-1.5 py-0.5 min-w-[20px] text-center",
                        localCategory === cat.key ? "bg-primary/20 text-primary" : "bg-muted text-muted-foreground",
                      )}>
                        {count}
                      </span>
                    </button>
                  );
                })}
              </div>

              {/* Subscribed agents */}
              <div className="px-4 py-3 border-t">
                <div className="text-[11px] font-medium text-muted-foreground mb-2 flex items-center gap-1.5">
                  <Filter className="size-3" />
                  订阅的智能体
                </div>
                <div className="space-y-1.5">
                  {BUILTIN_PERSONAS.filter((p) =>
                    LOCAL_EVENTS.some((e) => e.subscribers.includes(p.id)),
                  ).map((persona) => {
                    const cfg = genConfig(persona.avatar);
                    const subCount = LOCAL_EVENTS.filter((e) => e.subscribers.includes(persona.id)).length;
                    return (
                      <div key={persona.id} className="flex items-center gap-2 text-xs text-muted-foreground">
                        <NiceAvatar className="size-5" {...cfg} />
                        <span className="flex-1 truncate">{persona.name}</span>
                        <span className="text-[10px]">{subCount} 事件</span>
                      </div>
                    );
                  })}
                </div>
              </div>
            </>
          ) : (
            <>
              {/* Polymarket info */}
              <div className="p-4 space-y-3">
                <div className="text-[11px] text-muted-foreground leading-relaxed">
                  <p className="font-medium text-foreground mb-1">Polymarket 预测市场</p>
                  <p>来自全球最大去中心化预测市场的实时事件合约数据，按 24h 成交量排序。</p>
                </div>
                <div className="rounded bg-muted/40 px-3 py-2 space-y-1 text-[11px] text-muted-foreground">
                  <div className="flex justify-between">
                    <span>已加载事件</span>
                    <span className="font-medium text-foreground">{pmEvents.length}</span>
                  </div>
                  <div className="flex justify-between">
                    <span>数据语言</span>
                    <span className="font-medium text-foreground">English</span>
                  </div>
                  <div className="flex justify-between">
                    <span>数据状态</span>
                    <span className={cn("font-medium", pmError ? "text-red-500" : "text-green-600 dark:text-green-400")}>
                      {pmLoading ? "加载中..." : pmError ? "错误" : pmLoaded ? "已连接" : "未加载"}
                    </span>
                  </div>
                </div>

                {/* Polymarket subscribed agents */}
                <div className="text-[11px] font-medium text-muted-foreground flex items-center gap-1.5">
                  <Filter className="size-3" />
                  订阅 Polymarket 的智能体
                </div>
                <div className="space-y-1.5">
                  {["quant-researcher", "risk-analyst", "financial-analyst"].map((pid) => {
                    const persona = BUILTIN_PERSONAS.find((p) => p.id === pid);
                    if (!persona) return null;
                    const cfg = genConfig(persona.avatar);
                    return (
                      <div key={pid} className="flex items-center gap-2 text-xs text-muted-foreground">
                        <NiceAvatar className="size-5" {...cfg} />
                        <span className="flex-1 truncate">{persona.name}</span>
                      </div>
                    );
                  })}
                </div>
              </div>
            </>
          )}
        </ScrollArea>
      </div>

      {/* Right: event feed */}
      <div className="flex-1 flex flex-col">
        {/* Toolbar */}
        <div className="px-4 py-3 border-b flex items-center gap-3">
          {viewTab === "local" ? (
            <>
              <div className="relative flex-1 max-w-md">
                <Search className="absolute left-2.5 top-2.5 size-4 text-muted-foreground" />
                <Input
                  placeholder="搜索本地事件..."
                  value={localSearch}
                  onChange={(e) => setLocalSearch(e.target.value)}
                  className="pl-8 h-9"
                />
              </div>
              <div className="flex items-center gap-2 text-xs text-muted-foreground">
                <span>{filteredLocal.length} 个事件</span>
                {localCategory !== "all" && (
                  <Button variant="ghost" size="sm" className="h-7 text-xs" onClick={() => setLocalCategory("all")}>
                    清除筛选
                  </Button>
                )}
              </div>
            </>
          ) : (
            <>
              <div className="relative flex-1 max-w-md">
                <Search className="absolute left-2.5 top-2.5 size-4 text-muted-foreground" />
                <Input
                  placeholder="Search Polymarket events (e.g. Bitcoin, Trump, Fed)..."
                  value={pmSearch}
                  onChange={(e) => setPmSearch(e.target.value)}
                  className="pl-8 h-9"
                />
              </div>
              <div className="flex items-center gap-2 text-xs text-muted-foreground">
                <span>{pmEvents.length} events</span>
                <Button
                  variant="ghost"
                  size="sm"
                  className="h-7 text-xs gap-1"
                  onClick={() => loadPolymarket(pmQ || undefined)}
                  disabled={pmLoading}
                >
                  <RefreshCw className={cn("size-3", pmLoading && "animate-spin")} />
                  刷新
                </Button>
              </div>
            </>
          )}
        </div>

        {/* Content area */}
        <ScrollArea className="flex-1">
          <div className="p-4 space-y-3">
            {viewTab === "local" ? (
              <>
                {filteredLocal.map((event) => (
                  <LocalEventCard key={event.id} event={event} />
                ))}
                {filteredLocal.length === 0 && (
                  <div className="flex flex-col items-center justify-center py-16 text-muted-foreground">
                    <Bell className="size-10 mb-3 opacity-30" />
                    <p className="text-sm">{localQ ? "未找到匹配的事件" : "暂无事件"}</p>
                  </div>
                )}
              </>
            ) : (
              <>
                {pmLoading && pmEvents.length === 0 && (
                  <div className="flex items-center justify-center py-12 text-muted-foreground gap-2">
                    <Loader2 className="size-4 animate-spin" />
                    <span className="text-sm">正在加载 Polymarket 实时数据...</span>
                  </div>
                )}
                {pmError && (
                  <div className="rounded-lg border border-red-200 dark:border-red-800 bg-red-50 dark:bg-red-950/30 p-4">
                    <p className="text-sm text-red-600 dark:text-red-400">Polymarket 数据加载失败: {pmError}</p>
                    <Button
                      variant="outline"
                      size="sm"
                      className="mt-2 h-7 text-xs"
                      onClick={() => loadPolymarket(pmQ || undefined)}
                    >
                      重试
                    </Button>
                  </div>
                )}
                {pmEvents.length > 0 && (
                  <>
                    <div className="flex items-center gap-2 text-xs text-muted-foreground mb-1">
                      <TrendingUp className="size-3.5 text-primary" />
                      <span className="font-medium text-primary">
                        {pmQ ? `"${pmQ}" 搜索结果` : "热门预测市场（按 24h 成交量排序）"}
                      </span>
                      {pmLoading && <Loader2 className="size-3 animate-spin" />}
                    </div>
                    {pmEvents.map((pe) => (
                      <PolymarketCard key={pe.id} event={pe} markets={pe.markets} />
                    ))}
                  </>
                )}
                {!pmLoading && !pmError && pmEvents.length === 0 && pmLoaded && (
                  <div className="flex flex-col items-center justify-center py-16 text-muted-foreground">
                    <Globe className="size-10 mb-3 opacity-30" />
                    <p className="text-sm">{pmQ ? `未找到 "${pmQ}" 相关的预测市场` : "暂无 Polymarket 数据"}</p>
                  </div>
                )}
              </>
            )}
          </div>
        </ScrollArea>
      </div>
    </div>
  );
}
