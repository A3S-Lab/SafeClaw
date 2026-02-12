/** Detect Tauri environment */
const IS_TAURI =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

/**
 * In Tauri: use the native HTTP plugin (bypasses CORS).
 * In browser dev mode: use rsbuild dev proxy at /api/polymarket.
 */
const GAMMA_API = IS_TAURI
  ? "https://gamma-api.polymarket.com"
  : "/api/polymarket";

async function httpFetch(url: string): Promise<Response> {
  if (IS_TAURI) {
    const { fetch: tauriFetch } = await import("@tauri-apps/plugin-http");
    return tauriFetch(url, { method: "GET" });
  }
  return fetch(url);
}

export interface PolymarketEvent {
  id: string;
  title: string;
  slug: string;
  description: string;
  active: boolean;
  closed: boolean;
  volume: number;
  volume24hr: number;
  liquidity: number;
  startDate: string;
  endDate: string;
  image: string;
  markets: PolymarketMarket[];
}

export interface PolymarketMarket {
  id: string;
  question: string;
  slug: string;
  outcomePrices: string; // JSON string: '["0.65", "0.35"]'
  outcomes: string; // JSON string: '["Yes", "No"]'
  volume: string;
  volume24hr: number;
  active: boolean;
  closed: boolean;
  lastTradePrice: number;
  oneDayPriceChange: number;
  bestAsk: number;
}

export interface ParsedMarket {
  id: string;
  question: string;
  slug: string;
  yesPrice: number;
  noPrice: number;
  volume24hr: number;
  lastTradePrice: number;
  oneDayPriceChange: number;
  active: boolean;
  closed: boolean;
}

export interface ParsedEvent {
  id: string;
  title: string;
  slug: string;
  description: string;
  active: boolean;
  closed: boolean;
  volume: number;
  volume24hr: number;
  liquidity: number;
  endDate: string;
  image: string;
  markets: ParsedMarket[];
}

function parseMarket(m: PolymarketMarket): ParsedMarket {
  let yesPrice = 0;
  let noPrice = 0;
  try {
    const prices = JSON.parse(m.outcomePrices) as string[];
    yesPrice = Number.parseFloat(prices[0]) || 0;
    noPrice = Number.parseFloat(prices[1]) || 0;
  } catch {
    // fallback
  }
  return {
    id: m.id,
    question: m.question,
    slug: m.slug,
    yesPrice,
    noPrice,
    volume24hr: m.volume24hr || 0,
    lastTradePrice: m.lastTradePrice || 0,
    oneDayPriceChange: m.oneDayPriceChange || 0,
    active: m.active,
    closed: m.closed,
  };
}

export function parseEvent(e: PolymarketEvent): ParsedEvent {
  return {
    id: e.id,
    title: e.title,
    slug: e.slug,
    description: e.description,
    active: e.active,
    closed: e.closed,
    volume: e.volume,
    volume24hr: e.volume24hr,
    liquidity: e.liquidity,
    endDate: e.endDate,
    image: e.image,
    markets: (e.markets || [])
      .filter((m) => m.active && !m.closed)
      .map(parseMarket),
  };
}

/** Fetch trending events sorted by 24h volume */
export async function fetchTrendingEvents(limit = 10): Promise<ParsedEvent[]> {
  const url = `${GAMMA_API}/events?limit=${limit}&active=true&order=volume24hr&ascending=false`;
  const res = await httpFetch(url);
  if (!res.ok) throw new Error(`Polymarket API error: ${res.status}`);
  const data: PolymarketEvent[] = await res.json();
  return data.map(parseEvent);
}

/** Search events by keyword */
export async function searchEvents(query: string, limit = 10): Promise<ParsedEvent[]> {
  const url = `${GAMMA_API}/events?limit=${limit}&active=true&title=${encodeURIComponent(query)}`;
  const res = await httpFetch(url);
  if (!res.ok) throw new Error(`Polymarket API error: ${res.status}`);
  const data: PolymarketEvent[] = await res.json();
  return data.map(parseEvent);
}

/** Format volume as readable string */
export function formatVolume(vol: number): string {
  if (vol >= 1_000_000) return `$${(vol / 1_000_000).toFixed(1)}M`;
  if (vol >= 1_000) return `$${(vol / 1_000).toFixed(1)}K`;
  return `$${vol.toFixed(0)}`;
}

/** Format price as percentage */
export function formatProbability(price: number): string {
  return `${(price * 100).toFixed(1)}%`;
}
