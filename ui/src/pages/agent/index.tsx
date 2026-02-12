import {
  ResizableHandle,
  ResizablePanel,
  ResizablePanelGroup,
} from "@/components/ui/resizable";
import agentModel from "@/models/agent.model";
import personaModel from "@/models/persona.model";
import type { AgentProcessInfo } from "@/typings/agent";
import { useEffect, useRef } from "react";
import { useSnapshot } from "valtio";
import AgentSessionList from "./components/agent-session-list";
import AgentChat from "./components/agent-chat";
import EventNotifications from "./components/event-notifications";
import OverviewDashboard from "./components/overview-dashboard";

function EmptyState() {
  return (
    <div className="flex items-center justify-center h-full text-muted-foreground">
      <div className="text-center space-y-3 max-w-xs">
        <div className="text-4xl">🤖</div>
        <p className="text-lg font-medium text-foreground">选择智能体</p>
        <p className="text-sm">
          从左侧选择一个智能体开始对话，或点击 + 自定义创建
        </p>
      </div>
    </div>
  );
}

/** Seed mock sessions — pure static data, no API/WS calls */
function seedMockSessions() {
  const now = Date.now();
  const mockSessions: AgentProcessInfo[] = [
    { session_id: "mock-fullstack-1", state: "connected", cwd: "/workspace", created_at: now - 5 * 60_000, archived: false, name: "新会话" },
    { session_id: "mock-fullstack-2", state: "exited", cwd: "/workspace/api", created_at: now - 3 * 3600_000, archived: false, name: "API 接口设计" },
    { session_id: "mock-quant-1", state: "running", cwd: "/workspace/strategy", created_at: now - 15 * 60_000, archived: false, name: "因子回测分析" },
    { session_id: "mock-risk-1", state: "connected", cwd: "/workspace/risk", created_at: now - 45 * 60_000, archived: false, name: "信用风险模型" },
    { session_id: "mock-devops-1", state: "exited", cwd: "/workspace/infra", created_at: now - 2 * 86400_000, archived: false, name: "K8s 集群部署" },
    { session_id: "mock-data-eng-1", state: "connected", cwd: "/workspace/pipeline", created_at: now - 30 * 60_000, archived: false, name: "实时数据管道" },
    { session_id: "mock-product-1", state: "exited", cwd: "/workspace/docs", created_at: now - 6 * 3600_000, archived: false, name: "支付产品 PRD" },
    { session_id: "mock-finance-1", state: "connected", cwd: "/workspace/finance", created_at: now - 20 * 60_000, archived: false, name: "供应商付款审批" },
  ];

  const personaMap: Record<string, string> = {
    "mock-fullstack-1": "fullstack-engineer",
    "mock-fullstack-2": "fullstack-engineer",
    "mock-quant-1": "quant-researcher",
    "mock-risk-1": "risk-analyst",
    "mock-devops-1": "devops-engineer",
    "mock-data-eng-1": "data-engineer",
    "mock-product-1": "product-manager",
    "mock-finance-1": "financial-analyst",
  };

  for (const [sid, pid] of Object.entries(personaMap)) {
    personaModel.setSessionPersona(sid, pid);
  }
  for (const s of mockSessions) {
    if (s.name) agentModel.setSessionName(s.session_id, s.name);
  }
  agentModel.setSdkSessions(mockSessions);
}

export default function AgentPage() {
  const { currentSessionId } = useSnapshot(agentModel.state);
  const seeded = useRef(false);

  // Seed mock data once on mount — no API/WS calls
  useEffect(() => {
    if (!seeded.current) {
      seeded.current = true;
      seedMockSessions();
    }
  }, []);

  return (
    <>
      <ResizablePanelGroup direction="horizontal" className="h-full w-full">
        <ResizablePanel defaultSize={22} minSize={15} maxSize={40}>
          <AgentSessionList />
        </ResizablePanel>
        <ResizableHandle withHandle />
        <ResizablePanel defaultSize={78} minSize={50}>
          {currentSessionId === "__overview__" ? (
            <OverviewDashboard />
          ) : currentSessionId ? (
            <AgentChat key={currentSessionId} sessionId={currentSessionId} />
          ) : (
            <EmptyState />
          )}
        </ResizablePanel>
      </ResizablePanelGroup>
      <EventNotifications />
    </>
  );
}
