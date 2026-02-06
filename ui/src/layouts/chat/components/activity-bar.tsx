import { Badge } from "@/components/ui/badge";
import constants from "@/constants";
import { cn } from "@/lib/utils";
import chatModel from "@/models/chat.model";
import { useLocalStorageState } from "ahooks";
import {
	Aperture,
	Box,
	FileCode2,
	LayoutGrid,
	MessageCircle,
	Settings,
	Workflow
} from "lucide-react";
import { ReactNode, useEffect } from "react";
import { useNavigate } from "react-router-dom";
import { useSnapshot } from "valtio";
import { User } from "./user";

interface ActivityItemProps {
	isActive: boolean;
	count?: number;
	icon: ReactNode;
	onClick: () => void;
}

const ActivityItem = ({
	icon,
	isActive,
	count = 0,
	onClick,
}: ActivityItemProps) => {
	return (
		<div
			className={cn(
				"flex flex-col justify-center items-center w-full h-12 cursor-pointer hover:text-primary-foreground",
				isActive && "text-primary-foreground",
			)}
			onClick={onClick}
		>
			<div className="relative size-6">
				{icon}
				{count > 0 && (
					<Badge className="absolute -top-1 -right-2 flex items-center justify-center p-0 rounded-full size-4 !bg-rose-500 text-white">
						{count}
					</Badge>
				)}
			</div>
		</div>
	);
};

export default function ActivityBar() {
	const { activeKey } = useSnapshot(chatModel.state);
	const nav = useNavigate();

	const [activeKeyState, setActiveKeyState] = useLocalStorageState(
		`${constants.localStorageKeyPrefix}-active-key`,
		{
			defaultValue: "chat",
		});

	useEffect(() => {
		chatModel.setActiveKey(activeKeyState!);
	}, [activeKeyState]);

	useEffect(() => {
		nav(`/${activeKey === "chat" ? "" : activeKey}`);
	}, [activeKey]);

	return (
		<aside className="flex flex-col h-full w-[48] bg-primary text-primary-foreground/60 shadow-lg">
			<User />
			<div className="flex-1">
				<ActivityItem
					icon={<MessageCircle className="size-6" />}
					isActive={"chat" === activeKey}
					count={1}
					onClick={() => {
						setActiveKeyState("chat");
					}}
				/>
				<ActivityItem
					icon={<Box className="size-6" />}
					isActive={"base" === activeKey}
					onClick={() => {
						setActiveKeyState("base");
					}}
				/>
				<ActivityItem
					icon={<FileCode2 className="size-6" />}
					isActive={"code" === activeKey}
					onClick={() => {
						setActiveKeyState("code");
					}}
				/>
				<ActivityItem
					icon={<Workflow className="size-6" />}
					isActive={"workflow" === activeKey}
					onClick={() => {
						setActiveKeyState("workflow");
					}}
				/>
				<ActivityItem
					icon={<LayoutGrid className="size-6" />}
					isActive={"market" === activeKey}
					onClick={() => {
						setActiveKeyState("market");
					}}
				/>
				<ActivityItem
					icon={<Aperture className="size-6" />}
					isActive={"moment" === activeKey}
					onClick={() => {
						chatModel.setMomentPanelOpen(true);
					}}
				/>
			</div>
			<ActivityItem icon={<Settings />} isActive onClick={() => { }} />
		</aside>
	);
}
