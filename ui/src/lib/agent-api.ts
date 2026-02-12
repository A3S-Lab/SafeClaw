import { getGatewayUrl } from "@/models/settings.model";

function baseUrl() {
  return `${getGatewayUrl()}/api/agent`;
}

const jsonHeaders = { "Content-Type": "application/json" };

export const agentApi = {
  createSession: (params: {
    model?: string;
    permission_mode?: string;
    cwd?: string;
    base_url?: string;
    api_key?: string;
    system_prompt?: string;
    skills?: string[];
  }) =>
    fetch(`${baseUrl()}/sessions`, {
      method: "POST",
      headers: jsonHeaders,
      body: JSON.stringify(params),
    }).then((r) => r.json()),

  listSessions: () => fetch(`${baseUrl()}/sessions`).then((r) => r.json()),

  getSession: (id: string) =>
    fetch(`${baseUrl()}/sessions/${id}`).then((r) => r.json()),

  updateSession: (id: string, updates: { name?: string; archived?: boolean }) =>
    fetch(`${baseUrl()}/sessions/${id}`, {
      method: "PATCH",
      headers: jsonHeaders,
      body: JSON.stringify(updates),
    }).then((r) => r.json()),

  deleteSession: (id: string) =>
    fetch(`${baseUrl()}/sessions/${id}`, { method: "DELETE" }),

  relaunchSession: (id: string) =>
    fetch(`${baseUrl()}/sessions/${id}/relaunch`, { method: "POST" }).then((r) =>
      r.json(),
    ),

  listBackends: () => fetch(`${baseUrl()}/backends`).then((r) => r.json()),
};
