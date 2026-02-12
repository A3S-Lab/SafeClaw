import constants from "@/constants";
import { proxy, subscribe } from "valtio";

// =============================================================================
// Types
// =============================================================================

export interface ModelCost {
  input: number;
  output: number;
  cacheRead?: number;
  cacheWrite?: number;
}

export interface ModelLimit {
  context: number;
  output: number;
}

export interface ModelModalities {
  input: string[];
  output: string[];
}

export interface ModelConfig {
  id: string;
  name: string;
  family?: string;
  /** Per-model override (e.g. proxy for a specific model) */
  apiKey?: string;
  baseUrl?: string;
  attachment?: boolean;
  reasoning?: boolean;
  toolCall?: boolean;
  temperature?: boolean;
  releaseDate?: string;
  modalities?: ModelModalities;
  cost?: ModelCost;
  limit?: ModelLimit;
}

export interface ProviderConfig {
  name: string;
  apiKey?: string;
  baseUrl?: string;
  models: ModelConfig[];
}

export interface SettingsState {
  defaultProvider: string;
  defaultModel: string;
  providers: ProviderConfig[];
  /** Gateway base URL (empty = use default) */
  baseUrl: string;
}

// =============================================================================
// Persistence
// =============================================================================

const STORAGE_KEY = "safeclaw-settings";

const DEFAULTS: SettingsState = {
  defaultProvider: "anthropic",
  defaultModel: "claude-sonnet-4-20250514",
  providers: [
    {
      name: "anthropic",
      apiKey: "",
      baseUrl: "",
      models: [
        {
          id: "claude-sonnet-4-20250514",
          name: "Claude Sonnet 4 (20250514)",
          family: "claude-sonnet",
          attachment: true,
          reasoning: false,
          toolCall: true,
          temperature: true,
          releaseDate: "2025-05-14",
          modalities: { input: ["text", "image", "pdf"], output: ["text"] },
          cost: { input: 3, output: 15, cacheRead: 0.3, cacheWrite: 3.75 },
          limit: { context: 200000, output: 64000 },
        },
      ],
    },
  ],
  baseUrl: "",
};

function loadSettings(): SettingsState {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw);
      // Migrate from old flat format
      if ("provider" in parsed && !("providers" in parsed)) {
        return {
          ...DEFAULTS,
          defaultProvider: parsed.provider || DEFAULTS.defaultProvider,
          defaultModel: parsed.model || DEFAULTS.defaultModel,
          baseUrl: parsed.baseUrl || "",
          providers: DEFAULTS.providers.map((p) =>
            p.name === (parsed.provider || "anthropic")
              ? { ...p, apiKey: parsed.apiKey || "" }
              : p,
          ),
        };
      }
      return { ...DEFAULTS, ...parsed };
    }
  } catch {
    // Ignore parse errors
  }
  return DEFAULTS;
}

const state = proxy<SettingsState>(loadSettings());

subscribe(state, () => {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
  } catch {
    // Storage unavailable
  }
});

// =============================================================================
// Actions
// =============================================================================

function setDefault(provider: string, model: string) {
  state.defaultProvider = provider;
  state.defaultModel = model;
}

function setBaseUrl(url: string) {
  state.baseUrl = url;
}

function addProvider(provider: ProviderConfig) {
  state.providers.push(provider);
}

function updateProvider(name: string, patch: Partial<Omit<ProviderConfig, "name">>) {
  const p = state.providers.find((p) => p.name === name);
  if (p) Object.assign(p, patch);
}

function removeProvider(name: string) {
  const idx = state.providers.findIndex((p) => p.name === name);
  if (idx >= 0) state.providers.splice(idx, 1);
  // Reset default if removed
  if (state.defaultProvider === name) {
    const first = state.providers[0];
    state.defaultProvider = first?.name || "";
    state.defaultModel = first?.models[0]?.id || "";
  }
}

function addModel(providerName: string, model: ModelConfig) {
  const p = state.providers.find((p) => p.name === providerName);
  if (p) p.models.push(model);
}

function updateModel(providerName: string, modelId: string, patch: Partial<ModelConfig>) {
  const p = state.providers.find((p) => p.name === providerName);
  if (!p) return;
  const m = p.models.find((m) => m.id === modelId);
  if (m) Object.assign(m, patch);
}

function removeModel(providerName: string, modelId: string) {
  const p = state.providers.find((p) => p.name === providerName);
  if (!p) return;
  const idx = p.models.findIndex((m) => m.id === modelId);
  if (idx >= 0) p.models.splice(idx, 1);
  // Reset default if removed
  if (state.defaultProvider === providerName && state.defaultModel === modelId) {
    state.defaultModel = p.models[0]?.id || "";
  }
}

function resetSettings() {
  Object.assign(state, structuredClone(DEFAULTS));
}

// =============================================================================
// Helpers
// =============================================================================

/** Resolve the effective API key for a model (model-level > provider-level) */
export function resolveApiKey(providerName: string, modelId: string): string {
  const p = state.providers.find((p) => p.name === providerName);
  if (!p) return "";
  const m = p.models.find((m) => m.id === modelId);
  return m?.apiKey || p.apiKey || "";
}

/** Resolve the effective base URL for a model (model-level > provider-level) */
export function resolveBaseUrl(providerName: string, modelId: string): string {
  const p = state.providers.find((p) => p.name === providerName);
  if (!p) return "";
  const m = p.models.find((m) => m.id === modelId);
  return m?.baseUrl || p.baseUrl || "";
}

export function getGatewayUrl(): string {
  return state.baseUrl || constants.gatewayUrl;
}

/** Get all models across all providers as flat list */
export function getAllModels(): { provider: string; model: ModelConfig }[] {
  return state.providers.flatMap((p) =>
    p.models.map((m) => ({ provider: p.name, model: m })),
  );
}

export default {
  state,
  setDefault,
  setBaseUrl,
  addProvider,
  updateProvider,
  removeProvider,
  addModel,
  updateModel,
  removeModel,
  resetSettings,
};
