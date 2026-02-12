import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover";
import { ScrollArea } from "@/components/ui/scroll-area";
import { cn } from "@/lib/utils";
import { BUILTIN_PERSONAS } from "@/lib/builtin-personas";
import agentModel from "@/models/agent.model";
import personaModel from "@/models/persona.model";
import type { AgentProcessInfo } from "@/typings/agent";
import type { AgentPersona } from "@/typings/persona";
import {
  Code2,
  FileText,
  LayoutDashboard,
  Lock,
  Plus,
  Search,
  Sparkles,
  Terminal,
  Wrench,
} from "lucide-react";
import Avatar, { genConfig } from "react-nice-avatar";
import { useMemo, useState } from "react";
import { useSnapshot } from "valtio";

// ---------------------------------------------------------------------------
// Mock skills & tools per persona
// ---------------------------------------------------------------------------

interface AgentSkill {
  name: string;
  description: string;
}

interface AgentTool {
  name: string;
  icon: "terminal" | "file" | "code" | "wrench";
}

const MOCK_SKILLS: Record<string, AgentSkill[]> = {
  "fullstack-engineer": [
    { name: "factor_analysis", description: "批量因子检验 — IC、分层回测、归因" },
    { name: "model_monitor", description: "模型监控 — PSI、AUC 衰减、漂移告警" },
    { name: "k8s_upgrade_preflight", description: "K8s 升级预检自动化" },
    { name: "pipeline_quality_monitor", description: "数据管道质量监控" },
    { name: "competitive_intel", description: "竞品情报自动采集与分析" },
  ],
  "data-engineer": [
    { name: "pipeline_quality_monitor", description: "数据管道质量监控" },
    { name: "schema_migration", description: "数据库 Schema 迁移管理" },
  ],
  "devops-engineer": [
    { name: "k8s_upgrade_preflight", description: "K8s 升级预检自动化" },
    { name: "incident_runbook", description: "故障应急 Runbook 执行" },
  ],
  "quant-researcher": [
    { name: "factor_analysis", description: "批量因子检验 — IC、分层回测、归因" },
    { name: "backtest_report", description: "策略回测报告生成" },
  ],
  "risk-analyst": [
    { name: "model_monitor", description: "模型监控 — PSI、AUC 衰减、漂移告警" },
    { name: "credit_feature_eng", description: "信用特征工程自动化" },
  ],
  "product-manager": [
    { name: "competitive_intel", description: "竞品情报自动采集与分析" },
    { name: "prd_template", description: "PRD 模板生成与校验" },
  ],
  "data-scientist": [
    { name: "factor_analysis", description: "批量因子检验 — IC、分层回测、归因" },
    { name: "ab_test_analyzer", description: "A/B 测试显著性分析" },
  ],
  "financial-analyst": [
    { name: "payment_approval", description: "供应商付款审批与执行" },
    { name: "invoice_reconcile", description: "发票自动核对与对账" },
  ],
};

const MOCK_TOOLS: Record<string, AgentTool[]> = {
  "fullstack-engineer": [
    { name: "Read", icon: "file" },
    { name: "Write", icon: "file" },
    { name: "Edit", icon: "code" },
    { name: "Bash", icon: "terminal" },
    { name: "Grep", icon: "file" },
    { name: "SkillRegister", icon: "wrench" },
  ],
  "data-engineer": [
    { name: "Bash", icon: "terminal" },
    { name: "Read", icon: "file" },
    { name: "Write", icon: "file" },
    { name: "SQLExecute", icon: "code" },
  ],
  "devops-engineer": [
    { name: "Bash", icon: "terminal" },
    { name: "Read", icon: "file" },
    { name: "Write", icon: "file" },
    { name: "Kubectl", icon: "terminal" },
  ],
  "quant-researcher": [
    { name: "Bash", icon: "terminal" },
    { name: "KnowledgeBase", icon: "file" },
    { name: "PythonExec", icon: "code" },
  ],
  "risk-analyst": [
    { name: "KnowledgeBase", icon: "file" },
    { name: "DocQuery", icon: "file" },
    { name: "PythonExec", icon: "code" },
  ],
  "product-manager": [
    { name: "KnowledgeBase", icon: "file" },
    { name: "DocQuery", icon: "file" },
    { name: "WebSearch", icon: "wrench" },
  ],
  "data-scientist": [
    { name: "KnowledgeBase", icon: "file" },
    { name: "PythonExec", icon: "code" },
    { name: "Bash", icon: "terminal" },
  ],
  "financial-analyst": [
    { name: "KnowledgeBase", icon: "file" },
    { name: "DocQuery", icon: "file" },
    { name: "TEEPayment", icon: "wrench" },
    { name: "InvoiceVerify", icon: "wrench" },
  ],
};

const toolIconMap = {
  terminal: Terminal,
  file: FileText,
  code: Code2,
  wrench: Wrench,
};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function relativeTime(ts: number): string {
  const now = Date.now();
  const diff = now - ts;
  const mins = Math.floor(diff / 60000);
  if (mins < 1) return "刚刚";
  if (mins < 60) return `${mins}分钟前`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}小时前`;
  const days = Math.floor(hours / 24);
  if (days < 7) return `${days}天前`;
  return new Date(ts).toLocaleDateString();
}

// ---------------------------------------------------------------------------
// Agent item — single row, no expand
// ---------------------------------------------------------------------------

function AgentItem({
  persona,
  sessions,
  isActive,
  unreadCount,
  onSelect,
}: {
  persona: AgentPersona;
  sessions: AgentProcessInfo[];
  isActive: boolean;
  unreadCount: number;
  onSelect: () => void;
}) {
  const cfg = useMemo(() => genConfig(persona.avatar), [persona.avatar]);
  const activeSessions = useMemo(
    () => [...sessions].filter((s) => !s.archived).sort((a, b) => b.created_at - a.created_at),
    [sessions],
  );
  const hasActiveSessions = activeSessions.length > 0;
  const skills = MOCK_SKILLS[persona.id] || [];
  const tools = MOCK_TOOLS[persona.id] || [];

  return (
    <div
      role="option"
      aria-selected={isActive}
      tabIndex={-1}
      className={cn(
        "group flex items-center gap-3 px-3 py-2.5 w-full cursor-pointer transition-colors",
        "hover:bg-foreground/[0.04]",
        isActive && "bg-foreground/[0.03]",
      )}
      onClick={onSelect}
      onKeyDown={(e) => {
        if (e.key === "Enter" || e.key === " ") {
          e.preventDefault();
          onSelect();
        }
      }}
    >
      {/* Avatar with popover on click */}
      <Popover>
        <PopoverTrigger asChild>
          <button
            type="button"
            className="shrink-0 rounded-full focus:outline-none focus:ring-2 focus:ring-primary/50"
            onClick={(e) => e.stopPropagation()}
            aria-label={`查看 ${persona.name} 详情`}
          >
            <Avatar className="w-9 h-9" {...cfg} />
          </button>
        </PopoverTrigger>
        <PopoverContent side="right" align="start" className="w-72 p-0" onClick={(e) => e.stopPropagation()}>
          {/* Header */}
          <div className="flex items-center gap-3 px-4 pt-4 pb-3 border-b">
            <Avatar className="w-10 h-10 shrink-0" {...cfg} />
            <div className="min-w-0">
              <div className="flex items-center gap-1">
                <span className="text-sm font-semibold truncate">{persona.name}</span>
                {persona.undeletable && <Lock className="size-3 text-muted-foreground shrink-0" />}
              </div>
              <p className="text-xs text-muted-foreground truncate">{persona.description}</p>
            </div>
          </div>

          {/* Skills */}
          <div className="px-4 py-3 border-b">
            <div className="flex items-center gap-1.5 mb-2">
              <Sparkles className="size-3.5 text-primary" />
              <span className="text-xs font-semibold">技能</span>
              <span className="text-[10px] text-muted-foreground">({skills.length})</span>
            </div>
            {skills.length > 0 ? (
              <div className="space-y-1.5">
                {skills.map((s) => (
                  <div key={s.name} className="flex items-start gap-2">
                    <code className="text-[11px] font-mono text-primary bg-primary/5 rounded px-1 py-0.5 shrink-0">{s.name}</code>
                    <span className="text-[11px] text-muted-foreground leading-tight">{s.description}</span>
                  </div>
                ))}
              </div>
            ) : (
              <p className="text-[11px] text-muted-foreground">暂无技能</p>
            )}
          </div>

          {/* Tools */}
          <div className="px-4 py-3">
            <div className="flex items-center gap-1.5 mb-2">
              <Wrench className="size-3.5 text-primary" />
              <span className="text-xs font-semibold">工具</span>
              <span className="text-[10px] text-muted-foreground">({tools.length})</span>
            </div>
            {tools.length > 0 ? (
              <div className="flex flex-wrap gap-1.5">
                {tools.map((t) => {
                  const Icon = toolIconMap[t.icon];
                  return (
                    <span
                      key={t.name}
                      className="inline-flex items-center gap-1 rounded-md border px-2 py-1 text-[11px] text-muted-foreground"
                    >
                      <Icon className="size-3" />
                      {t.name}
                    </span>
                  );
                })}
              </div>
            ) : (
              <p className="text-[11px] text-muted-foreground">暂无工具</p>
            )}
          </div>
        </PopoverContent>
      </Popover>

      <div className="flex-1 min-w-0">
        <div className="flex justify-between items-baseline">
          <div className="flex items-center gap-1 min-w-0">
            <span className="text-sm font-medium truncate">{persona.name}</span>
            {persona.undeletable && <Lock className="size-3 text-muted-foreground shrink-0" />}
          </div>
          <div className="flex items-center gap-1.5 shrink-0 ml-2">
            {unreadCount > 0 && (
              <span className="flex items-center justify-center min-w-[18px] h-[18px] rounded-full bg-primary text-primary-foreground text-[10px] font-bold px-1 leading-none">
                {unreadCount > 99 ? "99+" : unreadCount}
              </span>
            )}
            <time className="text-[10px] text-muted-foreground">
              {hasActiveSessions ? relativeTime(activeSessions[0].created_at) : ""}
            </time>
          </div>
        </div>
        <p className="text-xs text-muted-foreground truncate mt-0.5">
          {persona.description}
        </p>
      </div>
    </div>
  );
}

// ---------------------------------------------------------------------------
// Main List
// ---------------------------------------------------------------------------

export default function AgentSessionList() {
  const snap = useSnapshot(agentModel.state);
  const sdkSessions = snap.sdkSessions;
  const currentSessionId = snap.currentSessionId;
  const personaSnap = useSnapshot(personaModel.state);

  const [search, setSearch] = useState("");
  const q = search.trim().toLowerCase();

  // Group sessions by persona
  const sessionsByPersona = useMemo(() => {
    const map: Record<string, AgentProcessInfo[]> = {};
    for (const s of sdkSessions) {
      const pid = personaSnap.sessionPersonas[s.session_id] || "unknown";
      if (!map[pid]) map[pid] = [];
      map[pid].push(s as AgentProcessInfo);
    }
    return map;
  }, [sdkSessions, personaSnap.sessionPersonas]);

  // Which persona is currently active
  const currentPersonaId = currentSessionId
    ? personaSnap.sessionPersonas[currentSessionId] || null
    : null;

  // Filter agents by search
  const filteredPersonas = useMemo(() => {
    if (!q) return BUILTIN_PERSONAS;
    return BUILTIN_PERSONAS.filter(
      (p) =>
        p.name.toLowerCase().includes(q) ||
        p.description.toLowerCase().includes(q) ||
        p.id.toLowerCase().includes(q),
    );
  }, [q]);

  // Compute total unread per persona (sum of all session unreads)
  const unreadByPersona = useMemo(() => {
    const map: Record<string, number> = {};
    for (const s of sdkSessions) {
      const pid = personaSnap.sessionPersonas[s.session_id] || "unknown";
      const count = snap.unreadCounts[s.session_id] || 0;
      if (count > 0) {
        map[pid] = (map[pid] || 0) + count;
      }
    }
    return map;
  }, [sdkSessions, personaSnap.sessionPersonas, snap.unreadCounts]);

  // Select agent → select its latest session + clear unread
  const handleSelectAgent = (personaId: string) => {
    const sessions = [...(sessionsByPersona[personaId] || [])]
      .filter((s) => !s.archived)
      .sort((a, b) => b.created_at - a.created_at);
    if (sessions.length > 0) {
      const sid = sessions[0].session_id;
      agentModel.setCurrentSession(sid);
      agentModel.clearUnread(sid);
    }
  };

  return (
    <div className="flex flex-col h-full overflow-hidden border-r">
      {/* Header */}
      <div className="flex items-center justify-between px-3 py-3 border-b">
        <h2 className="text-sm font-semibold truncate">智能体</h2>
        <Button variant="ghost" size="icon" className="size-7" aria-label="自定义新建会话">
          <Plus className="size-4" />
        </Button>
      </div>

      {/* Search */}
      <div className="px-3 py-2 border-b">
        <div className="relative">
          <Search className="absolute left-2.5 top-2.5 size-4 text-muted-foreground" />
          <Input
            placeholder="搜索智能体..."
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="pl-8 h-9"
          />
        </div>
      </div>

      <ScrollArea className="flex-1 w-full">
        <div role="listbox" aria-label="智能体列表" className="w-full">
          {/* Overview entry — always at top */}
          {!q && (
            <div
              role="option"
              aria-selected={currentSessionId === "__overview__"}
              tabIndex={-1}
              className={cn(
                "group flex items-center gap-3 px-3 py-2.5 w-full cursor-pointer transition-colors",
                "hover:bg-foreground/[0.04]",
                currentSessionId === "__overview__" && "bg-foreground/[0.03]",
              )}
              onClick={() => agentModel.setCurrentSession("__overview__")}
              onKeyDown={(e) => {
                if (e.key === "Enter" || e.key === " ") {
                  e.preventDefault();
                  agentModel.setCurrentSession("__overview__");
                }
              }}
            >
              <div className="flex items-center justify-center size-9 rounded-full bg-primary/10 shrink-0">
                <LayoutDashboard className="size-4 text-primary" />
              </div>
              <div className="flex-1 min-w-0">
                <div className="text-sm font-medium">任务总览</div>
                <p className="text-xs text-muted-foreground truncate mt-0.5">
                  所有智能体的任务队列与定时任务
                </p>
              </div>
            </div>
          )}

          {/* Separator */}
          {!q && <div className="mx-3 border-b" />}

          {filteredPersonas.map((persona) => (
            <AgentItem
              key={persona.id}
              persona={persona}
              sessions={sessionsByPersona[persona.id] || []}
              isActive={currentPersonaId === persona.id && currentSessionId !== "__overview__"}
              unreadCount={unreadByPersona[persona.id] || 0}
              onSelect={() => handleSelectAgent(persona.id)}
            />
          ))}

          {filteredPersonas.length === 0 && (
            <div className="px-3 py-8 text-center text-sm text-muted-foreground">
              {q ? "未找到匹配的智能体" : "选择智能体开始对话"}
            </div>
          )}
        </div>
      </ScrollArea>
    </div>
  );
}
