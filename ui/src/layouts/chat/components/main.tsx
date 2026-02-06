import chatModel from "@/models/chat.model";
import { ReactNode } from "react";
import { useSnapshot } from "valtio";
import ChatPanel from "./chat-panel";

export default function Main({ children }: { children: ReactNode }) {
	const { activeKey } = useSnapshot(chatModel.state);

	return (
		<aside className="flex flex-col h-full w-full overflow-hidden">
			{!['chat'].includes(activeKey) ? children :
				<div className="flex h-full">
					<div className="w-[320px] min-w-[200px] max-w-[40%] border-r">
						{"chat" === activeKey && <ChatPanel />}
					</div>
					<div className="flex-1">
						{children}
					</div>
				</div>}
		</aside>
	);
}
