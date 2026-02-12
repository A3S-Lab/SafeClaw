/**
 * Floating event notifications in the bottom-left corner.
 * Periodically triggers mock events (system, news, policy, social, market, compliance).
 * User can dismiss or respond by assigning a task to selected agents.
 */
import { cn } from "@/lib/utils";
import { BUILTIN_PERSONAS } from "@/lib/builtin-personas";
import NiceAvatar, { genConfig } from "react-nice-avatar";
import {
  Bell,
  CheckCircle2,
  ChevronDown,
  ChevronUp,
  FileText,
  Gavel,
  Globe2,
  Megaphone,
  Send,
  ShieldCheck,
  Terminal,
  X,
  Zap,
} from "lucide-react";
import { useCallback, useEffect, useRef, useState } from "react";

// =============================================================================
// Mock event pool
// =============================================================================

interface MockEvent {
  id: string;
  source: "system" | "news" | "policy" | "social" | "market" | "compliance";
  title: string;
  summary: string;
  detail?: string;
  /** Suggested agents for handling */
  suggestedAgents?: string[];
}

const EVENT_POOL: Omit<MockEvent, "id">[] = [
  {
    source: "system",
    title: "风控引擎延迟飙升",
    summary: "risk-engine P99 延迟从 12ms 升至 340ms，触发告警阈值",
    detail: "Prometheus alert: risk_engine_p99_latency_ms > 200 持续 5 分钟。可能原因：特征服务 Redis 连接池耗尽。",
    suggestedAgents: ["devops-engineer", "risk-analyst"],
  },
  {
    source: "news",
    title: "Stripe 宣布进入东南亚市场",
    summary: "Stripe 获得新加坡、印尼支付牌照，将直接与本地支付网关竞争",
    detail: "Reuters: Stripe obtained MAS and OJK licenses. Plans to launch in Q3 2025 with local acquiring capabilities.",
    suggestedAgents: ["product-manager", "financial-analyst"],
  },
  {
    source: "policy",
    title: "央行发布《个人金融信息保护》征求意见稿",
    summary: "新规要求金融机构对 C3 级数据实施 TEE 或同态加密处理，过渡期 6 个月",
    detail: "PBOC Draft: Personal Financial Information Protection Regulation. C3 data (account numbers, transaction records) must be processed in TEE/HE. Deadline: 6 months from publication.",
    suggestedAgents: ["compliance-officer", "security-engineer"],
  },
  {
    source: "social",
    title: "知乎热议：SafeClaw 跨境支付体验",
    summary: "帖子热度 2.4K，用户反馈到账速度快但手续费偏高",
    detail: "知乎话题「2025年跨境支付哪家强」中 SafeClaw 被多次提及。正面：到账 T+0、API 文档清晰。负面：中小商户费率缺乏竞争力。",
    suggestedAgents: ["product-manager"],
  },
  {
    source: "market",
    title: "美元兑人民币突破 7.35",
    summary: "离岸人民币汇率跌破关键支撑位，做市商报价波动加大",
    detail: "USD/CNH: 7.3520 (+0.45%). 外汇市场波动率 VIX 升至 14.2。建议检查外汇风险敞口和对冲策略。",
    suggestedAgents: ["financial-analyst", "risk-analyst"],
  },
  {
    source: "compliance",
    title: "反洗钱筛查命中率异常升高",
    summary: "今日 AML 规则命中率从 0.3% 升至 1.2%，需排查是否为规则误报",
    detail: "AML screening hit rate 4x above baseline. Top triggered rules: PEP-003 (politically exposed persons), GEO-012 (high-risk jurisdictions). Manual review queue backlog: 47 cases.",
    suggestedAgents: ["compliance-officer", "risk-analyst"],
  },
  {
    source: "system",
    title: "数据库主从切换完成",
    summary: "PostgreSQL 主节点 failover 自动完成，切换耗时 2.3s，无数据丢失",
    detail: "PG cluster: primary node pg-node-1 unreachable at 14:32. Patroni promoted pg-node-2 to primary. Replication lag at switchover: 0 bytes. All connections re-established.",
    suggestedAgents: ["devops-engineer", "data-engineer"],
  },
  {
    source: "news",
    title: "蚂蚁集团发布企业级 AI Agent 平台",
    summary: "蚂蚁集团推出面向金融机构的 AI Agent 开发平台，支持多模态与合规内置",
    detail: "AntGroup launched \"AntAgent\" platform for financial institutions. Features: multi-modal LLM, built-in compliance rules, TEE support. Target: banks and insurance companies.",
    suggestedAgents: ["product-manager", "fullstack-engineer"],
  },
  {
    source: "policy",
    title: "欧盟 MiCA 法案正式生效",
    summary: "加密资产市场监管框架落地，影响跨境加密支付通道合规要求",
    detail: "EU Markets in Crypto-Assets Regulation (MiCA) effective. Crypto-Asset Service Providers (CASPs) must obtain authorization. Stablecoin issuers face reserve requirements.",
    suggestedAgents: ["compliance-officer", "legal-counsel"],
  },
  {
    source: "market",
    title: "Polymarket: 美联储 6 月降息概率升至 72%",
    summary: "降息预期走强推动风险资产上涨，跨境资金流向或发生变化",
    detail: "Polymarket \"Fed Rate Cut June 2025\" contract: $0.72 (+8%). Bond yields dropping. Implications: USD weakening, cross-border payment volumes may increase.",
    suggestedAgents: ["quant-researcher", "financial-analyst"],
  },
  {
    source: "social",
    title: "推特热搜：#FintechRegulation",
    summary: "多家金融科技公司被监管约谈，行业情绪偏负面",
    detail: "Twitter trending: #FintechRegulation. Multiple fintech companies reportedly called in by regulators for data practice reviews. Sentiment analysis: 65% negative, 20% neutral, 15% positive.",
    suggestedAgents: ["compliance-officer", "product-manager"],
  },
  {
    source: "system",
    title: "Kafka 消费者组 lag 告警",
    summary: "payment-processor 消费者组 lag 突增至 50K 条，可能影响交易处理时效",
    detail: "Kafka consumer group 'payment-processor' lag: 50,247 messages on topic 'transactions.confirmed'. Usual lag: < 100. Possible cause: downstream service bottleneck.",
    suggestedAgents: ["data-engineer", "devops-engineer"],
  },
];

const sourceConfig: Record<MockEvent["source"], { icon: typeof Bell; label: string; color: string }> = {
  system: { icon: Terminal, label: "系统", color: "text-blue-500" },
  news: { icon: FileText, label: "新闻", color: "text-teal-500" },
  policy: { icon: Gavel, label: "政策", color: "text-amber-600 dark:text-amber-400" },
  social: { icon: Megaphone, label: "社交", color: "text-pink-500" },
  market: { icon: Zap, label: "市场", color: "text-purple-500" },
  compliance: { icon: ShieldCheck, label: "合规", color: "text-red-500" },
};

// =============================================================================
// Selectable agents (exclude group chat)
// =============================================================================

const SELECTABLE_AGENTS = BUILTIN_PERSONAS.filter((p) => p.id !== "company-group");

// =============================================================================
// Single event notification card
// =============================================================================

function EventCard({
  event,
  onDismiss,
  onDone,
}: {
  event: MockEvent;
  onDismiss: () => void;
  onDone: () => void;
}) {
  const [expanded, setExpanded] = useState(false);
  const [responding, setResponding] = useState(false);
  const [task, setTask] = useState("");
  const [selectedAgents, setSelectedAgents] = useState<Set<string>>(
    () => new Set(event.suggestedAgents || []),
  );
  const [dispatched, setDispatched] = useState(false);

  const cfg = sourceConfig[event.source];
  const Icon = cfg.icon;

  const toggleAgent = useCallback((id: string) => {
    setSelectedAgents((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  }, []);

  const handleDispatch = useCallback(() => {
    setDispatched(true);
    setTimeout(onDone, 1500);
  }, [onDone]);

  return (
    <div className="rounded-lg border bg-popover text-popover-foreground shadow-lg w-[380px] overflow-hidden animate-in slide-in-from-left-5 fade-in duration-300">
      {/* Header */}
      <div className="flex items-center gap-2 px-3 py-2 border-b bg-muted/30">
        <Icon className={cn("size-3.5", cfg.color)} />
        <span className={cn("text-[10px] font-semibold uppercase tracking-wide", cfg.color)}>{cfg.label}</span>
        <time className="text-[10px] text-muted-foreground ml-auto mr-1">
          {new Date().toLocaleTimeString("zh-CN", { hour: "2-digit", minute: "2-digit" })}
        </time>
        <button
          type="button"
          className="text-muted-foreground hover:text-foreground transition-colors"
          onClick={onDismiss}
          aria-label="Dismiss"
        >
          <X className="size-3.5" />
        </button>
      </div>

      {/* Body */}
      <div className="px-3 py-2.5">
        <div className="text-sm font-medium mb-1">{event.title}</div>
        <p className="text-xs text-muted-foreground leading-relaxed">{event.summary}</p>

        {/* Expandable detail */}
        {event.detail && (
          <button
            type="button"
            className="flex items-center gap-1 text-[10px] text-muted-foreground/70 hover:text-muted-foreground mt-1.5 transition-colors"
            onClick={() => setExpanded(!expanded)}
          >
            {expanded ? <ChevronUp className="size-3" /> : <ChevronDown className="size-3" />}
            {expanded ? "收起详情" : "查看详情"}
          </button>
        )}
        {expanded && event.detail && (
          <div className="mt-1.5 text-[11px] text-muted-foreground bg-muted/40 rounded-md px-2.5 py-2 leading-relaxed whitespace-pre-wrap">
            {event.detail}
          </div>
        )}
      </div>

      {/* Actions / Response area */}
      <div className="px-3 pb-3">
        {!responding && !dispatched && (
          <div className="flex items-center gap-2">
            <button
              type="button"
              className="flex items-center gap-1.5 rounded-md bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground hover:bg-primary/90 transition-colors"
              onClick={() => setResponding(true)}
            >
              <Globe2 className="size-3" />
              响应
            </button>
            <button
              type="button"
              className="flex items-center gap-1.5 rounded-md border px-3 py-1.5 text-xs text-muted-foreground hover:bg-foreground/[0.04] transition-colors"
              onClick={onDismiss}
            >
              忽略
            </button>
          </div>
        )}

        {responding && !dispatched && (
          <div className="space-y-2.5 animate-in fade-in duration-200">
            {/* Task input */}
            <div>
              <label className="text-[10px] font-medium text-muted-foreground mb-1 block">任务描述</label>
              <textarea
                className="w-full rounded-md border px-2.5 py-1.5 text-xs bg-background focus:outline-none focus:ring-1 focus:ring-primary resize-none"
                rows={2}
                placeholder="描述需要处理的任务..."
                value={task}
                onChange={(e) => setTask(e.target.value)}
                autoFocus
              />
            </div>

            {/* Agent selection */}
            <div>
              <label className="text-[10px] font-medium text-muted-foreground mb-1.5 block">指派智能体</label>
              <div className="flex flex-wrap gap-1.5">
                {SELECTABLE_AGENTS.map((agent) => {
                  const isSelected = selectedAgents.has(agent.id);
                  const avatarCfg = genConfig(agent.avatar);
                  return (
                    <button
                      key={agent.id}
                      type="button"
                      className={cn(
                        "flex items-center gap-1.5 rounded-full border px-2 py-1 text-[11px] transition-colors",
                        isSelected
                          ? "border-primary bg-primary/10 text-primary"
                          : "border-border text-muted-foreground hover:bg-foreground/[0.04]",
                      )}
                      onClick={() => toggleAgent(agent.id)}
                    >
                      <NiceAvatar className="size-4" {...avatarCfg} />
                      <span>{agent.name}</span>
                    </button>
                  );
                })}
              </div>
            </div>

            {/* Dispatch / Cancel */}
            <div className="flex items-center gap-2">
              <button
                type="button"
                className={cn(
                  "flex items-center gap-1.5 rounded-md bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground transition-colors",
                  !task.trim() || selectedAgents.size === 0
                    ? "opacity-50 cursor-not-allowed"
                    : "hover:bg-primary/90",
                )}
                disabled={!task.trim() || selectedAgents.size === 0}
                onClick={handleDispatch}
              >
                <Send className="size-3" />
                派发任务
              </button>
              <button
                type="button"
                className="text-xs text-muted-foreground hover:text-foreground transition-colors"
                onClick={() => setResponding(false)}
              >
                取消
              </button>
            </div>
          </div>
        )}

        {dispatched && (
          <div className="flex items-center gap-1.5 text-xs text-primary animate-in fade-in duration-200">
            <CheckCircle2 className="size-3.5" />
            <span>已派发给 {[...selectedAgents].map((id) => SELECTABLE_AGENTS.find((a) => a.id === id)?.name).filter(Boolean).join("、")}</span>
          </div>
        )}
      </div>
    </div>
  );
}

// =============================================================================
// Notification container — manages event queue and timing
// =============================================================================

/** Interval range for new events (ms) */
const MIN_INTERVAL = 15_000;
const MAX_INTERVAL = 35_000;
/** Maximum visible notifications at once */
const MAX_VISIBLE = 3;

export default function EventNotifications() {
  const [events, setEvents] = useState<MockEvent[]>([]);
  const counterRef = useRef(0);
  const poolIndexRef = useRef(0);
  const timerRef = useRef<ReturnType<typeof setTimeout>>();

  const scheduleNext = useCallback(() => {
    const delay = MIN_INTERVAL + Math.random() * (MAX_INTERVAL - MIN_INTERVAL);
    timerRef.current = setTimeout(() => {
      setEvents((prev) => {
        // Pick next event from pool in order, cycling through
        const template = EVENT_POOL[poolIndexRef.current % EVENT_POOL.length];
        poolIndexRef.current++;
        counterRef.current++;
        const newEvent: MockEvent = { ...template, id: `evt-${counterRef.current}` };
        // Keep max visible, drop oldest
        const next = [...prev, newEvent];
        if (next.length > MAX_VISIBLE) return next.slice(next.length - MAX_VISIBLE);
        return next;
      });
      scheduleNext();
    }, delay);
  }, []);

  useEffect(() => {
    // Fire first event faster
    const firstDelay = 5_000 + Math.random() * 8_000;
    timerRef.current = setTimeout(() => {
      const template = EVENT_POOL[poolIndexRef.current % EVENT_POOL.length];
      poolIndexRef.current++;
      counterRef.current++;
      setEvents([{ ...template, id: `evt-${counterRef.current}` }]);
      scheduleNext();
    }, firstDelay);

    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, [scheduleNext]);

  const dismiss = useCallback((id: string) => {
    setEvents((prev) => prev.filter((e) => e.id !== id));
  }, []);

  if (events.length === 0) return null;

  return (
    <div className="fixed bottom-4 left-4 z-50 flex flex-col-reverse gap-2.5">
      {events.map((event) => (
        <EventCard
          key={event.id}
          event={event}
          onDismiss={() => dismiss(event.id)}
          onDone={() => dismiss(event.id)}
        />
      ))}
    </div>
  );
}
