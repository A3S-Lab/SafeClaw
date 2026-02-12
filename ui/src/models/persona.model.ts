import { proxy } from "valtio";
import { BUILTIN_PERSONAS, DEFAULT_PERSONA_ID, getPersonaById } from "@/lib/builtin-personas";
import type { AgentPersona } from "@/typings/persona";

const STORAGE_KEY = "safeclaw-session-personas";

interface PersonaStoreState {
  /** Maps session_id → persona_id */
  sessionPersonas: Record<string, string>;
  /** Custom (user-created) personas */
  customPersonas: AgentPersona[];
}

const state = proxy<PersonaStoreState>({
  sessionPersonas: JSON.parse(localStorage.getItem(STORAGE_KEY) || "{}"),
  customPersonas: [],
});

function persistSessionPersonas() {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(state.sessionPersonas));
}

const actions = {
  /** Assign a persona to a session */
  setSessionPersona(sessionId: string, personaId: string) {
    state.sessionPersonas[sessionId] = personaId;
    persistSessionPersonas();
  },

  /** Remove persona mapping when session is deleted */
  removeSessionPersona(sessionId: string) {
    delete state.sessionPersonas[sessionId];
    persistSessionPersonas();
  },

  /** Get the persona for a session, falling back to default */
  getSessionPersona(sessionId: string): AgentPersona {
    const personaId = state.sessionPersonas[sessionId] || DEFAULT_PERSONA_ID;
    return (
      getPersonaById(personaId) ??
      state.customPersonas.find((p) => p.id === personaId) ??
      BUILTIN_PERSONAS[0]
    );
  },

  /** Get all available personas (builtin + custom) */
  getAllPersonas(): AgentPersona[] {
    return [...BUILTIN_PERSONAS, ...state.customPersonas];
  },
};

export default { state, ...actions };
