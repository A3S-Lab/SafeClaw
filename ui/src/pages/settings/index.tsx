import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { useModal } from "@/components/custom/modal-provider";
import { cn } from "@/lib/utils";
import settingsModel from "@/models/settings.model";
import {
  Bot,
  Check,
  ChevronRight,
  Eye,
  EyeOff,
  Info,
  KeyRound,
  Layers,
  RotateCcw,
  Server,
  ShieldCheck,
  Sparkles,
} from "lucide-react";
import { useCallback, useState } from "react";
import { useSnapshot } from "valtio";
import { toast } from "sonner";

// =============================================================================
// Section definitions
// =============================================================================

const sections = [
  { id: "ai", label: "AI 服务", icon: Bot, description: "模型与认证" },
  { id: "gateway", label: "网关连接", icon: Server, description: "服务地址" },
  { id: "about", label: "关于", icon: Info, description: "版本与数据" },
] as const;

type SectionId = (typeof sections)[number]["id"];

// =============================================================================
// Sidebar
// =============================================================================

function SettingsSidebar({
  current,
  onChange,
}: {
  current: SectionId;
  onChange: (id: SectionId) => void;
}) {
  return (
    <nav aria-label="Settings sections" className="w-52 shrink-0 border-r border-border flex flex-col">
      <div className="px-5 pt-5 pb-4">
        <h1 className="text-base font-bold">设置</h1>
        <p className="text-xs text-muted-foreground mt-0.5">管理应用配置</p>
      </div>
      <div className="px-3 space-y-0.5 flex-1">
        {sections.map((s) => {
          const active = current === s.id;
          return (
            <button
              key={s.id}
              onClick={() => onChange(s.id)}
              aria-current={active ? "page" : undefined}
              className={cn(
                "w-full flex items-center gap-3 text-left px-3 py-2.5 rounded-lg text-sm transition-all group",
                active
                  ? "bg-primary/10 text-primary"
                  : "text-muted-foreground hover:text-foreground hover:bg-muted/50",
              )}
            >
              <div className={cn(
                "flex items-center justify-center size-8 rounded-lg shrink-0 transition-colors",
                active ? "bg-primary/15" : "bg-muted group-hover:bg-muted",
              )}>
                <s.icon className={cn("size-4", active ? "text-primary" : "text-muted-foreground group-hover:text-foreground")} />
              </div>
              <div className="flex-1 min-w-0">
                <div className="font-medium text-[13px] leading-tight">{s.label}</div>
                <div className={cn("text-[10px] leading-tight mt-0.5", active ? "text-primary/70" : "text-muted-foreground/70")}>{s.description}</div>
              </div>
              <ChevronRight className={cn("size-3.5 shrink-0 transition-opacity", active ? "opacity-60" : "opacity-0 group-hover:opacity-40")} />
            </button>
          );
        })}
      </div>
      <div className="px-5 py-4 border-t">
        <div className="flex items-center gap-2 text-[10px] text-muted-foreground/60">
          <ShieldCheck className="size-3" />
          <span>SafeClaw v0.1.0</span>
        </div>
      </div>
    </nav>
  );
}

// =============================================================================
// Reusable components
// =============================================================================

function SectionHeader({ title, description, icon: Icon }: { title: string; description: string; icon: typeof Bot }) {
  return (
    <div className="flex items-start gap-3 mb-6">
      <div className="flex items-center justify-center size-10 rounded-xl bg-primary/10 shrink-0 mt-0.5">
        <Icon className="size-5 text-primary" />
      </div>
      <div>
        <h2 className="text-lg font-bold">{title}</h2>
        <p className="text-sm text-muted-foreground mt-0.5">{description}</p>
      </div>
    </div>
  );
}

function SettingRow({
  label,
  hint,
  children,
  action,
}: {
  label: string;
  hint?: string;
  children: React.ReactNode;
  action?: React.ReactNode;
}) {
  return (
    <div className="flex items-start justify-between gap-8 py-4 border-b border-border/50 last:border-b-0">
      <div className="shrink-0 min-w-[120px]">
        <div className="text-sm font-medium">{label}</div>
        {hint && <p className="text-xs text-muted-foreground mt-0.5 leading-relaxed">{hint}</p>}
      </div>
      <div className="flex-1 max-w-sm flex items-center gap-2">
        <div className="flex-1">{children}</div>
        {action}
      </div>
    </div>
  );
}

function SaveBar({ dirty, onSave, saved }: { dirty: boolean; onSave: () => void; saved: boolean }) {
  if (!dirty && !saved) return null;
  return (
    <div className={cn(
      "flex items-center gap-3 rounded-lg px-4 py-2.5 mt-6 transition-all",
      dirty ? "bg-primary/5 border border-primary/20" : "bg-muted/50 border border-border",
    )}>
      {dirty ? (
        <>
          <div className="flex-1 text-xs text-muted-foreground">有未保存的更改</div>
          <Button size="sm" onClick={onSave} className="h-7 text-xs">
            <Check className="size-3 mr-1" />
            保存
          </Button>
        </>
      ) : saved ? (
        <>
          <Check className="size-3.5 text-primary" />
          <span className="text-xs text-primary font-medium">已保存</span>
        </>
      ) : null}
    </div>
  );
}

// =============================================================================
// Provider logos / badges
// =============================================================================

const PROVIDERS = [
  { value: "anthropic", label: "Anthropic", badge: "Claude", color: "bg-orange-500/10 text-orange-600 dark:text-orange-400 border-orange-500/20" },
  { value: "openai", label: "OpenAI", badge: "GPT", color: "bg-teal-500/10 text-teal-600 dark:text-teal-400 border-teal-500/20" },
  { value: "custom", label: "自定义", badge: "Custom", color: "bg-purple-500/10 text-purple-600 dark:text-purple-400 border-purple-500/20" },
];

const MODEL_PRESETS: Record<string, string[]> = {
  anthropic: ["claude-sonnet-4-20250514", "claude-opus-4-20250514", "claude-haiku-3-20250414"],
  openai: ["gpt-4o", "gpt-4o-mini", "o3-mini"],
  custom: [],
};

// =============================================================================
// AI Section
// =============================================================================

function AiSection() {
  const snap = useSnapshot(settingsModel.state);
  const [provider, setProvider] = useState(snap.provider);
  const [model, setModel] = useState(snap.model);
  const [apiKey, setApiKey] = useState(snap.apiKey);
  const [dirty, setDirty] = useState(false);
  const [saved, setSaved] = useState(false);
  const [showKey, setShowKey] = useState(false);

  const markDirty = () => { setDirty(true); setSaved(false); };

  const handleSave = useCallback(() => {
    settingsModel.updateSettings({ provider, model, apiKey });
    setDirty(false);
    setSaved(true);
    toast.success("AI 服务设置已保存");
    setTimeout(() => setSaved(false), 2000);
  }, [provider, model, apiKey]);

  const currentProvider = PROVIDERS.find((p) => p.value === provider) || PROVIDERS[0];
  const presets = MODEL_PRESETS[provider] || [];

  return (
    <div>
      <SectionHeader icon={Bot} title="AI 服务" description="配置 AI 模型提供商、模型和 API 密钥。" />

      {/* Provider card */}
      <div className="rounded-xl border bg-card p-5 mb-4">
        <div className="flex items-center gap-2 mb-4">
          <Sparkles className="size-4 text-primary" />
          <span className="text-sm font-semibold">提供商</span>
        </div>
        <div className="grid grid-cols-3 gap-2">
          {PROVIDERS.map((p) => (
            <button
              key={p.value}
              type="button"
              className={cn(
                "rounded-lg border-2 px-3 py-3 text-center transition-all",
                provider === p.value
                  ? "border-primary bg-primary/5"
                  : "border-border hover:border-primary/30 hover:bg-muted/50",
              )}
              onClick={() => { setProvider(p.value); markDirty(); }}
            >
              <div className={cn("inline-flex items-center rounded-md border px-2 py-0.5 text-[10px] font-bold mb-1.5", p.color)}>
                {p.badge}
              </div>
              <div className="text-xs font-medium">{p.label}</div>
            </button>
          ))}
        </div>
      </div>

      {/* Model & Auth */}
      <div className="rounded-xl border bg-card p-5">
        <SettingRow
          label="模型"
          hint="创建会话时使用的默认模型"
        >
          {presets.length > 0 ? (
            <Select value={model} onValueChange={(v) => { setModel(v); markDirty(); }}>
              <SelectTrigger className="h-9 text-sm">
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                {presets.map((m) => (
                  <SelectItem key={m} value={m}>
                    <span className="font-mono text-xs">{m}</span>
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
          ) : (
            <Input
              className="h-9 text-sm font-mono"
              placeholder="输入模型名称"
              value={model}
              onChange={(e) => { setModel(e.target.value); markDirty(); }}
            />
          )}
        </SettingRow>

        <SettingRow
          label="API 密钥"
          hint="用于访问 AI 服务的凭证"
          action={
            <button
              type="button"
              className="text-muted-foreground hover:text-foreground transition-colors p-1"
              onClick={() => setShowKey(!showKey)}
              aria-label={showKey ? "Hide API key" : "Show API key"}
            >
              {showKey ? <EyeOff className="size-4" /> : <Eye className="size-4" />}
            </button>
          }
        >
          <div className="relative">
            <KeyRound className="absolute left-2.5 top-1/2 -translate-y-1/2 size-3.5 text-muted-foreground pointer-events-none" />
            <Input
              type={showKey ? "text" : "password"}
              className="h-9 text-sm font-mono pl-8"
              placeholder="sk-..."
              value={apiKey}
              onChange={(e) => { setApiKey(e.target.value); markDirty(); }}
            />
          </div>
        </SettingRow>

        {/* Current config summary */}
        <div className="mt-4 flex items-center gap-2 flex-wrap">
          <span className={cn("inline-flex items-center rounded-md border px-2 py-0.5 text-[10px] font-bold", currentProvider.color)}>
            {currentProvider.badge}
          </span>
          <span className="text-[11px] font-mono text-muted-foreground">{model}</span>
          {apiKey && (
            <span className="inline-flex items-center gap-1 text-[10px] text-muted-foreground">
              <Check className="size-2.5 text-primary" />
              密钥已设置
            </span>
          )}
        </div>
      </div>

      <SaveBar dirty={dirty} saved={saved} onSave={handleSave} />
    </div>
  );
}

// =============================================================================
// Gateway Section
// =============================================================================

function GatewaySection() {
  const snap = useSnapshot(settingsModel.state);
  const [baseUrl, setBaseUrl] = useState(snap.baseUrl);
  const [dirty, setDirty] = useState(false);
  const [saved, setSaved] = useState(false);

  const handleSave = useCallback(() => {
    settingsModel.updateSettings({ baseUrl });
    setDirty(false);
    setSaved(true);
    toast.success("网关设置已保存");
    setTimeout(() => setSaved(false), 2000);
  }, [baseUrl]);

  return (
    <div>
      <SectionHeader icon={Server} title="网关连接" description="配置 SafeClaw 网关的连接地址。" />

      <div className="rounded-xl border bg-card p-5">
        <SettingRow
          label="网关地址"
          hint="API 和 WebSocket 连接的服务端地址，留空使用默认值。"
        >
          <div className="relative">
            <Server className="absolute left-2.5 top-1/2 -translate-y-1/2 size-3.5 text-muted-foreground pointer-events-none" />
            <Input
              className="h-9 text-sm font-mono pl-8"
              placeholder="http://127.0.0.1:18790"
              value={baseUrl}
              onChange={(e) => { setBaseUrl(e.target.value); setDirty(true); setSaved(false); }}
            />
          </div>
        </SettingRow>

        {/* Connection status mock */}
        <div className="mt-4 flex items-center gap-2 text-xs">
          <span className="relative flex size-2">
            <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-primary opacity-75" />
            <span className="relative inline-flex rounded-full size-2 bg-primary" />
          </span>
          <span className="text-muted-foreground">已连接</span>
          <span className="text-muted-foreground/50">·</span>
          <span className="font-mono text-muted-foreground">{baseUrl || "http://127.0.0.1:18790"}</span>
        </div>
      </div>

      <SaveBar dirty={dirty} saved={saved} onSave={handleSave} />
    </div>
  );
}

// =============================================================================
// About Section
// =============================================================================

const INFO_ITEMS = [
  { label: "应用名称", value: "SafeClaw" },
  { label: "版本", value: "0.1.0" },
  { label: "运行时", value: "Tauri v2 + React 19" },
  { label: "TEE 支持", value: "Intel SGX / TDX" },
  { label: "许可证", value: "Apache-2.0" },
];

function AboutSection() {
  const modal = useModal();

  const handleReset = () => {
    modal.alert({
      title: "重置设置",
      description: "确认重置所有设置为默认值？此操作不可撤销。",
      confirmText: "重置",
      onConfirm: () => {
        settingsModel.resetSettings();
        toast.success("设置已重置为默认值");
        setTimeout(() => window.location.reload(), 500);
      },
    });
  };

  return (
    <div>
      <SectionHeader icon={Info} title="关于" description="应用信息与数据管理。" />

      {/* App info card */}
      <div className="rounded-xl border bg-card p-5 mb-4">
        <div className="flex items-center gap-3 mb-4">
          <div className="flex items-center justify-center size-12 rounded-xl bg-primary/10">
            <ShieldCheck className="size-6 text-primary" />
          </div>
          <div>
            <div className="text-base font-bold">SafeClaw</div>
            <div className="text-xs text-muted-foreground">Secure Personal AI Assistant with TEE Support</div>
          </div>
        </div>
        <div className="rounded-lg bg-muted/30 divide-y divide-border/50">
          {INFO_ITEMS.map((item) => (
            <div key={item.label} className="flex justify-between items-center px-4 py-2.5">
              <span className="text-xs text-muted-foreground">{item.label}</span>
              <span className="text-xs font-medium font-mono">{item.value}</span>
            </div>
          ))}
        </div>
      </div>

      {/* Tech stack */}
      <div className="rounded-xl border bg-card p-5 mb-4">
        <div className="flex items-center gap-2 mb-3">
          <Layers className="size-4 text-primary" />
          <span className="text-sm font-semibold">技术栈</span>
        </div>
        <div className="flex flex-wrap gap-1.5">
          {["Rust", "Tauri v2", "React 19", "TypeScript", "Tailwind CSS", "Valtio", "gRPC", "Intel SGX", "RA-TLS"].map((tech) => (
            <span key={tech} className="inline-flex items-center rounded-md border bg-muted/50 px-2 py-0.5 text-[11px] font-medium text-muted-foreground">
              {tech}
            </span>
          ))}
        </div>
      </div>

      {/* Danger zone */}
      <div className="rounded-xl border border-destructive/20 bg-destructive/[0.03] p-5">
        <div className="flex items-center gap-2 mb-2">
          <RotateCcw className="size-4 text-destructive" />
          <span className="text-sm font-semibold text-destructive">危险操作</span>
        </div>
        <p className="text-xs text-muted-foreground mb-3">
          重置后所有配置将恢复为默认值，包括 AI 提供商、模型、API 密钥和网关地址。
        </p>
        <Button variant="destructive" size="sm" onClick={handleReset}>
          <RotateCcw className="size-3.5 mr-1.5" />
          重置所有设置
        </Button>
      </div>
    </div>
  );
}

// =============================================================================
// Settings Page
// =============================================================================

export default function SettingsPage() {
  const [section, setSection] = useState<SectionId>("ai");

  return (
    <div className="flex h-full w-full">
      <SettingsSidebar current={section} onChange={setSection} />
      <main className="flex-1 overflow-y-auto">
        <div className="max-w-2xl mx-auto px-8 py-8">
          {section === "ai" && <AiSection />}
          {section === "gateway" && <GatewaySection />}
          {section === "about" && <AboutSection />}
        </div>
      </main>
    </div>
  );
}
