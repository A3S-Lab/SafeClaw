import agentModel from "@/models/agent.model";
import { getGatewayUrl } from "@/models/settings.model";
import type {
  AgentChatMessage,
  BrowserIncomingMessage,
  BrowserOutgoingMessage,
} from "@/typings/agent";

// Module-level state (outside Valtio to avoid proxy overhead)
const sockets = new Map<string, WebSocket>();
const reconnectTimers = new Map<string, ReturnType<typeof setTimeout>>();

function getWsUrl(sessionId: string): string {
  const base = getGatewayUrl().replace(/^http/, "ws");
  return `${base}/ws/agent/browser/${sessionId}`;
}

/** Connect a WebSocket for the given session */
export function connectSession(sessionId: string): void {
  if (sockets.has(sessionId)) return;

  agentModel.setConnectionStatus(sessionId, "connecting");

  const ws = new WebSocket(getWsUrl(sessionId));
  sockets.set(sessionId, ws);

  ws.onopen = () => {
    agentModel.setConnectionStatus(sessionId, "connected");
    const timer = reconnectTimers.get(sessionId);
    if (timer) {
      clearTimeout(timer);
      reconnectTimers.delete(sessionId);
    }
  };

  ws.onmessage = (event) => {
    try {
      const data = JSON.parse(event.data) as BrowserIncomingMessage;
      handleMessage(sessionId, data);
    } catch {
      // Ignore malformed messages
    }
  };

  ws.onclose = () => {
    sockets.delete(sessionId);
    agentModel.setConnectionStatus(sessionId, "disconnected");
    scheduleReconnect(sessionId);
  };

  ws.onerror = () => {
    ws.close();
  };
}

/** Disconnect a session's WebSocket */
export function disconnectSession(sessionId: string): void {
  const timer = reconnectTimers.get(sessionId);
  if (timer) {
    clearTimeout(timer);
    reconnectTimers.delete(sessionId);
  }
  const ws = sockets.get(sessionId);
  if (ws) {
    ws.close();
    sockets.delete(sessionId);
  }
  agentModel.setConnectionStatus(sessionId, "disconnected");
}

/** Disconnect all sessions */
export function disconnectAll(): void {
  for (const sessionId of sockets.keys()) {
    disconnectSession(sessionId);
  }
}

/** Send a message to a session's WebSocket */
export function sendToSession(
  sessionId: string,
  msg: BrowserOutgoingMessage,
): void {
  const ws = sockets.get(sessionId);
  if (ws?.readyState === WebSocket.OPEN) {
    ws.send(JSON.stringify(msg));

    // Track user messages locally
    if (msg.type === "user_message") {
      agentModel.appendMessage(sessionId, {
        id: Date.now().toString(),
        role: "user",
        content: msg.content,
        images: msg.images,
        timestamp: Date.now(),
      });
      agentModel.setSessionStatus(sessionId, "running");
    }
  }
}

function scheduleReconnect(sessionId: string): void {
  if (reconnectTimers.has(sessionId)) return;

  const timer = setTimeout(() => {
    reconnectTimers.delete(sessionId);
    // Only reconnect if session still exists
    if (agentModel.state.sessions[sessionId]) {
      connectSession(sessionId);
    }
  }, 2000);

  reconnectTimers.set(sessionId, timer);
}

function handleMessage(
  sessionId: string,
  msg: BrowserIncomingMessage,
): void {
  switch (msg.type) {
    case "session_init":
      agentModel.addSession(msg.session);
      agentModel.setCliConnected(sessionId, true);
      agentModel.setSessionStatus(sessionId, "idle");
      break;

    case "session_update":
      agentModel.updateSession(sessionId, msg.session);
      break;

    case "assistant": {
      const textParts = msg.message.content
        .filter((b) => b.type === "text")
        .map((b) => (b as { type: "text"; text: string }).text)
        .join("");

      const chatMsg: AgentChatMessage = {
        id: msg.message.id,
        role: "assistant",
        content: textParts,
        contentBlocks: msg.message.content,
        timestamp: Date.now(),
        parentToolUseId: msg.parent_tool_use_id,
        model: msg.message.model,
        stopReason: msg.message.stop_reason,
      };
      agentModel.appendMessage(sessionId, chatMsg);
      agentModel.setStreaming(sessionId, null);
      break;
    }

    case "stream_event": {
      const event = msg.event as Record<string, unknown>;
      const eventType = event.type as string;

      if (eventType === "message_start") {
        agentModel.setStreamingStartedAt(sessionId, Date.now());
        agentModel.setStreaming(sessionId, "");
        agentModel.setSessionStatus(sessionId, "running");
      } else if (eventType === "content_block_delta") {
        const delta = event.delta as Record<string, unknown> | undefined;
        if (delta?.type === "text_delta" && typeof delta.text === "string") {
          const current = agentModel.state.streaming[sessionId] || "";
          agentModel.setStreaming(sessionId, current + delta.text);
        }
      }
      break;
    }

    case "result": {
      const data = msg.data;
      agentModel.updateSession(sessionId, {
        total_cost_usd: data.total_cost_usd as number | undefined,
        num_turns: data.num_turns as number | undefined,
        total_lines_added: data.total_lines_added as number | undefined,
        total_lines_removed: data.total_lines_removed as number | undefined,
        context_used_percent: data.context_used_percent as number | undefined,
      } as Partial<import("@/typings/agent").AgentSessionState>);
      agentModel.setStreaming(sessionId, null);
      agentModel.setSessionStatus(sessionId, "idle");

      if (data.is_error) {
        agentModel.appendMessage(sessionId, {
          id: Date.now().toString(),
          role: "system",
          content: (data.result as string) || "An error occurred",
          timestamp: Date.now(),
        });
      }
      break;
    }

    case "permission_request":
      agentModel.addPermission(sessionId, msg.request);
      break;

    case "permission_cancelled":
      agentModel.removePermission(sessionId, msg.request_id);
      break;

    case "status_change":
      if (msg.status === "compacting") {
        agentModel.setSessionStatus(sessionId, "compacting");
        agentModel.updateSession(sessionId, { is_compacting: true });
      } else {
        agentModel.setSessionStatus(sessionId, "idle");
        agentModel.updateSession(sessionId, { is_compacting: false });
      }
      break;

    case "error":
      agentModel.appendMessage(sessionId, {
        id: Date.now().toString(),
        role: "system",
        content: msg.message,
        timestamp: Date.now(),
      });
      break;

    case "cli_connected":
      agentModel.setCliConnected(sessionId, true);
      break;

    case "cli_disconnected":
      agentModel.setCliConnected(sessionId, false);
      agentModel.setSessionStatus(sessionId, null);
      break;

    case "user_message":
      // Echo from server (for history replay)
      agentModel.appendMessage(sessionId, {
        id: Date.now().toString(),
        role: "user",
        content: msg.content,
        timestamp: msg.timestamp,
      });
      break;

    case "message_history": {
      const chatMessages = convertHistoryMessages(msg.messages);
      const existing = agentModel.state.messages[sessionId] || [];
      if (chatMessages.length >= existing.length) {
        agentModel.setMessages(sessionId, chatMessages);
      }
      break;
    }

    case "session_name_update": {
      const currentName = agentModel.state.sessionNames[sessionId];
      if (!currentName) {
        agentModel.setSessionName(sessionId, msg.name);
      }
      break;
    }

    default:
      break;
  }
}

/** Convert message history from server format to chat messages */
function convertHistoryMessages(
  messages: BrowserIncomingMessage[],
): AgentChatMessage[] {
  const result: AgentChatMessage[] = [];

  for (const msg of messages) {
    if (msg.type === "assistant") {
      const textParts = msg.message.content
        .filter((b) => b.type === "text")
        .map((b) => (b as { type: "text"; text: string }).text)
        .join("");

      result.push({
        id: msg.message.id,
        role: "assistant",
        content: textParts,
        contentBlocks: msg.message.content,
        timestamp: Date.now(),
        parentToolUseId: msg.parent_tool_use_id,
        model: msg.message.model,
        stopReason: msg.message.stop_reason,
      });
    } else if (msg.type === "result" && msg.data.is_error) {
      result.push({
        id: Date.now().toString(),
        role: "system",
        content: (msg.data.result as string) || "An error occurred",
        timestamp: Date.now(),
      });
    } else if (msg.type === "user_message") {
      result.push({
        id: Date.now().toString(),
        role: "user",
        content: msg.content,
        timestamp: msg.timestamp,
      });
    }
  }

  return result;
}
