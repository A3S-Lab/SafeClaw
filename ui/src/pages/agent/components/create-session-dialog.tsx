import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Textarea } from "@/components/ui/textarea";
import { ScrollArea } from "@/components/ui/scroll-area";
import { agentApi } from "@/lib/agent-api";
import settingsModel from "@/models/settings.model";
import { ChevronDown, Shuffle } from "lucide-react";
import Avatar, { genConfig } from "react-nice-avatar";
import type { AvatarFullConfig } from "react-nice-avatar";
import { useCallback, useMemo, useState } from "react";

interface CreateSessionDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onCreated: (sessionId: string) => void;
  /** Pre-filled from builtin agent card */
  defaults?: {
    personaId: string;
    sessionName: string;
    systemPrompt: string;
    avatar: AvatarFullConfig;
    model?: string;
  };
}

export default function CreateSessionDialog({
  open,
  onOpenChange,
  onCreated,
  defaults,
}: CreateSessionDialogProps) {
  // Avatar / identity
  const [avatarConfig, setAvatarConfig] = useState<AvatarFullConfig>(
    defaults?.avatar || genConfig(),
  );
  const [sessionName, setSessionName] = useState(defaults?.sessionName || "");
  const [systemPrompt, setSystemPrompt] = useState(defaults?.systemPrompt || "");

  // Session config
  const [model, setModel] = useState(defaults?.model || settingsModel.state.model);
  const [permissionMode, setPermissionMode] = useState("default");
  const [cwd, setCwd] = useState("");

  // Advanced
  const [baseUrl, setBaseUrl] = useState(settingsModel.state.baseUrl);
  const [apiKey, setApiKey] = useState(settingsModel.state.apiKey);
  const [advancedOpen, setAdvancedOpen] = useState(false);

  // State
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const currentAvatarConfig = useMemo(() => genConfig(avatarConfig), [avatarConfig]);

  // Sync defaults when they change (dialog re-opened with different persona)
  const [prevDefaults, setPrevDefaults] = useState(defaults);
  if (defaults !== prevDefaults) {
    setPrevDefaults(defaults);
    if (defaults) {
      setAvatarConfig(defaults.avatar);
      setSessionName(defaults.sessionName);
      setSystemPrompt(defaults.systemPrompt);
      if (defaults.model) setModel(defaults.model);
    }
  }

  const handleRandomAvatar = useCallback(() => {
    setAvatarConfig(genConfig());
  }, []);

  const resetForm = useCallback(() => {
    setAvatarConfig(genConfig());
    setSessionName("");
    setSystemPrompt("");
    setModel(settingsModel.state.model);
    setPermissionMode("default");
    setCwd("");
    setBaseUrl(settingsModel.state.baseUrl);
    setApiKey(settingsModel.state.apiKey);
    setAdvancedOpen(false);
    setError(null);
  }, []);

  const handleCreate = async () => {
    setLoading(true);
    setError(null);
    try {
      const result = await agentApi.createSession({
        model,
        permission_mode: permissionMode,
        cwd: cwd || undefined,
        base_url: baseUrl || undefined,
        api_key: apiKey || undefined,
        system_prompt: systemPrompt || undefined,
      });
      if (result.error) {
        setError(result.error);
      } else {
        const sid = result.session_id;
        const { default: personaModel } = await import("@/models/persona.model");
        personaModel.setSessionPersona(sid, defaults?.personaId || "general");
        if (sessionName) {
          const { default: agentModel } = await import("@/models/agent.model");
          agentModel.setSessionName(sid, sessionName);
        }
        onCreated(sid);
        onOpenChange(false);
        resetForm();
      }
    } catch (err) {
      setError("无法连接到网关");
    } finally {
      setLoading(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[480px] max-h-[85vh] flex flex-col">
        <DialogHeader>
          <DialogTitle>新建 Agent 会话</DialogTitle>
        </DialogHeader>

        <ScrollArea className="flex-1 -mx-6 px-6">
          <div className="grid gap-5 py-4">
            {/* === Avatar === */}
            <div className="flex items-center gap-4">
              <Avatar className="w-16 h-16 shrink-0" {...currentAvatarConfig} />
              <Button
                type="button"
                variant="outline"
                size="sm"
                onClick={handleRandomAvatar}
              >
                <Shuffle className="size-4 mr-1.5" />
                随机头像
              </Button>
            </div>

            {/* === Name === */}
            <div className="grid gap-2">
              <Label htmlFor="session-name">会话名称</Label>
              <Input
                id="session-name"
                placeholder="给会话起个名字"
                value={sessionName}
                onChange={(e) => setSessionName(e.target.value)}
              />
            </div>

            {/* === System prompt === */}
            <div className="grid gap-2">
              <Label htmlFor="system-prompt">系统提示词</Label>
              <Textarea
                id="system-prompt"
                placeholder="定义 Agent 的行为和角色..."
                value={systemPrompt}
                onChange={(e) => setSystemPrompt(e.target.value)}
                className="min-h-[80px] resize-y"
              />
            </div>

            {/* === Model === */}
            <div className="grid gap-2">
              <Label htmlFor="model">模型</Label>
              <Input
                id="model"
                placeholder="claude-sonnet-4-20250514"
                value={model}
                onChange={(e) => setModel(e.target.value)}
              />
            </div>

            {/* === Permission mode === */}
            <div className="grid gap-2">
              <Label htmlFor="permission-mode">权限模式</Label>
              <Select value={permissionMode} onValueChange={setPermissionMode}>
                <SelectTrigger id="permission-mode">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="default">默认</SelectItem>
                  <SelectItem value="plan">计划模式</SelectItem>
                  <SelectItem value="bypassPermissions">跳过权限验证</SelectItem>
                </SelectContent>
              </Select>
            </div>

            {/* === Working directory === */}
            <div className="grid gap-2">
              <Label htmlFor="cwd">工作目录</Label>
              <Input
                id="cwd"
                placeholder="默认：当前目录"
                value={cwd}
                onChange={(e) => setCwd(e.target.value)}
              />
            </div>

            {/* === Advanced === */}
            <div className="border-t pt-3">
              <button
                type="button"
                className="flex items-center gap-1.5 text-sm font-medium text-muted-foreground hover:text-foreground transition-colors"
                onClick={() => setAdvancedOpen(!advancedOpen)}
              >
                <ChevronDown
                  className={`size-4 transition-transform duration-200 ${advancedOpen ? "rotate-180" : ""}`}
                />
                高级选项（API 配置）
              </button>
              {advancedOpen && (
                <div className="grid gap-4 pt-3">
                  <div className="grid gap-2">
                    <Label htmlFor="base-url">API Base URL</Label>
                    <Input
                      id="base-url"
                      placeholder="https://api.anthropic.com"
                      value={baseUrl}
                      onChange={(e) => setBaseUrl(e.target.value)}
                    />
                    <p className="text-xs text-muted-foreground">留空则使用全局设置</p>
                  </div>
                  <div className="grid gap-2">
                    <Label htmlFor="api-key">API Key</Label>
                    <Input
                      id="api-key"
                      type="password"
                      placeholder="sk-ant-..."
                      value={apiKey}
                      onChange={(e) => setApiKey(e.target.value)}
                    />
                    <p className="text-xs text-muted-foreground">留空则使用全局设置</p>
                  </div>
                </div>
              )}
            </div>

            {error && (
              <div className="text-sm text-destructive">{error}</div>
            )}
          </div>
        </ScrollArea>

        <DialogFooter>
          <Button
            variant="outline"
            onClick={() => { onOpenChange(false); resetForm(); }}
            disabled={loading}
          >
            取消
          </Button>
          <Button onClick={handleCreate} disabled={loading}>
            {loading ? "创建中..." : "创建"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
