import { Badge } from "@/components/ui/badge";
import { cn } from "@/lib/utils";
import dayjs from "dayjs";
import { BellOff } from "lucide-react";
import { ReactNode } from "react";
import Avatar, { genConfig, NiceAvatarProps } from "react-nice-avatar";

export interface ItemProps {
	type?: "group" | "user" | "assistant";
	name: string;
	organization?: string;
	count?: number;
	date?: string;
	avatar?: string | NiceAvatarProps | (() => ReactNode);
	isActive?: boolean;
	isMute?: boolean;
	onClick: () => void;
}

export default function Item({
	name,
	organization,
	count = 0,
	avatar,
	date,
	onClick,
	isActive = false,
	isMute = false,
}: ItemProps) {
	return (
		<div
			className={cn(
				"flex items-center py-3 px-2 space-x-2 cursor-pointer hover:bg-gray-200/50 overflow-hidden text-secondary-foreground",
				isActive && "bg-gray-200",
			)}
			onClick={onClick}
		>
			<div className="relative size-10">
				{typeof avatar === "string" ? (
					<img alt="avatar" className="size-10 min-w-10 rounded" src={avatar} />
				) : typeof avatar === "function" ? (
					avatar()
				) : (
					<Avatar
						className="size-10 min-w-10"
						shape="rounded"
						{...genConfig(avatar)}
					/>
				)}
				{count > 0 && (
					<Badge className="absolute -top-1 -right-1 flex items-center justify-center p-0 rounded-full size-4 !bg-rose-500 text-white">
						{count}
					</Badge>
				)}
			</div>
			<div className="flex flex-1 flex-col">
				<div className="flex justify-between">
					<div className="font-mono text-sm whitespace-nowrap text-ellipsis">
						{name}
						{organization && (
							<span className="text-xs text-market-primary">
								@{organization}
							</span>
						)}
					</div>
					<div className="flex flex-col">
						<div className="text-xs text-muted-foreground/50">
							{dayjs(date).format("HH:mm")}
						</div>
					</div>
				</div>
			</div>
		</div>
	);
}
