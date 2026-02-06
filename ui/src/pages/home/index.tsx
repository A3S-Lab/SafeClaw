import { ResizableHandle, ResizablePanel, ResizablePanelGroup } from "@/components/ui/resizable";
import constants from "@/constants";
import globalModel from "@/models/global.model";
import { Message } from "@/typings";
import { useEffect, useRef, useState } from "react";
import { useSnapshot } from "valtio";
import Header from "./components/header";
import MessageInput from "./components/message-input";
import MessageList, { MessageListViewRef } from "./components/message-list";
import MessageItem from "./components/message-list/message-item";

type Status = 'ready' | 'submitted' | 'streaming' | 'error';

export default () => {
	const { user } = useSnapshot(globalModel.state);
	const [messages, setMessages] = useState<Message[]>([]);
	const [status, setStatus] = useState<Status>('ready');
	const listRef = useRef<MessageListViewRef>(null);

	useEffect(() => {
		if (messages.length > 0) {
			listRef.current?.scrollToIndex(messages.length);
		}
	}, [messages.length]);

	const handleSubmit = async (_e: unknown, value: string) => {
		if (!value?.trim()) return;

		const userMessage: Message = {
			id: Date.now().toString(),
			role: 'user',
			content: value,
			type: 'text',
			createdAt: new Date().toISOString(),
			parts: [{ type: 'text', text: value }]
		};

		setMessages(prev => [...prev, userMessage]);
		setStatus('submitted');

		try {
			const resp = await fetch(`${constants.gatewayUrl}/message`, {
				method: 'POST',
				headers: { 'Content-Type': 'application/json' },
				body: JSON.stringify({
					channel: 'webchat',
					chat_id: user.id?.toString() || 'default',
					content: value,
				}),
			});

			const data = await resp.json();

			const assistantMessage: Message = {
				id: data.message_id || (Date.now() + 1).toString(),
				role: 'assistant',
				content: data.status === 'sent'
					? `Message delivered (ID: ${data.message_id})`
					: (data.status || 'No response from gateway'),
				type: 'text',
				createdAt: new Date().toISOString(),
				parts: [{
					type: 'text',
					text: data.status === 'sent'
						? `Message delivered (ID: ${data.message_id})`
						: (data.status || 'No response from gateway'),
				}],
			};
			setMessages(prev => [...prev, assistantMessage]);
			setStatus('ready');
		} catch (err) {
			const errorMessage: Message = {
				id: (Date.now() + 1).toString(),
				role: 'assistant',
				content: `Failed to connect to gateway at ${constants.gatewayUrl}. Is it running?\n\nStart it with: \`safeclaw gateway\``,
				type: 'text',
				createdAt: new Date().toISOString(),
				parts: [{
					type: 'text',
					text: `Failed to connect to gateway at ${constants.gatewayUrl}. Is it running?\n\nStart it with: \`safeclaw gateway\``,
				}],
			};
			setMessages(prev => [...prev, errorMessage]);
			setStatus('error');
		}
	};

	return (
		<ResizablePanelGroup className="bg-gray-200/20" direction="vertical">
			<ResizablePanel>
				<Header />
				<MessageList
					className="w-full h-full overflow-x-hidden pt-[14px] pb-[60px]"
					ref={listRef}
					messages={messages}
					itemRender={(message, index) => (
						<MessageItem key={index} isSelf={message.role === 'user'} user={user} {...message} />
					)}
				/>
			</ResizablePanel>
			<ResizableHandle />
			<ResizablePanel defaultSize={20} minSize={10} maxSize={35}>
				<MessageInput
					status={status}
					onSubmit={handleSubmit}
				/>
			</ResizablePanel>
		</ResizablePanelGroup>
	);
};
