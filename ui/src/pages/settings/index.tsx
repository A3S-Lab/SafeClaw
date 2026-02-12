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
import type { ProviderConfig, ModelConfig } from "@/models/settings.model";
import {
  Bot, Check, ChevronRight, Eye, EyeOff, Info, KeyRound, Layers,
  Plus, RotateCcw, Server, ShieldCheck, Star, Trash2, X,
} from "lucide-react";
import { useState } from "react";
import { useSnapshot } from "valtio";
import { toast } from "sonner";

const sections = [
  { id: "ai", label: "AI 服务", icon: Bot, description: "模型与认证" },
  { id: "gateway", label: "网关连接", icon: Server, description: "服务地址" },
  { id: "about", label: "关于", icon: Info, description: "版本与数据" },
] as const;
type SectionId = (typeof sections)[number]["id"];

const PROVIDER_COLORS: Record<string, string> = {
  anthropic: "bg-orange-500/10 text-orange-600 dark:text-orange-400 border-orange-500/20",
  openai: "bg-teal-500/10 text-teal-600 dark:text-teal-400 border-teal-500/20",
  google: "bg-blue-500/10 text-blue-600 dark:text-blue-400 border-blue-500/20",
  deepseek: "bg-indigo-500/10 text-indigo-600 dark:text-indigo-400 border-indigo-500/20",
};
const pColor = (n: string) => PROVIDER_COLORS[n] || "bg-purple-500/10 text-purple-600 dark:text-purple-400 border-purple-500/20";

function SettingsSidebar({ current, onChange }: { current: SectionId; onChange: (id: SectionId) => void }) {
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
            <button key={s.id} onClick={() => onChange(s.id)} aria-current={active ? "page" : undefined}
              className={cn("w-full flex items-center gap-3 text-left px-3 py-2.5 rounded-lg text-sm transition-all group",
                active ? "bg-primary/10 text-primary" : "text-muted-foreground hover:text-foreground hover:bg-muted/50")}>
              <div className={cn("flex items-center justify-center size-8 rounded-lg shrink-0", active ? "bg-primary/15" : "bg-muted group-hover:bg-muted")}>
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
        <div className="flex items-center gap-2 text-[10px] text-muted-foreground/60"><ShieldCheck className="size-3" /><span>SafeClaw v0.1.0</span></div>
      </div>
    </nav>
  );
}

function SectionHeader({ title, description, icon: Icon }: { title: string; description: string; icon: typeof Bot }) {
  return (
    <div className="flex items-start gap-3 mb-6">
      <div className="flex items-center justify-center size-10 rounded-xl bg-primary/10 shrink-0 mt-0.5"><Icon className="size-5 text-primary" /></div>
      <div><h2 className="text-lg font-bold">{title}</h2><p className="text-sm text-muted-foreground mt-0.5">{description}</p></div>
    </div>
  );
}

function SettingRow({ label, hint, children, action }: { label: string; hint?: string; children: React.ReactNode; action?: React.ReactNode }) {
  return (
    <div className="flex items-start justify-between gap-8 py-4 border-b border-border/50 last:border-b-0">
      <div className="shrink-0 min-w-[120px]">
        <div className="text-sm font-medium">{label}</div>
        {hint && <p className="text-xs text-muted-foreground mt-0.5 leading-relaxed">{hint}</p>}
      </div>
      <div className="flex-1 max-w-sm flex items-center gap-2"><div className="flex-1">{children}</div>{action}</div>
    </div>
  );
}

function AddProviderForm({ onAdd, onCancel }: { onAdd: (p: ProviderConfig) => void; onCancel: () => void }) {
  const [name, setName] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [baseUrl, setBaseUrl] = useState("");
  return (
    <div className="rounded-xl border-2 border-dashed border-primary/30 bg-primary/[0.02] p-4 space-y-3">
      <div className="flex items-center justify-between">
        <span className="text-sm font-semibold">添加 Provider</span>
        <button type="button" onClick={onCancel} className="text-muted-foreground hover:text-foreground"><X className="size-4" /></button>
      </div>
      <Input className="h-8 text-sm" placeholder="Provider 名称 (如 anthropic, openai)" value={name} onChange={(e) => setName(e.target.value)} />
      <Input className="h-8 text-sm font-mono" placeholder="API Key (可选)" type="password" value={apiKey} onChange={(e) => setApiKey(e.target.value)} />
      <Input className="h-8 text-sm font-mono" placeholder="Base URL (可选)" value={baseUrl} onChange={(e) => setBaseUrl(e.target.value)} />
      <div className="flex justify-end gap-2">
        <Button variant="ghost" size="sm" className="h-7 text-xs" onClick={onCancel}>取消</Button>
        <Button size="sm" className="h-7 text-xs" disabled={!name.trim()} onClick={() => onAdd({ name: name.trim().toLowerCase(), apiKey, baseUrl, models: [] })}>
          <Plus className="size-3 mr-1" />添加
        </Button>
      </div>
    </div>
  );
}

function AddModelForm({ onAdd, onCancel }: { onAdd: (m: ModelConfig) => void; onCancel: () => void }) {
  const [id, setId] = useState(""); const [name, setName] = useState("");
  const [apiKey, setApiKey] = useState(""); const [baseUrl, setBaseUrl] = useState("");
  const [context, setContext] = useState("128000"); const [output, setOutput] = useState("4096");
  return (
    <div className="rounded-lg border-2 border-dashed border-primary/30 bg-primary/[0.02] p-3 space-y-2.5 mt-2">
      <div className="flex items-center justify-between">
        <span className="text-xs font-semibold">添加模型</span>
        <button type="button" onClick={onCancel} className="text-muted-foreground hover:text-foreground"><X className="size-3.5" /></button>
      </div>
      <div className="grid grid-cols-2 gap-2">
        <Input className="h-7 text-xs font-mono" placeholder="模型 ID" value={id} onChange={(e) => setId(e.target.value)} />
        <Input className="h-7 text-xs" placeholder="显示名称" value={name} onChange={(e) => setName(e.target.value)} />
      </div>
      <Input className="h-7 text-xs font-mono" placeholder="API Key (可选，覆盖 Provider)" type="password" value={apiKey} onChange={(e) => setApiKey(e.target.value)} />
      <Input className="h-7 text-xs font-mono" placeholder="Base URL (可选，覆盖 Provider)" value={baseUrl} onChange={(e) => setBaseUrl(e.target.value)} />
      <div className="grid grid-cols-2 gap-2">
        <div><label className="text-[10px] text-muted-foreground">上下文窗口</label><Input className="h-7 text-xs font-mono" value={context} onChange={(e) => setContext(e.target.value)} /></div>
        <div><label className="text-[10px] text-muted-foreground">最大输出</label><Input className="h-7 text-xs font-mono" value={output} onChange={(e) => setOutput(e.target.value)} /></div>
      </div>
      <div className="flex justify-end gap-2">
        <Button variant="ghost" size="sm" className="h-6 text-[11px]" onClick={onCancel}>取消</Button>
        <Button size="sm" className="h-6 text-[11px]" disabled={!id.trim()} onClick={() => onAdd({
          id: id.trim(), name: name.trim() || id.trim(), apiKey: apiKey || undefined, baseUrl: baseUrl || undefined,
          toolCall: true, temperature: true, modalities: { input: ["text"], output: ["text"] },
          limit: { context: Number(context) || 128000, output: Number(output) || 4096 },
        })}>
          <Plus className="size-3 mr-1" />添加
        </Button>
      </div>
    </div>
  );
}
function ProviderCard({ provider, isDefault, defaultModel, onSetDefault, onRemove }: {
  provider: ProviderConfig; isDefault: boolean; defaultModel: string;
  onSetDefault: (p: string, m: string) => void; onRemove: (n: string) => void;
}) {
  const [showKey, setShowKey] = useState(false);
  const [addingModel, setAddingModel] = useState(false);
  const [editingKey, setEditingKey] = useState(false);
  const [editingUrl, setEditingUrl] = useState(false);
  const [keyDraft, setKeyDraft] = useState(provider.apiKey || "");
  const [urlDraft, setUrlDraft] = useState(provider.baseUrl || "");
  const modal = useModal();

  return (
    <div className={cn("rounded-xl border bg-card transition-all", isDefault && "ring-2 ring-primary/30")}>
      <div className="flex items-center gap-3 px-4 py-3 border-b">
        <span className={cn("inline-flex items-center rounded-md border px-2 py-0.5 text-[10px] font-bold uppercase", pColor(provider.name))}>{provider.name}</span>
        {isDefault && <span className="inline-flex items-center gap-1 text-[10px] text-primary font-medium"><Star className="size-3 fill-primary" />默认</span>}
        <span className="text-[10px] text-muted-foreground">{provider.models.length} 个模型</span>
        <div className="flex-1" />
        <Button variant="ghost" size="sm" className="h-6 text-[10px] text-destructive hover:text-destructive" onClick={() => {
          modal.alert({ title: `删除 ${provider.name}`, description: `确认删除 "${provider.name}" 及其所有模型？`, confirmText: "删除",
            onConfirm: () => { onRemove(provider.name); toast.success(`已删除 ${provider.name}`); } });
        }}><Trash2 className="size-3" /></Button>
      </div>

      <div className="px-4 py-3 space-y-2 border-b bg-muted/20">
        <div className="flex items-center gap-2">
          <KeyRound className="size-3 text-muted-foreground shrink-0" />
          {editingKey ? (
            <div className="flex-1 flex items-center gap-1.5">
              <Input className="h-6 text-[11px] font-mono flex-1" type={showKey ? "text" : "password"} value={keyDraft} onChange={(e) => setKeyDraft(e.target.value)} placeholder="API Key" />
              <button type="button" onClick={() => setShowKey(!showKey)} className="text-muted-foreground hover:text-foreground">{showKey ? <EyeOff className="size-3" /> : <Eye className="size-3" />}</button>
              <Button size="sm" className="h-6 text-[10px] px-2" onClick={() => { settingsModel.updateProvider(provider.name, { apiKey: keyDraft }); setEditingKey(false); toast.success("API Key 已更新"); }}><Check className="size-3" /></Button>
              <button type="button" onClick={() => { setEditingKey(false); setKeyDraft(provider.apiKey || ""); }} className="text-muted-foreground hover:text-foreground"><X className="size-3" /></button>
            </div>
          ) : (
            <button type="button" className="text-[11px] font-mono text-muted-foreground hover:text-foreground" onClick={() => { setEditingKey(true); setKeyDraft(provider.apiKey || ""); }}>
              {provider.apiKey ? `${provider.apiKey.slice(0, 8)}${"•".repeat(12)}` : "点击设置 API Key"}
            </button>
          )}
        </div>
        <div className="flex items-center gap-2">
          <Server className="size-3 text-muted-foreground shrink-0" />
          {editingUrl ? (
            <div className="flex-1 flex items-center gap-1.5">
              <Input className="h-6 text-[11px] font-mono flex-1" value={urlDraft} onChange={(e) => setUrlDraft(e.target.value)} placeholder="Base URL" />
              <Button size="sm" className="h-6 text-[10px] px-2" onClick={() => { settingsModel.updateProvider(provider.name, { baseUrl: urlDraft }); setEditingUrl(false); toast.success("Base URL 已更新"); }}><Check className="size-3" /></Button>
              <button type="button" onClick={() => { setEditingUrl(false); setUrlDraft(provider.baseUrl || ""); }} className="text-muted-foreground hover:text-foreground"><X className="size-3" /></button>
            </div>
          ) : (
            <button type="button" className="text-[11px] font-mono text-muted-foreground hover:text-foreground" onClick={() => { setEditingUrl(true); setUrlDraft(provider.baseUrl || ""); }}>
              {provider.baseUrl || "点击设置 Base URL"}
            </button>
          )}
        </div>
      </div>

      <div className="px-4 py-3 space-y-1.5">
        {provider.models.map((m) => {
          const isDef = isDefault && defaultModel === m.id;
          return (
            <div key={m.id} className={cn("flex items-center gap-2.5 rounded-lg border px-3 py-2 transition-all group", isDef ? "border-primary/40 bg-primary/5" : "hover:border-primary/20")}>
              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                  <span className="text-xs font-medium truncate">{m.name}</span>
                  {isDef && <Star className="size-3 text-primary fill-primary shrink-0" />}
                </div>
                <div className="flex items-center gap-2 mt-0.5">
                  <span className="text-[10px] font-mono text-muted-foreground">{m.id}</span>
                  {m.limit && <span className="text-[9px] text-muted-foreground">{(m.limit.context / 1000).toFixed(0)}K ctx</span>}
                  {m.apiKey && <span className="text-[9px] text-muted-foreground/60 italic">自定义 Key</span>}
                  {m.baseUrl && <span className="text-[9px] text-muted-foreground/60 italic">自定义 URL</span>}
                </div>
              </div>
              <div className="flex items-center gap-1 opacity-0 group-hover:opacity-100 transition-opacity">
                {!isDef && <Button variant="ghost" size="sm" className="h-6 text-[10px] px-2" onClick={() => onSetDefault(provider.name, m.id)}><Star className="size-3 mr-1" />设为默认</Button>}
                <button type="button" className="text-muted-foreground hover:text-destructive p-1" onClick={() => {
                  modal.alert({ title: "删除模型", description: `确认删除 "${m.name}"？`, confirmText: "删除",
                    onConfirm: () => { settingsModel.removeModel(provider.name, m.id); toast.success(`已删除 ${m.name}`); } });
                }}><Trash2 className="size-3" /></button>
              </div>
            </div>
          );
        })}
        {addingModel ? (
          <AddModelForm onAdd={(m) => { settingsModel.addModel(provider.name, m); setAddingModel(false); toast.success(`已添加 ${m.name}`); }} onCancel={() => setAddingModel(false)} />
        ) : (
          <button type="button" className="flex items-center gap-1.5 w-full rounded-lg border border-dashed px-3 py-2 text-xs text-muted-foreground hover:text-foreground hover:border-primary/30 transition-colors" onClick={() => setAddingModel(true)}>
            <Plus className="size-3.5" />添加模型
          </button>
        )}
      </div>
    </div>
  );
}
function AiSection() {
  const snap = useSnapshot(settingsModel.state);
  const [addingProvider, setAddingProvider] = useState(false);

  const handleSetDefault = (pName: string, mId: string) => { settingsModel.setDefault(pName, mId); toast.success("已设置默认模型"); };
  const handleAddProvider = (p: ProviderConfig) => {
    if (snap.providers.some((ep) => ep.name === p.name)) { toast.error(`"${p.name}" 已存在`); return; }
    settingsModel.addProvider(p); setAddingProvider(false); toast.success(`已添加 ${p.name}`);
  };

  const defProvider = snap.providers.find((p) => p.name === snap.defaultProvider);
  const defModel = defProvider?.models.find((m) => m.id === snap.defaultModel);

  return (
    <div>
      <SectionHeader icon={Bot} title="AI 服务" description="管理模型提供商、模型和默认配置。" />
      <div className="rounded-xl border bg-card p-4 mb-4">
        <div className="flex items-center gap-2 mb-3"><Star className="size-4 text-primary fill-primary" /><span className="text-sm font-semibold">默认模型</span></div>
        {defProvider && defModel ? (
          <div className="flex items-center gap-3">
            <span className={cn("inline-flex items-center rounded-md border px-2 py-0.5 text-[10px] font-bold uppercase", pColor(defProvider.name))}>{defProvider.name}</span>
            <span className="text-sm font-medium">{defModel.name}</span>
            <span className="text-[11px] font-mono text-muted-foreground">{defModel.id}</span>
          </div>
        ) : (
          <Select value={snap.defaultProvider ? `${snap.defaultProvider}::${snap.defaultModel}` : ""} onValueChange={(v) => { const [p, m] = v.split("::"); if (p && m) handleSetDefault(p, m); }}>
            <SelectTrigger className="h-8 text-sm"><SelectValue placeholder="选择默认模型" /></SelectTrigger>
            <SelectContent>
              {snap.providers.flatMap((p) => p.models.map((m) => (
                <SelectItem key={`${p.name}::${m.id}`} value={`${p.name}::${m.id}`}><span className="font-mono text-xs">{p.name} / {m.id}</span></SelectItem>
              )))}
            </SelectContent>
          </Select>
        )}
      </div>
      <div className="space-y-4">
        {snap.providers.map((p) => (
          <ProviderCard key={p.name} provider={p as ProviderConfig} isDefault={snap.defaultProvider === p.name} defaultModel={snap.defaultModel} onSetDefault={handleSetDefault} onRemove={(n) => settingsModel.removeProvider(n)} />
        ))}
        {addingProvider ? (
          <AddProviderForm onAdd={handleAddProvider} onCancel={() => setAddingProvider(false)} />
        ) : (
          <button type="button" className="flex items-center justify-center gap-2 w-full rounded-xl border-2 border-dashed px-4 py-4 text-sm text-muted-foreground hover:text-foreground hover:border-primary/30 transition-colors" onClick={() => setAddingProvider(true)}>
            <Plus className="size-4" />添加 Provider
          </button>
        )}
      </div>
    </div>
  );
}

function GatewaySection() {
  const snap = useSnapshot(settingsModel.state);
  const [baseUrl, setBaseUrl] = useState(snap.baseUrl);
  const [dirty, setDirty] = useState(false);
  const [saved, setSaved] = useState(false);
  return (
    <div>
      <SectionHeader icon={Server} title="网关连接" description="配置 SafeClaw 网关的连接地址。" />
      <div className="rounded-xl border bg-card p-5">
        <SettingRow label="网关地址" hint="API 和 WebSocket 连接的服务端地址，留空使用默认值。">
          <div className="relative">
            <Server className="absolute left-2.5 top-1/2 -translate-y-1/2 size-3.5 text-muted-foreground pointer-events-none" />
            <Input className="h-9 text-sm font-mono pl-8" placeholder="http://127.0.0.1:18790" value={baseUrl} onChange={(e) => { setBaseUrl(e.target.value); setDirty(true); setSaved(false); }} />
          </div>
        </SettingRow>
        <div className="mt-4 flex items-center gap-2 text-xs">
          <span className="relative flex size-2"><span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-primary opacity-75" /><span className="relative inline-flex rounded-full size-2 bg-primary" /></span>
          <span className="text-muted-foreground">已连接</span><span className="text-muted-foreground/50">·</span>
          <span className="font-mono text-muted-foreground">{baseUrl || "http://127.0.0.1:18790"}</span>
        </div>
      </div>
      {(dirty || saved) && (
        <div className={cn("flex items-center gap-3 rounded-lg px-4 py-2.5 mt-6", dirty ? "bg-primary/5 border border-primary/20" : "bg-muted/50 border border-border")}>
          {dirty ? (<><div className="flex-1 text-xs text-muted-foreground">有未保存的更改</div><Button size="sm" className="h-7 text-xs" onClick={() => { settingsModel.setBaseUrl(baseUrl); setDirty(false); setSaved(true); toast.success("网关设置已保存"); setTimeout(() => setSaved(false), 2000); }}><Check className="size-3 mr-1" />保存</Button></>) : (<><Check className="size-3.5 text-primary" /><span className="text-xs text-primary font-medium">已保存</span></>)}
        </div>
      )}
    </div>
  );
}

const INFO_ITEMS = [
  { label: "应用名称", value: "SafeClaw" }, { label: "版本", value: "0.1.0" },
  { label: "运行时", value: "Tauri v2 + React 19" }, { label: "TEE 支持", value: "Intel SGX / TDX" },
  { label: "许可证", value: "Apache-2.0" },
];

function AboutSection() {
  const modal = useModal();
  return (
    <div>
      <SectionHeader icon={Info} title="关于" description="应用信息与数据管理。" />
      <div className="rounded-xl border bg-card p-5 mb-4">
        <div className="flex items-center gap-3 mb-4">
          <div className="flex items-center justify-center size-12 rounded-xl bg-primary/10"><ShieldCheck className="size-6 text-primary" /></div>
          <div><div className="text-base font-bold">SafeClaw</div><div className="text-xs text-muted-foreground">Secure Personal AI Assistant with TEE Support</div></div>
        </div>
        <div className="rounded-lg bg-muted/30 divide-y divide-border/50">
          {INFO_ITEMS.map((item) => (<div key={item.label} className="flex justify-between items-center px-4 py-2.5"><span className="text-xs text-muted-foreground">{item.label}</span><span className="text-xs font-medium font-mono">{item.value}</span></div>))}
        </div>
      </div>
      <div className="rounded-xl border bg-card p-5 mb-4">
        <div className="flex items-center gap-2 mb-3"><Layers className="size-4 text-primary" /><span className="text-sm font-semibold">技术栈</span></div>
        <div className="flex flex-wrap gap-1.5">
          {["Rust", "Tauri v2", "React 19", "TypeScript", "Tailwind CSS", "Valtio", "gRPC", "Intel SGX", "RA-TLS"].map((t) => (<span key={t} className="inline-flex items-center rounded-md border bg-muted/50 px-2 py-0.5 text-[11px] font-medium text-muted-foreground">{t}</span>))}
        </div>
      </div>
      <div className="rounded-xl border border-destructive/20 bg-destructive/[0.03] p-5">
        <div className="flex items-center gap-2 mb-2"><RotateCcw className="size-4 text-destructive" /><span className="text-sm font-semibold text-destructive">危险操作</span></div>
        <p className="text-xs text-muted-foreground mb-3">重置后所有配置将恢复为默认值，包括所有 Provider、模型和网关地址。</p>
        <Button variant="destructive" size="sm" onClick={() => { modal.alert({ title: "重置设置", description: "确认重置所有设置为默认值？此操作不可撤销。", confirmText: "重置", onConfirm: () => { settingsModel.resetSettings(); toast.success("设置已重置"); setTimeout(() => window.location.reload(), 500); } }); }}>
          <RotateCcw className="size-3.5 mr-1.5" />重置所有设置
        </Button>
      </div>
    </div>
  );
}

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
