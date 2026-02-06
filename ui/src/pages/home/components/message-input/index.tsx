import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { cn } from "@/lib/utils";
import { useReactive } from "ahooks";
import { Check, ChevronDown, Command, CornerDownLeft, Earth, FolderClosed, Lightbulb, MessageCircleMore, Smile } from "lucide-react";
import { useMemo, useState } from "react";

interface MessageInputProps {
	className?: string;
	value?: string;
	onChange?: (e: React.ChangeEvent<HTMLInputElement> | React.ChangeEvent<HTMLTextAreaElement>) => void;
	stop?: () => void;
	onSubmit: (event?: {
		preventDefault?: () => void;
	}, chatRequestOptions?: any) => Promise<void> | void;
	status: 'error' | 'submitted' | 'streaming' | 'ready';
}

export default function MessageInput({ className, onSubmit, stop, onChange, status }: MessageInputProps) {
	const [showDropdown, setShowDropdown] = useState(false);
	const state = useReactive<{
		value: string;
		mode: number;
		enableSearch: boolean;
		enableThought: boolean;
		controller: AbortController | null;
	}>({
		value: '',
		mode: 1,
		enableSearch: true,
		enableThought: true,
		controller: null
	});

	const handleChange = (e: React.ChangeEvent<HTMLTextAreaElement>) => {
		state.value = e.target.value;
		onChange?.(e);
	};

	const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
		if ((e.ctrlKey || e.metaKey) && e.key === 'Enter') {
			if (2 === state.mode) {
				handleSubmit(e);
			} else {
				state.value = state.value + '\n';
				e.currentTarget.value = state.value;
				e.currentTarget.selectionStart = e.currentTarget.selectionEnd = state.value.length;
			}
			e.preventDefault();
			return;
		}
		if (e.key === 'Enter') {
			if (1 === state.mode) {
				handleSubmit(e);
			} else {
				state.value = state.value + '\n';
				e.currentTarget.value = state.value;
				e.currentTarget.selectionStart = e.currentTarget.selectionEnd = state.value.length;
			}
			e.preventDefault();
			return;
		}
	};

	const loading = useMemo(() => {
		return status === 'submitted' || status === 'streaming';
	}, [status]);

	const handleSubmit = (e: any) => {
		if (!state.value?.trim() || loading) {
			return;
		}
		onSubmit(e, state.value);
		state.value = '';
	}

	return (
		<form className={cn("h-full flex flex-col p-[12px]", className)}>
			<div className="flex justify-between items-center">
				<div className="flex flex-1 items-center space-x-1">
					<div className="cursor-pointer p-2 rounded hover:bg-slate-200">
						<Smile className="size-4" />
					</div>
					<div className="cursor-pointer p-2 rounded hover:bg-slate-200">
						<FolderClosed className="size-4" />
					</div>
					<div className="cursor-pointer p-2 rounded hover:bg-slate-200">
						<MessageCircleMore className="size-4" />
					</div>
				</div>
				<div className="flex items-center space-x-1">
					<div className="flex items-center space-x-2">
						<Switch id="airplane-mode" />
						<Label
							htmlFor="airplane-mode"
							className="text-xs text-muted-foreground/80"
						>
							自主讨论
						</Label>
					</div>
				</div>
			</div>
			<textarea
				className="w-full flex-1 py-[12px] px-[4px] text-[14px] focus:outline-none bg-transparent caret-primary resize-none no-scrollbar"
				name="prompt"
				value={state.value}
				onChange={handleChange}
				onKeyDown={handleKeyDown}
				placeholder="请输入您的问题"
			/>
			<div className="flex justify-between items-center">
				<div className="flex items-center gap-[8px]">
					<div
						className={
							cn(
								"flex items-center gap-[4px] text-[12px] border rounded-lg px-[8px] py-[4px] cursor-pointer hover:bg-gray-50",
								state.enableSearch ? 'border-primary text-primary' : ''
							)
						}
						onClick={() => state.enableSearch = !state.enableSearch}
					>
						<Earth className="size-[14px]" />
						联网搜索
					</div>
					<div
						className={
							cn(
								"flex items-center gap-[4px] text-[12px] border rounded-lg px-[8px] py-[4px] cursor-pointer hover:bg-gray-50",
								state.enableThought ? 'border-primary text-primary' : ''
							)
						}
						onClick={() => state.enableThought = !state.enableThought}
					>
						<Lightbulb className="size-[14px]" />
						深度思考
					</div>
				</div>
				<div className="flex items-center gap-[8px]">
					<div className="flex items-center text-[12px] text-gray-400">
						<div className="flex items-center gap-[4px]">
							<CornerDownLeft className="size-[12px]" />
							{1 === state.mode ? '发送' : '换行'}
						</div>
						<div className="mx-[4px]">/</div>
						<div className="flex items-center gap-[4px]">
							<Command className="size-[12px]" />
							<CornerDownLeft className="size-[12px]" />
							{2 === state.mode ? '发送' : '换行'}
						</div>
					</div>
					<div className="relative">
						<button
							type="button"
							className={cn(
								"flex items-center gap-2 px-3 py-1.5 rounded text-sm font-medium text-white bg-primary",
								!state.value?.trim() || loading ? 'opacity-50 cursor-not-allowed' : 'hover:bg-primary/90'
							)}
							disabled={!state.value?.trim() || loading}
							onClick={(e) => handleSubmit(e)}
						>
							发送
							<div
								className="cursor-pointer"
								onClick={(e) => {
									e.stopPropagation();
									setShowDropdown(!showDropdown);
								}}
							>
								<ChevronDown className="size-4" />
							</div>
						</button>
						{showDropdown && (
							<div className="absolute bottom-full right-0 mb-1 bg-white border rounded-lg shadow-lg py-1 min-w-[200px]">
								<div
									className="flex items-center px-3 py-2 hover:bg-gray-50 cursor-pointer"
									onClick={() => {
										state.mode = 1;
										setShowDropdown(false);
									}}
								>
									<div className="w-4 mr-2">
										{1 === state.mode && <Check className="size-4" />}
									</div>
									按 Enter 键发送
								</div>
								<div
									className="flex items-center px-3 py-2 hover:bg-gray-50 cursor-pointer"
									onClick={() => {
										state.mode = 2;
										setShowDropdown(false);
									}}
								>
									<div className="w-4 mr-2">
										{2 === state.mode && <Check className="size-4" />}
									</div>
									按<Command className="size-3.5 mx-1 inline" /> + Enter 键发送
								</div>
							</div>
						)}
					</div>
				</div>
			</div>
		</form >
	);
}
