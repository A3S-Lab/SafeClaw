import { cn } from "@/lib/utils";
import {
	Bell,
	BookOpen,
	Boxes,
	Building2,
	Globe,
	MessageCircle,
	Settings,
} from "lucide-react";
import { ReactNode, useCallback, useEffect, useRef, KeyboardEvent } from "react";
import { useLocation, useNavigate } from "react-router-dom";
import { User } from "./user";

const STORAGE_KEY = "safeclaw-active-route";

const NAV_ITEMS = [
	{ key: "chat", label: "Chat", icon: MessageCircle, path: "/" },
	{ key: "events", label: "Events", icon: Bell, path: "/events" },
	{ key: "knowledge", label: "Knowledge", icon: BookOpen, path: "/knowledge" },
	{ key: "assets", label: "Assets", icon: Boxes, path: "/assets" },
	{ key: "systems", label: "Systems", icon: Building2, path: "/systems" },
	{ key: "marketplace", label: "Marketplace", icon: Globe, path: "/marketplace" },
] as const;

const BOTTOM_ITEMS = [
	{ key: "settings", label: "Settings", icon: Settings, path: "/settings" },
] as const;

const ALL_KEYS = [...NAV_ITEMS, ...BOTTOM_ITEMS].map((i) => i.key);

const ROUTE_MAP: Record<string, string> = Object.fromEntries(
	[...NAV_ITEMS, ...BOTTOM_ITEMS].map((i) => [i.key, i.path]),
);

function pathToKey(pathname: string): string {
	const segment = pathname.replace(/^\//, "") || "chat";
	return segment in ROUTE_MAP ? segment : "chat";
}

interface ActivityItemProps {
	isActive: boolean;
	icon: ReactNode;
	label: string;
	onClick: () => void;
	onKeyDown: (e: KeyboardEvent) => void;
	tabIndex: number;
	itemRef: (el: HTMLButtonElement | null) => void;
}

const ActivityItem = ({
	icon,
	isActive,
	label,
	onClick,
	onKeyDown,
	tabIndex,
	itemRef,
}: ActivityItemProps) => {
	return (
		<button
			ref={itemRef}
			type="button"
			role="tab"
			aria-selected={isActive}
			aria-label={label}
			tabIndex={tabIndex}
			className={cn(
				"flex flex-col justify-center items-center w-full h-12 cursor-pointer",
				"hover:text-primary-foreground focus-visible:text-primary-foreground",
				"focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-inset",
				isActive && "text-primary-foreground",
			)}
			onClick={onClick}
			onKeyDown={onKeyDown}
		>
			<div className="size-6">{icon}</div>
		</button>
	);
};

export default function ActivityBar() {
	const location = useLocation();
	const nav = useNavigate();
	const activeKey = pathToKey(location.pathname);
	const itemRefs = useRef<Map<string, HTMLButtonElement>>(new Map());

	const handleNavigate = useCallback(
		(key: string) => {
			const path = ROUTE_MAP[key] ?? "/";
			nav(path);
			try {
				localStorage.setItem(STORAGE_KEY, key);
			} catch {
				// Storage unavailable
			}
		},
		[nav],
	);

	const handleKeyDown = useCallback(
		(e: KeyboardEvent, currentKey: string) => {
			const idx = ALL_KEYS.indexOf(currentKey);
			let nextIdx = -1;

			if (e.key === "ArrowDown") {
				e.preventDefault();
				nextIdx = (idx + 1) % ALL_KEYS.length;
			} else if (e.key === "ArrowUp") {
				e.preventDefault();
				nextIdx = (idx - 1 + ALL_KEYS.length) % ALL_KEYS.length;
			} else if (e.key === "Home") {
				e.preventDefault();
				nextIdx = 0;
			} else if (e.key === "End") {
				e.preventDefault();
				nextIdx = ALL_KEYS.length - 1;
			}

			if (nextIdx >= 0) {
				const nextKey = ALL_KEYS[nextIdx];
				itemRefs.current.get(nextKey)?.focus();
			}
		},
		[],
	);

	useEffect(() => {
		if (location.pathname !== "/") return;
		try {
			const stored = localStorage.getItem(STORAGE_KEY);
			if (stored && stored !== "chat" && stored in ROUTE_MAP) {
				nav(ROUTE_MAP[stored], { replace: true });
			}
		} catch {
			// Storage unavailable
		}
	}, []); // eslint-disable-line react-hooks/exhaustive-deps

	const setRef = (key: string) => (el: HTMLButtonElement | null) => {
		if (el) itemRefs.current.set(key, el);
		else itemRefs.current.delete(key);
	};

	return (
		<nav
			aria-label="Main navigation"
			className="flex flex-col h-full w-[var(--activity-bar-width)] bg-primary text-primary-foreground/60 shadow-lg"
		>
			<User />
			<div className="flex-1" role="tablist" aria-orientation="vertical">
				{NAV_ITEMS.map((item) => (
					<ActivityItem
						key={item.key}
						icon={<item.icon className="size-6" />}
						isActive={activeKey === item.key}
						label={item.label}
						tabIndex={activeKey === item.key ? 0 : -1}
						onClick={() => handleNavigate(item.key)}
						onKeyDown={(e) => handleKeyDown(e, item.key)}
						itemRef={setRef(item.key)}
					/>
				))}
			</div>
			<div className="pb-2" role="tablist" aria-orientation="vertical">
				{BOTTOM_ITEMS.map((item) => (
					<ActivityItem
						key={item.key}
						icon={<item.icon className="size-6" />}
						isActive={activeKey === item.key}
						label={item.label}
						tabIndex={activeKey === item.key ? 0 : -1}
						onClick={() => handleNavigate(item.key)}
						onKeyDown={(e) => handleKeyDown(e, item.key)}
						itemRef={setRef(item.key)}
					/>
				))}
			</div>
		</nav>
	);
}
