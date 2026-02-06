import { Message } from '@/typings';
import { forwardRef, useImperativeHandle, useMemo, useRef } from 'react';
import { Virtuoso, VirtuosoHandle } from 'react-virtuoso';

export interface MessageListViewRef {
	scrollToIndex: (index: number, align?: 'start' | 'center' | 'end', behavior?: 'auto' | 'smooth') => void;
}

interface MessageListProps {
	className?: string;
	messages: Message[];
	itemRender: (item: Message, index: number) => JSX.Element | null;
}

const MessageList = forwardRef<MessageListViewRef, MessageListProps>(({
	className,
	messages,
	itemRender
}, ref) => {
	const virtuosoRef = useRef<VirtuosoHandle>(null);

	useImperativeHandle(ref, () => ({
		scrollToIndex: (index: number, align: 'start' | 'center' | 'end' = 'end', behavior: 'auto' | 'smooth' = 'auto') => {
			if (virtuosoRef.current) {
				virtuosoRef.current.scrollToIndex({ index, align, behavior });
			}
		}
	}));

	const data = useMemo(() => messages, [messages]);

	return (
		<Virtuoso
			className={className}
			ref={virtuosoRef}
			data={data}
			itemContent={(index, item) => itemRender(item, index)}
		/>
	);
});

export default MessageList;