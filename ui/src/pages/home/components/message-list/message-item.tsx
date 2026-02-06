import { cn } from "@/lib/utils";
import { Message } from "@/typings";
import { useReactive } from "ahooks";
import dayjs from "dayjs";
import { CircleCheckBig, RefreshCw, Sparkles, ThumbsDown, ThumbsUp } from "lucide-react";
import Avatar, { genConfig } from "react-nice-avatar";
import CopyButton from "../copy-button";
import { MemoizedMarkdown } from "../memoized-markdown";
import "./message-item.css";

export interface MessageItemProps extends Message {
	isSelf?: boolean;
	onClick?: () => void;
}

export default function MessageItem({
	isSelf,
	id,
	user,
	parts,
	type,
	createdAt,
	onClick,
}: MessageItemProps) {
	const loading = false;

	const renderContent = () => {
		const state = useReactive({
			isOpen: true
		});
		return (
			<div className="flex flex-col px-[4px]">
				{!isSelf && <div className="mb-2 inline-flex items-center box-border">
					<div className="inline-flex space-x-[4px] items-center font-medium py-2 px-[8px] rounded-lg bg-primary/20">
						<CircleCheckBig className="size-4 text-primary" />
						<span>已阅读知识库 2 个相关文件</span>
					</div>
				</div>}
				{parts?.map((part: any, index: number) => {
					if ('reasoning' === part?.type) {
						return (
							<div
								key={index}
								className="mb-[8px] p-[8px] relative flex flex-col cursor-pointer rounded bg-secondary"
							>
								<div
									className="font-bold text-[14px]"
									onClick={() => {
										state.isOpen = !state.isOpen
									}}
								>
									<div className="flex items-center gap-[4px]">
										<Sparkles className="size-[14px]" />思考和行动过程
									</div>
								</div>
								{state.isOpen && (
									<div className={cn(
										"text-[13px] font-[400] text-secondary-foreground/60",
										state.isOpen ? "mt-[4px]" : ""
									)}>
										{part?.details?.map((detail: any) =>
											detail?.type === 'text' ? detail?.text : '<redacted>',
										)}
									</div>
								)}
							</div>
						);
					}

					if ('text' === part?.type) {
						return <MemoizedMarkdown key={index} id={id} content={part?.text} />
					}
				})}
			</div>
		);
	};

	return (
		<div
			className={cn(
				"flex flex-col space-y-2 px-[12px] pt-[12px] pb-[24px]",
				isSelf ? "items-end" : "items-start",
			)}
			onClick={onClick}
		>
			<div className={cn("flex space-x-2", isSelf ? "flex-row-reverse" : "")}>
				{isSelf ? (
					<img alt="avatar" className="size-8 rounded" src={user?.avatar} />
				) : (
					<Avatar className="size-8" shape="rounded" {...genConfig({
						sex: "man",
						faceColor: "#F9C9B6",
						earSize: "big",
						eyeStyle: "circle",
						noseStyle: "long",
						mouthStyle: "peace",
						shirtStyle: "polo",
						glassesStyle: "round",
						hairColor: "#506AF4",
						hairStyle: "thick",
						hatStyle: "none",
						hatColor: "#F48150",
						shirtColor: "#F4D150",
						bgColor: "linear-gradient(45deg, #176fff 0%, #68ffef 100%)",
					})} />
				)}
				<div className="flex flex-col space-y-1">
					<div className="flex flex-col space-y-1">
						{!isSelf && (
							<div className="text-xs text-muted-foreground/50">楚留香</div>
						)}
						<div
							className={cn(
								"relative text-[13px] bg-gray-50 p-[8px] rounded max-w-[600px]",
								isSelf
									? "file" === type
										? "arrow-reverse bg-gray-50 text-card-foreground"
										: "arrow-reverse arrow-primary bg-primary text-primary-foreground"
									: "arrow",
							)}
						>
							{renderContent()}
						</div>
					</div>
					{!isSelf &&
						<div className="flex justify-between items-center text-muted-foreground/50 pt-2 cursor-pointer text-xs">
							{!loading ? <>
								<div className="flex items-center space-x-[2px]">
									<CopyButton content={''} />
									<div className="flex items-center space-x-1 py-[4px] px-[8px] cursor-pointer group">
										<RefreshCw className="group-hover:text-primary size-[14px] text-secondary-foreground/50" />
										<span className="group-hover:text-primary text-[12px] text-secondary-foreground/50">重新生成</span>
									</div>
								</div>
								<div className="flex items-center space-x-3">
									<ThumbsUp className="size-4 hover:text-primary" />
									<ThumbsDown className="size-4 hover:text-primary" />
								</div>
							</> : <></>}
						</div>
					}
				</div>
			</div>
			{createdAt && (
				<div className="mt-4 w-full text-center text-xs text-muted-foreground/50">
					{dayjs(createdAt).format("HH:mm")}
				</div>
			)}
		</div>
	);
}
