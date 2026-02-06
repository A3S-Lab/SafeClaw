import { ScrollArea } from "@/components/ui/scroll-area";
import { getAssetsUrl } from "@/lib/url";
import { cn } from "@/lib/utils";
import chatModel from "@/models/chat.model";
import { useReactive } from "ahooks";
import { useScroll } from "ahooks";
import { Expand, Minus, RotateCw, X } from "lucide-react";
import { useEffect, useRef } from "react";
import Draggable from "react-draggable";
import { ResizableBox } from "react-resizable";
import { useSnapshot } from "valtio";
import { User } from "../user";
import MomentList from "./moment-list";

import "./index.css";

export default function MomentPanel() {
	const { momentPanelOpen } = useSnapshot(chatModel.state);
	const ref = useRef(null);
	const scroll = useScroll(ref);

	const state = useReactive({
		x: window.innerWidth / 2 - 275,
		y: 30,
		width: 550,
		height: 1140,
		isResizing: false,
		headerOpacity: 0,
	});

	useEffect(() => {
		const opacity = scroll?.top! / 100;
		state.headerOpacity = opacity < 1 ? opacity : 1;
	}, [state.headerOpacity, scroll?.top]);

	const handleResize = (_: any, { size }: any) => {
		state.height = size.height;
	};

	return (
		<Draggable
			handle=".drag-handle"
			defaultPosition={{ x: state.x, y: state.y }}
			onDrag={(_, data) => {
				state.x = data.x;
				state.y = data.y;
			}}
		>
			<ResizableBox
				className={cn(
					"fixed top-0 z-10 bg-card rounded-2xl shadow-custom",
					momentPanelOpen ? "" : "hidden",
					state.isResizing
						? ""
						: "transition-[height] duration-300 ease-in-out",
				)}
				width={state.width}
				height={state.height}
				axis="y"
				resizeHandles={["n", "ne", "nw", "s", "se", "sw"]}
				minConstraints={[550, 550]}
				maxConstraints={[550, window.innerHeight]}
				onResize={handleResize}
				onResizeStart={() => {
					state.isResizing = true;
				}}
				onResizeStop={() => {
					state.isResizing = false;
				}}
			>
				<div className="drag-handle w-full h-full rounded-2xl box-border overflow-hidden">
					<ScrollArea
						ref={ref}
						className="h-full overflow-y-auto overflow-x-hidden"
					>
						<div
							className="fixed top-0 z-10 w-full flex items-center justify-between px-4 py-2 bg-transparent rounded-t-2xl"
							style={{
								backgroundColor: `rgba(240, 240, 240, ${state.headerOpacity})`,
								transition: "background-color 0.3s",
							}}
						>
							<div
								className="win-handle flex items-center space-x-3"
								onMouseDown={(e) => {
									e.stopPropagation();
								}}
							>
								<div
									className="btn btn-close cursor-pointer flex justify-center items-center p-[1px] size-3 rounded-full"
									onClick={() => {
										chatModel.setMomentPanelOpen(false);
									}}
								>
									<X className="size-4" />
								</div>
								<div
									className="btn btn-minmize cursor-pointer flex justify-center items-center p-[1px] size-3 rounded-full"
									onClick={() => {
										if (state.height > 550) {
											state.height = 550;
										} else {
											state.height = 1140;
										}
									}}
								>
									<Minus className="size-2" />
								</div>
								<div
									className="btn btn-maxmize cursor-pointer flex justify-center items-center p-[1px] size-3 rounded-full"
									onClick={() => {
										if (state.height < window.innerHeight) {
											state.height = window.innerHeight;
											state.y = 0;
										} else {
											state.height = 1140;
											state.y = 30;
										}
									}}
								>
									<Expand className="size-2" />
								</div>
							</div>
							{scroll?.top! > 50 && <div className="text-sm">朋友圈</div>}
							<div
								className="flex items-center space-x-2"
								onMouseDown={(e) => {
									e.stopPropagation();
								}}
							>
								<div
									className={cn(
										"p-2 rounded hover:bg-black/5 cursor-pointer",
										scroll?.top! > 50
											? "text-muted-foreground/40"
											: "text-primary-foreground",
									)}
								>
									<RotateCw className="size-6" />
								</div>
							</div>
						</div>
						<div
							className="relative h-[300px] rounded-t-2xl bg-bottom bg-cover bg-no-repeat"
							style={{
								backgroundImage: `url(${getAssetsUrl("images/avatar.jpg")})`,
							}}
						>
							<div className="h-full w-full rounded-t-2xl bg-black/10" />
							<div className="absolute -bottom-3 right-7 flex justify-end items-center w-full">
								<span className="text-base font-bold text-white mr-4 mb-2">
									RoyLin
								</span>
								<User size="2xl" />
							</div>
						</div>
						{momentPanelOpen && <MomentList />}
					</ScrollArea>
				</div>
			</ResizableBox>
		</Draggable>
	);
}
