/**
 * Overview Dashboard — priority-queue swimlane board showing all agent tasks
 * and scheduled cron jobs. Clicking a running task opens the VM execution viewer.
 */
import { cn } from "@/lib/utils";
import { BUILTIN_PERSONAS } from "@/lib/builtin-personas";
import NiceAvatar, { genConfig } from "react-nice-avatar";
import { ScrollArea } from "@/components/ui/scroll-area";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Activity,
  Calendar,
  CheckCircle2,
  Circle,
  Clock,
  Cpu,
  Database,
  Flag,
  Flame,
  HardDrive,
  LayoutDashboard,
  Loader2,
  Lock,
  MemoryStick,
  Network,
  Pause,
  Play,
  ShieldCheck,
  Terminal,
  Timer,
  TrendingUp,
} from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

// =============================================================================
// Data types
// =============================================================================

type Priority = "critical" | "high" | "medium" | "low";
type TaskStatus = "running" | "pending" | "done" | "paused";

interface LaneTask {
  id: string;
  label: string;
  status: TaskStatus;
  agentId: string;
  priority: Priority;
  /** Elapsed or estimated time */
  elapsed?: string;
  /** Dependency hint */
  dependsOn?: string;
}

interface CronJob {
  id: string;
  name: string;
  schedule: string;
  /** Natural language schedule */
  scheduleLabel: string;
  agentId: string;
  nextRun: string;
  lastStatus: "success" | "failed" | "running" | "never";
  enabled: boolean;
}

// =============================================================================
// VM execution log types
// =============================================================================

interface VmLogEntry {
  timestamp: string;
  level: "info" | "warn" | "error" | "debug" | "tee";
  message: string;
}

interface VmInfo {
  vmId: string;
  teeType: string;
  kernel: string;
  vcpu: number;
  memoryMb: number;
  diskGb: number;
  networkMode: string;
}

interface VmExecution {
  vm: VmInfo;
  logs: VmLogEntry[];
  /** Resource usage snapshots (simulated) */
  cpuPercent: number;
  memoryPercent: number;
  networkKbps: number;
  diskIops: number;
}

// =============================================================================
// Mock VM execution data per running task
// =============================================================================

const VM_EXECUTIONS: Record<string, VmExecution> = {
  "lt-1": {
    vm: { vmId: "vm-tee-a7f3", teeType: "Intel TDX", kernel: "linux-6.6-tdx", vcpu: 4, memoryMb: 8192, diskGb: 40, networkMode: "vhost-vsock" },
    cpuPercent: 87, memoryPercent: 72, networkKbps: 2340, diskIops: 1280,
    logs: [
      { timestamp: "10:32:01", level: "tee", message: "TEE attestation started — Intel TDX Quote Generation" },
      { timestamp: "10:32:02", level: "tee", message: "TDX Report: MRENCLAVE verified, MRSIGNER matched ✓" },
      { timestamp: "10:32:02", level: "tee", message: "Remote attestation complete — certificate chain valid" },
      { timestamp: "10:32:03", level: "info", message: "Mounting encrypted volume /dev/vda → /workspace (LUKS2 + integrity)" },
      { timestamp: "10:32:04", level: "info", message: "Loading model artifact: credit_score_v3.onnx (284 MB)" },
      { timestamp: "10:32:06", level: "info", message: "Decrypting training dataset: credit_train_2024.parquet.enc" },
      { timestamp: "10:32:08", level: "warn", message: "Feature flagged for removal: social_network_score (policy violation)" },
      { timestamp: "10:32:08", level: "warn", message: "Feature flagged for removal: contact_frequency (policy violation)" },
      { timestamp: "10:32:09", level: "info", message: "Rebuilding feature pipeline — 21/23 features retained" },
      { timestamp: "10:32:12", level: "info", message: "XGBoost training started: 21 features × 1,247,832 samples" },
      { timestamp: "10:32:15", level: "debug", message: "Epoch 50/500 — train_auc: 0.7412, val_auc: 0.7389" },
      { timestamp: "10:32:19", level: "debug", message: "Epoch 100/500 — train_auc: 0.7634, val_auc: 0.7601" },
      { timestamp: "10:32:24", level: "debug", message: "Epoch 200/500 — train_auc: 0.7821, val_auc: 0.7793" },
      { timestamp: "10:32:31", level: "debug", message: "Epoch 350/500 — train_auc: 0.7918, val_auc: 0.7876" },
      { timestamp: "10:32:38", level: "info", message: "Early stopping at epoch 412 — val_auc plateau: 0.7891" },
      { timestamp: "10:32:39", level: "info", message: "Model evaluation: AUC=0.7891, KS=0.4523, Gini=0.5782" },
      { timestamp: "10:32:40", level: "info", message: "SHAP explanation layer generating... 1,247,832 explanations" },
      { timestamp: "10:32:58", level: "info", message: "Sealing model + SHAP artifacts to TEE-bound encryption key" },
      { timestamp: "10:32:59", level: "tee", message: "Model artifact sealed with TDX sealing key (AES-256-GCM)" },
      { timestamp: "10:33:00", level: "info", message: "Uploading sealed model to secure artifact store..." },
    ],
  },
  "lt-4": {
    vm: { vmId: "vm-tee-c2d8", teeType: "AMD SEV-SNP", kernel: "linux-6.6-snp", vcpu: 2, memoryMb: 4096, diskGb: 20, networkMode: "vhost-vsock" },
    cpuPercent: 45, memoryPercent: 58, networkKbps: 890, diskIops: 340,
    logs: [
      { timestamp: "08:15:01", level: "tee", message: "SEV-SNP attestation — AMD vTPM measurement verified" },
      { timestamp: "08:15:02", level: "tee", message: "Guest policy: debug=off, migration=off, single-socket=on ✓" },
      { timestamp: "08:15:03", level: "info", message: "Loading reconciliation engine: smart-recon v2.3.0" },
      { timestamp: "08:15:04", level: "info", message: "Connecting to encrypted ledger database (TLS 1.3 + mTLS)" },
      { timestamp: "08:15:05", level: "info", message: "Fetching unmatched transactions: 12,847 invoices, 15,291 bank entries" },
      { timestamp: "08:15:08", level: "info", message: "Running ML matching model... confidence threshold: 0.92" },
      { timestamp: "08:15:14", level: "info", message: "Pass 1 complete: 11,203 auto-matched (87.2% hit rate)" },
      { timestamp: "08:15:16", level: "info", message: "Pass 2 (fuzzy): 1,089 additional matches (95.7% cumulative)" },
      { timestamp: "08:15:17", level: "warn", message: "555 transactions remain unmatched — flagged for manual review" },
      { timestamp: "08:15:18", level: "info", message: "Generating reconciliation report with amount verification..." },
      { timestamp: "08:15:20", level: "info", message: "Total matched: ¥847,293,120.56 — variance within ¥0.02 tolerance" },
      { timestamp: "08:15:21", level: "tee", message: "Report signed with TEE attestation key for audit trail" },
      { timestamp: "08:15:22", level: "info", message: "Syncing reconciliation results to SAP interface..." },
    ],
  },
  "lt-5": {
    vm: { vmId: "vm-std-e9a1", teeType: "Standard VM", kernel: "linux-6.6", vcpu: 8, memoryMb: 16384, diskGb: 100, networkMode: "virtio-net" },
    cpuPercent: 32, memoryPercent: 41, networkKbps: 5670, diskIops: 890,
    logs: [
      { timestamp: "11:48:01", level: "info", message: "Connecting to Kubernetes API: https://k8s-api:6443" },
      { timestamp: "11:48:02", level: "info", message: "kubectl get pods -n risk-engine -l app=risk-engine" },
      { timestamp: "11:48:02", level: "info", message: "Found 6 pods across 3 nodes (2 replicas × 3 zones)" },
      { timestamp: "11:48:03", level: "info", message: "Collecting Prometheus metrics: risk_engine_p99_latency_ms" },
      { timestamp: "11:48:04", level: "warn", message: "Current P99: 340ms (threshold: 200ms) — SLO violation" },
      { timestamp: "11:48:05", level: "info", message: "Running flame graph profiler on risk-engine-pod-7f8d..." },
      { timestamp: "11:48:12", level: "info", message: "Hotspot identified: RedisPool::get_connection (67% of latency)" },
      { timestamp: "11:48:13", level: "info", message: "Redis connection pool stats: max=50, active=50, waiting=127" },
      { timestamp: "11:48:14", level: "warn", message: "Connection pool exhausted — all 50 connections in use" },
      { timestamp: "11:48:15", level: "info", message: "Applying fix: increase pool_size 50 → 200, idle_timeout 30s → 60s" },
      { timestamp: "11:48:16", level: "info", message: "kubectl patch cm risk-engine-config --type merge..." },
      { timestamp: "11:48:17", level: "info", message: "Rolling restart: risk-engine deployment (maxUnavailable: 1)" },
      { timestamp: "11:48:25", level: "info", message: "Pod risk-engine-pod-a2c4 restarted — checking latency..." },
      { timestamp: "11:48:30", level: "info", message: "P99 latency dropping: 340ms → 128ms → 47ms ✓" },
      { timestamp: "11:48:31", level: "info", message: "SLO restored. Monitoring for 5 minutes for stability..." },
    ],
  },
  "lt-6": {
    vm: { vmId: "vm-tee-b5f2", teeType: "Intel TDX", kernel: "linux-6.6-tdx", vcpu: 16, memoryMb: 32768, diskGb: 200, networkMode: "vhost-vsock" },
    cpuPercent: 94, memoryPercent: 83, networkKbps: 450, diskIops: 2100,
    logs: [
      { timestamp: "11:12:01", level: "tee", message: "TDX attestation verified — enclave measurement matches" },
      { timestamp: "11:12:02", level: "info", message: "Loading factor_analysis skill v1.0.0" },
      { timestamp: "11:12:03", level: "info", message: "Decrypting market data: A-share daily 2022-2024 (3.8M records)" },
      { timestamp: "11:12:06", level: "info", message: "Computing momentum factor: window=20d, skip=5d" },
      { timestamp: "11:12:10", level: "info", message: "Factor matrix shape: (731 days × 5,102 stocks)" },
      { timestamp: "11:12:11", level: "info", message: "run_ic_analysis() — computing cross-sectional IC series" },
      { timestamp: "11:12:16", level: "debug", message: "IC series: mean=0.032, std=0.078, IR=0.41, win_rate=58.3%" },
      { timestamp: "11:12:17", level: "info", message: "run_layer_backtest() — 5 groups, 20-day holding" },
      { timestamp: "11:12:28", level: "debug", message: "Group returns (ann.): G1=+12.3%, G2=+6.1%, G3=+2.8%, G4=-1.2%, G5=-3.6%" },
      { timestamp: "11:12:29", level: "info", message: "Long-short: +14.9% ann., Sharpe 1.21, MaxDD -12.3%" },
      { timestamp: "11:12:30", level: "info", message: "run_attribution() — Barra multi-factor decomposition" },
      { timestamp: "11:12:38", level: "debug", message: "Alpha: +8.2% | Market: +3.1% | Size: -1.4% | Value: +2.1% | Residual: +2.9%" },
      { timestamp: "11:12:39", level: "info", message: "Generating HTML report with interactive charts..." },
      { timestamp: "11:12:42", level: "tee", message: "Sealing factor analysis results — proprietary strategy data" },
      { timestamp: "11:12:43", level: "info", message: "Report exported: /workspace/reports/momentum_factor_2024.html" },
    ],
  },
  "lt-7": {
    vm: { vmId: "vm-std-d4e7", teeType: "Standard VM", kernel: "linux-6.6", vcpu: 4, memoryMb: 8192, diskGb: 50, networkMode: "virtio-net" },
    cpuPercent: 28, memoryPercent: 35, networkKbps: 12400, diskIops: 560,
    logs: [
      { timestamp: "12:08:01", level: "info", message: "Loading pipeline_quality_monitor skill v1.0.0" },
      { timestamp: "12:08:02", level: "info", message: "Connecting to Kafka broker: kafka-1:9092 (SASL/SCRAM)" },
      { timestamp: "12:08:03", level: "info", message: "Connecting to ClickHouse: clickhouse-1:9000 (TLS)" },
      { timestamp: "12:08:04", level: "info", message: "Pipeline: trades-realtime (Kafka topic → ClickHouse table)" },
      { timestamp: "12:08:05", level: "info", message: "LatencyProbe: injecting watermark message into trades.raw..." },
      { timestamp: "12:08:06", level: "info", message: "LatencyProbe: watermark received in ClickHouse — latency: 287ms ✓" },
      { timestamp: "12:08:07", level: "info", message: "CompletenessChecker: Kafka offset=2,847,291 vs CH count=2,847,288" },
      { timestamp: "12:08:07", level: "warn", message: "3 missing records detected — checking replay buffer..." },
      { timestamp: "12:08:08", level: "info", message: "DuplicateDetector: scanning last 10,000 trade_ids..." },
      { timestamp: "12:08:09", level: "info", message: "Duplicate rate: 0.02% (within 0.1% threshold) ✓" },
      { timestamp: "12:08:10", level: "info", message: "SchemaValidator: comparing Kafka schema v3 ↔ ClickHouse DDL" },
      { timestamp: "12:08:10", level: "info", message: "Schema match: 12/12 fields aligned ✓" },
      { timestamp: "12:08:11", level: "info", message: "ThroughputMonitor: current 4,231 msg/min (baseline: 3,800 ± 600)" },
      { timestamp: "12:08:11", level: "info", message: "Health score: 96/100 — all checks passed" },
    ],
  },
  "lt-8": {
    vm: { vmId: "vm-std-f1a3", teeType: "Standard VM", kernel: "linux-6.6", vcpu: 2, memoryMb: 4096, diskGb: 20, networkMode: "virtio-net" },
    cpuPercent: 15, memoryPercent: 29, networkKbps: 340, diskIops: 120,
    logs: [
      { timestamp: "10:42:01", level: "info", message: "Loading PRD document: cross-border-payment-prd-v2.md" },
      { timestamp: "10:42:02", level: "info", message: "Parsing document structure — 7 sections, 23 sub-sections" },
      { timestamp: "10:42:03", level: "info", message: "Running technical feasibility analysis..." },
      { timestamp: "10:42:06", level: "info", message: "Section 3 (Core Features): 5 API endpoints identified" },
      { timestamp: "10:42:08", level: "info", message: "Cross-referencing with existing codebase modules..." },
      { timestamp: "10:42:10", level: "warn", message: "Feature gap: 'multi-currency virtual accounts' — no existing module" },
      { timestamp: "10:42:12", level: "info", message: "Estimating engineering effort per feature..." },
      { timestamp: "10:42:14", level: "info", message: "Reviewing compliance requirements against current capabilities..." },
      { timestamp: "10:42:16", level: "info", message: "FATF Travel Rule: integration with Chainalysis required" },
      { timestamp: "10:42:18", level: "info", message: "Generating technical review summary with recommendations..." },
    ],
  },
};

// =============================================================================
// Mock data — agent tasks in priority queue
// =============================================================================

const LANE_TASKS: LaneTask[] = [
  // Critical
  { id: "lt-1", label: "信用模型移除违规特征 + 重训练", status: "running", agentId: "risk-analyst", priority: "critical", elapsed: "2h 15m" },
  { id: "lt-2", label: "AML 规则命中率异常排查", status: "pending", agentId: "compliance-officer", priority: "critical" },
  // High
  { id: "lt-3", label: "模型 OOT 验证", status: "pending", agentId: "risk-analyst", priority: "high", dependsOn: "信用模型重训练" },
  { id: "lt-4", label: "Smart-reconciliation 加速上线", status: "running", agentId: "financial-analyst", priority: "high", elapsed: "4h 30m" },
  { id: "lt-5", label: "风控引擎 P99 延迟优化", status: "running", agentId: "devops-engineer", priority: "high", elapsed: "45m" },
  { id: "lt-6", label: "用新技能跑动量因子", status: "running", agentId: "quant-researcher", priority: "high", elapsed: "1h 20m" },
  // Medium
  { id: "lt-7", label: "接入管道监控并验证", status: "running", agentId: "data-engineer", priority: "medium", elapsed: "35m" },
  { id: "lt-8", label: "技术评审: 支付产品 PRD", status: "running", agentId: "product-manager", priority: "medium", elapsed: "2h" },
  { id: "lt-9", label: "Kafka 消费者组 lag 排查", status: "pending", agentId: "data-engineer", priority: "medium" },
  { id: "lt-10", label: "全面合规审计启动", status: "pending", agentId: "compliance-officer", priority: "medium" },
  // Low
  { id: "lt-11", label: "竞品情报技能优化", status: "done", agentId: "fullstack-engineer", priority: "low", elapsed: "6h" },
  { id: "lt-12", label: "入职指南文档更新", status: "pending", agentId: "product-manager", priority: "low" },
  { id: "lt-13", label: "数据字典同步", status: "done", agentId: "data-scientist", priority: "low", elapsed: "1h 10m" },
];

// =============================================================================
// Mock cron jobs
// =============================================================================

const CRON_JOBS: CronJob[] = [
  { id: "cj-1", name: "模型 PSI 漂移检测", schedule: "0 */4 * * *", scheduleLabel: "每 4 小时", agentId: "risk-analyst", nextRun: "14:00", lastStatus: "success", enabled: true },
  { id: "cj-2", name: "数据管道质量扫描", schedule: "0 2 * * *", scheduleLabel: "每天凌晨 2 点", agentId: "data-engineer", nextRun: "02:00", lastStatus: "success", enabled: true },
  { id: "cj-3", name: "竞品动态采集", schedule: "0 8 * * 1-5", scheduleLabel: "工作日上午 8 点", agentId: "product-manager", nextRun: "明天 08:00", lastStatus: "success", enabled: true },
  { id: "cj-4", name: "反洗钱日报生成", schedule: "30 9 * * *", scheduleLabel: "每天 9:30", agentId: "compliance-officer", nextRun: "09:30", lastStatus: "running", enabled: true },
  { id: "cj-5", name: "K8s 节点健康巡检", schedule: "*/30 * * * *", scheduleLabel: "每 30 分钟", agentId: "devops-engineer", nextRun: "12:30", lastStatus: "success", enabled: true },
  { id: "cj-6", name: "发票自动核对", schedule: "0 10 * * 1,3,5", scheduleLabel: "周一三五 10 点", agentId: "financial-analyst", nextRun: "周三 10:00", lastStatus: "failed", enabled: true },
  { id: "cj-7", name: "因子收益归因报告", schedule: "0 18 * * 5", scheduleLabel: "每周五下午 6 点", agentId: "quant-researcher", nextRun: "周五 18:00", lastStatus: "success", enabled: true },
  { id: "cj-8", name: "数据库全量备份", schedule: "0 3 * * 0", scheduleLabel: "每周日凌晨 3 点", agentId: "devops-engineer", nextRun: "周日 03:00", lastStatus: "success", enabled: false },
];

// =============================================================================
// Priority lane config
// =============================================================================

const PRIORITY_CONFIG: Record<Priority, { label: string; color: string; borderColor: string; headerBg: string; icon: typeof Flame }> = {
  critical: { label: "紧急", color: "text-red-600 dark:text-red-400", borderColor: "border-red-500/30", headerBg: "bg-red-500/10", icon: Flame },
  high: { label: "高优先级", color: "text-orange-600 dark:text-orange-400", borderColor: "border-orange-500/30", headerBg: "bg-orange-500/10", icon: Flag },
  medium: { label: "中优先级", color: "text-blue-600 dark:text-blue-400", borderColor: "border-blue-500/30", headerBg: "bg-blue-500/10", icon: TrendingUp },
  low: { label: "低优先级", color: "text-muted-foreground", borderColor: "border-border", headerBg: "bg-muted/50", icon: Circle },
};

// =============================================================================
// Helpers
// =============================================================================

function getPersona(agentId: string) {
  return BUILTIN_PERSONAS.find((p) => p.id === agentId);
}

function StatusBadge({ status }: { status: TaskStatus }) {
  switch (status) {
    case "running":
      return (
        <span className="inline-flex items-center gap-1 rounded-full bg-primary/10 px-1.5 py-0.5 text-[10px] font-medium text-primary">
          <Loader2 className="size-2.5 animate-spin" />执行中
        </span>
      );
    case "pending":
      return (
        <span className="inline-flex items-center gap-1 rounded-full bg-amber-500/10 px-1.5 py-0.5 text-[10px] font-medium text-amber-600 dark:text-amber-400">
          <Clock className="size-2.5" />排队中
        </span>
      );
    case "done":
      return (
        <span className="inline-flex items-center gap-1 rounded-full bg-primary/10 px-1.5 py-0.5 text-[10px] font-medium text-primary">
          <CheckCircle2 className="size-2.5" />已完成
        </span>
      );
    case "paused":
      return (
        <span className="inline-flex items-center gap-1 rounded-full bg-muted px-1.5 py-0.5 text-[10px] font-medium text-muted-foreground">
          <Pause className="size-2.5" />已暂停
        </span>
      );
  }
}

// =============================================================================
// Resource gauge bar
// =============================================================================

function ResourceGauge({ label, icon: Icon, value, unit, color }: {
  label: string;
  icon: typeof Cpu;
  value: number;
  unit: string;
  color: string;
}) {
  return (
    <div className="flex-1 min-w-0">
      <div className="flex items-center justify-between mb-1">
        <div className="flex items-center gap-1">
          <Icon className={cn("size-3", color)} />
          <span className="text-[10px] text-muted-foreground">{label}</span>
        </div>
        <span className={cn("text-[10px] font-mono font-bold", color)}>{value}{unit}</span>
      </div>
      <div className="h-1.5 rounded-full bg-muted overflow-hidden">
        <div
          className={cn("h-full rounded-full transition-all duration-1000", {
            "bg-primary": value < 50,
            "bg-amber-500": value >= 50 && value < 80,
            "bg-red-500": value >= 80,
          })}
          style={{ width: `${Math.min(value, 100)}%` }}
        />
      </div>
    </div>
  );
}

// =============================================================================
// VM Execution Dialog — terminal log viewer + resource metrics
// =============================================================================

function VmExecutionDialog({ task, open, onClose }: { task: LaneTask; open: boolean; onClose: () => void }) {
  const exec = VM_EXECUTIONS[task.id];
  const persona = getPersona(task.agentId);
  const avatarCfg = persona ? genConfig(persona.avatar) : genConfig();
  const [visibleCount, setVisibleCount] = useState(0);
  const logEndRef = useRef<HTMLDivElement>(null);
  const timerRef = useRef<ReturnType<typeof setTimeout>>();

  // Progressive log reveal
  useEffect(() => {
    if (!open || !exec) return;
    setVisibleCount(0);

    let idx = 0;
    const reveal = () => {
      idx++;
      setVisibleCount(idx);
      if (idx < exec.logs.length) {
        timerRef.current = setTimeout(reveal, 200 + Math.random() * 400);
      }
    };
    timerRef.current = setTimeout(reveal, 300);

    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, [open, exec]);

  // Auto-scroll log
  useEffect(() => {
    logEndRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [visibleCount]);

  if (!exec) return null;

  const isTee = exec.vm.teeType !== "Standard VM";
  const visibleLogs = exec.logs.slice(0, visibleCount);

  const logLevelColor: Record<VmLogEntry["level"], string> = {
    info: "text-blue-400",
    warn: "text-amber-400",
    error: "text-red-400",
    debug: "text-zinc-500",
    tee: "text-cyan-400",
  };

  return (
    <Dialog open={open} onOpenChange={(v) => !v && onClose()}>
      <DialogContent className="max-w-3xl max-h-[85vh] flex flex-col gap-0 p-0 overflow-hidden">
        {/* Header */}
        <DialogHeader className="px-5 pt-5 pb-3 border-b shrink-0">
          <div className="flex items-center gap-3">
            <div className={cn(
              "flex items-center justify-center size-9 rounded-lg shrink-0",
              isTee ? "bg-primary/10" : "bg-blue-500/10",
            )}>
              {isTee ? <ShieldCheck className="size-4.5 text-primary" /> : <Terminal className="size-4.5 text-blue-500" />}
            </div>
            <div className="flex-1 min-w-0">
              <DialogTitle className="text-sm font-bold truncate">{task.label}</DialogTitle>
              <DialogDescription className="text-[11px] mt-0.5">
                <span className="font-mono">{exec.vm.vmId}</span>
                <span className="mx-1.5 text-muted-foreground/40">·</span>
                <span className={isTee ? "text-primary font-medium" : ""}>{exec.vm.teeType}</span>
                <span className="mx-1.5 text-muted-foreground/40">·</span>
                {task.elapsed && <span>已运行 {task.elapsed}</span>}
              </DialogDescription>
            </div>
            <div className="flex items-center gap-1.5 shrink-0">
              <NiceAvatar className="size-5" {...avatarCfg} />
              <span className="text-[11px] text-muted-foreground">{persona?.name}</span>
            </div>
          </div>
        </DialogHeader>

        {/* VM Info + Resource Metrics */}
        <div className="px-5 py-3 border-b bg-muted/20 shrink-0">
          {/* VM specs */}
          <div className="flex items-center gap-4 mb-3 text-[10px] text-muted-foreground">
            <span className="flex items-center gap-1"><Cpu className="size-3" />{exec.vm.vcpu} vCPU</span>
            <span className="flex items-center gap-1"><MemoryStick className="size-3" />{exec.vm.memoryMb >= 1024 ? `${(exec.vm.memoryMb / 1024).toFixed(0)} GB` : `${exec.vm.memoryMb} MB`} RAM</span>
            <span className="flex items-center gap-1"><HardDrive className="size-3" />{exec.vm.diskGb} GB</span>
            <span className="flex items-center gap-1"><Network className="size-3" />{exec.vm.networkMode}</span>
            <span className="flex items-center gap-1 font-mono">{exec.vm.kernel}</span>
            {isTee && (
              <span className="flex items-center gap-1 text-primary font-medium">
                <Lock className="size-3" />TEE Enclave
              </span>
            )}
          </div>
          {/* Resource gauges */}
          <div className="flex items-center gap-4">
            <ResourceGauge label="CPU" icon={Cpu} value={exec.cpuPercent} unit="%" color="text-blue-500" />
            <ResourceGauge label="Memory" icon={MemoryStick} value={exec.memoryPercent} unit="%" color="text-purple-500" />
            <ResourceGauge label="Network" icon={Activity} value={Math.round(exec.networkKbps / 100)} unit=" Mb/s" color="text-cyan-500" />
            <ResourceGauge label="Disk I/O" icon={Database} value={Math.round(exec.diskIops / 30)} unit=" IOPS" color="text-orange-500" />
          </div>
        </div>

        {/* Terminal log */}
        <div className="flex-1 min-h-0 bg-zinc-950 overflow-hidden">
          <div className="flex items-center gap-2 px-4 py-1.5 bg-zinc-900/80 border-b border-zinc-800">
            <Terminal className="size-3 text-zinc-500" />
            <span className="text-[10px] font-mono text-zinc-500">execution log</span>
            <span className="ml-auto text-[10px] font-mono text-zinc-600">
              {visibleCount}/{exec.logs.length} entries
            </span>
            {visibleCount < exec.logs.length && (
              <Loader2 className="size-2.5 text-primary animate-spin" />
            )}
          </div>
          <ScrollArea className="h-[320px]">
            <div className="px-4 py-2 font-mono text-[11px] leading-[1.7]">
              {visibleLogs.map((log, i) => (
                <div key={i} className="flex gap-2 hover:bg-zinc-800/40 px-1 -mx-1 rounded">
                  <span className="text-zinc-600 shrink-0 select-none">{log.timestamp}</span>
                  <span className={cn("shrink-0 w-[38px] text-right uppercase", logLevelColor[log.level])}>
                    {log.level === "tee" ? "TEE" : log.level}
                  </span>
                  <span className={cn(
                    "flex-1",
                    log.level === "debug" ? "text-zinc-500" :
                    log.level === "warn" ? "text-amber-300/90" :
                    log.level === "error" ? "text-red-300" :
                    log.level === "tee" ? "text-cyan-300/90" :
                    "text-zinc-300",
                  )}>
                    {log.message}
                  </span>
                </div>
              ))}
              {visibleCount < exec.logs.length && (
                <div className="flex items-center gap-1.5 text-zinc-600 mt-1">
                  <span className="inline-block w-1.5 h-3.5 bg-primary/80 animate-pulse" />
                </div>
              )}
              <div ref={logEndRef} />
            </div>
          </ScrollArea>
        </div>
      </DialogContent>
    </Dialog>
  );
}

// =============================================================================
// Swimlane task card
// =============================================================================

function SwimlaneTaskCard({ task, onClick }: { task: LaneTask; onClick?: () => void }) {
  const persona = getPersona(task.agentId);
  const avatarCfg = persona ? genConfig(persona.avatar) : genConfig();
  const isClickable = task.status === "running" && VM_EXECUTIONS[task.id];

  return (
    <div
      className={cn(
        "rounded-lg border bg-card p-2.5 transition-shadow",
        isClickable ? "hover:shadow-md hover:border-primary/40 cursor-pointer active:scale-[0.98]" : "hover:shadow-sm",
      )}
      onClick={isClickable ? onClick : undefined}
      role={isClickable ? "button" : undefined}
      tabIndex={isClickable ? 0 : undefined}
      onKeyDown={isClickable ? (e) => { if (e.key === "Enter" || e.key === " ") onClick?.(); } : undefined}
    >
      <div className="flex items-start justify-between gap-1.5 mb-1.5">
        <span className="text-[11px] font-medium leading-tight flex-1 line-clamp-2">{task.label}</span>
        {isClickable && (
          <Terminal className="size-3 text-primary/60 shrink-0 mt-0.5" />
        )}
      </div>
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-1.5">
          <NiceAvatar className="size-3.5" {...avatarCfg} />
          <span className="text-[10px] text-muted-foreground truncate max-w-[80px]">{persona?.name || task.agentId}</span>
        </div>
        <StatusBadge status={task.status} />
      </div>
      {(task.elapsed || task.dependsOn) && (
        <div className="flex items-center gap-2 mt-1.5">
          {task.elapsed && (
            <span className="text-[10px] text-muted-foreground/70 flex items-center gap-0.5">
              <Timer className="size-2.5" />{task.elapsed}
            </span>
          )}
          {task.dependsOn && (
            <span className="text-[10px] text-muted-foreground/50 truncate">
              ← {task.dependsOn}
            </span>
          )}
        </div>
      )}
    </div>
  );
}

// =============================================================================
// Swimlane column (one per priority)
// =============================================================================

function SwimlaneColumn({ priority, tasks, onTaskClick }: { priority: Priority; tasks: LaneTask[]; onTaskClick: (task: LaneTask) => void }) {
  const cfg = PRIORITY_CONFIG[priority];
  const Icon = cfg.icon;
  const running = tasks.filter((t) => t.status === "running").length;
  const pending = tasks.filter((t) => t.status === "pending").length;

  return (
    <div className={cn("flex flex-col rounded-xl border min-w-[220px] flex-1", cfg.borderColor)}>
      {/* Column header */}
      <div className={cn("flex items-center gap-2 px-3 py-2.5 rounded-t-xl border-b", cfg.headerBg, cfg.borderColor)}>
        <Icon className={cn("size-3.5 shrink-0", cfg.color)} />
        <span className={cn("text-xs font-bold uppercase tracking-wide", cfg.color)}>{cfg.label}</span>
        <span className="ml-auto rounded-full bg-background/80 px-1.5 py-0.5 text-[10px] font-bold text-foreground/70">{tasks.length}</span>
      </div>

      {/* Column counters */}
      {(running > 0 || pending > 0) && (
        <div className="flex items-center gap-2 px-3 py-1.5 border-b border-border/50">
          {running > 0 && (
            <span className="text-[10px] text-primary font-medium flex items-center gap-0.5">
              <Loader2 className="size-2 animate-spin" />{running}
            </span>
          )}
          {pending > 0 && (
            <span className="text-[10px] text-amber-600 dark:text-amber-400 font-medium flex items-center gap-0.5">
              <Clock className="size-2" />{pending}
            </span>
          )}
        </div>
      )}

      {/* Task cards */}
      <div className="flex-1 p-2 space-y-2 overflow-y-auto">
        {tasks.length === 0 ? (
          <div className="flex items-center justify-center h-16 text-[10px] text-muted-foreground/40">
            暂无任务
          </div>
        ) : (
          tasks.map((task) => (
            <SwimlaneTaskCard key={task.id} task={task} onClick={() => onTaskClick(task)} />
          ))
        )}
      </div>
    </div>
  );
}

// =============================================================================
// Cron job row
// =============================================================================

function CronJobRow({ job }: { job: CronJob }) {
  const persona = getPersona(job.agentId);
  const avatarCfg = persona ? genConfig(persona.avatar) : genConfig();

  return (
    <div className={cn(
      "flex items-center gap-3 rounded-lg border bg-card px-3 py-2.5 transition-colors",
      !job.enabled && "opacity-50",
    )}>
      <div className={cn(
        "flex items-center justify-center size-7 rounded-lg shrink-0",
        job.lastStatus === "failed" ? "bg-red-500/10" : job.lastStatus === "running" ? "bg-primary/10" : "bg-muted",
      )}>
        {job.lastStatus === "running" ? (
          <Loader2 className="size-3.5 text-primary animate-spin" />
        ) : job.lastStatus === "failed" ? (
          <span className="text-[10px] text-red-500 font-bold">!</span>
        ) : (
          <Calendar className="size-3.5 text-muted-foreground" />
        )}
      </div>
      <div className="flex-1 min-w-0">
        <div className="flex items-center gap-2">
          <span className="text-xs font-medium truncate">{job.name}</span>
          {!job.enabled && (
            <span className="text-[9px] text-muted-foreground bg-muted rounded px-1">已停用</span>
          )}
        </div>
        <div className="flex items-center gap-2 mt-0.5">
          <code className="text-[10px] font-mono text-muted-foreground">{job.schedule}</code>
          <span className="text-[10px] text-muted-foreground/60">·</span>
          <span className="text-[10px] text-muted-foreground">{job.scheduleLabel}</span>
        </div>
      </div>
      <div className="flex items-center gap-2 shrink-0">
        <div className="flex items-center gap-1">
          <NiceAvatar className="size-4" {...avatarCfg} />
          <span className="text-[10px] text-muted-foreground">{persona?.name}</span>
        </div>
        <div className="text-right ml-2">
          <div className="text-[10px] text-muted-foreground">下次执行</div>
          <div className="text-[10px] font-mono font-medium">{job.nextRun}</div>
        </div>
        <button
          type="button"
          className={cn(
            "size-6 flex items-center justify-center rounded-md border transition-colors",
            job.enabled ? "text-primary hover:bg-primary/10" : "text-muted-foreground hover:bg-muted",
          )}
          aria-label={job.enabled ? "暂停" : "启用"}
        >
          {job.enabled ? <Pause className="size-3" /> : <Play className="size-3" />}
        </button>
      </div>
    </div>
  );
}

// =============================================================================
// Stats summary bar
// =============================================================================

function StatsSummary({ tasks }: { tasks: LaneTask[] }) {
  const running = tasks.filter((t) => t.status === "running").length;
  const pending = tasks.filter((t) => t.status === "pending").length;
  const done = tasks.filter((t) => t.status === "done").length;
  const total = tasks.length;
  const agents = new Set(tasks.filter((t) => t.status !== "done").map((t) => t.agentId)).size;

  const stats = [
    { label: "总任务", value: total, color: "text-foreground" },
    { label: "执行中", value: running, color: "text-primary" },
    { label: "排队中", value: pending, color: "text-amber-600 dark:text-amber-400" },
    { label: "已完成", value: done, color: "text-primary" },
    { label: "活跃智能体", value: agents, color: "text-purple-600 dark:text-purple-400" },
  ];

  return (
    <div className="grid grid-cols-5 gap-3 mb-6">
      {stats.map((s) => (
        <div key={s.label} className="rounded-lg border bg-card px-3 py-2.5 text-center">
          <div className={cn("text-xl font-bold", s.color)}>{s.value}</div>
          <div className="text-[10px] text-muted-foreground mt-0.5">{s.label}</div>
        </div>
      ))}
    </div>
  );
}

// =============================================================================
// Main Dashboard
// =============================================================================

export default function OverviewDashboard() {
  const [selectedTask, setSelectedTask] = useState<LaneTask | null>(null);

  const tasksByPriority = useMemo(() => {
    const map: Record<Priority, LaneTask[]> = { critical: [], high: [], medium: [], low: [] };
    for (const task of LANE_TASKS) {
      map[task.priority].push(task);
    }
    return map;
  }, []);

  const handleTaskClick = useCallback((task: LaneTask) => {
    if (task.status === "running" && VM_EXECUTIONS[task.id]) {
      setSelectedTask(task);
    }
  }, []);

  return (
    <div className="flex flex-col h-full overflow-hidden">
      {/* Header */}
      <div className="flex items-center gap-3 px-6 py-4 border-b shrink-0">
        <div className="flex items-center justify-center size-8 rounded-lg bg-primary/10">
          <LayoutDashboard className="size-4 text-primary" />
        </div>
        <div>
          <h1 className="text-base font-bold">任务总览</h1>
          <p className="text-xs text-muted-foreground">所有智能体的实时任务队列与定时任务</p>
        </div>
      </div>

      <ScrollArea className="flex-1">
        <div className="px-6 py-5">
          {/* Stats */}
          <StatsSummary tasks={LANE_TASKS} />

          {/* Priority swimlanes — horizontal columns */}
          <div className="mb-6">
            <div className="flex items-center gap-2 mb-3">
              <Flag className="size-4 text-primary" />
              <h2 className="text-sm font-bold">优先级队列</h2>
              <span className="text-[10px] text-muted-foreground">A3S Lane · 泳道视图</span>
            </div>
            <div className="flex gap-3 overflow-x-auto pb-2">
              {(["critical", "high", "medium", "low"] as Priority[]).map((p) => (
                <SwimlaneColumn key={p} priority={p} tasks={tasksByPriority[p]} onTaskClick={handleTaskClick} />
              ))}
            </div>
          </div>

          {/* Cron jobs */}
          <div>
            <div className="flex items-center gap-2 mb-3">
              <Calendar className="size-4 text-primary" />
              <h2 className="text-sm font-bold">定时任务</h2>
              <span className="text-[10px] text-muted-foreground">{CRON_JOBS.filter((j) => j.enabled).length} 个活跃</span>
            </div>
            <div className="space-y-2">
              {CRON_JOBS.map((job) => (
                <CronJobRow key={job.id} job={job} />
              ))}
            </div>
          </div>
        </div>
      </ScrollArea>

      {/* VM Execution Dialog */}
      {selectedTask && (
        <VmExecutionDialog
          task={selectedTask}
          open={!!selectedTask}
          onClose={() => setSelectedTask(null)}
        />
      )}
    </div>
  );
}
