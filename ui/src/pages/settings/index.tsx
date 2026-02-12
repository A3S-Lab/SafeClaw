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
  Bot,
  Check,
  ChevronRight,
  Eye,
  EyeOff,
  Info,
  KeyRound,
  Layers,
  Plus,
  RotateCcw,
  Server,
  ShieldCheck,
  Sparkles,
  Star,
  Trash2,
  X,
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
