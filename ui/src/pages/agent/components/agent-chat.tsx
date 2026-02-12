import { MemoizedMarkdown } from "@/components/custom/memoized-markdown";
import TiptapEditor, { type TiptapEditorRef } from "@/components/custom/tiptap-editor";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@/components/ui/resizable";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { BUILTIN_PERSONAS } from "@/lib/builtin-personas";
import { cn } from "@/lib/utils";
import agentModel from "@/models/agent.model";
import globalModel from "@/models/global.model";
import personaModel from "@/models/persona.model";
import settingsModel from "@/models/settings.model";
import dayjs from "dayjs";
import {
  Bell,
  Brain,
  CheckCircle2,
  ChevronDown,
  ChevronRight,
  Circle,
  Code2,
  Compass,
  CornerDownLeft,
  Cpu,
  CreditCard,
  Database,
  Eye,
  EyeOff,
  FileCode,
  FileText,
  Gauge,
  Image,
  KeyRound,
  Loader2,
  Lock,
  MessageSquare,
  Paperclip,
  Search,
  Send,
  ShieldCheck,
  Sparkles,
  Terminal,
  Wrench,
  X,
  Zap
} from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import NiceAvatar, { genConfig } from "react-nice-avatar";
import { Virtuoso, type VirtuosoHandle } from "react-virtuoso";
import { toast } from "sonner";
import { useSnapshot } from "valtio";

// =============================================================================
// Rich message types
// =============================================================================

interface ThinkingBlock {
  type: "thinking";
  content: string;
  durationMs?: number;
}

interface ToolCallBlock {
  type: "tool_call";
  tool: string;
  input: string;
  output?: string;
  durationMs?: number;
  isError?: boolean;
}

interface SubAgentBlock {
  type: "sub_agent";
  agentName: string;
  task: string;
  result?: string;
  durationMs?: number;
}

interface TextBlock {
  type: "text";
  content: string;
}

interface HilOption {
  label: string;
  value: string;
}

interface HilBlock {
  type: "hil";
  action: string;
  targetAgent: string;
  description: string;
  confirmed?: boolean;
  /** Interaction mode: confirm = simple yes/no, single = radio, multi = checkbox */
  mode?: "confirm" | "single" | "multi";
  options?: HilOption[];
  /** Whether to show a supplementary text input alongside options */
  allowInput?: boolean;
  inputPlaceholder?: string;
}

interface PaymentBlock {
  type: "payment";
  payee: string;
  amount: string;
  currency: string;
  description: string;
  invoiceId?: string;
}

interface EventBlock {
  type: "event";
  /** Event source category */
  source: "task" | "news" | "social" | "market" | "system" | "compliance";
  /** Event topic / subscription name */
  topic: string;
  /** Brief event summary */
  summary: string;
  /** Original event payload or detail (collapsible) */
  detail?: string;
  /** Timestamp of the event itself */
  eventTime?: number;
}

interface TeeDataQueryBlock {
  type: "tee_data_query";
  /** SQL or query description */
  query: string;
  /** Data source / database name */
  dataSource: string;
  /** Why sensitive data is needed */
  reason: string;
  /** Privacy level label */
  privacyLevel: string;
  /** Redacted result rows to display after authorization */
  redactedResult: { columns: string[]; rows: string[][] };
  /** Summary of redaction applied */
  redactionSummary: string;
}

type RichBlock = ThinkingBlock | ToolCallBlock | SubAgentBlock | TextBlock | HilBlock | PaymentBlock | EventBlock | TeeDataQueryBlock;

type MessageSource = "app" | "dingtalk" | "feishu" | "wecom";

interface RichMessage {
  id: string;
  role: "user" | "assistant" | "system";
  blocks: RichBlock[];
  timestamp: number;
  /** Where this user message was sent from */
  source?: MessageSource;
}

// =============================================================================
// Collapsible Block Components
// =============================================================================

function ThinkingBlockView({ block }: { block: ThinkingBlock }) {
  const [open, setOpen] = useState(false);
  return (
    <div className="my-1">
      <button
        type="button"
        className="flex items-center gap-1.5 text-xs text-muted-foreground hover:text-foreground transition-colors py-0.5"
        onClick={() => setOpen(!open)}
      >
        {open ? <ChevronDown className="size-3" /> : <ChevronRight className="size-3" />}
        <Brain className="size-3.5 text-primary" />
        <span className="font-medium">思考过程</span>
        {block.durationMs && (
          <span className="text-muted-foreground/60">{(block.durationMs / 1000).toFixed(1)}s</span>
        )}
      </button>
      {open && (
        <div className="ml-5 mt-1 pl-3 border-l-2 border-primary/30 text-xs text-muted-foreground whitespace-pre-wrap leading-relaxed">
          {block.content}
        </div>
      )}
    </div>
  );
}

function ToolCallBlockView({ block }: { block: ToolCallBlock }) {
  const [open, setOpen] = useState(false);

  const iconMap: Record<string, React.ReactNode> = {
    Read: <FileText className="size-3.5 text-primary" />,
    Write: <FileCode className="size-3.5 text-primary" />,
    Edit: <Code2 className="size-3.5 text-primary" />,
    Bash: <Terminal className="size-3.5 text-primary" />,
    Grep: <Search className="size-3.5 text-primary" />,
    KnowledgeBase: <Search className="size-3.5 text-primary" />,
    DocQuery: <FileText className="size-3.5 text-primary" />,
    WebSearch: <Search className="size-3.5 text-primary" />,
    SQLExecute: <Code2 className="size-3.5 text-primary" />,
    PythonExec: <Code2 className="size-3.5 text-primary" />,
    Kubectl: <Terminal className="size-3.5 text-primary" />,
    TEEPayment: <Lock className="size-3.5 text-primary" />,
    InvoiceVerify: <FileText className="size-3.5 text-primary" />,
  };

  const toolIcon = iconMap[block.tool] || <Wrench className="size-3.5 text-primary" />;

  return (
    <div className="my-1">
      <button
        type="button"
        className="flex items-center gap-1.5 text-xs hover:text-foreground transition-colors py-0.5 w-full text-left"
        onClick={() => setOpen(!open)}
      >
        {open ? <ChevronDown className="size-3" /> : <ChevronRight className="size-3" />}
        {toolIcon}
        <span className="font-mono font-medium text-foreground">{block.tool}</span>
        <span className="text-muted-foreground truncate flex-1">{block.input}</span>
        {block.durationMs && (
          <span className="text-muted-foreground/60 shrink-0">{(block.durationMs / 1000).toFixed(1)}s</span>
        )}
        {block.isError ? (
          <span className="text-destructive text-[10px] shrink-0">✗</span>
        ) : block.output ? (
          <span className="text-primary text-[10px] shrink-0">✓</span>
        ) : null}
      </button>
      {open && (
        <div className="ml-5 mt-1 space-y-1">
          <pre className="rounded bg-muted/50 p-2 text-[11px] font-mono overflow-x-auto max-h-48 whitespace-pre-wrap text-muted-foreground">
            {block.input}
          </pre>
          {block.output && (
            <pre className={cn(
              "rounded p-2 text-[11px] font-mono overflow-x-auto max-h-48 whitespace-pre-wrap",
              block.isError ? "bg-destructive/5 text-destructive border border-destructive/20" : "bg-primary/5 text-foreground border border-primary/10",
            )}>
              {block.output}
            </pre>
          )}
        </div>
      )}
    </div>
  );
}

function SubAgentBlockView({ block }: { block: SubAgentBlock }) {
  const [open, setOpen] = useState(false);
  return (
    <div className="my-1.5">
      <button
        type="button"
        className="flex items-center gap-1.5 text-xs hover:text-foreground transition-colors py-0.5 w-full text-left"
        onClick={() => setOpen(!open)}
      >
        {open ? <ChevronDown className="size-3" /> : <ChevronRight className="size-3" />}
        <span className="inline-flex items-center gap-0.5 rounded-full bg-primary/10 px-2 py-0.5 text-primary font-medium">
          @{block.agentName}
        </span>
        <span className="text-muted-foreground truncate flex-1">{block.task}</span>
        {block.durationMs && (
          <span className="text-muted-foreground/60 shrink-0">{(block.durationMs / 1000).toFixed(1)}s</span>
        )}
      </button>
      {open && block.result && (
        <div className="ml-5 mt-1 pl-3 border-l-2 border-primary/30 text-xs text-muted-foreground whitespace-pre-wrap leading-relaxed">
          {block.result}
        </div>
      )}
    </div>
  );
}

function HilBlockView({ block, onConfirm }: { block: HilBlock; onConfirm?: () => void }) {
  const mode = block.mode || "confirm";
  const [status, setStatus] = useState<"pending" | "confirmed" | "rejected">(
    block.confirmed === true ? "confirmed" : block.confirmed === false ? "rejected" : "pending",
  );
  const [selected, setSelected] = useState<Set<string>>(new Set());
  const [extraInput, setExtraInput] = useState("");

  const fireToast = useCallback(() => {
    const targetPersona = BUILTIN_PERSONAS.find((p) => p.name === block.targetAgent);
    const avatarCfg = targetPersona ? genConfig(targetPersona.avatar) : genConfig();
    toast.custom(
      () => (
        <div className="flex items-center gap-3 bg-background border rounded-lg shadow-lg px-4 py-3 w-[340px]">
          <NiceAvatar className="size-9 shrink-0" {...avatarCfg} />
          <div className="flex-1 min-w-0">
            <div className="text-sm font-medium">{block.targetAgent} 已收到任务</div>
            <p className="text-xs text-muted-foreground truncate mt-0.5">{block.description}</p>
          </div>
        </div>
      ),
      { duration: 3000 },
    );
  }, [block.targetAgent, block.description]);

  const handleConfirm = useCallback(() => {
    setStatus("confirmed");
    fireToast();
    onConfirm?.();
  }, [fireToast, onConfirm]);

  const handleReject = useCallback(() => {
    setStatus("rejected");
    toast.error("已拒绝该操作", { duration: 2000 });
  }, []);

  const toggleOption = useCallback((value: string) => {
    setSelected((prev) => {
      const next = new Set(prev);
      if (mode === "single") {
        return new Set([value]);
      }
      if (next.has(value)) next.delete(value);
      else next.add(value);
      return next;
    });
  }, [mode]);

  const hasOptions = block.options && block.options.length > 0;
  const canSubmit = mode === "confirm" || selected.size > 0 || (block.allowInput && extraInput.trim());

  return (
    <div className="my-2 rounded-lg border border-primary/20 bg-primary/[0.03] p-3">
      <div className="flex items-center gap-2 mb-2">
        <ShieldCheck className="size-4 text-primary" />
        <span className="text-xs font-semibold text-primary">需要确认</span>
      </div>
      <div className="text-sm mb-2">
        <span className="text-foreground">{block.action}</span>
        <span className="mx-1.5 text-muted-foreground">→</span>
        <span className="inline-flex items-center gap-0.5 rounded-full bg-primary/10 px-2 py-0.5 text-xs text-primary font-medium">
          @{block.targetAgent}
        </span>
      </div>
      <p className="text-xs text-muted-foreground mb-3">{block.description}</p>

      {status === "pending" ? (
        <>
          {/* Options: single or multi select */}
          {hasOptions && (
            <div className="space-y-1.5 mb-3">
              {block.options!.map((opt) => {
                const isSelected = selected.has(opt.value);
                return (
                  <label
                    key={opt.value}
                    className={cn(
                      "flex items-center gap-2 rounded-md border px-3 py-2 text-xs cursor-pointer transition-colors",
                      isSelected
                        ? "border-primary bg-primary/5 text-foreground"
                        : "border-border hover:bg-foreground/[0.02] text-muted-foreground",
                    )}
                  >
                    <input
                      type={mode === "single" ? "radio" : "checkbox"}
                      name="hil-option"
                      checked={isSelected}
                      onChange={() => toggleOption(opt.value)}
                      className="accent-primary size-3.5"
                    />
                    <span>{opt.label}</span>
                  </label>
                );
              })}
            </div>
          )}

          {/* Supplementary text input */}
          {block.allowInput && (
            <input
              type="text"
              className="w-full rounded-md border px-3 py-1.5 text-xs mb-3 bg-background focus:outline-none focus:ring-1 focus:ring-primary"
              placeholder={block.inputPlaceholder || "补充说明..."}
              value={extraInput}
              onChange={(e) => setExtraInput(e.target.value)}
            />
          )}

          {/* Action buttons */}
          <div className="flex items-center gap-2">
            <button
              type="button"
              className={cn(
                "flex items-center gap-1.5 rounded-md bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground transition-colors",
                canSubmit ? "hover:bg-primary/90" : "opacity-50 cursor-not-allowed",
              )}
              disabled={!canSubmit}
              onClick={handleConfirm}
            >
              <CheckCircle2 className="size-3" />
              确认执行
            </button>
            <button
              type="button"
              className="flex items-center gap-1.5 rounded-md border px-3 py-1.5 text-xs font-medium text-muted-foreground hover:bg-foreground/[0.04] transition-colors"
              onClick={handleReject}
            >
              <X className="size-3" />
              拒绝
            </button>
          </div>
        </>
      ) : status === "confirmed" ? (
        <div className="space-y-1">
          <div className="flex items-center gap-1.5 text-xs text-primary">
            <CheckCircle2 className="size-3.5" />
            <span>已确认 — 任务已派发给</span>
            <span className="inline-flex items-center rounded-full bg-primary/10 px-1.5 py-0.5 text-primary font-medium">@{block.targetAgent}</span>
          </div>
          {selected.size > 0 && (
            <div className="text-[11px] text-muted-foreground ml-5">
              选择：{[...selected].map((v) => block.options?.find((o) => o.value === v)?.label).join("、")}
            </div>
          )}
          {extraInput.trim() && (
            <div className="text-[11px] text-muted-foreground ml-5">补充：{extraInput}</div>
          )}
        </div>
      ) : (
        <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
          <X className="size-3.5" />
          <span>已拒绝</span>
        </div>
      )}
    </div>
  );
}

// =============================================================================
// Payment Block — TEE-secured payment authorization
// =============================================================================

function PaymentBlockView({ block }: { block: PaymentBlock }) {
  const [status, setStatus] = useState<"pending" | "authorized" | "rejected">("pending");
  const [showTeeDialog, setShowTeeDialog] = useState(false);
  const [teeProgress, setTeeProgress] = useState(0);
  const [verifyStep, setVerifyStep] = useState<"input" | "executing" | "done">("input");
  const [verifyMethod, setVerifyMethod] = useState<"password" | "sms">("password");
  const [verifyValue, setVerifyValue] = useState("");
  const [smsSent, setSmsSent] = useState(false);
  const [smsCountdown, setSmsCountdown] = useState(0);
  const [verifyError, setVerifyError] = useState("");

  const handleSendSms = useCallback(() => {
    setSmsSent(true);
    setSmsCountdown(60);
    const timer = setInterval(() => {
      setSmsCountdown((prev) => {
        if (prev <= 1) { clearInterval(timer); return 0; }
        return prev - 1;
      });
    }, 1000);
  }, []);

  const startTeeExecution = useCallback(() => {
    setVerifyStep("executing");
    setTeeProgress(0);
    const steps = [10, 30, 55, 75, 90, 100];
    steps.forEach((p, i) => {
      setTimeout(() => {
        setTeeProgress(p);
        if (p === 100) {
          setTimeout(() => {
            setVerifyStep("done");
            setStatus("authorized");
            setShowTeeDialog(false);
            toast.success("支付已在 TEE 中安全执行", { duration: 3000 });
          }, 800);
        }
      }, (i + 1) * 600);
    });
  }, []);

  const handleVerifySubmit = useCallback(() => {
    if (!verifyValue.trim()) {
      setVerifyError(verifyMethod === "password" ? "请输入支付密码" : "请输入验证码");
      return;
    }
    setVerifyError("");
    // Mock: any non-empty input passes
    startTeeExecution();
  }, [verifyValue, verifyMethod, startTeeExecution]);

  const handleAuthorize = useCallback(() => {
    setShowTeeDialog(true);
    setVerifyStep("input");
    setVerifyValue("");
    setVerifyError("");
    setTeeProgress(0);
  }, []);

  const handleReject = useCallback(() => {
    setStatus("rejected");
  }, []);

  return (
    <>
      <div className="my-2 rounded-lg border border-primary/20 bg-primary/[0.03] p-3">
        <div className="flex items-center gap-2 mb-3">
          <CreditCard className="size-4 text-primary" />
          <span className="text-xs font-semibold text-primary">支付请求</span>
        </div>

        {/* Payment details */}
        <div className="rounded-md border bg-background p-3 mb-3 space-y-2">
          <div className="flex justify-between items-center">
            <span className="text-xs text-muted-foreground">收款方</span>
            <span className="text-sm font-medium">{block.payee}</span>
          </div>
          <div className="flex justify-between items-center">
            <span className="text-xs text-muted-foreground">金额</span>
            <span className="text-lg font-bold text-primary">{block.currency} {block.amount}</span>
          </div>
          <div className="flex justify-between items-center">
            <span className="text-xs text-muted-foreground">用途</span>
            <span className="text-xs">{block.description}</span>
          </div>
          {block.invoiceId && (
            <div className="flex justify-between items-center">
              <span className="text-xs text-muted-foreground">发票号</span>
              <code className="text-[11px] font-mono text-muted-foreground">{block.invoiceId}</code>
            </div>
          )}
        </div>

        {status === "pending" ? (
          <div className="flex items-center gap-2">
            <button
              type="button"
              className="flex items-center gap-1.5 rounded-md bg-primary px-3 py-1.5 text-xs font-medium text-primary-foreground hover:bg-primary/90 transition-colors"
              onClick={handleAuthorize}
            >
              <Lock className="size-3" />
              授权支付
            </button>
            <button
              type="button"
              className="flex items-center gap-1.5 rounded-md border px-3 py-1.5 text-xs font-medium text-muted-foreground hover:bg-foreground/[0.04] transition-colors"
              onClick={handleReject}
            >
              <X className="size-3" />
              拒绝
            </button>
            <span className="text-[10px] text-muted-foreground ml-auto flex items-center gap-1">
              <Lock className="size-2.5" />
              TEE 安全执行
            </span>
          </div>
        ) : status === "authorized" ? (
          <div className="flex items-center gap-1.5 text-xs text-primary">
            <CheckCircle2 className="size-3.5" />
            <span>已授权 — 支付已在可信执行环境中安全完成</span>
          </div>
        ) : (
          <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
            <X className="size-3.5" />
            <span>已拒绝支付</span>
          </div>
        )}
      </div>

      {/* TEE execution dialog */}
      <Dialog open={showTeeDialog} onOpenChange={(open) => { if (verifyStep !== "executing") setShowTeeDialog(open); }}>
        <DialogContent className="sm:max-w-md" onPointerDownOutside={(e) => { if (verifyStep === "executing") e.preventDefault(); }}>
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Lock className="size-5 text-primary" />
              {verifyStep === "input" ? "身份验证" : "可信执行环境 (TEE)"}
            </DialogTitle>
            <DialogDescription>
              {verifyStep === "input"
                ? "请验证身份以授权此笔支付，验证通过后将在 TEE 中安全执行。"
                : "此支付操作正在安全隔离的可信执行环境中执行，确保交易数据不被篡改或泄露。"}
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4 py-4">
            {/* Transaction summary — always visible */}
            <div className="rounded-lg border p-3 space-y-1.5 text-xs">
              <div className="flex justify-between">
                <span className="text-muted-foreground">收款方</span>
                <span className="font-medium">{block.payee}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-muted-foreground">金额</span>
                <span className="font-bold text-primary">{block.currency} {block.amount}</span>
              </div>
              {block.invoiceId && (
                <div className="flex justify-between">
                  <span className="text-muted-foreground">发票号</span>
                  <code className="font-mono">{block.invoiceId}</code>
                </div>
              )}
            </div>

            {/* Step 1: Identity verification */}
            {verifyStep === "input" && (
              <>
                {/* Method toggle */}
                <div className="flex rounded-lg border p-0.5 gap-0.5">
                  <button
                    type="button"
                    className={cn(
                      "flex-1 rounded-md py-1.5 text-xs font-medium transition-colors",
                      verifyMethod === "password" ? "bg-foreground text-background" : "text-muted-foreground hover:text-foreground",
                    )}
                    onClick={() => { setVerifyMethod("password"); setVerifyValue(""); setVerifyError(""); }}
                  >
                    支付密码
                  </button>
                  <button
                    type="button"
                    className={cn(
                      "flex-1 rounded-md py-1.5 text-xs font-medium transition-colors",
                      verifyMethod === "sms" ? "bg-foreground text-background" : "text-muted-foreground hover:text-foreground",
                    )}
                    onClick={() => { setVerifyMethod("sms"); setVerifyValue(""); setVerifyError(""); }}
                  >
                    短信验证码
                  </button>
                </div>

                {/* Input */}
                {verifyMethod === "password" ? (
                  <div className="space-y-1.5">
                    <input
                      type="password"
                      className={cn(
                        "w-full rounded-md border px-3 py-2 text-sm bg-background focus:outline-none focus:ring-2 focus:ring-primary/50",
                        verifyError && "border-red-400 focus:ring-red-500/50",
                      )}
                      placeholder="请输入 6 位支付密码"
                      maxLength={6}
                      value={verifyValue}
                      onChange={(e) => { setVerifyValue(e.target.value); setVerifyError(""); }}
                      onKeyDown={(e) => { if (e.key === "Enter") handleVerifySubmit(); }}
                      autoFocus
                    />
                    {verifyError && <p className="text-[11px] text-red-500">{verifyError}</p>}
                  </div>
                ) : (
                  <div className="space-y-1.5">
                    <div className="flex gap-2">
                      <input
                        type="text"
                        className={cn(
                          "flex-1 rounded-md border px-3 py-2 text-sm bg-background focus:outline-none focus:ring-2 focus:ring-primary/50",
                          verifyError && "border-red-400 focus:ring-red-500/50",
                        )}
                        placeholder="请输入 6 位验证码"
                        maxLength={6}
                        value={verifyValue}
                        onChange={(e) => { setVerifyValue(e.target.value); setVerifyError(""); }}
                        onKeyDown={(e) => { if (e.key === "Enter") handleVerifySubmit(); }}
                        autoFocus
                      />
                      <button
                        type="button"
                        className={cn(
                          "shrink-0 rounded-md border px-3 py-2 text-xs font-medium transition-colors",
                          smsCountdown > 0 ? "text-muted-foreground cursor-not-allowed" : "text-primary hover:bg-primary/5",
                        )}
                        disabled={smsCountdown > 0}
                        onClick={handleSendSms}
                      >
                        {smsCountdown > 0 ? `${smsCountdown}s` : smsSent ? "重新发送" : "获取验证码"}
                      </button>
                    </div>
                    {smsSent && !verifyError && (
                      <p className="text-[11px] text-muted-foreground">验证码已发送至 138****6789</p>
                    )}
                    {verifyError && <p className="text-[11px] text-red-500">{verifyError}</p>}
                  </div>
                )}

                {/* Submit */}
                <button
                  type="button"
                  className="w-full rounded-md bg-primary py-2 text-sm font-medium text-primary-foreground hover:bg-primary/90 transition-colors"
                  onClick={handleVerifySubmit}
                >
                  确认并支付
                </button>
              </>
            )}

            {/* Step 2: TEE execution */}
            {verifyStep === "executing" && (
              <>
                <div className="rounded-lg border bg-muted/30 p-3 space-y-2 text-xs">
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">执行环境</span>
                    <span className="font-mono text-primary">Intel SGX Enclave</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">远程证明</span>
                    <span className="font-mono text-primary">已验证 ✓</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">加密通道</span>
                    <span className="font-mono text-primary">TLS 1.3 + RA-TLS</span>
                  </div>
                </div>

                <div className="space-y-2">
                  <div className="flex justify-between text-xs">
                    <span className="text-muted-foreground">执行进度</span>
                    <span className="font-medium">{teeProgress}%</span>
                  </div>
                  <div className="h-2 rounded-full bg-muted overflow-hidden">
                    <div
                      className="h-full rounded-full bg-primary transition-all duration-500"
                      style={{ width: `${teeProgress}%` }}
                    />
                  </div>
                  <div className="text-[11px] text-muted-foreground">
                    {teeProgress < 30 && "正在建立安全通道..."}
                    {teeProgress >= 30 && teeProgress < 60 && "验证交易参数..."}
                    {teeProgress >= 60 && teeProgress < 90 && "执行支付指令..."}
                    {teeProgress >= 90 && teeProgress < 100 && "生成交易凭证..."}
                    {teeProgress === 100 && "✓ 交易已安全完成"}
                  </div>
                </div>
              </>
            )}
          </div>
        </DialogContent>
      </Dialog>
    </>
  );
}

// =============================================================================
// TEE Data Query Block — sensitive data query with authorization + redaction
// =============================================================================

const TEE_QUERY_STEPS = [
  { pct: 10, label: "建立 RA-TLS 安全通道..." },
  { pct: 25, label: "远程证明验证 Enclave..." },
  { pct: 45, label: "在 TEE 中执行查询..." },
  { pct: 65, label: "隐私分类器扫描结果..." },
  { pct: 80, label: "应用脱敏规则..." },
  { pct: 95, label: "生成审计日志..." },
  { pct: 100, label: "✓ 查询完成，结果已脱敏" },
];

function TeeDataQueryBlockView({ block, onConfirm }: { block: TeeDataQueryBlock; onConfirm?: () => void }) {
  const [status, setStatus] = useState<"pending" | "authorized" | "rejected">("pending");
  const [showDialog, setShowDialog] = useState(false);
  const [step, setStep] = useState<"auth" | "executing" | "done">("auth");
  const [password, setPassword] = useState("");
  const [pwdError, setPwdError] = useState("");
  const [progress, setProgress] = useState(0);
  const [progressLabel, setProgressLabel] = useState("");
  const [showRedacted, setShowRedacted] = useState(true);

  const startTeeQuery = useCallback(() => {
    setStep("executing");
    setProgress(0);
    TEE_QUERY_STEPS.forEach((s, i) => {
      setTimeout(() => {
        setProgress(s.pct);
        setProgressLabel(s.label);
        if (s.pct === 100) {
          setTimeout(() => {
            setStep("done");
            setStatus("authorized");
            setShowDialog(false);
            toast.success("敏感数据已在 TEE 中查询并脱敏返回", { duration: 3000 });
            onConfirm?.();
          }, 800);
        }
      }, (i + 1) * 700);
    });
  }, [onConfirm]);

  const handleSubmitAuth = useCallback(() => {
    if (!password.trim()) {
      setPwdError("请输入授权密码");
      return;
    }
    setPwdError("");
    startTeeQuery();
  }, [password, startTeeQuery]);

  const handleAuthorize = useCallback(() => {
    setShowDialog(true);
    setStep("auth");
    setPassword("");
    setPwdError("");
    setProgress(0);
  }, []);

  return (
    <>
      <div className="my-2 rounded-lg border border-amber-500/20 bg-amber-500/[0.03] p-3">
        <div className="flex items-center gap-2 mb-2">
          <Database className="size-4 text-amber-600 dark:text-amber-400" />
          <span className="text-xs font-semibold text-amber-700 dark:text-amber-400">敏感数据查询</span>
          <span className="ml-auto inline-flex items-center gap-1 rounded-full bg-red-500/10 px-2 py-0.5 text-[10px] font-medium text-red-600 dark:text-red-400">
            <Lock className="size-2.5" />
            {block.privacyLevel}
          </span>
        </div>

        {/* Query info */}
        <div className="rounded-md border bg-background p-3 mb-3 space-y-2 text-xs">
          <div className="flex justify-between items-start gap-4">
            <span className="text-muted-foreground shrink-0">数据源</span>
            <span className="font-mono text-right">{block.dataSource}</span>
          </div>
          <div className="flex justify-between items-start gap-4">
            <span className="text-muted-foreground shrink-0">查询语句</span>
            <code className="font-mono text-[11px] text-right break-all">{block.query}</code>
          </div>
          <div className="flex justify-between items-start gap-4">
            <span className="text-muted-foreground shrink-0">查询原因</span>
            <span className="text-right">{block.reason}</span>
          </div>
        </div>

        {status === "pending" ? (
          <div className="flex items-center gap-2">
            <button
              type="button"
              className="flex items-center gap-1.5 rounded-md bg-amber-600 px-3 py-1.5 text-xs font-medium text-white hover:bg-amber-600/90 transition-colors"
              onClick={handleAuthorize}
            >
              <KeyRound className="size-3" />
              授权查询
            </button>
            <button
              type="button"
              className="flex items-center gap-1.5 rounded-md border px-3 py-1.5 text-xs font-medium text-muted-foreground hover:bg-foreground/[0.04] transition-colors"
              onClick={() => setStatus("rejected")}
            >
              <X className="size-3" />
              拒绝
            </button>
            <span className="text-[10px] text-muted-foreground ml-auto flex items-center gap-1">
              <Lock className="size-2.5" />
              TEE 隔离查询 + 自动脱敏
            </span>
          </div>
        ) : status === "authorized" ? (
          <div className="space-y-3">
            <div className="flex items-center gap-1.5 text-xs text-amber-700 dark:text-amber-400">
              <CheckCircle2 className="size-3.5" />
              <span>已授权 — 数据已在 TEE 中查询并脱敏</span>
              <button
                type="button"
                className="ml-auto flex items-center gap-1 text-[10px] text-muted-foreground hover:text-foreground transition-colors"
                onClick={() => setShowRedacted(!showRedacted)}
              >
                {showRedacted ? <EyeOff className="size-3" /> : <Eye className="size-3" />}
                {showRedacted ? "隐藏结果" : "查看结果"}
              </button>
            </div>

            {showRedacted && (
              <>
                {/* Redacted result table */}
                <div className="rounded-md border overflow-hidden">
                  <table className="w-full text-xs">
                    <thead>
                      <tr className="bg-muted/50 border-b">
                        {block.redactedResult.columns.map((col) => (
                          <th key={col} className="text-left px-3 py-1.5 font-medium text-muted-foreground">{col}</th>
                        ))}
                      </tr>
                    </thead>
                    <tbody>
                      {block.redactedResult.rows.map((row, ri) => (
                        <tr key={ri} className="border-b last:border-b-0 hover:bg-muted/20">
                          {row.map((cell, ci) => (
                            <td key={ci} className="px-3 py-1.5 font-mono text-[11px]">
                              {cell.includes("***") ? (
                                <span className="text-amber-600 dark:text-amber-400">{cell}</span>
                              ) : cell}
                            </td>
                          ))}
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>

                {/* Redaction summary */}
                <div className="flex items-start gap-1.5 text-[10px] text-muted-foreground bg-muted/30 rounded-md px-2.5 py-1.5">
                  <ShieldCheck className="size-3 shrink-0 mt-0.5 text-amber-600 dark:text-amber-400" />
                  <span>{block.redactionSummary}</span>
                </div>
              </>
            )}
          </div>
        ) : (
          <div className="flex items-center gap-1.5 text-xs text-muted-foreground">
            <X className="size-3.5" />
            <span>已拒绝查询</span>
          </div>
        )}
      </div>

      {/* TEE authorization dialog */}
      <Dialog open={showDialog} onOpenChange={(open) => { if (step !== "executing") setShowDialog(open); }}>
        <DialogContent className="sm:max-w-md" onPointerDownOutside={(e) => { if (step === "executing") e.preventDefault(); }}>
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Lock className="size-5 text-amber-600 dark:text-amber-400" />
              {step === "auth" ? "敏感数据授权" : "可信执行环境 (TEE)"}
            </DialogTitle>
            <DialogDescription>
              {step === "auth"
                ? "该查询涉及敏感数据，需要授权密码验证身份。验证通过后将在 TEE 中执行查询并自动脱敏结果。"
                : "查询正在安全隔离的可信执行环境中执行，原始数据不会离开 Enclave，返回结果将自动脱敏。"}
            </DialogDescription>
          </DialogHeader>

          <div className="space-y-4 py-4">
            {/* Query summary */}
            <div className="rounded-lg border p-3 space-y-1.5 text-xs">
              <div className="flex justify-between">
                <span className="text-muted-foreground">数据源</span>
                <span className="font-mono font-medium">{block.dataSource}</span>
              </div>
              <div className="flex justify-between">
                <span className="text-muted-foreground">隐私级别</span>
                <span className="font-medium text-red-600 dark:text-red-400">{block.privacyLevel}</span>
              </div>
              <div className="pt-1 border-t">
                <span className="text-muted-foreground">查询</span>
                <code className="block mt-1 font-mono text-[11px] break-all">{block.query}</code>
              </div>
            </div>

            {/* Auth step */}
            {step === "auth" && (
              <>
                <div className="space-y-1.5">
                  <label className="text-xs font-medium text-foreground">授权密码</label>
                  <input
                    type="password"
                    className={cn(
                      "w-full rounded-md border px-3 py-2 text-sm bg-background focus:outline-none focus:ring-2 focus:ring-amber-500/50",
                      pwdError && "border-red-400 focus:ring-red-500/50",
                    )}
                    placeholder="请输入数据访问授权密码"
                    value={password}
                    onChange={(e) => { setPassword(e.target.value); setPwdError(""); }}
                    onKeyDown={(e) => { if (e.key === "Enter") handleSubmitAuth(); }}
                    autoFocus
                  />
                  {pwdError && <p className="text-[11px] text-red-500">{pwdError}</p>}
                </div>
                <button
                  type="button"
                  className="w-full rounded-md bg-amber-600 py-2 text-sm font-medium text-white hover:bg-amber-600/90 transition-colors"
                  onClick={handleSubmitAuth}
                >
                  验证并在 TEE 中执行查询
                </button>
              </>
            )}

            {/* Execution step */}
            {step === "executing" && (
              <>
                <div className="rounded-lg border bg-muted/30 p-3 space-y-2 text-xs">
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">执行环境</span>
                    <span className="font-mono text-amber-600 dark:text-amber-400">Intel SGX Enclave</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">远程证明</span>
                    <span className="font-mono text-amber-600 dark:text-amber-400">已验证 ✓</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">加密通道</span>
                    <span className="font-mono text-amber-600 dark:text-amber-400">TLS 1.3 + RA-TLS</span>
                  </div>
                  <div className="flex justify-between">
                    <span className="text-muted-foreground">脱敏策略</span>
                    <span className="font-mono text-amber-600 dark:text-amber-400">PII / 金融数据自动遮蔽</span>
                  </div>
                </div>

                <div className="space-y-2">
                  <div className="flex justify-between text-xs">
                    <span className="text-muted-foreground">执行进度</span>
                    <span className="font-medium">{progress}%</span>
                  </div>
                  <div className="h-2 rounded-full bg-muted overflow-hidden">
                    <div
                      className="h-full rounded-full bg-amber-500 transition-all duration-500"
                      style={{ width: `${progress}%` }}
                    />
                  </div>
                  <div className="text-[11px] text-muted-foreground">{progressLabel}</div>
                </div>
              </>
            )}
          </div>
        </DialogContent>
      </Dialog>
    </>
  );
}

// =============================================================================
// Event Block — agent reacting to subscribed events
// =============================================================================

const eventSourceConfig: Record<string, { icon: typeof Bell; label: string }> = {
  task: { icon: CheckCircle2, label: "任务事件" },
  news: { icon: FileText, label: "新闻资讯" },
  social: { icon: Bell, label: "社交媒体" },
  market: { icon: Zap, label: "市场行情" },
  system: { icon: Terminal, label: "系统事件" },
  compliance: { icon: ShieldCheck, label: "合规监控" },
};

function EventBlockView({ block }: { block: EventBlock }) {
  const [open, setOpen] = useState(false);
  const cfg = eventSourceConfig[block.source] || { icon: Bell, label: "事件" };
  const Icon = cfg.icon;

  return (
    <div className="my-2 rounded-lg border border-primary/15 bg-primary/[0.02] p-3">
      <div className="flex items-center gap-2 mb-1.5">
        <div className="flex items-center justify-center size-5 rounded bg-primary/10">
          <Icon className="size-3 text-primary" />
        </div>
        <span className="text-[10px] font-medium text-primary/70 uppercase tracking-wide">{cfg.label}</span>
        <span className="text-[10px] text-muted-foreground">·</span>
        <span className="text-[10px] text-muted-foreground">{block.topic}</span>
        {block.eventTime && (
          <time className="text-[10px] text-muted-foreground ml-auto">
            {dayjs(block.eventTime).format("HH:mm")}
          </time>
        )}
      </div>
      <p className="text-sm leading-relaxed">{block.summary}</p>
      {block.detail && (
        <>
          <button
            type="button"
            className="flex items-center gap-1 mt-1.5 text-[11px] text-muted-foreground hover:text-foreground transition-colors"
            onClick={() => setOpen(!open)}
          >
            {open ? <ChevronDown className="size-3" /> : <ChevronRight className="size-3" />}
            <span>事件详情</span>
          </button>
          {open && (
            <pre className="mt-1.5 rounded bg-muted/50 p-2 text-[11px] font-mono overflow-x-auto max-h-40 whitespace-pre-wrap text-muted-foreground">
              {block.detail}
            </pre>
          )}
        </>
      )}
    </div>
  );
}

// =============================================================================
// Message source badge — shows where the user message originated
// =============================================================================

const SOURCE_CONFIG: Record<MessageSource, { label: string; color: string; bg: string; icon: string }> = {
  app: { label: "SafeClaw", color: "text-primary", bg: "bg-primary/10", icon: "🛡️" },
  dingtalk: { label: "钉钉", color: "text-blue-600 dark:text-blue-400", bg: "bg-blue-500/10", icon: "💬" },
  feishu: { label: "飞书", color: "text-indigo-600 dark:text-indigo-400", bg: "bg-indigo-500/10", icon: "📮" },
  wecom: { label: "企业微信", color: "text-teal-600 dark:text-teal-400", bg: "bg-teal-500/10", icon: "💼" },
};

function SourceBadge({ source }: { source: MessageSource }) {
  const cfg = SOURCE_CONFIG[source];
  return (
    <span className={cn("inline-flex items-center gap-1 rounded-full px-1.5 py-0.5 text-[10px] font-medium", cfg.bg, cfg.color)}>
      <span className="text-[9px] leading-none">{cfg.icon}</span>
      {cfg.label}
    </span>
  );
}

// =============================================================================
// Message Item — Claude Code style linear layout
// =============================================================================

function MessageItem({ msg, sessionId, onHilConfirm }: { msg: RichMessage; sessionId: string; onHilConfirm?: () => void }) {
  if (msg.role === "system") {
    return (
      <div className="flex justify-center px-4 py-2">
        <div className="rounded-lg bg-muted px-3 py-1.5 text-xs text-muted-foreground">
          {msg.blocks[0]?.type === "text" ? (msg.blocks[0] as TextBlock).content : ""}
        </div>
      </div>
    );
  }

  const isUser = msg.role === "user";
  const persona = personaModel.getSessionPersona(sessionId);
  const avatarConfig = genConfig(persona.avatar);
  const { user } = useSnapshot(globalModel.state);

  return (
    <div className={cn("px-4 py-3", isUser && "bg-foreground/[0.06]")}>
      {/* Header: avatar + name + time */}
      <div className="flex items-center gap-2 mb-1.5">
        {isUser ? (
          <img
            src={user.avatar}
            alt={user.nickname}
            className="size-6 shrink-0 rounded-full object-cover"
          />
        ) : (
          <NiceAvatar className="size-6 shrink-0" {...avatarConfig} />
        )}
        <span className="text-xs font-semibold">{isUser ? user.nickname : persona.name}</span>
        {isUser && msg.source && <SourceBadge source={msg.source} />}
        <time className="text-[10px] text-muted-foreground">{dayjs(msg.timestamp).format("HH:mm:ss")}</time>
      </div>

      {/* Content blocks */}
      <div className="ml-8">
        {msg.blocks.map((block, i) => {
          switch (block.type) {
            case "thinking":
              return <ThinkingBlockView key={i} block={block} />;
            case "tool_call":
              return <ToolCallBlockView key={i} block={block} />;
            case "sub_agent":
              return <SubAgentBlockView key={i} block={block} />;
            case "hil":
              return <HilBlockView key={i} block={block} onConfirm={onHilConfirm} />;
            case "payment":
              return <PaymentBlockView key={i} block={block} />;
            case "tee_data_query":
              return <TeeDataQueryBlockView key={i} block={block} onConfirm={onHilConfirm} />;
            case "event":
              return <EventBlockView key={i} block={block} />;
            case "text":
              return (
                <div key={i} className="text-sm leading-relaxed">
                  <MemoizedMarkdown id={`${msg.id}-${i}`} content={block.content} />
                </div>
              );
            default:
              return null;
          }
        })}
      </div>
    </div>
  );
}

// =============================================================================
// Streaming Display — matches linear style
// =============================================================================

function StreamingDisplay({ sessionId }: { sessionId: string }) {
  const { streaming } = useSnapshot(agentModel.state);
  const text = streaming[sessionId];
  const persona = personaModel.getSessionPersona(sessionId);
  const avatarConfig = genConfig(persona.avatar);

  if (!text && text !== "") return null;

  return (
    <div className="px-4 py-3">
      <div className="flex items-center gap-2 mb-1.5">
        <NiceAvatar className="size-6 shrink-0" {...avatarConfig} />
        <span className="text-xs font-semibold">{persona.name}</span>
        <Loader2 className="size-3 text-primary animate-spin" />
      </div>
      <div className="ml-8 text-sm leading-relaxed">
        {text ? (
          <MemoizedMarkdown id={`streaming-${sessionId}`} content={text} />
        ) : (
          <span className="text-xs text-muted-foreground">思考中...</span>
        )}
        <span className="inline-block w-0.5 h-4 bg-primary animate-pulse ml-0.5 align-middle" />
      </div>
    </div>
  );
}

// =============================================================================
// Mock context usage per session
// =============================================================================

const MOCK_CONTEXT_USAGE: Record<string, { used: number; total: number; mode: string }> = {
  "mock-fullstack-1": { used: 48200, total: 200000, mode: "agent" },
  "mock-fullstack-2": { used: 31500, total: 200000, mode: "agent" },
  "mock-quant-1": { used: 127800, total: 200000, mode: "agent" },
  "mock-risk-1": { used: 62300, total: 200000, mode: "agent" },
  "mock-devops-1": { used: 89100, total: 200000, mode: "chat" },
  "mock-data-eng-1": { used: 15600, total: 200000, mode: "agent" },
  "mock-product-1": { used: 145000, total: 200000, mode: "chat" },
  "mock-finance-1": { used: 53700, total: 200000, mode: "agent" },
};

function formatTokens(n: number): string {
  if (n >= 1000) return `${(n / 1000).toFixed(1)}K`;
  return `${n}`;
}

// =============================================================================
// Status Bar — model, mode, context usage
// =============================================================================

function SessionStatusBar({ sessionId }: { sessionId: string }) {
  const settingsSnap = useSnapshot(settingsModel.state);
  const { sessionStatus } = useSnapshot(agentModel.state);
  const ctx = MOCK_CONTEXT_USAGE[sessionId] || { used: 0, total: 200000, mode: "agent" };
  const pct = Math.round((ctx.used / ctx.total) * 100);
  const status = sessionStatus[sessionId] || "idle";

  const modelShort = useMemo(() => {
    const m = settingsSnap.model;
    if (m.includes("opus")) return "Opus";
    if (m.includes("sonnet")) return "Sonnet";
    if (m.includes("haiku")) return "Haiku";
    if (m.includes("gpt-4")) return "GPT-4o";
    if (m.includes("gpt-3")) return "GPT-3.5";
    return m.split("/").pop()?.split("-").slice(0, 2).join("-") || m;
  }, [settingsSnap.model]);

  return (
    <div className="flex items-center gap-3 px-3 py-1.5 border-t bg-muted/30 text-[11px] text-muted-foreground shrink-0 select-none">
      {/* Model */}
      <div className="flex items-center gap-1.5" title={settingsSnap.model}>
        <Cpu className="size-3" />
        <span className="font-medium text-foreground/80">{modelShort}</span>
        <span className="text-muted-foreground/60">({settingsSnap.provider})</span>
      </div>

      <div className="w-px h-3 bg-border" />

      {/* Mode */}
      <div className="flex items-center gap-1">
        {ctx.mode === "agent" ? (
          <>
            <Sparkles className="size-3 text-primary" />
            <span>Agent</span>
          </>
        ) : (
          <>
            <MessageSquare className="size-3" />
            <span>Chat</span>
          </>
        )}
      </div>

      <div className="w-px h-3 bg-border" />

      {/* Session status */}
      <div className="flex items-center gap-1">
        {status === "running" ? (
          <Loader2 className="size-3 text-primary animate-spin" />
        ) : status === "compacting" ? (
          <Loader2 className="size-3 text-orange-500 animate-spin" />
        ) : (
          <Circle className="size-2.5 fill-green-500 text-green-500" />
        )}
        <span>
          {status === "running" ? "运行中" : status === "compacting" ? "压缩中" : "就绪"}
        </span>
      </div>

      <div className="w-px h-3 bg-border" />

      {/* Context usage */}
      <div className="flex items-center gap-1.5" title={`${ctx.used.toLocaleString()} / ${ctx.total.toLocaleString()} tokens`}>
        <Gauge className="size-3" />
        <span>上下文</span>
        <div className="w-16 h-1.5 bg-muted rounded-full overflow-hidden">
          <div
            className={cn(
              "h-full rounded-full transition-all",
              pct >= 80 ? "bg-red-500" : pct >= 50 ? "bg-yellow-500" : "bg-primary",
            )}
            style={{ width: `${pct}%` }}
          />
        </div>
        <span className={cn(pct >= 80 && "text-red-500 font-medium")}>
          {formatTokens(ctx.used)}/{formatTokens(ctx.total)}
        </span>
      </div>

      {/* Right side: shortcuts hint */}
      <div className="ml-auto flex items-center gap-1 text-muted-foreground/60">
        <CornerDownLeft className="size-3" />
        <span>发送</span>
        <span className="mx-0.5">/</span>
        <span>Shift+Enter 换行</span>
      </div>
    </div>
  );
}

// =============================================================================
// Message Input — fills remaining bottom space
// =============================================================================

function AgentInput({ sessionId, disabled, onSend }: { sessionId: string; disabled: boolean; onSend?: (text: string) => void }) {
  const editorRef = useRef<TiptapEditorRef>(null);
  const [isEmpty, setIsEmpty] = useState(true);

  const handleSubmit = useCallback(() => {
    if (!editorRef.current || disabled) return;
    const text = editorRef.current.getText().trim();
    if (!text) return;
    onSend?.(text);
    editorRef.current.clear();
    setIsEmpty(true);
  }, [disabled, onSend]);

  const handleEditorChange = useCallback((text: string) => {
    setIsEmpty(!text.trim());
  }, []);

  return (
    <div className="flex flex-col h-full">
      {/* Action toolbar */}
      <div className="flex items-center gap-0.5 px-2 py-1.5 border-b shrink-0">
        <button type="button" className="flex items-center gap-1.5 rounded-md px-2 py-1 text-xs text-muted-foreground hover:text-foreground hover:bg-foreground/[0.04] transition-colors" title="上传文件">
          <Paperclip className="size-3.5" />
          <span>文件</span>
        </button>
        <button type="button" className="flex items-center gap-1.5 rounded-md px-2 py-1 text-xs text-muted-foreground hover:text-foreground hover:bg-foreground/[0.04] transition-colors" title="上传图片">
          <Image className="size-3.5" />
          <span>图片</span>
        </button>
        {/* Right side: send button */}
        <div className="ml-auto flex items-center gap-1.5">
          <button
            type="button"
            className={cn(
              "flex items-center gap-1 rounded-md bg-primary px-2.5 py-1 text-xs font-medium text-primary-foreground",
              "hover:bg-primary/90",
            )}
            aria-label="Send message"
          >
            <Compass className="size-3" />
            压缩
          </button>
        </div>
      </div>

      {/* TipTap rich text editor — fills remaining space */}
      <div className="flex-1 min-h-0">
        <TiptapEditor
          ref={editorRef}
          placeholder={disabled ? "连接中..." : "输入消息，/ 触发技能，@ 引用上下文..."}
          disabled={disabled}
          onSubmit={(text) => {
            if (disabled) return;
            onSend?.(text);
            setIsEmpty(true);
          }}
          onChange={handleEditorChange}
        />
      </div>

      {/* Status bar — at very bottom */}
      <SessionStatusBar sessionId={sessionId} />
    </div>
  );
}

// =============================================================================
// Mock task data per session
// =============================================================================

interface SubTask {
  id: string;
  label: string;
  status: "done" | "running" | "pending";
}

const MOCK_TASKS: Record<string, SubTask[]> = {
  // mock-fullstack-1: starts empty, tasks are generated dynamically
  "mock-fullstack-2": [
    { id: "t1", label: "定义订单数据模型", status: "done" },
    { id: "t2", label: "实现 CRUD 接口", status: "done" },
    { id: "t3", label: "编写 OpenAPI 文档", status: "done" },
  ],
  "mock-quant-1": [
    { id: "t1", label: "获取 A 股行情数据", status: "done" },
    { id: "t2", label: "计算动量因子", status: "done" },
    { id: "t3", label: "IC 分析与分层回测", status: "done" },
    { id: "t4", label: "生成回测报告", status: "done" },
    { id: "t5", label: "编写因子检验技能模块", status: "done" },
    { id: "t6", label: "注册技能到系统", status: "done" },
    { id: "t7", label: "用新技能跑动量因子", status: "running" },
  ],
  "mock-risk-1": [
    { id: "t1", label: "评估当前模型瓶颈", status: "done" },
    { id: "t2", label: "特征工程优化", status: "done" },
    { id: "t3", label: "编写模型监控技能", status: "done" },
    { id: "t4", label: "注册技能到系统", status: "done" },
    { id: "t5", label: "模型重训练", status: "running" },
    { id: "t6", label: "OOT 验证", status: "pending" },
  ],
  "mock-devops-1": [
    { id: "t1", label: "预检环境与 API 兼容性", status: "done" },
    { id: "t2", label: "备份 etcd 快照", status: "done" },
    { id: "t3", label: "升级控制面 1.27→1.28", status: "done" },
    { id: "t4", label: "升级控制面 1.28→1.29", status: "done" },
    { id: "t5", label: "滚动升级工作节点", status: "done" },
    { id: "t6", label: "→ 全栈工程师：编写升级预检技能", status: "done" },
    { id: "t7", label: "注册预检技能到系统", status: "done" },
  ],
  "mock-data-eng-1": [
    { id: "t1", label: "设计 Kafka Topic 结构", status: "done" },
    { id: "t2", label: "编写 Flink 清洗 Job", status: "done" },
    { id: "t3", label: "创建 ClickHouse 表", status: "done" },
    { id: "t4", label: "→ 全栈工程师：开发管道质量监控技能", status: "done" },
    { id: "t5", label: "接入监控并验证", status: "running" },
  ],
  "mock-product-1": [
    { id: "t1", label: "竞品分析", status: "done" },
    { id: "t2", label: "用户画像与需求", status: "done" },
    { id: "t3", label: "PRD 初稿", status: "done" },
    { id: "t4", label: "→ 全栈工程师：开发竞品情报技能", status: "done" },
    { id: "t5", label: "技术评审", status: "running" },
  ],
  "mock-finance-1": [
    { id: "t1", label: "核对供应商发票", status: "done" },
    { id: "t2", label: "验证合同条款", status: "done" },
    { id: "t3", label: "生成付款审批单", status: "done" },
    { id: "t4", label: "等待用户授权支付", status: "running" },
    { id: "t5", label: "记账与凭证归档", status: "pending" },
  ],
};

function TaskStatusIcon({ status }: { status: SubTask["status"] }) {
  switch (status) {
    case "done":
      return <CheckCircle2 className="size-3.5 text-primary shrink-0" />;
    case "running":
      return <Loader2 className="size-3.5 text-primary animate-spin shrink-0" />;
    case "pending":
      return <Circle className="size-3.5 text-muted-foreground/40 shrink-0" />;
  }
}

// =============================================================================
// Chat Header — session dropdown + task tracker
// =============================================================================

function ChatHeader({ sessionId }: { sessionId: string }) {
  const { sdkSessions, sessionNames } = useSnapshot(agentModel.state);
  const personaSnap = useSnapshot(personaModel.state);
  const persona = personaModel.getSessionPersona(sessionId);
  const personaId = personaSnap.sessionPersonas[sessionId];

  const siblingsSessions = useMemo(() => {
    return [...sdkSessions]
      .filter((s) => !s.archived && personaSnap.sessionPersonas[s.session_id] === personaId)
      .sort((a, b) => b.created_at - a.created_at);
  }, [sdkSessions, personaSnap.sessionPersonas, personaId]);

  const tasks = MOCK_TASKS[sessionId] || [];
  const doneCount = tasks.filter((t) => t.status === "done").length;
  const runningCount = tasks.filter((t) => t.status === "running").length;
  const totalCount = tasks.length;

  return (
    <div className="flex items-center justify-between px-3 py-2 border-b bg-background gap-3">
      {/* Left: avatar + session dropdown */}
      <div className="flex items-center gap-2 min-w-0">
        <NiceAvatar className="size-7 shrink-0" {...genConfig(persona.avatar)} />
        {siblingsSessions.length <= 1 ? (
          <span className="text-sm font-medium truncate">{persona.name}</span>
        ) : (
          <Select
            value={sessionId}
            onValueChange={(val) => agentModel.setCurrentSession(val)}
          >
            <SelectTrigger className="h-7 text-xs font-medium border-none shadow-none px-2 gap-1 min-w-[120px] max-w-[200px]">
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {siblingsSessions.map((s) => {
                const name = sessionNames[s.session_id] || s.name || `会话 ${s.session_id.slice(0, 6)}`;
                return (
                  <SelectItem key={s.session_id} value={s.session_id} className="text-xs">
                    {name}
                  </SelectItem>
                );
              })}
            </SelectContent>
          </Select>
        )}
      </div>

      {/* Right: task tracker */}
      {totalCount > 0 && (
        <div className="relative group shrink-0">
          <div className="flex items-center gap-1.5 cursor-default px-2 py-1 rounded-md hover:bg-foreground/[0.04] transition-colors">
            {runningCount > 0 && <Loader2 className="size-3 text-primary animate-spin" />}
            {runningCount === 0 && doneCount === totalCount && <CheckCircle2 className="size-3 text-primary" />}
            {runningCount === 0 && doneCount < totalCount && <Circle className="size-3 text-muted-foreground" />}
            <span className="text-xs text-muted-foreground">
              {runningCount > 0 ? `${runningCount} 执行中` : `${doneCount}/${totalCount}`}
            </span>
          </div>

          {/* Hover popover — task list */}
          <div className="absolute right-0 top-full mt-1 hidden group-hover:block z-50">
            <div className="bg-popover text-popover-foreground border rounded-lg shadow-lg p-3 w-64">
              <div className="text-xs font-medium mb-2">任务进度 ({doneCount}/{totalCount})</div>
              <div className="space-y-1.5">
                {tasks.map((task) => (
                  <div key={task.id} className="flex items-center gap-2">
                    <TaskStatusIcon status={task.status} />
                    <span className={cn(
                      "text-xs truncate",
                      task.status === "done" && "text-muted-foreground line-through",
                      task.status === "running" && "text-foreground font-medium",
                      task.status === "pending" && "text-muted-foreground",
                    )}>
                      {task.label}
                    </span>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

// =============================================================================
// Mock rich messages per session
// =============================================================================

const now = Date.now();

const MOCK_RICH_MESSAGES: Record<string, RichMessage[]> = {
  // mock-fullstack-1: starts empty — messages are generated dynamically by the simulation engine
  "mock-fullstack-2": [
    {
      id: "m2-1", role: "user", timestamp: now - 3 * 3600_000, source: "feishu" as MessageSource,
      blocks: [{ type: "text", content: "设计一套 RESTful API 用于交易订单管理" }],
    },
    {
      id: "m2-2", role: "assistant", timestamp: now - 2.8 * 3600_000,
      blocks: [
        { type: "thinking", content: "交易订单 API 设计要点：\n1. 资源命名：/api/v1/orders\n2. 状态机：pending → confirmed → executing → filled/cancelled\n3. 金额用 string 避免浮点精度\n4. 分页用 cursor-based 适配实时数据\n5. 乐观锁防并发修改", durationMs: 1800 },
        { type: "sub_agent", agentName: "数据工程师", task: "评审数据库 schema 设计", result: "建议 orders 表使用 BIGINT 主键 + UUID session_id，金额字段用 DECIMAL(18,4)，添加 version 字段支持乐观锁。索引：(user_id, created_at), (symbol, status)。", durationMs: 4500 },
        { type: "text", content: "交易订单 API 设计方案：\n\n| 方法 | 路径 | 说明 |\n|------|------|------|\n| POST | `/api/v1/orders` | 创建订单 |\n| GET | `/api/v1/orders` | 查询订单列表 |\n| GET | `/api/v1/orders/:id` | 获取订单详情 |\n| PATCH | `/api/v1/orders/:id` | 更新订单状态 |\n| DELETE | `/api/v1/orders/:id` | 撤销订单 |\n\n关键设计：乐观锁 `version` 字段、cursor-based 分页、金额使用 `string` 类型。" },
      ],
    },
  ],
  "mock-quant-1": [
    {
      id: "m3-1", role: "user", timestamp: now - 14 * 60_000, source: "dingtalk" as MessageSource,
      blocks: [{ type: "text", content: "帮我分析一下动量因子在 A 股市场的有效性，最近三年的数据" }],
    },
    {
      id: "m3-2", role: "assistant", timestamp: now - 12 * 60_000,
      blocks: [
        { type: "thinking", content: "动量因子分析步骤：\n1. 定义因子：过去20日收益率（剔除最近5日，避免短期反转）\n2. IC分析：截面相关性\n3. 分层回测：5组，看多空收益\n4. 需要关注2023年风格切换对动量的影响", durationMs: 3200 },
        { type: "tool_call", tool: "Bash", input: "python scripts/fetch_a_share_data.py --start 2022-01-01 --end 2024-12-31", output: "Fetched 3,847,291 daily records for 5,102 stocks", durationMs: 15000 },
        { type: "tool_call", tool: "Bash", input: "python scripts/momentum_analysis.py --window 20 --skip 5", output: "IC mean: 0.032, ICIR: 0.41, IC win rate: 58.3%\nTop group excess: +8.7% ann.\nLong-short: 14.9% ann., max DD: -12.3%", durationMs: 28000 },
        { type: "text", content: "基于 2022-2024 年 A 股全市场数据的动量因子分析：\n\n**IC 分析：** IC 均值 0.032，IC_IR 0.41，胜率 58.3%\n\n**分层回测（5 组）：** Top 组年化超额 +8.7%，多空年化 14.9%\n\n**结论：** 动量因子有效但 2023Q2-Q3 回撤明显。建议正交化处理 + 波动率调整 + 行业中性化。" },
      ],
    },
    {
      id: "m3-3", role: "user", timestamp: now - 8 * 60_000, source: "app" as MessageSource,
      blocks: [{ type: "text", content: "我需要一个自动化的因子检验工具，能批量跑多个因子的 IC、分层回测和归因分析，其他量化研究员也能直接用" }],
    },
    {
      id: "m3-4", role: "assistant", timestamp: now - 6 * 60_000,
      blocks: [
        { type: "thinking", content: "用户需要一个可复用的因子检验技能（Skill），不仅自己用，还要让其他量化研究员智能体也能调用。\n\n这属于「动态技能生成」场景：\n1. 我需要委托全栈工程师智能体编写一个标准化的 Skill 模块\n2. Skill 需要定义清晰的输入/输出接口\n3. 注册到技能系统后，所有智能体都能发现和调用\n\n技能接口设计：\n- 输入：因子矩阵 DataFrame、收益率矩阵、参数配置\n- 输出：IC 报告、分层净值、归因分解\n- 格式：标准 JSON + 可视化图表", durationMs: 3800 },
        { type: "hil", action: "派发新技能开发任务", targetAgent: "全栈工程师", description: "编写「因子检验」技能模块（IC 分析、分层回测、归因分解），注册到技能系统供所有量化研究员使用", confirmed: undefined },
        { type: "sub_agent", agentName: "全栈工程师", task: "编写「因子检验」技能模块，注册到技能系统供所有智能体使用", result: "已创建技能模块 skills/factor-analysis/\n\n技能清单（skill.yaml）：\n```yaml\nname: factor_analysis\nversion: 1.0.0\ndescription: 批量因子检验工具 — IC分析、分层回测、归因分解\nauthor: quant-researcher\ninterface:\n  input:\n    factor_matrix: DataFrame  # 因子值矩阵 (date × stock)\n    returns_matrix: DataFrame # 收益率矩阵\n    config:\n      groups: int             # 分层组数，默认5\n      holding_period: int     # 持仓周期（天）\n      benchmark: string       # 基准指数代码\n  output:\n    ic_report: ICReport\n    layer_backtest: LayerBacktest\n    attribution: Attribution\n```\n\n核心实现 factor_analysis.py（238行）：\n- `run_ic_analysis()` — 截面IC、Rank IC、IC衰减\n- `run_layer_backtest()` — 分层净值、多空收益、换手率\n- `run_attribution()` — Barra风格归因、行业归因\n- `generate_report()` — 输出 HTML 可视化报告\n\n已注册到技能系统，所有智能体可通过 `use_skill('factor_analysis', ...)` 调用。", durationMs: 18500 },
        { type: "tool_call", tool: "Write", input: "skills/factor-analysis/skill.yaml", output: "File written successfully", durationMs: 200 },
        { type: "tool_call", tool: "Write", input: "skills/factor-analysis/factor_analysis.py", output: "File written successfully (238 lines)", durationMs: 350 },
        { type: "tool_call", tool: "Write", input: "skills/factor-analysis/templates/report.html", output: "File written successfully", durationMs: 150 },
        { type: "tool_call", tool: "Bash", input: "safeclaw skill register skills/factor-analysis/", output: "✓ Skill 'factor_analysis' v1.0.0 registered successfully\n✓ Available to all agents", durationMs: 1200 },
        { type: "text", content: "已为你创建并注册了「因子检验」技能，现在所有智能体都能使用：\n\n```python\n# 任何智能体都可以这样调用\nresult = use_skill('factor_analysis', {\n  'factor_matrix': momentum_factor,\n  'returns_matrix': forward_returns,\n  'config': {\n    'groups': 5,\n    'holding_period': 20,\n    'benchmark': '000300.SH'\n  }\n})\n\nprint(result.ic_report.summary())\nresult.layer_backtest.plot()\nresult.attribution.to_html('report.html')\n```\n\n技能包含三个核心模块：\n- **IC 分析** — 截面 IC、Rank IC、IC 衰减曲线\n- **分层回测** — 分组净值、多空收益、换手率统计\n- **归因分解** — Barra 风格归因 + 行业归因\n\n其他量化研究员可以直接调用，不需要重复编写检验代码了。需要我用这个技能跑一下你刚才的动量因子吗？" },
      ],
    },
    {
      id: "m3-5", role: "assistant", timestamp: now - 2 * 60_000,
      blocks: [
        { type: "event", source: "market", topic: "polymarket.odds_shift", summary: "Polymarket 预测市场「美联储 2025Q1 降息」合约价格从 $0.42 急跌至 $0.28（-33%），大量卖单涌入", eventTime: now - 3 * 60_000, detail: "Market: \"Fed Rate Cut Q1 2025\"\nContract: POLY-FED-Q1-2025\nPrice: $0.42 → $0.28 (-33.3%)\nVolume 24h: $2.8M (+420%)\nTop traders: 78% selling\nTrigger: CPI data release (higher than expected)" },
        { type: "thinking", content: "Polymarket 降息预期大幅下降，说明市场对通胀预期重新定价。这对我们的量化策略有直接影响：\n\n1. 利率敏感因子（久期、债券相关）需要重新评估\n2. 动量因子在利率转向期通常表现不稳定\n3. 价值因子可能受益于高利率环境延续\n\n建议：\n- 降低动量因子权重\n- 增加价值因子和低波因子配置\n- 对冲利率风险敞口", durationMs: 2800 },
        { type: "tool_call", tool: "KnowledgeBase", input: "查询当前组合利率敏感度和因子暴露", output: "Portfolio rate sensitivity: +0.32 (long duration)\nMomentum exposure: 0.45\nValue exposure: 0.12\nVolatility exposure: -0.08", durationMs: 350 },
        { type: "text", content: "⚡ 检测到 Polymarket 降息预期大幅下调，当前组合利率敏感度偏高（+0.32），动量暴露 0.45。建议调整因子配置以应对利率环境变化：" },
        {
          type: "hil", action: "调整因子组合权重", targetAgent: "量化研究员", description: "根据 Polymarket 降息预期下调，重新配置因子权重以降低利率风险", mode: "single", options: [
            { label: "保守方案：动量 0.25 / 价值 0.35 / 低波 0.25 / 质量 0.15", value: "conservative" },
            { label: "均衡方案：动量 0.30 / 价值 0.30 / 低波 0.20 / 质量 0.20", value: "balanced" },
            { label: "维持现状，仅对冲利率敞口", value: "hedge_only" },
          ], allowInput: true, inputPlaceholder: "自定义权重或补充说明..."
        },
      ],
    },
  ],
  "mock-risk-1": [
    {
      id: "m4-1", role: "user", timestamp: now - 40 * 60_000, source: "wecom" as MessageSource,
      blocks: [{ type: "text", content: "我们的信用评分模型 AUC 只有 0.72，怎么提升？" }],
    },
    {
      id: "m4-2", role: "assistant", timestamp: now - 35 * 60_000,
      blocks: [
        { type: "thinking", content: "AUC 0.72 说明模型区分度一般。可能原因：特征不够丰富、模型太简单、样本不均衡未处理。", durationMs: 2100 },
        { type: "sub_agent", agentName: "数据科学家", task: "分析当前特征重要性和缺失率", result: "当前 23 个特征中 IV > 0.1 的仅 8 个。建议增加时序聚合特征和交叉特征，预计可新增 15-20 个有效特征。", durationMs: 5200 },
        { type: "tool_call", tool: "KnowledgeBase", input: "查询信用评分模型 v3 特征配置", output: "features:\n  - income_level\n  - debt_ratio\n  - credit_history_months\n  ...(23 features total)", durationMs: 80 },
        { type: "text", content: "优化方向：\n\n**特征工程：** 时序特征 + 交叉特征 + 外部数据\n**模型优化：** XGBoost + LR 两阶段，SMOTE 处理不均衡\n**评估改进：** 分群体 AUC + KS + PSI + OOT 验证\n\n预期优化后 AUC 可达 0.78-0.82。" },
      ],
    },
    {
      id: "m4-3", role: "user", timestamp: now - 30 * 60_000, source: "feishu" as MessageSource,
      blocks: [{ type: "text", content: "帮我做一个模型监控的自动化工具，能定期检测 PSI 漂移和 AUC 衰减，其他风控分析师也能用" }],
    },
    {
      id: "m4-4", role: "assistant", timestamp: now - 25 * 60_000,
      blocks: [
        { type: "thinking", content: "用户需要一个模型监控技能，核心功能：\n1. PSI（Population Stability Index）漂移检测\n2. AUC 时序衰减监控\n3. 特征分布漂移告警\n4. 自动生成监控报告\n\n这个技能对所有风控分析师都有价值，需要委托工程师智能体编写并注册。", durationMs: 2600 },
        {
          type: "hil", action: "派发新技能开发任务", targetAgent: "全栈工程师", description: "编写「模型监控」技能模块 — PSI 漂移检测、AUC 衰减监控、特征分布告警，注册供所有风控分析师使用", mode: "single", options: [
            { label: "仅 PSI 漂移检测 + AUC 衰减监控", value: "basic" },
            { label: "完整版：PSI + AUC + 特征漂移 + 告警推送", value: "full" },
            { label: "完整版 + 自动重训练触发", value: "full_retrain" },
          ], allowInput: true, inputPlaceholder: "补充需求，如告警渠道、检测频率等..."
        },
        { type: "sub_agent", agentName: "全栈工程师", task: "编写「模型监控」技能模块，支持 PSI 漂移检测和 AUC 衰减告警", result: "已创建技能模块 skills/model-monitor/\n\n```yaml\nname: model_monitor\nversion: 1.0.0\ndescription: 信用评分模型自动化监控 — PSI、AUC衰减、特征漂移\nschedule: daily  # 支持定时执行\ninterface:\n  input:\n    model_id: string\n    baseline_data: DataFrame\n    current_data: DataFrame\n    thresholds:\n      psi_warn: 0.1\n      psi_alert: 0.25\n      auc_drop: 0.03\n  output:\n    psi_report: PSIReport\n    auc_trend: AUCTrend\n    drift_alerts: list[DriftAlert]\n```\n\n实现了 4 个核心检测器：\n- `PSIDetector` — 分箱计算 PSI，支持连续/离散变量\n- `AUCMonitor` — 滑动窗口 AUC 趋势，自动检测拐点\n- `FeatureDriftDetector` — KS 检验 + 分布可视化\n- `AlertDispatcher` — 飞书/邮件告警推送\n\n已注册，所有风控智能体可调用。", durationMs: 22000 },
        { type: "tool_call", tool: "Bash", input: "safeclaw skill register skills/model-monitor/", output: "✓ Skill 'model_monitor' v1.0.0 registered successfully\n✓ Available to all agents\n✓ Daily schedule configured", durationMs: 1500 },
        { type: "text", content: "「模型监控」技能已创建并注册，支持定时自动执行：\n\n```python\n# 所有风控分析师可直接调用\nresult = use_skill('model_monitor', {\n  'model_id': 'credit_score_v3',\n  'baseline_data': train_data,\n  'current_data': latest_month_data,\n  'thresholds': {'psi_warn': 0.1, 'psi_alert': 0.25}\n})\n\nif result.drift_alerts:\n  print(f'⚠️ 发现 {len(result.drift_alerts)} 个漂移告警')\nresult.psi_report.plot()\n```\n\n核心能力：PSI 漂移检测、AUC 衰减趋势、特征分布漂移告警、飞书/邮件自动推送。已配置每日自动执行。" },
      ],
    },
    {
      id: "m4-5", role: "assistant", timestamp: now - 10 * 60_000,
      blocks: [
        { type: "event", source: "compliance", topic: "regulation.pboc.update", summary: "中国人民银行发布《个人信息保护与信用评估管理办法（征求意见稿）》，要求信用评分模型必须提供可解释性报告，禁止使用社交关系数据作为评分特征", eventTime: now - 12 * 60_000, detail: "Source: 中国人民银行官网\nDocument: 银发〔2025〕18号\nEffective: 2025-07-01\nKey changes:\n  1. 信用评分模型必须提供逐客户可解释性报告\n  2. 禁止使用社交关系、通讯录等数据\n  3. 模型变更需提前30天向监管报备\n  4. 年度模型审计报告强制提交" },
        { type: "thinking", content: "这个监管变更对我们影响很大：\n\n1. 可解释性要求 — 当前 XGBoost 模型需要加 SHAP 解释层\n2. 特征合规 — 需要排查是否使用了社交关系类特征\n3. 报备流程 — 模型变更需要提前30天报备，影响迭代节奏\n4. 审计报告 — 需要建立年度模型审计机制\n\n这些变更需要用户决策优先级和资源分配。", durationMs: 3200 },
        { type: "tool_call", tool: "KnowledgeBase", input: "查询当前信用评分模型使用的特征列表及数据来源", output: "23 features total. 2 features flagged:\n  - social_network_score (来源: 第三方社交数据)\n  - contact_frequency (来源: 通讯录授权)\nBoth must be removed per new regulation.", durationMs: 280 },
        { type: "text", content: "⚠️ 检测到央行新规，对我们的信用评分模型有直接影响：\n\n1. **特征合规风险** — 发现 2 个特征（`social_network_score`、`contact_frequency`）使用了社交关系数据，新规明确禁止\n2. **可解释性缺口** — 当前模型缺少逐客户 SHAP 解释报告\n3. **合规截止日** — 2025-07-01 生效，剩余约 6 个月\n\n建议立即启动合规整改：" },
        {
          type: "hil", action: "启动信用模型合规整改", targetAgent: "风控分析师", description: "根据央行新规，需要移除违规特征、增加模型可解释性、建立报备和审计流程", mode: "multi", options: [
            { label: "移除社交关系类特征并重训练模型", value: "remove_features" },
            { label: "集成 SHAP 可解释性报告", value: "add_shap" },
            { label: "建立模型变更报备流程", value: "filing_process" },
            { label: "搭建年度模型审计框架", value: "audit_framework" },
            { label: "委托全栈工程师开发合规报告自动生成技能", value: "compliance_skill" },
          ]
        },
      ],
    },
  ],
  "mock-devops-1": [
    {
      id: "m5-1", role: "user", timestamp: now - 2 * 86400_000, source: "dingtalk" as MessageSource,
      blocks: [{ type: "text", content: "生产环境 K8s 集群需要从 1.27 升级到 1.29，帮我制定升级方案" }],
    },
    {
      id: "m5-2", role: "assistant", timestamp: now - 1.9 * 86400_000,
      blocks: [
        { type: "thinking", content: "K8s 升级必须逐版本：1.27→1.28→1.29。关键风险：废弃 API、etcd 兼容性、CNI/CSI 插件。", durationMs: 2800 },
        { type: "tool_call", tool: "Bash", input: "kubectl get nodes -o wide", output: "NAME     STATUS   VERSION\nnode-1   Ready    v1.27.8\nnode-2   Ready    v1.27.8\nnode-3   Ready    v1.27.8", durationMs: 1200 },
        { type: "tool_call", tool: "Bash", input: "pluto detect-all-in-cluster --target-versions k8s=v1.29.0", output: "No deprecated APIs found! Ready to upgrade.", durationMs: 3500 },
        { type: "text", content: "升级方案：\n\n**Phase 1 预检：** 节点 Ready ✅，废弃 API 扫描通过 ✅，备份 etcd\n**Phase 2 控制面：** 逐版本升级 apiserver → controller-manager → scheduler\n**Phase 3 节点：** drain → 升级 kubelet → uncordon，每批 2 节点\n**回滚：** etcd 快照恢复\n\n预计 4 小时，建议周六凌晨执行。" },
      ],
    },
    {
      id: "m5-3", role: "user", timestamp: now - 1.8 * 86400_000, source: "app" as MessageSource,
      blocks: [{ type: "text", content: "升级完成了，以后每次升级都要手动检查这些，能不能自动化？" }],
    },
    {
      id: "m5-4", role: "assistant", timestamp: now - 1.7 * 86400_000,
      blocks: [
        { type: "thinking", content: "用户希望将 K8s 升级预检流程自动化。这涉及：\n1. 节点状态检查\n2. 废弃 API 扫描\n3. etcd 自动备份\n4. PDB 配置验证\n5. 插件兼容性矩阵检查\n\n这些步骤可以封装成一个标准化技能，以后每次升级前一键执行。我需要请全栈工程师帮忙把这个流程写成可复用的技能模块。", durationMs: 2200 },
        { type: "text", content: "好想法！每次升级前的预检流程完全可以自动化。我来请全栈工程师把这套流程封装成一个技能，以后所有 DevOps 工程师升级前一键执行就行。" },
        { type: "hil", action: "派发新技能开发任务", targetAgent: "全栈工程师", description: "编写「K8s 升级预检」技能 — 节点检查、废弃 API 扫描、etcd 备份、PDB 验证、插件兼容性检查，供所有 DevOps 工程师使用", confirmed: undefined },
        { type: "sub_agent", agentName: "全栈工程师", task: "编写「K8s 升级预检」技能，自动化节点检查、API 兼容性扫描、etcd 备份、PDB 验证", result: "已创建技能 skills/k8s-upgrade-preflight/\n\n```yaml\nname: k8s_upgrade_preflight\nversion: 1.0.0\ndescription: K8s 集群升级前自动化预检\ninterface:\n  input:\n    target_version: string\n    backup_path: string\n    skip_checks: list[string]  # 可选跳过某些检查\n  output:\n    report: PreflightReport\n    blockers: list[Blocker]  # 阻断性问题\n    warnings: list[Warning]  # 警告\n```\n\n实现了 5 个检查器：\n- `NodeReadinessChecker` — 验证所有节点 Ready + 资源充足\n- `DeprecatedAPIScanner` — 基于 pluto 扫描废弃 API\n- `EtcdBackupRunner` — 自动执行 etcd snapshot\n- `PDBValidator` — 检查 PDB 配置是否允许 drain\n- `AddonCompatChecker` — CNI/CSI/Ingress 版本兼容性矩阵\n\n预检通过返回绿色报告，有阻断问题会自动中止并给出修复建议。", durationMs: 16000 },
        { type: "tool_call", tool: "Bash", input: "safeclaw skill register skills/k8s-upgrade-preflight/", output: "✓ Skill 'k8s_upgrade_preflight' v1.0.0 registered\n✓ Available to all agents", durationMs: 1100 },
        { type: "text", content: "「K8s 升级预检」技能已注册，以后升级前直接调用：\n\n```bash\nuse_skill('k8s_upgrade_preflight', {\n  target_version: 'v1.30.0',\n  backup_path: '/backup/etcd/'\n})\n```\n\n会自动跑完 5 项检查，有阻断问题会中止并给修复建议。所有 DevOps 工程师都能用了。" },
      ],
    },
  ],
  "mock-data-eng-1": [
    {
      id: "m6-1", role: "user", timestamp: now - 28 * 60_000, source: "wecom" as MessageSource,
      blocks: [{ type: "text", content: "设计一个实时交易数据管道，从 Kafka 到 ClickHouse" }],
    },
    {
      id: "m6-2", role: "assistant", timestamp: now - 25 * 60_000,
      blocks: [
        { type: "thinking", content: "实时管道：Kafka → Flink → ClickHouse + Redis。按品种分区，lz4 压缩，5秒 watermark。", durationMs: 2500 },
        { type: "tool_call", tool: "Write", input: "infra/kafka/trades-topic.yaml", output: "File written successfully", durationMs: 150 },
        { type: "tool_call", tool: "Write", input: "sql/clickhouse/trades_table.sql", output: "File written successfully", durationMs: 120 },
        { type: "text", content: "架构：`Trading Engine → Kafka → Flink (清洗+聚合) → ClickHouse + Redis`\n\n```sql\nCREATE TABLE trades (\n  trade_id String,\n  symbol LowCardinality(String),\n  price Decimal64(4),\n  volume Decimal64(2),\n  trade_time DateTime64(3)\n) ENGINE = MergeTree()\nPARTITION BY toYYYYMMDD(trade_time)\nORDER BY (symbol, trade_time);\n```\n\n端到端延迟目标 < 500ms。" },
      ],
    },
    {
      id: "m6-3", role: "user", timestamp: now - 20 * 60_000, source: "dingtalk" as MessageSource,
      blocks: [{ type: "text", content: "管道上线后怎么监控数据质量？延迟、丢数、重复这些" }],
    },
    {
      id: "m6-4", role: "assistant", timestamp: now - 18 * 60_000,
      blocks: [
        { type: "thinking", content: "数据管道监控需要覆盖：\n1. 端到端延迟（Kafka → ClickHouse 的时间差）\n2. 数据完整性（上下游 count 对比）\n3. 重复检测（trade_id 去重率）\n4. Schema 漂移（字段类型变更）\n5. 流量异常（突增/骤降告警）\n\n目前没有现成的技能可以做这个，我需要请全栈工程师开发一个数据质量监控技能，这样其他数据工程师也能复用。", durationMs: 3100 },
        { type: "text", content: "目前系统里还没有针对实时管道的数据质量监控技能。我来请全栈工程师开发一个，这样所有数据管道都能接入统一监控。" },
        {
          type: "hil", action: "派发新技能开发任务", targetAgent: "全栈工程师", description: "开发「数据管道质量监控」技能 — 延迟探测、完整性校验、重复检测、Schema 漂移、流量异常告警，供所有数据工程师使用", mode: "multi", options: [
            { label: "端到端延迟探测", value: "latency" },
            { label: "数据完整性校验（上下游 count 对比）", value: "completeness" },
            { label: "重复数据检测", value: "dedup" },
            { label: "Schema 漂移检测", value: "schema" },
            { label: "流量异常告警（3σ 检测）", value: "throughput" },
          ]
        },
        { type: "sub_agent", agentName: "全栈工程师", task: "开发「数据管道质量监控」技能，支持延迟、完整性、重复检测、Schema 漂移和流量异常告警", result: "已创建技能 skills/pipeline-quality-monitor/\n\n```yaml\nname: pipeline_quality_monitor\nversion: 1.0.0\ndescription: 实时数据管道质量监控\nschedule: every_5min\ninterface:\n  input:\n    pipeline_id: string\n    source: { type: kafka, topic: string }\n    sink: { type: clickhouse, table: string }\n    thresholds:\n      max_latency_ms: 1000\n      max_duplicate_rate: 0.001\n      min_throughput_per_min: 100\n  output:\n    health_score: float  # 0-100\n    alerts: list[Alert]\n    metrics: PipelineMetrics\n```\n\n5 个检测模块：\n- `LatencyProbe` — 注入 watermark 测量端到端延迟\n- `CompletenessChecker` — Kafka offset vs ClickHouse count 对比\n- `DuplicateDetector` — 滑动窗口 trade_id 去重率\n- `SchemaValidator` — 检测字段类型/数量变更\n- `ThroughputMonitor` — 流量基线 + 3σ 异常检测\n\n告警渠道：飞书 webhook + Grafana annotation。", durationMs: 20000 },
        { type: "tool_call", tool: "Bash", input: "safeclaw skill register skills/pipeline-quality-monitor/", output: "✓ Skill 'pipeline_quality_monitor' v1.0.0 registered\n✓ Schedule: every 5 minutes\n✓ Available to all agents", durationMs: 1300 },
        { type: "text", content: "「数据管道质量监控」技能已上线，接入方式：\n\n```python\nuse_skill('pipeline_quality_monitor', {\n  'pipeline_id': 'trades-realtime',\n  'source': {'type': 'kafka', 'topic': 'trades.raw'},\n  'sink': {'type': 'clickhouse', 'table': 'trades'},\n  'thresholds': {'max_latency_ms': 500}\n})\n```\n\n每 5 分钟自动检测延迟、完整性、重复率、Schema 漂移和流量异常。所有数据工程师的管道都能接入。" },
      ],
    },
  ],
  "mock-product-1": [
    {
      id: "m7-1", role: "user", timestamp: now - 6 * 3600_000, source: "feishu" as MessageSource,
      blocks: [{ type: "text", content: "写一个跨境支付产品的 PRD 大纲" }],
    },
    {
      id: "m7-2", role: "assistant", timestamp: now - 5.5 * 3600_000,
      blocks: [
        { type: "thinking", content: "跨境支付 PRD 需覆盖：产品定位、用户画像、核���功能、合规、技术方案、商业模型、里程碑。", durationMs: 1900 },
        { type: "sub_agent", agentName: "合规专员", task: "审查跨境支付合规要求", result: "需持有跨境支付牌照，对接 FATF 旅行规则，符合 PIPL 和各国数据保护法。", durationMs: 6000 },
        { type: "sub_agent", agentName: "法务顾问", task: "评估知识产权和合同风险", result: "建议签订 SLA 保障协议，用户协议明确汇率风险承担方，注册相关商标。", durationMs: 4200 },
        { type: "text", content: "**跨境支付产品 PRD 大纲**\n\n**1. 产品概述** — 中小企业跨境 B2B 支付，中国 ↔ 东南亚\n**2. 用户画像** — 年交易额 100万-5000万 USD 进出口企业\n**3. 核心功能** — 在线换汇、批量付款、虚拟收款账户、交易追踪\n**4. 里程碑** — P1 单笔付款(8周) → P2 批量+API(4周) → P3 收款账户(6周)" },
      ],
    },
    {
      id: "m7-3", role: "user", timestamp: now - 5 * 3600_000, source: "app" as MessageSource,
      blocks: [{ type: "text", content: "PRD 写完后每次都要手动做竞品对比和市场数据更新，太费时间了" }],
    },
    {
      id: "m7-4", role: "assistant", timestamp: now - 4.8 * 3600_000,
      blocks: [
        { type: "thinking", content: "用户的痛点是竞品分析和市场数据收集太耗时。这个需求很通用——所有产品经理都需要定期更新竞品情报。\n\n可以做成一个自动化技能：\n1. 爬取竞品官网/App Store 更新\n2. 汇总行业报告关键数据\n3. 生成结构化竞品对比矩阵\n4. 定期推送变更摘要\n\n我没有编写代码的能力，需要请全栈工程师来实现这个技能。", durationMs: 2400 },
        { type: "text", content: "这个确实可以自动化。我来请全栈工程师开发一个「竞品情报」技能，以后所有产品经理都能一键获取最新竞品动态和市场数据。" },
        {
          type: "hil", action: "派发新技能开发任务", targetAgent: "全栈工程师", description: "开发「竞品情报分析」技能 — 竞品动态采集、功能对比矩阵、价格追踪、用户评价分析，供所有产品经理使用", mode: "single", options: [
            { label: "仅功能对比矩阵 + 价格追踪", value: "basic" },
            { label: "完整版：采集 + 对比 + 评价分析 + 周报", value: "full" },
          ], allowInput: true, inputPlaceholder: "指定关注的竞品或维度..."
        },
        { type: "sub_agent", agentName: "全栈工程师", task: "开发「竞品情报分析」技能，自动采集竞品动态、市场数据，生成对比报告", result: "已创建技能 skills/competitive-intel/\n\n```yaml\nname: competitive_intel\nversion: 1.0.0\ndescription: 竞品情报自动采集与分析\nschedule: weekly\ninterface:\n  input:\n    product_domain: string  # 产品领域\n    competitors: list[Competitor]\n    dimensions:\n      - pricing\n      - features\n      - market_share\n      - user_reviews\n  output:\n    comparison_matrix: ComparisonMatrix\n    changes_since_last: list[Change]\n    market_summary: MarketSummary\n```\n\n核心模块：\n- `WebIntelCollector` — 竞品官网/App Store/社媒动态采集\n- `FeatureMatrixBuilder` — 功能对比矩阵自动生成\n- `PricingTracker` — 价格变动追踪与历史趋势\n- `ReviewAnalyzer` — 用户评价情感分析 + 关键词提取\n- `ReportGenerator` — 输出 Notion/飞书文档格式报告\n\n每周自动执行，有重大变更（新功能上线、价格调整）实时推送。", durationMs: 19000 },
        { type: "tool_call", tool: "Bash", input: "safeclaw skill register skills/competitive-intel/", output: "✓ Skill 'competitive_intel' v1.0.0 registered\n✓ Schedule: weekly (Monday 9:00)\n✓ Available to all agents", durationMs: 1200 },
        { type: "text", content: "「竞品情报分析」技能已注册，使用方式：\n\n```python\nresult = use_skill('competitive_intel', {\n  'product_domain': '跨境支付',\n  'competitors': [\n    {'name': 'Wise', 'url': 'wise.com'},\n    {'name': 'Airwallex', 'url': 'airwallex.com'},\n    {'name': 'PingPong', 'url': 'pingpongx.com'}\n  ],\n  'dimensions': ['pricing', 'features', 'market_share']\n})\n\nresult.comparison_matrix.to_notion()\nresult.changes_since_last  # 上次以来的变更\n```\n\n每周一自动生成报告，有重大竞品动态会实时推送。所有产品经理都能用了，不用再手动收集了。" },
      ],
    },
    {
      id: "m7-5", role: "assistant", timestamp: now - 1.5 * 3600_000,
      blocks: [
        { type: "event", source: "social", topic: "twitter.competitor.alert", summary: "Airwallex 在 Twitter/X 宣布推出「AI 智能对账」功能，支持自动匹配发票与银行流水，号称准确率 99.2%", eventTime: now - 2 * 3600_000, detail: "Source: @Airwallex (Twitter/X)\nPost: \"Introducing AI-Powered Reconciliation 🚀\nAutomatically match invoices to bank statements with 99.2% accuracy.\nNow available for all Business accounts.\"\nEngagement: 2.3K likes, 891 retweets, 234 comments\nSentiment: 82% positive\nKey comments: Users praising time savings, some concerns about edge cases" },
        { type: "event", source: "news", topic: "polymarket.crossborder_regulation", summary: "Polymarket「2025年跨境支付监管收紧」合约价格升至 $0.71，市场预期东南亚多国将出台新的跨境支付牌照要求", eventTime: now - 1.8 * 3600_000, detail: "Market: \"Stricter Cross-Border Payment Regulation in SEA by 2025\"\nContract: POLY-XBORDER-SEA-2025\nPrice: $0.58 → $0.71 (+22.4%)\nVolume 24h: $1.2M\nRelated markets:\n  - \"Thailand e-Payment License Required\": $0.82\n  - \"Indonesia BI New Rules\": $0.67" },
        { type: "thinking", content: "两个重要事件需要关注：\n\n1. Airwallex 推出 AI 对账 — 这是我们 PRD 中规划的功能，竞品已经先发。需要评估是否加速开发或差异化。\n\n2. Polymarket 跨境监管预期上升 — 东南亚市场牌照要求可能变严，影响我们的出海时间线和合规成本。\n\n这两个事件都需要用户决策：产品路线图是否调整、合规预算是否追加。", durationMs: 2600 },
        { type: "tool_call", tool: "KnowledgeBase", input: "查询当前产品路线图中对账功能的排期", output: "AI 对账功能排在 P3 阶段（预计 Q3 启动），当前优先级较低。Airwallex 已先发，建议重新评估优先级。", durationMs: 220 },
        { type: "text", content: "检测到两个需要关注的事件：\n\n**竞品动态：** Airwallex 已上线 AI 智能对账，而我们排在 P3（Q3）。如果不提前，可能失去差异化窗口。\n\n**监管风险：** Polymarket 数据显示东南亚跨境支付监管收紧概率升至 71%，可能影响出海时间线和合规成本。\n\n建议调整产品策略：" },
        {
          type: "hil", action: "调整产品路线图", targetAgent: "产品经理", description: "根据竞品动态和监管预期，评估是否调整跨境支付产品路线图", mode: "single", options: [
            { label: "提前 AI 对账到 P2，与批量付款并行开发", value: "accelerate" },
            { label: "维持现有排期，但增加差异化功能（多币种智能匹配）", value: "differentiate" },
            { label: "优先处理东南亚合规，推迟功能开发", value: "compliance_first" },
            { label: "同时推进：对账提前 + 合规并行", value: "both" },
          ], allowInput: true, inputPlaceholder: "补充产品策略考虑..."
        },
      ],
    },
  ],
  "mock-finance-1": [
    {
      id: "mf-1", role: "user", timestamp: now - 18 * 60_000, source: "wecom" as MessageSource,
      blocks: [{ type: "text", content: "帮我处理一下这个月的供应商付款，有三笔待审批" }],
    },
    {
      id: "mf-2", role: "assistant", timestamp: now - 16 * 60_000,
      blocks: [
        { type: "thinking", content: "用户需要处理供应商付款审批。我需要：\n1. 查看待审批的付款单\n2. 核对发票与合同\n3. 验证金额和付款条件\n4. 逐笔请求用户授权\n\n涉及资金操作，必须走人机回路确认，且支付执行需要在 TEE 中完成以确保安全。", durationMs: 1800 },
        { type: "tool_call", tool: "DocQuery", input: "查询 2024年12月待审批付款单", output: "[\n  {payee: \"深圳云算科技\", amount: \"128,000.00\", invoice: \"INV-2024-1201\"},\n  {payee: \"杭州数据智能\", amount: \"256,500.00\", invoice: \"INV-2024-1215\"},\n  {payee: \"AWS China\", amount: \"$18,432.50\", invoice: \"INV-AWS-1228\"}\n]", durationMs: 200 },
        { type: "tool_call", tool: "KnowledgeBase", input: "核对供应商合同条款与付款条件", output: "All 3 vendors have valid contracts. Payment terms verified.", durationMs: 150 },
        { type: "text", content: "已核对完毕，共 3 笔待审批付款。发票与合同条款均已验证通过，请逐笔确认授权：" },
        { type: "payment", payee: "深圳云算科技有限公司", amount: "128,000.00", currency: "¥", description: "12月 GPU 算力服务费", invoiceId: "INV-2024-1201" },
        { type: "payment", payee: "杭州数据智能科技有限公司", amount: "256,500.00", currency: "¥", description: "数据标注服务费（Q4结算）", invoiceId: "INV-2024-1215" },
        { type: "payment", payee: "AWS China (Ningxia)", amount: "18,432.50", currency: "$", description: "12月云服务费用", invoiceId: "INV-AWS-1228" },
        { type: "text", content: "所有支付操作将在可信执行环境（TEE）中安全执行，交易数据全程加密，确保资金安全。授权后我会自动完成记账和凭证归档。" },
      ],
    },
    {
      id: "mf-3", role: "assistant", timestamp: now - 8 * 60_000,
      blocks: [
        { type: "event", source: "market", topic: "forex.usd_cny.alert", summary: "美元兑人民币汇率突破 7.35 关口（7.3521），创近 3 个月新高，日内波动 +0.42%", eventTime: now - 10 * 60_000, detail: "Pair: USD/CNY\nRate: 7.3521 (+0.42%)\nPrev close: 7.3213\n24h range: 7.3180 - 7.3558\nTrigger: US non-farm payroll data beat expectations\nPolymarket \"CNY depreciation >7.4 by Q1\": $0.38 → $0.52" },
        { type: "event", source: "task", topic: "skill.model_monitor.completed", summary: "模型监控技能执行完成：信用评分模型 PSI = 0.18（预警阈值 0.1），AUC 从 0.78 降至 0.74", eventTime: now - 9 * 60_000, detail: "Skill: model_monitor\nModel: credit_score_v3\nPSI: 0.18 (threshold: warn=0.1, alert=0.25)\nAUC: 0.78 → 0.74 (-0.04)\nDrift features: income_level (KS=0.12), debt_ratio (KS=0.09)\nRecommendation: Schedule retraining" },
        { type: "thinking", content: "两个事件需要关注：\n\n1. 汇率波动 — 我们有一笔 $18,432.50 的 AWS 待付款。如果汇率继续走高，延迟支付可能增加成本。但如果短期回调，等待可能更划算。Polymarket 数据显示人民币继续贬值的概率在上升。\n\n2. 模型监控告警 — PSI 超过预警阈值，AUC 下降 0.04，说明模型性能在衰减。这会影响信贷审批的准确性，间接影响坏账率和财务指标。\n\n需要用户对这两个事件做出决策。", durationMs: 2400 },
        { type: "tool_call", tool: "DocQuery", input: "查询当前未结算外币应付账款", output: "USD payables: $18,432.50 (AWS) + $5,200.00 (Stripe fees)\nTotal: $23,632.50\nAt current rate (7.3521): ¥173,741.24\nAt yesterday rate (7.3213): ¥173,013.73\nDifference: +¥727.51", durationMs: 180 },
        { type: "text", content: "检测到两个需要关注的事件：\n\n**汇率波动：** USD/CNY 突破 7.35，当前未结算美元应付 $23,632.50，按今日汇率比昨日多付 ¥727.51。Polymarket 显示人民币继续贬值概率升至 52%。\n\n**模型告警：** 信用评分模型 PSI 达 0.18（超预警线），AUC 降至 0.74，建议安排重训练以控制坏账风险。" },
        {
          type: "hil", action: "外币付款时机决策", targetAgent: "财务分析师", description: "美元汇率走高，需决定是否立即执行美元付款还是等待汇率回调", mode: "single", options: [
            { label: "立即支付 — 锁定当前汇率，避免进一步贬值风险", value: "pay_now" },
            { label: "等待 48 小时 — 观察汇率走势再决定", value: "wait_48h" },
            { label: "分批支付 — 先付 AWS（金额大），Stripe 费用延后", value: "split" },
            { label: "购买远期外汇合约对冲", value: "hedge" },
          ], allowInput: true, inputPlaceholder: "补充汇率判断或风控要求..."
        },
      ],
    },
  ],
};

// =============================================================================
// Simulation engine — progressive block reveal for mock-fullstack-1
// =============================================================================

/** A simulation step: either a block to add to a message, or a side-effect */
interface SimStep {
  /** Delay in ms before this step executes */
  delay: number;
  /** The block to append to the current assistant message */
  block?: RichBlock;
  /** If true, pause simulation until the HIL block is confirmed */
  pauseForHil?: boolean;
  /** Streaming text to show while "thinking" */
  streamText?: string;
  /** Clear streaming display */
  clearStream?: boolean;
  /** Side-effect: push unread to another session */
  notify?: { sessionId: string; count: number };
  /** Side-effect: start a new assistant message (for multi-message responses) */
  newMessage?: boolean;
}

const FULLSTACK_SIMULATION: SimStep[] = [
  // --- User sends message, assistant starts responding ---
  { delay: 500, streamText: "正在分析公司整体运营状况..." },
  { delay: 2000, clearStream: true, block: { type: "thinking", content: "用户询问公司情况，我需要从多个维度综合分析：\n\n1. 技术层面：各服务状态、代码质量、基础设施\n2. 业务层面：交易量、用户增长、合规状态\n3. 团队层面：各智能体的工作进展\n4. 风险层面：模型监控、安全扫描\n\n需要调用多个工具和子智能体来收集信息。", durationMs: 2800 } },
  // Tool calls
  { delay: 800, block: { type: "tool_call", tool: "Bash", input: "kubectl get pods -n safeclaw --no-headers | wc -l && kubectl top nodes", output: "42 pods running\nNAME     CPU(cores)   MEMORY(bytes)\nnode-1   1247m        6841Mi\nnode-2   892m         5203Mi\nnode-3   1103m        7122Mi", durationMs: 1500 } },
  { delay: 600, block: { type: "tool_call", tool: "Bash", input: "curl -s http://gateway:8080/metrics | grep -E 'requests_total|error_rate|p99_latency'", output: "http_requests_total{service=\"gateway\"} 2847291\nhttp_error_rate{service=\"gateway\"} 0.0023\nhttp_p99_latency_ms{service=\"gateway\"} 47.2\nhttp_requests_total{service=\"risk-engine\"} 1293847\nhttp_error_rate{service=\"risk-engine\"} 0.0008\nhttp_p99_latency_ms{service=\"risk-engine\"} 12.8", durationMs: 800 } },
  { delay: 500, block: { type: "tool_call", tool: "KnowledgeBase", input: "查询本月业务指标：交易量、用户数、合规状态", output: "本月数据（截至今日）：\n- 跨境交易量：¥3.8亿（环比 +12%）\n- 活跃商户数：1,247（新增 89）\n- 交易成功率：99.7%\n- 合规审查通过率：98.2%\n- 未决合规事项：3件", durationMs: 350 } },
  // Sensitive data query — requires human authorization + TEE execution
  { delay: 600, block: { type: "thinking", content: "需要查询核心商户的详细交易数据来评估公司营收结构，但该数据包含商户真实名称、银行账号和交易明细，属于高度敏感数据。必须请求用户授权，并在 TEE 中执行查询，返回脱敏结果。", durationMs: 1200 } },
  {
    delay: 400, block: {
      type: "tee_data_query",
      query: "SELECT merchant_name, bank_account, total_volume, avg_txn_amount, risk_score FROM core_merchants WHERE monthly_volume > 10000000 ORDER BY total_volume DESC LIMIT 5",
      dataSource: "safeclaw-prod.core_merchants",
      reason: "评估公司 Top 商户营收贡献和风险分布，用于综合运营分析",
      privacyLevel: "L3 - 高度敏感",
      redactedResult: {
        columns: ["商户名称", "银行账号", "月交易额", "笔均金额", "风险评分"],
        rows: [
          ["深圳***科技有限公司", "6225 **** **** 3041", "¥8,720万", "¥4,230", "A"],
          ["上海***国际贸易", "6228 **** **** 5587", "¥6,150万", "¥12,800", "A"],
          ["杭州***电子商务", "6222 **** **** 9912", "¥4,890万", "¥2,560", "B+"],
          ["北京***金融信息", "6217 **** **** 7703", "¥3,210万", "¥89,500", "A-"],
          ["广州***供应链管理", "6225 **** **** 1268", "¥2,470万", "¥6,780", "B"],
        ],
      },
      redactionSummary: "已脱敏：商户名称部分遮蔽（保留地区 + 后缀）、银行账号仅保留前 4 位和后 4 位、原始交易流水已过滤。本次查询已记录审计日志 #AUD-20250212-0847。",
    }, pauseForHil: true
  },
  // --- After TEE query authorized, continue ---
  // Sub-agent calls
  { delay: 1000, streamText: "正在收集各团队智能体的工作进展..." },
  {
    delay: 1500, clearStream: true, block: { type: "sub_agent", agentName: "风控分析师", task: "汇报当前风控模型状态和关键指标", result: "信用评分模型 v3 AUC 0.74（预警中，PSI=0.18 超阈值）。反欺诈模型正常，拦截率 99.2%。建议尽快安排信用模型重训练。已有 2 个特征因央行新规需移除。", durationMs: 4200 },
    notify: { sessionId: "mock-risk-1", count: 1 },
  },
  {
    delay: 800, block: { type: "sub_agent", agentName: "数据工程师", task: "汇报数据管道运行状况", result: "实时管道正常运行，Kafka 延迟 < 50ms。日均处理 1.2 亿条记录。上周修复了 ClickHouse 慢查询问题（p99 从 2.3s 降至 180ms）。数据质量监控已全面部署，零漂移告警。", durationMs: 3800 },
    notify: { sessionId: "mock-data-eng-1", count: 1 },
  },
  {
    delay: 800, block: { type: "sub_agent", agentName: "产品经理", task: "汇报产品迭代和竞品动态", result: "SafeClaw v0.9.4 已发布，新增知识库和资产管理功能。竞品动态：Airwallex 上线了智能对账功能（与我们 smart-reconciliation 项目方向一致），Stripe 在东南亚获得新牌照。建议加速对账功能上线。", durationMs: 5100 },
    notify: { sessionId: "mock-product-1", count: 1 },
  },
  // Event triggered during execution
  { delay: 1200, newMessage: true },
  {
    delay: 200, block: { type: "event", source: "market", topic: "polymarket.regulation", summary: "Polymarket 预测「中国金融科技监管加严 2025H2」合约涨至 $0.67（+15%），大额买单涌入", eventTime: Date.now(), detail: "Market: \"China Fintech Regulation Tightening 2025 H2\"\nPrice: $0.58 → $0.67 (+15.5%)\nVolume 24h: $890K (+280%)\nTrigger: PBOC draft regulation on credit scoring models" },
    notify: { sessionId: "mock-finance-1", count: 1 },
  },
  { delay: 600, block: { type: "thinking", content: "Polymarket 监管加严预期上升，与央行信用评分新规呼应。这会影响我们的合规计划和产品迭代节奏。需要在总结中提及并让用户决策。", durationMs: 1500 } },
  // Summary text
  { delay: 800, block: { type: "text", content: "## 公司运营状况总览\n\n### 基础设施\n- K8s 集群 3 节点 42 Pod 运行正常，CPU/内存利用率健康\n- 网关 P99 延迟 47ms，错误率 0.23%，风控引擎 P99 仅 12.8ms\n\n### 业务指标\n- 本月跨境交易量 **¥3.8亿**（环比 +12%）\n- 活跃商户 1,247 家（新增 89）\n- 交易成功率 **99.7%**\n\n### 需关注事项\n1. **信用评分模型衰减** — AUC 降至 0.74，PSI 超预警线，且央行新规要求移除 2 个社交特征\n2. **竞品威胁** — Airwallex 上线智能对账，建议加速我们的 smart-reconciliation 项目\n3. **监管趋严信号** — Polymarket 监管加严预测升至 67%，需提前准备合规整改\n\n请确认下一步工作优先级：" } },
  // HIL — needs user confirmation before continuing
  {
    delay: 500, block: {
      type: "hil", action: "确认下一步工作优先级", targetAgent: "全栈工程师", description: "根据当前公司状况，需要确定近期工作重点", mode: "multi", options: [
        { label: "紧急：信用模型合规整改（移除违规特征 + 重训练）", value: "credit_model" },
        { label: "高优：加速智能对账项目上线（应对 Airwallex 竞争）", value: "reconciliation" },
        { label: "中优：全面合规审计（应对监管趋严预期）", value: "compliance_audit" },
        { label: "常规：继续当前迭代计划", value: "continue" },
      ], allowInput: true, inputPlaceholder: "补充优先级考虑或其他工作安排..."
    }, pauseForHil: true
  },
  // --- After HIL confirmed, continue ---
  { delay: 800, newMessage: true },
  { delay: 100, streamText: "已收到您的确认，正在制定执行计划..." },
  { delay: 1500, clearStream: true, block: { type: "thinking", content: "用户已确认优先级。根据选择需要：\n1. 将任务分配给对应智能体\n2. 生成具体执行计划和时间表\n3. 设置监控和汇报机制", durationMs: 1800 } },
  {
    delay: 600, block: { type: "sub_agent", agentName: "风控分析师", task: "启动信用模型合规整改：移除 social_network_score 和 contact_frequency 特征，重训练并评估", result: "已启动整改流程：\n1. 移除 2 个违规特征后，初步评估 AUC 降至 0.71\n2. 正在添加替代特征（交易行为时序特征）补偿\n3. 预计重训练后 AUC 可恢复至 0.76-0.78\n4. 计划 3 天内完成，7 天完成 OOT 验证", durationMs: 8500 },
    notify: { sessionId: "mock-risk-1", count: 2 },
  },
  {
    delay: 600, block: { type: "sub_agent", agentName: "财务分析师", task: "评估合规整改的预算影响和资源需求", result: "合规整改预计需要：\n- 算力成本：约 ¥15,000（GPU 重训练）\n- 人力：风控组 2人 × 2周，工程组 1人 × 1周\n- 外部审计费用：约 ¥50,000（年度模型审计）\n- 总预算影响：¥80,000 以内，在季度预算范围内", durationMs: 6200 },
    notify: { sessionId: "mock-finance-1", count: 2 },
  },
  { delay: 800, block: { type: "text", content: "执行计划已下发各智能体，关键安排：\n\n| 优先级 | 任务 | 负责人 | 预计完成 |\n|--------|------|--------|----------|\n| P0 | 信用模型移除违规特征 + 重训练 | 风控分析师 | 3 天 |\n| P0 | 模型 OOT 验证 + 合规报告 | 风控分析师 | 7 天 |\n| P1 | smart-reconciliation 加速上线 | 财务分析师 | 2 周 |\n| P2 | 全面合规审计启动 | 合规专员 | 本月内 |\n\n所有任务进展将实时汇报到本会话。如需调整优先级或有其他安排，随时告诉我。" } },
];

/** Manages the simulation lifecycle for a session */
function useSimulation(sessionId: string) {
  const [messages, setMessages] = useState<RichMessage[]>([]);
  const [isRunning, setIsRunning] = useState(false);
  const [waitingForHil, setWaitingForHil] = useState(false);
  const stepIndexRef = useRef(0);
  const currentMsgIdRef = useRef(0);
  const timersRef = useRef<ReturnType<typeof setTimeout>[]>([]);

  const clearTimers = useCallback(() => {
    for (const t of timersRef.current) clearTimeout(t);
    timersRef.current = [];
  }, []);

  // Clean up on unmount
  useEffect(() => clearTimers, [clearTimers]);

  const resumeSimulation = useCallback(() => {
    setWaitingForHil(false);
    // Continue from current step
    runFromStep(stepIndexRef.current);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  const runFromStep = useCallback((startIdx: number) => {
    const steps = FULLSTACK_SIMULATION;
    let cumulativeDelay = 0;

    for (let i = startIdx; i < steps.length; i++) {
      const step = steps[i];
      cumulativeDelay += step.delay;

      const timer = setTimeout(() => {
        stepIndexRef.current = i + 1;

        if (step.streamText) {
          agentModel.setStreaming(sessionId, step.streamText);
        }
        if (step.clearStream) {
          agentModel.setStreaming(sessionId, null);
        }
        if (step.notify) {
          agentModel.incrementUnread(step.notify.sessionId, step.notify.count);
        }
        if (step.newMessage) {
          currentMsgIdRef.current++;
        }

        if (step.block) {
          setMessages((prev) => {
            const copy = [...prev];
            const msgIdx = copy.findIndex((m) => m.id === `sim-a-${currentMsgIdRef.current}`);
            if (msgIdx >= 0) {
              // Append block to existing assistant message
              copy[msgIdx] = {
                ...copy[msgIdx],
                blocks: [...copy[msgIdx].blocks, step.block!],
              };
            } else {
              // Create new assistant message
              copy.push({
                id: `sim-a-${currentMsgIdRef.current}`,
                role: "assistant",
                timestamp: Date.now(),
                blocks: [step.block!],
              });
            }
            return copy;
          });
        }

        // Check if we should pause for HIL
        if (step.pauseForHil) {
          setWaitingForHil(true);
          agentModel.setStreaming(sessionId, null);
          // Don't schedule further steps
        }

        // If this is the last step, mark simulation as done
        if (i === steps.length - 1) {
          setIsRunning(false);
          agentModel.setStreaming(sessionId, null);
          agentModel.setSessionStatus(sessionId, "idle");
        }
      }, cumulativeDelay);

      timersRef.current.push(timer);

      // If this step pauses, don't schedule further steps
      if (step.pauseForHil) {
        break;
      }
    }
  }, [sessionId]);

  const startSimulation = useCallback((userText: string) => {
    clearTimers();
    setIsRunning(true);
    setWaitingForHil(false);
    stepIndexRef.current = 0;
    currentMsgIdRef.current = 0;
    agentModel.setSessionStatus(sessionId, "running");

    // Add user message
    const sources: MessageSource[] = ["app", "dingtalk", "feishu", "wecom"];
    const userMsg: RichMessage = {
      id: `sim-u-${Date.now()}`,
      role: "user",
      timestamp: Date.now(),
      source: sources[Math.floor(Math.random() * sources.length)],
      blocks: [{ type: "text", content: userText }],
    };
    setMessages([userMsg]);

    // Start simulation steps
    runFromStep(0);
  }, [sessionId, clearTimers, runFromStep]);

  return { messages, isRunning, waitingForHil, startSimulation, resumeSimulation };
}

// =============================================================================
// Main Chat Component
// =============================================================================

export default function AgentChat({ sessionId }: { sessionId: string }) {
  const staticMessages = MOCK_RICH_MESSAGES[sessionId] || [];
  const isSimulatable = sessionId === "mock-fullstack-1";
  const sim = useSimulation(sessionId);
  const richMessages = isSimulatable ? sim.messages : staticMessages;
  const virtuosoRef = useRef<VirtuosoHandle>(null);

  // Auto-scroll on new messages
  useEffect(() => {
    if (richMessages.length > 0) {
      virtuosoRef.current?.scrollToIndex({
        index: richMessages.length - 1,
        align: "end",
        behavior: "smooth",
      });
    }
  }, [richMessages.length, richMessages]);

  // Handle user input — for simulatable sessions, trigger simulation
  const handleSend = useCallback((text: string) => {
    if (isSimulatable && !sim.isRunning) {
      sim.startSimulation(text);
    }
  }, [isSimulatable, sim]);

  // Handle HIL confirmation — resume simulation
  const handleHilConfirm = useCallback(() => {
    if (isSimulatable && sim.waitingForHil) {
      sim.resumeSimulation();
    }
  }, [isSimulatable, sim]);

  return (
    <ResizablePanelGroup direction="vertical" className="h-full">
      <ResizablePanel className="flex flex-col overflow-hidden">
        <ChatHeader sessionId={sessionId} />
        <div className="flex-1 min-h-0" role="log" aria-live="polite" aria-label="Chat messages">
          <Virtuoso
            ref={virtuosoRef}
            className="h-full"
            data={richMessages}
            itemContent={(_index, msg) => <MessageItem msg={msg} sessionId={sessionId} onHilConfirm={handleHilConfirm} />}
            followOutput="smooth"
            components={{ Footer: () => <StreamingDisplay sessionId={sessionId} /> }}
          />
        </div>
      </ResizablePanel>
      <ResizableHandle />
      <ResizablePanel defaultSize={20} minSize={10} maxSize={35}>
        <AgentInput sessionId={sessionId} disabled={sim.isRunning && !sim.waitingForHil} onSend={handleSend} />
      </ResizablePanel>
    </ResizablePanelGroup>
  );
}
