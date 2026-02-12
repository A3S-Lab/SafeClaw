import constants from "@/constants";
import { proxy, subscribe } from "valtio";

interface SettingsState {
  provider: string;
  model: string;
  baseUrl: string;
  apiKey: string;
}

const STORAGE_KEY = "safeclaw-settings";

function loadSettings(): SettingsState {
  const defaults: SettingsState = {
    provider: "anthropic",
    model: "claude-sonnet-4-20250514",
    baseUrl: "",
    apiKey: "",
  };
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) {
      return { ...defaults, ...JSON.parse(raw) };
    }
  } catch {
    // Ignore parse errors
  }
  return defaults;
}

const state = proxy<SettingsState>(loadSettings());

subscribe(state, () => {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(state));
  } catch {
    // Storage unavailable
  }
});

function updateSettings(partial: Partial<SettingsState>) {
  Object.assign(state, partial);
}

function resetSettings() {
  state.provider = "anthropic";
  state.model = "claude-sonnet-4-20250514";
  state.baseUrl = "";
  state.apiKey = "";
}

export function getGatewayUrl(): string {
  return state.baseUrl || constants.gatewayUrl;
}

export default { state, updateSettings, resetSettings };
